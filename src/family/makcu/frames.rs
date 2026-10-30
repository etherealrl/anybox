use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};

use crate::backend::{Input, InputKind, LockSlot, Sign};
use crate::capability::Axis;
use crate::capability::{Action, Button, MakcuAction};
use crate::key::Key;
use crate::transport::codec::{Codec, Frame, Incoming};

/// a MAK_API frame is `DE AD | LEN:u16 LE | CMD | PAYLOAD[LEN]` and LEN only counts the payload. a SET that
/// works gets no reply, and a refusal is `DE AD 01 00 <CMD> FF`
const START: [u8; 2] = [0xDE, 0xAD];
const HEADER: usize = 5;
/// the longest frame this backend reads is INJECT_SNAPSHOT's 40 bytes, so a LEN past this means the header is
/// corrupt
const MAX_LEN: usize = 64;
const PROMPT: &[u8] = b">>> ";
const REFUSED: u8 = 0xFF;

pub(crate) const DEVICE: u8 = 0x02;
pub(crate) const FIRMWARE_VERSION: u8 = 0x04;
pub(crate) const KEY_DOWN: u8 = 0x20;
pub(crate) const KEY_UP: u8 = 0x21;
pub(crate) const KEY_PRESS: u8 = 0x23;
pub(crate) const KEY_STRING: u8 = 0x24;
pub(crate) const KEY_MULTI_DOWN: u8 = 0x26;
pub(crate) const KEY_MULTI_UP: u8 = 0x27;
pub(crate) const KEY_MULTI_PRESS: u8 = 0x28;
pub(crate) const KEY_MASK: u8 = 0x29;
pub(crate) const LEFT: u8 = 0x11;
pub(crate) const MOVE: u8 = 0x18;
pub(crate) const INTERPOLATE: u8 = 0x1F;
pub(crate) const WHEEL: u8 = 0x19;
pub(crate) const INPUT_STREAM: u8 = 0x52;
const INPUT_CHANGE: u8 = 0x53;
pub(crate) const INJECT_SNAPSHOT: u8 = 0x56;
pub(crate) const LOCK: u8 = 0x60;
pub(crate) const CLICK: u8 = 0x61;
pub(crate) const MOVING: u8 = 0x66;
pub(crate) const MOVE_NOW: u8 = 0x67;
pub(crate) const AUTORELEASE: u8 = 0x6E;
/// INJECT_SNAPSHOT's spread byte, 255 is AUTO (checked on V4.094)
pub(crate) const SPREAD: usize = 36;

/// the mouse's kind in INPUT_CHANGE and INPUT_STREAM
pub(crate) const MOUSE: u8 = 1;
/// the keyboard's kind in the same two
pub(crate) const KEYBOARD: u8 = 2;
/// the bit for a routed mouse in DEVICE's kinds
pub(crate) const MOUSE_ROUTED: u8 = 0x01;
/// set when the device behind the box has a keyboard interface, so a mouse with one sets it too (checked on
/// hardware)
pub(crate) const KEYBOARD_ROUTED: u8 = 0x02;

pub(crate) const VERSION_QUERY: &[u8] = b"\r\nkm.version()\r\n";

pub(crate) fn frame(command: u8, payload: &[u8]) -> Vec<u8> {
    let len = u16::try_from(payload.len()).expect("MAK_API payloads are short");
    let mut frame = START.to_vec();
    frame.extend_from_slice(&len.to_le_bytes());
    frame.push(command);
    frame.extend_from_slice(payload);
    frame
}

/// the command byte and payload of a binary frame, or none for a text frame
pub(crate) fn parts(frame: &[u8]) -> Option<(u8, &[u8])> {
    if frame.starts_with(&START) && frame.len() >= HEADER {
        Some((frame[4], &frame[HEADER..]))
    } else {
        None
    }
}

pub(crate) fn refused(payload: &[u8]) -> bool {
    payload == [REFUSED]
}

/// the reply to `km.version()` has a line that is exactly `km.MAKCU`. a YSC answers `km.version(km.MAKCU)`,
/// which a plain substring check would take for a MAKCU
pub(crate) fn says_makcu(frame: &[u8]) -> bool {
    parts(frame).is_none()
        && frame
            .split(|&byte| byte == b'\r' || byte == b'\n')
            .any(|line| line == b"km.MAKCU")
}

pub(crate) fn button_id(button: Button) -> Option<u8> {
    match button {
        Button::Left => Some(0),
        Button::Right => Some(1),
        Button::Middle => Some(2),
        Button::Side1 => Some(3),
        Button::Side2 => Some(4),
        _ => None,
    }
}

