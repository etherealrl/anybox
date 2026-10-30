//! the calls only one box has, on the device and named after the box. on another box each is
//! `ANYBOX_STATUS_UNSUPPORTED` naming the box, after close `ANYBOX_STATUS_CLOSED`

use std::ffi::{CString, c_char};
use std::ptr;
use std::time::Duration;

use anybox::{
    Action, Device, Error, Family, FerrumAction, Interpolation, MakcuAction, YscAction,
    YscInterpolation, YscInterpolationSettings, YscProfile, YscSerial,
};

use crate::device::AnyboxDevice;
use crate::failure::Failure;
use crate::status::{AnyboxStatus, error, guard, guard_free, read, set_failure, write};

/// how long a MAKCU spreads a move, `automatic` for AUTO and otherwise `percent` from 0 to 100
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct AnyboxMakcuInterpolation {
    pub automatic: bool,
    pub percent: u8,
}

/// a YSC's interpolation as the box reports it. each `has_` is false when the box didn't send that field, the
/// vendor's own app only reads `enabled`, `profile` and the measured interval
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct AnyboxYscInterpolation {
    pub enabled: bool,
    /// 0 linear, 1 ease in out, 2 min jerk, another number kept as the box sent it
    pub profile: u32,
    pub has_window: bool,
    /// zero is adaptive
    pub window_ms: u32,
    pub has_max_window: bool,
    pub max_window_ms: u32,
    pub has_measured_interval: bool,
    /// in microseconds, the box measures it finer than a millisecond
    pub measured_interval_us: u64,
}

/// what `anybox_ysc_set_interpolation` sends, the windows in whole milliseconds up to 60 s
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct AnyboxYscInterpolationSettings {
    pub enabled: bool,
    pub profile: u32,
    pub window_ms: u32,
    pub max_window_ms: u32,
}

/// the serial a YSC gives the mouse behind it, read with the `anybox_ysc_serial_*` getters. free it with
/// `anybox_ysc_serial_free`
pub struct AnyboxYscSerial {
    custom: Option<CString>,
    physical: Option<CString>,
    active: Option<CString>,
}

/// the device's handle for one box's calls, or the error a call on another box gets
fn handle<'a, T>(
    wrapper: &'a AnyboxDevice,
    family: Family,
    action: Action,
    get: impl FnOnce(&'a Device) -> Option<T>,
) -> Result<T, AnyboxStatus> {
    get(&wrapper.0).ok_or_else(|| {
        let found = wrapper.0.snapshot().info.family;
        // closed comes before the family, like in the core's own order of checks
        if found == family || wrapper.closed() {
            error(Error::Closed)
        } else {
            set_failure(Failure::unsupported(
                action,
                found,
                format!("only a {family} has this"),
            ))
        }
    })
}

unsafe fn device<'a>(device: *const AnyboxDevice) -> Result<&'a AnyboxDevice, AnyboxStatus> {
    unsafe { read(device, "device") }
}

fn makcu_interpolation(value: Interpolation) -> AnyboxMakcuInterpolation {
    match value {
        Interpolation::Auto => AnyboxMakcuInterpolation {
            automatic: true,
            percent: 0,
        },
        Interpolation::Percent(percent) => AnyboxMakcuInterpolation {
            automatic: false,
            percent,
        },
    }
}

fn ysc_interpolation(value: YscInterpolation) -> AnyboxYscInterpolation {
    let ms = |duration: Option<Duration>| {
        duration.map_or((false, 0), |duration| {
            (
                true,
                u32::try_from(duration.as_millis()).unwrap_or(u32::MAX),
            )
        })
    };
    let (has_window, window_ms) = ms(value.window);
    let (has_max_window, max_window_ms) = ms(value.max_window);
    AnyboxYscInterpolation {
        enabled: value.enabled,
        profile: profile_number(value.profile),
        has_window,
        window_ms,
        has_max_window,
        max_window_ms,
        has_measured_interval: value.measured_interval.is_some(),
        measured_interval_us: value.measured_interval.map_or(0, |interval| {
            u64::try_from(interval.as_micros()).unwrap_or(u64::MAX)
        }),
    }
}

fn profile_number(profile: YscProfile) -> u32 {
    match profile {
        YscProfile::Linear => 0,
        YscProfile::EaseInOut => 1,
        YscProfile::MinJerk => 2,
        YscProfile::Other(n) => n,
        _ => u32::MAX,
    }
}

fn profile(number: u32) -> YscProfile {
    match number {
        0 => YscProfile::Linear,
        1 => YscProfile::EaseInOut,
        2 => YscProfile::MinJerk,
        n => YscProfile::Other(n),
    }
}

fn interpolation(raw: &AnyboxMakcuInterpolation) -> Interpolation {
    if raw.automatic {
        Interpolation::Auto
    } else {
        Interpolation::Percent(raw.percent)
    }
}

