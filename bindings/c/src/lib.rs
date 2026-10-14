//! the C ABI over anybox. the header is `include/anybox.h`, made from this crate by `scripts/gen-header.sh`

mod connect;
mod device;
mod events;
// only the header uses these, so they have to be reachable for the compiler to count them as used
pub mod keys;
#[cfg(feature = "mock")]
mod mock;
mod snapshot;
mod status;
mod values;

use std::ffi::c_char;

/// bumped on any change to a struct, a constant or a function, so a caller compares it with `anybox_abi_version()`
/// once at start and calls nothing else when they differ
pub const ANYBOX_ABI_VERSION: u32 = 1;

#[unsafe(no_mangle)]
pub extern "C" fn anybox_abi_version() -> u32 {
    ANYBOX_ABI_VERSION
}

/// the anybox version this library was built from
#[unsafe(no_mangle)]
pub extern "C" fn anybox_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast()
}

/// whether this library was built with the mock functions, ask BEFORE calling any of them
#[unsafe(no_mangle)]
pub extern "C" fn anybox_has_mock() -> bool {
    cfg!(feature = "mock")
}

// they drive everything through the mock box
#[cfg(all(test, feature = "mock"))]
mod tests;
