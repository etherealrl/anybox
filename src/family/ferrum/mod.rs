use std::any::Any;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::backend::{
    Backend, ChordKind, Command, InputKind, InputKinds, LOCKED, LockSlot, Sign, Sink,
    StreamFailure, Typing, UNLOCKED, Usage,
};
use crate::capability::{
    Action, Axis, Button, Capabilities, Capability, Family, FerrumAction, FerrumApi, Firmware,
    Info, Quirk, Snapshot, UsbIds,
};
use crate::error::{Error, Result};
use crate::extension::{Extension, ExtensionReply};
use crate::transport::link::{Link, Target};
use crate::transport::probe::Probe;
use crate::transport::{Candidate, Port, System, is_ch343, is_ysc};

mod frames;
mod poller;

use frames::{END, FerrumFrames, PROMPT};
pub(crate) use poller::Poller;

/// the legacy port starts at 115200 after every power cycle, and an earlier `km.baud` may have left it at
/// another rate. the vendor says not to go past 3,000,000. the Software API's virtual port takes any rate, so it
/// answers on the first
pub(crate) const BAUDS: [u32; 11] = [
    115_200, 3_000_000, 2_000_000, 1_000_000, 921_600, 460_800, 230_400, 57_600, 38_400, 19_200,
    9_600,
];
/// nothing documents how fast the box answers, the vendor just says a command takes well under a ms on the wire
const WAIT: Duration = Duration::from_millis(300);
/// the box clamps the wheel to -128..127, so anything past this wouldn't do what was asked
const NOTCHES: u16 = 127;
const NO_TYPING: &str = "Ferrum has no command to type text, tap the keys yourself instead";
/// the box's own press for `km.click` and `km.press`, a random hold in this range
const CLICK_HOLD: (Duration, Duration) = (Duration::from_millis(75), Duration::from_millis(125));
/// after a release the box keeps it up for a random while in this range, then the physical button counts again
const FORCED_UP: (Duration, Duration) = (Duration::from_millis(125), Duration::from_millis(175));

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn not_found(path: &Path) -> Error {
    Error::NotFound {
        tried: vec![path.display().to_string()],
    }
}

/// what the box has behind it right now, and whether the app knows the directional axis locks
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Roles {
    pub(crate) mouse: bool,
    pub(crate) keyboard: bool,
    pub(crate) directional: bool,
    pub(crate) peripheral: Option<UsbIds>,
    /// the Ferrum API version, none on the legacy port and on an app too old to say
    pub(crate) firmware: Option<Firmware>,
}

/// opens a Ferrum on one port, or returns `NotFound` when what's there isn't one
pub(crate) fn open(candidate: &Candidate, sink: Sink) -> Result<(Box<dyn Backend>, Snapshot)> {
    let (backend, snapshot, poller) =
        attach(Arc::new(crate::transport::os::OsSystem), candidate, sink)?;
    if let Some(poller) = poller {
        *lock(&backend.poller) = Some(poller.spawn()?);
    }
    Ok((Box::new(backend), snapshot))
}

/// does everything `open` does except starting the poller, so tests can run its rounds by hand
pub(crate) fn attach(
    system: Arc<dyn System>,
    candidate: &Candidate,
    sink: Sink,
) -> Result<(FerrumBackend, Snapshot, Option<Poller>)> {
    let path = candidate.path.as_path();
    // Medius and MAKCU sit behind a CH343 and a YSC 8K has its own id, a Ferrum is never behind either
    if candidate
        .usb
        .as_ref()
        .is_some_and(|usb| is_ch343(usb) || is_ysc(usb))
    {
        return Err(not_found(path));
    }
    let stop = AtomicBool::new(false);
    for (at, &baud) in BAUDS.iter().enumerate() {
        let port = match system.open(path, baud) {
            Ok(port) => port,
            Err(Error::NotFound { .. }) => return Err(not_found(path)),
            Err(error) => return Err(error),
        };
        let probe = Probe::new(&*port, &stop);
        let Some(api) = identify(&probe) else {
            continue;
        };
        let roles = describe(&probe, api)?;
        let mut bauds = BAUDS.to_vec();
        bauds.rotate_left(at);
        let target = Target {
            family: Family::Ferrum,
            path: path.to_path_buf(),
            usb: candidate.usb.clone(),
            id: None,
            bauds,
        };
        return start(system, port, api, roles, target, sink);
    }
    Err(not_found(path))
}

