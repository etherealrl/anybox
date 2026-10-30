//! the fake Ferrum the crate tests drive, answering the way the vendor docs show it, and `Device::mock_ferrum` over it
#![cfg_attr(not(test), allow(dead_code))]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::frames::{LEGACY_VERSION, SOFTWARE_VERSION};
use crate::capability::FerrumApi;
use crate::transport::PortUsb;

/// what the fake Ferrum knows, answering the way the vendor docs show it. a test can change it while it runs
pub(crate) struct State {
    pub(crate) api: FerrumApi,
    /// the legacy port only answers at this rate, the virtual port at any
    pub(crate) baud: u32,
    pub(crate) mouse: bool,
    pub(crate) keyboard: bool,
    /// an app new enough to know `km.has_mouse()` and the others
    pub(crate) roles: bool,
    /// an app new enough to know the directional axis locks
    pub(crate) directional: bool,
    /// what `device.VID()` and `device.PID()` answer, none for an app that doesn't know them
    pub(crate) ids: Option<(&'static str, &'static str)>,
    /// an app that answers nothing at all to `device.` commands, against what the docs say
    pub(crate) ids_silent: bool,
    /// an app that answers nothing at all to `ferrum.` commands
    pub(crate) version_silent: bool,
    pub(crate) silent: bool,
    /// whether the legacy port puts a prompt after what it sends, the docs contradict themselves on it
    pub(crate) legacy_prompt: bool,
    /// what `ferrum.version()` answers, none for an app that doesn't know it
    pub(crate) version: Option<&'static str>,
    /// what `km.catch_xy` answers
    pub(crate) catch: &'static str,
    pub(crate) streams: HashMap<String, bool>,
    /// every command line it got, without the line end
    pub(crate) lines: Vec<String>,
}

impl State {
    pub(crate) fn software() -> Self {
        Self {
            api: FerrumApi::Software,
            baud: 0,
            mouse: true,
            keyboard: true,
            roles: true,
            directional: true,
            ids: Some(("046d", "c08b")),
            ids_silent: false,
            version_silent: false,
            silent: false,
            legacy_prompt: false,
            version: Some("v2.0.6"),
            catch: "(-3, 10)",
            streams: HashMap::new(),
            lines: Vec::new(),
        }
    }

    pub(crate) fn legacy(baud: u32) -> Self {
        Self {
            api: FerrumApi::Legacy,
            baud,
            keyboard: false,
            roles: false,
            directional: false,
            ..Self::software()
        }
    }

    /// the result line, none for a command with no result, and for one the box doesn't know
    fn answer(&mut self, line: &str) -> Option<String> {
        let (name, args) = line.split_once('(')?;
        let args = args.strip_suffix(')')?;
        let bit = |on: bool| if on { "1" } else { "0" }.to_string();
        match (name, args) {
            ("km.version", "") => Some(
                String::from_utf8_lossy(match self.api {
                    FerrumApi::Software => SOFTWARE_VERSION,
                    FerrumApi::Legacy => LEGACY_VERSION,
                })
                .into(),
            ),
            ("km.has_mouse", "") if self.roles => Some(bit(self.mouse)),
            ("km.has_keyboard", "") if self.roles => Some(bit(self.keyboard)),
            ("km.lock_mx+", "") if self.directional => Some("0".into()),
            ("ferrum.version", "") => self.version.map(String::from),
            ("km.catch_xy", _) => Some(self.catch.into()),
            ("device.VID", "") => self.ids.map(|(vid, _)| format!("VID={vid}")),
            ("device.PID", "") => self.ids.map(|(_, pid)| format!("PID={pid}")),
            ("km.buttons" | "km.axes" | "km.keys", "") => {
                Some(bit(self.streams.get(name).copied().unwrap_or(false)))
            }
            ("km.buttons" | "km.axes" | "km.keys", on) => {
                self.streams.insert(name.into(), on == "1");
                None
            }
            _ => None,
        }
    }
}

pub(crate) type Shared = Arc<Mutex<State>>;

pub(crate) fn respond(shared: &Shared, baud: u32, bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut state = shared.lock().unwrap();
    if state.silent || (state.api == FerrumApi::Legacy && baud != state.baud) {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(bytes).to_string();
    let mut out = Vec::new();
    for line in text.split(['\r', '\n']).filter(|line| !line.is_empty()) {
        state.lines.push(line.into());
        if (state.ids_silent && line.starts_with("device."))
            || (state.version_silent && line.starts_with("ferrum."))
        {
            continue;
        }
        let result = state.answer(line);
        match state.api {
            FerrumApi::Software => {
                out.extend_from_slice(line.as_bytes());
                out.extend_from_slice(b"\r\n");
                if let Some(result) = result {
                    out.extend_from_slice(result.as_bytes());
                    out.extend_from_slice(b"\r\n");
                }
                out.extend_from_slice(b">>> ");
            }
            FerrumApi::Legacy => {
                if let Some(result) = result {
                    out.extend_from_slice(result.as_bytes());
                    out.extend_from_slice(b"\r\n");
                    if state.legacy_prompt {
                        out.extend_from_slice(b">>> ");
                    }
                }
            }
        }
    }
    vec![out]
}

pub(crate) fn cp210x() -> Option<PortUsb> {
    Some(PortUsb {
        vid: 0x10C4,
        pid: 0xEA60,
        serial: None,
    })
}

#[cfg(feature = "mock")]
impl crate::device::Device {
    /// a device over anybox's own model of a Ferrum on the app's port with a mouse and a keyboard set, for testing
    /// your code without a box. it runs the exact code a real one gets over a fake port, so it proves your code
    /// works and nothing about the hardware
    pub fn mock_ferrum(click_timing: crate::click::ClickTiming) -> crate::error::Result<Self> {
        use std::sync::mpsc;

        use crate::backend::Sink;
        use crate::device::Start;
        use crate::transport::Candidate;
        use crate::transport::fake::{FakeBox, FakeSystem};

        const PORT: &str = "mock Ferrum";
        let state = Arc::new(Mutex::new(State::software()));
        let fake = FakeBox::by_baud(move |baud, bytes| respond(&state, baud, bytes));
        let system = FakeSystem::default();
        system.plug(PORT, cp210x(), fake);
        let candidate = Candidate {
            path: PORT.into(),
            usb: cp210x(),
        };
        let (signals, incoming) = mpsc::channel();
        let (backend, snapshot, poller) =
            super::attach(Arc::new(system), &candidate, Sink(signals.clone()))?;
        if let Some(poller) = poller {
            *super::lock(&backend.poller) = Some(poller.spawn()?);
        }
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
