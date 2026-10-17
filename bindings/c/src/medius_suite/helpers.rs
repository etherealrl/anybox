// the crate's helpers and Display texts, through the crate itself, since medius.h has no function for most of them.
// the ones a binding reaches through a medius-capi function (the caps, a rate, the locks, a clip status) go through
// that function instead, which is the crate's own answer too. every value comes in and goes out in the suite's JSON

use std::time::Duration;

use medius as m;

use super::*;

// an enum argument a helper takes, which a scenario only ever gives a name or number of
fn member(name: &str, value: &Value) -> u8 {
    match enum_value(name, value) {
        Ok(number) => number as u8,
        Err(_) => panic!("{name} has no {value}"),
    }
}

fn direction(value: &Value) -> m::Direction {
    m::Direction::from_u8(member("Direction", value)).unwrap()
}

fn class(value: &Value) -> m::Class {
    m::Class::from_u8(member("Class", value)).unwrap()
}

fn catch_class(value: &Value) -> m::CatchClass {
    m::CatchClass::from_u8(member("CatchClass", value)).unwrap()
}

fn traffic_class(value: &Value) -> m::TrafficClass {
    m::TrafficClass::try_from(m::CatchClass::from_u8(member("TrafficClass", value)).unwrap())
        .unwrap()
}

fn axis(value: &Value) -> m::Axis {
    m::Axis::from_u16(u16::from(member("Axis", value))).unwrap()
}

fn clock(value: &Value) -> m::ClockDomain {
    match value.as_str() {
        Some("HostChip") => m::ClockDomain::HostChip,
        Some("DeviceChip") => m::ClockDomain::DeviceChip,
        _ => panic!("a clock domain is HostChip or DeviceChip, the scenario has {value}"),
    }
}

fn usage(value: &Value) -> m::Usage {
    m::Usage::new(class(&value["kind"]), number(&value["id"], "Usage.id"))
}

fn lock_target(value: &Value) -> m::LockTarget {
    match value["kind"].as_str() {
        Some("Usage") => m::LockTarget::Usage(usage(&value["usage"])),
        Some(_) => m::LockTarget::Axis(axis(&value["kind"])),
        None => panic!("a lock target has a kind, the scenario has {value}"),
    }
}

fn clock_estimate(value: &Value) -> m::ClockEstimate {
    m::ClockEstimate {
        offset_us: number(&value["offset_us"], "offset_us"),
        rate_ppb: value["rate_ppb"].as_i64().map(|rate| rate as i32),
        delay_us: number(&value["delay_us"], "delay_us"),
        age: value["age_ms"].as_u64().map(Duration::from_millis),
    }
}

fn chip_firmware(value: &Value) -> m::ChipFirmware {
    m::ChipFirmware {
        major: number(&value["major"], "major"),
        minor: number(&value["minor"], "minor"),
        patch: number(&value["patch"], "patch"),
        slot: number(&value["slot"], "slot"),
        state: m::ImageState::from_u8(member("ImageState", &value["state"])),
    }
}

fn device_info(value: &Value) -> m::DeviceInfo {
    m::DeviceInfo {
        vid: number(&value["vid"], "vid"),
        pid: number(&value["pid"], "pid"),
        bcd_device: number(&value["bcd_device"], "bcd_device"),
        bcd_usb: number(&value["bcd_usb"], "bcd_usb"),
        has_serial: flag(&value["has_serial"]),
        has_bos: flag(&value["has_bos"]),
        kind: match value["kind"].as_str() {
            Some("Keyboard") => m::DeviceKind::Keyboard,
            Some("Mouse") => m::DeviceKind::Mouse,
            _ => m::DeviceKind::Unknown,
        },
        product: value["product"].as_str().unwrap().to_string(),
    }
}

fn version(value: &Value) -> m::Version {
    m::Version {
        proto_ver: number(&value["proto_ver"], "proto_ver"),
        fw_major: number(&value["fw_major"], "fw_major"),
        fw_minor: number(&value["fw_minor"], "fw_minor"),
        fw_patch: number(&value["fw_patch"], "fw_patch"),
        mac: unhex(&value["mac"], "mac").try_into().unwrap(),
        name: value["name"].as_str().unwrap().to_string(),
    }
}

