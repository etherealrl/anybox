use std::any::Any;
use std::cell::RefCell;
use std::path::Path;
use std::time::Duration;

use super::{Connect, Search, port_order};
use crate::backend::{Backend, Command, InputKind, InputKinds, LockSlot, Sink, Usage};
use crate::capability::{
    Action, Button, Capabilities, Capability, Family, FerrumApi, Firmware, Info, Snapshot, StableId,
};
use crate::error::{Error, Result};
use crate::family::Open;
use crate::transport::{Candidate, PortUsb};

const A: StableId = StableId::Mac([1, 2, 3, 4, 5, 6]);
const B: StableId = StableId::Mac([6, 5, 4, 3, 2, 1]);

#[derive(Clone)]
enum Reply {
    Box(Option<StableId>),
    /// a Ferrum on one of its two APIs
    Ferrum(FerrumApi),
    /// a box whose mouse moves and clicks only through a press and a release, like a Medius
    Mouse,
    Busy,
    TooOld,
}

thread_local! {
    /// what each port and family pair answers, anything not in here is NotFound
    static SCRIPT: RefCell<Vec<(String, Family, Reply)>> = const { RefCell::new(Vec::new()) };
    /// every open and shutdown, in order
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

fn script(replies: &[(&str, Family, Reply)]) {
    SCRIPT.with_borrow_mut(|script| {
        *script = replies
            .iter()
            .map(|(port, family, reply)| (port.to_string(), *family, reply.clone()))
            .collect();
    });
    LOG.with_borrow_mut(Vec::clear);
}

fn log() -> Vec<String> {
    LOG.with_borrow(Clone::clone)
}

struct Quiet(String);

impl Backend for Quiet {
    fn send(&self, _: Command) -> Result<()> {
        Ok(())
    }

    fn set_streams(&self, _: InputKinds) -> std::result::Result<(), crate::backend::StreamFailure> {
        Ok(())
    }

    fn restart_stream(&self, _: InputKind) -> Result<()> {
        Ok(())
    }

    fn unlock_all(&self, _: &[LockSlot]) -> Result<()> {
        Ok(())
    }

    fn release_all(&self, _: &[Usage]) -> Result<()> {
        Ok(())
    }

    fn type_piece(&self, _: &[u8], _: &mut crate::backend::Typing) -> Result<()> {
        Ok(())
    }

    fn shutdown(&self, _: Duration) {
        LOG.with_borrow_mut(|log| log.push(format!("shut {}", self.0)));
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn answer(family: Family, candidate: &Candidate) -> Result<(Box<dyn Backend>, Snapshot)> {
    let port = candidate.path.display().to_string();
    LOG.with_borrow_mut(|log| log.push(format!("{family:?} {port}")));
    let reply = SCRIPT.with_borrow(|script| {
        script
            .iter()
            .find(|(at, of, _)| *at == port && *of == family)
            .map(|(_, _, reply)| reply.clone())
    });
    match reply {
        Some(Reply::Box(id)) => {
            let mut info = Info::new(family);
            info.id = id;
            Ok((
                Box::new(Quiet(port)),
                Snapshot::new(info, Capabilities::new(Vec::new())),
            ))
        }
        Some(Reply::Ferrum(api)) => {
            let mut info = Info::new(family);
            info.ferrum_api = Some(api);
            Ok((
                Box::new(Quiet(port)),
                Snapshot::new(info, Capabilities::new(Vec::new())),
            ))
        }
        Some(Reply::Mouse) => Ok((
            Box::new(Quiet(port)),
            Snapshot::new(
                Info::new(family),
                Capabilities::new(vec![Button::Left])
                    .set(Action::Move, Capability::native())
                    .set(Action::Press, Capability::native())
                    .set(Action::Release, Capability::native()),
            ),
        )),
        Some(Reply::Busy) => Err(Error::PortBusy { port }),
        Some(Reply::TooOld) => Err(Error::FirmwareTooOld {
            family,
            found: Firmware::Build(4000),
            needed: Firmware::Build(4085),
        }),
        None => Err(Error::NotFound { tried: vec![port] }),
    }
}

fn medius(candidate: &Candidate, _: Sink) -> Result<(Box<dyn Backend>, Snapshot)> {
    answer(Family::Medius, candidate)
}

fn makcu(candidate: &Candidate, _: Sink) -> Result<(Box<dyn Backend>, Snapshot)> {
    answer(Family::MakcuV4, candidate)
}

const PROBES: &[(Family, Open)] = &[(Family::Medius, medius), (Family::MakcuV4, makcu)];

fn usb(path: &str, vid: u16, pid: u16, serial: Option<&str>) -> Candidate {
    Candidate {
        path: path.into(),
        usb: Some(PortUsb {
            vid,
            pid,
            serial: serial.map(String::from),
        }),
    }
}

fn ch343(path: &str) -> Candidate {
    usb(path, 0x1A86, 0x55D3, None)
}

fn ch343_serial(path: &str, serial: &str) -> Candidate {
    usb(path, 0x1A86, 0x55D3, Some(serial))
}

fn ftdi(path: &str) -> Candidate {
    usb(path, 0x0403, 0x6001, None)
}

fn bare(path: &str) -> Candidate {
    Candidate {
        path: path.into(),
        usb: None,
    }
}

fn first(connect: &Connect, listed: Vec<Candidate>) -> std::result::Result<String, Error> {
    let mut search = connect.search(&|| listed.clone(), PROBES);
    match search.opened.take() {
        Some(opened) => Ok(opened.port),
        None => Err(search.error()),
    }
}

fn found(connect: &Connect, listed: Vec<Candidate>) -> Vec<(String, Family)> {
    connect
        .found_in(&|| listed.clone(), PROBES)
        .into_iter()
        .map(|found| (found.port, found.info.family))
        .collect()
}

#[test]
fn auto_never_opens_a_port_no_family_claims() {
    script(&[
        ("COM4", Family::Medius, Reply::Box(Some(A))),
        ("COM5", Family::Medius, Reply::Box(Some(A))),
    ]);
    let result = found(
        &Connect::auto(),
        vec![ch343("COM3"), ftdi("COM4"), bare("COM5")],
    );
    assert_eq!(result, vec![]);
    assert_eq!(log(), ["Medius COM3", "MakcuV4 COM3"]);
}

#[test]
fn two_boxes_are_both_found_in_port_order() {
    script(&[
        ("COM11", Family::Medius, Reply::Box(Some(A))),
        ("COM3", Family::MakcuV4, Reply::Box(None)),
    ]);
    let result = found(&Connect::auto(), vec![ch343("COM11"), ch343("COM3")]);
    assert_eq!(
        result,
        [
            ("COM3".to_string(), Family::MakcuV4),
            ("COM11".to_string(), Family::Medius)
        ]
    );
}

#[test]
fn ports_compare_by_their_number() {
    let order = |a: &str, b: &str| port_order(Path::new(a), Path::new(b));
    assert!(order("COM3", "COM11").is_lt());
    assert!(order("COM11", "COM3").is_gt());
    assert!(order("/dev/ttyACM9", "/dev/ttyACM10").is_lt());
    assert!(order("/dev/ttyACM1", "/dev/ttyUSB0").is_lt());
    assert!(order("COM3", "COM3").is_eq());
}

#[test]
fn medius_is_asked_first_and_makcu_only_after_it_said_no() {
    script(&[("COM3", Family::MakcuV4, Reply::Box(None))]);
    assert_eq!(
        first(&Connect::auto(), vec![ch343("COM3")]).unwrap(),
        "COM3"
    );
    assert_eq!(log(), ["Medius COM3", "MakcuV4 COM3"]);

    script(&[("COM3", Family::Medius, Reply::Box(Some(A)))]);
    assert_eq!(
        first(&Connect::auto(), vec![ch343("COM3")]).unwrap(),
        "COM3"
    );
    assert_eq!(log(), ["Medius COM3"]);
}

#[test]
fn family_and_id_pick_the_box_they_name() {
    let replies = [
        ("COM3", Family::Medius, Reply::Box(Some(A))),
        ("COM4", Family::MakcuV4, Reply::Box(None)),
    ];
    let listed = || vec![ch343("COM3"), ch343("COM4")];
    script(&replies);
    let by_family = Connect::auto().family(Family::MakcuV4);
    assert_eq!(first(&by_family, listed()).unwrap(), "COM4");
    script(&replies);
    assert_eq!(first(&Connect::auto().id(A), listed()).unwrap(), "COM3");
    script(&replies);
    match first(&Connect::auto().id(B), listed()) {
        Err(Error::NotFound { tried }) => assert_eq!(tried, ["COM3", "COM4"]),
        other => panic!("expected NotFound, got {other:?}"),
    }
    assert_eq!(log(), ["Medius COM3", "shut COM3", "Medius COM4"]);
}

#[test]
fn a_busy_port_is_left_out_of_a_scan_and_is_the_error_from_connect() {
    script(&[
        ("COM3", Family::Medius, Reply::Busy),
        ("COM4", Family::Medius, Reply::Box(Some(A))),
    ]);
    let result = found(&Connect::auto(), vec![ch343("COM3"), ch343("COM4")]);
    assert_eq!(result, [("COM4".to_string(), Family::Medius)]);

    script(&[("COM3", Family::Medius, Reply::Busy)]);
    assert!(matches!(
        first(&Connect::auto(), vec![ch343("COM3")]),
        Err(Error::PortBusy { port }) if port == "COM3"
    ));
}

#[test]
fn a_busy_port_does_not_hide_a_box_on_another_port() {
    script(&[
        ("COM3", Family::Medius, Reply::Busy),
        ("COM4", Family::MakcuV4, Reply::Box(None)),
    ]);
    assert_eq!(
        first(&Connect::auto(), vec![ch343("COM3"), ch343("COM4")]).unwrap(),
        "COM4"
    );
}

#[test]
fn a_box_too_old_is_the_error_when_nothing_answered_and_a_scan_leaves_it_out() {
    script(&[
        ("COM3", Family::Medius, Reply::Busy),
        ("COM4", Family::MakcuV4, Reply::TooOld),
    ]);
    assert!(matches!(
        first(&Connect::auto(), vec![ch343("COM3"), ch343("COM4")]),
        Err(Error::FirmwareTooOld { .. })
    ));

    script(&[
        ("COM4", Family::MakcuV4, Reply::TooOld),
        ("COM5", Family::Medius, Reply::Box(Some(A))),
    ]);
    let result = found(&Connect::auto(), vec![ch343("COM4"), ch343("COM5")]);
    assert_eq!(result, [("COM5".to_string(), Family::Medius)]);
}

#[test]
fn a_scan_closes_every_box_before_asking_the_next_port() {
    script(&[
        ("COM3", Family::Medius, Reply::Box(Some(A))),
        ("COM4", Family::MakcuV4, Reply::Box(None)),
    ]);
    found(&Connect::auto(), vec![ch343("COM3"), ch343("COM4")]);
    assert_eq!(
        log(),
        [
            "Medius COM3",
            "shut COM3",
            "Medius COM4",
            "MakcuV4 COM4",
            "shut COM4"
        ]
    );
}

#[cfg(all(feature = "medius", feature = "makcu", feature = "ferrum"))]
#[test]
fn the_real_probe_table_asks_medius_first_and_ferrum_last() {
    let families: Vec<_> = crate::family::PROBES
        .iter()
        .map(|(family, _)| *family)
        .collect();
    let mut expected = vec![Family::Medius, Family::MakcuV4];
    #[cfg(feature = "ysc8k")]
    expected.push(Family::Ysc);
    expected.push(Family::Ferrum);
    assert_eq!(families, expected);
}

#[test]
fn ysc_listing_is_family_filtered_and_only_uses_the_complete_left_tuple() {
    for family in [
        None,
        Some(Family::Medius),
        Some(Family::MakcuV4),
        Some(Family::Ferrum),
        Some(Family::Ysc),
    ] {
        let connect = family.map_or_else(Connect::auto, |family| Connect::auto().family(family));
        for serial in [
            None,
            Some("TOWMCULEFT"),
            Some("TOWMCURIGHT"),
            Some("TOWMCUIAP"),
            Some("UNKNOWN"),
        ] {
            let candidate = usb("COM7", 0x1a86, 0xfe0c, serial);
            assert_eq!(
                connect.lists(&candidate),
                serial == Some("TOWMCULEFT") && (family.is_none() || family == Some(Family::Ysc))
            );
        }
    }
}

#[test]
fn connect_stops_at_the_first_box() {
    script(&[
        ("COM3", Family::Medius, Reply::Box(Some(A))),
        ("COM4", Family::Medius, Reply::Box(Some(B))),
    ]);
    assert_eq!(
        first(&Connect::auto(), vec![ch343("COM3"), ch343("COM4")]).unwrap(),
        "COM3"
    );
    assert_eq!(log(), ["Medius COM3"]);
}

#[test]
fn a_named_port_is_asked_whatever_its_usb_id() {
    script(&[("COM7", Family::Medius, Reply::Box(Some(A)))]);
    assert_eq!(
        first(&Connect::port("COM7"), vec![ftdi("COM7")]).unwrap(),
        "COM7"
    );
    script(&[("COM7", Family::Medius, Reply::Box(Some(A)))]);
    assert_eq!(first(&Connect::port("COM7"), vec![]).unwrap(), "COM7");
}

#[test]
fn two_boxes_of_one_family_are_told_apart_by_id() {
    let replies = [
        ("COM3", Family::Medius, Reply::Box(Some(A))),
        ("COM4", Family::Medius, Reply::Box(Some(B))),
    ];
    script(&replies);
    let result = found(&Connect::auto(), vec![ch343("COM3"), ch343("COM4")]);
    assert_eq!(result.len(), 2);
    script(&replies);
    assert_eq!(
        first(&Connect::auto().id(B), vec![ch343("COM3"), ch343("COM4")]).unwrap(),
        "COM4"
    );
}

#[test]
fn a_port_gone_since_the_listing_is_skipped() {
    // a port that's gone opens as NotFound in both backends
    script(&[("COM4", Family::MakcuV4, Reply::Box(None))]);
    assert_eq!(
        first(&Connect::auto(), vec![ch343("COM3"), ch343("COM4")]).unwrap(),
        "COM4"
    );
}

#[test]
fn a_usb_serial_id_never_opens_a_port_listing_another_serial() {
    script(&[
        (
            "COM3",
            Family::MakcuV4,
            Reply::Box(Some(StableId::UsbSerial("S1".into()))),
        ),
        (
            "COM4",
            Family::MakcuV4,
            Reply::Box(Some(StableId::UsbSerial("S2".into()))),
        ),
    ]);
    let wanted = Connect::auto().id(StableId::UsbSerial("S2".into()));
    let listed = vec![
        ch343_serial("COM3", "S1"),
        ch343_serial("COM4", "S2"),
        ch343("COM5"),
    ];
    assert_eq!(first(&wanted, listed).unwrap(), "COM4");
    assert_eq!(log(), ["MakcuV4 COM4"]);
}

#[test]
fn each_id_kind_only_asks_its_family() {
    script(&[]);
    let _ = first(&Connect::auto().id(A), vec![ch343("COM3")]);
    assert_eq!(log(), ["Medius COM3"]);
    script(&[]);
    let serial = Connect::auto().id(StableId::UsbSerial("S1".into()));
    let _ = first(&serial, vec![ch343_serial("COM3", "S1")]);
    assert_eq!(log(), ["MakcuV4 COM3"]);
}

#[test]
fn a_busy_port_is_not_asked_by_the_other_families() {
    script(&[("COM3", Family::Medius, Reply::Busy)]);
    let _ = first(&Connect::auto(), vec![ch343("COM3")]);
    assert_eq!(log(), ["Medius COM3"]);
}

#[test]
fn the_most_useful_error_wins() {
    let busy = || Error::PortBusy {
        port: "COM4".into(),
    };
    let denied = || Error::PermissionDenied {
        port: "COM5".into(),
    };
    let too_old = || Error::FirmwareTooOld {
        family: Family::MakcuV4,
        found: Firmware::Build(4000),
        needed: Firmware::Build(4085),
    };
    let other = || Error::WrongFamily {
        port: "COM6".into(),
        expected: Family::Medius,
        found: None,
    };
    let error = |skipped: Vec<Error>| {
        Search {
            skipped,
            tried: vec!["COM3".into()],
            ..Search::default()
        }
        .error()
    };
    assert!(matches!(
        error(vec![other(), denied(), busy(), too_old()]),
        Error::FirmwareTooOld { .. }
    ));
    assert!(matches!(
        error(vec![other(), denied(), busy()]),
        Error::PortBusy { .. }
    ));
    assert!(matches!(
        error(vec![other(), denied()]),
        Error::PermissionDenied { .. }
    ));
    assert!(matches!(error(vec![other()]), Error::WrongFamily { .. }));
    assert!(matches!(error(vec![]), Error::NotFound { tried } if tried == ["COM3"]));
}

#[test]
fn a_scan_of_a_busy_box_says_busy_and_with_no_ports_is_empty() {
    script(&[("COM3", Family::Medius, Reply::Busy)]);
    assert!(matches!(
        &Connect::auto().scan_in(&|| vec![ch343("COM3")], PROBES)[0].result,
        Err(Error::PortBusy { .. })
    ));
    assert!(Connect::auto().scan_in(&Vec::new, PROBES).is_empty());
    script(&[]);
    assert_eq!(found(&Connect::auto(), vec![ftdi("COM4")]), vec![]);
    assert_eq!(found(&Connect::auto(), vec![ch343("COM3")]), vec![]);
}

#[test]
fn a_named_port_with_another_box_is_wrong_box() {
    script(&[("COM3", Family::Medius, Reply::Box(Some(A)))]);
    match first(&Connect::port("COM3").id(B), vec![ch343("COM3")]) {
        Err(Error::WrongBox {
            port,
            expected,
            found,
        }) => {
            assert_eq!(port, "COM3");
            assert_eq!(expected, B);
            assert_eq!(found, Some(A));
        }
        other => panic!("expected WrongBox, got {other:?}"),
    }
    assert_eq!(log(), ["Medius COM3", "shut COM3"]);
}

#[test]
fn a_named_port_gone_while_it_was_asked_is_not_found_not_wrong_family() {
    script(&[]);
    let listings = std::cell::Cell::new(0);
    let ports = || {
        listings.set(listings.get() + 1);
        if listings.get() == 1 {
            vec![ch343("COM3")]
        } else {
            vec![]
        }
    };
    let makcu_there = Connect::port("COM3").family(Family::MakcuV4);
    let mut search = makcu_there.search(&ports, PROBES);
    assert!(search.opened.take().is_none());
    assert!(matches!(search.error(), Error::NotFound { .. }));
}

#[test]
fn open_starts_the_device_on_the_first_box_only() {
    script(&[
        ("COM3", Family::Medius, Reply::Box(Some(A))),
        ("COM4", Family::Medius, Reply::Box(Some(B))),
    ]);
    let listed = vec![ch343("COM4"), ch343("COM3")];
    let device = Connect::auto().open_in(&|| listed.clone(), PROBES).unwrap();
    assert_eq!(device.snapshot().info.id, Some(A));
    drop(device);
    assert_eq!(log(), ["Medius COM3", "shut COM3"]);
}

#[test]
fn a_named_port_with_another_id_of_either_kind_is_wrong_box() {
    let s1 = StableId::UsbSerial("S1".into());
    let s2 = StableId::UsbSerial("S2".into());
    script(&[("COM3", Family::MakcuV4, Reply::Box(Some(s1.clone())))]);
    let by_serial = Connect::port("COM3").id(s2);
    assert!(matches!(
        first(&by_serial, vec![ch343_serial("COM3", "S1")]),
        Err(Error::WrongBox { found: Some(found), .. }) if found == s1
    ));
    script(&[("COM3", Family::MakcuV4, Reply::Box(Some(s1.clone())))]);
    assert!(matches!(
        first(&Connect::port("COM3").id(A), vec![ch343_serial("COM3", "S1")]),
        Err(Error::WrongBox { found: Some(found), .. }) if found == s1
    ));
}

#[test]
fn a_named_port_with_another_family_is_wrong_family_and_a_missing_one_not_found() {
    script(&[("COM3", Family::Medius, Reply::Box(Some(A)))]);
    let makcu_there = Connect::port("COM3").family(Family::MakcuV4);
    assert!(matches!(
        first(&makcu_there, vec![ch343("COM3")]),
        Err(Error::WrongFamily {
            expected: Family::MakcuV4,
            ..
        })
    ));
    assert!(matches!(
        first(&makcu_there, vec![]),
        Err(Error::NotFound { .. })
    ));
}

#[test]
fn a_scan_takes_the_filters() {
    script(&[
        ("COM3", Family::Medius, Reply::Box(Some(A))),
        ("COM4", Family::MakcuV4, Reply::Box(None)),
    ]);
    let result = found(
        &Connect::auto().family(Family::MakcuV4),
        vec![ch343("COM3"), ch343("COM4")],
    );
    assert_eq!(result, [("COM4".to_string(), Family::MakcuV4)]);
    assert!(log().iter().all(|entry| !entry.starts_with("Medius")));
}

/// these need a Medius and a MAKCU plugged in, and keep your hands off the mouse while they run:
/// ANYBOX_MEDIUS=COM18 ANYBOX_MAKCU=COM11 cargo test connect::tests::hardware -- --ignored --nocapture --test-threads 1
#[cfg(windows)]
#[allow(unsafe_code)]
mod hardware {
    use std::time::{Duration, Instant};

    use crate::capability::{Family, StableId};
    use crate::connect::{Connect, scan};
    use crate::error::Error;

    #[link(name = "user32")]
    unsafe extern "system" {
        fn GetCursorPos(point: *mut [i32; 2]) -> i32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn Beep(frequency: u32, millis: u32) -> i32;
    }

    /// a high beep says start, a low one says do the next thing, two low ones say done. blocks while it sounds
    fn beep(high: bool) {
        // safety: Beep takes two plain numbers
        unsafe {
            Beep(if high { 1500 } else { 500 }, 400);
        }
    }

    fn done() {
        beep(false);
        std::thread::sleep(Duration::from_millis(150));
        beep(false);
    }

    fn cursor() -> [i32; 2] {
        let mut point = [0; 2];
        // safety: GetCursorPos writes one POINT, two i32
        assert_ne!(unsafe { GetCursorPos(&mut point) }, 0);
        point
    }

    fn port(family: Family) -> String {
        let name = match family {
            Family::Medius => "ANYBOX_MEDIUS",
            _ => "ANYBOX_MAKCU",
        };
        std::env::var(name).unwrap_or_else(|_| panic!("set {name} to the box's port"))
    }

    fn listed(port: &str) -> bool {
        serialport::available_ports()
            .unwrap_or_default()
            .iter()
            .any(|info| info.port_name == port)
    }

    fn print_usb_strings() {
        for info in serialport::available_ports().unwrap() {
            if let serialport::SerialPortType::UsbPort(usb) = info.port_type {
                println!(
                    "{} {:04X}:{:04X} manufacturer {:?} product {:?} serial {:?}",
                    info.port_name,
                    usb.vid,
                    usb.pid,
                    usb.manufacturer,
                    usb.product,
                    usb.serial_number
                );
            }
        }
    }

    fn both() -> Vec<(String, Family, Option<StableId>)> {
        let mut boxes: Vec<_> = scan()
            .into_iter()
            .filter_map(|scanned| scanned.result.ok())
            .map(|found| (found.port, found.info.family, found.info.id))
            .collect();
        boxes.sort_by_key(|(_, family, _)| *family != Family::Medius);
        boxes
    }

    #[test]
    #[ignore]
    fn twenty_rounds_find_both_boxes_and_never_move_the_cursor() {
        print_usb_strings();
        let first = both();
        println!("{first:?}");
        assert_eq!(first.len(), 2, "expected both boxes");
        assert_eq!(first[0].0, port(Family::Medius));
        assert_eq!(first[1].0, port(Family::MakcuV4));
        assert_eq!(first[1].1, Family::MakcuV4);
        assert!(
            first.iter().all(|(_, _, id)| id.is_some()),
            "both boxes should have an id"
        );
        for round in 1..=20 {
            let before = cursor();
            let started = Instant::now();
            let boxes = both();
            let finding = started.elapsed();
            assert_eq!(boxes, first, "round {round}");
            // right after the scan closed them both they open again, so nothing is still busy
            let mut drops = Vec::new();
            for (_, family, id) in &boxes {
                let connect = match id {
                    Some(id) => Connect::auto().id(id.clone()),
                    None => Connect::port(port(*family)),
                };
                let device = connect
                    .family(*family)
                    .open()
                    .unwrap_or_else(|error| panic!("round {round}: {error}"));
                let dropping = Instant::now();
                drop(device);
                drops.push(dropping.elapsed());
            }
            assert_eq!(cursor(), before, "round {round} moved the cursor");
            println!("round {round}: scan {finding:?}, drops {drops:?}");
        }
    }

    #[test]
    #[ignore]
    fn the_makcu_probe_leaves_a_medius_alone() {
        let medius = port(Family::Medius);
        let before = Connect::port(&medius)
            .open()
            .unwrap_or_else(|error| panic!("{error}"));
        let identity = before.snapshot().info.clone();
        drop(before);
        let started = Instant::now();
        let wrong = Connect::port(&medius).family(Family::MakcuV4).open();
        println!("makcu probe on the medius took {:?}", started.elapsed());
        assert!(matches!(wrong, Err(Error::WrongFamily { .. })), "{wrong:?}");
        let after = Connect::port(&medius)
            .open()
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(after.snapshot().info, identity);
    }

    #[test]
    #[ignore]
    fn connect_by_family_and_by_id_opens_the_right_box() {
        for family in [Family::Medius, Family::MakcuV4] {
            let device = Connect::auto()
                .family(family)
                .open()
                .unwrap_or_else(|error| panic!("{error}"));
            let info = device.snapshot().info.clone();
            assert_eq!(info.family, family);
            drop(device);
            if let Some(id) = info.id {
                let device = Connect::auto()
                    .id(id.clone())
                    .open()
                    .unwrap_or_else(|error| panic!("{error}"));
                assert_eq!(device.snapshot().info.id, Some(id));
            }
        }
    }

    #[test]
    #[ignore]
    fn a_held_box_is_busy_and_a_scan_still_gives_the_other() {
        let held = Connect::port(port(Family::Medius))
            .family(Family::Medius)
            .open()
            .unwrap_or_else(|error| panic!("{error}"));
        let boxes = both();
        assert_eq!(boxes.len(), 1, "{boxes:?}");
        assert_eq!(boxes[0].1, Family::MakcuV4);
        let busy = Connect::auto().family(Family::Medius).open();
        assert!(matches!(busy, Err(Error::PortBusy { .. })), "{busy:?}");
        drop(held);
    }

    #[test]
    #[ignore]
    fn afterwards_each_box_still_answers_its_own_probe() {
        for family in [Family::Medius, Family::MakcuV4] {
            let started = Instant::now();
            let device = Connect::port(port(family))
                .family(family)
                .open()
                .unwrap_or_else(|error| panic!("{error}"));
            println!(
                "{family:?} opened in {:?}: {:?}",
                started.elapsed(),
                device.snapshot().info
            );
        }
    }

    fn boxes() -> [(Family, String); 2] {
        [
            (Family::Medius, port(Family::Medius)),
            (Family::MakcuV4, port(Family::MakcuV4)),
        ]
    }

    fn unplug(plug: u32) {
        println!("UNPLUG BOTH ({plug}/3)");
        while boxes().iter().any(|(_, port)| listed(port)) {
            std::thread::sleep(Duration::from_millis(50));
        }
        println!("unplugged, plug them back in");
    }

    /// needs you at the PC. unplug both boxes when it says so, wait a second and plug them back in, three times
    /// over. each box only gets asked with its own family, from the moment its own port shows up
    #[test]
    #[ignore]
    fn cold_plug() {
        for plug in 1..=3 {
            unplug(plug);
            std::thread::scope(|scope| {
                for (family, port) in boxes() {
                    scope.spawn(move || {
                        while !listed(&port) {
                            std::thread::sleep(Duration::from_millis(20));
                        }
                        let appeared = Instant::now();
                        for tries in 1.. {
                            match Connect::port(&port).family(family).open() {
                                Ok(_) => {
                                    println!(
                                        "{family:?} answered {:?} after its port appeared, try {tries}",
                                        appeared.elapsed()
                                    );
                                    return;
                                }
                                Err(error) if appeared.elapsed() < Duration::from_secs(30) => {
                                    println!(
                                        "{family:?} try {tries} failed {:?} after: {error}",
                                        appeared.elapsed()
                                    );
                                }
                                Err(error) => panic!("{family:?} never answered: {error}"),
                            }
                        }
                    });
                }
            });
        }
        println!("DONE");
    }

    /// needs you at the PC, same as `cold_plug`, but here `scan()` with no filters runs from the moment the MAKCU
    /// port shows up
    #[test]
    #[ignore]
    fn cold_plug_with_scan() {
        let makcu = port(Family::MakcuV4);
        for plug in 1..=3 {
            unplug(plug);
            while !listed(&makcu) {
                std::thread::sleep(Duration::from_millis(20));
            }
            let appeared = Instant::now();
            for tries in 1.. {
                let found: Vec<_> = scan()
                    .into_iter()
                    .filter_map(|scanned| scanned.result.ok())
                    .collect();
                let families: Vec<_> = found.iter().map(|found| found.info.family).collect();
                println!(
                    "try {tries}, {:?} after the MAKCU port appeared: {families:?}",
                    appeared.elapsed()
                );
                if families.len() == 2 {
                    break;
                }
                assert!(
                    appeared.elapsed() < Duration::from_secs(30),
                    "never found both"
                );
            }
        }
        println!("DONE");
    }

    /// needs you at the PC with Notepad open and focused. on each box in turn it types `a`, then `A` with shift,
    /// then taps `b`, then holds `c` for 700 ms (so Windows starts repeating it) and drops the device, which has to
    /// stop the repeat. so each box should leave `aAbccc...` with a short run of c and nothing typed after DONE
    #[test]
    #[ignore]
    fn keys_type_into_the_focused_window() {
        use crate::key::Key;
        for family in [Family::Medius, Family::MakcuV4] {
            let device = Connect::port(port(family))
                .family(family)
                .open()
                .unwrap_or_else(|error| panic!("{error}"));
            println!("{family:?} typing");
            device.press_key(Key::A).unwrap();
            device.release_key(Key::A).unwrap();
            device.press_key(Key::LEFT_SHIFT).unwrap();
            device.press_key(Key::A).unwrap();
            device.release_key(Key::A).unwrap();
            device.release_key(Key::LEFT_SHIFT).unwrap();
            device.tap_key(Key::B).unwrap();
            device.press_key(Key::C).unwrap();
            std::thread::sleep(Duration::from_millis(700));
            let dropping = Instant::now();
            drop(device);
            println!("{family:?} dropped in {:?}", dropping.elapsed());
            device_enter(family);
        }
        println!("DONE");
    }

    /// a new line in Notepad between the boxes, typed by the same box so nothing else touches the keyboard
    fn device_enter(family: Family) {
        use crate::key::Key;
        let device = Connect::port(port(family)).family(family).open().unwrap();
        device.tap_key(Key::ENTER).unwrap();
    }

    /// needs you at the PC. when it says TYPE on a box, type on the keyboard behind that box for 10 s (letters,
    /// shift, then hold 7 letters at once). it prints every key event it gets, and an F13 it taps itself must NOT
    /// come back as an event
    #[test]
    #[ignore]
    fn key_events_from_the_keyboard_behind_each_box() {
        use crate::event::Event;
        use crate::key::Key;
        let only: Option<Family> =
            std::env::var("ANYBOX_ONLY")
                .ok()
                .map(|name| match name.as_str() {
                    "medius" => Family::Medius,
                    _ => Family::MakcuV4,
                });
        for family in [Family::Medius, Family::MakcuV4] {
            if only.is_some_and(|only| only != family) {
                continue;
            }
            let device = Connect::port(port(family))
                .family(family)
                .open()
                .unwrap_or_else(|error| panic!("{error}"));
            println!(
                "{family:?} key events: {:?}",
                device
                    .snapshot()
                    .capabilities
                    .get(crate::capability::Action::KeyEvents)
                    .execution
            );
            let events = device.events();
            std::thread::sleep(Duration::from_millis(300));
            device.tap_key(Key(0x68)).unwrap();
            println!("TYPE on the keyboard behind {family:?} now");
            let until = Instant::now() + Duration::from_secs(30);
            let mut count = 0;
            while Instant::now() < until {
                match events.recv_timeout(Duration::from_millis(100)) {
                    Ok(Event::Key {
                        key, pressed, at, ..
                    }) => {
                        count += 1;
                        assert_ne!(key, Key(0x68), "the injected F13 came back as an event");
                        println!(
                            "  {key} {} box {:?}",
                            if pressed { "down" } else { "up" },
                            at.box_time.map(|time| time.micros)
                        );
                    }
                    Ok(Event::Button { .. } | Event::Motion { .. }) | Err(_) => {}
                    Ok(other) => println!("  {other:?}"),
                }
            }
            println!("{family:?}: {count} key events, STOP typing");
        }
        println!("DONE");
    }

    /// needs you at the PC and a Medius whose device has a media key interface. it turns the volume up twice and
    /// then down twice, so watch the Windows volume overlay
    #[test]
    #[ignore]
    fn media_keys_move_the_volume() {
        use crate::key::MediaKey;
        let device = Connect::port(port(Family::Medius))
            .family(Family::Medius)
            .open()
            .unwrap_or_else(|error| panic!("{error}"));
        println!(
            "media: {:?}",
            device
                .snapshot()
                .capabilities
                .get(crate::capability::Action::MediaTap)
                .execution
        );
        println!("WATCH the volume");
        for key in [
            MediaKey::VOLUME_UP,
            MediaKey::VOLUME_UP,
            MediaKey::VOLUME_DOWN,
            MediaKey::VOLUME_DOWN,
        ] {
            device.tap_media(key).unwrap();
            std::thread::sleep(Duration::from_millis(1500));
        }
        println!("DONE, STOP");
    }

    /// needs you at the PC, one box at a time (ANYBOX_LOCK_BOX=medius or makcu). each phase starts with PHASE,
    /// lasts 10 s and counts the physical events that still come in while the lock is on
    #[test]
    #[ignore]
    fn locks_with_you() {
        use crate::capability::{Action, Axis, Button, Direction};
        use crate::event::Event;
        use crate::key::Key;
        let family = match std::env::var("ANYBOX_LOCK_BOX").as_deref() {
            Ok("makcu") => Family::MakcuV4,
            _ => Family::Medius,
        };
        let device = Connect::port(port(family))
            .family(family)
            .open()
            .unwrap_or_else(|error| panic!("{error}"));
        let events = device.events();
        let capabilities = device.snapshot().capabilities.clone();
        let phase = |name: &str| {
            println!("PHASE {name}");
            let until = Instant::now() + Duration::from_secs(10);
            let (mut buttons, mut keys, mut motion) = (0, 0, 0);
            while Instant::now() < until {
                match events.recv_timeout(Duration::from_millis(100)) {
                    Ok(Event::Button { .. }) => buttons += 1,
                    Ok(Event::Key { .. }) => keys += 1,
                    Ok(Event::Motion { .. }) => motion += 1,
                    _ => {}
                }
            }
            println!("  events while locked: {buttons} button, {keys} key, {motion} motion");
        };
        let keys_only = std::env::var("ANYBOX_LOCK_ONLY").as_deref() == Ok("keys");
        if !keys_only && capabilities.get(Action::LockButton).is_supported() {
            device.lock_button(Button::Left).unwrap();
            phase("left button locked");
            device.unlock_button(Button::Left).unwrap();
            device.lock_axis(Axis::X, Direction::Positive).unwrap();
            phase("X positive locked");
            device.unlock_axis(Axis::X, Direction::Positive).unwrap();
            device.lock_axis(Axis::Y, Direction::Positive).unwrap();
            phase("Y positive locked");
            device.unlock_axis(Axis::Y, Direction::Positive).unwrap();
            if capabilities.get(Action::ScaleAxis).is_supported() {
                device.scale_axis(Axis::Y, Direction::Both, 30).unwrap();
                phase("Y scaled to 30");
                device.unlock_axis(Axis::Y, Direction::Both).unwrap();
            }
        }
        let mouse_only = std::env::var("ANYBOX_LOCK_ONLY").as_deref() == Ok("mouse");
        if !mouse_only && capabilities.get(Action::LockKey).is_supported() {
            device.lock_key(Key::A).unwrap();
            phase("key A locked");
            device.unlock_key(Key::A).unwrap();
        }
        // one left locked on purpose, drop has to give it back
        if capabilities.get(Action::LockButton).is_supported() {
            device.lock_button(Button::Left).unwrap();
        }
        drop(events);
        drop(device);
        println!("DONE, STOP");
    }

    /// needs you at the PC. hold the physical left button at HOLD and anybox locks it, then let go at RELEASE and
    /// move the mouse. it tells whether the PC still thinks the button is down after a lock lands on a held button
    #[test]
    #[ignore]
    fn a_lock_on_a_held_button() {
        use crate::capability::Button;
        let family = match std::env::var("ANYBOX_LOCK_BOX").as_deref() {
            Ok("makcu") => Family::MakcuV4,
            _ => Family::Medius,
        };
        let device = Connect::port(port(family))
            .family(family)
            .open()
            .unwrap_or_else(|error| panic!("{error}"));
        eprintln!("HOLD");
        beep(true);
        std::thread::sleep(Duration::from_secs(4));
        let started = Instant::now();
        device.lock_button(Button::Left).unwrap();
        eprintln!("locked in {:?}", started.elapsed());
        std::thread::sleep(Duration::from_secs(3));
        eprintln!("RELEASE");
        beep(false);
        std::thread::sleep(Duration::from_secs(6));
        device.unlock_button(Button::Left).unwrap();
        eprintln!("DONE, STOP");
        done();
    }

    /// needs you at the PC and a Medius with a mouse that has pan. move the mouse during the first 5 s, then watch
    /// a page that scrolls sideways while anybox pans right 5 notches and back. it prints the polling rate and the
    /// identity the box shows the PC, both read through medius()
    #[cfg(feature = "medius")]
    #[test]
    #[ignore]
    fn pan_and_what_medius_says_about_the_mouse() {
        let device = Connect::port(port(Family::Medius))
            .family(Family::Medius)
            .open()
            .unwrap_or_else(|error| panic!("{error}"));
        let crate_device = device.medius().unwrap();
        println!("identity {:?}", crate_device.device_info());
        println!("rate before moving {:?}", crate_device.query_rate());
        eprintln!("MOVE");
        beep(true);
        std::thread::sleep(Duration::from_secs(5));
        println!("rate after moving {:?}", crate_device.query_rate());
        eprintln!("WATCH");
        beep(false);
        for notches in [1, 1, 1, 1, 1, -1, -1, -1, -1, -1] {
            device.pan(notches).unwrap();
            std::thread::sleep(Duration::from_millis(400));
        }
        eprintln!("DONE, STOP");
        done();
    }
}

#[test]
fn can_skips_a_box_that_cant_and_opens_the_next() {
    script(&[
        ("COM3", Family::Medius, Reply::Box(Some(A))),
        ("COM4", Family::Medius, Reply::Mouse),
    ]);
    let listed = vec![ch343("COM3"), ch343("COM4")];
    let connect = Connect::auto().can(Action::Move);
    assert_eq!(first(&connect, listed.clone()).unwrap(), "COM4");
    assert_eq!(log(), ["Medius COM3", "shut COM3", "Medius COM4"]);
    let device = connect.open_in(&|| listed.clone(), PROBES).unwrap();
    assert_eq!(device.snapshot().info.port, "COM4");
}

#[test]
fn can_counts_a_click_the_pc_builds() {
    script(&[("COM3", Family::Medius, Reply::Mouse)]);
    let connect = Connect::auto().can(Action::Click);
    assert_eq!(first(&connect, vec![ch343("COM3")]).unwrap(), "COM3");
}

#[test]
fn a_scan_with_can_lists_only_the_boxes_that_can_and_says_what_they_can() {
    script(&[
        ("COM3", Family::Medius, Reply::Box(Some(A))),
        ("COM4", Family::Medius, Reply::Mouse),
    ]);
    let listed = vec![ch343("COM3"), ch343("COM4")];
    let boxes = Connect::auto()
        .can(Action::Move)
        .found_in(&|| listed.clone(), PROBES);
    assert_eq!(boxes.len(), 1);
    assert_eq!(boxes[0].port, "COM4");
    assert_eq!(boxes[0].info.port, "COM4");
    assert!(boxes[0].capabilities.get(Action::Move).is_supported());
    assert!(boxes[0].capabilities.get(Action::Click).is_supported());
    script(&[("COM3", Family::Medius, Reply::Box(Some(A)))]);
    let none = Connect::auto()
        .can(Action::Move)
        .found_in(&|| vec![ch343("COM3")], PROBES);
    assert!(none.is_empty());
}

#[test]
fn when_no_box_can_the_error_names_each_one_and_what_it_couldnt_do() {
    script(&[
        ("COM3", Family::Medius, Reply::Box(Some(A))),
        ("COM4", Family::Medius, Reply::Mouse),
    ]);
    let error = first(
        &Connect::auto().can(Action::Move).can(Action::KeyPress),
        vec![ch343("COM3"), ch343("COM4")],
    )
    .unwrap_err();
    match &error {
        Error::NoBoxCan { boxes } => {
            assert_eq!(boxes.len(), 2);
            assert_eq!(boxes[0].1, vec![Action::Move, Action::KeyPress]);
            assert_eq!(boxes[1].1, vec![Action::KeyPress]);
        }
        other => panic!("{other}"),
    }
    assert!(error.to_string().contains("COM4"), "{error}");
}

#[test]
fn a_busy_port_still_wins_over_boxes_that_cant() {
    script(&[
        ("COM3", Family::Medius, Reply::Busy),
        ("COM4", Family::Medius, Reply::Box(Some(A))),
    ]);
    let error = first(
        &Connect::auto().can(Action::Move),
        vec![ch343("COM3"), ch343("COM4")],
    )
    .unwrap_err();
    assert!(matches!(error, Error::PortBusy { .. }), "{error}");
}

fn ferrum(candidate: &Candidate, _: Sink) -> Result<(Box<dyn Backend>, Snapshot)> {
    answer(Family::Ferrum, candidate)
}

const WITH_FERRUM: &[(Family, Open)] = &[
    (Family::Medius, medius),
    (Family::MakcuV4, makcu),
    (Family::Ferrum, ferrum),
];

fn cp210x(path: &str) -> Candidate {
    usb(path, 0x10C4, 0xEA60, None)
}

#[test]
fn a_ferrum_is_never_asked_unless_someone_asks_for_one() {
    script(&[("COM7", Family::Ferrum, Reply::Box(None))]);
    let listed = vec![ch343("COM3"), cp210x("COM7"), bare("COM8")];
    let seen = Connect::auto().found_in(&|| listed.clone(), WITH_FERRUM);
    assert!(seen.is_empty());
    assert_eq!(log(), ["Medius COM3", "MakcuV4 COM3"]);
}

#[test]
fn asking_for_a_ferrum_skips_the_ch343_ports_and_asks_the_rest() {
    script(&[("COM7", Family::Ferrum, Reply::Box(None))]);
    let listed = vec![ch343("COM3"), cp210x("COM7"), ftdi("COM9"), bare("COM8")];
    let seen = Connect::auto()
        .family(Family::Ferrum)
        .found_in(&|| listed.clone(), WITH_FERRUM);
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].port, "COM7");
    // the Ferrum app's virtual port only exists on Windows, so a port with no USB id is only asked there
    let expected: &[&str] = if cfg!(windows) {
        &["Ferrum COM7", "shut COM7", "Ferrum COM8", "Ferrum COM9"]
    } else {
        &["Ferrum COM7", "shut COM7", "Ferrum COM9"]
    };
    assert_eq!(log(), expected);
}

#[test]
fn a_named_port_asks_a_ferrum_after_the_others() {
    script(&[("COM7", Family::Ferrum, Reply::Box(None))]);
    let mut search = Connect::port("COM7").search(&|| vec![cp210x("COM7")], WITH_FERRUM);
    assert_eq!(search.opened.take().unwrap().port, "COM7");
    assert_eq!(log(), ["Medius COM7", "MakcuV4 COM7", "Ferrum COM7"]);
}

#[test]
fn an_id_never_asks_a_ferrum_since_it_has_none() {
    script(&[("COM7", Family::Ferrum, Reply::Box(None))]);
    let mut search = Connect::auto()
        .family(Family::Ferrum)
        .id(A)
        .search(&|| vec![cp210x("COM7")], WITH_FERRUM);
    assert!(search.opened.take().is_none());
    assert!(log().is_empty());
}

#[test]
fn asking_for_one_ferrum_api_skips_the_other_with_a_reason() {
    script(&[
        ("COM7", Family::Ferrum, Reply::Ferrum(FerrumApi::Legacy)),
        ("COM8", Family::Ferrum, Reply::Ferrum(FerrumApi::Software)),
    ]);
    let listed = vec![cp210x("COM7"), cp210x("COM8")];
    let connect = Connect::auto()
        .family(Family::Ferrum)
        .ferrum_api(FerrumApi::Software);
    let mut search = connect.search(&|| listed.clone(), WITH_FERRUM);
    assert_eq!(search.opened.take().unwrap().port, "COM8");
    assert_eq!(log(), ["Ferrum COM7", "shut COM7", "Ferrum COM8"]);

    script(&[("COM7", Family::Ferrum, Reply::Ferrum(FerrumApi::Legacy))]);
    let error = Connect::port("COM7")
        .ferrum_api(FerrumApi::Software)
        .search(&|| vec![cp210x("COM7")], WITH_FERRUM)
        .error();
    assert!(
        matches!(
            error,
            Error::WrongFerrumApi {
                expected: FerrumApi::Software,
                found: FerrumApi::Legacy,
                ..
            }
        ),
        "{error:?}"
    );
    assert!(
        error.to_string().contains("start the Ferrum app"),
        "{error}"
    );
}

#[test]
fn a_ferrum_search_on_windows_only_takes_the_app_unless_legacy_is_asked_for() {
    script(&[("COM7", Family::Ferrum, Reply::Ferrum(FerrumApi::Legacy))]);
    let listed = vec![cp210x("COM7")];
    let mut search = Connect::auto()
        .family(Family::Ferrum)
        .search(&|| listed.clone(), WITH_FERRUM);
    assert_eq!(search.opened.take().is_some(), !cfg!(windows));

    script(&[("COM7", Family::Ferrum, Reply::Ferrum(FerrumApi::Legacy))]);
    let mut search = Connect::auto()
        .family(Family::Ferrum)
        .ferrum_api(FerrumApi::Legacy)
        .search(&|| listed.clone(), WITH_FERRUM);
    assert!(search.opened.take().is_some());

    // naming the port is asking for whatever is on it
    script(&[("COM7", Family::Ferrum, Reply::Ferrum(FerrumApi::Legacy))]);
    let mut search = Connect::port("COM7").search(&|| listed.clone(), WITH_FERRUM);
    assert!(search.opened.take().is_some());
}

#[test]
fn a_ferrum_search_never_lists_a_ysc_port() {
    script(&[("COM9", Family::Ferrum, Reply::Ferrum(FerrumApi::Software))]);
    let listed = vec![usb("COM9", 0x1A86, 0xFE0C, Some("TOWMCUIAP"))];
    let seen = Connect::auto()
        .family(Family::Ferrum)
        .found_in(&|| listed.clone(), WITH_FERRUM);
    assert!(seen.is_empty());
    assert!(log().is_empty());
}

#[test]
fn candidates_are_the_ports_a_search_would_look_at_in_port_order() {
    let listed = vec![
        ch343_serial("COM11", "A1B2C3D4E5"),
        ftdi("COM4"),
        usb("COM5", 0x1A86, 0xFE0C, Some("TOWMCULEFT")),
        usb("COM6", 0x1A86, 0xFE0C, Some("TOWMCURIGHT")),
        bare("COM7"),
        ch343("COM3"),
    ];
    let candidates = |connect: Connect| connect.candidates_in(&|| listed.clone());
    assert_eq!(candidates(Connect::auto()), ["COM3", "COM5", "COM11"]);
    assert_eq!(
        candidates(Connect::auto().family(Family::Ysc)),
        ["COM5", "COM3", "COM11"]
    );
    assert_eq!(
        candidates(Connect::auto().family(Family::MakcuV4)),
        ["COM3", "COM11"]
    );
    let ferrum: &[&str] = if cfg!(windows) {
        &["COM4", "COM7"]
    } else {
        &["COM4"]
    };
    assert_eq!(candidates(Connect::auto().family(Family::Ferrum)), ferrum);
    assert_eq!(
        candidates(Connect::auto().id(StableId::UsbSerial("A1B2C3D4E5".into()))),
        ["COM11"]
    );
    assert_eq!(candidates(Connect::port("COM9")), ["COM9"]);
}

fn ysc(candidate: &Candidate, _: Sink) -> Result<(Box<dyn Backend>, Snapshot)> {
    answer(Family::Ysc, candidate)
}

const WITH_YSC: &[(Family, Open)] = &[
    (Family::Medius, medius),
    (Family::MakcuV4, makcu),
    (Family::Ysc, ysc),
];

#[test]
fn only_a_ysc_search_sweeps_a_port_that_isnt_a_v2_one() {
    let listed = vec![
        ch343("COM3"),
        usb("COM5", 0x1A86, 0xFE0C, Some("TOWMCULEFT")),
    ];
    let asked = |connect: Connect| {
        script(&[]);
        let _ = connect.found_in(&|| listed.clone(), WITH_YSC);
        log()
            .into_iter()
            .filter(|line| line.starts_with("Ysc"))
            .collect::<Vec<_>>()
    };
    assert_eq!(asked(Connect::auto()), ["Ysc COM5"]);
    assert!(asked(Connect::port("COM3")).is_empty());
    assert!(asked(Connect::port("COM9")).is_empty());
    assert_eq!(
        asked(Connect::auto().family(Family::Ysc)),
        ["Ysc COM5", "Ysc COM3"]
    );
    assert_eq!(
        asked(Connect::port("COM9").family(Family::Ysc)),
        ["Ysc COM9"]
    );
}

#[test]
fn a_scan_gives_every_port_its_own_outcome_in_port_order() {
    script(&[
        ("COM3", Family::Medius, Reply::Busy),
        ("COM4", Family::MakcuV4, Reply::TooOld),
        ("COM6", Family::Medius, Reply::Box(Some(A))),
    ]);
    let scanned = Connect::auto().scan_in(
        &|| vec![ch343("COM6"), ch343("COM3"), ch343("COM5"), ch343("COM4")],
        PROBES,
    );
    let ports: Vec<&str> = scanned.iter().map(|s| s.port.as_str()).collect();
    assert_eq!(ports, ["COM3", "COM4", "COM5", "COM6"]);
    assert!(matches!(&scanned[0].result, Err(Error::PortBusy { port }) if port == "COM3"));
    assert!(matches!(
        &scanned[1].result,
        Err(Error::FirmwareTooOld { .. })
    ));
    assert!(matches!(&scanned[2].result, Err(Error::NotFound { tried }) if tried == &["COM5"]));
    assert!(
        matches!(&scanned[3].result, Ok(found) if found.port == "COM6" && found.info.family == Family::Medius)
    );
    assert!(log().contains(&"shut COM6".to_string()));
}

#[test]
fn a_scan_says_which_box_fell_short_of_can_on_its_own_port() {
    script(&[
        ("COM3", Family::Medius, Reply::Box(Some(A))),
        ("COM4", Family::Medius, Reply::Mouse),
    ]);
    let scanned = Connect::auto()
        .can(Action::KeyPress)
        .scan_in(&|| vec![ch343("COM3"), ch343("COM4")], PROBES);
    assert!(
        matches!(&scanned[0].result, Err(Error::NoBoxCan { boxes }) if boxes.len() == 1 && boxes[0].0.port == "COM3")
    );
    assert!(
        matches!(&scanned[1].result, Err(Error::NoBoxCan { boxes }) if boxes[0].1 == [Action::KeyPress])
    );
}

#[test]
fn a_scan_of_a_named_port_says_what_open_would() {
    script(&[("COM3", Family::Medius, Reply::Box(Some(A)))]);
    let scan = |connect: Connect| connect.scan_in(&|| vec![ch343("COM3")], PROBES);
    assert!(matches!(
        &scan(Connect::port("COM3").id(B))[0].result,
        Err(Error::WrongBox { .. })
    ));
    assert!(matches!(
        &scan(Connect::port("COM3").family(Family::MakcuV4))[0].result,
        Err(Error::WrongFamily { .. })
    ));

    script(&[("COM7", Family::Ferrum, Reply::Ferrum(FerrumApi::Legacy))]);
    let scanned = Connect::port("COM7")
        .ferrum_api(FerrumApi::Software)
        .scan_in(&|| vec![cp210x("COM7")], WITH_FERRUM);
    assert!(matches!(
        &scanned[0].result,
        Err(Error::WrongFerrumApi { .. })
    ));
}

#[test]
fn a_scan_describes_a_box_the_same_way_open_does() {
    script(&[("COM3", Family::Medius, Reply::Mouse)]);
    let connect =
        Connect::auto().click_timing(crate::ClickTiming::Fixed(Duration::from_millis(50)));
    let listed = || vec![ch343("COM3")];
    let scanned = connect.scan_in(&listed, PROBES);
    let found = scanned[0].result.as_ref().unwrap();
    let device = connect.clone().open_in(&listed, PROBES).unwrap();
    assert_eq!(found.capabilities, device.snapshot().capabilities);
    assert_eq!(found.info, device.snapshot().info);
}
