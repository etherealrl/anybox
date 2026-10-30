use std::collections::HashSet;
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::frames::{self, MakcuFrames, frame};
use super::mock::{SERIAL, Shared, State, makcu_usb, respond};
use super::{MakcuBackend, Poller, attach};
use crate::backend::{
    Backend, Command, Input, InputKind, InputKinds, LockSlot, Sign, Signal, Sink, Usage,
};
use crate::capability::{
    Action, Axis, Button, Execution, Family, Firmware, MakcuAction, Snapshot, StableId,
};
use crate::click::{Click, ClickTiming, Holds};
use crate::device::{Device, Start};
use crate::error::Error;
use crate::event::{DropSource, Event};
use crate::extension::{Extension, Interpolation};
use crate::key::Key;
use crate::transport::codec::{Codec, Incoming};
use crate::transport::fake::{FakeBox, FakeSystem};
use crate::transport::{Candidate, PortUsb, System};

const PORT: &str = "COM11";

struct Rig {
    system: FakeSystem,
    fake: Arc<FakeBox>,
    state: Shared,
}

fn rig(state: State) -> Rig {
    rig_with(state, makcu_usb())
}

fn rig_with(state: State, usb: Option<PortUsb>) -> Rig {
    let state = Arc::new(Mutex::new(state));
    let fake = FakeBox::by_baud({
        let state = state.clone();
        move |baud, bytes| respond(&state, baud, bytes)
    });
    let system = FakeSystem::default();
    system.plug(PORT, usb, fake.clone());
    Rig {
        system,
        fake,
        state,
    }
}

impl Rig {
    fn candidate(&self) -> Candidate {
        self.system
            .ports()
            .into_iter()
            .find(|candidate| candidate.path == Path::new(PORT))
            .unwrap_or(Candidate {
                path: PORT.into(),
                usb: None,
            })
    }

    fn attach(&self) -> crate::error::Result<(MakcuBackend, Snapshot, Poller, Receiver<Signal>)> {
        let (signals, incoming) = mpsc::channel();
        let (backend, snapshot, poller) = attach(
            Arc::new(self.system.clone()),
            &self.candidate(),
            Sink(signals),
        )?;
        Ok((backend, snapshot, poller, incoming))
    }

    fn device(&self) -> (Device, Poller) {
        let (signals, incoming) = mpsc::channel();
        let (backend, snapshot, poller) = attach(
            Arc::new(self.system.clone()),
            &self.candidate(),
            Sink(signals.clone()),
        )
        .unwrap();
        let device = Device::start(Start {
            backend: Box::new(backend),
            snapshot,
            signals,
            incoming,
            click_timing: ClickTiming::Makcu,
            holds: Holds::from_clock(),
            port: PORT.into(),
        })
        .unwrap();
        (device, poller)
    }

    fn written(&self) -> Vec<Vec<u8>> {
        self.fake.written()
    }

    fn clear(&self) {
        self.fake.clear_written();
    }

    fn set(&self, change: impl FnOnce(&mut State)) {
        change(&mut self.state.lock().unwrap());
    }
}

fn until(mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !done() {
        assert!(Instant::now() < deadline, "gave up waiting");
        thread::sleep(Duration::from_millis(5));
    }
}

fn link_events(signals: &Receiver<Signal>) -> Vec<&'static str> {
    signals
        .try_iter()
        .filter_map(|signal| match signal {
            Signal::SessionReset => Some("session reset"),
            Signal::Disconnected => Some("disconnected"),
            Signal::Reconnected(_) => Some("reconnected"),
            _ => None,
        })
        .collect()
}

fn split_all(codec: &mut MakcuFrames, bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut buffer = bytes.to_vec();
    let mut out = Vec::new();
    while let Some(frame) = codec.split(&mut buffer) {
        out.push(frame);
    }
    out
}

// the wire format, checked against the vendor's examples and what a V4.094 box actually sent

#[test]
fn a_firmware_reply_carries_the_build() {
    // captured from a V4.094 box, build 4094
    let reply = [0xDE, 0xAD, 0x04, 0x00, 0x04, 0xFE, 0x0F, 0x00, 0x00];
    assert_eq!(
        frame(frames::FIRMWARE_VERSION, &[]),
        [0xDE, 0xAD, 0, 0, 0x04]
    );
    assert_eq!(
        split_all(&mut MakcuFrames::binary(Default::default()), &reply),
        vec![reply.to_vec()]
    );
    let (_, payload) = frames::parts(&reply).unwrap();
    assert_eq!(u32::from_le_bytes(payload.try_into().unwrap()), 4094);
}

#[test]
fn a_mouse_input_change_is_a_button_event_and_other_kinds_are_noise() {
    let mut codec = MakcuFrames::binary(Default::default());
    let right = vec![0xDE, 0xAD, 0x03, 0x00, 0x53, 0x01, 0x01, 0x01];
    assert!(matches!(
        codec.classify(&right),
        Incoming::Input {
            input: Input::Button {
                button: Button::Right,
                pressed: true
            },
            box_time: None
        }
    ));
    let extra = vec![0xDE, 0xAD, 0x03, 0x00, 0x53, 0x01, 31, 0x00];
    assert!(matches!(
        codec.classify(&extra),
        Incoming::Input {
            input: Input::Button {
                button: Button::Extra(31),
                pressed: false
            },
            ..
        }
    ));
    // the vendor's controller trigger example, a keyboard usage that isn't a key, the V4.087 wheel subtype, and
    // an id past 31
    for noise in [
        vec![0xDE, 0xAD, 0x04, 0x00, 0x53, 0x03, 0x0A, 0xFF, 0x03],
        vec![0xDE, 0xAD, 0x03, 0x00, 0x53, 0x02, 0x00, 0x01],
        vec![0xDE, 0xAD, 0x04, 0x00, 0x53, 0x01, 0x20, 0x01, 0x00],
        vec![0xDE, 0xAD, 0x03, 0x00, 0x53, 0x01, 32, 0x01],
    ] {
        assert!(
            matches!(codec.classify(&noise), Incoming::Noise),
            "{noise:02X?}"
        );
    }
}

#[test]
fn the_overflow_frame_is_a_lost_stream() {
    let overflow = vec![0xDE, 0xAD, 0x03, 0x00, 0x53, 0x01, 0xFF, 0xFF];
    assert!(matches!(
        MakcuFrames::binary(Default::default()).classify(&overflow),
        Incoming::StreamLost(crate::backend::InputKind::Buttons)
    ));
}

#[test]
fn a_refusal_names_what_was_refused_and_a_zero_reply_is_just_a_reply() {
    // the refusal doesn't say if it was a press or a release, so it goes by the last thing sent, which was a
    // press for left and a release for side2
    let last = Arc::new(frames::LastSent::default());
    last.set(0, 1);
    let mut codec = MakcuFrames::binary(last);
    let cases = [
        (0x18, Action::Move),
        (0x67, Action::MoveNow),
        (0x19, Action::Wheel),
        (0x61, Action::Click),
        (0x11, Action::Press),
        (0x15, Action::Release),
        (0x52, Action::ButtonEvents),
    ];
    for (command, expected) in cases {
        match codec.classify(&frame(command, &[0xFF])) {
            Incoming::Refused { key, action } => {
                assert_eq!(key, u32::from(command));
                assert_eq!(action, Some(expected));
            }
            other => panic!("{command:02X} gave {other:?}"),
        }
    }
    assert!(matches!(
        codec.classify(&frame(0x52, &[0x00])),
        Incoming::Reply(0x52)
    ));
    // when the library's own query gets refused late, only the request waiting on it needs to know
    assert!(matches!(
        codec.classify(&frame(frames::INJECT_SNAPSHOT, &[0xFF])),
        Incoming::Refused { action: None, .. }
    ));
}

#[test]
fn text_ends_at_the_prompt_and_a_binary_frame_after_it_splits_on_its_own() {
    let mut bytes = b"ERR\r\n>>> km.MAKCU\r\n>>> ".to_vec();
    bytes.extend(frame(frames::DEVICE, &[0x03]));
    let frames = split_all(&mut MakcuFrames::probe(), &bytes);
    assert_eq!(
        frames,
        vec![
            b"ERR\r\n>>> ".to_vec(),
            b"km.MAKCU\r\n>>> ".to_vec(),
            frame(frames::DEVICE, &[0x03])
        ]
    );
    assert!(!frames::says_makcu(&frames[0]));
    assert!(frames::says_makcu(&frames[1]));
    assert!(!frames::says_makcu(b"km.version(km.MAKCU)\r\n>>> "));
}

#[test]
fn a_frame_split_across_reads_waits_for_the_rest_and_an_empty_payload_is_whole() {
    let mut codec = MakcuFrames::binary(Default::default());
    let mut buffer = vec![0xDE, 0xAD, 0x04, 0x00, 0x04, 0xFE];
    assert_eq!(codec.split(&mut buffer), None);
    assert_eq!(buffer.len(), 6);
    let mut buffer = vec![0xDE, 0xAD, 0x00];
    assert_eq!(codec.split(&mut buffer), None);
    let mut buffer = vec![0xDE, 0xAD, 0x00, 0x00, 0x52];
    assert_eq!(
        codec.split(&mut buffer),
        Some(vec![0xDE, 0xAD, 0x00, 0x00, 0x52])
    );
}

#[test]
fn junk_and_text_never_hold_up_the_events_behind_them() {
    let event = vec![0xDE, 0xAD, 0x03, 0x00, 0x53, 0x01, 0x00, 0x01];
    let mut bytes = b"\x00\xFFjunk km.MAKCU\r\n".to_vec();
    bytes.extend(&event);
    assert_eq!(
        split_all(&mut MakcuFrames::binary(Default::default()), &bytes),
        vec![event]
    );
}

#[test]
fn a_corrupt_length_is_skipped_and_the_next_frame_still_parses() {
    // LEN 0x41 is past the cap, and after it come a stray DE and then the real frame
    let device = frame(frames::DEVICE, &[0x03]);
    let mut bytes = vec![0xDE, 0xAD, 0x41, 0x00, 0xDE];
    bytes.extend(&device);
    assert_eq!(
        split_all(&mut MakcuFrames::binary(Default::default()), &bytes),
        vec![device]
    );
}

// the probe and the open, on the fake system

#[test]
fn a_v4_box_opens_with_its_identity() {
    let rig = rig(State::makcu(4108));
    let (_backend, snapshot, _poller, _signals) = rig.attach().unwrap();
    assert_eq!(snapshot.info.family, Family::MakcuV4);
    assert_eq!(snapshot.info.model.as_deref(), Some("MAKCU"));
    assert_eq!(snapshot.info.firmware, Some(Firmware::Build(4108)));
    assert_eq!(snapshot.info.id, Some(StableId::UsbSerial(SERIAL.into())));
    assert_eq!(
        snapshot.capabilities.buttons(),
        [
            Button::Left,
            Button::Right,
            Button::Middle,
            Button::Side1,
            Button::Side2
        ]
    );
    for action in [
        Action::Move,
        Action::MoveNow,
        Action::Press,
        Action::Click,
        Action::ButtonEvents,
    ] {
        assert_eq!(
            snapshot.capabilities.get(action).execution,
            Execution::Native,
            "{action}"
        );
    }
    for action in [Action::DiscardPendingMoves, Action::MotionEvents] {
        assert!(
            !snapshot.capabilities.get(action).is_supported(),
            "{action}"
        );
    }
}

#[test]
fn the_probe_only_asks_and_switches_the_stream_off() {
    let rig = rig(State::makcu(4094));
    let _open = rig.attach().unwrap();
    assert_eq!(
        rig.written(),
        vec![
            frames::VERSION_QUERY.to_vec(),
            frame(frames::FIRMWARE_VERSION, &[]),
            frame(frames::DEVICE, &[]),
            frame(frames::INPUT_STREAM, &[1, 0]),
            frame(frames::INPUT_STREAM, &[2, 0]),
        ]
    );
}

#[test]
fn a_box_switched_to_115200_is_found_there() {
    let mut state = State::makcu(4094);
    state.baud = 115_200;
    let rig = rig(state);
    let (_backend, snapshot, _poller, _signals) = rig.attach().unwrap();
    assert_eq!(snapshot.info.firmware, Some(Firmware::Build(4094)));
    assert_eq!(rig.system.opened_at(), vec![4_000_000, 115_200]);
}

