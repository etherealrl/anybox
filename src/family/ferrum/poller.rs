use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use super::frames;
use super::{Roles, Shared, line, snapshot};
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
}

impl Poller {
    pub(crate) fn new(shared: Arc<Shared>, link: Arc<Link>, sink: Sink) -> Self {
        Self { shared, link, sink }
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
        if (mouse, keyboard)
            == (
                capabilities.get(Action::Move).is_supported(),
                capabilities.get(Action::KeyPress).is_supported(),
            )
        {
            return;
        }
        let mut roles = Roles {
            mouse,
            keyboard,
            directional: self.shared.directional.load(Ordering::Acquire),
            peripheral: None,
        };
        if self.shared.api == FerrumApi::Software {
            let Ok(peripheral) = super::peripheral(&roles, |command| self.result(command)) else {
                return;
            };
            roles.peripheral = peripheral;
        }
        let fresh = snapshot(self.shared.api, roles);
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
        });
    }
}
