use std::collections::VecDeque;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock, PoisonError, Weak};
use std::time::{Duration, Instant};

use crate::capability::{Action, Button};
use crate::key::{Key, MediaKey};

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Event {
    /// a physical button
    #[non_exhaustive]
    Button {
        button: Button,
        pressed: bool,
        at: Stamp,
    },
    /// a physical key on the keyboard behind the box. just know that you see every key typed there, passwords
    /// included
    #[non_exhaustive]
    Key {
        key: Key,
        pressed: bool,
        at: Stamp,
    },
    /// a physical media key, like volume or play
    #[non_exhaustive]
    Media {
        key: MediaKey,
        pressed: bool,
        at: Stamp,
    },
    /// physical motion. the wheel is positive going up and pan (horizontal scroll) is positive going right
    #[non_exhaustive]
    Motion {
        dx: i32,
        dy: i32,
        wheel: i32,
        pan: i32,
        at: Stamp,
    },
    /// how many events never reached you since the last `Dropped` from the same source. one from the box can mean
    /// it restarted a stream, so what you knew about which physical buttons and keys were held may be out of date.
    /// the one for your own queue (`DropSource::Host`) sits at the front of the queue, not where the loss happened,
    /// and on Medius the one from the box comes from a box wide counter read about once a second, so it can also
    /// count what your own subscriptions through `medius()` lost
    #[non_exhaustive]
    Dropped {
        count: u64,
        source: DropSource,
    },
    /// the box lost its state, so the buttons and keys you were holding got released (on a box with
    /// `Quirk::KeepsHeld` they stay held). what you knew about which physical buttons and keys were held is out of
    /// date too, because one held through the reset can show a second press or never show its release
    SessionReset,
    /// the box refused something after the call already returned ok, MAKCU does that when its command queue is
    /// full. slow down and send it again if it mattered. `count` is how many of that action got refused since you
    /// last read one, so a flood of refusals never pushes your input out of the queue
    #[non_exhaustive]
    Rejected {
        action: Action,
        count: u64,
    },
    Disconnected,
    /// the box is back, the buttons and keys you were holding got released (on a box with `Quirk::KeepsHeld`
    /// they stay held), and the snapshot may have changed. on a MAKCU you also get this when a mouse or keyboard
    /// gets plugged into the box or pulled out, and on a Medius when the device it clones gets swapped. what you
    /// knew about which physical buttons and keys were held is out of date, same as after `SessionReset`
    Reconnected,
    /// the snapshot changed with no reconnect, so read `snapshot()` again. a mouse or keyboard set or pulled on a
    /// Ferrum, a YSC turning its keys off, or a Medius clone whose details changed come this way. so does a Ferrum
    /// unplugged from this PC, because the app's port stays open and the app just loses its devices, and once it's
    /// back you get a `SessionReset` after this one, since the box forgot its locks. a MAKCU gets a `Reconnected`
    /// for those instead, so watch both. two of these never wait unread in the same receiver
    Changed,
}

/// for tests and for code that builds events itself. the variants stay `#[non_exhaustive]` so a field added
/// later doesn't break anyone, and these are how you make them from outside
impl Event {
    pub fn button(button: Button, pressed: bool, at: Stamp) -> Self {
        Self::Button {
            button,
            pressed,
            at,
        }
    }

    pub fn key(key: Key, pressed: bool, at: Stamp) -> Self {
        Self::Key { key, pressed, at }
    }

    pub fn media(key: MediaKey, pressed: bool, at: Stamp) -> Self {
        Self::Media { key, pressed, at }
    }

    pub fn motion(dx: i32, dy: i32, wheel: i32, pan: i32, at: Stamp) -> Self {
        Self::Motion {
            dx,
            dy,
            wheel,
            pan,
            at,
        }
    }

    pub fn dropped(count: u64, source: DropSource) -> Self {
        Self::Dropped { count, source }
    }

    pub fn rejected(action: Action, count: u64) -> Self {
        Self::Rejected { action, count }
    }
}

