use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use super::Stream;
use super::frames::{self, frame, parts};
use super::{Expect, Shared, kind_byte, lock, snapshot};
use crate::backend::Sink;
use crate::capability::{Action, Button, Firmware, Snapshot};
use crate::event::DropSource;
use crate::transport::link::Link;

const PAUSE: Duration = Duration::from_millis(250);
const NAP: Duration = Duration::from_millis(10);
/// a GET can get its answer before a press that's still waiting in the box's queue, so one tick on its own isn't
/// a reset
const MISSES_FOR_RESET: u32 = 2;
/// a lock the box refuses this many times in a row is left alone until the app writes it again, so it can't flood
/// `Rejected`
const MAX_LOCK_RETRIES: u8 = 3;

/// a box reset doesn't drop the COM port (the CH343 is its own chip), so this polls the box. one `tick` is one
/// pass, the thread runs them one after another with a pause in between and tests just call it directly
pub(crate) struct Poller {
    shared: Arc<Shared>,
    link: Arc<Link>,
    sink: Sink,
    misses: u32,
    ticks: u64,
    /// per LOCK target, the app's write count the last time this poller looked and how many times it was sent
    /// again since, so a target the box keeps refusing stops after three
    lock_retries: [(u32, u8); 14],
}

impl Poller {
    pub(crate) fn new(shared: Arc<Shared>, link: Arc<Link>, sink: Sink) -> Self {
        Self {
            shared,
            link,
            sink,
            misses: 0,
            ticks: 0,
            lock_retries: [(0, 0); 14],
        }
    }

    pub(crate) fn spawn(mut self) -> std::io::Result<JoinHandle<()>> {
        thread::Builder::new()
            .name("anybox-makcu-poller".into())
            .spawn(move || {
                while !self.stopped() {
                    self.tick();
                    let until = Instant::now() + PAUSE;
                    while Instant::now() < until && !self.stopped() {
                        thread::sleep(NAP);
                    }
                }
            })
    }

    fn stopped(&self) -> bool {
        self.shared.stop.load(Ordering::Acquire)
    }

    /// a missed reply already takes the link down inside `request`, so a failed GET just ends the tick
    fn get(&self, command: u8, payload: &[u8]) -> Option<Vec<u8>> {
        let reply = self
            .link
            .request(frame(command, payload), Some(u32::from(command)))
            .ok()??;
        let (_, payload) = parts(&reply)?;
        (!frames::refused(payload)).then(|| payload.to_vec())
    }

    /// whether the box says this stream is on, or none when it didn't answer. it runs under the switch lock
    /// because INPUT_STREAM replies don't say the kind, so a request for another kind could take its reply
    fn stream_on(&self, kind: Stream) -> Option<bool> {
        let _switching = lock(&self.shared.streams);
        self.get(frames::INPUT_STREAM, &[kind_byte(kind)])
            .map(|reply| reply.first() == Some(&1))
    }

