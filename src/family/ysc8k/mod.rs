use std::any::Any;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use crate::backend::{
    Backend, Command, InputKind, InputKinds, LOCKED, LockSlot, Sign, Sink, StreamFailure, UNLOCKED,
    Usage, WRITE_DEADLINE,
};
use crate::capability::{
    Action, Axis, Button, Capabilities, Capability, Family, Firmware, Info, Quirk, Snapshot,
};
use crate::error::{Error, Result};
use crate::transport::link::{Link, Target};
use crate::transport::probe::Probe;
use crate::transport::{Candidate, Port, System, is_ysc};

mod frames;
use frames::YscFrames;

const BAUD: u32 = 115_200;
/// the vendor driver's own order for finding a V1's rate, read out of its auto detect
const V1_BAUDS: [u32; 9] = [
    4_000_000, 115_200, 3_000_000, 230_400, 2_000_000, 460_800, 1_500_000, 921_600, 1_000_000,
];
/// how long the vendor driver waits for a V1's answer at each rate
const V1_WAIT: Duration = Duration::from_millis(200);
const BOOTLOADER: &str = "the YSC box is in its bootloader, finish or leave the firmware update in the vendor tool and connect again";
const KEYBOARD: &str =
    "the YSC box says there's no keyboard behind it, so plug one in and connect again";
const MEDIA: &str = "YSC has no media keys";
const BUTTONS: [Button; 5] = [
    Button::Left,
    Button::Right,
    Button::Middle,
    Button::Side1,
    Button::Side2,
];

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// the single chip box behind a USB serial bridge, or the dual chip one on its own CDC ports
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Variant {
    V1,
    V2,
}

/// a V2 only gets anything on its left port. any other port is taken as a V1, the connect code only lets a YSC
/// probe there when the caller asked for a YSC
fn guard(candidate: &Candidate) -> Result<Variant> {
    let Some(usb) = candidate.usb.as_ref().filter(|usb| is_ysc(usb)) else {
        return Ok(Variant::V1);
    };
    match usb.serial.as_deref() {
        Some("TOWMCULEFT") => return Ok(Variant::V2),
        Some(role @ ("TOWMCURIGHT" | "TOWMCUIAP")) => {
            return Err(Error::Unsupported {
                action: Action::Connect,
                family: Family::Ysc,
                reason: format!("this is the {role} port, use the TOWMCULEFT port instead").into(),
            });
        }
        _ => {}
    }
    Err(Error::NotFound {
        tried: vec![candidate.path.display().to_string()],
    })
}

fn packet(payload: &str) -> Result<Vec<u8>> {
    frames::json(payload).ok_or(Error::InvalidArgument {
        action: Action::Connect,
        detail: "the YSC JSON payload must have 1 to 4096 bytes".into(),
    })
}

fn line(text: &str) -> Vec<u8> {
    format!("{text}\r\n").into_bytes()
}

fn version(variant: Variant, frame: &[u8]) -> Option<String> {
    match variant {
        Variant::V1 => frames::v1_version(frame),
        Variant::V2 => frames::version(frame),
    }
}

