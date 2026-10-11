use std::sync::mpsc;

use super::{Poller, build};
use crate::backend::Sink;
use crate::click::{ClickTiming, Holds};
use crate::device::{Device, Start};
use crate::error::{Error, Result};

/// the poller of a device made with `Device::mock_medius_manual`, ticked by hand so a test doesn't wait on its
/// timing
pub struct MockPoller(Poller);

impl MockPoller {
    /// one poller round, what runs every 250 ms on a real device. the clone and the box drop counter are only
    /// looked at every fourth round, and the mock keeps its own timers (a detach waits 250 ms before it counts,
    /// a restarted clone comes back after 100 ms), so a test still waits for those
    pub fn tick(&mut self) {
        self.0.tick();
    }
}

/// a device over the medius crate's mock box, for testing your own code without a box plugged in. it runs the
/// exact code a real Medius gets, only the box is the crate's model of one, so it proves nothing about hardware
/// and it can only be shaped like a Medius
///
/// keep a clone of the `MockBox` and drive it like a box: `push_usages` pushes the whole held set (a release is
/// pushing the empty set), `push_motion` pushes motion, `restart()` gives `Disconnected` then `Reconnected`, and
/// `detach(true)` and `link_lost()` give `SessionReset`. for a drop from the box, take the state from
/// `query_catch()`, raise its `dropped` and give it back with `with_catch_state`, and it shows up as
/// `Dropped { Device }`. those come on a poller round, so wait with `recv_timeout`. many mock devices at once on a
/// busy machine can run past the 500 ms command deadline and show a `Disconnected` that heals on the next round
impl Device {
    pub fn mock_medius(mock: medius::MockBox, click_timing: ClickTiming) -> Result<Device> {
        let (device, poller) = Self::mock_medius_manual(mock, click_timing)?;
        poller.0.spawn()?;
        Ok(device)
    }

    /// the same, but the poller doesn't run by itself, you tick it with `MockPoller::tick`
    pub fn mock_medius_manual(
        mock: medius::MockBox,
        click_timing: ClickTiming,
    ) -> Result<(Device, MockPoller)> {
        let crate_device = medius::Device::open_mock(mock)
            .map_err(|error| Error::Io(std::io::Error::other(error)))?;
        let (signals, incoming) = mpsc::channel();
        let (backend, snapshot, poller) = build(crate_device, Sink(signals.clone()))?;
        let device = Device::start(Start {
            backend: Box::new(backend),
            snapshot,
            signals,
            incoming,
            click_timing,
            holds: Holds::from_clock(),
            port: "mock box".into(),
        })?;
        Ok((device, MockPoller(poller)))
    }
}
