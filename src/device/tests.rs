use std::collections::HashSet;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::backend::{Command, InputKind, InputKinds, LockSlot, Sign, Usage};
use crate::capability::{
    Action, Axis, Button, Capabilities, Capability, Direction, Execution, Quirk,
};
use crate::click::{Click, ClickTiming};
use crate::error::Error;
use crate::event::{DropSource, Event, EventKinds, RecvTimeoutError};
use crate::family::fake::{self, mouse};
use crate::key::{Key, MediaKey};

const SHORT: Duration = Duration::from_millis(200);

fn with_native_click() -> crate::capability::Capabilities {
    mouse().set(
        Action::Click,
        Capability::native().with(Quirk::MaxHold(Duration::from_millis(5000))),
    )
}

#[test]
fn a_native_click_is_one_command() {
    let (device, fake) = fake::open(with_native_click(), ClickTiming::Makcu);
    device.click(Button::Left).unwrap();
    assert_eq!(
        fake.log(),
        vec![Command::Click {
            button: Button::Left,
            hold: None
        }]
    );
}

#[test]
fn a_box_without_a_click_gets_one_timed_on_the_host() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    let capability = device.snapshot().capabilities.get(Action::Click).clone();
    assert_eq!(capability.execution, Execution::Host);
    assert_eq!(
        capability.quirks,
        vec![Quirk::HostClickTiming(ClickTiming::Makcu)]
    );

    device.click(Button::Left).unwrap();
    let log = fake.timed_log();
    assert_eq!(
        log.iter().map(|(_, c)| *c).collect::<Vec<_>>(),
        vec![Command::Press(Button::Left), Command::Release(Button::Left)]
    );
    let held = log[1].0 - log[0].0;
    assert!(held >= Duration::from_millis(35), "held only {held:?}");
    assert!(device.held().is_empty());
}

#[test]
fn a_host_click_uses_the_hold_it_was_given() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Off);
    device
        .click_with(Button::Right, Click::hold(Duration::from_millis(20)))
        .unwrap();
    let log = fake.timed_log();
    assert!(log[1].0 - log[0].0 >= Duration::from_millis(20));
}

#[test]
fn click_timing_off_makes_a_plain_click_fail_where_the_box_has_none() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Off);
    let error = device.click(Button::Left).unwrap_err();
    assert!(matches!(
        error,
        Error::Unsupported {
            action: Action::Click,
            ..
        }
    ));
    assert!(fake.log().is_empty());
}

#[test]
fn native_only_refuses_a_host_click_and_sends_nothing() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    let error = device
        .click_with(Button::Left, Click::default().native_only())
        .unwrap_err();
    assert!(matches!(error, Error::Unsupported { .. }));
    assert!(fake.log().is_empty());
}

#[test]
fn a_click_is_only_composed_when_press_and_release_both_work() {
    let capabilities = mouse().set(Action::Release, Capability::unsupported("no"));
    let (device, _fake) = fake::open(capabilities, ClickTiming::Makcu);
    let click = device.snapshot().capabilities.get(Action::Click).clone();
    assert!(matches!(click.execution, Execution::Unsupported { .. }));
}

#[test]
fn a_fixed_timing_click_refuses_a_hold() {
    let capabilities = mouse().set(
        Action::Click,
        Capability::native().with(Quirk::FixedClickTiming),
    );
    let (device, fake) = fake::open(capabilities, ClickTiming::Makcu);
    let error = device
        .click_with(Button::Left, Click::hold(Duration::from_millis(40)))
        .unwrap_err();
    assert!(matches!(error, Error::Unsupported { .. }));
    assert!(fake.log().is_empty());
    device.click(Button::Left).unwrap();
    assert_eq!(fake.log().len(), 1);
}

#[test]
fn a_hold_past_the_box_maximum_is_an_invalid_argument() {
    let (device, fake) = fake::open(with_native_click(), ClickTiming::Makcu);
    let error = device
        .click_with(Button::Left, Click::hold(Duration::from_secs(6)))
        .unwrap_err();
    assert!(matches!(error, Error::InvalidArgument { .. }));
    assert!(fake.log().is_empty());
}

#[test]
fn a_button_the_box_lacks_is_an_invalid_argument() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    let error = device.press(Button::Extra(9)).unwrap_err();
    assert!(matches!(error, Error::InvalidArgument { .. }));
    assert!(fake.log().is_empty());
}

#[test]
fn a_wheel_bigger_than_the_box_takes_is_unsupported_not_split() {
    let capabilities = mouse().set(
        Action::Wheel,
        Capability::native().with(Quirk::NotchesPerCommand(1)),
    );
    let (device, fake) = fake::open(capabilities, ClickTiming::Makcu);
    assert!(matches!(
        device.wheel(-3).unwrap_err(),
        Error::Unsupported { .. }
    ));
    device.wheel(-1).unwrap();
    assert_eq!(fake.log(), vec![Command::Wheel(-1)]);
}

#[test]
fn an_unsupported_action_names_itself_the_family_and_why() {
    let (device, _fake) = fake::open(mouse(), ClickTiming::Makcu);
    let error = device.discard_pending_moves().unwrap_err();
    assert!(matches!(
        error,
        Error::Unsupported {
            action: Action::DiscardPendingMoves,
            ..
        }
    ));
    assert_eq!(
        error.to_string(),
        "Medius can't discard pending moves, this box doesn't have it"
    );
}

#[test]
fn move_now_goes_out_where_native_and_fails_elsewhere() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    assert!(device.move_now(1, 2).is_err());
    let (device, fake2) = fake::open(
        mouse().set(Action::MoveNow, Capability::native()),
        ClickTiming::Makcu,
    );
    device.move_now(1, 2).unwrap();
    assert!(fake.log().is_empty());
    assert_eq!(fake2.log(), vec![Command::MoveNow(1, 2)]);
}

#[test]
fn a_release_that_fails_keeps_the_button_and_drop_lets_go() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    device.press(Button::Middle).unwrap();
    fake.fail_next(Error::Disconnected);
    assert!(device.release(Button::Middle).is_err());
    assert_eq!(
        device.held(),
        HashSet::from([Usage::Button(Button::Middle)])
    );
    drop(device);
    assert_eq!(
        fake.released_all(),
        vec![vec![Usage::Button(Button::Middle)]]
    );
}

#[test]
fn a_device_can_be_shared_across_threads() {
    fn shareable<T: Send + Sync>() {}
    shareable::<crate::device::Device>();
}

#[test]
fn drop_releases_what_was_held_and_nothing_when_nothing_was() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    device.press(Button::Left).unwrap();
    device.press(Button::Side1).unwrap();
    device.release(Button::Side1).unwrap();
    drop(device);
    assert_eq!(fake.released_all(), vec![vec![Usage::Button(Button::Left)]]);

    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    device.close().unwrap();
    assert!(fake.released_all().is_empty());
}