fn describe(probe: &Probe<'_>, reconnect: bool, variant: Variant) -> Result<Snapshot> {
    let budget = match (variant, reconnect) {
        (Variant::V1, _) => V1_WAIT,
        (Variant::V2, true) => WRITE_DEADLINE,
        (Variant::V2, false) => Duration::from_millis(1500),
    };
    let reply = probe.ask(
        &packet("{\"cmd\":132}")?,
        &mut YscFrames::probe(),
        |frame| {
            version(variant, frame).is_some()
                || (variant == Variant::V1 && frames::bootloader(frame))
        },
        budget,
    )?;
    if variant == Variant::V1 && reply.as_deref().is_some_and(frames::bootloader) {
        return Err(Error::Unsupported {
            action: Action::Connect,
            family: Family::Ysc,
            reason: BOOTLOADER.into(),
        });
    }
    let version = reply
        .as_deref()
        .and_then(|frame| version(variant, frame))
        .ok_or(Error::NotFound { tried: Vec::new() })?;
    let debug_budget = if reconnect {
        WRITE_DEADLINE
    } else {
        Duration::from_millis(1200)
    };
    let status = probe.ask(
        &packet("{\"cmd\":702}")?,
        &mut YscFrames::probe(),
        |frame| frames::debug(frame).is_some(),
        debug_budget,
    )?;
    if status.as_deref().and_then(frames::debug) == Some(true) {
        return Err(Error::Rejected { action: Action::Connect, detail: "the YSC box is in debug mode, leave debug mode in the vendor tool and connect again".into() });
    }
    probe.say(&packet("{\"cmd\":34,\"status\":0}")?)?;
    // no echo means every text command after it would time out. it's asked twice because the vendor tool can turn
    // echo off, and then only the second one comes back
    let mut echoed = false;
    for _ in 0..2 {
        echoed = probe
            .ask(
                &line("km.echo(1)"),
                &mut YscFrames::probe(),
                |frame| frame.starts_with(b"km.echo(1)\r\n"),
                WRITE_DEADLINE,
            )?
            .is_some();
        if echoed {
            break;
        }
    }
    if !echoed {
        return Err(Error::Rejected {
            action: Action::Connect,
            detail: "the YSC box answers but ignores its text commands, which it's been seen doing after a replug. turning debug mode on and off in the vendor tool fixed it once, so try that and connect again".into(),
        });
    }
    Ok(snapshot(version, variant))
}

pub(crate) fn open(candidate: &Candidate, sink: Sink) -> Result<(Box<dyn Backend>, Snapshot)> {
    let (backend, snapshot) = attach(Arc::new(crate::transport::os::OsSystem), candidate, sink)?;
    Ok((Box::new(backend), snapshot))
}

pub(crate) fn attach(
    system: Arc<dyn System>,
    candidate: &Candidate,
    sink: Sink,
) -> Result<(YscBackend, Snapshot)> {
    let stop = AtomicBool::new(false);
    if guard(candidate)? == Variant::V2 {
        let port = system.open(&candidate.path, BAUD)?;
        let snapshot = describe(&Probe::new(&*port, &stop), false, Variant::V2)?;
        return start(
            system,
            port,
            candidate,
            snapshot,
            Variant::V2,
            vec![BAUD],
            sink,
        );
    }
    // only when no rate opens at all, the driver's error says more than NotFound
    let mut opened = false;
    let mut refused = None;
    for &baud in &V1_BAUDS {
        let port = match system.open(&candidate.path, baud) {
            Ok(port) => {
                opened = true;
                port
            }
            // a bridge whose driver doesn't take this rate can still take the next one
            Err(error @ Error::Io(_)) => {
                refused.get_or_insert(error);
                continue;
            }
            Err(error) => return Err(error),
        };
        match describe(&Probe::new(&*port, &stop), false, Variant::V1) {
            Ok(snapshot) => {
                // a reconnect tries the rate that worked first, then the rest in the vendor's order
                let bauds = std::iter::once(baud)
                    .chain(V1_BAUDS.into_iter().filter(|&other| other != baud))
                    .collect();
                return start(system, port, candidate, snapshot, Variant::V1, bauds, sink);
            }
            Err(Error::NotFound { .. }) => {}
            Err(error) => return Err(error),
        }
    }
    if !opened && let Some(error) = refused {
        return Err(error);
    }
    Err(Error::NotFound {
        tried: vec![candidate.path.display().to_string()],
    })
}

