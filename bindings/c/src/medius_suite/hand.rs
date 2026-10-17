// the records medius-capi's structs carry that the record table leaves to the bindings' own code, and the functions
// both bindings wrap by hand, in the same JSON and with the same arguments the C++ methods take

use super::*;

// the clip's autolock bits, the crate's own CLIP_LOCK ones, which medius.h names but never defines, like C++ has them
const AUTOLOCK: [(u8, &str); 5] = [
    (0x01, "Aim"),
    (0x02, "Wheel"),
    (0x04, "Buttons"),
    (0x08, "Keys"),
    (0x10, "Media"),
];

pub(super) fn usage_to_json(raw: &MediusUsage) -> Value {
    json!({"kind": enum_name("Class", raw.kind as i64), "id": raw.id})
}

pub(super) fn usage_from_json(value: &Value) -> Result<MediusUsage, Failure> {
    Ok(MediusUsage {
        kind: enum_value("Class", &value["kind"])? as u8,
        id: number(&value["id"], "Usage.id"),
    })
}

pub(super) fn lock_target_to_json(raw: &MediusLockTarget) -> Value {
    let usage = if raw.kind == MediusLockTargetKind::Usage as u8 {
        usage_to_json(&raw.usage)
    } else {
        Value::Null
    };
    json!({"kind": enum_name("LockTargetKind", raw.kind as i64), "usage": usage})
}

pub(super) fn lock_target_from_json(value: &Value) -> Result<MediusLockTarget, Failure> {
    let kind = enum_value("LockTargetKind", &value["kind"])?;
    if kind == MediusLockTargetKind::Usage as i64 {
        Ok(medius_lock_target_usage(usage_from_json(&value["usage"])?))
    } else {
        Ok(medius_lock_target_axis(kind as u8))
    }
}

// a motion argument by its kind, the cursor's dx and dy or the wheel's or the pan's delta
pub(super) fn motion_from_json(value: &Value) -> Result<MediusMotion, Failure> {
    Ok(match value["kind"].as_str() {
        Some("Cursor") => medius_motion_cursor(
            number(&value["dx"], "Motion.dx"),
            number(&value["dy"], "Motion.dy"),
        ),
        Some("Wheel") => medius_motion_wheel(number(&value["delta"], "Motion.delta")),
        Some("Pan") => medius_motion_pan(number(&value["delta"], "Motion.delta")),
        _ => panic!("a motion is Cursor, Wheel or Pan, the scenario has {value}"),
    })
}

pub(super) fn catch_filter_to_json(raw: &MediusCatchFilter) -> Value {
    json!({
        "class_": if raw.class == MEDIUS_CATCH_CLASS_ANY { Value::Null } else { enum_name("CatchClass", raw.class as i64) },
        "id": if raw.id == MEDIUS_CATCH_ID_ANY { Value::Null } else { Value::from(raw.id) },
        "direction": enum_name("Direction", raw.direction as i64),
        "capture": raw.capture,
    })
}

// a filter argument is the calls that build it, a constructor and then the modifiers, like
// [["watch_class", "Key"], ["with_direction", "Positive"]], so every binding builds it through its own API
pub(super) fn catch_filter_from_json(value: &Value) -> Result<MediusCatchFilter, Failure> {
    let calls = value
        .as_array()
        .unwrap_or_else(|| panic!("a filter is the calls that make it, the scenario has {value}"));
    let mut filter: Option<MediusCatchFilter> = None;
    for call in calls {
        let call = call.as_array().unwrap();
        let args = &call[1..];
        filter = Some(match (call[0].as_str().unwrap(), filter) {
            ("watch", None) => medius_catch_filter_watch(usage_from_json(&args[0])?),
            ("watch_axis", None) => {
                medius_catch_filter_watch_axis(enum_value("Axis", &args[0])? as u8)
            }
            ("watch_class", None) => {
                medius_catch_filter_watch_class(enum_value("Class", &args[0])? as u8)
            }
            ("watch_axes", None) => medius_catch_filter_watch_axes(),
            ("traffic", None) => medius_catch_filter_traffic(
                enum_value("TrafficClass", &args[0])? as u8,
                number(&args[1], "traffic id"),
            ),
            ("traffic_of", None) => {
                medius_catch_filter_traffic_class(enum_value("TrafficClass", &args[0])? as u8)
            }
            ("everything", None) => medius_catch_filter_everything(),
            ("with_direction", Some(f)) => {
                medius_catch_filter_with_direction(f, enum_value("Direction", &args[0])? as u8)
            }
            ("with_capture", Some(f)) => {
                medius_catch_filter_with_capture(f, number(&args[0], "capture"))
            }
            ("on_press", Some(f)) => medius_catch_filter_on_press(f),
            ("on_release", Some(f)) => medius_catch_filter_on_release(f),
            ("inbound", Some(f)) => medius_catch_filter_inbound(f),
            ("outbound", Some(f)) => medius_catch_filter_outbound(f),
            (name, _) => panic!("a filter can't start with or take {name}"),
        });
    }
    Ok(filter.unwrap_or_else(|| panic!("a filter needs a constructor")))
}

