// the match has no wildcard, so a new kind of error doesn't build until it has an example, which the bindings'
// conformance suite then holds every binding to
#[cfg(feature = "mock")]
#[test]
fn the_examples_have_every_kind_of_error() {
    use super::Error;
    let mut seen = [false; 17];
    for error in Error::examples() {
        let index = match error {
            Error::NotFound { .. } => 0,
            Error::Closed => 1,
            Error::PortBusy { .. } => 2,
            Error::PermissionDenied { .. } => 3,
            Error::NoBoxCan { .. } => 4,
            Error::WrongFamily { .. } => 5,
            Error::WrongBox { .. } => 6,
            Error::WrongFerrumApi { .. } => 7,
            Error::FirmwareTooOld { .. } => 8,
            Error::LibraryTooOld { .. } => 9,
            Error::Unsupported { .. } => 10,
            Error::InvalidArgument { .. } => 11,
            Error::Rejected { .. } => 12,
            Error::Timeout { .. } => 13,
            Error::Disconnected => 14,
            Error::Interrupted { .. } => 15,
            Error::Io(_) => 16,
        };
        seen[index] = true;
    }
    assert!(seen.iter().all(|seen| *seen), "{seen:?}");
}
