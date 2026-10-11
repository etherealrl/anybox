use std::collections::{HashMap, VecDeque};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use super::{Candidate, Port, READ_SLICE, System, UsbId, open_error};
use crate::error::Result;

type Responder = Box<dyn FnMut(u32, &[u8]) -> Vec<Vec<u8>> + Send>;

/// a box on the other end of a fake port, it answers whatever the host writes through its responder
pub(crate) struct FakeBox {
    to_host: Mutex<VecDeque<u8>>,
    arrived: Condvar,
    written: Mutex<Vec<Vec<u8>>>,
    responder: Mutex<Responder>,
    gone: AtomicBool,
    /// the most one read hands over, so a test can split frames across reads
    pub(crate) max_read: AtomicUsize,
    stall_writes: AtomicBool,
    /// a write only returns once the host has read its reply, so the reply always lands before the write returns
    pub(crate) reply_before_return: AtomicBool,
}

impl FakeBox {
    pub(crate) fn new(
        mut responder: impl FnMut(&[u8]) -> Vec<Vec<u8>> + Send + 'static,
    ) -> Arc<Self> {
        Self::by_baud(move |_, bytes| responder(bytes))
    }

    /// a box whose answers depend on the baud rate the port was opened at, like a real box that only
    /// understands its own rate
    pub(crate) fn by_baud(
        responder: impl FnMut(u32, &[u8]) -> Vec<Vec<u8>> + Send + 'static,
    ) -> Arc<Self> {
        Arc::new(Self {
            to_host: Mutex::new(VecDeque::new()),
            arrived: Condvar::new(),
            written: Mutex::new(Vec::new()),
            responder: Mutex::new(Box::new(responder)),
            gone: AtomicBool::new(false),
            max_read: AtomicUsize::new(usize::MAX),
            stall_writes: AtomicBool::new(false),
            reply_before_return: AtomicBool::new(false),
        })
    }

    pub(crate) fn send(&self, bytes: &[u8]) {
        self.to_host.lock().unwrap().extend(bytes);
        self.arrived.notify_all();
    }

    pub(crate) fn written(&self) -> Vec<Vec<u8>> {
        self.written.lock().unwrap().clone()
    }

    pub(crate) fn clear_written(&self) {
        self.written.lock().unwrap().clear();
    }

    pub(crate) fn vanish(&self) {
        self.gone.store(true, Ordering::Release);
        self.arrived.notify_all();
    }

    /// every write takes longer than any deadline, like a port that stopped draining
    pub(crate) fn stall_writes(&self) {
        self.stall_writes.store(true, Ordering::Release);
    }
}

#[derive(Default)]
struct State {
    boxes: HashMap<PathBuf, (Option<UsbId>, Arc<FakeBox>)>,
    refuse: Option<i32>,
    opened_at: Vec<u32>,
}

#[derive(Clone, Default)]
pub(crate) struct FakeSystem {
    state: Arc<Mutex<State>>,
    open_ports: Arc<AtomicUsize>,
    retired: Arc<AtomicUsize>,
}

impl FakeSystem {
    pub(crate) fn plug(&self, path: &str, usb: Option<UsbId>, fake: Arc<FakeBox>) {
        self.state
            .lock()
            .unwrap()
            .boxes
            .insert(PathBuf::from(path), (usb, fake));
    }

    pub(crate) fn unplug(&self, path: &str) {
        if let Some((_, fake)) = self.state.lock().unwrap().boxes.remove(Path::new(path)) {
            fake.vanish();
        }
    }

    /// every open after this fails with this raw OS error code
    pub(crate) fn refuse(&self, code: Option<i32>) {
        self.state.lock().unwrap().refuse = code;
    }

    /// how many times a reading thread said it was done with its port
    pub(crate) fn retired(&self) -> usize {
        self.retired.load(Ordering::Acquire)
    }