#[test]
fn a_reconnect_releases_held_buttons_and_swaps_the_snapshot() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    let events = device.events();
    device.press(Button::Left).unwrap();
    fake.disconnect();
    device.settle();
    assert!(matches!(
        device.press(Button::Right),
        Err(Error::Disconnected)
    ));
    assert_eq!(device.held(), HashSet::from([Usage::Button(Button::Left)]));

    fake.reconnect(mouse().set(Action::MoveNow, Capability::native()));
    device.settle();
    assert!(device.held().is_empty());
    assert_eq!(fake.log().last(), Some(&Command::Release(Button::Left)));
    assert!(
        device
            .snapshot()
            .capabilities
            .get(Action::MoveNow)
            .is_supported()
    );
    assert_eq!(events.recv(), Ok(Event::Disconnected));
    assert_eq!(events.recv(), Ok(Event::Reconnected));
}

#[test]
fn a_session_reset_releases_held_buttons() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    device.press(Button::Middle).unwrap();
    fake.session_reset();
    device.settle();
    assert!(device.held().is_empty());
    assert_eq!(fake.log().last(), Some(&Command::Release(Button::Middle)));
}

fn keeper() -> Capabilities {
    mouse().set(Action::Press, Capability::native().with(Quirk::KeepsHeld))
}

#[test]
fn a_box_that_keeps_held_buttons_is_not_told_to_let_go_after_a_reconnect_or_a_reset() {
    let (device, fake) = fake::open(keeper(), ClickTiming::Makcu);
    let events = device.events();
    device.press(Button::Left).unwrap();
    fake.disconnect();
    fake.reconnect(keeper());
    fake.session_reset();
    device.settle();
    assert_eq!(device.held(), HashSet::from([Usage::Button(Button::Left)]));
    assert_eq!(fake.log().last(), Some(&Command::Press(Button::Left)));
    assert_eq!(events.recv(), Ok(Event::Disconnected));
    assert_eq!(events.recv(), Ok(Event::Reconnected));
    assert_eq!(events.recv(), Ok(Event::SessionReset));
    drop(device);
    assert_eq!(fake.released_all(), vec![vec![Usage::Button(Button::Left)]]);
}

#[test]
fn a_host_click_across_a_reconnect_still_lets_go_on_a_box_that_keeps_held_buttons() {
    let (device, fake) = fake::open(keeper(), ClickTiming::Fixed(Duration::from_millis(150)));
    let device = Arc::new(device);
    let clicker = thread::spawn({
        let device = device.clone();
        move || device.click(Button::Left)
    });
    fake.wait_for(Command::Press(Button::Left));
    fake.disconnect();
    fake.reconnect(keeper());
    device.settle();
    clicker.join().unwrap().unwrap();
    assert!(device.held().is_empty());
    assert_eq!(fake.log().last(), Some(&Command::Release(Button::Left)));
}

#[test]
fn a_host_click_that_couldnt_let_go_is_let_go_after_the_reconnect_on_a_box_that_keeps_held_buttons()
{
    let (device, fake) = fake::open(keeper(), ClickTiming::Fixed(Duration::from_millis(60)));
    let device = Arc::new(device);
    let clicker = thread::spawn({
        let device = device.clone();
        move || device.click(Button::Left)
    });
    fake.wait_for(Command::Press(Button::Left));
    fake.disconnect();
    // the box is still down when the hold ends, so the click can't send its release
    assert!(matches!(clicker.join().unwrap(), Err(Error::Disconnected)));
    fake.reconnect(keeper());
    device.settle();
    assert!(device.held().is_empty());
    assert_eq!(fake.log().last(), Some(&Command::Release(Button::Left)));
}

#[test]
fn a_press_that_failed_is_let_go_after_the_reconnect_on_a_box_that_keeps_held_buttons() {
    let (device, fake) = fake::open(keeper(), ClickTiming::Makcu);
    device.press(Button::Right).unwrap();
    fake.fail_next(Error::Disconnected);
    assert!(device.press(Button::Left).is_err());
    fake.disconnect();
    fake.reconnect(keeper());
    device.settle();
    assert_eq!(device.held(), HashSet::from([Usage::Button(Button::Right)]));
    assert_eq!(fake.log().last(), Some(&Command::Release(Button::Left)));
}

#[test]
fn the_snapshot_the_box_comes_back_with_decides_whether_held_buttons_are_let_go() {
    let (device, fake) = fake::open(keeper(), ClickTiming::Makcu);
    device.press(Button::Left).unwrap();
    fake.disconnect();
    fake.reconnect(mouse());
    device.settle();
    assert!(device.held().is_empty());

    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    device.press(Button::Left).unwrap();
    fake.disconnect();
    fake.reconnect(keeper());
    device.settle();
    assert_eq!(device.held(), HashSet::from([Usage::Button(Button::Left)]));
    assert_eq!(fake.log().last(), Some(&Command::Press(Button::Left)));
}

#[test]
fn streams_follow_the_receivers_and_come_back_after_a_reconnect() {
    let capabilities = mouse()
        .set(Action::ButtonEvents, Capability::native())
        .set(Action::MotionEvents, Capability::native());
    let (device, fake) = fake::open(capabilities.clone(), ClickTiming::Makcu);
    let on = InputKinds {
        buttons: true,
        motion: true,
        ..InputKinds::default()
    };
    let first = device.events();
    let second = device.events();
    assert_eq!(fake.streams(), vec![on]);
    drop(first);
    assert_eq!(fake.streams(), vec![on]);

    fake.disconnect();
    fake.reconnect(capabilities);
    device.settle();
    assert_eq!(fake.streams(), vec![on, on]);

    drop(second);
    assert_eq!(fake.streams(), vec![on, on, InputKinds::default()]);
}

#[test]
fn input_and_link_events_arrive_in_order_with_stamps() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    let events = device.events();
    fake.button(Button::Left, true);
    fake.motion(5);
    fake.dropped(3);
    device.settle();

    match events.recv_timeout(SHORT).unwrap() {
        Event::Button {
            button: Button::Left,
            pressed: true,
            at,
        } => assert_eq!(at.box_time.map(|t| t.micros), Some(1000)),
        other => panic!("expected a left press, got {other:?}"),
    }
    assert!(matches!(
        events.recv_timeout(SHORT).unwrap(),
        Event::Motion { dx: 5, .. }
    ));
    assert_eq!(
        events.recv_timeout(SHORT).unwrap(),
        Event::Dropped {
            count: 3,
            source: DropSource::Device
        }
    );
}

#[test]
fn a_full_receiver_still_reports_its_losses() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    let events = device.events();
    for _ in 0..crate::event::QUEUE_CAPACITY + 10 {
        fake.motion(1);
    }
    device.settle();
    assert_eq!(
        events.try_recv(),
        Ok(Event::Dropped {
            count: 11,
            source: DropSource::Host,
        })
    );
}

#[test]
fn events_end_when_the_device_is_gone() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    let events = device.events();
    fake.motion(1);
    device.settle();
    drop(device);
    assert!(matches!(events.recv(), Ok(Event::Motion { .. })));
    assert!(events.recv().is_err());
    assert_eq!(events.recv_timeout(SHORT), Err(RecvTimeoutError::Closed));
}

