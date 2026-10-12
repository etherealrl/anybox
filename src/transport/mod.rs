use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::error::{Error, Result};

pub(crate) mod codec;
pub(crate) mod link;
pub(crate) mod os;
pub(crate) mod probe;

#[cfg(test)]
pub(crate) mod fake;
#[cfg(unix)]
pub(crate) mod unix;
#[cfg(windows)]
mod windows;

/// how long one read waits before the reader checks whether it should stop
pub(crate) const READ_SLICE: Duration = Duration::from_millis(100);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct UsbId {
    pub(crate) vid: u16,
    pub(crate) pid: u16,
    /// only set when it's a real serial number, see `trustworthy_serial`
    pub(crate) serial: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Candidate {
    pub(crate) path: PathBuf,
    pub(crate) usb: Option<UsbId>,
}

/// the operating system's side, so the link can run on a fake one in tests
pub(crate) trait System: Send + Sync + 'static {
    fn ports(&self) -> Vec<Candidate>;
    fn open(&self, path: &Path, baud: u32) -> Result<Box<dyn Port>>;
}

/// one open port. reads and writes can run at the same time from different threads
pub(crate) trait Port: Send + Sync {
    /// waits `READ_SLICE` at most. if nothing came in it returns `TimedOut`, and `Ok(0)` means the port is gone
    fn read(&self, buffer: &mut [u8]) -> io::Result<usize>;
    /// gives up when one call takes longer than `WRITE_DEADLINE`
    fn write_all(&self, bytes: &[u8]) -> io::Result<()>;
    /// cuts a waiting read short, any thread can call it
    fn wake(&self);
    /// the reading thread calls this before it stops reading for good, because Windows ties a pending read to
    /// the thread that started it
    fn retire(&self);
}

/// the WCH CH343 bridge that Medius and MAKCU V4 both sit behind. MAKXD, other mak-suite boards and a lot of
/// ESP32 dev boards use it too, so it only says a box might be there, not which one or that it's a box at all
pub(crate) fn is_ch343(usb: &UsbId) -> bool {
    (usb.vid, usb.pid) == (0x1A86, 0x55D3)
}

/// the YSC 8K V2's three ports, its command port, the chip facing the mouse and its bootloader. text meant for
/// another box must never reach any of them
pub(crate) fn is_ysc(usb: &UsbId) -> bool {
    (usb.vid, usb.pid) == (0x1A86, 0xFE0C)
}

/// when a device has no serial number, serialport hands back a piece of the id Windows made up for it instead
pub(crate) fn trustworthy_serial(serial: &str) -> bool {
    serial.len() > 2 && !serial.contains('&')
}

/// turns the OS error from an open into one the user can actually do something about
pub(crate) fn open_error(path: &Path, error: io::Error) -> Error {
    let port = path.display().to_string();
    match classify_open(error.raw_os_error()) {
        Some(OpenFailure::Busy) => Error::PortBusy { port },
        Some(OpenFailure::Forbidden) => Error::PermissionDenied { port },
        Some(OpenFailure::Missing) => Error::NotFound { tried: vec![port] },
        None => Error::Io(error),
    }
}

enum OpenFailure {
    Busy,
    // Windows says access denied for a port someone else has open, so only unix ever tells a permission problem apart
    #[cfg_attr(windows, allow(dead_code))]
    Forbidden,
    Missing,
}

#[cfg(windows)]
fn classify_open(code: Option<i32>) -> Option<OpenFailure> {
    match code? {
        // ERROR_ACCESS_DENIED. the driver only lets one program open a COM port, so this means someone else has it
        5 => Some(OpenFailure::Busy),
        // ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND
        2 | 3 => Some(OpenFailure::Missing),
        _ => None,
    }
}

#[cfg(unix)]
fn classify_open(code: Option<i32>) -> Option<OpenFailure> {
    match code? {
        // EBUSY comes from someone else's TIOCEXCL and EWOULDBLOCK from our own flock
        libc::EBUSY | libc::EWOULDBLOCK => Some(OpenFailure::Busy),
        libc::EACCES => Some(OpenFailure::Forbidden),
        libc::ENOENT => Some(OpenFailure::Missing),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
