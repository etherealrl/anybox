//! the fake YSC the crate tests drive, answering the way the vendor's source does, and `Device::mock_ysc` over it
#![cfg_attr(not(test), allow(dead_code))]

pub(crate) fn wire(text: &str) -> Vec<u8> {
    let total = u16::try_from(text.len() + 14).unwrap();
    [
        b"<START>".as_slice(),
        &total.to_be_bytes(),
        text.as_bytes(),
        b"<END>",
    ]
    .concat()
}

pub(crate) struct BoxState {
    pub(crate) version: String,
    pub(crate) debug: Option<String>,
    pub(crate) echo: bool,
    pub(crate) wrong_echo: bool,
    /// only the next text command gets someone else's echo, so a retry can't race the test switching it off
    pub(crate) wrong_echo_once: bool,
    pub(crate) partial: bool,
    pub(crate) partial_text: bool,
    pub(crate) key_error: bool,
    pub(crate) key_silent: bool,
    pub(crate) key_unreadable: bool,
    /// echo is off, and `km.echo(1)` turns it on without echoing itself
    pub(crate) echo_switch: bool,
    /// every text a cmd 47 asked for
    pub(crate) texts: Vec<String>,
    /// what cmd 47 answers instead of typing_started
    pub(crate) text_reply: Option<&'static str>,
    /// how many cmd 48 still say busy for a text
    pub(crate) busy_polls: usize,
    pub(crate) busy_left: usize,
    /// cmd 48 says it typed one character less than it was given
    pub(crate) types_short: bool,
    /// what cmd 48 answers instead of type_status
    pub(crate) status_reply: Option<&'static str>,
    /// cmd 51's e, p, w and mx as the box keeps them
    pub(crate) interp: (u64, u64, u64, u64),
    /// cmd 52 answers only what the vendor's own app reads, enabled, profile and ema
    pub(crate) interp_short: bool,
    /// the ema cmd 52 sends in its full answer, as JSON
    pub(crate) ema: &'static str,
    /// cmd 51 does nothing
    pub(crate) ignores_interp: bool,
    /// cmd 52 and 54 get no answer
    pub(crate) extension_silent: bool,
    /// cmd 52 and 54 get a 400, the box failing to read them
    pub(crate) extension_400: bool,
    /// what cmd 54's data says
    pub(crate) serial: &'static str,
}

