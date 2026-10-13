use std::sync::Arc;

use serde_json::Value;

use super::{KEYBOARD, Shared, lock};
use crate::backend::{Input, Sink};
use crate::capability::{Action, Capability};
use crate::event::DropSource;
use crate::transport::codec::{self, Codec, Frame, Incoming};

const START: &[u8] = b"<START>";
const END: &[u8] = b"<END>";
const PROMPT: &[u8] = b">>>";
pub(super) const MAX_PAYLOAD: usize = 4096;
const MAX_FRAME: usize = MAX_PAYLOAD + 14;
const MAX_TEXT: usize = 1024;

pub(super) fn json(payload: &str) -> Option<Vec<u8>> {
    if payload.is_empty() || payload.len() > MAX_PAYLOAD {
        return None;
    }
    let mut bytes = START.to_vec();
    bytes.extend_from_slice(&u16::try_from(payload.len() + 14).ok()?.to_be_bytes());
    bytes.extend_from_slice(payload.as_bytes());
    bytes.extend_from_slice(END);
    Some(bytes)
}

pub(super) fn object(frame: &[u8]) -> Option<Value> {
    let length = usize::from(u16::from_be_bytes([*frame.get(7)?, *frame.get(8)?]));
    if !frame.starts_with(START)
        || !(15..=MAX_FRAME).contains(&length)
        || frame.len() != length
        || !frame.ends_with(END)
    {
        return None;
    }
    let value: Value = serde_json::from_slice(&frame[9..length - 5]).ok()?;
    value.is_object().then_some(value)
}

fn left(version: &str) -> bool {
    version
        .strip_prefix("ysc-towmcu-L")
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
}

fn conflicting(value: &Value) -> bool {
    match value {
        Value::String(text) => text.contains("ysc-towmcu-R") || text.contains("IAP"),
        Value::Array(values) => values.iter().any(conflicting),
        Value::Object(values) => values.values().any(conflicting),
        _ => false,
    }
}

pub(super) fn version(frame: &[u8]) -> Option<String> {
    let value = object(frame)?;
    if value.get("code")?.as_u64()? != 200 || conflicting(&value) {
        return None;
    }
    let message = value.get("message")?.as_str()?;
    let version = if left(message) {
        message
    } else if message == "version" {
        value.get("data")?.as_str()?
    } else {
        return None;
    };
    left(version).then(|| version.to_owned())
}

/// the V1 app's cmd 132 answer, `{"code":200,"message":"version","data":"<build date>"}`. a V2 or bootloader answer
/// isn't one
pub(super) fn v1_version(frame: &[u8]) -> Option<String> {
    let value = object(frame)?;
    if value.get("code")?.as_u64()? != 200
        || conflicting(&value)
        || value.get("message")?.as_str()? != "version"
    {
        return None;
    }
    let date = value.get("data")?.as_str()?;
    (!date.is_empty() && !date.starts_with("ysc-towmcu")).then(|| date.to_owned())
}

/// the bootloader answers cmd 132 with `YSC-IAP` on V1 and V2
pub(super) fn bootloader(frame: &[u8]) -> bool {
    object(frame).is_some_and(|value| {
        value.get("code").and_then(Value::as_u64) == Some(200) && names_iap(&value)
    })
}

fn names_iap(value: &Value) -> bool {
    match value {
        Value::String(text) => text.contains("-IAP"),
        Value::Array(values) => values.iter().any(names_iap),
        Value::Object(values) => values.values().any(names_iap),
        _ => false,
    }
}

fn data(value: &Value) -> Option<Value> {
    match value.get("data")? {
        Value::Object(_) => Some(value["data"].clone()),
        Value::String(text) if text.len() <= MAX_PAYLOAD => {
            let nested: Value = serde_json::from_str(text).ok()?;
            nested.is_object().then_some(nested)
        }
        _ => None,
    }
}

pub(super) fn debug(frame: &[u8]) -> Option<bool> {
    let value = object(frame)?;
    if value.get("code")?.as_u64()? != 702 {
        return None;
    }
    data(&value)?.get("debug")?.as_bool()
}

/// every reply code the vendor documents or its app reads, and anything with a `cmd`, which is a command going to
/// the box. an upload is never one of these even when it carries x and y
fn known_reply(value: &Value) -> bool {
    value.get("cmd").is_some()
        || value
            .get("code")
            .and_then(Value::as_u64)
            .is_some_and(|code| matches!(code, 200 | 404 | 100..=104 | 700..=715))
}

pub(super) fn key(command: &[u8]) -> codec::Key {
    // a collision only matters between commands in flight, so the requests are serialized
    command.iter().fold(0x811c_9dc5, |hash: u32, &byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    })
}

#[derive(Default)]
pub(super) struct Seen {
    line: Vec<u8>,
    reply: Vec<u8>,
    prompt_space: bool,
    pub(super) buttons: Option<u8>,
}

pub(super) struct YscFrames {
    shared: Arc<Shared>,
    /// none while probing a bare port, where nobody listens yet. it only queues, so the reader never waits on
    /// the core, and the reader is also what reports Disconnected, so these always land before it
    sink: Option<Sink>,
}

impl YscFrames {
    pub(super) fn new(shared: Arc<Shared>, sink: Option<Sink>) -> Self {
        Self { shared, sink }
    }

    pub(super) fn probe() -> Self {
        Self::new(
            Arc::new(Shared::new(super::snapshot(
                String::new(),
                super::Variant::V2,
            ))),
            None,
        )
    }

    /// malformed input proves at least one report got lost on the PC side, not how many the box sent
    fn lost(&self) {
        if let Some(sink) = &self.sink {
            sink.dropped(1, DropSource::Host);
        }
    }

