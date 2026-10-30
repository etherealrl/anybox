use std::any::Any;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::backend::{
    Backend, ChordKind, Command, InputKind, InputKinds, LOCKED, LockSlot, Sink, StreamFailure,
    Typing, UNLOCKED, Usage, WRITE_DEADLINE,
};
use crate::capability::{
    Action, Axis, Button, Capabilities, Capability, Family, Firmware, Info, MakcuAction, Quirk,
    Snapshot, StableId,
};
use crate::error::{Error, Result};
use crate::extension::{Extension, ExtensionReply, Interpolation};
use crate::key::Key;
use crate::transport::link::{Link, Target};
use crate::transport::probe::Probe;
use crate::transport::{Candidate, System, is_ch343};

mod frames;
mod poller;

use frames::{MakcuFrames, frame, parts, refused};
pub(crate) use poller::Poller;

/// the two rates a V4 box can run at. the button next to USB1 switches between them and the box remembers it
pub(crate) const BAUDS: [u32; 2] = [4_000_000, 115_200];
/// the official build when this backend was written, nothing older was checked against it
pub(crate) const MIN_BUILD: u32 = 4085;
/// the vendor reads the build as major * 1000 + minor, so this would be a V5
const NEXT_MAJOR: u32 = 5000;
/// V4.097 is the first official build that sends `move_now` in the next report, older ones can smooth it
const MOVE_NOW_BUILD: u32 = 4097;
/// the first official build with AUTORELEASE
const AUTORELEASE_BUILD: u32 = 4108;
/// the reference code's timeout for a text reply
const TEXT_WAIT: Duration = Duration::from_millis(500);
const BINARY_WAIT: Duration = Duration::from_millis(250);
/// how long an empty KEY_STRING waits for the box to refuse it, which it does while it types. the slowest refusal
/// measured came 15 ms after the write, so silence for this long means the box is done
const TYPING_WINDOW: Duration = Duration::from_millis(40);
/// the pause between two of those checks while the box is still typing
const TYPING_PAUSE: Duration = Duration::from_millis(10);
const NO_KEYBOARD: &str = "the device behind the box has no keyboard interface right now";
const TRIES: usize = 2;
/// CLICK takes a hold of up to 5000 ms, that's documented. KEY_PRESS has no documented limit, so it gets the same
/// one
const MAX_HOLD: Duration = Duration::from_millis(5000);

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn not_found(path: &Path) -> Error {
    Error::NotFound {
        tried: vec![path.display().to_string()],
    }
}

/// opens a MAKCU V4 box on one port, or returns `NotFound` when what's there isn't one
pub(crate) fn open(candidate: &Candidate, sink: Sink) -> Result<(Box<dyn Backend>, Snapshot)> {
    let (backend, snapshot, poller) =
        attach(Arc::new(crate::transport::os::OsSystem), candidate, sink)?;
    *lock(&backend.poller) = Some(poller.spawn()?);
    Ok((Box::new(backend), snapshot))
}

/// does everything `open` does except starting the poller, so tests can run its ticks by hand
pub(crate) fn attach(
    system: Arc<dyn System>,
    candidate: &Candidate,
    sink: Sink,
) -> Result<(MakcuBackend, Snapshot, Poller)> {
    let path = candidate.path.as_path();
    let usb = candidate.usb.clone();
    // a YSC answers km.version() just like a MAKCU, so we don't even ask a port whose USB chip says it's
    // something else
    if usb.as_ref().is_some_and(|usb| !is_ch343(usb)) {
        return Err(not_found(path));
    }
    let id = usb
        .as_ref()
        .and_then(|usb| usb.serial.clone())
        .map(StableId::UsbSerial);
    let stop = AtomicBool::new(false);
    for (at, &baud) in BAUDS.iter().enumerate() {
        let port = match system.open(path, baud) {
            Ok(port) => port,
            Err(Error::NotFound { .. }) => return Err(not_found(path)),
            Err(error) => return Err(error),
        };
        let probe = Probe::new(&*port, &stop);
        if !identify(&probe) {
            continue;
        }
        let snapshot = describe(&probe, id.clone())?;
        // an earlier session that crashed can leave a stream on, and it would start sending before anyone asked
        for kind in [frames::MOUSE, frames::KEYBOARD] {
            port.write_all(&frame(frames::INPUT_STREAM, &[kind, 0]))
                .map_err(|_| Error::Disconnected)?;
        }
        let mut bauds = BAUDS.to_vec();
        bauds.rotate_left(at);
        let target = Target {
            family: Family::MakcuV4,
            path: path.to_path_buf(),
            usb,
            id: id.clone(),
            bauds,
        };
        return start(system, port, target, snapshot, sink);
    }
    Err(not_found(path))
}

