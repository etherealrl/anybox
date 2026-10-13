use std::path::{Path, PathBuf};

use serialport::SerialPortType;

use super::{Candidate, Port, PortUsb, System, open_error, trustworthy_serial};
use crate::error::Result;

/// the real serial ports of this machine
#[derive(Debug, Default)]
pub(crate) struct OsSystem;

impl System for OsSystem {
    fn ports(&self) -> Vec<Candidate> {
        // if the port list can't be read there's just nothing to try, and the caller reports NotFound
        let ports = serialport::available_ports().unwrap_or_default();
        ports
            .into_iter()
            .map(|port| Candidate {
                path: PathBuf::from(port.port_name),
                usb: match port.port_type {
                    SerialPortType::UsbPort(usb) => Some(PortUsb {
                        vid: usb.vid,
                        pid: usb.pid,
                        serial: usb.serial_number.filter(|s| trustworthy_serial(s)),
                    }),
                    _ => None,
                },
            })
            .collect()
    }

    fn open(&self, path: &Path, baud: u32) -> Result<Box<dyn Port>> {
        #[cfg(windows)]
        let port = super::windows::open(path, baud);
        #[cfg(unix)]
        let port = super::unix::open(path, baud);
        let port = port.map_err(|error| open_error(path, error));
        #[cfg(feature = "trace")]
        let port = super::trace::wrap(path, port);
        port
    }
}

/// when anybox read some bytes off a port, taken right after the read returns
#[derive(Clone, Copy, Debug)]
pub(crate) struct Read {
    pub(crate) at: std::time::Instant,
    pub(crate) qpc: Option<i64>,
}

impl Read {
    pub(crate) fn now() -> Self {
        #[cfg(windows)]
        let qpc = super::windows::qpc();
        #[cfg(not(windows))]
        let qpc = None;
        Self {
            at: std::time::Instant::now(),
            qpc,
        }
    }
}
