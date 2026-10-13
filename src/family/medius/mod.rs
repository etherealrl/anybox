use std::any::Any;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock};
use std::thread;
use std::time::{Duration, Instant};

use medius::{CatchFilter, ClockDomain, InputEvent, InputStream, Timeline};

use crate::backend::{
    Backend, Command, Input, InputKind, InputKinds, LockSlot, Sign, Sink, StreamFailure, UNLOCKED,
    Usage, WRITE_DEADLINE,
};
use crate::capability::{
    Action, Axis, Button, Capabilities, Capability, Family, Firmware, Info, Quirk, Snapshot,
    StableId, UsbIds,
};
use crate::error::{Error, Result};
use crate::event::{BoxTime, Clock, DropSource};
use crate::key::{Key, MediaKey};
use crate::transport::os::Read;
use crate::transport::probe::Probe;
use crate::transport::{Candidate, Port, open_error};

pub(crate) mod frame;
#[cfg(feature = "mock")]
pub(crate) mod mock;
mod poller;

pub(crate) use poller::Poller;

/// the protocol this version of the medius crate speaks. the crate refuses a box on any other one
pub(crate) const PROTOCOL: u8 = 9;

/// Medius firmware before 3.4.0 ran its control link at 4 Mbaud
pub(crate) const OLD_BAUD: u32 = 4_000_000;

const POLL_EVERY: Duration = Duration::from_millis(250);
const PUMP_SLICE: Duration = Duration::from_millis(50);

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// opens a Medius box on one port, or returns `NotFound` when what's there isn't one
pub(crate) fn open(candidate: &Candidate, sink: Sink) -> Result<(Box<dyn Backend>, Snapshot)> {
    let path = candidate.path.as_path();
    if !could_be_one(candidate) {
        return Err(not_found(path));
    }
    // the crate already locks the port on Linux (exclusive while it opens, shared after), so a second Anybox
    // can't get in. the thing is it drops the OS error code, so we take the lock and let go of it here just to
    // tell a busy, forbidden or missing port apart
    #[cfg(unix)]
    drop(crate::transport::unix::lock_port(path).map_err(|error| open_error(path, error))?);
    let device = match medius::Device::open(path) {
        Ok(device) => device,
        Err(medius::Error::NoReply) => return Err(old_firmware_or_nothing(path)),
        Err(error) => return Err(open_failure(path, error)),
    };
    let (backend, snapshot, poller) = build(device, sink)?;
    poller.spawn()?;
    Ok((Box::new(backend), snapshot))
}

/// a Medius is always behind a CH343, so a named port with another chip never gets the 6 Mbaud probe. a port with
/// no USB id might still be one
pub(crate) fn could_be_one(candidate: &Candidate) -> bool {
    candidate
        .usb
        .as_ref()
        .is_none_or(crate::transport::is_ch343)
}

fn old_firmware_or_nothing(path: &Path) -> Error {
    let system = crate::transport::os::OsSystem;
    match crate::transport::System::open(&system, path, OLD_BAUD) {
        Ok(port) => probe_old_firmware(&*port, path),
        Err(_) => not_found(path),
    }
}

fn not_found(path: &Path) -> Error {
    Error::NotFound {
        tried: vec![path.display().to_string()],
    }
}

/// sends one raw QUERY(VERSION) at the old control link speed. a box that answers there is a Medius too old for
/// this crate
pub(crate) fn probe_old_firmware(port: &dyn Port, path: &Path) -> Error {
    let stop = AtomicBool::new(false);
    let probe = Probe::new(port, &stop);
    let reply = probe.ask(
        &frame::query_version(1),
        &mut frame::MediusFrames,
        |frame| frame::version_reply(frame).is_some(),
        Duration::from_millis(300),
    );
    match reply
        .ok()
        .flatten()
        .and_then(|frame| frame::version_reply(&frame))
    {
        Some(found) => Error::FirmwareTooOld {
            family: Family::Medius,
            found,
            needed: Firmware::Protocol(PROTOCOL),
        },
        None => not_found(path),
    }
}

