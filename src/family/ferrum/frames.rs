use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::backend::Input;
use crate::capability::Button;
use crate::key::Key;
use crate::transport::codec::{self, Codec, Frame, Incoming};

pub(crate) const PROMPT: &[u8] = b">>> ";
pub(crate) const END: &[u8] = b"\r\n";
/// nothing the box sends comes close to this, so a longer line is garbage
const MAX_LINE: usize = 1024;
/// `km.`, the state byte and the line end
const BUTTONS_LEN: usize = 6;

pub(crate) const SOFTWARE_VERSION: &[u8] = b"kmbox: Ferrum";
/// a real KMBox B+ answers the exact same thing, the docs say it's there for old KMBox software
pub(crate) const LEGACY_VERSION: &[u8] = b"kmbox: 2.0.0 Aug 31 2020 21:49:51";
/// the line end in front closes whatever an earlier probe left half written in the box's parser
pub(crate) const VERSION_QUERY: &[u8] = b"\r\nkm.version()\r\n";
/// every callback the docs list besides the button one. anybox doesn't use the controller ones, but they survive
/// a port reopen too, so they can show up anyway
const CALLBACKS: [&[u8]; 4] = [
    b"Axes(",
    b"Keys(",
    b"ControllerAxes(",
    b"ControllerButtons(",
];

/// the prefixes of every command anybox sends, so a line starting with one is an echo
fn echoed(line: &[u8]) -> bool {
    [&b"km."[..], b"device.", b"mouse_proxy."]
        .iter()
        .any(|prefix| line.starts_with(prefix))
}

/// what a reply waits under, the command as the box echoes it back. the box has no ids, so this is the only way to
/// tell which command a reply belongs to when one of them lost its prompt
pub(crate) fn key(command: &[u8]) -> codec::ReplyKey {
    // FNV-1a, a collision only matters between two commands in flight at the same time
    command.iter().fold(0x811c_9dc5, |hash: u32, &byte| {
        (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193)
    })
}

/// the buttons and keys the last callbacks showed, because each callback is the whole state and the events are
/// what changed since
#[derive(Default)]
pub(crate) struct Seen {
    buttons: u8,
    keys: Vec<u8>,
}

impl Seen {
    pub(crate) fn forget_buttons(&mut self) {
        self.buttons = 0;
    }

    pub(crate) fn forget_keys(&mut self) {
        self.keys.clear();
    }
}

pub(crate) fn lock(seen: &Mutex<Seen>) -> MutexGuard<'_, Seen> {
    seen.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(crate) struct FerrumFrames {
    /// the Software API echoes every command and ends it with a prompt, the legacy port does neither
    echo: bool,
    /// the echo and result of the command that's still waiting for its prompt
    reply: Vec<u8>,
    seen: Arc<Mutex<Seen>>,
}

impl FerrumFrames {
    pub(crate) fn new(echo: bool, seen: Arc<Mutex<Seen>>) -> Self {
        Self {
            echo,
            reply: Vec::new(),
            seen,
        }
    }

    /// for asking a bare port, where nobody cares about the callbacks
    pub(crate) fn probe(echo: bool) -> Self {
        Self::new(echo, Arc::default())
    }

    /// with echo on a callback always ends in a prompt, so it waits for it and takes it along. the legacy port
    /// may or may not send one, so there a prompt that comes later is just noise
    fn callback(&self, buffer: &mut Vec<u8>, len: usize) -> Option<Frame> {
        let rest = &buffer[len..];
        if rest.starts_with(PROMPT) {
            return Some(buffer.drain(..len + PROMPT.len()).collect());
        }
        if self.echo && PROMPT.starts_with(rest) {
            return None;
        }
        Some(buffer.drain(..len).collect())
    }

    fn buttons(&self, state: u8) -> Incoming {
        let state = state & 0x1F;
        let mut seen = lock(&self.seen);
        let changed = state ^ seen.buttons;
        seen.buttons = state;
        // releases first, so a swap in one report reads as one going up and then the other going down
        let edges = [false, true].into_iter().flat_map(|pressed| {
            (0..5u8)
                .filter(move |bit| {
                    changed & (1 << bit) != 0 && (state & (1 << bit) != 0) == pressed
                })
                .map(move |bit| Input::Button {
                    button: BUTTONS[usize::from(bit)],
                    pressed,
                })
        });
        Incoming::Inputs(edges.collect())
    }

    fn keys(&self, held: Vec<u8>) -> Incoming {
        let held: Vec<u8> = held
            .into_iter()
            .filter(|&key| Key(key).is_valid())
            .collect();
        let mut seen = lock(&self.seen);
        let released = seen.keys.iter().filter(|key| !held.contains(key));
        let pressed = held.iter().filter(|key| !seen.keys.contains(key));
        let edges = released
            .map(|&key| Input::Key {
                key: Key(key),
                pressed: false,
            })
            .chain(pressed.map(|&key| Input::Key {
                key: Key(key),
                pressed: true,
            }))
            .collect();
        seen.keys = held;
        Incoming::Inputs(edges)
    }
}

