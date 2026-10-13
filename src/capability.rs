use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::time::Duration;

use crate::click::ClickTiming;

/// the box families anybox knows how to talk to
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Family {
    Medius,
    MakcuV4,
    Ferrum,
    Ysc,
}

impl fmt::Display for Family {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Medius => "Medius",
            Self::MakcuV4 => "MAKCU V4",
            Self::Ferrum => "Ferrum",
            Self::Ysc => "YSC 8K",
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Button {
    Left,
    Right,
    Middle,
    Side1,
    Side2,
    /// a button past side2, numbered from 5. Medius has these when the mouse behind it does, MAKCU only reports
    /// them as input
    Extra(u8),
}

impl fmt::Display for Button {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Left => f.write_str("left"),
            Self::Right => f.write_str("right"),
            Self::Middle => f.write_str("middle"),
            Self::Side1 => f.write_str("side1"),
            Self::Side2 => f.write_str("side2"),
            Self::Extra(n) => write!(f, "extra {n}"),
        }
    }
}

/// a physical axis you can lock or scale
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Axis {
    X,
    Y,
    Wheel,
    /// horizontal scroll
    Pan,
}

impl fmt::Display for Axis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::X => "X",
            Self::Y => "Y",
            Self::Wheel => "wheel",
            Self::Pan => "pan",
        })
    }
}

/// which way along an axis. positive is the way the box moves for a positive value, so right for X, down for
/// Y, up for the wheel and right for pan
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Direction {
    Both,
    Positive,
    Negative,
}

/// every action a box gets asked whether it can do, and the one an error is about
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Action {
    Connect,
    Move,
    MoveNow,
    Press,
    Release,
    Click,
    Wheel,
    Pan,
    DiscardPendingMoves,
    ButtonEvents,
    MotionEvents,
    KeyEvents,
    MediaEvents,
    KeyPress,
    KeyRelease,
    KeyTap,
    MediaPress,
    MediaRelease,
    MediaTap,
    LockButton,
    LockAxis,
    LockKey,
    ScaleAxis,
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Connect => "connect",
            Self::Move => "move",
            Self::MoveNow => "move now",
            Self::Press => "press",
            Self::Release => "release",
            Self::Click => "click",
            Self::Wheel => "scroll",
            Self::Pan => "scroll sideways",
            Self::DiscardPendingMoves => "discard pending moves",
            Self::ButtonEvents => "report button events",
            Self::MotionEvents => "report motion events",
            Self::KeyEvents => "report key events",
            Self::MediaEvents => "report media key events",
            Self::LockButton => "lock or unlock a button",
            Self::LockAxis => "lock or unlock an axis",
            Self::LockKey => "lock or unlock a key",
            Self::ScaleAxis => "scale an axis",
            Self::KeyPress => "press a key",
            Self::KeyRelease => "release a key",
            Self::KeyTap => "tap a key",
            Self::MediaPress => "press a media key",
            Self::MediaRelease => "release a media key",
            Self::MediaTap => "tap a media key",
        })
    }
}

/// how a box does an action
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Execution {
    /// the box does it, with its own timing
    Native,
    /// anybox builds it on the PC out of things the box can do, so it runs on the PC's timing
    Host,
    Unsupported {
        reason: Cow<'static, str>,
    },
}

/// a way this box behaves differently that anybox can't hide, so it tells you instead of smoothing it over
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Quirk {
    /// after a release the box keeps the button up for a random while, and only then does the physical button
    /// count again. that's the box's default, a setting on the box that anybox can't read may turn it off
    ForcedUpOnRelease { min: Duration, max: Duration },
    /// pressing and letting go of the physical button or key ends a press the box holds for you, the physical one
    /// wins
    PhysicalOverride,
    /// the box only locks a whole axis, so locking or unlocking one way of it alone gets refused
    WholeAxisOnly,
    /// the most wheel notches the box takes in one command
    NotchesPerCommand(u16),
    /// the box's click has its own timing and doesn't take a hold
    FixedClickTiming,
    /// the longest hold the box's click takes
    MaxHold(Duration),
    /// how long the box holds a click you don't give a hold to
    BoxClickTiming { min: Duration, max: Duration },
    /// the box can't time a click or tap itself, so this is what anybox does on the PC instead
    HostClickTiming(ClickTiming),
    /// the box presses held buttons and keys again by itself after a reconnect or a session reset, so they
    /// stay held through it instead of being let go
    KeepsHeld,
    /// the most keys the box holds down at once, not counting modifiers. it can't report any past that
    MaxKeysHeld(u8),
    /// the box puts locks back by itself after a reconnect or a session reset
    KeepsLocked,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Capability {
    pub execution: Execution,
    pub quirks: Vec<Quirk>,
}

impl Capability {
    pub(crate) fn native() -> Self {
        Self {
            execution: Execution::Native,
            quirks: Vec::new(),
        }
    }

    pub(crate) fn host() -> Self {
        Self {
            execution: Execution::Host,
            quirks: Vec::new(),
        }
    }

    pub(crate) fn unsupported(reason: impl Into<Cow<'static, str>>) -> Self {
        Self {
            execution: Execution::Unsupported {
                reason: reason.into(),
            },
            quirks: Vec::new(),
        }
    }

    pub(crate) fn with(mut self, quirk: Quirk) -> Self {
        self.quirks.push(quirk);
        self
    }

    pub fn is_supported(&self) -> bool {
        !matches!(self.execution, Execution::Unsupported { .. })
    }

    /// why it's unsupported, for an error that comes up after the capability check already passed
    pub(crate) fn reason(&self) -> Cow<'static, str> {
        match &self.execution {
            Execution::Unsupported { reason } => reason.clone(),
            _ => "the box can't do it".into(),
        }
    }
}