#[test]
fn nothing_answering_at_either_rate_is_not_found() {
    let mut state = State::makcu(4094);
    state.text = None;
    let rig = rig(state);
    assert!(matches!(rig.attach(), Err(Error::NotFound { .. })));
    let asked = rig.written();
    assert_eq!(asked.len(), 4, "two tries per rate");
    assert!(asked.iter().all(|bytes| bytes == frames::VERSION_QUERY));
}

#[test]
fn one_silent_identity_query_is_tried_again_at_the_same_rate() {
    let mut state = State::makcu(4094);
    state.silent_versions = 1;
    state.silent_versions_left = 1;
    let rig = rig(state);
    assert!(rig.attach().is_ok());
    assert_eq!(rig.system.opened_at(), vec![4_000_000]);
}

#[test]
fn junk_left_by_an_earlier_probe_is_skipped() {
    let mut state = State::makcu(4094);
    state.text = Some(b"ERR\r\n>>> km.MAKCU\r\n>>> ");
    assert!(rig(state).attach().is_ok());
}

#[test]
fn a_box_that_only_answers_km_text_is_told_to_update() {
    let mut state = State::makcu(4094);
    state.build = None;
    match rig(state).attach() {
        Err(Error::FirmwareTooOld {
            found: Firmware::Text(text),
            needed: Firmware::Build(4085),
            ..
        }) => assert_eq!(text, "V3 or older"),
        other => panic!("{:?}", other.err()),
    }
}

#[test]
fn the_firmware_gate_sits_at_4085_and_below_5000() {
    assert!(matches!(
        rig(State::makcu(4084)).attach().err(),
        Some(Error::FirmwareTooOld {
            found: Firmware::Build(4084),
            needed: Firmware::Build(4085),
            ..
        })
    ));
    // move_now from the first official build that sends it in the next report, autorelease from V4.108
    let (_backend, snapshot, _poller, _signals) = rig(State::makcu(4096)).attach().unwrap();
    assert!(!snapshot.capabilities.get(Action::MoveNow).is_supported());
    let (_backend, snapshot, _poller, _signals) = rig(State::makcu(4097)).attach().unwrap();
    assert!(snapshot.capabilities.get(Action::MoveNow).is_supported());
    let autorelease = Action::Makcu(MakcuAction::Autorelease);
    assert!(!snapshot.capabilities.get(autorelease).is_supported());
    let (_backend, snapshot, _poller, _signals) = rig(State::makcu(4108)).attach().unwrap();
    assert!(snapshot.capabilities.get(autorelease).is_supported());
    assert!(matches!(
        rig(State::makcu(5000)).attach().err(),
        Some(Error::LibraryTooOld {
            found: Firmware::Build(5000),
            ..
        })
    ));
}

#[test]
fn a_port_that_says_its_another_box_is_never_asked() {
    let ysc = Some(PortUsb {
        vid: 0x1A86,
        pid: 0xFE0C,
        serial: Some("TOWMCULEFT".into()),
    });
    let rig = rig_with(State::makcu(4094), ysc);
    assert!(matches!(rig.attach(), Err(Error::NotFound { .. })));
    assert!(rig.system.opened_at().is_empty());
}

#[test]
fn a_ysc_style_answer_is_not_a_makcu() {
    let mut state = State::makcu(4094);
    state.text = Some(b"km.version(km.MAKCU)\r\n>>> ");
    let rig = rig(state);
    assert!(matches!(rig.attach(), Err(Error::NotFound { .. })));
    assert!(
        !rig.written()
            .contains(&frame(frames::FIRMWARE_VERSION, &[]))
    );
}

#[test]
fn a_port_with_no_usb_id_is_still_probed() {
    let rig = rig_with(State::makcu(4094), None);
    let (_backend, snapshot, _poller, _signals) = rig.attach().unwrap();
    assert_eq!(snapshot.info.id, None);
}

#[test]
fn without_a_mouse_routed_mouse_output_says_so() {
    let mut state = State::makcu(4094);
    state.kinds = 0x02;
    let rig = rig(state);
    let (device, _poller) = rig.device();
    rig.clear();
    match device.move_by(1, 0) {
        Err(Error::Unsupported { reason, .. }) => assert!(reason.contains("mouse"), "{reason}"),
        other => panic!("{other:?}"),
    }
    assert!(device.snapshot().capabilities.buttons().is_empty());
    assert!(
        !device
            .snapshot()
            .capabilities
            .get(Action::ButtonEvents)
            .is_supported()
    );
    assert!(rig.written().is_empty());
}

// commands, through the backend on a fake port

#[test]
fn each_command_is_its_own_frame() {
    let rig = rig(State::makcu(4108));
    let (device, _poller) = rig.device();
    rig.clear();
    device.move_by(10, -20).unwrap();
    device.move_now(5, -5).unwrap();
    device.press(Button::Side1).unwrap();
    device.release(Button::Side1).unwrap();
    device.wheel(-1).unwrap();
    assert_eq!(
        rig.written(),
        vec![
            vec![0xDE, 0xAD, 0x04, 0x00, 0x18, 0x0A, 0x00, 0xEC, 0xFF],
            vec![0xDE, 0xAD, 0x04, 0x00, 0x67, 0x05, 0x00, 0xFB, 0xFF],
            vec![0xDE, 0xAD, 0x01, 0x00, 0x14, 0x01],
            vec![0xDE, 0xAD, 0x01, 0x00, 0x14, 0x00],
            vec![0xDE, 0xAD, 0x02, 0x00, 0x19, 0xFF, 0xFF],
        ]
    );
}

#[test]
fn a_click_lets_the_box_time_it_or_sends_the_hold_in_whole_ms_rounded_up() {
    let rig = rig(State::makcu(4094));
    let (device, _poller) = rig.device();
    rig.clear();
    device.click(Button::Left).unwrap();
    device
        .click_with(Button::Right, Click::hold(Duration::from_millis(120)))
        .unwrap();
    device
        .click_with(Button::Side2, Click::hold(Duration::from_micros(400)))
        .unwrap();
    device
        .click_with(Button::Middle, Click::hold(Duration::ZERO))
        .unwrap();
    assert_eq!(
        rig.written(),
        vec![
            vec![0xDE, 0xAD, 0x04, 0x00, 0x61, 0x01, 0x01, 0x00, 0x00],
            vec![0xDE, 0xAD, 0x04, 0x00, 0x61, 0x02, 0x01, 0x78, 0x00],
            vec![0xDE, 0xAD, 0x04, 0x00, 0x61, 0x05, 0x01, 0x01, 0x00],
            vec![0xDE, 0xAD, 0x04, 0x00, 0x61, 0x03, 0x01, 0x01, 0x00],
        ]
    );
}

#[test]
fn discard_is_unsupported_and_sends_nothing() {
    let rig = rig(State::makcu(4094));
    let (device, _poller) = rig.device();
    rig.clear();
    assert!(matches!(
        device.discard_pending_moves(),
        Err(Error::Unsupported { .. })
    ));
    assert!(rig.written().is_empty());
}

#[test]
fn release_all_releases_each_button() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, _poller, _signals) = rig.attach().unwrap();
    rig.clear();
    backend
        .release_all(&[
            crate::backend::Usage::Button(Button::Left),
            crate::backend::Usage::Button(Button::Side2),
        ])
        .unwrap();
    assert_eq!(rig.written(), vec![frame(0x11, &[0]), frame(0x15, &[0])]);
}

#[test]
fn the_backend_refuses_what_the_box_can_no_longer_do_but_always_releases() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, _poller, _signals) = rig.attach().unwrap();
    *backend.shared.snapshot.write().unwrap() = super::snapshot(4094, 0x02, None);
    rig.clear();
    assert!(matches!(
        backend.send(Command::Move(1, 1)),
        Err(Error::Unsupported { .. })
    ));
    backend.send(Command::Release(Button::Left)).unwrap();
    assert_eq!(rig.written(), vec![frame(0x11, &[0])]);
}

// streams and events

#[test]
fn streams_go_on_with_the_first_receiver_confirmed_and_off_with_the_last() {
    let rig = rig(State::makcu(4094));
    let (device, _poller) = rig.device();
    rig.clear();
    let events = device.events();
    // the box routes a keyboard, so its stream goes on and off together with the mouse one, and each switch
    // gets confirmed
    assert_eq!(
        rig.written(),
        vec![
            frame(0x52, &[1, 1]),
            frame(0x52, &[1]),
            frame(0x52, &[2, 1]),
            frame(0x52, &[2])
        ]
    );
    rig.clear();
    drop(events);
    assert_eq!(
        rig.written(),
        vec![
            frame(0x52, &[1, 0]),
            frame(0x52, &[1]),
            frame(0x52, &[2, 0]),
            frame(0x52, &[2])
        ]
    );
}

#[test]
fn a_stream_the_box_refuses_is_rejected() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, _poller, signals) = rig.attach().unwrap();
    rig.set(|state| {
        state.refuse.insert(0x52);
    });
    assert!(matches!(
        backend
            .set_streams(InputKinds {
                buttons: true,
                ..InputKinds::default()
            })
            .map_err(|failure| failure.error),
        Err(Error::Rejected { .. })
    ));
    // nothing waits on a refused SET, so one of the two refusals also goes out as an event
    until(|| {
        signals
            .try_iter()
            .any(|signal| matches!(signal, Signal::Refused(Action::ButtonEvents)))
    });
}

#[test]
fn a_refusal_a_get_was_waiting_for_is_its_answer_and_not_an_event() {
    let rig = rig(State::makcu(4094));
    let (_backend, _snapshot, mut poller, signals) = rig.attach().unwrap();
    rig.set(|state| {
        state.refuse.insert(frames::INJECT_SNAPSHOT);
    });
    poller.tick();
    assert!(
        signals
            .try_iter()
            .all(|signal| !matches!(signal, Signal::Refused(_)))
    );
}

#[test]
fn a_physical_click_comes_out_as_press_and_release() {
    let rig = rig(State::makcu(4094));
    let (device, _poller) = rig.device();
    let events = device.events();
    rig.fake
        .send(&[0xDE, 0xAD, 0x03, 0x00, 0x53, 0x01, 0x02, 0x01]);
    rig.fake
        .send(&[0xDE, 0xAD, 0x03, 0x00, 0x53, 0x01, 0x02, 0x00]);
    let first = events.recv_timeout(Duration::from_secs(2)).unwrap();
    let second = events.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(
        matches!(first, Event::Button { button: Button::Middle, pressed: true, at } if at.box_time.is_none())
    );
    assert!(matches!(
        second,
        Event::Button {
            button: Button::Middle,
            pressed: false,
            ..
        }
    ));
}

#[test]
fn an_overflow_reports_one_drop_and_turns_the_stream_back_on() {
    let rig = rig(State::makcu(4094));
    let (device, _poller) = rig.device();
    let events = device.events();
    rig.clear();
    rig.fake
        .send(&[0xDE, 0xAD, 0x03, 0x00, 0x53, 0x01, 0xFF, 0xFF]);
    assert_eq!(
        events.recv_timeout(Duration::from_secs(2)).unwrap(),
        Event::Dropped {
            count: 1,
            source: DropSource::Device
        }
    );
    until(|| rig.written().contains(&frame(0x52, &[1, 1])));
}

#[test]
fn a_refused_move_reaches_the_app_as_an_event() {
    let rig = rig(State::makcu(4094));
    let (device, _poller) = rig.device();
    let events = device.events();
    rig.set(|state| {
        state.refuse.insert(0x18);
    });
    device.move_by(1, 0).unwrap();
    let event = (0..3)
        .filter_map(|_| events.recv_timeout(Duration::from_secs(2)).ok())
        .find(|event| matches!(event, Event::Rejected { .. }));
    assert_eq!(
        event,
        Some(Event::Rejected {
            action: Action::Move,
            count: 1
        })
    );
}

// the poller

#[test]
fn a_box_that_lost_a_held_button_for_two_ticks_is_a_session_reset() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, signals) = rig.attach().unwrap();
    backend.send(Command::Press(Button::Left)).unwrap();
    poller.tick();
    assert!(link_events(&signals).is_empty());
    // the box rebooted by itself but the COM port stayed
    rig.set(|state| state.injected = 0);
    poller.tick();
    assert!(
        link_events(&signals).is_empty(),
        "one tick could be a press still queued"
    );
    poller.tick();
    assert_eq!(link_events(&signals), vec!["session reset"]);
}

