use std::time::Duration;

use anybox::{
    Action, Axis, Button, Click, ClickTiming, Direction, EventKinds, Execution, Family, FerrumApi,
};

use crate::status::{AnyboxStatus, invalid};

// every value is fixed here by hand, never taken from a Rust discriminant, so reordering an enum in anybox can't
// change what C sees

/// no filter in the connect options
pub const ANYBOX_FAMILY_ANY: u32 = 0;
pub const ANYBOX_FAMILY_MEDIUS: u32 = 1;
pub const ANYBOX_FAMILY_MAKCU_V4: u32 = 2;
pub const ANYBOX_FAMILY_FERRUM: u32 = 3;
pub const ANYBOX_FAMILY_YSC: u32 = 4;

/// no filter in the connect options, and in `AnyboxInfo` a box that isn't a Ferrum
pub const ANYBOX_FERRUM_API_NONE: u32 = 0;
pub const ANYBOX_FERRUM_API_SOFTWARE: u32 = 1;
pub const ANYBOX_FERRUM_API_LEGACY: u32 = 2;

pub const ANYBOX_BUTTON_LEFT: u32 = 0;
pub const ANYBOX_BUTTON_RIGHT: u32 = 1;
pub const ANYBOX_BUTTON_MIDDLE: u32 = 2;
pub const ANYBOX_BUTTON_SIDE1: u32 = 3;
pub const ANYBOX_BUTTON_SIDE2: u32 = 4;
/// a button past side2 is this plus its number, from 0 to 255
pub const ANYBOX_BUTTON_EXTRA_BASE: u32 = 256;

pub const ANYBOX_AXIS_X: u32 = 0;
pub const ANYBOX_AXIS_Y: u32 = 1;
pub const ANYBOX_AXIS_WHEEL: u32 = 2;
/// horizontal scroll
pub const ANYBOX_AXIS_PAN: u32 = 3;

pub const ANYBOX_DIRECTION_BOTH: u32 = 0;
pub const ANYBOX_DIRECTION_POSITIVE: u32 = 1;
pub const ANYBOX_DIRECTION_NEGATIVE: u32 = 2;

pub const ANYBOX_ACTION_CONNECT: u32 = 0;
pub const ANYBOX_ACTION_MOVE: u32 = 1;
pub const ANYBOX_ACTION_MOVE_NOW: u32 = 2;
pub const ANYBOX_ACTION_PRESS: u32 = 3;
pub const ANYBOX_ACTION_RELEASE: u32 = 4;
pub const ANYBOX_ACTION_CLICK: u32 = 5;
pub const ANYBOX_ACTION_WHEEL: u32 = 6;
pub const ANYBOX_ACTION_PAN: u32 = 7;
pub const ANYBOX_ACTION_DISCARD_PENDING_MOVES: u32 = 8;
pub const ANYBOX_ACTION_BUTTON_EVENTS: u32 = 9;
pub const ANYBOX_ACTION_MOTION_EVENTS: u32 = 10;
pub const ANYBOX_ACTION_KEY_EVENTS: u32 = 11;
pub const ANYBOX_ACTION_MEDIA_EVENTS: u32 = 12;
pub const ANYBOX_ACTION_KEY_PRESS: u32 = 13;
pub const ANYBOX_ACTION_KEY_RELEASE: u32 = 14;
pub const ANYBOX_ACTION_KEY_TAP: u32 = 15;
pub const ANYBOX_ACTION_MEDIA_PRESS: u32 = 16;
pub const ANYBOX_ACTION_MEDIA_RELEASE: u32 = 17;
pub const ANYBOX_ACTION_MEDIA_TAP: u32 = 18;
pub const ANYBOX_ACTION_LOCK_BUTTON: u32 = 19;
pub const ANYBOX_ACTION_LOCK_AXIS: u32 = 20;
pub const ANYBOX_ACTION_LOCK_KEY: u32 = 21;
pub const ANYBOX_ACTION_SCALE_AXIS: u32 = 22;