    /// the baud rate of every open so far, in order
    pub(crate) fn opened_at(&self) -> Vec<u32> {
        self.state.lock().unwrap().opened_at.clone()
    }

    pub(crate) fn open_ports(&self) -> usize {
        self.open_ports.load(Ordering::Acquire)
    }
}

impl System for FakeSystem {
    fn ports(&self) -> Vec<Candidate> {
        self.state
            .lock()
            .unwrap()
            .boxes
            .iter()
            .map(|(path, (usb, _))| Candidate {
                path: path.clone(),
                usb: usb.clone(),
            })
            .collect()
    }

    fn open(&self, path: &Path, baud: u32) -> Result<Box<dyn Port>> {
        let mut state = self.state.lock().unwrap();
        if let Some(code) = state.refuse {
            return Err(open_error(path, io::Error::from_raw_os_error(code)));
        }
        let missing = || open_error(path, io::Error::from_raw_os_error(2));
        let (_, fake) = state.boxes.get(path).ok_or_else(missing)?;
        if fake.gone.load(Ordering::Acquire) {
            return Err(missing());
        }
        let fake = fake.clone();
        state.opened_at.push(baud);
        self.open_ports.fetch_add(1, Ordering::AcqRel);
        Ok(Box::new(FakePort {
            fake,
            baud,
            open_ports: self.open_ports.clone(),
            woken: AtomicBool::new(false),
            retired: self.retired.clone(),
        }))
    }
}

struct FakePort {
    fake: Arc<FakeBox>,
    baud: u32,
    open_ports: Arc<AtomicUsize>,
    woken: AtomicBool,
    retired: Arc<AtomicUsize>,
}

impl Drop for FakePort {
    fn drop(&mut self) {
        self.open_ports.fetch_sub(1, Ordering::AcqRel);
    }
}

impl Port for FakePort {
    fn read(&self, buffer: &mut [u8]) -> io::Result<usize> {
        let deadline = Instant::now() + READ_SLICE;
        let mut to_host = self.fake.to_host.lock().unwrap();
        loop {
            if self.fake.gone.load(Ordering::Acquire) {
                return Ok(0);
            }
            if !to_host.is_empty() {
                let n = buffer
                    .len()
                    .min(to_host.len())
                    .min(self.fake.max_read.load(Ordering::Acquire));
                for (slot, byte) in buffer.iter_mut().zip(to_host.drain(..n)) {
                    *slot = byte;
                }
                return Ok(n);
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() || self.woken.swap(false, Ordering::AcqRel) {
                return Err(io::ErrorKind::TimedOut.into());
            }
            to_host = self.fake.arrived.wait_timeout(to_host, left).unwrap().0;
        }
    }

    fn write_all(&self, bytes: &[u8]) -> io::Result<()> {
        if self.fake.gone.load(Ordering::Acquire) {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        if self.fake.stall_writes.load(Ordering::Acquire) {
            std::thread::sleep(Duration::from_millis(700));
            return Err(io::ErrorKind::TimedOut.into());
        }
        self.fake.written.lock().unwrap().push(bytes.to_vec());
        let replies = (self.fake.responder.lock().unwrap())(self.baud, bytes);
        for reply in replies {
            self.fake.send(&reply);
        }
        if self.fake.reply_before_return.load(Ordering::Acquire) {
            let deadline = Instant::now() + Duration::from_secs(1);
            let mut to_host = self.fake.to_host.lock().unwrap();
            while !to_host.is_empty() && Instant::now() < deadline {
                to_host = self
                    .fake
                    .arrived
                    .wait_timeout(to_host, Duration::from_millis(5))
                    .unwrap()
                    .0;
            }
        }
        Ok(())
    }

    fn wake(&self) {
        self.woken.store(true, Ordering::Release);
        self.fake.arrived.notify_all();
    }

    fn retire(&self) {
        self.retired.fetch_add(1, Ordering::AcqRel);
    }
}
