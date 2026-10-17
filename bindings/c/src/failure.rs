use std::ffi::{CString, c_char};
use std::io::ErrorKind;
use std::ptr;
use std::sync::Arc;

use anybox::{Action, Error, FerrumApi, Firmware, Found};

use crate::connect::{AnyboxScanList, AnyboxStringList, Entry};
use crate::snapshot::{ANYBOX_FIRMWARE_NONE, ANYBOX_ID_NONE, firmware_fields, id_fields};
use crate::status::{
    ANYBOX_STATUS_INVALID_ARGUMENT, ANYBOX_STATUS_OK, AnyboxStatus, c_string, last_failure,
    status_of,
};
use crate::values::{
    ANYBOX_FAMILY_ANY, ANYBOX_FERRUM_API_NONE, action_number, family_number, ferrum_api_number,
};

// fixed by hand like every other value, `ErrorKind`'s own discriminants aren't a contract

/// not an `ANYBOX_STATUS_IO` error
pub const ANYBOX_IO_KIND_NONE: u32 = 0;
/// a kind this table has no number for, the message still says what it was
pub const ANYBOX_IO_KIND_OTHER: u32 = 1;
pub const ANYBOX_IO_KIND_NOT_FOUND: u32 = 2;
pub const ANYBOX_IO_KIND_PERMISSION_DENIED: u32 = 3;
pub const ANYBOX_IO_KIND_TIMED_OUT: u32 = 4;
pub const ANYBOX_IO_KIND_BROKEN_PIPE: u32 = 5;
pub const ANYBOX_IO_KIND_WOULD_BLOCK: u32 = 6;
pub const ANYBOX_IO_KIND_INVALID_INPUT: u32 = 7;
pub const ANYBOX_IO_KIND_INVALID_DATA: u32 = 8;
pub const ANYBOX_IO_KIND_UNEXPECTED_EOF: u32 = 9;
pub const ANYBOX_IO_KIND_INTERRUPTED: u32 = 10;
pub const ANYBOX_IO_KIND_UNSUPPORTED: u32 = 11;
pub const ANYBOX_IO_KIND_OUT_OF_MEMORY: u32 = 12;
pub const ANYBOX_IO_KIND_RESOURCE_BUSY: u32 = 13;
pub const ANYBOX_IO_KIND_WRITE_ZERO: u32 = 14;
pub const ANYBOX_IO_KIND_NOT_CONNECTED: u32 = 15;

const IO_KINDS: [(u32, ErrorKind); 14] = [
    (ANYBOX_IO_KIND_NOT_FOUND, ErrorKind::NotFound),
    (
        ANYBOX_IO_KIND_PERMISSION_DENIED,
        ErrorKind::PermissionDenied,
    ),
    (ANYBOX_IO_KIND_TIMED_OUT, ErrorKind::TimedOut),
    (ANYBOX_IO_KIND_BROKEN_PIPE, ErrorKind::BrokenPipe),
    (ANYBOX_IO_KIND_WOULD_BLOCK, ErrorKind::WouldBlock),
    (ANYBOX_IO_KIND_INVALID_INPUT, ErrorKind::InvalidInput),
    (ANYBOX_IO_KIND_INVALID_DATA, ErrorKind::InvalidData),
    (ANYBOX_IO_KIND_UNEXPECTED_EOF, ErrorKind::UnexpectedEof),
    (ANYBOX_IO_KIND_INTERRUPTED, ErrorKind::Interrupted),
    (ANYBOX_IO_KIND_UNSUPPORTED, ErrorKind::Unsupported),
    (ANYBOX_IO_KIND_OUT_OF_MEMORY, ErrorKind::OutOfMemory),
    (ANYBOX_IO_KIND_RESOURCE_BUSY, ErrorKind::ResourceBusy),
    (ANYBOX_IO_KIND_WRITE_ZERO, ErrorKind::WriteZero),
    (ANYBOX_IO_KIND_NOT_CONNECTED, ErrorKind::NotConnected),
];

pub(crate) fn io_kind_number(kind: ErrorKind) -> u32 {
    IO_KINDS
        .iter()
        .find(|(_, known)| *known == kind)
        .map_or(ANYBOX_IO_KIND_OTHER, |(number, _)| *number)
}