fn start(
    system: Arc<dyn System>,
    port: Box<dyn Port>,
    api: FerrumApi,
    roles: Roles,
    target: Target,
    sink: Sink,
) -> Result<(FerrumBackend, Snapshot, Option<Poller>)> {
    let snapshot = snapshot(api, &roles);
    let shared = Arc::new(Shared {
        api,
        snapshot: RwLock::new(snapshot.clone()),
        directional: AtomicBool::new(roles.directional),
        streams: Mutex::new(InputKinds::default()),
        seen: Arc::default(),
        generation: AtomicU64::new(0),
        stop: AtomicBool::new(false),
    });
    let reprobe = {
        let shared = shared.clone();
        Box::new(move |probe: &Probe<'_>| {
            if identify(probe) != Some(shared.api) {
                return Err(Error::Disconnected);
            }
            let roles = describe(probe, shared.api)?;
            let snapshot = self::snapshot(shared.api, &roles);
            // the callbacks just got switched off, so the core's stream switch after Reconnected starts clean
            *lock(&shared.streams) = InputKinds::default();
            shared.generation.fetch_add(1, Ordering::AcqRel);
            shared
                .directional
                .store(roles.directional, Ordering::Release);
            *shared
                .snapshot
                .write()
                .unwrap_or_else(PoisonError::into_inner) = snapshot.clone();
            Ok(snapshot)
        })
    };
    let link = Arc::new(Link::start(
        system,
        port,
        Box::new(FerrumFrames::new(
            api == FerrumApi::Software,
            shared.seen.clone(),
        )),
        target,
        reprobe,
        sink.clone(),
    )?);
    // the legacy port can't say what's behind it, so there's nothing to poll
    let poller = (api == FerrumApi::Software)
        .then(|| Poller::new(shared.clone(), link.clone(), sink.clone()));
    let backend = FerrumBackend {
        shared,
        link,
        poller: Mutex::new(None),
    };
    Ok((backend, snapshot, poller))
}

fn identify(probe: &Probe<'_>) -> Option<FerrumApi> {
    let reply = probe.ask(
        frames::VERSION_QUERY,
        &mut FerrumFrames::probe(false),
        |frame| frames::version(frame).is_some(),
        WAIT,
    );
    frames::version(&reply.ok()??)
}

fn line(command: &str) -> Vec<u8> {
    let mut bytes = command.as_bytes().to_vec();
    bytes.extend_from_slice(END);
    bytes
}

/// asks over the bare port and returns the result line, or none when the box knows no such command. the box
/// already said it's a Ferrum, so no answer at all is a timeout and not NotFound
fn query(probe: &Probe<'_>, command: &str) -> Result<Option<Vec<u8>>> {
    let reply = probe
        .ask(
            &line(command),
            &mut FerrumFrames::probe(true),
            |frame| frame.starts_with(command.as_bytes()) && frame.ends_with(PROMPT),
            WAIT,
        )?
        .ok_or(Error::Timeout {
            action: Action::Connect,
        })?;
    Ok(frames::result(&reply, true).map(<[u8]>::to_vec))
}

