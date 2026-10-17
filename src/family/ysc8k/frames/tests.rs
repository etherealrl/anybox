use super::*;
use crate::backend::Input;
use crate::transport::codec::{Codec, Incoming};

mod examples;

fn wire(text: &str) -> Vec<u8> {
    let total = u16::try_from(text.len() + 14).unwrap();
    [
        b"<START>".as_slice(),
        &total.to_be_bytes(),
        text.as_bytes(),
        b"<END>",
    ]
    .concat()
}

/// the frame a V1 streams for physical input, the shape and field names are what the box sent on hardware
fn upload(b: i64, x: i64, y: i64, wh: i64) -> Vec<u8> {
    wire(&format!(
        r#"{{"code":200,"message":null,"data":"{{\"b\":{b},\"x\":{x},\"y\":{y},\"wh\":{wh}}}"}}"#
    ))
}

fn drain(codec: &mut YscFrames, buffer: &mut Vec<u8>) -> Vec<Incoming> {
    let mut incoming = Vec::new();
    while let Some(frame) = codec.split(buffer) {
        incoming.push(codec.classify(&frame));
    }
    incoming
}

#[test]
fn identity_has_twenty_five_bytes_without_a_checksum_or_newline() {
    assert_eq!(
        json("{\"cmd\":132}").unwrap(),
        b"<START>\x00\x19{\"cmd\":132}<END>"
    );
}

#[test]
fn lengths_count_utf8_bytes_and_the_end_marker_only_at_the_boundary() {
    let bytes = json("{\"x\":\"é<END><START>\"}").unwrap();
    assert_eq!(
        bytes,
        b"<START>\x00\x24{\"x\":\"\xc3\xa9<END><START>\"}<END>"
    );
    let mut codec = YscFrames::probe();
    assert_eq!(codec.split(&mut bytes.clone()), Some(bytes));
}

#[test]
fn outgoing_empty_and_oversized_payloads_are_refused() {
    assert!(json("").is_none());
    assert!(json(&"x".repeat(4097)).is_none());
    assert_eq!(json(&"x".repeat(4096)).unwrap().len(), 4110);
}

#[test]
fn identity_accepts_only_the_left_application_at_a_token_boundary() {
    for text in [
        r#"{"code":200,"message":"ysc-towmcu-L v1.0","data":""}"#,
        r#"{"code":200,"message":"version","data":"ysc-towmcu-L v1.0"}"#,
        r#"{"code":200,"message":"ok","data":"ysc-towmcu-L v1.0"}"#,
        r#"{"code":200,"data":"ysc-towmcu-L v1.0"}"#,
    ] {
        assert_eq!(version(&wire(text)).as_deref(), Some("ysc-towmcu-L v1.0"));
    }
    for text in [
        r#"{"code":200,"message":"ysc-towmcu-Liar"}"#,
        r#"{"code":200,"message":"version","data":"ysc-towmcu-Liar"}"#,
        r#"{"code":200,"message":"ok","data":"ysc-towmcu-R v1.0"}"#,
        r#"{"code":200,"message":"version","data":"Jul 23 2026"}"#,
        r#"{"code":200,"message":"ysc-towmcu-R v1.0"}"#,
        r#"{"code":200,"message":"ysc-towmcu-L v1.0","data":"YSC-IAP"}"#,
        r#"{"code":200,"message":"version","data":"YSC-IAP"}"#,
        r#"{"code":200,"message":"ok"}"#,
        r#"{"code":404,"message":"ysc-towmcu-L v1.0"}"#,
        r#"{"code":200,"message":"ysc-towmcu-L v1.0","other":{"version":"ysc-towmcu-R v1.0"}}"#,
    ] {
        assert_eq!(version(&wire(text)), None, "{text}");
    }
    assert_eq!(version(b"km.MAKCU\r\n"), None);
}

