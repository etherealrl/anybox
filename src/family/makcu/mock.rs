//! the fake MAKCU the crate tests drive, in the shape the vendor documents it, and `Device::mock_makcu` over it
#![cfg_attr(not(test), allow(dead_code))]

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::frames::{self, frame};
use crate::key::Key;
use crate::transport::PortUsb;

pub(crate) const SERIAL: &str = "SERIAL0001";

/// what the fake box knows, in the shape the vendor documents it. a test can change it while the box runs
pub(crate) struct State {
    pub(crate) baud: u32,
    /// what `km.version()` answers, none for a box that doesn't speak km text
    pub(crate) text: Option<&'static [u8]>,
    /// what FIRMWARE_VERSION answers, none for V3 because it ignores binary frames
    pub(crate) build: Option<u32>,
    pub(crate) kinds: u8,
    pub(crate) stream: u8,
    pub(crate) key_stream: u8,
    pub(crate) injected: u8,
    /// opcodes the box refuses with `FF`
    pub(crate) refuse: HashSet<u8>,
    /// how many identity queries the box ignores before it answers
    pub(crate) silent_versions: usize,
    pub(crate) silent_versions_left: usize,
    /// how many firmware queries the box ignores before it answers
    pub(crate) silent_builds_left: usize,
    /// a malformed payload the firmware reply sends instead of the build
    pub(crate) build_reply: Option<Vec<u8>>,
    /// opcodes the box refuses once and then takes
    pub(crate) refuse_once: HashSet<u8>,
    /// false makes presses wait forever, like they're still queued behind moves
    pub(crate) apply_presses: bool,
    /// the injected keys the box is holding, by usage
    pub(crate) keys: HashSet<u8>,
    /// which LOCK targets are on, bit n for target n
    pub(crate) locks: u16,
    pub(crate) masks: HashSet<u8>,
    /// how long the box takes to answer each of these opcodes
    pub(crate) slow: HashMap<u8, Duration>,
    /// how long the box takes to type one character
    pub(crate) per_character: Duration,
    /// when the box is done with the text it's typing. until then it refuses every key SET, like the real one
    pub(crate) typing_until: Option<Instant>,
    /// everything the box typed
    pub(crate) typed: Vec<u8>,
    /// a box that takes a text without typing it
    pub(crate) ignores_text: bool,
    /// a box that never gets done typing
    pub(crate) types_forever: bool,
    pub(crate) chords: Vec<(u8, Vec<u8>)>,
    /// INTERPOLATE's spread, 255 is AUTO, and what INJECT_SNAPSHOT shows at byte 36
    pub(crate) spread: u8,
    /// a box that echoes an interpolation it doesn't take
    pub(crate) ignores_spread: bool,
    pub(crate) moving: u8,
    pub(crate) autorelease: u32,
}

impl State {
    pub(crate) fn makcu(build: u32) -> Self {
        Self {
            baud: 4_000_000,
            text: Some(b"km.MAKCU\r\n>>> "),
            build: Some(build),
            kinds: 0x03,
            stream: 0,
            key_stream: 0,
            injected: 0,
            refuse: HashSet::new(),
            silent_versions: 0,
            silent_versions_left: 0,
            silent_builds_left: 0,
            build_reply: None,
            refuse_once: HashSet::new(),
            apply_presses: true,
            keys: HashSet::new(),
            locks: 0,
            masks: HashSet::new(),
            slow: HashMap::new(),
            per_character: Duration::from_millis(30),
            typing_until: None,
            typed: Vec::new(),
            ignores_text: false,
            types_forever: false,
            chords: Vec::new(),
            spread: 0xFF,
            ignores_spread: false,
            moving: 0,
            autorelease: 0,
        }
    }
}

pub(crate) type Shared = Arc<Mutex<State>>;