/// the callbacks survive a port reopen, so an earlier session can leave them on and they'd start sending before
/// anyone asked. on the Software API this also reads what's behind the box
fn describe(probe: &Probe<'_>, api: FerrumApi) -> Result<Roles> {
    match api {
        FerrumApi::Legacy => {
            probe.say(&line("km.buttons(0)"))?;
            Ok(Roles {
                mouse: true,
                keyboard: false,
                directional: false,
                peripheral: None,
                firmware: None,
            })
        }
        FerrumApi::Software => {
            for stream in ["buttons", "axes", "keys"] {
                query(probe, &format!("km.{stream}(0)"))?;
            }
            // a proxy an earlier session left running keeps every other mouse command from doing anything. what an
            // app that doesn't know the command sends back isn't documented, so no answer at all is fine here
            match query(probe, "mouse_proxy.stop()") {
                Ok(_) | Err(Error::Timeout { .. }) => {}
                Err(error) => return Err(error),
            }
            let has = |command| {
                bit(query(probe, command)?.as_deref()).ok_or_else(|| Error::Unsupported {
                    action: Action::Connect,
                    family: Family::Ferrum,
                    reason: format!(
                        "the Ferrum app doesn't answer {command}, so anybox can't tell what's plugged into the box. update the app"
                    )
                    .into(),
                })
            };
            let mut roles = Roles {
                mouse: has("km.has_mouse()")?,
                keyboard: has("km.has_keyboard()")?,
                // the read form only gets a result from an app that knows the command
                directional: bit(query(probe, "km.lock_mx+()")?.as_deref()).is_some(),
                peripheral: None,
                // documented since app 2.0.6, an older app may not answer at all
                firmware: match query(probe, "ferrum.version()") {
                    Ok(result) => app_version(result.as_deref()),
                    Err(Error::Timeout { .. }) => None,
                    Err(error) => return Err(error),
                },
            };
            roles.peripheral = peripheral(&roles, |command| query(probe, command))?;
            Ok(roles)
        }
    }
}

/// what's plugged into the box by its USB ids, none when nothing is (the box then answers a default pair that's
/// also a real Logitech receiver) or when the app doesn't know the commands
pub(crate) fn peripheral(
    roles: &Roles,
    mut ask: impl FnMut(&str) -> Result<Option<Vec<u8>>>,
) -> Result<Option<UsbIds>> {
    if !roles.mouse && !roles.keyboard {
        return Ok(None);
    }
    let mut id = |name: &str| match ask(&format!("device.{name}()")) {
        Ok(result) => Ok(usb_id(result.as_deref(), name)),
        // the docs say an app echoes a command it doesn't know with no result, silence is caught too in case one doesn't
        Err(Error::Timeout { .. }) => Ok(None),
        Err(error) => Err(error),
    };
    Ok(id("VID")?
        .zip(id("PID")?)
        .map(|(vid, pid)| UsbIds::new(vid, pid)))
}

/// `VID=046d` for `device.VID()`, 4 hex digits and the name it was asked for
pub(crate) fn usb_id(result: Option<&[u8]>, name: &str) -> Option<u16> {
    let hex = std::str::from_utf8(result?)
        .ok()?
        .strip_prefix(name)?
        .strip_prefix('=')?;
    (hex.len() == 4 && hex.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| u16::from_str_radix(hex, 16).ok())
        .flatten()
}

/// `v2.0.6` for `ferrum.version()`, the Ferrum API version, kept as text when it isn't three numbers
pub(crate) fn app_version(result: Option<&[u8]>) -> Option<Firmware> {
    let text = std::str::from_utf8(result?).ok()?.trim();
    let numbers: Option<Vec<u16>> = text
        .strip_prefix('v')
        .and_then(|rest| rest.split('.').map(|n| n.parse().ok()).collect());
    match numbers.as_deref() {
        Some(&[major, minor, patch]) => Some(Firmware::Version {
            major,
            minor,
            patch,
        }),
        // anything without the v isn't a version, an older app could answer with whatever
        _ if text.starts_with('v') => Some(Firmware::Text(text.to_owned())),
        _ => None,
    }
}

/// a `1` or `0` result, none for no result or anything else
pub(crate) fn bit(result: Option<&[u8]>) -> Option<bool> {
    match result? {
        b"1" => Some(true),
        b"0" => Some(false),
        _ => None,
    }
}