/// turns an error from the crate's open into one the app can act on
pub(crate) fn open_failure(path: &Path, error: medius::Error) -> Error {
    match error {
        medius::Error::BadProtoVer { got } => protocol_mismatch(got),
        medius::Error::Io(io) => open_error(path, io),
        medius::Error::NoReply | medius::Error::NotFound => not_found(path),
        other => Error::Io(std::io::Error::other(other)),
    }
}

pub(crate) fn protocol_mismatch(got: u8) -> Error {
    if got < PROTOCOL {
        Error::FirmwareTooOld {
            family: Family::Medius,
            found: Firmware::Protocol(got),
            needed: Firmware::Protocol(PROTOCOL),
        }
    } else {
        Error::LibraryTooOld {
            family: Family::Medius,
            found: Firmware::Protocol(got),
        }
    }
}

fn to_crate(button: Button) -> medius::Button {
    medius::Button(match button {
        Button::Left => 0,
        Button::Right => 1,
        Button::Middle => 2,
        Button::Side1 => 3,
        Button::Side2 => 4,
        Button::Extra(id) => id,
    })
}

fn axis_to_crate(axis: Axis) -> medius::Axis {
    match axis {
        Axis::X => medius::Axis::X,
        Axis::Y => medius::Axis::Y,
        Axis::Wheel => medius::Axis::Wheel,
        Axis::Pan => medius::Axis::Pan,
    }
}

fn usage_to_crate(usage: Usage) -> medius::Usage {
    match usage {
        Usage::Button(button) => to_crate(button).into(),
        Usage::Key(key) => medius::Key(key.0).into(),
        Usage::Media(key) => medius::MediaKey(key.0).into(),
    }
}

fn from_crate(id: u16) -> Button {
    match id {
        0 => Button::Left,
        1 => Button::Right,
        2 => Button::Middle,
        3 => Button::Side1,
        4 => Button::Side2,
        id => Button::Extra(id as u8),
    }
}

/// what the box said about the device plugged into it, so the poller can rebuild the snapshot when that changes
pub(crate) type CloneState = (bool, bool, bool, medius::Caps);

