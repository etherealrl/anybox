use std::time::Duration;

use super::{Extension, Interpolation, YscProfile};
use crate::capability::{Action, MakcuAction};
use crate::error::Error;

#[test]
fn arguments_are_checked_in_whole_milliseconds_and_within_their_range() {
    for (request, ok) in [
        (
            Extension::MakcuSetInterpolation(Interpolation::Percent(100)),
            true,
        ),
        (
            Extension::MakcuSetInterpolation(Interpolation::Percent(101)),
            false,
        ),
        (Extension::MakcuSetAutorelease(None), true),
        (
            Extension::MakcuSetAutorelease(Some(Duration::from_secs(60))),
            true,
        ),
        (Extension::MakcuSetAutorelease(Some(Duration::ZERO)), false),
        (
            Extension::FerrumCatchXy {
                window: Duration::ZERO,
                with_software: false,
            },
            true,
        ),
        (
            Extension::FerrumCatchXy {
                window: Duration::from_micros(999_500),
                with_software: false,
            },
            false,
        ),
    ] {
        assert_eq!(request.check().is_ok(), ok, "{request:?}");
    }
    assert!(matches!(
        Extension::MakcuSetInterpolation(Interpolation::Percent(101)).check(),
        Err(Error::InvalidArgument {
            action: Action::Makcu(MakcuAction::Interpolation),
            ..
        })
    ));
}

#[test]
fn a_profile_number_the_vendor_has_no_name_for_is_kept() {
    for n in 0..5 {
        assert_eq!(YscProfile::from_number(n).number(), n);
    }
    assert_eq!(YscProfile::from_number(3), YscProfile::Other(3));
}

#[cfg(feature = "mock")]
mod mock {
    use std::time::Duration;

    use crate::click::ClickTiming;
    use crate::device::Device;
    use crate::extension::Interpolation;

    #[test]
    fn each_mock_box_answers_its_own_calls() {
        let device = Device::mock_makcu(ClickTiming::Makcu).unwrap();
        let makcu = device.makcu().unwrap();
        makcu.set_interpolation(Interpolation::Percent(10)).unwrap();
        assert_eq!(makcu.interpolation().unwrap(), Interpolation::Percent(10));
        makcu
            .set_autorelease(Some(Duration::from_millis(800)))
            .unwrap();
        assert!(!makcu.moving().unwrap());
        let device = Device::mock_ysc(ClickTiming::Makcu).unwrap();
        assert!(device.ysc().unwrap().interpolation().unwrap().enabled);
        assert!(device.ysc().unwrap().serial().is_ok());
        let device = Device::mock_ferrum(ClickTiming::Makcu).unwrap();
        assert_eq!(
            device
                .ferrum()
                .unwrap()
                .catch_xy(Duration::from_millis(10), true)
                .unwrap(),
            (-3, 10)
        );
    }
}
