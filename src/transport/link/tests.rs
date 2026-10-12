use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::{Link, Reprobe, Target, same_box};
use crate::backend::{Input, Signal, Sink, WRITE_DEADLINE};
use crate::capability::{Capabilities, Family, Info, Snapshot, StableId};
use crate::error::Error;
use crate::transport::Candidate;
use crate::transport::codec::{Codec, Frame, Incoming};
use crate::transport::fake::{FakeBox, FakeSystem};
use crate::transport::probe::Probe;
use crate::transport::{System, UsbId};

#[test]
fn targets_without_a_unique_id_require_the_original_path_and_exact_usb_tuple() {
    for family in [Family::Ferrum, Family::Ysc] {
        for original in [
            None,
            usb("SERIAL01"),
            Some(UsbId {
                vid: 0x1a86,
                pid: 0xfe0c,
                serial: None,
            }),
        ] {
            let target = Target {
                family,
                path: "COM1".into(),
                usb: original.clone(),
                id: None,
                bauds: vec![115200],
            };
            assert!(same_box(
                &Candidate {
                    path: "COM1".into(),
                    usb: original.clone()
                },
                &target
            ));
            assert!(!same_box(
                &Candidate {
                    path: "COM2".into(),
                    usb: original.clone()
                },
                &target
            ));
            for changed in [
                None,
                usb("OTHER0001"),
                Some(UsbId {
                    vid: 0x1234,
                    pid: 0xfe0c,
                    serial: None,
                }),
                Some(UsbId {
                    vid: 0x1a86,
                    pid: 0x1234,
                    serial: Some("SERIAL01".into()),
                }),
            ] {
                if changed != original {
                    assert!(!same_box(
                        &Candidate {
                            path: "COM1".into(),
                            usb: changed
                        },
                        &target
                    ));
                }
            }
        }
    }
}

#[test]
fn a_nonunique_serial_does_not_follow_a_renamed_port_but_reopens_the_original_one() {
    let setup = setup(echo);
    setup.system.unplug("COM1");
    assert_eq!(next_link_event(&setup.signals), "disconnected");
    let renamed = FakeBox::new(echo);
    setup.system.plug("COM7", usb("SERIAL01"), renamed.clone());
    assert!(
        setup
            .signals
            .recv_timeout(Duration::from_millis(350))
            .is_err()
    );
    assert!(renamed.written().is_empty());
    assert_eq!(setup.system.opened_at().len(), 1);
    let original = FakeBox::new(echo);
    setup.system.plug("COM1", usb("SERIAL01"), original);
    assert_eq!(next_link_event(&setup.signals), "reconnected");
    assert_eq!(
        setup
            .link
            .request(b"Q6\n".to_vec(), Some(6))
            .unwrap()
            .as_deref(),
        Some(&b"R6"[..])
    );
}

/// a codec for plain text lines, where `R<key> ...` is a reply, `I<dx>` is motion and anything else is noise
struct Lines;

impl Codec for Lines {
    fn split(&mut self, buffer: &mut Vec<u8>) -> Option<Frame> {
        let end = buffer.iter().position(|&b| b == b'\n')?;
        let mut line: Vec<u8> = buffer.drain(..=end).collect();
        line.pop();
        Some(line)
    }

    fn classify(&mut self, frame: &Frame) -> Incoming {
        let text = String::from_utf8_lossy(frame);
        let number = |rest: &str| rest.split_whitespace().next()?.parse::<i32>().ok();
        match text.chars().next() {
            Some('R') => {
                number(&text[1..]).map_or(Incoming::Noise, |key| Incoming::Reply(key as u32))
            }
            Some('I') => number(&text[1..]).map_or(Incoming::Noise, |dx| Incoming::Input {
                input: Input::Motion {
                    dx,
                    dy: 0,
                    wheel: 0,
                    pan: 0,
                },
                box_time: None,
            }),
            _ => Incoming::Noise,
        }
    }
}

/// answers `Q<key>` with `R<key>`, anything else gets no answer
fn echo(bytes: &[u8]) -> Vec<Vec<u8>> {
    let text = String::from_utf8_lossy(bytes);
    match text.strip_prefix('Q') {
        Some(rest) => vec![format!("R{rest}").into_bytes()],
        None => Vec::new(),
    }
}