    fn upload(&self, value: &Value) -> Option<Vec<Input>> {
        let value = value.get("data").unwrap_or(value);
        if !value.is_object() {
            return None;
        }
        let signed = |name| i32::try_from(value.get(name)?.as_i64()?).ok();
        let dx = signed("x")?;
        let dy = signed("y")?;
        let wheel = if value.get("wheel").is_some() {
            signed("wheel")?
        } else {
            0
        };
        let buttons = match value.get("b") {
            Some(value) => Some(u8::try_from(value.as_u64()?).ok()? & 0x1f),
            None => None,
        };
        let mut inputs = Vec::new();
        if let Some(buttons) = buttons {
            let mut state = lock(&self.shared.state);
            if let Some(previous) = state.seen.buttons {
                let changed = previous ^ buttons;
                for pressed in [false, true] {
                    for (bit, &button) in super::BUTTONS.iter().enumerate() {
                        if changed & (1 << bit) != 0 && (buttons & (1 << bit) != 0) == pressed {
                            inputs.push(Input::Button { button, pressed });
                        }
                    }
                }
            }
            state.seen.buttons = Some(buttons);
        }
        inputs.push(Input::Motion {
            dx,
            dy,
            wheel,
            pan: 0,
        });
        Some(inputs)
    }
}

impl Codec for YscFrames {
    fn split(&mut self, buffer: &mut Vec<u8>) -> Option<Frame> {
        while !buffer.is_empty() {
            if buffer.starts_with(START) {
                if buffer.len() < 9 {
                    return None;
                }
                let length = usize::from(u16::from_be_bytes([buffer[7], buffer[8]]));
                if !(15..=MAX_FRAME).contains(&length) {
                    buffer.remove(0);
                    self.lost();
                    continue;
                }
                if buffer.len() < length {
                    return None;
                }
                if &buffer[length - 5..length] != END {
                    buffer.remove(0);
                    self.lost();
                    continue;
                }
                return Some(buffer.drain(..length).collect());
            }
            if buffer.starts_with(PROMPT) {
                buffer.drain(..PROMPT.len());
                if buffer.first() == Some(&b' ') {
                    buffer.remove(0);
                }
                let mut state = lock(&self.shared.state);
                state.seen.prompt_space = buffer.is_empty();
                state.seen.line.clear();
                let mut reply = std::mem::take(&mut state.seen.reply);
                reply.extend_from_slice(PROMPT);
                return Some(reply);
            }
            if START.starts_with(buffer) || PROMPT.starts_with(buffer) {
                return None;
            }
            let byte = buffer.remove(0);
            let mut state = lock(&self.shared.state);
            if std::mem::take(&mut state.seen.prompt_space) && byte == b' ' {
                continue;
            }
            state.seen.line.push(byte);
            if state.seen.line.ends_with(b"\r\n") {
                let line = std::mem::take(&mut state.seen.line);
                if line.starts_with(b"km.") && line.ends_with(b")\r\n") {
                    state.seen.reply = line;
                } else if !state.seen.reply.is_empty() {
                    state.seen.reply.extend(line);
                }
            }
            if state.seen.line.len() + state.seen.reply.len() > MAX_TEXT {
                state.seen.line.clear();
                state.seen.reply.clear();
                drop(state);
                self.lost();
            }
        }
        None
    }

    fn classify(&mut self, frame: &Frame) -> Incoming {
        if frame.starts_with(START) {
            let Some(value) = object(frame) else {
                self.lost();
                return Incoming::Noise;
            };
            if value.get("code").and_then(Value::as_u64) == Some(404)
                && value.get("message").and_then(Value::as_str) == Some("no_keyboard")
            {
                let mut state = lock(&self.shared.state);
                let first = std::mem::replace(&mut state.keys, false);
                if first {
                    for action in [Action::KeyPress, Action::KeyRelease, Action::KeyTap] {
                        state.snapshot.capabilities = state
                            .snapshot
                            .capabilities
                            .clone()
                            .set(action, Capability::unsupported(KEYBOARD));
                    }
                }
                // the refusal doesn't name the key or the direction, so it only gets blamed on one kind of
                // command when that's the only kind this connection sent
                let action = match (state.pressed_keys, state.released_keys) {
                    (true, false) => Some(Action::KeyPress),
                    (false, true) => Some(Action::KeyRelease),
                    _ => None,
                }
                .filter(|_| !state.refused);
                state.refused |= action.is_some();
                let snapshot = first.then(|| state.snapshot.clone());
                drop(state);
                if let Some(sink) = &self.sink {
                    if let Some(snapshot) = snapshot {
                        sink.changed(snapshot);
                    }
                    if let Some(action) = action {
                        sink.refused(action);
                    }
                }
                return Incoming::Noise;
            }
            if known_reply(&value) {
                return Incoming::Noise;
            }
            // the vendor's own parser looks for x and y before anything else, flat or under data, so an upload
            // wrapped in an envelope still counts, and anything else nobody can read is a lost report
            return match self.upload(&value) {
                Some(inputs) => Incoming::Inputs(inputs),
                None => {
                    self.lost();
                    Incoming::Noise
                }
            };
        }
        if frame.starts_with(b"km.")
            && frame.ends_with(PROMPT)
            && let Some(end) = frame.windows(2).position(|bytes| bytes == b"\r\n")
            && frame[..end].ends_with(b")")
            && std::str::from_utf8(frame).is_ok()
        {
            Incoming::Reply(key(&frame[..end]))
        } else {
            Incoming::Noise
        }
    }
}

#[cfg(test)]
mod tests;
