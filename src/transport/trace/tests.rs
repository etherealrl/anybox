use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use super::{Trace, create, traced};
use crate::transport::System;
use crate::transport::fake::{FakeBox, FakeSystem};

#[derive(Clone, Default)]
struct Shared(Arc<Mutex<Vec<u8>>>);

impl Write for Shared {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Shared {
    /// every line without its timestamp
    fn lines(&self) -> Vec<String> {
        String::from_utf8(self.0.lock().unwrap().clone())
            .unwrap()
            .lines()
            .map(|line| line.split_once(' ').unwrap().1.to_string())
            .collect()
    }
}

fn rig() -> (FakeSystem, Arc<FakeBox>, Shared, Arc<Trace>) {
    let system = FakeSystem::default();
    let fake = FakeBox::new(|bytes| {
        if bytes == b"ping" {
            vec![b"pong\r\n".to_vec()]
        } else {
            Vec::new()
        }
    });
    system.plug("COM3", None, fake.clone());
    let out = Shared::default();
    let trace = Arc::new(Trace::new(Box::new(out.clone())));
    (system, fake, out, trace)
}

#[test]
fn reads_writes_and_closes_end_up_in_order_with_their_bytes() {
    let (system, _fake, out, trace) = rig();
    let port = traced(
        Path::new("COM3"),
        system.open(Path::new("COM3"), 115_200),
        trace,
    )
    .unwrap();
    port.write_all(b"ping").unwrap();
    let mut buffer = [0u8; 16];
    assert_eq!(port.read(&mut buffer).unwrap(), 6);
    assert!(port.read(&mut buffer).is_err());
    drop(port);
    assert_eq!(
        out.lines(),
        [
            "COM3 open",
            "COM3 write ok 4 70 69 6e 67 |ping|",
            "COM3 read 6 70 6f 6e 67 0d 0a |pong..|",
            "COM3 close",
        ]
    );
}

#[test]
fn a_failed_write_and_a_vanished_port_say_so() {
    let (system, fake, out, trace) = rig();
    let port = traced(
        Path::new("COM3"),
        system.open(Path::new("COM3"), 115_200),
        trace,
    )
    .unwrap();
    fake.vanish();
    assert!(port.write_all(b"x").is_err());
    let mut buffer = [0u8; 16];
    assert_eq!(port.read(&mut buffer).unwrap(), 0);
    assert_eq!(out.lines()[1..], ["COM3 write failed 1 78 |x|", "COM3 eof"]);
}

#[test]
fn a_reopen_writes_into_the_same_trace() {
    let (system, _fake, out, trace) = rig();
    for _ in 0..2 {
        let port = traced(
            Path::new("COM3"),
            system.open(Path::new("COM3"), 115_200),
            trace.clone(),
        )
        .unwrap();
        port.write_all(b"ping").unwrap();
    }
    let lines = out.lines();
    assert_eq!(lines.iter().filter(|l| *l == "COM3 open").count(), 2);
    assert_eq!(lines.iter().filter(|l| *l == "COM3 close").count(), 2);
}

#[test]
fn a_trace_file_that_cant_be_created_says_where() {
    let missing = Path::new("no such folder").join("trace.txt");
    let error = create(&missing).err().unwrap();
    assert!(error.contains("trace.txt"), "{error}");
}

#[test]
fn a_failed_open_is_in_the_trace_and_keeps_its_error() {
    let (system, _fake, out, trace) = rig();
    let opened = traced(
        Path::new("COM9"),
        system.open(Path::new("COM9"), 115_200),
        trace,
    );
    assert!(matches!(opened, Err(crate::error::Error::NotFound { .. })));
    let lines = out.lines();
    assert_eq!(lines.len(), 1);
    assert!(
        lines[0].starts_with("COM9 open failed no box answered"),
        "{lines:?}"
    );
}