fn usb(serial: &str) -> Option<UsbId> {
    Some(UsbId {
        vid: 0x1A86,
        pid: 0x55D3,
        serial: Some(serial.into()),
    })
}

fn snapshot() -> Snapshot {
    Snapshot::new(Info::new(Family::Medius), Capabilities::new(Vec::new()))
}

fn reprobe() -> Reprobe {
    answering_as(Arc::new(Mutex::new(snapshot())))
}

/// probes again by asking the box, then says the box is whatever `identity` holds at that moment
fn answering_as(identity: Arc<Mutex<Snapshot>>) -> Reprobe {
    Box::new(move |probe: &Probe<'_>| {
        match probe.ask(
            b"Q99\n",
            &mut Lines,
            |frame| frame == b"R99",
            Duration::from_millis(300),
        )? {
            Some(_) => Ok(identity.lock().unwrap().clone()),
            None => Err(Error::NotFound { tried: Vec::new() }),
        }
    })
}

fn with_id(id: Option<StableId>) -> Snapshot {
    let mut info = Info::new(Family::Medius);
    info.id = id;
    Snapshot::new(info, Capabilities::new(Vec::new()))
}

struct Setup {
    system: FakeSystem,
    fake: Arc<FakeBox>,
    link: Link,
    signals: Receiver<Signal>,
}

fn setup(responder: impl FnMut(&[u8]) -> Vec<Vec<u8>> + Send + 'static) -> Setup {
    setup_with(responder, None, reprobe())
}

fn setup_with(
    responder: impl FnMut(&[u8]) -> Vec<Vec<u8>> + Send + 'static,
    id: Option<StableId>,
    reprobe: Reprobe,
) -> Setup {
    let system = FakeSystem::default();
    let fake = FakeBox::new(responder);
    system.plug("COM1", usb("SERIAL01"), fake.clone());
    let port = system.open("COM1".as_ref(), 4_000_000).unwrap();
    let (tx, signals) = mpsc::channel();
    let link = Link::start(
        Arc::new(system.clone()),
        port,
        Box::new(Lines),
        Target {
            family: Family::Medius,
            path: "COM1".into(),
            usb: usb("SERIAL01"),
            id,
            bauds: vec![4_000_000],
        },
        reprobe,
        Sink(tx),
    )
    .unwrap();
    Setup {
        system,
        fake,
        link,
        signals,
    }
}

fn next_link_event(signals: &Receiver<Signal>) -> &'static str {
    let deadline = Instant::now() + Duration::from_secs(5);
    while let Ok(signal) = signals.recv_timeout(deadline.saturating_duration_since(Instant::now()))
    {
        match signal {
            Signal::Disconnected => return "disconnected",
            Signal::Reconnected(_) => return "reconnected",
            _ => {}
        }
    }
    "nothing"
}

#[test]
fn a_request_gets_its_reply() {
    let setup = setup(echo);
    let reply = setup.link.request(b"Q1\n".to_vec(), Some(1)).unwrap();
    assert_eq!(reply.as_deref(), Some(&b"R1"[..]));
}

#[test]
fn a_reply_that_beats_the_write_still_finds_its_request() {
    let setup = setup(echo);
    setup
        .fake
        .reply_before_return
        .store(true, std::sync::atomic::Ordering::Release);
    let reply = setup.link.request(b"Q8\n".to_vec(), Some(8)).unwrap();
    assert_eq!(reply.as_deref(), Some(&b"R8"[..]));
}

#[test]
fn a_box_with_another_id_after_a_replug_is_not_reattached() {
    let ours = StableId::Mac([1, 2, 3, 4, 5, 6]);
    let identity = Arc::new(Mutex::new(with_id(Some(ours.clone()))));
    let setup = setup_with(echo, Some(ours.clone()), answering_as(identity.clone()));
    setup.system.unplug("COM1");
    assert_eq!(next_link_event(&setup.signals), "disconnected");

    *identity.lock().unwrap() = with_id(Some(StableId::Mac([9; 6])));
    setup
        .system
        .plug("COM1", usb("SERIAL01"), FakeBox::new(echo));
    thread::sleep(Duration::from_millis(500));
    assert!(
        setup.signals.try_recv().is_err(),
        "reattached to a box with another id"
    );

    *identity.lock().unwrap() = with_id(Some(ours));
    assert_eq!(next_link_event(&setup.signals), "reconnected");
}