/// the state last written for each of LEFT..SIDE2 and for each key. a refusal doesn't echo it, so this is what
/// tells a refused press from a refused release, and the poller uses it to resend a release the box didn't take
pub(crate) struct LastSent {
    buttons: [AtomicU8; 5],
    keys: [AtomicBool; 256],
    /// the INPUT_STREAM kind switched last
    stream: AtomicU8,
    /// per LOCK target 0..13, 0 when it was never written, 1 when a lock was written last and 2 when an unlock was
    locks: [AtomicU8; 14],
    /// how many times the app wrote each LOCK target, so the poller can tell a new write from its own retry
    lock_writes: [AtomicU32; 14],
    /// the LOCK target written last, because a refusal only says LOCK and this tells a button from an axis
    lock_target: AtomicU8,
    /// whether the KEY_MASK written last was a force and not a lock, because a refusal only says KEY_MASK
    mask_forced: AtomicBool,
}

impl Default for LastSent {
    fn default() -> Self {
        Self {
            buttons: Default::default(),
            keys: std::array::from_fn(|_| AtomicBool::new(false)),
            stream: AtomicU8::new(MOUSE),
            locks: Default::default(),
            lock_writes: Default::default(),
            lock_target: AtomicU8::new(0),
            mask_forced: AtomicBool::new(false),
        }
    }
}

impl LastSent {
    pub(crate) fn set(&self, id: u8, state: u8) {
        self.buttons[usize::from(id)].store(state, Ordering::Release);
    }

    pub(crate) fn get(&self, id: u8) -> u8 {
        self.buttons[usize::from(id)].load(Ordering::Acquire)
    }

    pub(crate) fn set_key(&self, key: u8, pressed: bool) {
        self.keys[usize::from(key)].store(pressed, Ordering::Release);
    }

    pub(crate) fn key(&self, key: u8) -> bool {
        self.keys[usize::from(key)].load(Ordering::Acquire)
    }

    pub(crate) fn set_stream(&self, kind: u8) {
        self.stream.store(kind, Ordering::Release);
    }

    pub(crate) const NEVER: u8 = 0;
    pub(crate) const LOCKED: u8 = 1;
    pub(crate) const UNLOCKED: u8 = 2;

    pub(crate) fn set_lock(&self, target: u8, state: u8) {
        self.locks[usize::from(target)].store(state, Ordering::Release);
        self.lock_writes[usize::from(target)].fetch_add(1, Ordering::AcqRel);
        self.name_lock(target);
    }

    /// whether a refusal of the next KEY_MASK gets blamed on a force or on a lock
    pub(crate) fn name_mask(&self, forced: bool) {
        self.mask_forced.store(forced, Ordering::Release);
    }

    /// the target that a refusal of the next LOCK gets blamed on
    pub(crate) fn name_lock(&self, target: u8) {
        self.lock_target.store(target, Ordering::Release);
    }

    pub(crate) fn lock_writes(&self, target: u8) -> u32 {
        self.lock_writes[usize::from(target)].load(Ordering::Acquire)
    }

    pub(crate) fn lock(&self, target: u8) -> u8 {
        self.locks[usize::from(target)].load(Ordering::Acquire)
    }

    /// forgets every lock that was written, because after a reprobe or a reset the core sends the wanted ones again
    pub(crate) fn forget_locks(&self) {
        for lock in &self.locks {
            lock.store(Self::NEVER, Ordering::Release);
        }
    }
}

pub(crate) struct MakcuFrames {
    /// text frames only show up while probing, the link only reads binary
    text: bool,
    last: Arc<LastSent>,
}

impl MakcuFrames {
    pub(crate) fn probe() -> Self {
        Self {
            text: true,
            last: Arc::default(),
        }
    }

    pub(crate) fn binary(last: Arc<LastSent>) -> Self {
        Self { text: false, last }
    }

    /// which action a refusal of this opcode was for
    fn refused_action(&self, command: u8) -> Option<Action> {
        Some(match command {
            MOVE => Action::Move,
            MOVE_NOW => Action::MoveNow,
            WHEEL => Action::Wheel,
            CLICK => Action::Click,
            INPUT_STREAM if self.last.stream.load(Ordering::Acquire) == KEYBOARD => {
                Action::KeyEvents
            }
            INPUT_STREAM => Action::ButtonEvents,
            LOCK if self.last.lock_target.load(Ordering::Acquire) < 5 => Action::LockButton,
            LOCK => Action::LockAxis,
            KEY_MASK if self.last.mask_forced.load(Ordering::Acquire) => Action::ForceReleaseKey,
            KEY_MASK => Action::LockKey,
            KEY_DOWN => Action::KeyPress,
            KEY_UP => Action::KeyRelease,
            KEY_PRESS => Action::KeyTap,
            KEY_MULTI_DOWN..=KEY_MULTI_PRESS => Action::KeyChord,
            INTERPOLATE => Action::Makcu(MakcuAction::Interpolation),
            MOVING => Action::Makcu(MakcuAction::Moving),
            AUTORELEASE => Action::Makcu(MakcuAction::Autorelease),
            LEFT..=0x15 if self.last.get(command - LEFT) == 1 => Action::Press,
            LEFT..=0x15 => Action::Release,
            _ => return None,
        })
    }