#[cfg(feature = "mock")]
impl Event {
    /// one of each event with every field it has filled in, for testing how code handles them with no box. the
    /// input ones carry `at` as their host time and a fixed box time
    pub fn examples(at: Stamp) -> Vec<Event> {
        let stamped = at.with_box_time(BoxTime::new(4_000_000_000, Clock::MediusDeviceChip));
        vec![
            Event::button(Button::Side2, true, stamped),
            Event::key(Key::LEFT_SHIFT, false, stamped),
            Event::media(MediaKey::VOLUME_DOWN, true, stamped),
            Event::motion(-3, 7, -1, 2, stamped),
            Event::dropped(12, DropSource::Device),
            Event::dropped(3, DropSource::Host),
            Event::SessionReset,
            Event::rejected(Action::Move, 4),
            Event::Disconnected,
            Event::Reconnected,
            Event::Changed,
        ]
    }
}

/// which input a receiver wants, combined with `|`. a receiver only gets what it asked for, the box streams what
/// any receiver wants, and connection events and drops reach every receiver
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct EventKinds {
    buttons: bool,
    motion: bool,
    keys: bool,
    media: bool,
}

impl EventKinds {
    pub const NONE: Self = Self {
        buttons: false,
        motion: false,
        keys: false,
        media: false,
    };
    pub const BUTTONS: Self = Self {
        buttons: true,
        ..Self::NONE
    };
    pub const MOTION: Self = Self {
        motion: true,
        ..Self::NONE
    };
    pub const KEYS: Self = Self {
        keys: true,
        ..Self::NONE
    };
    pub const MEDIA: Self = Self {
        media: true,
        ..Self::NONE
    };
    /// everything the box can stream, what `events()` gives you
    pub const ALL: Self = Self {
        buttons: true,
        motion: true,
        keys: true,
        media: true,
    };

    pub fn contains(self, other: Self) -> bool {
        self & other == other
    }

    pub fn is_empty(self) -> bool {
        self == Self::NONE
    }

    /// the kind an event belongs to, none for the ones every receiver gets
    pub(crate) fn of(event: &Event) -> Option<Self> {
        match event {
            Event::Button { .. } => Some(Self::BUTTONS),
            Event::Motion { .. } => Some(Self::MOTION),
            Event::Key { .. } => Some(Self::KEYS),
            Event::Media { .. } => Some(Self::MEDIA),
            Event::Rejected { action, .. } => Self::of_action(*action),
            _ => None,
        }
    }

    /// the kind a stream's action is about, so its refusal only goes to the receivers that wanted it
    pub(crate) fn of_action(action: Action) -> Option<Self> {
        match action {
            Action::ButtonEvents => Some(Self::BUTTONS),
            Action::MotionEvents => Some(Self::MOTION),
            Action::KeyEvents => Some(Self::KEYS),
            Action::MediaEvents => Some(Self::MEDIA),
            _ => None,
        }
    }

    pub(crate) fn to_input(self) -> crate::backend::InputKinds {
        crate::backend::InputKinds {
            buttons: self.buttons,
            motion: self.motion,
            keys: self.keys,
            media: self.media,
        }
    }

    pub(crate) fn from_input(kinds: crate::backend::InputKinds) -> Self {
        Self {
            buttons: kinds.buttons,
            motion: kinds.motion,
            keys: kinds.keys,
            media: kinds.media,
        }
    }

    /// each single kind in this set with the action that streams it
    pub(crate) fn each(self) -> impl Iterator<Item = (Self, Action)> {
        [
            (Self::BUTTONS, Action::ButtonEvents),
            (Self::MOTION, Action::MotionEvents),
            (Self::KEYS, Action::KeyEvents),
            (Self::MEDIA, Action::MediaEvents),
        ]
        .into_iter()
        .filter(move |(kind, _)| self.contains(*kind))
    }
}

impl std::ops::BitOr for EventKinds {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self {
            buttons: self.buttons || other.buttons,
            motion: self.motion || other.motion,
            keys: self.keys || other.keys,
            media: self.media || other.media,
        }
    }
}

impl std::ops::BitAnd for EventKinds {
    type Output = Self;

