use super::{Lang, doc, find, marked, pascal, replace, wrap};
use crate::table::{CALLS, Call, On, Ret, Ty};

#[test]
fn a_region_gets_replaced_with_its_marks_and_indent_kept() {
    let text = "a\n    // made by bindings/gen from its call table (x), more\n    old\n    // the end of what bindings/gen makes (x)\nb\n";
    let made = replace(
        text,
        "x",
        &["new".into(), String::new(), "two".into(), String::new()],
    )
    .unwrap();
    assert_eq!(
        made,
        "a\n    // made by bindings/gen from its call table (x), more\n    new\n\n    two\n\n    // the end of what bindings/gen makes (x)\nb\n"
    );
    assert!(replace(text, "y", &[]).is_err());
}

#[test]
fn a_region_keeps_the_files_line_ends() {
    let text = "// made by bindings/gen from its call table (x)\r\nold\r\n// the end of what bindings/gen makes (x)\r\n";
    let made = replace(text, "x", &["new".into()]).unwrap();
    assert_eq!(
        made,
        "// made by bindings/gen from its call table (x)\r\nnew\r\n// the end of what bindings/gen makes (x)\r\n"
    );
}

#[test]
fn a_long_text_wraps_at_the_width() {
    let lines = wrap("one two three four", "// ", "// ", 12);
    assert_eq!(lines, ["// one two", "// three", "// four"]);
    assert!(lines.iter().all(|line| line.len() <= 12));
    let tag = wrap("one <see cref=\"Two\"/>", "// ", "// ", 12);
    assert_eq!(tag, ["// one", "// <see cref=\"Two\"/>"]);
}

#[test]
fn a_reference_to_another_call_is_written_each_languages_way() {
    let press_key = CALLS.iter().find(|call| call.name == "press_key").unwrap();
    assert!(doc(press_key, Lang::C).contains("`anybox_device_release_key`"));
    assert!(doc(press_key, Lang::Python).contains("`release_key`"));
    assert!(doc(press_key, Lang::CSharp).contains("<see cref=\"ReleaseKey\"/>"));
    assert!(doc(press_key, Lang::Cpp).contains("`release_key`"));
    assert_eq!(pascal("discard_pending_moves"), "DiscardPendingMoves");
    let tap_key_with = CALLS
        .iter()
        .find(|call| call.name == "tap_key_with")
        .unwrap();
    assert!(doc(tap_key_with, Lang::C).contains("`tap` says how long"));
}

// Python and C# bind the folded call under the C-only one's name, so the two have to line up or a binding loses one
#[test]
fn every_folded_call_is_a_c_only_call_with_a_click_added() {
    let shape = |call: &Call| -> Vec<Ty> {
        call.args
            .iter()
            .map(|arg| arg.ty)
            .filter(|ty| *ty != Ty::Click)
            .collect()
    };
    for c_only in CALLS.iter().filter(|call| call.only_in_c) {
        let folds = CALLS
            .iter()
            .filter(|call| call.on == c_only.on && call.folds_into == Some(c_only.name))
            .count();
        assert_eq!(folds, 1, "{} has {folds} calls folded into it", c_only.name);
    }
    for call in CALLS.iter().filter(|call| call.folds_into.is_some()) {
        let into = find(call.on, call.folds_into.unwrap());
        assert!(
            into.only_in_c && !call.only_in_c,
            "{} folds into {}",
            call.name,
            into.name
        );
        assert_eq!(shape(call), shape(into), "{} and {}", call.name, into.name);
        assert_eq!(
            call.args.iter().filter(|arg| arg.ty == Ty::Click).count(),
            1
        );
    }
}

#[test]
#[should_panic(expected = "there's no call missing_call on Device")]
fn a_doc_naming_a_call_that_isnt_there_stops_even_in_c() {
    let call = Call {
        on: On::Device,
        name: "x",
        args: &[],
        doc: "see {missing_call}",
        folds_into: None,
        only_in_c: true,
        ret: Ret::Unit,
        action: None,
    };
    doc(&call, Lang::C);
}

#[test]
fn a_region_marked_twice_or_left_unknown_is_refused() {
    let text = "// made by bindings/gen from its call table (x), more\nold\n// the end of what bindings/gen makes (x)\n";
    let twice = format!("{text}{text}");
    assert!(
        replace(&twice, "x", &[])
            .unwrap_err()
            .contains("more than once")
    );
    let other = format!("{text}// made by bindings/gen from its call table (y)\n");
    assert_eq!(marked(&other), ["x", "x", "y"]);
}

#[test]
fn an_extension_doc_says_off_rejected_and_its_arguments_each_languages_way() {
    let set_autorelease = find(On::Makcu, "set_autorelease");
    assert!(doc(set_autorelease, Lang::C).contains("or off with 0"));
    assert!(doc(set_autorelease, Lang::Python).contains("or off with None"));
    assert!(doc(set_autorelease, Lang::CSharp).contains("or off with null"));
    assert!(doc(set_autorelease, Lang::Cpp).contains("or off with an empty one"));
    let set_interpolation = find(On::Ysc, "set_interpolation");
    assert!(doc(set_interpolation, Lang::C).contains("is `ANYBOX_STATUS_REJECTED`"));
    assert!(doc(set_interpolation, Lang::CSharp).contains("<see cref=\"RejectedException\"/>"));
    // the C argument is in milliseconds, and a call on the device is the device's in C#
    let catch_xy = find(On::Ferrum, "catch_xy");
    assert!(doc(catch_xy, Lang::C).contains("`window_ms`"));
    assert!(doc(catch_xy, Lang::C).contains("`anybox_device_move_by`"));
    assert!(doc(catch_xy, Lang::CSharp).contains("<paramref name=\"withSoftware\"/>"));
    assert!(doc(catch_xy, Lang::CSharp).contains("<see cref=\"Device.MoveBy\"/>"));
}
