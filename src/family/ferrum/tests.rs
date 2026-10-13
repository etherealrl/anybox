use std::collections::HashMap;
use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::frames::{self, FerrumFrames, LEGACY_VERSION, SOFTWARE_VERSION};
use super::{BAUDS, FerrumBackend, Poller, attach};
use crate::backend::{Input, Signal, Sink};
use crate::capability::{
    Action, Axis, Button, Direction, Family, FerrumApi, Quirk, Snapshot, UsbIds,
};
use crate::click::{Click, ClickTiming, Holds};
use crate::device::{Device, Start};
use crate::error::Error;
use crate::event::{Event, EventKinds};
use crate::key::Key;
use crate::transport::codec::{Codec, Incoming};
use crate::transport::fake::{FakeBox, FakeSystem};
use crate::transport::{Candidate, PortUsb, System};

const PORT: &str = "COM7";
const SHORT: Duration = Duration::from_millis(500);

/// what the fake Ferrum knows, answering the way the vendor docs show it. a test can change it while it runs
struct State {
    api: FerrumApi,
    /// the legacy port only answers at this rate, the virtual port at any
    baud: u32,
    mouse: bool,
    keyboard: bool,
    /// an app new enough to know `km.has_mouse()` and the others
    roles: bool,
    /// an app new enough to know the directional axis locks
    directional: bool,
    /// what `device.VID()` and `device.PID()` answer, none for an app that doesn't know them
    ids: Option<(&'static str, &'static str)>,
    /// an app that answers nothing at all to `device.` commands, against what the docs say
    ids_silent: bool,
    silent: bool,
    /// whether the legacy port puts a prompt after what it sends, the docs contradict themselves on it
    legacy_prompt: bool,
    streams: HashMap<String, bool>,
    /// every command line it got, without the line end
    lines: Vec<String>,
}

impl State {
    fn software() -> Self {
        Self {
            api: FerrumApi::Software,
            baud: 0,
            mouse: true,
            keyboard: true,
            roles: true,
            directional: true,
            ids: Some(("046d", "c08b")),
            ids_silent: false,
            silent: false,
            legacy_prompt: false,
            streams: HashMap::new(),
            lines: Vec::new(),
        }
    }

