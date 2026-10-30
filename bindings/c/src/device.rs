use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anybox::{Device, Family, Key, MediaKey};
use medius_capi::MediusDevice;

use crate::events::AnyboxEvents;
use crate::snapshot::AnyboxSnapshot;
use crate::status::{
    ANYBOX_STATUS_UNSUPPORTED, AnyboxStatus, error, fail, guard, guard_free, read, write,
};
use crate::values::{self, AnyboxClick, event_kinds};

/// one open box. every call on it can come from any thread at the same time, only `anybox_device_free` must not
/// run while another call on it is still going
pub struct AnyboxDevice(
    pub(crate) Device,
    // set by `anybox_device_close`, the only way to close it from C, so a call can say closed before it looks at
    // which box it is
    pub(crate) AtomicBool,
);

impl AnyboxDevice {
    pub(crate) fn boxed(device: Device) -> *mut Self {
        Box::into_raw(Box::new(Self(device, AtomicBool::new(false))))
    }

    pub(crate) fn closed(&self) -> bool {
        self.1.load(Ordering::Acquire)
    }
}

unsafe fn device_ref<'a>(device: *const AnyboxDevice) -> Result<&'a Device, AnyboxStatus> {
    Ok(&unsafe { read(device, "device") }?.0)
}

/// who the box is and what it can do, both read from the current connection. it can change after
/// `ANYBOX_EVENT_RECONNECTED` or `ANYBOX_EVENT_CHANGED`. free it with `anybox_snapshot_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_snapshot(
    device: *const AnyboxDevice,
    out: *mut *mut AnyboxSnapshot,
) -> AnyboxStatus {
    guard(|| {
        let device = unsafe { device_ref(device) }?;
        let out = write(out, "out")?;
        let snapshot: Arc<anybox::Snapshot> = device.snapshot();
        let snapshot = AnyboxSnapshot::new(&snapshot.info, &snapshot.capabilities);
        out.set(Box::into_raw(Box::new(snapshot)));
        Ok(())
    })
}

// made by bindings/gen from its call table (calls), change the table and run cargo run -p anybox-gen
/// moves by counts, going through whatever smoothing the box has on
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_move_by(
    device: *const AnyboxDevice,
    dx: i16,
    dy: i16,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .move_by(dx, dy)
            .map_err(error)
    })
}

/// moves by counts on the box's next report, skipping its smoothing for this move
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_move_now(
    device: *const AnyboxDevice,
    dx: i16,
    dy: i16,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .move_now(dx, dy)
            .map_err(error)
    })
}

/// holds a button down until `anybox_device_release`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_press(
    device: *const AnyboxDevice,
    button: u32,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .press(values::button(button)?)
            .map_err(error)
    })
}

/// lets go of a button
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_release(
    device: *const AnyboxDevice,
    button: u32,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .release(values::button(button)?)
            .map_err(error)
    })
}

/// the box's own click, or a click timed on the PC with the device's click timing if the box doesn't have one
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_click(
    device: *const AnyboxDevice,
    button: u32,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .click(values::button(button)?)
            .map_err(error)
    })
}

/// the box's own click, or one timed on the PC with the device's click timing if the box has none, which blocks the
/// thread for the hold. `click` says how long to hold it and whether to fail instead of letting anybox time it from
/// the PC
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_click_with(
    device: *const AnyboxDevice,
    button: u32,
    click: *const AnyboxClick,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .click_with(
                values::button(button)?,
                values::click(unsafe { read(click, "click") }?),
            )
            .map_err(error)
    })
}

/// holds a key down until `anybox_device_release_key`. a key is its HID usage, from 0x04 to 0xE7
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_press_key(
    device: *const AnyboxDevice,
    key: u8,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .press_key(Key(key))
            .map_err(error)
    })
}

/// lets go of a key
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_release_key(
    device: *const AnyboxDevice,
    key: u8,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .release_key(Key(key))
            .map_err(error)
    })
}

/// presses and lets go, timed by the box, or on the PC with the device's click timing if the box can't
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_tap_key(
    device: *const AnyboxDevice,
    key: u8,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .tap_key(Key(key))
            .map_err(error)
    })
}

/// presses and lets go, timed by the box, or on the PC with the device's click timing if the box can't, which blocks
/// the thread for the hold. `tap` says how long to hold it and whether to fail instead of letting anybox time it
/// from the PC
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_tap_key_with(
    device: *const AnyboxDevice,
    key: u8,
    tap: *const AnyboxClick,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .tap_key_with(Key(key), values::click(unsafe { read(tap, "tap") }?))
            .map_err(error)
    })
}