    pub(crate) fn tick(&mut self) {
        let (generation, wanted) = {
            let mut expect = lock(&self.shared.expect);
            let wanted =
                [Stream::Mouse, Stream::Keyboard].map(|kind| (kind, expect.stream(kind).wanted));
            (expect.generation, wanted)
        };
        // a kind that didn't answer just gets skipped this tick, the retries and the route check still run
        let on =
            wanted.map(|(kind, wanted)| (kind, wanted.then(|| self.stream_on(kind)).flatten()));
        // a piece of text that types while this tick reads the snapshot would make the key it's typing look like a
        // lost release
        let pieces = self.shared.pieces.load(Ordering::Acquire);
        let Some(injected) = self.get(frames::INJECT_SNAPSHOT, &[]) else {
            return;
        };
        let mask = injected.first().copied().unwrap_or(0);
        let holds =
            |button: Button| frames::button_id(button).is_some_and(|id| mask & (1 << id) != 0);
        let mut expect = lock(&self.shared.expect);
        if expect.generation != generation {
            // a reconnect happened while we were asking, so what came back belongs to the box from before it
            self.misses = 0;
            return;
        }
        let mut lost = false;
        for (&button, seen) in &mut expect.held {
            if holds(button) {
                *seen = true;
            } else if *seen {
                lost = true;
            }
        }
        let mut keys_lost = false;
        let mut retry = Vec::new();
        for (kind, on) in on {
            let stream = expect.stream(kind);
            if !stream.wanted {
                continue;
            }
            match (stream.confirmed, on) {
                (false, Some(true)) => stream.confirmed = true,
                // only the mouse stream going off means the box lost its state. pulling a keyboard out may switch
                // its own stream off, and that must NEVER let go of the app's mouse buttons
                (true, Some(false)) if kind == Stream::Mouse => lost = true,
                (true, Some(false)) => {
                    stream.confirmed = false;
                    keys_lost = true;
                }
                _ => {}
            }
            // a stream the box never switched on just gets asked again, that isn't a reset
            if !stream.confirmed {
                retry.push(kind);
            }
        }
        self.misses = if lost { self.misses + 1 } else { 0 };
        if self.misses >= MISSES_FOR_RESET {
            self.misses = 0;
            // the box lost its state without the port going away, so the core lets go of everything and turns the
            // streams back on
            *expect = Expect {
                generation: expect.generation,
                ..Expect::default()
            };
            drop(expect);
            self.shared.last.forget_locks();
            lock(&self.shared.key_masks).clear();
            self.sink.session_reset();
            // the box forgot it with the rest, and what anybox set is anybox's to put back
            if let Some(ms) = *lock(&self.shared.autorelease) {
                self.get(frames::AUTORELEASE, &ms.to_le_bytes());
            }
        } else {
            drop(expect);
            if keys_lost {
                // the box doesn't say how many it lost, so this reports at least one
                self.sink.dropped(1, DropSource::Device);
            }
            for kind in retry {
                if kind == Stream::Keyboard
                    && !self
                        .shared
                        .snapshot()
                        .capabilities
                        .get(Action::KeyEvents)
                        .is_supported()
                {
                    // the keyboard went away, so the route check publishes a new snapshot and the core turns the
                    // stream off
                    continue;
                }
                // checked again under the switch lock, because the core may have switched it off since
                let _switching = lock(&self.shared.streams);
                let still = {
                    let mut expect = lock(&self.shared.expect);
                    let stream = expect.stream(kind);
                    stream.wanted && !stream.confirmed
                };
                if still {
                    self.shared.last.set_stream(kind_byte(kind));
                    let _ = self
                        .link
                        .request(frame(frames::INPUT_STREAM, &[kind_byte(kind), 1]), None);
                }
            }
        }
        // a release the box refused or lost leaves the button held, so it goes out again. a button pressed or
        // clicked since then isn't touched, because its last write wasn't a release. this is checked again under the
        // button lock so a press can't slip in between
        for id in 0..5u8 {
            if mask & (1 << id) != 0 {
                let _buttons = lock(&self.shared.buttons);
                if self.shared.last.get(id) == 0 {
                    let _ = self.link.request(frame(frames::LEFT + id, &[0]), None);
                }
            }
        }
        for key in frames::injected_keys(&injected) {
            let _buttons = lock(&self.shared.buttons);
            if self.shared.pieces.load(Ordering::Acquire) != pieces {
                break;
            }
            if !self.shared.last.key(key) {
                self.shared.key_writes.fetch_add(1, Ordering::AcqRel);
                let _ = self.link.request(frame(frames::KEY_UP, &[key]), None);
            }
        }
        self.retry_locks(frames::locked_targets(&injected));
        self.ticks += 1;
        if self.ticks.is_multiple_of(4) {
            self.check_routes(generation);
            // key masks don't show in INJECT_SNAPSHOT, so sending the wanted ones again is the only way to fix a
            // lost one
            let snapshot = self.shared.snapshot();
            let masks: Vec<(u8, Action)> = lock(&self.shared.key_masks)
                .iter()
                .map(|(&key, &action)| (key, action))
                .filter(|&(_, action)| snapshot.capabilities.get(action).is_supported())
                .collect();
            for (key, action) in masks {
                let _buttons = lock(&self.shared.buttons);
                // the box refuses key writes while it types, and the app would get that refusal
                if self.shared.pieces.load(Ordering::Acquire) != pieces {
                    break;
                }
                if lock(&self.shared.key_masks).get(&key) == Some(&action) {
                    self.shared.key_writes.fetch_add(1, Ordering::AcqRel);
                    self.shared
                        .last
                        .name_mask(action == Action::ForceReleaseKey);
                    let _ = self.link.request(frame(frames::KEY_MASK, &[key, 1]), None);
                }
            }
        }
    }

    /// a lock Anybox wrote that the box doesn't show goes out again, and so does an unlock the box didn't take. a
    /// target Anybox never wrote belongs to someone else, so it's left alone. each one is checked under the button
    /// lock so the app can't slip a write in between
    fn retry_locks(&mut self, shown: u16) {
        let snapshot = self.shared.snapshot();
        for target in 0..14u8 {
            let _buttons = lock(&self.shared.buttons);
            let written = self.shared.last.lock(target);
            let writes = self.shared.last.lock_writes(target);
            let (seen, retries) = &mut self.lock_retries[usize::from(target)];
            // a new write by the app starts the count again
            if *seen != writes {
                *seen = writes;
                *retries = 0;
            }
            let on = shown & (1 << target) != 0;
            let state = match written {
                frames::LastSent::LOCKED if !on => 1,
                frames::LastSent::UNLOCKED if on => 0,
                _ => {
                    // the box shows what was asked, so a later loss gets its own three tries
                    *retries = 0;
                    continue;
                }
            };
            let action = if target < 5 {
                Action::LockButton
            } else {
                Action::LockAxis
            };
            if *retries >= MAX_LOCK_RETRIES || !snapshot.capabilities.get(action).is_supported() {
                continue;
            }
            *retries += 1;
            self.shared.last.name_lock(target);
            let _ = self
                .link
                .request(frame(frames::LOCK, &[target, state]), None);
        }
    }

    /// asks DEVICE again, so a mouse or keyboard plugged in or pulled out after connecting shows up
    fn check_routes(&self, generation: u64) {
        let Some(kinds) = self.get(frames::DEVICE, &[]) else {
            return;
        };
        let Some(&kinds) = kinds.first() else {
            return;
        };
        let current = self.shared.snapshot();
        let capabilities = &current.capabilities;
        let routed = (
            capabilities.get(Action::Move).is_supported(),
            capabilities.get(Action::KeyPress).is_supported(),
        );
        let now = (
            kinds & frames::MOUSE_ROUTED != 0,
            kinds & frames::KEYBOARD_ROUTED != 0,
        );
        if routed == now {
            return;
        }
        let Some(Firmware::Build(build)) = current.info.firmware else {
            return;
        };
        let fresh: Snapshot = snapshot(build, kinds, current.info.id.clone());
        // held while publishing, so a reprobe can't slip a newer snapshot in between, and the port can't be
        // dropped (which sends Disconnected) until this Reconnected is out
        let expect = lock(&self.shared.expect);
        if expect.generation != generation {
            return;
        }
        self.link.while_up(|| {
            *self
                .shared
                .snapshot
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = fresh.clone();
            self.sink.reconnected(fresh);
        });
        drop(expect);
    }
}