fn ysc_settings(raw: &AnyboxYscInterpolationSettings) -> YscInterpolationSettings {
    YscInterpolationSettings {
        enabled: raw.enabled,
        profile: profile(raw.profile),
        window: Duration::from_millis(u64::from(raw.window_ms)),
        max_window: Duration::from_millis(u64::from(raw.max_window_ms)),
    }
}

/// 0 is off in C
fn ms_or_off(ms: u32) -> Option<Duration> {
    (ms != 0).then(|| Duration::from_millis(u64::from(ms)))
}

fn ms_of(after: Option<Duration>) -> u32 {
    after.map_or(0, |after| {
        u32::try_from(after.as_millis()).unwrap_or(u32::MAX)
    })
}

fn ysc_serial(serial: YscSerial) -> *mut AnyboxYscSerial {
    let YscSerial {
        custom,
        physical,
        active,
        ..
    } = serial;
    let c = |text: Option<String>| {
        text.map(|text| CString::new(text.replace('\0', "")).unwrap_or_default())
    };
    Box::into_raw(Box::new(AnyboxYscSerial {
        custom: c(custom),
        physical: c(physical),
        active: c(active),
    }))
}

// made by bindings/gen from its call table (makcu_calls), change the table and run cargo run -p anybox-gen
/// how long the box spreads a move, read fresh from the box. it's global to the box and every program on it shares
/// it
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_makcu_interpolation(
    device: *const AnyboxDevice,
    out: *mut AnyboxMakcuInterpolation,
) -> AnyboxStatus {
    guard(|| {
        let device = unsafe { self::device(device) }?;
        let out = write(out, "out")?;
        let got = handle(
            device,
            Family::MakcuV4,
            Action::Makcu(MakcuAction::Interpolation),
            Device::makcu,
        )?
        .interpolation()
        .map_err(error)?;
        out.set(makcu_interpolation(got));
        Ok(())
    })
}

/// sets it and reads it back, a box that didn't take it is `ANYBOX_STATUS_REJECTED`. it changes how moves land for
/// every program on the box, and anybox leaves it set when you close
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_makcu_set_interpolation(
    device: *const AnyboxDevice,
    value: *const AnyboxMakcuInterpolation,
) -> AnyboxStatus {
    guard(|| {
        let device = unsafe { self::device(device) }?;
        let value = unsafe { read(value, "value") }?;
        handle(
            device,
            Family::MakcuV4,
            Action::Makcu(MakcuAction::Interpolation),
            Device::makcu,
        )?
        .set_interpolation(interpolation(value))
        .map_err(error)
    })
}

/// whether injected motion is still queued or being spread
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_makcu_moving(
    device: *const AnyboxDevice,
    out: *mut bool,
) -> AnyboxStatus {
    guard(|| {
        let device = unsafe { self::device(device) }?;
        let out = write(out, "out")?;
        let got = handle(
            device,
            Family::MakcuV4,
            Action::Makcu(MakcuAction::Moving),
            Device::makcu,
        )?
        .moving()
        .map_err(error)?;
        out.set(got);
        Ok(())
    })
}

/// how long the box waits with nothing coming in before it lets go of injected mouse buttons, 0 when it's off
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_makcu_autorelease(
    device: *const AnyboxDevice,
    ms_out: *mut u32,
) -> AnyboxStatus {
    guard(|| {
        let device = unsafe { self::device(device) }?;
        let ms_out = write(ms_out, "ms_out")?;
        let got = handle(
            device,
            Family::MakcuV4,
            Action::Makcu(MakcuAction::Autorelease),
            Device::makcu,
        )?
        .autorelease()
        .map_err(error)?;
        ms_out.set(ms_of(got));
        Ok(())
    })
}

/// turns autorelease on for 1 ms to 60 s (whole milliseconds), or off with 0. anybox asks the box something about
/// every 250 ms, so with more than that it only fires once your app dies or hangs, and a button it held gets let go
/// instead of staying down. the box shares the timer with every program on it and forgets it when it loses power, so
/// anybox sends it again after a reconnect. needs V4.108
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_makcu_set_autorelease(
    device: *const AnyboxDevice,
    after_ms: u32,
) -> AnyboxStatus {
    guard(|| {
        let device = unsafe { self::device(device) }?;
        handle(
            device,
            Family::MakcuV4,
            Action::Makcu(MakcuAction::Autorelease),
            Device::makcu,
        )?
        .set_autorelease(ms_or_off(after_ms))
        .map_err(error)
    })
}

// the end of what bindings/gen makes (makcu_calls)