/// the fields of a failure, so code can tell errors apart without reading the message. which fields mean something
/// depends on `kind`, the rest are 0 and null. every string lives as long as the failure it came from, see
/// `anybox_last_error_detail`
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AnyboxErrorDetail {
    /// the failure's status, `ANYBOX_STATUS_OK` when nothing failed
    pub kind: u32,
    /// set for `ANYBOX_STATUS_UNSUPPORTED`, `ANYBOX_STATUS_INVALID_ARGUMENT`, `ANYBOX_STATUS_REJECTED`,
    /// `ANYBOX_STATUS_TIMEOUT` and `ANYBOX_STATUS_INTERRUPTED` when anybox made them
    pub has_action: bool,
    pub action: u32,
    /// the box family the error is about, `ANYBOX_FAMILY_ANY` when it names none. the same goes for the two below,
    /// which `ANYBOX_STATUS_WRONG_FAMILY` sets
    pub family: u32,
    pub expected_family: u32,
    pub found_family: u32,
    /// the port the error is about, null when it names none
    pub port: *const c_char,
    /// the reason of `ANYBOX_STATUS_UNSUPPORTED`, the detail of `ANYBOX_STATUS_INVALID_ARGUMENT` and
    /// `ANYBOX_STATUS_REJECTED`, null otherwise
    pub text: *const c_char,
    /// the ids `ANYBOX_STATUS_WRONG_BOX` compares, flattened like in `AnyboxInfo`
    pub expected_id_kind: u32,
    pub expected_id_mac: [u8; 6],
    pub expected_id_serial: *const c_char,
    pub found_id_kind: u32,
    pub found_id_mac: [u8; 6],
    pub found_id_serial: *const c_char,
    /// the firmware `ANYBOX_STATUS_FIRMWARE_TOO_OLD` and `ANYBOX_STATUS_LIBRARY_TOO_OLD` found, flattened like in
    /// `AnyboxInfo`
    pub found_firmware_kind: u32,
    pub found_firmware_major: u16,
    pub found_firmware_minor: u16,
    pub found_firmware_patch: u16,
    pub found_firmware_protocol: u8,
    pub found_firmware_build: u32,
    pub found_firmware_text: *const c_char,
    /// the firmware `ANYBOX_STATUS_FIRMWARE_TOO_OLD` needs
    pub needed_firmware_kind: u32,
    pub needed_firmware_major: u16,
    pub needed_firmware_minor: u16,
    pub needed_firmware_patch: u16,
    pub needed_firmware_protocol: u8,
    pub needed_firmware_build: u32,
    pub needed_firmware_text: *const c_char,
    /// the APIs `ANYBOX_STATUS_WRONG_FERRUM_API` compares
    pub expected_ferrum_api: u32,
    pub found_ferrum_api: u32,
    /// an `ANYBOX_IO_KIND_*` for `ANYBOX_STATUS_IO`
    pub io_kind: u32,
    /// the operating system's own error code, when the I/O error came from it
    pub has_os_error: bool,
    pub os_error: i32,
}

impl AnyboxErrorDetail {
    pub(crate) fn empty(kind: AnyboxStatus) -> Self {
        Self {
            kind,
            has_action: false,
            action: 0,
            family: ANYBOX_FAMILY_ANY,
            expected_family: ANYBOX_FAMILY_ANY,
            found_family: ANYBOX_FAMILY_ANY,
            port: ptr::null(),
            text: ptr::null(),
            expected_id_kind: ANYBOX_ID_NONE,
            expected_id_mac: [0; 6],
            expected_id_serial: ptr::null(),
            found_id_kind: ANYBOX_ID_NONE,
            found_id_mac: [0; 6],
            found_id_serial: ptr::null(),
            found_firmware_kind: ANYBOX_FIRMWARE_NONE,
            found_firmware_major: 0,
            found_firmware_minor: 0,
            found_firmware_patch: 0,
            found_firmware_protocol: 0,
            found_firmware_build: 0,
            found_firmware_text: ptr::null(),
            needed_firmware_kind: ANYBOX_FIRMWARE_NONE,
            needed_firmware_major: 0,
            needed_firmware_minor: 0,
            needed_firmware_patch: 0,
            needed_firmware_protocol: 0,
            needed_firmware_build: 0,
            needed_firmware_text: ptr::null(),
            expected_ferrum_api: ANYBOX_FERRUM_API_NONE,
            found_ferrum_api: ANYBOX_FERRUM_API_NONE,
            io_kind: ANYBOX_IO_KIND_NONE,
            has_os_error: false,
            os_error: 0,
        }
    }
}