    fn bitand(self, other: Self) -> Self {
        Self {
            buttons: self.buttons && other.buttons,
            motion: self.motion && other.motion,
            keys: self.keys && other.keys,
            media: self.media && other.media,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum DropSource {
    /// the box's own counters
    Device,
    /// your receiver fell behind and its queue overflowed
    Host,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Stamp {
    /// the best guess at when the event happened, on the PC's clock. on Medius it's the box clock mapped onto the
    /// PC's through the medius crate, which keeps it from ever going backwards and smooths it after bursts, so it's
    /// an estimate there and NOT when the PC got the event, use `received` for that
    pub arrived: Instant,
    /// the same moment as the time since the device was opened, for code that can't use `Instant`
    pub since_open: Duration,
    pub box_time: Option<BoxTime>,
    /// when anybox got the event. on MAKCU that's right after the port read returned with the report's last byte,
    /// on Medius it's when anybox took it from the medius crate, which reads the port on its own thread, so it's a
    /// bit after the read there. either way it's the read and not the USB arrival, driver buffering and a backlog
    /// make it late, and reports that came in one read share it
    pub received: Instant,
    /// the same moment as `received`, as the time since the device was opened
    pub received_since_open: Duration,
    /// the raw QueryPerformanceCounter value at `received`, on Windows only, since an `Instant` can't be turned
    /// back into one and a USB capture stamps its packets in QPC
    pub received_qpc: Option<i64>,
}

impl Stamp {
    /// for tests and for code that builds events itself. `received` starts out as `arrived`, and `since_open`
    /// isn't checked against `arrived`
    pub fn new(arrived: Instant, since_open: Duration) -> Self {
        Self {
            arrived,
            since_open,
            box_time: None,
            received: arrived,
            received_since_open: since_open,
            received_qpc: None,
        }
    }

    pub fn with_box_time(mut self, box_time: BoxTime) -> Self {
        self.box_time = Some(box_time);
        self
    }

    /// `arrived` in nanoseconds on the clock `monotonic_ns` reads, so it lines up with stamps from other devices
    /// and other programs on this machine
    pub fn arrived_ns(&self) -> u64 {
        instant_ns(self.arrived)
    }

    /// `received` in nanoseconds on the clock `monotonic_ns` reads
    pub fn received_ns(&self) -> u64 {
        instant_ns(self.received)
    }

    /// `received_since_open` comes out of `received` and `since_open`, so set those right first
    pub fn with_received(mut self, received: Instant, qpc: Option<i64>) -> Self {
        self.received = received;
        self.received_since_open = if received >= self.arrived {
            self.since_open
                .saturating_add(received.duration_since(self.arrived))
        } else {
            self.since_open
                .saturating_sub(self.arrived.duration_since(received))
        };
        self.received_qpc = qpc;
        self
    }
}

/// now in nanoseconds on the OS monotonic clock, the one `Instant` reads: QueryPerformanceCounter on Windows,
/// CLOCK_MONOTONIC on Linux and CLOCK_UPTIME_RAW on macOS. Python's `time.perf_counter_ns()` and .NET's `Stopwatch`
/// read the same one on Windows and Linux. it only compares on the same machine within one boot
pub fn monotonic_ns() -> u64 {
    crate::transport::monotonic_ns()
}

/// `Instant` hides its raw value, so this is off by the anchor's bracket, which is tens of nanoseconds unless
/// every sample got preempted
fn instant_ns(at: Instant) -> u64 {
    let (anchor, anchor_ns) = anchor();
    if at >= anchor {
        anchor_ns.saturating_add(nanos(at.duration_since(anchor)))
    } else {
        anchor_ns.saturating_sub(nanos(anchor.duration_since(at)))
    }
}

/// an `Instant` and the clock read around it, the tightest of a few brackets so one preempted sample doesn't count
fn anchor() -> (Instant, u64) {
    static ANCHOR: OnceLock<(Instant, u64)> = OnceLock::new();
    *ANCHOR.get_or_init(|| {
        let mut best = (u64::MAX, Instant::now(), 0);
        for _ in 0..16 {
            let before = monotonic_ns();
            let at = Instant::now();
            let after = monotonic_ns();
            let bracket = after.saturating_sub(before);
            if bracket < best.0 {
                best = (bracket, at, before + bracket / 2);
            }
        }
        (best.1, best.2)
    })
}

fn nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct BoxTime {
    pub micros: u32,
    pub clock: Clock,
}

impl BoxTime {
    pub fn new(micros: u32, clock: Clock) -> Self {
        Self { micros, clock }
    }
}

/// each box clock is its own counter, so only compare stamps that come from the same one
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Clock {
    MediusHostChip,
    MediusDeviceChip,
}

impl Clock {
    /// a u32 microsecond counter rolls over every 71.6 minutes
    pub fn wraps_after(self) -> Duration {
        Duration::from_micros(u64::from(u32::MAX) + 1)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Closed;

impl fmt::Display for Closed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the device is gone, no more events will come")
    }
}

impl std::error::Error for Closed {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecvTimeoutError {
    Timeout,
    Closed,
}

impl fmt::Display for RecvTimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Timeout => f.write_str("no event came in time"),
            Self::Closed => Closed.fmt(f),
        }
    }
}

impl std::error::Error for RecvTimeoutError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TryRecvError {
    Empty,
    Closed,
}

impl fmt::Display for TryRecvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("no event is waiting"),
            Self::Closed => Closed.fmt(f),
        }
    }
}