pub(crate) fn snapshot(api: FerrumApi, roles: &Roles) -> Snapshot {
    let mut info = Info::new(Family::Ferrum);
    info.ferrum_api = Some(api);
    info.peripheral = roles.peripheral;
    info.firmware = roles.firmware.clone();
    info.model = Some(
        match api {
            FerrumApi::Software => "Ferrum",
            FerrumApi::Legacy => "Ferrum legacy",
        }
        .into(),
    );
    let released = || {
        Capability::native().with(Quirk::ForcedUpOnRelease {
            min: FORCED_UP.0,
            max: FORCED_UP.1,
        })
    };
    let tap = || {
        released()
            .with(Quirk::BoxClickTiming {
                min: CLICK_HOLD.0,
                max: CLICK_HOLD.1,
            })
            .with(Quirk::FixedClickTiming)
    };
    let mouse = [
        Action::Move,
        Action::Press,
        Action::Release,
        Action::Click,
        Action::Wheel,
        Action::ButtonEvents,
        Action::MotionEvents,
        Action::LockButton,
        Action::LockAxis,
    ];
    let keyboard = [
        Action::KeyPress,
        Action::KeyRelease,
        Action::KeyTap,
        Action::KeyEvents,
        Action::LockKey,
        Action::KeyChord,
    ];
    let mut capabilities = if roles.mouse {
        Capabilities::new(BUTTONS.to_vec())
            .set(Action::Move, Capability::native())
            .set(
                Action::Press,
                Capability::native().with(Quirk::PhysicalOverride),
            )
            .set(Action::Release, released())
            .set(Action::Click, tap())
            .set(
                Action::Wheel,
                Capability::native().with(Quirk::NotchesPerCommand(NOTCHES)),
            )
            .set(Action::ButtonEvents, Capability::native())
            .set(Action::LockButton, Capability::native())
            .set(
                Action::MotionEvents,
                match api {
                    FerrumApi::Software => Capability::native(),
                    FerrumApi::Legacy => Capability::unsupported(
                        "the legacy port only reports buttons, motion needs the Ferrum app's port",
                    ),
                },
            )
            .set(
                Action::LockAxis,
                if roles.directional {
                    Capability::native()
                } else {
                    Capability::native().with(Quirk::WholeAxisOnly)
                },
            )
    } else {
        let none = Capability::unsupported(
            "the Ferrum app has no device set as a mouse right now, or the box isn't plugged into this PC, set one or plug the box back in and it shows up by itself",
        );
        mouse
            .into_iter()
            .fold(Capabilities::new(Vec::new()), |capabilities, action| {
                capabilities.set(action, none.clone())
            })
    };
    if roles.mouse {
        capabilities = capabilities.with_axes(vec![Axis::X, Axis::Y, Axis::Wheel]);
    }
    capabilities = capabilities.set(
        Action::Ferrum(FerrumAction::CatchXy),
        match (api, roles.mouse) {
            (FerrumApi::Legacy, _) => Capability::unsupported(
                "the legacy port has no catch_xy, it needs the Ferrum app's port",
            ),
            (FerrumApi::Software, true) => Capability::native(),
            (FerrumApi::Software, false) => Capability::unsupported(
                "the Ferrum app has no device set as a mouse right now, set one and it shows up by itself",
            ),
        },
    );
    capabilities = match api {
        FerrumApi::Software if roles.keyboard => capabilities
            .set(
                Action::KeyPress,
                Capability::native().with(Quirk::PhysicalOverride),
            )
            .set(Action::KeyRelease, released())
            .set(Action::KeyTap, tap())
            .set(Action::KeyEvents, Capability::native())
            .set(Action::LockKey, Capability::native())
            // km.multipress holds each key for its own random while, so they can come up one after the other
            .set(
                Action::KeyChord,
                released().with(Quirk::BoxClickTiming {
                    min: CLICK_HOLD.0,
                    max: CLICK_HOLD.1,
                }),
            ),
        FerrumApi::Software => {
            let none = Capability::unsupported(
                "the Ferrum app has no device set as a keyboard right now, or the box isn't plugged into this PC, set one or plug the box back in and it shows up by itself",
            );
            keyboard
                .into_iter()
                .fold(capabilities, |capabilities, action| {
                    capabilities.set(action, none.clone())
                })
        }
        FerrumApi::Legacy => {
            let none = Capability::unsupported(
                "the legacy port has no keyboard commands, only the Ferrum app's port has them",
            );
            keyboard
                .into_iter()
                .fold(capabilities, |capabilities, action| {
                    capabilities.set(action, none.clone())
                })
        }
    };
    let no_media = Capability::unsupported("Ferrum has no media keys, only keyboard keys");
    let capabilities = capabilities
        .set(Action::TypeText, Capability::unsupported(NO_TYPING))
        .set(
            Action::ForceReleaseKey,
            Capability::unsupported(
                "it isn't known yet whether Ferrum's key mask lets go of a key that's already held",
            ),
        )
        .set(Action::MediaPress, no_media.clone())
        .set(Action::MediaRelease, no_media.clone())
        .set(Action::MediaTap, no_media.clone())
        .set(Action::MediaEvents, no_media)
        .set(
            Action::MoveNow,
            Capability::unsupported("Ferrum only has km.move, there's no move that skips ahead"),
        )
        .set(
            Action::DiscardPendingMoves,
            Capability::unsupported("Ferrum can't drop moves it already took"),
        )
        .set(
            Action::Pan,
            Capability::unsupported("Ferrum has no horizontal scroll"),
        )
        .set(
            Action::ScaleAxis,
            Capability::unsupported("Ferrum can only block an axis, not scale it"),
        );
    Snapshot::new(info, capabilities)
}

