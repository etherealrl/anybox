use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock, Weak};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::backend::{
    Backend, ChordKind, Command, Input, InputKinds, Keys, LOCKED, LockSlot, MAX_CHORD, Sign,
    Signal, Typing, UNLOCKED, Usage,
};
use crate::capability::{Action, Axis, Button, Direction, Execution, Family, Quirk, Snapshot};
use crate::click::{Click, ClickTiming, Holds, Timing};
use crate::error::{Error, Result};
use crate::event::{
    DropSource, Event, EventKinds, Events, Listeners, QUEUE_CAPACITY, Queue, Stamp,
};
use crate::extension::{Extension, ExtensionReply, Ferrum, Makcu, Ysc};
use crate::key::{Key, MediaKey};

/// how long drop waits for the backend and the link thread before it gives up on them
const SHUTDOWN_DEADLINE: Duration = Duration::from_secs(1);
/// how close two refusals of one stream have to be to count as the same one, see `publish`
const MERGE: Duration = Duration::from_secs(1);
/// a text goes to the box this many characters at a time, so a piece fits in `WRITE_DEADLINE` on a box that
/// types about 100 ms a character (MAKCU, measured) and a call waiting for the output lock gets it in between
const PIECE: usize = 2;
/// how long a text leaves the output lock free between pieces. the standard mutex isn't fair, so without it the
/// typing thread would usually take the lock straight back
const BETWEEN_PIECES: Duration = Duration::from_millis(2);

/// one connected box. every call checks what the box can do first, so nothing is faked and nothing is
/// silently dropped. dropping it lets go of every button and key you were holding and closes the port
pub struct Device {
    core: Arc<Core>,
    link: Mutex<Option<(JoinHandle<()>, Receiver<()>)>>,
    /// held for the whole shutdown, so a second `close` waits for the first one to finish
    closed: Mutex<bool>,
}

struct Core {
    backend: Box<dyn Backend>,
    snapshot: RwLock<Arc<Snapshot>>,
    output: Mutex<Output>,
    receivers: Mutex<Receivers>,
    click_timing: ClickTiming,
    holds: Mutex<Holds>,
    opened: Instant,
    signals: Sender<Signal>,
    port: String,
    /// when each stream action was last refused, see `publish`
    refused: Mutex<Refusals>,
    /// set by close and drop before they wait for the output lock, so a text stops at the next piece
    closing: AtomicBool,
}

/// when each stream action was last refused, and a number that goes up every time a refusal actually went out, so
/// a new receiver can tell whether one reached it since it was registered
#[derive(Default)]
struct Refusals {
    last: HashMap<Action, (Instant, u64)>,
    sent: u64,
}

impl Refusals {
    fn note(&mut self, action: Action, at: Instant) {
        self.sent += 1;
        self.last.insert(action, (at, self.sent));
    }
}

/// what was sent to the box, kept under one lock with the sending so the record never disagrees with the box
struct Output {
    held: HashSet<Usage>,
    /// held buttons and keys where the app was told it failed, like a press that ran out of time or a tap timed
    /// on the PC that couldn't let go. a box that keeps held buttons on its own would press these again after a
    /// reconnect, so they get let go
    unsure: HashSet<Usage>,
    /// what anybox locked or scaled, with how much of the physical input each slot keeps. it all gets unlocked on
    /// drop, and on a box that doesn't keep locks itself it gets sent again after a disconnect or a reset
    locked: HashMap<LockSlot, i16>,
    /// keys `force_release_key` holds up for the PC. they get restored on drop, and on a box that doesn't keep them
    /// itself they get forced again after a disconnect or a reset, like locks
    forced: HashSet<Key>,
    /// how many `type_text` calls are running. they let go of the output lock between pieces, so a key can't get
    /// forced up in the middle of a text. a count and not a flag, otherwise the first of two texts to finish would
    /// clear it while the other one still types
    typing: u32,
    link_up: bool,
    /// set once drop or close starts, and after that nothing from the link brings the device back
    closing: bool,
    /// goes up on every disconnect, reconnect and box reset, so a tap timed on the PC can tell its press was
    /// already let go
    session: u64,
}

struct Receivers {
    next_id: u64,
    entries: Vec<Entry>,
}

/// one receiver and the kinds of input it asked for
struct Entry {
    id: u64,
    kinds: EventKinds,
    queue: Weak<Queue>,
}

impl Receivers {
    /// every kind some receiver wants, which is what the box streams
    fn union(&self) -> EventKinds {
        self.entries
            .iter()
            .fold(EventKinds::NONE, |all, entry| all | entry.kinds)
    }
}

pub(crate) struct Start {
    pub(crate) backend: Box<dyn Backend>,
    pub(crate) snapshot: Snapshot,
    pub(crate) signals: Sender<Signal>,
    pub(crate) incoming: Receiver<Signal>,
    pub(crate) click_timing: ClickTiming,
    pub(crate) holds: Holds,
    pub(crate) port: String,
}

impl Device {
    pub(crate) fn start(start: Start) -> Result<Self> {
        let core = Arc::new(Core {
            snapshot: RwLock::new(prepare(start.snapshot, start.click_timing, &start.port)),
            backend: start.backend,
            output: Mutex::new(Output {
                held: HashSet::new(),
                locked: HashMap::new(),
                forced: HashSet::new(),
                typing: 0,
                unsure: HashSet::new(),
                link_up: true,
                closing: false,
                session: 0,
            }),
            receivers: Mutex::new(Receivers {
                next_id: 0,
                entries: Vec::new(),
            }),
            click_timing: start.click_timing,
            holds: Mutex::new(start.holds),
            opened: Instant::now(),
            signals: start.signals,
            port: start.port.clone(),
            refused: Mutex::new(Refusals::default()),
            closing: AtomicBool::new(false),
        });
        let link = spawn_link(Arc::downgrade(&core), start.incoming)?;
        Ok(Self {
            core,
            link: Mutex::new(Some(link)),
            closed: Mutex::new(false),
        })
    }

    /// who the box is and what it can do, both read from the current connection. it can change after
    /// `Reconnected`
    pub fn snapshot(&self) -> Arc<Snapshot> {
        self.core.snapshot()
    }

    /// moves by counts, going through whatever smoothing the box has on
    pub fn move_by(&self, dx: i16, dy: i16) -> Result<()> {
        self.core
            .run(Action::Move, Timing::Any, Command::Move(dx, dy))
    }

    /// moves by counts on the box's next report, skipping its smoothing for this move
    pub fn move_now(&self, dx: i16, dy: i16) -> Result<()> {
        self.core
            .run(Action::MoveNow, Timing::Any, Command::MoveNow(dx, dy))
    }

    pub fn press(&self, button: Button) -> Result<()> {
        self.core
            .run(Action::Press, Timing::Any, Command::Press(button))
    }

    pub fn release(&self, button: Button) -> Result<()> {
        self.core
            .run(Action::Release, Timing::Any, Command::Release(button))
    }

    /// the box's own click, or a click timed on the PC with the device's `ClickTiming` if the box doesn't have one
    pub fn click(&self, button: Button) -> Result<()> {
        self.click_with(button, Click::default())
    }

    pub fn click_with(&self, button: Button, click: Click) -> Result<()> {
        self.core.tap(Usage::Button(button), click)
    }

    /// holds a key down until `release_key`
    pub fn press_key(&self, key: Key) -> Result<()> {
        self.core
            .run(Action::KeyPress, Timing::Any, Command::KeyPress(key))
    }