#[test]
fn threads_racing_each_other_leave_the_record_matching_what_was_sent() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Fixed(Duration::from_micros(50)));
    let device = Arc::new(device);
    let workers: Vec<_> = (0..4)
        .map(|n| {
            let device = device.clone();
            thread::spawn(move || {
                for i in 0..200 {
                    let button = [Button::Left, Button::Right][(n + i) % 2];
                    let _ = match i % 3 {
                        0 => device.press(button),
                        1 => device.release(button),
                        _ => device.click(button),
                    };
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    let mut expected = HashSet::new();
    for command in fake.log() {
        match command {
            Command::Press(button) => {
                expected.insert(Usage::Button(button));
            }
            Command::Release(button) => {
                expected.remove(&Usage::Button(button));
            }
            _ => {}
        }
    }
    assert_eq!(device.held(), expected);
}

#[test]
fn a_reconnect_can_take_a_capability_away() {
    let (device, fake) = fake::open(
        mouse().set(Action::MoveNow, Capability::native()),
        ClickTiming::Makcu,
    );
    fake.reconnect(mouse());
    device.settle();
    assert!(matches!(
        device.move_now(1, 1).unwrap_err(),
        Error::Unsupported { .. }
    ));
}

#[test]
fn a_disconnect_reaches_events_made_before_and_after() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    let before = device.events();
    fake.disconnect();
    device.settle();
    let after = device.events();
    fake.reconnect(mouse());
    device.settle();
    assert_eq!(before.try_recv(), Ok(Event::Disconnected));
    assert_eq!(after.try_recv(), Ok(Event::Reconnected));
}

#[test]
fn a_press_racing_the_reconnect_cleanup_waits_and_is_not_undone() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    let device = Arc::new(device);
    device.press(Button::Left).unwrap();
    fake.disconnect();
    device.settle();

    let release = fake.pause_next_release();
    fake.reconnect(mouse());
    // the link thread is in the middle of the cleanup release now, so it holds the output lock
    release.wait();
    let presser = thread::spawn({
        let device = device.clone();
        move || device.press(Button::Left)
    });
    thread::sleep(Duration::from_millis(20));
    release.wait();
    presser.join().unwrap().unwrap();
    device.settle();

    let log = fake.log();
    let cleanup = log
        .iter()
        .rposition(|c| *c == Command::Release(Button::Left))
        .unwrap();
    let fresh = log
        .iter()
        .rposition(|c| *c == Command::Press(Button::Left))
        .unwrap();
    assert!(
        cleanup < fresh,
        "the cleanup release went out after the new press: {log:?}"
    );
    assert_eq!(device.held(), HashSet::from([Usage::Button(Button::Left)]));
}

#[test]
fn reconnected_is_only_seen_once_the_old_presses_are_released() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    let events = device.events();
    device.press(Button::Right).unwrap();
    fake.disconnect();
    fake.reconnect(mouse());
    assert_eq!(events.recv_timeout(SHORT), Ok(Event::Disconnected));
    assert_eq!(events.recv_timeout(SHORT), Ok(Event::Reconnected));
    assert!(device.held().is_empty());
}

#[test]
fn the_backend_check_catches_a_box_that_changed_quietly() {
    let (device, fake) = fake::open(
        mouse().set(Action::MoveNow, Capability::native()),
        ClickTiming::Makcu,
    );
    fake.change_quietly(mouse());
    assert!(matches!(
        device.move_now(1, 1).unwrap_err(),
        Error::Unsupported { .. }
    ));
    assert!(fake.log().is_empty());
}

#[test]
fn a_cleanup_release_that_fails_keeps_the_button_and_stays_down() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    let events = device.events();
    device.press(Button::Left).unwrap();
    fake.disconnect();
    fake.fail_releases(true);
    fake.reconnect(mouse());
    device.settle();
    assert_eq!(device.held(), HashSet::from([Usage::Button(Button::Left)]));
    assert!(matches!(
        device.press(Button::Right),
        Err(Error::Disconnected)
    ));
    assert_eq!(events.try_recv(), Ok(Event::Disconnected));
    assert!(events.try_recv().is_err());

    fake.fail_releases(false);
    fake.reconnect(mouse());
    device.settle();
    assert!(device.held().is_empty());
    assert_eq!(events.try_recv(), Ok(Event::Reconnected));
}

#[test]
fn a_host_click_cut_by_a_reconnect_leaves_a_new_press_alone() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Fixed(Duration::from_millis(150)));
    let device = Arc::new(device);
    let clicker = thread::spawn({
        let device = device.clone();
        move || device.click(Button::Left)
    });
    fake.wait_for(Command::Press(Button::Left));
    fake.disconnect();
    fake.reconnect(mouse());
    device.settle();
    device.press(Button::Left).unwrap();

    assert!(matches!(
        clicker.join().unwrap(),
        Err(Error::Interrupted {
            action: Action::Click
        })
    ));
    assert_eq!(device.held(), HashSet::from([Usage::Button(Button::Left)]));
    assert_eq!(fake.log().last(), Some(&Command::Press(Button::Left)));
}

#[test]
fn a_closing_device_ignores_link_events_still_in_the_queue() {
    let capabilities = mouse().set(Action::ButtonEvents, Capability::native());
    let (device, fake) = fake::open(capabilities.clone(), ClickTiming::Makcu);
    let events = device.events();
    let streams_before = fake.streams().len();
    device.start_closing();
    fake.disconnect();
    fake.reconnect(capabilities.clone());
    fake.session_reset();
    device.settle();
    assert_eq!(fake.streams().len(), streams_before);
    assert!(matches!(device.press(Button::Left), Err(Error::Closed)));
    assert!(events.try_recv().is_err());
    drop(events);
    assert_eq!(fake.streams().len(), streams_before);
}

#[test]
fn a_session_reset_turns_streams_back_on() {
    let capabilities = mouse().set(Action::ButtonEvents, Capability::native());
    let (device, fake) = fake::open(capabilities, ClickTiming::Makcu);
    let _events = device.events();
    fake.session_reset();
    device.settle();
    assert_eq!(fake.streams().len(), 2);
}

#[test]
fn streams_stay_off_while_down_and_come_on_with_the_reconnect() {
    let capabilities = mouse().set(Action::ButtonEvents, Capability::native());
    let (device, fake) = fake::open(capabilities.clone(), ClickTiming::Makcu);
    fake.disconnect();
    device.settle();
    let _events = device.events();
    assert!(fake.streams().is_empty());
    fake.reconnect(capabilities);
    device.settle();
    assert_eq!(fake.streams().len(), 1);
}

#[test]
fn a_fixed_click_timing_holds_that_long() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Fixed(Duration::from_millis(25)));
    device.click(Button::Left).unwrap();
    let log = fake.timed_log();
    assert!(log[1].0 - log[0].0 >= Duration::from_millis(25));
}

#[test]
fn the_event_iterator_ends_with_the_device() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    let events = device.events();
    fake.motion(1);
    fake.motion(2);
    device.settle();
    drop(device);
    let collected: Vec<Event> = events.collect();
    assert_eq!(collected.len(), 2);
}

