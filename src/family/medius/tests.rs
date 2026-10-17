use std::collections::HashSet;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use medius::MockBox;
use medius::{Caps, Class, Direction, FrameType, Health, KbdCaps, MouseCaps, Usage, Version};

use super::{MediusBackend, Poller, build, frame, open_failure, probe_old_firmware};
use crate::backend::{Backend, Command, LockSlot, Sign, Signal, Sink, WRITE_DEADLINE};
use crate::capability::{
    Action, Axis, Button, Execution, Family, Firmware, Quirk, Snapshot, StableId,
};
use crate::click::{ClickTiming, Holds};
use crate::device::{Device, Start};
use crate::error::Error;
use crate::event::{Clock, Event};
use crate::key::{Key, MediaKey};
use crate::transport::System;
use crate::transport::fake::{FakeBox, FakeSystem};

const MAC: [u8; 6] = [0x5c, 0x68, 0x24, 0x46, 0x73, 0x01];

fn mouse(buttons: u8, wheel: bool) -> MockBox {
    MockBox::new()
        .with_version(Version {
            proto_ver: 9,
            fw_major: 3,
            fw_minor: 4,
            fw_patch: 5,
            mac: MAC,
            name: "Medius-test".into(),
        })
        .with_caps(Caps {
            mouse: MouseCaps {
                n_buttons: buttons,
                has_x: true,
                has_y: true,
                has_wheel: wheel,
                n_hid: 1,
                ..MouseCaps::default()
            },
            ..Caps::default()
        })
}

fn backend(mock: &MockBox) -> (MediusBackend, Snapshot, Poller, Receiver<Signal>) {
    let device = medius::Device::open_mock(mock.clone()).unwrap();
    let (tx, rx) = mpsc::channel();
    let (backend, snapshot, poller) = build(device, Sink(tx)).unwrap();
    (backend, snapshot, poller, rx)
}

fn device(mock: &MockBox) -> (Device, Poller) {
    let crate_device = medius::Device::open_mock(mock.clone()).unwrap();
    let (signals, incoming) = mpsc::channel();
    let (backend, snapshot, poller) = build(crate_device, Sink(signals.clone())).unwrap();
    let device = Device::start(Start {
        backend: Box::new(backend),
        snapshot,
        signals,
        incoming,
        click_timing: ClickTiming::Fixed(Duration::from_millis(5)),
        holds: Holds::seeded(1),
        port: "test".into(),
    })
    .unwrap();
    (device, poller)
}

/// the button injects the box saw, as (button id, action). action 1 is a press, 0 a soft release and 2 a
/// force release
fn injects(mock: &MockBox) -> Vec<(u16, u8)> {
    mock.recorded_frames()
        .into_iter()
        .filter(|frame| frame.ty == FrameType::Inject && frame.payload[0] == 0)
        .map(|frame| {
            (
                u16::from_le_bytes([frame.payload[1], frame.payload[2]]),
                frame.payload[3],
            )
        })
        .collect()
}

/// the MOVE frames the box saw, as (motion, first value, flags). motion 0 is the cursor (dx) and 1 the
/// wheel (dz)
fn moves(mock: &MockBox) -> Vec<(u8, i16, u8)> {
    mock.recorded_frames()
        .into_iter()
        .filter(|frame| frame.ty == FrameType::Move)
        .map(|frame| {
            let p = &frame.payload;
            let value = i16::from_le_bytes([p[1], p[2]]);
            (p[0], value, *p.last().unwrap())
        })
        .collect()
}

fn link_events(signals: &Receiver<Signal>) -> Vec<&'static str> {
    let mut seen = Vec::new();
    while let Ok(signal) = signals.try_recv() {
        match signal {
            Signal::Disconnected => seen.push("disconnected"),
            Signal::Reconnected(_) => seen.push("reconnected"),
            Signal::SessionReset => seen.push("session reset"),
            _ => {}
        }
    }
    seen
}

fn until(mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !done() {
        assert!(Instant::now() < deadline, "gave up waiting");
        thread::sleep(Duration::from_millis(5));
    }
}

// the snapshot

#[test]
fn a_protocol_9_box_gives_a_snapshot_with_its_firmware_mac_and_buttons() {
    let (_backend, snapshot, _poller, _signals) = backend(&mouse(5, true));
    assert_eq!(snapshot.info.family, Family::Medius);
    assert_eq!(
        snapshot.info.firmware,
        Some(Firmware::Version {
            major: 3,
            minor: 4,
            patch: 5
        })
    );
    assert_eq!(snapshot.info.id, Some(StableId::Mac(MAC)));
    assert_eq!(snapshot.info.model.as_deref(), Some("Medius-test"));
    assert_eq!(
        snapshot.capabilities.buttons(),
        &[
            Button::Left,
            Button::Right,
            Button::Middle,
            Button::Side1,
            Button::Side2
        ]
    );
}

#[test]
fn a_clone_with_three_buttons_offers_no_side_buttons() {
    let mock = mouse(3, true);
    let (device, _poller) = device(&mock);
    mock.clear_recorded();
    assert!(matches!(
        device.press(Button::Side1),
        Err(Error::InvalidArgument { .. })
    ));
    assert!(injects(&mock).is_empty());
}

#[test]
fn the_last_declared_button_works_and_the_next_one_up_is_refused() {
    let mock = mouse(7, true);
    let (device, _poller) = device(&mock);
    mock.clear_recorded();
    device.press(Button::Extra(6)).unwrap();
    assert!(matches!(
        device.press(Button::Extra(7)),
        Err(Error::InvalidArgument { .. })
    ));
    assert_eq!(injects(&mock), vec![(6, 1)]);
}

#[test]
fn a_mouse_without_a_wheel_says_wheel_is_unsupported() {
    let (device, _poller) = device(&mouse(5, false));
    assert!(matches!(
        device.wheel(1),
        Err(Error::Unsupported {
            action: Action::Wheel,
            ..
        })
    ));
}

#[test]
fn a_box_on_another_protocol_is_told_which_side_to_update() {
    let old = mouse(5, true).with_version(Version {
        proto_ver: 4,
        fw_major: 3,
        fw_minor: 1,
        fw_patch: 0,
        mac: MAC,
        name: "old".into(),
    });
    let error = medius::Device::open_mock(old).unwrap_err();
    assert!(matches!(
        open_failure("COM9".as_ref(), error),
        Error::FirmwareTooOld {
            found: Firmware::Protocol(4),
            needed: Firmware::Protocol(9),
            ..
        }
    ));
    let new = mouse(5, true).with_version(Version {
        proto_ver: 10,
        fw_major: 4,
        fw_minor: 0,
        fw_patch: 0,
        mac: MAC,
        name: "new".into(),
    });
    let error = medius::Device::open_mock(new).unwrap_err();
    assert!(matches!(
        open_failure("COM9".as_ref(), error),
        Error::LibraryTooOld {
            found: Firmware::Protocol(10),
            ..
        }
    ));
}

// output

#[test]
fn each_command_reaches_the_box_as_its_own_frame() {
    let mock = mouse(5, true);
    let (device, _poller) = device(&mock);
    mock.clear_recorded();
    device.move_by(12, -7).unwrap();
    device.move_now(-2, 3).unwrap();
    device.wheel(-1).unwrap();
    device.discard_pending_moves().unwrap();
    // the MOVE flags are NOW 0x01 and DISCARD 0x04, like in the medius crate's source
    assert_eq!(
        moves(&mock),
        vec![(0, 12, 0), (0, -2, 0x01), (1, -1, 0), (0, 0, 0x04)]
    );
    device.press(Button::Side2).unwrap();
    device.release(Button::Side2).unwrap();
    // the INJECT actions are 1 for a press and 0 for a soft release, like in the medius crate's source
    assert_eq!(injects(&mock), vec![(4, 1), (4, 0)]);
}

#[test]
fn release_all_releases_each_button_and_never_resets() {
    let mock = mouse(7, true);
    let (backend, _snapshot, _poller, _signals) = backend(&mock);
    mock.clear_recorded();
    backend
        .release_all(&[
            crate::backend::Usage::Button(Button::Left),
            crate::backend::Usage::Button(Button::Extra(5)),
        ])
        .unwrap();
    assert_eq!(injects(&mock), vec![(0, 0), (5, 0)]);
    assert!(!mock.saw(FrameType::Reset));
}

#[test]
fn a_click_on_medius_is_composed_on_the_host() {
    let mock = mouse(5, true);
    let (device, _poller) = device(&mock);
    assert_eq!(
        device.snapshot().capabilities.get(Action::Click).execution,
        Execution::Host
    );
    mock.clear_recorded();
    device.click(Button::Left).unwrap();
    assert_eq!(injects(&mock), vec![(0, 1), (0, 0)]);
}

// the worker