fn start(
    system: Arc<dyn System>,
    port: Box<dyn crate::transport::Port>,
    target: Target,
    snapshot: Snapshot,
    sink: Sink,
) -> Result<(MakcuBackend, Snapshot, Poller)> {
    let shared = Arc::new(Shared {
        snapshot: RwLock::new(snapshot.clone()),
        expect: Mutex::new(Expect::default()),
        streams: Mutex::new(()),
        buttons: Mutex::new(()),
        key_masks: Mutex::new(HashMap::new()),
        last: Arc::default(),
        stop: AtomicBool::new(false),
        pieces: AtomicU64::new(0),
        key_writes: AtomicU64::new(0),
        checked: AtomicU64::new(u64::MAX),
        autorelease: Mutex::new(None),
    });
    let id = target.id.clone();
    let reprobe = {
        let shared = shared.clone();
        Box::new(move |probe: &Probe<'_>| {
            if !identify(probe) {
                return Err(Error::Disconnected);
            }
            let snapshot = describe(probe, id.clone())?;
            if let Some(ms) = *lock(&shared.autorelease) {
                // the echo is read here, a request after the reconnect would take it for its own answer otherwise.
                // a refusal leaves it off on the box, which `autorelease()` reads from the box anyway
                probe.ask(
                    &frame(frames::AUTORELEASE, &ms.to_le_bytes()),
                    &mut MakcuFrames::probe(),
                    |frame| parts(frame).is_some_and(|(cmd, _)| cmd == frames::AUTORELEASE),
                    BINARY_WAIT,
                )?;
            }
            // the core turns streams back on and lets go of what it held once it hears Reconnected
            shared.last.forget_locks();
            lock(&shared.key_masks).clear();
            let mut expect = lock(&shared.expect);
            *expect = Expect {
                generation: expect.generation + 1,
                ..Expect::default()
            };
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
        Box::new(MakcuFrames::binary(shared.last.clone())),
        target,
        reprobe,
        sink.clone(),
    )?);
    let poller = Poller::new(shared.clone(), link.clone(), sink);
    let backend = MakcuBackend {
        shared,
        link,
        poller: Mutex::new(None),
    };
    Ok((backend, snapshot, poller))
}

/// true when `km.version()` gets a line back that is exactly `km.MAKCU`. the line ending at the start of the query
/// closes whatever an earlier probe left half written in the box's text parser
fn identify(probe: &Probe<'_>) -> bool {
    (0..TRIES).any(|_| {
        matches!(
            probe.ask(
                frames::VERSION_QUERY,
                &mut MakcuFrames::probe(),
                |frame| frames::says_makcu(frame),
                TEXT_WAIT,
            ),
            Ok(Some(_))
        )
    })
}

/// a GET over the bare port, tried twice. a malformed reply counts as no reply
fn get(probe: &Probe<'_>, command: u8, len: usize) -> Option<Vec<u8>> {
    (0..TRIES).find_map(|_| {
        let reply = probe.ask(
            &frame(command, &[]),
            &mut MakcuFrames::probe(),
            |frame| {
                parts(frame).is_some_and(|(cmd, payload)| {
                    cmd == command && (payload.len() == len || refused(payload))
                })
            },
            BINARY_WAIT,
        );
        let frame = reply.ok().flatten()?;
        let (_, payload) = parts(&frame)?;
        (payload.len() == len).then(|| payload.to_vec())
    })
}

/// the box already said it's a MAKCU, so from here on it's never NotFound
fn describe(probe: &Probe<'_>, id: Option<StableId>) -> Result<Snapshot> {
    let Some(build) = get(probe, frames::FIRMWARE_VERSION, 4) else {
        return Err(Error::FirmwareTooOld {
            family: Family::MakcuV4,
            found: Firmware::Text("V3 or older".into()),
            needed: Firmware::Build(MIN_BUILD),
        });
    };
    let build = u32::from_le_bytes([build[0], build[1], build[2], build[3]]);
    if build < MIN_BUILD {
        return Err(Error::FirmwareTooOld {
            family: Family::MakcuV4,
            found: Firmware::Build(build),
            needed: Firmware::Build(MIN_BUILD),
        });
    }
    if build >= NEXT_MAJOR {
        return Err(Error::LibraryTooOld {
            family: Family::MakcuV4,
            found: Firmware::Build(build),
        });
    }
    let kinds = get(probe, frames::DEVICE, 1).ok_or(Error::Timeout {
        action: Action::Connect,
    })?[0];
    Ok(snapshot(build, kinds, id))
}

