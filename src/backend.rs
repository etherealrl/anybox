use std::any::Any;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use crate::capability::{Action, Axis, Button, Snapshot};
use crate::error::{Error, Result};
use crate::event::{BoxTime, DropSource};
use crate::extension::{Extension, ExtensionReply};
use crate::key::{Key, MediaKey};

/// what the core asks a backend to do on the box. it never sends one the box's capabilities don't list
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Move(i16, i16),
    MoveNow(i16, i16),
    Press(Button),
    Release(Button),
    Click {
        button: Button,
        hold: Option<Duration>,
    },
    Wheel(i16),
    Pan(i16),
    DiscardPendingMoves,
    KeyPress(Key),
    KeyRelease(Key),
    KeyTap {
        key: Key,
        hold: Option<Duration>,
    },
    MediaPress(MediaKey),
    MediaRelease(MediaKey),
    MediaTap {
        key: MediaKey,
        hold: Option<Duration>,
    },
    /// how much of the physical input on a slot goes through, in percent. 0 is locked, 100 is free and anything
    /// else is a scale
    Lock {
        slot: LockSlot,
        keep: i16,
    },
    KeyChord {
        kind: ChordKind,
        keys: Keys,
    },
    /// `force` makes the PC see the key up whatever the finger does, otherwise the PC gets the finger's state back
    KeyForce {
        key: Key,
        force: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChordKind {
    Press,
    Release,
    Tap,
}

/// the keys of one chord, at most `MAX_CHORD`, kept in the command so it stays `Copy`
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Keys {
    keys: [Key; MAX_CHORD],
    len: u8,
}

/// anybox's own limit for every box, MAKCU's chord command takes 1 to 14
pub(crate) const MAX_CHORD: usize = 14;

impl Keys {
    /// none when there are more than `MAX_CHORD`
    pub(crate) fn new(keys: &[Key]) -> Option<Self> {
        let mut all = [Key(0); MAX_CHORD];
        all.get_mut(..keys.len())?.copy_from_slice(keys);
        Some(Self {
            keys: all,
            len: u8::try_from(keys.len()).ok()?,
        })
    }

    pub(crate) fn as_slice(&self) -> &[Key] {
        &self.keys[..usize::from(self.len)]
    }
}

#[derive(Debug, Default)]
pub(crate) struct Typing {
    /// the link generation the text's first request went out on. every later one may only go out on it, so after
    /// a reconnect nothing more of the text is typed
    pub(crate) generation: Option<u64>,
    /// set by the backend once any of the text went to the box, from then on the box may have typed some of it
    pub(crate) sent: bool,
}

pub(crate) const UNLOCKED: i16 = 100;
pub(crate) const LOCKED: i16 = 0;

/// what a lock sits on. an axis gets one per direction, so unlocking one way doesn't touch the other
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum LockSlot {
    Button(Button),
    Key(Key),
    Axis(Axis, Sign),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Sign {
    Positive,
    Negative,
}

impl LockSlot {
    pub(crate) fn action(self, keep: i16) -> Action {
        match self {
            Self::Button(_) => Action::LockButton,
            Self::Key(_) => Action::LockKey,
            Self::Axis(..) if keep == LOCKED || keep == UNLOCKED => Action::LockAxis,
            Self::Axis(..) => Action::ScaleAxis,
        }
    }
}

/// anything that can be held down. the kind is part of it, so a button and a key with the same number never
/// get mixed up
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum Usage {
    Button(Button),
    Key(Key),
    Media(MediaKey),
}

impl Usage {
    pub(crate) fn press(self) -> Command {
        match self {
            Self::Button(button) => Command::Press(button),
            Self::Key(key) => Command::KeyPress(key),
            Self::Media(key) => Command::MediaPress(key),
        }
    }

    pub(crate) fn release(self) -> Command {
        match self {
            Self::Button(button) => Command::Release(button),
            Self::Key(key) => Command::KeyRelease(key),
            Self::Media(key) => Command::MediaRelease(key),
        }
    }

    /// the box's own click or tap of this button or key
    pub(crate) fn tap(self, hold: Option<Duration>) -> Command {
        match self {
            Self::Button(button) => Command::Click { button, hold },
            Self::Key(key) => Command::KeyTap { key, hold },
            Self::Media(key) => Command::MediaTap { key, hold },
        }
    }
}

impl Command {
    /// the action the capability check looks up for this command
    pub(crate) fn action(self) -> Action {
        match self {
            Self::Move(..) => Action::Move,
            Self::MoveNow(..) => Action::MoveNow,
            Self::Press(_) => Action::Press,
            Self::Release(_) => Action::Release,
            Self::Click { .. } => Action::Click,
            Self::Wheel(_) => Action::Wheel,
            Self::Pan(_) => Action::Pan,
            Self::DiscardPendingMoves => Action::DiscardPendingMoves,
            Self::KeyPress(_) => Action::KeyPress,
            Self::KeyRelease(_) => Action::KeyRelease,
            Self::KeyTap { .. } => Action::KeyTap,
            Self::MediaPress(_) => Action::MediaPress,
            Self::MediaRelease(_) => Action::MediaRelease,
            Self::MediaTap { .. } => Action::MediaTap,
            Self::Lock { slot, keep } => slot.action(keep),
            Self::KeyChord { .. } => Action::KeyChord,
            Self::KeyForce { .. } => Action::ForceReleaseKey,
        }
    }

    /// what this command holds down or lets go of, if anything
    pub(crate) fn holds(self) -> Option<(Usage, bool)> {
        match self {
            Self::Press(button) => Some((Usage::Button(button), true)),
            Self::Release(button) => Some((Usage::Button(button), false)),
            Self::KeyPress(key) => Some((Usage::Key(key), true)),
            Self::KeyRelease(key) => Some((Usage::Key(key), false)),
            Self::MediaPress(key) => Some((Usage::Media(key), true)),
            Self::MediaRelease(key) => Some((Usage::Media(key), false)),
            Self::Move(..)
            | Self::MoveNow(..)
            | Self::Click { .. }
            | Self::Wheel(_)
            | Self::Pan(_)
            | Self::DiscardPendingMoves
            | Self::KeyTap { .. }
            | Self::MediaTap { .. }
            | Self::Lock { .. }
            | Self::KeyChord { .. }
            | Self::KeyForce { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct InputKinds {
    pub(crate) buttons: bool,
    pub(crate) motion: bool,
    pub(crate) keys: bool,
    pub(crate) media: bool,
}

/// an input stream the box can switch off by itself
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InputKind {
    Buttons,
    Motion,
    Keys,
    Media,
}

impl InputKind {
    pub(crate) fn kinds(self) -> InputKinds {
        let mut kinds = InputKinds::default();
        match self {
            Self::Buttons => kinds.buttons = true,
            Self::Motion => kinds.motion = true,
            Self::Keys => kinds.keys = true,
            Self::Media => kinds.media = true,
        }
        kinds
    }
}

/// the kinds that didn't switch the way they were asked to, and why. `Disconnected` means the reconnect asks again
#[derive(Debug)]
pub(crate) struct StreamFailure {
    pub(crate) failed: InputKinds,
    pub(crate) error: crate::error::Error,
}

/// one family's connection to one box. every method gives up and returns `Disconnected` after `WRITE_DEADLINE`
/// per request it makes (one per button or key for `release_all`, a switch and its confirmation per stream for
/// `set_streams`), because the core calls them while it holds the output lock and drop waits on that lock
pub(crate) trait Backend: Send + Sync + 'static {
    /// has to check the command against what the box can do right now, because a reconnect can change that after the
    /// core already said yes
    fn send(&self, command: Command) -> Result<()>;
    /// switches each kind on or off to match `kinds`, and leaves a kind that's already right alone
    fn set_streams(&self, kinds: InputKinds) -> std::result::Result<(), StreamFailure>;
    /// switches back on the one stream the box switched off, the others are left alone
    fn restart_stream(&self, kind: InputKind) -> Result<()>;
    fn release_all(&self, held: &[Usage]) -> Result<()>;
    /// types one piece of printable ASCII and returns once the box is done with it, all of it within
    /// `WRITE_DEADLINE`. a box that doesn't type answers `Unsupported`, the core never gets that far on one
    fn type_piece(&self, piece: &[u8], typing: &mut Typing) -> Result<()>;
    /// unlocks each slot, and once one comes back `Disconnected` it doesn't wait on the rest. Medius still queues
    /// them, MAKCU leaves them to its last check at shutdown, which only gets to them if the box answers in time
    fn unlock_all(&self, slots: &[LockSlot]) -> Result<()>;
    /// gives back each key `force_release_key` held up, on drop and close. every one goes out even after one fails,
    /// a box that queues its writes overrides this to stop waiting once one ran out of time
    fn restore_all(&self, keys: &[Key]) -> Result<()> {
        let mut first = Ok(());
        for &key in keys {
            let restored = self.send(Command::KeyForce { key, force: false });
            if first.is_ok() {
                first = restored;
            }
        }
        first
    }
    fn shutdown(&self, deadline: Duration);
    /// lets explicit close report a failed final write, while drop can still give it a best effort
    fn close(&self, deadline: Duration) -> Result<()> {
        self.shutdown(deadline);
        Ok(())
    }
    /// one call only this family has. the core checks the capability first, so a backend without extensions
    /// never gets here, this answer is for one that does by mistake
    fn extension(&self, request: Extension) -> Result<ExtensionReply> {
        Err(Error::Unsupported {
            action: request.action(),
            family: crate::extension::family(request),
            reason: "this backend has no calls of its own".into(),
        })
    }
    fn as_any(&self) -> &dyn Any;
}

/// how long a send may hold the output lock on a port that stopped answering
pub(crate) const WRITE_DEADLINE: Duration = Duration::from_millis(500);

#[derive(Debug)]
pub(crate) enum Input {
    Button {
        button: Button,
        pressed: bool,
    },
    Motion {
        dx: i32,
        dy: i32,
        wheel: i32,
        pan: i32,
    },
    Key {
        key: Key,
        pressed: bool,
    },
    Media {
        key: MediaKey,
        pressed: bool,
    },
}

#[derive(Debug)]
pub(crate) enum Signal {
    Input {
        input: Input,
        arrived: Instant,
        box_time: Option<BoxTime>,
        /// none where the box's own library reads the port, so anybox never sees the moment
        read: Option<crate::transport::os::Read>,
    },
    /// with a kind when only that kind's stream lost them, like one Medius subscription overflowing
    Dropped(u64, DropSource, Option<InputKind>),
    /// the box switched one input stream off by itself and threw away what it had queued
    StreamLost(InputKind),
    /// the box refused a command after its call already returned
    Refused(Action),
    SessionReset,
    Disconnected,
    Reconnected(Box<Snapshot>),
    /// still the same box, just with other details (a new key limit, or a device that came or went on a box that
    /// doesn't lose anything over it), so nothing gets let go. only the streams follow what it can do now
    Changed(Box<Snapshot>),
    /// lets a test wait until everything sent before it got handled
    #[cfg(test)]
    Barrier(std::sync::mpsc::Sender<()>),
    /// events that only go out to the receivers, with none of what the real ones change in the device, so a
    /// test can show every kind without a disconnect actually disconnecting
    #[cfg(feature = "mock")]
    Examples(Vec<crate::event::Event>),
    Stop,
}

/// how a backend reports to the core. it only queues the report, so a backend thread never waits on the output
/// lock
#[derive(Clone, Debug)]
pub(crate) struct Sink(pub(crate) Sender<Signal>);

impl Sink {
    pub(crate) fn input(
        &self,
        input: Input,
        arrived: Instant,
        box_time: Option<BoxTime>,
        read: Option<crate::transport::os::Read>,
    ) {
        self.send(Signal::Input {
            input,
            arrived,
            box_time,
            read,
        });
    }

    pub(crate) fn dropped(&self, count: u64, source: DropSource) {
        self.send(Signal::Dropped(count, source, None));
    }

    /// events one kind's stream lost on the PC side, so only that kind's receivers hear about it
    pub(crate) fn lost(&self, count: u64, kind: InputKind) {
        self.send(Signal::Dropped(count, DropSource::Host, Some(kind)));
    }

    pub(crate) fn stream_lost(&self, kind: InputKind) {
        self.send(Signal::StreamLost(kind));
    }

    pub(crate) fn refused(&self, action: Action) {
        self.send(Signal::Refused(action));
    }

    pub(crate) fn session_reset(&self) {
        self.send(Signal::SessionReset);
    }

    pub(crate) fn disconnected(&self) {
        self.send(Signal::Disconnected);
    }

    pub(crate) fn reconnected(&self, snapshot: Snapshot) {
        self.send(Signal::Reconnected(Box::new(snapshot)));
    }

    pub(crate) fn changed(&self, snapshot: Snapshot) {
        self.send(Signal::Changed(Box::new(snapshot)));
    }

    fn send(&self, signal: Signal) {
        // this only fails once the core is gone, and then there's nobody left to tell
        let _ = self.0.send(signal);
    }
}