#[test]
fn a_held_worker_makes_a_send_give_up_in_time_and_take_the_link_down() {
    let mock = mouse(5, true);
    let (backend, _snapshot, _poller, signals) = backend(&mock);
    let held = backend
        .shared()
        .hold
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let started = Instant::now();
    assert!(matches!(
        backend.send(Command::Move(1, 1)),
        Err(Error::Disconnected)
    ));
    assert!(started.elapsed() < WRITE_DEADLINE * 2);
    drop(held);
    assert_eq!(link_events(&signals), vec!["disconnected"]);
}

#[test]
fn a_press_that_gave_up_is_skipped_but_a_release_that_gave_up_still_goes_out() {
    let mock = mouse(5, true);
    let (backend, _snapshot, _poller, _signals) = backend(&mock);
    let backend = Arc::new(backend);
    mock.clear_recorded();
    let held = backend
        .shared()
        .hold
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    assert!(backend.send(Command::Press(Button::Left)).is_err());
    let releaser = thread::spawn({
        let backend = backend.clone();
        move || backend.send(Command::Release(Button::Left))
    });
    assert!(releaser.join().unwrap().is_err());
    drop(held);
    until(|| !injects(&mock).is_empty());
    assert_eq!(injects(&mock), vec![(0, 0)]);
}

#[test]
fn while_down_a_move_fails_at_once_and_a_release_still_reaches_the_crate() {
    let mock = mouse(5, true);
    let (backend, _snapshot, _poller, _signals) = backend(&mock);
    backend.shared().go_down();
    mock.clear_recorded();
    let started = Instant::now();
    assert!(matches!(
        backend.send(Command::Move(3, 3)),
        Err(Error::Disconnected)
    ));
    // at once means well under the deadline a send that waited would take
    assert!(started.elapsed() < WRITE_DEADLINE / 2);
    let _ = backend.send(Command::Release(Button::Right));
    assert!(moves(&mock).is_empty());
    assert_eq!(injects(&mock), vec![(1, 0)]);
}

// input

#[test]
fn motion_arrives_with_box_time_from_the_host_chip() {
    let mock = mouse(5, true);
    let (device, _poller) = device(&mock);
    let events = device.events();
    until(|| mock.saw(FrameType::Catch));
    mock.push_motion(2, 123_456, -2, 3, 1, 0);
    match events.recv_timeout(Duration::from_secs(2)).unwrap() {
        Event::Motion {
            dx, dy, wheel, at, ..
        } => {
            assert_eq!((dx, dy, wheel), (-2, 3, 1));
            let box_time = at.box_time.unwrap();
            assert_eq!(box_time.micros, 123_456);
            assert_eq!(box_time.clock, Clock::MediusHostChip);
        }
        other => panic!("expected motion, got {other:?}"),
    }
}

#[test]
fn a_physical_button_arrives_as_a_press_then_a_release() {
    let mock = mouse(5, true);
    let (device, _poller) = device(&mock);
    let events = device.events();
    until(|| mock.saw(FrameType::Catch));
    let left = Usage::new(Class::Button, 0);
    mock.push_usages(1, 10, Class::Button, Direction::PRESS, &[left]);
    mock.push_usages(2, 20, Class::Button, Direction::RELEASE, &[]);
    let mut seen = Vec::new();
    while seen.len() < 2 {
        if let Ok(Event::Button {
            button, pressed, ..
        }) = events.recv_timeout(Duration::from_secs(2))
        {
            seen.push((button, pressed));
        }
    }
    assert_eq!(seen, vec![(Button::Left, true), (Button::Left, false)]);
}

#[test]
fn input_streams_start_with_the_first_receiver_and_stop_with_the_last() {
    let mock = mouse(5, true);
    let (device, _poller) = device(&mock);
    mock.clear_recorded();
    let events = device.events();
    until(|| mock.saw(FrameType::Catch));
    mock.clear_recorded();
    drop(events);
    until(|| mock.saw(FrameType::Catch));
}

// the poller

#[test]
fn a_box_session_release_gives_session_reset_and_the_crate_presses_held_buttons_again() {
    let mock = mouse(5, true);
    let (device, mut poller) = device(&mock);
    device.press(Button::Left).unwrap();
    mock.clear_recorded();
    mock.detach(true);
    poller.tick();
    device.settle();
    assert_eq!(
        device.held(),
        HashSet::from([crate::backend::Usage::Button(Button::Left)])
    );
    // this is the crate's own restore, and it's the reason the core leaves held buttons alone on Medius. if a
    // crate update drops it this fails, and then the special case in the core has to go too
    until(|| injects(&mock).contains(&(0, 1)));
    assert!(!injects(&mock).contains(&(0, 0)));
}

#[test]
fn a_mouse_plugged_in_after_the_box_gives_a_fresh_snapshot() {
    let mock = mouse(5, true);
    let (_backend, _snapshot, mut poller, signals) = backend(&mock);
    // the HEALTH bits are 0x01 for the link, 0x02 for a mouse attached and 0x04 for the clone configured, like
    // in the medius crate's source
    mock.set_health(Health::from_flags(0x07));
    for _ in 0..4 {
        poller.tick();
    }
    assert_eq!(link_events(&signals), vec!["reconnected"]);
}

#[test]
fn a_query_that_fails_and_then_answers_brings_the_link_back() {
    let mock = mouse(5, true);
    let (backend, _snapshot, mut poller, signals) = backend(&mock);
    backend.shared().go_down();
    poller.tick();
    assert_eq!(link_events(&signals), vec!["disconnected", "reconnected"]);
    assert!(backend.shared().is_up());
}

#[test]
fn after_a_poller_reconnect_held_buttons_stay_held_and_new_presses_work() {
    let mock = mouse(5, true);
    let (device, mut poller) = device(&mock);
    device.press(Button::Middle).unwrap();
    if let Some(backend) = device_backend(&device) {
        backend.shared().go_down();
    }
    mock.clear_recorded();
    poller.tick();
    device.settle();
    assert_eq!(
        device.held(),
        HashSet::from([crate::backend::Usage::Button(Button::Middle)])
    );
    assert!(!injects(&mock).contains(&(2, 0)));
    device.press(Button::Right).unwrap();
    assert_eq!(
        device.held(),
        HashSet::from([
            crate::backend::Usage::Button(Button::Middle),
            crate::backend::Usage::Button(Button::Right),
        ])
    );
}

#[test]
fn a_box_reboot_is_one_disconnect_and_reconnect_never_a_session_reset() {
    let mock = mouse(5, true);
    let (backend, _snapshot, mut poller, signals) = backend(&mock);
    // a session release happens first, so the reboot can take the session back to 0
    backend.send(Command::Press(Button::Left)).unwrap();
    mock.detach(true);
    poller.tick();
    assert_eq!(link_events(&signals), vec!["session reset"]);
    mock.restart();
    poller.tick();
    for _ in 0..8 {
        thread::sleep(Duration::from_millis(50));
        poller.tick();
    }
    // the mock fills its CAPS in again a few ticks after the restart, which publishes the snapshot once more
    let events = link_events(&signals);
    assert_eq!(events[..2], ["disconnected", "reconnected"]);
    assert!(!events.contains(&"session reset"), "{events:?}");
}

#[test]
fn lost_events_are_counted_per_subscription() {
    let mock = mouse(5, true);
    let (backend, _snapshot, mut poller, signals) = backend(&mock);
    let host_losses = |signals: &Receiver<Signal>| -> u64 {
        let mut lost = 0;
        while let Ok(signal) = signals.recv_timeout(Duration::from_millis(300)) {
            if let Signal::Dropped(count, crate::event::DropSource::Host, Some(_)) = signal {
                lost += count;
            }
        }
        lost
    };
    for round in 0..2 {
        // when a lot of tests run in parallel a subscribe can miss the worker deadline, which takes the
        // adapter down like a real stall would, so bring it back and ask again
        until(|| {
            if !backend.shared().is_up() {
                poller.tick();
            }
            backend
                .set_streams(crate::backend::InputKinds {
                    buttons: false,
                    motion: true,
                    ..crate::backend::InputKinds::default()
                })
                .is_ok()
        });
        // holding the stream lock makes the pump wait, so the crate's 256 event queue fills up and drops the rest
        let paused = backend.shared().slots[1]
            .stream
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        for seq in 0..300u32 {
            mock.push_motion((seq % 250) as u8, seq, 1, 0, 0, 0);
        }
        thread::sleep(Duration::from_millis(200));
        drop(paused);
        assert!(host_losses(&signals) > 0, "round {round}: no loss reported");
    }
}

#[test]
fn the_backend_refuses_what_the_box_can_no_longer_do_but_always_releases() {
    let mock = mouse(5, false);
    let (backend, _snapshot, _poller, _signals) = backend(&mock);
    mock.clear_recorded();
    assert!(matches!(
        backend.send(Command::Wheel(1)),
        Err(Error::Unsupported { .. })
    ));
    assert!(matches!(
        backend.send(Command::Press(Button::Extra(9))),
        Err(Error::Unsupported { .. })
    ));
    backend.send(Command::Release(Button::Extra(9))).unwrap();
    assert!(moves(&mock).is_empty());
    assert_eq!(injects(&mock), vec![(9, 0)]);
}

