use std::io;
use std::path::Path;

use super::{open_error, trustworthy_serial};
use crate::error::Error;

#[test]
fn windows_generated_id_fragments_are_not_serials() {
    assert!(!trustworthy_serial("5"));
    assert!(!trustworthy_serial("6&"));
    assert!(!trustworthy_serial("5&1A2B&0&3"));
    assert!(trustworthy_serial("TOWMCULEFT"));
    assert!(trustworthy_serial("58A3012345"));
}

fn mapped(code: i32) -> Error {
    open_error(Path::new("COM9"), io::Error::from_raw_os_error(code))
}

#[cfg(windows)]
#[test]
fn windows_open_errors_say_what_happened() {
    assert!(matches!(mapped(5), Error::PortBusy { .. }));
    assert!(matches!(mapped(2), Error::NotFound { .. }));
    assert!(matches!(mapped(3), Error::NotFound { .. }));
    assert!(matches!(mapped(87), Error::Io(_)));
}

#[cfg(unix)]
#[test]
fn unix_open_errors_say_what_happened() {
    assert!(matches!(mapped(libc::EBUSY), Error::PortBusy { .. }));
    assert!(matches!(mapped(libc::EWOULDBLOCK), Error::PortBusy { .. }));
    assert!(matches!(
        mapped(libc::EACCES),
        Error::PermissionDenied { .. }
    ));
    assert!(matches!(mapped(libc::ENOENT), Error::NotFound { .. }));
}