#[test]
fn a_matching_tick_in_between_sets_the_count_back() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, signals) = rig.attach().unwrap();
    backend.send(Command::Press(Button::Left)).unwrap();
    // the box showed the press first, so misses count from here
    poller.tick();
    rig.set(|state| state.injected = 0);
    poller.tick();
    rig.set(|state| state.injected = 1);
    poller.tick();
    rig.set(|state| state.injected = 0);
    poller.tick();
    assert!(link_events(&signals).is_empty());
}

#[test]
fn a_wanted_stream_that_reads_off_for_two_ticks_is_a_session_reset() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, signals) = rig.attach().unwrap();
    backend
        .set_streams(InputKinds {
            buttons: true,
            motion: false,
            ..InputKinds::default()
        })
        .unwrap();
    rig.set(|state| state.stream = 0);
    poller.tick();
    poller.tick();
    assert_eq!(link_events(&signals), vec!["session reset"]);
}

#[test]
fn nothing_held_and_no_stream_is_never_a_reset() {
    let rig = rig(State::makcu(4094));
    let (_backend, _snapshot, mut poller, signals) = rig.attach().unwrap();
    for _ in 0..3 {
        poller.tick();
    }
    assert!(link_events(&signals).is_empty());
}

#[test]
fn a_mouse_plugged_in_later_shows_up_on_the_fourth_tick() {
    let mut state = State::makcu(4094);
    state.kinds = 0x02;
    let rig = rig(state);
    let (_backend, snapshot, mut poller, signals) = rig.attach().unwrap();
    assert!(snapshot.capabilities.buttons().is_empty());
    rig.set(|state| state.kinds = 0x03);
    for _ in 0..3 {
        poller.tick();
    }
    assert!(link_events(&signals).is_empty());
    poller.tick();
    let fresh = signals.try_iter().find_map(|signal| match signal {
        Signal::Reconnected(snapshot) => Some(snapshot),
        _ => None,
    });
    assert_eq!(
        fresh.map(|snapshot| snapshot.capabilities.buttons().len()),
        Some(5)
    );
}

// reconnect and shutdown

#[test]
fn a_box_switched_to_the_other_rate_comes_back_there() {
    let rig = rig(State::makcu(4094));
    let (_backend, _snapshot, _poller, signals) = rig.attach().unwrap();
    rig.set(|state| state.baud = 115_200);
    rig.fake.vanish();
    let unplug = rig.system.clone();
    unplug.unplug(PORT);
    let fake = FakeBox::by_baud({
        let state = rig.state.clone();
        move |baud, bytes| respond(&state, baud, bytes)
    });
    rig.system.plug(PORT, makcu_usb(), fake);
    until(|| {
        let seen: Vec<_> = link_events(&signals);
        seen.contains(&"reconnected")
    });
    assert!(rig.system.opened_at().ends_with(&[115_200]));
}

#[test]
fn another_box_on_the_same_port_is_not_taken_back() {
    let rig = rig(State::makcu(4094));
    let (_backend, _snapshot, _poller, signals) = rig.attach().unwrap();
    rig.system.unplug(PORT);
    let other = Some(PortUsb {
        vid: 0x1A86,
        pid: 0x55D3,
        serial: Some("5C00000000".into()),
    });
    let fake = FakeBox::by_baud({
        let state = rig.state.clone();
        move |baud, bytes| respond(&state, baud, bytes)
    });
    rig.system.plug(PORT, other, fake);
    thread::sleep(Duration::from_millis(400));
    assert_eq!(link_events(&signals), vec!["disconnected"]);
}

#[test]
fn shutdown_switches_the_stream_off() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, _poller, _signals) = rig.attach().unwrap();
    backend
        .set_streams(InputKinds {
            buttons: true,
            motion: false,
            ..InputKinds::default()
        })
        .unwrap();
    rig.clear();
    backend.shutdown(Duration::from_millis(500));
    assert!(rig.written().contains(&frame(0x52, &[1, 0])));
}

// what must never count as a reset, and the edge cases around it

#[test]
fn a_press_the_box_refused_never_makes_a_reset_that_lets_go_of_a_real_hold() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, signals) = rig.attach().unwrap();
    backend.send(Command::Press(Button::Right)).unwrap();
    poller.tick();
    rig.set(|state| {
        state.refuse.insert(0x11);
    });
    backend.send(Command::Press(Button::Left)).unwrap();
    for _ in 0..4 {
        poller.tick();
    }
    assert!(link_events(&signals).is_empty());
    assert_eq!(rig.state.lock().unwrap().injected, 0b10);
}

#[test]
fn a_press_still_queued_in_the_box_never_counts_as_lost() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, signals) = rig.attach().unwrap();
    rig.set(|state| state.apply_presses = false);
    backend.send(Command::Press(Button::Left)).unwrap();
    for _ in 0..4 {
        poller.tick();
    }
    assert!(link_events(&signals).is_empty());
}

#[test]
fn a_release_on_its_way_is_not_a_lost_hold() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, signals) = rig.attach().unwrap();
    backend.send(Command::Press(Button::Left)).unwrap();
    poller.tick();
    backend.send(Command::Release(Button::Left)).unwrap();
    rig.set(|state| state.injected = 0);
    poller.tick();
    poller.tick();
    assert!(link_events(&signals).is_empty());
}

#[test]
fn a_stream_the_box_refused_once_is_asked_again_and_never_makes_a_reset() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, signals) = rig.attach().unwrap();
    rig.set(|state| {
        state.refuse_once.insert(0x52);
    });
    let on = InputKinds {
        buttons: true,
        motion: false,
        ..InputKinds::default()
    };
    assert!(backend.set_streams(on).is_err());
    poller.tick();
    poller.tick();
    poller.tick();
    assert_eq!(rig.state.lock().unwrap().stream, 1);
    assert!(link_events(&signals).is_empty());
    // it's confirmed now, so losing it for two ticks is a reset
    rig.set(|state| state.stream = 0);
    rig.set(|state| {
        state.refuse_once.clear();
    });
    poller.tick();
    poller.tick();
    assert_eq!(link_events(&signals), vec!["session reset"]);
}

#[test]
fn after_a_mouse_appears_moves_work() {
    let mut state = State::makcu(4094);
    state.kinds = 0x02;
    let rig = rig(state);
    let (backend, _snapshot, mut poller, _signals) = rig.attach().unwrap();
    assert!(backend.send(Command::Move(1, 0)).is_err());
    rig.set(|state| state.kinds = 0x03);
    for _ in 0..4 {
        poller.tick();
    }
    backend.send(Command::Move(1, 0)).unwrap();
}

#[test]
fn a_frame_split_right_after_its_first_byte_still_arrives() {
    let mut codec = MakcuFrames::binary(Default::default());
    let device = frame(frames::DEVICE, &[0x03]);
    let mut buffer = vec![0x00, device[0]];
    assert_eq!(codec.split(&mut buffer), None);
    buffer.extend(&device[1..]);
    assert_eq!(codec.split(&mut buffer), Some(device));
}

#[test]
fn while_probing_text_ends_where_a_binary_frame_starts() {
    let device = frame(frames::DEVICE, &[0x03]);
    let mut bytes = b"km.MAK".to_vec();
    bytes.extend(&device);
    let frames = split_all(&mut MakcuFrames::probe(), &bytes);
    assert_eq!(frames, vec![b"km.MAK".to_vec(), device]);
}

#[test]
fn a_button_state_past_one_is_noise() {
    let odd = vec![0xDE, 0xAD, 0x03, 0x00, 0x53, 0x01, 0x00, 0x02];
    assert!(matches!(
        MakcuFrames::binary(Default::default()).classify(&odd),
        Incoming::Noise
    ));
}

#[test]
fn a_busy_port_says_busy() {
    let rig = rig(State::makcu(4094));
    // what each OS says when another program holds the port
    rig.system.refuse(Some(if cfg!(windows) { 5 } else { 16 }));
    assert!(matches!(rig.attach(), Err(Error::PortBusy { .. })));
}

#[test]
fn one_silent_firmware_query_is_asked_again() {
    let mut state = State::makcu(4094);
    state.silent_builds_left = 1;
    assert!(rig(state).attach().is_ok());
}

#[test]
fn a_malformed_firmware_reply_is_too_old_never_not_found() {
    let mut state = State::makcu(4094);
    state.build_reply = Some(vec![0xFE, 0x0F]);
    assert!(matches!(
        rig(state).attach().err(),
        Some(Error::FirmwareTooOld { .. })
    ));
}

#[test]
fn a_refused_re_enable_after_an_overflow_is_asked_again_not_a_reset() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, signals) = rig.attach().unwrap();
    let on = InputKinds {
        buttons: true,
        motion: false,
        ..InputKinds::default()
    };
    backend.set_streams(on).unwrap();
    poller.tick();
    // the box turned the stream off by itself and refuses the first try to turn it back on
    rig.set(|state| {
        state.stream = 0;
        state.refuse_once.insert(0x52);
    });
    assert!(backend.restart_stream(InputKind::Buttons).is_err());
    poller.tick();
    poller.tick();
    assert!(link_events(&signals).is_empty());
    assert_eq!(rig.state.lock().unwrap().stream, 1);
}

#[test]
fn a_stream_nobody_asked_for_is_never_switched_on() {
    let rig = rig(State::makcu(4094));
    let (_backend, _snapshot, mut poller, _signals) = rig.attach().unwrap();
    rig.clear();
    for _ in 0..4 {
        poller.tick();
    }
    assert!(!rig.written().contains(&frame(0x52, &[1, 1])));
}

#[test]
fn a_late_refusal_of_the_backends_own_query_is_no_event() {
    let rig = rig(State::makcu(4094));
    let (device, _poller) = rig.device();
    let events = device.events();
    rig.fake.send(&frame(frames::INJECT_SNAPSHOT, &[0xFF]));
    rig.fake
        .send(&[0xDE, 0xAD, 0x03, 0x00, 0x53, 0x01, 0x00, 0x01]);
    // the button event that comes after it proves the refusal was read first
    assert!(matches!(
        events.recv_timeout(Duration::from_secs(2)),
        Ok(Event::Button { .. })
    ));
    assert!(events.try_recv().is_err());
}

#[test]
fn three_notches_go_out_as_one_wheel_command() {
    // on V4.094 `wheel(3)` scrolled three notches, so we don't split it or cap it
    let rig = rig(State::makcu(4094));
    let (device, _poller) = rig.device();
    rig.clear();
    device.wheel(3).unwrap();
    assert_eq!(rig.written(), vec![frame(frames::WHEEL, &[0x03, 0x00])]);
}

#[test]
fn a_release_the_box_refused_goes_again_and_a_refused_release_says_release() {
    let rig = rig(State::makcu(4094));
    let (device, mut poller) = rig.device();
    let events = device.events();
    device.press(Button::Left).unwrap();
    rig.set(|state| {
        state.refuse_once.insert(0x11);
    });
    device.release(Button::Left).unwrap();
    let event = (0..3)
        .filter_map(|_| events.recv_timeout(Duration::from_secs(2)).ok())
        .find(|event| matches!(event, Event::Rejected { .. }));
    assert_eq!(
        event,
        Some(Event::Rejected {
            action: Action::Release,
            count: 1
        })
    );
    assert_eq!(rig.state.lock().unwrap().injected, 1);
    poller.tick();
    assert_eq!(rig.state.lock().unwrap().injected, 0);
}

#[test]
fn a_held_press_is_never_released_by_the_poller() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, _signals) = rig.attach().unwrap();
    backend.send(Command::Press(Button::Left)).unwrap();
    poller.tick();
    poller.tick();
    assert_eq!(rig.state.lock().unwrap().injected, 1);
}

#[test]
fn a_refused_press_through_the_backend_says_press() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, _poller, signals) = rig.attach().unwrap();
    rig.set(|state| {
        state.refuse.insert(0x12);
    });
    backend.send(Command::Press(Button::Right)).unwrap();
    until(|| {
        signals
            .try_iter()
            .any(|signal| matches!(signal, Signal::Refused(Action::Press)))
    });
}

#[test]
fn nothing_held_means_the_poller_writes_no_releases() {
    let rig = rig(State::makcu(4094));
    let (_backend, _snapshot, mut poller, _signals) = rig.attach().unwrap();
    rig.clear();
    poller.tick();
    poller.tick();
    assert!(
        rig.written()
            .iter()
            .all(|bytes| !matches!(frames::parts(bytes), Some((0x11..=0x15, _))))
    );
}