pub(crate) fn respond(shared: &Shared, baud: u32, bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut state = shared.lock().unwrap();
    if baud != state.baud {
        return Vec::new();
    }
    if bytes == frames::VERSION_QUERY {
        if state.silent_versions_left > 0 {
            state.silent_versions_left -= 1;
            return Vec::new();
        }
        return state
            .text
            .map(|text| vec![text.to_vec()])
            .unwrap_or_default();
    }
    let Some((command, payload)) = frames::parts(bytes) else {
        return Vec::new();
    };
    if let Some(&delay) = state.slow.get(&command) {
        drop(state);
        thread::sleep(delay);
        state = shared.lock().unwrap();
    }
    if state.refuse.contains(&command) || state.refuse_once.remove(&command) {
        return vec![frame(command, &[0xFF])];
    }
    let typing = state.types_forever
        || state
            .typing_until
            .is_some_and(|until| Instant::now() < until);
    if typing
        && matches!(
            command,
            frames::KEY_STRING
                | frames::KEY_DOWN
                | frames::KEY_UP
                | frames::KEY_PRESS
                | frames::KEY_MASK
                | frames::KEY_MULTI_DOWN..=frames::KEY_MULTI_PRESS
        )
    {
        return vec![frame(command, &[0xFF])];
    }
    match (command, payload) {
        // measured on V4.108: a u32 of ms up to a minute, echoed, more is refused
        (frames::AUTORELEASE, []) => vec![frame(command, &state.autorelease.to_le_bytes())],
        (frames::AUTORELEASE, &[a, b, c, d]) if u32::from_le_bytes([a, b, c, d]) <= 60_000 => {
            state.autorelease = u32::from_le_bytes([a, b, c, d]);
            vec![frame(command, payload)]
        }
        (frames::AUTORELEASE, _) => vec![frame(command, &[0xFF])],
        (frames::INTERPOLATE, &[n]) if n <= 100 || n == 0xFF => {
            if !state.ignores_spread {
                state.spread = n;
            }
            vec![frame(command, &[n])]
        }
        (frames::INTERPOLATE, _) => vec![frame(command, &[0xFF])],
        (frames::MOVING, []) => vec![frame(command, &[state.moving])],
        (frames::KEY_STRING, []) => Vec::new(),
        (frames::KEY_STRING, _) if state.ignores_text => Vec::new(),
        (frames::KEY_STRING, text) => {
            state.typed.extend_from_slice(text);
            let per_character = state.per_character;
            state.typing_until =
                Some(Instant::now() + per_character * u32::try_from(text.len()).unwrap());
            // measured: typing lets go of a held key it types and of Left Shift when it needs Shift
            for &byte in text {
                if let Some((key, shift)) = Key::for_ascii(byte) {
                    state.keys.remove(&key.0);
                    if shift {
                        state.keys.remove(&Key::LEFT_SHIFT.0);
                    }
                }
            }
            Vec::new()
        }
        (frames::KEY_MULTI_DOWN..=frames::KEY_MULTI_PRESS, keys) => {
            match command {
                frames::KEY_MULTI_DOWN => state.keys.extend(keys.iter().copied()),
                frames::KEY_MULTI_UP => {
                    for key in keys {
                        state.keys.remove(key);
                    }
                }
                _ => {}
            }
            state.chords.push((command, keys.to_vec()));
            Vec::new()
        }
        (frames::FIRMWARE_VERSION, []) if state.silent_builds_left > 0 => {
            state.silent_builds_left -= 1;
            Vec::new()
        }
        (frames::FIRMWARE_VERSION, []) if state.build_reply.is_some() => {
            vec![frame(command, state.build_reply.as_deref().unwrap())]
        }
        (frames::FIRMWARE_VERSION, []) => state
            .build
            .map(|build| vec![frame(command, &build.to_le_bytes())])
            .unwrap_or_default(),
        (frames::DEVICE, []) => vec![frame(command, &[state.kinds])],
        (frames::INPUT_STREAM, [1, on]) => {
            state.stream = *on;
            Vec::new()
        }
        (frames::INPUT_STREAM, [1]) => vec![frame(command, &[state.stream])],
        (frames::INPUT_STREAM, [2, on]) => {
            state.key_stream = *on;
            Vec::new()
        }
        (frames::INPUT_STREAM, [2]) => vec![frame(command, &[state.key_stream])],
        (frames::INJECT_SNAPSHOT, []) => {
            let mut snapshot = vec![0u8; 40];
            snapshot[0] = state.injected;
            snapshot[1..3].copy_from_slice(&state.locks.to_le_bytes());
            snapshot[frames::SPREAD] = state.spread;
            // offset 3 holds the modifiers and the key bitmap starts at offset 4. that's worked out from the vendor's
            // field order and was checked on a V4.094 box
            let mut keys = state.keys.clone();
            // measured: the snapshot shows the key it's typing while that key is down
            if typing
                && let Some((key, _)) = state.typed.last().and_then(|&byte| Key::for_ascii(byte))
            {
                keys.insert(key.0);
            }
            for &key in &keys {
                if (0xE0..=0xE7).contains(&key) {
                    snapshot[3] |= 1 << (key - 0xE0);
                } else {
                    snapshot[4 + usize::from(key / 8)] |= 1 << (key % 8);
                }
            }
            vec![frame(command, &snapshot)]
        }
        // a click that's still running shows as held. a physical release can cut one short, which shows the
        // box tracks it
        (frames::CLICK, [button, ..]) => {
            state.injected |= 1 << (button - 1);
            Vec::new()
        }
        (frames::LOCK, [target, on]) => {
            if *on == 1 {
                state.locks |= 1 << target;
            } else {
                state.locks &= !(1 << target);
            }
            Vec::new()
        }
        (frames::KEY_MASK, [key, on]) => {
            if *on == 1 {
                state.masks.insert(*key);
            } else {
                state.masks.remove(key);
            }
            Vec::new()
        }
        (frames::KEY_DOWN, [key]) => {
            state.keys.insert(*key);
            Vec::new()
        }
        (frames::KEY_UP, [key]) => {
            state.keys.remove(key);
            Vec::new()
        }
        (0x11..=0x15, [1]) if !state.apply_presses => Vec::new(),
        (0x11..=0x15, [on]) => {
            let bit = 1 << (command - 0x11);
            if *on == 1 {
                state.injected |= bit;
            } else {
                state.injected &= !bit;
            }
            Vec::new()
        }
        _ => Vec::new(),
    }
}

