use std::fmt::Write as _;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use super::Port;
use crate::error::{Error, Result};

/// the file `ANYBOX_TRACE` names, opened the first time a port is. every port writes into the same one, so the
/// lines from discovery, the reader and a reopen end up in the order they happened
pub(crate) struct Trace {
    out: Mutex<Box<dyn Write + Send>>,
}

impl Trace {
    pub(crate) fn new(out: Box<dyn Write + Send>) -> Self {
        Self {
            out: Mutex::new(out),
        }
    }

    fn line(&self, port: &str, what: &str, bytes: &[u8]) {
        // seconds since 1970, so the lines match up with whatever the app logs next to them
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let mut line = format!("{:.6} {port} {what}", now.as_secs_f64());
        if !bytes.is_empty() {
            let _ = write!(line, " {}", bytes.len());
            for byte in bytes {
                let _ = write!(line, " {byte:02x}");
            }
            let text: String = bytes
                .iter()
                .map(|&b| {
                    if b.is_ascii_graphic() || b == b' ' {
                        b as char
                    } else {
                        '.'
                    }
                })
                .collect();
            let _ = write!(line, " |{text}|");
        }
        line.push('\n');
        // a trace that can't be written must never break the box it's watching
        let _ = self.out.lock().unwrap().write_all(line.as_bytes());
    }

    fn flush(&self) {
        let _ = self.out.lock().unwrap().flush();
    }
}

/// wraps the port when `ANYBOX_TRACE` is set. a trace file that can't be created fails the open, because a run
/// that was asked to record and quietly didn't is worse than one that stops
pub(crate) fn wrap(path: &Path, port: Result<Box<dyn Port>>) -> Result<Box<dyn Port>> {
    static TRACE: OnceLock<Option<std::result::Result<Arc<Trace>, String>>> = OnceLock::new();
    let trace =
        TRACE.get_or_init(|| std::env::var_os("ANYBOX_TRACE").map(|file| create(Path::new(&file))));
    match trace {
        None => port,
        Some(Ok(trace)) => traced(path, port, trace.clone()),
        Some(Err(message)) => Err(Error::Io(io::Error::other(message.clone()))),
    }
}

pub(crate) fn create(file: &Path) -> std::result::Result<Arc<Trace>, String> {
    File::create(file)
        // a big buffer so the reader thread almost never waits on the disk, a close flushes it anyway
        .map(|f| Arc::new(Trace::new(Box::new(BufWriter::with_capacity(1 << 20, f)))))
        .map_err(|error| format!("couldn't create the trace file {}: {error}", file.display()))
}

/// a failed open goes in too, it's often the most useful line when a box won't connect
pub(crate) fn traced(
    path: &Path,
    port: Result<Box<dyn Port>>,
    trace: Arc<Trace>,
) -> Result<Box<dyn Port>> {
    let path = path.display().to_string();
    match port {
        Ok(port) => {
            trace.line(&path, "open", &[]);
            Ok(Box::new(Traced { port, path, trace }))
        }
        Err(error) => {
            trace.line(&path, &format!("open failed {error}"), &[]);
            trace.flush();
            Err(error)
        }
    }
}

struct Traced {
    port: Box<dyn Port>,
    path: String,
    trace: Arc<Trace>,
}

impl Port for Traced {
    fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        let read = self.port.read(buffer);
        match &read {
            Ok(0) => self.trace.line(&self.path, "eof", &[]),
            Ok(n) => self.trace.line(&self.path, "read", &buffer[..*n]),
            // the reader times out every READ_SLICE, logging that would bury everything else
            Err(error) if error.kind() == io::ErrorKind::TimedOut => {}
            Err(error) => self
                .trace
                .line(&self.path, &format!("read error {error}"), &[]),
        }
        read
    }

    fn write_all(&self, bytes: &[u8]) -> io::Result<()> {
        let written = self.port.write_all(bytes);
        // a failed write may have sent part of it or nothing, so it's never shown as bytes that went out
        let what = if written.is_ok() {
            "write ok"
        } else {
            "write failed"
        };
        self.trace.line(&self.path, what, bytes);
        written
    }

    fn wake(&self) {
        self.port.wake();
    }

    fn retire(&self) {
        self.port.retire();
    }
}

impl Drop for Traced {
    fn drop(&mut self) {
        self.trace.line(&self.path, "close", &[]);
        self.trace.flush();
    }
}

#[cfg(test)]
mod tests;
