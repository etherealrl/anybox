use std::any::Any;
use std::cell::RefCell;
use std::ffi::{CStr, CString, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use anybox::Error;

use crate::failure::Failure;

/// what a call did, `ANYBOX_STATUS_OK` or the reason it failed. `anybox_last_error_message` has the text and
/// `anybox_last_error_detail` the fields
pub type AnyboxStatus = u32;

pub const ANYBOX_STATUS_OK: AnyboxStatus = 0;
pub const ANYBOX_STATUS_NOT_FOUND: AnyboxStatus = 1;
pub const ANYBOX_STATUS_CLOSED: AnyboxStatus = 2;
pub const ANYBOX_STATUS_PORT_BUSY: AnyboxStatus = 3;
pub const ANYBOX_STATUS_PERMISSION_DENIED: AnyboxStatus = 4;
pub const ANYBOX_STATUS_NO_BOX_CAN: AnyboxStatus = 5;
pub const ANYBOX_STATUS_WRONG_FAMILY: AnyboxStatus = 6;
pub const ANYBOX_STATUS_WRONG_BOX: AnyboxStatus = 7;
pub const ANYBOX_STATUS_WRONG_FERRUM_API: AnyboxStatus = 8;
pub const ANYBOX_STATUS_FIRMWARE_TOO_OLD: AnyboxStatus = 9;
pub const ANYBOX_STATUS_LIBRARY_TOO_OLD: AnyboxStatus = 10;
pub const ANYBOX_STATUS_UNSUPPORTED: AnyboxStatus = 11;
pub const ANYBOX_STATUS_INVALID_ARGUMENT: AnyboxStatus = 12;
pub const ANYBOX_STATUS_REJECTED: AnyboxStatus = 13;
pub const ANYBOX_STATUS_TIMEOUT: AnyboxStatus = 14;
pub const ANYBOX_STATUS_DISCONNECTED: AnyboxStatus = 15;
pub const ANYBOX_STATUS_INTERRUPTED: AnyboxStatus = 16;
pub const ANYBOX_STATUS_IO: AnyboxStatus = 17;
/// anybox panicked inside the call. the handle still works and can be freed, but what the call already did on the
/// box isn't known
pub const ANYBOX_STATUS_PANIC: AnyboxStatus = 18;
/// an error this C ABI has no status for yet, the message still says what happened
pub const ANYBOX_STATUS_UNKNOWN: AnyboxStatus = 19;
/// `anybox_events_recv` got nothing in time
pub const ANYBOX_STATUS_RECV_TIMEOUT: AnyboxStatus = 20;
/// `anybox_events_recv` has nothing left, the device is gone and everything queued was read
pub const ANYBOX_STATUS_RECV_CLOSED: AnyboxStatus = 21;

thread_local! {
    static LAST_ERROR: RefCell<Option<Arc<Failure>>> = const { RefCell::new(None) };
}

/// what went wrong in the last call that failed on this thread, in UTF-8. it stays valid until the next anybox call
/// on this thread that fails, and it's empty when nothing failed yet. `anybox_last_error_detail` has its fields
#[unsafe(no_mangle)]
pub extern "C" fn anybox_last_error_message() -> *const c_char {
    // a thread that's being torn down has no last error left to give
    LAST_ERROR
        .try_with(|last| {
            last.borrow()
                .as_ref()
                .map_or(c"".as_ptr(), |failure| failure.message.as_ptr())
        })
        .unwrap_or(c"".as_ptr())
}

pub(crate) fn last_failure() -> Option<Arc<Failure>> {
    LAST_ERROR
        .try_with(|last| last.borrow().clone())
        .ok()
        .flatten()
}

/// every failure replaces the whole last error, so nothing of an earlier one is left behind
pub(crate) fn set_failure(failure: Arc<Failure>) -> AnyboxStatus {
    let status = failure.status;
    let _ = LAST_ERROR.try_with(|last| *last.borrow_mut() = Some(failure));
    status
}

pub(crate) fn fail(status: AnyboxStatus, message: impl Into<String>) -> AnyboxStatus {
    set_failure(Failure::plain(status, message))
}

pub(crate) fn invalid(message: impl Into<String>) -> AnyboxStatus {
    fail(ANYBOX_STATUS_INVALID_ARGUMENT, message)
}

pub(crate) fn error(error: Error) -> AnyboxStatus {
    set_failure(Failure::of(&error))
}

pub(crate) fn status_of(error: &Error) -> AnyboxStatus {
    match error {
        Error::NotFound { .. } => ANYBOX_STATUS_NOT_FOUND,
        Error::Closed => ANYBOX_STATUS_CLOSED,
        Error::PortBusy { .. } => ANYBOX_STATUS_PORT_BUSY,
        Error::PermissionDenied { .. } => ANYBOX_STATUS_PERMISSION_DENIED,
        Error::NoBoxCan { .. } => ANYBOX_STATUS_NO_BOX_CAN,
        Error::WrongFamily { .. } => ANYBOX_STATUS_WRONG_FAMILY,
        Error::WrongBox { .. } => ANYBOX_STATUS_WRONG_BOX,
        Error::WrongFerrumApi { .. } => ANYBOX_STATUS_WRONG_FERRUM_API,
        Error::FirmwareTooOld { .. } => ANYBOX_STATUS_FIRMWARE_TOO_OLD,
        Error::LibraryTooOld { .. } => ANYBOX_STATUS_LIBRARY_TOO_OLD,
        Error::Unsupported { .. } => ANYBOX_STATUS_UNSUPPORTED,
        Error::InvalidArgument { .. } => ANYBOX_STATUS_INVALID_ARGUMENT,
        Error::Rejected { .. } => ANYBOX_STATUS_REJECTED,
        Error::Timeout { .. } => ANYBOX_STATUS_TIMEOUT,
        Error::Disconnected => ANYBOX_STATUS_DISCONNECTED,
        Error::Interrupted { .. } => ANYBOX_STATUS_INTERRUPTED,
        Error::Io(_) => ANYBOX_STATUS_IO,
        _ => ANYBOX_STATUS_UNKNOWN,
    }
}

/// runs a call so a panic never unwinds into C, it comes out as `ANYBOX_STATUS_PANIC` instead
pub(crate) fn guard(call: impl FnOnce() -> Result<(), AnyboxStatus>) -> AnyboxStatus {
    match catch_unwind(AssertUnwindSafe(call)) {
        Ok(Ok(())) => ANYBOX_STATUS_OK,
        Ok(Err(status)) => status,
        Err(panic) => fail(ANYBOX_STATUS_PANIC, panic_text(panic)),
    }
}

/// a free can't report anything, so a panic in it only leaves its text behind
pub(crate) fn guard_free(free: impl FnOnce()) {
    if let Err(panic) = catch_unwind(AssertUnwindSafe(free)) {
        fail(ANYBOX_STATUS_PANIC, panic_text(panic));
    }
}

fn panic_text(panic: Box<dyn Any + Send>) -> String {
    let text = match (panic.downcast_ref::<&str>(), panic.downcast_ref::<String>()) {
        (Some(text), _) => text,
        (_, Some(text)) => text.as_str(),
        _ => "no message",
    };
    format!("anybox panicked ({text}), that's a bug in anybox so please report it")
}

/// a NUL inside the text would end it early in C anyway, so it's cut there
pub(crate) fn c_string(text: impl Into<Vec<u8>>) -> CString {
    let mut bytes = text.into();
    if let Some(nul) = bytes.iter().position(|byte| *byte == 0) {
        bytes.truncate(nul);
    }
    CString::new(bytes).unwrap_or_default()
}

/// a pointer the call reads from, null refused with its name
pub(crate) unsafe fn read<'a, T>(pointer: *const T, name: &str) -> Result<&'a T, AnyboxStatus> {
    // SAFETY: the header says a non-null pointer has to be aligned and point at a live value of its type
    unsafe { pointer.as_ref() }.ok_or_else(|| invalid(format!("{name} is null")))
}

/// a pointer the call writes its result through, null refused with its name. nothing is written until the call
/// worked
pub(crate) fn write<T>(pointer: *mut T, name: &str) -> Result<Out<T>, AnyboxStatus> {
    if pointer.is_null() {
        Err(invalid(format!("{name} is null")))
    } else {
        Ok(Out(pointer))
    }
}

pub(crate) struct Out<T>(*mut T);

impl<T> Out<T> {
    pub(crate) fn set(self, value: T) {
        // SAFETY: not null, and the header says it has to be aligned and writable for a whole `T`
        unsafe { self.0.write(value) }
    }
}

/// a string the call reads, which has to be UTF-8
pub(crate) unsafe fn text<'a>(pointer: *const c_char, name: &str) -> Result<&'a str, AnyboxStatus> {
    if pointer.is_null() {
        return Err(invalid(format!("{name} is null")));
    }
    // SAFETY: the header says a string has to end in a NUL and stay alive for the call
    unsafe { CStr::from_ptr(pointer) }
        .to_str()
        .map_err(|_| invalid(format!("{name} isn't UTF-8")))
}

/// a list the call reads, `len` values from `pointer`. null is fine for an empty one
pub(crate) unsafe fn slice<'a, T>(
    pointer: *const T,
    len: usize,
    name: &str,
) -> Result<&'a [T], AnyboxStatus> {
    if len == 0 {
        return Ok(&[]);
    }
    if pointer.is_null() {
        return Err(invalid(format!("{name} is null but its length is {len}")));
    }
    // SAFETY: the header says a non-null list has to be aligned and hold `len` values that stay alive for the call
    Ok(unsafe { std::slice::from_raw_parts(pointer, len) })
}