fn setup(value: &Value) -> m::Setup {
    m::Setup {
        request_type: number(&value["request_type"], "request_type"),
        request: number(&value["request"], "request"),
        value: number(&value["value"], "value"),
        index: number(&value["index"], "index"),
        length: number(&value["length"], "length"),
    }
}

fn traffic_event(value: &Value) -> m::TrafficEvent {
    m::TrafficEvent {
        ts_us: 0,
        clock: m::ClockDomain::HostChip,
        class: catch_class(&value["class_"]),
        id: number(&value["id"], "id"),
        direction: direction(&value["direction"]),
        flags: number(&value["flags"], "flags"),
        true_len: number(&value["true_len"], "true_len"),
        bytes: unhex(&value["bytes"], "bytes"),
    }
}

fn usage_snapshot(value: &Value) -> m::UsageSnapshot {
    m::UsageSnapshot {
        ts_us: 0,
        clock: m::ClockDomain::HostChip,
        class: class(&value["class_"]),
        direction: direction(&value["direction"]),
        usages: value["usages"]
            .as_array()
            .unwrap()
            .iter()
            .map(usage)
            .collect(),
    }
}

fn motion_event(value: &Value) -> m::MotionEvent {
    m::MotionEvent {
        ts_us: 0,
        clock: m::ClockDomain::HostChip,
        dx: number(&value["dx"], "dx"),
        dy: number(&value["dy"], "dy"),
        dz: number(&value["dz"], "dz"),
        pan: number(&value["pan"], "pan"),
    }
}

// a catch event as a stream gives it, the kind, the stamp and the event of that kind
fn catch_event(value: &Value) -> m::CatchEvent {
    let data = &value["data"];
    let (ts_us, clock_domain) = (number(&value["ts_us"], "ts_us"), clock(&value["clock"]));
    match value["kind"].as_str() {
        Some("Motion") => m::CatchEvent::Motion(m::MotionEvent {
            ts_us,
            clock: clock_domain,
            ..motion_event(data)
        }),
        Some("Usages") => m::CatchEvent::Usages(m::UsageSnapshot {
            ts_us,
            clock: clock_domain,
            ..usage_snapshot(data)
        }),
        Some("Traffic") => m::CatchEvent::Traffic(m::TrafficEvent {
            ts_us,
            clock: clock_domain,
            ..traffic_event(data)
        }),
        _ => panic!("a catch event is Motion, Usages or Traffic, the scenario has {value}"),
    }
}

fn input(value: &Value) -> m::Input {
    match value["kind"].as_str() {
        Some("Press") => m::Input::Press(usage(&value["usage"])),
        Some("Release") => m::Input::Release(usage(&value["usage"])),
        Some("Motion") => m::Input::Motion {
            dx: number(&value["dx"], "dx"),
            dy: number(&value["dy"], "dy"),
            dz: number(&value["dz"], "dz"),
            pan: number(&value["pan"], "pan"),
        },
        _ => panic!("an input event is Press, Release or Motion, the scenario has {value}"),
    }
}

// a filter argument is the calls that build it, like the device calls take one
fn catch_filter(value: &Value) -> m::CatchFilter {
    let mut filter: Option<m::CatchFilter> = None;
    for call in value.as_array().unwrap() {
        let call = call.as_array().unwrap();
        let args = &call[1..];
        filter = Some(match (call[0].as_str().unwrap(), filter) {
            ("watch", None) => m::CatchFilter::watch(usage(&args[0])),
            ("watch_axis", None) => m::CatchFilter::watch_axis(axis(&args[0])),
            ("watch_class", None) => m::CatchFilter::watch_class(class(&args[0])),
            ("watch_axes", None) => m::CatchFilter::watch_axes(),
            ("traffic", None) => {
                m::CatchFilter::traffic(traffic_class(&args[0]), number(&args[1], "id"))
            }
            ("traffic_of", None) => m::CatchFilter::traffic_class(traffic_class(&args[0])),
            ("everything", None) => m::CatchFilter::everything(),
            ("with_direction", Some(f)) => f.with_direction(direction(&args[0])),
            ("with_capture", Some(f)) => {
                f.with_capture(m::Capture::from_u8(number(&args[0], "capture")))
            }
            ("on_press", Some(f)) => f.on_press(),
            ("on_release", Some(f)) => f.on_release(),
            ("inbound", Some(f)) => f.inbound(),
            ("outbound", Some(f)) => f.outbound(),
            (name, _) => panic!("a filter can't start with or take {name}"),
        });
    }
    filter.unwrap()
}

