use std::sync::{Mutex, PoisonError};

use anybox::medius::{CatchFilter, Class, Direction, FrameType, KbdCaps, MockBox, Usage};
use anybox::{Device, EventKinds, MockPoller};

use crate::device::AnyboxDevice;
use crate::snapshot::copy_out;
use crate::status::{
    AnyboxStatus, error, guard, guard_free, invalid, read, slice, status_of, write,
};
use crate::values::{AnyboxClickTiming, event_kinds};

/// the medius crate's model of a Medius box, for testing code that uses anybox with no box plugged in. it proves
/// nothing about hardware and it can only be shaped like a Medius
pub struct AnyboxMock(pub(crate) MockBox, Option<Mutex<MockPoller>>);

/// a device over a new mock box, a five button mouse with a wheel, and with `keyboard` a six key keyboard with media
/// keys behind it too. it runs the exact code a real Medius gets, checking the link every 250 ms like a real one.
/// null `click_timing` is the default. free the mock with `anybox_mock_free` and the device with
/// `anybox_device_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_mock_medius_open(
    click_timing: *const AnyboxClickTiming,
    keyboard: bool,
    mock: *mut *mut AnyboxMock,
    out: *mut *mut AnyboxDevice,
) -> AnyboxStatus {
    unsafe { open(click_timing, keyboard, mock, out, false) }
}

/// the same as `anybox_mock_medius_open`, except that the link only gets checked on `anybox_mock_tick`, so a test
/// decides when a link event gets noticed and nothing lands between the frames it records
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_mock_medius_open_manual(
    click_timing: *const AnyboxClickTiming,
    keyboard: bool,
    mock: *mut *mut AnyboxMock,
    out: *mut *mut AnyboxDevice,
) -> AnyboxStatus {
    unsafe { open(click_timing, keyboard, mock, out, true) }
}

unsafe fn open(
    click_timing: *const AnyboxClickTiming,
    keyboard: bool,
    mock: *mut *mut AnyboxMock,
    out: *mut *mut AnyboxDevice,
    manual: bool,
) -> AnyboxStatus {
    guard(|| {
        // SAFETY: the header says a non-null click timing points at a whole `AnyboxClickTiming`
        let timing = match unsafe { click_timing.as_ref() } {
            Some(timing) => crate::values::click_timing(timing)?,
            None => anybox::ClickTiming::default(),
        };
        let mock = write(mock, "mock")?;
        let out = write(out, "out")?;
        let mock_box = if keyboard {
            MockBox::new().with_kbd_caps(KbdCaps {
                n_keys: 6,
                has_consumer: true,
                ..KbdCaps::default()
            })
        } else {
            MockBox::new()
        };
        let (device, poller) = if manual {
            let (device, poller) =
                Device::mock_medius_manual(mock_box.clone(), timing).map_err(error)?;
            (device, Some(Mutex::new(poller)))
        } else {
            (
                Device::mock_medius(mock_box.clone(), timing).map_err(error)?,
                None,
            )
        };
        mock.set(Box::into_raw(Box::new(AnyboxMock(mock_box, poller))));
        out.set(AnyboxDevice::boxed(device));
        Ok(())
    })
}

/// whether the mock box got the subscription of every kind in `kinds` (`ANYBOX_EVENTS_*` combined with `|`) yet.
/// a receiver subscribes each kind on the device's own threads, the keys and media a moment after the buttons, and
/// the box drops what's pushed before its kind is in like a real one would, so wait for this before you push. it's
/// false for a null mock or a bit that isn't a kind
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_mock_subscribed(mock: *const AnyboxMock, kinds: u32) -> bool {
    // SAFETY: the header says a non-null mock came from `anybox_mock_medius_open` and wasn't freed
    let (Some(mock), Ok(kinds)) = (unsafe { mock.as_ref() }, event_kinds(kinds)) else {
        return false;
    };
    let frames = mock.0.recorded_frames();
    // the filters anybox's Medius side subscribes with, a wrong one only makes a wait run out
    [
        (EventKinds::BUTTONS, CatchFilter::watch_class(Class::Button)),
        (EventKinds::MOTION, CatchFilter::watch_axes()),
        (EventKinds::KEYS, CatchFilter::watch_class(Class::Key)),
        (EventKinds::MEDIA, CatchFilter::watch_class(Class::Media)),
    ]
    .into_iter()
    .filter(|(kind, _)| kinds.contains(*kind))
    .all(|(_, filter)| {
        let (class, id) = filter.wire();
        let [low, high] = id.to_le_bytes();
        let key = [class, low, high, filter.direction().as_u8()];
        // a catch frame is the class, the id, the direction and then 1 to subscribe or 0 to let go, the way medius
        // writes it, so the last one for the filter says where it is. a reset lets go of every class (0xFF) at once
        frames
            .iter()
            .rev()
            .find(|frame| {
                frame.ty == FrameType::Catch
                    && frame.payload.len() > 4
                    && (frame.payload[..4] == key || frame.payload[0] == 0xFF)
            })
            .is_some_and(|frame| frame.payload[4] == 1)
    })
}