pub(super) fn emit_pace_to_json(mode: MediusEmitMode, hz: u16) -> Value {
    let fixed = mode == MediusEmitMode::Fixed;
    json!({"mode": enum_name("EmitMode", mode as i64), "hz": if fixed { hz } else { 0 }})
}

pub(super) fn emit_pace_status_to_json(raw: &MediusEmitPaceStatus) -> Value {
    json!({
        "mode": emit_pace_to_json(raw.mode, raw.fixed_hz),
        "resolved_hz": raw.resolved_hz,
        "force_hz": if raw.force_hz == 0 { Value::Null } else { Value::from(raw.force_hz) },
        "advertised_hz": raw.advertised_hz,
        "force_active": raw.force_active != 0,
    })
}

pub(super) fn clip_settings_to_json(raw: &MediusClipSettings) -> Value {
    let autolock: Vec<Value> = AUTOLOCK
        .iter()
        .filter(|(bit, _)| raw.autolock_bits & bit != 0)
        .map(|(_, name)| Value::from(*name))
        .collect();
    let triggers = &raw.triggers[..usize::from(raw.n).min(raw.triggers.len())];
    let packets = &raw.packet_triggers[..usize::from(raw.packet_n).min(raw.packet_triggers.len())];
    json!({
        "autolock": autolock,
        "loop": raw.loop_ != 0,
        "retain": raw.retain != 0,
        "finalized": raw.finalized != 0,
        "ride": raw.ride != 0,
        "triggers": triggers.iter().map(clip_trigger_to_json).collect::<Vec<_>>(),
        "packet_triggers": packets.iter().map(clip_packet_trigger_to_json).collect::<Vec<_>>(),
    })
}

pub(super) fn clip_settings_from_json(value: &Value) -> Result<MediusClipSettings, Failure> {
    // plain data with no enum in it, so zero is a valid value
    let mut raw: MediusClipSettings = unsafe { std::mem::zeroed() };
    for blanket in value["autolock"].as_array().into_iter().flatten() {
        let name = enum_name("Blanket", enum_value("Blanket", blanket)?);
        raw.autolock_bits |= AUTOLOCK.iter().find(|(_, each)| *each == name).unwrap().0;
    }
    raw.loop_ = u8::from(flag(&value["loop"]));
    raw.retain = u8::from(flag(&value["retain"]));
    raw.finalized = u8::from(flag(&value["finalized"]));
    raw.ride = u8::from(flag(&value["ride"]));
    raw.n = fill_list(
        &mut raw.triggers,
        &value["triggers"],
        "ClipSettings.triggers",
        clip_trigger_from_json,
    )?
    .try_into()
    .unwrap();
    raw.packet_n = fill_list(
        &mut raw.packet_triggers,
        &value["packet_triggers"],
        "ClipSettings.packet_triggers",
        clip_packet_trigger_from_json,
    )?
    .try_into()
    .unwrap();
    Ok(raw)
}

pub(super) fn catch_event_to_json(raw: &MediusCatchEvent) -> Value {
    let data = match raw.kind {
        MediusCatchEventKind::Motion => motion_event_to_json(unsafe { &raw.data.motion }),
        MediusCatchEventKind::Usages => usage_event_to_json(unsafe { &raw.data.usages }),
        MediusCatchEventKind::Traffic => traffic_event_to_json(unsafe { &raw.data.traffic }),
    };
    json!({
        "kind": enum_name("CatchEventKind", raw.kind as i64),
        "ts_us": raw.ts_us,
        "clock": enum_name("ClockDomain", raw.clock as i64),
        "data": data,
    })
}