#[test]
fn a_one_notch_box_says_so_in_plain_words() {
    let capabilities = mouse().set(
        Action::Wheel,
        Capability::native().with(Quirk::NotchesPerCommand(1)),
    );
    let (device, _fake) = fake::open(capabilities, ClickTiming::Makcu);
    assert_eq!(
        device.wheel(2).unwrap_err().to_string(),
        "Medius can't scroll, it takes one wheel notch per command, send them one at a time"
    );
    assert_eq!(
        device.press(Button::Extra(9)).unwrap_err().to_string(),
        "can't press like that, the box has no extra 9 button"
    );
}

#[test]
fn a_host_click_cut_by_a_box_reset_leaves_a_new_press_alone() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Fixed(Duration::from_millis(150)));
    let device = Arc::new(device);
    let clicker = thread::spawn({
        let device = device.clone();
        move || device.click(Button::Left)
    });
    fake.wait_for(Command::Press(Button::Left));
    fake.session_reset();
    device.settle();
    device.press(Button::Left).unwrap();

    let error = clicker.join().unwrap().unwrap_err();
    assert!(matches!(error, Error::Interrupted { .. }));
    assert!(error.to_string().contains("reconnected or reset"));
    assert_eq!(device.held(), HashSet::from([Usage::Button(Button::Left)]));
}

#[test]
fn a_reconnect_out_of_nowhere_whose_cleanup_fails_goes_down_and_says_so() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    let events = device.events();
    device.press(Button::Left).unwrap();
    fake.fail_releases(true);
    fake.reconnect(mouse());
    device.settle();
    assert!(matches!(
        device.press(Button::Right),
        Err(Error::Disconnected)
    ));
    assert_eq!(events.try_recv(), Ok(Event::Disconnected));
    assert_eq!(device.held(), HashSet::from([Usage::Button(Button::Left)]));
}

#[test]
fn a_press_that_ran_out_of_time_counts_as_held() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    fake.fail_next(Error::Disconnected);
    assert!(matches!(
        device.press(Button::Left),
        Err(Error::Disconnected)
    ));
    assert_eq!(device.held(), HashSet::from([Usage::Button(Button::Left)]));
    drop(device);
    assert_eq!(fake.released_all(), vec![vec![Usage::Button(Button::Left)]]);
}

#[test]
fn a_press_the_box_refused_is_not_held() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    fake.fail_next(Error::Rejected {
        action: Action::Press,
        detail: "no".into(),
    });
    assert!(device.press(Button::Left).is_err());
    assert!(device.held().is_empty());
}

fn with_buttons() -> Capabilities {
    mouse().set(Action::ButtonEvents, Capability::native())
}

#[test]
fn a_lost_stream_reports_one_drop_and_turns_streams_back_on_while_someone_listens() {
    let (device, fake) = fake::open(with_buttons(), ClickTiming::Makcu);
    let events = device.events();
    let before = fake.restarted().len();
    fake.stream_lost(InputKind::Buttons);
    device.settle();
    assert_eq!(
        events.try_recv(),
        Ok(Event::Dropped {
            count: 1,
            source: DropSource::Device
        })
    );
    assert_eq!(fake.restarted().len(), before + 1);
}

#[test]
fn a_lost_stream_after_the_last_receiver_left_stays_off() {
    let (device, fake) = fake::open(with_buttons(), ClickTiming::Makcu);
    drop(device.events());
    let before = fake.restarted().len();
    fake.stream_lost(InputKind::Buttons);
    device.settle();
    assert_eq!(fake.restarted().len(), before);
}

#[test]
fn a_lost_stream_while_down_or_closing_asks_for_nothing() {
    let (device, fake) = fake::open(with_buttons(), ClickTiming::Makcu);
    let _events = device.events();
    fake.disconnect();
    device.settle();
    let before = fake.restarted().len();
    fake.stream_lost(InputKind::Buttons);
    device.settle();
    assert_eq!(fake.restarted().len(), before);
    device.start_closing();
    fake.stream_lost(InputKind::Buttons);
    device.settle();
    assert_eq!(fake.restarted().len(), before);
}

#[test]
fn a_refusal_reaches_every_receiver_as_an_event() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    let events = device.events();
    fake.refused(Action::Move);
    device.settle();
    assert_eq!(
        events.try_recv(),
        Ok(Event::Rejected {
            action: Action::Move,
            count: 1
        })
    );
}

fn with_keyboard(max: Option<u8>) -> crate::capability::Capabilities {
    let press = match max {
        Some(max) => Capability::native().with(Quirk::MaxKeysHeld(max)),
        None => Capability::native(),
    };
    mouse()
        .set(Action::KeyPress, press)
        .set(Action::KeyRelease, Capability::native())
}

fn keeping_keyboard() -> crate::capability::Capabilities {
    mouse()
        .set(Action::Press, Capability::native().with(Quirk::KeepsHeld))
        .set(
            Action::KeyPress,
            Capability::native().with(Quirk::KeepsHeld),
        )
        .set(Action::KeyRelease, Capability::native())
}

#[test]
fn keys_are_gated_by_their_own_capability() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    for error in [
        device.press_key(Key::A).unwrap_err(),
        device.release_key(Key::A).unwrap_err(),
        device.tap_key(Key::A).unwrap_err(),
        device.press_media(MediaKey::MUTE).unwrap_err(),
    ] {
        assert!(matches!(error, Error::Unsupported { .. }), "{error:?}");
    }
    assert!(fake.log().is_empty());
}

#[test]
fn a_key_and_a_button_held_at_once_stay_apart() {
    let (device, fake) = fake::open(with_keyboard(None), ClickTiming::Makcu);
    device.press(Button::Left).unwrap();
    device.press_key(Key::A).unwrap();
    assert_eq!(
        device.held(),
        HashSet::from([Usage::Button(Button::Left), Usage::Key(Key::A)])
    );
    device.release_key(Key::A).unwrap();
    assert_eq!(device.held(), HashSet::from([Usage::Button(Button::Left)]));
    drop(device);
    assert_eq!(fake.released_all(), vec![vec![Usage::Button(Button::Left)]]);
}

#[test]
fn a_held_key_is_let_go_on_drop_and_after_a_reconnect() {
    let (device, fake) = fake::open(with_keyboard(None), ClickTiming::Makcu);
    device.press_key(Key::LEFT_SHIFT).unwrap();
    fake.disconnect();
    fake.reconnect(with_keyboard(None));
    device.settle();
    assert!(fake.log().contains(&Command::KeyRelease(Key::LEFT_SHIFT)));
    assert!(device.held().is_empty());

    device.press_key(Key::B).unwrap();
    drop(device);
    assert_eq!(fake.released_all(), vec![vec![Usage::Key(Key::B)]]);
}

#[test]
fn a_box_that_keeps_held_usages_keeps_a_key_through_a_reconnect() {
    let (device, fake) = fake::open(keeping_keyboard(), ClickTiming::Makcu);
    device.press_key(Key::A).unwrap();
    fake.disconnect();
    fake.reconnect(keeping_keyboard());
    device.settle();
    assert!(!fake.log().contains(&Command::KeyRelease(Key::A)));
    assert_eq!(device.held(), HashSet::from([Usage::Key(Key::A)]));
}