/// the crate's own default five button mouse, because its restart recovery needs the caps it ships with
fn plain_box() -> MockBox {
    MockBox::new()
}

fn wait_for_restart(backend: &MediusBackend) {
    until(|| {
        backend
            .shared()
            .device()
            .is_some_and(|device| device.counters().restarts >= 1)
    });
}

#[test]
fn a_restart_the_crate_counts_brings_the_link_down_and_back() {
    let mock = plain_box();
    let (backend, _snapshot, mut poller, signals) = backend(&mock);
    mock.restart();
    wait_for_restart(&backend);
    poller.tick();
    assert_eq!(link_events(&signals), vec!["disconnected", "reconnected"]);
}

#[test]
fn a_restart_and_a_session_change_on_one_tick_are_one_reconnect() {
    let mock = plain_box();
    let (backend, _snapshot, mut poller, signals) = backend(&mock);
    backend.send(Command::Press(Button::Left)).unwrap();
    mock.detach(true);
    mock.restart();
    wait_for_restart(&backend);
    poller.tick();
    poller.tick();
    poller.tick();
    assert_eq!(link_events(&signals), vec!["disconnected", "reconnected"]);
}

#[test]
fn an_app_press_right_after_reconnected_goes_through() {
    let mock = mouse(5, true);
    let (device, mut poller) = device(&mock);
    let device = Arc::new(device);
    let events = device.events();
    let presser = thread::spawn({
        let device = device.clone();
        move || loop {
            match events.recv_timeout(Duration::from_secs(5)) {
                Ok(Event::Reconnected) => return device.press(Button::Right),
                Ok(_) => {}
                Err(_) => return Err(Error::Disconnected),
            }
        }
    });
    if let Some(backend) = device_backend(&device) {
        backend.shared().go_down();
    }
    poller.tick();
    presser.join().unwrap().unwrap();
}

fn catch_frames(mock: &MockBox) -> usize {
    mock.recorded_frames()
        .into_iter()
        .filter(|frame| frame.ty == FrameType::Catch)
        .count()
}

#[test]
fn the_same_kinds_again_leave_the_subscriptions_alone() {
    let mock = mouse(5, true);
    let (backend, _snapshot, _poller, _signals) = backend(&mock);
    let motion = crate::backend::InputKinds {
        motion: true,
        ..crate::backend::InputKinds::default()
    };
    backend.set_streams(motion).unwrap();
    mock.clear_recorded();
    backend.set_streams(motion).unwrap();
    assert_eq!(catch_frames(&mock), 0);
}

#[test]
fn adding_a_kind_leaves_the_other_kinds_subscription_alone() {
    let mock = mouse(5, true);
    let (backend, _snapshot, _poller, _signals) = backend(&mock);
    let buttons = crate::backend::InputKinds {
        buttons: true,
        ..crate::backend::InputKinds::default()
    };
    backend.set_streams(buttons).unwrap();
    mock.clear_recorded();
    backend
        .set_streams(crate::backend::InputKinds {
            motion: true,
            ..buttons
        })
        .unwrap();
    // only the axes class gets subscribed, nothing is dropped
    let classes: Vec<(u8, u8)> = mock
        .recorded_frames()
        .into_iter()
        .filter(|frame| frame.ty == FrameType::Catch)
        .map(|frame| (frame.payload[0], frame.payload[4]))
        .collect();
    assert!(
        classes
            .iter()
            .all(|&(class, state)| class == 3 && state == 1),
        "{classes:?}"
    );
}

#[test]
fn a_port_that_isnt_there_is_not_found_not_the_wrong_family() {
    let missing = if cfg!(windows) {
        "COM250"
    } else {
        "/dev/ttyANYBOX250"
    };
    let error = crate::connect::Connect::port(missing)
        .family(Family::Medius)
        .open()
        .unwrap_err();
    assert!(matches!(error, Error::NotFound { .. }), "{error}");
}

fn device_backend(device: &Device) -> Option<&MediusBackend> {
    device.backend_for_tests().downcast_ref::<MediusBackend>()
}

#[test]
fn device_medius_hands_out_the_crate_device_of_this_box() {
    let (device, _poller) = device(&mouse(5, true));
    let crate_device = device.medius().unwrap();
    assert_eq!(crate_device.query_version().unwrap().mac, MAC);
}

// the probe for old firmware

/// a RESP(VERSION) built by hand from the frame layout in the medius crate's protocol module, not
/// by the code under test
fn resp_version(major: u8, minor: u8, patch: u8) -> Vec<u8> {
    let mut body = vec![0x06, 0x01, 11, 0x00];
    body.extend_from_slice(&[0x00, 6, major, minor, patch]);
    body.extend_from_slice(&MAC);
    let mut out = vec![0xA5];
    out.extend_from_slice(&body);
    out.extend_from_slice(&frame::crc16(&body).to_le_bytes());
    out
}

fn probed(responder: impl FnMut(&[u8]) -> Vec<Vec<u8>> + Send + 'static) -> Error {
    let system = FakeSystem::default();
    system.plug("COM9", None, FakeBox::new(responder));
    let port = system.open("COM9".as_ref(), super::OLD_BAUD).unwrap();
    probe_old_firmware(&*port, "COM9".as_ref())
}

#[test]
fn the_crc_matches_the_catalogue_check_value() {
    assert_eq!(frame::crc16(b"123456789"), 0x29B1);
}

#[test]
fn an_old_medius_answering_at_4_mbaud_is_told_to_update() {
    let error = probed(|_| vec![resp_version(3, 3, 1)]);
    assert!(matches!(
        error,
        Error::FirmwareTooOld {
            found: Firmware::Version {
                major: 3,
                minor: 3,
                patch: 1
            },
            ..
        }
    ));
}

#[test]
fn nothing_answering_at_4_mbaud_is_not_found() {
    assert!(matches!(probed(|_| Vec::new()), Error::NotFound { .. }));
}

#[test]
fn a_box_that_answers_with_something_else_at_4_mbaud_is_not_found() {
    let km_text = probed(|_| vec![b"km.MAKCU\r\n>>> ".to_vec()]);
    assert!(matches!(km_text, Error::NotFound { .. }));
    let mut broken = resp_version(3, 3, 1);
    let last = broken.len() - 1;
    broken[last] ^= 0xFF;
    assert!(matches!(
        probed(move |_| vec![broken.clone()]),
        Error::NotFound { .. }
    ));
}

/// these need a Medius box. run them with
/// `ANYBOX_PORT=COM18 cargo test medius::tests::hardware -- --ignored --nocapture --test-threads 1`, with your
/// box's port in place of COM18. the ones marked "with you" need you to do something with the mouse or the
/// cable while they run
mod hardware {
    use std::time::{Duration, Instant};

    use crate::capability::{Family, StableId};
    use crate::connect::Connect;
    use crate::error::Error;
    use crate::event::{Event, RecvTimeoutError};

    fn port() -> String {
        std::env::var("ANYBOX_PORT").expect("set ANYBOX_PORT to the port your box is on")
    }

    fn open() -> crate::device::Device {
        Connect::port(port())
            .family(Family::Medius)
            .open()
            .unwrap_or_else(|error| panic!("{error}"))
    }

    #[test]
    #[ignore]
    fn a_real_box_opens_with_its_identity() {
        let device = open();
        let snapshot = device.snapshot();
        println!("{:?}", snapshot.info);
        println!("buttons {:?}", snapshot.capabilities.buttons());
        let crate_device = device.medius().unwrap();
        println!("caps {:?}", crate_device.caps());
        println!("cloned {:?}", crate_device.device_info());
        assert_eq!(snapshot.info.family, Family::Medius);
        assert!(matches!(snapshot.info.id, Some(StableId::Mac(_))));
    }

    #[test]
    #[ignore]
    fn a_second_open_of_the_same_box_is_busy() {
        let _first = open();
        let second = Connect::port(port()).family(Family::Medius).open();
        assert!(
            matches!(second, Err(Error::PortBusy { .. })),
            "{:?}",
            second.err()
        );
    }

    /// checks that a dropped device has really let go of the port by the time drop returns
    #[test]
    #[ignore]
    fn a_reopen_right_after_a_drop_works() {
        for round in 0..10 {
            let device = open();
            device
                .move_by(0, 0)
                .unwrap_or_else(|error| panic!("round {round}: {error}"));
            drop(device);
        }
    }