// a filter in the form the box's table lists it, which only the wire form builds
fn listed_filter(value: &Value) -> m::CatchFilter {
    let class = if value["class_"].is_null() {
        MEDIUS_CATCH_CLASS_ANY
    } else {
        member("CatchClass", &value["class_"])
    };
    let id = if value["id"].is_null() {
        MEDIUS_CATCH_ID_ANY
    } else {
        number(&value["id"], "id")
    };
    m::CatchFilter::from_wire(
        class,
        id,
        member("Direction", &value["direction"]),
        number(&value["capture"], "capture"),
    )
    .unwrap()
}

fn name_of(name: &str, value: u8) -> Value {
    enum_name(name, i64::from(value))
}

fn usage_json(usage: m::Usage) -> Value {
    json!({"kind": name_of("Class", usage.class.as_u8()), "id": usage.id})
}

fn lock_target_json(target: m::LockTarget) -> Value {
    match target {
        m::LockTarget::Axis(axis) => {
            json!({"kind": name_of("LockTargetKind", axis.as_u16() as u8), "usage": Value::Null})
        }
        m::LockTarget::Usage(usage) => json!({"kind": "Usage", "usage": usage_json(usage)}),
    }
}

fn catch_filter_json(filter: m::CatchFilter) -> Value {
    json!({
        "class_": filter.class().map_or(Value::Null, |class| name_of("CatchClass", class.as_u8())),
        "id": filter.id(),
        "direction": name_of("Direction", filter.direction().as_u8()),
        "capture": filter.capture().as_u8(),
    })
}

fn axes_json(axes: impl IntoIterator<Item = (m::Axis, i16)>) -> Value {
    Value::Array(
        axes.into_iter()
            .map(|(axis, delta)| json!([name_of("Axis", axis.as_u16() as u8), delta]))
            .collect(),
    )
}

fn patch_json(patch: &m::Patch) -> Value {
    json!({
        "section": name_of("PatchSection", patch.section.as_u8()),
        "cfg": patch.cfg,
        "index": patch.index,
        "offset": patch.offset,
        "bytes": hex(&patch.bytes),
    })
}

fn transform_json(transform: &m::Transform) -> Value {
    json!({
        "op": name_of("TransformOp", transform.op.as_u8()),
        "source": lock_target_json(transform.source),
        "dest": lock_target_json(transform.dest),
    })
}

fn bus_event_json(event: m::BusEvent) -> Value {
    let (kind, configuration, interface, alt) = match event {
        m::BusEvent::Reset => ("Reset", 0, 0, 0),
        m::BusEvent::Suspend => ("Suspend", 0, 0, 0),
        m::BusEvent::Resume => ("Resume", 0, 0, 0),
        m::BusEvent::Configured(configuration) => ("Configured", configuration, 0, 0),
        m::BusEvent::Deconfigured => ("Deconfigured", 0, 0, 0),
        m::BusEvent::SetInterface { interface, alt } => ("SetInterface", 0, interface, alt),
        m::BusEvent::DeviceAttached => ("DeviceAttached", 0, 0, 0),
        m::BusEvent::DeviceDetached => ("DeviceDetached", 0, 0, 0),
        m::BusEvent::CloneUp => ("CloneUp", 0, 0, 0),
        m::BusEvent::CloneDown => ("CloneDown", 0, 0, 0),
    };
    json!({"kind": kind, "configuration": configuration, "interface": interface, "alt": alt})
}

fn control_status_json(status: m::ControlStatus) -> Value {
    Value::from(match status {
        m::ControlStatus::Ok => "Ok",
        m::ControlStatus::Stalled => "Stalled",
        m::ControlStatus::Naked => "Naked",
        m::ControlStatus::Other(_) => "Other",
    })
}

