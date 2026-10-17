use std::ffi::{CStr, CString, c_char};
use std::mem::{align_of, offset_of, size_of};
use std::ptr;
use std::thread;
use std::time::{Duration, Instant};

use anybox::medius::FrameType;
use anybox::{
    Action, Button, ClickTiming, Connect, DropSource, Event, EventKinds, Family, FerrumApi, Key,
    MediaKey, Quirk, StableId, Stamp,
};

use crate::connect::*;
use crate::device::*;
use crate::events::*;
use crate::failure::*;
use crate::keys::{ANYBOX_KEY_A, ANYBOX_KEY_LEFT_SHIFT, ANYBOX_MEDIA_VOLUME_UP};
use crate::mock::*;
use crate::snapshot::*;
use crate::status::*;
use crate::values::*;
use crate::{ANYBOX_ABI_VERSION, anybox_abi_version, anybox_has_mock};

fn last_error() -> String {
    unsafe { CStr::from_ptr(anybox_last_error_message()) }
        .to_str()
        .unwrap()
        .to_string()
}

fn text(pointer: *const c_char) -> String {
    unsafe { CStr::from_ptr(pointer) }
        .to_str()
        .unwrap()
        .to_string()
}

struct Mock {
    mock: *mut AnyboxMock,
    device: *mut AnyboxDevice,
}

impl Drop for Mock {
    fn drop(&mut self) {
        unsafe {
            anybox_device_free(self.device);
            anybox_mock_free(self.mock);
        }
    }
}

fn mock() -> Mock {
    mock_with(false)
}

fn mock_with(keyboard: bool) -> Mock {
    let timing = AnyboxClickTiming {
        kind: ANYBOX_CLICK_TIMING_FIXED,
        fixed_us: 5_000,
    };
    let mut mock = ptr::null_mut();
    let mut device = ptr::null_mut();
    let status = unsafe { anybox_mock_medius_open(&timing, keyboard, &mut mock, &mut device) };
    assert_eq!(status, ANYBOX_STATUS_OK, "{}", last_error());
    Mock { mock, device }
}

fn events(device: *const AnyboxDevice, kinds: u32) -> *mut AnyboxEvents {
    let mut events = ptr::null_mut();
    assert_eq!(
        unsafe { anybox_device_events(device, kinds, &mut events) },
        ANYBOX_STATUS_OK
    );
    events
}