/// the callback's bits in order, bit 3 is the rear side button and bit 4 the front one
const BUTTONS: [Button; 5] = [
    Button::Left,
    Button::Right,
    Button::Middle,
    Button::Side1,
    Button::Side2,
];

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// the numbers between the brackets of `Axes(..)` or `Keys(..)`, none when it's malformed
fn arguments<T: std::str::FromStr>(frame: &[u8], name: &[u8]) -> Option<Vec<T>> {
    let inside = frame.strip_prefix(name)?;
    let close = inside.iter().position(|&byte| byte == b')')?;
    let inside = std::str::from_utf8(&inside[..close]).ok()?;
    if inside.trim().is_empty() {
        return Some(Vec::new());
    }
    inside
        .split(',')
        .map(|number| number.trim().parse().ok())
        .collect()
}

/// the result line of a reply, none when the command had no result. a command the box doesn't know still gets
/// its echo and prompt, just no result, so this is also how a read finds out the box knows the command
pub(crate) fn result(frame: &[u8], echo: bool) -> Option<&[u8]> {
    let start = if echo {
        find(frame, END)? + END.len()
    } else {
        0
    };
    let rest = &frame[start..];
    find(rest, END).map(|end| &rest[..end])
}

/// which API answered `km.version()`, from one line of the reply
pub(crate) fn version(frame: &[u8]) -> Option<super::FerrumApi> {
    match frame.strip_suffix(END).unwrap_or(frame) {
        SOFTWARE_VERSION => Some(super::FerrumApi::Software),
        LEGACY_VERSION => Some(super::FerrumApi::Legacy),
        _ => None,
    }
}

impl Codec for FerrumFrames {
    fn split(&mut self, buffer: &mut Vec<u8>) -> Option<Frame> {
        loop {
            // the button callback's state byte can be 0x0A or 0x0D, so it gets matched by its length before
            // anything splits on line ends
            if buffer.starts_with(b"km.") {
                if buffer.len() < BUTTONS_LEN {
                    return None;
                }
                if &buffer[4..BUTTONS_LEN] == END {
                    return self.callback(buffer, BUTTONS_LEN);
                }
            }
            if buffer.starts_with(PROMPT) {
                buffer.drain(..PROMPT.len());
                let mut frame = std::mem::take(&mut self.reply);
                frame.extend_from_slice(PROMPT);
                return Some(frame);
            }
            if PROMPT.starts_with(buffer) {
                return None;
            }
            let Some(end) = find(buffer, END) else {
                if buffer.len() > MAX_LINE {
                    buffer.clear();
                    self.reply.clear();
                }
                return None;
            };
            let len = end + END.len();
            if CALLBACKS.iter().any(|name| buffer.starts_with(name)) {
                return self.callback(buffer, len);
            }
            let line: Vec<u8> = buffer.drain(..len).collect();
            if end == 0 {
                continue;
            }
            if !self.echo {
                // the legacy docs contradict themselves about echo, so an echo that does show up is skipped
                if echoed(&line) {
                    continue;
                }
                return Some(line);
            }
            // an echo starts a new reply, so one that lost its prompt can't swallow the next command's. a line
            // with no echo in front of it belongs to nothing we sent
            if echoed(&line) {
                self.reply.clear();
            } else if self.reply.is_empty() {
                continue;
            }
            self.reply.extend_from_slice(&line);
        }
    }

    fn classify(&mut self, frame: &Frame) -> Incoming {
        if frame.starts_with(b"km.") && frame.get(4..BUTTONS_LEN) == Some(END) {
            return self.buttons(frame[3]);
        }
        if frame.starts_with(b"Axes(") {
            return match arguments::<i32>(frame, b"Axes(").as_deref() {
                Some(&[dx, dy, wheel]) => Incoming::Input {
                    input: Input::Motion {
                        dx,
                        dy,
                        wheel,
                        pan: 0,
                    },
                    box_time: None,
                },
                _ => Incoming::Noise,
            };
        }
        if frame.starts_with(b"Keys(") {
            return arguments::<u8>(frame, b"Keys(")
                .map_or(Incoming::Noise, |held| self.keys(held));
        }
        // the legacy port never gets waited on, and a prompt with no echo before it closes no command
        match find(frame, END) {
            Some(end) if self.echo && echoed(frame) && frame.ends_with(PROMPT) => {
                Incoming::Reply(key(&frame[..end]))
            }
            _ => Incoming::Noise,
        }
    }
}