#[test]
fn a_running_box_click_is_never_cut_short_by_the_poller() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, _signals) = rig.attach().unwrap();
    backend
        .send(Command::Click {
            button: Button::Left,
            hold: Some(Duration::from_secs(5)),
        })
        .unwrap();
    rig.clear();
    poller.tick();
    poller.tick();
    assert!(!rig.written().contains(&frame(0x11, &[0])));
}

/// these need a MAKCU V4 box. run them with
/// `ANYBOX_PORT=COM11 cargo test makcu::tests::hardware -- --ignored --nocapture --test-threads 1`, with your
/// box's port in place of COM11. the ones marked "with you" need you to do something with the mouse or a
/// cable while they run
mod hardware {
    use std::time::{Duration, Instant};

    use crate::capability::{Action, Button, Family, Firmware, StableId};
    use crate::click::Click;
    use crate::connect::Connect;
    use crate::error::Error;
    use crate::event::{Event, RecvTimeoutError};

    fn port() -> String {
        std::env::var("ANYBOX_PORT").expect("set ANYBOX_PORT to the port your box is on")
    }

    fn open() -> crate::device::Device {
        Connect::port(port())
            .family(Family::MakcuV4)
            .open()
            .unwrap_or_else(|error| panic!("{error}"))
    }

    #[cfg(windows)]
    #[allow(unsafe_code)]
    mod beeps {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn Beep(frequency: u32, millis: u32) -> i32;
        }

        /// a high beep says start, two low ones say done. blocks while it sounds
        pub(super) fn beep(high: bool) {
            // SAFETY: Beep takes two plain numbers
            unsafe {
                Beep(if high { 1500 } else { 500 }, 400);
            }
        }

        pub(super) fn done() {
            beep(false);
            std::thread::sleep(std::time::Duration::from_millis(150));
            beep(false);
        }
    }

    /// needs you at the PC. after the high beep, click any mouse button about twenty times within 30 seconds, until
    /// the two low beeps. it checks each event's `received` against a QPC read right after the event came out of
    /// the receiver, so the read time is real and in order
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn received_time_next_to_a_qpc_read_right_after() {
        use crate::event::EventKinds;
        use crate::transport::os::Read;

        let device = open();
        let events = device.events_of(EventKinds::BUTTONS);
        beeps::beep(true);
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut gaps = Vec::new();
        let mut last_qpc = 0i64;
        while gaps.len() < 20 {
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            match events.recv_timeout(left) {
                Ok(Event::Button { at, .. }) => {
                    let after = Read::now();
                    let qpc = at.received_qpc.expect("a QPC on Windows");
                    assert!(qpc >= last_qpc, "received went backwards");
                    last_qpc = qpc;
                    let ticks = after.qpc.unwrap() - qpc;
                    let elapsed = after.at.duration_since(at.received);
                    assert!(ticks >= 0, "the QPC read after came before received");
                    gaps.push((ticks, elapsed));
                }
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout) => break,
                Err(error) => panic!("{error:?}"),
            }
        }
        beeps::done();
        for (ticks, elapsed) in &gaps {
            println!("qpc ticks after received {ticks:>7}, instant {elapsed:?}");
        }
        assert!(gaps.len() >= 5, "only {} clicks came in", gaps.len());
        assert!(
            gaps.iter()
                .all(|(_, elapsed)| *elapsed < Duration::from_millis(50)),
            "an event took more than 50 ms from the read to the app"
        );
    }

    #[test]
    #[ignore]
    fn a_real_box_opens_with_its_identity() {
        let device = open();
        let snapshot = device.snapshot();
        println!("{:?}", snapshot.info);
        println!("buttons {:?}", snapshot.capabilities.buttons());
        assert_eq!(snapshot.info.family, Family::MakcuV4);
        assert!(matches!(snapshot.info.firmware, Some(Firmware::Build(_))));
        assert!(matches!(snapshot.info.id, Some(StableId::UsbSerial(_))));
    }

    #[test]
    #[ignore]
    fn a_second_open_is_busy_and_reopens_right_after_a_drop_work() {
        let first = open();
        let second = Connect::port(port()).family(Family::MakcuV4).open();
        assert!(
            matches!(second, Err(Error::PortBusy { .. })),
            "{:?}",
            second.err()
        );
        drop(first);
        for round in 0..10 {
            let device = open();
            device
                .move_by(0, 0)
                .unwrap_or_else(|error| panic!("round {round}: {error}"));
        }
    }

    /// holds F13 and right ctrl for a moment and checks that INJECT_SNAPSHOT shows them at the documented
    /// offsets. F13 types nothing in most apps and ctrl alone does nothing
    #[test]
    #[ignore]
    fn the_snapshot_shows_held_keys_where_the_docs_say() {
        use super::super::frames::{self, frame};
        use crate::transport::System;
        let port = crate::transport::os::OsSystem
            .open(std::path::Path::new(&port()), 4_000_000)
            .unwrap();
        port.write_all(&frame(frames::KEY_DOWN, &[0x68])).unwrap();
        port.write_all(&frame(frames::KEY_DOWN, &[0xE4])).unwrap();
        std::thread::sleep(Duration::from_millis(50));
        port.write_all(&frame(frames::INJECT_SNAPSHOT, &[]))
            .unwrap();
        std::thread::sleep(Duration::from_millis(100));
        let mut chunk = [0u8; 256];
        let got = port.read(&mut chunk).unwrap_or(0);
        port.write_all(&frame(frames::KEY_UP, &[0x68])).unwrap();
        port.write_all(&frame(frames::KEY_UP, &[0xE4])).unwrap();
        println!("{:02X?}", &chunk[..got]);
        let reply = &chunk[..got];
        let at = reply
            .windows(5)
            .position(|window| window == [0xDE, 0xAD, 40, 0, frames::INJECT_SNAPSHOT])
            .expect("the box didn't send back a 40 byte snapshot");
        let payload = &reply[at + 5..at + 45];
        assert_eq!(frames::injected_keys(payload), vec![0xE4, 0x68]);
    }

    /// holds a key right after the box's own tap and reads it back from the snapshot. F13 and F14 type
    /// nothing in most apps
    #[test]
    #[ignore]
    fn a_key_pressed_right_after_a_box_tap() {
        use super::super::frames::{self, frame};
        use crate::transport::System;
        let port = crate::transport::os::OsSystem
            .open(std::path::Path::new(&port()), 4_000_000)
            .unwrap();
        let read = |what: &str| {
            std::thread::sleep(Duration::from_millis(30));
            let mut chunk = [0u8; 512];
            let got = port.read(&mut chunk).unwrap_or(0);
            let reply = &chunk[..got];
            let held = reply
                .windows(5)
                .position(|window| window == [0xDE, 0xAD, 40, 0, frames::INJECT_SNAPSHOT])
                .map(|at| frames::injected_keys(&reply[at + 5..at + 45]));
            println!("{what}: held {held:02X?}, all bytes {reply:02X?}");
        };
        let snapshot = || {
            port.write_all(&frame(frames::INJECT_SNAPSHOT, &[]))
                .unwrap();
        };
        for gap in [0u64, 10, 50, 100, 300] {
            port.write_all(&frame(frames::KEY_PRESS, &[0x69])).unwrap();
            std::thread::sleep(Duration::from_millis(gap));
            port.write_all(&frame(frames::KEY_DOWN, &[0x68])).unwrap();
            std::thread::sleep(Duration::from_millis(50));
            snapshot();
            read(&format!("tap then down after {gap} ms"));
            port.write_all(&frame(frames::KEY_UP, &[0x68])).unwrap();
            std::thread::sleep(Duration::from_millis(100));
            let mut drain = [0u8; 512];
            let _ = port.read(&mut drain);
        }
        port.write_all(&frame(frames::KEY_DOWN, &[0x68])).unwrap();
        std::thread::sleep(Duration::from_millis(50));
        snapshot();
        read("down with no tap before");
        port.write_all(&frame(frames::KEY_UP, &[0x68])).unwrap();
    }

    /// shows how LOCK target 5 (X both ways) relates to 6 (+X) and 7 (-X), read from the snapshot's lock bits
    #[test]
    #[ignore]
    fn how_the_x_lock_targets_relate() {
        use super::super::frames::{self, frame};
        use crate::transport::System;
        let port = crate::transport::os::OsSystem
            .open(std::path::Path::new(&port()), 4_000_000)
            .unwrap();
        let bits = |what: &str| {
            std::thread::sleep(Duration::from_millis(50));
            let mut drain = [0u8; 512];
            let _ = port.read(&mut drain);
            port.write_all(&frame(frames::INJECT_SNAPSHOT, &[]))
                .unwrap();
            std::thread::sleep(Duration::from_millis(50));
            let mut chunk = [0u8; 512];
            let got = port.read(&mut chunk).unwrap_or(0);
            let reply = &chunk[..got];
            let locks = reply
                .windows(5)
                .position(|window| window == [0xDE, 0xAD, 40, 0, frames::INJECT_SNAPSHOT])
                .map(|at| frames::locked_targets(&reply[at + 5..at + 45]));
            println!("{what}: {:?}", locks.map(|bits| format!("{bits:014b}")));
        };
        let set = |target: u8, on: u8| {
            port.write_all(&frame(frames::LOCK, &[target, on])).unwrap();
        };
        bits("nothing");
        set(5, 1);
        bits("lock 5");
        set(5, 0);
        bits("unlock 5");
        set(6, 1);
        set(7, 1);
        bits("lock 6 and 7");
        set(6, 0);
        set(7, 0);
        set(5, 1);
        set(6, 0);
        bits("lock 5 then unlock 6");
        for target in 5..8 {
            set(target, 0);
        }
        bits("all off");
    }

    /// prints what comes back, byte for byte, for a text command the box refuses and one it doesn't know
    #[test]
    #[ignore]
    fn refused_and_unknown_text_commands_reply() {
        use crate::transport::System;
        let port = crate::transport::os::OsSystem
            .open(std::path::Path::new(&port()), 4_000_000)
            .unwrap();
        for command in [
            &b"km.left(9)\r\n"[..],
            b"km.nothing()\r\n",
            b"km.version()\r\n",
        ] {
            port.write_all(command).unwrap();
            std::thread::sleep(Duration::from_millis(300));
            let mut chunk = [0u8; 256];
            let got = port.read(&mut chunk).unwrap_or(0);
            println!(
                "{:?} -> {:02X?} {:?}",
                String::from_utf8_lossy(command),
                &chunk[..got],
                String::from_utf8_lossy(&chunk[..got])
            );
        }
    }

    /// a box that resets when DTR and RTS change on open answers late or not at all, so every open has to
    /// answer within 2 seconds
    #[test]
    #[ignore]
    fn fifty_opens_each_answer_right_away() {
        let mut slowest = Duration::ZERO;
        for round in 0..50 {
            let started = Instant::now();
            let device = open();
            slowest = slowest.max(started.elapsed());
            drop(device);
            assert!(started.elapsed() < Duration::from_secs(2), "round {round}");
        }
        println!("slowest open {slowest:?}");
    }

    #[test]
    #[ignore]
    fn connect_without_a_family_still_finds_it_after_the_medius_probe() {
        let device = Connect::port(port())
            .open()
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(device.snapshot().info.family, Family::MakcuV4);
    }

    #[test]
    #[ignore]
    fn the_cursor_moves_right_and_back() {
        let device = open();
        for _ in 0..20 {
            device.move_by(2, 0).unwrap();
            std::thread::sleep(Duration::from_millis(10));
        }
        for _ in 0..20 {
            device.move_now(-2, 0).unwrap();
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// with you. you have 3 seconds to put the cursor on an empty spot of the desktop, then the box's own
    /// right click should open the context menu
    #[test]
    #[ignore]
    fn a_box_right_click_reaches_the_pc() {
        let device = open();
        std::thread::sleep(Duration::from_secs(3));
        device.click(Button::Right).unwrap();
    }

    /// with you. you have 2 seconds to put the cursor on a long web page, then it scrolls three notches down
    /// in one command and 2 seconds later three notches up in one command
    #[test]
    #[ignore]
    fn three_notches_in_one_command_each_way() {
        let device = open();
        std::thread::sleep(Duration::from_secs(2));
        device.wheel(-3).unwrap();
        std::thread::sleep(Duration::from_secs(2));
        device.wheel(3).unwrap();
        std::thread::sleep(Duration::from_millis(500));
    }

    /// with you. when it says so, unplug only USB1 (the box's cable that carries the mouse to the PC), wait 3
    /// seconds and plug it back in. on V4.094 the control port goes down with it, so this shows a disconnect,
    /// a reconnect, and the release that cleans up the held middle button
    #[test]
    #[ignore]
    fn unplugging_usb1_shows_what_the_box_lost() {
        let device = open();
        let events = device.events();
        device.press(Button::Middle).unwrap();
        println!("middle held, unplug USB1 now, then plug it back in");
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(40) {
            if let Ok(event) = events.recv_timeout(Duration::from_millis(200))
                && !matches!(event, Event::Button { .. })
            {
                println!(
                    "{event:?} after {:?}, held {:?}",
                    started.elapsed(),
                    device.held()
                );
            }
        }
    }

    /// run it while you capture the box's USB traffic. it clicks middle with no hold, then with a hold of
    /// 120 ms, 1 ms and 5000 ms
    #[test]
    #[ignore]
    fn box_click_holds_for_a_capture() {
        let device = open();
        std::thread::sleep(Duration::from_secs(1));
        for hold in [None, Some(120), Some(1), Some(5000)] {
            let click =
                hold.map_or_else(Click::default, |ms| Click::hold(Duration::from_millis(ms)));
            println!("middle click, hold {hold:?} ms, at {:.6}", unix_now());
            device.click_with(Button::Middle, click).unwrap();
            std::thread::sleep(Duration::from_millis(5500));
        }
    }

    /// with you. you have 2 seconds to put the cursor on an empty spot of the desktop, then the box holds right
    /// and dropping the device lets it go, so the context menu opens about a second after the press
    #[test]
    #[ignore]
    fn a_held_button_is_let_go_when_the_device_is_dropped() {
        let device = open();
        std::thread::sleep(Duration::from_secs(2));
        device.press(Button::Right).unwrap();
        std::thread::sleep(Duration::from_secs(1));
        drop(device);
    }

    /// with you. when it says so, press left, right, middle, side1 (back) and side2 (forward) once each, in
    /// that order, within 30 seconds
    #[test]
    #[ignore]
    fn all_five_physical_buttons_arrive_in_order() {
        let device = open();
        let events = device.events();
        println!("press left, right, middle, back and forward now, one after another");
        let mut pressed = Vec::new();
        let until = Instant::now() + Duration::from_secs(30);
        while pressed.len() < 5 && Instant::now() < until {
            match events.recv_timeout(Duration::from_millis(200)) {
                Ok(Event::Button {
                    button,
                    pressed: true,
                    ..
                }) => {
                    println!("{button}");
                    pressed.push(button);
                }
                Ok(_) | Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Closed) => break,
            }
        }
        assert_eq!(
            pressed,
            [
                Button::Left,
                Button::Right,
                Button::Middle,
                Button::Side1,
                Button::Side2
            ]
        );
    }

    /// with you. when it says so, unplug the MOUSE from the box (USB3) within 15 seconds, then plug it back in
    #[test]
    #[ignore]
    fn unplugging_the_mouse_shows_in_the_snapshot() {
        let device = open();
        let events = device.events();
        println!("unplug the mouse from the box now, then plug it back in");
        let started = Instant::now();
        let mut seen = Vec::new();
        while started.elapsed() < Duration::from_secs(40) {
            if let Ok(event) = events.recv_timeout(Duration::from_millis(200))
                && !matches!(event, Event::Button { .. })
            {
                let routed = device
                    .snapshot()
                    .capabilities
                    .get(Action::Move)
                    .is_supported();
                println!(
                    "{event:?} after {:?}, mouse routed {routed}",
                    started.elapsed()
                );
                seen.push(event);
            }
        }
        assert!(!seen.is_empty(), "the box reported nothing");
    }

    /// with you. when it says so, unplug the box's control cable (USB2) within 15 seconds, then plug it back
    /// in. the middle button stays held the whole time, so a USB capture can show what happens to it
    #[test]
    #[ignore]
    fn unplugging_the_control_cable_disconnects_and_reconnects() {
        let device = open();
        let events = device.events();
        device.press(Button::Middle).unwrap();
        println!("middle held at {:.6}", unix_now());
        println!("unplug the control cable now, then plug it back in");
        let started = Instant::now();
        let mut seen = Vec::new();
        while !seen.contains(&"reconnected") && started.elapsed() < Duration::from_secs(45) {
            match events.recv_timeout(Duration::from_millis(200)) {
                Ok(Event::Disconnected) => {
                    println!(
                        "disconnected after {:?}, at {:.6}",
                        started.elapsed(),
                        unix_now()
                    );
                    seen.push("disconnected");
                }
                Ok(Event::Reconnected) => {
                    println!(
                        "reconnected after {:?}, at {:.6}",
                        started.elapsed(),
                        unix_now()
                    );
                    seen.push("reconnected");
                }
                _ => {}
            }
        }
        assert_eq!(seen, vec!["disconnected", "reconnected"]);
        device.move_by(1, 0).unwrap();
        device.move_by(-1, 0).unwrap();
    }

    /// sends moves as fast as the port takes them for a second and prints how many come back rejected
    #[test]
    #[ignore]
    fn a_flood_of_moves_shows_what_the_box_refuses() {
        let device = open();
        let events = device.events();
        let started = Instant::now();
        let mut sent = 0u32;
        while started.elapsed() < Duration::from_secs(1) {
            device
                .move_by(if sent.is_multiple_of(2) { 1 } else { -1 }, 0)
                .unwrap();
            sent += 1;
        }
        std::thread::sleep(Duration::from_millis(500));
        let rejected: u64 = std::iter::from_fn(|| events.try_recv().ok())
            .map(|event| match event {
                Event::Rejected { count, .. } => count,
                _ => 0,
            })
            .sum();
        println!("sent {sent} moves, {rejected} rejected");
    }

    /// sends moves at a fixed rate for a second, then counts the refusals once they've all had time to arrive.
    /// with ONE_WAY set every move goes one count right, so a USB capture can add up what reached the PC
    fn moves_at(
        device: &crate::device::Device,
        events: &crate::event::Events,
        per_second: Option<u32>,
    ) -> (u32, u64) {
        let started = Instant::now();
        let mut sent = 0u32;
        while started.elapsed() < Duration::from_secs(1) {
            if let Some(rate) = per_second {
                let due = started + Duration::from_secs_f64(f64::from(sent) / f64::from(rate));
                while Instant::now() < due {}
            }
            device
                .move_by(
                    if sent.is_multiple_of(2) || std::env::var("ONE_WAY").is_ok() {
                        1
                    } else {
                        -1
                    },
                    0,
                )
                .unwrap();
            sent += 1;
        }
        std::thread::sleep(Duration::from_secs(1));
        let rejected: u64 = std::iter::from_fn(|| events.try_recv().ok())
            .map(|event| match event {
                Event::Rejected { count, .. } => count,
                _ => 0,
            })
            .sum();
        (sent, rejected)
    }

    #[test]
    #[ignore]
    fn how_many_moves_a_second_the_box_takes() {
        let device = open();
        let events = device.events();
        for rate in [None, Some(1000), Some(2000)] {
            println!("phase {rate:?} starts at {:.6}", unix_now());
            let (sent, rejected) = moves_at(&device, &events, rate);
            let accepted = u64::from(sent) - rejected;
            println!("{rate:?} per second: sent {sent}, rejected {rejected}, accepted {accepted}");
            std::thread::sleep(Duration::from_millis(500));
        }
    }

    /// wall clock seconds, because that's the clock a USBPcap capture stamps its packets with
    fn unix_now() -> f64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
    }
}

