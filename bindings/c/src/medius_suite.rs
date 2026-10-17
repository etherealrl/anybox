// the recorder of the Medius suite. it runs every scenario in bindings/conformance/medius.json against the medius
// crate, the calls through medius-capi's own C functions (the crate's C face, which every binding sits on) and the
// helpers, texts and defaults through the crate itself, and writes what came out to target/conformance/medius-expected.json.
// the C++ and C# runners replay the same scenarios through their binding and hold it to that file. a value is the same
// JSON everywhere: a record keyed by medius.h's field names, bytes as hex, an enum by its C++ member name
//
// a restart (a RESET with the NVS flag, a reboot into DeviceRun, the mock's own restart) has to be a scenario's last
// step, and a scenario can't hold anything (a lock, a press, a subscription, a rule) across a session release on the
// box, like the mock's link_lost or detach, a patch that presents the clone again or the opt-in going off with rules
// held. medius recovers on its own thread after both, so when those frames land depends on timing. the recorder
// refuses a step after a restart, but nothing can check the release, the mock doesn't say when the box let go

use std::ffi::{CStr, CString, c_char};

use medius_capi::*;
use serde_json::{Map, Value, json};

mod generated;
mod hand;
mod helpers;
mod tests;

use generated::*;
use hand::*;

/// how a step that didn't give a value ended
pub(super) enum Failure {
    /// a binding refuses it before medius sees it, so no frame goes out
    Refused(String),
    /// medius' own error, its status, text, protocol byte and an update's detail
    Status(Value),
}

pub(super) type Outcome = Result<Value, Failure>;

/// the handles a scenario works on, made as it needs them
pub(super) struct Context {
    pub(super) mock: *mut MediusMockBox,
    pub(super) device: *mut MediusDevice,
    pub(super) clip: *mut MediusClip,
    pub(super) builder: *mut MediusClipBuilder,
    pub(super) frame: *mut MediusClipFrame,
    pub(super) timeline: *mut MediusTimeline,
    pub(super) events: *mut MediusEventStream,
    pub(super) inputs: *mut MediusInputStream,
    pub(super) logs: *mut MediusLogStream,
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            medius_event_stream_free(self.events);
            medius_input_stream_free(self.inputs);
            medius_log_stream_free(self.logs);
            medius_clip_free(self.clip);
            medius_clip_builder_free(self.builder);
            medius_clip_frame_free(self.frame);
            medius_timeline_free(self.timeline);
            medius_device_free(self.device);
            medius_mock_free(self.mock);
        }
    }
}

pub(super) fn number<T: TryFrom<i64>>(value: &Value, what: &str) -> T {
    value
        .as_i64()
        .and_then(|number| T::try_from(number).ok())
        .unwrap_or_else(|| panic!("{what} needs a number that fits, the scenario has {value}"))
}

pub(super) fn flag(value: &Value) -> bool {
    value
        .as_bool()
        .unwrap_or_else(|| panic!("a flag is true or false, the scenario has {value}"))
}

pub(super) fn hex(bytes: &[u8]) -> Value {
    Value::from(
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
    )
}

pub(super) fn unhex(value: &Value, what: &str) -> Vec<u8> {
    let text = value
        .as_str()
        .unwrap_or_else(|| panic!("{what} is bytes as hex, the scenario has {value}"));
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).unwrap())
        .collect()
}

pub(super) fn text(raw: &[c_char]) -> Value {
    let bytes: Vec<u8> = raw
        .iter()
        .take_while(|c| **c != 0)
        .map(|c| *c as u8)
        .collect();
    Value::from(String::from_utf8_lossy(&bytes).into_owned())
}

pub(super) fn c_text(raw: *const c_char) -> Value {
    if raw.is_null() {
        return Value::Null;
    }
    Value::from(
        unsafe { CStr::from_ptr(raw) }
            .to_string_lossy()
            .into_owned(),
    )
}

pub(super) fn c_string(value: &Value) -> CString {
    CString::new(
        value
            .as_str()
            .unwrap_or_else(|| panic!("a text argument, the scenario has {value}")),
    )
    .unwrap()
}

// cut to fit with room for the NUL, byte by byte like the bindings cut it
pub(super) fn write_text(out: &mut [c_char], value: &Value) {
    let Some(text) = value.as_str() else { return };
    let room = out.len() - 1;
    for (slot, byte) in out.iter_mut().zip(text.bytes().take(room)) {
        *slot = byte as c_char;
    }
}

pub(super) fn exact<const N: usize>(value: &Value, what: &str) -> Result<[u8; N], Failure> {
    unhex(value, what).try_into().map_err(|bytes: Vec<u8>| {
        Failure::Refused(format!("{what} is {} bytes, it has to be {N}", bytes.len()))
    })
}