#[test]
fn a_held_key_can_still_be_released_after_the_keyboard_went_away() {
    let (device, fake) = fake::open(keeping_keyboard(), ClickTiming::Makcu);
    device.press_key(Key::A).unwrap();
    let without = mouse().set(Action::Press, Capability::native().with(Quirk::KeepsHeld));
    fake.disconnect();
    fake.reconnect(without);
    device.settle();
    device.release_key(Key::A).unwrap();
    assert!(fake.log().contains(&Command::KeyRelease(Key::A)));
    assert!(device.held().is_empty());
    assert!(matches!(
        device.release_key(Key::B),
        Err(Error::Unsupported { .. })
    ));
}

#[test]
fn keys_past_what_the_box_holds_at_once_are_refused_and_nothing_goes_out() {
    let (device, fake) = fake::open(with_keyboard(Some(6)), ClickTiming::Makcu);
    for key in 0x04..0x0A {
        device.press_key(Key(key)).unwrap();
    }
    // modifiers go in a byte of their own, so they never count
    device.press_key(Key::LEFT_CTRL).unwrap();
    device.press_key(Key::RIGHT_SHIFT).unwrap();
    // pressing the same key again doesn't take another slot
    device.press_key(Key(0x04)).unwrap();
    let sent = fake.log().len();
    let error = device.press_key(Key(0x0A)).unwrap_err();
    assert!(
        matches!(
            &error,
            Error::Unsupported { action: Action::KeyPress, reason, .. } if reason.contains("at most 6 keys")
        ),
        "{error:?}"
    );
    assert_eq!(fake.log().len(), sent);
    device.release_key(Key(0x05)).unwrap();
    device.press_key(Key(0x0A)).unwrap();
}

#[test]
fn a_key_whose_press_may_have_gone_out_counts_toward_the_limit() {
    let (device, fake) = fake::open(with_keyboard(Some(1)), ClickTiming::Makcu);
    fake.fail_next(Error::Disconnected);
    assert!(device.press_key(Key::A).is_err());
    assert!(matches!(
        device.press_key(Key::B),
        Err(Error::Unsupported { .. })
    ));
}

#[test]
fn a_box_without_a_limit_holds_any_number_of_keys() {
    let (device, _fake) = fake::open(with_keyboard(None), ClickTiming::Makcu);
    for key in 0x04..0x20 {
        device.press_key(Key(key)).unwrap();
    }
}

#[test]
fn the_key_limit_follows_the_snapshot_after_a_reconnect() {
    let (device, fake) = fake::open(keeping_keyboard(), ClickTiming::Makcu);
    device.press_key(Key::A).unwrap();
    device.press_key(Key::B).unwrap();
    let limited = keeping_keyboard().set(
        Action::KeyPress,
        Capability::native()
            .with(Quirk::KeepsHeld)
            .with(Quirk::MaxKeysHeld(2)),
    );
    fake.disconnect();
    fake.reconnect(limited);
    device.settle();
    assert!(matches!(
        device.press_key(Key::C),
        Err(Error::Unsupported { .. })
    ));
}

#[test]
fn codes_that_are_no_key_are_refused() {
    let (device, fake) = fake::open(
        with_keyboard(None)
            .set(Action::MediaPress, Capability::native())
            .set(Action::MediaRelease, Capability::native()),
        ClickTiming::Makcu,
    );
    for key in [Key(0x00), Key(0x01), Key(0x03), Key(0xE8), Key(0xFF)] {
        assert!(
            matches!(device.press_key(key), Err(Error::InvalidArgument { .. })),
            "{key}"
        );
    }
    assert!(matches!(
        device.press_media(MediaKey(0)),
        Err(Error::InvalidArgument { .. })
    ));
    device.press_key(Key(0x04)).unwrap();
    device.press_key(Key(0xE7)).unwrap();
    device.press_media(MediaKey(0x1234)).unwrap();
    assert_eq!(fake.log().len(), 3);
}

#[test]
fn a_tap_on_a_box_without_one_is_press_wait_release_from_here() {
    let (device, fake) = fake::open(with_keyboard(None), ClickTiming::Makcu);
    assert_eq!(
        device.snapshot().capabilities.get(Action::KeyTap).execution,
        Execution::Host
    );
    device
        .tap_key_with(Key::A, Click::hold(Duration::from_millis(20)))
        .unwrap();
    let log = fake.timed_log();
    assert_eq!(
        log.iter().map(|(_, c)| *c).collect::<Vec<_>>(),
        vec![Command::KeyPress(Key::A), Command::KeyRelease(Key::A)]
    );
    assert!(log[1].0 - log[0].0 >= Duration::from_millis(20));
    assert!(device.held().is_empty());
    assert!(matches!(
        device.tap_key_with(Key::A, Click::default().native_only()),
        Err(Error::Unsupported {
            action: Action::KeyTap,
            ..
        })
    ));
}

#[test]
fn a_host_tap_needs_both_halves() {
    let only_press = mouse().set(Action::KeyPress, Capability::native());
    let (device, _fake) = fake::open(only_press, ClickTiming::Makcu);
    assert!(
        !device
            .snapshot()
            .capabilities
            .get(Action::KeyTap)
            .is_supported()
    );
}

#[test]
fn a_tap_past_the_key_limit_names_the_tap() {
    let (device, fake) = fake::open(with_keyboard(Some(1)), ClickTiming::Makcu);
    device.press_key(Key::A).unwrap();
    let sent = fake.log().len();
    assert!(matches!(
        device.tap_key(Key::B),
        Err(Error::Unsupported {
            action: Action::KeyTap,
            ..
        })
    ));
    assert_eq!(fake.log().len(), sent);
}

#[test]
fn a_box_with_its_own_tap_gets_one_command() {
    let native = with_keyboard(None).set(Action::KeyTap, Capability::native());
    let (device, fake) = fake::open(native, ClickTiming::Off);
    device.tap_key(Key::ENTER).unwrap();
    assert_eq!(
        fake.log(),
        vec![Command::KeyTap {
            key: Key::ENTER,
            hold: None
        }]
    );
}

#[test]
fn a_box_that_keeps_held_keys_only_says_so_on_its_key_press() {
    let keyboard_only = Capabilities::new(Vec::new())
        .set(
            Action::KeyPress,
            Capability::native().with(Quirk::KeepsHeld),
        )
        .set(Action::KeyRelease, Capability::native());
    let (device, fake) = fake::open(keyboard_only.clone(), ClickTiming::Makcu);
    device.press_key(Key::A).unwrap();
    fake.disconnect();
    fake.reconnect(keyboard_only);
    device.settle();
    assert!(!fake.log().contains(&Command::KeyRelease(Key::A)));
}

#[test]
fn a_held_key_is_let_go_after_a_session_reset() {
    let (device, fake) = fake::open(with_keyboard(None), ClickTiming::Makcu);
    device.press_key(Key::A).unwrap();
    fake.session_reset();
    device.settle();
    assert!(fake.log().contains(&Command::KeyRelease(Key::A)));
    assert!(device.held().is_empty());
}