impl Default for BoxState {
    fn default() -> Self {
        Self {
            version: "ysc-towmcu-L v1.0".into(),
            debug: Some(r#"{"code":702,"data":"{\"debug\":false}"}"#.into()),
            echo: true,
            wrong_echo: false,
            wrong_echo_once: false,
            partial: false,
            partial_text: false,
            key_error: false,
            key_silent: false,
            key_unreadable: false,
            echo_switch: false,
            texts: Vec::new(),
            text_reply: None,
            busy_polls: 2,
            busy_left: 0,
            types_short: false,
            status_reply: None,
            interp: (1, 2, 0, 50),
            interp_short: false,
            ema: "4",
            ignores_interp: false,
            extension_silent: false,
            extension_400: false,
            serial: r#"{\"custom\":\"\",\"physical\":\"PHYS01\",\"active\":\"PHYS01\"}"#,
        }
    }
}

pub(crate) fn respond(state: &mut BoxState, bytes: &[u8]) -> Vec<Vec<u8>> {
    if bytes == b"<START>\x00\x19{\"cmd\":132}<END>" {
        return vec![wire(&format!(
            "{{\"code\":200,\"message\":\"{}\",\"data\":\"\"}}",
            state.version
        ))];
    }
    if bytes == b"<START>\x00\x19{\"cmd\":702}<END>" {
        return state.debug.as_deref().map(wire).into_iter().collect();
    }
    if bytes.starts_with(b"<START>") {
        let value = super::frames::object(bytes).unwrap_or_default();
        match value.get("cmd").and_then(serde_json::Value::as_u64) {
            Some(47) => {
                state
                    .texts
                    .push(value["s"].as_str().unwrap_or_default().to_owned());
                state.busy_left = state.busy_polls;
                return vec![wire(
                    state
                        .text_reply
                        .unwrap_or(r#"{"code":200,"message":"typing_started","data":""}"#),
                )];
            }
            Some(51) => {
                if !state.ignores_interp {
                    let field = |name: &str| value[name].as_u64().unwrap_or(0);
                    state.interp = (field("e"), field("p"), field("w"), field("mx"));
                }
                return Vec::new();
            }
            Some(53) => {
                // what the vendor's app shows after its reset, on with linear
                state.interp = (1, 0, 0, 50);
                return Vec::new();
            }
            Some(52 | 54) if state.extension_silent => return Vec::new(),
            Some(52 | 54) if state.extension_400 => {
                return vec![wire(r#"{"code":400,"message":"4","data":null}"#)];
            }
            Some(52) => {
                let (e, p, w, mx) = state.interp;
                let data = if state.interp_short {
                    format!(r#"{{\"enabled\":{e},\"profile\":{p},\"ema\":0.5}}"#)
                } else {
                    format!(
                        r#"{{\"enabled\":{e},\"profile\":{p},\"window\":{w},\"maxw\":{mx},\"ema\":{ema}}}"#,
                        ema = state.ema
                    )
                };
                return vec![wire(&format!(
                    r#"{{"code":200,"message":"interp","data":"{data}"}}"#
                ))];
            }
            Some(54) => {
                return vec![wire(&format!(
                    r#"{{"code":200,"message":"sn","data":"{}"}}"#,
                    state.serial
                ))];
            }
            Some(48) if state.status_reply.is_some() => {
                return vec![wire(state.status_reply.unwrap())];
            }
            Some(48) => {
                let total = state.texts.last().map_or(0, String::len);
                let typed = total - usize::from(state.types_short);
                if state.busy_left > 0 {
                    state.busy_left -= 1;
                    return vec![wire(&format!(
                        r#"{{"code":200,"message":"type_status","data":"{{\"busy\":1,\"remaining\":1,\"total\":{total},\"typed\":0,\"skipped\":0}}"}}"#
                    ))];
                }
                // the async type_done comes first, and it isn't cmd 48's answer
                return vec![
                    wire(&format!(
                        r#"{{"code":200,"message":"type_done","data":"{{\"typed\":{typed},\"skipped\":0,\"total\":{total}}}"}}"#
                    )),
                    wire(&format!(
                        r#"{{"code":200,"message":"type_status","data":"{{\"busy\":0,\"remaining\":0,\"total\":{total},\"typed\":{typed},\"skipped\":0}}"}}"#
                    )),
                ];
            }
            _ => {}
        }
        if bytes.windows(8).any(|bytes| bytes == b"\"cmd\":45") {
            if state.key_silent {
                return Vec::new();
            }
            if state.key_unreadable {
                return vec![wire(r#"{"code":400,"message":"2","data":null}"#)];
            }
            if state.key_error {
                return vec![wire(r#"{"code":404,"message":"no_keyboard","data":""}"#)];
            }
            let message = if bytes.ends_with(b"\"down\":1}<END>") {
                "key_down"
            } else {
                "key_up"
            };
            return vec![wire(&format!(
                "{{\"code\":200,\"message\":\"{message}\",\"data\":null}}"
            ))];
        }
        return Vec::new();
    }
    if !state.echo {
        state.echo = state.echo_switch && bytes == b"km.echo(1)\r\n";
        return Vec::new();
    }
    if bytes == b"km.echo(1)\r\n" {
        return vec![b"km.echo(1)\r\n>>> ".to_vec()];
    }
    if state.partial {
        return vec![b"<START>\x00\x80{".to_vec()];
    }
    if state.partial_text {
        return vec![bytes.to_vec()];
    }
    if state.wrong_echo || std::mem::take(&mut state.wrong_echo_once) {
        return vec![b"km.move(99,99)\r\n>>> ".to_vec()];
    }
    vec![[bytes, b">>> "].concat()]
}

pub(crate) fn usb(role: Option<&str>) -> Option<crate::transport::PortUsb> {
    Some(crate::transport::PortUsb {
        vid: 0x1a86,
        pid: 0xfe0c,
        serial: role.map(str::to_owned),
    })
}

#[cfg(feature = "mock")]
impl crate::device::Device {
    /// a device over anybox's own model of a YSC8K V1, for testing your code without a box. it runs the exact code
    /// a real one gets over a fake port, so it proves your code works and nothing about the hardware
    pub fn mock_ysc(click_timing: crate::click::ClickTiming) -> crate::error::Result<Self> {
        use std::sync::{Arc, Mutex, mpsc};

        use crate::backend::Sink;
        use crate::device::Start;
        use crate::transport::Candidate;
        use crate::transport::fake::{FakeBox, FakeSystem};

        const PORT: &str = "mock YSC8K";
        let state = Arc::new(Mutex::new(BoxState::default()));
        let fake = FakeBox::new(move |bytes| respond(&mut state.lock().unwrap(), bytes));
        let system = FakeSystem::default();
        system.plug(PORT, usb(Some("TOWMCULEFT")), fake);
        let candidate = Candidate {
            path: PORT.into(),
            usb: usb(Some("TOWMCULEFT")),
        };
        let (signals, incoming) = mpsc::channel();
        let (backend, snapshot) =
            super::attach(Arc::new(system), &candidate, Sink(signals.clone()))?;
        Self::start(Start {
            backend: Box::new(backend),
            snapshot,
            signals,
            incoming,
            click_timing,
            holds: crate::click::Holds::from_clock(),
            port: PORT.into(),
        })
    }
}