    fn binary_frame(buffer: &mut Vec<u8>) -> Option<Frame> {
        loop {
            if buffer.len() < HEADER {
                return None;
            }
            let len = usize::from(u16::from_le_bytes([buffer[2], buffer[3]]));
            if len > MAX_LEN {
                // the header is corrupt, so the next real start is somewhere after this one
                buffer.drain(..1);
                drop_to_start(buffer);
                continue;
            }
            if buffer.len() < HEADER + len {
                return None;
            }
            return Some(buffer.drain(..HEADER + len).collect());
        }
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// drops everything before the next `DE AD`, but keeps a lone `DE` at the end because it may be the start of one
fn drop_to_start(buffer: &mut Vec<u8>) {
    match find(buffer, &START) {
        Some(start) => {
            buffer.drain(..start);
        }
        None => {
            let keep = usize::from(buffer.last() == Some(&START[0]));
            buffer.drain(..buffer.len() - keep);
        }
    }
}

impl Codec for MakcuFrames {
    fn split(&mut self, buffer: &mut Vec<u8>) -> Option<Frame> {
        if !self.text {
            drop_to_start(buffer);
            return Self::binary_frame(buffer);
        }
        let start = find(buffer, &START);
        if start == Some(0) {
            return Self::binary_frame(buffer);
        }
        let prompt = find(buffer, PROMPT).map(|at| at + PROMPT.len());
        // text ends at its prompt, or where a binary frame starts if that comes first
        let end = match (prompt, start) {
            (Some(prompt), Some(start)) => prompt.min(start),
            (Some(end), None) | (None, Some(end)) => end,
            (None, None) => return None,
        };
        Some(buffer.drain(..end).collect())
    }

    fn classify(&mut self, frame: &Frame) -> Incoming {
        let Some((command, payload)) = parts(frame) else {
            return Incoming::Noise;
        };
        if command == INPUT_CHANGE {
            return match *payload {
                [MOUSE, 0xFF, 0xFF] => Incoming::StreamLost(InputKind::Buttons),
                [KEYBOARD, 0xFF, 0xFF] => Incoming::StreamLost(InputKind::Keys),
                [KEYBOARD, usage, state @ 0..=1] if Key(usage).is_valid() => Incoming::Input {
                    input: Input::Key {
                        key: Key(usage),
                        pressed: state == 1,
                    },
                    box_time: None,
                },
                [MOUSE, id @ 0..=31, state @ 0..=1] => Incoming::Input {
                    input: Input::Button {
                        button: match id {
                            0 => Button::Left,
                            1 => Button::Right,
                            2 => Button::Middle,
                            3 => Button::Side1,
                            4 => Button::Side2,
                            n => Button::Extra(n),
                        },
                        pressed: state == 1,
                    },
                    box_time: None,
                },
                _ => Incoming::Noise,
            };
        }
        if refused(payload) {
            return Incoming::Refused {
                key: u32::from(command),
                action: self.refused_action(command),
            };
        }
        Incoming::Reply(u32::from(command))
    }
}

/// the keys INJECT_SNAPSHOT says the box holds. the modifier byte is at offset 3 (bit n is 0xE0 + n) and the key
/// bitmap at offset 4 (bit n is usage n), the offsets come from the documented field order and were checked on
/// hardware
pub(crate) fn injected_keys(snapshot: &[u8]) -> Vec<u8> {
    let mut keys = Vec::new();
    if let Some(&modifiers) = snapshot.get(3) {
        keys.extend(
            (0..8u8)
                .filter(|bit| modifiers & (1 << bit) != 0)
                .map(|bit| 0xE0 + bit),
        );
    }
    if let Some(bitmap) = snapshot.get(4..36) {
        for (byte, &bits) in bitmap.iter().enumerate() {
            for bit in 0..8u8 {
                let key = u8::try_from(byte * 8).unwrap_or(u8::MAX) + bit;
                if bits & (1 << bit) != 0 && !keys.contains(&key) {
                    keys.push(key);
                }
            }
        }
    }
    keys
}

/// an axis only uses its signed LOCK targets, because the ones for both directions (5, 8 and 11) are just the two
/// signed ones together (checked for X on hardware, Y and the wheel are assumed to work the same)
pub(crate) fn lock_target(slot: LockSlot) -> Option<u8> {
    Some(match slot {
        LockSlot::Button(button) => button_id(button)?,
        LockSlot::Axis(Axis::X, Sign::Positive) => 6,
        LockSlot::Axis(Axis::X, Sign::Negative) => 7,
        LockSlot::Axis(Axis::Y, Sign::Positive) => 9,
        LockSlot::Axis(Axis::Y, Sign::Negative) => 10,
        LockSlot::Axis(Axis::Wheel, Sign::Positive) => 12,
        LockSlot::Axis(Axis::Wheel, Sign::Negative) => 13,
        LockSlot::Axis(..) | LockSlot::Key(_) => return None,
    })
}

/// the LOCK targets INJECT_SNAPSHOT says are on. they're a u16 at offset 1 with bit n for target n
pub(crate) fn locked_targets(snapshot: &[u8]) -> u16 {
    match snapshot.get(1..3) {
        Some(&[low, high]) => u16::from_le_bytes([low, high]),
        _ => 0,
    }
}