pub const ANYBOX_EXECUTION_NATIVE: u32 = 0;
pub const ANYBOX_EXECUTION_HOST: u32 = 1;
pub const ANYBOX_EXECUTION_UNSUPPORTED: u32 = 2;

pub const ANYBOX_CLICK_TIMING_MAKCU: u32 = 0;
pub const ANYBOX_CLICK_TIMING_FERRUM: u32 = 1;
pub const ANYBOX_CLICK_TIMING_FIXED: u32 = 2;
pub const ANYBOX_CLICK_TIMING_OFF: u32 = 3;

/// combined with `|` for `anybox_device_events`
pub const ANYBOX_EVENTS_BUTTONS: u32 = 1;
pub const ANYBOX_EVENTS_MOTION: u32 = 2;
pub const ANYBOX_EVENTS_KEYS: u32 = 4;
pub const ANYBOX_EVENTS_MEDIA: u32 = 8;
/// everything the box can stream, a kind it can't is left out without a `Rejected`
pub const ANYBOX_EVENTS_ALL: u32 = 15;

const FAMILIES: [(u32, Family); 4] = [
    (ANYBOX_FAMILY_MEDIUS, Family::Medius),
    (ANYBOX_FAMILY_MAKCU_V4, Family::MakcuV4),
    (ANYBOX_FAMILY_FERRUM, Family::Ferrum),
    (ANYBOX_FAMILY_YSC, Family::Ysc),
];

const FERRUM_APIS: [(u32, FerrumApi); 2] = [
    (ANYBOX_FERRUM_API_SOFTWARE, FerrumApi::Software),
    (ANYBOX_FERRUM_API_LEGACY, FerrumApi::Legacy),
];

const BUTTONS: [(u32, Button); 5] = [
    (ANYBOX_BUTTON_LEFT, Button::Left),
    (ANYBOX_BUTTON_RIGHT, Button::Right),
    (ANYBOX_BUTTON_MIDDLE, Button::Middle),
    (ANYBOX_BUTTON_SIDE1, Button::Side1),
    (ANYBOX_BUTTON_SIDE2, Button::Side2),
];

const AXES: [(u32, Axis); 4] = [
    (ANYBOX_AXIS_X, Axis::X),
    (ANYBOX_AXIS_Y, Axis::Y),
    (ANYBOX_AXIS_WHEEL, Axis::Wheel),
    (ANYBOX_AXIS_PAN, Axis::Pan),
];

const DIRECTIONS: [(u32, Direction); 3] = [
    (ANYBOX_DIRECTION_BOTH, Direction::Both),
    (ANYBOX_DIRECTION_POSITIVE, Direction::Positive),
    (ANYBOX_DIRECTION_NEGATIVE, Direction::Negative),
];

pub(crate) const ACTIONS: [(u32, Action); 23] = [
    (ANYBOX_ACTION_CONNECT, Action::Connect),
    (ANYBOX_ACTION_MOVE, Action::Move),
    (ANYBOX_ACTION_MOVE_NOW, Action::MoveNow),
    (ANYBOX_ACTION_PRESS, Action::Press),
    (ANYBOX_ACTION_RELEASE, Action::Release),
    (ANYBOX_ACTION_CLICK, Action::Click),
    (ANYBOX_ACTION_WHEEL, Action::Wheel),
    (ANYBOX_ACTION_PAN, Action::Pan),
    (
        ANYBOX_ACTION_DISCARD_PENDING_MOVES,
        Action::DiscardPendingMoves,
    ),
    (ANYBOX_ACTION_BUTTON_EVENTS, Action::ButtonEvents),
    (ANYBOX_ACTION_MOTION_EVENTS, Action::MotionEvents),
    (ANYBOX_ACTION_KEY_EVENTS, Action::KeyEvents),
    (ANYBOX_ACTION_MEDIA_EVENTS, Action::MediaEvents),
    (ANYBOX_ACTION_KEY_PRESS, Action::KeyPress),
    (ANYBOX_ACTION_KEY_RELEASE, Action::KeyRelease),
    (ANYBOX_ACTION_KEY_TAP, Action::KeyTap),
    (ANYBOX_ACTION_MEDIA_PRESS, Action::MediaPress),
    (ANYBOX_ACTION_MEDIA_RELEASE, Action::MediaRelease),
    (ANYBOX_ACTION_MEDIA_TAP, Action::MediaTap),
    (ANYBOX_ACTION_LOCK_BUTTON, Action::LockButton),
    (ANYBOX_ACTION_LOCK_AXIS, Action::LockAxis),
    (ANYBOX_ACTION_LOCK_KEY, Action::LockKey),
    (ANYBOX_ACTION_SCALE_AXIS, Action::ScaleAxis),
];