#[test]
fn a_box_of_another_family_after_a_replug_is_not_reattached() {
    let identity = Arc::new(Mutex::new(Snapshot::new(
        Info::new(Family::Ferrum),
        Capabilities::new(Vec::new()),
    )));
    let setup = setup_with(echo, None, answering_as(identity));
    setup.system.unplug("COM1");
    assert_eq!(next_link_event(&setup.signals), "disconnected");
    setup
        .system
        .plug("COM1", usb("SERIAL01"), FakeBox::new(echo));
    thread::sleep(Duration::from_millis(500));
    assert!(setup.signals.try_recv().is_err());
}

#[test]
fn a_reply_split_across_reads_still_arrives() {
    let setup = setup(echo);
    setup
        .fake
        .max_read
        .store(1, std::sync::atomic::Ordering::Release);
    let reply = setup.link.request(b"Q42\n".to_vec(), Some(42)).unwrap();
    assert_eq!(reply.as_deref(), Some(&b"R42"[..]));
}

#[test]
fn input_between_a_request_and_its_reply_goes_to_the_sink() {
    let setup = setup(|bytes: &[u8]| {
        let mut out = vec![b"I5\n".to_vec()];
        out.extend(echo(bytes));
        out
    });
    let reply = setup.link.request(b"Q2\n".to_vec(), Some(2)).unwrap();
    assert_eq!(reply.as_deref(), Some(&b"R2"[..]));
    let input = setup.signals.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(matches!(
        input,
        Signal::Input {
            input: Input::Motion { dx: 5, .. },
            ..
        }
    ));
}

#[test]
fn two_requests_with_the_same_key_complete_in_order() {
    let held = Arc::new(Mutex::new(false));
    let setup = setup({
        let held = held.clone();
        move |bytes: &[u8]| {
            let mut first = held.lock().unwrap();
            if bytes == b"Q7 a\n" {
                *first = true;
                Vec::new()
            } else {
                vec![b"R7 a\n".to_vec(), b"R7 b\n".to_vec()]
            }
        }
    });
    let link = Arc::new(setup.link);
    let first = thread::spawn({
        let link = link.clone();
        move || link.request(b"Q7 a\n".to_vec(), Some(7))
    });
    while !*held.lock().unwrap() {
        thread::sleep(Duration::from_millis(1));
    }
    let second = link.request(b"Q7 b\n".to_vec(), Some(7)).unwrap();
    assert_eq!(
        first.join().unwrap().unwrap().as_deref(),
        Some(&b"R7 a"[..])
    );
    assert_eq!(second.as_deref(), Some(&b"R7 b"[..]));
}

#[test]
fn a_caller_is_bounded_while_the_writer_is_stuck() {
    let setup = setup(echo);
    setup.fake.stall_writes();
    let link = Arc::new(setup.link);
    let stuck = thread::spawn({
        let link = link.clone();
        move || link.request(b"first\n".to_vec(), None)
    });
    thread::sleep(Duration::from_millis(50));
    let started = Instant::now();
    assert!(matches!(
        link.request(b"second\n".to_vec(), None),
        Err(Error::Disconnected)
    ));
    assert!(started.elapsed() < WRITE_DEADLINE + Duration::from_millis(200));
    assert!(matches!(stuck.join().unwrap(), Err(Error::Disconnected)));
}

