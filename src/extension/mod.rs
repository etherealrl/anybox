//! what only one box has, reached through `device.makcu()`, `device.ysc()` and `device.ferrum()`. each call goes
//! through the same checks as a core call (closed, link down, capability, arguments) and fails as unsupported when
//! the box lost the feature since the handle was taken

use std::time::Duration;

use crate::capability::{Action, Family, FerrumAction, MakcuAction, YscAction};
use crate::device::Device;
use crate::error::{Error, Result};

#[cfg(test)]
mod tests;

/// how long the MAKCU spreads an injected move over its reports. AUTO picks 1 to 64 ms from how often moves
/// come
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Interpolation {
    Auto,
    /// 0 to 100. 0 sends a move in the next report on V4.097 and later
    Percent(u8),
}

/// the shape the YSC gives a spread move
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum YscProfile {
    Linear,
    EaseInOut,
    MinJerk,
    /// a number the vendor doesn't document yet, kept as the box sent it
    Other(u32),
}

impl YscProfile {
    pub(crate) fn number(self) -> u32 {
        match self {
            Self::Linear => 0,
            Self::EaseInOut => 1,
            Self::MinJerk => 2,
            Self::Other(n) => n,
        }
    }

    pub(crate) fn from_number(n: u32) -> Self {
        match n {
            0 => Self::Linear,
            1 => Self::EaseInOut,
            2 => Self::MinJerk,
            n => Self::Other(n),
        }
    }
}

/// the YSC's interpolation as the box reports it. a field the box didn't send is none, the vendor's own app only
/// reads `enabled`, `profile` and the measured interval, so a firmware may leave the windows out
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct YscInterpolation {
    pub enabled: bool,
    pub profile: YscProfile,
    /// a fixed window, zero means the box adapts it to how often moves come
    pub window: Option<Duration>,
    /// the longest window the adaptive mode goes to
    pub max_window: Option<Duration>,
    /// how often moves have been coming, the box's own average
    pub measured_interval: Option<Duration>,
}

/// what `set_interpolation` sends, every field required since the box takes them together
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct YscInterpolationSettings {
    pub enabled: bool,
    pub profile: YscProfile,
    /// whole milliseconds up to 60 s, zero for adaptive
    pub window: Duration,
    /// whole milliseconds up to 60 s, the vendor's default is 50 ms
    pub max_window: Duration,
}

/// the serial the YSC gives the mouse behind it, each none when the box didn't send it
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct YscSerial {
    /// the one set in the vendor's app, empty when there's none
    pub custom: Option<String>,
    /// the mouse's own
    pub physical: Option<String>,
    /// the one the PC sees now
    pub active: Option<String>,
}