const EVENT_KINDS: [(u32, EventKinds); 4] = [
    (ANYBOX_EVENTS_BUTTONS, EventKinds::BUTTONS),
    (ANYBOX_EVENTS_MOTION, EventKinds::MOTION),
    (ANYBOX_EVENTS_KEYS, EventKinds::KEYS),
    (ANYBOX_EVENTS_MEDIA, EventKinds::MEDIA),
];

/// how `anybox_device_click` and the taps are timed on a box that can't time them itself. `fixed_us` is only read for
/// `ANYBOX_CLICK_TIMING_FIXED`
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnyboxClickTiming {
    pub kind: u32,
    pub fixed_us: u64,
}

/// how one click or tap goes. with no hold, the box uses its own timing, or the device's click timing if it has
/// none
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnyboxClick {
    /// when anybox times the click on the PC, the call waits this long, so a huge one blocks the thread that long
    pub hold_us: u64,
    pub has_hold: bool,
    /// fail instead of letting anybox time it from the PC
    pub native_only: bool,
}

fn from_table<T: Copy>(table: &[(u32, T)], value: u32, what: &str) -> Result<T, AnyboxStatus> {
    table
        .iter()
        .find(|(number, _)| *number == value)
        .map(|(_, item)| *item)
        .ok_or_else(|| invalid(format!("{value} isn't a {what} anybox knows")))
}

/// a value anybox added after this table was written. the tests walk every value there is, so this only fires
/// when the two drift apart, and it fires loudly
fn to_table<T: Copy + PartialEq + std::fmt::Debug>(table: &[(u32, T)], item: T) -> u32 {
    table
        .iter()
        .find(|(_, known)| *known == item)
        .map(|(number, _)| *number)
        .unwrap_or_else(|| panic!("the C ABI has no number for {item:?} yet"))
}

pub(crate) fn family_filter(value: u32) -> Result<Option<Family>, AnyboxStatus> {
    if value == ANYBOX_FAMILY_ANY {
        return Ok(None);
    }
    from_table(&FAMILIES, value, "family").map(Some)
}

pub(crate) fn family_number(family: Family) -> u32 {
    to_table(&FAMILIES, family)
}

pub(crate) fn ferrum_api_filter(value: u32) -> Result<Option<FerrumApi>, AnyboxStatus> {
    if value == ANYBOX_FERRUM_API_NONE {
        return Ok(None);
    }
    from_table(&FERRUM_APIS, value, "Ferrum API").map(Some)
}

pub(crate) fn ferrum_api_number(api: Option<FerrumApi>) -> u32 {
    api.map_or(ANYBOX_FERRUM_API_NONE, |api| to_table(&FERRUM_APIS, api))
}

pub(crate) fn button(value: u32) -> Result<Button, AnyboxStatus> {
    match value.checked_sub(ANYBOX_BUTTON_EXTRA_BASE) {
        Some(extra) => u8::try_from(extra)
            .map(Button::Extra)
            .map_err(|_| invalid(format!("{value} isn't a button anybox knows"))),
        None => from_table(&BUTTONS, value, "button"),
    }
}

pub(crate) fn button_number(button: Button) -> u32 {
    match button {
        Button::Extra(extra) => ANYBOX_BUTTON_EXTRA_BASE + u32::from(extra),
        named => to_table(&BUTTONS, named),
    }
}