/// types the text on the box with the box's own timing, as US layout key presses, and returns once the box has typed
/// all of it. it takes printable ASCII only, so tap keys like Enter yourself after it. other calls on the device can
/// wait while it types, and a close stops it after the few characters going out. after an error part of it may
/// already be typed
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_type_text(
    device: *const AnyboxDevice,
    text: *const std::ffi::c_char,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .type_text(unsafe { crate::status::text(text, "text") }?)
            .map_err(error)
    })
}

/// holds the keys down with one command of the box, up to 14 of them, until `anybox_device_release_keys`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_press_keys(
    device: *const AnyboxDevice,
    keys: *const u8,
    len: usize,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .press_keys(&values::keys(unsafe {
                crate::status::slice(keys, len, "keys")
            }?))
            .map_err(error)
    })
}

/// lets go of the keys with one command of the box, or one at a time like `anybox_device_release_key` on a box that
/// can't, so a key you hold always gets let go. a key you don't hold still has to be one the box can let go of, so
/// it can fail as unsupported after the held ones already went
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_release_keys(
    device: *const AnyboxDevice,
    keys: *const u8,
    len: usize,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .release_keys(&values::keys(unsafe {
                crate::status::slice(keys, len, "keys")
            }?))
            .map_err(error)
    })
}

/// presses and lets go of the keys with one command of the box, timed by the box
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_tap_keys(
    device: *const AnyboxDevice,
    keys: *const u8,
    len: usize,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .tap_keys(&values::keys(unsafe {
                crate::status::slice(keys, len, "keys")
            }?))
            .map_err(error)
    })
}

/// holds a media key down until `anybox_device_release_media`. a media key is its HID consumer usage
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_press_media(
    device: *const AnyboxDevice,
    key: u16,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .press_media(MediaKey(key))
            .map_err(error)
    })
}

/// lets go of a media key
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_release_media(
    device: *const AnyboxDevice,
    key: u16,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .release_media(MediaKey(key))
            .map_err(error)
    })
}

/// presses and lets go of a media key, timed like a key's tap
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_tap_media(
    device: *const AnyboxDevice,
    key: u16,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .tap_media(MediaKey(key))
            .map_err(error)
    })
}

/// presses and lets go of a media key, timed like a key's tap, which blocks the thread for the hold. `tap` says how
/// long to hold it and whether to fail instead of letting anybox time it from the PC
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_tap_media_with(
    device: *const AnyboxDevice,
    key: u16,
    tap: *const AnyboxClick,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .tap_media_with(MediaKey(key), values::click(unsafe { read(tap, "tap") }?))
            .map_err(error)
    })
}

/// keeps the physical button from reaching the PC until `anybox_device_unlock_button`. your own presses still go
/// through and its events still come, so you can take the button over
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_lock_button(
    device: *const AnyboxDevice,
    button: u32,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .lock_button(values::button(button)?)
            .map_err(error)
    })
}

/// lets the physical button through again
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_unlock_button(
    device: *const AnyboxDevice,
    button: u32,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .unlock_button(values::button(button)?)
            .map_err(error)
    })
}

/// the PC sees the key up from now on, whatever the finger does, until `anybox_device_restore_key`. key events keep
/// reporting the finger, so you can tell whether it's still held. a key anybox holds or has locked itself, or one a
/// text needs while it types, is refused, since each box would do something else with it
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_force_release_key(
    device: *const AnyboxDevice,
    key: u8,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .force_release_key(Key(key))
            .map_err(error)
    })
}

/// gives the PC the finger's state of the key again, so a key that's still held goes down for it. it also clears a
/// force another program left, except on a key anybox holds or has locked
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_restore_key(
    device: *const AnyboxDevice,
    key: u8,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .restore_key(Key(key))
            .map_err(error)
    })
}

/// keeps the physical key from reaching the PC until `anybox_device_unlock_key`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_lock_key(
    device: *const AnyboxDevice,
    key: u8,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .lock_key(Key(key))
            .map_err(error)
    })
}

/// lets the physical key through again
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_unlock_key(
    device: *const AnyboxDevice,
    key: u8,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .unlock_key(Key(key))
            .map_err(error)
    })
}

