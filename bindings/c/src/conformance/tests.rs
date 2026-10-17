use super::*;

fn scenarios() -> Vec<Value> {
    let text = std::fs::read_to_string(root().join("bindings/conformance/scenarios.json")).unwrap();
    serde_json::from_str::<Value>(&text)
        .unwrap()
        .as_array()
        .unwrap()
        .clone()
}

#[test]
fn record_what_the_rust_crate_does() {
    // a recording that fails halfway mustn't leave the last one behind, the runners would pass against it
    let path = root().join("target/conformance/expected.json");
    if path.exists() {
        std::fs::remove_file(&path).unwrap();
    }
    let header = header();
    let expected: Vec<Value> = scenarios()
        .iter()
        .map(|scenario| run(scenario, &header))
        .collect();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, serde_json::to_string_pretty(&expected).unwrap()).unwrap();
}

/// every whole word starting with `anybox_`
fn anybox_names(code: &str) -> std::collections::BTreeSet<String> {
    let word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut names = std::collections::BTreeSet::new();
    for (at, _) in code.match_indices("anybox_") {
        if code[..at].chars().next_back().is_some_and(word) {
            continue;
        }
        let name: String = code[at..].chars().take_while(|c| word(*c)).collect();
        names.insert(name);
    }
    names
}

// the C++ wrapper is written by hand around its generated plain calls, so like the Python and C# header tests this
// makes it reach every function the header has, or leave it out with the reason

#[test]
fn every_header_function_is_used_by_the_cpp_wrapper_or_left_out_on_purpose() {
    let header = std::fs::read_to_string(root().join("bindings/c/include/anybox.h")).unwrap();
    // a declaration starts at the start of its line with its return type, a comment or a #define never does
    let declared: std::collections::BTreeSet<String> = header
        .lines()
        .filter(|line| line.starts_with(|c: char| c.is_ascii_alphabetic()) && line.contains('('))
        .flat_map(|line| {
            anybox_names(line)
                .into_iter()
                .filter(|name| line.contains(&format!("{name}(")))
        })
        .collect();
    let wrapper = std::fs::read_to_string(root().join("bindings/c/include/anybox.hpp")).unwrap();
    let used = anybox_names(&cpp_code(&wrapper));
    // connect is `Connect().open()`, a struct of zeros is the default options, the plain click and taps are the
    // `_with` ones with no hold, and a scan entry's error is read from the last error its result call sets, which
    // also has the message and the lists
    let left_out = [
        "anybox_connect",
        "anybox_connect_options_default",
        "anybox_device_click",
        "anybox_device_tap_key",
        "anybox_device_tap_media",
        "anybox_scan_list_error_detail",
    ];
    let missing: Vec<&String> = declared
        .iter()
        .filter(|name| !used.contains(*name) && !left_out.contains(&name.as_str()))
        .collect();
    assert!(
        missing.is_empty(),
        "anybox.hpp doesn't use {missing:?}, wrap them or leave them out here with the reason"
    );
    for name in left_out {
        assert!(declared.contains(name), "anybox.h has no {name} any more");
        assert!(
            !used.contains(name),
            "anybox.hpp uses {name} now, take it off the left out list"
        );
    }
    assert!(
        declared.len() > 70,
        "only {} functions found in anybox.h",
        declared.len()
    );
}

// the scenarios have to reach everything, otherwise a binding could get something wrong that no runner ever sees

#[test]
fn every_plain_device_function_is_a_call_and_every_call_is_in_a_scenario() {
    let text = std::fs::read_to_string(root().join("bindings/c/include/anybox.h")).unwrap();
    let mut declared: Vec<&str> = text
        .split("anybox_device_")
        .skip(1)
        .filter_map(|rest| rest.split_once('(').map(|(name, _)| name))
        .filter(|name| name.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
        .filter(|name| !NOT_CALLS.contains(name))
        .collect();
    declared.sort_unstable();
    declared.dedup();
    let mut calls = CALLS.to_vec();
    calls.sort_unstable();
    assert_eq!(
        declared, calls,
        "a device function in anybox.h that's neither a call nor in NOT_CALLS"
    );
    let used: Vec<String> = scenarios()
        .iter()
        .flat_map(|scenario| scenario["steps"].as_array().unwrap().clone())
        .filter_map(|step| {
            step.get("call")
                .map(|name| name.as_str().unwrap().to_string())
        })
        .collect();
    for name in CALLS {
        assert!(
            used.iter().any(|used| used == name),
            "no scenario calls {name}"
        );
    }
}

#[test]
fn every_error_and_every_event_is_in_a_scenario() {
    let header = header();
    let steps: Vec<Value> = scenarios()
        .iter()
        .flat_map(|scenario| scenario["steps"].as_array().unwrap().clone())
        .collect();
    let examples: Vec<&Value> = steps
        .iter()
        .filter_map(|step| step.get("example"))
        .collect();
    for example in Error::examples() {
        let wanted = name(&header, "STATUS", &example);
        assert!(
            examples.contains(&&wanted),
            "no scenario has the example of {example:?}"
        );
    }
    // that the examples have every kind of event is the core's own test
    let kinds = Event::examples(Stamp::new(Instant::now(), Duration::ZERO)).len();
    assert!(
        steps
            .iter()
            .any(|step| step.get("example_events").is_some()),
        "no scenario sends the example events"
    );
    assert!(
        steps
            .iter()
            .any(|step| step.get("recv").and_then(Value::as_u64) == Some(kinds as u64)),
        "no scenario receives all {kinds} example events"
    );
}