/// what this box, on this firmware, with this mouse behind it, can do
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capabilities {
    actions: HashMap<Action, Capability>,
    buttons: Vec<Button>,
    axes: Vec<Axis>,
    missing: Capability,
}

impl Capabilities {
    pub(crate) fn new(buttons: Vec<Button>) -> Self {
        Self {
            actions: HashMap::new(),
            buttons,
            axes: Vec::new(),
            missing: Capability::unsupported("this box doesn't have it"),
        }
    }

    pub(crate) fn set(mut self, action: Action, capability: Capability) -> Self {
        self.actions.insert(action, capability);
        self
    }

    /// anything the box didn't list counts as unsupported
    pub fn get(&self, action: Action) -> &Capability {
        self.actions.get(&action).unwrap_or(&self.missing)
    }

    pub fn buttons(&self) -> &[Button] {
        &self.buttons
    }

    /// the axes you can lock or scale on this box
    pub fn axes(&self) -> &[Axis] {
        &self.axes
    }

    pub(crate) fn with_axes(mut self, axes: Vec<Axis>) -> Self {
        self.axes = axes;
        self
    }

    pub fn iter(&self) -> impl Iterator<Item = (Action, &Capability)> {
        self.actions
            .iter()
            .map(|(action, capability)| (*action, capability))
    }

    /// a box that can press and release but has no click or tap of its own gets one built on the PC
    pub(crate) fn with_host_taps(mut self, timing: ClickTiming) -> Self {
        for (tap, press, release) in [
            (Action::Click, Action::Press, Action::Release),
            (Action::KeyTap, Action::KeyPress, Action::KeyRelease),
            (Action::MediaTap, Action::MediaPress, Action::MediaRelease),
        ] {
            let native = self.get(tap).is_supported();
            let parts = self.get(press).is_supported() && self.get(release).is_supported();
            if !native && parts {
                self.actions
                    .insert(tap, Capability::host().with(Quirk::HostClickTiming(timing)));
            }
        }
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Firmware {
    Version {
        major: u16,
        minor: u16,
        patch: u16,
    },
    Build(u32),
    /// for boxes that report a protocol number instead, like Medius
    Protocol(u8),
    Text(String),
}

impl fmt::Display for Firmware {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Version {
                major,
                minor,
                patch,
            } => write!(f, "{major}.{minor}.{patch}"),
            Self::Build(build) => write!(f, "build {build}"),
            Self::Protocol(protocol) => write!(f, "protocol {protocol}"),
            Self::Text(text) => f.write_str(text),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum StableId {
    Mac([u8; 6]),
    UsbSerial(String),
}

impl fmt::Display for StableId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mac(mac) => {
                let hex: Vec<String> = mac.iter().map(|byte| format!("{byte:02x}")).collect();
                write!(f, "MAC {}", hex.join(":"))
            }
            Self::UsbSerial(serial) => write!(f, "USB serial {serial}"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Info {
    /// the port the device was opened on, so you can show it or open the same box again with `Connect::port`
    pub port: String,
    pub family: Family,
    pub model: Option<String>,
    pub firmware: Option<Firmware>,
    pub id: Option<StableId>,
    /// which of Ferrum's two APIs this is, none on other boxes
    pub ferrum_api: Option<FerrumApi>,
    /// the USB ids of the device plugged into the box, which is the identity the box shows the PC, so you can
    /// tell which device on the PC goes through the box. a Ferrum on its Software API and a Medius report it, a
    /// MAKCU can't and a YSC isn't read yet. on a Ferrum it's the first device plugged into the box, not always the
    /// mouse. it's read when you connect and again when what's plugged in changes, so swapping one mouse for
    /// another within a second can leave the old ids here until the next reconnect
    pub peripheral: Option<UsbIds>,
}

impl Info {
    pub(crate) fn new(family: Family) -> Self {
        Self {
            port: String::new(),
            family,
            model: None,
            firmware: None,
            id: None,
            ferrum_api: None,
            peripheral: None,
        }
    }
}

/// the two ways into a Ferrum
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FerrumApi {
    /// the virtual port the Ferrum app makes, Windows only. everything anybox does on a Ferrum works here
    Software,
    /// the box's own port, the only way in from Linux. it only moves, clicks, scrolls, locks buttons and whole
    /// axes, and reports buttons, and a KMBox B+ answers on it exactly like a Ferrum does
    Legacy,
}

impl fmt::Display for FerrumApi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Software => "the Ferrum app's port",
            Self::Legacy => "the box's own legacy port",
        })
    }
}

/// a USB vendor and product id
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct UsbIds {
    pub vid: u16,
    pub pid: u16,
}

impl UsbIds {
    pub fn new(vid: u16, pid: u16) -> Self {
        Self { vid, pid }
    }
}

/// who the box is and what it can do, both always read from the same connection
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Snapshot {
    pub info: Info,
    pub capabilities: Capabilities,
}

impl Snapshot {
    pub(crate) fn new(info: Info, capabilities: Capabilities) -> Self {
        Self { info, capabilities }
    }
}
