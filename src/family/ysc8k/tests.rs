use std::cell::RefCell;
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::*;
use crate::backend::Signal;
use crate::capability::{Direction, Execution, StableId};
use crate::click::{Click, ClickTiming, Holds};
use crate::connect::Connect;
use crate::device::{Device, Start};
use crate::event::{Event, EventKinds};
use crate::key::{Key, MediaKey};
use crate::transport::fake::{FakeBox, FakeSystem};

const WAIT: Duration = Duration::from_secs(3);

fn wire(text: &str) -> Vec<u8> {
    let total = u16::try_from(text.len() + 14).unwrap();
    [
        b"<START>".as_slice(),
        &total.to_be_bytes(),
        text.as_bytes(),
        b"<END>",
    ]
    .concat()
}

fn upload(b: i64, x: i64, y: i64, wh: i64) -> Vec<u8> {
    wire(&format!(
        r#"{{"code":200,"message":null,"data":"{{\"b\":{b},\"x\":{x},\"y\":{y},\"wh\":{wh}}}"}}"#
    ))
}

struct BoxState {
    version: String,
    debug: Option<String>,
    echo: bool,
    wrong_echo: bool,
    partial: bool,
    partial_text: bool,
    key_error: bool,
    key_silent: bool,
    key_unreadable: bool,
    /// echo is off, and `km.echo(1)` turns it on without echoing itself
    echo_switch: bool,
}

