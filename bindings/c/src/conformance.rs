// the recorder of the conformance suite. it runs every scenario in bindings/conformance/scenarios.json through the
// Rust crate itself and writes what came out to target/conformance/expected.json, which the Python, C# and C++
// runners hold their binding to. a scenario is a name, whether the mock has a keyboard, and its steps, and a step is
// a call with its arguments, a snapshot, a push on the mock, a poller tick, a receive, the example events, an
// example error or a value's text. what each step gave is recorded with the frames the box got during it, and every
// value goes in and comes out by its anybox.h name, made here from the Rust names and never through the C ABI's own
// conversions, so a wrong mapping there shows up as a difference instead of on both sides

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::ptr;
use std::thread;
use std::time::{Duration, Instant};

use anybox::{
    Action, Axis, Button, Click, ClickTiming, Device, Direction, Error, Event, Execution, Family,
    FerrumApi, Firmware, Info, Quirk, Snapshot, StableId, Stamp,
};
use serde_json::{Value, json};

use crate::device::AnyboxDevice;
use crate::mock::*;
use crate::status::AnyboxStatus;
use crate::values::{ACTIONS, ANYBOX_CLICK_TIMING_FIXED, AnyboxClickTiming};

/// the device calls a scenario can make, each one is a C function the header has with the same name
const CALLS: &[&str] = &[
    // made by bindings/gen from its call table (calls), change the table and run cargo run -p anybox-gen
    "move_by",
    "move_now",
    "press",
    "release",
    "click",
    "click_with",
    "press_key",
    "release_key",
    "tap_key",
    "tap_key_with",
    "type_text",
    "press_keys",
    "release_keys",
    "tap_keys",
    "press_media",
    "release_media",
    "tap_media",
    "tap_media_with",
    "lock_button",
    "unlock_button",
    "force_release_key",
    "restore_key",
    "lock_key",
    "unlock_key",
    "lock_axis",
    "unlock_axis",
    "scale_axis",
    "wheel",
    "pan",
    "discard_pending_moves",
    // the end of what bindings/gen makes (calls)
];

/// every call only one family has, by its identity in the suite
const EXTENSIONS: &[&str] = &[
    // made by bindings/gen from its call table (makcu_calls), change the table and run cargo run -p anybox-gen
    "makcu.interpolation",
    "makcu.set_interpolation",
    "makcu.moving",
    "makcu.autorelease",
    "makcu.set_autorelease",
    // the end of what bindings/gen makes (makcu_calls)
    // made by bindings/gen from its call table (ysc_calls), change the table and run cargo run -p anybox-gen
    "ysc.interpolation",
    "ysc.set_interpolation",
    "ysc.reset_interpolation",
    "ysc.serial",
    // the end of what bindings/gen makes (ysc_calls)
    // made by bindings/gen from its call table (ferrum_calls), change the table and run cargo run -p anybox-gen
    "ferrum.catch_xy",
    // the end of what bindings/gen makes (ferrum_calls)
];

/// the device functions that aren't a plain call, each one has its own scenario step or its own tests
const NOT_CALLS: [&str; 5] = ["snapshot", "events", "medius", "close", "free"];

const BUTTONS: [Button; 5] = [
    Button::Left,
    Button::Right,
    Button::Middle,
    Button::Side1,
    Button::Side2,
];
const AXES: [Axis; 4] = [Axis::X, Axis::Y, Axis::Wheel, Axis::Pan];
const DIRECTIONS: [Direction; 3] = [Direction::Both, Direction::Positive, Direction::Negative];
const FAMILIES: [Family; 4] = [Family::Medius, Family::MakcuV4, Family::Ferrum, Family::Ysc];
const FERRUM_APIS: [FerrumApi; 2] = [FerrumApi::Software, FerrumApi::Legacy];

// cargo says where the crate is at run time too, and a test binary copied from another checkout has the other one's
// path built in, so it would read and write that one
pub(crate) fn root() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..")
}

/// every `#define ANYBOX_NAME number` in anybox.h
fn header() -> BTreeMap<String, i64> {
    let text = std::fs::read_to_string(root().join("bindings/c/include/anybox.h")).unwrap();
    text.lines()
        .filter_map(|line| {
            let mut parts = line.strip_prefix("#define ")?.split_whitespace();
            let name = parts.next()?;
            let number = parts.next()?.parse().ok()?;
            Some((name.to_string(), number))
        })
        .collect()
}

/// a Rust name the way anybox.h spells it, `MakcuV4` is `MAKCU_V4`
fn upper(name: &str) -> String {
    let mut out = String::new();
    let mut previous: Option<char> = None;
    for c in name.chars() {
        if c.is_ascii_uppercase()
            && previous.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit())
        {
            out.push('_');
        }
        out.push(c.to_ascii_uppercase());
        previous = Some(c);
    }
    out
}