    pub fn release_key(&self, key: Key) -> Result<()> {
        self.core
            .run(Action::KeyRelease, Timing::Any, Command::KeyRelease(key))
    }

    /// presses and lets go, timed by the box, or on the PC with the device's `ClickTiming` if the box can't
    pub fn tap_key(&self, key: Key) -> Result<()> {
        self.tap_key_with(key, Click::default())
    }

    pub fn tap_key_with(&self, key: Key, tap: Click) -> Result<()> {
        self.core.tap(Usage::Key(key), tap)
    }

    /// types the text on the box with the box's own timing, as US layout key presses, and returns once the box
    /// has typed all of it. it takes printable ASCII only, so tap keys like Enter yourself after it. it goes out a
    /// few characters at a time and other calls on the device can wait in between, and a `close` stops it after
    /// the few going out. after an error part of it may already be typed, and a reconnect never types the rest
    pub fn type_text(&self, text: &str) -> Result<()> {
        self.core.type_text(text)
    }

    /// holds the keys down with one command of the box, up to 14 of them, until `release_keys` or
    /// `release_key`
    pub fn press_keys(&self, keys: &[Key]) -> Result<()> {
        self.core.chord(ChordKind::Press, keys)
    }

    /// lets go of the keys with one command of the box. on a box that can't do that it lets go of them one at a
    /// time like `release_key`, so a key you hold always gets let go. a key you don't hold still has to be one the
    /// box can let go of, so it can be `Unsupported` after the held ones already went
    pub fn release_keys(&self, keys: &[Key]) -> Result<()> {
        self.core.release_keys(keys)
    }

    /// presses and lets go of the keys with one command of the box, timed by the box
    pub fn tap_keys(&self, keys: &[Key]) -> Result<()> {
        self.core.chord(ChordKind::Tap, keys)
    }

    /// holds a media key down until `release_media`
    pub fn press_media(&self, key: MediaKey) -> Result<()> {
        self.core
            .run(Action::MediaPress, Timing::Any, Command::MediaPress(key))
    }

    pub fn release_media(&self, key: MediaKey) -> Result<()> {
        self.core.run(
            Action::MediaRelease,
            Timing::Any,
            Command::MediaRelease(key),
        )
    }

    pub fn tap_media(&self, key: MediaKey) -> Result<()> {
        self.tap_media_with(key, Click::default())
    }

    pub fn tap_media_with(&self, key: MediaKey, tap: Click) -> Result<()> {
        self.core.tap(Usage::Media(key), tap)
    }

    /// keeps the physical button from reaching the PC until `unlock_button`. your own presses still go through and
    /// its events still come, so you can take the button over
    pub fn lock_button(&self, button: Button) -> Result<()> {
        self.core
            .lock(Action::LockButton, &[LockSlot::Button(button)], LOCKED)
    }

    pub fn unlock_button(&self, button: Button) -> Result<()> {
        self.core
            .lock(Action::LockButton, &[LockSlot::Button(button)], UNLOCKED)
    }

    /// keeps the physical key from reaching the PC until `unlock_key`
    pub fn lock_key(&self, key: Key) -> Result<()> {
        self.core
            .lock(Action::LockKey, &[LockSlot::Key(key)], LOCKED)
    }

    pub fn unlock_key(&self, key: Key) -> Result<()> {
        self.core
            .lock(Action::LockKey, &[LockSlot::Key(key)], UNLOCKED)
    }

    /// the PC sees the key up from now on, whatever the finger does, until `restore_key`. key events keep reporting
    /// the finger, so you can tell whether it's still held. a key anybox holds or has locked itself, or one a text
    /// needs while it types, is refused, since each box would do something else with it
    pub fn force_release_key(&self, key: Key) -> Result<()> {
        self.core.force(key, true)
    }

    /// gives the PC the finger's state of the key again, so a key that's still held goes down for it. it also
    /// clears a force another program left, except on a key anybox holds or has locked
    pub fn restore_key(&self, key: Key) -> Result<()> {
        self.core.force(key, false)
    }

    /// keeps physical motion one way or both ways along an axis from reaching the PC until `unlock_axis`. your own
    /// moves still go through
    pub fn lock_axis(&self, axis: Axis, direction: Direction) -> Result<()> {
        self.core
            .lock(Action::LockAxis, &slots(axis, direction), LOCKED)
    }

    /// unlocks that way along the axis and leaves the other way as it was. it also undoes a scale there
    pub fn unlock_axis(&self, axis: Axis, direction: Direction) -> Result<()> {
        self.core
            .lock(Action::LockAxis, &slots(axis, direction), UNLOCKED)
    }

    /// keeps `percent` of the physical motion one way or both ways along an axis, from -255 to 255. 100 leaves it
    /// alone, 0 blocks it and -100 inverts it. `unlock_axis` puts it back to 100
    pub fn scale_axis(&self, axis: Axis, direction: Direction, percent: i16) -> Result<()> {
        self.core
            .lock(Action::ScaleAxis, &slots(axis, direction), percent)
    }

    pub fn wheel(&self, notches: i16) -> Result<()> {
        self.core
            .run(Action::Wheel, Timing::Any, Command::Wheel(notches))
    }

    /// horizontal scroll, positive goes right
    pub fn pan(&self, notches: i16) -> Result<()> {
        self.core
            .run(Action::Pan, Timing::Any, Command::Pan(notches))
    }

    /// throws away the moves the box took but hasn't sent yet. while the link is down it fails with `Disconnected`,
    /// and on a Medius with riding on, the moves it holds back are then only bounded by its riding window
    pub fn discard_pending_moves(&self) -> Result<()> {
        self.core.run(
            Action::DiscardPendingMoves,
            Timing::Any,
            Command::DiscardPendingMoves,
        )
    }

    /// a new receiver for all the input the box can stream, plus the connection events. the box only streams while
    /// at least one receiver is alive. just know that a receiver of keys sees every key typed on the keyboard behind
    /// the box, passwords included
    pub fn events(&self) -> Events {
        self.receiver(EventKinds::ALL, false)
    }

    /// a receiver for only these kinds of input, plus the connection events. the box streams what any receiver
    /// wants, so a receiver for buttons alone doesn't make it send motion. a kind this box can't stream gets one
    /// `Rejected` for it right away, and so does a kind the box refuses to switch on, even when an older receiver
    /// was just told the same thing. on Medius each kind is its own subscription, so the order between two kinds
    /// is only as good as their box time
    pub fn events_of(&self, kinds: EventKinds) -> Events {
        // `ALL` means everything this box has, so it never gets refusals for what it doesn't
        self.receiver(kinds, !kinds.contains(EventKinds::ALL))
    }