    fn legacy(baud: u32) -> Self {
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

type Shared = Arc<Mutex<State>>;

fn respond(shared: &Shared, baud: u32, bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut state = shared.lock().unwrap();
    if state.silent || (state.api == FerrumApi::Legacy && baud != state.baud) {
        return Vec::new();
    }
    let text = String::from_utf8_lossy(bytes).to_string();
    let mut out = Vec::new();
    for line in text.split(['\r', '\n']).filter(|line| !line.is_empty()) {
        state.lines.push(line.into());
        if state.ids_silent && line.starts_with("device.") {
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

fn cp210x() -> Option<PortUsb> {
    Some(PortUsb {
        vid: 0x10C4,
        pid: 0xEA60,
        serial: None,
    })
}

struct Rig {
    system: FakeSystem,
    fake: Arc<FakeBox>,
    state: Shared,
}

fn rig(state: State) -> Rig {
    rig_with(state, cp210x())
}

fn rig_with(state: State, usb: Option<PortUsb>) -> Rig {
    let state = Arc::new(Mutex::new(state));
    let fake = FakeBox::by_baud({
        let state = state.clone();
        move |baud, bytes| respond(&state, baud, bytes)
    });
    let system = FakeSystem::default();
    system.plug(PORT, usb, fake.clone());
    Rig {
        system,
        fake,
        state,
    }
}

type Attached = (FerrumBackend, Snapshot, Option<Poller>, Receiver<Signal>);

impl Rig {
    fn candidate(&self) -> Candidate {
        self.system
            .ports()
            .into_iter()
            .find(|candidate| candidate.path == Path::new(PORT))
            .unwrap()
    }

    fn attach(&self) -> crate::error::Result<Attached> {
        let (signals, incoming) = mpsc::channel();
        let (backend, snapshot, poller) = attach(
            Arc::new(self.system.clone()),
            &self.candidate(),
            Sink(signals),
        )?;
        Ok((backend, snapshot, poller, incoming))
    }

    fn device(&self) -> (Device, Option<Poller>) {
        let (signals, incoming) = mpsc::channel();
        let (backend, snapshot, poller) = attach(
            Arc::new(self.system.clone()),
            &self.candidate(),
            Sink(signals.clone()),
        )
        .unwrap();
        let device = Device::start(Start {
            backend: Box::new(backend),
            snapshot,
            signals,
            incoming,
            click_timing: ClickTiming::Makcu,
            holds: Holds::from_clock(),
            port: PORT.into(),
        })
        .unwrap();
        (device, poller)
    }

    fn lines(&self) -> Vec<String> {
        self.state.lock().unwrap().lines.clone()
    }

    fn clear(&self) {
        self.state.lock().unwrap().lines.clear();
        self.fake.clear_written();
    }

    fn set(&self, change: impl FnOnce(&mut State)) {
        change(&mut self.state.lock().unwrap());
    }

    fn streaming(&self, name: &str) -> bool {
        self.state
            .lock()
            .unwrap()
            .streams
            .get(name)
            .copied()
            .unwrap_or(false)
    }
}

fn until(mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !done() {
        assert!(Instant::now() < deadline, "gave up waiting");
        thread::sleep(Duration::from_millis(5));
    }
}

fn split_all(codec: &mut FerrumFrames, bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut buffer = bytes.to_vec();
    let mut out = Vec::new();
    while let Some(frame) = codec.split(&mut buffer) {
        out.push(frame);
    }
    out
}

/// every edge the frames turn into, fed one byte at a time when `trickle` is set, like a slow port
fn inputs(codec: &mut FerrumFrames, bytes: &[u8], trickle: bool) -> Vec<String> {
    let mut buffer = Vec::new();
    let mut out = Vec::new();
    let chunks: Vec<&[u8]> = if trickle {
        bytes.chunks(1).collect()
    } else {
        vec![bytes]
    };
    for chunk in chunks {
        buffer.extend_from_slice(chunk);
        while let Some(frame) = codec.split(&mut buffer) {
            let found = match codec.classify(&frame) {
                Incoming::Inputs(inputs) => inputs,
                Incoming::Input { input, .. } => vec![input],
                Incoming::Reply(_) => {
                    out.push(format!("reply {}", String::from_utf8_lossy(&frame)));
                    continue;
                }
                _ => continue,
            };
            out.extend(found.into_iter().map(|input| match input {
                Input::Button { button, pressed } => format!("{button} {pressed}"),
                Input::Key { key, pressed } => format!("{} {pressed}", key.0),
                Input::Motion { dx, dy, wheel, pan } => format!("move {dx} {dy} {wheel} {pan}"),
                Input::Media { .. } => "media".into(),
            }));
        }
    }
    out
}

fn software_codec() -> FerrumFrames {
    FerrumFrames::new(true, Arc::default())
}

// the wire format, against every example the vendor docs give

#[test]
fn a_read_comes_back_as_its_echo_its_result_and_the_prompt() {
    let mut codec = software_codec();
    let frames = split_all(&mut codec, b"km.left()\r\n1\r\n>>> ");
    assert_eq!(frames, vec![b"km.left()\r\n1\r\n>>> ".to_vec()]);
    assert!(matches!(
        codec.classify(&frames[0]),
        Incoming::Reply(key) if key == frames::key(b"km.left()")
    ));
    assert_eq!(frames::result(&frames[0], true), Some(&b"1"[..]));
}

#[test]
fn a_controller_callback_never_answers_a_command() {
    for bytes in [
        &b"ControllerButtons(64)\r\n>>> km.has_mouse()\r\n0\r\n>>> "[..],
        b"km.has_mouse()\r\nControllerAxes(0, 0, 1, 2, 3, 4)\r\n>>> 0\r\n>>> ",
    ] {
        assert_eq!(
            inputs(&mut software_codec(), bytes, true),
            ["reply km.has_mouse()\r\n0\r\n>>> "]
        );
    }
}

#[test]
fn a_result_line_with_no_echo_in_front_belongs_to_nothing() {
    let mut codec = software_codec();
    let frames = split_all(&mut codec, b"1\r\n>>> ");
    assert!(matches!(codec.classify(&frames[0]), Incoming::Noise));
}

#[test]
fn a_setter_and_an_unknown_command_come_back_with_no_result() {
    let mut codec = software_codec();
    let frames = split_all(&mut codec, b"km.left(1)\r\n>>> km.lock_mx+()\r\n>>> ");
    assert_eq!(frames.len(), 2);
    for frame in &frames {
        assert!(matches!(codec.classify(frame), Incoming::Reply(_)));
        assert_eq!(frames::result(frame, true), None);
    }
}

#[test]
fn the_docs_button_callbacks_turn_into_presses_and_releases() {
    // the docs' own sequence, 1 0 2 3 1 17 25 24 8 0
    let mut bytes = Vec::new();
    for state in [1u8, 0, 2, 3, 1, 17, 25, 24, 8, 0] {
        bytes.extend_from_slice(b"km.");
        bytes.push(state);
        bytes.extend_from_slice(b"\r\n>>> ");
    }
    let expected = [
        "left true",
        "left false",
        "right true",
        "left true",
        "right false",
        "side2 true",
        "side1 true",
        "left false",
        "side2 false",
        "side1 false",
    ];
    for trickle in [false, true] {
        assert_eq!(
            inputs(&mut software_codec(), &bytes, trickle),
            expected,
            "trickle {trickle}"
        );
    }
}

#[test]
fn a_button_state_that_looks_like_a_line_end_is_still_one_callback() {
    // 0x0A is right and rear side, 0x0D is left, middle and rear side
    let bytes = b"km.\x0a\r\n>>> km.\x0d\r\n>>> km.\x00\r\n>>> ";
    assert_eq!(
        inputs(&mut software_codec(), bytes, true),
        [
            "right true",
            "side1 true",
            "right false",
            "left true",
            "middle true",
            "left false",
            "middle false",
            "side1 false",
        ]
    );
}

#[test]
fn the_docs_axes_callbacks_turn_into_motion() {
    let bytes = b"Axes(7, 0, 0)\r\n>>> Axes(-8, 0, 0)\r\n>>> Axes(4, 5, 0)\r\n>>> Axes(0, 0, 1)\r\n>>> Axes(0, 0, -1)\r\n>>> ";
    assert_eq!(
        inputs(&mut software_codec(), bytes, false),
        [
            "move 7 0 0 0",
            "move -8 0 0 0",
            "move 4 5 0 0",
            "move 0 0 1 0",
            "move 0 0 -1 0"
        ]
    );
}

#[test]
fn the_docs_keys_callbacks_turn_into_presses_and_releases() {
    let bytes = b"Keys(4)\r\n>>> Keys()\r\n>>> Keys(14)\r\n>>> Keys(6, 14)\r\n>>> Keys(6)\r\n>>> Keys(6, 225)\r\n>>> Keys(6, 42, 225)\r\n>>> Keys(42, 225)\r\n>>> Keys(42)\r\n>>> Keys()\r\n>>> ";
    assert_eq!(
        inputs(&mut software_codec(), bytes, true),
        [
            "4 true",
            "4 false",
            "14 true",
            "6 true",
            "14 false",
            "225 true",
            "42 true",
            "6 false",
            "225 false",
            "42 false",
        ]
    );
}

#[test]
fn a_key_number_that_isnt_a_key_is_left_out() {
    assert_eq!(
        inputs(&mut software_codec(), b"Keys(1, 4, 254)\r\n>>> ", false),
        ["4 true"]
    );
}

#[test]
fn a_callback_in_the_middle_of_a_reply_stays_an_event_and_the_reply_stays_whole() {
    let bytes = b"km.left()\r\nAxes(1, 2, 0)\r\n>>> 1\r\n>>> ";
    assert_eq!(
        inputs(&mut software_codec(), bytes, true),
        ["move 1 2 0 0", "reply km.left()\r\n1\r\n>>> "]
    );
}

#[test]
fn an_echo_that_never_got_its_prompt_doesnt_swallow_the_next_reply() {
    let mut codec = software_codec();
    let frames = split_all(&mut codec, b"km.move(1,1)\r\nkm.left()\r\n0\r\n>>> ");
    assert_eq!(frames, vec![b"km.left()\r\n0\r\n>>> ".to_vec()]);
}

#[test]
fn a_prompt_on_its_own_answers_nothing() {
    let mut codec = software_codec();
    let frames = split_all(&mut codec, b">>> ");
    assert!(matches!(codec.classify(&frames[0]), Incoming::Noise));
}

#[test]
fn the_legacy_port_answers_bare_lines_and_callbacks_with_or_without_a_prompt() {
    let bytes =
        b"kmbox: 2.0.0 Aug 31 2020 21:49:51\r\n>>> km.\x01\r\nkm.\x00\r\n>>> km.version()\r\n";
    let frames = split_all(&mut FerrumFrames::probe(false), bytes);
    assert_eq!(frames[0], b"kmbox: 2.0.0 Aug 31 2020 21:49:51\r\n");
    assert_eq!(
        inputs(&mut FerrumFrames::new(false, Arc::default()), bytes, true),
        ["left true", "left false"]
    );
}

#[test]
fn both_version_answers_are_told_apart() {
    assert_eq!(
        frames::version(b"kmbox: Ferrum\r\n"),
        Some(FerrumApi::Software)
    );
    assert_eq!(
        frames::version(b"kmbox: 2.0.0 Aug 31 2020 21:49:51\r\n"),
        Some(FerrumApi::Legacy)
    );
    assert_eq!(frames::version(b"km.MAKCU\r\n"), None);
    assert_eq!(frames::version(b"kmbox: Ferrum2\r\n"), None);
}

#[test]
fn a_line_that_never_ends_gets_thrown_away() {
    let mut codec = software_codec();
    let mut buffer = vec![b'x'; 2000];
    assert!(codec.split(&mut buffer).is_none());
    assert!(buffer.is_empty());
}

// opening and describing

#[test]
fn the_software_api_opens_and_says_what_is_behind_it() {
    let rig = rig(State::software());
    let (_backend, snapshot, poller, _) = rig.attach().unwrap();
    assert!(poller.is_some());
    assert_eq!(snapshot.info.family, Family::Ferrum);
    assert_eq!(snapshot.info.model.as_deref(), Some("Ferrum"));
    assert_eq!(snapshot.info.ferrum_api, Some(FerrumApi::Software));
    for press in [Action::Press, Action::KeyPress] {
        assert!(
            snapshot
                .capabilities
                .get(press)
                .quirks
                .contains(&Quirk::PhysicalOverride)
        );
    }
    assert_eq!(snapshot.info.firmware, None);
    assert_eq!(snapshot.info.id, None);
    let capabilities = &snapshot.capabilities;
    for action in [
        Action::Move,
        Action::Press,
        Action::Release,
        Action::Click,
        Action::Wheel,
        Action::KeyPress,
        Action::KeyRelease,
        Action::KeyTap,
        Action::ButtonEvents,
        Action::MotionEvents,
        Action::KeyEvents,
        Action::LockButton,
        Action::LockAxis,
        Action::LockKey,
    ] {
        assert!(capabilities.get(action).is_supported(), "{action}");
    }
    for action in [
        Action::MoveNow,
        Action::Pan,
        Action::DiscardPendingMoves,
        Action::ScaleAxis,
        Action::MediaPress,
        Action::MediaEvents,
    ] {
        assert!(!capabilities.get(action).is_supported(), "{action}");
    }
    assert_eq!(capabilities.axes(), [Axis::X, Axis::Y, Axis::Wheel]);
    assert!(
        capabilities
            .get(Action::Release)
            .quirks
            .contains(&Quirk::ForcedUpOnRelease {
                min: Duration::from_millis(125),
                max: Duration::from_millis(175)
            })
    );
    let tap = &capabilities.get(Action::KeyTap).quirks;
    assert!(tap.contains(&Quirk::FixedClickTiming));
    assert!(tap.contains(&Quirk::BoxClickTiming {
        min: Duration::from_millis(75),
        max: Duration::from_millis(125)
    }));
    // the callbacks go off first, since an earlier session could have left them on
    assert_eq!(
        rig.lines(),
        [
            "km.version()",
            "km.buttons(0)",
            "km.axes(0)",
            "km.keys(0)",
            "mouse_proxy.stop()",
            "km.has_mouse()",
            "km.has_keyboard()",
            "km.lock_mx+()",
            "device.VID()",
            "device.PID()"
        ]
    );
}

#[test]
fn an_app_that_cant_say_what_is_plugged_in_is_refused() {
    let rig = rig(State {
        roles: false,
        ..State::software()
    });
    let Err(Error::Unsupported { reason, .. }) = rig.attach() else {
        panic!("it opened without knowing what's behind the box");
    };
    assert!(reason.contains("update the app"), "{reason}");
}

#[test]
fn an_app_without_directional_locks_locks_whole_axes_only() {
    let rig = rig(State {
        directional: false,
        ..State::software()
    });
    let (device, _) = rig.device();
    let capabilities = device.snapshot().capabilities.clone();
    assert_eq!(capabilities.axes(), [Axis::X, Axis::Y, Axis::Wheel]);
    assert!(
        capabilities
            .get(Action::LockAxis)
            .quirks
            .contains(&Quirk::WholeAxisOnly)
    );
    rig.clear();
    device.lock_axis(Axis::Y, Direction::Both).unwrap();
    device.unlock_axis(Axis::Y, Direction::Both).unwrap();
    assert!(matches!(
        device.lock_axis(Axis::X, Direction::Positive),
        Err(Error::Unsupported { .. })
    ));
    // an unlock goes out for both ways, since one way alone can be what's left from before. a second unlock of an
    // unlocked axis does nothing
    assert_eq!(
        rig.lines(),
        ["km.lock_my(1)", "km.lock_my(0)", "km.lock_my(0)"]
    );
}

#[test]
fn an_app_that_ignores_mouse_proxy_stop_still_opens() {
    let rig = rig(State::software());
    let state = rig.state.clone();
    rig.system.plug(
        PORT,
        cp210x(),
        FakeBox::by_baud(move |baud, bytes| {
            let answered = respond(&state, baud, bytes);
            if bytes.starts_with(b"mouse_proxy.") {
                Vec::new()
            } else {
                answered
            }
        }),
    );
    let (_backend, snapshot, _, _) = rig.attach().unwrap();
    assert_eq!(snapshot.info.ferrum_api, Some(FerrumApi::Software));
}

#[test]
fn no_keyboard_set_in_the_app_means_no_keys_and_no_mouse_means_no_mouse() {
    let rig = rig(State {
        keyboard: false,
        mouse: false,
        ..State::software()
    });
    let (_backend, snapshot, _, _) = rig.attach().unwrap();
    for action in [
        Action::Move,
        Action::Click,
        Action::LockAxis,
        Action::ButtonEvents,
        Action::KeyPress,
        Action::KeyEvents,
        Action::LockKey,
    ] {
        assert!(
            !snapshot.capabilities.get(action).is_supported(),
            "{action}"
        );
    }
}

#[test]
fn the_legacy_port_opens_at_whatever_rate_it_was_left_at() {
    let rig = rig(State::legacy(3_000_000));
    let (_backend, snapshot, poller, _) = rig.attach().unwrap();
    assert!(poller.is_none());
    assert_eq!(rig.system.opened_at(), [115_200, 3_000_000]);
    assert_eq!(snapshot.info.model.as_deref(), Some("Ferrum legacy"));
    let capabilities = &snapshot.capabilities;
    assert!(capabilities.get(Action::Click).is_supported());
    assert!(capabilities.get(Action::ButtonEvents).is_supported());
    for action in [
        Action::KeyPress,
        Action::KeyEvents,
        Action::LockKey,
        Action::MotionEvents,
    ] {
        assert!(!capabilities.get(action).is_supported(), "{action}");
    }
    assert_eq!(snapshot.info.ferrum_api, Some(FerrumApi::Legacy));
    assert!(
        capabilities
            .get(Action::LockAxis)
            .quirks
            .contains(&Quirk::WholeAxisOnly)
    );
    assert_eq!(rig.lines(), ["km.version()", "km.buttons(0)"]);
}

#[test]
fn a_legacy_port_with_a_prompt_opens_too() {
    let rig = rig(State {
        legacy_prompt: true,
        ..State::legacy(115_200)
    });
    let (_backend, snapshot, _, _) = rig.attach().unwrap();
    assert_eq!(snapshot.info.model.as_deref(), Some("Ferrum legacy"));
}

#[test]
fn a_ysc_port_never_gets_asked() {
    for serial in ["TOWMCULEFT", "TOWMCURIGHT", "TOWMCUIAP"] {
        let rig = rig_with(
            State::software(),
            Some(PortUsb {
                vid: 0x1A86,
                pid: 0xFE0C,
                serial: Some(serial.into()),
            }),
        );
        assert!(matches!(rig.attach(), Err(Error::NotFound { .. })));
        assert!(rig.fake.written().is_empty(), "{serial}");
    }
}

#[test]
fn a_ch343_port_never_gets_asked() {
    let rig = rig_with(
        State::software(),
        Some(PortUsb {
            vid: 0x1A86,
            pid: 0x55D3,
            serial: None,
        }),
    );
    assert!(matches!(rig.attach(), Err(Error::NotFound { .. })));
    assert!(rig.fake.written().is_empty());
}

#[test]
fn a_port_that_never_answers_is_not_a_ferrum_after_every_rate() {
    let rig = rig(State {
        silent: true,
        ..State::software()
    });
    assert!(matches!(rig.attach(), Err(Error::NotFound { .. })));
    assert_eq!(rig.system.opened_at(), BAUDS);
}

#[test]
fn a_ferrum_that_stops_answering_after_its_version_is_a_timeout() {
    let rig = rig(State::software());
    let state = rig.state.clone();
    let fake = FakeBox::by_baud(move |baud, bytes| {
        let answered = respond(&state, baud, bytes);
        if bytes.starts_with(b"km.has") {
            Vec::new()
        } else {
            answered
        }
    });
    rig.system.plug(PORT, cp210x(), fake);
    assert!(matches!(
        rig.attach(),
        Err(Error::Timeout {
            action: Action::Connect
        })
    ));
}

// commands

#[test]
fn every_command_goes_out_the_way_the_docs_write_it() {
    let rig = rig(State::software());
    let (device, _) = rig.device();
    rig.clear();
    device.move_by(5, -3).unwrap();
    device.press(Button::Side1).unwrap();
    device.release(Button::Side1).unwrap();
    device.click(Button::Middle).unwrap();
    device.wheel(-1).unwrap();
    device.press_key(Key::A).unwrap();
    device.release_key(Key::A).unwrap();
    device.tap_key(Key(30)).unwrap();
    device.lock_button(Button::Left).unwrap();
    device.unlock_button(Button::Left).unwrap();
    device.lock_axis(Axis::X, Direction::Both).unwrap();
    device
        .unlock_axis(Axis::Wheel, Direction::Negative)
        .unwrap();
    device.lock_key(Key(26)).unwrap();
    device.unlock_key(Key(26)).unwrap();
    assert_eq!(
        rig.lines(),
        [
            "km.move(5,-3)",
            "km.side1(1)",
            "km.side1(0)",
            "km.click(2)",
            "km.wheel(-1)",
            "km.down(4)",
            "km.up(4)",
            "km.press(30)",
            "km.lock_ml(1)",
            "km.lock_ml(0)",
            "km.lock_mx+(1)",
            "km.lock_mx-(1)",
            "km.lock_mw-(0)",
            "km.mask(26,1)",
            "km.mask(26,0)",
        ]
    );
    assert_eq!(rig.fake.written()[0], b"km.move(5,-3)\r\n");
}

#[test]
fn a_hold_or_a_big_scroll_is_refused_before_it_goes_out() {
    let rig = rig(State::software());
    let (device, _) = rig.device();
    rig.clear();
    let hold = Click::hold(Duration::from_millis(50));
    assert!(matches!(
        device.click_with(Button::Left, hold),
        Err(Error::Unsupported { .. })
    ));
    assert!(matches!(
        device.tap_key_with(Key::A, hold),
        Err(Error::Unsupported { .. })
    ));
    assert!(matches!(device.wheel(128), Err(Error::Unsupported { .. })));
    device.wheel(127).unwrap();
    assert_eq!(rig.lines(), ["km.wheel(127)"]);
}

#[test]
fn a_command_whose_prompt_got_lost_isnt_answered_by_the_next_one() {
    let rig = rig(State::software());
    let state = rig.state.clone();
    rig.system.plug(
        PORT,
        cp210x(),
        FakeBox::by_baud(move |baud, bytes| {
            let mut reply = respond(&state, baud, bytes);
            if bytes == b"km.move(17,18)\r\n" {
                let len = reply[0].len() - b">>> ".len();
                reply[0].truncate(len);
            }
            reply
        }),
    );
    let (device, poller) = rig.device();
    let poller = poller.unwrap();
    let device = Arc::new(device);
    let moving = {
        let device = device.clone();
        thread::spawn(move || device.move_by(17, 18))
    };
    until(|| rig.lines().iter().any(|line| line == "km.move(17,18)"));
    poller.tick();
    assert!(matches!(moving.join().unwrap(), Err(Error::Disconnected)));
}

#[test]
fn a_poller_round_that_spans_a_reprobe_publishes_nothing() {
    use std::sync::atomic::{AtomicBool, Ordering};

    let rig = rig(State {
        keyboard: false,
        ..State::software()
    });
    let state = rig.state.clone();
    let armed = Arc::new(AtomicBool::new(false));
    let (reached, at) = mpsc::channel();
    let (go, wait) = mpsc::channel::<()>();
    let wait = Mutex::new(wait);
    rig.system.plug(PORT, cp210x(), {
        let armed = armed.clone();
        FakeBox::by_baud(move |baud, bytes| {
            if bytes == b"km.has_keyboard()\r\n" && armed.load(Ordering::Acquire) {
                reached.send(()).unwrap();
                let _ = wait.lock().unwrap().recv_timeout(Duration::from_secs(1));
            }
            respond(&state, baud, bytes)
        })
    });
    let (backend, _, poller, signals) = rig.attach().unwrap();
    let poller = poller.unwrap();
    rig.set(|state| state.keyboard = true);
    armed.store(true, Ordering::Release);
    let round = thread::spawn(move || poller.tick());
    at.recv_timeout(Duration::from_secs(1)).unwrap();
    // what a reprobe does to the generation, while the round still holds the old answers
    backend.shared.generation.fetch_add(1, Ordering::AcqRel);
    go.send(()).unwrap();
    round.join().unwrap();
    assert!(
        !signals
            .try_iter()
            .any(|signal| matches!(signal, Signal::Changed(_)))
    );
}

#[test]
fn a_command_the_box_never_echoes_takes_the_link_down() {
    let rig = rig(State::software());
    let (device, _) = rig.device();
    let events = device.events_of(EventKinds::NONE);
    rig.set(|state| state.silent = true);
    assert!(matches!(device.move_by(1, 1), Err(Error::Disconnected)));
    assert_eq!(events.recv_timeout(SHORT), Ok(Event::Disconnected));
}

#[test]
fn the_legacy_port_sends_commands_without_waiting_for_anything() {
    let rig = rig(State::legacy(115_200));
    let (device, _) = rig.device();
    rig.clear();
    device.move_by(1, 2).unwrap();
    device.click(Button::Left).unwrap();
    device.lock_button(Button::Right).unwrap();
    assert_eq!(
        rig.lines(),
        ["km.move(1,2)", "km.click(0)", "km.lock_mr(1)"]
    );
    assert!(matches!(
        device.press_key(Key::A),
        Err(Error::Unsupported { .. })
    ));
}

// events

#[test]
fn a_receiver_turns_its_callback_on_and_the_last_one_gone_turns_it_off() {
    let rig = rig(State::software());
    let (device, _) = rig.device();
    let buttons = device.events_of(EventKinds::BUTTONS);
    until(|| rig.streaming("km.buttons"));
    assert!(!rig.streaming("km.axes"));
    rig.fake.send(b"km.\x01\r\n>>> ");
    let Ok(Event::Button {
        button, pressed, ..
    }) = buttons.recv_timeout(SHORT)
    else {
        panic!("no press");
    };
    assert_eq!((button, pressed), (Button::Left, true));
    let motion = device.events_of(EventKinds::MOTION);
    until(|| rig.streaming("km.axes"));
    rig.fake.send(b"Axes(3, -4, 0)\r\n>>> ");
    assert!(matches!(
        motion.recv_timeout(SHORT),
        Ok(Event::Motion { dx: 3, dy: -4, .. })
    ));
    drop(buttons);
    until(|| !rig.streaming("km.buttons"));
    drop(motion);
    until(|| !rig.streaming("km.axes"));
}

#[test]
fn keys_come_in_as_edges_from_the_whole_held_list() {
    let rig = rig(State::software());
    let (device, _) = rig.device();
    let keys = device.events_of(EventKinds::KEYS);
    until(|| rig.streaming("km.keys"));
    rig.fake.send(b"Keys(4, 225)\r\n>>> Keys(225)\r\n>>> ");
    let mut seen = Vec::new();
    while seen.len() < 3 {
        match keys.recv_timeout(SHORT) {
            Ok(Event::Key { key, pressed, .. }) => seen.push((key.0, pressed)),
            other => panic!("{other:?} after {seen:?}"),
        }
    }
    assert_eq!(seen, [(4, true), (225, true), (4, false)]);
}

#[test]
fn turning_a_stream_back_on_starts_from_nothing_held() {
    let rig = rig(State::software());
    let (device, _) = rig.device();
    let buttons = device.events_of(EventKinds::BUTTONS);
    until(|| rig.streaming("km.buttons"));
    rig.fake.send(b"km.\x01\r\n>>> ");
    assert!(buttons.recv_timeout(SHORT).is_ok());
    drop(buttons);
    until(|| !rig.streaming("km.buttons"));
    let buttons = device.events_of(EventKinds::BUTTONS);
    until(|| rig.streaming("km.buttons"));
    // left was down when the stream went off, and nothing says it still is, so this is a press again
    rig.fake.send(b"km.\x01\r\n>>> ");
    assert!(matches!(
        buttons.recv_timeout(SHORT),
        Ok(Event::Button { pressed: true, .. })
    ));
}

#[test]
fn the_legacy_button_callback_comes_through_without_a_prompt() {
    let rig = rig(State::legacy(115_200));
    let (device, _) = rig.device();
    let buttons = device.events_of(EventKinds::BUTTONS);
    until(|| rig.streaming("km.buttons"));
    rig.fake.send(b"km.\x02\r\n");
    assert!(matches!(
        buttons.recv_timeout(SHORT),
        Ok(Event::Button {
            button: Button::Right,
            pressed: true,
            ..
        })
    ));
}

#[test]
fn closing_turns_the_callbacks_off() {
    let rig = rig(State::software());
    let (device, _) = rig.device();
    let _events = device.events();
    until(|| rig.streaming("km.keys"));
    device.close().unwrap();
    assert!(!rig.streaming("km.buttons"));
    assert!(!rig.streaming("km.axes"));
    assert!(!rig.streaming("km.keys"));
}

// what's behind the box changing, and the box coming back

#[test]
fn a_keyboard_set_in_the_app_later_shows_up_and_starts_streaming() {
    let rig = rig(State {
        keyboard: false,
        ..State::software()
    });
    let (device, poller) = rig.device();
    let poller = poller.unwrap();
    let _events = device.events();
    until(|| rig.streaming("km.buttons"));
    assert!(!rig.streaming("km.keys"));
    poller.tick();
    assert!(
        !device
            .snapshot()
            .capabilities
            .get(Action::KeyPress)
            .is_supported()
    );
    rig.set(|state| state.keyboard = true);
    poller.tick();
    until(|| {
        device
            .snapshot()
            .capabilities
            .get(Action::KeyPress)
            .is_supported()
    });
    until(|| rig.streaming("km.keys"));
    device.press_key(Key::A).unwrap();
}

#[test]
fn a_reconnect_asks_the_box_again_and_turns_the_streams_back_on() {
    let rig = rig(State::software());
    let (device, _) = rig.device();
    let events = device.events_of(EventKinds::BUTTONS);
    until(|| rig.streaming("km.buttons"));
    rig.system.unplug(PORT);
    assert_eq!(events.recv_timeout(SHORT), Ok(Event::Disconnected));
    let state = rig.state.clone();
    rig.set(|state| state.streams.clear());
    let fake = FakeBox::by_baud(move |baud, bytes| respond(&state, baud, bytes));
    rig.system.plug(PORT, cp210x(), fake.clone());
    assert_eq!(
        events.recv_timeout(Duration::from_secs(3)),
        Ok(Event::Reconnected)
    );
    until(|| rig.streaming("km.buttons"));
    fake.send(b"km.\x10\r\n>>> ");
    assert!(matches!(
        events.recv_timeout(SHORT),
        Ok(Event::Button {
            button: Button::Side2,
            pressed: true,
            ..
        })
    ));
}

#[test]
fn a_ferrum_usb_serial_does_not_follow_a_renamed_port_without_a_unique_id() {
    let identity = Some(PortUsb {
        vid: 0x10c4,
        pid: 0xea60,
        serial: Some("FERRUM01".into()),
    });
    let rig = rig_with(State::software(), identity.clone());
    let (device, _) = rig.device();
    assert_eq!(device.snapshot().info.id, None);
    let events = device.events_of(EventKinds::NONE);
    rig.system.unplug(PORT);
    assert_eq!(events.recv_timeout(SHORT), Ok(Event::Disconnected));
    let state = rig.state.clone();
    let renamed = FakeBox::by_baud(move |baud, bytes| respond(&state, baud, bytes));
    let opens = rig.system.opened_at().len();
    rig.system.plug("COM8", identity.clone(), renamed.clone());
    assert!(events.recv_timeout(Duration::from_millis(350)).is_err());
    assert!(renamed.written().is_empty());
    assert_eq!(rig.system.opened_at().len(), opens);
    let state = rig.state.clone();
    let original = FakeBox::by_baud(move |baud, bytes| respond(&state, baud, bytes));
    rig.system.plug(PORT, identity, original.clone());
    assert_eq!(
        events.recv_timeout(Duration::from_secs(3)),
        Ok(Event::Reconnected)
    );
    device.move_by(1, 2).unwrap();
    assert!(original.written().contains(&b"km.move(1,2)\r\n".to_vec()));
}

#[test]
fn a_legacy_port_that_comes_back_as_the_app_isnt_the_same_box() {
    let rig = rig(State::legacy(115_200));
    let (device, _) = rig.device();
    let events = device.events_of(EventKinds::NONE);
    rig.system.unplug(PORT);
    assert_eq!(events.recv_timeout(SHORT), Ok(Event::Disconnected));
    let state = Arc::new(Mutex::new(State::software()));
    let fake = FakeBox::by_baud(move |baud, bytes| respond(&state, baud, bytes));
    rig.system.plug(PORT, cp210x(), fake);
    assert!(events.recv_timeout(Duration::from_secs(1)).is_err());
}

/// a box that comes back after an unplug, answering from `state`
fn replug(rig: &Rig, state: State) -> Arc<FakeBox> {
    *rig.state.lock().unwrap() = state;
    let shared = rig.state.clone();
    let fake = FakeBox::by_baud(move |baud, bytes| respond(&shared, baud, bytes));
    rig.system.plug(PORT, cp210x(), fake.clone());
    fake
}

#[test]
fn a_one_way_lock_never_turns_into_a_whole_axis_after_a_reconnect() {
    let rig = rig(State::software());
    let (device, _) = rig.device();
    let events = device.events_of(EventKinds::NONE);
    device.lock_axis(Axis::X, Direction::Positive).unwrap();
    device.lock_axis(Axis::Y, Direction::Both).unwrap();
    rig.system.unplug(PORT);
    assert_eq!(events.recv_timeout(SHORT), Ok(Event::Disconnected));
    replug(
        &rig,
        State {
            directional: false,
            ..State::software()
        },
    );
    assert_eq!(
        events.recv_timeout(Duration::from_secs(3)),
        Ok(Event::Reconnected)
    );
    let lines = rig.lines();
    assert!(
        !lines.iter().any(|line| line.starts_with("km.lock_mx(")),
        "{lines:?}"
    );
    // both ways of Y were locked, so the whole axis comes back
    assert!(
        lines.iter().any(|line| line == "km.lock_my(1)"),
        "{lines:?}"
    );
    rig.clear();
    device.unlock_axis(Axis::X, Direction::Positive).unwrap();
    assert!(matches!(
        device.unlock_axis(Axis::Y, Direction::Positive),
        Err(Error::Unsupported { .. })
    ));
    assert_eq!(rig.lines(), ["km.lock_mx(0)"]);
}

#[test]
fn a_lone_negative_lock_can_still_be_unlocked_on_a_whole_axis_box() {
    let rig = rig(State::software());
    let (device, _) = rig.device();
    let events = device.events_of(EventKinds::NONE);
    device.lock_axis(Axis::Wheel, Direction::Negative).unwrap();
    rig.system.unplug(PORT);
    assert_eq!(events.recv_timeout(SHORT), Ok(Event::Disconnected));
    replug(
        &rig,
        State {
            directional: false,
            ..State::software()
        },
    );
    assert_eq!(
        events.recv_timeout(Duration::from_secs(3)),
        Ok(Event::Reconnected)
    );
    rig.clear();
    device
        .unlock_axis(Axis::Wheel, Direction::Negative)
        .unwrap();
    assert_eq!(rig.lines(), ["km.lock_mw(0)"]);
}

#[test]
fn a_has_answer_other_than_one_or_zero_counts_as_none() {
    let rig = rig(State::software());
    let state = rig.state.clone();
    rig.system.plug(
        PORT,
        cp210x(),
        FakeBox::by_baud(move |baud, bytes| {
            let answered = respond(&state, baud, bytes);
            if bytes == b"km.has_mouse()\r\n" {
                vec![b"km.has_mouse()\r\n2\r\n>>> ".to_vec()]
            } else {
                answered
            }
        }),
    );
    assert!(matches!(rig.attach(), Err(Error::Unsupported { .. })));
}

#[test]
fn a_directional_probe_answering_garbage_means_whole_axes_only() {
    let rig = rig(State::software());
    let state = rig.state.clone();
    rig.system.plug(
        PORT,
        cp210x(),
        FakeBox::by_baud(move |baud, bytes| {
            let answered = respond(&state, baud, bytes);
            if bytes == b"km.lock_mx+()\r\n" {
                vec![b"km.lock_mx+()\r\nwhat\r\n>>> ".to_vec()]
            } else {
                answered
            }
        }),
    );
    let (_backend, snapshot, _, _) = rig.attach().unwrap();
    assert!(
        snapshot
            .capabilities
            .get(Action::LockAxis)
            .quirks
            .contains(&Quirk::WholeAxisOnly)
    );
}

#[test]
fn stopping_a_leftover_mouse_proxy_gets_its_answer_and_doesnt_wait_out_the_timeout() {
    let rig = rig(State::software());
    let at = Instant::now();
    rig.attach().unwrap();
    // every answer comes right away, so only a missed one would take as long as a probe's wait
    assert!(at.elapsed() < super::WAIT, "{:?}", at.elapsed());
}

/// these need a Ferrum. run them with
/// `ANYBOX_PORT=COM7 cargo test ferrum::tests::hardware -- --ignored --nocapture --test-threads 1`, with the
/// port of the Ferrum app's virtual port (or the box's own CP210x port for the legacy API) in place of COM7. the
/// ones that say "with you" need you at the PC, they beep once high when they start and twice low when they're done
mod hardware {
    use std::time::{Duration, Instant};

    use crate::capability::{Action, Axis, Button, Direction, Family};
    use crate::connect::Connect;
    use crate::event::{Event, EventKinds, RecvTimeoutError};
    use crate::key::Key;

    fn open() -> crate::device::Device {
        let port =
            std::env::var("ANYBOX_PORT").expect("set ANYBOX_PORT to the port your Ferrum is on");
        Connect::port(port)
            .family(Family::Ferrum)
            .open()
            .unwrap_or_else(|error| panic!("{error}"))
    }

    #[cfg(windows)]
    #[allow(unsafe_code)]
    mod beeps {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn Beep(frequency: u32, millis: u32) -> i32;
        }

        pub(super) fn beep(high: bool) {
            // SAFETY: Beep takes two plain numbers
            unsafe {
                Beep(if high { 1500 } else { 500 }, 400);
            }
        }

        pub(super) fn done() {
            beep(false);
            std::thread::sleep(std::time::Duration::from_millis(150));
            beep(false);
        }
    }

    #[cfg(not(windows))]
    mod beeps {
        pub(super) fn beep(_: bool) {
            println!("go");
        }

        pub(super) fn done() {
            println!("done");
        }
    }

    /// every event of these kinds for 30 seconds after the high beep
    fn watch(kinds: EventKinds) -> Vec<Event> {
        let device = open();
        let events = device.events_of(kinds);
        beeps::beep(true);
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut seen = Vec::new();
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            match events.recv_timeout(left) {
                Ok(event) => {
                    println!("{event:?}");
                    seen.push(event);
                }
                Err(RecvTimeoutError::Timeout) => break,
                Err(error) => panic!("{error:?}"),
            }
        }
        beeps::done();
        seen
    }

    #[test]
    #[ignore]
    fn a_real_box_opens_and_says_what_it_has() {
        let device = open();
        let snapshot = device.snapshot();
        println!("{:?}", snapshot.info);
        for (action, capability) in snapshot.capabilities.iter() {
            println!("{action}: {:?}", capability.execution);
        }
        assert_eq!(snapshot.info.family, Family::Ferrum);
    }

    /// moves the cursor a bit in a square, scrolls one notch each way and right clicks once, so watch the screen
    #[test]
    #[ignore]
    fn moves_scrolls_and_clicks() {
        let device = open();
        for (dx, dy) in [(100, 0), (0, 100), (-100, 0), (0, -100)] {
            let at = Instant::now();
            device.move_by(dx, dy).unwrap();
            println!("move took {:?}", at.elapsed());
            std::thread::sleep(Duration::from_millis(300));
        }
        device.wheel(1).unwrap();
        device.wheel(-1).unwrap();
        let at = Instant::now();
        device.click(Button::Right).unwrap();
        println!("click took {:?}", at.elapsed());
    }

    /// with you: after the high beep, hold the left button down and keep holding it. anybox presses and releases
    /// left too, and you should see the release force it up for a moment even though you hold it, then it's down
    /// again by itself. it prints what the box reported
    #[test]
    #[ignore]
    fn a_release_forces_the_button_up_for_a_moment() {
        let device = open();
        let events = device.events_of(EventKinds::BUTTONS);
        beeps::beep(true);
        std::thread::sleep(Duration::from_secs(3));
        device.press(Button::Left).unwrap();
        std::thread::sleep(Duration::from_millis(500));
        device.release(Button::Left).unwrap();
        std::thread::sleep(Duration::from_secs(2));
        beeps::done();
        while let Ok(event) = events.try_recv() {
            println!("{event:?}");
        }
    }

    /// types nothing visible: holds F13 and taps F14, which most apps ignore
    #[test]
    #[ignore]
    fn keys_go_out() {
        let device = open();
        if !device
            .snapshot()
            .capabilities
            .get(Action::KeyPress)
            .is_supported()
        {
            println!("no keyboard set in the Ferrum app, skipped");
            return;
        }
        device.press_key(Key(0x68)).unwrap();
        std::thread::sleep(Duration::from_millis(100));
        device.release_key(Key(0x68)).unwrap();
        device.tap_key(Key(0x69)).unwrap();
    }

    /// with you: after the high beep, move the mouse and click left. for the first 15 seconds left is locked and
    /// X is locked one way at a time, so the cursor only goes left, then only right, and left clicks do nothing. on
    /// the legacy port or an app without directional locks X is locked both ways the whole time instead, so it
    /// doesn't go sideways at all. after that everything works again
    #[test]
    #[ignore]
    fn locks_block_the_physical_input() {
        let device = open();
        beeps::beep(true);
        device.lock_button(Button::Left).unwrap();
        let one_way = !device
            .snapshot()
            .capabilities
            .get(Action::LockAxis)
            .quirks
            .contains(&crate::capability::Quirk::WholeAxisOnly);
        let first = if one_way {
            Direction::Positive
        } else {
            Direction::Both
        };
        device.lock_axis(Axis::X, first).unwrap();
        std::thread::sleep(Duration::from_millis(7500));
        if one_way {
            device.unlock_axis(Axis::X, Direction::Positive).unwrap();
            device.lock_axis(Axis::X, Direction::Negative).unwrap();
        }
        std::thread::sleep(Duration::from_millis(7500));
        device.close().unwrap();
        beeps::done();
    }

    /// with you: click every button, move the mouse, scroll, and press a few keys within 30 seconds
    #[test]
    #[ignore]
    fn every_callback_comes_through() {
        let seen = watch(EventKinds::ALL);
        assert!(
            seen.iter()
                .any(|event| matches!(event, Event::Button { .. }))
        );
    }

    /// with you: click left a few times within 30 seconds. while it runs anybox also clicks right every 2 seconds,
    /// and the output says whether those show up as events too (the docs suggest callbacks only show what you
    /// do yourself)
    #[test]
    #[ignore]
    fn whether_the_callbacks_show_what_anybox_sends() {
        let device = open();
        let events = device.events_of(EventKinds::BUTTONS);
        beeps::beep(true);
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut next = Instant::now();
        while Instant::now() < deadline {
            if Instant::now() >= next {
                device.click(Button::Right).unwrap();
                next += Duration::from_secs(2);
            }
            if let Ok(event) = events.recv_timeout(Duration::from_millis(50)) {
                println!("{event:?}");
            }
        }
        beeps::done();
    }

    /// presses left on the bare port, closes it and opens it again, then reads left back. the docs say only echo
    /// resets when the port closes, so it should still read as held (if the read shows what software pressed at
    /// all, which the docs don't say either, so the output says both)
    #[test]
    #[ignore]
    fn a_press_survives_the_port_closing() {
        use crate::transport::System;
        let path = std::env::var("ANYBOX_PORT").unwrap();
        let ask = |bytes: &[u8]| {
            let port = crate::transport::os::OsSystem
                .open(std::path::Path::new(&path), 115_200)
                .unwrap();
            port.write_all(bytes).unwrap();
            std::thread::sleep(Duration::from_millis(200));
            let mut chunk = [0u8; 256];
            let got = port.read(&mut chunk).unwrap_or(0);
            port.retire();
            String::from_utf8_lossy(&chunk[..got]).to_string()
        };
        println!("before {:?}", ask(b"km.left()\r\n"));
        println!("press {:?}", ask(b"km.left(1)\r\n"));
        println!("after reopening {:?}", ask(b"km.left()\r\n"));
        println!("release {:?}", ask(b"km.left(0)\r\n"));
    }
}

// the device plugged into the box, by its USB ids

#[test]
fn the_first_peripheral_s_usb_ids_are_read_at_open() {
    let rig = rig(State::software());
    let (device, _) = rig.device();
    assert_eq!(
        device.snapshot().info.peripheral,
        Some(UsbIds::new(0x046d, 0xc08b))
    );
}

#[test]
fn nothing_plugged_in_is_no_peripheral_and_the_default_pair_is_never_asked() {
    let rig = rig(State {
        mouse: false,
        keyboard: false,
        ..State::software()
    });
    let (device, _) = rig.device();
    assert_eq!(device.snapshot().info.peripheral, None);
    assert!(!rig.lines().iter().any(|line| line.starts_with("device.")));
}

#[test]
fn an_app_that_doesnt_know_the_ids_still_opens_with_no_peripheral() {
    let rig = rig(State {
        ids: None,
        ..State::software()
    });
    let (device, _) = rig.device();
    assert_eq!(device.snapshot().info.peripheral, None);
}

#[test]
fn the_legacy_port_is_never_asked_for_ids() {
    let rig = rig(State::legacy(115_200));
    let (device, _) = rig.device();
    assert_eq!(device.snapshot().info.peripheral, None);
    assert!(!rig.lines().iter().any(|line| line.starts_with("device.")));
}

#[test]
fn only_four_hex_digits_after_the_asked_name_are_an_id() {
    assert_eq!(super::usb_id(Some(b"VID=046d"), "VID"), Some(0x046d));
    assert_eq!(super::usb_id(Some(b"PID=C53F"), "PID"), Some(0xc53f));
    assert_eq!(super::usb_id(Some(b"PID=c53f"), "VID"), None);
    assert_eq!(super::usb_id(Some(b"VID=46d"), "VID"), None);
    assert_eq!(super::usb_id(Some(b"VID=046d0"), "VID"), None);
    assert_eq!(super::usb_id(Some(b"VID=04zd"), "VID"), None);
    assert_eq!(super::usb_id(Some(b""), "VID"), None);
    assert_eq!(super::usb_id(None, "VID"), None);
}

#[test]
fn a_mouse_set_in_the_app_later_brings_its_ids_and_a_changed() {
    let rig = rig(State {
        mouse: false,
        keyboard: false,
        ..State::software()
    });
    let (device, poller) = rig.device();
    let events = device.events();
    assert_eq!(device.snapshot().info.peripheral, None);
    rig.set(|state| state.mouse = true);
    poller.unwrap().tick();
    until(|| device.snapshot().info.peripheral == Some(UsbIds::new(0x046d, 0xc08b)));
    assert!(
        std::iter::from_fn(|| events.recv_timeout(SHORT).ok()).any(|e| matches!(e, Event::Changed))
    );
}

#[test]
fn an_app_that_says_nothing_to_the_ids_still_opens_with_no_peripheral() {
    let rig = rig(State {
        ids_silent: true,
        ..State::software()
    });
    let (device, _) = rig.device();
    assert_eq!(device.snapshot().info.peripheral, None);
}