/// physical motion as if from the box. `seq` is the box's frame counter and `ts_us` its clock.
/// a gap in `seq` doesn't show up as a drop
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_mock_push_motion(
    mock: *const AnyboxMock,
    seq: u8,
    ts_us: u32,
    dx: i16,
    dy: i16,
    wheel: i16,
    pan: i16,
) -> AnyboxStatus {
    guard(|| {
        unsafe { read(mock, "mock") }?
            .0
            .push_motion(seq, ts_us, dx, dy, wheel, pan);
        Ok(())
    })
}

/// the physical buttons held now, as if from the box, bit n for the box's button n, so bits 0 to 4 are left to side2
/// and bit n from 5 up comes out as `ANYBOX_BUTTON_EXTRA_BASE` + n. it's the whole held set,
/// so a release is pushing the set without that button, and `pressed` says whether the set grew or shrank
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_mock_push_buttons(
    mock: *const AnyboxMock,
    seq: u8,
    ts_us: u32,
    pressed: bool,
    held: u32,
) -> AnyboxStatus {
    guard(|| {
        let mock = unsafe { read(mock, "mock") }?;
        if held >= 1 << 16 {
            return Err(invalid(format!(
                "{held:#x} holds buttons past 15, the mock only has 16"
            )));
        }
        let usages: Vec<Usage> = (0..16)
            .filter(|bit| held & (1 << bit) != 0)
            .map(|bit| Usage::new(Class::Button, bit))
            .collect();
        mock.0
            .push_usages(seq, ts_us, Class::Button, edge(pressed), &usages);
        Ok(())
    })
}

/// the physical keys held now, as if from the box, `len` HID usages from `keys`, modifiers included. it's the whole
/// held set like `anybox_mock_push_buttons`, so the last release is pushing no keys at all. `keys` can be null when
/// `len` is 0
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_mock_push_keys(
    mock: *const AnyboxMock,
    seq: u8,
    ts_us: u32,
    pressed: bool,
    keys: *const u8,
    len: usize,
) -> AnyboxStatus {
    guard(|| {
        let mock = unsafe { read(mock, "mock") }?;
        let keys = unsafe { slice(keys, len, "keys") }?;
        let usages: Vec<Usage> = keys
            .iter()
            .map(|key| Usage::new(Class::Key, u16::from(*key)))
            .collect();
        mock.0
            .push_usages(seq, ts_us, Class::Key, edge(pressed), &usages);
        Ok(())
    })
}

/// the media keys held now, as if from the box, `len` consumer usages from `media`, the whole held set like the keys
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_mock_push_media(
    mock: *const AnyboxMock,
    seq: u8,
    ts_us: u32,
    pressed: bool,
    media: *const u16,
    len: usize,
) -> AnyboxStatus {
    guard(|| {
        let mock = unsafe { read(mock, "mock") }?;
        let media = unsafe { slice(media, len, "media") }?;
        let usages: Vec<Usage> = media
            .iter()
            .map(|key| Usage::new(Class::Media, *key))
            .collect();
        mock.0
            .push_usages(seq, ts_us, Class::Media, edge(pressed), &usages);
        Ok(())
    })
}

/// one pass of what a real device checks on the link every 250 ms, on a mock from
/// `anybox_mock_medius_open_manual`, which is where it notices `anybox_mock_link_lost` and `anybox_mock_restart`.
/// the box drop counter and the clone are only looked at every fourth pass, and the mock keeps its own timers (a
/// restarted clone is back 100 ms later), so a test still waits for those
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_mock_tick(mock: *const AnyboxMock) -> AnyboxStatus {
    guard(|| {
        let poller = unsafe { read(mock, "mock") }?.1.as_ref().ok_or_else(|| {
            invalid(
                "this mock checks its link by itself, open it with anybox_mock_medius_open_manual to tick it",
            )
        })?;
        poller.lock().unwrap_or_else(PoisonError::into_inner).tick();
        Ok(())
    })
}

/// how many frames the mock box got since it opened or since `anybox_mock_clear_recorded`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_mock_recorded(mock: *const AnyboxMock) -> usize {
    // SAFETY: the header says a non-null mock came from `anybox_mock_medius_open` and wasn't freed
    unsafe { mock.as_ref() }.map_or(0, |mock| mock.0.recorded())
}

/// frame number `index` the mock box got, its type byte (the protocol's, `MOVE` is 1) in `type` and its payload
/// copied into `payload`, `len` getting the payload's whole length even when `cap` was too small
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_mock_recorded_frame(
    mock: *const AnyboxMock,
    index: usize,
    r#type: *mut u8,
    payload: *mut u8,
    cap: usize,
    len: *mut usize,
) -> AnyboxStatus {
    guard(|| {
        let mock = unsafe { read(mock, "mock") }?;
        let r#type = write(r#type, "type")?;
        let frames = mock.0.recorded_frames();
        let frame = frames.get(index).ok_or_else(|| {
            invalid(format!(
                "the mock only got {} frames, there's no frame {index}",
                frames.len()
            ))
        })?;
        unsafe { copy_out(&frame.payload, payload, cap, len) }?;
        r#type.set(frame.ty as u8);
        Ok(())
    })
}

