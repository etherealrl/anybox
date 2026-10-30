//! the C ABI over anybox. the header is `include/anybox.h`, made from this crate by `scripts/gen-header.sh`

mod connect;
mod device;
mod extension;
// the box clock wrap is only used by the header
pub mod events;
mod failure;
// only the header uses these, so they have to be reachable for the compiler to count them as used
pub mod keys;
#[cfg(feature = "mock")]
mod mock;
mod snapshot;
mod status;
// ANYBOX_EVENTS_NONE is only used by the header, like the keys
pub mod values;

use std::ffi::c_char;

/// bumped on any change to a struct, a constant or a function, so a caller compares it with `anybox_abi_version()`
/// once at start and calls nothing else when they differ
pub const ANYBOX_ABI_VERSION: u32 = 3;

#[unsafe(no_mangle)]
pub extern "C" fn anybox_abi_version() -> u32 {
    ANYBOX_ABI_VERSION
}

/// the anybox version this library was built from
#[unsafe(no_mangle)]
pub extern "C" fn anybox_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast()
}

/// now in nanoseconds on the OS monotonic clock, the one the `arrived_ns` and `received_ns` of an event are on:
/// QueryPerformanceCounter on Windows, CLOCK_MONOTONIC on Linux and CLOCK_UPTIME_RAW on macOS. it only compares on
/// the same machine within one boot
#[unsafe(no_mangle)]
pub extern "C" fn anybox_monotonic_ns() -> u64 {
    anybox::monotonic_ns()
}

/// whether this library was built with the mock functions, ask BEFORE calling any of them
#[unsafe(no_mangle)]
pub extern "C" fn anybox_has_mock() -> bool {
    cfg!(feature = "mock")
}

// they drive everything through the mock box
#[cfg(all(test, feature = "mock"))]
mod tests;

#[cfg(all(test, feature = "mock"))]
mod conformance;

// the ledger checks share the conformance suite's helpers
#[cfg(all(test, feature = "mock"))]
mod surface;

// the Medius suite records what the medius crate does for the C++ and C# runners
#[cfg(all(test, feature = "mock"))]
mod medius_suite;