    /// moves the cursor 40 counts right and back and nothing else, so you can see that injection works
    #[test]
    #[ignore]
    fn the_cursor_moves_right_and_back() {
        let device = open();
        for _ in 0..20 {
            device.move_by(2, 0).unwrap();
            std::thread::sleep(Duration::from_millis(10));
        }
        for _ in 0..20 {
            device.move_by(-2, 0).unwrap();
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// with you. when it says so, move the mouse and click a mouse button within 15 seconds
    #[test]
    #[ignore]
    fn physical_motion_and_clicks_arrive_with_box_time() {
        let device = open();
        let events = device.events();
        println!("move the mouse and click a button now");
        let until = Instant::now() + Duration::from_secs(15);
        let (mut motion, mut button) = (false, false);
        while !(motion && button) && Instant::now() < until {
            match events.recv_timeout(Duration::from_millis(200)) {
                Ok(Event::Motion { at, .. }) => motion |= at.box_time.is_some(),
                Ok(Event::Button { at, .. }) => button |= at.box_time.is_some(),
                Ok(_) | Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Closed) => break,
            }
        }
        assert!(motion, "no motion with box time");
        assert!(button, "no button with box time");
    }

    /// with you. you have 3 seconds to put the cursor on an empty spot of the desktop, then a right click
    /// timed on the host opens the context menu and the wheel scrolls three notches down and back up. press
    /// escape once it's done
    #[test]
    #[ignore]
    fn a_right_click_and_a_scroll_reach_the_pc() {
        let device = open();
        std::thread::sleep(Duration::from_secs(3));
        let started = Instant::now();
        device.click(crate::capability::Button::Right).unwrap();
        println!("right click took {:?} on the host", started.elapsed());
        for notch in [-1, -1, -1, 1, 1, 1] {
            device.wheel(notch).unwrap();
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// with you. you have 2 seconds to put the cursor on an empty spot of the desktop, then the library holds
    /// right and lets go when the device is dropped, so the context menu opens on the release about a second
    /// after the press. press escape once it's done
    #[test]
    #[ignore]
    fn a_held_button_is_let_go_when_the_device_is_dropped() {
        let device = open();
        std::thread::sleep(Duration::from_secs(2));
        device.press(crate::capability::Button::Right).unwrap();
        std::thread::sleep(Duration::from_secs(1));
        drop(device);
        println!("dropped with the right button held");
    }

    /// sends the MAKCU probe (text and binary, at both its baud rates) to a Medius, which then still has to
    /// open normally with the same identity
    #[test]
    #[ignore]
    fn a_makcu_probe_leaves_a_medius_alone() {
        use crate::transport::System;
        let before = open().snapshot().info.clone();
        for baud in [4_000_000, 115_200] {
            let port = crate::transport::os::OsSystem
                .open(std::path::Path::new(&port()), baud)
                .unwrap();
            for bytes in [
                &b"\r\nkm.version()\r\n"[..],
                &[0xDE, 0xAD, 0, 0, 0x04],
                &[0xDE, 0xAD, 0, 0, 0x02],
            ] {
                port.write_all(bytes).unwrap();
                std::thread::sleep(Duration::from_millis(300));
            }
            let mut chunk = [0u8; 256];
            let got = port.read(&mut chunk).unwrap_or(0);
            println!("at {baud}: {got} bytes back {:02X?}", &chunk[..got]);
            port.retire();
        }
        let started = Instant::now();
        let after = open().snapshot().info.clone();
        println!("opened again in {:?}", started.elapsed());
        assert_eq!(before, after);
    }

    /// with you. when it says so, unplug the MOUSE from the box (not the control cable) within 15 seconds, then
    /// plug it back in
    #[test]
    #[ignore]
    fn unplugging_the_mouse_resets_the_box_session() {
        let device = open();
        let events = device.events();
        device.move_by(1, 0).unwrap();
        device.move_by(-1, 0).unwrap();
        println!("unplug the mouse from the box now, then plug it back in");
        let started = Instant::now();
        let mut seen = Vec::new();
        while started.elapsed() < Duration::from_secs(30) && seen.is_empty() {
            match events.recv_timeout(Duration::from_millis(200)) {
                Ok(Event::SessionReset) => seen.push("session reset"),
                Ok(Event::Reconnected) => seen.push("reconnected"),
                Ok(Event::Disconnected) => seen.push("disconnected"),
                _ => {}
            }
        }
        println!("saw {seen:?} after {:?}", started.elapsed());
        assert!(!seen.is_empty(), "the box reported nothing");
    }

    /// with you. when it says so, unplug the box's control cable (USB2) within 15 seconds, then plug it back
    /// in. the library holds the middle button the whole time, so a USB capture of the box shows how long the
    /// press the crate replays stays live before the release that comes after the reconnect
    #[test]
    #[ignore]
    fn unplugging_the_control_cable_disconnects_and_reconnects() {
        let device = open();
        let events = device.events();
        device.press(crate::capability::Button::Middle).unwrap();
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
        // a capture of the box shows if the middle button stays held after the reconnect, then the drop lets go
        std::thread::sleep(Duration::from_secs(3));
        println!("dropping at {:.6}", unix_now());
    }

    /// wall clock seconds, because that's the clock a USBPcap capture stamps its packets with
    fn unix_now() -> f64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f64()
    }

    #[cfg(windows)]
    #[allow(unsafe_code)]
    mod beeps {
        #[link(name = "user32")]
        unsafe extern "system" {
            fn GetCursorPos(point: *mut [i32; 2]) -> i32;
        }

        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn Beep(frequency: u32, millis: u32) -> i32;
        }

        /// a high beep means start, a low one means do the next thing and two low ones mean it's done. it blocks
        /// while it sounds
        pub(super) fn beep(high: bool) {
            // safety: Beep takes two plain numbers
            unsafe {
                Beep(if high { 1500 } else { 500 }, 400);
            }
        }

        pub(super) fn done() {
            beep(false);
            std::thread::sleep(std::time::Duration::from_millis(150));
            beep(false);
        }

        pub(super) fn cursor() -> [i32; 2] {
            let mut point = [0; 2];
            // safety: GetCursorPos writes one POINT, two i32
            assert_ne!(unsafe { GetCursorPos(&mut point) }, 0);
            point
        }
    }

    /// waits for the first press of `wanted` on `events` within `window` and skips every other event
    #[cfg(windows)]
    fn pressed(
        events: &crate::event::Events,
        wanted: crate::capability::Button,
        window: Duration,
    ) -> bool {
        // drain first, otherwise an extra click from an earlier step would pass this one without a fresh click
        while events.try_recv().is_ok() {}
        let deadline = Instant::now() + window;
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            if let Ok(Event::Button {
                button,
                pressed: true,
                ..
            }) = events.recv_timeout(left)
                && button == wanted
            {
                return true;
            }
        }
        false
    }

    /// with you, with the mouse plugged into the box. it tries the features only Medius has through medius()
    /// while anybox's events keep running, and beeps each time you need to do the next step
    #[cfg(windows)]
    #[test]
    #[ignore]
    fn medius_only_features_next_to_anybox() {
        use crate::capability::Button;
        use beeps::{beep, cursor, done};
        use medius::{CatchFilter, Class, ClipBuilder, LedMode, LedTarget};

        let device = open();
        let events = device.events();
        let box_device = device.medius().unwrap();
        let window = Duration::from_secs(30);

        println!("1. the LEDs should blink for 3 seconds");
        beep(true);
        box_device
            .led(LedTarget::Both, LedMode::Blink, 255)
            .unwrap();
        std::thread::sleep(Duration::from_secs(3));
        box_device.led(LedTarget::Both, LedMode::Auto, 0).unwrap();

        println!(
            "2. left click once on the desktop. the box turns it into a right click, so the context menu should open"
        );
        box_device
            .transform_remap(medius::Button::LEFT, medius::Button::RIGHT)
            .unwrap();
        beep(false);
        let left_seen = pressed(&events, Button::Left, window);
        box_device.clear_transforms().unwrap();
        println!("   events() saw a Left press: {left_seen}");

        println!("3. take your hand off the mouse, the cursor draws a square by itself");
        beep(false);
        std::thread::sleep(Duration::from_secs(2));
        let start = cursor();
        let mut square = ClipBuilder::new();
        for (dx, dy) in [(2, 0), (0, 2), (-2, 0), (0, -2)] {
            for _ in 0..100 {
                square.move_by(dx, dy);
            }
        }
        let clip = box_device.clip();
        clip.append(&square).unwrap();
        clip.start().unwrap();
        let (mut far_x, mut far_y) = (0, 0);
        let until = Instant::now() + Duration::from_secs(2);
        while Instant::now() < until {
            let [x, y] = cursor();
            far_x = far_x.max(x - start[0]);
            far_y = far_y.max(y - start[1]);
            std::thread::sleep(Duration::from_millis(5));
        }
        let end = cursor();
        let _ = clip.clear();
        println!(
            "   furthest right {far_x}, furthest down {far_y}, back at {:?} from {start:?}",
            end
        );

        println!("4. left click once, the app's subscription and events() should both see it");
        let app = box_device
            .input_events([CatchFilter::watch_class(Class::Button)])
            .unwrap();
        beep(false);
        let anybox_seen = pressed(&events, Button::Left, window);
        let app_seen = std::iter::from_fn(|| app.try_recv()).any(|event| event.input.is_press());
        drop(app);
        println!("   events() {anybox_seen}, the app's subscription {app_seen}");

        println!("5. the box gets reset through medius(), then left click once");
        box_device.reset().unwrap();
        beep(false);
        let after_reset = pressed(&events, Button::Left, window);
        println!("   events() after the reset saw it: {after_reset}");

        let stats = box_device.query_stats().unwrap();
        println!("6. stats {stats:?}");

        println!(
            "7. medius() holds right, then everything gets dropped, so the menu should open about a second later"
        );
        box_device.press(medius::Button::RIGHT).unwrap();
        done();
        drop(events);
        drop(device);
        drop(box_device);
        std::thread::sleep(Duration::from_secs(3));
        assert!(left_seen && anybox_seen && app_seen && after_reset);
    }
}

