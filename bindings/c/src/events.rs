use std::time::Duration;

use anybox::{Clock, DropSource, Event, Events, RecvTimeoutError, Stamp, TryRecvError};

use crate::status::{
    ANYBOX_STATUS_RECV_CLOSED, ANYBOX_STATUS_RECV_TIMEOUT, AnyboxStatus, fail, guard, guard_free,
    read, write,
};
use crate::values::{action_number, button_number, nanos};

/// `button`, `pressed` and the stamp
pub const ANYBOX_EVENT_BUTTON: u32 = 0;
/// a physical key, `key`, `pressed` and the stamp
pub const ANYBOX_EVENT_KEY: u32 = 1;
/// `media`, `pressed` and the stamp
pub const ANYBOX_EVENT_MEDIA: u32 = 2;
/// physical motion, `dx`, `dy`, `wheel`, `pan` and the stamp. the wheel is positive going up and pan is positive
/// going right
pub const ANYBOX_EVENT_MOTION: u32 = 3;
/// `count` events never reached you since the last one from the same `source`, see `Event::Dropped` in the Rust
/// docs
pub const ANYBOX_EVENT_DROPPED: u32 = 4;
/// the box lost its state, so the buttons and keys you were holding got released (on a box with
/// `ANYBOX_QUIRK_KEEPS_HELD` they stay held). what you knew about which physical buttons and keys were held is out
/// of date too, because one held through the reset can show a second press or never show its release
pub const ANYBOX_EVENT_SESSION_RESET: u32 = 5;
/// the box refused `count` of `action` after the call already returned ok, slow down and send it again if it
/// mattered
pub const ANYBOX_EVENT_REJECTED: u32 = 6;
pub const ANYBOX_EVENT_DISCONNECTED: u32 = 7;
/// the box is back, the buttons and keys you were holding got released (on a box with `ANYBOX_QUIRK_KEEPS_HELD`
/// they stay held), and the snapshot may have changed. on a MAKCU you also get this when a mouse or keyboard gets
/// plugged into the box or pulled out, and on a Medius when the device it clones gets swapped. what you knew about
/// which physical buttons and keys were held is out of date, same as after `ANYBOX_EVENT_SESSION_RESET`
pub const ANYBOX_EVENT_RECONNECTED: u32 = 8;
/// the snapshot changed with no reconnect, so read it again
pub const ANYBOX_EVENT_CHANGED: u32 = 9;

/// the box's own counters
pub const ANYBOX_DROP_SOURCE_DEVICE: u32 = 0;
/// your receiver fell behind and its queue overflowed
pub const ANYBOX_DROP_SOURCE_HOST: u32 = 1;

pub const ANYBOX_CLOCK_MEDIUS_HOST_CHIP: u32 = 0;
pub const ANYBOX_CLOCK_MEDIUS_DEVICE_CHIP: u32 = 1;

/// a receiver of events from one device. it can be read from any thread, and two threads reading the same one
/// split the events between them
pub struct AnyboxEvents(Events);

impl AnyboxEvents {
    pub(crate) fn boxed(events: Events) -> *mut Self {
        Box::into_raw(Box::new(Self(events)))
    }
}

/// one event. which fields mean something depends on `kind`, the rest are 0. the stamp fields are set on button,
/// key, media and motion events
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnyboxEvent {
    pub kind: u32,
    pub button: u32,
    pub key: u8,
    pub pressed: bool,
    pub media: u16,
    pub dx: i32,
    pub dy: i32,
    pub wheel: i32,
    pub pan: i32,
    pub source: u32,
    pub action: u32,
    pub count: u64,
    /// the best guess at when it happened, as the time since the device was opened. it's the one to compare events
    /// by
    pub since_open_ns: u64,
    /// when anybox got it, as the time since the device was opened
    pub received_since_open_ns: u64,
    /// the raw QueryPerformanceCounter value when anybox got it, Windows only
    pub received_qpc: i64,
    pub has_received_qpc: bool,
    /// the box's own clock, only compare the ones from the same `box_clock`
    pub has_box_time: bool,
    pub box_clock: u32,
    pub box_time_us: u32,
}

fn stamped(kind: u32, at: &Stamp) -> AnyboxEvent {
    AnyboxEvent {
        kind,
        since_open_ns: nanos(at.since_open),
        received_since_open_ns: nanos(at.received_since_open),
        received_qpc: at.received_qpc.unwrap_or(0),
        has_received_qpc: at.received_qpc.is_some(),
        has_box_time: at.box_time.is_some(),
        box_clock: at.box_time.map_or(0, |time| clock_number(time.clock)),
        box_time_us: at.box_time.map_or(0, |time| time.micros),
        ..AnyboxEvent::default()
    }
}

