use std::collections::VecDeque;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use super::codec::{Codec, Frame, Incoming, Key};
use super::probe::Probe;
use super::{Candidate, Port, System, UsbId};
use crate::backend::{Sink, WRITE_DEADLINE};
use crate::capability::{Family, Snapshot, StableId};
use crate::error::{Error, Result};

const BACKOFF_START: Duration = Duration::from_millis(100);
const BACKOFF_MAX: Duration = Duration::from_secs(2);

/// which box this link belongs to, so a reconnect only ever reattaches to that one
#[derive(Clone, Debug)]
pub(crate) struct Target {
    pub(crate) family: Family,
    pub(crate) path: PathBuf,
    pub(crate) usb: Option<UsbId>,
    pub(crate) id: Option<StableId>,
    /// tried in order on every reconnect, so a box that got switched to another baud rate still comes back
    pub(crate) bauds: Vec<u32>,
}

/// asks a port that was just reopened what box it is, the same way detection did
pub(crate) type Reprobe = Box<dyn Fn(&Probe<'_>) -> Result<Snapshot> + Send + Sync>;

type Reply = Sender<Result<Option<Frame>>>;

struct Request {
    id: u64,
    bytes: Vec<u8>,
    expect: Option<Key>,
    deadline: Instant,
    given_up: Arc<AtomicBool>,
    reply: Reply,
}

struct Pending {
    id: u64,
    key: Key,
    /// which opening of the port the request went out on, because a reply that comes in on a later one isn't
    /// its reply
    generation: u64,
    reply: Reply,
}

/// the port that's open right now. it gets a number so work that started on an older opening can't touch it
type Current = Option<(Arc<dyn Port>, u64)>;

struct Shared {
    system: Arc<dyn System>,
    port: RwLock<Current>,
    pending: Mutex<VecDeque<Pending>>,
    stop: AtomicBool,
    down: AtomicBool,
    next_id: AtomicU64,
    generations: AtomicU64,
    sink: Sink,
}

/// a reader thread and a writer thread on one port, used by every family that doesn't bring its own transport
pub(crate) struct Link {
    shared: Arc<Shared>,
    queue: Mutex<Option<Sender<Request>>>,
    threads: Mutex<Vec<(JoinHandle<()>, Receiver<()>)>>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Link {
    pub(crate) fn start(
        system: Arc<dyn System>,
        port: Box<dyn Port>,
        codec: Box<dyn Codec>,
        target: Target,
        reprobe: Reprobe,
        sink: Sink,
    ) -> Result<Self> {
        let port: Arc<dyn Port> = Arc::from(port);
        let shared = Arc::new(Shared {
            system,
            port: RwLock::new(Some((port.clone(), 0))),
            pending: Mutex::new(VecDeque::new()),
            stop: AtomicBool::new(false),
            down: AtomicBool::new(false),
            next_id: AtomicU64::new(0),
            generations: AtomicU64::new(1),
            sink,
        });
        let (queue, requests) = mpsc::channel();
        let reader = {
            let shared = shared.clone();
            spawn("anybox-reader", move || {
                read_loop(&shared, Some((port, 0)), codec, target, &reprobe);
            })?
        };
        let writer = {
            let shared = shared.clone();
            spawn("anybox-writer", move || write_loop(&shared, &requests))?
        };
        Ok(Self {
            shared,
            queue: Mutex::new(Some(queue)),
            threads: Mutex::new(vec![reader, writer]),
        })
    }

    /// sends the bytes and, when `expect` is set, waits for the reply with that key. after `WRITE_DEADLINE`
    /// it gives up and returns `Disconnected`, and a missed reply also takes the link down
    pub(crate) fn request(&self, bytes: Vec<u8>, expect: Option<Key>) -> Result<Option<Frame>> {
        self.request_until(bytes, expect, Instant::now() + WRITE_DEADLINE)
    }

    /// same as `request`, but it gives up at `until` when that comes before `WRITE_DEADLINE`
    pub(crate) fn request_until(
        &self,
        bytes: Vec<u8>,
        expect: Option<Key>,
        until: Instant,
    ) -> Result<Option<Frame>> {
        let (reply, answer) = mpsc::channel();
        let id = self.shared.next_id.fetch_add(1, Ordering::Relaxed);
        let deadline = until.min(Instant::now() + WRITE_DEADLINE);
        let given_up = Arc::new(AtomicBool::new(false));
        let request = Request {
            id,
            bytes,
            expect,
            deadline,
            given_up: given_up.clone(),
            reply,
        };
        match lock(&self.queue).as_ref() {
            Some(queue) if queue.send(request).is_ok() => {}
            _ => return Err(Error::Disconnected),
        }
        match answer.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(result) => result,
            Err(_) => {
                given_up.store(true, Ordering::Release);
                let missed = {
                    let mut pending = lock(&self.shared.pending);
                    let index = pending.iter().position(|p| p.id == id);
                    index.and_then(|index| pending.remove(index))
                };
                // after a missed reply there's no way to tell which later reply belongs to which request, so
                // the box gets probed again. if losing the link already failed this request, there's
                // nothing left to take down
                if let Some(missed) = missed {
                    self.shared.take_down(missed.generation);
                }
                Err(Error::Disconnected)
            }
        }
    }

    /// runs `work` only while the port is open and not being taken down. it holds on to the port the whole
    /// time, so the reader can't drop it (and report Disconnected) until `work` is done
    pub(crate) fn while_up<R>(&self, work: impl FnOnce() -> R) -> Option<R> {
        let port = self
            .shared
            .port
            .read()
            .unwrap_or_else(PoisonError::into_inner);
        (port.is_some() && !self.shared.down.load(Ordering::Acquire)).then(work)
    }

    pub(crate) fn shutdown(&self, deadline: Duration) {
        self.shared.stop.store(true, Ordering::Release);
        if let Some((port, _)) = self.shared.current() {
            port.wake();
        }
        lock(&self.queue).take();
        let until = Instant::now() + deadline;
        for (handle, done) in lock(&self.threads).drain(..) {
            // a thread that's still stuck after the deadline is left behind, it closes its port once it sees the flag
            if done
                .recv_timeout(until.saturating_duration_since(Instant::now()))
                .is_ok()
            {
                let _ = handle.join();
            }
        }
    }
}

impl Drop for Link {
    fn drop(&mut self) {
        self.shutdown(WRITE_DEADLINE);
    }
}

impl Shared {
    fn current(&self) -> Current {
        self.port
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn set_port(&self, port: Current) {
        *self.port.write().unwrap_or_else(PoisonError::into_inner) = port;
    }

    fn stopping(&self) -> bool {
        self.stop.load(Ordering::Acquire)
    }

    /// asks the reader to drop the port, because it's the only thread allowed to retire the port's read.
    /// a failure that happened on an older opening of the port leaves the current one alone
    fn take_down(&self, generation: u64) {
        if let Some((port, current)) = self.current()
            && current == generation
        {
            self.down.store(true, Ordering::Release);
            port.wake();
        }
    }

    /// returns false when nothing was waiting for this key
    fn complete(&self, key: Key, generation: u64, frame: Frame) -> bool {
        let mut pending = lock(&self.pending);
        let index = pending
            .iter()
            .position(|p| p.key == key && p.generation == generation);
        match index.and_then(|index| pending.remove(index)) {
            Some(waiting) => {
                let _ = waiting.reply.send(Ok(Some(frame)));
                true
            }
            None => false,
        }
    }

    fn fail_pending(&self) {
        for waiting in lock(&self.pending).drain(..) {
            let _ = waiting.reply.send(Err(Error::Disconnected));
        }
    }
}

fn spawn(
    name: &str,
    body: impl FnOnce() + Send + 'static,
) -> io::Result<(JoinHandle<()>, Receiver<()>)> {
    let (done, finished) = mpsc::channel();
    let handle = thread::Builder::new().name(name.into()).spawn(move || {
        body();
        let _ = done.send(());
    })?;
    Ok((handle, finished))
}

fn write_loop(shared: &Shared, requests: &Receiver<Request>) {
    for request in requests.iter() {
        if shared.stopping() {
            break;
        }
        // the caller already got Disconnected, so writing it now would do something nobody is waiting for
        if request.given_up.load(Ordering::Acquire) || Instant::now() >= request.deadline {
            continue;
        }
        let Some((port, generation)) = shared.current() else {
            let _ = request.reply.send(Err(Error::Disconnected));
            continue;
        };
        // this is registered before the write, so a reply that arrives before the write returns still finds
        // its request
        if let Some(key) = request.expect {
            lock(&shared.pending).push_back(Pending {
                id: request.id,
                key,
                generation,
                reply: request.reply.clone(),
            });
        }
        match port.write_all(&request.bytes) {
            Ok(()) => {
                if request.expect.is_none() {
                    let _ = request.reply.send(Ok(None));
                }
            }
            Err(_) => {
                lock(&shared.pending).retain(|pending| pending.id != request.id);
                let _ = request.reply.send(Err(Error::Disconnected));
                shared.take_down(generation);
            }
        }
    }
}

fn read_loop(
    shared: &Shared,
    mut port: Current,
    mut codec: Box<dyn Codec>,
    mut target: Target,
    reprobe: &Reprobe,
) {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        if shared.stopping() {
            if let Some((port, _)) = port.take() {
                port.retire();
            }
            shared.set_port(None);
            return;
        }
        let Some((current, generation)) = port.clone() else {
            port = reconnect(shared, &mut target, reprobe);
            buffer.clear();
            continue;
        };
        let lost = if shared.down.swap(false, Ordering::AcqRel) {
            true
        } else {
            match current.read(&mut chunk) {
                Ok(0) => true,
                Ok(n) => {
                    buffer.extend_from_slice(&chunk[..n]);
                    let arrived = Instant::now();
                    while let Some(frame) = codec.split(&mut buffer) {
                        match codec.classify(&frame) {
                            Incoming::Reply(key) => {
                                shared.complete(key, generation, frame);
                            }
                            Incoming::Refused { key, action } => {
                                if !shared.complete(key, generation, frame)
                                    && let Some(action) = action
                                {
                                    shared.sink.refused(action);
                                }
                            }
                            Incoming::StreamLost(kind) => shared.sink.stream_lost(kind),
                            Incoming::Input { input, box_time } => {
                                shared.sink.input(input, arrived, box_time);
                            }
                            Incoming::Noise => {}
                        }
                    }
                    false
                }
                Err(error) => error.kind() != io::ErrorKind::TimedOut,
            }
        };
        if lost {
            current.retire();
            shared.set_port(None);
            port = None;
            // the port has to be closed before anything gets enumerated, otherwise Linux hands the box back under a
            // new name
            drop(current);
            shared.fail_pending();
            shared.sink.disconnected();
        }
    }
}

fn same_box(candidate: &Candidate, target: &Target) -> bool {
    match (&target.usb, &candidate.usb) {
        (
            Some(UsbId {
                serial: Some(serial),
                ..
            }),
            Some(found),
        ) => found.serial.as_ref() == Some(serial),
        (Some(_), None) => false,
        (Some(wanted), Some(found)) => {
            candidate.path == target.path && found.vid == wanted.vid && found.pid == wanted.pid
        }
        (None, _) => candidate.path == target.path,
    }
}

/// keeps trying until the same box answers again or the link is told to stop. the stop flag is checked
/// after every step that can block, so shutdown never waits for a whole try and never leaves a port open
fn reconnect(shared: &Shared, target: &mut Target, reprobe: &Reprobe) -> Current {
    let mut backoff = BACKOFF_START;
    loop {
        if shared.stopping() {
            return None;
        }
        let candidate = shared
            .system
            .ports()
            .into_iter()
            .find(|candidate| same_box(candidate, target));
        if let Some(candidate) = candidate {
            for &baud in &target.bauds.clone() {
                if shared.stopping() {
                    return None;
                }
                let Ok(port) = shared.system.open(&candidate.path, baud) else {
                    continue;
                };
                let probe = Probe::new(&*port, &shared.stop);
                let snapshot = reprobe(&probe).ok().filter(|snapshot| {
                    snapshot.info.family == target.family
                        && (target.id.is_none() || snapshot.info.id == target.id)
                });
                match snapshot {
                    Some(snapshot) if !shared.stopping() => {
                        let port: Arc<dyn Port> = Arc::from(port);
                        let generation = shared.generations.fetch_add(1, Ordering::AcqRel);
                        target.path = candidate.path;
                        shared.down.store(false, Ordering::Release);
                        shared.set_port(Some((port.clone(), generation)));
                        shared.sink.reconnected(snapshot);
                        return Some((port, generation));
                    }
                    _ => port.retire(),
                }
            }
        }
        let until = Instant::now() + backoff;
        while Instant::now() < until {
            if shared.stopping() {
                return None;
            }
            thread::sleep(Duration::from_millis(10));
        }
        backoff = (backoff * 2).min(BACKOFF_MAX);
    }
}

#[cfg(test)]
mod tests;