#[test]
fn the_v1_identity_is_the_build_date_and_nothing_else() {
    assert_eq!(
        v1_version(&wire(
            r#"{"code":200,"message":"version","data":"Jul 23 2026"}"#
        ))
        .as_deref(),
        Some("Jul 23 2026")
    );
    for text in [
        r#"{"code":200,"message":"ysc-towmcu-L v1.0","data":""}"#,
        r#"{"code":200,"message":"version","data":"ysc-towmcu-L v1.0"}"#,
        r#"{"code":200,"message":"version","data":"YSC-IAP"}"#,
        r#"{"code":200,"message":"version","data":""}"#,
        r#"{"code":200,"message":"version"}"#,
        r#"{"code":200,"message":"ok","data":"Jul 23 2026"}"#,
        r#"{"code":404,"message":"version","data":"Jul 23 2026"}"#,
        r#"{"message":"version","data":"Jul 23 2026"}"#,
        r#"{"cmd":132}"#,
    ] {
        assert_eq!(v1_version(&wire(text)), None, "{text}");
    }
    assert!(bootloader(&wire(
        r#"{"code":200,"message":"version","data":"YSC-IAP"}"#
    )));
    assert!(!bootloader(&wire(
        r#"{"code":200,"message":"version","data":"Jul 23 2026"}"#
    )));
    assert!(!bootloader(&wire(
        r#"{"code":404,"message":"version","data":"YSC-IAP"}"#
    )));
}