// a catch event going back only needs what a timeline reads, the stamp and the chip
fn stamp_from_json(value: &Value) -> Result<MediusCatchEvent, Failure> {
    // plain data and every enum in it has a member at 0, so zero is a valid value
    let mut raw: MediusCatchEvent = unsafe { std::mem::zeroed() };
    raw.ts_us = number(&value["ts_us"], "the stamp");
    raw.clock = if enum_value("ClockDomain", &value["clock"])? == 1 {
        MediusClockDomain::DeviceChip
    } else {
        MediusClockDomain::HostChip
    };
    Ok(raw)
}

pub(super) fn input_event_to_json(raw: &MediusInputEvent) -> Value {
    let usage = if raw.kind == MediusInputKind::Motion {
        Value::Null
    } else {
        usage_to_json(&raw.usage)
    };
    json!({
        "kind": enum_name("InputKind", raw.kind as i64),
        "ts_us": raw.ts_us,
        "clock": enum_name("ClockDomain", raw.clock as i64),
        "usage": usage,
        "dx": raw.dx,
        "dy": raw.dy,
        "dz": raw.dz,
        "pan": raw.pan,
    })
}

// a window in whole ms from 1 to 65535 like the Rust API, since 0 is off on the wire
fn window(value: &Value) -> u16 {
    if value.is_null() {
        return 0;
    }
    number::<i64>(value, "a window").clamp(1, 65535) as u16
}

fn filters(value: &Value) -> Result<Vec<MediusCatchFilter>, Failure> {
    value
        .as_array()
        .unwrap_or_else(|| panic!("a list of filters, the scenario has {value}"))
        .iter()
        .map(catch_filter_from_json)
        .collect()
}

fn received<T>(got: bool, out: &T, to_json: fn(&T) -> Value) -> Outcome {
    Ok(if got { to_json(out) } else { Value::Null })
}