// made by bindings/gen from its call table (ysc_calls), change the table and run cargo run -p anybox-gen
/// how the box spreads moves. it's on by default and kept on the box, so another app may have changed it
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_ysc_interpolation(
    device: *const AnyboxDevice,
    out: *mut AnyboxYscInterpolation,
) -> AnyboxStatus {
    guard(|| {
        let device = unsafe { self::device(device) }?;
        let out = write(out, "out")?;
        let got = handle(
            device,
            Family::Ysc,
            Action::Ysc(YscAction::Interpolation),
            Device::ysc,
        )?
        .interpolation()
        .map_err(error)?;
        out.set(ysc_interpolation(got));
        Ok(())
    })
}

/// sets it and reads it back, a box that didn't take it is `ANYBOX_STATUS_REJECTED`. the box keeps it, anybox leaves
/// it set when you close
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_ysc_set_interpolation(
    device: *const AnyboxDevice,
    settings: *const AnyboxYscInterpolationSettings,
) -> AnyboxStatus {
    guard(|| {
        let device = unsafe { self::device(device) }?;
        let settings = unsafe { read(settings, "settings") }?;
        handle(
            device,
            Family::Ysc,
            Action::Ysc(YscAction::Interpolation),
            Device::ysc,
        )?
        .set_interpolation(ysc_settings(settings))
        .map_err(error)
    })
}

/// puts it back to the box's default and gives you what that is
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_ysc_reset_interpolation(
    device: *const AnyboxDevice,
    out: *mut AnyboxYscInterpolation,
) -> AnyboxStatus {
    guard(|| {
        let device = unsafe { self::device(device) }?;
        let out = write(out, "out")?;
        let got = handle(
            device,
            Family::Ysc,
            Action::Ysc(YscAction::Interpolation),
            Device::ysc,
        )?
        .reset_interpolation()
        .map_err(error)?;
        out.set(ysc_interpolation(got));
        Ok(())
    })
}

/// the serial the box gives the mouse behind it. free it with `anybox_ysc_serial_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_ysc_serial(
    device: *const AnyboxDevice,
    out: *mut *mut AnyboxYscSerial,
) -> AnyboxStatus {
    guard(|| {
        let device = unsafe { self::device(device) }?;
        let out = write(out, "out")?;
        let got = handle(
            device,
            Family::Ysc,
            Action::Ysc(YscAction::Serial),
            Device::ysc,
        )?
        .serial()
        .map_err(error)?;
        out.set(ysc_serial(got));
        Ok(())
    })
}

// the end of what bindings/gen makes (ysc_calls)

fn text(text: &Option<CString>) -> *const c_char {
    text.as_ref().map_or(ptr::null(), |text| text.as_ptr())
}

/// the serial set in the vendor's app, empty when there's none, null when the box didn't send it. it lives as long
/// as the serial
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_ysc_serial_custom(serial: *const AnyboxYscSerial) -> *const c_char {
    unsafe { read(serial, "serial") }.map_or(ptr::null(), |serial| text(&serial.custom))
}

/// the mouse's own serial, null when the box didn't send it. it lives as long as the serial
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_ysc_serial_physical(
    serial: *const AnyboxYscSerial,
) -> *const c_char {
    unsafe { read(serial, "serial") }.map_or(ptr::null(), |serial| text(&serial.physical))
}

/// the serial the PC sees now, null when the box didn't send it. it lives as long as the serial
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_ysc_serial_active(serial: *const AnyboxYscSerial) -> *const c_char {
    unsafe { read(serial, "serial") }.map_or(ptr::null(), |serial| text(&serial.active))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_ysc_serial_free(serial: *mut AnyboxYscSerial) {
    guard_free(|| {
        if !serial.is_null() {
            drop(unsafe { Box::from_raw(serial) });
        }
    });
}

// made by bindings/gen from its call table (ferrum_calls), change the table and run cargo run -p anybox-gen
/// the mouse's own motion summed over the last `window_ms` (whole milliseconds, up to a second), and what software
/// sent too with `with_software`. y is positive down like `anybox_device_move_by` per the vendor's first example,
/// its second says the opposite and nobody checked it on a box yet. what a 0 window does isn't documented
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_ferrum_catch_xy(
    device: *const AnyboxDevice,
    window_ms: u32,
    with_software: bool,
    x: *mut i32,
    y: *mut i32,
) -> AnyboxStatus {
    guard(|| {
        let device = unsafe { self::device(device) }?;
        let x = write(x, "x")?;
        let y = write(y, "y")?;
        let got = handle(
            device,
            Family::Ferrum,
            Action::Ferrum(FerrumAction::CatchXy),
            Device::ferrum,
        )?
        .catch_xy(Duration::from_millis(u64::from(window_ms)), with_software)
        .map_err(error)?;
        x.set(got.0);
        y.set(got.1);
        Ok(())
    })
}

// the end of what bindings/gen makes (ferrum_calls)