/// one failure with everything C can read about it. it's shared, so a scan list entry and the thread's last error
/// can hold the same one and the strings live as long as either does
pub(crate) struct Failure {
    pub(crate) status: AnyboxStatus,
    pub(crate) message: CString,
    pub(crate) detail: AnyboxErrorDetail,
    // the strings `detail` points into
    _strings: Vec<CString>,
    /// the ports `NotFound` tried
    pub(crate) tried: Vec<CString>,
    /// the boxes `NoBoxCan` found, with the `ANYBOX_ACTION_*` each couldn't do
    pub(crate) incapable: Vec<(Found, Vec<u32>)>,
}

// SAFETY: the only raw pointers are the ones in `detail`, which point into strings this failure owns and never
// changes after it's built, so it can be read from and dropped on any thread
unsafe impl Send for Failure {}
unsafe impl Sync for Failure {}

impl Failure {
    /// a failure anybox's `Error` doesn't cover, like a null pointer or a panic
    pub(crate) fn plain(status: AnyboxStatus, message: impl Into<String>) -> Arc<Self> {
        Arc::new(Self {
            status,
            message: c_string(message.into()),
            detail: AnyboxErrorDetail::empty(status),
            _strings: Vec::new(),
            tried: Vec::new(),
            incapable: Vec::new(),
        })
    }

    pub(crate) fn of(error: &Error) -> Arc<Self> {
        let status = status_of(error);
        let mut strings = Vec::new();
        let mut keep = |text: &str| {
            let text = c_string(text);
            let pointer = text.as_ptr();
            strings.push(text);
            pointer
        };
        let mut detail = AnyboxErrorDetail::empty(status);
        let mut tried = Vec::new();
        let mut incapable = Vec::new();
        match error {
            Error::NotFound { tried: ports, .. } => {
                tried = ports.iter().map(|port| c_string(port.as_str())).collect();
            }
            Error::PortBusy { port, .. } | Error::PermissionDenied { port, .. } => {
                detail.port = keep(port);
            }
            Error::NoBoxCan { boxes, .. } => {
                incapable = boxes
                    .iter()
                    .map(|(found, missing)| {
                        let missing = missing.iter().map(|action| action_number(*action));
                        (found.clone(), missing.collect())
                    })
                    .collect();
            }
            Error::WrongFamily {
                port,
                expected,
                found,
                ..
            } => {
                detail.port = keep(port);
                detail.expected_family = family_number(*expected);
                detail.found_family = found.map_or(ANYBOX_FAMILY_ANY, family_number);
            }
            Error::WrongBox {
                port,
                expected,
                found,
                ..
            } => {
                detail.port = keep(port);
                (
                    detail.expected_id_kind,
                    detail.expected_id_mac,
                    detail.expected_id_serial,
                ) = id_fields(Some(expected), &mut keep);
                (
                    detail.found_id_kind,
                    detail.found_id_mac,
                    detail.found_id_serial,
                ) = id_fields(found.as_ref(), &mut keep);
            }
            Error::WrongFerrumApi {
                port,
                expected,
                found,
                ..
            } => {
                detail.port = keep(port);
                detail.expected_ferrum_api = ferrum_api(*expected);
                detail.found_ferrum_api = ferrum_api(*found);
            }
            Error::FirmwareTooOld {
                family,
                found,
                needed,
                ..
            } => {
                detail.family = family_number(*family);
                found_firmware(&mut detail, found, &mut keep);
                let needed = firmware_fields(Some(needed), &mut keep);
                detail.needed_firmware_kind = needed.kind;
                detail.needed_firmware_major = needed.major;
                detail.needed_firmware_minor = needed.minor;
                detail.needed_firmware_patch = needed.patch;
                detail.needed_firmware_protocol = needed.protocol;
                detail.needed_firmware_build = needed.build;
                detail.needed_firmware_text = needed.text;
            }
            Error::LibraryTooOld { family, found, .. } => {
                detail.family = family_number(*family);
                found_firmware(&mut detail, found, &mut keep);
            }
            Error::Unsupported {
                action,
                family,
                reason,
                ..
            } => {
                with_action(&mut detail, action);
                detail.family = family_number(*family);
                detail.text = keep(reason);
            }
            Error::InvalidArgument {
                action,
                detail: text,
                ..
            } => {
                with_action(&mut detail, action);
                detail.text = keep(text);
            }
            Error::Rejected {
                action,
                detail: text,
                ..
            } => {
                with_action(&mut detail, action);
                detail.text = keep(text);
            }
            Error::Timeout { action, .. } | Error::Interrupted { action, .. } => {
                with_action(&mut detail, action);
            }
            Error::Io(io) => {
                detail.io_kind = io_kind_number(io.kind());
                detail.has_os_error = io.raw_os_error().is_some();
                detail.os_error = io.raw_os_error().unwrap_or(0);
            }
            _ => {}
        }
        Arc::new(Self {
            status,
            message: c_string(error.to_string()),
            detail,
            _strings: strings,
            tried,
            incapable,
        })
    }

