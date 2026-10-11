use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, Weak};
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
    /// it restarted a stream, so what you knew about which physical buttons and keys were held may be out of date
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
    /// they stay held), and the snapshot may have changed. you also get this when a mouse or keyboard gets plugged
    /// into the box or pulled out. what you knew about which physical buttons and keys were held is out of date,
    /// same as after `SessionReset`
    Reconnected,
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
    /// when the event reached the PC. on Medius it's the box clock mapped onto the PC's, so it's only an estimate
    /// there
    pub arrived: Instant,
    /// the same moment as the time since the device was opened, for code that can't use `Instant`
    pub since_open: Duration,
    pub box_time: Option<BoxTime>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct BoxTime {
    pub micros: u32,
    pub clock: Clock,
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
    fn receiver_dropped(&self);
}

/// a receiver of events from one device. it ends once the device is gone and you've read everything queued
pub struct Events {
    queue: Arc<Queue>,
    listeners: Weak<dyn Listeners>,
}

impl Events {
    pub(crate) fn new(queue: Arc<Queue>, listeners: Weak<dyn Listeners>) -> Self {
        Self { queue, listeners }
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
        let deadline = Instant::now() + timeout;
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
        if let Some(listeners) = self.listeners.upgrade() {
            listeners.receiver_dropped();
        }
    }
}

impl fmt::Debug for Events {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Events").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;