// keyboard

fn keyboard(n_keys: u8, consumer: bool) -> MockBox {
    mouse(5, true).with_kbd_caps(KbdCaps {
        n_keys,
        has_consumer: consumer,
        ..KbdCaps::default()
    })
}

/// INJECT frames of one class the box saw, as (id, action)
fn injects_of(mock: &MockBox, class: u8) -> Vec<(u16, u8)> {
    mock.recorded_frames()
        .into_iter()
        .filter(|frame| frame.ty == FrameType::Inject && frame.payload[0] == class)
        .map(|frame| {
            (
                u16::from_le_bytes([frame.payload[1], frame.payload[2]]),
                frame.payload[3],
            )
        })
        .collect()
}

#[test]
fn a_clone_with_a_keyboard_offers_keys_with_its_slot_limit() {
    let (_backend, snapshot, _poller, _signals) = backend(&keyboard(6, false));
    let capabilities = &snapshot.capabilities;
    let press = capabilities.get(Action::KeyPress);
    assert_eq!(press.execution, Execution::Native);
    assert!(press.quirks.contains(&Quirk::MaxKeysHeld(6)));
    assert!(press.quirks.contains(&Quirk::KeepsHeld));
    assert_eq!(
        capabilities.get(Action::KeyRelease).execution,
        Execution::Native
    );
    assert!(!capabilities.get(Action::MediaPress).is_supported());
}

#[test]
fn an_nkro_clone_has_no_slot_limit() {
    for n_keys in [0xFF, 6] {
        let mock = mouse(5, true).with_kbd_caps(KbdCaps {
            n_keys,
            nkro: n_keys == 6,
            ..KbdCaps::default()
        });
        let (_backend, snapshot, _poller, _signals) = backend(&mock);
        let press = snapshot.capabilities.get(Action::KeyPress);
        assert!(
            !press
                .quirks
                .iter()
                .any(|quirk| matches!(quirk, Quirk::MaxKeysHeld(_))),
            "{n_keys}"
        );
    }
}

#[test]
fn a_clone_without_a_keyboard_says_so() {
    let (_backend, snapshot, _poller, _signals) = backend(&mouse(5, true));
    match &snapshot.capabilities.get(Action::KeyPress).execution {
        Execution::Unsupported { reason } => assert!(reason.contains("no keyboard"), "{reason}"),
        other => panic!("expected unsupported, got {other:?}"),
    }
}

#[test]
fn keys_and_media_keys_reach_the_box_as_inject_class_1_and_2() {
    let mock = keyboard(6, true);
    let (backend, _snapshot, _poller, _signals) = backend(&mock);
    mock.clear_recorded();
    backend.send(Command::KeyPress(Key::A)).unwrap();
    backend.send(Command::KeyRelease(Key::A)).unwrap();
    backend.send(Command::MediaPress(MediaKey(0x1234))).unwrap();
    backend
        .send(Command::MediaRelease(MediaKey(0x1234)))
        .unwrap();
    assert_eq!(injects_of(&mock, 1), vec![(0x04, 1), (0x04, 0)]);
    assert_eq!(injects_of(&mock, 2), vec![(0x1234, 1), (0x1234, 0)]);
}

#[test]
fn a_key_press_that_gave_up_is_skipped_but_its_release_still_goes_out() {
    let mock = keyboard(6, false);
    let (backend, _snapshot, _poller, _signals) = backend(&mock);
    let backend = Arc::new(backend);
    mock.clear_recorded();
    let held = backend
        .shared()
        .hold
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    assert!(backend.send(Command::KeyPress(Key::A)).is_err());
    let releaser = thread::spawn({
        let backend = backend.clone();
        move || backend.send(Command::KeyRelease(Key::A))
    });
    assert!(releaser.join().unwrap().is_err());
    drop(held);
    until(|| !injects_of(&mock, 1).is_empty());
    assert_eq!(injects_of(&mock, 1), vec![(0x04, 0)]);
}

#[test]
fn a_key_the_box_can_no_longer_do_is_refused_but_its_release_passes() {
    let mock = mouse(5, true);
    let (backend, _snapshot, _poller, _signals) = backend(&mock);
    mock.clear_recorded();
    assert!(matches!(
        backend.send(Command::KeyPress(Key::A)),
        Err(Error::Unsupported { .. })
    ));
    backend.send(Command::KeyRelease(Key::A)).unwrap();
    assert_eq!(injects_of(&mock, 1), vec![(0x04, 0)]);
}

#[test]
fn the_crate_presses_held_keys_again_after_a_session_release() {
    let mock = keyboard(6, false);
    let (device, mut poller) = device(&mock);
    device.press_key(Key::A).unwrap();
    mock.clear_recorded();
    mock.detach(true);
    poller.tick();
    device.settle();
    assert_eq!(
        device.held(),
        HashSet::from([crate::backend::Usage::Key(Key::A)])
    );
    until(|| injects_of(&mock, 1).contains(&(0x04, 1)));
    assert!(!injects_of(&mock, 1).contains(&(0x04, 0)));
}

#[test]
fn a_keyboard_plugged_in_after_the_box_gives_a_fresh_snapshot() {
    // the HEALTH bits are 0x07 for the link, mouse and clone, and 0x80 for a keyboard attached, like in the
    // medius crate's source
    let mock = mouse(5, true).with_health(Health::from_flags(0x07));
    let (_backend, _snapshot, mut poller, signals) = backend(&mock);
    mock.set_health(Health::from_flags(0x87));
    for _ in 0..4 {
        poller.tick();
    }
    assert_eq!(link_events(&signals), vec!["reconnected"]);
}

#[test]
fn a_tap_on_medius_is_composed_on_the_host() {
    let mock = keyboard(6, false);
    let (device, _poller) = device(&mock);
    assert_eq!(
        device.snapshot().capabilities.get(Action::KeyTap).execution,
        Execution::Host
    );
    mock.clear_recorded();
    device.tap_key(Key::B).unwrap();
    assert_eq!(injects_of(&mock, 1), vec![(0x05, 1), (0x05, 0)]);
}