pub(crate) fn snapshot(build: u32, kinds: u8, id: Option<StableId>) -> Snapshot {
    let mut info = Info::new(Family::MakcuV4);
    info.model = Some("MAKCU".into());
    info.firmware = Some(Firmware::Build(build));
    info.id = id;
    let mouse = kinds & frames::MOUSE_ROUTED != 0;
    let keyboard = kinds & frames::KEYBOARD_ROUTED != 0;
    let capabilities = if mouse {
        let move_now = if build >= MOVE_NOW_BUILD {
            Capability::native()
        } else {
            Capability::unsupported(
                "older builds can still smooth move_now, so update the box to V4.097 or later",
            )
        };
        Capabilities::new(vec![
            Button::Left,
            Button::Right,
            Button::Middle,
            Button::Side1,
            Button::Side2,
        ])
        .set(Action::Move, Capability::native())
        .set(Action::MoveNow, move_now)
        .set(Action::Press, Capability::native())
        .set(Action::Release, Capability::native())
        .set(Action::Wheel, Capability::native())
        .set(
            Action::Click,
            Capability::native()
                .with(Quirk::BoxClickTiming {
                    min: Duration::from_millis(35),
                    max: Duration::from_millis(75),
                })
                .with(Quirk::MaxHold(MAX_HOLD)),
        )
        .set(Action::ButtonEvents, Capability::native())
        .set(
            Action::Makcu(MakcuAction::Interpolation),
            Capability::native(),
        )
        .set(Action::Makcu(MakcuAction::Moving), Capability::native())
        .set(
            Action::Makcu(MakcuAction::Autorelease),
            if build >= AUTORELEASE_BUILD {
                Capability::native()
            } else {
                Capability::unsupported("autorelease needs V4.108 or later, update the box")
            },
        )
    } else {
        let none = Capability::unsupported(
            "the box doesn't route a mouse right now, plug one in and it shows up by itself",
        );
        [
            Action::Move,
            Action::MoveNow,
            Action::Press,
            Action::Release,
            Action::Wheel,
            Action::Click,
            Action::ButtonEvents,
            Action::Makcu(MakcuAction::Interpolation),
            Action::Makcu(MakcuAction::Moving),
            Action::Makcu(MakcuAction::Autorelease),
        ]
        .into_iter()
        .fold(Capabilities::new(Vec::new()), |capabilities, action| {
            capabilities.set(action, none.clone())
        })
    };
    let capabilities = if keyboard {
        capabilities
            .set(Action::KeyPress, Capability::native())
            .set(Action::KeyRelease, Capability::native())
            // there's no documented longest hold for a tap, so it borrows the click's
            .set(
                Action::KeyTap,
                Capability::native().with(Quirk::MaxHold(MAX_HOLD)),
            )
            .set(Action::KeyEvents, Capability::native())
            // measured on V4.094: the same keys with Caps Lock on and off, and a held key the text types or a held
            // Left Shift the text needs gets let go
            .set(
                Action::TypeText,
                Capability::native()
                    .with(Quirk::TypedTextIgnoresCapsLock)
                    .with(Quirk::TypingReleasesHeldKeys),
            )
            // measured on V4.094, KEY_MULTI_PRESS holds its keys 8 to 12 ms
            .set(
                Action::KeyChord,
                Capability::native().with(Quirk::BoxClickTiming {
                    min: Duration::from_millis(8),
                    max: Duration::from_millis(12),
                }),
            )
    } else {
        let none = Capability::unsupported(NO_KEYBOARD);
        capabilities
            .set(Action::KeyPress, none.clone())
            .set(Action::KeyRelease, none.clone())
            .set(Action::KeyTap, none.clone())
            .set(Action::KeyEvents, none.clone())
            .set(Action::TypeText, none.clone())
            .set(Action::KeyChord, none)
    };
    let capabilities = if mouse {
        capabilities
            .with_axes(vec![Axis::X, Axis::Y, Axis::Wheel])
            .set(Action::LockButton, Capability::native())
            .set(Action::LockAxis, Capability::native())
    } else {
        let none = Capability::unsupported(
            "the box doesn't route a mouse right now, plug one in and it shows up by itself",
        );
        capabilities
            .set(Action::LockButton, none.clone())
            .set(Action::LockAxis, none)
    };
    let keys = if keyboard {
        Capability::native()
    } else {
        Capability::unsupported("the device behind the box has no keyboard interface right now")
    };
    let capabilities = capabilities
        .set(Action::LockKey, keys.clone())
        // its key mask, which cuts a key that's already held. measured by K4 on V4.094, not checked here
        .set(Action::ForceReleaseKey, keys)
        .set(
            Action::ScaleAxis,
            Capability::unsupported("MAKCU can only block an axis, not scale it"),
        );
    let no_media = Capability::unsupported("MAKCU V4 has no media keys");
    let capabilities = capabilities
        .set(Action::MediaPress, no_media.clone())
        .set(Action::MediaRelease, no_media.clone())
        .set(Action::MediaTap, no_media.clone())
        .set(Action::MediaEvents, no_media)
        .set(
            Action::DiscardPendingMoves,
            Capability::unsupported("MAKCU can't drop moves it already took"),
        )
        .set(
            Action::Pan,
            Capability::unsupported("MAKCU V4 has no horizontal scroll, km.pan only exists on V3"),
        )
        .set(
            Action::MotionEvents,
            Capability::unsupported("MAKCU doesn't report mouse motion"),
        );
    Snapshot::new(info, capabilities)
}

