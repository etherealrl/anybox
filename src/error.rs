use std::borrow::Cow;

use crate::capability::{Action, Family, FerrumApi, Firmware, StableId};

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("no box answered{}, check it's plugged in and give it a few seconds if you just plugged it in. without a port, anybox asks CH343 ports (1A86:55D3) and YSC V2 left ports (1A86:FE0C with serial TOWMCULEFT), so if your box sits behind another chip, name its port with Connect::port", tried_text(.tried))]
    #[non_exhaustive]
    NotFound { tried: Vec<String> },

    #[error("this device was closed, open the box again if you need it")]
    Closed,

    #[error(
        "{port} is already open in another program or another Device in this one, close it there first"
    )]
    #[non_exhaustive]
    PortBusy { port: String },

    #[error(
        "you're not allowed to open {port}, on Linux that usually means adding yourself to the dialout group"
    )]
    #[non_exhaustive]
    PermissionDenied { port: String },

    #[error("{}, so plug in a box that can, or ask Connect::can for less", incapable_text(.boxes))]
    #[non_exhaustive]
    NoBoxCan {
        /// each box that answered, with what it couldn't do
        boxes: Vec<(crate::connect::Found, Vec<Action>)>,
    },

    #[error("the box on {port} isn't a {expected}{}, pick the right family or leave it out and let anybox find it. if you just plugged it in, give it a couple seconds to start first", found_text(.found))]
    #[non_exhaustive]
    WrongFamily {
        port: String,
        expected: Family,
        found: Option<Family>,
    },

    #[error("the box on {port} is {}, not {expected}, so use Connect::auto().id(..) and anybox finds it on whatever port it's on", found_id_text(.found))]
    #[non_exhaustive]
    WrongBox {
        port: String,
        expected: StableId,
        found: Option<StableId>,
    },

    #[error("the Ferrum on {port} is on {found}, not {expected}{}", ferrum_api_hint(*.expected))]
    #[non_exhaustive]
    WrongFerrumApi {
        port: String,
        expected: FerrumApi,
        found: FerrumApi,
    },

    #[error(
        "this {family} runs firmware {found} and anybox needs {needed} or newer, so update the box first"
    )]
    #[non_exhaustive]
    FirmwareTooOld {
        family: Family,
        found: Firmware,
        needed: Firmware,
    },

    #[error(
        "this {family} runs firmware {found}, which is newer than this anybox knows, so update anybox"
    )]
    #[non_exhaustive]
    LibraryTooOld { family: Family, found: Firmware },

    #[error("{family} can't {action}, {reason}")]
    #[non_exhaustive]
    Unsupported {
        action: Action,
        family: Family,
        reason: Cow<'static, str>,
    },

    #[error("can't {action} like that, {detail}")]
    #[non_exhaustive]
    InvalidArgument {
        action: Action,
        detail: Cow<'static, str>,
    },

    #[error("the box refused to {action}, {detail}")]
    #[non_exhaustive]
    Rejected { action: Action, detail: String },

    #[error(
        "the box didn't answer in time when asked to {action}, check it's still plugged in and nothing else is talking to it"
    )]
    #[non_exhaustive]
    Timeout { action: Action },

    #[error(
        "the box is disconnected and anybox is already trying to reconnect, so try again once you get Reconnected"
    )]
    Disconnected,

    #[error(
        "couldn't {action} because the box reconnected or reset in the middle of it. anybox already let go of it, so do it again if you still want it"
    )]
    #[non_exhaustive]
    Interrupted { action: Action },

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

fn incapable_text(boxes: &[(crate::connect::Found, Vec<Action>)]) -> String {
    let each: Vec<String> = boxes
        .iter()
        .map(|(found, missing)| {
            let missing: Vec<String> = missing.iter().map(ToString::to_string).collect();
            format!(
                "the {} on {} can't {}",
                found.info.family,
                found.port,
                missing.join(" or ")
            )
        })
        .collect();
    each.join(", ")
}

fn tried_text(tried: &[String]) -> String {
    if tried.is_empty() {
        String::new()
    } else {
        format!(" on {}", tried.join(", "))
    }
}

fn found_id_text(found: &Option<StableId>) -> String {
    match found {
        Some(id) => id.to_string(),
        None => "a box with no id".into(),
    }
}

fn found_text(found: &Option<Family>) -> String {
    match found {
        Some(family) => format!(", it's a {family}"),
        None => String::new(),
    }
}

fn ferrum_api_hint(expected: FerrumApi) -> &'static str {
    match expected {
        FerrumApi::Software => {
            ", so start the Ferrum app and open its port, or ask for .ferrum_api(FerrumApi::Legacy) if the box's own port is really what you want"
        }
        _ => ", so open the box's own port (the CP210x one) instead",
    }
}

#[cfg(feature = "mock")]
impl Error {
    /// one of each error with every field it has filled in, for testing how code handles them with no box
    pub fn examples() -> Vec<Error> {
        use crate::capability::{Capabilities, Info};
        let mut makcu = Info::new(Family::MakcuV4);
        makcu.port = "COM5".into();
        let found = crate::connect::Found {
            port: "COM5".into(),
            info: makcu,
            capabilities: Capabilities::new(Vec::new()),
        };
        vec![
            Error::NotFound {
                tried: vec!["COM3".into(), "COM4".into()],
            },
            Error::Closed,
            Error::PortBusy {
                port: "COM3".into(),
            },
            Error::PermissionDenied {
                port: "/dev/ttyACM0".into(),
            },
            Error::NoBoxCan {
                boxes: vec![(found, vec![Action::KeyTap, Action::LockKey])],
            },
            Error::WrongFamily {
                port: "COM3".into(),
                expected: Family::Ferrum,
                found: Some(Family::MakcuV4),
            },
            Error::WrongBox {
                port: "COM3".into(),
                expected: StableId::UsbSerial("ABC123".into()),
                found: Some(StableId::Mac([1, 2, 3, 4, 5, 6])),
            },
            Error::WrongFerrumApi {
                port: "COM3".into(),
                expected: FerrumApi::Software,
                found: FerrumApi::Legacy,
            },
            Error::FirmwareTooOld {
                family: Family::Medius,
                found: Firmware::Protocol(8),
                needed: Firmware::Version {
                    major: 3,
                    minor: 4,
                    patch: 0,
                },
            },
            Error::LibraryTooOld {
                family: Family::MakcuV4,
                found: Firmware::Text("V9.9".into()),
            },
            Error::Unsupported {
                action: Action::KeyPress,
                family: Family::Medius,
                reason: "no keyboard is plugged into it".into(),
            },
            Error::InvalidArgument {
                action: Action::ScaleAxis,
                detail: "300 is out of range, a scale goes from -255 to 255".into(),
            },
            Error::Rejected {
                action: Action::Move,
                detail: "its queue is full".into(),
            },
            Error::Timeout {
                action: Action::LockAxis,
            },
            Error::Disconnected,
            Error::Interrupted {
                action: Action::Click,
            },
            // access denied on Windows, an I/O error on Linux and macOS
            Error::Io(std::io::Error::from_raw_os_error(5)),
        ]
    }
}

#[cfg(test)]
mod tests;
