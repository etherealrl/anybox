// based on the medius crate's Windows serial reader, MIT licensed, its notice is in NOTICE
#![allow(unsafe_code)]

use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::Path;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};
use std::thread::{self, ThreadId};
use std::time::{Duration, Instant};

use serial2::os::windows::CommTimeouts;
use serial2::{SerialPort, Settings};
use windows_sys::Win32::Devices::Communication::{
    COMSTAT, ClearCommError, EV_RXCHAR, SetCommMask, WaitCommEvent,
};
use windows_sys::Win32::Foundation::{
    ERROR_IO_PENDING, ERROR_OPERATION_ABORTED, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use windows_sys::Win32::System::Threading::{CreateEventW, WaitForSingleObject};

use super::{Port, READ_SLICE};
use crate::backend::WRITE_DEADLINE;

// DCB bit 14 is fAbortOnError. with it set the driver fails every transfer after one line error until
// ClearCommError runs, so a single overrun at a few Mbaud would wedge the port
const ABORT_ON_ERROR: u32 = 1 << 14;

// the WCH CH343 cancel routine races its own completion, so a wait gets this long to end by itself after
// a wake, and the same again after a cancel, before we leave it to the kernel
const END_GRACE: Duration = Duration::from_millis(20);

// three wakes in a row with nothing to read means the driver keeps firing on old bytes, so from then on
// each wake waits a moment, otherwise the reader burns a whole core
const SPURIOUS_WAKES: u32 = 3;
const SPURIOUS_PAUSE: Duration = Duration::from_millis(1);

pub(super) fn open(path: &Path, baud: u32) -> io::Result<Box<dyn Port>> {
    let port = SerialPort::open(path, |mut settings: Settings| {
        settings.set_raw();
        settings.set_baud_rate(baud)?;
        settings.as_raw_dbc_mut()._bitfield &= !ABORT_ON_ERROR;
        Ok(settings)
    })?;
    // the WCH CH343 driver can bugcheck in the timer it arms for a total read timeout, so a read only ever
    // takes what's already buffered and the comm event wait does the sleeping. this has to be set before
    // the first read, and set_read_timeout is never used because it would arm a total timeout again
    port.set_windows_timeouts(&CommTimeouts {
        read_interval_timeout: u32::MAX,
        read_total_timeout_multiplier: 0,
        read_total_timeout_constant: 0,
        write_total_timeout_multiplier: 0,
        write_total_timeout_constant: WRITE_DEADLINE.as_millis() as u32,
    })?;
    // SAFETY: the port's handle is open for the call
    if unsafe { SetCommMask(port.as_raw_handle(), EV_RXCHAR) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let sleeper = Sleeper::new()?;
    let _ = port.discard_input_buffer();
    Ok(Box::new(WindowsPort {
        port,
        sleeper: Mutex::new(sleeper),
        woken: AtomicBool::new(false),
    }))
}

struct WindowsPort {
    port: SerialPort,
    sleeper: Mutex<Sleeper>,
    woken: AtomicBool,
}

impl Port for WindowsPort {
    fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        let mut sleeper = self.sleeper.lock().unwrap_or_else(PoisonError::into_inner);
        let until = Instant::now() + READ_SLICE;
        let mut empty_wakes = 0;
        loop {
            match self.port.read(buffer) {
                Err(error) if error.kind() == io::ErrorKind::TimedOut => {}
                other => return other,
            }
            if Instant::now() >= until || self.woken.swap(false, Ordering::AcqRel) {
                return Err(io::ErrorKind::TimedOut.into());
            }
            if empty_wakes >= SPURIOUS_WAKES {
                thread::sleep(SPURIOUS_PAUSE);
            }
            if !sleeper.until_data(&self.port, until)? {
                return Err(io::ErrorKind::TimedOut.into());
            }
            empty_wakes += 1;
        }
    }

    fn write_all(&self, bytes: &[u8]) -> io::Result<()> {
        self.port.write_all(bytes)
    }

    fn wake(&self) {
        self.woken.store(true, Ordering::Release);
        nudge(&self.port);
    }

    fn retire(&self) {
        let _ = self
            .sleeper
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .end(&self.port);
    }
}

impl Drop for WindowsPort {
    fn drop(&mut self) {
        let sleeper = self
            .sleeper
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner);
        let _ = sleeper.end(&self.port);
    }
}

/// setting the event mask again completes a comm event wait that's in flight, and any thread can do it
fn nudge(port: &SerialPort) {
    // SAFETY: the port's handle is open for the call
    unsafe { SetCommMask(port.as_raw_handle(), EV_RXCHAR) };
}