pub(super) fn fill(out: &mut [u8], value: &Value, what: &str) -> Result<usize, Failure> {
    let bytes = unhex(value, what);
    if bytes.len() > out.len() {
        return Err(Failure::Refused(format!(
            "{what} is {} bytes, over {}",
            bytes.len(),
            out.len()
        )));
    }
    out[..bytes.len()].copy_from_slice(&bytes);
    Ok(bytes.len())
}

// a match or mask goes over with its whole length and as much as the array holds, so medius refuses one too long
pub(super) fn fill_match(out: &mut [u8], value: &Value, what: &str) -> Result<u16, Failure> {
    let bytes = unhex(value, what);
    let length = u16::try_from(bytes.len()).map_err(|_| {
        Failure::Refused(format!(
            "{what} is {} bytes, over what medius can be told",
            bytes.len()
        ))
    })?;
    let kept = bytes.len().min(out.len());
    out[..kept].copy_from_slice(&bytes[..kept]);
    Ok(length)
}

pub(super) fn fill_list<T>(
    out: &mut [T],
    value: &Value,
    what: &str,
    read: fn(&Value) -> Result<T, Failure>,
) -> Result<usize, Failure> {
    let items = value
        .as_array()
        .unwrap_or_else(|| panic!("{what} is a list, the scenario has {value}"));
    if items.len() > out.len() {
        return Err(Failure::Refused(format!(
            "{what} has {} entries, over {}",
            items.len(),
            out.len()
        )));
    }
    for (slot, item) in out.iter_mut().zip(items) {
        *slot = read(item)?;
    }
    Ok(items.len())
}

/// a name or a number for an enum, refused when a binding checks it and it names nothing
pub(super) fn enum_value(name: &str, value: &Value) -> Result<i64, Failure> {
    let number = match value {
        Value::String(member) => enum_number(name, member),
        _ => number(value, name),
    };
    if checked(name) && !enum_name(name, number).is_string() {
        return Err(Failure::Refused(format!("{number} isn't a {name}")));
    }
    Ok(number)
}

pub(super) fn status(code: MediusStatus, value: impl FnOnce() -> Value) -> Outcome {
    if code == MediusStatus::Ok {
        Ok(value())
    } else {
        Err(last_error(code))
    }
}

pub(super) fn last_error(code: MediusStatus) -> Failure {
    let mut buffer = vec![0 as c_char; 512];
    let full = unsafe { medius_last_error_message(buffer.as_mut_ptr(), buffer.len()) };
    if full >= buffer.len() {
        buffer = vec![0 as c_char; full + 1];
        unsafe { medius_last_error_message(buffer.as_mut_ptr(), buffer.len()) };
    }
    let mut error = Map::new();
    error.insert("status".into(), enum_name("Status", code as i64));
    error.insert("text".into(), text(&buffer));
    error.insert(
        "proto_ver".into(),
        Value::from(unsafe { medius_last_error_proto_ver() }),
    );
    let mut update = MediusUpdateError {
        op: 0,
        status: 0,
        arg: 0,
    };
    if unsafe { medius_last_error_update(&mut update) } {
        error.insert(
            "update".into(),
            json!({"op": update.op, "status": update.status, "arg": update.arg}),
        );
    }
    Failure::Status(Value::Object(error))
}

/// the frames the mock got since `seen`, without the keepalive's: its stats and clip queries, and a catch, rewrite or
/// transform frame it sends again. that's weaker than an exact list, since a call sending the same frame twice shows
/// once, but medius-capi gives the mock no way to stop the keepalive. once the box restarts, medius recovers on its own
/// thread and when those frames land depends on timing, so nothing after the restart counts
pub(super) fn frames(
    mock: *mut MediusMockBox,
    seen: &mut usize,
    sent: &mut Vec<Value>,
    restarted: &mut bool,
) -> Value {
    let total = unsafe { medius_mock_recorded(mock) };
    // clear_recorded empties the list, so the count starts over
    if total < *seen {
        *seen = 0;
    }
    let mut out = Vec::new();
    for index in *seen..total {
        if *restarted {
            break;
        }
        let mut ty = MediusFrameType::Move;
        let mut seq = 0u8;
        let mut payload = vec![0u8; 600];
        let length = unsafe {
            medius_mock_recorded_frame(
                mock,
                index,
                &mut ty,
                &mut seq,
                payload.as_mut_ptr(),
                payload.len(),
            )
        };
        let frame = json!({"type": enum_name("FrameType", ty as i64), "payload": hex(&payload[..length.min(payload.len())])});
        let keepalive = match ty {
            MediusFrameType::Query => length == 1 && matches!(payload[0], 5 | 10),
            MediusFrameType::Catch | MediusFrameType::Rewrite | MediusFrameType::Transform => {
                sent.contains(&frame)
            }
            _ => false,
        };
        if !keepalive {
            sent.push(frame.clone());
            out.push(frame);
        }
        // the chip boots on a RESET with the NVS flag (0x01) and on a reboot into the device chip's firmware
        *restarted = length > 0
            && match ty {
                MediusFrameType::Reset => payload[0] & 0x01 != 0,
                MediusFrameType::RebootDl => payload[0] == MediusRebootTarget::DeviceRun as u8,
                _ => false,
            };
    }
    *seen = total;
    Value::Array(out)
}