// keyboard

#[test]
fn keys_go_out_as_their_own_opcodes() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, _poller, _signals) = rig.attach().unwrap();
    rig.clear();
    backend.send(Command::KeyPress(Key::A)).unwrap();
    backend.send(Command::KeyRelease(Key::A)).unwrap();
    backend
        .send(Command::KeyTap {
            key: Key::B,
            hold: None,
        })
        .unwrap();
    backend
        .send(Command::KeyTap {
            key: Key::B,
            hold: Some(Duration::from_millis(300)),
        })
        .unwrap();
    assert_eq!(
        rig.written(),
        vec![
            frame(0x20, &[0x04]),
            frame(0x21, &[0x04]),
            frame(0x23, &[0x05]),
            // the hold is a u32 in ms
            frame(0x23, &[0x05, 0x2C, 0x01, 0x00, 0x00]),
        ]
    );
}

#[test]
fn keys_follow_the_keyboard_bit_and_media_keys_are_never_there() {
    for (kinds, keys) in [(0x03, true), (0x01, false)] {
        let rig = rig(State {
            kinds,
            ..State::makcu(4094)
        });
        let (_backend, snapshot, _poller, _signals) = rig.attach().unwrap();
        let capabilities = &snapshot.capabilities;
        for action in [Action::KeyPress, Action::KeyRelease, Action::KeyTap] {
            assert_eq!(capabilities.get(action).is_supported(), keys, "{action}");
        }
        if !keys {
            match &capabilities.get(Action::KeyPress).execution {
                Execution::Unsupported { reason } => {
                    assert!(reason.contains("no keyboard interface"), "{reason}");
                }
                other => panic!("{other:?}"),
            }
        }
        for action in [Action::MediaPress, Action::MediaRelease, Action::MediaTap] {
            assert!(!capabilities.get(action).is_supported(), "{action}");
        }
    }
}

#[test]
fn a_keyboard_routed_or_pulled_after_connect_shows_up_on_the_fourth_tick() {
    let rig = rig(State::makcu(4094));
    let (_backend, _snapshot, mut poller, signals) = rig.attach().unwrap();
    rig.set(|state| state.kinds = 0x01);
    for _ in 0..4 {
        poller.tick();
    }
    let signals: Vec<Signal> = signals.try_iter().collect();
    // held state gets let go on a MAKCU route change, so it's a reconnect and never a quiet change
    assert!(
        !signals
            .iter()
            .any(|signal| matches!(signal, Signal::Changed(_)))
    );
    let snapshots: Vec<Snapshot> = signals
        .into_iter()
        .filter_map(|signal| match signal {
            Signal::Reconnected(snapshot) => Some(*snapshot),
            _ => None,
        })
        .collect();
    assert_eq!(snapshots.len(), 1);
    assert!(
        !snapshots[0]
            .capabilities
            .get(Action::KeyPress)
            .is_supported()
    );
    assert!(snapshots[0].capabilities.get(Action::Move).is_supported());
}

#[test]
fn each_refused_key_opcode_says_which_key_action() {
    let mut codec = MakcuFrames::binary(Arc::new(frames::LastSent::default()));
    for (command, expected) in [
        (0x20, Action::KeyPress),
        (0x21, Action::KeyRelease),
        (0x23, Action::KeyTap),
    ] {
        match codec.classify(&frame(command, &[0xFF])) {
            Incoming::Refused { action, .. } => assert_eq!(action, Some(expected)),
            other => panic!("{command:02X} gave {other:?}"),
        }
    }
}

#[test]
fn a_key_release_the_box_lost_goes_again_and_a_held_key_is_left_alone() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, _signals) = rig.attach().unwrap();
    backend.send(Command::KeyPress(Key::B)).unwrap();
    backend.send(Command::KeyPress(Key::LEFT_SHIFT)).unwrap();
    // the box still holds A and right ctrl even though the last writes for them were releases
    rig.set(|state| {
        state.keys.insert(0x04);
        state.keys.insert(0xE4);
    });
    rig.clear();
    poller.tick();
    let written = rig.written();
    assert!(written.contains(&frame(0x21, &[0x04])), "{written:02X?}");
    assert!(written.contains(&frame(0x21, &[0xE4])), "{written:02X?}");
    assert!(!written.contains(&frame(0x21, &[0x05])));
    assert!(!written.contains(&frame(0x21, &[0xE1])));
    assert_eq!(rig.state.lock().unwrap().keys, HashSet::from([0x05, 0xE1]));
}

#[test]
fn a_held_key_never_makes_a_session_reset() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, signals) = rig.attach().unwrap();
    backend.send(Command::KeyPress(Key::A)).unwrap();
    rig.set(|state| state.keys.clear());
    for _ in 0..3 {
        poller.tick();
    }
    assert!(link_events(&signals).is_empty());
}

#[test]
fn release_all_lets_go_of_each_key_and_never_sends_key_init() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, _poller, _signals) = rig.attach().unwrap();
    rig.clear();
    backend
        .release_all(&[Usage::Key(Key::A), Usage::Button(Button::Left)])
        .unwrap();
    let written = rig.written();
    assert!(written.contains(&frame(0x21, &[0x04])));
    assert!(written.contains(&frame(0x11, &[0x00])));
    assert!(
        written
            .iter()
            .all(|bytes| frames::parts(bytes).map(|(c, _)| c) != Some(0x22))
    );
}