/// how many bytes the driver holds that a read hasn't taken yet. ClearCommError also clears a pending
/// line error
fn queued(port: &SerialPort) -> io::Result<u32> {
    let mut errors = 0;
    let mut status = COMSTAT::default();
    // SAFETY: the port's handle is open, both out pointers are to live locals
    if unsafe { ClearCommError(port.as_raw_handle(), &mut errors, &mut status) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(status.cbInQue)
}

/// the overlapped and the mask that WaitCommEvent fills in, kept together because the kernel writes both
#[repr(C)]
#[derive(Default)]
struct Wait {
    overlapped: OVERLAPPED,
    mask: u32,
}

/// puts the reading thread to sleep with an overlapped WaitCommEvent until the port has bytes
struct Sleeper {
    event: OwnedHandle,
    /// heap memory the kernel writes into while a wait is in flight, so it's only freed when no wait is
    wait: *mut Wait,
    /// the thread that started the wait in flight. Windows cancels a thread's I/O when that thread exits, so
    /// a wait gets ended before its thread goes away. any thread can wake it or cancel it
    in_flight: Option<ThreadId>,
}

// SAFETY: `wait` is reached only through this struct, the event handle works from any thread, and the
// port's mutex (or `&mut self` in drop) serialises every use
unsafe impl Send for Sleeper {}

impl Sleeper {
    fn new() -> io::Result<Self> {
        // SAFETY: creates an unnamed manual reset event with default security, owned by nothing else
        let raw = unsafe { CreateEventW(ptr::null(), 1, 0, ptr::null()) };
        if raw.is_null() {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            // SAFETY: `raw` is a fresh valid handle that only this struct will close
            event: unsafe { OwnedHandle::from_raw_handle(raw) },
            wait: Box::into_raw(Box::default()),
            in_flight: None,
        })
    }

    /// returns true once there might be bytes to read, or false when `until` has passed
    fn until_data(&mut self, port: &SerialPort, until: Instant) -> io::Result<bool> {
        let me = thread::current().id();
        if self.in_flight.is_some_and(|thread| thread != me) {
            self.end(port)?;
        }
        if self.in_flight.is_none() {
            // SAFETY: nothing is in flight, so the kernel isn't writing `wait` while it's reset and handed over
            let fired = unsafe {
                self.wait.write(Wait::default());
                (*self.wait).overlapped.hEvent = self.event.as_raw_handle();
                WaitCommEvent(
                    port.as_raw_handle(),
                    &raw mut (*self.wait).mask,
                    &raw mut (*self.wait).overlapped,
                )
            };
            if fired != 0 {
                return Ok(true);
            }
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(ERROR_IO_PENDING as i32) {
                return Err(error);
            }
            self.in_flight = Some(me);
        }
        // bytes that landed before the wait started don't fire it, so the queue length is what catches those
        if queued(port)? > 0 {
            return Ok(true);
        }
        let left = until.saturating_duration_since(Instant::now());
        if !self.signalled(left)? {
            return Ok(false);
        }
        self.in_flight = None;
        let mut transferred = 0;
        // SAFETY: the event is set, so the wait on `wait` has completed and its result can be read
        let ok = unsafe {
            GetOverlappedResult(
                port.as_raw_handle(),
                &raw const (*self.wait).overlapped,
                &mut transferred,
                0,
            )
        };
        if ok == 0 {
            let error = io::Error::last_os_error();
            // a wait that got aborted (its thread ended or it was cancelled) just gets started again
            if error.raw_os_error() != Some(ERROR_OPERATION_ABORTED as i32) {
                return Err(error);
            }
        }
        Ok(true)
    }

    fn signalled(&self, timeout: Duration) -> io::Result<bool> {
        let millis = u32::try_from(timeout.as_millis()).unwrap_or(u32::MAX - 1);
        // SAFETY: the event handle is owned by `self` and open
        match unsafe { WaitForSingleObject(self.event.as_raw_handle(), millis) } {
            WAIT_OBJECT_0 => Ok(true),
            WAIT_TIMEOUT => Ok(false),
            _ => Err(io::Error::last_os_error()),
        }
    }

    /// finishes the wait in flight with a nudge first, and only cancels it when the nudge doesn't do it
    fn end(&mut self, port: &SerialPort) -> io::Result<()> {
        if self.in_flight.is_none() {
            return Ok(());
        }
        nudge(port);
        if !self.signalled(END_GRACE)? {
            // SAFETY: the port's handle is open and the overlapped is the one of the wait in flight
            unsafe { CancelIoEx(port.as_raw_handle(), &raw const (*self.wait).overlapped) };
            if !self.signalled(END_GRACE)? {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "the serial driver wouldn't finish a wait on the port, if this keeps happening unplug the box and plug it back in",
                ));
            }
        }
        self.in_flight = None;
        Ok(())
    }
}

impl Drop for Sleeper {
    fn drop(&mut self) {
        // the kernel might still write into a wait that never finished, so its memory is leaked on purpose
        if self.in_flight.is_none() {
            // SAFETY: `new` made `wait` with Box::into_raw and nothing is in flight on it
            drop(unsafe { Box::from_raw(self.wait) });
        }
    }
}

/// the raw QueryPerformanceCounter value, because an `Instant` can't be turned back into one and a USB capture
/// stamps its packets in QPC
pub(crate) fn qpc() -> Option<i64> {
    let mut count = 0i64;
    // SAFETY: QueryPerformanceCounter writes one i64 and can't fail on Windows XP or later
    let ok =
        unsafe { windows_sys::Win32::System::Performance::QueryPerformanceCounter(&mut count) };
    (ok != 0).then_some(count)
}