#[test]
fn debug_requires_its_own_code_and_a_boolean_in_an_object() {
    assert_eq!(
        debug(&wire(r#"{"code":702,"data":{"debug":true}}"#)),
        Some(true)
    );
    assert_eq!(
        debug(&wire(r#"{"code":702,"data":"{\"debug\":false}"}"#)),
        Some(false)
    );
    for text in [
        r#"{"code":200,"data":{"debug":true}}"#,
        r#"{"code":702,"data":{"debug":1}}"#,
        r#"{"code":702,"data":"[]"}"#,
        r#"{"code":702,"debug":false}"#,
    ] {
        assert_eq!(debug(&wire(text)), None);
    }
}

#[test]
fn every_split_keeps_json_uploads_separate_from_echo_results_and_prompts() {
    let stream = [
        b"km.move(100,50)\r\n".as_slice(),
        &upload(0, 3, -4, 0),
        b"result\r\n>>> km.left(1)\r\n>>>",
    ]
    .concat();
    for split in 0..=stream.len() {
        let mut codec = YscFrames::probe();
        let mut buffer = stream[..split].to_vec();
        let mut incoming = drain(&mut codec, &mut buffer);
        buffer.extend_from_slice(&stream[split..]);
        incoming.extend(drain(&mut codec, &mut buffer));
        assert_eq!(incoming.len(), 3, "split {split}");
        assert!(
            matches!(&incoming[0], Incoming::Inputs(inputs) if matches!(inputs.as_slice(), [Input::Motion { dx: 3, dy: -4, wheel: 0, pan: 0 }]))
        );
        assert!(matches!(incoming[1], Incoming::Reply(value) if value == key(b"km.move(100,50)")));
        assert!(matches!(incoming[2], Incoming::Reply(value) if value == key(b"km.left(1)")));
    }
}

#[test]
fn byte_by_byte_reads_accept_both_prompt_endings() {
    for prompt in [">>>", ">>> "] {
        let mut codec = YscFrames::probe();
        let mut buffer = Vec::new();
        let mut incoming = Vec::new();
        for &byte in format!("km.left(1)\r\n{prompt}km.right(0)\r\n{prompt}").as_bytes() {
            buffer.push(byte);
            incoming.extend(drain(&mut codec, &mut buffer));
        }
        assert_eq!(incoming.len(), 2);
        assert!(matches!(incoming[1], Incoming::Reply(value) if value == key(b"km.right(0)")));
    }
}

#[test]
fn a_lone_prompt_and_invalid_utf8_cannot_complete_output() {
    let mut codec = YscFrames::probe();
    for bytes in [b">>>".as_slice(), b"km.move(1,2)\r\n\xff\r\n>>>"] {
        let incoming = drain(&mut codec, &mut bytes.to_vec());
        assert!(matches!(incoming.as_slice(), [Incoming::Noise]));
    }
}

#[test]
fn bad_lengths_and_end_markers_resynchronize_overlapping_starts() {
    for bad in [
        b"<ST".as_slice(),
        b"<START>\x00\x0e<END>",
        b"<START>\xff\xff",
        b"<START>\x00\x10{}WRONG",
    ] {
        let expected = upload(0, 1, 2, 0);
        let mut buffer = [bad, &expected].concat();
        let mut codec = YscFrames::probe();
        let frames = drain(&mut codec, &mut buffer);
        assert!(
            matches!(frames.as_slice(), [Incoming::Inputs(inputs)] if matches!(inputs.as_slice(), [Input::Motion { dx: 1, dy: 2, .. }]))
        );
        assert!(buffer.is_empty());
    }
}

#[test]
fn malformed_json_utf8_nested_data_and_numbers_never_become_motion() {
    let mut codec = YscFrames::probe();
    let mut bad = b"<START>\x00\x10\xff\xff<END>".to_vec();
    assert!(matches!(
        drain(&mut codec, &mut bad).as_slice(),
        [Incoming::Noise]
    ));
    for text in [
        "{",
        "[]",
        r#"{"x":"1","y":2}"#,
        r#"{"x":1.0,"y":2}"#,
        r#"{"x":1,"y":2147483648}"#,
        r#"{"x":1}"#,
        r#"{"x":1,"y":2,"b":0,"wh":0}"#,
        r#"{"data":{"x":1,"y":2,"b":0,"wh":0}}"#,
        r#"{"code":200,"message":null,"data":"{\"x\":1,\"y\":2,\"wh\":0}"}"#,
        r#"{"code":200,"message":"mouse","data":"{\"x\":1,\"b\":0,\"wh\":0}"}"#,
        r#"{"code":200,"message":"mice","data":"{\"x\":1,\"y\":2,\"b\":0,\"wh\":0}"}"#,
        r#"{"code":200,"message":null,"data":"{\"x\":0,\"y\":0,\"b\":0,\"wh\":\"1\"}"}"#,
        r#"{"code":200,"message":null,"data":"{\"x\":0,\"y\":0,\"b\":-1,\"wh\":0}"}"#,
        r#"{"code":200,"message":null,"data":"{\"x\":1.0,\"y\":0,\"b\":0,\"wh\":0}"}"#,
        r#"{"code":200,"message":"kbd","data":"{\"x\":1,\"y\":2,\"b\":0,\"wh\":0}"}"#,
    ] {
        assert!(
            matches!(
                drain(&mut codec, &mut wire(text)).as_slice(),
                [Incoming::Noise]
            ),
            "{text}"
        );
    }
}

#[test]
fn first_mask_is_a_baseline_and_releases_precede_presses() {
    let mut codec = YscFrames::probe();
    let first = drain(&mut codec, &mut upload(1, 0, 0, 0));
    assert!(matches!(first.as_slice(), [Incoming::Inputs(inputs)] if inputs.len() == 1));
    let moved = drain(&mut codec, &mut upload(1, -1, 2, -1));
    assert!(
        matches!(moved.as_slice(), [Incoming::Inputs(inputs)] if matches!(inputs.as_slice(), [Input::Motion { dx: -1, dy: 2, wheel: -1, pan: 0 }]))
    );
    let changed = drain(&mut codec, &mut upload(226, 0, 0, 0));
    assert!(
        matches!(changed.as_slice(), [Incoming::Inputs(inputs)] if matches!(inputs.as_slice(), [Input::Button { button: crate::Button::Left, pressed: false }, Input::Button { button: crate::Button::Right, pressed: true }, Input::Motion { .. }]))
    );
    let duplicate = drain(&mut codec, &mut upload(2, 0, 0, 0));
    assert!(matches!(duplicate.as_slice(), [Incoming::Inputs(inputs)] if inputs.len() == 1));
}

#[test]
fn known_reply_examples_are_drained_before_considering_upload_fields() {
    let mut codec = YscFrames::probe();
    for (code, message, data) in [
        (200, "version", "Jul 23 2026"),
        (200, "ysc-towmcu-R v1.0", ""),
        (200, "version", "YSC-IAP"),
        (200, "macro_set", "0"),
        (200, "macro_reset", ""),
        (200, "macro", "[{\"s\":0,\"e\":0}]"),
        (
            200,
            "jitter",
            "{\"e\":1,\"t\":1,\"ax\":10,\"fx\":0,\"py\":5,\"fy\":0}",
        ),
        (
            200,
            "mouse_curve",
            "{\"enabled\":1,\"profile\":2,\"segments\":4,\"duration\":0,\"jitter\":15}",
        ),
        (200, "entering_iap", ""),
        (200, "switching", ""),
        (200, "type_done", "{\"typed\":3,\"skipped\":0,\"total\":3}"),
        (
            200,
            "type_status",
            "{\"busy\":false,\"remaining\":0,\"total\":3,\"typed\":3,\"skipped\":0}",
        ),
        (200, "interp", "{\"enabled\":1,\"profile\":2,\"ema\":0.5}"),
        (
            200,
            "sn",
            "{\"custom\":\"\",\"physical\":\"abc\",\"active\":\"abc\"}",
        ),
        (700, "debug_enter", ""),
        (701, "debug_exit", ""),
        (702, "status", "{\"debug\":false}"),
        (710, "", "{\"hex\":\"0102\"}"),
        (711, "", "{\"hex\":\"0102\"}"),
        (712, "", "{\"hex\":\"0102\",\"total\":2}"),
        (713, "", "{\"len\":2,\"hex\":\"0102\"}"),
        (714, "debug_cleared", ""),
        (715, "", "{}"),
        (100, "ok", ""),
        (101, "", "{}"),
        (103, "", "{}"),
        (104, "ok", ""),
    ] {
        let text =
            serde_json::json!({"code":code,"message":message,"data":data,"x":7,"y":8}).to_string();
        assert!(
            matches!(
                drain(&mut codec, &mut wire(&text)).as_slice(),
                [Incoming::Noise]
            ),
            "{code} {message}"
        );
    }
    assert!(matches!(
        drain(
            &mut codec,
            &mut wire(r#"{"code":702,"data":{"debug":false,"x":1,"y":2}}"#)
        )
        .as_slice(),
        [Incoming::Noise]
    ));
}

#[test]
fn noise_and_text_accumulators_are_bounded_and_report_host_loss() {
    let (tx, lost) = std::sync::mpsc::channel();
    let mut codec = YscFrames::new(YscFrames::probe().shared, Some(crate::backend::Sink(tx)));
    let mut buffer = vec![b'x'; 9000];
    assert!(codec.split(&mut buffer).is_none());
    assert!(buffer.len() <= MAX_FRAME + 4096);
    assert!(matches!(
        lost.try_recv(),
        Ok(crate::backend::Signal::Dropped(
            1,
            crate::event::DropSource::Host,
            None
        ))
    ));
    let mut bytes = [b"km.left(1)\r\n".as_slice(), &vec![b'x'; 1025], b"\r\n>>>"].concat();
    assert!(matches!(
        drain(&mut codec, &mut bytes).as_slice(),
        [Incoming::Noise]
    ));
    assert!(lock(&codec.shared.state).seen.reply.len() <= MAX_TEXT);
}

#[test]
fn incomplete_json_waits_instead_of_scanning_for_an_embedded_end_marker() {
    let mut codec = YscFrames::probe();
    let mut buffer = b"<START>\x00\x40{\"text\":\"<END>\"}".to_vec();
    let before = buffer.clone();
    assert!(codec.split(&mut buffer).is_none());
    assert_eq!(buffer, before);
}

#[test]
fn binary_baud_and_vendor_version_wrappers_are_noise_until_their_text_prompt() {
    let mut codec = YscFrames::probe();
    let incoming = drain(
        &mut codec,
        &mut b"\xde\xad\x05\x00\xa5\xc0\xc6\x2d\x00\r\n>>> ".to_vec(),
    );
    assert!(matches!(incoming.as_slice(), [Incoming::Noise]));
    let incoming = drain(
        &mut codec,
        &mut b"km.version()\r\nkm.version(km.MAKCU)\r\n>>> ".to_vec(),
    );
    assert!(
        matches!(incoming.as_slice(),[Incoming::Reply(value)] if *value == key(b"km.version(km.MAKCU)"))
    );
}

fn listening() -> (YscFrames, std::sync::mpsc::Receiver<crate::backend::Signal>) {
    let (tx, rx) = std::sync::mpsc::channel();
    (
        YscFrames::new(YscFrames::probe().shared, Some(crate::backend::Sink(tx))),
        rx,
    )
}

fn lost(rx: &std::sync::mpsc::Receiver<crate::backend::Signal>) -> bool {
    matches!(
        rx.try_recv(),
        Ok(crate::backend::Signal::Dropped(
            1,
            crate::event::DropSource::Host,
            None
        ))
    )
}

#[test]
fn an_object_that_is_neither_a_reply_nor_an_upload_counts_as_lost() {
    let (mut codec, rx) = listening();
    assert!(matches!(
        drain(&mut codec, &mut wire(r#"{"y":2}"#)).as_slice(),
        [Incoming::Noise]
    ));
    assert!(lost(&rx));
}

#[test]
fn a_bad_end_marker_counts_as_lost() {
    let (mut codec, rx) = listening();
    let mut bytes = wire(r#"{"x":1,"y":2}"#);
    let at = bytes.len() - 1;
    bytes[at] = b'X';
    drain(&mut codec, &mut bytes);
    assert!(lost(&rx));
}

#[test]
fn an_upload_shaped_200_without_a_message_that_cant_be_read_counts_as_lost() {
    let (mut codec, rx) = listening();
    drain(
        &mut codec,
        &mut wire(r#"{"code":200,"data":"{\"b\":0,\"x\":1,\"wh\":0}"}"#),
    );
    assert!(lost(&rx));
}

#[test]
fn key_answers_and_a_400_complete_the_waiting_key_and_are_not_lost_reports() {
    let (mut codec, rx) = listening();
    for text in [
        r#"{"code":200,"message":"key_down","data":null}"#,
        r#"{"code":200,"message":"key_up","data":null}"#,
        r#"{"code":404,"message":"no_keyboard","data":null}"#,
        r#"{"code":400,"message":"2","data":null}"#,
    ] {
        assert!(
            matches!(drain(&mut codec, &mut wire(text)).as_slice(), [Incoming::Reply(value)] if *value == key_reply()),
            "{text}"
        );
    }
    while let Ok(signal) = rx.try_recv() {
        assert!(!matches!(signal, crate::backend::Signal::Dropped(..)));
    }
}

/// what the tester's V1 sent with a keyboard behind it, `a` going down and coming back up, the box's own counters
/// after `k` included
const PRESSED_A: &str = r#"{"code":200,"message":"kbd","data":"{\"mod\":0,\"n\":1,\"ov\":0,\"k\":[4],\"ok\":1,\"dEp\":1,\"dIfc\":0,\"dLv\":1,\"dRid\":0,\"dHr\":0,\"dMp\":1,\"dBm\":0,\"dBl\":8,\"dKo\":2,\"dKc\":6,\"dMb\":64,\"dInj\":0,\"dIm\":0,\"dUn\":16,\"dRm\":0,\"dOk\":77,\"dFail\":0,\"dRdy\":0,\"dEnum\":1,\"dTxl\":8,\"dTx\":\"0000040000000000\"}"}"#;
const RELEASED_A: &str = r#"{"code":200,"message":"kbd","data":"{\"mod\":0,\"n\":0,\"ov\":0,\"k\":[],\"ok\":1,\"dEp\":1,\"dIfc\":0,\"dLv\":1,\"dRid\":0,\"dHr\":0,\"dMp\":1,\"dBm\":0,\"dBl\":8,\"dKo\":2,\"dKc\":6,\"dMb\":64,\"dInj\":0,\"dIm\":0,\"dUn\":0,\"dRm\":0,\"dOk\":79,\"dFail\":0,\"dRdy\":0,\"dEnum\":1,\"dTxl\":8,\"dTx\":\"0000000000000000\"}"}"#;
/// the same V1 just before the tester's replug, with the keyboard gone from behind it
const KEYBOARD_GONE: &str = r#"{"code":200,"message":"kbd","data":"{\"mod\":0,\"n\":0,\"ov\":0,\"k\":[],\"ok\":0,\"dEp\":0,\"dIfc\":255,\"dLv\":0,\"dRid\":0,\"dHr\":0,\"dMp\":0,\"dBm\":0,\"dBl\":0,\"dKo\":0,\"dKc\":0,\"dMb\":0,\"dInj\":0,\"dIm\":0,\"dUn\":0,\"dRm\":0,\"dOk\":222,\"dFail\":0,\"dRdy\":0,\"dEnum\":1,\"dTxl\":8,\"dTx\":\"0000000000000000\"}"}"#;
/// what it sent after anybox's own cmd 45 with the tester's mouse behind it, the pressed key only in `dTx`
const INJECTED: &str = r#"{"code":200,"message":"kbd","data":"{\"mod\":0,\"n\":0,\"ov\":0,\"k\":[],\"ok\":1,\"dEp\":2,\"dIfc\":1,\"dLv\":1,\"dRid\":1,\"dHr\":1,\"dMp\":1,\"dBm\":0,\"dBl\":15,\"dKo\":1,\"dKc\":14,\"dMb\":120,\"dInj\":0,\"dIm\":0,\"dUn\":0,\"dRm\":0,\"dOk\":1,\"dFail\":0,\"dRdy\":1,\"dEnum\":1,\"dTxl\":16,\"dTx\":\"0100000000000000\"}"}"#;

fn kbd(data: &str) -> Vec<u8> {
    wire(&serde_json::json!({"code": 200, "message": "kbd", "data": data}).to_string())
}

fn report(modifiers: u8, keys: &[u8], overflow: u8) -> Vec<u8> {
    kbd(&format!(
        r#"{{"mod":{modifiers},"n":{},"ov":{overflow},"k":{keys:?},"ok":1}}"#,
        keys.len()
    ))
}

fn key_edges(incoming: &[Incoming]) -> Vec<(u8, bool)> {
    let mut edges = Vec::new();
    for incoming in incoming {
        let Incoming::Inputs(inputs) = incoming else {
            panic!("a keyboard report has to give key edges, not {incoming:?}");
        };
        for input in inputs {
            let Input::Key { key, pressed } = input else {
                panic!("a keyboard report only gives keys, not {input:?}");
            };
            edges.push((key.0, *pressed));
        }
    }
    edges
}

#[test]
fn the_v1s_own_keyboard_reports_become_key_edges_in_64_byte_reads() {
    let stream = [wire(PRESSED_A), wire(RELEASED_A)].concat();
    let mut codec = YscFrames::probe();
    let mut buffer = Vec::new();
    let mut incoming = Vec::new();
    for read in stream.chunks(64) {
        buffer.extend_from_slice(read);
        incoming.extend(drain(&mut codec, &mut buffer));
    }
    assert_eq!(key_edges(&incoming), [(4, true), (4, false)]);
    assert!(buffer.is_empty());
}

#[test]
fn modifiers_come_from_their_byte_and_releases_go_before_presses() {
    let mut codec = YscFrames::probe();
    assert_eq!(
        key_edges(&drain(&mut codec, &mut report(0x22, &[4], 0))),
        [(0xE1, true), (0xE5, true), (4, true)]
    );
    assert_eq!(
        key_edges(&drain(&mut codec, &mut report(0x02, &[5], 0))),
        [(0xE5, false), (4, false), (5, true)]
    );
}

#[test]
fn a_report_that_changes_no_key_gives_nothing() {
    let mut codec = YscFrames::probe();
    assert_eq!(
        key_edges(&drain(&mut codec, &mut wire(PRESSED_A))),
        [(4, true)]
    );
    assert!(key_edges(&drain(&mut codec, &mut wire(PRESSED_A))).is_empty());
    let mut codec = YscFrames::probe();
    assert!(key_edges(&drain(&mut codec, &mut wire(INJECTED))).is_empty());
}

#[test]
fn the_report_with_the_keyboard_gone_lets_go_of_every_key() {
    let mut codec = YscFrames::probe();
    drain(&mut codec, &mut report(0x01, &[4, 5], 0));
    assert_eq!(
        key_edges(&drain(&mut codec, &mut wire(KEYBOARD_GONE))),
        [(0xE0, false), (4, false), (5, false)]
    );
}

#[test]
fn only_the_first_n_valid_keys_count_and_each_once() {
    for (data, expected) in [
        (r#"{"mod":0,"n":1,"ov":0,"k":[4,5]}"#, vec![(4, true)]),
        (r#"{"mod":0,"n":3,"ov":0,"k":[4]}"#, vec![(4, true)]),
        (r#"{"mod":0,"n":2,"ov":0,"k":[4,4]}"#, vec![(4, true)]),
        (r#"{"mod":0,"n":3,"ov":0,"k":[1,4,232]}"#, vec![(4, true)]),
        (r#"{"mod":1,"n":1,"ov":0,"k":[224]}"#, vec![(0xE0, true)]),
        (r#"{"mod":0,"n":0,"ov":0,"k":[4]}"#, vec![]),
    ] {
        let mut codec = YscFrames::probe();
        assert_eq!(
            key_edges(&drain(&mut codec, &mut kbd(data))),
            expected,
            "{data}"
        );
    }
}

#[test]
fn an_overflow_keeps_only_the_keys_above_the_highest_one_named() {
    let mut codec = YscFrames::probe();
    let mut edges = |modifiers: u8, keys: &[u8], overflow: u8| {
        key_edges(&drain(&mut codec, &mut report(modifiers, keys, overflow)))
    };
    assert_eq!(edges(0, &[5, 6, 7, 8, 9, 10], 0).len(), 6);
    // 4 goes down and pushes 10 off the list, though 10 is still down
    assert_eq!(edges(0, &[4, 5, 6, 7, 8, 9], 1), [(4, true)]);
    // then 10 goes up, and the first report without an overflow settles it, though the keys it names didn't change
    assert_eq!(edges(0, &[4, 5, 6, 7, 8, 9], 0), [(10, false)]);
    // 11 and 12 go down past the list, and a low key let go still shows at once
    assert!(edges(0, &[4, 5, 6, 7, 8, 9], 2).is_empty());
    assert_eq!(edges(0, &[4, 6, 7, 8, 9, 11], 1), [(5, false), (11, true)]);
    // a modifier is never kept, its byte always has all of them
    assert_eq!(edges(0x01, &[4, 6, 7, 8, 9, 11], 1), [(0xE0, true)]);
    assert_eq!(edges(0, &[4, 6, 7, 8, 9, 11], 1), [(0xE0, false)]);
}

#[test]
fn a_keyboard_report_that_cant_be_read_is_lost_for_key_receivers_only() {
    for data in [
        r#"{"n":1,"ov":0,"k":[4]}"#,
        r#"{"mod":0,"ov":0,"k":[4]}"#,
        r#"{"mod":0,"n":1,"k":[4]}"#,
        r#"{"mod":0,"n":1,"ov":0}"#,
        r#"{"mod":256,"n":1,"ov":0,"k":[4]}"#,
        r#"{"mod":0,"n":-1,"ov":0,"k":[4]}"#,
        r#"{"mod":0,"n":1,"ov":-1,"k":[4]}"#,
        r#"{"mod":0,"n":1,"ov":0,"k":4}"#,
        r#"{"mod":0,"n":1,"ov":0,"k":["4"]}"#,
        r#"{"mod":0,"n":1,"ov":0,"k":[4.5]}"#,
        r#"{"mod":0,"n":1,"ov":0,"k":[256]}"#,
        "[]",
    ] {
        let (mut codec, rx) = listening();
        assert!(
            matches!(
                drain(&mut codec, &mut kbd(data)).as_slice(),
                [Incoming::Noise]
            ),
            "{data}"
        );
        assert!(
            matches!(
                rx.try_recv(),
                Ok(crate::backend::Signal::Dropped(
                    1,
                    crate::event::DropSource::Host,
                    Some(InputKind::Keys)
                ))
            ),
            "{data}"
        );
        assert!(rx.try_recv().is_err());
    }
}

#[test]
fn keys_stop_going_out_once_the_box_says_theres_no_keyboard() {
    let (mut codec, rx) = listening();
    drain(
        &mut codec,
        &mut wire(r#"{"code":404,"message":"no_keyboard","data":null}"#),
    );
    while rx.try_recv().is_ok() {}
    assert!(matches!(
        drain(&mut codec, &mut report(0, &[4], 0)).as_slice(),
        [Incoming::Noise]
    ));
    assert!(matches!(
        drain(&mut codec, &mut kbd("{}")).as_slice(),
        [Incoming::Noise]
    ));
    assert!(rx.try_recv().is_err());
}

#[test]
fn a_report_named_mouse_with_no_message_or_without_a_wheel_is_still_an_upload() {
    for (text, expected) in [
        (
            r#"{"code":200,"message":"mouse","data":"{\"b\":1,\"x\":3,\"y\":-4,\"wh\":1}"}"#,
            1,
        ),
        (
            r#"{"code":200,"data":"{\"b\":1,\"x\":3,\"y\":-4,\"wh\":-1}"}"#,
            -1,
        ),
        (
            r#"{"code":200,"message":null,"data":"{\"b\":1,\"x\":3,\"y\":-4}"}"#,
            0,
        ),
    ] {
        let mut codec = YscFrames::probe();
        assert!(
            matches!(drain(&mut codec, &mut wire(text)).as_slice(), [Incoming::Inputs(inputs)] if matches!(inputs.as_slice(), [Input::Motion { dx: 3, dy: -4, wheel, pan: 0 }] if *wheel == expected)),
            "{text}"
        );
    }
}

#[test]
fn an_upload_shaped_200_named_mouse_that_cant_be_read_counts_as_lost() {
    for text in [
        r#"{"code":200,"message":"mouse","data":"{\"b\":0,\"x\":1}"}"#,
        r#"{"code":200,"message":"mouse","data":"{\"b\":0,\"x\":1,\"y\":0,\"wh\":\"1\"}"}"#,
    ] {
        let (mut codec, rx) = listening();
        drain(&mut codec, &mut wire(text));
        assert!(lost(&rx), "{text}");
    }
}