/// builds the snapshot and returns what it was built from, so the poller can tell when the device behind the box
/// changed
pub(crate) fn snapshot(device: &medius::Device) -> Result<(Snapshot, CloneState)> {
    let map = |error| match error {
        medius::Error::QueryTimeout => Error::Timeout {
            action: Action::Connect,
        },
        medius::Error::Io(_) => Error::Disconnected,
        other => Error::Io(std::io::Error::other(other)),
    };
    let version = device.query_version().map_err(map)?;
    let all_caps = device.caps().map_err(map)?;
    let caps = all_caps.mouse;
    let keyboard = all_caps.keyboard;
    let health = device.query_health().map_err(map)?;
    let buttons: Vec<Button> = (0..u16::from(caps.n_buttons)).map(from_crate).collect();
    let pointer = if caps.has_x || caps.has_y {
        Capability::native()
    } else {
        Capability::unsupported("the mouse behind the box doesn't move")
    };
    let clicky = if buttons.is_empty() {
        Capability::unsupported("there's no mouse with buttons behind the box")
    } else {
        Capability::native()
    };
    let clicky_locks = clicky.clone();
    let wheel = if caps.has_wheel {
        Capability::native()
    } else {
        Capability::unsupported("the mouse behind the box has no wheel")
    };
    let pan = if caps.pan {
        Capability::native()
    } else {
        Capability::unsupported("the mouse behind the box has no horizontal scroll")
    };
    let keys = if keyboard.n_keys > 0 {
        Capability::native()
    } else {
        Capability::unsupported("the device plugged into the box has no keyboard")
    };
    let media = if keyboard.has_consumer {
        Capability::native()
    } else {
        Capability::unsupported("the device plugged into the box has no media keys")
    };
    // CAPS says NKRO with 0xFF. otherwise the box has that many key slots and sends ErrorRollOver once they're full
    let key_press = if keyboard.nkro || keyboard.n_keys == 0xFF || keyboard.n_keys == 0 {
        keys.clone()
    } else {
        keys.clone().with(Quirk::MaxKeysHeld(keyboard.n_keys))
    };
    // the crate presses held buttons and keys again after a reconnect, and its author confirmed that's on purpose
    let capabilities = Capabilities::new(buttons)
        .set(Action::Move, pointer.clone())
        .set(Action::MoveNow, pointer)
        .set(Action::Press, clicky.clone().with(Quirk::KeepsHeld))
        .set(Action::KeyPress, key_press.with(Quirk::KeepsHeld))
        .set(Action::KeyRelease, keys.clone())
        .set(Action::MediaPress, media.clone().with(Quirk::KeepsHeld))
        .set(Action::MediaRelease, media.clone())
        .set(Action::Release, clicky)
        .set(Action::Wheel, wheel)
        .set(Action::Pan, pan)
        .set(Action::DiscardPendingMoves, Capability::native())
        .set(Action::ButtonEvents, Capability::native())
        .set(Action::MotionEvents, Capability::native())
        .set(Action::KeyEvents, keys.clone())
        .set(Action::MediaEvents, media);
    let axes: Vec<Axis> = [
        (caps.has_x, Axis::X),
        (caps.has_y, Axis::Y),
        (caps.has_wheel, Axis::Wheel),
        (caps.pan, Axis::Pan),
    ]
    .into_iter()
    .filter_map(|(has, axis)| has.then_some(axis))
    .collect();
    let axis_locks = if axes.is_empty() {
        Capability::unsupported("the device plugged into the box has no axis to lock")
    } else {
        Capability::native()
    };
    // the crate keeps its own lock table and sends it again after a reconnect or a session release on the box
    let capabilities = capabilities
        .with_axes(axes)
        .set(Action::LockButton, clicky_locks.with(Quirk::KeepsLocked))
        .set(
            Action::LockAxis,
            axis_locks.clone().with(Quirk::KeepsLocked),
        )
        .set(Action::ScaleAxis, axis_locks.with(Quirk::KeepsLocked))
        .set(Action::LockKey, keys.with(Quirk::KeepsLocked));
    let mut info = Info::new(Family::Medius);
    info.model = Some(version.name.clone());
    info.firmware = Some(Firmware::Version {
        major: u16::from(version.fw_major),
        minor: u16::from(version.fw_minor),
        patch: u16::from(version.fw_patch),
    });
    info.id = Some(StableId::Mac(version.mac));
    info.peripheral = device
        .device_info()
        .ok()
        .filter(medius::DeviceInfo::is_cloned)
        .map(|clone| UsbIds::new(clone.vid, clone.pid));
    Ok((
        Snapshot::new(info, capabilities),
        (
            health.mouse_attached,
            health.clone_configured,
            health.kbd_attached,
            all_caps,
        ),
    ))
}

/// the box can't time a press by itself, so the core builds clicks and taps out of a press and a release
fn untimed(action: Action) -> Error {
    Error::Unsupported {
        action,
        family: Family::Medius,
        reason: "the box has no timed press of its own, use a press and a release instead".into(),
    }
}

/// a release is never skipped, because the crate records a press even when its write fails and only a release
/// takes it out again. otherwise the crate would replay that press on every reconnect
enum Job {
    Send(Command),
    Streams(InputKinds),
    /// the crate ended one kind's subscription, so it gets a new one
    Restart(InputKind),
    Release(Usage),
}

impl Job {
    fn is_release(&self) -> bool {
        matches!(
            self,
            Job::Send(
                Command::Release(_)
                    | Command::KeyRelease(_)
                    | Command::MediaRelease(_)
                    | Command::Lock { keep: UNLOCKED, .. }
            ) | Job::Release(_)
        )
    }
}

struct Order {
    job: Job,
    given_up: Arc<AtomicBool>,
    done: Sender<Outcome>,
}

/// what a job did. only a stream job can have failed kinds, every other job fails as a whole
type Outcome = std::result::Result<(), StreamFailure>;

