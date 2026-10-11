use std::any::Any;
use std::sync::mpsc;
use std::sync::{Arc, Barrier, Mutex};
use std::time::{Duration, Instant};

use crate::backend::{
    Backend, Command, Input, InputKind, InputKinds, LockSlot, Sink, UNLOCKED, Usage,
};
use crate::capability::{Action, Button, Capabilities, Capability, Family, Info, Snapshot};
use crate::click::{ClickTiming, Holds};
use crate::device::{Device, Start};
use crate::error::{Error, Result};
use crate::event::{BoxTime, Clock};
use crate::key::{Key, MediaKey};

#[derive(Default)]
struct State {
    log: Vec<(Instant, Command)>,
    streams: Vec<InputKinds>,
    /// what the fake streams now, so like the real backends it only records a change, and a reconnect forgets it
    streaming: Option<InputKinds>,
    restarted: Vec<InputKind>,
    released_all: Vec<Vec<Usage>>,
    unlocked_all: Vec<Vec<LockSlot>>,
    fail_next: Option<Error>,
    /// the next stream switch fails for these kinds with this error
    fail_streams: Option<(InputKinds, Error)>,
    /// the next restart of a stream fails with this error
    fail_restart: Option<Error>,
    shutdowns: usize,
    fail_releases: bool,
    pause_release: Option<Arc<Barrier>>,
    snapshot: Option<Snapshot>,
}

/// a box that does whatever its snapshot says and writes down everything it was asked
struct Fake(Arc<Mutex<State>>);

/// the test's side of a fake box
#[derive(Clone)]
pub(crate) struct Handle {
    state: Arc<Mutex<State>>,
    sink: Sink,
}

pub(crate) fn mouse() -> Capabilities {
    Capabilities::new(vec![
        Button::Left,
        Button::Right,
        Button::Middle,
        Button::Side1,
        Button::Side2,
    ])
    .set(Action::Move, Capability::native())
    .set(Action::Press, Capability::native())
    .set(Action::Release, Capability::native())
    .set(Action::Wheel, Capability::native())
}

pub(crate) fn open(capabilities: Capabilities, click_timing: ClickTiming) -> (Device, Handle) {
    let snapshot = Snapshot::new(Info::new(Family::Medius), capabilities);
    let state = Arc::new(Mutex::new(State {
        snapshot: Some(snapshot.clone()),
        ..State::default()
    }));
    let (signals, incoming) = mpsc::channel();
    let handle = Handle {
        state: state.clone(),
        sink: Sink(signals.clone()),
    };
    let device = Device::start(Start {
        backend: Box::new(Fake(state)),
        snapshot,
        signals,
        incoming,
        click_timing,
        holds: Holds::seeded(42),
        port: "fake".into(),
    })
    .unwrap();
    (device, handle)
}

impl Handle {
    pub(crate) fn log(&self) -> Vec<Command> {
        self.state
            .lock()
            .unwrap()
            .log
            .iter()
            .map(|(_, c)| *c)
            .collect()
    }