/// the calls the ledger holds on hardware, by their C++ name. a mock device's reconnect still rescans the machine's
/// serial ports and takes the first box that opens, so none of them runs on the mock
pub(super) fn on_hardware() -> &'static std::collections::BTreeSet<String> {
    static CALLS: std::sync::OnceLock<std::collections::BTreeSet<String>> =
        std::sync::OnceLock::new();
    CALLS.get_or_init(|| {
        crate::surface::ledger()
            .into_iter()
            .filter(|row| row.fields[0] == "hardware")
            .map(|row| row.fields[2].clone())
            .collect()
    })
}

fn outcome(result: Outcome) -> Value {
    match result {
        Ok(value) => json!({"value": value}),
        Err(Failure::Refused(_)) => json!({"refused": true}),
        Err(Failure::Status(error)) => json!({"error": error}),
    }
}

/// one step, a call on a handle, a helper, a text or a default
fn step(ctx: &mut Context, step: &Value) -> Outcome {
    let args = step["args"].as_array().cloned().unwrap_or_default();
    if let Some(name) = step["helper"].as_str() {
        return helpers::helper(name, &args);
    }
    if let Some(name) = step["text"].as_str() {
        return helpers::text(name, &step["value"]);
    }
    if let Some(name) = step["default"].as_str() {
        return helpers::default(name);
    }
    let class = step["on"].as_str().unwrap_or("Device");
    let method = step["call"]
        .as_str()
        .unwrap_or_else(|| panic!("a step needs a call, a helper, a text or a default: {step}"));
    assert!(
        !on_hardware().contains(&format!("{class}::{method}")),
        "{class}::{method} only runs on a real box, on the mock it would reach whatever box is plugged in"
    );
    if let Some(result) = generated::call(ctx, class, method, &args) {
        return result;
    }
    hand::call_by_hand(ctx, class, method, &args)
        .unwrap_or_else(|| panic!("the suite has no {class}::{method}"))
}

/// a scenario's mock setup, its device, and every step with the frames the box got during it
fn run(scenario: &Value) -> Value {
    let mut ctx = Context {
        mock: medius_mock_new(),
        device: std::ptr::null_mut(),
        clip: std::ptr::null_mut(),
        builder: medius_clip_builder_new(),
        frame: medius_clip_frame_new(),
        timeline: medius_timeline_new(),
        events: std::ptr::null_mut(),
        inputs: std::ptr::null_mut(),
        logs: std::ptr::null_mut(),
    };
    for setup in scenario["setup"].as_array().into_iter().flatten() {
        let mock = json!({"on": "MockBox", "call": setup["call"], "args": setup["args"]});
        match step(&mut ctx, &mock) {
            Ok(_) => {}
            Err(Failure::Refused(why)) => {
                panic!("the setup of {} was refused: {why}", scenario["name"])
            }
            Err(Failure::Status(error)) => {
                panic!("the setup of {} failed: {error}", scenario["name"])
            }
        }
    }
    let code = if scenario["bare"].as_bool() == Some(true) {
        unsafe { medius_device_with_mock(ctx.mock, &mut ctx.device) }
    } else {
        unsafe { medius_device_open_mock(ctx.mock, &mut ctx.device) }
    };
    let mut seen = 0;
    let mut sent = Vec::new();
    let mut restarted = false;
    let opened = outcome(status(code, || Value::Null));
    let mut out = scenario.as_object().unwrap().clone();
    out.insert(
        "open".into(),
        json!({"outcome": opened, "frames": frames(ctx.mock, &mut seen, &mut sent, &mut restarted)}),
    );
    let steps: Vec<Value> = scenario["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|each| {
            assert!(
                !restarted,
                "{} goes on after the box restarts, a restart has to be its last step",
                scenario["name"]
            );
            let result = outcome(step(&mut ctx, each));
            // the mock's own restart boots the chip without a frame
            restarted = each["on"] == "MockBox" && each["call"] == "restart";
            json!({"outcome": result, "frames": frames(ctx.mock, &mut seen, &mut sent, &mut restarted)})
        })
        .collect();
    out.insert("steps".into(), Value::Array(steps));
    Value::Object(out)
}