fn start(
    system: Arc<dyn System>,
    port: Box<dyn Port>,
    candidate: &Candidate,
    snapshot: Snapshot,
    variant: Variant,
    bauds: Vec<u32>,
    sink: Sink,
) -> Result<(YscBackend, Snapshot)> {
    let shared = Arc::new(Shared::new(snapshot.clone()));
    let reprobe = {
        let shared = shared.clone();
        Box::new(move |probe: &Probe<'_>| {
            let snapshot = describe(probe, true, variant)?;
            // a key write that's half recorded would land in the old history and survive the reset
            let _requests = lock(&shared.requests);
            let mut state = lock(&shared.state);
            let held_keys = std::mem::take(&mut state.held_keys);
            *state = State::new(snapshot.clone());
            state.held_keys = held_keys;
            Ok(snapshot)
        })
    };
    let link = Arc::new(Link::start(
        system,
        port,
        Box::new(YscFrames::new(shared.clone(), Some(sink.clone()))),
        Target {
            family: Family::Ysc,
            path: candidate.path.clone(),
            usb: candidate.usb.clone(),
            id: None,
            bauds,
        },
        reprobe,
        sink,
    )?);
    Ok((YscBackend { shared, link }, snapshot))
}

fn snapshot(version: String, variant: Variant) -> Snapshot {
    let mut info = Info::new(Family::Ysc);
    info.model = Some(
        match variant {
            Variant::V1 => "YSC 8K V1",
            Variant::V2 => "YSC 8K V2",
        }
        .into(),
    );
    info.firmware = Some(Firmware::Text(version));
    let capabilities = Capabilities::new(BUTTONS.to_vec())
        .with_axes(vec![Axis::X, Axis::Y])
        .set(Action::Move, Capability::native())
        .set(Action::Press, Capability::native())
        .set(Action::Release, Capability::native())
        .set(
            Action::Wheel,
            Capability::native().with(Quirk::NotchesPerCommand(1)),
        )
        .set(Action::LockButton, Capability::native())
        .set(
            Action::LockAxis,
            Capability::native().with(Quirk::WholeAxisOnly),
        )
        .set(Action::ButtonEvents, Capability::native())
        .set(Action::MotionEvents, Capability::native())
        .set(Action::KeyPress, Capability::native())
        .set(Action::KeyRelease, Capability::native())
        .set(Action::KeyEvents, Capability::native())
        .set(
            Action::MoveNow,
            Capability::unsupported("YSC has no move that skips ahead of the others"),
        )
        .set(
            Action::DiscardPendingMoves,
            Capability::unsupported("YSC can't drop moves it already took"),
        )
        .set(
            Action::Pan,
            Capability::unsupported("YSC has no horizontal scroll command"),
        )
        .set(
            Action::ScaleAxis,
            Capability::unsupported("YSC can only block a whole axis, not scale it"),
        )
        .set(
            Action::LockKey,
            Capability::unsupported("YSC has no way to lock a key"),
        );
    let no_media = Capability::unsupported(MEDIA);
    let capabilities = [
        Action::MediaPress,
        Action::MediaRelease,
        Action::MediaTap,
        Action::MediaEvents,
    ]
    .into_iter()
    .fold(capabilities, |capabilities, action| {
        capabilities.set(action, no_media.clone())
    });
    Snapshot::new(info, capabilities)
}

struct State {
    snapshot: Snapshot,
    seen: frames::Seen,
    streams: InputKinds,
    keys: bool,
    held_keys: HashSet<crate::key::Key>,
    /// what kinds of key command went out on this connection, since a refusal doesn't say which one it's for
    pressed_keys: bool,
    released_keys: bool,
    refused: bool,
}

impl State {
    fn new(snapshot: Snapshot) -> Self {
        Self {
            snapshot,
            seen: frames::Seen::default(),
            streams: InputKinds::default(),
            keys: true,
            held_keys: HashSet::new(),
            pressed_keys: false,
            released_keys: false,
            refused: false,
        }
    }
}

struct Shared {
    state: Mutex<State>,
    requests: Mutex<()>,
    stop: AtomicBool,
}