/// one subscription per kind, so switching one on or off never touches another kind's held state or its buffer
const KINDS: [InputKind; 4] = [
    InputKind::Buttons,
    InputKind::Motion,
    InputKind::Keys,
    InputKind::Media,
];

fn wants(kinds: InputKinds, kind: InputKind) -> bool {
    match kind {
        InputKind::Buttons => kinds.buttons,
        InputKind::Motion => kinds.motion,
        InputKind::Keys => kinds.keys,
        InputKind::Media => kinds.media,
    }
}

fn filter(kind: InputKind) -> CatchFilter {
    match kind {
        InputKind::Buttons => CatchFilter::watch_class(medius::Class::Button),
        InputKind::Motion => CatchFilter::watch_axes(),
        InputKind::Keys => CatchFilter::watch_class(medius::Class::Key),
        InputKind::Media => CatchFilter::watch_class(medius::Class::Media),
    }
}

/// one kind's subscription and its pump's view of it
#[derive(Default)]
struct Slot {
    /// the subscription and how many of its losses were already reported, they're always replaced together
    stream: Mutex<Option<(InputStream, u64)>>,
    /// set while the worker swaps the subscription. the pump grabs the stream lock again right after each wait
    /// and std's mutex isn't fair, so without this the swap could starve past its deadline
    swapping: AtomicBool,
}

/// everything the worker, the poller and the pump share
pub(crate) struct Shared {
    device: RwLock<Option<medius::Device>>,
    up: AtomicBool,
    stop: AtomicBool,
    sink: Sink,
    /// in the order of `KINDS`
    slots: [Slot; 4],
    /// what the box can do right now. the worker checks it again, because a reconnect can change it after the
    /// core already said yes
    snapshot: RwLock<Snapshot>,
    /// joined before the crate's device is dropped, because any of them can be holding a clone of it in the
    /// middle of a call
    threads: Mutex<Vec<thread::JoinHandle<()>>>,
    timeline: Mutex<Timeline>,
    /// a test holds this to keep the worker from running a job
    #[cfg(test)]
    pub(crate) hold: Mutex<()>,
}