#[test]
fn a_box_timed_tap_is_capped_like_a_click() {
    let rig = rig(State::makcu(4094));
    let (device, _poller) = rig.device();
    assert!(matches!(
        device.tap_key_with(Key::A, Click::hold(Duration::from_millis(5001))),
        Err(Error::InvalidArgument { .. })
    ));
    rig.clear();
    device
        .tap_key_with(Key::A, Click::hold(Duration::from_millis(5000)))
        .unwrap();
    assert_eq!(
        rig.written(),
        vec![frame(0x23, &[0x04, 0x88, 0x13, 0x00, 0x00])]
    );
}

#[test]
fn injected_keys_reads_modifiers_and_the_bitmap() {
    let mut snapshot = vec![0u8; 40];
    snapshot[3] = 0b1000_0001;
    snapshot[4] = 0b0001_0000;
    snapshot[35] = 0b0000_0001;
    assert_eq!(
        frames::injected_keys(&snapshot),
        vec![0xE0, 0xE7, 0x04, 0xF8]
    );
    assert!(frames::injected_keys(&[1]).is_empty());
}

#[test]
fn a_key_release_the_box_refused_goes_again() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, _signals) = rig.attach().unwrap();
    backend.send(Command::KeyPress(Key::A)).unwrap();
    rig.set(|state| {
        state.refuse_once.insert(frames::KEY_UP);
    });
    backend.send(Command::KeyRelease(Key::A)).unwrap();
    until(|| rig.state.lock().unwrap().refuse_once.is_empty());
    assert!(rig.state.lock().unwrap().keys.contains(&0x04));
    poller.tick();
    assert!(!rig.state.lock().unwrap().keys.contains(&0x04));
}

#[test]
fn a_running_box_tap_is_never_cut_short_by_the_poller() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, _signals) = rig.attach().unwrap();
    backend
        .send(Command::KeyTap {
            key: Key::A,
            hold: Some(Duration::from_millis(300)),
        })
        .unwrap();
    // the box shows a tap that's still running as held
    rig.set(|state| {
        state.keys.insert(0x04);
    });
    rig.clear();
    poller.tick();
    assert!(!rig.written().contains(&frame(frames::KEY_UP, &[0x04])));
}

// key events

fn key_frame(usage: u8, state: u8) -> Vec<u8> {
    vec![0xDE, 0xAD, 0x03, 0x00, 0x53, 0x02, usage, state]
}

#[test]
fn physical_keys_arrive_as_key_events_and_odd_ones_are_noise() {
    let rig = rig(State::makcu(4094));
    let (device, _poller) = rig.device();
    let events = device.events();
    for frame in [
        key_frame(0xE1, 1),
        key_frame(0x00, 1),
        key_frame(0x03, 1),
        key_frame(0x04, 2),
        key_frame(0x04, 1),
        key_frame(0x04, 0),
    ] {
        rig.fake.send(&frame);
    }
    let mut seen = Vec::new();
    while seen.len() < 3 {
        if let Ok(Event::Key { key, pressed, .. }) = events.recv_timeout(Duration::from_secs(2)) {
            seen.push((key, pressed));
        }
    }
    assert_eq!(
        seen,
        [(Key::LEFT_SHIFT, true), (Key::A, true), (Key::A, false)]
    );
    assert!(
        events
            .recv_timeout(Duration::from_millis(100))
            .is_err_and(|error| error == crate::event::RecvTimeoutError::Timeout)
    );
}

#[test]
fn a_keyboard_overflow_restarts_only_the_keyboard_stream() {
    let rig = rig(State::makcu(4094));
    let (device, _poller) = rig.device();
    let events = device.events();
    rig.clear();
    rig.fake
        .send(&[0xDE, 0xAD, 0x03, 0x00, 0x53, 0x02, 0xFF, 0xFF]);
    assert_eq!(
        events.recv_timeout(Duration::from_secs(2)).unwrap(),
        Event::Dropped {
            count: 1,
            source: DropSource::Device
        }
    );
    until(|| rig.written().contains(&frame(0x52, &[2, 1])));
    assert!(!rig.written().contains(&frame(0x52, &[1, 1])));
}

#[test]
fn a_keyboard_stream_never_confirmed_is_asked_again_and_never_a_reset() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, signals) = rig.attach().unwrap();
    rig.set(|state| {
        state.refuse_once.insert(0x52);
    });
    let keys = InputKinds {
        keys: true,
        ..InputKinds::default()
    };
    assert!(backend.set_streams(keys).is_err());
    rig.set(|state| state.key_stream = 0);
    for _ in 0..3 {
        poller.tick();
    }
    assert!(link_events(&signals).is_empty());
    assert_eq!(rig.state.lock().unwrap().key_stream, 1);
}

#[test]
fn a_keyboard_stream_that_reads_off_is_one_drop_and_switched_on_again_never_a_reset() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, signals) = rig.attach().unwrap();
    let both = InputKinds {
        buttons: true,
        keys: true,
        ..InputKinds::default()
    };
    backend.set_streams(both).unwrap();
    rig.set(|state| state.key_stream = 0);
    for _ in 0..3 {
        poller.tick();
    }
    let signals: Vec<Signal> = signals.try_iter().collect();
    assert!(
        !signals
            .iter()
            .any(|signal| matches!(signal, Signal::SessionReset)),
        "{signals:?}"
    );
    assert_eq!(
        signals
            .iter()
            .filter(|signal| matches!(signal, Signal::Dropped(1, DropSource::Device, _)))
            .count(),
        1
    );
    assert_eq!(rig.state.lock().unwrap().key_stream, 1);
}

#[test]
fn a_keyboard_stream_query_the_box_refuses_never_stops_the_tick() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, _signals) = rig.attach().unwrap();
    backend
        .set_streams(InputKinds {
            keys: true,
            ..InputKinds::default()
        })
        .unwrap();
    // the box still holds left even though the last write for it was a release
    rig.set(|state| {
        state.injected = 1;
        state.refuse.insert(0x52);
    });
    poller.tick();
    assert_eq!(rig.state.lock().unwrap().injected, 0);
}

#[test]
fn a_refused_keyboard_stream_switch_names_key_events() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, _poller, _signals) = rig.attach().unwrap();
    rig.set(|state| {
        state.refuse.insert(0x52);
    });
    assert!(matches!(
        backend
            .set_streams(InputKinds {
                keys: true,
                ..InputKinds::default()
            })
            .map_err(|failure| failure.error),
        Err(Error::Rejected {
            action: Action::KeyEvents,
            ..
        })
    ));
    let mut codec = MakcuFrames::binary(backend.shared.last.clone());
    assert!(matches!(
        codec.classify(&frame(0x52, &[0xFF])),
        Incoming::Refused {
            action: Some(Action::KeyEvents),
            ..
        }
    ));
}

#[test]
fn the_keyboard_going_away_turns_its_stream_off_with_no_reset() {
    let rig = rig(State::makcu(4094));
    let (device, mut poller) = rig.device();
    let _events = device.events();
    assert_eq!(rig.state.lock().unwrap().key_stream, 1);
    rig.set(|state| state.kinds = 0x01);
    let reset = device.events();
    for _ in 0..4 {
        poller.tick();
    }
    device.settle();
    until(|| rig.state.lock().unwrap().key_stream == 0);
    assert!(
        !std::iter::from_fn(|| reset.try_recv().ok()).any(|event| event == Event::SessionReset)
    );
}

#[test]
fn shutdown_switches_both_streams_off() {
    let rig = rig(State::makcu(4094));
    let (device, _poller) = rig.device();
    let events = device.events();
    assert_eq!(rig.state.lock().unwrap().key_stream, 1);
    drop(events);
    let _events = device.events();
    drop(device);
    let state = rig.state.lock().unwrap();
    assert_eq!((state.stream, state.key_stream), (0, 0));
}

#[test]
fn a_box_without_the_keyboard_bit_never_gets_its_stream_switched_on() {
    let rig = rig(State {
        kinds: 0x01,
        ..State::makcu(4094)
    });
    let (device, _poller) = rig.device();
    rig.clear();
    let _events = device.events();
    assert!(!rig.written().contains(&frame(0x52, &[2, 1])));
}

#[test]
fn a_refused_keyboard_stream_retry_names_key_events() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, _signals) = rig.attach().unwrap();
    rig.set(|state| {
        state.refuse_once.insert(0x52);
    });
    let _ = backend.set_streams(InputKinds {
        keys: true,
        ..InputKinds::default()
    });
    // the last stream switch that went out was for the mouse, then the poller asks for the keyboard again
    backend.shared.last.set_stream(frames::MOUSE);
    rig.set(|state| state.key_stream = 0);
    poller.tick();
    let mut codec = MakcuFrames::binary(backend.shared.last.clone());
    assert!(matches!(
        codec.classify(&frame(0x52, &[0xFF])),
        Incoming::Refused {
            action: Some(Action::KeyEvents),
            ..
        }
    ));
}

#[test]
fn restarting_a_stream_nobody_wants_sends_nothing() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, _poller, _signals) = rig.attach().unwrap();
    rig.clear();
    backend.restart_stream(InputKind::Keys).unwrap();
    assert!(rig.written().is_empty());
}

#[test]
fn a_keyboard_stream_lost_with_its_bit_gone_isnt_asked_again() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, _signals) = rig.attach().unwrap();
    backend
        .set_streams(InputKinds {
            keys: true,
            ..InputKinds::default()
        })
        .unwrap();
    rig.set(|state| {
        state.key_stream = 0;
        state.kinds = 0x01;
    });
    // the route check runs on the fourth tick, and that's where the snapshot loses the keyboard
    for _ in 0..4 {
        poller.tick();
    }
    rig.clear();
    rig.set(|state| state.key_stream = 0);
    for _ in 0..3 {
        poller.tick();
    }
    assert!(!rig.written().contains(&frame(0x52, &[2, 1])));
}

// locks

fn lock(backend: &MakcuBackend, slot: LockSlot, keep: i16) -> crate::error::Result<()> {
    backend.send(Command::Lock { slot, keep })
}

#[test]
fn locks_go_out_as_lock_and_key_mask_with_the_signed_axis_targets() {
    let rig = rig(State::makcu(4094));
    let (backend, snapshot, _poller, _signals) = rig.attach().unwrap();
    assert_eq!(
        snapshot.capabilities.axes(),
        [Axis::X, Axis::Y, Axis::Wheel]
    );
    assert!(!snapshot.capabilities.get(Action::ScaleAxis).is_supported());
    rig.clear();
    lock(&backend, LockSlot::Button(Button::Side2), 0).unwrap();
    for (axis, positive, negative) in [(Axis::X, 6, 7), (Axis::Y, 9, 10), (Axis::Wheel, 12, 13)] {
        lock(&backend, LockSlot::Axis(axis, Sign::Positive), 0).unwrap();
        lock(&backend, LockSlot::Axis(axis, Sign::Negative), 100).unwrap();
        assert!(rig.written().contains(&frame(0x60, &[positive, 1])));
        assert!(rig.written().contains(&frame(0x60, &[negative, 0])));
    }
    lock(&backend, LockSlot::Key(Key::A), 0).unwrap();
    assert!(rig.written().contains(&frame(0x60, &[4, 1])));
    assert!(rig.written().contains(&frame(0x29, &[0x04, 1])));
    assert!(matches!(
        lock(&backend, LockSlot::Axis(Axis::X, Sign::Positive), 50),
        Err(Error::Unsupported { .. })
    ));
}

#[test]
fn a_lock_the_box_lost_goes_again_and_someone_elses_is_left_alone() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, _signals) = rig.attach().unwrap();
    lock(&backend, LockSlot::Button(Button::Left), 0).unwrap();
    // the box lost Anybox's lock and shows a lock on right that Anybox never wrote
    rig.set(|state| state.locks = 1 << 1);
    poller.tick();
    assert_eq!(rig.state.lock().unwrap().locks, 0b11);
}

#[test]
fn an_unlock_the_box_didnt_take_goes_again() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, _signals) = rig.attach().unwrap();
    lock(&backend, LockSlot::Button(Button::Left), 0).unwrap();
    rig.set(|state| {
        state.refuse_once.insert(0x60);
    });
    lock(&backend, LockSlot::Button(Button::Left), 100).unwrap();
    until(|| rig.state.lock().unwrap().refuse_once.is_empty());
    assert_eq!(rig.state.lock().unwrap().locks, 1);
    poller.tick();
    assert_eq!(rig.state.lock().unwrap().locks, 0);
}