/// a value's anybox.h name after its prefix, a nested one joined by its parts, so `Makcu(Interpolation)` is
/// `MAKCU_INTERPOLATION`
fn header_name(value: impl std::fmt::Debug) -> String {
    format!("{value:?}")
        .split(|c: char| !c.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(upper)
        .collect::<Vec<_>>()
        .join("_")
}

/// a fieldless value's anybox.h name, checked to be there under its prefix, so a name that drifted fails here
fn name(header: &BTreeMap<String, i64>, prefix: &str, value: impl std::fmt::Debug) -> Value {
    let debug = format!("{value:?}");
    // an error with fields is named by its variant, a nested action by all its parts
    let first = upper(debug.split(|c: char| !c.is_alphanumeric()).next().unwrap());
    let named = if header.contains_key(&format!("ANYBOX_{prefix}_{first}")) {
        first
    } else {
        header_name(&value)
    };
    let key = format!("ANYBOX_{prefix}_{named}");
    assert!(
        header.contains_key(&key),
        "anybox.h has no {key} for {debug}"
    );
    Value::String(named)
}

fn button_name(header: &BTreeMap<String, i64>, button: Button) -> Value {
    match button {
        Button::Extra(number) => json!({"Extra": number}),
        other => name(header, "BUTTON", other),
    }
}

/// the Rust value with that anybox.h name, out of the list of them
fn by_name<T: Copy + std::fmt::Debug>(all: &[T], wanted: &str) -> T {
    *all.iter()
        .find(|value| header_name(value) == wanted)
        .unwrap_or_else(|| panic!("the suite has no value named {wanted}"))
}

fn header_number(header: &BTreeMap<String, i64>, prefix: &str, value: &Value) -> i64 {
    match value {
        Value::String(named) => header[&format!("ANYBOX_{prefix}_{named}")],
        other => other
            .as_i64()
            .unwrap_or_else(|| panic!("{other} isn't a {prefix}")),
    }
}

fn arg<'a>(args: &'a [Value], index: usize, kind: &str) -> &'a Value {
    args[index]
        .get(kind)
        .unwrap_or_else(|| panic!("argument {index} isn't a {kind}: {}", args[index]))
}

fn call(
    device: &Device,
    header: &BTreeMap<String, i64>,
    name: &str,
    args: &[Value],
) -> anybox::Result<()> {
    let i16 = |index| arg(args, index, "i16").as_i64().unwrap() as i16;
    let button = |index| match arg(args, index, "Button") {
        Value::String(named) => by_name(&BUTTONS, named),
        extra => Button::Extra(extra["Extra"].as_u64().unwrap() as u8),
    };
    let key = |index| anybox::Key(header_number(header, "KEY", arg(args, index, "Key")) as u8);
    let text = |index| arg(args, index, "Text").as_str().unwrap().to_string();
    let keys = |index| -> Vec<anybox::Key> {
        arg(args, index, "Keys")
            .as_array()
            .unwrap()
            .iter()
            .map(|key| anybox::Key(header_number(header, "KEY", key) as u8))
            .collect()
    };
    let media = |index| {
        anybox::MediaKey(header_number(header, "MEDIA", arg(args, index, "MediaKey")) as u16)
    };
    let axis = |index| by_name(&AXES, arg(args, index, "Axis").as_str().unwrap());
    let direction = |index| by_name(&DIRECTIONS, arg(args, index, "Direction").as_str().unwrap());
    let click = |index| {
        let click = arg(args, index, "Click");
        let mut made = match click.get("hold_us") {
            Some(hold) => Click::hold(Duration::from_micros(hold.as_u64().unwrap())),
            None => Click::default(),
        };
        if click
            .get("native_only")
            .is_some_and(|only| only.as_bool().unwrap())
        {
            made = made.native_only();
        }
        made
    };
    match name {
        // made by bindings/gen from its call table (dispatch), change the table and run cargo run -p anybox-gen
        "move_by" => device.move_by(i16(0), i16(1)),
        "move_now" => device.move_now(i16(0), i16(1)),
        "press" => device.press(button(0)),
        "release" => device.release(button(0)),
        "click" => device.click(button(0)),
        "click_with" => device.click_with(button(0), click(1)),
        "press_key" => device.press_key(key(0)),
        "release_key" => device.release_key(key(0)),
        "tap_key" => device.tap_key(key(0)),
        "tap_key_with" => device.tap_key_with(key(0), click(1)),
        "type_text" => device.type_text(&text(0)),
        "press_keys" => device.press_keys(&keys(0)),
        "release_keys" => device.release_keys(&keys(0)),
        "tap_keys" => device.tap_keys(&keys(0)),
        "press_media" => device.press_media(media(0)),
        "release_media" => device.release_media(media(0)),
        "tap_media" => device.tap_media(media(0)),
        "tap_media_with" => device.tap_media_with(media(0), click(1)),
        "lock_button" => device.lock_button(button(0)),
        "unlock_button" => device.unlock_button(button(0)),
        "force_release_key" => device.force_release_key(key(0)),
        "restore_key" => device.restore_key(key(0)),
        "lock_key" => device.lock_key(key(0)),
        "unlock_key" => device.unlock_key(key(0)),
        "lock_axis" => device.lock_axis(axis(0), direction(1)),
        "unlock_axis" => device.unlock_axis(axis(0), direction(1)),
        "scale_axis" => device.scale_axis(axis(0), direction(1), i16(2)),
        "wheel" => device.wheel(i16(0)),
        "pan" => device.pan(i16(0)),
        "discard_pending_moves" => device.discard_pending_moves(),
        // the end of what bindings/gen makes (dispatch)
        other => panic!("there's no call {other}"),
    }
}