impl Shared {
    fn new(snapshot: Snapshot) -> Self {
        Self {
            state: Mutex::new(State::new(snapshot)),
            requests: Mutex::new(()),
            stop: AtomicBool::new(false),
        }
    }
}

pub(crate) struct YscBackend {
    shared: Arc<Shared>,
    link: Arc<Link>,
}

fn button_name(button: Button) -> Option<&'static str> {
    Some(match button {
        Button::Left => "left",
        Button::Right => "right",
        Button::Middle => "middle",
        Button::Side1 => "ms1",
        Button::Side2 => "ms2",
        _ => return None,
    })
}

fn lock_name(slot: LockSlot) -> Option<&'static str> {
    Some(match slot {
        LockSlot::Button(Button::Left) => "ml",
        LockSlot::Button(Button::Right) => "mr",
        LockSlot::Button(Button::Middle) => "mm",
        LockSlot::Button(Button::Side1) => "ms1",
        LockSlot::Button(Button::Side2) => "ms2",
        LockSlot::Axis(Axis::X, _) => "mx",
        LockSlot::Axis(Axis::Y, _) => "my",
        _ => return None,
    })
}

impl YscBackend {
    fn unsupported(&self, action: Action, reason: &'static str) -> Error {
        Error::Unsupported {
            action,
            family: Family::Ysc,
            reason: reason.into(),
        }
    }

    fn send_until(&self, command: Command, until: Instant) -> Result<()> {
        let _request = lock(&self.shared.requests);
        let action = command.action();
        if let Command::KeyPress(key) | Command::KeyRelease(key) = command {
            if !key.is_valid() {
                return Err(Error::InvalidArgument {
                    action,
                    detail: "YSC keys have to be HID usages from 0x04 to 0xE7".into(),
                });
            }
            let down = matches!(command, Command::KeyPress(_));
            {
                let mut state = lock(&self.shared.state);
                if !state.keys && (down || !state.held_keys.contains(&key)) {
                    return Err(self.unsupported(action, KEYBOARD));
                }
                // a refusal doesn't say which key it's for, so even a cleanup release could be what it's about
                if down {
                    state.pressed_keys = true;
                } else {
                    state.released_keys = true;
                }
                if down {
                    state.held_keys.insert(key);
                }
            }
            let sent = self
                .link
                .request_until(
                    packet(&format!(
                        "{{\"cmd\":45,\"kc\":{},\"down\":{}}}",
                        key.0,
                        u8::from(down)
                    ))?,
                    // the box chokes on writes that come right after one another, so each key waits for its answer
                    Some(frames::key_reply()),
                    until,
                )
                .and_then(|reply| match reply.as_deref().and_then(frames::object) {
                    Some(value)
                        if value.get("code").and_then(serde_json::Value::as_u64) == Some(400) =>
                    {
                        Err(Error::Rejected {
                            action,
                            detail: "the YSC box couldn't read the key command, try it again"
                                .into(),
                        })
                    }
                    _ => Ok(()),
                });
            if sent.is_ok() && !down {
                lock(&self.shared.state).held_keys.remove(&key);
            }
            return sent;
        }
        let text = match command {
            Command::Move(dx, dy) => format!("km.move({dx},{dy})"),
            Command::Press(button) | Command::Release(button) => {
                let name = button_name(button)
                    .ok_or_else(|| self.unsupported(action, "YSC has five mouse buttons"))?;
                format!(
                    "km.{name}({})",
                    u8::from(matches!(command, Command::Press(_)))
                )
            }
            Command::Wheel(0) => return Ok(()),
            Command::Wheel(notches) if (-1..=1).contains(&notches) => {
                format!("km.wheel({notches})")
            }
            Command::Lock { slot, keep } if keep == LOCKED || keep == UNLOCKED => {
                let name = lock_name(slot).ok_or_else(|| {
                    self.unsupported(action, "YSC only locks buttons and whole X or Y axes")
                })?;
                // the core pairs whole-axis slots, the positive slot carries the lock and either can clean it up
                if matches!(slot, LockSlot::Axis(_, Sign::Negative)) && keep == LOCKED {
                    return Ok(());
                }
                format!("km.lock_{name}({})", u8::from(keep == LOCKED))
            }
            _ => {
                return Err(
                    self.unsupported(action, "YSC has no common command for this operation")
                );
            }
        };
        self.link
            .request_until(line(&text), Some(frames::key(text.as_bytes())), until)
            .map(|_| ())
    }

