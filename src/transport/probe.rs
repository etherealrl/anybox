use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use super::Port;
use super::codec::{Codec, Frame};
use crate::error::{Error, Result};

/// talks to a bare port with no link running, on whatever thread calls it. detection and reconnect use it,
/// otherwise a probe could end up waiting on a reader thread that is busy running that same probe
pub(crate) struct Probe<'a> {
    port: &'a dyn Port,
    stop: &'a AtomicBool,
}

impl<'a> Probe<'a> {
    pub(crate) fn new(port: &'a dyn Port, stop: &'a AtomicBool) -> Self {
        Self { port, stop }
    }

    /// writes, then reads until the codec splits off a frame that `accept` takes. it returns none when time
    /// runs out or the probe was told to stop
    pub(crate) fn ask(
        &self,
        bytes: &[u8],
        codec: &mut dyn Codec,
        accept: impl Fn(&Frame) -> bool,
        within: Duration,
    ) -> Result<Option<Frame>> {
        let answer = self.exchange(bytes, codec, accept, within);
        // a wait that's still listening gets cancelled when this thread ends, and on Windows the CH343 driver
        // races that cancel
        self.port.retire();
        answer
    }

    /// writes and waits for nothing, for a box that doesn't answer this
    pub(crate) fn say(&self, bytes: &[u8]) -> Result<()> {
        self.port.write_all(bytes).map_err(|_| Error::Disconnected)
    }

    fn exchange(
        &self,
        bytes: &[u8],
        codec: &mut dyn Codec,
        accept: impl Fn(&Frame) -> bool,
        within: Duration,
    ) -> Result<Option<Frame>> {
        self.port
            .write_all(bytes)
            .map_err(|_| Error::Disconnected)?;
        let deadline = Instant::now() + within;
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 512];
        loop {
            while let Some(frame) = codec.split(&mut buffer) {
                if accept(&frame) {
                    return Ok(Some(frame));
                }
            }
            if self.stop.load(Ordering::Acquire) || Instant::now() >= deadline {
                return Ok(None);
            }
            match self.port.read(&mut chunk) {
                Ok(0) => return Err(Error::Disconnected),
                Ok(n) => buffer.extend_from_slice(&chunk[..n]),
                Err(error) if error.kind() == io::ErrorKind::TimedOut => {}
                Err(_) => return Err(Error::Disconnected),
            }
        }
    }
}