fn firmware(firmware: &Firmware) -> Value {
    match firmware {
        Firmware::Version {
            major,
            minor,
            patch,
        } => json!({"version": [major, minor, patch]}),
        Firmware::Build(build) => json!({"build": build}),
        Firmware::Protocol(protocol) => json!({"protocol": protocol}),
        Firmware::Text(text) => json!({"text": text}),
        other => panic!("the suite doesn't know the firmware {other:?}"),
    }
}

fn id(id: &StableId) -> Value {
    match id {
        StableId::Mac(mac) => json!({"mac": hex(mac)}),
        StableId::UsbSerial(serial) => json!({"usb_serial": serial}),
        other => panic!("the suite doesn't know the id {other:?}"),
    }
}

/// a firmware the way `firmware` writes one
fn firmware_of(value: &Value) -> Firmware {
    let number = |field: &str| value[field].as_u64().unwrap();
    if let Some(version) = value.get("version") {
        let part = |at: usize| version[at].as_u64().unwrap() as u16;
        Firmware::Version {
            major: part(0),
            minor: part(1),
            patch: part(2),
        }
    } else if value.get("build").is_some() {
        Firmware::Build(number("build") as u32)
    } else if value.get("protocol").is_some() {
        Firmware::Protocol(number("protocol") as u8)
    } else {
        Firmware::Text(value["text"].as_str().unwrap().to_string())
    }
}

/// an id the way `id` writes one
fn id_of(value: &Value) -> StableId {
    match value.get("mac") {
        Some(mac) => {
            let mac = mac.as_str().unwrap();
            let byte = |at: usize| u8::from_str_radix(&mac[at * 2..at * 2 + 2], 16).unwrap();
            StableId::Mac(std::array::from_fn(byte))
        }
        None => StableId::UsbSerial(value["usb_serial"].as_str().unwrap().to_string()),
    }
}

