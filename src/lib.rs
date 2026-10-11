//! one API for the most popular passthrough boxes. you connect, and anybox figures out which box it is, the
//! firmware, and what it can do, then you call the same methods without having to worry about which box is plugged in
//!
//! ```no_run
//! use anybox::{Button, Event, Key};
//!
//! fn main() -> anybox::Result<()> {
//!     let device = anybox::connect()?;
//!     device.move_by(100, 0)?;
//!     device.click(Button::Left)?;
//!     device.tap_key(Key::A)?;
//!
//!     for event in device.events() {
//!         if let Event::Button { button, pressed: true, .. } = event {
//!             println!("{button:?} down");
//!         }
//!     }
//!     Ok(())
//! }
//! ```
//!
//! if a box can't do something, the call fails with [`Error::Unsupported`] and says why, nothing is faked or
//! changed behind your back. [`Device::snapshot`] tells you up front what the box in front of you can do right
//! now, which also depends on what's plugged into it. things only one box has stay reachable through that box's
//! own API, like `device.medius()` on a Medius
//!
//! the README has the full picture, which box does what, the differences between them, and how to add your own
//! fallback for something a box can't do: <https://github.com/etherealrl/anybox>

// with one box family left out, the shared code only the other one uses has nothing calling it
#![cfg_attr(not(all(feature = "medius", feature = "makcu")), allow(dead_code))]

mod backend;
mod capability;
mod click;
mod connect;
mod device;
mod error;
mod event;
mod family;
mod key;
mod transport;

pub use capability::{
    Action, Axis, Button, Capabilities, Capability, Direction, Execution, Family, Firmware, Info,
    Quirk, Snapshot, StableId,
};
pub use click::{Click, ClickTiming, Timing};
pub use connect::{Connect, Found, connect, find};
pub use device::Device;
pub use error::{Error, Result};
pub use event::{
    BoxTime, Clock, Closed, DropSource, Event, Events, RecvTimeoutError, Stamp, TryRecvError,
};
pub use key::{Key, MediaKey};
#[cfg(feature = "medius")]
pub use medius;

// the readme's examples get compiled with the tests, so they can't drift from the API
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeExamples;