/// the two input streams a MAKCU has
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stream {
    Mouse,
    Keyboard,
}

impl Stream {
    pub(crate) fn kind(self) -> InputKind {
        match self {
            Self::Mouse => InputKind::Buttons,
            Self::Keyboard => InputKind::Keys,
        }
    }

    fn of(kind: InputKind) -> Option<Self> {
        match kind {
            InputKind::Buttons => Some(Self::Mouse),
            InputKind::Keys => Some(Self::Keyboard),
            InputKind::Motion | InputKind::Media => None,
        }
    }
}

/// one input stream, what the core asked for and whether the box said it's on
#[derive(Clone, Copy, Default)]
pub(crate) struct StreamState {
    pub(crate) wanted: bool,
    pub(crate) confirmed: bool,
}

impl Expect {
    pub(crate) fn stream(&mut self, kind: Stream) -> &mut StreamState {
        match kind {
            Stream::Mouse => &mut self.mouse,
            Stream::Keyboard => &mut self.keys,
        }
    }
}

pub(crate) fn kind_byte(kind: Stream) -> u8 {
    match kind {
        Stream::Mouse => frames::MOUSE,
        Stream::Keyboard => frames::KEYBOARD,
    }
}

/// what the box should be holding and streaming, as far as this backend knows
#[derive(Default)]
pub(crate) struct Expect {
    /// presses that were written and not released yet, true once the box shows it holds that one. a press the
    /// box refused or hasn't gotten to yet never shows up, so it never counts as lost
    held: HashMap<Button, bool>,
    mouse: StreamState,
    keys: StreamState,
    /// goes up on every reprobe, so a poller tick that spans a reconnect throws away what it saw
    generation: u64,
}

/// state the backend, the poller and the reprobe share
pub(crate) struct Shared {
    snapshot: RwLock<Snapshot>,
    expect: Mutex<Expect>,
    /// only one stream switch at a time, the core's or the poller's retry, so a retry can never land after an off
    streams: Mutex<()>,
    /// only one button write at a time, the backend's or the poller's release retry
    buttons: Mutex<()>,
    /// the keys masked through Anybox, each with the action it's masked for (`LockKey` or `ForceReleaseKey`, a key
    /// is never both). masks don't show in INJECT_SNAPSHOT, so the poller sends them again every now and then
    key_masks: Mutex<HashMap<u8, Action>>,
    last: Arc<frames::LastSent>,
    stop: AtomicBool,
    /// goes up under `buttons` with every piece of text, so a poller tick that read the snapshot while one was
    /// typing doesn't let go of the key it saw being typed
    pieces: AtomicU64,
    /// every key SET anybox writes other than a text's own, because a tap that's still running keeps the box busy
    key_writes: AtomicU64,
    /// `key_writes` when the box was last seen done typing, so a piece only checks it's idle when something went
    /// out since
    checked: AtomicU64,
    /// what was set through anybox, in ms. the box forgets it when it loses power, so it goes out again after a
    /// reconnect and a session reset
    autorelease: Mutex<Option<u32>>,
}