pub(crate) fn makcu_usb() -> Option<PortUsb> {
    Some(PortUsb {
        vid: 0x1A86,
        pid: 0x55D3,
        serial: Some(SERIAL.into()),
    })
}

#[cfg(feature = "mock")]
impl crate::device::Device {
    /// a device over anybox's own model of a MAKCU V4.108 with a mouse and a keyboard behind it, for testing your
    /// code without a box. it runs the exact code a real MAKCU gets over a fake port, so it proves your code works
    /// and nothing about the hardware
    pub fn mock_makcu(click_timing: crate::click::ClickTiming) -> crate::error::Result<Self> {
        use std::sync::mpsc;

        use crate::backend::Sink;
        use crate::device::Start;
        use crate::transport::Candidate;
        use crate::transport::fake::{FakeBox, FakeSystem};

        const PORT: &str = "mock MAKCU";
        let state = Arc::new(Mutex::new(State::makcu(4108)));
        let fake = FakeBox::by_baud(move |baud, bytes| respond(&state, baud, bytes));
        let system = FakeSystem::default();
        system.plug(PORT, makcu_usb(), fake);
        let candidate = Candidate {
            path: PORT.into(),
            usb: makcu_usb(),
        };
        let (signals, incoming) = mpsc::channel();
        let (backend, snapshot, poller) =
            super::attach(Arc::new(system), &candidate, Sink(signals.clone()))?;
        *super::lock(&backend.poller) = Some(poller.spawn()?);
        Self::start(Start {
            backend: Box::new(backend),
            snapshot,
            signals,
            incoming,
            click_timing,
            holds: crate::click::Holds::from_clock(),
            port: PORT.into(),
        })
    }
}
