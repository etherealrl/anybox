use super::{YscFrames, drain, key, wire};
use crate::transport::codec::Incoming;

#[test]
fn every_literal_example_in_the_shipped_docs_is_split_without_becoming_input() {
    let examples = [
        r#"{"cmd":30,"x":100,"y":50,"c":1,"rx":1.0,"ry":1.0,"rc":0}"#,
        r#"{"cmd":31,"x":100,"y":50,"c":1,"rx":1.0,"ry":1.0,"rc":0}"#,
        r#"{"cmd":32,"b":1,"c":10}"#,
        r#"{"cmd":33,"b":1,"s":1}"#,
        r#"{"cmd":34,"status":1}"#,
        r#"{"cmd":35,"w":-1,"c":100}"#,
        r#"{"cmd":36,"s":0,"e":1,"t":3,"p":1,"w":-1,"i":30,"d":10,"ij":5,"dj":2}"#,
        r#"{"cmd":37,"s":-1}"#,
        r#"{"cmd":38}"#,
        r#"{"cmd":39,"e":1,"t":1,"ax":10,"fx":0,"py":5,"fy":0}"#,
        r#"{"cmd":40}"#,
        r#"{"cmd":41}"#,
        r#"{"cmd":42,"e":1,"p":2,"n":4,"d":0,"j":15}"#,
        r#"{"cmd":43}"#,
        r#"{"cmd":44}"#,
        r#"{"cmd":50}"#,
        r#"{"cmd":132}"#,
        r#"{"cmd":133,"baud":3000000}"#,
        r#"{"cmd":34,"status":0}"#,
        r#"{"cmd":35,"w":-1,"c":200}"#,
        r#"{"cmd":35,"w":1,"c":100}"#,
        r#"{"cmd":36,"s":0,"e":1,"t":3,"p":1,"w":-1,"i":30,"d":10}"#,
        r#"{"cmd":36,"s":0,"e":1,"t":3,"p":1,"w":-1}"#,
        r#"{"cmd":36,"s":2,"e":1,"t":16,"p":0,"w":-1}"#,
        r#"{"cmd":36,"s":2,"e":1,"t":16,"p":0,"w":-1,"i":30,"d":10}"#,
        r#"{"cmd":36,"s":0,"e":0,"t":3,"p":1,"w":-1}"#,
        r#"{"cmd":37,"s":0}"#,
        r#"{"cmd":45,"kc":4,"down":1}"#,
        r#"{"cmd":45,"kc":4,"down":0}"#,
        r#"{"cmd":45,"kc":225,"down":1}"#,
        r#"{"cmd":45,"kc":225,"down":0}"#,
        r#"{"cmd":46}"#,
        r#"{"cmd":47,"s":"Wasd123A123vciseC"}"#,
        r#"{"cmd":47,"s":"Hello World!"}"#,
        r#"{"cmd":47,"s":"test@123"}"#,
        r#"{"cmd":47,"s":"!@#$%^&*()"}"#,
        r#"{"cmd":47,"s":""}"#,
        r#"{"cmd":48}"#,
        r#"km.move(100,50)"#,
        r#"km.left(1)"#,
        r#"km.left(0)"#,
        r#"km.left(2)"#,
        r#"km.right(1)"#,
        r#"km.right(0)"#,
        r#"km.right(2)"#,
        r#"km.middle(1)"#,
        r#"km.middle(0)"#,
        r#"km.middle(2)"#,
        r#"km.ms1(1)"#,
        r#"km.ms1(0)"#,
        r#"km.ms2(1)"#,
        r#"km.ms2(0)"#,
        r#"km.wheel(1)"#,
        r#"km.wheel(-1)"#,
        r#"km.lock_mx(1)"#,
        r#"km.lock_mx(0)"#,
        r#"km.lock_my(1)"#,
        r#"km.lock_my(0)"#,
        r#"km.lock_ml(1)"#,
        r#"km.lock_ml(0)"#,
        r#"km.lock_mr(1)"#,
        r#"km.lock_mr(0)"#,
        r#"km.lock_mm(1)"#,
        r#"km.lock_mm(0)"#,
        r#"km.lock_ms1(1)"#,
        r#"km.lock_ms1(0)"#,
        r#"km.lock_ms2(1)"#,
        r#"km.lock_ms2(0)"#,
        r#"km.catch_ml(1)"#,
        r#"km.catch_ml(0)"#,
        r#"km.catch_mm(1)"#,
        r#"km.catch_mr(1)"#,
        r#"km.catch_ms1(1)"#,
        r#"km.catch_ms2(1)"#,
        r#"km.invert_x(1)"#,
        r#"km.invert_x(0)"#,
        r#"km.invert_y(1)"#,
        r#"km.invert_y(0)"#,
        r#"km.swap_xy(1)"#,
        r#"km.swap_xy(0)"#,
        r#"km.scroll_side(1)"#,
        r#"km.scroll_side(0)"#,
        r#"km.scroll_lr(1)"#,
        r#"km.scroll_lr(0)"#,
        r#"km.buttons(1,10)"#,
        r#"km.serial("abc123")"#,
        r#"km.serial()"#,
        r#"km.serial(0)"#,
        r#"km.echo(1)"#,
        r#"km.echo(0)"#,
        r#"km.baud(3000000)"#,
        r#"km.version()"#,
        r#"km.reboot()"#,
    ];
    assert_eq!(examples.len(), 93);
    for example in examples {
        let bytes = if example.starts_with('{') {
            wire(example)
        } else {
            format!("{example}\r\n>>> ").into_bytes()
        };
        for boundary in 0..=bytes.len() {
            let mut codec = YscFrames::probe();
            let mut buffer = bytes[..boundary].to_vec();
            let mut incoming = drain(&mut codec, &mut buffer);
            buffer.extend_from_slice(&bytes[boundary..]);
            incoming.extend(drain(&mut codec, &mut buffer));
            assert_eq!(incoming.len(), 1, "{example} split {boundary}");
            if example.starts_with('{') {
                assert!(matches!(incoming[0], Incoming::Noise), "{example}");
            } else {
                assert!(
                    matches!(incoming[0], Incoming::Reply(value) if value == key(example.as_bytes())),
                    "{example}"
                );
            }
        }
    }
}