impl std::error::Error for TryRecvError {}

pub(crate) const QUEUE_CAPACITY: usize = 1024;

#[derive(Debug)]
struct State {
    events: VecDeque<Event>,
    closed: bool,
}

/// one receiver's queue. when it's full the oldest events get thrown out and one `Dropped` at the front counts
/// them, so a busy producer can never push that report out. only input events get thrown out, never a `Dropped`
/// from the box or a disconnect or reconnect, so a slow reader still finds out that what it held got let go
#[derive(Debug)]
pub(crate) struct Queue {
    state: Mutex<State>,
    ready: Condvar,
    capacity: usize,
}

impl Queue {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            state: Mutex::new(State {
                events: VecDeque::new(),
                closed: false,
            }),
            ready: Condvar::new(),
            capacity: capacity.max(2),
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(crate) fn push(&self, event: Event) {
        let mut state = self.lock();
        if state.closed {
            return;
        }
        let events = &mut state.events;
        // the snapshot is read live, so one unread is as good as many, and these never get evicted
        if matches!(event, Event::Changed) && events.iter().any(|e| matches!(e, Event::Changed)) {
            return;
        }
        if let Event::Rejected { action, count } = event {
            // an unread refusal of the same action just gets its count bumped, so refusals take a few slots at most
            let unread = events.iter_mut().find_map(|queued| match queued {
                Event::Rejected {
                    action: queued_action,
                    count: queued_count,
                } if *queued_action == action => Some(queued_count),
                _ => None,
            });
            if let Some(queued_count) = unread {
                *queued_count += count;
                return;
            }
        }
        if events.len() >= self.capacity {
            let counted = matches!(
                events.front(),
                Some(Event::Dropped {
                    source: DropSource::Host,
                    ..
                })
            );
            if !counted {
                events.push_front(Event::Dropped {
                    count: 0,
                    source: DropSource::Host,
                });
            }
            let motion_queued = events.iter().any(|e| matches!(e, Event::Motion { .. }));
            if matches!(event, Event::Motion { .. }) && !motion_queued {
                // a queue full of presses and releases keeps them and the new motion gets thrown out instead.
                // the `Dropped` at the front still needs a slot though, so the oldest press or release makes room
                // for it once
                let mut lost = 1;
                if events.len() > self.capacity
                    && let Some(oldest) = events.iter().skip(1).position(|e| {
                        matches!(
                            e,
                            Event::Button { .. } | Event::Key { .. } | Event::Media { .. }
                        )
                    })
                {
                    events.remove(oldest + 1);
                    lost += 1;
                }
                if let Some(Event::Dropped { count, .. }) = events.front_mut() {
                    *count += lost;
                }
                drop(state);
                self.ready.notify_all();
                return;
            }
            while events.len() >= self.capacity {
                // motion gets thrown out first, otherwise a flood of it could push out the release of a key that's
                // still held
                let motion = events
                    .iter()
                    .skip(1)
                    .position(|e| matches!(e, Event::Motion { .. }));
                let edge = || {
                    events.iter().skip(1).position(|e| {
                        matches!(
                            e,
                            Event::Button { .. } | Event::Key { .. } | Event::Media { .. }
                        )
                    })
                };
                let Some(oldest) = motion.or_else(edge) else {
                    break;
                };
                events.remove(oldest + 1);
                if let Some(Event::Dropped { count, .. }) = events.front_mut() {
                    *count += 1;
                }
            }
        }
        events.push_back(event);
        drop(state);
        self.ready.notify_all();
    }