#[test]
fn a_lock_the_box_keeps_refusing_stops_after_three_tries() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, _signals) = rig.attach().unwrap();
    rig.set(|state| {
        state.refuse.insert(0x60);
    });
    lock(&backend, LockSlot::Button(Button::Left), 0).unwrap();
    rig.clear();
    for _ in 0..6 {
        poller.tick();
    }
    let tries = rig
        .written()
        .iter()
        .filter(|bytes| **bytes == frame(0x60, &[0, 1]))
        .count();
    assert_eq!(tries, 3);
}

#[test]
fn key_masks_go_again_every_fourth_tick() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, _signals) = rig.attach().unwrap();
    lock(&backend, LockSlot::Key(Key::A), 0).unwrap();
    rig.set(|state| state.masks.clear());
    for _ in 0..4 {
        poller.tick();
    }
    assert!(rig.state.lock().unwrap().masks.contains(&0x04));
    lock(&backend, LockSlot::Key(Key::A), 100).unwrap();
    rig.clear();
    for _ in 0..4 {
        poller.tick();
    }
    assert!(!rig.written().contains(&frame(0x29, &[0x04, 1])));
}

#[test]
fn shutdown_looks_once_more_for_an_unlock_the_box_didnt_take() {
    let rig = rig(State::makcu(4094));
    let (device, _poller) = rig.device();
    device.lock_button(Button::Left).unwrap();
    rig.set(|state| {
        state.refuse_once.insert(0x60);
    });
    drop(device);
    assert_eq!(rig.state.lock().unwrap().locks, 0);
}

#[test]
fn a_refused_lock_names_a_button_or_an_axis_and_a_refused_mask_a_key() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, _poller, _signals) = rig.attach().unwrap();
    let mut codec = MakcuFrames::binary(backend.shared.last.clone());
    let refused =
        |codec: &mut MakcuFrames, command: u8| match codec.classify(&frame(command, &[0xFF])) {
            Incoming::Refused { action, .. } => action,
            other => panic!("{other:?}"),
        };
    lock(&backend, LockSlot::Button(Button::Left), 0).unwrap();
    assert_eq!(refused(&mut codec, 0x60), Some(Action::LockButton));
    lock(&backend, LockSlot::Axis(Axis::Y, Sign::Positive), 0).unwrap();
    assert_eq!(refused(&mut codec, 0x60), Some(Action::LockAxis));
    assert_eq!(refused(&mut codec, 0x29), Some(Action::LockKey));
}

#[test]
fn every_loss_of_an_accepted_lock_gets_its_own_tries() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, _signals) = rig.attach().unwrap();
    lock(&backend, LockSlot::Button(Button::Left), 0).unwrap();
    for round in 0..5 {
        rig.set(|state| state.locks = 0);
        poller.tick();
        assert_eq!(rig.state.lock().unwrap().locks, 1, "round {round}");
        // this tick sees the lock back on
        poller.tick();
    }
}

#[test]
fn a_new_write_after_the_tries_ran_out_starts_the_count_again() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, _signals) = rig.attach().unwrap();
    rig.set(|state| {
        state.refuse.insert(0x60);
    });
    lock(&backend, LockSlot::Button(Button::Left), 0).unwrap();
    for _ in 0..4 {
        poller.tick();
    }
    rig.set(|state| {
        state.refuse.clear();
    });
    lock(&backend, LockSlot::Button(Button::Left), 0).unwrap();
    rig.set(|state| state.locks = 0);
    poller.tick();
    assert_eq!(rig.state.lock().unwrap().locks, 1);
}

#[test]
fn shutdown_leaves_a_lock_anybox_never_wrote_alone() {
    let rig = rig(State::makcu(4094));
    let (device, _poller) = rig.device();
    rig.set(|state| state.locks = 1 << 1);
    drop(device);
    assert_eq!(rig.state.lock().unwrap().locks, 1 << 1);
}

#[test]
fn pan_is_unsupported_and_says_why() {
    let rig = rig(State::makcu(4094));
    let (_backend, snapshot, _poller, _signals) = rig.attach().unwrap();
    match &snapshot.capabilities.get(Action::Pan).execution {
        Execution::Unsupported { reason } => {
            assert!(reason.contains("only exists on V3"), "{reason}")
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_off_the_deadline_skipped_still_goes_out_at_shutdown() {
    let rig = rig(State::makcu(4094));
    let (device, _poller) = rig.device();
    let events = device.events();
    assert_eq!(rig.state.lock().unwrap().key_stream, 1);
    rig.set(|state| {
        state
            .slow
            .insert(frames::INPUT_STREAM, Duration::from_millis(300));
    });
    drop(events);
    assert_eq!(rig.state.lock().unwrap().key_stream, 1);
    rig.set(|state| state.slow.clear());
    drop(device);
    assert_eq!(rig.state.lock().unwrap().key_stream, 0);
}

#[test]
fn shutdown_stays_within_its_deadline_on_a_slow_box() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, _poller, _signals) = rig.attach().unwrap();
    backend
        .set_streams(InputKinds {
            buttons: true,
            keys: true,
            ..InputKinds::default()
        })
        .unwrap();
    rig.set(|state| {
        state
            .slow
            .insert(frames::INPUT_STREAM, Duration::from_millis(400));
        state
            .slow
            .insert(frames::INJECT_SNAPSHOT, Duration::from_millis(400));
    });
    let started = Instant::now();
    backend.shutdown(Duration::from_secs(1));
    let took = started.elapsed();
    assert!(took < Duration::from_secs(2), "{took:?}");
}

#[test]
fn shutdown_unlocks_a_lock_whose_unlock_was_never_sent() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, _poller, _signals) = rig.attach().unwrap();
    lock(&backend, LockSlot::Button(Button::Left), 0).unwrap();
    assert_eq!(rig.state.lock().unwrap().locks, 1);
    backend.shutdown(Duration::from_secs(1));
    assert_eq!(rig.state.lock().unwrap().locks, 0);
}

#[test]
fn a_slow_stream_off_never_costs_the_last_unlock() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, _poller, _signals) = rig.attach().unwrap();
    backend
        .set_streams(InputKinds {
            buttons: true,
            keys: true,
            ..InputKinds::default()
        })
        .unwrap();
    lock(&backend, LockSlot::Button(Button::Left), 0).unwrap();
    rig.set(|state| {
        state
            .slow
            .insert(frames::INPUT_STREAM, Duration::from_millis(450));
    });
    backend.shutdown(Duration::from_millis(800));
    assert_eq!(rig.state.lock().unwrap().locks, 0);
}

#[test]
fn shutdown_unmasks_a_key_whose_unlock_was_never_sent() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, _poller, _signals) = rig.attach().unwrap();
    lock(&backend, LockSlot::Key(Key::A), 0).unwrap();
    assert!(rig.state.lock().unwrap().masks.contains(&0x04));
    backend.shutdown(Duration::from_secs(1));
    assert!(rig.state.lock().unwrap().masks.is_empty());
}

#[test]
fn an_event_says_when_its_bytes_were_read_and_the_qpc_on_windows() {
    let rig = rig(State::makcu(4094));
    let (device, _poller) = rig.device();
    let events = device.events();
    rig.fake
        .send(&[0xDE, 0xAD, 0x03, 0x00, 0x53, 0x01, 0x02, 0x01]);
    let Ok(Event::Button { at, .. }) = events.recv_timeout(Duration::from_secs(2)) else {
        panic!("no button")
    };
    // on MAKCU the read is the best time anybox has, so it's both
    assert_eq!(at.received, at.arrived);
    assert_eq!(at.received_since_open, at.since_open);
    assert_eq!(at.received_qpc.is_some(), cfg!(windows));
}

#[test]
fn a_refused_stream_switch_is_one_rejected() {
    let rig = rig(State::makcu(4094));
    let (device, _poller) = rig.device();
    rig.set(|state| {
        state.refuse.insert(0x52);
    });
    let events = device.events_of(crate::event::EventKinds::BUTTONS);
    let mut count = 0;
    while let Ok(event) = events.recv_timeout(Duration::from_millis(700)) {
        if let Event::Rejected {
            action: Action::ButtonEvents,
            count: n,
        } = event
        {
            count += n;
        }
    }
    assert_eq!(count, 1);
}

#[test]
fn a_receiver_made_right_after_a_refused_one_gets_its_own_single_rejected() {
    let rig = rig(State::makcu(4094));
    let (device, _poller) = rig.device();
    rig.set(|state| {
        state.refuse.insert(0x52);
    });
    let rejected = |events: &crate::event::Events| {
        let mut count = 0;
        while let Ok(event) = events.recv_timeout(Duration::from_millis(300)) {
            if let Event::Rejected {
                action: Action::ButtonEvents,
                count: n,
            } = event
            {
                count += n;
            }
        }
        count
    };
    let first = device.events_of(crate::event::EventKinds::BUTTONS);
    assert_eq!(rejected(&first), 1);
    drop(first);
    let second = device.events_of(crate::event::EventKinds::BUTTONS);
    assert_eq!(rejected(&second), 1);
}

fn typing_rig() -> (Rig, Device, Poller) {
    let rig = rig(State::makcu(4094));
    let (device, poller) = rig.device();
    rig.clear();
    (rig, device, poller)
}

fn strings(written: &[Vec<u8>]) -> Vec<Vec<u8>> {
    written
        .iter()
        .filter_map(|bytes| frames::parts(bytes))
        .filter(|(command, payload)| *command == frames::KEY_STRING && !payload.is_empty())
        .map(|(_, payload)| payload.to_vec())
        .collect()
}

#[test]
fn a_text_waits_for_the_box_to_type_each_piece() {
    let (rig, device, _poller) = typing_rig();
    let started = Instant::now();
    device.type_text("hello").unwrap();
    // 5 characters at 30 ms each, the call only returns once the box is done
    assert!(started.elapsed() >= Duration::from_millis(150));
    assert_eq!(rig.state.lock().unwrap().typed, b"hello");
    assert_eq!(
        strings(&rig.written()),
        vec![b"he".to_vec(), b"ll".to_vec(), b"o".to_vec()]
    );
    // a key right after the text goes out once the box takes keys again, so it isn't refused
    device.tap_key(Key::ENTER).unwrap();
    assert!(
        !rig.written()
            .iter()
            .any(|bytes| bytes == &frame(frames::KEY_PRESS, &[0xFF]))
    );
}

#[test]
fn a_piece_the_box_never_types_is_rejected_and_never_sent_again() {
    let (rig, device, _poller) = typing_rig();
    rig.state.lock().unwrap().ignores_text = true;
    match device.type_text("abcd") {
        Err(Error::Rejected {
            action: Action::TypeText,
            ..
        }) => {}
        other => panic!("{other:?}"),
    }
    assert_eq!(strings(&rig.written()), vec![b"ab".to_vec()]);
}

#[test]
fn a_box_that_never_gets_done_typing_takes_the_link_down() {
    let (rig, device, _poller) = typing_rig();
    rig.state.lock().unwrap().per_character = Duration::from_secs(10);
    assert!(matches!(device.type_text("ab"), Err(Error::Disconnected)));
}

#[test]
fn a_piece_waits_for_a_tap_thats_still_running() {
    let (rig, device, _poller) = typing_rig();
    // the box is still busy for a bit, like after its own tap
    rig.state.lock().unwrap().typing_until = Some(Instant::now() + Duration::from_millis(80));
    device.type_text("ab").unwrap();
    assert_eq!(rig.state.lock().unwrap().typed, b"ab");
}

#[test]
fn a_box_still_busy_past_the_bound_is_rejected_and_nothing_is_typed() {
    let (rig, device, _poller) = typing_rig();
    device.tap_key(Key::A).unwrap();
    rig.state.lock().unwrap().types_forever = true;
    assert!(matches!(
        device.type_text("ab"),
        Err(Error::Rejected {
            action: Action::TypeText,
            ..
        })
    ));
    assert!(strings(&rig.written()).is_empty());
}

#[test]
fn typing_takes_a_typed_key_off_what_anybox_holds_and_the_poller_leaves_it() {
    let (rig, device, mut poller) = typing_rig();
    device.press_key(Key::A).unwrap();
    device.press_key(Key::F1).unwrap();
    device.type_text("a").unwrap();
    assert_eq!(device.held(), HashSet::from([Usage::Key(Key::F1)]));
    rig.clear();
    poller.tick();
    assert!(!rig.written().iter().any(|bytes| frames::parts(bytes).is_some_and(|(command, _)| command == frames::KEY_UP)));
}