    /// waits until the box was asked for this, so a test doesn't have to guess with a sleep
    pub(crate) fn wait_for(&self, command: Command) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !self.log().contains(&command) {
            assert!(Instant::now() < deadline, "the box never got {command:?}");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    pub(crate) fn timed_log(&self) -> Vec<(Instant, Command)> {
        self.state.lock().unwrap().log.clone()
    }

    pub(crate) fn streams(&self) -> Vec<InputKinds> {
        self.state.lock().unwrap().streams.clone()
    }

    pub(crate) fn restarted(&self) -> Vec<InputKind> {
        self.state.lock().unwrap().restarted.clone()
    }

    pub(crate) fn released_all(&self) -> Vec<Vec<Usage>> {
        self.state.lock().unwrap().released_all.clone()
    }

    pub(crate) fn unlocked_all(&self) -> Vec<Vec<LockSlot>> {
        self.state.lock().unwrap().unlocked_all.clone()
    }

    pub(crate) fn fail_next(&self, error: Error) {
        self.state.lock().unwrap().fail_next = Some(error);
    }

    pub(crate) fn fail_restart(&self, error: Error) {
        self.state.lock().unwrap().fail_restart = Some(error);
    }

    pub(crate) fn shutdowns(&self) -> usize {
        self.state.lock().unwrap().shutdowns
    }

    /// a button whose bytes were read at `read`, which a real link takes before anything else
    pub(crate) fn button_read(&self, button: Button, read: crate::transport::os::Read) {
        self.sink.input(
            Input::Button {
                button,
                pressed: true,
            },
            Instant::now(),
            None,
            Some(read),
        );
    }

    pub(crate) fn fail_streams(&self, kinds: InputKinds, error: Error) {
        self.state.lock().unwrap().fail_streams = Some((kinds, error));
    }

    pub(crate) fn fail_releases(&self, fail: bool) {
        self.state.lock().unwrap().fail_releases = fail;
    }

    /// the next release waits at the barrier twice, once when it starts and once right before it goes out, so the
    /// test can step in between
    pub(crate) fn pause_next_release(&self) -> Arc<Barrier> {
        let barrier = Arc::new(Barrier::new(2));
        self.state.lock().unwrap().pause_release = Some(barrier.clone());
        barrier
    }

    /// the box changes without telling the core, like a reconnect the core hasn't heard of yet
    pub(crate) fn change_quietly(&self, capabilities: Capabilities) {
        let snapshot = Snapshot::new(Info::new(Family::Medius), capabilities);
        self.state.lock().unwrap().snapshot = Some(snapshot);
    }

    pub(crate) fn button(&self, button: Button, pressed: bool) {
        self.sink.input(
            Input::Button { button, pressed },
            Instant::now(),
            Some(BoxTime {
                micros: 1000,
                clock: Clock::MediusHostChip,
            }),
            None,
        );
    }

    pub(crate) fn key(&self, key: Key, pressed: bool) {
        self.sink
            .input(Input::Key { key, pressed }, Instant::now(), None, None);
    }

    pub(crate) fn media(&self, key: MediaKey, pressed: bool) {
        self.sink
            .input(Input::Media { key, pressed }, Instant::now(), None, None);
    }

    pub(crate) fn motion(&self, dx: i32) {
        self.sink.input(
            Input::Motion {
                dx,
                dy: 0,
                wheel: 0,
                pan: 0,
            },
            Instant::now(),
            None,
            None,
        );
    }

    pub(crate) fn dropped(&self, count: u64) {
        self.sink.dropped(count, crate::event::DropSource::Device);
    }

    pub(crate) fn disconnect(&self) {
        self.sink.disconnected();
    }

    /// the same device with different details, like a new key limit
    pub(crate) fn changed(&self, capabilities: Capabilities) {
        let snapshot = Snapshot::new(Info::new(Family::Medius), capabilities);
        self.state.lock().unwrap().snapshot = Some(snapshot.clone());
        self.sink.changed(snapshot);
    }

    /// the box is back, maybe with a different mouse behind it
    pub(crate) fn reconnect(&self, capabilities: Capabilities) {
        let snapshot = Snapshot::new(Info::new(Family::Medius), capabilities);
        self.state.lock().unwrap().snapshot = Some(snapshot.clone());
        self.state.lock().unwrap().streaming = None;
        self.sink.reconnected(snapshot);
    }

    pub(crate) fn stream_lost(&self, kind: InputKind) {
        self.sink.stream_lost(kind);
    }

    pub(crate) fn refused(&self, action: Action) {
        self.sink.refused(action);
    }

    pub(crate) fn session_reset(&self) {
        self.state.lock().unwrap().streaming = None;
        self.sink.session_reset();
    }
}

impl Backend for Fake {
    fn send(&self, command: Command) -> Result<()> {
        let release = matches!(command.holds(), Some((_, false)))
            || matches!(command, Command::Lock { keep: UNLOCKED, .. });
        if release {
            let pause = self.0.lock().unwrap().pause_release.take();
            if let Some(barrier) = pause {
                barrier.wait();
                barrier.wait();
            }
        }
        let mut state = self.0.lock().unwrap();
        if let Some(error) = state.fail_next.take() {
            return Err(error);
        }
        if state.fail_releases && release {
            return Err(Error::Disconnected);
        }
        let snapshot = state.snapshot.as_ref().unwrap();
        // a release always goes through, same as on the real boxes
        if !release && !snapshot.capabilities.get(command.action()).is_supported() {
            return Err(Error::Unsupported {
                action: command.action(),
                family: snapshot.info.family,
                reason: "the box changed during this call, check its snapshot again".into(),
            });
        }
        state.log.push((Instant::now(), command));
        Ok(())
    }

    fn set_streams(
        &self,
        kinds: InputKinds,
    ) -> std::result::Result<(), crate::backend::StreamFailure> {
        let mut state = self.0.lock().unwrap();
        if let Some((failed, error)) = state.fail_streams.take() {
            return Err(crate::backend::StreamFailure { failed, error });
        }
        if state.streaming != Some(kinds) {
            state.streaming = Some(kinds);
            state.streams.push(kinds);
        }
        Ok(())
    }

    fn restart_stream(&self, kind: InputKind) -> Result<()> {
        let mut state = self.0.lock().unwrap();
        state.restarted.push(kind);
        state.fail_restart.take().map_or(Ok(()), Err)
    }

    fn unlock_all(&self, slots: &[LockSlot]) -> Result<()> {
        let mut slots = slots.to_vec();
        slots.sort_by_key(|slot| format!("{slot:?}"));
        self.0.lock().unwrap().unlocked_all.push(slots);
        Ok(())
    }

    fn release_all(&self, held: &[Usage]) -> Result<()> {
        let mut held = held.to_vec();
        held.sort_by_key(|usage| format!("{usage:?}"));
        self.0.lock().unwrap().released_all.push(held);
        Ok(())
    }

    fn shutdown(&self, _deadline: Duration) {
        self.0.lock().unwrap().shutdowns += 1;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