mod writer {
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicBool, AtomicU64};
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex, RwLock};
    use std::time::{Duration, Instant};

    use super::{echo, usb};
    use crate::backend::Sink;
    use crate::error::Error;
    use crate::transport::System;
    use crate::transport::fake::{FakeBox, FakeSystem};
    use crate::transport::link::{Request, Shared, write_loop};

    /// runs the writer over one request and returns what reached the box and what the caller was told
    fn write_one(given_up: bool, deadline: Instant) -> (Vec<Vec<u8>>, Option<Result<(), Error>>) {
        let system = FakeSystem::default();
        let fake = FakeBox::new(echo);
        system.plug("COM1", usb("SERIAL01"), fake.clone());
        let port = system.open("COM1".as_ref(), 115_200).unwrap();
        let (tx, _signals) = mpsc::channel();
        let shared = Shared {
            system: Arc::new(system),
            port: RwLock::new(Some((Arc::from(port), 0))),
            pending: Mutex::new(VecDeque::new()),
            stop: AtomicBool::new(false),
            down: AtomicBool::new(false),
            next_id: AtomicU64::new(0),
            generations: AtomicU64::new(1),
            sink: Sink(tx),
        };
        let (queue, requests) = mpsc::channel();
        let (reply, answer) = mpsc::channel();
        queue
            .send(Request {
                id: 1,
                bytes: b"command\n".to_vec(),
                expect: None,
                deadline,
                given_up: Arc::new(AtomicBool::new(given_up)),
                reply,
            })
            .unwrap();
        drop(queue);
        write_loop(&shared, &requests);
        let told = answer.try_recv().ok().map(|result| result.map(|_| ()));
        (fake.written(), told)
    }

    #[test]
    fn a_command_whose_caller_gave_up_is_never_written() {
        let (written, told) = write_one(true, Instant::now() + Duration::from_secs(5));
        assert!(written.is_empty());
        assert!(told.is_none());
    }

    #[test]
    fn a_command_past_its_deadline_is_never_written() {
        let (written, told) = write_one(false, Instant::now() - Duration::from_millis(1));
        assert!(written.is_empty());
        assert!(told.is_none());
    }

    fn shared_on_generation(generation: u64) -> Shared {
        let system = FakeSystem::default();
        system.plug("COM1", usb("SERIAL01"), FakeBox::new(echo));
        let port = system.open("COM1".as_ref(), 115_200).unwrap();
        let (tx, _signals) = mpsc::channel();
        Shared {
            system: Arc::new(system),
            port: RwLock::new(Some((Arc::from(port), generation))),
            pending: Mutex::new(VecDeque::new()),
            stop: AtomicBool::new(false),
            down: AtomicBool::new(false),
            next_id: AtomicU64::new(0),
            generations: AtomicU64::new(generation + 1),
            sink: Sink(tx),
        }
    }

    #[test]
    fn trouble_seen_on_an_older_opening_leaves_the_current_port_alone() {
        let shared = shared_on_generation(3);
        shared.take_down(2);
        assert!(!shared.down.load(std::sync::atomic::Ordering::Acquire));
        shared.take_down(3);
        assert!(shared.down.load(std::sync::atomic::Ordering::Acquire));
    }

    #[test]
    fn a_reply_on_a_newer_opening_never_completes_an_older_request() {
        let shared = shared_on_generation(3);
        let (stale, stale_answer) = mpsc::channel();
        let (fresh, fresh_answer) = mpsc::channel();
        {
            let mut pending = shared.pending.lock().unwrap();
            pending.push_back(super::super::Pending {
                id: 1,
                key: 7,
                generation: 2,
                reply: stale,
            });
            pending.push_back(super::super::Pending {
                id: 2,
                key: 7,
                generation: 3,
                reply: fresh,
            });
        }
        shared.complete(7, 3, b"R7".to_vec());
        assert!(stale_answer.try_recv().is_err());
        assert!(matches!(fresh_answer.try_recv(), Ok(Ok(Some(_)))));
    }

    #[test]
    fn a_live_command_is_written_and_acknowledged() {
        let (written, told) = write_one(false, Instant::now() + Duration::from_secs(5));
        assert_eq!(written, vec![b"command\n".to_vec()]);
        assert!(matches!(told, Some(Ok(()))));
    }
}

#[test]
fn a_missed_reply_takes_the_link_down_and_it_comes_back() {
    let setup = setup(echo);
    let started = Instant::now();
    assert!(matches!(
        setup.link.request(b"silent\n".to_vec(), Some(3)),
        Err(Error::Disconnected)
    ));
    assert!(started.elapsed() < WRITE_DEADLINE + Duration::from_millis(200));
    assert_eq!(next_link_event(&setup.signals), "disconnected");
    assert_eq!(next_link_event(&setup.signals), "reconnected");
    let reply = setup.link.request(b"Q4\n".to_vec(), Some(4)).unwrap();
    assert_eq!(reply.as_deref(), Some(&b"R4"[..]));
}