/// a crate helper by its Rust name
pub(super) fn helper(name: &str, args: &[Value]) -> Outcome {
    let arg = |at: usize| &args[at];
    Ok(match name {
        "Bearing::is_live" => Value::from(arg(0)["window_ms"].is_u64()),
        "Caps::has_keyboard" => Value::from(medius_caps_has_keyboard(caps_from_json(arg(0))?)),
        "Caps::has_mouse" => Value::from(medius_caps_has_mouse(caps_from_json(arg(0))?)),
        "Caps::is_composite" => Value::from(medius_caps_is_composite(caps_from_json(arg(0))?)),
        "Capture::bytes" => Value::from(m::Capture::from_u8(number(arg(0), "capture")).bytes()),
        "Capture::widest" => Value::from(
            m::Capture::from_u8(number(arg(0), "capture"))
                .widest(m::Capture::from_u8(number(arg(1), "capture")))
                .as_u8(),
        ),
        "CatchClass: From<Class>" => {
            name_of("CatchClass", m::CatchClass::from(class(arg(0))).as_u8())
        }
        "CatchClass: From<TrafficClass>" => name_of(
            "CatchClass",
            m::CatchClass::from(traffic_class(arg(0))).as_u8(),
        ),
        "CatchClass::direction_meaning" => {
            Value::from(format!("{:?}", catch_class(arg(0)).direction_meaning()))
        }
        "CatchClass::is_input" => Value::from(catch_class(arg(0)).is_input()),
        "CatchClass::is_traffic" => Value::from(catch_class(arg(0)).is_traffic()),
        "CatchEvent::bytes" => hex(catch_event(arg(0)).bytes()),
        "CatchEvent::class" => name_of("CatchClass", catch_event(arg(0)).class().as_u8()),
        "CatchEvent::clock" => {
            Value::from(format!("{:?}", m::Timestamped::clock(&catch_event(arg(0)))))
        }
        "CatchEvent::direction" => name_of("Direction", catch_event(arg(0)).direction().as_u8()),
        "CatchEvent::id" => Value::from(catch_event(arg(0)).id()),
        "CatchEvent::ts_us" => Value::from(m::Timestamped::ts_us(&catch_event(arg(0)))),
        "CatchFilter::all_input" => Value::Array(
            m::CatchFilter::all_input()
                .into_iter()
                .map(catch_filter_json)
                .collect(),
        ),
        "CatchFilter::capture" => Value::from(catch_filter(arg(0)).capture().as_u8()),
        "CatchFilter::class" => catch_filter(arg(0))
            .class()
            .map_or(Value::Null, |class| name_of("CatchClass", class.as_u8())),
        "CatchFilter::direction" => name_of("Direction", catch_filter(arg(0)).direction().as_u8()),
        "CatchFilter::id" => Value::from(catch_filter(arg(0)).id()),
        "CatchFilter::same_address" => {
            Value::from(catch_filter(arg(0)).same_address(catch_filter(arg(1))))
        }
        "CatchFilter::everything"
        | "CatchFilter::watch"
        | "CatchFilter::watch_axes"
        | "CatchFilter::watch_axis"
        | "CatchFilter::watch_class"
        | "CatchFilter::traffic"
        | "CatchFilter::traffic_class" => {
            // the constructor's C++ name, the one filter arguments use
            let call = match name {
                "CatchFilter::traffic_class" => "traffic_of",
                other => &other["CatchFilter::".len()..],
            };
            let mut chain = vec![Value::from(call)];
            chain.extend(args.iter().cloned());
            catch_filter_json(catch_filter(&json!([chain])))
        }
        "CatchFilter::inbound"
        | "CatchFilter::outbound"
        | "CatchFilter::on_press"
        | "CatchFilter::on_release"
        | "CatchFilter::with_capture"
        | "CatchFilter::with_direction" => {
            let mut calls = arg(0).as_array().unwrap().clone();
            let mut modifier = vec![Value::from(&name["CatchFilter::".len()..])];
            modifier.extend(args[1..].iter().cloned());
            calls.push(Value::Array(modifier));
            catch_filter_json(catch_filter(&Value::Array(calls)))
        }
        "CatchState::is_empty" => {
            let state = m::CatchState {
                table_full: flag(&arg(0)["table_full"]),
                dropped: number(&arg(0)["dropped"], "dropped"),
                clock: clock_estimate(&arg(0)["clock"]),
                entries: arg(0)["entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|entry| m::CatchEntry {
                        filter: listed_filter(&entry["filter"]),
                        dropped: number(&entry["dropped"], "dropped"),
                    })
                    .collect(),
            };
            Value::from(state.is_empty())
        }
        "ClipStatus::is_held" => {
            let status = clip_status_from_json(arg(0))?;
            Value::from(unsafe { medius_clip_status_is_held(&status, usage_from_json(arg(1))?) })
        }
        "ClockEstimate::drift_us_over" => {
            // a Duration can't be negative, so the bindings refuse one
            let Some(elapsed) = arg(1).as_u64() else {
                return Err(Failure::Refused(format!("{} is no elapsed time", arg(1))));
            };
            Value::from(clock_estimate(arg(0)).drift_us_over(Duration::from_micros(elapsed)))
        }
        "ClockEstimate::error_bound_us" => Value::from(clock_estimate(arg(0)).error_bound_us()),
        "ClockEstimate::to_host_domain" => {
            Value::from(clock_estimate(arg(0)).to_host_domain(number(arg(1), "device_us")))
        }
        "DeviceInfo::is_cloned" => Value::from(device_info(arg(0)).is_cloned()),
        "Direction: From<Edge>" => {
            let edge = match arg(0).as_str() {
                Some("Both") => m::Edge::Both,
                Some("Press") => m::Edge::Press,
                Some("Release") => m::Edge::Release,
                _ => panic!("an edge is Both, Press or Release"),
            };
            name_of("Direction", m::Direction::from(edge).as_u8())
        }
        "Direction::admits" => Value::from(direction(arg(0)).admits(direction(arg(1)))),
        "Direction::is_relative" => Value::from(direction(arg(0)).is_relative()),
        "Direction::of_delta" => name_of(
            "Direction",
            m::Direction::of_delta(number(arg(0), "delta")).as_u8(),
        ),
        "FirmwareInfo::any_pending" => {
            let info = m::FirmwareInfo {
                device: chip_firmware(&arg(0)["device"]),
                host: (!arg(0)["host"].is_null()).then(|| chip_firmware(&arg(0)["host"])),
                slot_size: number(&arg(0)["slot_size"], "slot_size"),
                device_staged: flag(&arg(0)["device_staged"]),
                host_staged: flag(&arg(0)["host_staged"]),
            };
            Value::from(info.any_pending())
        }
        "ImageState::is_pending" => {
            Value::from(m::ImageState::from_u8(member("ImageState", arg(0))).is_pending())
        }
        "Input::axes" => axes_json(input(arg(0)).axes()),
        "Input::direction" => name_of("Direction", input(arg(0)).direction().as_u8()),
        "Input::is_press" => Value::from(input(arg(0)).is_press()),
        "Input::is_release" => Value::from(input(arg(0)).is_release()),
        "Input::usage" => input(arg(0)).usage().map_or(Value::Null, usage_json),
        "Key::is_modifier" => Value::from(m::Key::new(number(arg(0), "key")).is_modifier()),
        "LockEntry::is_block" => {
            let target = &arg(0)["target"];
            let entry = m::LockEntry {
                scope: if flag(&arg(0)["is_blanket"]) {
                    m::LockScope::Blanket(class(&target["usage"]["kind"]))
                } else {
                    m::LockScope::Target(lock_target(target))
                },
                direction: direction(&arg(0)["direction"]),
                scale: number(&arg(0)["scale"], "scale"),
            };
            Value::from(entry.is_block())
        }
        "LockTarget::as_axis" => lock_target(arg(0))
            .as_axis()
            .map_or(Value::Null, |axis| name_of("Axis", axis.as_u16() as u8)),
        "Locks::is_locked" => {
            let locks = locks_from_json(arg(0))?;
            let code = member("Direction", arg(2));
            Value::from(unsafe {
                medius_locks_is_locked(&locks, lock_target_from_json(arg(1))?, code)
            })
        }
        "Locks::scale_of" => {
            let locks = locks_from_json(arg(0))?;
            let code = member("Direction", arg(2));
            Value::from(unsafe {
                medius_locks_scale_of(&locks, lock_target_from_json(arg(1))?, code)
            })
        }
        "MotionEvent::all_axes" => axes_json(motion_event(arg(0)).all_axes()),
        "MotionEvent::axes" => axes_json(motion_event(arg(0)).axes()),
        "MouseCaps::is_composite" => {
            let caps = m::MouseCaps {
                n_buttons: number(&arg(0)["n_buttons"], "n_buttons"),
                has_x: flag(&arg(0)["has_x"]),
                has_y: flag(&arg(0)["has_y"]),
                has_wheel: flag(&arg(0)["has_wheel"]),
                pan: flag(&arg(0)["pan"]),
                has_report_id: flag(&arg(0)["has_report_id"]),
                n_hid: number(&arg(0)["n_hid"], "n_hid"),
            };
            Value::from(caps.is_composite())
        }
        "Patch::in_config" => patch_json(&m::Patch::in_config(
            number(arg(0), "cfg"),
            number(arg(1), "offset"),
            unhex(arg(2), "bytes"),
        )),
        "Patch::in_interface" => patch_json(&m::Patch::in_interface(
            number(arg(0), "cfg"),
            number(arg(1), "interface"),
            number(arg(2), "offset"),
            unhex(arg(3), "bytes"),
        )),
        "Patch::in_string" => patch_json(&m::Patch::in_string(
            number(arg(0), "index"),
            unhex(arg(1), "bytes"),
        )),
        "Patch::key" => {
            let key = m::Patch {
                section: m::PatchSection::from_u8(member("PatchSection", &arg(0)["section"]))
                    .unwrap(),
                cfg: number(&arg(0)["cfg"], "cfg"),
                index: number(&arg(0)["index"], "index"),
                offset: number(&arg(0)["offset"], "offset"),
                bytes: unhex(&arg(0)["bytes"], "bytes"),
            }
            .key();
            json!({"section": name_of("PatchSection", key.section.as_u8()), "cfg": key.cfg, "index": key.index, "offset": key.offset})
        }
        "Rate::native_hz" => {
            let mut hz = 0f32;
            let known = unsafe { medius_rate_native_hz(rate_from_json(arg(0))?, &mut hz) };
            // the bits, since the runners' JSON has no floats and the bits compare exactly
            if known {
                Value::from(hz.to_bits())
            } else {
                Value::Null
            }
        }
        "RewriteAction::carries_payload" => Value::from(
            m::RewriteAction::from_u8(member("RewriteAction", arg(0)))
                .unwrap()
                .carries_payload(),
        ),
        "RewriteAction::is_valid_for" => Value::from(
            m::RewriteAction::from_u8(member("RewriteAction", arg(0)))
                .unwrap()
                .is_valid_for(m::RewriteClass::from_u8(member("RewriteClass", arg(1))).unwrap()),
        ),
        "RewriteClass::is_control" => Value::from(
            m::RewriteClass::from_u8(member("RewriteClass", arg(0)))
                .unwrap()
                .is_control(),
        ),
        "RewriteRule::key" => {
            let rule = arg(0);
            let key = m::RewriteRule {
                class: m::RewriteClass::from_u8(member("RewriteClass", &rule["class_"])).unwrap(),
                id: number(&rule["id"], "id"),
                direction: direction(&rule["direction"]),
                action: m::RewriteAction::from_u8(member("RewriteAction", &rule["action"]))
                    .unwrap(),
                offset: number(&rule["offset"], "offset"),
                match_bytes: unhex(&rule["match_bytes"], "match_bytes"),
                mask: unhex(&rule["mask"], "mask"),
                payload: unhex(&rule["payload"], "payload"),
            }
            .key();
            json!({
                "class_": name_of("RewriteClass", key.class.as_u8()),
                "id": key.id,
                "direction": name_of("Direction", key.direction.as_u8()),
                "match_bytes": hex(&key.match_bytes),
                "mask": hex(&key.mask),
            })
        }
        "Setup::is_in" => Value::from(setup(arg(0)).is_in()),
        "Setup::to_bytes" => hex(&setup(arg(0)).to_bytes()),
        "TrafficClass: TryFrom<CatchClass>" => m::TrafficClass::try_from(catch_class(arg(0)))
            .map_or(Value::Null, |traffic| {
                name_of("TrafficClass", traffic.as_u8())
            }),
        "TrafficEvent::bulk_end_of_transfer" => {
            Value::from(traffic_event(arg(0)).bulk_end_of_transfer())
        }
        "TrafficEvent::bus_event" => traffic_event(arg(0))
            .bus_event()
            .map_or(Value::Null, bus_event_json),
        "TrafficEvent::control_status" => traffic_event(arg(0))
            .control_status()
            .map_or(Value::Null, control_status_json),
        "TrafficEvent::data" => hex(traffic_event(arg(0)).data()),
        "TrafficEvent::rule_acted" => Value::from(traffic_event(arg(0)).rule_acted()),
        "TrafficEvent::setup" => traffic_event(arg(0)).setup().map_or(Value::Null, hex),
        "TrafficEvent::transfer_status" => traffic_event(arg(0))
            .transfer_status()
            .map_or(Value::Null, |status| {
                name_of("TransferStatus", status.as_u8())
            }),
        "TrafficEvent::truncated" => Value::from(traffic_event(arg(0)).truncated()),
        "TrafficEvent::zlp" => Value::from(traffic_event(arg(0)).zlp()),
        "TransferOutcome::is_ok" | "TransferOutcome::ok_data" => {
            let outcome = m::TransferOutcome {
                status: m::TransferStatus::from_u8(member("TransferStatus", &arg(0)["status"])),
                data: unhex(&arg(0)["data"], "data"),
            };
            if name.ends_with("is_ok") {
                Value::from(outcome.is_ok())
            } else {
                outcome.ok_data().map_or(Value::Null, |data| hex(&data))
            }
        }
        "TransferStatus::is_ok" => {
            Value::from(m::TransferStatus::from_u8(member("TransferStatus", arg(0))).is_ok())
        }
        "Transform::key" => {
            let transform = m::Transform::new(
                m::TransformOp::from_u8(member("TransformOp", &arg(0)["op"])).unwrap(),
                lock_target(&arg(0)["source"]),
                lock_target(&arg(0)["dest"]),
            );
            let key = transform.key();
            json!({"source": lock_target_json(key.source), "dest": lock_target_json(key.dest)})
        }
        "Transform::remap" => transform_json(&m::Transform::remap(
            lock_target(arg(0)),
            lock_target(arg(1)),
        )),
        "Transform::swap" => transform_json(&m::Transform::swap(axis(arg(0)), axis(arg(1)))),
        "TransformOp::admits" => Value::from(
            m::TransformOp::from_u8(member("TransformOp", arg(0)))
                .unwrap()
                .admits(lock_target(arg(1)), lock_target(arg(2))),
        ),
        "UpdateProgress::percent" => Value::from(
            m::UpdateProgress {
                target: m::UpdateTarget::from_u8(member("UpdateTarget", &arg(0)["target"]))
                    .unwrap(),
                sent: number(&arg(0)["sent"], "sent"),
                total: number(&arg(0)["total"], "total"),
            }
            .percent(),
        ),
        "UsageSnapshot::is_held" => Value::from(usage_snapshot(arg(0)).is_held(usage(arg(1)))),
        "Version::mac_hex" => Value::from(version(arg(0)).mac_hex()),
        _ => panic!("the suite has no helper {name}"),
    })
}

