use super::*;
use crate::conformance::root;

fn scenarios() -> Vec<Value> {
    let text = std::fs::read_to_string(root().join("bindings/conformance/medius.json")).unwrap();
    serde_json::from_str::<Value>(&text)
        .unwrap()
        .as_array()
        .unwrap()
        .clone()
}

#[test]
fn record_what_the_medius_crate_does() {
    // a recording that fails halfway mustn't leave the last one behind, the runners would pass against it
    let path = root().join("target/conformance/medius-expected.json");
    if path.exists() {
        std::fs::remove_file(&path).unwrap();
    }
    let expected: Vec<Value> = scenarios().iter().map(run).collect();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, serde_json::to_string_pretty(&expected).unwrap()).unwrap();
}

#[test]
#[should_panic(expected = "only runs on a real box")]
fn a_call_held_on_hardware_never_reaches_the_mock() {
    // a mock device's reconnect would rescan the real serial ports and take whatever box is plugged in
    assert!(on_hardware().contains("Device::reconnect"));
    // open stands in for it, so this can't reach a box even with the check gone
    run(&json!({"name": "a call only a real box runs", "steps": [{"call": "open"}]}));
}

#[test]
fn nothing_the_box_gets_after_it_restarts_counts() {
    let mock = medius_mock_new();
    let mut device = std::ptr::null_mut();
    assert!(unsafe { medius_device_open_mock(mock, &mut device) } == MediusStatus::Ok);
    let (mut seen, mut sent, mut restarted) =
        (unsafe { medius_mock_recorded(mock) }, Vec::new(), false);
    let mut version: MediusVersion = unsafe { std::mem::zeroed() };
    unsafe {
        medius_device_factory_reset(device);
        medius_device_query_version(device, &mut version);
    }
    // the query and whatever medius sends to recover come after the restart
    assert_eq!(
        frames(mock, &mut seen, &mut sent, &mut restarted),
        json!([{"type": "Catch", "payload": "ffffff000000"}, {"type": "Reset", "payload": "01"}])
    );
    assert!(restarted);
    unsafe {
        medius_device_free(device);
        medius_mock_free(mock);
    }
}

#[test]
#[should_panic(expected = "a restart has to be its last step")]
fn a_scenario_cant_go_on_after_the_box_restarts() {
    run(
        &json!({"name": "a step after a restart", "steps": [{"call": "factory_reset"}, {"call": "query_version"}]}),
    );
}

#[test]
#[should_panic(expected = "a restart has to be its last step")]
fn a_scenario_cant_go_on_after_the_mocks_own_restart() {
    run(
        &json!({"name": "a step after the mock's restart", "steps": [{"on": "MockBox", "call": "restart"}, {"call": "query_version"}]}),
    );
}

// the ledger's rows on a handle, reached by a call on its C++ class, the others by a helper or a text step
const HANDLES: [&str; 9] = [
    "Device",
    "ClipHandle",
    "ClipBuilder",
    "ClipFrame",
    "MockBox",
    "EventStream",
    "InputStream",
    "LogStream",
    "Timeline",
];

#[test]
fn every_suite_row_of_the_ledger_and_every_generated_call_is_in_a_scenario() {
    let mut calls = std::collections::BTreeSet::new();
    let mut helpers = std::collections::BTreeSet::new();
    let mut texts = std::collections::BTreeSet::new();
    let mut defaults = std::collections::BTreeSet::new();
    for scenario in scenarios() {
        for setup in scenario["setup"].as_array().into_iter().flatten() {
            calls.insert((
                "MockBox".to_string(),
                setup["call"].as_str().unwrap().to_string(),
            ));
        }
        for step in scenario["steps"].as_array().unwrap() {
            if let Some(name) = step["helper"].as_str() {
                helpers.insert(name.to_string());
            } else if let Some(name) = step["text"].as_str() {
                texts.insert(name.to_string());
            } else if let Some(name) = step["default"].as_str() {
                defaults.insert(name.to_string());
            } else {
                let class = step["on"].as_str().unwrap_or("Device").to_string();
                calls.insert((class, step["call"].as_str().unwrap().to_string()));
            }
        }
    }
    let mut missing = Vec::new();
    for row in crate::surface::ledger()
        .iter()
        .filter(|row| row.fields[0] == "suite")
    {
        let reached = if let Some(ty) = row.rust.strip_suffix(": Display") {
            texts.contains(ty)
        } else if let Some(ty) = row.rust.strip_suffix(": Default") {
            defaults.contains(ty)
        } else if HANDLES
            .iter()
            .any(|handle| row.rust.starts_with(&format!("{handle}::")))
        {
            let (class, method) = row.fields[2].split_once("::").unwrap();
            let method = if class == method { "new" } else { method };
            calls.contains(&(class.to_string(), method.to_string()))
        } else {
            helpers.contains(&row.rust)
        };
        if !reached {
            missing.push(row.rust.clone());
        }
    }
    for (class, method) in generated::CALLS {
        if !calls.contains(&(class.to_string(), method.to_string()))
            && !on_hardware().contains(&format!("{class}::{method}"))
        {
            missing.push(format!("{class}::{method}"));
        }
    }
    assert!(
        missing.is_empty(),
        "no scenario in medius.json reaches these: {missing:?}"
    );
}