const BUTTONS: [Button; 5] = [
    Button::Left,
    Button::Right,
    Button::Middle,
    Button::Side1,
    Button::Side2,
];

/// state the backend, the poller and the reprobe share
pub(crate) struct Shared {
    api: FerrumApi,
    snapshot: RwLock<Snapshot>,
    directional: AtomicBool,
    /// what the box streams right now
    streams: Mutex<InputKinds>,
    seen: Arc<Mutex<frames::Seen>>,
    /// goes up on every reprobe, so a poller round that spans a reconnect throws away what it saw
    generation: AtomicU64,
    stop: AtomicBool,
}

impl Shared {
    fn snapshot(&self) -> Snapshot {
        self.snapshot
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

pub(crate) struct FerrumBackend {
    shared: Arc<Shared>,
    link: Arc<Link>,
    poller: Mutex<Option<JoinHandle<()>>>,
}

fn button_name(button: Button) -> Option<&'static str> {
    Some(match button {
        Button::Left => "left",
        Button::Right => "right",
        Button::Middle => "middle",
        Button::Side1 => "side1",
        Button::Side2 => "side2",
        _ => return None,
    })
}

/// `km.click` numbers them, rear side is 3 and front side is 4
fn button_number(button: Button) -> Option<u8> {
    BUTTONS
        .iter()
        .position(|&known| known == button)
        .and_then(|at| u8::try_from(at).ok())
}

/// with `whole` an axis gets the command for both ways, for an app or port with no directional locks
fn lock_name(slot: LockSlot, whole: bool) -> Option<String> {
    Some(match slot {
        LockSlot::Button(button) => match button {
            Button::Left => "ml".into(),
            Button::Right => "mr".into(),
            Button::Middle => "mm".into(),
            Button::Side1 => "ms1".into(),
            Button::Side2 => "ms2".into(),
            _ => return None,
        },
        LockSlot::Axis(axis, sign) => {
            let axis = match axis {
                Axis::X => "mx",
                Axis::Y => "my",
                Axis::Wheel => "mw",
                _ => return None,
            };
            let sign = match sign {
                _ if whole => "",
                Sign::Positive => "+",
                Sign::Negative => "-",
            };
            format!("{axis}{sign}")
        }
        LockSlot::Key(_) => return None,
    })
}

/// the callback's command name for each stream
fn stream_name(kind: InputKind) -> Option<&'static str> {
    match kind {
        InputKind::Buttons => Some("buttons"),
        InputKind::Motion => Some("axes"),
        InputKind::Keys => Some("keys"),
        InputKind::Media => None,
    }
}

fn kind_on(kinds: InputKinds, kind: InputKind) -> bool {
    match kind {
        InputKind::Buttons => kinds.buttons,
        InputKind::Motion => kinds.motion,
        InputKind::Keys => kinds.keys,
        InputKind::Media => kinds.media,
    }
}

fn set_kind(kinds: &mut InputKinds, kind: InputKind, on: bool) {
    match kind {
        InputKind::Buttons => kinds.buttons = on,
        InputKind::Motion => kinds.motion = on,
        InputKind::Keys => kinds.keys = on,
        InputKind::Media => kinds.media = on,
    }
}

