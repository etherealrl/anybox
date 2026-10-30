use std::collections::VecDeque;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, RwLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use super::codec::{Codec, Frame, Incoming, ReplyKey};
use super::os::Read;
use super::probe::Probe;
use super::{Candidate, Port, PortUsb, System};
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
    pub(crate) usb: Option<PortUsb>,
    pub(crate) id: Option<StableId>,
    /// tried in order on every reconnect, so a box that got switched to another baud rate still comes back
    pub(crate) bauds: Vec<u32>,
}

/// asks a port that was just reopened what box it is, the same way detection did
pub(crate) type Reprobe = Box<dyn Fn(&Probe<'_>) -> Result<Snapshot> + Send + Sync>;

/// a request's answer, with the opening of the port it went out on
type Answered = Result<(Option<Frame>, u64)>;

enum Answer {
    /// the bytes are on the port, so a wait for silence can start
    Written,
    Done(Answered),
}

type Reply = Sender<Answer>;

struct Request {
    id: u64,
    bytes: Vec<u8>,
    expect: Option<ReplyKey>,
    /// the opening of the port it may only go out on
    on: Option<u64>,
    deadline: Instant,
    given_up: Arc<AtomicBool>,
    reply: Reply,
}

struct Pending {
    id: u64,
    key: ReplyKey,
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
    pub(crate) fn request(
        &self,
        bytes: Vec<u8>,
        expect: Option<ReplyKey>,
    ) -> Result<Option<Frame>> {
        self.request_until(bytes, expect, Instant::now() + WRITE_DEADLINE)
    }

    /// same as `request`, but it gives up at `until` when that comes before `WRITE_DEADLINE`
    pub(crate) fn request_until(
        &self,
        bytes: Vec<u8>,
        expect: Option<ReplyKey>,
        until: Instant,
    ) -> Result<Option<Frame>> {
        self.send(bytes, expect, None, until, None)
            .map(|(frame, _)| frame)
    }

    /// same as `request_until`, and it says which opening of the port it went out on. with `on` it only goes
    /// out on that one, and once the link has moved on it's `Disconnected` without being written
    pub(crate) fn request_on(
        &self,
        bytes: Vec<u8>,
        expect: Option<ReplyKey>,
        on: Option<u64>,
        until: Instant,
    ) -> Answered {
        self.send(bytes, expect, on, until, None)
    }

    /// for a box that only answers when it refuses: no reply within `window` of the bytes being written is
    /// silence, a frame of none, and not a missed reply. not being written by `until` is still `Disconnected`
    pub(crate) fn request_or_silence(
        &self,
        bytes: Vec<u8>,
        expect: ReplyKey,
        window: Duration,
        on: Option<u64>,
        until: Instant,
    ) -> Answered {
        self.send(bytes, Some(expect), on, until, Some(window))
    }

    /// what a missed reply does, for a backend whose own wait ran out while the box kept answering
    pub(crate) fn take_down(&self, generation: u64) {
        self.shared.take_down(generation);
    }

    fn send(
        &self,
        bytes: Vec<u8>,
        expect: Option<ReplyKey>,
        on: Option<u64>,
        until: Instant,
        silence: Option<Duration>,
    ) -> Answered {
        let (reply, answer) = mpsc::channel();
        let id = self.shared.next_id.fetch_add(1, Ordering::Relaxed);
        let mut deadline = until.min(Instant::now() + WRITE_DEADLINE);
        let given_up = Arc::new(AtomicBool::new(false));
        let request = Request {
            id,
            bytes,
            expect,
            on,
            deadline,
            given_up: given_up.clone(),
            reply,
        };
        match lock(&self.queue).as_ref() {
            Some(queue) if queue.send(request).is_ok() => {}
            _ => return Err(Error::Disconnected),
        }
        let mut written = false;
        loop {
            match answer.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok(Answer::Done(result)) => return result,
                Ok(Answer::Written) => {
                    written = true;
                    if let Some(window) = silence {
                        deadline = Instant::now() + window;
                    }
                }
                Err(_) => break,
            }
        }
        given_up.store(true, Ordering::Release);
        let missed = {
            let mut pending = lock(&self.shared.pending);
            let index = pending.iter().position(|p| p.id == id);
            index.and_then(|index| pending.remove(index))
        };
        match missed {
            Some(missed) if written && silence.is_some() => Ok((None, missed.generation)),
            // after a missed reply there's no way to tell which later reply belongs to which request, so the box
            // gets probed again
            Some(missed) => {
                self.shared.take_down(missed.generation);
                Err(Error::Disconnected)
            }
            // the reply came in right as the wait ran out, or losing the link already failed this request
            None => answer
                .try_iter()
                .find_map(|answer| match answer {
                    Answer::Done(result) => Some(result),
                    Answer::Written => None,
                })
                .unwrap_or(Err(Error::Disconnected)),
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
    fn complete(&self, key: ReplyKey, generation: u64, frame: Frame) -> bool {
        let mut pending = lock(&self.pending);
        let index = pending
            .iter()
            .position(|p| p.key == key && p.generation == generation);
        match index.and_then(|index| pending.remove(index)) {
            Some(waiting) => {
                let _ = waiting
                    .reply
                    .send(Answer::Done(Ok((Some(frame), waiting.generation))));
                true
            }
            None => false,
        }
    }

    fn fail_pending(&self) {
        for waiting in lock(&self.pending).drain(..) {
            let _ = waiting.reply.send(Answer::Done(Err(Error::Disconnected)));
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
            let _ = request.reply.send(Answer::Done(Err(Error::Disconnected)));
            continue;
        };
        if request.on.is_some_and(|on| on != generation) {
            let _ = request.reply.send(Answer::Done(Err(Error::Disconnected)));
            continue;
        }
        // this is registered before the write, so a reply that arrives before the write returns still finds
        // its request. the give up is checked again under the lock the caller takes to remove it, so a request
        // it already gave up on can never take a later reply
        if let Some(key) = request.expect {
            let mut pending = lock(&shared.pending);
            if request.given_up.load(Ordering::Acquire) {
                continue;
            }
            pending.push_back(Pending {
                id: request.id,
                key,
                generation,
                reply: request.reply.clone(),
            });
        }
        match port.write_all(&request.bytes) {
            Ok(()) => {
                let _ = request.reply.send(Answer::Written);
                if request.expect.is_none() {
                    let _ = request.reply.send(Answer::Done(Ok((None, generation))));
                }
            }
            Err(_) => {
                lock(&shared.pending).retain(|pending| pending.id != request.id);
                let _ = request.reply.send(Answer::Done(Err(Error::Disconnected)));
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
                    // taken before anything else, it's the closest anybox gets to when the bytes came in
                    let read = Read::now();
                    let arrived = read.at;
                    buffer.extend_from_slice(&chunk[..n]);
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
                                shared.sink.input(input, arrived, box_time, Some(read));
                            }
                            Incoming::Inputs(inputs) => {
                                for input in inputs {
                                    shared.sink.input(input, arrived, None, Some(read));
                                }
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
    if target.id.is_none() {
        return candidate.path == target.path && candidate.usb == target.usb;
    }
    match (&target.usb, &candidate.usb) {
        (
            Some(PortUsb {
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