impl Shared {
    fn snapshot(&self) -> Snapshot {
        self.snapshot
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

pub(crate) struct MakcuBackend {
    shared: Arc<Shared>,
    link: Arc<Link>,
    poller: Mutex<Option<JoinHandle<()>>>,
}

/// the hold the box gets, in whole ms rounded up and never 0, because 0 tells the box to pick a random one. the
/// core already caps it at `MAX_HOLD`
fn hold_ms(hold: Duration) -> u32 {
    let ms = hold.as_micros().div_ceil(1000).max(1);
    u32::try_from(ms).unwrap_or(u32::MAX)
}

impl MakcuBackend {
    /// checks the command against what the box can do right now, because a reconnect can change that after the
    /// core already said yes. releases always go out
    fn check(&self, command: Command) -> Result<()> {
        let button = match command {
            Command::Release(_)
            | Command::KeyRelease(_)
            | Command::MediaRelease(_)
            | Command::KeyChord {
                kind: ChordKind::Release,
                ..
            } => {
                return Ok(());
            }
            Command::Lock { keep: UNLOCKED, .. } | Command::KeyForce { force: false, .. } => {
                return Ok(());
            }
            Command::KeyForce { force: true, .. } => None,
            Command::Lock {
                slot: LockSlot::Button(button),
                ..
            } => Some(button),
            Command::Lock { .. } => None,
            Command::Press(button) | Command::Click { button, .. } => Some(button),
            Command::Move(..)
            | Command::MoveNow(..)
            | Command::Wheel(_)
            | Command::Pan(_)
            | Command::DiscardPendingMoves
            | Command::KeyPress(_)
            | Command::KeyTap { .. }
            | Command::MediaPress(_)
            | Command::MediaTap { .. }
            | Command::KeyChord { .. } => None,
        };
        let action = command.action();
        let snapshot = self.shared.snapshot();
        let capability = snapshot.capabilities.get(action);
        let supported = capability.is_supported()
            && button.is_none_or(|button| snapshot.capabilities.buttons().contains(&button));
        if supported {
            Ok(())
        } else {
            Err(Error::Unsupported {
                action,
                family: Family::MakcuV4,
                reason: "the box changed during this call, check its snapshot again".into(),
            })
        }
    }

    fn encode(command: Command) -> Option<Vec<u8>> {
        let button = |button| frames::button_id(button).map(|id| frames::LEFT + id);
        Some(match command {
            Command::Move(dx, dy) => frame(frames::MOVE, &xy(dx, dy)),
            Command::MoveNow(dx, dy) => frame(frames::MOVE_NOW, &xy(dx, dy)),
            Command::Press(b) => frame(button(b)?, &[1]),
            Command::Release(b) => frame(button(b)?, &[0]),
            Command::Wheel(notches) => frame(frames::WHEEL, &notches.to_le_bytes()),
            Command::Click { button, hold } => {
                let hold = hold.map_or(0, hold_ms);
                let mut payload = vec![frames::button_id(button)? + 1, 1];
                payload.extend_from_slice(&u16::try_from(hold).unwrap_or(u16::MAX).to_le_bytes());
                frame(frames::CLICK, &payload)
            }
            Command::KeyPress(key) => frame(frames::KEY_DOWN, &[key.0]),
            Command::KeyRelease(key) => frame(frames::KEY_UP, &[key.0]),
            // with no hold the box picks one itself, same as CLICK
            Command::KeyTap { key, hold: None } => frame(frames::KEY_PRESS, &[key.0]),
            Command::KeyTap {
                key,
                hold: Some(hold),
            } => {
                let mut payload = vec![key.0];
                payload.extend_from_slice(&hold_ms(hold).to_le_bytes());
                frame(frames::KEY_PRESS, &payload)
            }
            Command::Lock {
                slot: LockSlot::Key(key),
                keep,
            } if keep == LOCKED || keep == UNLOCKED => {
                frame(frames::KEY_MASK, &[key.0, u8::from(keep == LOCKED)])
            }
            // the same mask a lock writes, which cuts a key that's already held (K4, V4.094)
            Command::KeyForce { key, force } => frame(frames::KEY_MASK, &[key.0, u8::from(force)]),
            Command::Lock { slot, keep } if keep == LOCKED || keep == UNLOCKED => frame(
                frames::LOCK,
                &[frames::lock_target(slot)?, u8::from(keep == LOCKED)],
            ),
            // a scale, which the box can't do because it only blocks or passes
            Command::Lock { .. } => return None,
            Command::KeyChord { kind, keys } => frame(
                match kind {
                    ChordKind::Press => frames::KEY_MULTI_DOWN,
                    ChordKind::Release => frames::KEY_MULTI_UP,
                    ChordKind::Tap => frames::KEY_MULTI_PRESS,
                },
                &keys.as_slice().iter().map(|key| key.0).collect::<Vec<_>>(),
            ),
            Command::DiscardPendingMoves
            | Command::Pan(_)
            | Command::MediaPress(_)
            | Command::MediaRelease(_)
            | Command::MediaTap { .. } => return None,
        })
    }

    /// an empty KEY_STRING, which the box refuses while it types and takes without a word once it's done. the
    /// first check of a text ties the rest of it to the opening it went out on
    fn typing(&self, typing: &mut Typing, until: Instant) -> Result<bool> {
        let (reply, generation) = self.link.request_or_silence(
            frame(frames::KEY_STRING, &[]),
            u32::from(frames::KEY_STRING),
            TYPING_WINDOW,
            typing.generation,
            until,
        )?;
        typing.generation.get_or_insert(generation);
        Ok(reply
            .as_deref()
            .and_then(parts)
            .is_some_and(|(_, payload)| refused(payload)))
    }

    /// the piece itself, see `type_piece`
    fn piece(&self, piece: &[u8], typing: &mut Typing) -> Result<()> {
        let until = Instant::now() + WRITE_DEADLINE;
        if let Some(error) = self.typing_unsupported() {
            return Err(error);
        }
        if self.shared.key_writes.load(Ordering::Acquire)
            != self.shared.checked.load(Ordering::Acquire)
        {
            while self.typing(typing, until)? {
                thread::sleep(TYPING_PAUSE);
                if Instant::now() >= until {
                    return Err(Error::Rejected {
                        action: Action::TypeText,
                        detail: "it's still busy with a key it taps, try again once that's done"
                            .into(),
                    });
                }
            }
        }
        let (_, generation) = self.link.request_on(
            frame(frames::KEY_STRING, piece),
            None,
            typing.generation,
            until,
        )?;
        typing.generation.get_or_insert(generation);
        typing.sent = true;
        let mut started = false;
        while self.typing(typing, until)? {
            started = true;
            if let Some(error) = self.typing_unsupported() {
                return Err(error);
            }
            thread::sleep(TYPING_PAUSE);
            if Instant::now() >= until {
                self.link.take_down(generation);
                return Err(Error::Disconnected);
            }
        }
        // two characters take the box at least 150 ms, so the first check always lands while it types one
        if !started {
            return Err(Error::Rejected {
                action: Action::TypeText,
                detail: "it didn't type it".into(),
            });
        }
        self.shared.checked.store(
            self.shared.key_writes.load(Ordering::Acquire),
            Ordering::Release,
        );
        // the box let go of what it typed, a held key of the same usage and a held Left Shift included
        for (key, shift) in piece.iter().filter_map(|&byte| Key::for_ascii(byte)) {
            self.shared.last.set_key(key.0, false);
            if shift {
                self.shared.last.set_key(Key::LEFT_SHIFT.0, false);
            }
        }
        Ok(())
    }

    fn typing_unsupported(&self) -> Option<Error> {
        let snapshot = self.shared.snapshot();
        let capability = snapshot.capabilities.get(Action::TypeText);
        (!capability.is_supported()).then(|| Error::Unsupported {
            action: Action::TypeText,
            family: Family::MakcuV4,
            reason: capability.reason(),
        })
    }

    fn stream(&self, kind: Stream, on: bool) -> Result<()> {
        let _switching = lock(&self.shared.streams);
        *lock(&self.shared.expect).stream(kind) = StreamState {
            wanted: on,
            confirmed: false,
        };
        let byte = kind_byte(kind);
        let action = match kind {
            Stream::Mouse => Action::ButtonEvents,
            Stream::Keyboard => Action::KeyEvents,
        };
        let name = match kind {
            Stream::Mouse => "button",
            Stream::Keyboard => "key",
        };
        // a refusal doesn't say which kind it was for, so this is what names it
        self.shared.last.set_stream(byte);
        let state = u8::from(on);
        self.link
            .request(frame(frames::INPUT_STREAM, &[byte, state]), None)?;
        let reply = self.link.request(
            frame(frames::INPUT_STREAM, &[byte]),
            Some(u32::from(frames::INPUT_STREAM)),
        )?;
        let payload = reply.as_deref().and_then(parts).map(|(_, payload)| payload);
        match payload {
            Some([got]) if *got == state => {
                lock(&self.shared.expect).stream(kind).confirmed = on;
                Ok(())
            }
            Some(payload) if refused(payload) => Err(Error::Rejected {
                action,
                detail: format!("it won't switch the {name} stream right now"),
            }),
            _ => Err(Error::Rejected {
                action,
                detail: format!(
                    "the {name} stream still isn't {} after asking",
                    if on { "on" } else { "off" }
                ),
            }),
        }
    }
}

fn xy(dx: i16, dy: i16) -> [u8; 4] {
    let [x0, x1] = dx.to_le_bytes();
    let [y0, y1] = dy.to_le_bytes();
    [x0, x1, y0, y1]
}

impl Backend for MakcuBackend {
    fn send(&self, command: Command) -> Result<()> {
        self.check(command)?;
        let Some(bytes) = Self::encode(command) else {
            let action = command.action();
            return Err(Error::Unsupported {
                action,
                family: Family::MakcuV4,
                reason: self.shared.snapshot().capabilities.get(action).reason(),
            });
        };
        // held while the state is recorded and written, so the poller's release retry can't land after a newer press
        let _buttons = lock(&self.shared.buttons);
        if let Command::Press(button) | Command::Release(button) | Command::Click { button, .. } =
            command
            && let Some(id) = frames::button_id(button)
        {
            // a click counts as a press here, so the retry never cuts the box's own hold short
            let pressed = !matches!(command, Command::Release(_));
            self.shared.last.set(id, u8::from(pressed));
        }
        if matches!(
            command,
            Command::KeyPress(_)
                | Command::KeyRelease(_)
                | Command::KeyTap { .. }
                | Command::KeyChord { .. }
                | Command::Lock {
                    slot: LockSlot::Key(_),
                    ..
                }
                | Command::KeyForce { .. }
        ) {
            self.shared.key_writes.fetch_add(1, Ordering::AcqRel);
        }
        match command {
            // a tap counts as a press, like a click
            Command::KeyPress(key) | Command::KeyTap { key, .. } => {
                self.shared.last.set_key(key.0, true)
            }
            Command::KeyRelease(key) => self.shared.last.set_key(key.0, false),
            Command::KeyChord { kind, keys } => {
                for key in keys.as_slice() {
                    self.shared.last.set_key(key.0, kind != ChordKind::Release);
                }
            }
            Command::Lock { slot, keep } => {
                let state = if keep == UNLOCKED {
                    frames::LastSent::UNLOCKED
                } else {
                    frames::LastSent::LOCKED
                };
                if let Some(target) = frames::lock_target(slot) {
                    self.shared.last.set_lock(target, state);
                }
                if let LockSlot::Key(key) = slot {
                    self.shared.last.name_mask(false);
                    let mut masks = lock(&self.shared.key_masks);
                    if keep == UNLOCKED {
                        masks.remove(&key.0);
                    } else {
                        masks.insert(key.0, Action::LockKey);
                    }
                }
            }
            Command::KeyForce { key, force } => {
                self.shared.last.name_mask(true);
                let mut masks = lock(&self.shared.key_masks);
                if force {
                    masks.insert(key.0, Action::ForceReleaseKey);
                } else {
                    masks.remove(&key.0);
                }
            }
            _ => {}
        }
        // a release comes off the record before the write, so the poller never takes one that's still on its way
        // for a lost hold. a press only goes on after the write, so a write that failed never counts
        let released = match command {
            Command::Release(button) => {
                Some((button, lock(&self.shared.expect).held.remove(&button)))
            }
            _ => None,
        };
        let sent = self.link.request(bytes, None);
        match (command, &sent, released) {
            (Command::Press(button), Ok(_), _) => {
                lock(&self.shared.expect)
                    .held
                    .entry(button)
                    .or_insert(false);
            }
            (_, Err(_), Some((button, Some(seen)))) => {
                lock(&self.shared.expect).held.insert(button, seen);
            }
            _ => {}
        }
        sent.map(|_| ())
    }

    /// checks the box is idle when a key went out since it was last seen done, writes the piece, then checks with an
    /// empty KEY_STRING until the box stops refusing it. all of it under `buttons`, so the poller writes no key in
    /// the middle, and within `WRITE_DEADLINE`. a piece is never sent twice, it could get typed twice
    fn type_piece(&self, piece: &[u8], typing: &mut Typing) -> Result<()> {
        let _buttons = lock(&self.shared.buttons);
        // bumped on both ends, so a poller that read the count and then a snapshot during the piece sees it moved
        self.shared.pieces.fetch_add(1, Ordering::AcqRel);
        let typed = self.piece(piece, typing);
        self.shared.pieces.fetch_add(1, Ordering::AcqRel);
        typed
    }

    /// switches each kind that isn't already where it's asked to be. no new kind starts once `WRITE_DEADLINE` has
    /// passed, and a kind that already started gets its SET and its confirming GET, so the call takes at most two
    /// requests per kind. an ON that's left over stays wanted and the poller asks again. an OFF that's left over
    /// also keeps the stream wanted, so the next call or shutdown still switches it off
    fn set_streams(&self, kinds: InputKinds) -> std::result::Result<(), StreamFailure> {
        let until = Instant::now() + WRITE_DEADLINE;
        let mut failed = InputKinds::default();
        let mut first = None;
        for (kind, on) in [
            (Stream::Mouse, kinds.buttons),
            (Stream::Keyboard, kinds.keys),
        ] {
            let current = *lock(&self.shared.expect).stream(kind);
            if current.wanted == on && (current.confirmed || !on) {
                continue;
            }
            if Instant::now() >= until {
                if on {
                    *lock(&self.shared.expect).stream(kind) = StreamState {
                        wanted: true,
                        confirmed: false,
                    };
                }
                continue;
            }
            match self.stream(kind, on) {
                Ok(()) => {}
                Err(Error::Disconnected) => {
                    return Err(StreamFailure {
                        failed: kinds,
                        error: Error::Disconnected,
                    });
                }
                // the box may have nothing left to stream keys from, and off is all we wanted anyway
                Err(_) if kind == Stream::Keyboard && !on => {}
                Err(error) => {
                    let one = kind.kind().kinds();
                    failed.buttons |= one.buttons;
                    failed.keys |= one.keys;
                    first.get_or_insert(error);
                }
            }
        }
        match first {
            None => Ok(()),
            Some(error) => Err(StreamFailure { failed, error }),
        }
    }

    fn restart_stream(&self, kind: InputKind) -> Result<()> {
        let Some(kind) = Stream::of(kind) else {
            return Ok(());
        };
        if lock(&self.shared.expect).stream(kind).wanted {
            self.stream(kind, true)
        } else {
            Ok(())
        }
    }

    /// one release for each, never KEY_INIT, because that also clears key masks and remaps
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
        // an unlock the box refused, or one that never went out after an earlier one ran out of time, would leave a
        // physical button dead after the app is gone, so we take one last look. it goes before the streams, so a
        // slow box spends the deadline on this
        let still_locked = self
            .link
            .request_until(
                frame(frames::INJECT_SNAPSHOT, &[]),
                Some(u32::from(frames::INJECT_SNAPSHOT)),
                until,
            )
            .ok()
            .flatten()
            .and_then(|reply| parts(&reply).map(|(_, payload)| frames::locked_targets(payload)))
            .unwrap_or(0);
        for target in 0..14u8 {
            if still_locked & (1 << target) != 0
                && self.shared.last.lock(target) != frames::LastSent::NEVER
            {
                let _ = self
                    .link
                    .request_until(frame(frames::LOCK, &[target, 0]), None, until);
            }
        }
        // masks don't show in INJECT_SNAPSHOT, so any key left here never got its unlock sent
        let masks: Vec<u8> = lock(&self.shared.key_masks)
            .drain()
            .map(|(key, _)| key)
            .collect();
        for key in masks {
            let _ = self
                .link
                .request_until(frame(frames::KEY_MASK, &[key, 0]), None, until);
        }
        let wanted = {
            let mut expect = lock(&self.shared.expect);
            [Stream::Mouse, Stream::Keyboard].map(|kind| (kind, expect.stream(kind).wanted))
        };
        for (kind, wanted) in wanted {
            if wanted {
                // if it's left on, the box keeps streaming to whoever opens the port next
                let _ = self.link.request_until(
                    frame(frames::INPUT_STREAM, &[kind_byte(kind), 0]),
                    None,
                    until,
                );
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
        let action = request.action();
        let rejected = |detail: &str| Error::Rejected {
            action,
            detail: detail.into(),
        };
        match request {
            Extension::MakcuInterpolation => self.spread().map(ExtensionReply::Interpolation),
            Extension::MakcuSetInterpolation(value) => {
                let byte = match value {
                    Interpolation::Auto => 0xFF,
                    Interpolation::Percent(n) => n,
                };
                let echo = self.get(frames::INTERPOLATE, &[byte])?;
                // AUTO echoes FF, the same byte as a refusal, so the snapshot settles it either way
                if refused(&echo) && byte != 0xFF {
                    return Err(rejected("it refused the interpolation"));
                }
                if self.spread()? != value {
                    return Err(rejected("it didn't take the interpolation"));
                }
                Ok(ExtensionReply::Done)
            }
            Extension::MakcuMoving => match self.get(frames::MOVING, &[])?[..] {
                [moving @ 0..=1] => Ok(ExtensionReply::Moving(moving == 1)),
                _ => Err(rejected("it refused to say whether it's still moving")),
            },
            Extension::MakcuAutorelease => match self.get(frames::AUTORELEASE, &[])?[..] {
                [a, b, c, d] => Ok(ExtensionReply::Autorelease(after(u32::from_le_bytes([
                    a, b, c, d,
                ])))),
                _ => Err(rejected("it refused to say its autorelease")),
            },
            Extension::MakcuSetAutorelease(wanted) => {
                let ms = wanted.map_or(0, |after| {
                    u32::try_from(after.as_millis()).expect("the core caps it at a minute")
                });
                // held over the exchange, otherwise a session reset could put the old value back between this
                // write and the remembered one changing
                let mut remembered = lock(&self.shared.autorelease);
                let echo = self.get(frames::AUTORELEASE, &ms.to_le_bytes())?;
                if echo != ms.to_le_bytes() {
                    return Err(rejected("it refused the autorelease"));
                }
                *remembered = wanted.map(|_| ms);
                Ok(ExtensionReply::Done)
            }
            _ => Err(Error::Unsupported {
                action,
                family: Family::MakcuV4,
                reason: "a MAKCU doesn't have this".into(),
            }),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl MakcuBackend {
    /// a GET or a SET that echoes, and its payload, a refusal's `FF` included. no answer takes the link down
    fn get(&self, command: u8, payload: &[u8]) -> Result<Vec<u8>> {
        let reply = self
            .link
            .request(frame(command, payload), Some(u32::from(command)))?
            .ok_or(Error::Disconnected)?;
        let (_, payload) = parts(&reply).ok_or(Error::Disconnected)?;
        Ok(payload.to_vec())
    }

    /// the spread from a fresh INJECT_SNAPSHOT, which can't be mistaken for a refusal the way INTERPOLATE's own
    /// GET can
    fn spread(&self) -> Result<Interpolation> {
        let snapshot = self.get(frames::INJECT_SNAPSHOT, &[])?;
        match snapshot.get(frames::SPREAD) {
            Some(0xFF) => Ok(Interpolation::Auto),
            Some(&n) if n <= 100 => Ok(Interpolation::Percent(n)),
            _ => Err(Error::Rejected {
                action: Action::Makcu(MakcuAction::Interpolation),
                detail: "it didn't send its state".into(),
            }),
        }
    }
}

fn after(ms: u32) -> Option<Duration> {
    (ms != 0).then(|| Duration::from_millis(u64::from(ms)))
}

#[cfg(any(test, feature = "mock"))]
pub(crate) mod mock;
#[cfg(test)]
mod tests;
