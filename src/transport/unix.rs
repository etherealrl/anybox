#![allow(unsafe_code)]

use std::io;
use std::os::fd::AsRawFd;
use std::path::Path;

use serial2::{SerialPort, Settings};

use super::{Port, READ_SLICE};
use crate::backend::WRITE_DEADLINE;

pub(super) fn open(path: &Path, baud: u32) -> io::Result<Box<dyn Port>> {
    let mut port = SerialPort::open(path, |mut settings: Settings| {
        settings.set_raw();
        settings.set_baud_rate(baud)?;
        Ok(settings)
    })?;
    let fd = port.as_raw_fd();
    // serial2 doesn't lock the port, and two programs talking to one box garble each other's commands.
    // the flock goes first because TIOCEXCL stays on the tty until its last close, so setting it on a
    // port someone else already holds would lock them out when they reopen it
    // SAFETY: fd is the port's open descriptor for both calls, and neither keeps it
    unsafe {
        if libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) != 0 {
            return Err(io::Error::last_os_error());
        }
        if libc::ioctl(fd, libc::TIOCEXCL) != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    port.set_read_timeout(READ_SLICE)?;
    port.set_write_timeout(WRITE_DEADLINE)?;
    Ok(Box::new(UnixPort(port)))
}

struct UnixPort(SerialPort);

impl Port for UnixPort {
    fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        self.0.read(buffer)
    }

    fn write_all(&self, bytes: &[u8]) -> io::Result<()> {
        self.0.write_all(bytes)
    }

    // a read only waits one poll slice, so there's nothing to cut short
    fn wake(&self) {}

    fn retire(&self) {}
}

/// a plain open and a flock on a port that another library opens itself, so the OS error tells us if it's
/// busy, forbidden or missing. there's no TIOCEXCL here because that would lock the other library out of
/// its own opens and reopens
pub(crate) fn lock_port(path: &Path) -> io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOCTTY | libc::O_NONBLOCK)
        .open(path)?;
    // SAFETY: the descriptor belongs to `file`, which stays open for the call
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(file)
}

/// the clock `Instant` reads, CLOCK_UPTIME_RAW on macOS and CLOCK_MONOTONIC everywhere else, in nanoseconds
pub(crate) fn monotonic_ns() -> u64 {
    #[cfg(target_vendor = "apple")]
    let clock = libc::CLOCK_UPTIME_RAW;
    #[cfg(not(target_vendor = "apple"))]
    let clock = libc::CLOCK_MONOTONIC;
    let mut now = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: clock_gettime writes one timespec, and this clock exists on every system std supports
    unsafe { libc::clock_gettime(clock, &mut now) };
    now.tv_sec as u64 * 1_000_000_000 + now.tv_nsec as u64
}