impl FerrumBackend {
    /// checks the command against what the box can do right now, because a reconnect or a role change can change
    /// that after the core already said yes. releases and unlocks always go out
    fn check(&self, command: Command) -> Result<()> {
        if matches!(
            command,
            Command::Release(_)
                | Command::KeyRelease(_)
                | Command::Lock { keep: UNLOCKED, .. }
                | Command::KeyChord {
                    kind: ChordKind::Release,
                    ..
                }
        ) {
            return Ok(());
        }
        let action = command.action();
        if self
            .shared
            .snapshot()
            .capabilities
            .get(action)
            .is_supported()
        {
            Ok(())
        } else {
            Err(Error::Unsupported {
                action,
                family: Family::Ferrum,
                reason: "the box changed during this call, check its snapshot again".into(),
            })
        }
    }

    fn encode(command: Command, whole: bool) -> Option<Vec<u8>> {
        let text = match command {
            Command::Move(dx, dy) => format!("km.move({dx},{dy})"),
            Command::Press(button) => format!("km.{}(1)", button_name(button)?),
            Command::Release(button) => format!("km.{}(0)", button_name(button)?),
            Command::Click { button, hold: None } => {
                format!("km.click({})", button_number(button)?)
            }
            Command::Wheel(notches) => format!("km.wheel({notches})"),
            // keys always go by number, because the digit strings 0 to 9 collide with the key numbers
            Command::KeyPress(key) => format!("km.down({})", key.0),
            Command::KeyRelease(key) => format!("km.up({})", key.0),
            Command::KeyTap { key, hold: None } => format!("km.press({})", key.0),
            Command::KeyChord { kind, keys } => format!(
                "km.{}({})",
                match kind {
                    ChordKind::Press => "multidown",
                    ChordKind::Release => "multiup",
                    ChordKind::Tap => "multipress",
                },
                keys.as_slice()
                    .iter()
                    .map(|key| key.0.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Command::Lock {
                slot: LockSlot::Key(key),
                keep,
            } if keep == LOCKED || keep == UNLOCKED => {
                format!("km.mask({},{})", key.0, u8::from(keep == LOCKED))
            }
            Command::Lock { slot, keep } if keep == LOCKED || keep == UNLOCKED => {
                format!(
                    "km.lock_{}({})",
                    lock_name(slot, whole)?,
                    u8::from(keep == LOCKED)
                )
            }
            // a hold, a scale and everything the box has no command for
            _ => return None,
        };
        Some(line(&text))
    }

    /// on the Software API every command waits for its echo and prompt, because that's the only way to tell a
    /// command that got through from one that got lost. the legacy port sends nothing back for a command
    fn request(&self, bytes: Vec<u8>, until: Instant) -> Result<()> {
        let echo = bytes.strip_suffix(END).unwrap_or(&bytes);
        let expect = (self.shared.api == FerrumApi::Software).then(|| frames::key(echo));
        self.link.request_until(bytes, expect, until).map(|_| ())
    }

    fn switch(&self, kind: InputKind, on: bool, until: Instant) -> Result<()> {
        let Some(name) = stream_name(kind) else {
            return Ok(());
        };
        if on {
            // what was held while the stream was off isn't known, so the next callback starts from nothing
            let mut seen = frames::lock(&self.shared.seen);
            match kind {
                InputKind::Buttons => seen.forget_buttons(),
                InputKind::Keys => seen.forget_keys(),
                InputKind::Motion | InputKind::Media => {}
            }
        }
        self.request(line(&format!("km.{name}({})", u8::from(on))), until)
    }
}

const KINDS: [InputKind; 3] = [InputKind::Buttons, InputKind::Motion, InputKind::Keys];

impl Backend for FerrumBackend {
    fn send(&self, command: Command) -> Result<()> {
        self.check(command)?;
        let whole = !self.shared.directional.load(Ordering::Acquire);
        // with whole axis locks the core only locks both ways together, so the positive one carries the axis. an
        // unlock still goes out, the core sends one way alone only when that's all that's locked
        if whole
            && let Command::Lock {
                slot: LockSlot::Axis(_, Sign::Negative),
                keep,
            } = command
            && keep != UNLOCKED
        {
            return Ok(());
        }
        let Some(bytes) = Self::encode(command, whole) else {
            let action = command.action();
            return Err(Error::Unsupported {
                action,
                family: Family::Ferrum,
                reason: self.shared.snapshot().capabilities.get(action).reason(),
            });
        };
        self.request(bytes, Instant::now() + crate::backend::WRITE_DEADLINE)
    }

    /// the box gives no error for anything, so a switch that got its echo is as confirmed as it gets
    fn set_streams(&self, kinds: InputKinds) -> std::result::Result<(), StreamFailure> {
        let until = Instant::now() + crate::backend::WRITE_DEADLINE;
        let mut streams = lock(&self.shared.streams);
        for kind in KINDS {
            let on = kind_on(kinds, kind);
            if kind_on(*streams, kind) == on {
                continue;
            }
            if let Err(error) = self.switch(kind, on, until) {
                return Err(StreamFailure {
                    failed: kinds,
                    error,
                });
            }
            set_kind(&mut streams, kind, on);
        }
        Ok(())
    }

    /// the box never switches a stream off by itself, so this just asks again
    fn restart_stream(&self, kind: InputKind) -> Result<()> {
        let streams = lock(&self.shared.streams);
        if kind_on(*streams, kind) {
            self.switch(kind, true, Instant::now() + crate::backend::WRITE_DEADLINE)
        } else {
            Ok(())
        }
    }

    fn type_piece(&self, _piece: &[u8], _typing: &mut Typing) -> Result<()> {
        Err(Error::Unsupported {
            action: Action::TypeText,
            family: Family::Ferrum,
            reason: NO_TYPING.into(),
        })
    }

    fn release_all(&self, held: &[Usage]) -> Result<()> {
        let mut first = Ok(());
        for &usage in held {
            let released = self.send(usage.release());
            if first.is_ok() {
                first = released;
            }
        }
        first
    }

    fn unlock_all(&self, slots: &[LockSlot]) -> Result<()> {
        let mut first = Ok(());
        for &slot in slots {
            let unlocked = self.send(Command::Lock {
                slot,
                keep: UNLOCKED,
            });
            if matches!(unlocked, Err(Error::Disconnected)) {
                return unlocked;
            }
            if first.is_ok() {
                first = unlocked;
            }
        }
        first
    }

    fn shutdown(&self, deadline: Duration) {
        let until = Instant::now() + deadline;
        let streams = *lock(&self.shared.streams);
        for kind in KINDS {
            // they survive the port closing, so left on they'd keep sending to whoever opens it next
            if kind_on(streams, kind) {
                let _ = self.switch(kind, false, until);
            }
        }
        self.shared.stop.store(true, Ordering::Release);
        if let Some(poller) = lock(&self.poller).take() {
            // the poller naps in short slices and its requests are bounded, so this ends within the deadline
            while !poller.is_finished() && Instant::now() < until {
                thread::sleep(Duration::from_millis(5));
            }
            if poller.is_finished() {
                let _ = poller.join();
            }
        }
        self.link
            .shutdown(until.saturating_duration_since(Instant::now()));
    }

    fn extension(&self, request: Extension) -> Result<ExtensionReply> {
        let Extension::FerrumCatchXy {
            window,
            with_software,
        } = request
        else {
            return Err(Error::Unsupported {
                action: request.action(),
                family: Family::Ferrum,
                reason: "a Ferrum doesn't have this".into(),
            });
        };
        // always both arguments in this exact form, the reply waits on the echo of the line
        let command = format!("km.catch_xy({}, {with_software})", window.as_millis());
        let reply = self
            .link
            .request(line(&command), Some(frames::key(command.as_bytes())))?
            .ok_or(Error::Disconnected)?;
        frames::result(&reply, true)
            .and_then(xy)
            .map(|(x, y)| ExtensionReply::Xy(x, y))
            .ok_or_else(|| Error::Rejected {
                action: request.action(),
                detail: "it answered something that isn't an (x, y)".into(),
            })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// `(500, -500)` for `km.catch_xy`
pub(crate) fn xy(result: &[u8]) -> Option<(i32, i32)> {
    let text = std::str::from_utf8(result).ok()?.trim();
    let (x, y) = text.strip_prefix('(')?.strip_suffix(')')?.split_once(',')?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?))
}

#[cfg(any(test, feature = "mock"))]
pub(crate) mod mock;
#[cfg(test)]
mod tests;