#[test]
fn a_keyboard_switching_to_nkro_changes_the_snapshot_quietly() {
    // a GK61 does this by itself once more than 6 keys are down, we saw it on the hardware
    let mock = keyboard(6, false).with_health(Health::from_flags(0x87));
    let (_backend, _snapshot, mut poller, signals) = backend(&mock);
    let _ = mock.clone().with_kbd_caps(KbdCaps {
        n_keys: 232,
        has_report_id: true,
        ..KbdCaps::default()
    });
    for _ in 0..4 {
        poller.tick();
    }
    let signals: Vec<Signal> = signals.try_iter().collect();
    assert!(
        !signals
            .iter()
            .any(|signal| matches!(signal, Signal::Reconnected(_))),
        "{signals:?}"
    );
    let changed: Vec<_> = signals
        .iter()
        .filter_map(|signal| match signal {
            Signal::Changed(snapshot) => {
                Some(snapshot.capabilities.get(Action::KeyPress).quirks.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(changed.len(), 1);
    assert!(changed[0].contains(&Quirk::MaxKeysHeld(232)));
}

#[test]
fn a_keyboard_pulled_out_is_a_reconnect_not_a_quiet_change() {
    let mock = keyboard(6, false).with_health(Health::from_flags(0x87));
    let (_backend, _snapshot, mut poller, signals) = backend(&mock);
    let _ = mock.clone().with_kbd_caps(KbdCaps::default());
    mock.set_health(Health::from_flags(0x07));
    for _ in 0..4 {
        poller.tick();
    }
    assert_eq!(link_events(&signals), vec!["reconnected"]);
}

#[test]
fn release_all_lets_go_of_keys_and_media_keys_by_their_class() {
    let mock = keyboard(6, true);
    let (backend, _snapshot, _poller, _signals) = backend(&mock);
    mock.clear_recorded();
    backend
        .release_all(&[
            crate::backend::Usage::Key(Key::A),
            crate::backend::Usage::Media(MediaKey(0x1234)),
        ])
        .unwrap();
    assert_eq!(injects_of(&mock, 1), vec![(0x04, 0)]);
    assert_eq!(injects_of(&mock, 2), vec![(0x1234, 0)]);
}

/// the classes the box was asked to catch, from the CATCH frames it saw
fn caught_classes(mock: &MockBox) -> HashSet<u8> {
    mock.recorded_frames()
        .into_iter()
        .filter(|frame| frame.ty == FrameType::Catch && frame.payload[4] == 1)
        .map(|frame| frame.payload[0])
        .collect()
}

#[test]
fn keys_and_media_keys_are_only_caught_where_the_clone_has_them() {
    for (mock, keys, media) in [
        (mouse(5, true), false, false),
        (keyboard(6, false), true, false),
        (keyboard(6, true), true, true),
    ] {
        let (device, _poller) = device(&mock);
        let _events = device.events();
        until(|| caught_classes(&mock).contains(&0));
        let classes = caught_classes(&mock);
        assert_eq!(classes.contains(&1), keys, "{classes:?}");
        assert_eq!(classes.contains(&2), media, "{classes:?}");
    }
}

#[test]
fn physical_keys_arrive_as_key_events_with_box_time() {
    let mock = keyboard(6, true);
    let (device, _poller) = device(&mock);
    let events = device.events();
    until(|| caught_classes(&mock).contains(&1));
    let a = Usage::new(Class::Key, 0x04);
    let shift = Usage::new(Class::Key, 0xE1);
    let mute = Usage::new(Class::Media, 0xE2);
    mock.push_usages(1, 100, Class::Key, Direction::PRESS, &[shift]);
    mock.push_usages(2, 200, Class::Key, Direction::PRESS, &[shift, a]);
    mock.push_usages(3, 300, Class::Key, Direction::RELEASE, &[]);
    mock.push_usages(4, 400, Class::Media, Direction::PRESS, &[mute]);
    // keys and media keys are separate subscriptions, so only the order within each kind is fixed
    let mut seen = Vec::new();
    let mut media = Vec::new();
    while seen.len() < 4 || media.is_empty() {
        match events.recv_timeout(Duration::from_secs(2)) {
            Ok(Event::Key {
                key, pressed, at, ..
            }) => {
                assert!(at.box_time.is_some());
                seen.push(format!("{key} {pressed}"));
            }
            Ok(Event::Media { key, pressed, .. }) => media.push(format!("{key} {pressed}")),
            Ok(_) => {}
            Err(error) => panic!("{error} after {seen:?}"),
        }
    }
    seen.extend(media);
    assert_eq!(seen[..2], ["key 0xE1 true", "key 0x04 true"]);
    // both releases go in one snapshot and the crate sends them in its own order
    let mut released = seen[2..4].to_vec();
    released.sort();
    assert_eq!(released, ["key 0x04 false", "key 0xE1 false"]);
    assert_eq!(seen[4], "media key 0x00E2 true");
}

#[test]
fn codes_that_are_no_key_never_come_out_as_key_events() {
    let mock = keyboard(6, false);
    let (device, _poller) = device(&mock);
    let events = device.events();
    until(|| caught_classes(&mock).contains(&1));
    let rollover = Usage::new(Class::Key, 0x01);
    let wide = Usage::new(Class::Key, 0x0104);
    let b = Usage::new(Class::Key, 0x05);
    mock.push_usages(1, 100, Class::Key, Direction::PRESS, &[rollover]);
    mock.push_usages(2, 200, Class::Key, Direction::PRESS, &[rollover, wide]);
    mock.push_usages(3, 300, Class::Key, Direction::PRESS, &[rollover, wide, b]);
    loop {
        match events.recv_timeout(Duration::from_secs(2)) {
            Ok(Event::Key { key, pressed, .. }) => {
                assert_eq!((key, pressed), (Key(0x05), true));
                break;
            }
            Ok(_) => {}
            Err(error) => panic!("{error}"),
        }
    }
}

#[test]
fn a_keyboard_plugged_in_later_gets_its_keys_caught_after_the_reconnect() {
    let mock = mouse(5, true).with_health(Health::from_flags(0x07));
    let (device, mut poller) = device(&mock);
    let _events = device.events();
    until(|| caught_classes(&mock).contains(&0));
    assert!(!caught_classes(&mock).contains(&1));
    let _ = mock.clone().with_kbd_caps(KbdCaps {
        n_keys: 6,
        ..KbdCaps::default()
    });
    mock.set_health(Health::from_flags(0x87));
    for _ in 0..4 {
        poller.tick();
    }
    until(|| caught_classes(&mock).contains(&1));
}

#[test]
fn a_clone_that_gains_media_keys_is_a_reconnect_even_with_health_unchanged() {
    let mock = keyboard(6, false).with_health(Health::from_flags(0x87));
    let (_backend, _snapshot, mut poller, signals) = backend(&mock);
    let _ = mock.clone().with_kbd_caps(KbdCaps {
        n_keys: 6,
        has_consumer: true,
        ..KbdCaps::default()
    });
    for _ in 0..4 {
        poller.tick();
    }
    assert_eq!(link_events(&signals), vec!["reconnected"]);
}

#[test]
fn a_quiet_change_is_said_once() {
    let mock = keyboard(6, false).with_health(Health::from_flags(0x87));
    let (_backend, _snapshot, mut poller, signals) = backend(&mock);
    let _ = mock.clone().with_kbd_caps(KbdCaps {
        n_keys: 232,
        ..KbdCaps::default()
    });
    for _ in 0..12 {
        poller.tick();
    }
    let changed = signals
        .try_iter()
        .filter(|signal| matches!(signal, Signal::Changed(_)))
        .count();
    assert_eq!(changed, 1);
}

#[test]
fn media_usage_zero_never_comes_out_as_an_event() {
    let mock = keyboard(6, true);
    let (device, _poller) = device(&mock);
    let events = device.events();
    until(|| caught_classes(&mock).contains(&2));
    mock.push_usages(
        1,
        100,
        Class::Media,
        Direction::PRESS,
        &[Usage::new(Class::Media, 0)],
    );
    mock.push_usages(
        2,
        200,
        Class::Media,
        Direction::PRESS,
        &[Usage::new(Class::Media, 0), Usage::new(Class::Media, 0xE9)],
    );
    loop {
        match events.recv_timeout(Duration::from_secs(2)) {
            Ok(Event::Media { key, .. }) => {
                assert_eq!(key, MediaKey::VOLUME_UP);
                break;
            }
            Ok(_) => {}
            Err(error) => panic!("{error}"),
        }
    }
}

// locks

/// LOCK frames the box saw, as (class, id, direction, scale)
fn locks(mock: &MockBox) -> Vec<(u8, u16, u8, i16)> {
    mock.recorded_frames()
        .into_iter()
        .filter(|frame| frame.ty == FrameType::Lock)
        .map(|frame| {
            let p = &frame.payload;
            (
                p[0],
                u16::from_le_bytes([p[1], p[2]]),
                p[3],
                i16::from_le_bytes([p[4], p[5]]),
            )
        })
        .collect()
}

#[test]
fn locks_reach_the_box_as_lock_frames_and_unlocks_are_100() {
    let mock = keyboard(6, false);
    let (backend, snapshot, _poller, _signals) = backend(&mock);
    assert_eq!(
        snapshot.capabilities.axes(),
        [Axis::X, Axis::Y, Axis::Wheel]
    );
    assert!(
        snapshot
            .capabilities
            .get(Action::LockButton)
            .quirks
            .contains(&Quirk::KeepsLocked)
    );
    mock.clear_recorded();
    let send = |slot, keep| backend.send(Command::Lock { slot, keep }).unwrap();
    send(LockSlot::Button(Button::Left), 0);
    send(LockSlot::Key(Key::A), 0);
    send(LockSlot::Axis(Axis::X, Sign::Positive), 0);
    send(LockSlot::Axis(Axis::Y, Sign::Negative), -100);
    send(LockSlot::Button(Button::Left), 100);
    // class 0 is a button, 1 a key and 3 an axis, and direction 0 is both, 1 positive and 2 negative, like in
    // the medius crate's source
    assert_eq!(
        locks(&mock),
        vec![
            (0, 0, 0, 0),
            (1, 0x04, 0, 0),
            (3, 0, 1, 0),
            (3, 1, 2, -100),
            (0, 0, 0, 100),
        ]
    );
}

#[test]
fn an_unlock_that_gave_up_still_reaches_the_crate() {
    let mock = keyboard(6, false);
    let (backend, _snapshot, _poller, _signals) = backend(&mock);
    let backend = Arc::new(backend);
    mock.clear_recorded();
    let held = backend
        .shared()
        .hold
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let unlocker = thread::spawn({
        let backend = backend.clone();
        move || {
            backend.send(Command::Lock {
                slot: LockSlot::Button(Button::Left),
                keep: 100,
            })
        }
    });
    assert!(unlocker.join().unwrap().is_err());
    drop(held);
    until(|| !locks(&mock).is_empty());
    assert_eq!(locks(&mock), vec![(0, 0, 0, 100)]);
}

#[test]
fn a_mouse_swap_that_changes_the_axes_is_a_reconnect() {
    let mock = mouse(5, true).with_health(Health::from_flags(0x07));
    let (_backend, _snapshot, mut poller, signals) = backend(&mock);
    let _ = mock.clone().with_mouse_caps(MouseCaps {
        n_buttons: 5,
        has_x: true,
        has_y: true,
        has_wheel: true,
        pan: true,
        n_hid: 1,
        ..MouseCaps::default()
    });
    for _ in 0..4 {
        poller.tick();
    }
    assert_eq!(link_events(&signals), vec!["reconnected"]);
}

#[test]
fn an_unlock_passes_the_backend_check_after_the_target_went_away() {
    let mock = mouse(0, false);
    let (backend, snapshot, _poller, _signals) = backend(&mock);
    assert!(!snapshot.capabilities.get(Action::LockButton).is_supported());
    mock.clear_recorded();
    assert!(matches!(
        backend.send(Command::Lock {
            slot: LockSlot::Button(Button::Left),
            keep: 0
        }),
        Err(Error::Unsupported { .. })
    ));
    backend
        .send(Command::Lock {
            slot: LockSlot::Button(Button::Left),
            keep: 100,
        })
        .unwrap();
    assert_eq!(locks(&mock), vec![(0, 0, 0, 100)]);
}

#[test]
fn unlock_all_frees_every_slot() {
    let mock = keyboard(6, false);
    let (backend, _snapshot, _poller, _signals) = backend(&mock);
    mock.clear_recorded();
    backend
        .unlock_all(&[
            LockSlot::Button(Button::Left),
            LockSlot::Key(Key::A),
            LockSlot::Axis(Axis::Y, Sign::Positive),
        ])
        .unwrap();
    assert_eq!(
        locks(&mock),
        vec![(0, 0, 0, 100), (1, 0x04, 0, 100), (3, 1, 1, 100)]
    );
}

// pan

fn with_pan() -> MockBox {
    mouse(5, true).with_mouse_caps(MouseCaps {
        n_buttons: 5,
        has_x: true,
        has_y: true,
        has_wheel: true,
        pan: true,
        n_hid: 1,
        ..MouseCaps::default()
    })
}

#[test]
fn pan_goes_out_as_move_motion_2_where_the_mouse_has_it() {
    let mock = with_pan();
    let (panning, snapshot, _poller, _signals) = backend(&mock);
    assert_eq!(
        snapshot.capabilities.get(Action::Pan).execution,
        Execution::Native
    );
    mock.clear_recorded();
    panning.send(Command::Pan(-3)).unwrap();
    assert_eq!(moves(&mock), vec![(2, -3, 0)]);

    let plain = mouse(5, true);
    let (no_pan, snapshot, _poller, _signals) = backend(&plain);
    assert!(!snapshot.capabilities.get(Action::Pan).is_supported());
    plain.clear_recorded();
    assert!(matches!(
        no_pan.send(Command::Pan(1)),
        Err(Error::Unsupported {
            action: Action::Pan,
            ..
        })
    ));
    assert!(moves(&plain).is_empty());
}

#[test]
fn a_pan_only_report_arrives_as_motion() {
    let mock = with_pan();
    let (device, _poller) = device(&mock);
    let events = device.events();
    until(|| mock.saw(FrameType::Catch));
    mock.push_motion(1, 100, 0, 0, 0, 2);
    loop {
        match events.recv_timeout(Duration::from_secs(2)) {
            Ok(Event::Motion { dx, wheel, pan, .. }) => {
                assert_eq!((dx, wheel, pan), (0, 0, 2));
                break;
            }
            Ok(_) => {}
            Err(error) => panic!("{error}"),
        }
    }
}

// next to the app's own use of medius()

/// the catch table the box ends up with, replayed from the CATCH frames it saw, as (class, id, direction).
/// class 0xFF with id 0xFFFF and state 0 clears it, like in the medius crate's source
fn catch_table(mock: &MockBox) -> HashSet<(u8, u16, u8)> {
    let mut table = HashSet::new();
    for frame in mock.recorded_frames() {
        if frame.ty != FrameType::Catch {
            continue;
        }
        let p = &frame.payload;
        let entry = (p[0], u16::from_le_bytes([p[1], p[2]]), p[3]);
        match p[4] {
            0 if entry.0 == 0xFF && entry.1 == 0xFFFF => table.clear(),
            0 => {
                table.remove(&entry);
            }
            _ => {
                table.insert(entry);
            }
        }
    }
    table
}

fn first_button(events: &crate::event::Events) -> (Button, bool) {
    loop {
        match events.recv_timeout(Duration::from_secs(2)) {
            Ok(Event::Button {
                button, pressed, ..
            }) => return (button, pressed),
            Ok(_) => {}
            Err(error) => panic!("{error}"),
        }
    }
}

#[test]
fn an_app_subscription_keeps_its_entries_while_anybox_swaps_its_own() {
    let mock = mouse(5, true);
    let (device, _poller) = device(&mock);
    let app = device
        .medius()
        .unwrap()
        .input_events([medius::CatchFilter::watch_class(Class::Button)])
        .unwrap();
    let events = device.events();
    until(|| catch_table(&mock).len() > 1);
    drop(events);
    until(|| !catch_table(&mock).iter().any(|entry| entry.0 == 3));
    let events = device.events();
    until(|| catch_table(&mock).iter().any(|entry| entry.0 == 3));
    drop(events);
    thread::sleep(Duration::from_millis(50));
    assert!(
        catch_table(&mock).iter().any(|entry| entry.0 == 0),
        "{:?}",
        catch_table(&mock)
    );
    mock.push_usages(
        1,
        10,
        Class::Button,
        Direction::PRESS,
        &[Usage::new(Class::Button, 0)],
    );
    let input = app.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(input.input.is_press());
}

#[test]
fn a_reset_through_medius_brings_anybox_events_back() {
    let mock = mouse(5, true);
    let (device, _poller) = device(&mock);
    let events = device.events();
    until(|| mock.saw(FrameType::Catch));
    mock.clear_recorded();
    device.medius().unwrap().reset().unwrap();
    // the stream comes back by itself, without the poller
    until(|| caught_classes(&mock).contains(&0));
    mock.push_usages(
        1,
        10,
        Class::Button,
        Direction::PRESS,
        &[Usage::new(Class::Button, 0)],
    );
    assert_eq!(first_button(&events), (Button::Left, true));
}

#[test]
fn a_catch_table_the_app_filled_is_rejected_on_the_receiver() {
    let mock = mouse(5, true);
    let (device, _poller) = device(&mock);
    let filters = (0..32).map(|id| medius::CatchFilter::watch(Usage::new(Class::Button, id)));
    let _app = device.medius().unwrap().input_events(filters).unwrap();
    let events = device.events();
    let mut rejected = HashSet::new();
    while rejected.len() < 2 {
        match events.recv_timeout(Duration::from_secs(2)) {
            Ok(Event::Rejected { action, .. }) => {
                rejected.insert(action);
            }
            Ok(_) => {}
            Err(error) => panic!("{error}"),
        }
    }
    assert_eq!(
        rejected,
        HashSet::from([Action::ButtonEvents, Action::MotionEvents])
    );
}

#[test]
fn drop_leaves_a_press_through_medius_to_the_crate() {
    let mock = mouse(5, true);
    let (device, _poller) = device(&mock);
    let app = device.medius().unwrap();
    app.press(medius::Button::RIGHT).unwrap();
    mock.clear_recorded();
    drop(device);
    assert!(!injects(&mock).contains(&(1, 0)));
    assert!(!mock.saw(FrameType::Reset));
    assert_eq!(app.query_version().unwrap().mac, MAC);
}

#[test]
fn unlocks_after_one_that_ran_out_of_time_still_reach_the_crate() {
    let mock = keyboard(6, false);
    let (backend, _snapshot, _poller, _signals) = backend(&mock);
    let backend = Arc::new(backend);
    mock.clear_recorded();
    let held = backend
        .shared()
        .hold
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let unlocker = thread::spawn({
        let backend = backend.clone();
        move || {
            let started = Instant::now();
            let unlocked = backend.unlock_all(&[
                LockSlot::Button(Button::Left),
                LockSlot::Key(Key::A),
                LockSlot::Axis(Axis::Y, Sign::Positive),
            ]);
            (unlocked, started.elapsed())
        }
    });
    let (unlocked, took) = unlocker.join().unwrap();
    assert!(matches!(unlocked, Err(Error::Disconnected)));
    // only the first one has to wait
    assert!(took < WRITE_DEADLINE * 2, "{took:?}");
    drop(held);
    until(|| locks(&mock).len() == 3);
    assert_eq!(
        locks(&mock),
        vec![(0, 0, 0, 100), (1, 0x04, 0, 100), (3, 1, 1, 100)]
    );
}

#[test]
fn releases_after_one_that_ran_out_of_time_still_reach_the_crate() {
    let mock = mouse(5, true);
    let (backend, _snapshot, _poller, _signals) = backend(&mock);
    let backend = Arc::new(backend);
    mock.clear_recorded();
    let held = backend
        .shared()
        .hold
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let releaser = thread::spawn({
        let backend = backend.clone();
        move || {
            let started = Instant::now();
            let released = backend.release_all(&[
                crate::backend::Usage::Button(Button::Left),
                crate::backend::Usage::Button(Button::Right),
                crate::backend::Usage::Button(Button::Middle),
            ]);
            (released, started.elapsed())
        }
    });
    let (released, took) = releaser.join().unwrap();
    assert!(matches!(released, Err(Error::Disconnected)));
    assert!(took < WRITE_DEADLINE * 2, "{took:?}");
    drop(held);
    until(|| injects(&mock).len() == 3);
    assert_eq!(injects(&mock), vec![(0, 0), (1, 0), (2, 0)]);
}

#[test]
fn a_new_kind_next_to_a_held_button_never_shows_a_second_press() {
    let mock = mouse(5, true);
    let (device, _poller) = device(&mock);
    let buttons = device.events_of(crate::event::EventKinds::BUTTONS);
    until(|| caught_classes(&mock).contains(&0));
    let left = Usage::new(Class::Button, 0);
    mock.push_usages(1, 10, Class::Button, Direction::PRESS, &[left]);
    assert!(matches!(
        buttons.recv_timeout(Duration::from_secs(2)),
        Ok(Event::Button { pressed: true, .. })
    ));
    let _motion = device.events_of(crate::event::EventKinds::MOTION);
    until(|| caught_classes(&mock).contains(&3));
    // the box reports the same held set again, then the release
    mock.push_usages(2, 20, Class::Button, Direction::PRESS, &[left]);
    mock.push_usages(3, 30, Class::Button, Direction::RELEASE, &[]);
    assert!(matches!(
        buttons.recv_timeout(Duration::from_secs(2)),
        Ok(Event::Button { pressed: false, .. })
    ));
}

#[test]
fn received_is_when_anybox_got_the_event_and_not_the_mapped_box_time() {
    let mock = mouse(5, true);
    let (device, _poller) = device(&mock);
    let events = device.events_of(crate::event::EventKinds::BUTTONS);
    until(|| caught_classes(&mock).contains(&0));
    let before = Instant::now();
    mock.push_usages(
        1,
        10,
        Class::Button,
        Direction::PRESS,
        &[Usage::new(Class::Button, 0)],
    );
    let Ok(Event::Button { at, .. }) = events.recv_timeout(Duration::from_secs(2)) else {
        panic!("no button")
    };
    assert!(at.received >= before);
    assert_eq!(at.received_qpc.is_some(), cfg!(windows));
}

// the public mock, used the way an app would

#[cfg(feature = "mock")]
mod public_mock {
    use std::time::Duration;

    use medius::{Class, Direction, Usage};

    use super::mouse;
    use crate::capability::Button;
    use crate::click::ClickTiming;
    use crate::device::Device;
    use crate::event::{DropSource, Event, EventKinds};

    #[test]
    fn a_mock_device_gives_what_the_mock_pushes_and_takes_what_the_app_sends() {
        let mock = mouse(5, true);
        let device =
            Device::mock_medius(mock.clone(), ClickTiming::Fixed(Duration::from_millis(5)))
                .unwrap();
        assert_eq!(device.snapshot().info.port, "mock box");
        let events = device.events_of(EventKinds::BUTTONS);
        super::until(|| super::caught_classes(&mock).contains(&0));
        mock.push_usages(
            1,
            10,
            Class::Button,
            Direction::PRESS,
            &[Usage::new(Class::Button, 0)],
        );
        assert!(matches!(
            events.recv_timeout(Duration::from_secs(2)),
            Ok(Event::Button {
                button: Button::Left,
                pressed: true,
                ..
            })
        ));
        device.press(Button::Right).unwrap();
        assert!(super::injects(&mock).contains(&(1, 1)));
    }

    #[test]
    fn the_manual_poller_brings_a_session_reset_and_box_drops_on_its_ticks() {
        let mock = mouse(5, true);
        let (device, mut poller) =
            Device::mock_medius_manual(mock.clone(), ClickTiming::Fixed(Duration::from_millis(5)))
                .unwrap();
        let events = device.events();
        mock.detach(true);
        poller.tick();
        assert!(matches!(
            events.recv_timeout(Duration::from_secs(2)),
            Ok(Event::SessionReset)
        ));
        let mut state = device.medius().unwrap().query_catch().unwrap();
        state.dropped += 5;
        let _ = mock.clone().with_catch_state(state);
        for _ in 0..4 {
            poller.tick();
        }
        let mut dropped = None;
        while let Ok(event) = events.recv_timeout(Duration::from_millis(500)) {
            if let Event::Dropped { count, source } = event {
                dropped = Some((count, source));
                break;
            }
        }
        assert_eq!(dropped, Some((5, DropSource::Device)));
    }
}

#[test]
fn a_restart_keeps_a_subscription_that_still_works() {
    let mock = mouse(5, true);
    let (backend, _snapshot, _poller, _signals) = backend(&mock);
    let motion = crate::backend::InputKinds {
        motion: true,
        ..crate::backend::InputKinds::default()
    };
    backend.set_streams(motion).unwrap();
    mock.clear_recorded();
    // a stale restart, say after a new receiver already brought the kind back, mustn't swap a live one
    backend
        .restart_stream(crate::backend::InputKind::Motion)
        .unwrap();
    assert_eq!(catch_frames(&mock), 0);
}

#[test]
fn one_kinds_overflow_only_reaches_the_receivers_of_that_kind() {
    let mock = mouse(5, true);
    let (device, _poller) = device(&mock);
    let buttons = device.events_of(crate::event::EventKinds::BUTTONS);
    let motion = device.events_of(crate::event::EventKinds::MOTION);
    until(|| caught_classes(&mock).contains(&3));
    {
        // holding the motion slot makes its pump wait, so the crate's 256 event queue overflows
        let _paused = device_backend(&device).unwrap().shared().slots[1]
            .stream
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        for seq in 0..300u32 {
            mock.push_motion((seq % 250) as u8, seq, 1, 0, 0, 0);
        }
        thread::sleep(Duration::from_millis(100));
    }
    let mut motion_lost = false;
    while let Ok(event) = motion.recv_timeout(Duration::from_millis(500)) {
        if matches!(event, Event::Dropped { .. }) {
            motion_lost = true;
            break;
        }
    }
    assert!(motion_lost);
    while let Ok(event) = buttons.try_recv() {
        assert!(!matches!(event, Event::Dropped { .. }), "{event:?}");
    }
}

#[test]
fn only_the_kind_that_didnt_fit_in_the_catch_table_is_rejected() {
    let mock = mouse(5, true);
    let (device, _poller) = device(&mock);
    // the app leaves room for one more entry, buttons get it and motion doesn't
    let filters = (0..31).map(|id| medius::CatchFilter::watch(Usage::new(Class::Button, id)));
    let _app = device.medius().unwrap().input_events(filters).unwrap();
    let events =
        device.events_of(crate::event::EventKinds::BUTTONS | crate::event::EventKinds::MOTION);
    let mut rejected = Vec::new();
    while let Ok(event) = events.recv_timeout(Duration::from_millis(500)) {
        if let Event::Rejected { action, .. } = event {
            rejected.push(action);
        }
    }
    assert_eq!(rejected, vec![Action::MotionEvents]);
}

#[test]
fn only_a_port_behind_a_ch343_or_with_no_usb_id_could_be_a_medius() {
    // the crate opens the port itself, so the check is all there is to see, a wrong chip never gets the probe
    let with = |usb| crate::transport::Candidate {
        path: "COM7".into(),
        usb,
    };
    let chip = |vid, pid| {
        Some(crate::transport::PortUsb {
            vid,
            pid,
            serial: None,
        })
    };
    assert!(super::could_be_one(&with(chip(0x1A86, 0x55D3))));
    assert!(super::could_be_one(&with(None)));
    assert!(!super::could_be_one(&with(chip(0x10C4, 0xEA60))));
}

#[test]
fn the_clone_s_usb_ids_are_the_peripheral_and_no_clone_is_none() {
    let mock = mouse(5, true).with_device_info(medius::DeviceInfo {
        vid: 0x046d,
        pid: 0xc08b,
        ..medius::DeviceInfo::default()
    });
    let (_, snapshot, _, _) = backend(&mock);
    assert_eq!(
        snapshot.info.peripheral,
        Some(crate::capability::UsbIds::new(0x046d, 0xc08b))
    );
    let (_, snapshot, _, _) = backend(&mouse(5, true));
    assert_eq!(snapshot.info.peripheral, None);
}