#[test]
fn a_key_press_that_ran_out_of_time_is_let_go_after_the_reconnect_on_a_keeping_box() {
    let (device, fake) = fake::open(keeping_keyboard(), ClickTiming::Makcu);
    fake.fail_next(Error::Disconnected);
    assert!(device.press_key(Key::A).is_err());
    fake.disconnect();
    fake.reconnect(keeping_keyboard());
    device.settle();
    assert!(fake.log().contains(&Command::KeyRelease(Key::A)));
}

#[test]
fn a_key_tap_with_click_timing_off_needs_a_hold() {
    let (device, fake) = fake::open(with_keyboard(None), ClickTiming::Off);
    assert!(matches!(
        device.tap_key(Key::A),
        Err(Error::Unsupported {
            action: Action::KeyTap,
            ..
        })
    ));
    assert!(fake.log().is_empty());
}

#[test]
fn a_host_tap_of_a_bad_key_names_the_tap() {
    let (device, fake) = fake::open(with_keyboard(None), ClickTiming::Makcu);
    assert!(matches!(
        device.tap_key(Key(0x01)),
        Err(Error::InvalidArgument {
            action: Action::KeyTap,
            ..
        })
    ));
    assert!(fake.log().is_empty());
}

#[test]
fn a_media_tap_on_a_box_without_one_is_composed_from_its_press_and_release() {
    let media = mouse()
        .set(Action::MediaPress, Capability::native())
        .set(Action::MediaRelease, Capability::native());
    let (device, fake) = fake::open(media, ClickTiming::Makcu);
    device
        .tap_media_with(MediaKey::MUTE, Click::hold(Duration::from_millis(5)))
        .unwrap();
    assert_eq!(
        fake.log(),
        vec![
            Command::MediaPress(MediaKey::MUTE),
            Command::MediaRelease(MediaKey::MUTE)
        ]
    );
    assert_eq!(
        device
            .snapshot()
            .capabilities
            .get(Action::MediaTap)
            .execution,
        Execution::Host
    );
}

#[test]
fn two_threads_racing_for_the_last_key_slot_get_one_each_way() {
    let (device, _fake) = fake::open(with_keyboard(Some(1)), ClickTiming::Makcu);
    let device = Arc::new(device);
    let racers: Vec<_> = [Key::A, Key::B]
        .into_iter()
        .map(|key| {
            let device = device.clone();
            thread::spawn(move || device.press_key(key).is_ok())
        })
        .collect();
    let won: usize = racers
        .into_iter()
        .map(|racer| usize::from(racer.join().unwrap()))
        .sum();
    assert_eq!(won, 1);
    assert_eq!(device.held().len(), 1);
}