    /// what the one box a single port's `NoBoxCan` found couldn't do, empty for any other error
    pub(crate) fn missing(&self) -> Vec<u32> {
        self.incapable
            .iter()
            .flat_map(|(_, missing)| missing.iter().copied())
            .collect()
    }
}

fn with_action(detail: &mut AnyboxErrorDetail, action: &Action) {
    detail.has_action = true;
    detail.action = action_number(*action);
}

fn ferrum_api(api: FerrumApi) -> u32 {
    ferrum_api_number(Some(api))
}

fn found_firmware(
    detail: &mut AnyboxErrorDetail,
    firmware: &Firmware,
    keep: &mut impl FnMut(&str) -> *const c_char,
) {
    let found = firmware_fields(Some(firmware), keep);
    detail.found_firmware_kind = found.kind;
    detail.found_firmware_major = found.major;
    detail.found_firmware_minor = found.minor;
    detail.found_firmware_patch = found.patch;
    detail.found_firmware_protocol = found.protocol;
    detail.found_firmware_build = found.build;
    detail.found_firmware_text = found.text;
}

/// the fields of the last call that failed on this thread, `kind` is `ANYBOX_STATUS_OK` before anything failed.
/// the strings stay valid as long as `anybox_last_error_message` does. this one and the two below never change the
/// last error, a null `out` is `ANYBOX_STATUS_INVALID_ARGUMENT` and leaves it as it was
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_last_error_detail(out: *mut AnyboxErrorDetail) -> AnyboxStatus {
    if out.is_null() {
        return ANYBOX_STATUS_INVALID_ARGUMENT;
    }
    let detail = last_failure().map_or(AnyboxErrorDetail::empty(ANYBOX_STATUS_OK), |failure| {
        failure.detail
    });
    // SAFETY: not null, and the header says it has to be aligned and writable for a whole `AnyboxErrorDetail`
    unsafe { out.write(detail) };
    ANYBOX_STATUS_OK
}

/// the ports the last failure on this thread looked at, when it was `ANYBOX_STATUS_NOT_FOUND`, and an empty list for
/// any other. it's a copy of its own, free it with `anybox_string_list_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_last_error_tried(out: *mut *mut AnyboxStringList) -> AnyboxStatus {
    if out.is_null() {
        return ANYBOX_STATUS_INVALID_ARGUMENT;
    }
    let tried = last_failure().map_or_else(Vec::new, |failure| failure.tried.clone());
    let list = Box::into_raw(Box::new(AnyboxStringList(tried)));
    // SAFETY: not null, and the header says it has to be aligned and writable for a pointer
    unsafe { out.write(list) };
    ANYBOX_STATUS_OK
}

/// the boxes that answered when the last failure on this thread was `ANYBOX_STATUS_NO_BOX_CAN`, an empty list for
/// any other. each entry is a box (its error detail is `ANYBOX_STATUS_OK`) and `anybox_scan_list_missing` says what
/// it couldn't do. it's a copy of its own, free it with `anybox_scan_list_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_last_error_incapable(
    out: *mut *mut AnyboxScanList,
) -> AnyboxStatus {
    if out.is_null() {
        return ANYBOX_STATUS_INVALID_ARGUMENT;
    }
    let entries = last_failure().map_or_else(Vec::new, |failure| {
        failure
            .incapable
            .iter()
            .map(|(found, missing)| Entry {
                port: c_string(found.port.as_str()),
                result: Ok(found.clone()),
                missing: missing.clone(),
            })
            .collect()
    });
    let list = Box::into_raw(Box::new(AnyboxScanList(entries)));
    // SAFETY: not null, and the header says it has to be aligned and writable for a pointer
    unsafe { out.write(list) };
    ANYBOX_STATUS_OK
}