impl Default for BoxState {
    fn default() -> Self {
        Self {
            version: "ysc-towmcu-L v1.0".into(),
            debug: Some(r#"{"code":702,"data":"{\"debug\":false}"}"#.into()),
            echo: true,
            wrong_echo: false,
            partial: false,
            partial_text: false,
            key_error: false,
            key_silent: false,
            key_unreadable: false,
            echo_switch: false,
        }
    }
}

fn respond(state: &mut BoxState, bytes: &[u8]) -> Vec<Vec<u8>> {
    if bytes == b"<START>\x00\x19{\"cmd\":132}<END>" {
        return vec![wire(&format!(
            "{{\"code\":200,\"message\":\"{}\",\"data\":\"\"}}",
            state.version
        ))];
    }
    if bytes == b"<START>\x00\x19{\"cmd\":702}<END>" {
        return state.debug.as_deref().map(wire).into_iter().collect();
    }
    if bytes.starts_with(b"<START>") {
        if bytes.windows(8).any(|bytes| bytes == b"\"cmd\":45") {
            if state.key_silent {
                return Vec::new();
            }
            if state.key_unreadable {
                return vec![wire(r#"{"code":400,"message":"2","data":null}"#)];
            }
            if state.key_error {
                return vec![wire(r#"{"code":404,"message":"no_keyboard","data":""}"#)];
            }
            let message = if bytes.ends_with(b"\"down\":1}<END>") {
                "key_down"
            } else {
                "key_up"
            };
            return vec![wire(&format!(
                "{{\"code\":200,\"message\":\"{message}\",\"data\":null}}"
            ))];
        }
        return Vec::new();
    }
    if !state.echo {
        state.echo = state.echo_switch && bytes == b"km.echo(1)\r\n";
        return Vec::new();
    }
    if bytes == b"km.echo(1)\r\n" {
        return vec![b"km.echo(1)\r\n>>> ".to_vec()];
    }
    if state.partial {
        return vec![b"<START>\x00\x80{".to_vec()];
    }
    if state.partial_text {
        return vec![bytes.to_vec()];
    }
    if state.wrong_echo {
        return vec![b"km.move(99,99)\r\n>>> ".to_vec()];
    }
    vec![[bytes, b">>> "].concat()]
}

fn usb(role: Option<&str>) -> Option<crate::transport::PortUsb> {
    Some(crate::transport::PortUsb {
        vid: 0x1a86,
        pid: 0xfe0c,
        serial: role.map(str::to_owned),
    })
}

struct Rig {
    system: FakeSystem,
    fake: Arc<FakeBox>,
    state: Arc<Mutex<BoxState>>,
}

impl Rig {
    fn new() -> Self {
        Self::with(BoxState::default())
    }

    fn with(state: BoxState) -> Self {
        let state = Arc::new(Mutex::new(state));
        let fake = FakeBox::new({
            let state = state.clone();
            move |bytes| respond(&mut state.lock().unwrap(), bytes)
        });
        let system = FakeSystem::default();
        system.plug("COM7", usb(Some("TOWMCULEFT")), fake.clone());
        Self {
            system,
            fake,
            state,
        }
    }

    fn candidate(&self) -> Candidate {
        self.system
            .ports()
            .into_iter()
            .find(|c| c.path == Path::new("COM7"))
            .unwrap()
    }

    fn attach(&self) -> Result<(YscBackend, Snapshot, Receiver<Signal>)> {
        let (tx, rx) = mpsc::channel();
        let (backend, snapshot) =
            attach(Arc::new(self.system.clone()), &self.candidate(), Sink(tx))?;
        Ok((backend, snapshot, rx))
    }

    fn device(&self) -> Device {
        let (tx, rx) = mpsc::channel();
        let (backend, snapshot) = attach(
            Arc::new(self.system.clone()),
            &self.candidate(),
            Sink(tx.clone()),
        )
        .unwrap();
        Device::start(Start {
            backend: Box::new(backend),
            snapshot,
            signals: tx,
            incoming: rx,
            click_timing: ClickTiming::Fixed(Duration::from_millis(15)),
            holds: Holds::seeded(42),
            port: "COM7".into(),
        })
        .unwrap()
    }
}

fn wait_for(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !condition() {
        assert!(
            Instant::now() < deadline,
            "the expected state never arrived"
        );
        thread::yield_now();
    }
}

fn signal(rx: &Receiver<Signal>, accept: impl Fn(&Signal) -> bool) -> Signal {
    let deadline = Instant::now() + WAIT;
    loop {
        let next = rx
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap();
        if accept(&next) {
            return next;
        }
    }
}

#[test]
fn open_writes_one_identity_one_optional_status_upload_off_and_waits_for_the_echo() {
    let rig = Rig::new();
    let (backend, snapshot, _) = rig.attach().unwrap();
    assert_eq!(
        rig.fake.written(),
        [
            b"<START>\x00\x19{\"cmd\":132}<END>".to_vec(),
            b"<START>\x00\x19{\"cmd\":702}<END>".to_vec(),
            b"<START>\x00\x23{\"cmd\":34,\"status\":0}<END>".to_vec(),
            b"km.echo(1)\r\n".to_vec(),
        ]
    );
    assert_eq!(rig.system.opened_at(), [115200]);
    assert_eq!(snapshot.info.family, Family::Ysc);
    assert_eq!(snapshot.info.model.as_deref(), Some("YSC 8K V2"));
    assert_eq!(
        snapshot.info.firmware,
        Some(Firmware::Text("ysc-towmcu-L v1.0".into()))
    );
    assert_eq!(snapshot.info.id, None);
    assert_eq!(snapshot.info.ferrum_api, None);
    assert!(!snapshot.capabilities.get(Action::Click).is_supported());
    assert!(!snapshot.capabilities.get(Action::KeyTap).is_supported());
    backend.shutdown(WRITE_DEADLINE);
}

macro_rules! output_test {
    ($name:ident, $call:expr, $expected:expr) => {
        #[test]
        fn $name() {
            let rig = Rig::new();
            let device = rig.device();
            rig.fake.clear_written();
            ($call)(&device).unwrap();
            assert_eq!(rig.fake.written(), vec![$expected.to_vec()]);
        }
    };
}

output_test!(
    move_uses_the_documented_text_example,
    |d: &Device| d.move_by(100, 50),
    b"km.move(100,50)\r\n"
);
output_test!(
    move_preserves_signed_i16_limits,
    |d: &Device| d.move_by(i16::MIN, i16::MAX),
    b"km.move(-32768,32767)\r\n"
);
output_test!(
    left_press_uses_one,
    |d: &Device| d.press(Button::Left),
    b"km.left(1)\r\n"
);
output_test!(
    left_release_uses_zero,
    |d: &Device| d.release(Button::Left),
    b"km.left(0)\r\n"
);
output_test!(
    right_press_uses_one,
    |d: &Device| d.press(Button::Right),
    b"km.right(1)\r\n"
);
output_test!(
    right_release_uses_zero,
    |d: &Device| d.release(Button::Right),
    b"km.right(0)\r\n"
);
output_test!(
    middle_press_uses_one,
    |d: &Device| d.press(Button::Middle),
    b"km.middle(1)\r\n"
);
output_test!(
    middle_release_uses_zero,
    |d: &Device| d.release(Button::Middle),
    b"km.middle(0)\r\n"
);
output_test!(
    side1_uses_ms1,
    |d: &Device| d.press(Button::Side1),
    b"km.ms1(1)\r\n"
);
output_test!(
    side1_release_uses_ms1_zero,
    |d: &Device| d.release(Button::Side1),
    b"km.ms1(0)\r\n"
);
output_test!(
    side2_uses_ms2,
    |d: &Device| d.press(Button::Side2),
    b"km.ms2(1)\r\n"
);
output_test!(
    side2_release_uses_ms2_zero,
    |d: &Device| d.release(Button::Side2),
    b"km.ms2(0)\r\n"
);
output_test!(
    wheel_up_uses_one,
    |d: &Device| d.wheel(1),
    b"km.wheel(1)\r\n"
);
output_test!(
    wheel_down_uses_minus_one,
    |d: &Device| d.wheel(-1),
    b"km.wheel(-1)\r\n"
);
output_test!(
    key_press_is_write_only_json,
    |d: &Device| d.press_key(Key::A),
    b"<START>\x00\x28{\"cmd\":45,\"kc\":4,\"down\":1}<END>"
);
output_test!(
    key_release_is_write_only_json,
    |d: &Device| d.release_key(Key::A),
    b"<START>\x00\x28{\"cmd\":45,\"kc\":4,\"down\":0}<END>"
);
output_test!(
    modifier_is_a_keyboard_usage,
    |d: &Device| d.press_key(Key::RIGHT_GUI),
    b"<START>\x00\x2a{\"cmd\":45,\"kc\":231,\"down\":1}<END>"
);
output_test!(
    lock_left_uses_ml,
    |d: &Device| d.lock_button(Button::Left),
    b"km.lock_ml(1)\r\n"
);
output_test!(
    unlock_left_uses_ml_zero,
    |d: &Device| d.unlock_button(Button::Left),
    b"km.lock_ml(0)\r\n"
);
output_test!(
    lock_right_uses_mr,
    |d: &Device| d.lock_button(Button::Right),
    b"km.lock_mr(1)\r\n"
);
output_test!(
    unlock_right_uses_mr_zero,
    |d: &Device| d.unlock_button(Button::Right),
    b"km.lock_mr(0)\r\n"
);
output_test!(
    lock_middle_uses_mm,
    |d: &Device| d.lock_button(Button::Middle),
    b"km.lock_mm(1)\r\n"
);
output_test!(
    unlock_middle_uses_mm_zero,
    |d: &Device| d.unlock_button(Button::Middle),
    b"km.lock_mm(0)\r\n"
);
output_test!(
    lock_side1_uses_ms1,
    |d: &Device| d.lock_button(Button::Side1),
    b"km.lock_ms1(1)\r\n"
);
output_test!(
    unlock_side1_uses_ms1_zero,
    |d: &Device| d.unlock_button(Button::Side1),
    b"km.lock_ms1(0)\r\n"
);
output_test!(
    lock_side2_uses_ms2,
    |d: &Device| d.lock_button(Button::Side2),
    b"km.lock_ms2(1)\r\n"
);
output_test!(
    unlock_side2_uses_ms2_zero,
    |d: &Device| d.unlock_button(Button::Side2),
    b"km.lock_ms2(0)\r\n"
);
output_test!(
    lock_x_writes_one_whole_axis_command,
    |d: &Device| d.lock_axis(Axis::X, Direction::Both),
    b"km.lock_mx(1)\r\n"
);
output_test!(
    lock_y_writes_one_whole_axis_command,
    |d: &Device| d.lock_axis(Axis::Y, Direction::Both),
    b"km.lock_my(1)\r\n"
);

#[test]
fn zero_wheel_large_wheel_and_unsupported_operations_write_nothing() {
    let rig = Rig::new();
    let device = rig.device();
    rig.fake.clear_written();
    device.wheel(0).unwrap();
    for result in [
        device.wheel(2),
        device.wheel(-2),
        device.pan(1),
        device.move_now(1, 2),
        device.discard_pending_moves(),
        device.press(Button::Extra(5)),
        device.lock_key(Key::A),
        device.press_media(MediaKey::VOLUME_UP),
        device.lock_axis(Axis::Wheel, Direction::Both),
        device.lock_axis(Axis::Pan, Direction::Both),
        device.lock_axis(Axis::X, Direction::Positive),
        device.unlock_axis(Axis::Y, Direction::Negative),
        device.scale_axis(Axis::X, Direction::Both, 50),
    ] {
        assert!(matches!(
            result,
            Err(Error::Unsupported { .. }) | Err(Error::InvalidArgument { .. })
        ));
    }
    assert!(rig.fake.written().is_empty());
}

#[test]
fn clicks_and_taps_are_host_timed_and_native_only_is_refused_before_writing() {
    let rig = Rig::new();
    let device = rig.device();
    assert_eq!(
        device.snapshot().capabilities.get(Action::Click).execution,
        Execution::Host
    );
    assert_eq!(
        device.snapshot().capabilities.get(Action::KeyTap).execution,
        Execution::Host
    );
    rig.fake.clear_written();
    assert!(matches!(
        device.click_with(Button::Left, Click::default().native_only()),
        Err(Error::Unsupported { .. })
    ));
    assert!(matches!(
        device.tap_key_with(Key::A, Click::default().native_only()),
        Err(Error::Unsupported { .. })
    ));
    assert!(rig.fake.written().is_empty());
    for (button, name) in [
        (Button::Left, "left"),
        (Button::Right, "right"),
        (Button::Middle, "middle"),
        (Button::Side1, "ms1"),
        (Button::Side2, "ms2"),
    ] {
        rig.fake.clear_written();
        let before = Instant::now();
        device.click(button).unwrap();
        assert!(before.elapsed() >= Duration::from_millis(15));
        assert_eq!(
            rig.fake.written(),
            [
                format!("km.{name}(1)\r\n").into_bytes(),
                format!("km.{name}(0)\r\n").into_bytes()
            ]
        );
    }
    rig.fake.clear_written();
    let before = Instant::now();
    device
        .tap_key_with(Key::A, Click::hold(Duration::from_millis(20)))
        .unwrap();
    assert!(before.elapsed() >= Duration::from_millis(20));
    assert_eq!(
        rig.fake.written(),
        [
            wire(r#"{"cmd":45,"kc":4,"down":1}"#),
            wire(r#"{"cmd":45,"kc":4,"down":0}"#)
        ]
    );
}

#[test]
fn buttons_and_motion_share_one_write_only_upload_switch() {
    let rig = Rig::new();
    let device = rig.device();
    rig.fake.clear_written();
    let buttons = device.events_of(EventKinds::BUTTONS);
    let motion = device.events_of(EventKinds::MOTION);
    assert_eq!(rig.fake.written(), [wire(r#"{"cmd":34,"status":1}"#)]);
    drop(buttons);
    assert_eq!(rig.fake.written().len(), 1);
    drop(motion);
    assert_eq!(
        rig.fake.written(),
        [
            wire(r#"{"cmd":34,"status":1}"#),
            wire(r#"{"cmd":34,"status":0}"#)
        ]
    );
}

#[test]
fn keys_and_media_streams_are_refused_without_writes() {
    let rig = Rig::new();
    let (backend, _, _) = rig.attach().unwrap();
    rig.fake.clear_written();
    for kind in [InputKind::Keys, InputKind::Media] {
        assert!(matches!(
            backend.set_streams(kind.kinds()),
            Err(StreamFailure {
                error: Error::Unsupported { .. },
                ..
            })
        ));
        assert!(matches!(
            backend.restart_stream(kind),
            Err(Error::Unsupported { .. })
        ));
    }
    assert!(rig.fake.written().is_empty());
}

#[test]
fn uploads_have_no_box_clock_seed_buttons_and_filter_receivers() {
    let rig = Rig::new();
    let device = rig.device();
    let buttons = device.events_of(EventKinds::BUTTONS);
    let motion = device.events_of(EventKinds::MOTION);
    rig.fake.send(&upload(1, 1, 2, 0));
    assert!(
        matches!(motion.recv_timeout(WAIT).unwrap(), Event::Motion { dx: 1, dy: 2, at, .. } if at.box_time.is_none())
    );
    device.settle();
    assert!(buttons.try_recv().is_err());
    rig.fake.send(&upload(2, -3, 4, -1));
    assert!(matches!(
        buttons.recv_timeout(WAIT).unwrap(),
        Event::Button {
            button: Button::Left,
            pressed: false,
            ..
        }
    ));
    assert!(matches!(
        buttons.recv_timeout(WAIT).unwrap(),
        Event::Button {
            button: Button::Right,
            pressed: true,
            ..
        }
    ));
    assert!(matches!(
        motion.recv_timeout(WAIT).unwrap(),
        Event::Motion {
            dx: -3,
            dy: 4,
            wheel: -1,
            pan: 0,
            ..
        }
    ));
    assert!(motion.try_recv().is_err());
}

#[test]
fn disabling_the_stream_forgets_the_button_baseline() {
    let rig = Rig::new();
    let device = rig.device();
    let events = device.events_of(EventKinds::BUTTONS | EventKinds::MOTION);
    rig.fake.send(&upload(1, 1, 0, 0));
    assert!(matches!(
        events.recv_timeout(WAIT).unwrap(),
        Event::Motion { .. }
    ));
    drop(events);
    let events = device.events_of(EventKinds::BUTTONS | EventKinds::MOTION);
    rig.fake.send(&upload(0, 1, 0, 0));
    assert!(matches!(
        events.recv_timeout(WAIT).unwrap(),
        Event::Motion { .. }
    ));
    assert!(events.try_recv().is_err());
}

#[test]
fn debug_mode_refuses_before_echo_stream_setup_or_output() {
    let rig = Rig::with(BoxState {
        debug: Some(r#"{"code":702,"data":{"debug":true}}"#.into()),
        ..BoxState::default()
    });
    let error = rig.attach().err().unwrap();
    assert!(matches!(
        error,
        Error::Rejected {
            action: Action::Connect,
            ..
        }
    ));
    assert!(
        error
            .to_string()
            .contains("leave debug mode in the vendor tool and connect again")
    );
    assert_eq!(
        rig.fake.written(),
        [wire(r#"{"cmd":132}"#), wire(r#"{"cmd":702}"#)]
    );
    assert_eq!(rig.system.open_ports(), 0);
}

#[test]
fn absent_malformed_or_unrelated_debug_replies_carry_on_once() {
    for debug in [
        None,
        Some(r#"{"code":702,"data":"broken"}"#.into()),
        Some(r#"{"code":200,"data":{"debug":true}}"#.into()),
    ] {
        let rig = Rig::with(BoxState {
            debug,
            ..BoxState::default()
        });
        let before = Instant::now();
        let (backend, _, _) = rig.attach().unwrap();
        assert!(before.elapsed() >= Duration::from_millis(1200));
        assert!(before.elapsed() < Duration::from_millis(1500));
        assert_eq!(rig.fake.written().len(), 4);
        assert_eq!(rig.fake.written()[1], wire(r#"{"cmd":702}"#));
        backend.send(Command::Move(1, 2)).unwrap();
    }
}

#[test]
fn a_key_waits_for_the_box_to_answer_it() {
    let rig = Rig::new();
    let (backend, _, _) = rig.attach().unwrap();
    backend.send(Command::KeyPress(Key::A)).unwrap();
    rig.state.lock().unwrap().key_silent = true;
    assert!(matches!(
        backend.send(Command::KeyRelease(Key::A)),
        Err(Error::Disconnected)
    ));
}

#[test]
fn a_key_the_box_couldnt_read_is_refused_and_the_link_stays_up() {
    let rig = Rig::new();
    let (backend, _, _) = rig.attach().unwrap();
    backend.send(Command::KeyPress(Key::A)).unwrap();
    rig.state.lock().unwrap().key_unreadable = true;
    assert!(matches!(
        backend.send(Command::KeyRelease(Key::A)),
        Err(Error::Rejected { .. })
    ));
    rig.state.lock().unwrap().key_unreadable = false;
    backend.send(Command::KeyRelease(Key::A)).unwrap();
    backend.send(Command::Move(1, 2)).unwrap();
}

#[test]
fn a_box_with_echo_turned_off_still_opens() {
    let rig = Rig::with(BoxState {
        echo: false,
        echo_switch: true,
        ..BoxState::default()
    });
    let (backend, _, _) = rig.attach().unwrap();
    backend.send(Command::Move(1, 2)).unwrap();
}

#[test]
fn a_box_that_ignores_text_at_open_is_refused_with_what_to_do() {
    let rig = Rig::with(BoxState {
        echo: false,
        ..BoxState::default()
    });
    assert!(
        matches!(rig.attach(), Err(Error::Rejected { action: Action::Connect, detail }) if detail.contains("text commands"))
    );
}

#[test]
fn a_late_debug_reply_cannot_answer_text_output() {
    let rig = Rig::with(BoxState {
        debug: None,
        ..BoxState::default()
    });
    let (backend, _, _) = rig.attach().unwrap();
    rig.state.lock().unwrap().echo = false;
    rig.fake
        .send(&wire(r#"{"code":702,"data":{"debug":true}}"#));
    rig.fake.send(b">>> ");
    assert!(matches!(
        backend.send(Command::Move(1, 2)),
        Err(Error::Disconnected)
    ));
}

#[test]
fn wrong_echo_cannot_answer_the_command_and_a_timeout_reprobes() {
    let rig = Rig::with(BoxState {
        wrong_echo: true,
        ..BoxState::default()
    });
    let (backend, _, rx) = rig.attach().unwrap();
    assert!(matches!(
        backend.send(Command::Move(1, 2)),
        Err(Error::Disconnected)
    ));
    signal(&rx, |signal| matches!(signal, Signal::Disconnected));
    rig.state.lock().unwrap().wrong_echo = false;
    signal(&rx, |signal| matches!(signal, Signal::Reconnected(_)));
    backend.send(Command::Move(3, 4)).unwrap();
}

#[test]
fn stuck_partial_json_times_out_and_reconnect_clears_both_codec_accumulators() {
    let rig = Rig::with(BoxState {
        partial: true,
        ..BoxState::default()
    });
    let (backend, _, rx) = rig.attach().unwrap();
    assert!(matches!(
        backend.send(Command::Move(1, 2)),
        Err(Error::Disconnected)
    ));
    signal(&rx, |signal| matches!(signal, Signal::Disconnected));
    rig.state.lock().unwrap().partial = false;
    signal(&rx, |signal| matches!(signal, Signal::Reconnected(_)));
    backend.send(Command::Move(3, 4)).unwrap();
    assert_eq!(
        rig.fake
            .written()
            .iter()
            .filter(|bytes| *bytes == &wire(r#"{"cmd":702}"#))
            .count(),
        2
    );
}

#[test]
fn every_no_keyboard_disables_keys_and_only_one_action_kind_can_be_attributed() {
    for (commands, expected) in [
        (vec![], None),
        (vec![Command::KeyPress(Key::A)], Some(Action::KeyPress)),
        (
            vec![Command::KeyPress(Key::A), Command::KeyPress(Key::B)],
            Some(Action::KeyPress),
        ),
        (vec![Command::KeyRelease(Key::A)], Some(Action::KeyRelease)),
        (
            vec![Command::KeyPress(Key::A), Command::KeyRelease(Key::A)],
            None,
        ),
        (
            vec![Command::KeyRelease(Key::A), Command::KeyPress(Key::B)],
            None,
        ),
    ] {
        let rig = Rig::new();
        let (backend, _, rx) = rig.attach().unwrap();
        for command in commands {
            backend.send(command).unwrap();
        }
        rig.fake
            .send(&wire(r#"{"code":404,"message":"no_keyboard","data":""}"#));
        let changed = signal(&rx, |signal| matches!(signal, Signal::Changed(_)));
        let Signal::Changed(snapshot) = changed else {
            unreachable!()
        };
        for action in [Action::KeyPress, Action::KeyRelease, Action::KeyTap] {
            assert!(!snapshot.capabilities.get(action).is_supported());
            assert!(
                snapshot
                    .capabilities
                    .get(action)
                    .reason()
                    .contains("keyboard behind")
            );
        }
        if let Some(action) = expected {
            assert!(
                matches!(signal(&rx, |s| matches!(s, Signal::Refused(_))), Signal::Refused(found) if found == action)
            );
        } else {
            assert!(rx.recv_timeout(Duration::from_millis(40)).is_err());
        }
        rig.fake.clear_written();
        assert!(matches!(
            backend.send(Command::KeyPress(Key::C)),
            Err(Error::Unsupported { .. })
        ));
        assert!(rig.fake.written().is_empty());
        // keys are already off, so another refusal changes nothing and says nothing
        rig.fake
            .send(&wire(r#"{"code":404,"message":"no_keyboard"}"#));
        assert!(rx.recv_timeout(Duration::from_millis(40)).is_err());
    }
}

#[test]
fn an_error_that_beats_write_return_disables_the_core_tap_snapshot() {
    let rig = Rig::new();
    let device = rig.device();
    let events = device.events_of(EventKinds::NONE);
    rig.fake.reply_before_return.store(true, Ordering::Release);
    rig.state.lock().unwrap().key_error = true;
    device.press_key(Key::A).unwrap();
    assert!(matches!(
        past_changed(&events),
        Event::Rejected {
            action: Action::KeyPress,
            count: 1
        }
    ));
    wait_for(|| {
        !device
            .snapshot()
            .capabilities
            .get(Action::KeyTap)
            .is_supported()
    });
    rig.fake.clear_written();
    assert!(matches!(
        device.tap_key(Key::B),
        Err(Error::Unsupported { .. })
    ));
    assert!(rig.fake.written().is_empty());
    device.release_key(Key::A).unwrap();
    assert_eq!(rig.fake.written(), [wire(r#"{"cmd":45,"kc":4,"down":0}"#)]);
}

#[test]
fn cleanup_releases_make_later_refusals_ambiguous() {
    let rig = Rig::new();
    let (backend, _, rx) = rig.attach().unwrap();
    backend.send(Command::KeyPress(Key::A)).unwrap();
    backend.release_all(&[Usage::Key(Key::A)]).unwrap();
    rig.fake
        .send(&wire(r#"{"code":404,"message":"no_keyboard"}"#));
    signal(&rx, |signal| matches!(signal, Signal::Changed(_)));
    assert!(rx.recv_timeout(Duration::from_millis(40)).is_err());
}

#[test]
fn whole_axis_slots_relock_and_cleanup_work_in_both_orders() {
    for axis in [Axis::X, Axis::Y] {
        for signs in [
            [Sign::Positive, Sign::Negative],
            [Sign::Negative, Sign::Positive],
        ] {
            let rig = Rig::new();
            let (backend, _, _) = rig.attach().unwrap();
            rig.fake.clear_written();
            for sign in signs {
                backend
                    .send(Command::Lock {
                        slot: LockSlot::Axis(axis, sign),
                        keep: LOCKED,
                    })
                    .unwrap();
            }
            for sign in signs {
                backend
                    .send(Command::Lock {
                        slot: LockSlot::Axis(axis, sign),
                        keep: LOCKED,
                    })
                    .unwrap();
            }
            backend
                .unlock_all(&signs.map(|sign| LockSlot::Axis(axis, sign)))
                .unwrap();
            let name = if axis == Axis::X { "mx" } else { "my" };
            assert_eq!(
                rig.fake.written(),
                [
                    format!("km.lock_{name}(1)\r\n").into_bytes(),
                    format!("km.lock_{name}(1)\r\n").into_bytes(),
                    format!("km.lock_{name}(0)\r\n").into_bytes(),
                    format!("km.lock_{name}(0)\r\n").into_bytes()
                ]
            );
            for sign in signs {
                rig.fake.clear_written();
                backend.unlock_all(&[LockSlot::Axis(axis, sign)]).unwrap();
                assert_eq!(
                    rig.fake.written(),
                    [format!("km.lock_{name}(0)\r\n").into_bytes()]
                );
            }
        }
    }
}

#[test]
fn close_releases_tracked_buttons_keys_and_locks_then_upload_off_without_ack() {
    let rig = Rig::new();
    let device = rig.device();
    let _events = device.events();
    device.press(Button::Side1).unwrap();
    device.press_key(Key::A).unwrap();
    device.lock_button(Button::Right).unwrap();
    device.lock_axis(Axis::Y, Direction::Both).unwrap();
    rig.fake.clear_written();
    device.close().unwrap();
    let written = rig.fake.written();
    assert!(written.contains(&b"km.ms1(0)\r\n".to_vec()));
    assert!(written.contains(&wire(r#"{"cmd":45,"kc":4,"down":0}"#)));
    assert!(written.contains(&b"km.lock_mr(0)\r\n".to_vec()));
    assert_eq!(
        written
            .iter()
            .filter(|bytes| bytes.as_slice() == b"km.lock_my(0)\r\n")
            .count(),
        2
    );
    assert_eq!(written.last(), Some(&wire(r#"{"cmd":34,"status":0}"#)));
    assert_eq!(rig.system.open_ports(), 0);
    let count = written.len();
    assert!(matches!(device.close(), Err(Error::Closed)));
    drop(device);
    assert_eq!(rig.fake.written().len(), count);
}

#[test]
fn real_port_failure_rebuilds_firmware_keys_history_locks_and_streams_on_the_original_path() {
    let rig = Rig::new();
    let device = rig.device();
    let events = device.events();
    device.press(Button::Left).unwrap();
    device.press_key(Key::A).unwrap();
    device.lock_axis(Axis::X, Direction::Both).unwrap();
    rig.fake
        .send(&wire(r#"{"code":404,"message":"no_keyboard"}"#));
    assert!(matches!(
        past_changed(&events),
        Event::Rejected {
            action: Action::KeyPress,
            ..
        }
    ));
    wait_for(|| {
        !device
            .snapshot()
            .capabilities
            .get(Action::KeyTap)
            .is_supported()
    });
    rig.system.unplug("COM7");
    assert_eq!(events.recv_timeout(WAIT).unwrap(), Event::Disconnected);
    let replacement = FakeBox::new(|bytes| {
        respond(
            &mut BoxState {
                version: "ysc-towmcu-L v2.0".into(),
                ..BoxState::default()
            },
            bytes,
        )
    });
    rig.system
        .plug("COM7", usb(Some("TOWMCULEFT")), replacement.clone());
    assert_eq!(events.recv_timeout(WAIT).unwrap(), Event::Reconnected);
    assert_eq!(
        device.snapshot().info.firmware,
        Some(Firmware::Text("ysc-towmcu-L v2.0".into()))
    );
    assert!(
        device
            .snapshot()
            .capabilities
            .get(Action::KeyTap)
            .is_supported()
    );
    let written = replacement.written();
    assert!(written.contains(&b"km.left(0)\r\n".to_vec()));
    assert!(written.contains(&wire(r#"{"cmd":45,"kc":4,"down":0}"#)));
    assert!(written.contains(&b"km.lock_mx(1)\r\n".to_vec()));
    assert!(written.contains(&wire(r#"{"cmd":34,"status":1}"#)));
    replacement.send(&wire(r#"{"code":404,"message":"no_keyboard"}"#));
    assert!(matches!(
        past_changed(&events),
        Event::Rejected {
            action: Action::KeyRelease,
            ..
        }
    ));
}

#[test]
fn a_role_serial_on_another_path_or_a_wrong_tuple_on_the_original_path_is_never_opened() {
    let rig = Rig::new();
    let (backend, _, rx) = rig.attach().unwrap();
    rig.system.unplug("COM7");
    signal(&rx, |signal| matches!(signal, Signal::Disconnected));
    let elsewhere = FakeBox::new(|bytes| respond(&mut BoxState::default(), bytes));
    rig.system
        .plug("COM8", usb(Some("TOWMCULEFT")), elsewhere.clone());
    for identity in [
        usb(Some("TOWMCURIGHT")),
        usb(Some("TOWMCUIAP")),
        None,
        Some(crate::transport::PortUsb {
            vid: 0x1234,
            pid: 0xfe0c,
            serial: Some("TOWMCULEFT".into()),
        }),
    ] {
        let wrong = FakeBox::new(|_| panic!("an excluded port got written to"));
        rig.system.plug("COM7", identity, wrong.clone());
        assert!(rx.recv_timeout(Duration::from_millis(120)).is_err());
        assert!(wrong.written().is_empty());
    }
    assert!(elsewhere.written().is_empty());
    assert_eq!(rig.system.opened_at(), [BAUD]);
    backend.shutdown(WRITE_DEADLINE);
}

#[test]
fn shutdown_interrupts_a_silent_reprobe_with_a_bounded_stop() {
    let rig = Rig::new();
    let (backend, _, rx) = rig.attach().unwrap();
    rig.system.unplug("COM7");
    signal(&rx, |signal| matches!(signal, Signal::Disconnected));
    let silent = FakeBox::new(|_| Vec::new());
    rig.system
        .plug("COM7", usb(Some("TOWMCULEFT")), silent.clone());
    wait_for(|| !silent.written().is_empty());
    let before = Instant::now();
    backend.shutdown(WRITE_DEADLINE);
    assert!(before.elapsed() < Duration::from_millis(650));
    wait_for(|| rig.system.open_ports() == 0);
    assert_eq!(silent.written(), [wire(r#"{"cmd":132}"#)]);
}

thread_local! { static SYSTEM: RefCell<Option<FakeSystem>> = const { RefCell::new(None) }; }

fn open_fake(candidate: &Candidate, sink: Sink) -> Result<(Box<dyn Backend>, Snapshot)> {
    let system = SYSTEM.with_borrow(|system| system.as_ref().unwrap().clone());
    let (backend, snapshot) = attach(Arc::new(system), candidate, sink)?;
    Ok((Box::new(backend), snapshot))
}

#[test]
fn discovery_uses_the_production_connect_path_and_only_lists_left_ports() {
    let rig = Rig::new();
    SYSTEM.with_borrow_mut(|system| *system = Some(rig.system.clone()));
    let mut excluded = Vec::new();
    for (path, role) in [
        ("COM1", Some("TOWMCURIGHT")),
        ("COM2", Some("TOWMCUIAP")),
        ("COM3", None),
        ("COM4", Some("UNKNOWN")),
    ] {
        let fake = FakeBox::new(|_| panic!("an excluded port got written to"));
        rig.system.plug(path, usb(role), fake.clone());
        excluded.push(fake);
    }
    let probes: &[(Family, crate::family::Open)] = &[(Family::Ysc, open_fake)];
    for connect in [
        Connect::auto(),
        Connect::auto().family(Family::Ysc),
        Connect::port("COM7").family(Family::Ysc),
    ] {
        let device = connect
            .can(Action::Click)
            .can(Action::KeyTap)
            .open_in(&|| rig.system.ports(), probes)
            .unwrap();
        assert_eq!(device.snapshot().info.family, Family::Ysc);
        assert_eq!(device.snapshot().info.id, None);
        device.close().unwrap();
    }
    let found = Connect::auto().found_in(&|| rig.system.ports(), probes);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].port, "COM7");
    assert_eq!(
        found[0].capabilities.get(Action::Click).execution,
        Execution::Host
    );
    assert!(excluded.iter().all(|fake| fake.written().is_empty()));
    assert_eq!(rig.system.opened_at().len(), 4);
    assert!(
        Connect::auto()
            .id(StableId::UsbSerial("TOWMCULEFT".into()))
            .found_in(&|| rig.system.ports(), probes)
            .is_empty()
    );
}

#[test]
fn named_right_iap_missing_serial_and_unlisted_ports_never_open() {
    let rig = Rig::new();
    SYSTEM.with_borrow_mut(|system| *system = Some(rig.system.clone()));
    let probes: &[(Family, crate::family::Open)] = &[(Family::Ysc, open_fake)];
    for (path, role) in [
        ("COM1", Some("TOWMCURIGHT")),
        ("COM2", Some("TOWMCUIAP")),
        ("COM3", None),
        ("COM4", Some("UNKNOWN")),
    ] {
        let fake = FakeBox::new(|_| panic!("an excluded port got written to"));
        rig.system.plug(path, usb(role), fake.clone());
        let error = Connect::port(path)
            .family(Family::Ysc)
            .open_in(&|| rig.system.ports(), probes)
            .err()
            .unwrap();
        if let Some(role @ ("TOWMCURIGHT" | "TOWMCUIAP")) = role {
            assert!(error.to_string().contains(role));
            assert!(error.to_string().contains("use the TOWMCULEFT port"));
        }
        assert!(fake.written().is_empty());
    }
    assert!(
        Connect::port("COM99")
            .family(Family::Ysc)
            .open_in(&|| rig.system.ports(), probes)
            .is_err()
    );
    assert!(rig.system.opened_at().is_empty());
}

#[test]
fn multiple_left_boxes_are_listed_in_port_order_and_cannot_be_selected_by_role_id() {
    let rig = Rig::new();
    let second = FakeBox::new(|bytes| respond(&mut BoxState::default(), bytes));
    rig.system.plug("COM10", usb(Some("TOWMCULEFT")), second);
    SYSTEM.with_borrow_mut(|system| *system = Some(rig.system.clone()));
    let probes: &[(Family, crate::family::Open)] = &[(Family::Ysc, open_fake)];
    let found = Connect::auto().found_in(&|| rig.system.ports(), probes);
    assert_eq!(
        found
            .iter()
            .map(|found| found.port.as_str())
            .collect::<Vec<_>>(),
        ["COM7", "COM10"]
    );
    assert!(found.iter().all(|found| found.info.id.is_none()));
}

#[test]
fn other_enabled_families_never_open_a_known_ysc_tuple() {
    let rig = Rig::new();
    for role in ["TOWMCULEFT", "TOWMCURIGHT", "TOWMCUIAP"] {
        let candidate = Candidate {
            path: "COM7".into(),
            usb: usb(Some(role)),
        };
        let (tx, _) = mpsc::channel::<Signal>();
        #[cfg(feature = "ferrum")]
        assert!(
            crate::family::ferrum::attach(
                Arc::new(rig.system.clone()),
                &candidate,
                Sink(tx.clone())
            )
            .is_err()
        );
        #[cfg(feature = "makcu")]
        assert!(
            crate::family::makcu::attach(
                Arc::new(rig.system.clone()),
                &candidate,
                Sink(tx.clone())
            )
            .is_err()
        );
        #[cfg(feature = "medius")]
        assert!(matches!(
            crate::family::medius::open(&candidate, Sink(tx.clone())),
            Err(Error::NotFound { .. })
        ));
        let _ = (&candidate, &tx);
    }
    assert!(rig.system.opened_at().is_empty());
}

#[test]
fn a_non_ysc_identity_is_not_classified_as_a_box() {
    let rig = Rig::with(BoxState {
        version: "km.MAKCU".into(),
        ..BoxState::default()
    });
    assert!(matches!(rig.attach(), Err(Error::NotFound { .. })));
    assert_eq!(rig.fake.written(), [wire(r#"{"cmd":132}"#)]);
}

#[test]
fn debug_probe_transport_failure_fails_the_open_instead_of_carrying_on() {
    let system = FakeSystem::default();
    let fake = FakeBox::new(|bytes| {
        if bytes == b"<START>\x00\x19{\"cmd\":132}<END>" {
            vec![wire(r#"{"code":200,"message":"ysc-towmcu-L v1.0"}"#)]
        } else {
            Vec::new()
        }
    });
    system.plug("COM7", usb(Some("TOWMCULEFT")), fake.clone());
    let (tx, _) = mpsc::channel();
    let candidate = system.ports().pop().unwrap();
    let task = thread::spawn({
        let system = system.clone();
        move || {
            attach(Arc::new(system), &candidate, Sink(tx))
                .err()
                .unwrap()
        }
    });
    wait_for(|| fake.written().len() == 2);
    fake.vanish();
    assert!(matches!(task.join().unwrap(), Error::Disconnected));
    assert_eq!(fake.written().len(), 2);
}

#[test]
fn busy_and_permission_errors_survive_discovery() {
    let rig = Rig::new();
    SYSTEM.with_borrow_mut(|system| *system = Some(rig.system.clone()));
    let probes: &[(Family, crate::family::Open)] = &[(Family::Ysc, open_fake)];
    #[cfg(windows)]
    rig.system.refuse(Some(5));
    #[cfg(unix)]
    rig.system.refuse(Some(libc::EBUSY));
    assert!(matches!(
        Connect::auto().open_in(&|| rig.system.ports(), probes),
        Err(Error::PortBusy { .. })
    ));
    assert!(matches!(
        &Connect::auto().scan_in(&|| rig.system.ports(), probes)[0].result,
        Err(Error::PortBusy { .. })
    ));
    #[cfg(unix)]
    {
        rig.system.refuse(Some(libc::EACCES));
        assert!(matches!(
            Connect::auto().open_in(&|| rig.system.ports(), probes),
            Err(Error::PermissionDenied { .. })
        ));
    }
    assert!(rig.fake.written().is_empty());
}

#[test]
fn explicit_close_reports_a_failed_upload_off_only_when_anybox_turned_uploads_on() {
    let rig = Rig::new();
    let device = rig.device();
    let events = device.events_of(EventKinds::BUTTONS);
    wait_for(|| {
        rig.fake
            .written()
            .contains(&wire(r#"{"cmd":34,"status":1}"#))
    });
    rig.fake.vanish();
    assert!(matches!(device.close(), Err(Error::Disconnected)));
    drop(events);
    wait_for(|| rig.system.open_ports() == 0);

    // nothing was streaming, so there's nothing to stop and nothing to report
    let rig = Rig::new();
    let device = rig.device();
    rig.fake.vanish();
    device.close().unwrap();
    wait_for(|| rig.system.open_ports() == 0);
}

#[test]
fn shutdown_during_the_optional_debug_wait_sends_no_session_setup() {
    let rig = Rig::new();
    let (backend, _, rx) = rig.attach().unwrap();
    rig.system.unplug("COM7");
    signal(&rx, |s| matches!(s, Signal::Disconnected));
    let fake = FakeBox::new(|bytes| {
        if bytes == b"<START>\x00\x19{\"cmd\":132}<END>" {
            vec![wire(r#"{"code":200,"message":"ysc-towmcu-L v1.0"}"#)]
        } else {
            Vec::new()
        }
    });
    rig.system
        .plug("COM7", usb(Some("TOWMCULEFT")), fake.clone());
    wait_for(|| fake.written().len() == 2);
    let started = Instant::now();
    backend.shutdown(WRITE_DEADLINE);
    assert!(started.elapsed() < Duration::from_millis(650));
    wait_for(|| rig.system.open_ports() == 0);
    assert_eq!(
        fake.written(),
        [wire(r#"{"cmd":132}"#), wire(r#"{"cmd":702}"#)]
    );
}

#[test]
fn stream_restart_is_write_only_and_reseeds_buttons() {
    let rig = Rig::new();
    let (backend, _, rx) = rig.attach().unwrap();
    backend.set_streams(InputKind::Buttons.kinds()).unwrap();
    rig.fake.send(&upload(1, 1, 2, 0));
    signal(&rx, |s| {
        matches!(
            s,
            Signal::Input {
                input: crate::backend::Input::Motion { .. },
                ..
            }
        )
    });
    rig.fake.clear_written();
    backend.restart_stream(InputKind::Buttons).unwrap();
    assert_eq!(rig.fake.written(), [wire(r#"{"cmd":34,"status":1}"#)]);
    rig.fake.send(&upload(0, 1, 2, 0));
    assert!(matches!(
        rx.recv_timeout(WAIT).unwrap(),
        Signal::Input {
            input: crate::backend::Input::Motion { .. },
            ..
        }
    ));
}

#[test]
fn a_failed_release_stays_tracked_for_reconnect_and_is_retried() {
    let rig = Rig::new();
    let device = rig.device();
    let events = device.events_of(EventKinds::NONE);
    device.press(Button::Side2).unwrap();
    rig.state.lock().unwrap().wrong_echo = true;
    assert!(matches!(
        device.release(Button::Side2),
        Err(Error::Disconnected)
    ));
    assert_eq!(events.recv_timeout(WAIT).unwrap(), Event::Disconnected);
    rig.state.lock().unwrap().wrong_echo = false;
    assert_eq!(events.recv_timeout(WAIT).unwrap(), Event::Reconnected);
    assert_eq!(
        rig.fake
            .written()
            .iter()
            .filter(|bytes| bytes.as_slice() == b"km.ms2(0)\r\n")
            .count(),
        2
    );
}

#[test]
fn an_early_close_ends_host_holds_and_releases_them() {
    let rig = Rig::new();
    let device = Arc::new(rig.device());
    rig.fake.clear_written();
    let call = thread::spawn({
        let device = device.clone();
        move || device.tap_key_with(Key::A, Click::hold(Duration::from_secs(1)))
    });
    wait_for(|| {
        rig.fake
            .written()
            .contains(&wire(r#"{"cmd":45,"kc":4,"down":1}"#))
    });
    device.close().unwrap();
    assert!(matches!(
        call.join().unwrap(),
        Err(Error::Closed) | Err(Error::Interrupted { .. })
    ));
    assert_eq!(
        rig.fake
            .written()
            .iter()
            .filter(|bytes| *bytes == &wire(r#"{"cmd":45,"kc":4,"down":0}"#))
            .count(),
        1
    );
}

#[test]
fn a_host_hold_interrupted_by_port_loss_never_releases_a_new_press() {
    let rig = Rig::new();
    let device = Arc::new(rig.device());
    let events = device.events_of(EventKinds::NONE);
    let call = thread::spawn({
        let device = device.clone();
        move || device.click_with(Button::Right, Click::hold(Duration::from_millis(300)))
    });
    wait_for(|| rig.fake.written().contains(&b"km.right(1)\r\n".to_vec()));
    device.move_by(1, 0).unwrap();
    rig.system.unplug("COM7");
    assert_eq!(events.recv_timeout(WAIT).unwrap(), Event::Disconnected);
    let fake = FakeBox::new(|bytes| respond(&mut BoxState::default(), bytes));
    rig.system
        .plug("COM7", usb(Some("TOWMCULEFT")), fake.clone());
    assert_eq!(events.recv_timeout(WAIT).unwrap(), Event::Reconnected);
    fake.clear_written();
    device.press(Button::Right).unwrap();
    assert!(matches!(
        call.join().unwrap(),
        Err(Error::Interrupted {
            action: Action::Click
        })
    ));
    assert_eq!(fake.written(), [b"km.right(1)\r\n".to_vec()]);
}

#[test]
fn a_keyboard_return_and_success_replies_do_not_restore_keys_without_reconnect() {
    let rig = Rig::new();
    let (backend, _, rx) = rig.attach().unwrap();
    rig.fake
        .send(&wire(r#"{"code":404,"message":"no_keyboard"}"#));
    signal(&rx, |s| matches!(s, Signal::Changed(_)));
    rig.fake.send(&wire(
        r#"{"code":702,"data":{"debug":false,"connected":true}}"#,
    ));
    rig.fake.send(&wire(r#"{"code":200,"message":"ok"}"#));
    backend.send(Command::Move(1, 2)).unwrap();
    assert!(matches!(
        backend.send(Command::KeyPress(Key::B)),
        Err(Error::Unsupported { .. })
    ));
    assert!(rx.recv_timeout(Duration::from_millis(40)).is_err());
}

#[test]
fn a_taps_press_and_release_cannot_be_guessed_from_a_delayed_404() {
    let rig = Rig::new();
    let device = rig.device();
    let events = device.events_of(EventKinds::NONE);
    device.tap_key(Key::A).unwrap();
    rig.fake
        .send(&wire(r#"{"code":404,"message":"no_keyboard"}"#));
    wait_for(|| {
        !device
            .snapshot()
            .capabilities
            .get(Action::KeyTap)
            .is_supported()
    });
    // the keys turning off is a change of the snapshot, nothing else
    assert_eq!(events.recv_timeout(WAIT), Ok(Event::Changed));
    assert!(events.recv_timeout(Duration::from_millis(40)).is_err());
}

#[test]
fn delayed_status_is_attributed_when_it_arrives_before_a_later_cleanup_write() {
    let rig = Rig::new();
    let (backend, _, rx) = rig.attach().unwrap();
    backend.send(Command::KeyPress(Key::A)).unwrap();
    rig.fake
        .send(&wire(r#"{"code":404,"message":"no_keyboard"}"#));
    signal(&rx, |s| matches!(s, Signal::Changed(_)));
    backend.release_all(&[Usage::Key(Key::A)]).unwrap();
    assert!(matches!(
        rx.recv_timeout(WAIT).unwrap(),
        Signal::Refused(Action::KeyPress)
    ));
}

#[test]
fn a_404_blocks_untracked_key_releases_but_allows_the_sessions_cleanup() {
    let rig = Rig::new();
    let (backend, _, rx) = rig.attach().unwrap();
    backend.send(Command::KeyPress(Key::A)).unwrap();
    rig.fake
        .send(&wire(r#"{"code":404,"message":"no_keyboard"}"#));
    signal(&rx, |s| matches!(s, Signal::Changed(_)));
    rig.fake.clear_written();
    assert!(matches!(
        backend.send(Command::KeyRelease(Key::B)),
        Err(Error::Unsupported { .. })
    ));
    assert!(rig.fake.written().is_empty());
    backend.release_all(&[Usage::Key(Key::A)]).unwrap();
    assert_eq!(rig.fake.written(), [wire(r#"{"cmd":45,"kc":4,"down":0}"#)]);
    // once it's let go it's not the session's anymore, and a key it still holds can't be pressed again either
    rig.fake.clear_written();
    assert!(matches!(
        backend.send(Command::KeyRelease(Key::A)),
        Err(Error::Unsupported { .. })
    ));
    backend.send(Command::KeyPress(Key::C)).unwrap_err();
    assert!(rig.fake.written().is_empty());
}

#[test]
fn a_key_the_session_still_holds_cant_be_pressed_again_after_a_404() {
    let rig = Rig::new();
    let (backend, _, rx) = rig.attach().unwrap();
    backend.send(Command::KeyPress(Key::A)).unwrap();
    rig.fake
        .send(&wire(r#"{"code":404,"message":"no_keyboard"}"#));
    signal(&rx, |s| matches!(s, Signal::Changed(_)));
    rig.fake.clear_written();
    assert!(matches!(
        backend.send(Command::KeyPress(Key::A)),
        Err(Error::Unsupported { .. })
    ));
    assert!(rig.fake.written().is_empty());
}

#[test]
fn a_late_prompt_cannot_complete_the_same_command_from_a_new_generation() {
    let rig = Rig::with(BoxState {
        partial_text: true,
        ..BoxState::default()
    });
    let (backend, _, rx) = rig.attach().unwrap();
    let backend = Arc::new(backend);
    assert!(matches!(
        backend.send(Command::Move(1, 2)),
        Err(Error::Disconnected)
    ));
    signal(&rx, |s| matches!(s, Signal::Disconnected));
    rig.state.lock().unwrap().partial_text = false;
    signal(&rx, |s| matches!(s, Signal::Reconnected(_)));
    rig.state.lock().unwrap().echo = false;
    let call = thread::spawn({
        let backend = backend.clone();
        move || backend.send(Command::Move(1, 2))
    });
    wait_for(|| {
        rig.fake
            .written()
            .iter()
            .filter(|bytes| bytes.as_slice() == b"km.move(1,2)\r\n")
            .count()
            == 2
    });
    rig.fake.send(b">>> ");
    assert!(matches!(call.join().unwrap(), Err(Error::Disconnected)));
}

#[test]
fn repeated_same_path_reopens_run_one_debug_check_per_attempt_and_reseed_events() {
    let rig = Rig::new();
    let (backend, _, rx) = rig.attach().unwrap();
    backend.set_streams(InputKind::Buttons.kinds()).unwrap();
    rig.fake.send(&upload(1, 0, 0, 0));
    signal(&rx, |s| matches!(s, Signal::Input { .. }));
    for _ in 0..3 {
        rig.fake
            .send(&wire(r#"{"code":404,"message":"no_keyboard"}"#));
        signal(&rx, |s| matches!(s, Signal::Changed(_)));
        rig.state.lock().unwrap().wrong_echo = true;
        assert!(matches!(
            backend.send(Command::Move(1, 2)),
            Err(Error::Disconnected)
        ));
        signal(&rx, |s| matches!(s, Signal::Disconnected));
        rig.state.lock().unwrap().wrong_echo = false;
        signal(&rx, |s| matches!(s, Signal::Reconnected(_)));
        backend.set_streams(InputKind::Buttons.kinds()).unwrap();
        rig.fake.send(&upload(0, 0, 0, 0));
        assert!(matches!(
            rx.recv_timeout(WAIT).unwrap(),
            Signal::Input {
                input: crate::backend::Input::Motion { .. },
                ..
            }
        ));
        backend.send(Command::Move(3, 4)).unwrap();
    }
    assert_eq!(rig.system.opened_at(), [115200; 4]);
    assert_eq!(
        rig.fake
            .written()
            .iter()
            .filter(|bytes| *bytes == &wire(r#"{"cmd":702}"#))
            .count(),
        4
    );
}

#[test]
fn duplicate_refusals_in_one_read_publish_one_change_and_one_refusal() {
    let rig = Rig::new();
    let (backend, _, rx) = rig.attach().unwrap();
    backend.send(Command::KeyPress(Key::A)).unwrap();
    let refusal = wire(r#"{"code":404,"message":"no_keyboard"}"#);
    rig.fake.send(&[refusal.clone(), refusal].concat());
    let mut changes = 0;
    let mut refusals = 0;
    while changes != 1 || refusals != 1 {
        match rx.recv_timeout(WAIT).unwrap() {
            Signal::Changed(_) => changes += 1,
            Signal::Refused(Action::KeyPress) => refusals += 1,
            other => panic!("unexpected status {other:?}"),
        }
    }
    assert!(rx.recv_timeout(Duration::from_millis(40)).is_err());
}

#[test]
fn an_optional_partial_debug_frame_is_discarded_without_another_identity_request() {
    let system = FakeSystem::default();
    let fake = FakeBox::new(|bytes| {
        if bytes == b"<START>\x00\x19{\"cmd\":702}<END>" {
            vec![b"<START>\x00\x80{\"code\":702".to_vec()]
        } else {
            respond(&mut BoxState::default(), bytes)
        }
    });
    system.plug("COM7", usb(Some("TOWMCULEFT")), fake.clone());
    let (tx, _) = mpsc::channel();
    let candidate = system.ports().pop().unwrap();
    let (backend, _) = attach(Arc::new(system), &candidate, Sink(tx)).unwrap();
    backend.send(Command::Move(1, 2)).unwrap();
    assert_eq!(
        fake.written()
            .iter()
            .filter(|bytes| *bytes == &wire(r#"{"cmd":132}"#))
            .count(),
        1
    );
    assert_eq!(
        fake.written()
            .iter()
            .filter(|bytes| *bytes == &wire(r#"{"cmd":702}"#))
            .count(),
        1
    );
}

#[test]
fn a_port_that_isnt_a_v2_one_only_gets_the_v1_sweep_and_a_v2_answer_there_isnt_a_v1() {
    let rig = Rig::new();
    let (tx, _) = mpsc::channel();
    let candidate = Candidate {
        path: "COM7".into(),
        usb: ch343(),
    };
    let started = Instant::now();
    assert!(matches!(
        attach(Arc::new(rig.system.clone()), &candidate, Sink(tx)),
        Err(Error::NotFound { .. })
    ));
    // 9 rates at the vendor's 200 ms each
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(rig.system.opened_at(), V1_BAUDS);
    assert!(
        rig.fake
            .written()
            .iter()
            .all(|bytes| *bytes == wire(r#"{"cmd":132}"#))
    );
}

#[test]
fn a_404_during_reconnect_cleanup_does_not_block_other_keys_held_by_the_session() {
    let rig = Rig::new();
    let (backend, _, rx) = rig.attach().unwrap();
    let backend = Arc::new(backend);
    backend.send(Command::KeyPress(Key::A)).unwrap();
    backend.send(Command::KeyPress(Key::B)).unwrap();
    rig.system.unplug("COM7");
    signal(&rx, |s| matches!(s, Signal::Disconnected));
    let pause = Arc::new(std::sync::Barrier::new(2));
    let holder = Arc::new(Mutex::new(None::<std::sync::Weak<FakeBox>>));
    let fake = FakeBox::new({
        let holder = holder.clone();
        let pause = pause.clone();
        let mut refused = false;
        move |bytes| {
            if bytes.starts_with(b"<START>")
                && bytes.windows(8).any(|bytes| bytes == b"\"down\":0")
                && !refused
            {
                refused = true;
                holder
                    .lock()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .upgrade()
                    .unwrap()
                    .send(&wire(r#"{"code":404,"message":"no_keyboard"}"#));
                pause.wait();
                Vec::new()
            } else {
                respond(&mut BoxState::default(), bytes)
            }
        }
    });
    *holder.lock().unwrap() = Some(Arc::downgrade(&fake));
    rig.system
        .plug("COM7", usb(Some("TOWMCULEFT")), fake.clone());
    signal(&rx, |s| matches!(s, Signal::Reconnected(_)));
    let cleanup = thread::spawn({
        let backend = backend.clone();
        move || backend.release_all(&[Usage::Key(Key::A), Usage::Key(Key::B)])
    });
    signal(&rx, |s| matches!(s, Signal::Changed(_)));
    pause.wait();
    cleanup.join().unwrap().unwrap();
    let written = fake.written();
    assert!(written.contains(&wire(r#"{"cmd":45,"kc":4,"down":0}"#)));
    assert!(written.contains(&wire(r#"{"cmd":45,"kc":5,"down":0}"#)));
}

/// keys turning off after a 404 also changes the snapshot, which comes as its own event before the refusal
fn past_changed(events: &crate::event::Events) -> Event {
    loop {
        match events.recv_timeout(WAIT).unwrap() {
            Event::Changed => continue,
            event => return event,
        }
    }
}

const V1_REPLY: &str = r#"{"code":200,"message":"version","data":"Jul 23 2026"}"#;

fn ch343() -> Option<crate::transport::PortUsb> {
    Some(crate::transport::PortUsb {
        vid: 0x1a86,
        pid: 0x55d3,
        serial: Some("5B69201745".into()),
    })
}

/// a V1 that only understands `rate` and never answers cmd 702
fn v1_at(rate: u32, reply: &'static str) -> Arc<FakeBox> {
    let mut state = BoxState {
        debug: None,
        ..BoxState::default()
    };
    FakeBox::by_baud(move |baud, bytes| {
        if baud != rate {
            Vec::new()
        } else if bytes == wire(r#"{"cmd":132}"#) {
            vec![wire(reply)]
        } else {
            respond(&mut state, bytes)
        }
    })
}

fn attach_at(system: &FakeSystem, path: &str) -> Result<(YscBackend, Snapshot, Receiver<Signal>)> {
    let candidate = system
        .ports()
        .into_iter()
        .find(|c| c.path == Path::new(path))
        .unwrap();
    let (tx, rx) = mpsc::channel();
    let (backend, snapshot) = attach(Arc::new(system.clone()), &candidate, Sink(tx))?;
    Ok((backend, snapshot, rx))
}

#[test]
fn a_v1_is_found_at_its_rate_in_the_vendor_order_and_keeps_being_a_v1_after_a_reconnect_at_another()
{
    let system = FakeSystem::default();
    system.plug("COM3", ch343(), v1_at(2_000_000, V1_REPLY));
    let (backend, snapshot, rx) = attach_at(&system, "COM3").unwrap();
    assert_eq!(
        system.opened_at(),
        [4_000_000, 115_200, 3_000_000, 230_400, 2_000_000]
    );
    assert_eq!(snapshot.info.model.as_deref(), Some("YSC 8K V1"));
    assert_eq!(
        snapshot.info.firmware,
        Some(Firmware::Text("Jul 23 2026".into()))
    );

    system.unplug("COM3");
    signal(&rx, |signal| matches!(signal, Signal::Disconnected));
    system.plug("COM3", ch343(), v1_at(4_000_000, V1_REPLY));
    let Signal::Reconnected(again) = signal(&rx, |signal| matches!(signal, Signal::Reconnected(_)))
    else {
        unreachable!()
    };
    assert_eq!(again.info.model.as_deref(), Some("YSC 8K V1"));
    // the rate it was at first, then the rest in the vendor order, so 4M is next
    assert_eq!(system.opened_at()[5..7], [2_000_000, 4_000_000]);
    backend.shutdown(WRITE_DEADLINE);
}

#[test]
fn a_v1_in_its_bootloader_is_refused_and_only_ever_asked_its_version() {
    let system = FakeSystem::default();
    let fake = v1_at(
        1_500_000,
        r#"{"code":200,"message":"version","data":"YSC-IAP"}"#,
    );
    system.plug("COM3", ch343(), fake.clone());
    let Err(Error::Unsupported { reason, .. }) = attach_at(&system, "COM3") else {
        panic!("the bootloader opened");
    };
    assert!(reason.contains("bootloader"), "{reason}");
    assert_eq!(system.opened_at().last(), Some(&1_500_000));
    assert!(
        fake.written()
            .iter()
            .all(|bytes| *bytes == wire(r#"{"cmd":132}"#))
    );
}

#[test]
fn a_rate_the_bridge_cant_take_is_skipped_and_the_next_one_still_finds_the_v1() {
    let system = FakeSystem::default();
    system.plug("COM3", ch343(), v1_at(115_200, V1_REPLY));
    system.refuse_baud(4_000_000);
    let (backend, snapshot, _) = attach_at(&system, "COM3").unwrap();
    assert_eq!(snapshot.info.model.as_deref(), Some("YSC 8K V1"));
    assert_eq!(system.opened_at(), [115_200]);
    backend.shutdown(WRITE_DEADLINE);
}

#[test]
fn a_port_that_opens_at_no_rate_gives_the_driver_error_and_one_rate_short_is_still_not_found() {
    let system = FakeSystem::default();
    system.plug("COM3", ch343(), FakeBox::new(|_| Vec::new()));
    system.refuse(Some(87));
    assert!(matches!(attach_at(&system, "COM3"), Err(Error::Io(_))));
    system.refuse(None);
    system.refuse_baud(4_000_000);
    assert!(matches!(
        attach_at(&system, "COM3"),
        Err(Error::NotFound { .. })
    ));
}