#[test]
fn key_and_media_events_come_in_order_with_the_rest() {
    let capabilities = with_keyboard(None)
        .set(Action::ButtonEvents, Capability::native())
        .set(Action::KeyEvents, Capability::native())
        .set(Action::MediaEvents, Capability::native());
    let (device, fake) = fake::open(capabilities, ClickTiming::Makcu);
    let events = device.events();
    assert_eq!(
        fake.streams(),
        vec![InputKinds {
            buttons: true,
            keys: true,
            media: true,
            ..InputKinds::default()
        }]
    );
    fake.key(Key::A, true);
    fake.button(Button::Left, true);
    fake.media(MediaKey::MUTE, true);
    fake.key(Key::A, false);
    device.settle();
    let seen: Vec<String> = std::iter::from_fn(|| events.try_recv().ok())
        .map(|event| match event {
            Event::Key { key, pressed, .. } => format!("{key} {pressed}"),
            Event::Media { key, pressed, .. } => format!("{key} {pressed}"),
            Event::Button {
                button, pressed, ..
            } => format!("{button} {pressed}"),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(
        seen,
        [
            "key 0x04 true",
            "left true",
            "media key 0x00E2 true",
            "key 0x04 false"
        ]
    );
}

#[test]
fn a_lost_key_stream_restarts_only_that_stream() {
    let capabilities = with_buttons().set(Action::KeyEvents, Capability::native());
    let (device, fake) = fake::open(capabilities, ClickTiming::Makcu);
    let _events = device.events();
    let streams = fake.streams().len();
    fake.stream_lost(InputKind::Keys);
    device.settle();
    assert_eq!(fake.restarted(), vec![InputKind::Keys]);
    assert_eq!(fake.streams().len(), streams);
}

#[test]
fn a_quiet_change_swaps_the_snapshot_and_nothing_else() {
    let (device, fake) = fake::open(with_keyboard(Some(6)), ClickTiming::Makcu);
    let events = device.events();
    device.press_key(Key::A).unwrap();
    let streams = fake.streams().len();
    fake.changed(with_keyboard(Some(232)));
    device.settle();
    assert!(
        device
            .snapshot()
            .capabilities
            .get(Action::KeyPress)
            .quirks
            .contains(&Quirk::MaxKeysHeld(232))
    );
    assert_eq!(device.held(), HashSet::from([Usage::Key(Key::A)]));
    assert!(!fake.log().contains(&Command::KeyRelease(Key::A)));
    assert_eq!(fake.streams().len(), streams);
    assert_eq!(events.try_recv(), Ok(Event::Changed));
    assert!(events.try_recv().is_err());
}

#[test]
fn the_same_snapshot_again_is_no_change() {
    let (device, fake) = fake::open(with_keyboard(Some(6)), ClickTiming::Makcu);
    let events = device.events();
    fake.changed(with_keyboard(Some(6)));
    device.settle();
    assert!(events.try_recv().is_err());
}

#[test]
fn unread_changes_wait_as_one_and_a_later_one_still_comes() {
    let (device, fake) = fake::open(with_keyboard(Some(6)), ClickTiming::Makcu);
    let events = device.events();
    let buttons = device.events_of(EventKinds::BUTTONS);
    fake.changed(with_keyboard(Some(232)));
    fake.changed(with_keyboard(Some(6)));
    device.settle();
    assert_eq!(events.try_recv(), Ok(Event::Changed));
    assert!(events.try_recv().is_err());
    // this box has no button stream, so that receiver also hears it refused
    assert!(std::iter::from_fn(|| buttons.try_recv().ok()).any(|event| event == Event::Changed));
    fake.changed(with_keyboard(Some(232)));
    device.settle();
    assert_eq!(events.try_recv(), Ok(Event::Changed));
}

fn with_locks() -> Capabilities {
    with_keyboard(None)
        .with_axes(vec![Axis::X, Axis::Y, Axis::Wheel])
        .set(Action::LockButton, Capability::native())
        .set(Action::LockAxis, Capability::native())
        .set(Action::LockKey, Capability::native())
        .set(Action::ScaleAxis, Capability::native())
}

fn lock_commands(fake: &fake::Handle) -> Vec<(LockSlot, i16)> {
    fake.log()
        .into_iter()
        .filter_map(|command| match command {
            Command::Lock { slot, keep } => Some((slot, keep)),
            _ => None,
        })
        .collect()
}

#[test]
fn each_kind_of_lock_is_gated_by_its_own_action_and_target_lists() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    assert!(matches!(
        device.lock_button(Button::Left),
        Err(Error::Unsupported { .. })
    ));
    let (device, _fake) = fake::open(with_locks(), ClickTiming::Makcu);
    assert!(matches!(
        device.lock_axis(Axis::Pan, Direction::Both),
        Err(Error::InvalidArgument { .. })
    ));
    assert!(matches!(
        device.lock_button(Button::Extra(9)),
        Err(Error::InvalidArgument { .. })
    ));
    assert!(matches!(
        device.lock_key(Key(0x01)),
        Err(Error::InvalidArgument { .. })
    ));
    assert!(matches!(
        device.scale_axis(Axis::X, Direction::Both, 256),
        Err(Error::InvalidArgument { .. })
    ));
    assert!(fake.log().is_empty());
}

#[test]
fn both_ways_is_two_slots_and_freeing_one_leaves_the_other() {
    let (device, fake) = fake::open(with_locks(), ClickTiming::Makcu);
    device.lock_axis(Axis::X, Direction::Both).unwrap();
    device.unlock_axis(Axis::X, Direction::Positive).unwrap();
    assert_eq!(
        lock_commands(&fake),
        vec![
            (LockSlot::Axis(Axis::X, Sign::Positive), 0),
            (LockSlot::Axis(Axis::X, Sign::Negative), 0),
            (LockSlot::Axis(Axis::X, Sign::Positive), 100),
        ]
    );
    drop(device);
    assert_eq!(
        fake.unlocked_all(),
        vec![vec![LockSlot::Axis(Axis::X, Sign::Negative)]]
    );
}

#[test]
fn a_scale_is_put_back_on_drop_and_nothing_else_is() {
    let (device, fake) = fake::open(with_locks(), ClickTiming::Makcu);
    device.scale_axis(Axis::Y, Direction::Both, -100).unwrap();
    device.lock_button(Button::Left).unwrap();
    device.unlock_button(Button::Left).unwrap();
    drop(device);
    assert_eq!(
        fake.unlocked_all(),
        vec![vec![
            LockSlot::Axis(Axis::Y, Sign::Negative),
            LockSlot::Axis(Axis::Y, Sign::Positive),
        ]]
    );
}

#[test]
fn a_lock_that_ran_out_of_time_is_recorded_and_a_failed_unlock_stays() {
    let (device, fake) = fake::open(with_locks(), ClickTiming::Makcu);
    fake.fail_next(Error::Disconnected);
    assert!(device.lock_key(Key::A).is_err());
    device.lock_button(Button::Left).unwrap();
    fake.fail_next(Error::Io(std::io::Error::other("refused")));
    assert!(device.unlock_button(Button::Left).is_err());
    drop(device);
    assert_eq!(
        fake.unlocked_all(),
        vec![vec![LockSlot::Button(Button::Left), LockSlot::Key(Key::A)]]
    );
}

#[test]
fn an_unlock_of_a_recorded_lock_passes_when_the_box_lost_the_target() {
    let (device, fake) = fake::open(with_locks(), ClickTiming::Makcu);
    device.lock_button(Button::Left).unwrap();
    fake.disconnect();
    fake.reconnect(mouse());
    device.settle();
    device.unlock_button(Button::Left).unwrap();
    assert!(lock_commands(&fake).contains(&(LockSlot::Button(Button::Left), 100)));
    // one that was never locked is still refused
    assert!(matches!(
        device.unlock_button(Button::Right),
        Err(Error::Unsupported { .. })
    ));
}

#[test]
fn locks_go_again_after_a_reconnect_and_a_reset_unless_the_box_keeps_them() {
    let (device, fake) = fake::open(with_locks(), ClickTiming::Makcu);
    device.lock_button(Button::Left).unwrap();
    fake.disconnect();
    fake.reconnect(with_locks());
    device.settle();
    fake.session_reset();
    device.settle();
    let left = (LockSlot::Button(Button::Left), 0);
    assert_eq!(
        lock_commands(&fake)
            .iter()
            .filter(|&&lock| lock == left)
            .count(),
        3
    );

    let keeping = with_locks().set(
        Action::LockButton,
        Capability::native().with(Quirk::KeepsLocked),
    );
    let (device, fake) = fake::open(keeping.clone(), ClickTiming::Makcu);
    device.lock_button(Button::Left).unwrap();
    fake.disconnect();
    fake.reconnect(keeping);
    device.settle();
    assert_eq!(lock_commands(&fake).len(), 1);
}

#[test]
fn a_lock_the_box_cant_do_any_more_waits_and_goes_once_it_can() {
    let (device, fake) = fake::open(with_locks(), ClickTiming::Makcu);
    device.lock_button(Button::Left).unwrap();
    fake.disconnect();
    fake.reconnect(mouse());
    device.settle();
    assert_eq!(lock_commands(&fake).len(), 1);
    fake.disconnect();
    fake.reconnect(with_locks());
    device.settle();
    assert_eq!(lock_commands(&fake).len(), 2);
    drop(device);
    assert_eq!(
        fake.unlocked_all(),
        vec![vec![LockSlot::Button(Button::Left)]]
    );
}

#[test]
fn pan_is_gated_by_its_own_action() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    assert!(matches!(
        device.pan(1),
        Err(Error::Unsupported {
            action: Action::Pan,
            ..
        })
    ));
    assert!(fake.log().is_empty());
    let (device, fake) = fake::open(
        mouse()
            .set(Action::Pan, Capability::native())
            .set(Action::Wheel, Capability::unsupported("no wheel")),
        ClickTiming::Makcu,
    );
    device.pan(1).unwrap();
    assert_eq!(fake.log(), vec![Command::Pan(1)]);
}

// receivers that want only some kinds of input

fn streamer() -> Capabilities {
    mouse()
        .set(Action::ButtonEvents, Capability::native())
        .set(Action::MotionEvents, Capability::native())
        .set(Action::KeyEvents, Capability::native())
}

fn kinds(buttons: bool, motion: bool, keys: bool) -> InputKinds {
    InputKinds {
        buttons,
        motion,
        keys,
        media: false,
    }
}

#[test]
fn the_box_streams_what_the_receivers_want_together() {
    let (device, fake) = fake::open(streamer(), ClickTiming::Makcu);
    let buttons = device.events_of(EventKinds::BUTTONS);
    assert_eq!(fake.streams().last(), Some(&kinds(true, false, false)));
    let motion = device.events_of(EventKinds::MOTION);
    assert_eq!(fake.streams().last(), Some(&kinds(true, true, false)));
    drop(buttons);
    assert_eq!(fake.streams().last(), Some(&kinds(false, true, false)));
    drop(motion);
    assert_eq!(fake.streams().last(), Some(&InputKinds::default()));
}