/// one extension call, what the core hands the backend
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Extension {
    MakcuInterpolation,
    MakcuSetInterpolation(Interpolation),
    MakcuMoving,
    MakcuAutorelease,
    MakcuSetAutorelease(Option<Duration>),
    YscInterpolation,
    YscSetInterpolation(YscInterpolationSettings),
    YscResetInterpolation,
    YscSerial,
    FerrumCatchXy {
        window: Duration,
        with_software: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ExtensionReply {
    Done,
    Interpolation(Interpolation),
    Moving(bool),
    Autorelease(Option<Duration>),
    YscInterpolation(YscInterpolation),
    YscSerial(YscSerial),
    Xy(i32, i32),
}

/// the MAKCU's autorelease takes up to a minute
pub(crate) const MAX_AUTORELEASE: Duration = Duration::from_secs(60);
/// the vendor gives the YSC windows no limit, a minute is anybox's
pub(crate) const MAX_YSC_WINDOW: Duration = Duration::from_secs(60);
/// the longest window `km.catch_xy` sums over
pub(crate) const MAX_CATCH: Duration = Duration::from_secs(1);

impl Extension {
    pub(crate) fn action(self) -> Action {
        match self {
            Self::MakcuInterpolation | Self::MakcuSetInterpolation(_) => {
                Action::Makcu(MakcuAction::Interpolation)
            }
            Self::MakcuMoving => Action::Makcu(MakcuAction::Moving),
            Self::MakcuAutorelease | Self::MakcuSetAutorelease(_) => {
                Action::Makcu(MakcuAction::Autorelease)
            }
            Self::YscInterpolation | Self::YscSetInterpolation(_) | Self::YscResetInterpolation => {
                Action::Ysc(YscAction::Interpolation)
            }
            Self::YscSerial => Action::Ysc(YscAction::Serial),
            Self::FerrumCatchXy { .. } => Action::Ferrum(FerrumAction::CatchXy),
        }
    }

    /// the arguments, checked after the capability like a core call's
    pub(crate) fn check(self) -> Result<()> {
        let action = self.action();
        let bad = |detail: String| {
            Err(Error::InvalidArgument {
                action,
                detail: detail.into(),
            })
        };
        match self {
            Self::MakcuSetInterpolation(Interpolation::Percent(n)) if n > 100 => bad(format!(
                "interpolation goes up to 100 percent, {n} is too much"
            )),
            Self::MakcuSetAutorelease(Some(after)) => {
                whole_ms(action, after, "autorelease")?;
                if after.is_zero() || after > MAX_AUTORELEASE {
                    return bad(format!(
                        "autorelease takes 1 ms to 60 s, {after:?} isn't in it, use none to turn it off"
                    ));
                }
                Ok(())
            }
            Self::YscSetInterpolation(settings) => {
                for (name, window) in [
                    ("the window", settings.window),
                    ("the max window", settings.max_window),
                ] {
                    whole_ms(action, window, name)?;
                    if window > MAX_YSC_WINDOW {
                        return bad(format!("{name} goes up to 60 s, {window:?} is too long"));
                    }
                }
                Ok(())
            }
            Self::FerrumCatchXy { window, .. } => {
                whole_ms(action, window, "the window")?;
                if window > MAX_CATCH {
                    return bad(format!("the window goes up to 1 s, {window:?} is too long"));
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

/// the boxes take milliseconds, so anything finer would get cut and could turn into a different setting
fn whole_ms(action: Action, duration: Duration, name: &str) -> Result<()> {
    if !duration.subsec_nanos().is_multiple_of(1_000_000) {
        return Err(Error::InvalidArgument {
            action,
            detail: format!("{name} has to be whole milliseconds, {duration:?} isn't").into(),
        });
    }
    Ok(())
}

/// what only a MAKCU has. it borrows the device, and each call checks what the box can do right now
pub struct Makcu<'a> {
    device: &'a Device,
}

impl<'a> Makcu<'a> {
    pub(crate) fn new(device: &'a Device) -> Self {
        Self { device }
    }

    /// how long the box spreads a move, read fresh from the box. it's global to the box and every program on it
    /// shares it
    pub fn interpolation(&self) -> Result<Interpolation> {
        match self.device.extension(Extension::MakcuInterpolation)? {
            ExtensionReply::Interpolation(value) => Ok(value),
            other => Err(unexpected(Extension::MakcuInterpolation, other)),
        }
    }

    /// sets it and reads it back, a box that didn't take it is `Rejected`. it changes how `move_by` lands for every
    /// program on the box, and anybox leaves it set when you close
    pub fn set_interpolation(&self, value: Interpolation) -> Result<()> {
        self.device
            .extension(Extension::MakcuSetInterpolation(value))
            .map(|_| ())
    }

    /// whether injected motion is still queued or being spread
    pub fn moving(&self) -> Result<bool> {
        match self.device.extension(Extension::MakcuMoving)? {
            ExtensionReply::Moving(moving) => Ok(moving),
            other => Err(unexpected(Extension::MakcuMoving, other)),
        }
    }

    /// how long the box waits with nothing coming in before it lets go of injected mouse buttons, none when it's off
    pub fn autorelease(&self) -> Result<Option<Duration>> {
        match self.device.extension(Extension::MakcuAutorelease)? {
            ExtensionReply::Autorelease(after) => Ok(after),
            other => Err(unexpected(Extension::MakcuAutorelease, other)),
        }
    }

    /// turns autorelease on (1 ms to 60 s) or off. anybox asks the box something about every 250 ms, so with more
    /// than that it only fires once your app dies or hangs, and a button it held gets let go instead of staying
    /// down. the box shares the timer with every program on it and forgets it when it loses power, so anybox sends
    /// it again after a reconnect. needs V4.108
    pub fn set_autorelease(&self, after: Option<Duration>) -> Result<()> {
        self.device
            .extension(Extension::MakcuSetAutorelease(after))
            .map(|_| ())
    }
}

/// what only a YSC8K has. it borrows the device, and each call checks what the box can do right now
pub struct Ysc<'a> {
    device: &'a Device,
}

impl<'a> Ysc<'a> {
    pub(crate) fn new(device: &'a Device) -> Self {
        Self { device }
    }

    /// how the box spreads moves. it's on by default and kept on the box, so another app may have changed it
    pub fn interpolation(&self) -> Result<YscInterpolation> {
        match self.device.extension(Extension::YscInterpolation)? {
            ExtensionReply::YscInterpolation(value) => Ok(value),
            other => Err(unexpected(Extension::YscInterpolation, other)),
        }
    }

    /// sets it and reads it back, a box that didn't take it is `Rejected`. the box keeps it, anybox leaves it set
    /// when you close
    pub fn set_interpolation(&self, settings: YscInterpolationSettings) -> Result<()> {
        self.device
            .extension(Extension::YscSetInterpolation(settings))
            .map(|_| ())
    }

    /// puts it back to the box's default and returns what that is
    pub fn reset_interpolation(&self) -> Result<YscInterpolation> {
        match self.device.extension(Extension::YscResetInterpolation)? {
            ExtensionReply::YscInterpolation(value) => Ok(value),
            other => Err(unexpected(Extension::YscResetInterpolation, other)),
        }
    }

    /// the serial the box gives the mouse behind it
    pub fn serial(&self) -> Result<YscSerial> {
        match self.device.extension(Extension::YscSerial)? {
            ExtensionReply::YscSerial(serial) => Ok(serial),
            other => Err(unexpected(Extension::YscSerial, other)),
        }
    }
}

/// what only a Ferrum has. it borrows the device, and each call checks what the box can do right now
pub struct Ferrum<'a> {
    device: &'a Device,
}

impl<'a> Ferrum<'a> {
    pub(crate) fn new(device: &'a Device) -> Self {
        Self { device }
    }

    /// the mouse's own motion summed over the last `window` (whole ms, up to 1 s), and what software sent too with
    /// `with_software`. y is positive down like `move_by` per the vendor's first example, its second one says the
    /// opposite and nobody checked it on a box yet. what a 0 window does isn't documented
    pub fn catch_xy(&self, window: Duration, with_software: bool) -> Result<(i32, i32)> {
        let request = Extension::FerrumCatchXy {
            window,
            with_software,
        };
        match self.device.extension(request)? {
            ExtensionReply::Xy(x, y) => Ok((x, y)),
            other => Err(unexpected(request, other)),
        }
    }
}

/// a backend that answered a call with the wrong kind of reply, which is a bug in anybox
fn unexpected(request: Extension, reply: ExtensionReply) -> Error {
    Error::Rejected {
        action: request.action(),
        detail: format!("anybox got {reply:?} back for {request:?}, which is a bug"),
    }
}

/// the family an extension belongs to, for the accessor and for the error a backend without it gives
pub(crate) fn family(request: Extension) -> Family {
    match request.action() {
        Action::Ysc(_) => Family::Ysc,
        Action::Ferrum(_) => Family::Ferrum,
        _ => Family::MakcuV4,
    }
}
