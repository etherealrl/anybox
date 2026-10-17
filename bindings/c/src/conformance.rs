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
    "press_media",
    "release_media",
    "tap_media",
    "tap_media_with",
    "lock_button",
    "unlock_button",
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

/// a fieldless value's anybox.h name, checked to be there under its prefix, so a name that drifted fails here
fn name(header: &BTreeMap<String, i64>, prefix: &str, value: impl std::fmt::Debug) -> Value {
    let debug = format!("{value:?}");
    let named = upper(debug.split(|c: char| !c.is_alphanumeric()).next().unwrap());
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
        .find(|value| upper(&format!("{value:?}")) == wanted)
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
        "press_media" => device.press_media(media(0)),
        "release_media" => device.release_media(media(0)),
        "tap_media" => device.tap_media(media(0)),
        "tap_media_with" => device.tap_media_with(media(0), click(1)),
        "lock_button" => device.lock_button(button(0)),
        "unlock_button" => device.unlock_button(button(0)),
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

fn run(scenario: &Value, header: &BTreeMap<String, i64>) -> Value {
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

fn scenarios() -> Vec<Value> {
    let text = std::fs::read_to_string(root().join("bindings/conformance/scenarios.json")).unwrap();
    serde_json::from_str::<Value>(&text)
        .unwrap()
        .as_array()
        .unwrap()
        .clone()
}

#[test]
fn record_what_the_rust_crate_does() {
    // a recording that fails halfway mustn't leave the last one behind, the runners would pass against it
    let path = root().join("target/conformance/expected.json");
    if path.exists() {
        std::fs::remove_file(&path).unwrap();
    }
    let header = header();
    let expected: Vec<Value> = scenarios()
        .iter()
        .map(|scenario| run(scenario, &header))
        .collect();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, serde_json::to_string_pretty(&expected).unwrap()).unwrap();
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

/// every whole word starting with `anybox_`
fn anybox_names(code: &str) -> std::collections::BTreeSet<String> {
    let word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut names = std::collections::BTreeSet::new();
    for (at, _) in code.match_indices("anybox_") {
        if code[..at].chars().next_back().is_some_and(word) {
            continue;
        }
        let name: String = code[at..].chars().take_while(|c| word(*c)).collect();
        names.insert(name);
    }
    names
}

// the C++ wrapper is written by hand around its generated plain calls, so like the Python and C# header tests this
// makes it reach every function the header has, or leave it out with the reason

#[test]
fn every_header_function_is_used_by_the_cpp_wrapper_or_left_out_on_purpose() {
    let header = std::fs::read_to_string(root().join("bindings/c/include/anybox.h")).unwrap();
    // a declaration starts at the start of its line with its return type, a comment or a #define never does
    let declared: std::collections::BTreeSet<String> = header
        .lines()
        .filter(|line| line.starts_with(|c: char| c.is_ascii_alphabetic()) && line.contains('('))
        .flat_map(|line| {
            anybox_names(line)
                .into_iter()
                .filter(|name| line.contains(&format!("{name}(")))
        })
        .collect();
    let wrapper = std::fs::read_to_string(root().join("bindings/c/include/anybox.hpp")).unwrap();
    let used = anybox_names(&cpp_code(&wrapper));
    // connect is `Connect().open()`, a struct of zeros is the default options, the plain click and taps are the
    // `_with` ones with no hold, and a scan entry's error is read from the last error its result call sets, which
    // also has the message and the lists
    let left_out = [
        "anybox_connect",
        "anybox_connect_options_default",
        "anybox_device_click",
        "anybox_device_tap_key",
        "anybox_device_tap_media",
        "anybox_scan_list_error_detail",
    ];
    let missing: Vec<&String> = declared
        .iter()
        .filter(|name| !used.contains(*name) && !left_out.contains(&name.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "anybox.hpp doesn't use {missing:?}, wrap them or leave them out here with the reason"
    );
    for name in left_out {
        assert!(declared.contains(name), "anybox.h has no {name} any more");
        assert!(
            !used.contains(name),
            "anybox.hpp uses {name} now, take it off the left out list"
        );
    }
    assert!(
        declared.len() > 70,
        "only {} functions found in anybox.h",
        declared.len()
    );
}

// the scenarios have to reach everything, otherwise a binding could get something wrong that no runner ever sees

#[test]
fn every_plain_device_function_is_a_call_and_every_call_is_in_a_scenario() {
    let text = std::fs::read_to_string(root().join("bindings/c/include/anybox.h")).unwrap();
    let mut declared: Vec<&str> = text
        .split("anybox_device_")
        .skip(1)
        .filter_map(|rest| rest.split_once('(').map(|(name, _)| name))
        .filter(|name| name.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
        .filter(|name| !NOT_CALLS.contains(name))
        .collect();
    declared.sort_unstable();
    declared.dedup();
    let mut calls = CALLS.to_vec();
    calls.sort_unstable();
    assert_eq!(
        declared, calls,
        "a device function in anybox.h that's neither a call nor in NOT_CALLS"
    );
    let used: Vec<String> = scenarios()
        .iter()
        .flat_map(|scenario| scenario["steps"].as_array().unwrap().clone())
        .filter_map(|step| {
            step.get("call")
                .map(|name| name.as_str().unwrap().to_string())
        })
        .collect();
    for name in CALLS {
        assert!(
            used.iter().any(|used| used == name),
            "no scenario calls {name}"
        );
    }
}

#[test]
fn every_error_and_every_event_is_in_a_scenario() {
    let header = header();
    let steps: Vec<Value> = scenarios()
        .iter()
        .flat_map(|scenario| scenario["steps"].as_array().unwrap().clone())
        .collect();
    let examples: Vec<&Value> = steps
        .iter()
        .filter_map(|step| step.get("example"))
        .collect();
    for example in Error::examples() {
        let wanted = name(&header, "STATUS", &example);
        assert!(
            examples.contains(&&wanted),
            "no scenario has the example of {example:?}"
        );
    }
    // that the examples have every kind of event is the core's own test
    let kinds = Event::examples(Stamp::new(Instant::now(), Duration::ZERO)).len();
    assert!(
        steps
            .iter()
            .any(|step| step.get("example_events").is_some()),
        "no scenario sends the example events"
    );
    assert!(
        steps
            .iter()
            .any(|step| step.get("recv").and_then(Value::as_u64) == Some(kinds as u64)),
        "no scenario receives all {kinds} example events"
    );
}