impl Shared {
    pub(crate) fn device(&self) -> Option<medius::Device> {
        self.device
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    pub(crate) fn is_up(&self) -> bool {
        self.up.load(Ordering::Acquire)
    }

    /// marks the link as gone and tells the core, only once
    pub(crate) fn go_down(&self) {
        if self.up.swap(false, Ordering::AcqRel) {
            self.sink.disconnected();
        }
    }

    /// marked up before the core hears about it, because the core locks again and turns the streams back on
    /// right away, and while it's down only releases get through
    pub(crate) fn come_up(&self, snapshot: Snapshot) {
        *self
            .snapshot
            .write()
            .unwrap_or_else(PoisonError::into_inner) = snapshot.clone();
        self.reset_timeline();
        self.up.store(true, Ordering::Release);
        self.sink.reconnected(snapshot);
    }

    pub(crate) fn current_snapshot(&self) -> Snapshot {
        self.snapshot
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// the same device with different details, so the core only swaps the snapshot
    pub(crate) fn change_quietly(&self, snapshot: Snapshot) {
        *self
            .snapshot
            .write()
            .unwrap_or_else(PoisonError::into_inner) = snapshot.clone();
        self.sink.changed(snapshot);
    }

    pub(crate) fn reset_timeline(&self) {
        let mut timeline = lock(&self.timeline);
        for domain in ClockDomain::ALL {
            timeline.reset(domain);
        }
    }

    pub(crate) fn session_reset(&self) {
        self.reset_timeline();
        self.sink.session_reset();
    }

    pub(crate) fn dropped(&self, count: u64, source: DropSource) {
        self.sink.dropped(count, source);
    }

    /// a release always passes, one the box doesn't need just does nothing
    fn check(&self, job: &Job) -> Result<()> {
        let (action, button) = match *job {
            Job::Send(Command::Move(..)) => (Action::Move, None),
            Job::Send(Command::MoveNow(..)) => (Action::MoveNow, None),
            Job::Send(Command::Press(button)) => (Action::Press, Some(button)),
            Job::Send(Command::Wheel(_)) => (Action::Wheel, None),
            Job::Send(Command::Pan(_)) => (Action::Pan, None),
            Job::Send(Command::DiscardPendingMoves) => (Action::DiscardPendingMoves, None),
            Job::Send(Command::KeyPress(_)) => (Action::KeyPress, None),
            Job::Send(Command::MediaPress(_)) => (Action::MediaPress, None),
            // an unlock always passes, like a release
            Job::Send(Command::Lock { keep: UNLOCKED, .. }) => return Ok(()),
            Job::Send(Command::Lock { slot, keep }) => (
                slot.action(keep),
                match slot {
                    LockSlot::Button(button) => Some(button),
                    LockSlot::Key(_) | LockSlot::Axis(..) => None,
                },
            ),
            // the box has no timed press, `run` returns the error for these
            Job::Send(
                Command::Click { .. } | Command::KeyTap { .. } | Command::MediaTap { .. },
            ) => {
                return Ok(());
            }
            Job::Send(Command::Release(_) | Command::KeyRelease(_) | Command::MediaRelease(_))
            | Job::Release(_)
            | Job::Streams(_)
            | Job::Restart(_) => return Ok(()),
        };
        let snapshot = self.snapshot.read().unwrap_or_else(PoisonError::into_inner);
        let capabilities = &snapshot.capabilities;
        let supported = capabilities.get(action).is_supported()
            && button.is_none_or(|button| capabilities.buttons().contains(&button));
        if supported {
            Ok(())
        } else {
            Err(Error::Unsupported {
                action,
                family: Family::Medius,
                reason: "the box changed during this call, check its snapshot again".into(),
            })
        }
    }

    fn run(&self, job: &Job) -> Outcome {
        let kinds = match *job {
            Job::Streams(kinds) => kinds,
            Job::Restart(kind) => kind.kinds(),
            _ => InputKinds::default(),
        };
        let whole = |error| StreamFailure {
            failed: kinds,
            error,
        };
        self.check(job).map_err(whole)?;
        let Some(device) = self.device() else {
            return Err(whole(Error::Disconnected));
        };
        let sent = match *job {
            Job::Send(Command::Move(dx, dy)) => device.move_rel(dx, dy),
            Job::Send(Command::MoveNow(dx, dy)) => device.move_rel_now(dx, dy),
            Job::Send(Command::Press(button)) => device.press(to_crate(button)),
            Job::Send(Command::KeyPress(key)) => device.press(medius::Key(key.0)),
            Job::Send(Command::MediaPress(key)) => device.press(medius::MediaKey(key.0)),
            Job::Send(Command::Release(button)) => device.release(to_crate(button)),
            Job::Send(Command::KeyRelease(key)) => device.release(medius::Key(key.0)),
            Job::Send(Command::MediaRelease(key)) => device.release(medius::MediaKey(key.0)),
            Job::Release(usage) => device.release(usage_to_crate(usage)),
            Job::Send(Command::Wheel(notches)) => device.wheel(notches),
            Job::Send(Command::Pan(notches)) => device.pan(notches),
            Job::Send(Command::DiscardPendingMoves) => device.discard_motion(),
            Job::Send(Command::Click { .. }) => return Err(whole(untimed(Action::Click))),
            Job::Send(Command::KeyTap { .. }) => return Err(whole(untimed(Action::KeyTap))),
            Job::Send(Command::MediaTap { .. }) => return Err(whole(untimed(Action::MediaTap))),
            Job::Send(Command::Lock { slot, keep }) => {
                let both = medius::Direction::Both;
                match slot {
                    LockSlot::Button(button) => device.scale(to_crate(button), both, keep),
                    LockSlot::Key(key) => device.scale(medius::Key(key.0), both, keep),
                    LockSlot::Axis(axis, sign) => device.scale_axis(
                        axis_to_crate(axis),
                        match sign {
                            Sign::Positive => medius::Direction::Positive,
                            Sign::Negative => medius::Direction::Negative,
                        },
                        keep,
                    ),
                }
            }
            Job::Streams(kinds) => return self.switch(&device, kinds, false),
            Job::Restart(kind) => return self.switch(&device, kind.kinds(), true),
        };
        sent.map_err(|error| whole(self.map_error(error)))
    }

    fn map_error(&self, error: medius::Error) -> Error {
        if matches!(
            error,
            medius::Error::Io(_) | medius::Error::BadProtoVer { .. }
        ) {
            self.go_down();
            Error::Disconnected
        } else {
            Error::Io(std::io::Error::other(error))
        }
    }

    /// each kind on or off to match `kinds`, and a kind that's already right keeps its subscription. with `only`
    /// the other kinds are left alone, for bringing back the one the crate ended (the pump already cleared its slot,
    /// and a live one is kept so a new subscription never shows a held button again). one kind failing doesn't stop
    /// the others
    fn switch(&self, device: &medius::Device, kinds: InputKinds, only: bool) -> Outcome {
        let mut failed = InputKinds::default();
        let mut first = None;
        for (index, kind) in KINDS.into_iter().enumerate() {
            let slot = &self.slots[index];
            let on = wants(kinds, kind);
            if only && !on {
                continue;
            }
            // even a look at the slot goes through the flag, the pump holds the lock while it waits for events
            slot.swapping.store(true, Ordering::Release);
            let mut stream = lock(&slot.stream);
            slot.swapping.store(false, Ordering::Release);
            if on == stream.is_some() {
                continue;
            }
            // the old one goes first, otherwise two subscriptions would double every event
            *stream = None;
            if !on {
                continue;
            }
            match device.input_events([filter(kind)]) {
                Ok(subscribed) => *stream = Some((subscribed, 0)),
                Err(error) => {
                    let one = kind.kinds();
                    failed.buttons |= one.buttons;
                    failed.motion |= one.motion;
                    failed.keys |= one.keys;
                    failed.media |= one.media;
                    let error = self.map_error(error);
                    if matches!(error, Error::Disconnected) {
                        return Err(StreamFailure {
                            failed: kinds,
                            error,
                        });
                    }
                    first.get_or_insert(error);
                }
            }
        }
        match first {
            None => Ok(()),
            Some(error) => Err(StreamFailure { failed, error }),
        }
    }
}

pub(crate) struct MediusBackend {
    shared: Arc<Shared>,
    orders: Mutex<Option<Sender<Order>>>,
}

/// builds the backend on a crate device that already passed its handshake. tests call this with the crate's
/// mock box, so everything after the handshake runs the real code
pub(crate) fn build(
    device: medius::Device,
    sink: Sink,
) -> Result<(MediusBackend, Snapshot, Poller)> {
    let (snapshot, clone) = snapshot(&device)?;
    let baseline = Poller::baseline(&device);
    let shared = Arc::new(Shared {
        device: RwLock::new(Some(device)),
        up: AtomicBool::new(true),
        stop: AtomicBool::new(false),
        sink,
        slots: Default::default(),
        snapshot: RwLock::new(snapshot.clone()),
        threads: Mutex::new(Vec::new()),
        timeline: Mutex::new(Timeline::new()),
        #[cfg(test)]
        hold: Mutex::new(()),
    });
    let (orders, inbox) = mpsc::channel();
    let worker = {
        let shared = shared.clone();
        thread::Builder::new()
            .name("anybox-medius-worker".into())
            .spawn(move || work(&shared, &inbox))?
    };
    lock(&shared.threads).push(worker);
    for index in 0..KINDS.len() {
        let pumper = {
            let shared = shared.clone();
            thread::Builder::new()
                .name("anybox-medius-pump".into())
                .spawn(move || pump(&shared, index))?
        };
        lock(&shared.threads).push(pumper);
    }
    let poller = Poller::new(shared.clone(), baseline, clone);
    Ok((
        MediusBackend {
            shared,
            orders: Mutex::new(Some(orders)),
        },
        snapshot,
        poller,
    ))
}

fn work(shared: &Shared, inbox: &Receiver<Order>) {
    for order in inbox.iter() {
        #[cfg(test)]
        let _held = lock(&shared.hold);
        if order.given_up.load(Ordering::Acquire) && !order.job.is_release() {
            continue;
        }
        let _ = order.done.send(shared.run(&order.job));
    }
}

fn pump(shared: &Shared, index: usize) {
    let slot = &shared.slots[index];
    while !shared.stop.load(Ordering::Acquire) {
        if slot.swapping.load(Ordering::Acquire) {
            thread::sleep(Duration::from_millis(1));
            continue;
        }
        let mut stream = lock(&slot.stream);
        let Some((current, reported)) = stream.as_mut() else {
            drop(stream);
            thread::sleep(PUMP_SLICE);
            continue;
        };
        let event = current.recv_timeout(PUMP_SLICE);
        // before the timeline, which smooths the box clock and isn't when anybox got the event
        let received = Read::now();
        // a crate reconnect keeps the subscriptions, only the crate's reset() or factory_reset() through medius()
        // ends them
        if event.is_none() && !current.is_connected() {
            *stream = None;
            drop(stream);
            shared.sink.stream_lost(KINDS[index]);
            continue;
        }
        let lost = current.dropped();
        let newly_lost = lost.saturating_sub(*reported);
        *reported = lost;
        drop(stream);
        if newly_lost > 0 {
            shared.sink.lost(newly_lost, KINDS[index]);
        }
        if let Some(event) = event {
            deliver(shared, &event, received);
        }
    }
}

fn deliver(shared: &Shared, event: &InputEvent, received: Read) {
    let input = match event.input {
        medius::Input::Press(usage) | medius::Input::Release(usage)
            if usage.class == medius::Class::Button =>
        {
            Input::Button {
                button: from_crate(usage.id),
                pressed: event.input.is_press(),
            }
        }
        // 0x01..0x03 are the HID error codes a keyboard sends past its rollover, and everything from 0xE8 up is
        // undefined
        medius::Input::Press(usage) | medius::Input::Release(usage)
            if usage.class == medius::Class::Key =>
        {
            let key = match u8::try_from(usage.id) {
                Ok(id) if Key(id).is_valid() => Key(id),
                _ => return,
            };
            Input::Key {
                key,
                pressed: event.input.is_press(),
            }
        }
        medius::Input::Press(usage) | medius::Input::Release(usage)
            if usage.class == medius::Class::Media && usage.id != 0 =>
        {
            Input::Media {
                key: MediaKey(usage.id),
                pressed: event.input.is_press(),
            }
        }
        // the crate already drops reports with no motion at all
        medius::Input::Motion { dx, dy, dz, pan } => Input::Motion {
            dx: i32::from(dx),
            dy: i32::from(dy),
            wheel: i32::from(dz),
            pan: i32::from(pan),
        },
        _ => return,
    };
    let clock = match event.clock {
        ClockDomain::HostChip => Clock::MediusHostChip,
        ClockDomain::DeviceChip => Clock::MediusDeviceChip,
    };
    let arrived = lock(&shared.timeline).observe_at(event, received.at).host;
    shared.sink.input(
        input,
        arrived,
        Some(BoxTime {
            micros: event.ts_us,
            clock,
        }),
        Some(received),
    );
}

impl MediusBackend {
    /// the crate's own device for this box, for everything only Medius does
    pub(crate) fn device(&self) -> Option<medius::Device> {
        self.shared.device()
    }

    /// queues a release nobody waits for, the worker runs it whenever it gets to it
    fn enqueue(&self, job: Job) {
        let (done, _) = mpsc::channel();
        if let Some(orders) = lock(&self.orders).as_ref() {
            let _ = orders.send(Order {
                job,
                given_up: Arc::new(AtomicBool::new(true)),
                done,
            });
        }
    }

    fn order(&self, job: Job) -> Result<()> {
        self.order_streams(job).map_err(|failure| failure.error)
    }

    fn order_streams(&self, job: Job) -> Outcome {
        let kinds = match job {
            Job::Streams(kinds) => kinds,
            Job::Restart(kind) => kind.kinds(),
            _ => InputKinds::default(),
        };
        let gone = || StreamFailure {
            failed: kinds,
            error: Error::Disconnected,
        };
        if !self.shared.is_up() && !job.is_release() {
            return Err(gone());
        }
        let (done, answer) = mpsc::channel();
        let given_up = Arc::new(AtomicBool::new(false));
        let order = Order {
            job,
            given_up: given_up.clone(),
            done,
        };
        match lock(&self.orders).as_ref() {
            Some(orders) if orders.send(order).is_ok() => {}
            _ => return Err(gone()),
        }
        match answer.recv_timeout(WRITE_DEADLINE) {
            Ok(result) => result,
            Err(_) => {
                given_up.store(true, Ordering::Release);
                // still stuck after the deadline means the crate is reconnecting, so we go down and the poller
                // brings it back
                self.shared.go_down();
                Err(gone())
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn shared(&self) -> &Arc<Shared> {
        &self.shared
    }
}

impl Backend for MediusBackend {
    fn send(&self, command: Command) -> Result<()> {
        self.order(Job::Send(command))
    }

    fn set_streams(&self, kinds: InputKinds) -> Outcome {
        self.order_streams(Job::Streams(kinds))
    }

    fn restart_stream(&self, kind: InputKind) -> Result<()> {
        self.order(Job::Restart(kind))
    }

    /// a soft release for each one and never the crate's reset, because that would clear the locks and transforms
    /// too. once one runs out of time, the rest get queued without waiting
    fn release_all(&self, held: &[Usage]) -> Result<()> {
        let mut result = Ok(());
        for &usage in held {
            if matches!(result, Err(Error::Disconnected)) {
                self.enqueue(Job::Release(usage));
                continue;
            }
            if let Err(error) = self.order(Job::Release(usage)) {
                result = Err(error);
            }
        }
        result
    }

    /// once a send runs out of time, the rest get queued without waiting. the worker still runs them, otherwise a
    /// crate clone the app kept would keep them locked
    fn unlock_all(&self, slots: &[LockSlot]) -> Result<()> {
        let mut first = Ok(());
        for &slot in slots {
            let job = Job::Send(Command::Lock {
                slot,
                keep: UNLOCKED,
            });
            if matches!(first, Err(Error::Disconnected)) {
                self.enqueue(job);
                continue;
            }
            let unlocked = self.order(job);
            if first.is_ok() || matches!(unlocked, Err(Error::Disconnected)) {
                first = unlocked;
            }
        }
        first
    }

    fn shutdown(&self, deadline: Duration) {
        self.shared.stop.store(true, Ordering::Release);
        lock(&self.orders).take();
        let shared = self.shared.clone();
        let (done, finished) = mpsc::channel();
        // the crate joins its reader with no timeout while it reconnects, so the last drop happens on another
        // thread and this one only waits until the deadline
        let helper = thread::Builder::new()
            .name("anybox-medius-close".into())
            .spawn(move || {
                let threads = std::mem::take(&mut *lock(&shared.threads));
                for handle in threads {
                    let _ = handle.join();
                }
                for slot in &shared.slots {
                    lock(&slot.stream).take();
                }
                shared
                    .device
                    .write()
                    .unwrap_or_else(PoisonError::into_inner)
                    .take();
                let _ = done.send(());
            });
        if helper.is_ok() {
            let _ = finished.recv_timeout(deadline);
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Poller {
    pub(crate) fn spawn(self) -> std::io::Result<()> {
        let shared = self.shared.clone();
        let handle = thread::Builder::new()
            .name("anybox-medius-poller".into())
            .spawn(move || {
                let mut poller = self;
                let mut next = Instant::now();
                while !poller.stopped() {
                    poller.tick();
                    next += POLL_EVERY;
                    // short naps so a shutdown never waits a whole period for this thread
                    while Instant::now() < next && !poller.stopped() {
                        thread::sleep(Duration::from_millis(10));
                    }
                }
            })?;
        lock(&shared.threads).push(handle);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