pub(crate) fn axis(value: u32) -> Result<Axis, AnyboxStatus> {
    from_table(&AXES, value, "axis")
}

pub(crate) fn axis_number(axis: Axis) -> u32 {
    to_table(&AXES, axis)
}

pub(crate) fn direction(value: u32) -> Result<Direction, AnyboxStatus> {
    from_table(&DIRECTIONS, value, "direction")
}

pub(crate) fn action(value: u32) -> Result<Action, AnyboxStatus> {
    from_table(&ACTIONS, value, "action")
}

pub(crate) fn action_number(action: Action) -> u32 {
    to_table(&ACTIONS, action)
}

/// a mask with one bit per action, bit n for action n
pub(crate) fn actions(mask: u64) -> Result<Vec<Action>, AnyboxStatus> {
    let known = (1u64 << ACTIONS.len()) - 1;
    if mask & !known != 0 {
        return Err(invalid(format!(
            "the action mask {mask:#x} has bits for actions anybox doesn't know"
        )));
    }
    Ok(ACTIONS
        .iter()
        .filter(|(number, _)| mask & (1 << number) != 0)
        .map(|(_, action)| *action)
        .collect())
}

pub(crate) fn event_kinds(mask: u32) -> Result<EventKinds, AnyboxStatus> {
    if mask & !ANYBOX_EVENTS_ALL != 0 {
        return Err(invalid(format!(
            "the event mask {mask:#x} has bits for kinds anybox doesn't know"
        )));
    }
    Ok(EVENT_KINDS
        .iter()
        .filter(|(bit, _)| mask & bit != 0)
        .fold(EventKinds::NONE, |kinds, (_, kind)| kinds | *kind))
}

pub(crate) fn execution_number(execution: &Execution) -> u32 {
    match execution {
        Execution::Native => ANYBOX_EXECUTION_NATIVE,
        Execution::Host => ANYBOX_EXECUTION_HOST,
        Execution::Unsupported { .. } => ANYBOX_EXECUTION_UNSUPPORTED,
        other => panic!("the C ABI has no number for {other:?} yet"),
    }
}

pub(crate) fn click_timing(timing: &AnyboxClickTiming) -> Result<ClickTiming, AnyboxStatus> {
    match timing.kind {
        ANYBOX_CLICK_TIMING_MAKCU => Ok(ClickTiming::Makcu),
        ANYBOX_CLICK_TIMING_FERRUM => Ok(ClickTiming::Ferrum),
        ANYBOX_CLICK_TIMING_FIXED => Ok(ClickTiming::Fixed(Duration::from_micros(timing.fixed_us))),
        ANYBOX_CLICK_TIMING_OFF => Ok(ClickTiming::Off),
        other => Err(invalid(format!(
            "{other} isn't a click timing anybox knows"
        ))),
    }
}

pub(crate) fn click_timing_struct(timing: ClickTiming) -> AnyboxClickTiming {
    let (kind, fixed_us) = match timing {
        ClickTiming::Makcu => (ANYBOX_CLICK_TIMING_MAKCU, 0),
        ClickTiming::Ferrum => (ANYBOX_CLICK_TIMING_FERRUM, 0),
        ClickTiming::Fixed(hold) => (ANYBOX_CLICK_TIMING_FIXED, micros(hold)),
        ClickTiming::Off => (ANYBOX_CLICK_TIMING_OFF, 0),
        other => panic!("the C ABI has no number for {other:?} yet"),
    };
    AnyboxClickTiming { kind, fixed_us }
}

pub(crate) fn click(click: &AnyboxClick) -> Click {
    let base = if click.has_hold {
        Click::hold(Duration::from_micros(click.hold_us))
    } else {
        Click::default()
    };
    if click.native_only {
        base.native_only()
    } else {
        base
    }
}

/// nothing in anybox is finer than a microsecond
pub(crate) fn micros(duration: Duration) -> u64 {
    u64::try_from(duration.as_micros()).unwrap_or(u64::MAX)
}

pub(crate) fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}
