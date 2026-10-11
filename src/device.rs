use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock, Weak};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::backend::{
    Backend, Command, Input, InputKinds, LOCKED, LockSlot, Sign, Signal, UNLOCKED, Usage,
};
use crate::capability::{Action, Axis, Button, Direction, Execution, Quirk, Snapshot};
use crate::click::{Click, ClickTiming, Holds, Timing};
use crate::error::{Error, Result};
use crate::event::{DropSource, Event, Events, Listeners, QUEUE_CAPACITY, Queue, Stamp};
use crate::key::{Key, MediaKey};

/// how long drop waits for the backend and the link thread before it gives up on them
const SHUTDOWN_DEADLINE: Duration = Duration::from_secs(1);

/// one connected box. every call checks what the box can do first, so nothing is faked and nothing is
/// silently dropped. dropping it lets go of every button and key you were holding and closes the port
pub struct Device {
    core: Arc<Core>,
    link: Mutex<Option<(JoinHandle<()>, Receiver<()>)>>,
    closed: bool,
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
    link_up: bool,
    /// set once drop or close starts, and after that nothing from the link brings the device back
    closing: bool,
    /// goes up on every disconnect, reconnect and box reset, so a tap timed on the PC can tell its press was
    /// already let go
    session: u64,
}

struct Receivers {
    queues: Vec<Weak<Queue>>,
    alive: usize,
}

pub(crate) struct Start {
    pub(crate) backend: Box<dyn Backend>,
    pub(crate) snapshot: Snapshot,
    pub(crate) signals: Sender<Signal>,
    pub(crate) incoming: Receiver<Signal>,
    pub(crate) click_timing: ClickTiming,
    pub(crate) holds: Holds,
}