    /// what only a MAKCU has, none on another box and after close
    pub fn makcu(&self) -> Option<Makcu<'_>> {
        self.is(Family::MakcuV4).then(|| Makcu::new(self))
    }

    /// what only a YSC8K has, none on another box and after close
    pub fn ysc(&self) -> Option<Ysc<'_>> {
        self.is(Family::Ysc).then(|| Ysc::new(self))
    }

    /// what only a Ferrum has, none on another box and after close
    pub fn ferrum(&self) -> Option<Ferrum<'_>> {
        self.is(Family::Ferrum).then(|| Ferrum::new(self))
    }

    fn is(&self, family: Family) -> bool {
        !lock(&self.core.output).closing && self.core.snapshot().info.family == family
    }

    pub(crate) fn extension(&self, request: Extension) -> Result<ExtensionReply> {
        self.core.extension(request)
    }

    fn receiver(&self, kinds: EventKinds, report_unsupported: bool) -> Events {
        let listeners: Weak<dyn Listeners> = Arc::downgrade(&self.core) as Weak<Core>;
        let (queue, id) = self.core.add_receiver(kinds, report_unsupported);
        Events::new(queue, listeners, id)
    }

    /// the medius crate's own device for this box, for everything only Medius does (clips, rendering, raw catch
    /// etc), and none on other boxes. it's also where you get the polling rate (`query_rate()`) and the identity the
    /// box shows the PC (`device_info()`)
    ///
    /// to tell a mouse from a keyboard, check the capabilities and not the crate's `DeviceKind`, which calls a mouse
    /// with a full key bitmap a keyboard
    ///
    /// a clone you keep holds the port open after this device is gone. anybox doesn't know about anything you press
    /// or lock through it, and its `reset()` lets go of and unlocks everything, including what you did through this
    /// device. `events()` comes back by itself after a `Dropped`, though a button held through the reset can show a
    /// second press with no release in between. anybox's stream takes up to four of the box's 32 catch entries, so
    /// if your own subscriptions fill the table the receiver gets a `Rejected` instead. once you've freed some,
    /// any receiver coming or going asks again. a reboot into a download target keeps the box down
    /// until you replug it
    #[cfg(feature = "medius")]
    pub fn medius(&self) -> Option<medius::Device> {
        if lock(&self.core.output).closing {
            return None;
        }
        self.core
            .backend
            .as_any()
            .downcast_ref::<crate::family::medius::MediusBackend>()
            .and_then(crate::family::medius::MediusBackend::device)
    }

    /// what drop does, but you get to see whether letting go of what you held and unlocking worked, and you choose
    /// the thread that waits for it (up to about 2 seconds when the box doesn't answer). it works on a shared
    /// device too. after it every command fails with `Error::Closed` and the receivers end, `events()` gives one
    /// that's already ended, `snapshot()` still works and `medius()` gives none. a second `close` waits for the
    /// first one and then says `Closed`. that's not the `Closed` an ended receiver returns
    pub fn close(&self) -> Result<()> {
        let mut closed = lock(&self.closed);
        if *closed {
            return Err(Error::Closed);
        }
        *closed = true;
        self.shutdown()
    }

    fn shutdown(&self) -> Result<()> {
        self.core.closing.store(true, Ordering::Release);
        let released = self.core.release_everything();
        let stopped = self.core.backend.close(SHUTDOWN_DEADLINE);
        let _ = self.core.signals.send(Signal::Stop);
        if let Some((handle, done)) = lock(&self.link).take() {
            // a link thread stuck in a dead backend gets left behind, otherwise it would hang the caller
            if done.recv_timeout(SHUTDOWN_DEADLINE).is_ok() {
                let _ = handle.join();
            }
        }
        self.core.close_receivers();
        released.and(stopped)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        let mut closed = lock(&self.closed);
        if !*closed {
            *closed = true;
            let _ = self.shutdown();
        }
    }
}

impl fmt::Debug for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Device")
            .field("info", &self.snapshot().info)
            .finish_non_exhaustive()
    }
}

fn prepare(snapshot: Snapshot, click_timing: ClickTiming, port: &str) -> Arc<Snapshot> {
    let Snapshot {
        mut info,
        capabilities,
    } = snapshot;
    // a backend's snapshot doesn't know the path, the core does and keeps it on every new snapshot
    info.port = port.to_owned();
    Arc::new(Snapshot::new(
        info,
        capabilities.with_host_taps(click_timing),
    ))
}

fn keeps_locked(snapshot: &Snapshot) -> bool {
    [
        Action::LockButton,
        Action::LockAxis,
        Action::LockKey,
        Action::ForceReleaseKey,
    ]
    .into_iter()
    .any(|lock| {
        snapshot
            .capabilities
            .get(lock)
            .quirks
            .contains(&Quirk::KeepsLocked)
    })
}

fn whole_axis_only(snapshot: &Snapshot) -> bool {
    snapshot
        .capabilities
        .get(Action::LockAxis)
        .quirks
        .contains(&Quirk::WholeAxisOnly)
}

/// the other way along the same axis, none for anything that isn't an axis
fn other_way(slot: LockSlot) -> Option<LockSlot> {
    match slot {
        LockSlot::Axis(axis, Sign::Positive) => Some(LockSlot::Axis(axis, Sign::Negative)),
        LockSlot::Axis(axis, Sign::Negative) => Some(LockSlot::Axis(axis, Sign::Positive)),
        _ => None,
    }
}

/// the slots for one way, or both ways, along an axis
fn slots(axis: Axis, direction: Direction) -> Vec<LockSlot> {
    let signs: &[Sign] = match direction {
        Direction::Both => &[Sign::Positive, Sign::Negative],
        Direction::Positive => &[Sign::Positive],
        Direction::Negative => &[Sign::Negative],
    };
    signs
        .iter()
        .map(|&sign| LockSlot::Axis(axis, sign))
        .collect()
}

/// a box keeps held buttons and keys for all of them or none, so the quirk sits on whichever press it has
fn keeps_held(snapshot: &Snapshot) -> bool {
    [Action::Press, Action::KeyPress, Action::MediaPress]
        .into_iter()
        .any(|press| {
            snapshot
                .capabilities
                .get(press)
                .quirks
                .contains(&Quirk::KeepsHeld)
        })
}

fn releases_typed_keys(snapshot: &Snapshot) -> bool {
    snapshot
        .capabilities
        .get(Action::TypeText)
        .quirks
        .contains(&Quirk::TypingReleasesHeldKeys)
}

fn max_keys_held(snapshot: &Snapshot) -> Option<u8> {
    snapshot
        .capabilities
        .get(Action::KeyPress)
        .quirks
        .iter()
        .find_map(|quirk| match quirk {
            Quirk::MaxKeysHeld(max) => Some(*max),
            _ => None,
        })
}