/// a crate type's Display text
pub(super) fn text(name: &str, value: &Value) -> Outcome {
    Ok(Value::from(match name {
        "ChipFirmware" => chip_firmware(value).to_string(),
        "DeviceInfo" => device_info(value).to_string(),
        "DeviceKind" => device_info(
            &json!({"vid": 0, "pid": 0, "bcd_device": 0, "bcd_usb": 0, "has_serial": false,
            "has_bos": false, "kind": value, "product": ""}),
        )
        .kind
        .to_string(),
        "ImageState" => m::ImageState::from_u8(member("ImageState", value)).to_string(),
        "UpdateTarget" => m::UpdateTarget::from_u8(member("UpdateTarget", value))
            .unwrap()
            .to_string(),
        "Version" => version(value).to_string(),
        _ => panic!("the suite has no text for {name}"),
    }))
}

/// a crate type's Default in the suite's JSON, through medius-capi's own conversion where it has one, an enum by its
/// member's name
pub(super) fn default(name: &str) -> Outcome {
    Ok(match name {
        "Bearing" => bearing_to_json(&MediusBearing::from(m::Bearing::default())),
        "Caps" => caps_to_json(&MediusCaps::from(m::Caps::default())),
        "CountersSnapshot" => {
            counters_snapshot_to_json(&MediusCountersSnapshot::from(m::CountersSnapshot::default()))
        }
        "DeviceInfo" => device_info_to_json(&MediusDeviceInfo::from(m::DeviceInfo::default())),
        "EmitPaceStatus" => {
            emit_pace_status_to_json(&MediusEmitPaceStatus::from(m::EmitPaceStatus::default()))
        }
        "ImperfectStatus" => {
            imperfect_status_to_json(&MediusImperfectStatus::from(m::ImperfectStatus::default()))
        }
        "KbdCaps" => kbd_caps_to_json(&MediusKbdCaps::from(m::KbdCaps::default())),
        "MouseCaps" => mouse_caps_to_json(&MediusMouseCaps::from(m::MouseCaps::default())),
        "Patch" => patch_to_json(&MediusPatch::from(m::Patch::default())),
        "PatchSet" => patch_set_to_json(&MediusPatchSet::from(m::PatchSet::default())),
        "RenderStatus" => {
            render_status_to_json(&MediusRenderStatus::from(m::RenderStatus::default()))
        }
        "RewriteRule" => rewrite_rule_to_json(&MediusRewriteRule::from(m::RewriteRule::default())),
        "RewriteTable" => {
            rewrite_table_to_json(&MediusRewriteTable::from(m::RewriteTable::default()))
        }
        "SpreadStatus" => {
            spread_status_to_json(&MediusSpreadStatus::from(m::SpreadStatus::default()))
        }
        "Transforms" => transforms_to_json(&MediusTransforms::from(m::Transforms::default())),
        // medius-capi converts these three only inside its own calls, so they're written out here
        "ClipStatus" => {
            let status = m::ClipStatus::default();
            json!({
                "state": format!("{:?}", status.state),
                "free": status.free,
                "total": status.total,
                "played": status.played,
                "ticks": status.ticks,
                "underruns": status.underruns,
                "overruns": status.overruns,
                "seq_gaps": status.seq_gaps,
                "xfers": status.xfers,
                "xfer_errs": status.xfer_errs,
                "gated": status.gated,
                "held": status.held.into_iter().map(usage_json).collect::<Vec<_>>(),
            })
        }
        "Setup" => {
            let setup = m::Setup::default();
            json!({
                "request_type": setup.request_type,
                "request": setup.request,
                "value": setup.value,
                "index": setup.index,
                "length": setup.length,
            })
        }
        "ClipSettings" => {
            let settings = m::ClipSettings::default();
            assert!(
                settings.triggers.is_empty() && settings.packet_triggers.is_empty(),
                "the default clip settings have triggers now, write them out here"
            );
            json!({
                "autolock": settings.autolock.iter().map(|blanket| format!("{blanket:?}")).collect::<Vec<_>>(),
                "loop": settings.loop_,
                "retain": settings.retain,
                "finalized": settings.finalized,
                "ride": settings.ride,
                "triggers": [],
                "packet_triggers": [],
            })
        }
        "EmitPace" => match m::EmitPace::default() {
            m::EmitPace::Fixed(hz) => json!({"mode": "Fixed", "hz": hz}),
            other => json!({"mode": format!("{other:?}"), "hz": 0}),
        },
        // a builder and a frame hold nothing a binding can read but their length, which a new one has none of
        "ClipBuilder" => Value::from(m::ClipBuilder::default().byte_len()),
        "ClipFrame" => Value::from(m::ClipFrame::default().byte_len()),
        "Capture" => Value::from(m::Capture::default().as_u8()),
        "BearingMode" => Value::from(format!("{:?}", m::BearingMode::default())),
        "ClipState" => Value::from(format!("{:?}", m::ClipState::default())),
        "DeviceKind" => Value::from(format!("{:?}", m::DeviceKind::default())),
        "Direction" => Value::from(format!("{:?}", m::Direction::default())),
        "MoveTiming" => Value::from(format!("{:?}", m::MoveTiming::default())),
        "PatchSection" => Value::from(format!("{:?}", m::PatchSection::default())),
        "PendingMotion" => Value::from(format!("{:?}", m::PendingMotion::default())),
        "RenderMode" => Value::from(format!("{:?}", m::RenderMode::default())),
        "RewriteAction" => Value::from(format!("{:?}", m::RewriteAction::default())),
        "RewriteClass" => Value::from(format!("{:?}", m::RewriteClass::default())),
        _ => panic!("the suite has no default for {name}"),
    })
}
