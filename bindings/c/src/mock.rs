use anybox::Device;
use anybox::medius::{Class, Direction, FrameType, MockBox, Usage};

use crate::device::AnyboxDevice;
use crate::status::{AnyboxStatus, error, guard, guard_free, invalid, read, write};
use crate::values::AnyboxClickTiming;

/// the medius crate's model of a Medius box, for testing code that uses anybox with no box plugged in. it proves
/// nothing about hardware and it can only be shaped like a Medius
pub struct AnyboxMock(pub(crate) MockBox);

/// a device over a new mock box, a five button mouse with a wheel. it runs the exact code a real Medius gets. null
/// `click_timing` is the default. free the mock with `anybox_mock_free` and the device with `anybox_device_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_mock_medius_open(
    click_timing: *const AnyboxClickTiming,
    mock: *mut *mut AnyboxMock,
    out: *mut *mut AnyboxDevice,
) -> AnyboxStatus {
    guard(|| {
        // SAFETY: the header says a non-null click timing points at a whole `AnyboxClickTiming`
        let timing = match unsafe { click_timing.as_ref() } {
            Some(timing) => crate::values::click_timing(timing)?,
            None => anybox::ClickTiming::default(),
        };
        let mock = write(mock, "mock")?;
        let out = write(out, "out")?;
        let mock_box = MockBox::new();
        let device = Device::mock_medius(mock_box.clone(), timing).map_err(error)?;
        mock.set(Box::into_raw(Box::new(AnyboxMock(mock_box))));
        out.set(AnyboxDevice::boxed(device));
        Ok(())
    })
}

/// whether anything subscribed to the mock's input yet. a receiver subscribes on its own thread, so wait for this
/// before you push, otherwise the box drops what you pushed like a real one would
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_mock_subscribed(mock: *const AnyboxMock) -> bool {
    // SAFETY: the header says a non-null mock came from `anybox_mock_medius_open` and wasn't freed
    unsafe { mock.as_ref() }.is_some_and(|mock| mock.0.saw(FrameType::Catch))
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
        let direction = if pressed {
            Direction::PRESS
        } else {
            Direction::RELEASE
        };
        mock.0
            .push_usages(seq, ts_us, Class::Button, direction, &usages);
        Ok(())
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