#[test]
fn a_vanished_box_is_only_reattached_when_it_is_the_same_one() {
    let id = StableId::UsbSerial("SERIAL01".into());
    let setup = setup_with(
        echo,
        Some(id.clone()),
        answering_as(Arc::new(Mutex::new(with_id(Some(id))))),
    );
    setup.system.unplug("COM1");
    assert_eq!(next_link_event(&setup.signals), "disconnected");
    assert!(matches!(
        setup.link.request(b"Q5\n".to_vec(), Some(5)),
        Err(Error::Disconnected)
    ));

    // a different box on the same port isn't ours
    setup
        .system
        .plug("COM1", usb("OTHER0001"), FakeBox::new(echo));
    thread::sleep(Duration::from_millis(400));
    assert!(setup.signals.try_recv().is_err());

    // ours comes back on another port and gets found by its serial
    setup
        .system
        .plug("COM7", usb("SERIAL01"), FakeBox::new(echo));
    assert_eq!(next_link_event(&setup.signals), "reconnected");
    let reply = setup.link.request(b"Q6\n".to_vec(), Some(6)).unwrap();
    assert_eq!(reply.as_deref(), Some(&b"R6"[..]));
}

#[test]
fn a_box_that_answers_wrong_after_a_replug_is_not_reattached() {
    let setup = setup(echo);
    setup.system.unplug("COM1");
    assert_eq!(next_link_event(&setup.signals), "disconnected");
    setup
        .system
        .plug("COM1", usb("SERIAL01"), FakeBox::new(|_: &[u8]| Vec::new()));
    thread::sleep(Duration::from_millis(800));
    assert!(setup.signals.try_recv().is_err());
}

#[test]
fn shutdown_is_quick_while_reading_and_while_reconnecting() {
    let setup = setup(echo);
    let started = Instant::now();
    setup.link.shutdown(Duration::from_secs(1));
    assert!(started.elapsed() < Duration::from_millis(500));
    drop(setup.link);
    assert_eq!(setup.system.open_ports(), 0);

    let setup = self::setup(echo);
    setup.system.unplug("COM1");
    assert_eq!(next_link_event(&setup.signals), "disconnected");
    thread::sleep(Duration::from_millis(150));
    let started = Instant::now();
    setup.link.shutdown(Duration::from_secs(1));
    assert!(started.elapsed() < Duration::from_millis(500));
    drop(setup.link);
    assert_eq!(setup.system.open_ports(), 0);
}

#[test]
fn a_refused_open_says_why() {
    let system = FakeSystem::default();
    system.plug("COM1", None, FakeBox::new(echo));
    #[cfg(windows)]
    let busy = 5;
    #[cfg(unix)]
    let busy = libc::EBUSY;
    system.refuse(Some(busy));
    assert!(matches!(
        system.open("COM1".as_ref(), 115_200),
        Err(Error::PortBusy { .. })
    ));
    system.refuse(None);
    assert!(matches!(
        system.open("COM2".as_ref(), 115_200),
        Err(Error::NotFound { .. })
    ));
}

mod hardware {
    use std::sync::Arc;
    use std::sync::mpsc;
    use std::time::Duration;

    use super::{Target, snapshot};
    use crate::backend::Sink;
    use crate::capability::Family;
    use crate::error::Error;
    use crate::transport::System;
    use crate::transport::codec::{Codec, Frame, Incoming};
    use crate::transport::link::{Link, Reprobe};
    use crate::transport::os::OsSystem;

    const MEDIUS_BAUD: u32 = 6_000_000;

    /// just enough of the Medius frame to send QUERY(VERSION) and read its reply. the full frame is in the
    /// medius crate's source
    struct MediusFrames;

    fn crc16(bytes: &[u8]) -> u16 {
        let mut crc = 0xFFFFu16;
        for &byte in bytes {
            crc ^= u16::from(byte) << 8;
            for _ in 0..8 {
                crc = if crc & 0x8000 != 0 {
                    (crc << 1) ^ 0x1021
                } else {
                    crc << 1
                };
            }
        }
        crc
    }

    fn query_version(seq: u8) -> Vec<u8> {
        let body = [0x05, seq, 0x01, 0x00, 0x00];
        let crc = crc16(&body);
        let mut frame = vec![0xA5];
        frame.extend_from_slice(&body);
        frame.extend_from_slice(&crc.to_le_bytes());
        frame
    }

