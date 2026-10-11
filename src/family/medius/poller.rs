use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use super::{Shared, snapshot};
use crate::capability::Snapshot;
use crate::event::DropSource;

/// the crate doesn't push any link events, so this polls for them. one `tick` is one pass, the thread runs one
/// every 250 ms and tests just call it directly
pub(crate) struct Poller {
    pub(super) shared: Arc<Shared>,
    reconnects: u64,
    restarts: u64,
    session: Option<u16>,
    catch_dropped: Option<u32>,
    clone: super::CloneState,
    ticks: u64,
    /// when the box reboots, the session goes back down before the crate counts the restart. so the reboot gets
    /// handled as soon as the session drops, and this says the restart that shows up after it is expected
    restart_expected: bool,
}

pub(crate) struct Baseline {
    reconnects: u64,
    restarts: u64,
    session: Option<u16>,
    catch_dropped: Option<u32>,
}

/// true when both support the same actions with the same buttons and axes, the quirks don't matter here
fn same_actions(a: &Snapshot, b: &Snapshot) -> bool {
    let supported = |snapshot: &Snapshot| {
        snapshot
            .capabilities
            .iter()
            .map(|(action, capability)| (action, capability.is_supported()))
            .collect::<HashSet<_>>()
    };
    a.capabilities.buttons() == b.capabilities.buttons()
        && a.capabilities.axes() == b.capabilities.axes()
        && supported(a) == supported(b)
}

impl Poller {
    pub(crate) fn baseline(device: &medius::Device) -> Baseline {
        let counters = device.counters();
        Baseline {
            reconnects: counters.reconnects,
            restarts: counters.restarts,
            session: device.query_stats().ok().map(|stats| stats.session),
            catch_dropped: device.query_catch().ok().map(|catch| catch.dropped),
        }
    }

    pub(crate) fn new(shared: Arc<Shared>, baseline: Baseline, clone: super::CloneState) -> Self {
        Self {
            shared,
            reconnects: baseline.reconnects,
            restarts: baseline.restarts,
            session: baseline.session,
            catch_dropped: baseline.catch_dropped,
            clone,
            ticks: 0,
            restart_expected: false,
        }
    }

    pub(crate) fn stopped(&self) -> bool {
        self.shared.stop.load(Ordering::Acquire)
    }

    pub(crate) fn tick(&mut self) {
        let Some(device) = self.shared.device() else {
            return;
        };
        let counters = device.counters();
        // a reboot can leave the session at 0 when it was already 0, so we watch the restart count too
        let restarted = counters.restarts != self.restarts;
        self.restarts = counters.restarts;
        if restarted
            && std::mem::take(&mut self.restart_expected)
            && counters.reconnects == self.reconnects
        {
            // the reboot behind this restart was already handled when the session went back down
        } else if counters.reconnects != self.reconnects || restarted {
            self.reconnects = counters.reconnects;
            self.shared.go_down();
            self.come_back(&device);
            return;
        }
        let stats = match device.query_stats() {
            Ok(stats) => stats,
            Err(_) => {
                self.shared.go_down();
                return;
            }
        };
        if !self.shared.is_up() {
            // a slow query or a reboot that didn't reconnect the port lands here, so the link never stays down for good
            self.come_back(&device);
            return;
        }
        match self.session {
            // the session only ever counts up, so going back down means the device chip rebooted
            Some(session) if stats.session < session => {
                self.restart_expected = true;
                self.shared.go_down();
                self.come_back(&device);
                return;
            }
            Some(session) if session != stats.session => {
                self.session = Some(stats.session);
                self.shared.session_reset();
            }
            _ => {}
        }
        self.session = Some(stats.session);
        self.ticks += 1;
        if self.ticks.is_multiple_of(4) {
            self.check_clone(&device);
            self.check_box_drops(&device);
        }
    }

    fn come_back(&mut self, device: &medius::Device) {
        let Ok((snapshot, clone)) = snapshot(device) else {
            return;
        };
        self.clone = clone;
        // a session change in this same tick is part of the reconnect, it isn't a separate reset
        self.session = device.query_stats().ok().map(|stats| stats.session);
        self.shared.come_up(snapshot);
    }

    /// notices a mouse or keyboard being plugged into the box, pulled out or swapped
    fn check_clone(&mut self, device: &medius::Device) {
        let (Ok(health), Ok(caps)) = (device.query_health(), device.caps()) else {
            return;
        };
        let now = (
            health.mouse_attached,
            health.clone_configured,
            health.kbd_attached,
            caps,
        );
        if now == self.clone {
            return;
        }
        let Ok((fresh, clone)) = snapshot(device) else {
            return;
        };
        let current = self.shared.current_snapshot();
        let same_device = (now.0, now.1, now.2) == (self.clone.0, self.clone.1, self.clone.2);
        if same_device && same_actions(&current, &fresh) {
            // a keyboard that switches between its 6 key and NKRO reports does this every time more keys go down,
            // and a reconnect would restart the key stream while you're typing. only the details changed (the key
            // limit), so the core just gets the new snapshot
            self.clone = clone;
            self.shared.change_quietly(fresh);
        } else {
            self.come_back(device);
        }
    }

    fn check_box_drops(&mut self, device: &medius::Device) {
        let Ok(catch) = device.query_catch() else {
            return;
        };
        // this counter is for the whole box and a reboot resets it, so only a rise counts
        if let Some(before) = self.catch_dropped
            && catch.dropped > before
        {
            self.shared
                .dropped(u64::from(catch.dropped - before), DropSource::Device);
        }
        self.catch_dropped = Some(catch.dropped);
    }
}
