use std::any::Any;
use std::sync::mpsc;
use std::sync::{Arc, Barrier, Mutex};
use std::time::{Duration, Instant};

use crate::backend::{
    Backend, ChordKind, Command, Input, InputKind, InputKinds, LockSlot, Sink, Typing, UNLOCKED,
    Usage,
};
use crate::capability::{Action, Button, Capabilities, Capability, Family, Info, Snapshot};
use crate::click::{ClickTiming, Holds};
use crate::device::{Device, Start};
use crate::error::{Error, Result};
use crate::event::{BoxTime, Clock};
use crate::extension::{Extension, ExtensionReply, Interpolation};
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
    /// the cleanup calls in the order they came, `release_all`, `restore_all` and `unlock_all`
    cleanup: Vec<&'static str>,
    fail_next: Option<Error>,
    /// the next stream switch fails for these kinds with this error
    fail_streams: Option<(InputKinds, Error)>,
    /// runs when that failing switch is about to return
    before_failing: Option<Box<dyn FnOnce() + Send>>,
    /// the next restart of a stream fails with this error
    fail_restart: Option<Error>,
    shutdowns: usize,
    fail_releases: bool,
    pause_release: Option<Arc<Barrier>>,
    snapshot: Option<Snapshot>,
    /// every piece of text the fake typed
    pieces: Vec<Vec<u8>>,
    /// runs before each piece with its number, and an error it gives fails that piece
    on_piece: Option<Box<dyn FnMut(usize) -> Option<Error> + Send>>,
    /// every extension call that reached the fake
    extensions: Vec<Extension>,
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
    open_as(Family::Medius, capabilities, click_timing)
}

/// the fake, saying it's another family, for the calls only that family has
pub(crate) fn open_as(
    family: Family,
    capabilities: Capabilities,
    click_timing: ClickTiming,
) -> (Device, Handle) {
    let snapshot = Snapshot::new(Info::new(family), capabilities);
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

    pub(crate) fn cleanup(&self) -> Vec<&'static str> {
        self.state.lock().unwrap().cleanup.clone()
    }

    pub(crate) fn unlocked_all(&self) -> Vec<Vec<LockSlot>> {
        self.state.lock().unwrap().unlocked_all.clone()
    }

    pub(crate) fn extensions(&self) -> Vec<Extension> {
        self.state.lock().unwrap().extensions.clone()
    }

    pub(crate) fn pieces(&self) -> Vec<Vec<u8>> {
        self.state.lock().unwrap().pieces.clone()
    }

    pub(crate) fn on_piece(&self, on_piece: impl FnMut(usize) -> Option<Error> + Send + 'static) {
        self.state.lock().unwrap().on_piece = Some(Box::new(on_piece));
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

    /// the next failing stream switch first reports this refusal on its own and waits until the core published it,
    /// the order a MAKCU's reader can give
    pub(crate) fn refuse_before_the_switch_returns(&self, action: Action) {
        let sink = self.sink.clone();
        self.state.lock().unwrap().before_failing = Some(Box::new(move || {
            sink.refused(action);
            let (done, published) = mpsc::channel();
            let _ = sink.0.send(crate::backend::Signal::Barrier(done));
            let _ = published.recv();
        }));
    }

    pub(crate) fn session_reset(&self) {
        self.state.lock().unwrap().streaming = None;
        self.sink.session_reset();
    }
}

impl Backend for Fake {
    fn send(&self, command: Command) -> Result<()> {
        let release = matches!(command.holds(), Some((_, false)))
            || matches!(
                command,
                Command::Lock { keep: UNLOCKED, .. }
                    | Command::KeyForce { force: false, .. }
                    | Command::KeyChord {
                        kind: ChordKind::Release,
                        ..
                    }
            );
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
            let before = state.before_failing.take();
            drop(state);
            if let Some(before) = before {
                before();
            }
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

    fn type_piece(&self, piece: &[u8], typing: &mut Typing) -> Result<()> {
        let mut on_piece = self.0.lock().unwrap().on_piece.take();
        let index = self.0.lock().unwrap().pieces.len();
        let failed = on_piece.as_mut().and_then(|on_piece| on_piece(index));
        let mut state = self.0.lock().unwrap();
        if state.on_piece.is_none() {
            state.on_piece = on_piece;
        }
        // a hook's error stands for a box that failed with the piece already sent
        if let Some(error) = failed {
            typing.sent = true;
            return Err(error);
        }
        let snapshot = state.snapshot.as_ref().unwrap();
        if !snapshot.capabilities.get(Action::TypeText).is_supported() {
            return Err(Error::Unsupported {
                action: Action::TypeText,
                family: snapshot.info.family,
                reason: "the box changed during this call, check its snapshot again".into(),
            });
        }
        state.pieces.push(piece.to_vec());
        typing.sent = true;
        Ok(())
    }

    fn unlock_all(&self, slots: &[LockSlot]) -> Result<()> {
        let mut slots = slots.to_vec();
        slots.sort_by_key(|slot| format!("{slot:?}"));
        let mut state = self.0.lock().unwrap();
        state.unlocked_all.push(slots);
        state.cleanup.push("unlock_all");
        Ok(())
    }

    fn release_all(&self, held: &[Usage]) -> Result<()> {
        let mut held = held.to_vec();
        held.sort_by_key(|usage| format!("{usage:?}"));
        let mut state = self.0.lock().unwrap();
        state.released_all.push(held);
        state.cleanup.push("release_all");
        Ok(())
    }

    fn restore_all(&self, keys: &[Key]) -> Result<()> {
        self.0.lock().unwrap().cleanup.push("restore_all");
        let mut first = Ok(());
        for &key in keys {
            let restored = self.send(Command::KeyForce { key, force: false });
            if first.is_ok() {
                first = restored;
            }
        }
        first
    }

    fn shutdown(&self, _deadline: Duration) {
        self.0.lock().unwrap().shutdowns += 1;
    }

    fn extension(&self, request: Extension) -> Result<ExtensionReply> {
        self.0.lock().unwrap().extensions.push(request);
        Ok(match request {
            Extension::MakcuInterpolation => ExtensionReply::Interpolation(Interpolation::Auto),
            Extension::MakcuMoving => ExtensionReply::Moving(false),
            Extension::MakcuAutorelease => ExtensionReply::Autorelease(None),
            Extension::FerrumCatchXy { .. } => ExtensionReply::Xy(0, 0),
            _ => ExtensionReply::Done,
        })
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