#[test]
fn a_snapshot_the_poller_read_during_a_piece_never_lets_go_of_the_typed_key() {
    let (rig, device, mut poller) = typing_rig();
    rig.state.lock().unwrap().per_character = Duration::from_millis(200);
    thread::scope(|scope| {
        let typing = scope.spawn(|| device.type_text("a"));
        while rig.state.lock().unwrap().typed.is_empty() {
            thread::sleep(Duration::from_millis(1));
        }
        rig.clear();
        // reads the count and a snapshot with A down, then waits for the piece to let go of `buttons`
        poller.tick();
        typing.join().unwrap().unwrap();
    });
    assert!(!rig.written().contains(&frame(frames::KEY_UP, &[Key::A.0])));
}

#[test]
fn a_chord_is_one_multi_key_command() {
    let (rig, device, _poller) = typing_rig();
    device.press_keys(&[Key::LEFT_CTRL, Key::A]).unwrap();
    device.release_keys(&[Key::LEFT_CTRL, Key::A]).unwrap();
    device.tap_keys(&[Key::B, Key::C]).unwrap();
    assert_eq!(
        rig.state.lock().unwrap().chords,
        vec![
            (frames::KEY_MULTI_DOWN, vec![0xE0, 0x04]),
            (frames::KEY_MULTI_UP, vec![0xE0, 0x04]),
            (frames::KEY_MULTI_PRESS, vec![0x05, 0x06]),
        ]
    );
}

#[test]
fn a_box_with_a_keyboard_types_and_presses_chords_and_one_without_says_why() {
    let snapshot = super::snapshot(4094, 0x03, None);
    let typing = snapshot.capabilities.get(Action::TypeText);
    assert_eq!(typing.execution, Execution::Native);
    assert!(
        typing
            .quirks
            .contains(&crate::capability::Quirk::TypedTextIgnoresCapsLock)
    );
    assert!(
        typing
            .quirks
            .contains(&crate::capability::Quirk::TypingReleasesHeldKeys)
    );
    assert!(snapshot.capabilities.get(Action::KeyChord).is_supported());
    let snapshot = super::snapshot(4094, 0x01, None);
    for action in [Action::TypeText, Action::KeyChord] {
        assert_eq!(
            snapshot.capabilities.get(action).reason(),
            super::NO_KEYBOARD
        );
    }
}

// the MAKCU's own calls

#[test]
fn interpolation_reads_off_the_snapshot_and_a_set_is_checked_against_it() {
    let rig = rig(State::makcu(4108));
    let (device, _poller) = rig.device();
    let makcu = device.makcu().unwrap();
    assert_eq!(makcu.interpolation().unwrap(), Interpolation::Auto);
    for value in [
        Interpolation::Percent(0),
        Interpolation::Percent(100),
        Interpolation::Auto,
    ] {
        makcu.set_interpolation(value).unwrap();
        assert_eq!(makcu.interpolation().unwrap(), value);
    }
    rig.clear();
    assert!(matches!(
        makcu.set_interpolation(Interpolation::Percent(101)),
        Err(Error::InvalidArgument { .. })
    ));
    assert!(rig.written().is_empty());
    rig.set(|state| state.ignores_spread = true);
    assert!(matches!(
        makcu.set_interpolation(Interpolation::Percent(40)),
        Err(Error::Rejected {
            action: Action::Makcu(MakcuAction::Interpolation),
            ..
        })
    ));
}

#[test]
fn a_refused_interpolation_is_rejected_even_for_auto() {
    let rig = rig(State::makcu(4108));
    let (device, _poller) = rig.device();
    rig.set(|state| {
        state.spread = 20;
        state.refuse.insert(frames::INTERPOLATE);
    });
    let makcu = device.makcu().unwrap();
    for value in [Interpolation::Percent(30), Interpolation::Auto] {
        assert!(matches!(
            makcu.set_interpolation(value),
            Err(Error::Rejected { .. })
        ));
    }
}

#[test]
fn moving_says_what_the_box_says_and_a_refusal_is_rejected() {
    let rig = rig(State::makcu(4108));
    let (device, _poller) = rig.device();
    let makcu = device.makcu().unwrap();
    assert!(!makcu.moving().unwrap());
    rig.set(|state| state.moving = 1);
    assert!(makcu.moving().unwrap());
    rig.set(|state| {
        state.refuse.insert(frames::MOVING);
    });
    assert!(matches!(makcu.moving(), Err(Error::Rejected { .. })));
}

#[test]
fn autorelease_takes_whole_ms_up_to_a_minute_and_none_turns_it_off() {
    let rig = rig(State::makcu(4108));
    let (device, _poller) = rig.device();
    let makcu = device.makcu().unwrap();
    assert_eq!(makcu.autorelease().unwrap(), None);
    makcu
        .set_autorelease(Some(Duration::from_millis(1500)))
        .unwrap();
    assert_eq!(rig.state.lock().unwrap().autorelease, 1500);
    assert_eq!(
        makcu.autorelease().unwrap(),
        Some(Duration::from_millis(1500))
    );
    makcu.set_autorelease(None).unwrap();
    assert_eq!(makcu.autorelease().unwrap(), None);
    rig.clear();
    for bad in [
        Duration::from_secs(61),
        Duration::from_micros(1500),
        Duration::ZERO,
    ] {
        assert!(matches!(
            makcu.set_autorelease(Some(bad)),
            Err(Error::InvalidArgument { .. })
        ));
    }
    assert!(rig.written().is_empty());
}

#[test]
fn autorelease_goes_out_again_after_a_reconnect_and_not_once_it_was_turned_off() {
    let rig = rig(State::makcu(4108));
    let (device, _poller) = rig.device();
    let events = device.events();
    device
        .makcu()
        .unwrap()
        .set_autorelease(Some(Duration::from_secs(2)))
        .unwrap();
    let replug = |rig: &Rig| {
        // the box forgets it when it loses power
        rig.set(|state| state.autorelease = 0);
        rig.fake.vanish();
        rig.system.unplug(PORT);
        let fake = FakeBox::by_baud({
            let state = rig.state.clone();
            move |baud, bytes| respond(&state, baud, bytes)
        });
        rig.system.plug(PORT, makcu_usb(), fake);
    };
    let reconnected = || {
        until(|| {
            std::iter::from_fn(|| events.try_recv().ok())
                .any(|event| matches!(event, Event::Reconnected))
        })
    };
    replug(&rig);
    reconnected();
    assert_eq!(rig.state.lock().unwrap().autorelease, 2000);
    // the reconnect read the box's echo of it, so the first request after doesn't get it as its own answer
    let makcu = device.makcu().unwrap();
    makcu.set_autorelease(Some(Duration::from_secs(3))).unwrap();
    assert_eq!(makcu.autorelease().unwrap(), Some(Duration::from_secs(3)));
    device.makcu().unwrap().set_autorelease(None).unwrap();
    replug(&rig);
    reconnected();
    assert_eq!(rig.state.lock().unwrap().autorelease, 0);
}

#[test]
fn autorelease_needs_v4_108_and_the_calls_need_a_mouse() {
    let old = rig(State::makcu(4107));
    let (device, _poller) = old.device();
    assert!(matches!(
        device.makcu().unwrap().autorelease(),
        Err(Error::Unsupported {
            action: Action::Makcu(MakcuAction::Autorelease),
            ..
        })
    ));
    let mut state = State::makcu(4108);
    state.kinds = 0x02;
    let rig = rig(state);
    let (device, _poller) = rig.device();
    assert!(matches!(
        device.makcu().unwrap().interpolation(),
        Err(Error::Unsupported { .. })
    ));
    assert!(device.ysc().is_none() && device.ferrum().is_none());
}

#[test]
fn a_refused_set_nobody_waits_for_names_the_makcu_call() {
    for (command, action) in [
        (frames::INTERPOLATE, MakcuAction::Interpolation),
        (frames::MOVING, MakcuAction::Moving),
        (frames::AUTORELEASE, MakcuAction::Autorelease),
    ] {
        let mut codec = MakcuFrames::binary(Arc::default());
        assert!(matches!(
            codec.classify(&frame(command, &[0xFF])),
            Incoming::Refused { action: Some(Action::Makcu(got)), .. } if got == action
        ));
    }
}

#[test]
fn a_session_reset_puts_the_autorelease_back_and_none_forgets_it() {
    let rig = rig(State::makcu(4108));
    let (backend, _snapshot, mut poller, signals) = rig.attach().unwrap();
    let reset = |poller: &mut Poller| {
        backend.send(Command::Press(Button::Left)).unwrap();
        poller.tick();
        // the box rebooted by itself and forgot it with the rest
        rig.set(|state| {
            state.injected = 0;
            state.autorelease = 0;
        });
        poller.tick();
        poller.tick();
        assert_eq!(link_events(&signals), vec!["session reset"]);
    };
    backend
        .extension(Extension::MakcuSetAutorelease(Some(Duration::from_secs(2))))
        .unwrap();
    reset(&mut poller);
    assert_eq!(rig.state.lock().unwrap().autorelease, 2000);
    backend
        .extension(Extension::MakcuSetAutorelease(None))
        .unwrap();
    rig.fake.clear_written();
    reset(&mut poller);
    // not even a 0, none means anybox has nothing to put back
    assert!(
        !rig.fake
            .written()
            .iter()
            .any(|written| frames::parts(written)
                .is_some_and(|(cmd, payload)| cmd == frames::AUTORELEASE && payload.len() == 4)),
        "it was turned off, so nothing goes back"
    );
}

fn force(backend: &MakcuBackend, key: Key, force: bool) -> crate::error::Result<()> {
    backend.send(Command::KeyForce { key, force })
}

#[test]
fn a_forced_key_is_the_key_mask_and_the_poller_masks_it_again() {
    let rig = rig(State::makcu(4094));
    let (backend, snapshot, mut poller, _signals) = rig.attach().unwrap();
    assert!(
        snapshot
            .capabilities
            .get(Action::ForceReleaseKey)
            .is_supported()
    );
    rig.clear();
    force(&backend, Key::W, true).unwrap();
    assert!(rig.written().contains(&frame(0x29, &[0x1A, 1])));
    rig.set(|state| state.masks.clear());
    for _ in 0..4 {
        poller.tick();
    }
    assert!(rig.state.lock().unwrap().masks.contains(&0x1A));
    force(&backend, Key::W, false).unwrap();
    assert!(!rig.state.lock().unwrap().masks.contains(&0x1A));
    rig.clear();
    for _ in 0..4 {
        poller.tick();
    }
    assert!(!rig.written().contains(&frame(0x29, &[0x1A, 1])));
}

#[test]
fn a_refused_mask_names_the_force_or_the_lock_that_wrote_it() {
    let rig = rig(State::makcu(4094));
    let (backend, _snapshot, mut poller, _signals) = rig.attach().unwrap();
    let mut codec = MakcuFrames::binary(backend.shared.last.clone());
    let refused = |codec: &mut MakcuFrames| match codec.classify(&frame(0x29, &[0xFF])) {
        Incoming::Refused { action, .. } => action,
        other => panic!("{other:?}"),
    };
    force(&backend, Key::W, true).unwrap();
    assert_eq!(refused(&mut codec), Some(Action::ForceReleaseKey));
    lock(&backend, LockSlot::Key(Key::A), 0).unwrap();
    assert_eq!(refused(&mut codec), Some(Action::LockKey));
    force(&backend, Key::W, false).unwrap();
    assert_eq!(refused(&mut codec), Some(Action::ForceReleaseKey));
    // the poller's own re-send of a lock names the lock
    for _ in 0..4 {
        poller.tick();
    }
    assert_eq!(refused(&mut codec), Some(Action::LockKey));
    // and of a force the force
    lock(&backend, LockSlot::Key(Key::A), 100).unwrap();
    force(&backend, Key::B, true).unwrap();
    lock(&backend, LockSlot::Key(Key::C), 0).unwrap();
    lock(&backend, LockSlot::Key(Key::C), 100).unwrap();
    assert_eq!(refused(&mut codec), Some(Action::LockKey));
    for _ in 0..4 {
        poller.tick();
    }
    assert_eq!(refused(&mut codec), Some(Action::ForceReleaseKey));
}