fn subscribed(mock: &Mock, kinds: u32) {
    let start = Instant::now();
    while !unsafe { anybox_mock_subscribed(mock.mock, kinds) } {
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "the receiver never subscribed"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

fn recv(events: *const AnyboxEvents, timeout_us: i64) -> (AnyboxStatus, AnyboxEvent) {
    let mut event = AnyboxEvent::default();
    let status = unsafe { anybox_events_recv(events, timeout_us, &mut event) };
    (status, event)
}

/// the next event that isn't a connection event or a drop, which the mock can send at any time
fn next_input(events: *const AnyboxEvents) -> AnyboxEvent {
    loop {
        let (status, event) = recv(events, 2_000_000);
        assert_eq!(status, ANYBOX_STATUS_OK, "{}", last_error());
        if matches!(
            event.kind,
            ANYBOX_EVENT_BUTTON | ANYBOX_EVENT_MOTION | ANYBOX_EVENT_KEY | ANYBOX_EVENT_MEDIA
        ) {
            return event;
        }
    }
}

// conventions

#[test]
fn versions_and_mock_say_what_this_build_is() {
    assert_eq!(anybox_abi_version(), ANYBOX_ABI_VERSION);
    assert_eq!(text(crate::anybox_version()), env!("CARGO_PKG_VERSION"));
    assert!(anybox_has_mock());
}

#[test]
fn a_panic_becomes_a_status_and_never_unwinds() {
    assert_eq!(guard(|| panic!("on purpose")), ANYBOX_STATUS_PANIC);
    assert!(last_error().contains("on purpose"), "{}", last_error());
    guard_free(|| panic!("in a free"));
    assert!(last_error().contains("in a free"), "{}", last_error());
}

#[test]
fn the_last_error_starts_empty_and_survives_a_call_that_works() {
    thread::spawn(|| {
        assert_eq!(last_error(), "");
        let mock = mock();
        unsafe { anybox_device_press(mock.device, 9999) };
        let refused = last_error();
        assert!(refused.contains("9999"), "{refused}");
        assert_eq!(
            unsafe { anybox_device_move_by(mock.device, 1, 0) },
            ANYBOX_STATUS_OK
        );
        assert_eq!(last_error(), refused);
    })
    .join()
    .unwrap();
}

// the second thread fails in between the first one's call and its read, so one error kept for the whole process
// would show the other thread's text
#[test]
fn an_error_on_another_thread_in_between_leaves_this_one_alone() {
    let mock = mock();
    let failed = std::sync::Arc::new(std::sync::Barrier::new(2));
    let read = std::sync::Arc::new(std::sync::Barrier::new(2));
    let other = {
        let (device, failed, read) = (Shared(mock.device), failed.clone(), read.clone());
        thread::spawn(move || {
            let device = device;
            failed.wait();
            unsafe { anybox_device_press(device.0, 8888) };
            assert!(last_error().contains("8888"), "{}", last_error());
            read.wait();
        })
    };
    unsafe { anybox_device_press(mock.device, 9999) };
    failed.wait();
    read.wait();
    let mine = last_error();
    other.join().unwrap();
    assert!(mine.contains("9999") && !mine.contains("8888"), "{mine}");
}

#[test]
fn a_nul_inside_a_string_ends_it() {
    assert_eq!(c_string("COM3\0junk").as_bytes(), b"COM3");
}

#[test]
fn null_arguments_are_refused_by_name() {
    let mock = mock();
    unsafe {
        assert_eq!(
            anybox_device_move_by(ptr::null(), 1, 1),
            ANYBOX_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(last_error(), "device is null");
        assert_eq!(
            anybox_device_snapshot(mock.device, ptr::null_mut()),
            ANYBOX_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(last_error(), "out is null");
        assert_eq!(
            anybox_device_click_with(mock.device, ANYBOX_BUTTON_LEFT, ptr::null()),
            ANYBOX_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(last_error(), "click is null");
        assert_eq!(
            anybox_open(ptr::null(), ptr::null_mut()),
            ANYBOX_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            anybox_connect(ptr::null_mut()),
            ANYBOX_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            anybox_events_recv(ptr::null(), 0, &mut AnyboxEvent::default()),
            ANYBOX_STATUS_INVALID_ARGUMENT
        );
        // the free functions and the getters take null too
        anybox_device_free(ptr::null_mut());
        anybox_events_free(ptr::null_mut());
        anybox_snapshot_free(ptr::null_mut());
        anybox_scan_list_free(ptr::null_mut());
        anybox_string_list_free(ptr::null_mut());
        anybox_mock_free(ptr::null_mut());
        assert_eq!(anybox_scan_list_count(ptr::null()), 0);
        assert!(anybox_scan_list_port(ptr::null(), 0).is_null());
        assert_eq!(anybox_string_list_count(ptr::null()), 0);
        assert!(anybox_string_list_get(ptr::null(), 0).is_null());
        assert!(!anybox_mock_subscribed(ptr::null(), ANYBOX_EVENTS_BUTTONS));
    }
}

#[test]
fn a_string_that_isnt_utf8_is_refused() {
    let port = CString::new(vec![0xC3, 0x28]).unwrap();
    let options = AnyboxConnectOptions {
        port: port.as_ptr(),
        ..AnyboxConnectOptions::default()
    };
    let mut device = ptr::null_mut();
    assert_eq!(
        unsafe { anybox_open(&options, &mut device) },
        ANYBOX_STATUS_INVALID_ARGUMENT
    );
    assert_eq!(last_error(), "options.port isn't UTF-8");
    assert!(device.is_null());
}

#[test]
fn out_params_stay_as_they_were_when_a_call_fails() {
    let mock = mock();
    let marker = 0x1234 as *mut AnyboxSnapshot;
    let mut snapshot = marker;
    unsafe {
        assert_eq!(
            anybox_device_snapshot(ptr::null(), &mut snapshot),
            ANYBOX_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(snapshot, marker);

        let mut receiver = 0x1234 as *mut AnyboxEvents;
        assert_eq!(
            anybox_device_events(mock.device, 1 << 9, &mut receiver),
            ANYBOX_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(receiver, 0x1234 as *mut AnyboxEvents);

        let mut options = AnyboxConnectOptions {
            family: 77,
            ..AnyboxConnectOptions::default()
        };
        let mut list = 0x1234 as *mut AnyboxScanList;
        assert_eq!(
            anybox_scan(&options, &mut list),
            ANYBOX_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(list, 0x1234 as *mut AnyboxScanList);
        options.family = ANYBOX_FAMILY_ANY;
        let unknown = [40];
        options.can = unknown.as_ptr();
        options.can_len = 1;
        let mut ports = 0x1234 as *mut AnyboxStringList;
        assert_eq!(
            anybox_candidates(&options, &mut ports),
            ANYBOX_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(ports, 0x1234 as *mut AnyboxStringList);

        let events = events(mock.device, ANYBOX_EVENTS_ALL);
        let before = AnyboxEvent {
            kind: 4242,
            ..AnyboxEvent::default()
        };
        while recv(events, 0).0 == ANYBOX_STATUS_OK {}
        let mut event = before;
        assert_eq!(
            anybox_events_recv(events, 0, &mut event),
            ANYBOX_STATUS_RECV_TIMEOUT
        );
        assert_eq!(event, before);
        anybox_events_free(events);

        let snapshot = take_snapshot(mock.device);
        let mut capability = AnyboxCapability {
            execution: 4242,
            reason: ptr::null(),
            quirk_count: 4242,
        };
        assert_eq!(
            anybox_snapshot_capability(snapshot, 23, &mut capability),
            ANYBOX_STATUS_INVALID_ARGUMENT
        );
        assert_eq!((capability.execution, capability.quirk_count), (4242, 4242));
        let mut quirk = AnyboxQuirk {
            kind: 4242,
            ..AnyboxQuirk::default()
        };
        assert_eq!(
            anybox_snapshot_quirk(snapshot, ANYBOX_ACTION_MOVE, 999, &mut quirk),
            ANYBOX_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(quirk.kind, 4242);
        let mut len = 4242;
        assert_eq!(
            anybox_snapshot_buttons(snapshot, ptr::null_mut(), 3, &mut len),
            ANYBOX_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(len, 4242);
        anybox_snapshot_free(snapshot);
    }
}

#[test]
fn values_anybox_doesnt_know_are_refused() {
    let mock = mock();
    unsafe {
        for status in [
            anybox_device_press(mock.device, 5),
            anybox_device_press(mock.device, ANYBOX_BUTTON_EXTRA_BASE + 256),
            anybox_device_lock_axis(mock.device, 4, ANYBOX_DIRECTION_BOTH),
            anybox_device_lock_axis(mock.device, ANYBOX_AXIS_X, 3),
            anybox_device_scale_axis(mock.device, ANYBOX_AXIS_X, 9, 50),
        ] {
            assert_eq!(status, ANYBOX_STATUS_INVALID_ARGUMENT, "{}", last_error());
        }
    }
    let refused = |options: AnyboxConnectOptions| {
        let mut list = ptr::null_mut();
        unsafe { anybox_candidates(&options, &mut list) }
    };
    for options in [
        AnyboxConnectOptions {
            family: 5,
            ..AnyboxConnectOptions::default()
        },
        AnyboxConnectOptions {
            ferrum_api: 3,
            ..AnyboxConnectOptions::default()
        },
        AnyboxConnectOptions {
            can: [23].as_ptr(),
            can_len: 1,
            ..AnyboxConnectOptions::default()
        },
        AnyboxConnectOptions {
            can: ptr::null(),
            can_len: 1,
            ..AnyboxConnectOptions::default()
        },
        AnyboxConnectOptions {
            id_kind: 3,
            ..AnyboxConnectOptions::default()
        },
        AnyboxConnectOptions {
            id_kind: ANYBOX_ID_USB_SERIAL,
            ..AnyboxConnectOptions::default()
        },
        AnyboxConnectOptions {
            click_timing: AnyboxClickTiming {
                kind: 4,
                fixed_us: 0,
            },
            ..AnyboxConnectOptions::default()
        },
    ] {
        assert_eq!(refused(options), ANYBOX_STATUS_INVALID_ARGUMENT);
    }
    assert_eq!(event_kinds(16), Err(ANYBOX_STATUS_INVALID_ARGUMENT));
}

#[test]
fn an_error_from_anybox_keeps_its_status_and_its_text() {
    let mock = mock();
    unsafe {
        // the mock is a mouse with no keyboard behind it
        assert_eq!(
            anybox_device_press_key(mock.device, Key::A.0),
            ANYBOX_STATUS_UNSUPPORTED
        );
        assert!(
            last_error().starts_with("Medius can't press a key"),
            "{}",
            last_error()
        );
        assert_eq!(
            anybox_device_scale_axis(mock.device, ANYBOX_AXIS_X, ANYBOX_DIRECTION_BOTH, 900),
            ANYBOX_STATUS_INVALID_ARGUMENT
        );
        assert!(
            last_error().starts_with("can't scale an axis like that"),
            "{}",
            last_error()
        );
        assert_eq!(anybox_device_close(mock.device), ANYBOX_STATUS_OK);
        assert_eq!(
            anybox_device_move_by(mock.device, 1, 0),
            ANYBOX_STATUS_CLOSED
        );
        assert_eq!(last_error(), anybox::Error::Closed.to_string());
        assert_eq!(anybox_device_close(mock.device), ANYBOX_STATUS_CLOSED);
    }
    for (error, status) in [
        (anybox::Error::Closed, ANYBOX_STATUS_CLOSED),
        (anybox::Error::Disconnected, ANYBOX_STATUS_DISCONNECTED),
        (
            anybox::Error::Io(std::io::Error::other("cable")),
            ANYBOX_STATUS_IO,
        ),
    ] {
        let text = error.to_string();
        assert_eq!(crate::status::error(error), status);
        assert_eq!(last_error(), text);
    }
}

// conversions

#[test]
fn every_table_value_goes_both_ways() {
    for family in [Family::Medius, Family::MakcuV4, Family::Ferrum, Family::Ysc] {
        assert_eq!(family_filter(family_number(family)), Ok(Some(family)));
    }
    assert_eq!(family_filter(ANYBOX_FAMILY_ANY), Ok(None));
    for api in [FerrumApi::Software, FerrumApi::Legacy] {
        assert_eq!(
            ferrum_api_filter(ferrum_api_number(Some(api))),
            Ok(Some(api))
        );
    }
    assert_eq!(ferrum_api_number(None), ANYBOX_FERRUM_API_NONE);
    for (number, action) in ACTIONS {
        assert_eq!(crate::values::action(number), Ok(action));
        assert_eq!(action_number(action), number);
    }
    let list = [ANYBOX_ACTION_KEY_TAP, ANYBOX_ACTION_MOVE];
    assert_eq!(
        unsafe { actions(list.as_ptr(), 2, "can") },
        Ok(vec![Action::KeyTap, Action::Move])
    );
    assert_eq!(unsafe { actions(ptr::null(), 0, "can") }, Ok(vec![]));
    assert_eq!(event_kinds(ANYBOX_EVENTS_ALL), Ok(EventKinds::ALL));
    assert_eq!(
        event_kinds(ANYBOX_EVENTS_BUTTONS | ANYBOX_EVENTS_MEDIA),
        Ok(EventKinds::BUTTONS | EventKinds::MEDIA)
    );
    assert_eq!(event_kinds(0), Ok(EventKinds::NONE));
}

#[test]
fn every_button_has_a_number() {
    let named = [
        (Button::Left, ANYBOX_BUTTON_LEFT),
        (Button::Right, ANYBOX_BUTTON_RIGHT),
        (Button::Middle, ANYBOX_BUTTON_MIDDLE),
        (Button::Side1, ANYBOX_BUTTON_SIDE1),
        (Button::Side2, ANYBOX_BUTTON_SIDE2),
        (Button::Extra(0), 256),
        (Button::Extra(255), 511),
    ];
    for (button, number) in named {
        assert_eq!(button_number(button), number);
        assert_eq!(crate::values::button(number), Ok(button));
    }
    assert!(crate::values::button(512).is_err());
    assert!(crate::values::button(255).is_err());
}

#[test]
fn every_click_timing_goes_both_ways() {
    for timing in [
        ClickTiming::Makcu,
        ClickTiming::Ferrum,
        ClickTiming::Fixed(Duration::from_micros(1_500)),
        ClickTiming::Off,
    ] {
        assert_eq!(click_timing(&click_timing_struct(timing)), Ok(timing));
    }
    assert_eq!(
        click_timing(&AnyboxClickTiming {
            kind: ANYBOX_CLICK_TIMING_FIXED,
            fixed_us: u64::MAX,
        }),
        Ok(ClickTiming::Fixed(Duration::from_micros(u64::MAX)))
    );
}

#[test]
fn a_click_keeps_its_hold_and_its_timing() {
    let click = |hold_us, has_hold, native_only| {
        crate::values::click(&AnyboxClick {
            hold_us,
            has_hold,
            native_only,
        })
    };
    assert_eq!(click(0, false, false), anybox::Click::default());
    assert_eq!(
        click(80_000, true, false),
        anybox::Click::hold(Duration::from_millis(80))
    );
    assert_eq!(
        click(80_000, true, true),
        anybox::Click::hold(Duration::from_millis(80)).native_only()
    );
    // the hold is only read when there is one
    assert_eq!(
        click(80_000, false, true),
        anybox::Click::default().native_only()
    );
}

#[test]
fn every_connect_option_reaches_the_builder() {
    let port = CString::new("COM7").unwrap();
    let serial = CString::new("ABC123").unwrap();
    let built =
        |options: AnyboxConnectOptions| format!("{:?}", unsafe { builder(&options) }.unwrap());
    assert_eq!(
        built(AnyboxConnectOptions::default()),
        format!("{:?}", Connect::auto())
    );
    assert_eq!(
        format!("{:?}", unsafe { builder(ptr::null()) }.unwrap()),
        format!("{:?}", Connect::auto())
    );
    let everything = AnyboxConnectOptions {
        port: port.as_ptr(),
        family: ANYBOX_FAMILY_FERRUM,
        ferrum_api: ANYBOX_FERRUM_API_LEGACY,
        can: [ANYBOX_ACTION_MOVE, ANYBOX_ACTION_LOCK_KEY].as_ptr(),
        can_len: 2,
        id_kind: ANYBOX_ID_USB_SERIAL,
        id_mac: [0; 6],
        id_serial: serial.as_ptr(),
        click_timing: AnyboxClickTiming {
            kind: ANYBOX_CLICK_TIMING_FIXED,
            fixed_us: 9_000,
        },
    };
    assert_eq!(
        built(everything),
        format!(
            "{:?}",
            Connect::port("COM7")
                .family(Family::Ferrum)
                .ferrum_api(FerrumApi::Legacy)
                .can(Action::Move)
                .can(Action::LockKey)
                .id(StableId::UsbSerial("ABC123".into()))
                .click_timing(ClickTiming::Fixed(Duration::from_millis(9)))
        )
    );
    let mac = [1, 2, 3, 4, 5, 6];
    assert_eq!(
        built(AnyboxConnectOptions {
            id_kind: ANYBOX_ID_MAC,
            id_mac: mac,
            ..AnyboxConnectOptions::default()
        }),
        format!("{:?}", Connect::auto().id(StableId::Mac(mac)))
    );
    // zeros are the defaults, so C code can start from `{0}`
    let mut zeroed: AnyboxConnectOptions = unsafe { std::mem::zeroed() };
    assert_eq!(built(zeroed), format!("{:?}", Connect::auto()));
    assert_eq!(
        unsafe { anybox_connect_options_default(&mut zeroed) },
        ANYBOX_STATUS_OK
    );
    assert_eq!(built(zeroed), format!("{:?}", Connect::auto()));
}

#[test]
fn every_quirk_has_its_fields() {
    let ms = Duration::from_millis;
    let quirks = [
        (
            Quirk::ForcedUpOnRelease {
                min: ms(1),
                max: ms(2),
            },
            ANYBOX_QUIRK_FORCED_UP_ON_RELEASE,
            1_000,
            2_000,
            0,
        ),
        (
            Quirk::PhysicalOverride,
            ANYBOX_QUIRK_PHYSICAL_OVERRIDE,
            0,
            0,
            0,
        ),
        (Quirk::WholeAxisOnly, ANYBOX_QUIRK_WHOLE_AXIS_ONLY, 0, 0, 0),
        (
            Quirk::NotchesPerCommand(127),
            ANYBOX_QUIRK_NOTCHES_PER_COMMAND,
            0,
            0,
            127,
        ),
        (
            Quirk::FixedClickTiming,
            ANYBOX_QUIRK_FIXED_CLICK_TIMING,
            0,
            0,
            0,
        ),
        (
            Quirk::MaxHold(ms(500)),
            ANYBOX_QUIRK_MAX_HOLD,
            0,
            500_000,
            0,
        ),
        (
            Quirk::BoxClickTiming {
                min: ms(75),
                max: ms(125),
            },
            ANYBOX_QUIRK_BOX_CLICK_TIMING,
            75_000,
            125_000,
            0,
        ),
        (Quirk::KeepsHeld, ANYBOX_QUIRK_KEEPS_HELD, 0, 0, 0),
        (Quirk::MaxKeysHeld(6), ANYBOX_QUIRK_MAX_KEYS_HELD, 0, 0, 6),
        (Quirk::KeepsLocked, ANYBOX_QUIRK_KEEPS_LOCKED, 0, 0, 0),
    ];
    for (quirk, kind, min_us, max_us, count) in quirks {
        assert_eq!(
            quirk_struct(&quirk),
            AnyboxQuirk {
                kind,
                min_us,
                max_us,
                count,
                click_timing: AnyboxClickTiming::default(),
            },
            "{quirk:?}"
        );
    }
    let host = quirk_struct(&Quirk::HostClickTiming(ClickTiming::Fixed(ms(7))));
    assert_eq!(host.kind, ANYBOX_QUIRK_HOST_CLICK_TIMING);
    assert_eq!(
        host.click_timing,
        AnyboxClickTiming {
            kind: ANYBOX_CLICK_TIMING_FIXED,
            fixed_us: 7_000,
        }
    );
}

#[test]
fn every_event_kind_has_its_fields() {
    let start = Instant::now();
    let at = Stamp::new(start, Duration::from_nanos(1_500))
        .with_box_time(anybox::BoxTime::new(77, anybox::Clock::MediusDeviceChip))
        .with_received(start + Duration::from_nanos(500), Some(-9));
    let stamp = AnyboxEvent {
        since_open_ns: 1_500,
        received_since_open_ns: 2_000,
        received_qpc: -9,
        has_received_qpc: true,
        has_box_time: true,
        box_clock: ANYBOX_CLOCK_MEDIUS_DEVICE_CHIP,
        box_time_us: 77,
        arrived_ns: at.arrived_ns(),
        received_ns: at.arrived_ns() + 500,
        ..AnyboxEvent::default()
    };
    let cases = [
        (
            Event::button(Button::Extra(3), true, at),
            AnyboxEvent {
                kind: ANYBOX_EVENT_BUTTON,
                button: 259,
                pressed: true,
                ..stamp
            },
        ),
        (
            Event::key(Key::LEFT_SHIFT, false, at),
            AnyboxEvent {
                kind: ANYBOX_EVENT_KEY,
                key: 0xE1,
                ..stamp
            },
        ),
        (
            Event::media(MediaKey::VOLUME_UP, true, at),
            AnyboxEvent {
                kind: ANYBOX_EVENT_MEDIA,
                media: 0xE9,
                pressed: true,
                ..stamp
            },
        ),
        (
            Event::motion(-1, 2, -3, 4, at),
            AnyboxEvent {
                kind: ANYBOX_EVENT_MOTION,
                dx: -1,
                dy: 2,
                wheel: -3,
                pan: 4,
                ..stamp
            },
        ),
        (
            Event::dropped(12, DropSource::Host),
            AnyboxEvent {
                kind: ANYBOX_EVENT_DROPPED,
                count: 12,
                source: ANYBOX_DROP_SOURCE_HOST,
                ..AnyboxEvent::default()
            },
        ),
        (
            Event::dropped(1, DropSource::Device),
            AnyboxEvent {
                kind: ANYBOX_EVENT_DROPPED,
                count: 1,
                source: ANYBOX_DROP_SOURCE_DEVICE,
                ..AnyboxEvent::default()
            },
        ),
        (
            Event::rejected(Action::Move, 3),
            AnyboxEvent {
                kind: ANYBOX_EVENT_REJECTED,
                action: ANYBOX_ACTION_MOVE,
                count: 3,
                ..AnyboxEvent::default()
            },
        ),
    ];
    for (event, expected) in cases {
        assert_eq!(event_struct(&event), expected, "{event:?}");
    }
    for (event, kind) in [
        (Event::SessionReset, ANYBOX_EVENT_SESSION_RESET),
        (Event::Disconnected, ANYBOX_EVENT_DISCONNECTED),
        (Event::Reconnected, ANYBOX_EVENT_RECONNECTED),
        (Event::Changed, ANYBOX_EVENT_CHANGED),
    ] {
        assert_eq!(
            event_struct(&event),
            AnyboxEvent {
                kind,
                ..AnyboxEvent::default()
            }
        );
    }
    let plain = Stamp::new(start, Duration::from_secs(1));
    let event = event_struct(&Event::button(Button::Left, true, plain));
    assert!(!event.has_box_time && !event.has_received_qpc);
    assert_eq!(event.received_since_open_ns, 1_000_000_000);
}

// snapshots

fn take_snapshot(device: *const AnyboxDevice) -> *mut AnyboxSnapshot {
    let mut snapshot = ptr::null_mut();
    assert_eq!(
        unsafe { anybox_device_snapshot(device, &mut snapshot) },
        ANYBOX_STATUS_OK
    );
    snapshot
}

#[test]
fn a_snapshot_describes_the_box() {
    let mock = mock();
    let snapshot = take_snapshot(mock.device);
    let rust = unsafe { &(*mock.device).0 }.snapshot();
    unsafe {
        let mut info: AnyboxInfo = std::mem::zeroed();
        assert_eq!(anybox_snapshot_info(snapshot, &mut info), ANYBOX_STATUS_OK);
        assert_eq!(text(info.port), "mock box");
        assert_eq!(info.family, ANYBOX_FAMILY_MEDIUS);
        assert_eq!(info.ferrum_api, ANYBOX_FERRUM_API_NONE);
        assert_eq!(info.firmware_kind, ANYBOX_FIRMWARE_VERSION);
        assert_eq!(info.id_kind, ANYBOX_ID_MAC);
        assert_eq!(rust.info.id, Some(StableId::Mac(info.id_mac)));
        assert_eq!(info.has_peripheral, rust.info.peripheral.is_some());

        for (number, action) in ACTIONS {
            let expected = rust.capabilities.get(action);
            let mut capability: AnyboxCapability = std::mem::zeroed();
            assert_eq!(
                anybox_snapshot_capability(snapshot, number, &mut capability),
                ANYBOX_STATUS_OK
            );
            assert_eq!(capability.execution, execution_number(&expected.execution));
            assert_eq!(capability.quirk_count as usize, expected.quirks.len());
            assert_eq!(
                capability.reason.is_null(),
                expected.is_supported(),
                "{action:?}"
            );
            if let anybox::Execution::Unsupported { reason } = &expected.execution {
                assert_eq!(text(capability.reason), reason.as_ref());
            }
            for (index, quirk) in expected.quirks.iter().enumerate() {
                let mut out = AnyboxQuirk::default();
                assert_eq!(
                    anybox_snapshot_quirk(snapshot, number, index as u32, &mut out),
                    ANYBOX_STATUS_OK
                );
                assert_eq!(out, quirk_struct(quirk));
            }
        }
        anybox_snapshot_free(snapshot);
    }
}

#[test]
fn the_arrays_give_their_size_first_and_fill_a_short_buffer() {
    let mock = mock();
    let snapshot = take_snapshot(mock.device);
    let rust = unsafe { &(*mock.device).0 }.snapshot();
    let buttons: Vec<u32> = rust
        .capabilities
        .buttons()
        .iter()
        .map(|button| button_number(*button))
        .collect();
    assert!(buttons.len() > 2);
    unsafe {
        let mut len = 0;
        assert_eq!(
            anybox_snapshot_buttons(snapshot, ptr::null_mut(), 0, &mut len),
            ANYBOX_STATUS_OK
        );
        assert_eq!(len, buttons.len());
        let mut short = [9999u32; 2];
        assert_eq!(
            anybox_snapshot_buttons(snapshot, short.as_mut_ptr(), 2, &mut len),
            ANYBOX_STATUS_OK
        );
        assert_eq!((len, &short[..]), (buttons.len(), &buttons[..2]));
        let mut all = vec![9999u32; buttons.len() + 3];
        assert_eq!(
            anybox_snapshot_buttons(snapshot, all.as_mut_ptr(), all.len(), &mut len),
            ANYBOX_STATUS_OK
        );
        assert_eq!(&all[..len], &buttons[..]);
        assert_eq!(all[len], 9999, "nothing past the count is written");
        assert_eq!(
            anybox_snapshot_axes(snapshot, ptr::null_mut(), 0, &mut len),
            ANYBOX_STATUS_OK
        );
        assert_eq!(len, rust.capabilities.axes().len());
        assert_eq!(
            anybox_snapshot_axes(snapshot, ptr::null_mut(), 0, ptr::null_mut()),
            ANYBOX_STATUS_INVALID_ARGUMENT
        );
        anybox_snapshot_free(snapshot);
    }
}

#[test]
fn snapshot_strings_live_until_the_snapshot_is_freed() {
    let mock = mock();
    let snapshot = take_snapshot(mock.device);
    unsafe {
        let mut info: AnyboxInfo = std::mem::zeroed();
        anybox_snapshot_info(snapshot, &mut info);
        // the device going away doesn't take the snapshot's strings with it
        anybox_device_close(mock.device);
        let _churn: Vec<String> = (0..1000).map(|n| n.to_string()).collect();
        assert_eq!(text(info.port), "mock box");
        anybox_snapshot_free(snapshot);
    }
}

// events

#[test]
fn pushed_input_comes_out_as_events() {
    let mock = mock();
    let events = events(mock.device, ANYBOX_EVENTS_ALL);
    subscribed(&mock, ANYBOX_EVENTS_BUTTONS | ANYBOX_EVENTS_MOTION);
    unsafe {
        anybox_mock_push_motion(mock.mock, 2, 123_456, -2, 3, 1, 0);
        let motion = next_input(events);
        assert_eq!(
            (motion.kind, motion.dx, motion.dy, motion.wheel, motion.pan),
            (ANYBOX_EVENT_MOTION, -2, 3, 1, 0)
        );
        assert!(motion.has_box_time);
        assert_eq!(
            (motion.box_clock, motion.box_time_us),
            (ANYBOX_CLOCK_MEDIUS_HOST_CHIP, 123_456)
        );
        anybox_mock_push_buttons(mock.mock, 3, 200_000, true, 1 << 0);
        anybox_mock_push_buttons(mock.mock, 4, 210_000, false, 0);
        let press = next_input(events);
        let release = next_input(events);
        assert_eq!(
            (press.kind, press.button, press.pressed),
            (ANYBOX_EVENT_BUTTON, ANYBOX_BUTTON_LEFT, true)
        );
        assert_eq!(
            (release.button, release.pressed),
            (ANYBOX_BUTTON_LEFT, false)
        );
        assert!(release.since_open_ns >= press.since_open_ns);
        anybox_mock_push_buttons(mock.mock, 5, 220_000, true, 1 << 5);
        assert_eq!(next_input(events).button, ANYBOX_BUTTON_EXTRA_BASE + 5);
        assert_eq!(
            anybox_mock_push_buttons(mock.mock, 6, 0, true, 1 << 16),
            ANYBOX_STATUS_INVALID_ARGUMENT
        );
        anybox_events_free(events);
    }
}

#[test]
fn recv_times_out_tries_and_ends() {
    let mock = mock();
    let events = events(mock.device, ANYBOX_EVENTS_BUTTONS);
    // a fresh receiver can get a connection event first, so read until the queue is empty
    while recv(events, 0).0 == ANYBOX_STATUS_OK {}
    let (status, _) = recv(events, 0);
    assert_eq!(status, ANYBOX_STATUS_RECV_TIMEOUT);
    let start = Instant::now();
    assert_eq!(recv(events, 30_000).0, ANYBOX_STATUS_RECV_TIMEOUT);
    assert!(start.elapsed() >= Duration::from_millis(30));
    assert_eq!(last_error(), "no event came in time");
    unsafe { anybox_device_close(mock.device) };
    assert_eq!(recv(events, i64::MAX).0, ANYBOX_STATUS_RECV_CLOSED);
    assert_eq!(recv(events, -1).0, ANYBOX_STATUS_RECV_CLOSED);
    assert_eq!(recv(events, 0).0, ANYBOX_STATUS_RECV_CLOSED);
    unsafe { anybox_events_free(events) };
}

#[test]
fn a_kind_the_box_cant_stream_is_rejected_on_its_own_receiver() {
    let mock = mock();
    let events = events(mock.device, ANYBOX_EVENTS_KEYS);
    let rejected = loop {
        let (status, event) = recv(events, 2_000_000);
        assert_eq!(status, ANYBOX_STATUS_OK, "{}", last_error());
        if event.kind == ANYBOX_EVENT_REJECTED {
            break event;
        }
    };
    assert_eq!(rejected.action, ANYBOX_ACTION_KEY_EVENTS);
    unsafe { anybox_events_free(events) };
}

struct Shared<T>(*mut T);
unsafe impl<T> Send for Shared<T> {}

fn woken_by_close(queued: bool) {
    let mock = mock();
    let events = events(mock.device, ANYBOX_EVENTS_MOTION);
    if queued {
        subscribed(&mock, ANYBOX_EVENTS_MOTION);
        unsafe { anybox_mock_push_motion(mock.mock, 2, 10, 5, 0, 0, 0) };
    }
    let reader = Shared(events);
    let reader = thread::spawn(move || {
        let reader = reader;
        let mut motion = 0;
        loop {
            match recv(reader.0, -1) {
                (ANYBOX_STATUS_OK, event) if event.kind == ANYBOX_EVENT_MOTION => motion += 1,
                (ANYBOX_STATUS_OK, _) => {}
                (status, _) => return (status, motion),
            }
        }
    });
    if !queued {
        // a reader with no timeout keeps waiting, so it has to still be there well past any short wait
        thread::sleep(Duration::from_millis(1_500));
        assert!(
            !reader.is_finished(),
            "a recv with no timeout returned on its own"
        );
    } else {
        // the motion has to reach the queue before the close, otherwise it just never arrives
        thread::sleep(Duration::from_millis(300));
    }
    let device = Shared(mock.device);
    let closer = thread::spawn(move || {
        let device = device;
        unsafe { anybox_device_close(device.0) }
    });
    assert_eq!(closer.join().unwrap(), ANYBOX_STATUS_OK);
    let (status, motion) = reader.join().unwrap();
    assert_eq!(status, ANYBOX_STATUS_RECV_CLOSED);
    assert_eq!(motion, usize::from(queued));
    unsafe { anybox_events_free(events) };
}

#[test]
fn closing_from_another_thread_wakes_a_reader_with_nothing_queued() {
    woken_by_close(false);
}

#[test]
fn closing_from_another_thread_still_hands_out_what_was_queued() {
    woken_by_close(true);
}

// the receiver's own close says CLOSED, not RECV_CLOSED, so a reader knows it was its receiver and not the device
#[test]
fn its_own_close_wakes_a_reader_and_says_closed() {
    let mock = mock();
    let events = events(mock.device, ANYBOX_EVENTS_MOTION);
    let reader = Shared(events);
    let ready = std::sync::Arc::new(std::sync::Barrier::new(2));
    let reader = {
        let ready = ready.clone();
        thread::spawn(move || {
            let reader = reader;
            ready.wait();
            loop {
                match recv(reader.0, -1) {
                    // the mock can send a connection event at any time
                    (ANYBOX_STATUS_OK, _) => {}
                    (status, _) => return status,
                }
            }
        })
    };
    ready.wait();
    thread::sleep(Duration::from_millis(50));
    let started = Instant::now();
    assert_eq!(unsafe { anybox_events_close(events) }, ANYBOX_STATUS_OK);
    assert_eq!(reader.join().unwrap(), ANYBOX_STATUS_CLOSED);
    assert!(started.elapsed() < Duration::from_secs(2));
    // every recv after it says so too, and a second close is fine
    assert_eq!(recv(events, 0).0, ANYBOX_STATUS_CLOSED);
    assert!(
        last_error().contains("this receiver was closed"),
        "{}",
        last_error()
    );
    assert_eq!(recv(events, 1_000).0, ANYBOX_STATUS_CLOSED);
    assert_eq!(unsafe { anybox_events_close(events) }, ANYBOX_STATUS_OK);
    // the device itself is still up
    assert_eq!(
        unsafe { anybox_device_move_by(mock.device, 1, 0) },
        ANYBOX_STATUS_OK
    );
    unsafe { anybox_events_free(events) };
}

#[test]
fn its_own_close_throws_out_what_was_queued() {
    let mock = mock();
    let closing = events(mock.device, ANYBOX_EVENTS_MOTION);
    // every receiver gets an event at the same moment, so once this one has it, the other one has it queued
    let witness = events(mock.device, ANYBOX_EVENTS_MOTION);
    subscribed(&mock, ANYBOX_EVENTS_MOTION);
    unsafe { anybox_mock_push_motion(mock.mock, 2, 10, 5, 0, 0, 0) };
    assert_eq!(next_input(witness).kind, ANYBOX_EVENT_MOTION);
    assert_eq!(unsafe { anybox_events_close(closing) }, ANYBOX_STATUS_OK);
    assert_eq!(recv(closing, 0).0, ANYBOX_STATUS_CLOSED);
    unsafe { anybox_events_free(closing) };
    unsafe { anybox_events_free(witness) };
}

#[test]
fn a_device_and_its_receivers_can_be_shared_between_threads() {
    fn shared<T: Send + Sync>() {}
    shared::<anybox::Device>();
    shared::<anybox::Events>();
}

// the device

#[test]
fn every_output_call_reaches_the_box() {
    let mock = mock();
    let device = mock.device;
    let click = AnyboxClick {
        hold_us: 1_000,
        has_hold: true,
        native_only: false,
    };
    unsafe {
        for (name, status) in [
            ("move_by", anybox_device_move_by(device, 3, -4)),
            ("move_now", anybox_device_move_now(device, 1, 1)),
            ("press", anybox_device_press(device, ANYBOX_BUTTON_RIGHT)),
            (
                "release",
                anybox_device_release(device, ANYBOX_BUTTON_RIGHT),
            ),
            ("click", anybox_device_click(device, ANYBOX_BUTTON_LEFT)),
            (
                "click_with",
                anybox_device_click_with(device, ANYBOX_BUTTON_LEFT, &click),
            ),
            (
                "lock_button",
                anybox_device_lock_button(device, ANYBOX_BUTTON_MIDDLE),
            ),
            (
                "unlock_button",
                anybox_device_unlock_button(device, ANYBOX_BUTTON_MIDDLE),
            ),
            (
                "lock_axis",
                anybox_device_lock_axis(device, ANYBOX_AXIS_Y, ANYBOX_DIRECTION_POSITIVE),
            ),
            (
                "unlock_axis",
                anybox_device_unlock_axis(device, ANYBOX_AXIS_Y, ANYBOX_DIRECTION_POSITIVE),
            ),
            (
                "scale_axis",
                anybox_device_scale_axis(device, ANYBOX_AXIS_X, ANYBOX_DIRECTION_BOTH, 50),
            ),
            ("wheel", anybox_device_wheel(device, -2)),
            (
                "discard_pending_moves",
                anybox_device_discard_pending_moves(device),
            ),
        ] {
            assert_eq!(status, ANYBOX_STATUS_OK, "{name}: {}", last_error());
        }
        // a mouse with no keyboard behind it and no pan refuses the rest, and that's the box answering, not the C
        // side
        for (name, status) in [
            ("press_key", anybox_device_press_key(device, Key::A.0)),
            ("release_key", anybox_device_release_key(device, Key::A.0)),
            ("tap_key", anybox_device_tap_key(device, Key::A.0)),
            (
                "tap_key_with",
                anybox_device_tap_key_with(device, Key::A.0, &click),
            ),
            (
                "press_media",
                anybox_device_press_media(device, MediaKey::MUTE.0),
            ),
            (
                "release_media",
                anybox_device_release_media(device, MediaKey::MUTE.0),
            ),
            (
                "tap_media",
                anybox_device_tap_media(device, MediaKey::MUTE.0),
            ),
            (
                "tap_media_with",
                anybox_device_tap_media_with(device, MediaKey::MUTE.0, &click),
            ),
            ("lock_key", anybox_device_lock_key(device, Key::A.0)),
            ("unlock_key", anybox_device_unlock_key(device, Key::A.0)),
            ("pan", anybox_device_pan(device, 1)),
        ] {
            assert_eq!(
                status,
                ANYBOX_STATUS_UNSUPPORTED,
                "{name}: {}",
                last_error()
            );
        }
    }
    let mock_box = unsafe { &(*mock.mock).0 };
    assert!(mock_box.saw(FrameType::Inject));
}

#[test]
fn the_medius_handle_shares_the_device() {
    use medius_capi::*;
    let mock = mock();
    unsafe {
        let mut medius = ptr::null_mut();
        assert_eq!(
            anybox_device_medius(mock.device, &mut medius),
            ANYBOX_STATUS_OK
        );
        assert_eq!(
            medius_device_led(
                medius,
                MediusLedTarget::Both as u8,
                MediusLedMode::Blink as u8,
                0
            ),
            MediusStatus::Ok
        );
        let mut version: MediusVersion = std::mem::zeroed();
        assert_eq!(
            medius_device_query_version(medius, &mut version),
            MediusStatus::Ok
        );
        assert_eq!(medius_abi_version(), MEDIUS_ABI_VERSION);
        medius_device_free(medius);
        assert!((*mock.mock).0.saw(FrameType::Led));
        // freeing the Medius handle leaves anybox's device open on the same connection
        assert_eq!(anybox_device_move_by(mock.device, 1, 0), ANYBOX_STATUS_OK);
        anybox_device_close(mock.device);
        let mut after = 0x1234 as *mut MediusDevice;
        assert_eq!(
            anybox_device_medius(mock.device, &mut after),
            ANYBOX_STATUS_CLOSED
        );
        assert_eq!(after, 0x1234 as *mut MediusDevice);
    }
}

// the layout every binding mirrors, checked against the committed table so a change to a struct can't slip by

macro_rules! layout {
    ($out:ident, $type:ty, $($field:ident),+) => {
        $out.push_str(&format!(
            "{} size {} align {}\n",
            stringify!($type),
            size_of::<$type>(),
            align_of::<$type>()
        ));
        $($out.push_str(&format!("  {} {}\n", stringify!($field), offset_of!($type, $field)));)+
    };
}

fn layout_table() -> String {
    let mut out = String::new();
    layout!(out, AnyboxClickTiming, kind, fixed_us);
    layout!(out, AnyboxClick, hold_us, has_hold, native_only);
    layout!(
        out,
        AnyboxConnectOptions,
        port,
        family,
        ferrum_api,
        can,
        can_len,
        id_kind,
        id_mac,
        id_serial,
        click_timing
    );
    layout!(
        out,
        AnyboxInfo,
        port,
        family,
        model,
        firmware_kind,
        firmware_major,
        firmware_minor,
        firmware_patch,
        firmware_protocol,
        firmware_build,
        firmware_text,
        id_kind,
        id_mac,
        id_serial,
        ferrum_api,
        has_peripheral,
        peripheral_vid,
        peripheral_pid
    );
    layout!(out, AnyboxCapability, execution, reason, quirk_count);
    layout!(out, AnyboxQuirk, kind, min_us, max_us, count, click_timing);
    layout!(
        out,
        AnyboxEvent,
        kind,
        button,
        key,
        pressed,
        media,
        dx,
        dy,
        wheel,
        pan,
        source,
        action,
        count,
        since_open_ns,
        received_since_open_ns,
        received_qpc,
        has_received_qpc,
        has_box_time,
        box_clock,
        box_time_us,
        arrived_ns,
        received_ns
    );
    layout!(
        out,
        AnyboxErrorDetail,
        kind,
        has_action,
        action,
        family,
        expected_family,
        found_family,
        port,
        text,
        expected_id_kind,
        expected_id_mac,
        expected_id_serial,
        found_id_kind,
        found_id_mac,
        found_id_serial,
        found_firmware_kind,
        found_firmware_major,
        found_firmware_minor,
        found_firmware_patch,
        found_firmware_protocol,
        found_firmware_build,
        found_firmware_text,
        needed_firmware_kind,
        needed_firmware_major,
        needed_firmware_minor,
        needed_firmware_patch,
        needed_firmware_protocol,
        needed_firmware_build,
        needed_firmware_text,
        expected_ferrum_api,
        found_ferrum_api,
        io_kind,
        has_os_error,
        os_error
    );
    out
}

// the header's number is what every binding's wrap helper comes from, so it has to be what Rust says for each clock
#[test]
fn the_wrap_constant_is_what_every_clock_says() {
    for clock in [
        anybox::Clock::MediusHostChip,
        anybox::Clock::MediusDeviceChip,
    ] {
        assert_eq!(
            Duration::from_micros(ANYBOX_BOX_CLOCK_WRAP_US),
            clock.wraps_after(),
            "{clock:?}"
        );
    }
}

#[test]
fn the_layout_matches_the_committed_table() {
    // cargo says where the crate is at run time too, and a test binary copied from another checkout has the other
    // one's path built in
    let path = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("tests/layout.txt");
    let table = layout_table();
    if std::env::var_os("ANYBOX_WRITE_LAYOUT").is_some() {
        std::fs::write(&path, &table).unwrap();
    }
    let committed = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert_eq!(
        committed, table,
        "the struct layout changed, so bump ANYBOX_ABI_VERSION and run the tests once with ANYBOX_WRITE_LAYOUT=1"
    );
}
#[test]
fn every_named_key_matches_anybox() {
    use crate::keys::*;
    let keys = [
        (ANYBOX_KEY_A, Key::A),
        (ANYBOX_KEY_B, Key::B),
        (ANYBOX_KEY_C, Key::C),
        (ANYBOX_KEY_D, Key::D),
        (ANYBOX_KEY_E, Key::E),
        (ANYBOX_KEY_F, Key::F),
        (ANYBOX_KEY_G, Key::G),
        (ANYBOX_KEY_H, Key::H),
        (ANYBOX_KEY_I, Key::I),
        (ANYBOX_KEY_J, Key::J),
        (ANYBOX_KEY_K, Key::K),
        (ANYBOX_KEY_L, Key::L),
        (ANYBOX_KEY_M, Key::M),
        (ANYBOX_KEY_N, Key::N),
        (ANYBOX_KEY_O, Key::O),
        (ANYBOX_KEY_P, Key::P),
        (ANYBOX_KEY_Q, Key::Q),
        (ANYBOX_KEY_R, Key::R),
        (ANYBOX_KEY_S, Key::S),
        (ANYBOX_KEY_T, Key::T),
        (ANYBOX_KEY_U, Key::U),
        (ANYBOX_KEY_V, Key::V),
        (ANYBOX_KEY_W, Key::W),
        (ANYBOX_KEY_X, Key::X),
        (ANYBOX_KEY_Y, Key::Y),
        (ANYBOX_KEY_Z, Key::Z),
        (ANYBOX_KEY_NUM1, Key::NUM1),
        (ANYBOX_KEY_NUM2, Key::NUM2),
        (ANYBOX_KEY_NUM3, Key::NUM3),
        (ANYBOX_KEY_NUM4, Key::NUM4),
        (ANYBOX_KEY_NUM5, Key::NUM5),
        (ANYBOX_KEY_NUM6, Key::NUM6),
        (ANYBOX_KEY_NUM7, Key::NUM7),
        (ANYBOX_KEY_NUM8, Key::NUM8),
        (ANYBOX_KEY_NUM9, Key::NUM9),
        (ANYBOX_KEY_NUM0, Key::NUM0),
        (ANYBOX_KEY_ENTER, Key::ENTER),
        (ANYBOX_KEY_ESCAPE, Key::ESCAPE),
        (ANYBOX_KEY_BACKSPACE, Key::BACKSPACE),
        (ANYBOX_KEY_TAB, Key::TAB),
        (ANYBOX_KEY_SPACE, Key::SPACE),
        (ANYBOX_KEY_MINUS, Key::MINUS),
        (ANYBOX_KEY_EQUAL, Key::EQUAL),
        (ANYBOX_KEY_LEFT_BRACKET, Key::LEFT_BRACKET),
        (ANYBOX_KEY_RIGHT_BRACKET, Key::RIGHT_BRACKET),
        (ANYBOX_KEY_BACKSLASH, Key::BACKSLASH),
        (ANYBOX_KEY_SEMICOLON, Key::SEMICOLON),
        (ANYBOX_KEY_APOSTROPHE, Key::APOSTROPHE),
        (ANYBOX_KEY_GRAVE, Key::GRAVE),
        (ANYBOX_KEY_COMMA, Key::COMMA),
        (ANYBOX_KEY_PERIOD, Key::PERIOD),
        (ANYBOX_KEY_SLASH, Key::SLASH),
        (ANYBOX_KEY_CAPS_LOCK, Key::CAPS_LOCK),
        (ANYBOX_KEY_F1, Key::F1),
        (ANYBOX_KEY_F2, Key::F2),
        (ANYBOX_KEY_F3, Key::F3),
        (ANYBOX_KEY_F4, Key::F4),
        (ANYBOX_KEY_F5, Key::F5),
        (ANYBOX_KEY_F6, Key::F6),
        (ANYBOX_KEY_F7, Key::F7),
        (ANYBOX_KEY_F8, Key::F8),
        (ANYBOX_KEY_F9, Key::F9),
        (ANYBOX_KEY_F10, Key::F10),
        (ANYBOX_KEY_F11, Key::F11),
        (ANYBOX_KEY_F12, Key::F12),
        (ANYBOX_KEY_PRINT_SCREEN, Key::PRINT_SCREEN),
        (ANYBOX_KEY_SCROLL_LOCK, Key::SCROLL_LOCK),
        (ANYBOX_KEY_PAUSE, Key::PAUSE),
        (ANYBOX_KEY_INSERT, Key::INSERT),
        (ANYBOX_KEY_HOME, Key::HOME),
        (ANYBOX_KEY_PAGE_UP, Key::PAGE_UP),
        (ANYBOX_KEY_DELETE, Key::DELETE),
        (ANYBOX_KEY_END, Key::END),
        (ANYBOX_KEY_PAGE_DOWN, Key::PAGE_DOWN),
        (ANYBOX_KEY_RIGHT, Key::RIGHT),
        (ANYBOX_KEY_LEFT, Key::LEFT),
        (ANYBOX_KEY_DOWN, Key::DOWN),
        (ANYBOX_KEY_UP, Key::UP),
        (ANYBOX_KEY_NUM_LOCK, Key::NUM_LOCK),
        (ANYBOX_KEY_KEYPAD_DIVIDE, Key::KEYPAD_DIVIDE),
        (ANYBOX_KEY_KEYPAD_MULTIPLY, Key::KEYPAD_MULTIPLY),
        (ANYBOX_KEY_KEYPAD_MINUS, Key::KEYPAD_MINUS),
        (ANYBOX_KEY_KEYPAD_PLUS, Key::KEYPAD_PLUS),
        (ANYBOX_KEY_KEYPAD_ENTER, Key::KEYPAD_ENTER),
        (ANYBOX_KEY_KEYPAD1, Key::KEYPAD1),
        (ANYBOX_KEY_KEYPAD2, Key::KEYPAD2),
        (ANYBOX_KEY_KEYPAD3, Key::KEYPAD3),
        (ANYBOX_KEY_KEYPAD4, Key::KEYPAD4),
        (ANYBOX_KEY_KEYPAD5, Key::KEYPAD5),
        (ANYBOX_KEY_KEYPAD6, Key::KEYPAD6),
        (ANYBOX_KEY_KEYPAD7, Key::KEYPAD7),
        (ANYBOX_KEY_KEYPAD8, Key::KEYPAD8),
        (ANYBOX_KEY_KEYPAD9, Key::KEYPAD9),
        (ANYBOX_KEY_KEYPAD0, Key::KEYPAD0),
        (ANYBOX_KEY_KEYPAD_PERIOD, Key::KEYPAD_PERIOD),
        (ANYBOX_KEY_MENU, Key::MENU),
        (ANYBOX_KEY_LEFT_CTRL, Key::LEFT_CTRL),
        (ANYBOX_KEY_LEFT_SHIFT, Key::LEFT_SHIFT),
        (ANYBOX_KEY_LEFT_ALT, Key::LEFT_ALT),
        (ANYBOX_KEY_LEFT_GUI, Key::LEFT_GUI),
        (ANYBOX_KEY_RIGHT_CTRL, Key::RIGHT_CTRL),
        (ANYBOX_KEY_RIGHT_SHIFT, Key::RIGHT_SHIFT),
        (ANYBOX_KEY_RIGHT_ALT, Key::RIGHT_ALT),
        (ANYBOX_KEY_RIGHT_GUI, Key::RIGHT_GUI),
    ];
    for (number, key) in keys {
        assert_eq!(number, key.0);
    }
    let media = [
        (ANYBOX_MEDIA_PLAY, MediaKey::PLAY),
        (ANYBOX_MEDIA_PAUSE, MediaKey::PAUSE),
        (ANYBOX_MEDIA_NEXT_TRACK, MediaKey::NEXT_TRACK),
        (ANYBOX_MEDIA_PREVIOUS_TRACK, MediaKey::PREVIOUS_TRACK),
        (ANYBOX_MEDIA_STOP, MediaKey::STOP),
        (ANYBOX_MEDIA_PLAY_PAUSE, MediaKey::PLAY_PAUSE),
        (ANYBOX_MEDIA_MUTE, MediaKey::MUTE),
        (ANYBOX_MEDIA_VOLUME_UP, MediaKey::VOLUME_UP),
        (ANYBOX_MEDIA_VOLUME_DOWN, MediaKey::VOLUME_DOWN),
    ];
    for (number, key) in media {
        assert_eq!(number, key.0);
    }
}

/// a mock with everything the C calls can reach, a mouse with pan and a keyboard with media keys
fn full_mock() -> anybox::medius::MockBox {
    use anybox::medius::{Caps, KbdCaps, MockBox, MouseCaps};
    MockBox::new()
        .with_caps(Caps {
            mouse: MouseCaps {
                n_buttons: 5,
                has_x: true,
                has_y: true,
                has_wheel: true,
                pan: true,
                n_hid: 1,
                ..MouseCaps::default()
            },
            ..Caps::default()
        })
        .with_kbd_caps(KbdCaps {
            n_keys: 6,
            has_consumer: true,
            ..KbdCaps::default()
        })
}

/// what the box got from one call, with no poller running so nothing else lands in between
fn frames_of(call: impl FnOnce(&AnyboxDevice)) -> Vec<(String, Vec<u8>)> {
    let mock = full_mock();
    let (device, _poller) = anybox::Device::mock_medius_manual(
        mock.clone(),
        ClickTiming::Fixed(Duration::from_millis(5)),
    )
    .unwrap();
    mock.clear_recorded();
    let device = AnyboxDevice(device);
    call(&device);
    mock.recorded_frames()
        .into_iter()
        .map(|frame| (format!("{:?}", frame.ty), frame.payload.to_vec()))
        .collect()
}

#[test]
fn every_output_call_sends_what_the_rust_call_sends() {
    use anybox::{Axis, Click, Direction};
    let click = AnyboxClick {
        hold_us: 7_000,
        has_hold: true,
        native_only: false,
    };
    let hold = Click::hold(Duration::from_millis(7));
    type CCall = Box<dyn Fn(*const AnyboxDevice) -> AnyboxStatus>;
    type RustCall = Box<dyn Fn(&anybox::Device) -> anybox::Result<()>>;
    let calls: Vec<(&str, CCall, RustCall)> = vec![
        (
            "move_by",
            Box::new(|d| unsafe { anybox_device_move_by(d, 3, -4) }),
            Box::new(|d| d.move_by(3, -4)),
        ),
        (
            "move_now",
            Box::new(|d| unsafe { anybox_device_move_now(d, -7, 2) }),
            Box::new(|d| d.move_now(-7, 2)),
        ),
        (
            "press",
            Box::new(|d| unsafe { anybox_device_press(d, ANYBOX_BUTTON_SIDE1) }),
            Box::new(|d| d.press(Button::Side1)),
        ),
        (
            "release",
            Box::new(|d| unsafe { anybox_device_release(d, ANYBOX_BUTTON_RIGHT) }),
            Box::new(|d| d.release(Button::Right)),
        ),
        (
            "click",
            Box::new(|d| unsafe { anybox_device_click(d, ANYBOX_BUTTON_MIDDLE) }),
            Box::new(|d| d.click(Button::Middle)),
        ),
        (
            "click_with",
            Box::new(move |d| unsafe { anybox_device_click_with(d, ANYBOX_BUTTON_SIDE2, &click) }),
            Box::new(move |d| d.click_with(Button::Side2, hold)),
        ),
        (
            "press_key",
            Box::new(|d| unsafe { anybox_device_press_key(d, Key::B.0) }),
            Box::new(|d| d.press_key(Key::B)),
        ),
        (
            "release_key",
            Box::new(|d| unsafe { anybox_device_release_key(d, Key::C.0) }),
            Box::new(|d| d.release_key(Key::C)),
        ),
        (
            "tap_key",
            Box::new(|d| unsafe { anybox_device_tap_key(d, Key::D.0) }),
            Box::new(|d| d.tap_key(Key::D)),
        ),
        (
            "tap_key_with",
            Box::new(move |d| unsafe { anybox_device_tap_key_with(d, Key::E.0, &click) }),
            Box::new(move |d| d.tap_key_with(Key::E, hold)),
        ),
        (
            "press_media",
            Box::new(|d| unsafe { anybox_device_press_media(d, MediaKey::MUTE.0) }),
            Box::new(|d| d.press_media(MediaKey::MUTE)),
        ),
        (
            "release_media",
            Box::new(|d| unsafe { anybox_device_release_media(d, MediaKey::PLAY.0) }),
            Box::new(|d| d.release_media(MediaKey::PLAY)),
        ),
        (
            "tap_media",
            Box::new(|d| unsafe { anybox_device_tap_media(d, MediaKey::STOP.0) }),
            Box::new(|d| d.tap_media(MediaKey::STOP)),
        ),
        (
            "tap_media_with",
            Box::new(move |d| unsafe {
                anybox_device_tap_media_with(d, MediaKey::VOLUME_UP.0, &click)
            }),
            Box::new(move |d| d.tap_media_with(MediaKey::VOLUME_UP, hold)),
        ),
        (
            "lock_button",
            Box::new(|d| unsafe { anybox_device_lock_button(d, ANYBOX_BUTTON_SIDE1) }),
            Box::new(|d| d.lock_button(Button::Side1)),
        ),
        (
            "unlock_button",
            Box::new(|d| unsafe { anybox_device_unlock_button(d, ANYBOX_BUTTON_RIGHT) }),
            Box::new(|d| d.unlock_button(Button::Right)),
        ),
        (
            "lock_key",
            Box::new(|d| unsafe { anybox_device_lock_key(d, Key::F.0) }),
            Box::new(|d| d.lock_key(Key::F)),
        ),
        (
            "unlock_key",
            Box::new(|d| unsafe { anybox_device_unlock_key(d, Key::G.0) }),
            Box::new(|d| d.unlock_key(Key::G)),
        ),
        (
            "lock_axis",
            Box::new(|d| unsafe {
                anybox_device_lock_axis(d, ANYBOX_AXIS_Y, ANYBOX_DIRECTION_NEGATIVE)
            }),
            Box::new(|d| d.lock_axis(Axis::Y, Direction::Negative)),
        ),
        (
            "unlock_axis",
            Box::new(|d| unsafe {
                anybox_device_unlock_axis(d, ANYBOX_AXIS_WHEEL, ANYBOX_DIRECTION_POSITIVE)
            }),
            Box::new(|d| d.unlock_axis(Axis::Wheel, Direction::Positive)),
        ),
        (
            "scale_axis",
            Box::new(|d| unsafe {
                anybox_device_scale_axis(d, ANYBOX_AXIS_X, ANYBOX_DIRECTION_POSITIVE, -40)
            }),
            Box::new(|d| d.scale_axis(Axis::X, Direction::Positive, -40)),
        ),
        (
            "wheel",
            Box::new(|d| unsafe { anybox_device_wheel(d, -3) }),
            Box::new(|d| d.wheel(-3)),
        ),
        (
            "pan",
            Box::new(|d| unsafe { anybox_device_pan(d, 2) }),
            Box::new(|d| d.pan(2)),
        ),
        (
            "discard_pending_moves",
            Box::new(|d| unsafe { anybox_device_discard_pending_moves(d) }),
            Box::new(|d| d.discard_pending_moves()),
        ),
    ];
    for (name, c, rust) in calls {
        let via_c = frames_of(|device| {
            let status = c(device);
            assert_eq!(status, ANYBOX_STATUS_OK, "{name}: {}", last_error());
        });
        let via_rust = frames_of(|device| rust(&device.0).unwrap());
        assert!(
            !via_rust.is_empty(),
            "{name} sent nothing, so the comparison says nothing"
        );
        assert_eq!(via_c, via_rust, "{name}");
    }
}

#[test]
fn a_named_port_is_the_one_candidate_and_its_scan_says_why_nothing_is_there() {
    // a name no listing has, so nothing real gets opened
    let port = CString::new("NOPE99").unwrap();
    let options = AnyboxConnectOptions {
        port: port.as_ptr(),
        ..AnyboxConnectOptions::default()
    };
    unsafe {
        let mut ports = ptr::null_mut();
        assert_eq!(anybox_candidates(&options, &mut ports), ANYBOX_STATUS_OK);
        assert_eq!(anybox_string_list_count(ports), 1);
        assert_eq!(text(anybox_string_list_get(ports, 0)), "NOPE99");
        assert!(anybox_string_list_get(ports, 1).is_null());
        anybox_string_list_free(ports);

        let mut list = ptr::null_mut();
        assert_eq!(anybox_scan(&options, &mut list), ANYBOX_STATUS_OK);
        assert_eq!(anybox_scan_list_count(list), 1);
        assert_eq!(text(anybox_scan_list_port(list, 0)), "NOPE99");
        assert!(anybox_scan_list_port(list, 1).is_null());
        let rust = Connect::port("NOPE99").scan().remove(0).result.unwrap_err();
        let marker = 0x1234 as *mut AnyboxSnapshot;
        let mut snapshot = marker;
        assert_eq!(
            anybox_scan_list_result(list, 0, &mut snapshot),
            status_of(&rust)
        );
        assert_ne!(status_of(&rust), ANYBOX_STATUS_OK);
        assert_eq!(last_error(), rust.to_string());
        assert_eq!(snapshot, marker);
        assert_eq!(
            anybox_scan_list_result(list, 1, &mut snapshot),
            ANYBOX_STATUS_INVALID_ARGUMENT
        );
        anybox_scan_list_free(list);
    }
}

#[test]
fn every_info_field_reaches_c() {
    use anybox::{Firmware, UsbIds};
    let mock = mock();
    let rust = unsafe { &(*mock.device).0 }.snapshot();
    let fill = |info: &anybox::Info| {
        let snapshot = AnyboxSnapshot::new(info, &rust.capabilities);
        let mut out: AnyboxInfo = unsafe { std::mem::zeroed() };
        assert_eq!(
            unsafe { anybox_snapshot_info(&snapshot, &mut out) },
            ANYBOX_STATUS_OK
        );
        // the strings only live as long as the snapshot, so they're read here
        let model = (!out.model.is_null()).then(|| text(out.model));
        let firmware_text = (!out.firmware_text.is_null()).then(|| text(out.firmware_text));
        let serial = (!out.id_serial.is_null()).then(|| text(out.id_serial));
        (out, model, firmware_text, serial)
    };

    let mut info = rust.info.clone();
    info.model = Some("Medius v2".into());
    info.firmware = Some(Firmware::Version {
        major: 3,
        minor: 4,
        patch: 5,
    });
    info.id = Some(StableId::UsbSerial("SER1".into()));
    info.ferrum_api = Some(FerrumApi::Legacy);
    info.peripheral = Some(UsbIds::new(0x046D, 0xC547));
    let (out, model, firmware_text, serial) = fill(&info);
    assert_eq!(model.as_deref(), Some("Medius v2"));
    assert_eq!(
        (
            out.firmware_kind,
            out.firmware_major,
            out.firmware_minor,
            out.firmware_patch
        ),
        (ANYBOX_FIRMWARE_VERSION, 3, 4, 5)
    );
    assert_eq!(
        (out.firmware_build, out.firmware_protocol, firmware_text),
        (0, 0, None)
    );
    assert_eq!(
        (out.id_kind, serial.as_deref(), out.id_mac),
        (ANYBOX_ID_USB_SERIAL, Some("SER1"), [0; 6])
    );
    assert_eq!(out.ferrum_api, ANYBOX_FERRUM_API_LEGACY);
    assert_eq!(
        (out.has_peripheral, out.peripheral_vid, out.peripheral_pid),
        (true, 0x046D, 0xC547)
    );

    info.model = None;
    info.id = None;
    info.peripheral = None;
    info.firmware = Some(Firmware::Build(4094));
    let (out, model, _, serial) = fill(&info);
    assert_eq!((model, serial, out.id_kind), (None, None, ANYBOX_ID_NONE));
    assert_eq!(
        (out.firmware_kind, out.firmware_build, out.firmware_protocol),
        (ANYBOX_FIRMWARE_BUILD, 4094, 0)
    );
    assert_eq!(
        (out.has_peripheral, out.peripheral_vid, out.peripheral_pid),
        (false, 0, 0)
    );

    info.firmware = Some(Firmware::Protocol(9));
    let (out, ..) = fill(&info);
    assert_eq!(
        (out.firmware_kind, out.firmware_protocol, out.firmware_build),
        (ANYBOX_FIRMWARE_PROTOCOL, 9, 0)
    );

    info.firmware = Some(Firmware::Text("V4.094".into()));
    let (out, _, firmware_text, _) = fill(&info);
    assert_eq!(
        (out.firmware_kind, firmware_text.as_deref()),
        (ANYBOX_FIRMWARE_TEXT, Some("V4.094"))
    );

    info.firmware = None;
    info.id = Some(StableId::Mac([9, 8, 7, 6, 5, 4]));
    let (out, _, firmware_text, serial) = fill(&info);
    assert_eq!(
        (out.firmware_kind, firmware_text),
        (ANYBOX_FIRMWARE_NONE, None)
    );
    assert_eq!(
        (out.id_kind, out.id_mac, serial),
        (ANYBOX_ID_MAC, [9, 8, 7, 6, 5, 4], None)
    );
}

// structured errors

fn detail() -> AnyboxErrorDetail {
    let mut out = AnyboxErrorDetail::empty(4242);
    assert_eq!(
        unsafe { anybox_last_error_detail(&mut out) },
        ANYBOX_STATUS_OK
    );
    out
}

fn optional(pointer: *const c_char) -> Option<String> {
    (!pointer.is_null()).then(|| text(pointer))
}

fn tried() -> Vec<String> {
    let mut list = ptr::null_mut();
    assert_eq!(
        unsafe { anybox_last_error_tried(&mut list) },
        ANYBOX_STATUS_OK
    );
    let count = unsafe { anybox_string_list_count(list) };
    let ports = (0..count)
        .map(|index| text(unsafe { anybox_string_list_get(list, index) }))
        .collect();
    unsafe { anybox_string_list_free(list) };
    ports
}

fn missing(list: *const AnyboxScanList, index: usize) -> Vec<u32> {
    let mut len = 0;
    unsafe {
        assert_eq!(
            anybox_scan_list_missing(list, index, ptr::null_mut(), 0, &mut len),
            ANYBOX_STATUS_OK
        );
        let mut actions = vec![0; len];
        assert_eq!(
            anybox_scan_list_missing(list, index, actions.as_mut_ptr(), len, &mut len),
            ANYBOX_STATUS_OK
        );
        actions
    }
}

#[test]
fn the_detail_is_empty_before_anything_failed_and_a_null_out_leaves_it_alone() {
    thread::spawn(|| {
        let empty = detail();
        assert_eq!((empty.kind, empty.has_action), (ANYBOX_STATUS_OK, false));
        assert!(empty.port.is_null() && empty.text.is_null());
        assert_eq!(tried(), Vec::<String>::new());
        assert_eq!(
            anybox_mock_fail(ANYBOX_STATUS_PORT_BUSY),
            ANYBOX_STATUS_PORT_BUSY
        );
        unsafe {
            assert_eq!(
                anybox_last_error_detail(ptr::null_mut()),
                ANYBOX_STATUS_INVALID_ARGUMENT
            );
            assert_eq!(
                anybox_last_error_tried(ptr::null_mut()),
                ANYBOX_STATUS_INVALID_ARGUMENT
            );
            assert_eq!(
                anybox_last_error_incapable(ptr::null_mut()),
                ANYBOX_STATUS_INVALID_ARGUMENT
            );
        }
        assert_eq!(detail().kind, ANYBOX_STATUS_PORT_BUSY);
        assert_eq!(optional(detail().port).as_deref(), Some("COM3"));
    })
    .join()
    .unwrap();
}

#[test]
fn every_error_hands_its_fields_to_c() {
    assert_eq!(
        anybox_mock_fail(ANYBOX_STATUS_NOT_FOUND),
        ANYBOX_STATUS_NOT_FOUND
    );
    assert_eq!(tried(), ["COM3", "COM4"]);
    assert_eq!(detail().kind, ANYBOX_STATUS_NOT_FOUND);

    for status in [ANYBOX_STATUS_CLOSED, ANYBOX_STATUS_DISCONNECTED] {
        assert_eq!(anybox_mock_fail(status), status);
        let empty = detail();
        assert_eq!((empty.kind, empty.has_action), (status, false));
        assert!(empty.port.is_null());
    }

    anybox_mock_fail(ANYBOX_STATUS_PERMISSION_DENIED);
    assert_eq!(optional(detail().port).as_deref(), Some("/dev/ttyACM0"));

    anybox_mock_fail(ANYBOX_STATUS_WRONG_FAMILY);
    let wrong = detail();
    assert_eq!(
        (wrong.expected_family, wrong.found_family, wrong.family),
        (
            ANYBOX_FAMILY_FERRUM,
            ANYBOX_FAMILY_MAKCU_V4,
            ANYBOX_FAMILY_ANY
        )
    );
    assert_eq!(optional(wrong.port).as_deref(), Some("COM3"));

    anybox_mock_fail(ANYBOX_STATUS_WRONG_BOX);
    let wrong = detail();
    assert_eq!(
        (
            wrong.expected_id_kind,
            optional(wrong.expected_id_serial).as_deref()
        ),
        (ANYBOX_ID_USB_SERIAL, Some("ABC123"))
    );
    assert_eq!(
        (
            wrong.found_id_kind,
            wrong.found_id_mac,
            optional(wrong.found_id_serial)
        ),
        (ANYBOX_ID_MAC, [1, 2, 3, 4, 5, 6], None)
    );

    anybox_mock_fail(ANYBOX_STATUS_WRONG_FERRUM_API);
    let wrong = detail();
    assert_eq!(
        (wrong.expected_ferrum_api, wrong.found_ferrum_api),
        (ANYBOX_FERRUM_API_SOFTWARE, ANYBOX_FERRUM_API_LEGACY)
    );

    anybox_mock_fail(ANYBOX_STATUS_FIRMWARE_TOO_OLD);
    let old = detail();
    assert_eq!(old.family, ANYBOX_FAMILY_MEDIUS);
    assert_eq!(
        (old.found_firmware_kind, old.found_firmware_protocol),
        (ANYBOX_FIRMWARE_PROTOCOL, 8)
    );
    assert_eq!(
        (
            old.needed_firmware_kind,
            old.needed_firmware_major,
            old.needed_firmware_minor,
            old.needed_firmware_patch
        ),
        (ANYBOX_FIRMWARE_VERSION, 3, 4, 0)
    );

    anybox_mock_fail(ANYBOX_STATUS_LIBRARY_TOO_OLD);
    let new = detail();
    assert_eq!(
        (
            new.family,
            new.found_firmware_kind,
            optional(new.found_firmware_text).as_deref()
        ),
        (ANYBOX_FAMILY_MAKCU_V4, ANYBOX_FIRMWARE_TEXT, Some("V9.9"))
    );
    assert_eq!(new.needed_firmware_kind, ANYBOX_FIRMWARE_NONE);

    for (status, action, family, reason) in [
        (
            ANYBOX_STATUS_UNSUPPORTED,
            ANYBOX_ACTION_KEY_PRESS,
            ANYBOX_FAMILY_MEDIUS,
            Some("no keyboard is plugged into it"),
        ),
        (
            ANYBOX_STATUS_INVALID_ARGUMENT,
            ANYBOX_ACTION_SCALE_AXIS,
            ANYBOX_FAMILY_ANY,
            Some("300 is out of range, a scale goes from -255 to 255"),
        ),
        (
            ANYBOX_STATUS_REJECTED,
            ANYBOX_ACTION_MOVE,
            ANYBOX_FAMILY_ANY,
            Some("its queue is full"),
        ),
        (
            ANYBOX_STATUS_TIMEOUT,
            ANYBOX_ACTION_LOCK_AXIS,
            ANYBOX_FAMILY_ANY,
            None,
        ),
        (
            ANYBOX_STATUS_INTERRUPTED,
            ANYBOX_ACTION_CLICK,
            ANYBOX_FAMILY_ANY,
            None,
        ),
    ] {
        assert_eq!(anybox_mock_fail(status), status);
        let about = detail();
        assert_eq!(
            (about.kind, about.has_action, about.action, about.family),
            (status, true, action, family)
        );
        assert_eq!(optional(about.text).as_deref(), reason);
    }

    anybox_mock_fail(ANYBOX_STATUS_IO);
    let io = detail();
    assert_eq!((io.has_os_error, io.os_error), (true, 5));
    let kind = if cfg!(windows) {
        ANYBOX_IO_KIND_PERMISSION_DENIED
    } else {
        ANYBOX_IO_KIND_OTHER
    };
    assert_eq!(io.io_kind, kind);
    let other = crate::status::error(anybox::Error::Io(std::io::Error::other("cable")));
    assert_eq!(other, ANYBOX_STATUS_IO);
    assert_eq!(
        (detail().io_kind, detail().has_os_error),
        (ANYBOX_IO_KIND_OTHER, false)
    );

    for status in [
        ANYBOX_STATUS_OK,
        ANYBOX_STATUS_PANIC,
        ANYBOX_STATUS_RECV_TIMEOUT,
        999,
    ] {
        assert_eq!(anybox_mock_fail(status), ANYBOX_STATUS_INVALID_ARGUMENT);
    }
}

#[test]
fn a_box_that_cant_comes_with_what_it_couldnt_do() {
    anybox_mock_fail(ANYBOX_STATUS_NO_BOX_CAN);
    let mut list = ptr::null_mut();
    unsafe {
        assert_eq!(anybox_last_error_incapable(&mut list), ANYBOX_STATUS_OK);
        assert_eq!(anybox_scan_list_count(list), 1);
        assert_eq!(text(anybox_scan_list_port(list, 0)), "COM5");
        assert_eq!(
            missing(list, 0),
            [ANYBOX_ACTION_KEY_TAP, ANYBOX_ACTION_LOCK_KEY]
        );
        let mut entry = AnyboxErrorDetail::empty(4242);
        assert_eq!(
            anybox_scan_list_error_detail(list, 0, &mut entry),
            ANYBOX_STATUS_OK
        );
        assert_eq!(entry.kind, ANYBOX_STATUS_OK);
        let mut snapshot = ptr::null_mut();
        assert_eq!(
            anybox_scan_list_result(list, 0, &mut snapshot),
            ANYBOX_STATUS_OK
        );
        let mut info: AnyboxInfo = std::mem::zeroed();
        anybox_snapshot_info(snapshot, &mut info);
        assert_eq!(info.family, ANYBOX_FAMILY_MAKCU_V4);
        anybox_snapshot_free(snapshot);
        // the list is a copy, so a later failure doesn't touch it
        anybox_mock_fail(ANYBOX_STATUS_CLOSED);
        assert_eq!(text(anybox_scan_list_port(list, 0)), "COM5");
        anybox_scan_list_free(list);

        assert_eq!(anybox_last_error_incapable(&mut list), ANYBOX_STATUS_OK);
        assert_eq!(anybox_scan_list_count(list), 0);
        anybox_scan_list_free(list);
    }
}

#[test]
fn every_failure_replaces_the_whole_last_error() {
    anybox_mock_fail(ANYBOX_STATUS_NOT_FOUND);
    assert!(detail().port.is_null());
    anybox_mock_fail(ANYBOX_STATUS_PORT_BUSY);
    assert_eq!(tried(), Vec::<String>::new());
    let mock = mock();
    unsafe { anybox_device_press(mock.device, 9999) };
    let invalid = detail();
    assert_eq!(
        (invalid.kind, invalid.has_action, invalid.port),
        (ANYBOX_STATUS_INVALID_ARGUMENT, false, ptr::null())
    );
    // a call that works leaves it as it was
    assert_eq!(
        unsafe { anybox_device_move_by(mock.device, 1, 0) },
        ANYBOX_STATUS_OK
    );
    assert_eq!(detail().kind, ANYBOX_STATUS_INVALID_ARGUMENT);
}

#[test]
fn a_scan_entry_of_a_box_that_cant_says_what_it_couldnt_do() {
    let no_box_can = anybox::Error::examples()
        .into_iter()
        .find(|error| status_of(error) == ANYBOX_STATUS_NO_BOX_CAN)
        .unwrap();
    let busy = anybox::Error::examples()
        .into_iter()
        .find(|error| status_of(error) == ANYBOX_STATUS_PORT_BUSY)
        .unwrap();
    let list = AnyboxScanList(vec![
        scan_entry("COM5".into(), Err(no_box_can)),
        scan_entry("COM3".into(), Err(busy)),
    ]);
    assert_eq!(
        missing(&list, 0),
        [ANYBOX_ACTION_KEY_TAP, ANYBOX_ACTION_LOCK_KEY]
    );
    assert_eq!(missing(&list, 1), Vec::<u32>::new());
    let mut entry = AnyboxErrorDetail::empty(4242);
    assert_eq!(
        unsafe { anybox_scan_list_error_detail(&list, 0, &mut entry) },
        ANYBOX_STATUS_OK
    );
    assert_eq!(entry.kind, ANYBOX_STATUS_NO_BOX_CAN);
}

#[test]
fn a_scan_entry_keeps_its_whole_error() {
    let port = CString::new("NOPE98").unwrap();
    let options = AnyboxConnectOptions {
        port: port.as_ptr(),
        ..AnyboxConnectOptions::default()
    };
    unsafe {
        let mut list = ptr::null_mut();
        assert_eq!(anybox_scan(&options, &mut list), ANYBOX_STATUS_OK);
        let rust = Connect::port("NOPE98").scan().remove(0).result.unwrap_err();
        let mut entry = AnyboxErrorDetail::empty(4242);
        assert_eq!(
            anybox_scan_list_error_detail(list, 0, &mut entry),
            ANYBOX_STATUS_OK
        );
        assert_eq!(entry.kind, status_of(&rust));
        assert_eq!(missing(list, 0), Vec::<u32>::new());
        assert_eq!(
            anybox_scan_list_error_detail(list, 1, &mut entry),
            ANYBOX_STATUS_INVALID_ARGUMENT
        );
        // reading the result makes it the last error, fields and all
        anybox_mock_fail(ANYBOX_STATUS_CLOSED);
        let mut snapshot = ptr::null_mut();
        assert_eq!(
            anybox_scan_list_result(list, 0, &mut snapshot),
            status_of(&rust)
        );
        assert_eq!(detail().kind, status_of(&rust));
        assert_eq!(last_error(), rust.to_string());
        anybox_scan_list_free(list);
    }
}

#[test]
fn a_snapshot_finds_a_capability_by_its_action_number() {
    let mock = mock();
    let snapshot = take_snapshot(mock.device);
    let mut capability = AnyboxCapability {
        execution: 4242,
        reason: ptr::null(),
        quirk_count: 0,
    };
    unsafe {
        // an extension number sits first and the core ones come backwards, so the place in the table is never
        // the number
        let table = &mut (*snapshot).capabilities;
        let keys = table
            .iter()
            .find(|(number, ..)| *number == ANYBOX_ACTION_KEY_PRESS)
            .cloned()
            .unwrap();
        table.reverse();
        table.insert(0, (0x10005, keys.1, keys.2));
        for (number, execution) in [
            (0x10005, ANYBOX_EXECUTION_UNSUPPORTED),
            (ANYBOX_ACTION_MOVE, ANYBOX_EXECUTION_NATIVE),
            (ANYBOX_ACTION_CLICK, ANYBOX_EXECUTION_HOST),
        ] {
            assert_eq!(
                anybox_snapshot_capability(snapshot, number, &mut capability),
                ANYBOX_STATUS_OK
            );
            assert_eq!(capability.execution, execution, "{number:#x}");
        }
        assert_eq!(
            anybox_snapshot_capability(snapshot, ANYBOX_ACTION_SCALE_AXIS, &mut capability),
            ANYBOX_STATUS_OK
        );
        assert_eq!(
            anybox_snapshot_capability(snapshot, 0x10000, &mut capability),
            ANYBOX_STATUS_INVALID_ARGUMENT
        );
        assert!(
            last_error().contains("65536 isn't an action"),
            "{}",
            last_error()
        );
        anybox_snapshot_free(snapshot);
    }
}

// the mock's other inputs

#[test]
fn the_mock_says_which_kinds_are_subscribed() {
    let mock = mock();
    assert!(!unsafe { anybox_mock_subscribed(mock.mock, ANYBOX_EVENTS_BUTTONS) });
    let events = events(mock.device, ANYBOX_EVENTS_BUTTONS);
    subscribed(&mock, ANYBOX_EVENTS_BUTTONS);
    unsafe {
        // only what a receiver asked for, and a mouse has no keys to stream
        assert!(!anybox_mock_subscribed(mock.mock, ANYBOX_EVENTS_MOTION));
        assert!(!anybox_mock_subscribed(mock.mock, ANYBOX_EVENTS_KEYS));
        assert!(!anybox_mock_subscribed(mock.mock, 1 << 7));
        anybox_events_free(events);
    }
    // the receiver going lets go of its subscription, and a new one takes it again
    let start = Instant::now();
    while unsafe { anybox_mock_subscribed(mock.mock, ANYBOX_EVENTS_BUTTONS) } {
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "the buttons stayed subscribed with no receiver"
        );
        thread::sleep(Duration::from_millis(5));
    }
    let again = self::events(mock.device, ANYBOX_EVENTS_BUTTONS);
    subscribed(&mock, ANYBOX_EVENTS_BUTTONS);
    unsafe { anybox_events_free(again) };
}

#[test]
fn the_mock_pushes_keys_media_a_link_loss_and_a_restart() {
    let mock = mock_with(true);
    let events = events(mock.device, ANYBOX_EVENTS_ALL);
    subscribed(&mock, ANYBOX_EVENTS_ALL);
    let before = crate::anybox_monotonic_ns();
    unsafe {
        let held = [ANYBOX_KEY_LEFT_SHIFT, ANYBOX_KEY_A];
        assert_eq!(
            anybox_mock_push_keys(mock.mock, 1, 100, true, held.as_ptr(), 2),
            ANYBOX_STATUS_OK
        );
        let shift = next_input(events);
        assert_eq!(
            (shift.kind, shift.key, shift.pressed),
            (ANYBOX_EVENT_KEY, ANYBOX_KEY_LEFT_SHIFT, true)
        );
        let now = crate::anybox_monotonic_ns();
        assert!(
            shift.received_ns >= before && shift.received_ns <= now,
            "{before} {} {now}",
            shift.received_ns
        );
        assert_eq!(next_input(events).key, ANYBOX_KEY_A);
        assert_eq!(
            anybox_mock_push_keys(mock.mock, 2, 200, false, ptr::null(), 0),
            ANYBOX_STATUS_OK
        );
        let released: Vec<(u8, bool)> = (0..2)
            .map(|_| next_input(events))
            .map(|event| (event.key, event.pressed))
            .collect();
        assert!(released.iter().all(|(_, pressed)| !pressed), "{released:?}");
        let volume = [ANYBOX_MEDIA_VOLUME_UP];
        anybox_mock_push_media(mock.mock, 3, 300, true, volume.as_ptr(), 1);
        let media = next_input(events);
        assert_eq!(
            (media.kind, media.media, media.pressed),
            (ANYBOX_EVENT_MEDIA, ANYBOX_MEDIA_VOLUME_UP, true)
        );
        assert_eq!(
            anybox_mock_push_keys(mock.mock, 4, 0, true, ptr::null(), 1),
            ANYBOX_STATUS_INVALID_ARGUMENT
        );

        let until = |kind: u32| {
            let start = Instant::now();
            loop {
                assert!(start.elapsed() < Duration::from_secs(10), "no event {kind}");
                let (status, event) = recv(events, 500_000);
                if status == ANYBOX_STATUS_OK && event.kind == kind {
                    return;
                }
            }
        };
        assert_eq!(anybox_mock_link_lost(mock.mock), ANYBOX_STATUS_OK);
        until(ANYBOX_EVENT_SESSION_RESET);
        assert_eq!(anybox_mock_restart(mock.mock), ANYBOX_STATUS_OK);
        until(ANYBOX_EVENT_DISCONNECTED);
        until(ANYBOX_EVENT_RECONNECTED);
        anybox_events_free(events);
    }
}