    fn upload(&self, on: bool, until: Instant) -> Result<()> {
        self.link
            .request_until(
                packet(&format!("{{\"cmd\":34,\"status\":{}}}", u8::from(on)))?,
                None,
                until,
            )
            .map(|_| ())
    }
}

impl Backend for YscBackend {
    fn send(&self, command: Command) -> Result<()> {
        self.send_until(command, Instant::now() + WRITE_DEADLINE)
    }

    fn set_streams(&self, kinds: InputKinds) -> std::result::Result<(), StreamFailure> {
        let result = (|| {
            let _request = lock(&self.shared.requests);
            if kinds.media {
                return Err(self.unsupported(Action::MediaEvents, MEDIA));
            }
            // the box sends its keyboard report by itself, so only buttons and motion have a switch
            let current = lock(&self.shared.state).streams;
            if (current.buttons || current.motion) != (kinds.buttons || kinds.motion) {
                self.upload(
                    kinds.buttons || kinds.motion,
                    Instant::now() + WRITE_DEADLINE,
                )?;
                lock(&self.shared.state).seen.buttons = None;
            }
            lock(&self.shared.state).streams = kinds;
            Ok(())
        })();
        result.map_err(|error| StreamFailure {
            failed: kinds,
            error,
        })
    }

    fn restart_stream(&self, kind: InputKind) -> Result<()> {
        let _request = lock(&self.shared.requests);
        match kind {
            InputKind::Media => return Err(self.unsupported(Action::MediaEvents, MEDIA)),
            InputKind::Keys => return Ok(()),
            InputKind::Buttons | InputKind::Motion => {}
        }
        let streams = lock(&self.shared.state).streams;
        if streams.buttons || streams.motion {
            lock(&self.shared.state).seen.buttons = None;
            self.upload(true, Instant::now() + WRITE_DEADLINE)?;
        }
        Ok(())
    }

    fn release_all(&self, held: &[Usage]) -> Result<()> {
        let mut first = Ok(());
        for &usage in held {
            let result = self.send(usage.release());
            if first.is_ok() {
                first = result;
            }
        }
        first
    }

    fn unlock_all(&self, slots: &[LockSlot]) -> Result<()> {
        let mut first = Ok(());
        for &slot in slots {
            let result = self.send(Command::Lock {
                slot,
                keep: UNLOCKED,
            });
            if matches!(result, Err(Error::Disconnected)) {
                return result;
            }
            if first.is_ok() {
                first = result;
            }
        }
        first
    }

    fn shutdown(&self, deadline: Duration) {
        let _ = self.close(deadline);
    }

    fn close(&self, deadline: Duration) -> Result<()> {
        if self.shared.stop.swap(true, Ordering::AcqRel) {
            return Ok(());
        }
        let until = Instant::now() + deadline;
        let streams = lock(&self.shared.state).streams;
        // the box keeps uploading to whoever opens the port next, so a failed off is worth reporting, but only
        // when anybox turned it on
        let stopped = if streams.buttons || streams.motion {
            self.upload(false, until)
        } else {
            Ok(())
        };
        self.link
            .shutdown(until.saturating_duration_since(Instant::now()));
        stopped
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Drop for YscBackend {
    fn drop(&mut self) {
        self.shutdown(WRITE_DEADLINE);
    }
}

#[cfg(test)]
mod tests;