impl Device {
    pub(crate) fn start(start: Start) -> Result<Self> {
        let core = Arc::new(Core {
            snapshot: RwLock::new(prepare(start.snapshot, start.click_timing)),
            backend: start.backend,
            output: Mutex::new(Output {
                held: HashSet::new(),
                locked: HashMap::new(),
                unsure: HashSet::new(),
                link_up: true,
                closing: false,
                session: 0,
            }),
            receivers: Mutex::new(Receivers {
                queues: Vec::new(),
                alive: 0,
            }),
            click_timing: start.click_timing,
            holds: Mutex::new(start.holds),
            opened: Instant::now(),
            signals: start.signals,
        });
        let link = spawn_link(Arc::downgrade(&core), start.incoming)?;
        Ok(Self {
            core,
            link: Mutex::new(Some(link)),
            closed: false,
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

    pub fn discard_pending_moves(&self) -> Result<()> {
        self.core.run(
            Action::DiscardPendingMoves,
            Timing::Any,
            Command::DiscardPendingMoves,
        )
    }

    /// a new receiver for input events and connection events. the box only streams input while at least one is
    /// alive. just know that every receiver sees every key typed on the keyboard behind the box, passwords included
    pub fn events(&self) -> Events {
        let listeners: Weak<dyn Listeners> = Arc::downgrade(&self.core) as Weak<Core>;
        Events::new(self.core.add_receiver(), listeners)
    }

    /// the medius crate's own device for this box, for everything only Medius does (clips, rendering, raw catch
    /// etc), and none on other boxes. it's also where you get the polling rate (`query_rate()`) and the identity the
    /// box shows the PC (`device_info()`)
    ///
    /// a clone you keep holds the port open after this device is gone. anybox doesn't know about anything you press
    /// or lock through it, and its `reset()` lets go of and unlocks everything, including what you did through this
    /// device. `events()` comes back by itself after a `Dropped`, though a button held through the reset can show a
    /// second press with no release in between. anybox's stream takes up to four of the box's 32 catch entries, so
    /// if your own subscriptions fill the table the receiver gets a `Rejected` instead. once you've freed some,
    /// drop every receiver and make a new one so it asks again. a reboot into a download target keeps the box down
    /// until you replug it
    #[cfg(feature = "medius")]
    pub fn medius(&self) -> Option<medius::Device> {
        self.core
            .backend
            .as_any()
            .downcast_ref::<crate::family::medius::MediusBackend>()
            .and_then(crate::family::medius::MediusBackend::device)
    }

    /// what drop does, but you get to see whether letting go of what you held worked
    pub fn close(mut self) -> Result<()> {
        self.shutdown()
    }

    fn shutdown(&mut self) -> Result<()> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;
        let released = self.core.release_everything();
        self.core.backend.shutdown(SHUTDOWN_DEADLINE);
        let _ = self.core.signals.send(Signal::Stop);
        if let Some((handle, done)) = lock(&self.link).take() {
            // a link thread stuck in a dead backend gets left behind, otherwise it would hang the caller
            if done.recv_timeout(SHUTDOWN_DEADLINE).is_ok() {
                let _ = handle.join();
            }
        }
        self.core.close_receivers();
        released
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

impl fmt::Debug for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Device")
            .field("info", &self.snapshot().info)
            .finish_non_exhaustive()
    }
}

fn prepare(snapshot: Snapshot, click_timing: ClickTiming) -> Arc<Snapshot> {
    let Snapshot { info, capabilities } = snapshot;
    Arc::new(Snapshot::new(
        info,
        capabilities.with_host_taps(click_timing),
    ))
}

fn keeps_locked(snapshot: &Snapshot) -> bool {
    [Action::LockButton, Action::LockAxis, Action::LockKey]
        .into_iter()
        .any(|lock| {
            snapshot
                .capabilities
                .get(lock)
                .quirks
                .contains(&Quirk::KeepsLocked)
        })
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
        let capability = snapshot.capabilities.get(action);
        match &capability.execution {
            Execution::Unsupported { reason } => {
                return Err(Self::unsupported(snapshot, action, reason.clone()));
            }
            Execution::Host if timing == Timing::NativeOnly => {
                return Err(Self::unsupported(
                    snapshot,
                    action,
                    "the box can't do it by itself and you asked for native timing only",
                ));
            }
            _ => {}
        }
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
                        Quirk::FixedClickTiming => {
                            return Err(Self::unsupported(
                                snapshot,
                                action,
                                "it has its own timing and doesn't take a hold, so press and release it yourself if you need one",
                            ));
                        }
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
            | Command::MediaTap { hold: None, .. } => Ok(()),
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

    fn open_output(&self) -> Result<(MutexGuard<'_, Output>, Arc<Snapshot>)> {
        let output = lock(&self.output);
        if output.closing || !output.link_up {
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
        let held = command.holds();
        // letting go of something held always goes out, even when the box can't do it any more, otherwise a
        // keyboard that got pulled out would leave a key anybox can never release
        let releasing_held = matches!(held, Some((usage, false)) if output.held.contains(&usage));
        if !releasing_held {
            Self::gate(snapshot, action, timing, command)?;
        }
        Self::check_rollover(output, snapshot, action, command)?;
        let sent = self.backend.send(command);
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

    /// locks, unlocks or scales each slot. unlocking a slot anybox locked always goes out, same as releasing
    /// something held. every slot gets checked before any of them goes out, so a refused call changes nothing
    fn lock(&self, action: Action, slots: &[LockSlot], keep: i16) -> Result<()> {
        let (mut output, snapshot) = self.open_output()?;
        for &slot in slots {
            let unlocking_held = keep == UNLOCKED && output.locked.contains_key(&slot);
            if !unlocking_held {
                Self::gate(&snapshot, action, Timing::Any, Command::Lock { slot, keep })?;
            }
        }
        let mut first = Ok(());
        for &slot in slots {
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

    /// a click or a key or media tap. it's the box's own if it has one, otherwise anybox presses, waits and
    /// releases from the PC
    fn tap(&self, usage: Usage, tap: Click) -> Result<()> {
        let (action, press, release) = tap_actions(usage);
        let (mut output, snapshot) = self.open_output()?;
        if snapshot.capabilities.get(action).execution != Execution::Host {
            return self.send_locked(
                &mut output,
                &snapshot,
                action,
                tap.timing,
                usage.tap(tap.hold),
            );
        }
        if tap.timing == Timing::NativeOnly {
            return Err(Self::unsupported(
                &snapshot,
                action,
                "the box can't time it by itself and you asked for native timing only",
            ));
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

    fn add_receiver(&self) -> Arc<Queue> {
        let output = lock(&self.output);
        let mut receivers = lock(&self.receivers);
        let queue = Arc::new(Queue::new(QUEUE_CAPACITY));
        receivers.queues.push(Arc::downgrade(&queue));
        receivers.alive += 1;
        if receivers.alive == 1 && output.link_up && !output.closing {
            // a box that refuses now gets asked again after the next reconnect
            let _ = self.backend.set_streams(stream_kinds(&self.snapshot()));
        }
        queue
    }

    fn publish(&self, event: Event) {
        let mut receivers = lock(&self.receivers);
        receivers.queues.retain(|queue| match queue.upgrade() {
            Some(queue) => {
                queue.push(event.clone());
                true
            }
            None => false,
        });
    }

    fn close_receivers(&self) {
        for queue in lock(&self.receivers).queues.drain(..) {
            if let Some(queue) = queue.upgrade() {
                queue.close();
            }
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

    fn restart_streams(&self, snapshot: &Snapshot) {
        if lock(&self.receivers).alive > 0 {
            // a box that refuses now gets asked again after the next reconnect
            let _ = self.backend.set_streams(stream_kinds(snapshot));
        }
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
        let locked: Vec<LockSlot> = output.locked.drain().map(|(slot, _)| slot).collect();
        let unlocked = if locked.is_empty() {
            Ok(())
        } else {
            self.backend.unlock_all(&locked)
        };
        released.and(unlocked)
    }

    /// sends the app's locks again after a reconnect or a reset, on a box that doesn't keep them itself. a lock the
    /// box can't do right now waits for a snapshot that can, and if sending one fails the backend fixes it itself
    fn relock(&self, output: &Output, snapshot: &Snapshot) {
        if keeps_locked(snapshot) {
            return;
        }
        for (&slot, &keep) in &output.locked {
            let command = Command::Lock { slot, keep };
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
            } => {
                let at = Stamp {
                    arrived,
                    since_open: arrived.saturating_duration_since(self.opened),
                    box_time,
                };
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
            Signal::Dropped(count, source) => self.publish(Event::Dropped { count, source }),
            Signal::StreamLost(kind) => {
                let output = lock(&self.output);
                if output.closing {
                    return;
                }
                // the box doesn't say how many it lost, so this means at least one
                self.publish(Event::Dropped {
                    count: 1,
                    source: DropSource::Device,
                });
                // only restart it while someone still listens, checked under the same locks a dropped receiver takes,
                // so a stream nobody wants stays off. and only the lost one, because the others are still running
                if output.link_up && lock(&self.receivers).alive > 0 {
                    let _ = self.backend.restart_stream(kind);
                }
            }
            Signal::Refused(action) => self.publish(Event::Rejected { action, count: 1 }),
            Signal::Disconnected => {
                let mut output = lock(&self.output);
                if output.closing {
                    return;
                }
                output.link_up = false;
                output.session += 1;
                drop(output);
                self.publish(Event::Disconnected);
            }
            Signal::Reconnected(snapshot) => {
                let mut output = lock(&self.output);
                if output.closing {
                    return;
                }
                output.session += 1;
                let snapshot = prepare(*snapshot, self.click_timing);
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
                self.restart_streams(&snapshot);
                output.link_up = true;
                drop(output);
                self.publish(Event::Reconnected);
            }
            Signal::Changed(snapshot) => {
                let output = lock(&self.output);
                if output.closing {
                    return;
                }
                *self
                    .snapshot
                    .write()
                    .unwrap_or_else(PoisonError::into_inner) =
                    prepare(*snapshot, self.click_timing);
                drop(output);
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
                if output.link_up {
                    self.restart_streams(&snapshot);
                }
                drop(output);
                self.publish(Event::SessionReset);
            }
            #[cfg(test)]
            Signal::Barrier(done) => {
                let _ = done.send(());
            }
            Signal::Stop => {}
        }
    }
}

impl Listeners for Core {
    fn receiver_dropped(&self) {
        let output = lock(&self.output);
        let mut receivers = lock(&self.receivers);
        receivers.alive = receivers.alive.saturating_sub(1);
        if receivers.alive == 0 && output.link_up && !output.closing {
            let _ = self.backend.set_streams(InputKinds::default());
        }
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