/// the action of the tap itself, then the actions of its press and its release
fn tap_actions(usage: Usage) -> (Action, Action, Action) {
    match usage {
        Usage::Button(_) => (Action::Click, Action::Press, Action::Release),
        Usage::Key(_) => (Action::KeyTap, Action::KeyPress, Action::KeyRelease),
        Usage::Media(_) => (Action::MediaTap, Action::MediaPress, Action::MediaRelease),
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn stream_kinds(snapshot: &Snapshot) -> InputKinds {
    let capabilities = &snapshot.capabilities;
    InputKinds {
        buttons: capabilities.get(Action::ButtonEvents).is_supported(),
        motion: capabilities.get(Action::MotionEvents).is_supported(),
        keys: capabilities.get(Action::KeyEvents).is_supported(),
        media: capabilities.get(Action::MediaEvents).is_supported(),
    }
}

impl Core {
    fn snapshot(&self) -> Arc<Snapshot> {
        self.snapshot
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn unsupported(
        snapshot: &Snapshot,
        action: Action,
        reason: impl Into<Cow<'static, str>>,
    ) -> Error {
        Error::Unsupported {
            action,
            family: snapshot.info.family,
            reason: reason.into(),
        }
    }

    /// checks a command against what the box can do. it runs under the output lock, and the link thread swaps
    /// snapshots under that same lock, so the snapshot checked here is the one the command goes out with
    fn gate(snapshot: &Snapshot, action: Action, timing: Timing, command: Command) -> Result<()> {
        Self::check_capability(snapshot, action, timing)?;
        let capability = snapshot.capabilities.get(action);
        match command {
            Command::Press(button) | Command::Release(button) | Command::Click { button, .. }
                if !snapshot.capabilities.buttons().contains(&button) =>
            {
                Err(Error::InvalidArgument {
                    action,
                    detail: format!("the box has no {button} button").into(),
                })
            }
            Command::KeyPress(key) | Command::KeyRelease(key) | Command::KeyTap { key, .. }
                if !key.is_valid() =>
            {
                Err(Error::InvalidArgument {
                    action,
                    detail: format!("{key} isn't a key, keys go from 0x04 to 0xE7").into(),
                })
            }
            Command::MediaPress(key)
            | Command::MediaRelease(key)
            | Command::MediaTap { key, .. }
                if key.0 == 0 =>
            {
                Err(Error::InvalidArgument {
                    action,
                    detail: "media key 0 isn't a key".into(),
                })
            }
            Command::Click {
                hold: Some(hold), ..
            }
            | Command::KeyTap {
                hold: Some(hold), ..
            }
            | Command::MediaTap {
                hold: Some(hold), ..
            } => {
                for quirk in &capability.quirks {
                    match quirk {
                        Quirk::MaxHold(max) if hold > *max => {
                            return Err(Error::InvalidArgument {
                                action,
                                detail: format!("the box holds it down for at most {max:?}").into(),
                            });
                        }
                        _ => {}
                    }
                }
                Ok(())
            }
            Command::Wheel(notches) => {
                for quirk in &capability.quirks {
                    if let Quirk::NotchesPerCommand(max) = quirk
                        && notches.unsigned_abs() > *max
                    {
                        return Err(Self::unsupported(
                            snapshot,
                            action,
                            match max {
                                1 => {
                                    "it takes one wheel notch per command, send them one at a time"
                                        .into()
                                }
                                _ => format!(
                                    "it takes at most {max} wheel notches per command, so split it up"
                                ),
                            },
                        ));
                    }
                }
                Ok(())
            }
            Command::Lock { slot, keep } => {
                let detail = match slot {
                    LockSlot::Button(button)
                        if !snapshot.capabilities.buttons().contains(&button) =>
                    {
                        Some(format!("the box has no {button} button"))
                    }
                    LockSlot::Key(key) if !key.is_valid() => {
                        Some(format!("{key} isn't a key, keys go from 0x04 to 0xE7"))
                    }
                    LockSlot::Axis(axis, _) if !snapshot.capabilities.axes().contains(&axis) => {
                        Some(format!("the box can't lock the {axis} axis"))
                    }
                    LockSlot::Button(_) | LockSlot::Key(_) | LockSlot::Axis(..)
                        if !(-255..=255).contains(&keep) =>
                    {
                        Some(format!(
                            "{keep} is out of range, a scale goes from -255 to 255"
                        ))
                    }
                    LockSlot::Button(_) | LockSlot::Key(_) | LockSlot::Axis(..) => None,
                };
                match detail {
                    Some(detail) => Err(Error::InvalidArgument {
                        action,
                        detail: detail.into(),
                    }),
                    None => Ok(()),
                }
            }
            Command::KeyForce { key, .. } if !key.is_valid() => Err(Error::InvalidArgument {
                action,
                detail: format!("{key} isn't a key, keys go from 0x04 to 0xE7").into(),
            }),
            Command::KeyChord { keys, .. } => {
                match keys.as_slice().iter().find(|key| !key.is_valid()) {
                    Some(key) => Err(Error::InvalidArgument {
                        action,
                        detail: format!("{key} isn't a key, keys go from 0x04 to 0xE7").into(),
                    }),
                    None => Ok(()),
                }
            }
            Command::Move(..)
            | Command::MoveNow(..)
            | Command::Press(_)
            | Command::Release(_)
            | Command::Click { hold: None, .. }
            | Command::Pan(_)
            | Command::DiscardPendingMoves
            | Command::KeyPress(_)
            | Command::KeyRelease(_)
            | Command::KeyTap { hold: None, .. }
            | Command::MediaPress(_)
            | Command::MediaRelease(_)
            | Command::MediaTap { hold: None, .. }
            | Command::KeyForce { .. } => Ok(()),
        }
    }

    /// a press of a key that's forced up would come out differently on each box (a Medius drops the force for the
    /// press, a MAKCU keeps its mask), so it's refused
    fn check_forced(output: &Output, action: Action, command: Command) -> Result<()> {
        let pressed = match command {
            Command::KeyPress(key) | Command::KeyTap { key, .. } => Some(key),
            Command::KeyChord {
                kind: ChordKind::Press | ChordKind::Tap,
                keys,
            } => keys
                .as_slice()
                .iter()
                .copied()
                .find(|key| output.forced.contains(key)),
            _ => None,
        };
        match pressed {
            Some(key) if output.forced.contains(&key) => Err(Error::InvalidArgument {
                action,
                detail: format!("{key} is forced up, restore_key it first").into(),
            }),
            _ => Ok(()),
        }
    }

    fn check_capability(snapshot: &Snapshot, action: Action, timing: Timing) -> Result<()> {
        match &snapshot.capabilities.get(action).execution {
            Execution::Unsupported { reason } => {
                Err(Self::unsupported(snapshot, action, reason.clone()))
            }
            Execution::Host if timing == Timing::NativeOnly => Err(Self::unsupported(
                snapshot,
                action,
                "the box can't do it by itself and you asked for native timing only",
            )),
            _ => Ok(()),
        }
    }

    /// refuses a key that would go past what the box can hold down at once, counting what anybox already holds
    fn check_rollover(
        output: &Output,
        snapshot: &Snapshot,
        action: Action,
        command: Command,
    ) -> Result<()> {
        let (Command::KeyPress(key) | Command::KeyTap { key, .. }) = command else {
            return Ok(());
        };
        let Some(max) = max_keys_held(snapshot) else {
            return Ok(());
        };
        if key.is_modifier() || output.held.contains(&Usage::Key(key)) {
            return Ok(());
        }
        let held = output
            .held
            .iter()
            .filter(|usage| matches!(usage, Usage::Key(held) if !held.is_modifier()))
            .count();
        if held >= usize::from(max) {
            return Err(Self::unsupported(
                snapshot,
                action,
                format!("this box holds at most {max} keys at once, let one go first"),
            ));
        }
        Ok(())
    }

    /// closed, link down, capability, then the arguments, like a core call, and the call under the output lock
    fn extension(&self, request: Extension) -> Result<ExtensionReply> {
        let (output, snapshot) = self.open_output()?;
        Self::check_capability(&snapshot, request.action(), Timing::Any)?;
        request.check()?;
        let reply = self.backend.extension(request);
        drop(output);
        reply
    }

    fn open_output(&self) -> Result<(MutexGuard<'_, Output>, Arc<Snapshot>)> {
        let output = lock(&self.output);
        if output.closing {
            return Err(Error::Closed);
        }
        if !output.link_up {
            return Err(Error::Disconnected);
        }
        let snapshot = self.snapshot();
        Ok((output, snapshot))
    }

    fn send_locked(
        &self,
        output: &mut Output,
        snapshot: &Snapshot,
        action: Action,
        timing: Timing,
        command: Command,
    ) -> Result<()> {
        // a forced key isn't held, so letting go of it does nothing, and on a Medius it would drop the force
        let command = match command {
            Command::KeyRelease(key) if output.forced.contains(&key) => return Ok(()),
            Command::KeyChord {
                kind: ChordKind::Release,
                keys,
            } if keys
                .as_slice()
                .iter()
                .any(|key| output.forced.contains(key)) =>
            {
                let kept: Vec<Key> = keys
                    .as_slice()
                    .iter()
                    .copied()
                    .filter(|key| !output.forced.contains(key))
                    .collect();
                match Keys::new(&kept) {
                    Some(keys) if !kept.is_empty() => Command::KeyChord {
                        kind: ChordKind::Release,
                        keys,
                    },
                    _ => return Ok(()),
                }
            }
            command => command,
        };
        let held = command.holds();
        // letting go of something held always goes out, even when the box can't do it any more, otherwise a
        // keyboard that got pulled out would leave a key anybox can never release
        let releasing_held = match command {
            Command::KeyChord {
                kind: ChordKind::Release,
                keys,
            } => keys
                .as_slice()
                .iter()
                .all(|&key| output.held.contains(&Usage::Key(key))),
            _ => matches!(held, Some((usage, false)) if output.held.contains(&usage)),
        };
        if !releasing_held {
            Self::gate(snapshot, action, timing, command)?;
        }
        Self::check_forced(output, action, command)?;
        Self::check_rollover(output, snapshot, action, command)?;
        let sent = self.backend.send(command);
        if let Command::KeyChord { kind, keys } = command {
            for &key in keys.as_slice() {
                let usage = Usage::Key(key);
                match (kind, &sent) {
                    (ChordKind::Press, Ok(())) => {
                        output.held.insert(usage);
                        output.unsure.remove(&usage);
                    }
                    (ChordKind::Press, Err(Error::Disconnected)) => {
                        output.held.insert(usage);
                        output.unsure.insert(usage);
                    }
                    (ChordKind::Release, Ok(())) => {
                        output.held.remove(&usage);
                        output.unsure.remove(&usage);
                    }
                    _ => {}
                }
            }
        }
        match (held, &sent) {
            // a press that ran out of time may still have gone out, and releasing something the box doesn't hold
            // does nothing, so it counts as held and the cleanup lets go of it
            (Some((usage, true)), Ok(())) => {
                output.held.insert(usage);
                output.unsure.remove(&usage);
            }
            (Some((usage, true)), Err(Error::Disconnected)) => {
                output.held.insert(usage);
                output.unsure.insert(usage);
            }
            (Some((usage, false)), Ok(())) => {
                output.held.remove(&usage);
                output.unsure.remove(&usage);
            }
            _ => {}
        }
        sent
    }

    fn run(&self, action: Action, timing: Timing, command: Command) -> Result<()> {
        let (mut output, snapshot) = self.open_output()?;
        self.send_locked(&mut output, &snapshot, action, timing, command)
    }

    /// the keys of a chord, checked: none past `MAX_CHORD` and none twice. whether each one is a key is the
    /// gate's
    fn chord_keys(keys: &[Key]) -> Result<Keys> {
        if let Some(key) = keys
            .iter()
            .enumerate()
            .find_map(|(index, key)| keys[..index].contains(key).then_some(key))
        {
            return Err(Error::InvalidArgument {
                action: Action::KeyChord,
                detail: format!("{key} is in it twice").into(),
            });
        }
        Keys::new(keys).ok_or_else(|| Error::InvalidArgument {
            action: Action::KeyChord,
            detail: format!("a chord takes at most {MAX_CHORD} keys").into(),
        })
    }

    fn chord(&self, kind: ChordKind, keys: &[Key]) -> Result<()> {
        let (mut output, snapshot) = self.open_output()?;
        Self::check_capability(&snapshot, Action::KeyChord, Timing::Any)?;
        let keys = Self::chord_keys(keys)?;
        if keys.as_slice().is_empty() {
            return Ok(());
        }
        self.send_locked(
            &mut output,
            &snapshot,
            Action::KeyChord,
            Timing::Any,
            Command::KeyChord { kind, keys },
        )
    }

    /// one chord when the box can do it, otherwise `release_key` for each key, so a key that's held always gets
    /// let go. the keys are checked first, so a bad list lets go of nothing
    fn release_keys(&self, keys: &[Key]) -> Result<()> {
        let (mut output, snapshot) = self.open_output()?;
        let chord = Self::chord_keys(keys)?;
        if let Some(key) = keys.iter().find(|key| !key.is_valid()) {
            return Err(Error::InvalidArgument {
                action: Action::KeyChord,
                detail: format!("{key} isn't a key, keys go from 0x04 to 0xE7").into(),
            });
        }
        if keys.is_empty() {
            return Ok(());
        }
        if snapshot.capabilities.get(Action::KeyChord).is_supported() {
            return self.send_locked(
                &mut output,
                &snapshot,
                Action::KeyChord,
                Timing::Any,
                Command::KeyChord {
                    kind: ChordKind::Release,
                    keys: chord,
                },
            );
        }
        let mut first = Ok(());
        for &key in keys {
            let sent = self.send_locked(
                &mut output,
                &snapshot,
                Action::KeyRelease,
                Timing::Any,
                Command::KeyRelease(key),
            );
            if first.is_ok() {
                first = sent;
            }
        }
        first
    }

    /// checks the whole text, then hands it to the box `PIECE` characters at a time, each under the output lock
    /// like any other command. once any of it went to the box it never answers `Unsupported`, an app falling back
    /// on that would type the whole text again
    fn type_text(&self, text: &str) -> Result<()> {
        let (output, snapshot) = self.open_output()?;
        Self::check_capability(&snapshot, Action::TypeText, Timing::Any)?;
        if let Some((index, c)) = text
            .chars()
            .enumerate()
            .find(|(_, c)| !matches!(c, ' '..='~'))
        {
            return Err(Error::InvalidArgument {
                action: Action::TypeText,
                detail: format!(
                    "it only types printable ASCII and character {} is {c:?}, so tap keys like Enter yourself",
                    index + 1
                )
                .into(),
            });
        }
        let mut output = output;
        if let Some(key) = text
            .bytes()
            .filter_map(Key::for_ascii)
            .find_map(|(key, shift)| {
                if output.forced.contains(&key) {
                    Some(key)
                } else if shift && output.forced.contains(&Key::LEFT_SHIFT) {
                    Some(Key::LEFT_SHIFT)
                } else {
                    None
                }
            })
        {
            return Err(Error::InvalidArgument {
                action: Action::TypeText,
                detail: format!("the text needs {key}, which is forced up, restore_key it first")
                    .into(),
            });
        }
        output.typing += 1;
        drop(output);
        let typed = self.type_pieces(text);
        lock(&self.output).typing -= 1;
        typed
    }

    fn type_pieces(&self, text: &str) -> Result<()> {
        let total = text.len();
        let mut typed = 0;
        let mut typing = Typing::default();
        for piece in text.as_bytes().chunks(PIECE) {
            if typed > 0 {
                thread::sleep(BETWEEN_PIECES);
            }
            let (mut output, snapshot) = self.open_output()?;
            if self.closing.load(Ordering::Acquire) {
                return Err(Error::Closed);
            }
            let piece_typed = Self::check_capability(&snapshot, Action::TypeText, Timing::Any)
                .and_then(|()| self.backend.type_piece(piece, &mut typing));
            match piece_typed {
                Ok(()) => {
                    typed += piece.len();
                    if releases_typed_keys(&snapshot) {
                        Self::forget_typed_keys(&mut output, piece);
                    }
                }
                Err(Error::Unsupported { reason, .. }) if typed > 0 || typing.sent => {
                    return Err(Error::Rejected {
                        action: Action::TypeText,
                        detail: format!("it stopped after {typed} of {total} characters, {reason}"),
                    });
                }
                Err(Error::Rejected { action, detail }) if typed > 0 => {
                    return Err(Error::Rejected {
                        action,
                        detail: format!("{detail}, after {typed} of {total} characters went out"),
                    });
                }
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    /// a box with `TypingReleasesHeldKeys` let go of a held key the piece typed, and of Left Shift when the piece
    /// needed Shift, so the record follows it
    fn forget_typed_keys(output: &mut Output, piece: &[u8]) {
        let mut shifted = false;
        for (key, shift) in piece.iter().filter_map(|&byte| Key::for_ascii(byte)) {
            shifted |= shift;
            output.held.remove(&Usage::Key(key));
            output.unsure.remove(&Usage::Key(key));
        }
        if shifted {
            output.held.remove(&Usage::Key(Key::LEFT_SHIFT));
            output.unsure.remove(&Usage::Key(Key::LEFT_SHIFT));
        }
    }

    /// locks, unlocks or scales each slot. unlocking a slot anybox locked always goes out, same as releasing
    /// something held. every slot gets checked before any of them goes out, so a refused call changes nothing
    fn lock(&self, action: Action, slots: &[LockSlot], keep: i16) -> Result<()> {
        let (mut output, snapshot) = self.open_output()?;
        // one way of an axis that's held alone (locked while the box could still do one way) can still be
        // unlocked, the box then unlocks the whole axis, which is all that's locked on it
        if let [slot] = *slots
            && let Some(other) = other_way(slot)
            && whole_axis_only(&snapshot)
            && !(keep == UNLOCKED
                && output.locked.contains_key(&slot)
                && !output.locked.contains_key(&other))
        {
            return Err(Self::unsupported(
                &snapshot,
                action,
                "it only locks a whole axis, so use Direction::Both",
            ));
        }
        for &slot in slots {
            let unlocking_held = keep == UNLOCKED && output.locked.contains_key(&slot);
            if !unlocking_held {
                Self::gate(&snapshot, action, Timing::Any, Command::Lock { slot, keep })?;
            }
            // on a MAKCU a lock and a force are the same mask, on a Medius two things the box keeps apart
            if let LockSlot::Key(key) = slot
                && keep != UNLOCKED
                && output.forced.contains(&key)
            {
                return Err(Error::InvalidArgument {
                    action,
                    detail: format!("{key} is forced up, restore_key it first").into(),
                });
            }
        }
        let mut first = Ok(());
        for &slot in slots {
            // a forced key can't be locked too, so there's nothing to unlock, and on a MAKCU the unlock would drop
            // the force since both are the same mask
            if let LockSlot::Key(key) = slot
                && keep == UNLOCKED
                && output.forced.contains(&key)
            {
                continue;
            }
            let sent = self.backend.send(Command::Lock { slot, keep });
            if keep == UNLOCKED {
                if sent.is_ok() {
                    output.locked.remove(&slot);
                }
            } else if matches!(sent, Ok(()) | Err(Error::Disconnected)) {
                // a lock that ran out of time may still have gone out, and unlocking a slot that isn't locked does
                // nothing, so it counts as locked
                output.locked.insert(slot, keep);
            }
            if first.is_ok() {
                first = sent;
            }
        }
        first
    }

    /// forces a key up for the PC or gives it back. a restore of a forced key always goes out, even when the box
    /// can't do it any more, like an unlock, so a force never outlives the record
    fn force(&self, key: Key, force: bool) -> Result<()> {
        let action = Action::ForceReleaseKey;
        let (mut output, snapshot) = self.open_output()?;
        let command = Command::KeyForce { key, force };
        let forced = output.forced.contains(&key);
        if force || !forced {
            Self::gate(&snapshot, action, Timing::Any, command)?;
        }
        let usage = Usage::Key(key);
        let held = output.held.contains(&usage) || output.unsure.contains(&usage);
        let locked = output.locked.contains_key(&LockSlot::Key(key));
        // each of these would come out differently on each box, or leave the record wrong
        let refused = if forced {
            None
        } else if held && force {
            Some(format!("you hold {key} yourself, release_key it first"))
        } else if held {
            Some(format!("you hold {key} yourself, release_key it instead"))
        } else if locked && force {
            Some(format!("{key} is locked, unlock_key it first"))
        } else if locked {
            Some(format!("{key} is locked, unlock_key it instead"))
        } else if force && output.typing > 0 {
            Some("a text is typing, force it after".to_string())
        } else {
            None
        };
        if let Some(detail) = refused {
            return Err(Error::InvalidArgument {
                action,
                detail: detail.into(),
            });
        }
        let sent = self.backend.send(command);
        if force {
            // a force that ran out of time may still have gone out, and a restore of a key that isn't forced does
            // nothing, so it counts as forced
            if matches!(sent, Ok(()) | Err(Error::Disconnected)) {
                output.forced.insert(key);
            }
        } else if sent.is_ok() {
            output.forced.remove(&key);
        }
        sent
    }

    /// a click or a key or media tap. it's the box's own if it has one, otherwise anybox presses, waits and
    /// releases from the PC
    fn tap(&self, usage: Usage, tap: Click) -> Result<()> {
        let (action, press, release) = tap_actions(usage);
        let (mut output, snapshot) = self.open_output()?;
        let capability = snapshot.capabilities.get(action);
        // a box whose own click has fixed timing can't take a hold, so a hold gets timed from the PC with a press and a
        // release, the way Ferrum's docs say to do it
        let fixed = tap.hold.is_some() && capability.quirks.contains(&Quirk::FixedClickTiming);
        if capability.execution != Execution::Host && !fixed {
            return self.send_locked(
                &mut output,
                &snapshot,
                action,
                tap.timing,
                usage.tap(tap.hold),
            );
        }
        if tap.timing == Timing::NativeOnly {
            let reason = if fixed {
                "the box's own click can't take a hold and you asked for native timing only"
            } else {
                "the box can't time it by itself and you asked for native timing only"
            };
            return Err(Self::unsupported(&snapshot, action, reason));
        }
        let hold = match tap.hold {
            Some(hold) => hold,
            None => lock(&self.holds).pick(self.click_timing).ok_or_else(|| {
                Self::unsupported(
                    &snapshot,
                    action,
                    "the box can't time it by itself and ClickTiming is off, so give it a hold",
                )
            })?,
        };
        // checked as the tap first, so a bad key or a full box gives an error about the tap and not about its press
        Self::gate(&snapshot, action, tap.timing, usage.tap(None))?;
        Self::check_forced(&output, action, usage.tap(None))?;
        Self::check_rollover(&output, &snapshot, action, usage.press())?;
        self.send_locked(&mut output, &snapshot, press, Timing::Any, usage.press())?;
        let session = output.session;
        drop(output);
        thread::sleep(hold);
        let (mut output, snapshot) = match self.open_output() {
            Ok(open) => open,
            Err(error) => {
                lock(&self.output).unsure.insert(usage);
                return Err(error);
            }
        };
        if output.session != session && !keeps_held(&snapshot) {
            // the box disconnected or reset during the hold, so this press was already let go, and the button or
            // key may belong to a new press by now
            return Err(Error::Interrupted { action });
        }
        let released = self.send_locked(
            &mut output,
            &snapshot,
            release,
            Timing::Any,
            usage.release(),
        );
        if released.is_err() {
            output.unsure.insert(usage);
        }
        released
    }

    fn add_receiver(&self, kinds: EventKinds, report_unsupported: bool) -> (Arc<Queue>, u64) {
        let output = lock(&self.output);
        let queue = Arc::new(Queue::new(QUEUE_CAPACITY));
        let (id, seen) = {
            // registered under the merge lock, so a refusal published after this reaches it
            let refused = lock(&self.refused);
            let mut receivers = lock(&self.receivers);
            let id = receivers.next_id;
            receivers.next_id += 1;
            if output.closing {
                queue.close();
                return (queue, id);
            }
            receivers.entries.push(Entry {
                id,
                kinds,
                queue: Arc::downgrade(&queue),
            });
            (id, refused.last.clone())
        };
        let snapshot = self.snapshot();
        if report_unsupported {
            let streamable = EventKinds::from_input(stream_kinds(&snapshot));
            for (_, action) in (kinds & !streamable).each() {
                queue.push(Event::Rejected { action, count: 1 });
            }
        }
        let refused = self.apply_streams(&output, &snapshot);
        drop(output);
        // the merge in publish is for the box saying the same refusal twice. this receiver just asked, so it hears
        // its answer once, otherwise a receiver made right after a refused one would wait forever for a stream
        // that never came on
        for action in refused {
            let event = Event::Rejected { action, count: 1 };
            let mut merge = lock(&self.refused);
            let last = merge.last.get(&action).copied();
            // a MAKCU's reader can publish the refusal before the switch returns, and then this receiver has it
            let heard = last.map(|(_, n)| n) != seen.get(&action).map(|(_, n)| *n);
            if !heard && EventKinds::of(&event).is_some_and(|kind| !(kinds & kind).is_empty()) {
                queue.push(event.clone());
            }
            let now = Instant::now();
            match last {
                Some((at, n)) if now.duration_since(at) < MERGE => {
                    // counted from this answer, so the box's second report of it merges for this receiver too
                    merge.last.insert(action, (now, n));
                }
                _ => {
                    merge.note(action, now);
                    self.deliver(&event, Some(id));
                }
            }
        }
        (queue, id)
    }

    /// to each receiver that wants this kind of event. connection events and drops from the box go to everyone
    fn publish(&self, event: Event) {
        // a MAKCU reports one refused stream switch twice, the refusal itself and the switch that then failed, in
        // either order, so a second one within a second is the same refusal. ponytail: a real second refusal that
        // close is merged too, which only costs one count
        if let Event::Rejected { action, .. } = &event
            && EventKinds::of_action(*action).is_some()
        {
            let now = Instant::now();
            // held while it goes out, so add_receiver can tell whether a refusal reached its new receiver
            let mut merge = lock(&self.refused);
            if merge
                .last
                .get(action)
                .is_some_and(|(at, _)| now.duration_since(*at) < MERGE)
            {
                return;
            }
            merge.note(*action, now);
            self.deliver(&event, None);
            return;
        }
        self.deliver(&event, None);
    }

    fn deliver(&self, event: &Event, skip: Option<u64>) {
        let kind = EventKinds::of(event);
        let mut receivers = lock(&self.receivers);
        receivers
            .entries
            .retain(|entry| match entry.queue.upgrade() {
                Some(queue) => {
                    if Some(entry.id) != skip
                        && kind.is_none_or(|kind| !(entry.kinds & kind).is_empty())
                    {
                        queue.push(event.clone());
                    }
                    true
                }
                None => false,
            });
    }

    fn publish_refused(&self, actions: Vec<Action>) {
        for action in actions {
            self.publish(Event::Rejected { action, count: 1 });
        }
    }

    /// only to the receivers that want one of these kinds
    fn publish_to(&self, kinds: EventKinds, event: Event) {
        let receivers = lock(&self.receivers);
        for entry in &receivers.entries {
            if !(entry.kinds & kinds).is_empty()
                && let Some(queue) = entry.queue.upgrade()
            {
                queue.push(event.clone());
            }
        }
    }

    fn close_receivers(&self) {
        for entry in lock(&self.receivers).entries.drain(..) {
            if let Some(queue) = entry.queue.upgrade() {
                queue.close();
            }
        }
    }

    /// streams what the receivers want and the box can, the empty set too, so nothing streams that nobody wants.
    /// called with the output lock held, which is what orders it against reconnects and the other receivers. it
    /// returns the refusals to publish once the output lock is let go, `Disconnected` isn't one because the
    /// reconnect asks again
    fn apply_streams(&self, output: &Output, snapshot: &Snapshot) -> Vec<Action> {
        if output.closing || !output.link_up {
            return Vec::new();
        }
        let union = lock(&self.receivers).union();
        let wanted = union & EventKinds::from_input(stream_kinds(snapshot));
        match self.backend.set_streams(wanted.to_input()) {
            Err(failure) if !matches!(failure.error, Error::Disconnected) => {
                (EventKinds::from_input(failure.failed) & wanted)
                    .each()
                    .map(|(_, action)| action)
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    /// after a reconnect or a reset this lets go of everything that's held. a box that keeps held buttons and keys
    /// on its own only gets the ones the app was told failed. a release that fails stays in the record, so drop
    /// tries it again
    fn release_held(&self, output: &mut Output, snapshot: &Snapshot) -> bool {
        let usages = if keeps_held(snapshot) {
            &output.unsure
        } else {
            &output.held
        };
        let usages: Vec<Usage> = usages.iter().copied().collect();
        let mut all_released = true;
        for usage in usages {
            if self.backend.send(usage.release()).is_ok() {
                output.held.remove(&usage);
                output.unsure.remove(&usage);
            } else {
                all_released = false;
            }
        }
        all_released
    }

    fn release_everything(&self) -> Result<()> {
        let mut output = lock(&self.output);
        output.closing = true;
        output.link_up = false;
        output.unsure.clear();
        let held: Vec<Usage> = output.held.drain().collect();
        let released = if held.is_empty() {
            Ok(())
        } else {
            self.backend.release_all(&held)
        };
        let forced: Vec<Key> = output.forced.drain().collect();
        let restored = if forced.is_empty() {
            Ok(())
        } else {
            self.backend.restore_all(&forced)
        };
        let locked: Vec<LockSlot> = output.locked.drain().map(|(slot, _)| slot).collect();
        let unlocked = if locked.is_empty() {
            Ok(())
        } else {
            self.backend.unlock_all(&locked)
        };
        released.and(restored).and(unlocked)
    }

    /// sends the app's locks again after a reconnect or a reset, on a box that doesn't keep them itself. a lock the
    /// box can't do right now waits for a snapshot that can, and if sending one fails the backend fixes it itself
    fn relock(&self, output: &Output, snapshot: &Snapshot) {
        if keeps_locked(snapshot) {
            return;
        }
        for (&slot, &keep) in &output.locked {
            // a box that only locks a whole axis only gets it back when both ways were locked the same, a lone way
            // waits for a box that can do it
            if whole_axis_only(snapshot)
                && let Some(other) = other_way(slot)
                && output.locked.get(&other) != Some(&keep)
            {
                continue;
            }
            let command = Command::Lock { slot, keep };
            if Self::gate(snapshot, command.action(), Timing::Any, command).is_ok() {
                let _ = self.backend.send(command);
            }
        }
        for &key in &output.forced {
            let command = Command::KeyForce { key, force: true };
            if Self::gate(snapshot, command.action(), Timing::Any, command).is_ok() {
                let _ = self.backend.send(command);
            }
        }
    }

    fn handle(&self, signal: Signal) {
        match signal {
            Signal::Input {
                input,
                arrived,
                box_time,
                read,
            } => {
                let mut at = Stamp::new(arrived, arrived.saturating_duration_since(self.opened));
                at.box_time = box_time;
                if let Some(read) = read {
                    at.received = read.at;
                    at.received_since_open = read.at.saturating_duration_since(self.opened);
                    at.received_qpc = read.qpc;
                }
                self.publish(match input {
                    Input::Button { button, pressed } => Event::Button {
                        button,
                        pressed,
                        at,
                    },
                    Input::Motion { dx, dy, wheel, pan } => Event::Motion {
                        dx,
                        dy,
                        wheel,
                        pan,
                        at,
                    },
                    Input::Key { key, pressed } => Event::Key { key, pressed, at },
                    Input::Media { key, pressed } => Event::Media { key, pressed, at },
                });
            }
            Signal::Dropped(count, source, kind) => match kind {
                Some(kind) => self.publish_to(
                    EventKinds::from_input(kind.kinds()),
                    Event::Dropped { count, source },
                ),
                None => self.publish(Event::Dropped { count, source }),
            },
            Signal::StreamLost(kind) => {
                let output = lock(&self.output);
                if output.closing {
                    return;
                }
                let lost = EventKinds::from_input(kind.kinds());
                // the box doesn't say how many it lost, so this means at least one, and only to the receivers of
                // that kind
                self.publish_to(
                    lost,
                    Event::Dropped {
                        count: 1,
                        source: DropSource::Device,
                    },
                );
                // only restart it while someone still wants it, checked under the same lock a dropped receiver takes,
                // so a stream nobody wants stays off. and only the lost one, because the others are still running
                if output.link_up && lock(&self.receivers).union().contains(lost) {
                    let restarted = self.backend.restart_stream(kind);
                    drop(output);
                    if let Err(error) = restarted
                        && !matches!(error, Error::Disconnected)
                    {
                        for (_, action) in lost.each() {
                            self.publish(Event::Rejected { action, count: 1 });
                        }
                    }
                }
            }
            Signal::Refused(action) => self.publish(Event::Rejected { action, count: 1 }),
            Signal::Disconnected => {
                let mut output = lock(&self.output);
                if output.closing {
                    return;
                }
                // a box that came back but never got up again was already said to be down
                let was_up = output.link_up;
                output.link_up = false;
                output.session += 1;
                drop(output);
                if was_up {
                    self.publish(Event::Disconnected);
                }
            }
            Signal::Reconnected(snapshot) => {
                let mut output = lock(&self.output);
                if output.closing {
                    return;
                }
                output.session += 1;
                let snapshot = prepare(*snapshot, self.click_timing, &self.port);
                *self
                    .snapshot
                    .write()
                    .unwrap_or_else(PoisonError::into_inner) = snapshot.clone();
                if !self.release_held(&mut output, &snapshot) {
                    // a release failing right after the box came back means it's gone again, so stay down until
                    // the next reconnect gets a clean slate
                    let was_up = output.link_up;
                    output.link_up = false;
                    drop(output);
                    if was_up {
                        self.publish(Event::Disconnected);
                    }
                    return;
                }
                self.relock(&output, &snapshot);
                output.link_up = true;
                let refused = self.apply_streams(&output, &snapshot);
                drop(output);
                self.publish(Event::Reconnected);
                self.publish_refused(refused);
            }
            Signal::Changed(snapshot) => {
                let output = lock(&self.output);
                if output.closing {
                    return;
                }
                let snapshot = prepare(*snapshot, self.click_timing, &self.port);
                let changed = {
                    let mut current = self
                        .snapshot
                        .write()
                        .unwrap_or_else(PoisonError::into_inner);
                    // compared after `prepare`, the backend's own snapshot has no port and no host taps
                    let changed = *current != snapshot;
                    *current = snapshot.clone();
                    changed
                };
                // a device that came or went can change which kinds the box streams
                let refused = self.apply_streams(&output, &snapshot);
                drop(output);
                if changed {
                    self.publish(Event::Changed);
                }
                self.publish_refused(refused);
            }
            Signal::SessionReset => {
                let mut output = lock(&self.output);
                if output.closing {
                    return;
                }
                output.session += 1;
                let snapshot = self.snapshot();
                self.release_held(&mut output, &snapshot);
                self.relock(&output, &snapshot);
                let refused = self.apply_streams(&output, &snapshot);
                drop(output);
                self.publish(Event::SessionReset);
                self.publish_refused(refused);
            }
            #[cfg(test)]
            Signal::Barrier(done) => {
                let _ = done.send(());
            }
            #[cfg(feature = "mock")]
            Signal::Examples(events) => {
                for event in events {
                    self.publish(event);
                }
            }
            Signal::Stop => {}
        }
    }
}

impl Listeners for Core {
    fn receiver_dropped(&self, id: u64) {
        let output = lock(&self.output);
        lock(&self.receivers).entries.retain(|entry| entry.id != id);
        let refused = self.apply_streams(&output, &self.snapshot());
        drop(output);
        self.publish_refused(refused);
    }
}

fn spawn_link(
    core: Weak<Core>,
    incoming: Receiver<Signal>,
) -> Result<(JoinHandle<()>, Receiver<()>)> {
    let (done, finished) = mpsc::channel();
    let handle = thread::Builder::new()
        .name("anybox-link".into())
        .spawn(move || {
            while let Ok(signal) = incoming.recv() {
                if matches!(signal, Signal::Stop) {
                    break;
                }
                match core.upgrade() {
                    Some(core) => core.handle(signal),
                    None => break,
                }
            }
            let _ = done.send(());
        })?;
    Ok((handle, finished))
}

#[cfg(feature = "mock")]
impl Device {
    /// sends every receiver one of each event from `Event::examples`, through the same queue as real ones, without
    /// any of them doing to the device what the real one would. for testing how code reads every kind of event
    pub fn mock_example_events(&self) {
        let now = Instant::now();
        let at = Stamp::new(now, now.saturating_duration_since(self.core.opened));
        let _ = self
            .core
            .signals
            .send(Signal::Examples(Event::examples(at)));
    }
}

#[cfg(test)]
impl Device {
    /// only the first part of what drop does, so a test can send link events into a device that's closing
    pub(crate) fn start_closing(&self) {
        let _ = self.core.release_everything();
    }

    pub(crate) fn backend_for_tests(&self) -> &dyn std::any::Any {
        self.core.backend.as_any()
    }

    pub(crate) fn held(&self) -> HashSet<Usage> {
        lock(&self.core.output).held.clone()
    }

    /// waits until the link thread handled everything that was signaled before this call
    pub(crate) fn settle(&self) {
        let (done, wait) = mpsc::channel();
        let _ = self.core.signals.send(Signal::Barrier(done));
        let _ = wait.recv_timeout(Duration::from_secs(5));
    }
}

#[cfg(test)]
mod tests;
