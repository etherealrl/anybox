use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::Ordering;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use super::frames;
use super::{Roles, Shared, line, lock, snapshot};
use crate::backend::Sink;
use crate::capability::{Action, FerrumApi};
use crate::error::Result;
use crate::transport::link::Link;

const PAUSE: Duration = Duration::from_secs(1);
const NAP: Duration = Duration::from_millis(10);

/// a mouse or keyboard set up in the Ferrum app after connecting doesn't show up anywhere on its own, and the box
/// silently drops commands for one that isn't there, so this asks every second. one `tick` is one round
pub(crate) struct Poller {
    shared: Arc<Shared>,
    link: Arc<Link>,
    sink: Sink,
    /// every role went away, which is what the app shows when the box itself gets unplugged
    gone: Mutex<Option<Gone>>,
}

/// the roles the box had before they all went, and on which connection, since a reconnect does its own reset
#[derive(Clone, Copy)]
struct Gone {
    generation: u64,
    mouse: bool,
    keyboard: bool,
}

impl Poller {
    pub(crate) fn new(shared: Arc<Shared>, link: Arc<Link>, sink: Sink) -> Self {
        Self {
            shared,
            link,
            sink,
            gone: Mutex::new(None),
        }
    }

    pub(crate) fn spawn(self) -> std::io::Result<JoinHandle<()>> {
        thread::Builder::new()
            .name("anybox-ferrum-poller".into())
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

    /// a missed reply already takes the link down inside `request`, so a failed ask just ends the round
    fn ask(&self, command: &str) -> Option<bool> {
        let reply = self
            .link
            .request(line(command), Some(frames::key(command.as_bytes())))
            .ok()??;
        super::bit(frames::result(&reply, true))
    }

    fn result(&self, command: &str) -> Result<Option<Vec<u8>>> {
        let reply = self
            .link
            .request(line(command), Some(frames::key(command.as_bytes())))?;
        Ok(reply.and_then(|reply| frames::result(&reply, true).map(<[u8]>::to_vec)))
    }

    pub(crate) fn tick(&self) {
        let generation = self.shared.generation.load(Ordering::Acquire);
        let (Some(mouse), Some(keyboard)) =
            (self.ask("km.has_mouse()"), self.ask("km.has_keyboard()"))
        else {
            return;
        };
        let current = self.shared.snapshot();
        let capabilities = &current.capabilities;
        let had = (
            capabilities.get(Action::Move).is_supported(),
            capabilities.get(Action::KeyPress).is_supported(),
        );
        if (mouse, keyboard) == had {
            return;
        }
        let mut roles = Roles {
            mouse,
            keyboard,
            directional: self.shared.directional.load(Ordering::Acquire),
            peripheral: None,
            firmware: current.info.firmware.clone(),
        };
        if self.shared.api == FerrumApi::Software {
            // the roles are news on their own, so a failed id query only leaves the ids out
            if let Ok(peripheral) = super::peripheral(&roles, |command| self.result(command)) {
                roles.peripheral = peripheral;
            }
        }
        let fresh = snapshot(self.shared.api, &roles);
        // a reprobe only runs while the port is down, so holding it up here means none can slip in between
        self.link.while_up(|| {
            if self.shared.generation.load(Ordering::Acquire) != generation {
                return;
            }
            *self
                .shared
                .snapshot
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = fresh.clone();
            self.sink.changed(fresh);
            // a box that comes back through the app has forgotten its locks, so once every role it had is back
            // the core treats it like a reset, which also lets go of what was held. ponytail: a replug quicker
            // than one round never shows up here, nothing on the app's port says so
            let mut gone = lock(&self.gone);
            if gone.is_some_and(|gone| gone.generation != generation) {
                *gone = None;
            }
            if !mouse && !keyboard {
                gone.get_or_insert(Gone {
                    generation,
                    mouse: had.0,
                    keyboard: had.1,
                });
            } else if gone
                .is_some_and(|gone| (mouse || !gone.mouse) && (keyboard || !gone.keyboard))
            {
                *gone = None;
                self.sink.session_reset();
            }
        });
    }
}