#[test]
fn a_receiver_only_gets_its_own_kinds_and_every_connection_event() {
    let (device, fake) = fake::open(streamer(), ClickTiming::Makcu);
    let buttons = device.events_of(EventKinds::BUTTONS);
    let keys = device.events_of(EventKinds::KEYS);
    fake.motion(3);
    fake.key(Key::A, true);
    fake.button(Button::Left, true);
    fake.disconnect();
    device.settle();
    let seen = |events: &crate::event::Events| {
        std::iter::from_fn(|| events.try_recv().ok()).collect::<Vec<_>>()
    };
    let for_buttons = seen(&buttons);
    assert!(
        matches!(for_buttons[0], Event::Button { .. }),
        "{for_buttons:?}"
    );
    assert_eq!(for_buttons[1], Event::Disconnected);
    assert_eq!(for_buttons.len(), 2);
    let for_keys = seen(&keys);
    assert!(matches!(for_keys[0], Event::Key { .. }), "{for_keys:?}");
    assert_eq!(for_keys[1], Event::Disconnected);
    assert_eq!(for_keys.len(), 2);
}

#[test]
fn asking_for_a_kind_the_box_cant_stream_says_so_once() {
    let (device, _fake) = fake::open(
        mouse().set(Action::ButtonEvents, Capability::native()),
        ClickTiming::Makcu,
    );
    let keys = device.events_of(EventKinds::KEYS | EventKinds::BUTTONS);
    assert_eq!(
        keys.try_recv(),
        Ok(Event::Rejected {
            action: Action::KeyEvents,
            count: 1
        })
    );
    assert!(keys.try_recv().is_err());
    // everything the box has, so nothing to refuse
    let all = device.events();
    assert!(all.try_recv().is_err());
}

#[test]
fn a_stream_that_doesnt_come_on_is_rejected_to_the_receivers_that_wanted_it() {
    let (device, fake) = fake::open(streamer(), ClickTiming::Makcu);
    let motion = device.events_of(EventKinds::MOTION);
    fake.fail_streams(
        kinds(true, false, false),
        Error::Io(std::io::Error::other("no")),
    );
    let buttons = device.events_of(EventKinds::BUTTONS);
    device.settle();
    assert_eq!(
        buttons.try_recv(),
        Ok(Event::Rejected {
            action: Action::ButtonEvents,
            count: 1
        })
    );
    assert!(motion.try_recv().is_err());
}

#[test]
fn a_stream_that_failed_on_a_lost_link_isnt_rejected() {
    let (device, fake) = fake::open(streamer(), ClickTiming::Makcu);
    fake.fail_streams(kinds(true, false, false), Error::Disconnected);
    let buttons = device.events_of(EventKinds::BUTTONS);
    device.settle();
    assert!(buttons.try_recv().is_err());
}

#[test]
fn a_lost_stream_only_tells_the_receivers_of_that_kind() {
    let (device, fake) = fake::open(streamer(), ClickTiming::Makcu);
    let buttons = device.events_of(EventKinds::BUTTONS);
    let keys = device.events_of(EventKinds::KEYS);
    fake.stream_lost(InputKind::Keys);
    device.settle();
    assert!(matches!(keys.try_recv(), Ok(Event::Dropped { .. })));
    assert!(buttons.try_recv().is_err());
    assert_eq!(fake.restarted(), vec![InputKind::Keys]);
}

#[test]
fn a_receiver_dropped_while_down_leaves_nothing_streaming_after_the_reconnect() {
    let (device, fake) = fake::open(streamer(), ClickTiming::Makcu);
    let buttons = device.events_of(EventKinds::BUTTONS);
    fake.disconnect();
    device.settle();
    drop(buttons);
    fake.reconnect(streamer());
    device.settle();
    assert_eq!(fake.streams().last(), Some(&InputKinds::default()));
}

// closing a shared device

#[test]
fn a_shared_device_closes_from_another_thread_and_then_says_closed() {
    let (device, _fake) = fake::open(mouse(), ClickTiming::Makcu);
    let device = Arc::new(device);
    let events = device.events();
    let closer = thread::spawn({
        let device = device.clone();
        move || device.close()
    });
    assert!(closer.join().unwrap().is_ok());
    assert!(matches!(device.press(Button::Left), Err(Error::Closed)));
    assert!(matches!(device.close(), Err(Error::Closed)));
    assert_eq!(events.recv_timeout(SHORT), Err(RecvTimeoutError::Closed));
    let late = device.events();
    assert_eq!(late.recv_timeout(SHORT), Err(RecvTimeoutError::Closed));
}

#[test]
fn the_constructors_build_what_the_device_gives_you() {
    let (device, fake) = fake::open(streamer(), ClickTiming::Makcu);
    let events = device.events();
    fake.key(Key::A, true);
    device.settle();
    let Ok(Event::Key { at, .. }) = events.try_recv() else {
        panic!("no key")
    };
    assert_eq!(
        Event::key(Key::A, true, at),
        Event::Key {
            key: Key::A,
            pressed: true,
            at
        }
    );
    let built = crate::event::Stamp::new(at.arrived, at.since_open);
    assert_eq!(built, at);
}

#[test]
fn drop_after_close_doesnt_shut_down_again() {
    let (device, fake) = fake::open(mouse(), ClickTiming::Makcu);
    device.close().unwrap();
    assert_eq!(fake.shutdowns(), 1);
    drop(device);
    assert_eq!(fake.shutdowns(), 1);
}

#[test]
fn a_lost_stream_that_wont_come_back_is_rejected_to_its_receivers() {
    let (device, fake) = fake::open(streamer(), ClickTiming::Makcu);
    let keys = device.events_of(EventKinds::KEYS);
    let buttons = device.events_of(EventKinds::BUTTONS);
    fake.fail_restart(Error::Io(std::io::Error::other("no")));
    fake.stream_lost(InputKind::Keys);
    device.settle();
    let seen: Vec<Event> = std::iter::from_fn(|| keys.try_recv().ok()).collect();
    assert!(
        seen.contains(&Event::Rejected {
            action: Action::KeyEvents,
            count: 1
        }),
        "{seen:?}"
    );
    assert!(buttons.try_recv().is_err());
}

#[test]
fn received_is_the_read_the_backend_took_not_when_the_core_got_it() {
    let (device, fake) = fake::open(streamer(), ClickTiming::Makcu);
    let events = device.events();
    let read_at = std::time::Instant::now();
    thread::sleep(Duration::from_millis(20));
    fake.button_read(
        Button::Left,
        crate::transport::os::Read {
            at: read_at,
            qpc: Some(42),
        },
    );
    device.settle();
    let Ok(Event::Button { at, .. }) = events.try_recv() else {
        panic!("no button")
    };
    assert_eq!(at.received, read_at);
    assert_eq!(at.received_qpc, Some(42));
    assert!(at.arrived > at.received);
}

#[test]
fn a_change_that_adds_a_kind_streams_it_to_whoever_wants_it() {
    let buttons = mouse().set(Action::ButtonEvents, Capability::native());
    let (device, fake) = fake::open(buttons.clone(), ClickTiming::Makcu);
    let _events = device.events();
    device.settle();
    fake.changed(buttons.set(Action::KeyEvents, Capability::native()));
    device.settle();
    assert_eq!(
        fake.streams().last(),
        Some(&InputKinds {
            buttons: true,
            keys: true,
            ..InputKinds::default()
        })
    );
}