fn clock_number(clock: Clock) -> u32 {
    match clock {
        Clock::MediusHostChip => ANYBOX_CLOCK_MEDIUS_HOST_CHIP,
        Clock::MediusDeviceChip => ANYBOX_CLOCK_MEDIUS_DEVICE_CHIP,
        other => panic!("the C ABI has no number for {other:?} yet"),
    }
}

pub(crate) fn event_struct(event: &Event) -> AnyboxEvent {
    let plain = |kind| AnyboxEvent {
        kind,
        ..AnyboxEvent::default()
    };
    match event {
        Event::Button {
            button,
            pressed,
            at,
            ..
        } => AnyboxEvent {
            button: button_number(*button),
            pressed: *pressed,
            ..stamped(ANYBOX_EVENT_BUTTON, at)
        },
        Event::Key {
            key, pressed, at, ..
        } => AnyboxEvent {
            key: key.0,
            pressed: *pressed,
            ..stamped(ANYBOX_EVENT_KEY, at)
        },
        Event::Media {
            key, pressed, at, ..
        } => AnyboxEvent {
            media: key.0,
            pressed: *pressed,
            ..stamped(ANYBOX_EVENT_MEDIA, at)
        },
        Event::Motion {
            dx,
            dy,
            wheel,
            pan,
            at,
            ..
        } => AnyboxEvent {
            dx: *dx,
            dy: *dy,
            wheel: *wheel,
            pan: *pan,
            ..stamped(ANYBOX_EVENT_MOTION, at)
        },
        Event::Dropped { count, source, .. } => AnyboxEvent {
            count: *count,
            source: match source {
                DropSource::Device => ANYBOX_DROP_SOURCE_DEVICE,
                DropSource::Host => ANYBOX_DROP_SOURCE_HOST,
                other => panic!("the C ABI has no number for {other:?} yet"),
            },
            ..plain(ANYBOX_EVENT_DROPPED)
        },
        Event::SessionReset => plain(ANYBOX_EVENT_SESSION_RESET),
        Event::Rejected { action, count, .. } => AnyboxEvent {
            action: action_number(*action),
            count: *count,
            ..plain(ANYBOX_EVENT_REJECTED)
        },
        Event::Disconnected => plain(ANYBOX_EVENT_DISCONNECTED),
        Event::Reconnected => plain(ANYBOX_EVENT_RECONNECTED),
        Event::Changed => plain(ANYBOX_EVENT_CHANGED),
        other => panic!("the C ABI has no number for {other:?} yet"),
    }
}

fn timed_out() -> AnyboxStatus {
    fail(
        ANYBOX_STATUS_RECV_TIMEOUT,
        RecvTimeoutError::Timeout.to_string(),
    )
}

fn ended() -> AnyboxStatus {
    fail(ANYBOX_STATUS_RECV_CLOSED, anybox::Closed.to_string())
}

/// the next event. a negative `timeout_us` waits as long as it takes, 0 doesn't wait at all, and anything else
/// waits up to that many microseconds. it's `ANYBOX_STATUS_RECV_TIMEOUT` when nothing came, and
/// `ANYBOX_STATUS_RECV_CLOSED` once the device is gone and everything queued was read, so a reader that loops until
/// then gets every event. closing the device from another thread wakes it
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_events_recv(
    events: *const AnyboxEvents,
    timeout_us: i64,
    out: *mut AnyboxEvent,
) -> AnyboxStatus {
    guard(|| {
        let events = &unsafe { read(events, "events") }?.0;
        let out = write(out, "out")?;
        let event = match timeout_us {
            ..0 => events.recv().map_err(|_| ended())?,
            0 => events.try_recv().map_err(|error| match error {
                TryRecvError::Empty => timed_out(),
                TryRecvError::Closed => ended(),
            })?,
            _ => events
                .recv_timeout(Duration::from_micros(timeout_us as u64))
                .map_err(|error| match error {
                    RecvTimeoutError::Timeout => timed_out(),
                    RecvTimeoutError::Closed => ended(),
                })?,
        };
        out.set(event_struct(&event));
        Ok(())
    })
}

/// null does nothing. it must not run while another thread is in `anybox_events_recv` on it, close the device to
/// wake that one first
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_events_free(events: *mut AnyboxEvents) {
    guard_free(|| {
        if !events.is_null() {
            // SAFETY: it came from `Box::into_raw` in this library and the header says it's freed once
            drop(unsafe { Box::from_raw(events) });
        }
    });
}