/// a function both bindings wrap by hand, by its C++ class and method, None for one the suite doesn't know
pub(super) fn call_by_hand(
    ctx: &mut Context,
    class: &str,
    method: &str,
    args: &[Value],
) -> Option<Outcome> {
    Some(match (class, method) {
        ("Device", "clip") => status(
            unsafe { medius_device_clip(ctx.device, &mut ctx.clip) },
            || Value::Null,
        ),
        ("Device", "catch_events") => filters(&args[0]).and_then(|filters| {
            status(
                unsafe {
                    medius_device_catch_events(
                        ctx.device,
                        filters.as_ptr(),
                        filters.len(),
                        &mut ctx.events,
                    )
                },
                || Value::Null,
            )
        }),
        ("Device", "input_events") => filters(&args[0]).and_then(|filters| {
            status(
                unsafe {
                    medius_device_input_events(
                        ctx.device,
                        filters.as_ptr(),
                        filters.len(),
                        &mut ctx.inputs,
                    )
                },
                || Value::Null,
            )
        }),
        ("Device", "logs") => status(
            unsafe { medius_device_logs(ctx.device, &mut ctx.logs) },
            || Value::Null,
        ),
        ("Device", "transfer") => setup_from_json(&args[1]).and_then(|setup| {
            let data = unhex(&args[2], "transfer data");
            let mut out: MediusTransferOutcome = unsafe { std::mem::zeroed() };
            let code = if args[3].is_null() {
                unsafe {
                    medius_device_transfer(
                        ctx.device,
                        number(&args[0], "ep"),
                        setup,
                        data.as_ptr(),
                        data.len(),
                        &mut out,
                    )
                }
            } else {
                unsafe {
                    medius_device_transfer_timeout(
                        ctx.device,
                        number(&args[0], "ep"),
                        setup,
                        data.as_ptr(),
                        data.len(),
                        number(&args[3], "timeout"),
                        &mut out,
                    )
                }
            };
            status(code, || transfer_outcome_to_json(&out))
        }),
        // the payload isn't read, so one past what medius.h holds still removes like in Rust
        ("Device", "remove_rewrite") => {
            let mut rule = args[0].clone();
            rule["payload"] = Value::from("");
            rewrite_rule_from_json(&rule).and_then(|rule| {
                status(
                    unsafe { medius_device_remove_rewrite(ctx.device, &rule) },
                    || Value::Null,
                )
            })
        }
        ("Device", "set_movement_riding") => status(
            unsafe {
                medius_device_set_movement_riding(
                    ctx.device,
                    !args[0].is_null(),
                    u32::from(window(&args[0])),
                )
            },
            || Value::Null,
        ),
        ("Device", "query_movement_riding") => {
            let mut enabled = false;
            let mut window_ms = 0u32;
            status(
                unsafe {
                    medius_device_query_movement_riding(ctx.device, &mut enabled, &mut window_ms)
                },
                || {
                    if enabled {
                        Value::from(window_ms)
                    } else {
                        Value::Null
                    }
                },
            )
        }
        ("Device", "set_emit_pace") => enum_value("EmitMode", &args[0]["mode"]).and_then(|mode| {
            let force = if args[1].is_null() {
                0
            } else {
                number(&args[1], "force_hz")
            };
            status(
                unsafe {
                    medius_device_set_emit_pace(
                        ctx.device,
                        mode as u8,
                        number(&args[0]["hz"], "hz"),
                        force,
                    )
                },
                || Value::Null,
            )
        }),
        ("Device", "set_bearing") => enum_value("BearingMode", &args[1]).and_then(|mode| {
            status(
                unsafe { medius_device_set_bearing(ctx.device, window(&args[0]), mode as u8) },
                || Value::Null,
            )
        }),
        ("Device", "stage_firmware") => enum_value("UpdateTarget", &args[0]).and_then(|target| {
            let image = unhex(&args[1], "the image");
            status(
                unsafe {
                    medius_device_stage_firmware(
                        ctx.device,
                        target as u8,
                        image.as_ptr(),
                        image.len(),
                        None,
                        std::ptr::null_mut(),
                    )
                },
                || Value::Null,
            )
        }),
        ("Device", "update_firmware") => enum_value("UpdateTarget", &args[0]).and_then(|target| {
            let image = unhex(&args[1], "the image");
            status(
                unsafe {
                    medius_device_update_firmware(
                        ctx.device,
                        target as u8,
                        image.as_ptr(),
                        image.len(),
                        None,
                        std::ptr::null_mut(),
                    )
                },
                || Value::Null,
            )
        }),
        ("Clip", "set_autolock") => {
            let groups: Result<Vec<u8>, Failure> = args[0]
                .as_array()
                .unwrap()
                .iter()
                .map(|blanket| enum_value("Blanket", blanket).map(|value| value as u8))
                .collect();
            groups.and_then(|groups| {
                status(
                    unsafe { medius_clip_set_autolock(ctx.clip, groups.as_ptr(), groups.len()) },
                    || Value::Null,
                )
            })
        }
        // only the key is read, the other fields go over as the C++ wrapper sends them
        ("Clip", "unbind_packet") => {
            let mut key = args[0].clone();
            key["action"] = Value::from("Start");
            key["consume"] = Value::from(false);
            key["once_per_run"] = Value::from(false);
            key["selector_len"] = Value::from(0);
            key["hits"] = Value::from(0);
            clip_packet_trigger_from_json(&key).and_then(|key| {
                status(unsafe { medius_clip_unbind_packet(ctx.clip, &key) }, || {
                    Value::Null
                })
            })
        }
        ("MockBox", "set_movement_riding") => {
            unsafe {
                medius_mock_set_movement_riding(
                    ctx.mock,
                    !args[0].is_null(),
                    u32::from(window(&args[0])),
                )
            };
            Ok(Value::Null)
        }
        // the mock takes the bearing record as it is, where the device takes a window it saturates
        ("MockBox", "set_bearing") => bearing_from_json(&args[0]).map(|bearing| {
            unsafe { medius_mock_set_bearing(ctx.mock, bearing.window_ms, bearing.mode as u8) };
            Value::Null
        }),
        ("MockBox", "set_emit_pace") => enum_value("EmitMode", &args[0]["mode"]).map(|mode| {
            let force = if args[1].is_null() {
                0
            } else {
                number(&args[1], "force_hz")
            };
            unsafe {
                medius_mock_set_emit_pace(ctx.mock, mode as u8, number(&args[0]["hz"], "hz"), force)
            };
            Value::Null
        }),
        ("MockBox", "clip_packet") => enum_value("TrafficClass", &args[0]).and_then(|class| {
            enum_value("Direction", &args[2]).map(|direction| {
                let head = unhex(&args[3], "the packet head");
                let mut action = 0u8;
                let mut consumed = false;
                let fired = unsafe {
                    medius_mock_clip_packet(
                        ctx.mock,
                        class as u8,
                        number(&args[1], "id"),
                        direction as u8,
                        head.as_ptr(),
                        head.len(),
                        &mut action,
                        &mut consumed,
                    )
                };
                let action = if fired {
                    enum_name("ClipAction", i64::from(action))
                } else {
                    Value::Null
                };
                json!({"action": action, "consumed": consumed})
            })
        }),
        ("MockBox", "push_raw") => {
            let bytes = unhex(&args[0], "the raw bytes");
            unsafe { medius_mock_push_raw(ctx.mock, bytes.as_ptr(), bytes.len()) };
            Ok(Value::Null)
        }
        ("EventStream", "recv") => {
            let mut out: MediusCatchEvent = unsafe { std::mem::zeroed() };
            let got = unsafe {
                medius_event_stream_recv_timeout(ctx.events, number(&args[0], "timeout"), &mut out)
            };
            received(got, &out, catch_event_to_json)
        }
        ("EventStream", "try_recv") => {
            let mut out: MediusCatchEvent = unsafe { std::mem::zeroed() };
            let got = unsafe { medius_event_stream_try_recv(ctx.events, &mut out) };
            received(got, &out, catch_event_to_json)
        }
        ("EventStream", "dropped") => Ok(Value::from(unsafe {
            medius_event_stream_dropped(ctx.events)
        })),
        ("EventStream", "is_connected") => Ok(Value::from(unsafe {
            medius_event_stream_is_connected(ctx.events)
        })),
        ("InputStream", "recv") => {
            let mut out: MediusInputEvent = unsafe { std::mem::zeroed() };
            let got = unsafe {
                medius_input_stream_recv_timeout(ctx.inputs, number(&args[0], "timeout"), &mut out)
            };
            received(got, &out, input_event_to_json)
        }
        ("InputStream", "try_recv") => {
            let mut out: MediusInputEvent = unsafe { std::mem::zeroed() };
            let got = unsafe { medius_input_stream_try_recv(ctx.inputs, &mut out) };
            received(got, &out, input_event_to_json)
        }
        ("InputStream", "dropped") => Ok(Value::from(unsafe {
            medius_input_stream_dropped(ctx.inputs)
        })),
        ("InputStream", "is_connected") => Ok(Value::from(unsafe {
            medius_input_stream_is_connected(ctx.inputs)
        })),
        ("InputStream", "held") => enum_value("Class", &args[0]).map(|class| {
            let mut out = [MediusUsage { kind: 0, id: 0 }; 64];
            let n = unsafe {
                medius_input_stream_held(ctx.inputs, class as u8, out.as_mut_ptr(), out.len())
            };
            Value::Array(out[..n.min(out.len())].iter().map(usage_to_json).collect())
        }),
        ("LogStream", "recv") => {
            let mut out: MediusLogLine = unsafe { std::mem::zeroed() };
            let got = unsafe {
                medius_log_stream_recv_timeout(ctx.logs, number(&args[0], "timeout"), &mut out)
            };
            received(got, &out, log_line_to_json)
        }
        ("LogStream", "try_recv") => {
            let mut out: MediusLogLine = unsafe { std::mem::zeroed() };
            let got = unsafe { medius_log_stream_try_recv(ctx.logs, &mut out) };
            received(got, &out, log_line_to_json)
        }
        ("LogStream", "is_connected") => Ok(Value::from(unsafe {
            medius_log_stream_is_connected(ctx.logs)
        })),
        ("Timeline", "observe") => stamp_from_json(&args[0]).map(|event| {
            let mut out: MediusStamped = unsafe { std::mem::zeroed() };
            unsafe {
                medius_timeline_observe(ctx.timeline, &event, number(&args[1], "now_ns"), &mut out)
            };
            stamped_to_json(&out)
        }),
        ("Timeline", "observe_stamp") => {
            let event = json!({"ts_us": args[0], "clock": args[1]});
            return call_by_hand(ctx, "Timeline", "observe", &[event, args[2].clone()]);
        }
        // a fresh one in place of the scenario's
        ("ClipBuilder", "new") => {
            unsafe { medius_clip_builder_free(ctx.builder) };
            ctx.builder = medius_clip_builder_new();
            Ok(Value::Null)
        }
        ("ClipFrame", "new") => {
            unsafe { medius_clip_frame_free(ctx.frame) };
            ctx.frame = medius_clip_frame_new();
            Ok(Value::Null)
        }
        ("Timeline", "new") => {
            unsafe { medius_timeline_free(ctx.timeline) };
            ctx.timeline = medius_timeline_new();
            Ok(Value::Null)
        }
        _ => return None,
    })
}