    pub(crate) fn close(&self) {
        self.lock().closed = true;
        self.ready.notify_all();
    }

    /// the receiver's own close, which throws out what's queued instead of leaving it to be read
    fn close_now(&self) {
        let mut state = self.lock();
        state.events.clear();
        state.closed = true;
        drop(state);
        self.ready.notify_all();
    }

    fn try_pop(state: &mut State) -> Result<Event, TryRecvError> {
        match state.events.pop_front() {
            Some(event) => Ok(event),
            None if state.closed => Err(TryRecvError::Closed),
            None => Err(TryRecvError::Empty),
        }
    }
}

/// lets a receiver tell the device when it's dropped, so the box only streams input while someone listens
pub(crate) trait Listeners: Send + Sync {
    fn receiver_dropped(&self, id: u64);
}

/// a receiver of events from one device. it ends once the device is gone and you've read everything queued, or
/// once you close it
pub struct Events {
    queue: Arc<Queue>,
    listeners: Weak<dyn Listeners>,
    id: u64,
    // the device hears about a receiver going once, whether that's a close, a drop or both
    gone: AtomicBool,
}

impl Events {
    pub(crate) fn new(queue: Arc<Queue>, listeners: Weak<dyn Listeners>, id: u64) -> Self {
        Self {
            queue,
            listeners,
            id,
            gone: AtomicBool::new(false),
        }
    }

    /// ends this receiver now, from any thread, so share it in an `Arc` when another thread reads it. a `recv`
    /// waiting on it returns `Closed` right away, whatever was queued is thrown out, and the box stops streaming
    /// what only this receiver wanted. a second close does nothing
    pub fn close(&self) {
        // the queue goes first, so a reader wakes without waiting on the device
        self.queue.close_now();
        self.leave();
    }

    fn leave(&self) {
        if !self.gone.swap(true, Ordering::AcqRel)
            && let Some(listeners) = self.listeners.upgrade()
        {
            listeners.receiver_dropped(self.id);
        }
    }

    pub fn recv(&self) -> Result<Event, Closed> {
        let mut state = self.queue.lock();
        loop {
            match Queue::try_pop(&mut state) {
                Ok(event) => return Ok(event),
                Err(TryRecvError::Closed) => return Err(Closed),
                Err(TryRecvError::Empty) => {
                    state = self
                        .queue
                        .ready
                        .wait(state)
                        .unwrap_or_else(PoisonError::into_inner);
                }
            }
        }
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Result<Event, RecvTimeoutError> {
        // a timeout too long for the clock to add is the same as waiting forever
        let Some(deadline) = Instant::now().checked_add(timeout) else {
            return self.recv().map_err(|Closed| RecvTimeoutError::Closed);
        };
        let mut state = self.queue.lock();
        loop {
            match Queue::try_pop(&mut state) {
                Ok(event) => return Ok(event),
                Err(TryRecvError::Closed) => return Err(RecvTimeoutError::Closed),
                Err(TryRecvError::Empty) => {
                    let left = deadline.saturating_duration_since(Instant::now());
                    if left.is_zero() {
                        return Err(RecvTimeoutError::Timeout);
                    }
                    state = self
                        .queue
                        .ready
                        .wait_timeout(state, left)
                        .unwrap_or_else(PoisonError::into_inner)
                        .0;
                }
            }
        }
    }

    pub fn try_recv(&self) -> Result<Event, TryRecvError> {
        Queue::try_pop(&mut self.queue.lock())
    }
}

impl Iterator for Events {
    type Item = Event;

    fn next(&mut self) -> Option<Event> {
        self.recv().ok()
    }
}

impl Drop for Events {
    fn drop(&mut self) {
        self.leave();
    }
}

impl fmt::Debug for Events {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Events").finish_non_exhaustive()
    }
}

impl std::ops::Not for EventKinds {
    type Output = Self;

    fn not(self) -> Self {
        Self {
            buttons: !self.buttons,
            motion: !self.motion,
            keys: !self.keys,
            media: !self.media,
        }
    }
}

#[cfg(test)]
mod tests;