/// the Display text of a value given like a call's argument, `{"Action": "MOVE"}`, or of a firmware or an id given
/// like a snapshot writes them
fn text(header: &BTreeMap<String, i64>, value: &Value) -> String {
    let (kind, raw) = value
        .as_object()
        .and_then(|object| object.iter().next())
        .unwrap_or_else(|| panic!("a text step names one value: {value}"));
    let named = || {
        raw.as_str()
            .unwrap_or_else(|| panic!("{kind} goes by its anybox.h name: {raw}"))
    };
    match kind.as_str() {
        "Action" => {
            let actions: Vec<Action> = ACTIONS.iter().map(|(_, action)| *action).collect();
            by_name(&actions, named()).to_string()
        }
        "Axis" => by_name(&AXES, named()).to_string(),
        "Button" => match raw {
            Value::String(name) => by_name(&BUTTONS, name),
            extra => Button::Extra(extra["Extra"].as_u64().unwrap() as u8),
        }
        .to_string(),
        "Family" => by_name(&FAMILIES, named()).to_string(),
        "FerrumApi" => by_name(&FERRUM_APIS, named()).to_string(),
        "Key" => anybox::Key(header_number(header, "KEY", raw) as u8).to_string(),
        "MediaKey" => anybox::MediaKey(header_number(header, "MEDIA", raw) as u16).to_string(),
        "Firmware" => firmware_of(raw).to_string(),
        "StableId" => id_of(raw).to_string(),
        other => panic!("the suite has no text for {other}"),
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn micros(duration: Duration) -> u64 {
    duration.as_micros() as u64
}

fn info(header: &BTreeMap<String, i64>, info: &Info) -> Value {
    json!({
        "port": info.port,
        "family": name(header, "FAMILY", info.family),
        "model": info.model,
        "firmware": info.firmware.as_ref().map(firmware),
        "id": info.id.as_ref().map(id),
        "ferrum_api": info.ferrum_api.map(|api| name(header, "FERRUM_API", api)),
        "peripheral": info.peripheral.map(|ids| json!({"vid": ids.vid, "pid": ids.pid})),
    })
}

fn click_timing(header: &BTreeMap<String, i64>, timing: ClickTiming) -> Value {
    let fixed = match timing {
        ClickTiming::Fixed(hold) => micros(hold),
        _ => 0,
    };
    json!({"kind": name(header, "CLICK_TIMING", timing), "fixed_us": fixed})
}

/// a quirk flat, the way every binding keeps one, with 0 and none for what its kind doesn't have
fn quirk(header: &BTreeMap<String, i64>, quirk: &Quirk) -> Value {
    let (min, max, count, timing) = match quirk {
        Quirk::ForcedUpOnRelease { min, max } | Quirk::BoxClickTiming { min, max } => {
            (micros(*min), micros(*max), 0, None)
        }
        Quirk::MaxHold(hold) => (0, micros(*hold), 0, None),
        Quirk::NotchesPerCommand(notches) => (0, 0, u32::from(*notches), None),
        Quirk::MaxKeysHeld(keys) => (0, 0, u32::from(*keys), None),
        Quirk::HostClickTiming(timing) => (0, 0, 0, Some(*timing)),
        _ => (0, 0, 0, None),
    };
    json!({
        "kind": name(header, "QUIRK", quirk),
        "min_us": min,
        "max_us": max,
        "count": count,
        "click_timing": timing.map_or_else(|| json!({"kind": null, "fixed_us": 0}), |timing| click_timing(header, timing)),
    })
}

fn snapshot(header: &BTreeMap<String, i64>, snapshot: &Snapshot) -> Value {
    let capabilities: Vec<Value> = ACTIONS
        .iter()
        .map(|(_, action)| {
            let capability = snapshot.capabilities.get(*action);
            let reason = match &capability.execution {
                Execution::Unsupported { reason } => Some(reason.to_string()),
                _ => None,
            };
            let quirks: Vec<Value> = capability
                .quirks
                .iter()
                .map(|one| quirk(header, one))
                .collect();
            json!({
                "action": name(header, "ACTION", action),
                "execution": name(header, "EXECUTION", &capability.execution),
                "reason": reason,
                "quirks": quirks,
            })
        })
        .collect();
    let buttons: Vec<Value> = snapshot
        .capabilities
        .buttons()
        .iter()
        .map(|button| button_name(header, *button))
        .collect();
    let axes: Vec<Value> = snapshot
        .capabilities
        .axes()
        .iter()
        .map(|axis| name(header, "AXIS", axis))
        .collect();
    json!({"info": info(header, &snapshot.info), "capabilities": capabilities, "buttons": buttons, "axes": axes})
}

fn actions(header: &BTreeMap<String, i64>, actions: &[Action]) -> Value {
    let mut names: Vec<String> = actions
        .iter()
        .map(|action| name(header, "ACTION", action).as_str().unwrap().to_string())
        .collect();
    names.sort_unstable();
    json!(names)
}

fn error(header: &BTreeMap<String, i64>, error: &Error) -> Value {
    let action = |action: &Action| name(header, "ACTION", action);
    let family = |family: &anybox::Family| name(header, "FAMILY", family);
    let fields = match error {
        Error::NotFound { tried, .. } => json!({"tried": tried}),
        Error::PortBusy { port, .. } | Error::PermissionDenied { port, .. } => {
            json!({"port": port})
        }
        Error::NoBoxCan { boxes, .. } => {
            let boxes: Vec<Value> = boxes
                .iter()
                .map(|(found, missing)| json!({"port": found.port, "missing": actions(header, missing)}))
                .collect();
            json!({"boxes": boxes})
        }
        Error::WrongFamily {
            port,
            expected,
            found,
            ..
        } => {
            json!({"port": port, "expected": family(expected), "found": found.as_ref().map(family)})
        }
        Error::WrongBox {
            port,
            expected,
            found,
            ..
        } => json!({"port": port, "expected": id(expected), "found": found.as_ref().map(id)}),
        Error::WrongFerrumApi {
            port,
            expected,
            found,
            ..
        } => json!({
            "port": port,
            "expected": name(header, "FERRUM_API", expected),
            "found": name(header, "FERRUM_API", found),
        }),
        Error::FirmwareTooOld {
            family: of,
            found,
            needed,
            ..
        } => json!({"family": family(of), "found": firmware(found), "needed": firmware(needed)}),
        Error::LibraryTooOld {
            family: of, found, ..
        } => json!({"family": family(of), "found": firmware(found)}),
        Error::Unsupported {
            action: what,
            family: of,
            reason,
            ..
        } => json!({"action": action(what), "family": family(of), "reason": reason}),
        Error::InvalidArgument {
            action: what,
            detail,
            ..
        } => json!({"action": action(what), "detail": detail}),
        Error::Rejected {
            action: what,
            detail,
            ..
        } => json!({"action": action(what), "detail": detail}),
        Error::Timeout { action: what, .. } | Error::Interrupted { action: what, .. } => {
            json!({"action": action(what)})
        }
        Error::Io(io) => {
            // a kind anybox.h has no name for is OTHER, like in the C ABI
            let kind = upper(&format!("{:?}", io.kind()));
            let kind = if header.contains_key(&format!("ANYBOX_IO_KIND_{kind}")) {
                kind
            } else {
                "OTHER".into()
            };
            json!({"kind": kind, "os_error": io.raw_os_error()})
        }
        Error::Closed | Error::Disconnected => json!({}),
        // a new kind has fields the suite has to compare, so it can't fall through to none
        other => panic!(
            "the suite doesn't know the error {other:?}, add its fields here and in the runners"
        ),
    };
    json!({"status": name(header, "STATUS", error), "text": error.to_string(), "fields": fields})
}

/// the Rust variant's name, which is what the bindings' event types are called too
fn event_name(event: &Event) -> String {
    let debug = format!("{event:?}");
    debug
        .split(|c: char| !c.is_alphanumeric())
        .next()
        .unwrap()
        .to_string()
}

fn at(header: &BTreeMap<String, i64>, at: &Stamp) -> Value {
    json!({"box_time": at.box_time.map(|box_time| json!({"micros": box_time.micros, "clock": name(header, "CLOCK", box_time.clock)}))})
}

fn event(header: &BTreeMap<String, i64>, event: &Event) -> Value {
    let fields = match event {
        Event::Button {
            button,
            pressed,
            at: stamp,
            ..
        } => {
            json!({"button": button_name(header, *button), "pressed": pressed, "at": at(header, stamp)})
        }
        Event::Key {
            key,
            pressed,
            at: stamp,
            ..
        } => json!({"key": key.0, "pressed": pressed, "at": at(header, stamp)}),
        Event::Media {
            key,
            pressed,
            at: stamp,
            ..
        } => json!({"key": key.0, "pressed": pressed, "at": at(header, stamp)}),
        Event::Motion {
            dx,
            dy,
            wheel,
            pan,
            at: stamp,
            ..
        } => json!({"dx": dx, "dy": dy, "wheel": wheel, "pan": pan, "at": at(header, stamp)}),
        Event::Dropped { count, source, .. } => {
            json!({"count": count, "source": name(header, "DROP_SOURCE", source)})
        }
        Event::Rejected { action, count, .. } => {
            json!({"action": name(header, "ACTION", action), "count": count})
        }
        Event::SessionReset | Event::Disconnected | Event::Reconnected | Event::Changed => {
            json!({})
        }
        // a new kind has fields the suite has to compare, so it can't fall through to none
        other => panic!(
            "the suite doesn't know the event {other:?}, add its fields here and in the runners"
        ),
    };
    json!({"event": event_name(event), "fields": fields})
}

struct Session {
    mock: *mut AnyboxMock,
    device: *mut AnyboxDevice,
}

impl Session {
    fn open(keyboard: bool) -> Self {
        // the same click timing in every language, since it shows in the snapshot's quirks, and a manual mock, so no
        // link check lands between a call's frames
        let timing = AnyboxClickTiming {
            kind: ANYBOX_CLICK_TIMING_FIXED,
            fixed_us: 5_000,
        };
        let mut mock = ptr::null_mut();
        let mut device = ptr::null_mut();
        let status: AnyboxStatus =
            unsafe { anybox_mock_medius_open_manual(&timing, keyboard, &mut mock, &mut device) };
        assert_eq!(status, 0);
        Self { mock, device }
    }

    fn device(&self) -> &Device {
        unsafe { &(*self.device).0 }
    }

    fn frames(&self) -> Vec<Value> {
        unsafe { &(*self.mock).0 }
            .recorded_frames()
            .iter()
            .filter(|frame| !keepalive(frame.ty as u8, &frame.payload))
            .map(|frame| json!([frame.ty as u8, hex(&frame.payload)]))
            .collect()
    }
}

/// what medius' own keepalive sends every 500 ms while something's held, on its own thread, which anybox can't stop
/// or time, so a stats query (QUERY with Q_STATS) and the subscriptions sent again (CATCH). no call sends either,
/// so the recorder and every runner leave them out, otherwise one landing inside a hold would fail the scenario
fn keepalive(ty: u8, payload: &[u8]) -> bool {
    (ty == 0x05 && payload == [0x05]) || ty == 0x0B
}

impl Drop for Session {
    fn drop(&mut self) {
        unsafe {
            crate::device::anybox_device_free(self.device);
            anybox_mock_free(self.mock);
        }
    }
}

fn push(session: &Session, header: &BTreeMap<String, i64>, step: &Value) {
    let field = |name: &str| step[name].as_i64().unwrap();
    let pressed = step
        .get("pressed")
        .is_some_and(|pressed| pressed.as_bool().unwrap());
    let held = |prefix: &str| -> Vec<i64> {
        step["held"]
            .as_array()
            .unwrap()
            .iter()
            .map(|key| header_number(header, prefix, key))
            .collect()
    };
    let status = unsafe {
        match step["push"].as_str().unwrap() {
            "buttons" => anybox_mock_push_buttons(
                session.mock,
                field("seq") as u8,
                field("ts_us") as u32,
                pressed,
                field("held") as u32,
            ),
            "keys" => {
                let keys: Vec<u8> = held("KEY").into_iter().map(|key| key as u8).collect();
                anybox_mock_push_keys(
                    session.mock,
                    field("seq") as u8,
                    field("ts_us") as u32,
                    pressed,
                    keys.as_ptr(),
                    keys.len(),
                )
            }
            "media" => {
                let media: Vec<u16> = held("MEDIA").into_iter().map(|key| key as u16).collect();
                anybox_mock_push_media(
                    session.mock,
                    field("seq") as u8,
                    field("ts_us") as u32,
                    pressed,
                    media.as_ptr(),
                    media.len(),
                )
            }
            "motion" => anybox_mock_push_motion(
                session.mock,
                field("seq") as u8,
                field("ts_us") as u32,
                field("dx") as i16,
                field("dy") as i16,
                field("wheel") as i16,
                field("pan") as i16,
            ),
            "link_lost" => anybox_mock_link_lost(session.mock),
            "restart" => anybox_mock_restart(session.mock),
            other => panic!("the mock can't push {other}"),
        }
    };
    assert_eq!(status, 0, "{step}");
}

/// a receiver subscribes each kind on the device's own threads, so a push before its kind is in would get dropped
/// like on a real box
fn subscribed(session: &Session, kinds: u32) {
    let start = Instant::now();
    while !unsafe { anybox_mock_subscribed(session.mock, kinds) } {
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "the receiver never subscribed"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn kinds(name: &str) -> anybox::EventKinds {
    use anybox::EventKinds;
    match name {
        "ALL" => EventKinds::ALL,
        "BUTTONS" => EventKinds::BUTTONS,
        "MOTION" => EventKinds::MOTION,
        "KEYS" => EventKinds::KEYS,
        "MEDIA" => EventKinds::MEDIA,
        other => panic!("the suite doesn't know the event kinds {other}"),
    }
}

/// what comes in after the events a step waits for, so an extra one shows up instead of being left in the queue
const QUIET: Duration = Duration::from_millis(150);

/// one call only one family has, `makcu.interpolation` and the like. on another box the handle is none, which every
/// binding gives as `{"none": null}`
fn extension(device: &Device, header: &BTreeMap<String, i64>, step: &Value) -> Value {
    let name = step["extension"].as_str().unwrap();
    let args = step["args"].as_array().unwrap();
    let interpolation = |index| match &arg(args, index, "Interpolation") {
        Value::String(auto) if auto == "AUTO" => anybox::Interpolation::Auto,
        percent => anybox::Interpolation::Percent(percent.as_u64().unwrap() as u8),
    };
    let ysc_settings = |index| {
        let settings = arg(args, index, "YscSettings");
        let ms = |field: &str| Duration::from_millis(settings[field].as_u64().unwrap());
        anybox::YscInterpolationSettings {
            enabled: settings["enabled"].as_bool().unwrap(),
            profile: profile(settings["profile"].as_u64().unwrap() as u32),
            window: ms("window_ms"),
            max_window: ms("max_window_ms"),
        }
    };
    let ms_or_off = |index| arg(args, index, "ms").as_u64().map(Duration::from_millis);
    let ms = |index| ms_or_off(index).unwrap();
    let flag = |index| arg(args, index, "bool").as_bool().unwrap();
    let result: Option<anybox::Result<Value>> = match name {
        // made by bindings/gen from its call table (makcu_dispatch), change the table and run cargo run -p anybox-gen
        "makcu.interpolation" => device
            .makcu()
            .map(|makcu| makcu.interpolation().map(interpolation_json)),
        "makcu.set_interpolation" => device.makcu().map(|makcu| {
            makcu
                .set_interpolation(interpolation(0))
                .map(|()| Value::Null)
        }),
        "makcu.moving" => device.makcu().map(|makcu| makcu.moving().map(Value::Bool)),
        "makcu.autorelease" => device.makcu().map(|makcu| makcu.autorelease().map(ms_json)),
        "makcu.set_autorelease" => device
            .makcu()
            .map(|makcu| makcu.set_autorelease(ms_or_off(0)).map(|()| Value::Null)),
        // the end of what bindings/gen makes (makcu_dispatch)
        // made by bindings/gen from its call table (ysc_dispatch), change the table and run cargo run -p anybox-gen
        "ysc.interpolation" => device.ysc().map(|ysc| ysc.interpolation().map(ysc_json)),
        "ysc.set_interpolation" => device
            .ysc()
            .map(|ysc| ysc.set_interpolation(ysc_settings(0)).map(|()| Value::Null)),
        "ysc.reset_interpolation" => device
            .ysc()
            .map(|ysc| ysc.reset_interpolation().map(ysc_json)),
        "ysc.serial" => device.ysc().map(|ysc| ysc.serial().map(serial_json)),
        // the end of what bindings/gen makes (ysc_dispatch)
        // made by bindings/gen from its call table (ferrum_dispatch), change the table and run cargo run -p anybox-gen
        "ferrum.catch_xy" => device
            .ferrum()
            .map(|ferrum| ferrum.catch_xy(ms(0), flag(1)).map(xy_json)),
        // the end of what bindings/gen makes (ferrum_dispatch)
        other => panic!("there's no extension call {other}"),
    };
    match result {
        None => json!({"none": null}),
        Some(Ok(value)) => json!({"ok": value}),
        Some(Err(failed)) => json!({"error": error(header, &failed)}),
    }
}

fn interpolation_json(value: anybox::Interpolation) -> Value {
    match value {
        anybox::Interpolation::Auto => json!({"Interpolation": "AUTO"}),
        anybox::Interpolation::Percent(n) => json!({"Interpolation": n}),
    }
}

fn ms_json(value: Option<Duration>) -> Value {
    json!(value.map(|value| value.as_millis() as u64))
}

fn ysc_json(value: anybox::YscInterpolation) -> Value {
    json!({
        "enabled": value.enabled,
        "profile": profile_number(value.profile),
        "window_ms": ms_json(value.window),
        "max_window_ms": ms_json(value.max_window),
        "measured_interval_us": value.measured_interval.map(micros),
    })
}

fn serial_json(serial: anybox::YscSerial) -> Value {
    json!({"custom": serial.custom, "physical": serial.physical, "active": serial.active})
}

fn xy_json((x, y): (i32, i32)) -> Value {
    json!([x, y])
}

fn profile_number(profile: anybox::YscProfile) -> u32 {
    match profile {
        anybox::YscProfile::Linear => 0,
        anybox::YscProfile::EaseInOut => 1,
        anybox::YscProfile::MinJerk => 2,
        anybox::YscProfile::Other(n) => n,
        other => panic!("the suite has no number for {other:?}"),
    }
}

fn profile(number: u32) -> anybox::YscProfile {
    match number {
        0 => anybox::YscProfile::Linear,
        1 => anybox::YscProfile::EaseInOut,
        2 => anybox::YscProfile::MinJerk,
        n => anybox::YscProfile::Other(n),
    }
}

/// a scenario on one family's mock, which only has extension steps since the mock records no frames
fn run_family(scenario: &Value, header: &BTreeMap<String, i64>, family: &str) -> Value {
    let timing = ClickTiming::Fixed(Duration::from_millis(5));
    let device = match family {
        "makcu" => Device::mock_makcu(timing),
        "ysc" => Device::mock_ysc(timing),
        "ferrum" => Device::mock_ferrum(timing),
        other => panic!("there's no mock {other}"),
    }
    .unwrap();
    let steps: Vec<Value> = scenario["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|step| {
            let mut out = step.as_object().unwrap().clone();
            out.insert("outcome".into(), extension(&device, header, step));
            Value::Object(out)
        })
        .collect();
    device.close().unwrap();
    let mut out = scenario.as_object().unwrap().clone();
    out.insert("steps".into(), Value::Array(steps));
    Value::Object(out)
}

fn run(scenario: &Value, header: &BTreeMap<String, i64>) -> Value {
    if let Some(family) = scenario.get("box") {
        return run_family(scenario, header, family.as_str().unwrap());
    }
    let keyboard = scenario["keyboard"].as_bool().unwrap();
    let session = Session::open(keyboard);
    let events = scenario.get("events").map(|wanted| {
        let wanted = wanted.as_str().unwrap();
        let receiver = session.device().events_of(kinds(wanted));
        let mut streamed = header[&format!("ANYBOX_EVENTS_{wanted}")];
        // a mock with no keyboard has no keys or media to subscribe
        if !keyboard {
            streamed &= header["ANYBOX_EVENTS_BUTTONS"] | header["ANYBOX_EVENTS_MOTION"];
        }
        subscribed(&session, streamed as u32);
        receiver
    });
    let recv = |count: usize, tick: bool| -> Value {
        let receiver = events
            .as_ref()
            .expect("a scenario that receives says which events");
        let start = Instant::now();
        let mut got = Vec::new();
        let mut quiet_from = None;
        loop {
            if got.len() >= count && quiet_from.get_or_insert_with(Instant::now).elapsed() >= QUIET
            {
                break;
            }
            assert!(
                start.elapsed() < Duration::from_secs(10),
                "only {} of {count} events came",
                got.len()
            );
            // no ticks while it waits for extras, so only what the scenario set off can come
            if tick && got.len() < count {
                assert_eq!(unsafe { anybox_mock_tick(session.mock) }, 0);
            }
            if let Ok(next) = receiver.recv_timeout(Duration::from_millis(50)) {
                got.push(event(header, &next));
            }
        }
        json!({"events": got})
    };
    let steps: Vec<Value> = scenario["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|step| {
            let mut out = step.as_object().unwrap().clone();
            let outcome = if let Some(name) = step.get("call") {
                unsafe { (*session.mock).0.clear_recorded() };
                let started = Instant::now();
                let result = call(
                    session.device(),
                    header,
                    name.as_str().unwrap(),
                    step["args"].as_array().unwrap(),
                );
                // a hold on a box that can't time it is a wait on the PC, which no frame shows
                if let Some(at_least) = step.get("takes_at_least_us") {
                    let took = started.elapsed();
                    assert!(
                        took >= Duration::from_micros(at_least.as_u64().unwrap()),
                        "{step} only took {took:?}"
                    );
                }
                out.insert("frames".into(), Value::Array(session.frames()));
                match result {
                    Ok(()) => json!({"ok": null}),
                    Err(failed) => json!({"error": error(header, &failed)}),
                }
            } else if step.get("extension").is_some() {
                extension(session.device(), header, step)
            } else if step.get("snapshot").is_some() {
                json!({"snapshot": snapshot(header, &session.device().snapshot())})
            } else if step.get("push").is_some() {
                push(&session, header, step);
                Value::Null
            } else if let Some(wait) = step.get("wait_us") {
                thread::sleep(Duration::from_micros(wait.as_u64().unwrap()));
                Value::Null
            } else if let Some(count) = step.get("recv") {
                recv(count.as_u64().unwrap() as usize, false)
            } else if let Some(count) = step.get("tick_until") {
                recv(count.as_u64().unwrap() as usize, true)
            } else if step.get("example_events").is_some() {
                assert_eq!(unsafe { anybox_mock_example_events(session.device) }, 0);
                Value::Null
            } else if let Some(value) = step.get("text") {
                json!({"text": text(header, value)})
            } else if let Some(wanted) = step.get("example") {
                let example = Error::examples()
                    .into_iter()
                    .find(|example| &name(header, "STATUS", example) == wanted)
                    .unwrap_or_else(|| panic!("anybox has no example of {wanted}"));
                json!({"error": error(header, &example)})
            } else {
                panic!("the suite doesn't know the step {step}");
            };
            out.insert("outcome".into(), outcome);
            Value::Object(out)
        })
        .collect();
    let mut out = scenario.as_object().unwrap().clone();
    out.insert("steps".into(), Value::Array(steps));
    Value::Object(out)
}

/// C++ source with its comments and string literals taken out, so a comment or a message naming a function doesn't
/// count as calling it
pub(crate) fn cpp_code(source: &str) -> String {
    let mut out = String::new();
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        match (c, chars.peek()) {
            ('/', Some('/')) => {
                for skipped in chars.by_ref() {
                    if skipped == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                chars.next();
                let mut previous = ' ';
                for skipped in chars.by_ref() {
                    if previous == '*' && skipped == '/' {
                        break;
                    }
                    previous = skipped;
                }
                out.push(' ');
            }
            ('"', _) => {
                while let Some(skipped) = chars.next() {
                    match skipped {
                        '\\' => {
                            chars.next();
                        }
                        '"' => break,
                        _ => {}
                    }
                }
                out.push_str("\"\"");
            }
            _ => out.push(c),
        }
    }
    out
}

mod tests;