/// keeps physical motion one way or both ways along an axis from reaching the PC until `anybox_device_unlock_axis`.
/// your own moves still go through
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_lock_axis(
    device: *const AnyboxDevice,
    axis: u32,
    direction: u32,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .lock_axis(values::axis(axis)?, values::direction(direction)?)
            .map_err(error)
    })
}

/// unlocks that way along the axis and leaves the other way as it was. it also undoes a scale there
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_unlock_axis(
    device: *const AnyboxDevice,
    axis: u32,
    direction: u32,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .unlock_axis(values::axis(axis)?, values::direction(direction)?)
            .map_err(error)
    })
}

/// keeps `percent` of the physical motion one way or both ways along an axis, from -255 to 255. 100 leaves it alone,
/// 0 blocks it and -100 inverts it. `anybox_device_unlock_axis` puts it back to 100
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_scale_axis(
    device: *const AnyboxDevice,
    axis: u32,
    direction: u32,
    percent: i16,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .scale_axis(values::axis(axis)?, values::direction(direction)?, percent)
            .map_err(error)
    })
}

/// positive scrolls up
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_wheel(
    device: *const AnyboxDevice,
    notches: i16,
) -> AnyboxStatus {
    guard(|| unsafe { device_ref(device) }?.wheel(notches).map_err(error))
}

/// horizontal scroll, positive goes right
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_pan(
    device: *const AnyboxDevice,
    notches: i16,
) -> AnyboxStatus {
    guard(|| unsafe { device_ref(device) }?.pan(notches).map_err(error))
}

/// throws away the moves the box took but hasn't sent yet
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_discard_pending_moves(
    device: *const AnyboxDevice,
) -> AnyboxStatus {
    guard(|| {
        unsafe { device_ref(device) }?
            .discard_pending_moves()
            .map_err(error)
    })
}

// the end of what bindings/gen makes (calls)

/// a new receiver for these kinds of input, `ANYBOX_EVENTS_*` combined with `|`, plus the connection events. the
/// box only streams while at least one receiver is alive, and it streams what any receiver wants. a kind this box
/// can't stream gets one `ANYBOX_EVENT_REJECTED` right away, except with `ANYBOX_EVENTS_ALL`, which means
/// everything this box has. just know that a receiver of keys sees every key typed on the keyboard behind the box,
/// passwords included. free it with `anybox_events_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_events(
    device: *const AnyboxDevice,
    kinds: u32,
    out: *mut *mut AnyboxEvents,
) -> AnyboxStatus {
    guard(|| {
        let device = unsafe { device_ref(device) }?;
        let kinds = event_kinds(kinds)?;
        let out = write(out, "out")?;
        out.set(AnyboxEvents::boxed(device.events_of(kinds)));
        Ok(())
    })
}

/// a medius-capi handle for everything only Medius does, used with medius.h and freed with `medius_device_free`.
/// it shares this device's connection, and keeping it holds the port open after this device is gone. anybox
/// doesn't know about anything you press or lock through it, see `Device::medius` in the Rust docs for the rest
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_medius(
    device: *const AnyboxDevice,
    out: *mut *mut MediusDevice,
) -> AnyboxStatus {
    guard(|| {
        let wrapper = unsafe { read(device, "device") }?;
        let device = &wrapper.0;
        let out = write(out, "out")?;
        let Some(medius) = device.medius() else {
            let family = device.snapshot().info.family;
            return Err(if family == Family::Medius || wrapper.closed() {
                error(anybox::Error::Closed)
            } else {
                fail(
                    ANYBOX_STATUS_UNSUPPORTED,
                    format!("a {family} has no Medius handle, only a Medius does"),
                )
            });
        };
        out.set(MediusDevice::into_raw(medius));
        Ok(())
    })
}

/// lets go of what you held, unlocks, and closes the port, and tells you whether that worked. it waits up to about
/// 2 s when the box doesn't answer. after it every call fails with `ANYBOX_STATUS_CLOSED`, the receivers end once
/// they're read empty, and the snapshot still works. you still free the device after
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_close(device: *const AnyboxDevice) -> AnyboxStatus {
    guard(|| {
        let device = unsafe { read(device, "device") }?;
        device.1.store(true, Ordering::Release);
        device.0.close().map_err(error)
    })
}

/// closes the device like `anybox_device_close` if it isn't yet, and frees it. null does nothing
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_device_free(device: *mut AnyboxDevice) {
    guard_free(|| {
        if !device.is_null() {
            // SAFETY: it came from `Box::into_raw` in this library and the header says it's freed once
            drop(unsafe { Box::from_raw(device) });
        }
    });
}
