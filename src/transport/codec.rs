use crate::backend::Input;
use crate::capability::Action;
use crate::event::BoxTime;

pub(crate) type Frame = Vec<u8>;

/// what ties a reply to the request waiting for it. when several requests share a key, the replies
/// complete them in the order they were sent
pub(crate) type ReplyKey = u32;

#[derive(Debug)]
pub(crate) enum Incoming {
    Reply(ReplyKey),
    Input {
        input: Input,
        box_time: Option<BoxTime>,
    },
    /// several at once, for a box that reports a whole state and leaves it to us to find what changed
    Inputs(Vec<Input>),
    /// the box turned an input stream off on its own
    StreamLost(crate::backend::InputKind),
    /// the box refused the command with this key. if a request is waiting on the key this completes it,
    /// otherwise the core gets told
    Refused {
        key: ReplyKey,
        /// none for the library's own queries, because the app never sent those
        action: Option<Action>,
    },
    Noise,
}

/// one family's wire format, so where a frame ends and what it is
pub(crate) trait Codec: Send + 'static {
    /// takes one whole frame off the front of the buffer, or leaves the buffer alone until more bytes come
    fn split(&mut self, buffer: &mut Vec<u8>) -> Option<Frame>;
    fn classify(&mut self, frame: &Frame) -> Incoming;
}