/// forgets every frame the mock box got so far
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_mock_clear_recorded(mock: *const AnyboxMock) -> AnyboxStatus {
    guard(|| {
        unsafe { read(mock, "mock") }?.0.clear_recorded();
        Ok(())
    })
}

/// sends every receiver of `device` one event of every kind, with every field filled in, through the same queue
/// as real ones, so you can test how your code reads each kind with no box. none of them does to the device what
/// the real one would, so the device stays connected through the example `ANYBOX_EVENT_DISCONNECTED`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_mock_example_events(device: *const AnyboxDevice) -> AnyboxStatus {
    guard(|| {
        unsafe { read(device, "device") }?.0.mock_example_events();
        Ok(())
    })
}

/// the link between the box's two chips drops and comes back, so the box lets go of what it held and the device
/// sends `ANYBOX_EVENT_SESSION_RESET` once it checks the link (within 250 ms, or on the next `anybox_mock_tick` of a
/// manual mock)
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_mock_link_lost(mock: *const AnyboxMock) -> AnyboxStatus {
    guard(|| {
        unsafe { read(mock, "mock") }?.0.link_lost();
        Ok(())
    })
}

/// the box restarts, so the device sends `ANYBOX_EVENT_DISCONNECTED` and then `ANYBOX_EVENT_RECONNECTED`, noticed the
/// way `anybox_mock_link_lost` is
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_mock_restart(mock: *const AnyboxMock) -> AnyboxStatus {
    guard(|| {
        unsafe { read(mock, "mock") }?.0.restart();
        Ok(())
    })
}

fn edge(pressed: bool) -> Direction {
    if pressed {
        Direction::PRESS
    } else {
        Direction::RELEASE
    }
}

/// makes the last error on this thread anybox's example of the error with this status, with every field it has
/// filled in, and returns the status, so you can test how your code handles each one with no box. a status with no
/// example (`ANYBOX_STATUS_OK`, the ones only this C ABI makes) is `ANYBOX_STATUS_INVALID_ARGUMENT`
#[unsafe(no_mangle)]
pub extern "C" fn anybox_mock_fail(status: AnyboxStatus) -> AnyboxStatus {
    guard(|| {
        let example = anybox::Error::examples()
            .into_iter()
            .find(|example| status_of(example) == status)
            .ok_or_else(|| invalid(format!("anybox has no example of status {status}")))?;
        Err(error(example))
    })
}

/// null does nothing
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_mock_free(mock: *mut AnyboxMock) {
    guard_free(|| {
        if !mock.is_null() {
            // SAFETY: it came from `Box::into_raw` in this library and the header says it's freed once
            drop(unsafe { Box::from_raw(mock) });
        }
    });
}

/// a device over anybox's own model of a MAKCU V4.108 with a mouse and a keyboard behind it, for testing the
/// `anybox_makcu_` calls with no box. null `click_timing` is the default. free it with `anybox_device_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_mock_makcu_open(
    click_timing: *const AnyboxClickTiming,
    out: *mut *mut AnyboxDevice,
) -> AnyboxStatus {
    unsafe { open_family(click_timing, out, Device::mock_makcu) }
}

/// a device over anybox's own model of a YSC8K V1, for testing the `anybox_ysc_` calls with no box
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_mock_ysc_open(
    click_timing: *const AnyboxClickTiming,
    out: *mut *mut AnyboxDevice,
) -> AnyboxStatus {
    unsafe { open_family(click_timing, out, Device::mock_ysc) }
}

/// a device over anybox's own model of a Ferrum on the app's port, for testing the `anybox_ferrum_` calls with no box
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_mock_ferrum_open(
    click_timing: *const AnyboxClickTiming,
    out: *mut *mut AnyboxDevice,
) -> AnyboxStatus {
    unsafe { open_family(click_timing, out, Device::mock_ferrum) }
}

unsafe fn open_family(
    click_timing: *const AnyboxClickTiming,
    out: *mut *mut AnyboxDevice,
    open: fn(anybox::ClickTiming) -> anybox::Result<Device>,
) -> AnyboxStatus {
    guard(|| {
        // SAFETY: the header says a non-null click timing points at a whole `AnyboxClickTiming`
        let timing = match unsafe { click_timing.as_ref() } {
            Some(timing) => crate::values::click_timing(timing)?,
            None => anybox::ClickTiming::default(),
        };
        let out = write(out, "out")?;
        out.set(AnyboxDevice::boxed(open(timing).map_err(error)?));
        Ok(())
    })
}