    impl Codec for MediusFrames {
        fn split(&mut self, buffer: &mut Vec<u8>) -> Option<Frame> {
            let start = buffer.iter().position(|&b| b == 0xA5)?;
            buffer.drain(..start);
            if buffer.len() < 5 {
                return None;
            }
            let total = 5 + usize::from(u16::from_le_bytes([buffer[3], buffer[4]])) + 2;
            if buffer.len() < total {
                return None;
            }
            Some(buffer.drain(..total).collect())
        }

        fn classify(&mut self, frame: &Frame) -> Incoming {
            // a RESP carries the same SEQ as the query it answers
            if frame[1] == 0x06 {
                Incoming::Reply(u32::from(frame[2]))
            } else {
                Incoming::Noise
            }
        }
    }

    #[test]
    fn the_crc_matches_the_catalogue_check_value() {
        assert_eq!(crc16(b"123456789"), 0x29B1);
    }

    /// needs a Medius box. run it with `ANYBOX_PORT=COM5 cargo test -- --ignored --nocapture`, with your box's
    /// port in place of COM5
    #[test]
    #[ignore]
    fn fifty_links_on_a_real_port_open_answer_and_close_cleanly() {
        let path =
            std::env::var("ANYBOX_PORT").expect("set ANYBOX_PORT to the port your box is on");
        let system = Arc::new(OsSystem);
        let entry = system
            .ports()
            .into_iter()
            .find(|candidate| candidate.path.to_string_lossy() == path);
        println!("ports() lists the box as {entry:?}");

        for round in 1..=50u8 {
            let port = system.open(path.as_ref(), MEDIUS_BAUD).unwrap();
            let (tx, _signals) = mpsc::channel();
            let reprobe: Reprobe = Box::new(|_| Ok(snapshot()));
            let link = Link::start(
                system.clone(),
                port,
                Box::new(MediusFrames),
                Target {
                    family: Family::Medius,
                    path: path.clone().into(),
                    usb: entry.as_ref().and_then(|e| e.usb.clone()),
                    id: None,
                    bauds: vec![MEDIUS_BAUD],
                },
                reprobe,
                Sink(tx),
            )
            .unwrap();
            // let the reader sit in its comm event wait for a moment before the query goes out
            std::thread::sleep(Duration::from_millis(30));
            let reply = link
                .request(query_version(round), Some(u32::from(round)))
                .unwrap_or_else(|error| panic!("round {round}: {error}"))
                .unwrap();
            assert_eq!(reply[1], 0x06, "round {round}: not a RESP");
            if round == 50 {
                assert!(matches!(
                    system.open(path.as_ref(), MEDIUS_BAUD),
                    Err(Error::PortBusy { .. })
                ));
                println!("protocol version {}", reply[6]);
            }
            link.shutdown(Duration::from_secs(1));
        }
    }
}

#[test]
fn a_probe_ends_its_wait_before_it_returns() {
    let system = FakeSystem::default();
    system.plug("COM1", usb("SERIAL01"), FakeBox::new(echo));
    let port = system.open("COM1".as_ref(), 115_200).unwrap();
    let stop = std::sync::atomic::AtomicBool::new(false);
    let probe = Probe::new(&*port, &stop);
    let answer = probe
        .ask(
            b"Q1
",
            &mut Lines,
            |frame| frame == b"R1",
            Duration::from_millis(300),
        )
        .unwrap();
    assert!(answer.is_some());
    assert_eq!(system.retired(), 1);
    let silent = probe
        .ask(
            b"nothing
",
            &mut Lines,
            |_| true,
            Duration::from_millis(50),
        )
        .unwrap();
    assert!(silent.is_none());
    assert_eq!(system.retired(), 2);
}

#[test]
fn every_reconnect_opens_a_new_generation() {
    let setup = setup(echo);
    let generation = || setup.link.shared.current().map(|(_, g)| g);
    let first = generation();
    setup.system.unplug("COM1");
    assert_eq!(next_link_event(&setup.signals), "disconnected");
    setup
        .system
        .plug("COM1", usb("SERIAL01"), FakeBox::new(echo));
    assert_eq!(next_link_event(&setup.signals), "reconnected");
    let second = generation();
    assert!(first.is_some() && second.is_some() && first != second);
}
