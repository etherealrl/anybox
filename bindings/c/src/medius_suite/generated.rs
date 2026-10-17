// made by bindings/csharp/tools/gen-medius.cs from bindings/c/include/medius.h and its tables, don't edit it by hand.
// every record gets both directions and only some get used, and medius-capi marks some C functions unsafe and
// some not, which medius.h doesn't say
#![allow(dead_code, unused_unsafe)]

use serde_json::{Map, Value};

use super::*;

pub(super) fn setup_to_json(raw: &MediusSetup) -> Value {
    let mut out = Map::new();
    out.insert("request_type".into(), Value::from(raw.request_type));
    out.insert("request".into(), Value::from(raw.request));
    out.insert("value".into(), Value::from(raw.value));
    out.insert("index".into(), Value::from(raw.index));
    out.insert("length".into(), Value::from(raw.length));
    Value::Object(out)
}

pub(super) fn setup_from_json(value: &Value) -> Result<MediusSetup, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusSetup = unsafe { std::mem::zeroed() };
    raw.request_type = number(&value["request_type"], "MediusSetup.request_type");
    raw.request = number(&value["request"], "MediusSetup.request");
    raw.value = number(&value["value"], "MediusSetup.value");
    raw.index = number(&value["index"], "MediusSetup.index");
    raw.length = number(&value["length"], "MediusSetup.length");
    Ok(raw)
}

pub(super) fn clip_trigger_to_json(raw: &MediusClipTrigger) -> Value {
    let mut out = Map::new();
    out.insert("on".into(), usage_to_json(&raw.on));
    out.insert("edge".into(), enum_name("Edge", raw.edge as i64));
    out.insert("action".into(), enum_name("ClipAction", raw.action as i64));
    out.insert("consume".into(), Value::from(raw.consume != 0));
    Value::Object(out)
}

pub(super) fn clip_trigger_from_json(value: &Value) -> Result<MediusClipTrigger, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusClipTrigger = unsafe { std::mem::zeroed() };
    raw.on = usage_from_json(&value["on"])?;
    raw.edge = enum_value("Edge", &value["edge"])? as u8;
    raw.action = enum_value("ClipAction", &value["action"])? as u8;
    raw.consume = u8::from(flag(&value["consume"]));
    Ok(raw)
}

pub(super) fn clip_packet_trigger_to_json(raw: &MediusClipPacketTrigger) -> Value {
    let mut out = Map::new();
    out.insert("class_".into(), enum_name("TrafficClass", raw.class as i64));
    out.insert("id".into(), Value::from(raw.id));
    out.insert(
        "direction".into(),
        enum_name("Direction", raw.direction as i64),
    );
    out.insert("action".into(), enum_name("ClipAction", raw.action as i64));
    out.insert("consume".into(), Value::from(raw.consume != 0));
    out.insert("once_per_run".into(), Value::from(raw.once_per_run != 0));
    out.insert("selector_len".into(), Value::from(raw.selector_len));
    out.insert(
        "match_bytes".into(),
        hex(&raw.match_bytes[..usize::from(raw.match_len).min(raw.match_bytes.len())]),
    );
    out.insert(
        "mask".into(),
        hex(&raw.mask[..usize::from(raw.mask_len).min(raw.mask.len())]),
    );
    out.insert("hits".into(), Value::from(raw.hits));
    Value::Object(out)
}

pub(super) fn clip_packet_trigger_from_json(
    value: &Value,
) -> Result<MediusClipPacketTrigger, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusClipPacketTrigger = unsafe { std::mem::zeroed() };
    raw.class = enum_value("TrafficClass", &value["class_"])? as u8;
    raw.id = number(&value["id"], "MediusClipPacketTrigger.id");
    raw.direction = enum_value("Direction", &value["direction"])? as u8;
    raw.action = enum_value("ClipAction", &value["action"])? as u8;
    raw.consume = u8::from(flag(&value["consume"]));
    raw.once_per_run = u8::from(flag(&value["once_per_run"]));
    raw.selector_len = number(
        &value["selector_len"],
        "MediusClipPacketTrigger.selector_len",
    );
    raw.match_len = fill_match(
        &mut raw.match_bytes,
        &value["match_bytes"],
        "MediusClipPacketTrigger.match_bytes",
    )?;
    raw.mask_len = fill_match(
        &mut raw.mask,
        &value["mask"],
        "MediusClipPacketTrigger.mask",
    )?;
    raw.hits = number(&value["hits"], "MediusClipPacketTrigger.hits");
    Ok(raw)
}

pub(super) fn clip_status_to_json(raw: &MediusClipStatus) -> Value {
    let mut out = Map::new();
    out.insert("state".into(), enum_name("ClipState", raw.state as i64));
    out.insert("free".into(), Value::from(raw.free));
    out.insert("total".into(), Value::from(raw.total));
    out.insert("played".into(), Value::from(raw.played));
    out.insert("ticks".into(), Value::from(raw.ticks));
    out.insert("underruns".into(), Value::from(raw.underruns));
    out.insert("overruns".into(), Value::from(raw.overruns));
    out.insert("seq_gaps".into(), Value::from(raw.seq_gaps));
    out.insert("xfers".into(), Value::from(raw.xfers));
    out.insert("xfer_errs".into(), Value::from(raw.xfer_errs));
    out.insert("gated".into(), Value::from(raw.gated));
    out.insert(
        "held".into(),
        Value::Array(
            raw.held[..usize::from(raw.held_n).min(raw.held.len())]
                .iter()
                .map(usage_to_json)
                .collect(),
        ),
    );
    Value::Object(out)
}

pub(super) fn clip_status_from_json(value: &Value) -> Result<MediusClipStatus, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusClipStatus = unsafe { std::mem::zeroed() };
    raw.state = enum_value("ClipState", &value["state"])? as u8;
    raw.free = number(&value["free"], "MediusClipStatus.free");
    raw.total = number(&value["total"], "MediusClipStatus.total");
    raw.played = number(&value["played"], "MediusClipStatus.played");
    raw.ticks = number(&value["ticks"], "MediusClipStatus.ticks");
    raw.underruns = number(&value["underruns"], "MediusClipStatus.underruns");
    raw.overruns = number(&value["overruns"], "MediusClipStatus.overruns");
    raw.seq_gaps = number(&value["seq_gaps"], "MediusClipStatus.seq_gaps");
    raw.xfers = number(&value["xfers"], "MediusClipStatus.xfers");
    raw.xfer_errs = number(&value["xfer_errs"], "MediusClipStatus.xfer_errs");
    raw.gated = number(&value["gated"], "MediusClipStatus.gated");
    raw.held_n = fill_list(
        &mut raw.held,
        &value["held"],
        "MediusClipStatus.held",
        usage_from_json,
    )?
    .try_into()
    .unwrap();
    Ok(raw)
}

pub(super) fn port_info_to_json(raw: &MediusPortInfo) -> Value {
    let mut out = Map::new();
    out.insert("path".into(), text(&raw.path));
    out.insert("vid".into(), Value::from(raw.vid));
    out.insert("pid".into(), Value::from(raw.pid));
    out.insert(
        "serial".into(),
        if raw.has_serial != 0 {
            text(&raw.serial)
        } else {
            Value::Null
        },
    );
    Value::Object(out)
}

pub(super) fn port_info_from_json(value: &Value) -> Result<MediusPortInfo, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusPortInfo = unsafe { std::mem::zeroed() };
    write_text(&mut raw.path, &value["path"]);
    raw.vid = number(&value["vid"], "MediusPortInfo.vid");
    raw.pid = number(&value["pid"], "MediusPortInfo.pid");
    raw.has_serial = u8::from(!value["serial"].is_null());
    write_text(&mut raw.serial, &value["serial"]);
    Ok(raw)
}

pub(super) fn version_to_json(raw: &MediusVersion) -> Value {
    let mut out = Map::new();
    out.insert("proto_ver".into(), Value::from(raw.proto_ver));
    out.insert("fw_major".into(), Value::from(raw.fw_major));
    out.insert("fw_minor".into(), Value::from(raw.fw_minor));
    out.insert("fw_patch".into(), Value::from(raw.fw_patch));
    out.insert("mac".into(), hex(&raw.mac));
    out.insert("name".into(), text(&raw.name));
    Value::Object(out)
}

pub(super) fn version_from_json(value: &Value) -> Result<MediusVersion, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusVersion = unsafe { std::mem::zeroed() };
    raw.proto_ver = number(&value["proto_ver"], "MediusVersion.proto_ver");
    raw.fw_major = number(&value["fw_major"], "MediusVersion.fw_major");
    raw.fw_minor = number(&value["fw_minor"], "MediusVersion.fw_minor");
    raw.fw_patch = number(&value["fw_patch"], "MediusVersion.fw_patch");
    raw.mac = exact(&value["mac"], "MediusVersion.mac")?;
    write_text(&mut raw.name, &value["name"]);
    Ok(raw)
}

pub(super) fn device_info_to_json(raw: &MediusDeviceInfo) -> Value {
    let mut out = Map::new();
    out.insert("vid".into(), Value::from(raw.vid));
    out.insert("pid".into(), Value::from(raw.pid));
    out.insert("bcd_device".into(), Value::from(raw.bcd_device));
    out.insert("bcd_usb".into(), Value::from(raw.bcd_usb));
    out.insert("has_serial".into(), Value::from(raw.has_serial != 0));
    out.insert("has_bos".into(), Value::from(raw.has_bos != 0));
    out.insert("kind".into(), enum_name("DeviceKind", raw.kind as i64));
    out.insert("product".into(), text(&raw.product));
    Value::Object(out)
}

pub(super) fn device_info_from_json(value: &Value) -> Result<MediusDeviceInfo, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusDeviceInfo = unsafe { std::mem::zeroed() };
    raw.vid = number(&value["vid"], "MediusDeviceInfo.vid");
    raw.pid = number(&value["pid"], "MediusDeviceInfo.pid");
    raw.bcd_device = number(&value["bcd_device"], "MediusDeviceInfo.bcd_device");
    raw.bcd_usb = number(&value["bcd_usb"], "MediusDeviceInfo.bcd_usb");
    raw.has_serial = u8::from(flag(&value["has_serial"]));
    raw.has_bos = u8::from(flag(&value["has_bos"]));
    raw.kind = enum_value("DeviceKind", &value["kind"])? as u8;
    write_text(&mut raw.product, &value["product"]);
    Ok(raw)
}

pub(super) fn box_info_to_json(raw: &MediusBoxInfo) -> Value {
    let mut out = Map::new();
    out.insert("port".into(), port_info_to_json(&raw.port));
    out.insert("version".into(), version_to_json(&raw.version));
    out.insert(
        "device".into(),
        if raw.has_device != 0 {
            device_info_to_json(&raw.device)
        } else {
            Value::Null
        },
    );
    Value::Object(out)
}

pub(super) fn box_info_from_json(value: &Value) -> Result<MediusBoxInfo, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusBoxInfo = unsafe { std::mem::zeroed() };
    raw.port = port_info_from_json(&value["port"])?;
    raw.version = version_from_json(&value["version"])?;
    if !value["device"].is_null() {
        raw.has_device = 1;
        raw.device = device_info_from_json(&value["device"])?;
    }
    Ok(raw)
}

pub(super) fn transfer_outcome_to_json(raw: &MediusTransferOutcome) -> Value {
    let mut out = Map::new();
    out.insert(
        "status".into(),
        enum_name("TransferStatus", raw.status as i64),
    );
    out.insert(
        "data".into(),
        hex(&raw.data[..usize::from(raw.len).min(raw.data.len())]),
    );
    Value::Object(out)
}

pub(super) fn transfer_outcome_from_json(value: &Value) -> Result<MediusTransferOutcome, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusTransferOutcome = unsafe { std::mem::zeroed() };
    raw.status = enum_value("TransferStatus", &value["status"])? as u8;
    raw.len = fill(&mut raw.data, &value["data"], "MediusTransferOutcome.data")?
        .try_into()
        .unwrap();
    Ok(raw)
}

pub(super) fn rewrite_rule_to_json(raw: &MediusRewriteRule) -> Value {
    let mut out = Map::new();
    out.insert("class_".into(), enum_name("RewriteClass", raw.class as i64));
    out.insert("id".into(), Value::from(raw.id));
    out.insert(
        "direction".into(),
        enum_name("Direction", raw.direction as i64),
    );
    out.insert(
        "action".into(),
        enum_name("RewriteAction", raw.action as i64),
    );
    out.insert("offset".into(), Value::from(raw.offset));
    out.insert(
        "match_bytes".into(),
        hex(&raw.match_bytes[..usize::from(raw.match_len).min(raw.match_bytes.len())]),
    );
    out.insert(
        "mask".into(),
        hex(&raw.mask[..usize::from(raw.mask_len).min(raw.mask.len())]),
    );
    out.insert(
        "payload".into(),
        hex(&raw.payload[..usize::from(raw.payload_len).min(raw.payload.len())]),
    );
    Value::Object(out)
}

pub(super) fn rewrite_rule_from_json(value: &Value) -> Result<MediusRewriteRule, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusRewriteRule = unsafe { std::mem::zeroed() };
    raw.class = enum_value("RewriteClass", &value["class_"])? as u8;
    raw.id = number(&value["id"], "MediusRewriteRule.id");
    raw.direction = enum_value("Direction", &value["direction"])? as u8;
    raw.action = enum_value("RewriteAction", &value["action"])? as u8;
    raw.offset = number(&value["offset"], "MediusRewriteRule.offset");
    raw.match_len = fill_match(
        &mut raw.match_bytes,
        &value["match_bytes"],
        "MediusRewriteRule.match_bytes",
    )?;
    raw.mask_len = fill_match(&mut raw.mask, &value["mask"], "MediusRewriteRule.mask")?;
    raw.payload_len = fill(
        &mut raw.payload,
        &value["payload"],
        "MediusRewriteRule.payload",
    )?
    .try_into()
    .unwrap();
    Ok(raw)
}

pub(super) fn rewrite_entry_to_json(raw: &MediusRewriteEntry) -> Value {
    let mut out = Map::new();
    out.insert("class_".into(), enum_name("RewriteClass", raw.class as i64));
    out.insert("id".into(), Value::from(raw.id));
    out.insert(
        "direction".into(),
        enum_name("Direction", raw.direction as i64),
    );
    out.insert(
        "action".into(),
        enum_name("RewriteAction", raw.action as i64),
    );
    out.insert("match_len".into(), Value::from(raw.match_len));
    out.insert("offset".into(), Value::from(raw.offset));
    out.insert("payload_len".into(), Value::from(raw.payload_len));
    out.insert("hits".into(), Value::from(raw.hits));
    Value::Object(out)
}

pub(super) fn rewrite_entry_from_json(value: &Value) -> Result<MediusRewriteEntry, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusRewriteEntry = unsafe { std::mem::zeroed() };
    raw.class = enum_value("RewriteClass", &value["class_"])? as u8;
    raw.id = number(&value["id"], "MediusRewriteEntry.id");
    raw.direction = enum_value("Direction", &value["direction"])? as u8;
    raw.action = enum_value("RewriteAction", &value["action"])? as u8;
    raw.match_len = number(&value["match_len"], "MediusRewriteEntry.match_len");
    raw.offset = number(&value["offset"], "MediusRewriteEntry.offset");
    raw.payload_len = number(&value["payload_len"], "MediusRewriteEntry.payload_len");
    raw.hits = number(&value["hits"], "MediusRewriteEntry.hits");
    Ok(raw)
}

pub(super) fn rewrite_table_to_json(raw: &MediusRewriteTable) -> Value {
    let mut out = Map::new();
    out.insert("table_full".into(), Value::from(raw.table_full != 0));
    out.insert("generation".into(), Value::from(raw.generation));
    out.insert(
        "entries".into(),
        Value::Array(
            raw.entries[..usize::from(raw.n).min(raw.entries.len())]
                .iter()
                .map(rewrite_entry_to_json)
                .collect(),
        ),
    );
    Value::Object(out)
}

pub(super) fn rewrite_table_from_json(value: &Value) -> Result<MediusRewriteTable, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusRewriteTable = unsafe { std::mem::zeroed() };
    raw.table_full = u8::from(flag(&value["table_full"]));
    raw.generation = number(&value["generation"], "MediusRewriteTable.generation");
    raw.n = fill_list(
        &mut raw.entries,
        &value["entries"],
        "MediusRewriteTable.entries",
        rewrite_entry_from_json,
    )?
    .try_into()
    .unwrap();
    Ok(raw)
}

pub(super) fn patch_to_json(raw: &MediusPatch) -> Value {
    let mut out = Map::new();
    out.insert(
        "section".into(),
        enum_name("PatchSection", raw.section as i64),
    );
    out.insert("cfg".into(), Value::from(raw.cfg));
    out.insert("index".into(), Value::from(raw.index));
    out.insert("offset".into(), Value::from(raw.offset));
    out.insert(
        "bytes".into(),
        hex(&raw.bytes[..usize::from(raw.len).min(raw.bytes.len())]),
    );
    Value::Object(out)
}

pub(super) fn patch_from_json(value: &Value) -> Result<MediusPatch, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusPatch = unsafe { std::mem::zeroed() };
    raw.section = enum_value("PatchSection", &value["section"])? as u8;
    raw.cfg = number(&value["cfg"], "MediusPatch.cfg");
    raw.index = number(&value["index"], "MediusPatch.index");
    raw.offset = number(&value["offset"], "MediusPatch.offset");
    raw.len = fill(&mut raw.bytes, &value["bytes"], "MediusPatch.bytes")?
        .try_into()
        .unwrap();
    Ok(raw)
}

pub(super) fn patch_entry_to_json(raw: &MediusPatchEntry) -> Value {
    let mut out = Map::new();
    out.insert(
        "section".into(),
        enum_name("PatchSection", raw.section as i64),
    );
    out.insert("cfg".into(), Value::from(raw.cfg));
    out.insert("index".into(), Value::from(raw.index));
    out.insert("offset".into(), Value::from(raw.offset));
    out.insert("len".into(), Value::from(raw.len));
    Value::Object(out)
}

pub(super) fn patch_entry_from_json(value: &Value) -> Result<MediusPatchEntry, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusPatchEntry = unsafe { std::mem::zeroed() };
    raw.section = enum_value("PatchSection", &value["section"])? as u8;
    raw.cfg = number(&value["cfg"], "MediusPatchEntry.cfg");
    raw.index = number(&value["index"], "MediusPatchEntry.index");
    raw.offset = number(&value["offset"], "MediusPatchEntry.offset");
    raw.len = number(&value["len"], "MediusPatchEntry.len");
    Ok(raw)
}

pub(super) fn patch_set_to_json(raw: &MediusPatchSet) -> Value {
    let mut out = Map::new();
    out.insert("applied".into(), Value::from(raw.applied != 0));
    out.insert("pending".into(), Value::from(raw.pending != 0));
    out.insert("refused".into(), Value::from(raw.refused != 0));
    out.insert("table_full".into(), Value::from(raw.table_full != 0));
    out.insert(
        "entries".into(),
        Value::Array(
            raw.entries[..usize::from(raw.n).min(raw.entries.len())]
                .iter()
                .map(patch_entry_to_json)
                .collect(),
        ),
    );
    Value::Object(out)
}

pub(super) fn patch_set_from_json(value: &Value) -> Result<MediusPatchSet, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusPatchSet = unsafe { std::mem::zeroed() };
    raw.applied = u8::from(flag(&value["applied"]));
    raw.pending = u8::from(flag(&value["pending"]));
    raw.refused = u8::from(flag(&value["refused"]));
    raw.table_full = u8::from(flag(&value["table_full"]));
    raw.n = fill_list(
        &mut raw.entries,
        &value["entries"],
        "MediusPatchSet.entries",
        patch_entry_from_json,
    )?
    .try_into()
    .unwrap();
    Ok(raw)
}

pub(super) fn transform_to_json(raw: &MediusTransform) -> Value {
    let mut out = Map::new();
    out.insert("op".into(), enum_name("TransformOp", raw.op as i64));
    out.insert("source".into(), lock_target_to_json(&raw.source));
    out.insert("dest".into(), lock_target_to_json(&raw.dest));
    Value::Object(out)
}

pub(super) fn transform_from_json(value: &Value) -> Result<MediusTransform, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusTransform = unsafe { std::mem::zeroed() };
    raw.op = enum_value("TransformOp", &value["op"])? as u8;
    raw.source = lock_target_from_json(&value["source"])?;
    raw.dest = lock_target_from_json(&value["dest"])?;
    Ok(raw)
}

pub(super) fn transforms_to_json(raw: &MediusTransforms) -> Value {
    let mut out = Map::new();
    out.insert("table_full".into(), Value::from(raw.table_full != 0));
    out.insert(
        "entries".into(),
        Value::Array(
            raw.entries[..usize::from(raw.n).min(raw.entries.len())]
                .iter()
                .map(transform_to_json)
                .collect(),
        ),
    );
    Value::Object(out)
}

pub(super) fn transforms_from_json(value: &Value) -> Result<MediusTransforms, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusTransforms = unsafe { std::mem::zeroed() };
    raw.table_full = u8::from(flag(&value["table_full"]));
    raw.n = fill_list(
        &mut raw.entries,
        &value["entries"],
        "MediusTransforms.entries",
        transform_from_json,
    )?
    .try_into()
    .unwrap();
    Ok(raw)
}

pub(super) fn chip_firmware_to_json(raw: &MediusChipFirmware) -> Value {
    let mut out = Map::new();
    out.insert("major".into(), Value::from(raw.major));
    out.insert("minor".into(), Value::from(raw.minor));
    out.insert("patch".into(), Value::from(raw.patch));
    out.insert("slot".into(), Value::from(raw.slot));
    out.insert("state".into(), enum_name("ImageState", raw.state as i64));
    Value::Object(out)
}

pub(super) fn chip_firmware_from_json(value: &Value) -> Result<MediusChipFirmware, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusChipFirmware = unsafe { std::mem::zeroed() };
    raw.major = number(&value["major"], "MediusChipFirmware.major");
    raw.minor = number(&value["minor"], "MediusChipFirmware.minor");
    raw.patch = number(&value["patch"], "MediusChipFirmware.patch");
    raw.slot = number(&value["slot"], "MediusChipFirmware.slot");
    raw.state = enum_value("ImageState", &value["state"])? as u8;
    Ok(raw)
}

pub(super) fn firmware_info_to_json(raw: &MediusFirmwareInfo) -> Value {
    let mut out = Map::new();
    out.insert("device".into(), chip_firmware_to_json(&raw.device));
    out.insert(
        "host".into(),
        if raw.host_present != 0 {
            chip_firmware_to_json(&raw.host)
        } else {
            Value::Null
        },
    );
    out.insert("slot_size".into(), Value::from(raw.slot_size));
    out.insert("device_staged".into(), Value::from(raw.device_staged != 0));
    out.insert("host_staged".into(), Value::from(raw.host_staged != 0));
    Value::Object(out)
}

pub(super) fn firmware_info_from_json(value: &Value) -> Result<MediusFirmwareInfo, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusFirmwareInfo = unsafe { std::mem::zeroed() };
    raw.device = chip_firmware_from_json(&value["device"])?;
    if !value["host"].is_null() {
        raw.host_present = 1;
        raw.host = chip_firmware_from_json(&value["host"])?;
    }
    raw.slot_size = number(&value["slot_size"], "MediusFirmwareInfo.slot_size");
    raw.device_staged = u8::from(flag(&value["device_staged"]));
    raw.host_staged = u8::from(flag(&value["host_staged"]));
    Ok(raw)
}

pub(super) fn health_to_json(raw: &MediusHealth) -> Value {
    let mut out = Map::new();
    out.insert("link_up".into(), Value::from(raw.link_up != 0));
    out.insert(
        "mouse_attached".into(),
        Value::from(raw.mouse_attached != 0),
    );
    out.insert(
        "clone_configured".into(),
        Value::from(raw.clone_configured != 0),
    );
    out.insert(
        "injection_active".into(),
        Value::from(raw.injection_active != 0),
    );
    out.insert(
        "rate_confident".into(),
        Value::from(raw.rate_confident != 0),
    );
    out.insert("lock_on".into(), Value::from(raw.lock_on != 0));
    out.insert("catch_on".into(), Value::from(raw.catch_on != 0));
    out.insert("kbd_attached".into(), Value::from(raw.kbd_attached != 0));
    out.insert("rewrite_on".into(), Value::from(raw.rewrite_on != 0));
    out.insert("patch_on".into(), Value::from(raw.patch_on != 0));
    out.insert("transform_on".into(), Value::from(raw.transform_on != 0));
    Value::Object(out)
}

pub(super) fn health_from_json(value: &Value) -> Result<MediusHealth, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusHealth = unsafe { std::mem::zeroed() };
    raw.link_up = u8::from(flag(&value["link_up"]));
    raw.mouse_attached = u8::from(flag(&value["mouse_attached"]));
    raw.clone_configured = u8::from(flag(&value["clone_configured"]));
    raw.injection_active = u8::from(flag(&value["injection_active"]));
    raw.rate_confident = u8::from(flag(&value["rate_confident"]));
    raw.lock_on = u8::from(flag(&value["lock_on"]));
    raw.catch_on = u8::from(flag(&value["catch_on"]));
    raw.kbd_attached = u8::from(flag(&value["kbd_attached"]));
    raw.rewrite_on = u8::from(flag(&value["rewrite_on"]));
    raw.patch_on = u8::from(flag(&value["patch_on"]));
    raw.transform_on = u8::from(flag(&value["transform_on"]));
    Ok(raw)
}

pub(super) fn mouse_caps_to_json(raw: &MediusMouseCaps) -> Value {
    let mut out = Map::new();
    out.insert("n_buttons".into(), Value::from(raw.n_buttons));
    out.insert("has_x".into(), Value::from(raw.has_x != 0));
    out.insert("has_y".into(), Value::from(raw.has_y != 0));
    out.insert("has_wheel".into(), Value::from(raw.has_wheel != 0));
    out.insert("pan".into(), Value::from(raw.pan != 0));
    out.insert("has_report_id".into(), Value::from(raw.has_report_id != 0));
    out.insert("n_hid".into(), Value::from(raw.n_hid));
    Value::Object(out)
}

pub(super) fn mouse_caps_from_json(value: &Value) -> Result<MediusMouseCaps, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusMouseCaps = unsafe { std::mem::zeroed() };
    raw.n_buttons = number(&value["n_buttons"], "MediusMouseCaps.n_buttons");
    raw.has_x = u8::from(flag(&value["has_x"]));
    raw.has_y = u8::from(flag(&value["has_y"]));
    raw.has_wheel = u8::from(flag(&value["has_wheel"]));
    raw.pan = u8::from(flag(&value["pan"]));
    raw.has_report_id = u8::from(flag(&value["has_report_id"]));
    raw.n_hid = number(&value["n_hid"], "MediusMouseCaps.n_hid");
    Ok(raw)
}

pub(super) fn kbd_caps_to_json(raw: &MediusKbdCaps) -> Value {
    let mut out = Map::new();
    out.insert("n_keys".into(), Value::from(raw.n_keys));
    out.insert("nkro".into(), Value::from(raw.nkro != 0));
    out.insert("has_consumer".into(), Value::from(raw.has_consumer != 0));
    out.insert("has_system".into(), Value::from(raw.has_system != 0));
    out.insert("has_report_id".into(), Value::from(raw.has_report_id != 0));
    Value::Object(out)
}

pub(super) fn kbd_caps_from_json(value: &Value) -> Result<MediusKbdCaps, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusKbdCaps = unsafe { std::mem::zeroed() };
    raw.n_keys = number(&value["n_keys"], "MediusKbdCaps.n_keys");
    raw.nkro = u8::from(flag(&value["nkro"]));
    raw.has_consumer = u8::from(flag(&value["has_consumer"]));
    raw.has_system = u8::from(flag(&value["has_system"]));
    raw.has_report_id = u8::from(flag(&value["has_report_id"]));
    Ok(raw)
}

pub(super) fn caps_to_json(raw: &MediusCaps) -> Value {
    let mut out = Map::new();
    out.insert("mouse".into(), mouse_caps_to_json(&raw.mouse));
    out.insert("keyboard".into(), kbd_caps_to_json(&raw.keyboard));
    out.insert(
        "mouse_change_driven".into(),
        Value::from(raw.mouse_change_driven != 0),
    );
    out.insert(
        "kbd_change_driven".into(),
        Value::from(raw.kbd_change_driven != 0),
    );
    Value::Object(out)
}

pub(super) fn caps_from_json(value: &Value) -> Result<MediusCaps, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusCaps = unsafe { std::mem::zeroed() };
    raw.mouse = mouse_caps_from_json(&value["mouse"])?;
    raw.keyboard = kbd_caps_from_json(&value["keyboard"])?;
    raw.mouse_change_driven = u8::from(flag(&value["mouse_change_driven"]));
    raw.kbd_change_driven = u8::from(flag(&value["kbd_change_driven"]));
    Ok(raw)
}

pub(super) fn rate_to_json(raw: &MediusRate) -> Value {
    let mut out = Map::new();
    out.insert("native_period_us".into(), Value::from(raw.native_period_us));
    out.insert("poll_period_us".into(), Value::from(raw.poll_period_us));
    out.insert("confident".into(), Value::from(raw.confident != 0));
    out.insert("change_driven".into(), Value::from(raw.change_driven != 0));
    Value::Object(out)
}

pub(super) fn rate_from_json(value: &Value) -> Result<MediusRate, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusRate = unsafe { std::mem::zeroed() };
    raw.native_period_us = number(&value["native_period_us"], "MediusRate.native_period_us");
    raw.poll_period_us = number(&value["poll_period_us"], "MediusRate.poll_period_us");
    raw.confident = u8::from(flag(&value["confident"]));
    raw.change_driven = u8::from(flag(&value["change_driven"]));
    Ok(raw)
}

pub(super) fn stats_to_json(raw: &MediusStats) -> Value {
    let mut out = Map::new();
    out.insert("inject_emits".into(), Value::from(raw.inject_emits));
    out.insert("tx_drops".into(), Value::from(raw.tx_drops));
    out.insert("tx_merges".into(), Value::from(raw.tx_merges));
    out.insert("tx_maxdepth".into(), Value::from(raw.tx_maxdepth));
    out.insert("tx_wedges".into(), Value::from(raw.tx_wedges));
    out.insert("wakeups".into(), Value::from(raw.wakeups));
    out.insert("reset_count".into(), Value::from(raw.reset_count));
    out.insert("config_count".into(), Value::from(raw.config_count));
    out.insert("link_rx_drops".into(), Value::from(raw.link_rx_drops));
    out.insert("host_rx_drops".into(), Value::from(raw.host_rx_drops));
    out.insert("relay_drops".into(), Value::from(raw.relay_drops));
    out.insert("session".into(), Value::from(raw.session));
    Value::Object(out)
}

pub(super) fn stats_from_json(value: &Value) -> Result<MediusStats, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusStats = unsafe { std::mem::zeroed() };
    raw.inject_emits = number(&value["inject_emits"], "MediusStats.inject_emits");
    raw.tx_drops = number(&value["tx_drops"], "MediusStats.tx_drops");
    raw.tx_merges = number(&value["tx_merges"], "MediusStats.tx_merges");
    raw.tx_maxdepth = number(&value["tx_maxdepth"], "MediusStats.tx_maxdepth");
    raw.tx_wedges = number(&value["tx_wedges"], "MediusStats.tx_wedges");
    raw.wakeups = number(&value["wakeups"], "MediusStats.wakeups");
    raw.reset_count = number(&value["reset_count"], "MediusStats.reset_count");
    raw.config_count = number(&value["config_count"], "MediusStats.config_count");
    raw.link_rx_drops = number(&value["link_rx_drops"], "MediusStats.link_rx_drops");
    raw.host_rx_drops = number(&value["host_rx_drops"], "MediusStats.host_rx_drops");
    raw.relay_drops = number(&value["relay_drops"], "MediusStats.relay_drops");
    raw.session = number(&value["session"], "MediusStats.session");
    Ok(raw)
}

pub(super) fn lock_entry_to_json(raw: &MediusLockEntry) -> Value {
    let mut out = Map::new();
    out.insert("target".into(), lock_target_to_json(&raw.target));
    out.insert("is_blanket".into(), Value::from(raw.is_blanket));
    out.insert(
        "direction".into(),
        enum_name("Direction", raw.direction as i64),
    );
    out.insert("scale".into(), Value::from(raw.scale));
    Value::Object(out)
}

pub(super) fn lock_entry_from_json(value: &Value) -> Result<MediusLockEntry, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusLockEntry = unsafe { std::mem::zeroed() };
    raw.target = lock_target_from_json(&value["target"])?;
    raw.is_blanket = flag(&value["is_blanket"]);
    raw.direction = enum_value("Direction", &value["direction"])? as u8;
    raw.scale = number(&value["scale"], "MediusLockEntry.scale");
    Ok(raw)
}

pub(super) fn locks_to_json(raw: &MediusLocks) -> Value {
    let mut out = Map::new();
    out.insert(
        "entries".into(),
        Value::Array(
            raw.entries[..usize::from(raw.n).min(raw.entries.len())]
                .iter()
                .map(lock_entry_to_json)
                .collect(),
        ),
    );
    Value::Object(out)
}

pub(super) fn locks_from_json(value: &Value) -> Result<MediusLocks, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusLocks = unsafe { std::mem::zeroed() };
    raw.n = fill_list(
        &mut raw.entries,
        &value["entries"],
        "MediusLocks.entries",
        lock_entry_from_json,
    )?
    .try_into()
    .unwrap();
    Ok(raw)
}

pub(super) fn clock_estimate_to_json(raw: &MediusClockEstimate) -> Value {
    let mut out = Map::new();
    out.insert("offset_us".into(), Value::from(raw.offset_us));
    out.insert(
        "rate_ppb".into(),
        if raw.rate_ppb == MEDIUS_CLOCK_RATE_NONE {
            Value::Null
        } else {
            Value::from(raw.rate_ppb)
        },
    );
    out.insert("delay_us".into(), Value::from(raw.delay_us));
    out.insert(
        "age_ms".into(),
        if raw.age_ms == MEDIUS_CLOCK_AGE_NONE {
            Value::Null
        } else {
            Value::from(raw.age_ms)
        },
    );
    Value::Object(out)
}

pub(super) fn clock_estimate_from_json(value: &Value) -> Result<MediusClockEstimate, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusClockEstimate = unsafe { std::mem::zeroed() };
    raw.offset_us = number(&value["offset_us"], "MediusClockEstimate.offset_us");
    raw.rate_ppb = if value["rate_ppb"].is_null() {
        MEDIUS_CLOCK_RATE_NONE
    } else {
        number(&value["rate_ppb"], "MediusClockEstimate.rate_ppb")
    };
    raw.delay_us = number(&value["delay_us"], "MediusClockEstimate.delay_us");
    raw.age_ms = if value["age_ms"].is_null() {
        MEDIUS_CLOCK_AGE_NONE
    } else {
        number(&value["age_ms"], "MediusClockEstimate.age_ms")
    };
    Ok(raw)
}

pub(super) fn catch_entry_to_json(raw: &MediusCatchEntry) -> Value {
    let mut out = Map::new();
    out.insert("filter".into(), catch_filter_to_json(&raw.filter));
    out.insert("dropped".into(), Value::from(raw.dropped));
    Value::Object(out)
}

pub(super) fn catch_entry_from_json(value: &Value) -> Result<MediusCatchEntry, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusCatchEntry = unsafe { std::mem::zeroed() };
    raw.filter = catch_filter_from_json(&value["filter"])?;
    raw.dropped = number(&value["dropped"], "MediusCatchEntry.dropped");
    Ok(raw)
}

pub(super) fn catch_state_to_json(raw: &MediusCatchState) -> Value {
    let mut out = Map::new();
    out.insert("table_full".into(), Value::from(raw.table_full != 0));
    out.insert("dropped".into(), Value::from(raw.dropped));
    out.insert("clock".into(), clock_estimate_to_json(&raw.clock));
    out.insert(
        "entries".into(),
        Value::Array(
            raw.entries[..usize::from(raw.n).min(raw.entries.len())]
                .iter()
                .map(catch_entry_to_json)
                .collect(),
        ),
    );
    Value::Object(out)
}

pub(super) fn catch_state_from_json(value: &Value) -> Result<MediusCatchState, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusCatchState = unsafe { std::mem::zeroed() };
    raw.table_full = u8::from(flag(&value["table_full"]));
    raw.dropped = number(&value["dropped"], "MediusCatchState.dropped");
    raw.clock = clock_estimate_from_json(&value["clock"])?;
    raw.n = fill_list(
        &mut raw.entries,
        &value["entries"],
        "MediusCatchState.entries",
        catch_entry_from_json,
    )?
    .try_into()
    .unwrap();
    Ok(raw)
}

pub(super) fn imperfect_status_to_json(raw: &MediusImperfectStatus) -> Value {
    let mut out = Map::new();
    out.insert("allowed".into(), Value::from(raw.allowed != 0));
    out.insert("over_capacity".into(), Value::from(raw.over_capacity != 0));
    out.insert(
        "clone_imperfect".into(),
        Value::from(raw.clone_imperfect != 0),
    );
    Value::Object(out)
}

pub(super) fn imperfect_status_from_json(value: &Value) -> Result<MediusImperfectStatus, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusImperfectStatus = unsafe { std::mem::zeroed() };
    raw.allowed = u8::from(flag(&value["allowed"]));
    raw.over_capacity = u8::from(flag(&value["over_capacity"]));
    raw.clone_imperfect = u8::from(flag(&value["clone_imperfect"]));
    Ok(raw)
}

pub(super) fn bearing_to_json(raw: &MediusBearing) -> Value {
    let mut out = Map::new();
    out.insert(
        "window_ms".into(),
        if raw.window_ms == 0 {
            Value::Null
        } else {
            Value::from(raw.window_ms)
        },
    );
    out.insert("mode".into(), enum_name("BearingMode", raw.mode as i64));
    Value::Object(out)
}

pub(super) fn bearing_from_json(value: &Value) -> Result<MediusBearing, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusBearing = unsafe { std::mem::zeroed() };
    raw.window_ms = if value["window_ms"].is_null() {
        0
    } else {
        number(&value["window_ms"], "MediusBearing.window_ms")
    };
    raw.mode = bearing_mode_from(enum_value("BearingMode", &value["mode"])?)?;
    Ok(raw)
}

pub(super) fn render_status_to_json(raw: &MediusRenderStatus) -> Value {
    let mut out = Map::new();
    out.insert("mode".into(), enum_name("RenderMode", raw.mode as i64));
    out.insert("full".into(), Value::from(raw.full != 0));
    out.insert("ready".into(), Value::from(raw.ready != 0));
    Value::Object(out)
}

pub(super) fn render_status_from_json(value: &Value) -> Result<MediusRenderStatus, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusRenderStatus = unsafe { std::mem::zeroed() };
    raw.mode = render_mode_from(enum_value("RenderMode", &value["mode"])?)?;
    raw.full = u8::from(flag(&value["full"]));
    raw.ready = u8::from(flag(&value["ready"]));
    Ok(raw)
}

pub(super) fn spread_status_to_json(raw: &MediusSpreadStatus) -> Value {
    let mut out = Map::new();
    out.insert("percent".into(), Value::from(raw.percent));
    out.insert("span_us".into(), Value::from(raw.span_us));
    Value::Object(out)
}

pub(super) fn spread_status_from_json(value: &Value) -> Result<MediusSpreadStatus, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusSpreadStatus = unsafe { std::mem::zeroed() };
    raw.percent = number(&value["percent"], "MediusSpreadStatus.percent");
    raw.span_us = number(&value["span_us"], "MediusSpreadStatus.span_us");
    Ok(raw)
}

pub(super) fn counters_snapshot_to_json(raw: &MediusCountersSnapshot) -> Value {
    let mut out = Map::new();
    out.insert("frames_tx".into(), Value::from(raw.frames_tx));
    out.insert("frames_rx".into(), Value::from(raw.frames_rx));
    out.insert("crc_drops".into(), Value::from(raw.crc_drops));
    out.insert("reconnects".into(), Value::from(raw.reconnects));
    out.insert("restarts".into(), Value::from(raw.restarts));
    Value::Object(out)
}

pub(super) fn counters_snapshot_from_json(
    value: &Value,
) -> Result<MediusCountersSnapshot, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusCountersSnapshot = unsafe { std::mem::zeroed() };
    raw.frames_tx = number(&value["frames_tx"], "MediusCountersSnapshot.frames_tx");
    raw.frames_rx = number(&value["frames_rx"], "MediusCountersSnapshot.frames_rx");
    raw.crc_drops = number(&value["crc_drops"], "MediusCountersSnapshot.crc_drops");
    raw.reconnects = number(&value["reconnects"], "MediusCountersSnapshot.reconnects");
    raw.restarts = number(&value["restarts"], "MediusCountersSnapshot.restarts");
    Ok(raw)
}

pub(super) fn usage_event_to_json(raw: &MediusUsageEvent) -> Value {
    let mut out = Map::new();
    out.insert("class_".into(), enum_name("Class", raw.class as i64));
    out.insert(
        "direction".into(),
        enum_name("Direction", raw.direction as i64),
    );
    out.insert(
        "usages".into(),
        Value::Array(
            raw.usages[..usize::from(raw.n).min(raw.usages.len())]
                .iter()
                .map(usage_to_json)
                .collect(),
        ),
    );
    Value::Object(out)
}

pub(super) fn usage_event_from_json(value: &Value) -> Result<MediusUsageEvent, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusUsageEvent = unsafe { std::mem::zeroed() };
    raw.class = enum_value("Class", &value["class_"])? as u8;
    raw.direction = enum_value("Direction", &value["direction"])? as u8;
    raw.n = fill_list(
        &mut raw.usages,
        &value["usages"],
        "MediusUsageEvent.usages",
        usage_from_json,
    )?
    .try_into()
    .unwrap();
    Ok(raw)
}

pub(super) fn traffic_event_to_json(raw: &MediusTrafficEvent) -> Value {
    let mut out = Map::new();
    out.insert("class_".into(), enum_name("CatchClass", raw.class as i64));
    out.insert("id".into(), Value::from(raw.id));
    out.insert(
        "direction".into(),
        enum_name("Direction", raw.direction as i64),
    );
    out.insert("flags".into(), Value::from(raw.flags));
    out.insert("true_len".into(), Value::from(raw.true_len));
    out.insert(
        "bytes".into(),
        hex(&raw.bytes[..usize::from(raw.len).min(raw.bytes.len())]),
    );
    Value::Object(out)
}

pub(super) fn traffic_event_from_json(value: &Value) -> Result<MediusTrafficEvent, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusTrafficEvent = unsafe { std::mem::zeroed() };
    raw.class = enum_value("CatchClass", &value["class_"])? as u8;
    raw.id = number(&value["id"], "MediusTrafficEvent.id");
    raw.direction = enum_value("Direction", &value["direction"])? as u8;
    raw.flags = number(&value["flags"], "MediusTrafficEvent.flags");
    raw.true_len = number(&value["true_len"], "MediusTrafficEvent.true_len");
    raw.len = fill(&mut raw.bytes, &value["bytes"], "MediusTrafficEvent.bytes")?
        .try_into()
        .unwrap();
    Ok(raw)
}

pub(super) fn bus_event_to_json(raw: &MediusBusEvent) -> Value {
    let mut out = Map::new();
    out.insert("kind".into(), enum_name("BusEventKind", raw.kind as i64));
    out.insert("configuration".into(), Value::from(raw.configuration));
    out.insert("interface".into(), Value::from(raw.interface));
    out.insert("alt".into(), Value::from(raw.alt));
    Value::Object(out)
}

pub(super) fn bus_event_from_json(value: &Value) -> Result<MediusBusEvent, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusBusEvent = unsafe { std::mem::zeroed() };
    raw.kind = bus_event_kind_from(enum_value("BusEventKind", &value["kind"])?)?;
    raw.configuration = number(&value["configuration"], "MediusBusEvent.configuration");
    raw.interface = number(&value["interface"], "MediusBusEvent.interface");
    raw.alt = number(&value["alt"], "MediusBusEvent.alt");
    Ok(raw)
}

pub(super) fn motion_event_to_json(raw: &MediusMotionEvent) -> Value {
    let mut out = Map::new();
    out.insert("dx".into(), Value::from(raw.dx));
    out.insert("dy".into(), Value::from(raw.dy));
    out.insert("dz".into(), Value::from(raw.dz));
    out.insert("pan".into(), Value::from(raw.pan));
    Value::Object(out)
}

pub(super) fn motion_event_from_json(value: &Value) -> Result<MediusMotionEvent, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusMotionEvent = unsafe { std::mem::zeroed() };
    raw.dx = number(&value["dx"], "MediusMotionEvent.dx");
    raw.dy = number(&value["dy"], "MediusMotionEvent.dy");
    raw.dz = number(&value["dz"], "MediusMotionEvent.dz");
    raw.pan = number(&value["pan"], "MediusMotionEvent.pan");
    Ok(raw)
}

pub(super) fn stamped_to_json(raw: &MediusStamped) -> Value {
    let mut out = Map::new();
    out.insert("host_ns".into(), Value::from(raw.host_ns));
    out.insert("box_us".into(), Value::from(raw.box_us));
    out.insert("excess_ns".into(), Value::from(raw.excess_ns));
    Value::Object(out)
}

pub(super) fn stamped_from_json(value: &Value) -> Result<MediusStamped, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusStamped = unsafe { std::mem::zeroed() };
    raw.host_ns = number(&value["host_ns"], "MediusStamped.host_ns");
    raw.box_us = number(&value["box_us"], "MediusStamped.box_us");
    raw.excess_ns = number(&value["excess_ns"], "MediusStamped.excess_ns");
    Ok(raw)
}

pub(super) fn log_line_to_json(raw: &MediusLogLine) -> Value {
    let mut out = Map::new();
    out.insert("level".into(), enum_name("LogLevel", raw.level as i64));
    out.insert("text".into(), text(&raw.text));
    Value::Object(out)
}

pub(super) fn log_line_from_json(value: &Value) -> Result<MediusLogLine, Failure> {
    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value
    let mut raw: MediusLogLine = unsafe { std::mem::zeroed() };
    raw.level = log_level_from(enum_value("LogLevel", &value["level"])?)?;
    write_text(&mut raw.text, &value["text"]);
    Ok(raw)
}

// every generated function, which the suite's coverage test makes the scenarios reach
pub(super) const CALLS: [(&str, &str); 145] = [
    ("ClipFrame", "clear"),
    ("ClipFrame", "move"),
    ("ClipFrame", "wheel"),
    ("ClipFrame", "pan"),
    ("ClipFrame", "edge"),
    ("ClipFrame", "press"),
    ("ClipFrame", "release"),
    ("ClipFrame", "force_release"),
    ("ClipFrame", "raw"),
    ("ClipFrame", "transfer"),
    ("ClipFrame", "byte_len"),
    ("ClipBuilder", "clear"),
    ("ClipBuilder", "byte_len"),
    ("ClipBuilder", "gap"),
    ("ClipBuilder", "move"),
    ("ClipBuilder", "wheel"),
    ("ClipBuilder", "pan"),
    ("ClipBuilder", "press"),
    ("ClipBuilder", "release"),
    ("ClipBuilder", "force_release"),
    ("ClipBuilder", "edge"),
    ("ClipBuilder", "raw"),
    ("ClipBuilder", "transfer"),
    ("ClipBuilder", "frame"),
    ("Clip", "append"),
    ("Clip", "set_loop"),
    ("Clip", "set_retain"),
    ("Clip", "set_ride"),
    ("Clip", "bind"),
    ("Clip", "unbind"),
    ("Clip", "bind_packet"),
    ("Clip", "clear_triggers"),
    ("Clip", "start"),
    ("Clip", "stop"),
    ("Clip", "pause"),
    ("Clip", "resume"),
    ("Clip", "restart"),
    ("Clip", "toggle"),
    ("Clip", "clear"),
    ("Clip", "finalize"),
    ("Clip", "lost"),
    ("Clip", "query_status"),
    ("Clip", "query_config"),
    ("Device", "move_rel"),
    ("Device", "wheel"),
    ("Device", "move_rel_now"),
    ("Device", "wheel_now"),
    ("Device", "pan"),
    ("Device", "pan_now"),
    ("Device", "flush_motion"),
    ("Device", "discard_motion"),
    ("Device", "move_axis"),
    ("Device", "inject"),
    ("Device", "press"),
    ("Device", "soft_release"),
    ("Device", "force_release"),
    ("Device", "scale"),
    ("Device", "scale_all"),
    ("Device", "lock"),
    ("Device", "unlock"),
    ("Device", "lock_all"),
    ("Device", "unlock_all"),
    ("Device", "led"),
    ("Device", "reset"),
    ("Device", "factory_reset"),
    ("Device", "reapply"),
    ("Device", "reconnect"),
    ("Device", "reboot"),
    ("Device", "allow_imperfect_clones"),
    ("Device", "raw"),
    ("Device", "set_rewrite"),
    ("Device", "clear_rewrite"),
    ("Device", "query_rewrite"),
    ("Device", "query_rewrite_entry"),
    ("Device", "set_patch"),
    ("Device", "apply_patch"),
    ("Device", "clear_patch"),
    ("Device", "query_patches"),
    ("Device", "query_patch_entry"),
    ("Device", "transform"),
    ("Device", "untransform"),
    ("Device", "clear_transforms"),
    ("Device", "transform_swap"),
    ("Device", "transform_remap"),
    ("Device", "query_transforms"),
    ("Device", "set_name"),
    ("Device", "clear_name"),
    ("Device", "set_render"),
    ("Device", "set_spread"),
    ("Device", "query_version"),
    ("Device", "firmware_info"),
    ("Device", "abort_update"),
    ("Device", "activate_firmware"),
    ("Device", "wait_firmware_confirmed"),
    ("Device", "query_health"),
    ("Device", "device_info"),
    ("Device", "caps"),
    ("Device", "query_rate"),
    ("Device", "query_stats"),
    ("Device", "query_locks"),
    ("Device", "query_catch"),
    ("Device", "query_imperfect"),
    ("Device", "query_emit_pace"),
    ("Device", "query_bearing"),
    ("Device", "query_render"),
    ("Device", "query_spread"),
    ("Device", "counters"),
    ("", "default_query_timeout_ms"),
    ("", "default_transfer_timeout_ms"),
    ("", "default_keepalive_cadence_ms"),
    ("", "abi_version"),
    ("", "version"),
    ("Timeline", "reset"),
    ("Timeline", "samples"),
    ("MockBox", "set_version"),
    ("MockBox", "set_health"),
    ("MockBox", "set_device_info"),
    ("MockBox", "set_caps"),
    ("MockBox", "set_mouse_caps"),
    ("MockBox", "set_kbd_caps"),
    ("MockBox", "set_rate"),
    ("MockBox", "set_stats"),
    ("MockBox", "set_locks"),
    ("MockBox", "set_catch_state"),
    ("MockBox", "set_imperfect_status"),
    ("MockBox", "set_transfer_reply"),
    ("MockBox", "set_render"),
    ("MockBox", "set_spread_learned"),
    ("MockBox", "set_advertised_hz"),
    ("MockBox", "set_clip_status"),
    ("MockBox", "set_clip_settings"),
    ("MockBox", "restart"),
    ("MockBox", "link_lost"),
    ("MockBox", "detach"),
    ("MockBox", "attach"),
    ("MockBox", "refuse_next_activate"),
    ("MockBox", "reboot_after_activate"),
    ("MockBox", "silent"),
    ("MockBox", "push_log"),
    ("MockBox", "push_motion"),
    ("MockBox", "push_usages"),
    ("MockBox", "push_traffic"),
    ("MockBox", "recorded"),
    ("MockBox", "saw"),
    ("MockBox", "clear_recorded"),
];

// a generated function by its C++ class and method, None for one that's written by hand
pub(super) fn call(ctx: &Context, class: &str, method: &str, args: &[Value]) -> Option<Outcome> {
    let run: fn(&Context, &[Value]) -> Outcome = match (class, method) {
        ("ClipFrame", "clear") => |ctx, _args| {
            status(unsafe { medius_clip_frame_clear(ctx.frame) }, || {
                Value::Null
            })
        },
        ("ClipFrame", "move") => |ctx, args| {
            let a1: i16 = number(&args[0], "medius_clip_frame_move dx");
            let a2: i16 = number(&args[1], "medius_clip_frame_move dy");
            status(unsafe { medius_clip_frame_move(ctx.frame, a1, a2) }, || {
                Value::Null
            })
        },
        ("ClipFrame", "wheel") => |ctx, args| {
            let a1: i16 = number(&args[0], "medius_clip_frame_wheel dz");
            status(unsafe { medius_clip_frame_wheel(ctx.frame, a1) }, || {
                Value::Null
            })
        },
        ("ClipFrame", "pan") => |ctx, args| {
            let a1: i16 = number(&args[0], "medius_clip_frame_pan dpan");
            status(unsafe { medius_clip_frame_pan(ctx.frame, a1) }, || {
                Value::Null
            })
        },
        ("ClipFrame", "edge") => |ctx, args| {
            let a1 = usage_from_json(&args[0])?;
            let a2 = enum_value("Action", &args[1])? as u8;
            status(unsafe { medius_clip_frame_edge(ctx.frame, a1, a2) }, || {
                Value::Null
            })
        },
        ("ClipFrame", "press") => |ctx, args| {
            let a1 = usage_from_json(&args[0])?;
            status(unsafe { medius_clip_frame_press(ctx.frame, a1) }, || {
                Value::Null
            })
        },
        ("ClipFrame", "release") => |ctx, args| {
            let a1 = usage_from_json(&args[0])?;
            status(unsafe { medius_clip_frame_release(ctx.frame, a1) }, || {
                Value::Null
            })
        },
        ("ClipFrame", "force_release") => |ctx, args| {
            let a1 = usage_from_json(&args[0])?;
            status(
                unsafe { medius_clip_frame_force_release(ctx.frame, a1) },
                || Value::Null,
            )
        },
        ("ClipFrame", "raw") => |ctx, args| {
            let a1: u8 = number(&args[0], "medius_clip_frame_raw ep_num");
            let a2 = enum_value("Direction", &args[1])? as u8;
            let a3 = unhex(&args[2], "medius_clip_frame_raw bytes");
            status(
                unsafe { medius_clip_frame_raw(ctx.frame, a1, a2, a3.as_ptr(), a3.len()) },
                || Value::Null,
            )
        },
        ("ClipFrame", "transfer") => |ctx, args| {
            let a1: u8 = number(&args[0], "medius_clip_frame_transfer ep");
            let a2 = setup_from_json(&args[1])?;
            let a3 = unhex(&args[2], "medius_clip_frame_transfer out_data");
            status(
                unsafe { medius_clip_frame_transfer(ctx.frame, a1, a2, a3.as_ptr(), a3.len()) },
                || Value::Null,
            )
        },
        ("ClipFrame", "byte_len") => |ctx, _args| {
            Ok(Value::from(unsafe {
                medius_clip_frame_byte_len(ctx.frame)
            }))
        },
        ("ClipBuilder", "clear") => |ctx, _args| {
            status(unsafe { medius_clip_builder_clear(ctx.builder) }, || {
                Value::Null
            })
        },
        ("ClipBuilder", "byte_len") => |ctx, _args| {
            Ok(Value::from(unsafe {
                medius_clip_builder_byte_len(ctx.builder)
            }))
        },
        ("ClipBuilder", "gap") => |ctx, args| {
            let a1: u16 = number(&args[0], "medius_clip_builder_gap frames");
            status(unsafe { medius_clip_builder_gap(ctx.builder, a1) }, || {
                Value::Null
            })
        },
        ("ClipBuilder", "move") => |ctx, args| {
            let a1: i16 = number(&args[0], "medius_clip_builder_move dx");
            let a2: i16 = number(&args[1], "medius_clip_builder_move dy");
            status(
                unsafe { medius_clip_builder_move(ctx.builder, a1, a2) },
                || Value::Null,
            )
        },
        ("ClipBuilder", "wheel") => |ctx, args| {
            let a1: i16 = number(&args[0], "medius_clip_builder_wheel dz");
            status(
                unsafe { medius_clip_builder_wheel(ctx.builder, a1) },
                || Value::Null,
            )
        },
        ("ClipBuilder", "pan") => |ctx, args| {
            let a1: i16 = number(&args[0], "medius_clip_builder_pan dpan");
            status(unsafe { medius_clip_builder_pan(ctx.builder, a1) }, || {
                Value::Null
            })
        },
        ("ClipBuilder", "press") => |ctx, args| {
            let a1 = usage_from_json(&args[0])?;
            status(
                unsafe { medius_clip_builder_press(ctx.builder, a1) },
                || Value::Null,
            )
        },
        ("ClipBuilder", "release") => |ctx, args| {
            let a1 = usage_from_json(&args[0])?;
            status(
                unsafe { medius_clip_builder_release(ctx.builder, a1) },
                || Value::Null,
            )
        },
        ("ClipBuilder", "force_release") => |ctx, args| {
            let a1 = usage_from_json(&args[0])?;
            status(
                unsafe { medius_clip_builder_force_release(ctx.builder, a1) },
                || Value::Null,
            )
        },
        ("ClipBuilder", "edge") => |ctx, args| {
            let a1 = usage_from_json(&args[0])?;
            let a2 = enum_value("Action", &args[1])? as u8;
            status(
                unsafe { medius_clip_builder_edge(ctx.builder, a1, a2) },
                || Value::Null,
            )
        },
        ("ClipBuilder", "raw") => |ctx, args| {
            let a1: u8 = number(&args[0], "medius_clip_builder_raw ep_num");
            let a2 = enum_value("Direction", &args[1])? as u8;
            let a3 = unhex(&args[2], "medius_clip_builder_raw bytes");
            status(
                unsafe { medius_clip_builder_raw(ctx.builder, a1, a2, a3.as_ptr(), a3.len()) },
                || Value::Null,
            )
        },
        ("ClipBuilder", "transfer") => |ctx, args| {
            let a1: u8 = number(&args[0], "medius_clip_builder_transfer ep");
            let a2 = setup_from_json(&args[1])?;
            let a3 = unhex(&args[2], "medius_clip_builder_transfer out_data");
            status(
                unsafe { medius_clip_builder_transfer(ctx.builder, a1, a2, a3.as_ptr(), a3.len()) },
                || Value::Null,
            )
        },
        ("ClipBuilder", "frame") => |ctx, _args| {
            status(
                unsafe { medius_clip_builder_frame(ctx.builder, ctx.frame as *const _) },
                || Value::Null,
            )
        },
        ("Clip", "append") => |ctx, _args| {
            status(
                unsafe { medius_clip_append(ctx.clip, ctx.builder as *const _) },
                || Value::Null,
            )
        },
        ("Clip", "set_loop") => |ctx, args| {
            let a1 = u8::from(flag(&args[0]));
            status(unsafe { medius_clip_set_loop(ctx.clip, a1) }, || {
                Value::Null
            })
        },
        ("Clip", "set_retain") => |ctx, args| {
            let a1 = u8::from(flag(&args[0]));
            status(unsafe { medius_clip_set_retain(ctx.clip, a1) }, || {
                Value::Null
            })
        },
        ("Clip", "set_ride") => |ctx, args| {
            let a1 = u8::from(flag(&args[0]));
            status(unsafe { medius_clip_set_ride(ctx.clip, a1) }, || {
                Value::Null
            })
        },
        ("Clip", "bind") => |ctx, args| {
            let a1 = clip_trigger_from_json(&args[0])?;
            status(unsafe { medius_clip_bind(ctx.clip, a1) }, || Value::Null)
        },
        ("Clip", "unbind") => |ctx, args| {
            let a1 = usage_from_json(&args[0])?;
            let a2 = enum_value("Edge", &args[1])? as u8;
            status(unsafe { medius_clip_unbind(ctx.clip, a1, a2) }, || {
                Value::Null
            })
        },
        ("Clip", "bind_packet") => |ctx, args| {
            let a1 = clip_packet_trigger_from_json(&args[0])?;
            status(unsafe { medius_clip_bind_packet(ctx.clip, &a1) }, || {
                Value::Null
            })
        },
        ("Clip", "clear_triggers") => |ctx, _args| {
            status(unsafe { medius_clip_clear_triggers(ctx.clip) }, || {
                Value::Null
            })
        },
        ("Clip", "start") => {
            |ctx, _args| status(unsafe { medius_clip_start(ctx.clip) }, || Value::Null)
        }
        ("Clip", "stop") => {
            |ctx, _args| status(unsafe { medius_clip_stop(ctx.clip) }, || Value::Null)
        }
        ("Clip", "pause") => {
            |ctx, _args| status(unsafe { medius_clip_pause(ctx.clip) }, || Value::Null)
        }
        ("Clip", "resume") => {
            |ctx, _args| status(unsafe { medius_clip_resume(ctx.clip) }, || Value::Null)
        }
        ("Clip", "restart") => {
            |ctx, _args| status(unsafe { medius_clip_restart(ctx.clip) }, || Value::Null)
        }
        ("Clip", "toggle") => {
            |ctx, _args| status(unsafe { medius_clip_toggle(ctx.clip) }, || Value::Null)
        }
        ("Clip", "clear") => {
            |ctx, _args| status(unsafe { medius_clip_clear(ctx.clip) }, || Value::Null)
        }
        ("Clip", "finalize") => {
            |ctx, _args| status(unsafe { medius_clip_finalize(ctx.clip) }, || Value::Null)
        }
        ("Clip", "lost") => |ctx, _args| Ok(Value::from(unsafe { medius_clip_lost(ctx.clip) })),
        ("Clip", "query_status") => |ctx, _args| {
            let mut out: MediusClipStatus = unsafe { std::mem::zeroed() };
            status(
                unsafe { medius_clip_query_status(ctx.clip, &mut out) },
                || clip_status_to_json(&out),
            )
        },
        ("Clip", "query_config") => |ctx, _args| {
            let mut out: MediusClipSettings = unsafe { std::mem::zeroed() };
            status(
                unsafe { medius_clip_query_config(ctx.clip, &mut out) },
                || clip_settings_to_json(&out),
            )
        },
        ("Device", "move_rel") => |ctx, args| {
            let a1: i16 = number(&args[0], "medius_device_move_rel dx");
            let a2: i16 = number(&args[1], "medius_device_move_rel dy");
            status(
                unsafe { medius_device_move_rel(ctx.device, a1, a2) },
                || Value::Null,
            )
        },
        ("Device", "wheel") => |ctx, args| {
            let a1: i16 = number(&args[0], "medius_device_wheel delta");
            status(unsafe { medius_device_wheel(ctx.device, a1) }, || {
                Value::Null
            })
        },
        ("Device", "move_rel_now") => |ctx, args| {
            let a1: i16 = number(&args[0], "medius_device_move_rel_now dx");
            let a2: i16 = number(&args[1], "medius_device_move_rel_now dy");
            status(
                unsafe { medius_device_move_rel_now(ctx.device, a1, a2) },
                || Value::Null,
            )
        },
        ("Device", "wheel_now") => |ctx, args| {
            let a1: i16 = number(&args[0], "medius_device_wheel_now delta");
            status(unsafe { medius_device_wheel_now(ctx.device, a1) }, || {
                Value::Null
            })
        },
        ("Device", "pan") => |ctx, args| {
            let a1: i16 = number(&args[0], "medius_device_pan delta");
            status(unsafe { medius_device_pan(ctx.device, a1) }, || Value::Null)
        },
        ("Device", "pan_now") => |ctx, args| {
            let a1: i16 = number(&args[0], "medius_device_pan_now delta");
            status(unsafe { medius_device_pan_now(ctx.device, a1) }, || {
                Value::Null
            })
        },
        ("Device", "flush_motion") => |ctx, _args| {
            status(unsafe { medius_device_flush_motion(ctx.device) }, || {
                Value::Null
            })
        },
        ("Device", "discard_motion") => |ctx, _args| {
            status(unsafe { medius_device_discard_motion(ctx.device) }, || {
                Value::Null
            })
        },
        ("Device", "move_axis") => |ctx, args| {
            let a1 = motion_from_json(&args[0])?;
            let a2 = enum_value("MoveTiming", &args[1])? as u8;
            let a3 = enum_value("PendingMotion", &args[2])? as u8;
            status(
                unsafe { medius_device_move_axis(ctx.device, a1, a2, a3) },
                || Value::Null,
            )
        },
        ("Device", "inject") => |ctx, args| {
            let a1 = usage_from_json(&args[0])?;
            let a2 = enum_value("Action", &args[1])? as u8;
            status(unsafe { medius_device_inject(ctx.device, a1, a2) }, || {
                Value::Null
            })
        },
        ("Device", "press") => |ctx, args| {
            let a1 = usage_from_json(&args[0])?;
            status(unsafe { medius_device_press(ctx.device, a1) }, || {
                Value::Null
            })
        },
        ("Device", "soft_release") => |ctx, args| {
            let a1 = usage_from_json(&args[0])?;
            status(
                unsafe { medius_device_soft_release(ctx.device, a1) },
                || Value::Null,
            )
        },
        ("Device", "force_release") => |ctx, args| {
            let a1 = usage_from_json(&args[0])?;
            status(
                unsafe { medius_device_force_release(ctx.device, a1) },
                || Value::Null,
            )
        },
        ("Device", "scale") => |ctx, args| {
            let a1 = lock_target_from_json(&args[0])?;
            let a2 = enum_value("Direction", &args[1])? as u8;
            let a3: i16 = number(&args[2], "medius_device_scale scale");
            status(
                unsafe { medius_device_scale(ctx.device, a1, a2, a3) },
                || Value::Null,
            )
        },
        ("Device", "scale_all") => |ctx, args| {
            let a1 = enum_value("Blanket", &args[0])? as u8;
            let a2 = enum_value("Direction", &args[1])? as u8;
            let a3: i16 = number(&args[2], "medius_device_scale_all scale");
            status(
                unsafe { medius_device_scale_all(ctx.device, a1, a2, a3) },
                || Value::Null,
            )
        },
        ("Device", "lock") => |ctx, args| {
            let a1 = lock_target_from_json(&args[0])?;
            let a2 = enum_value("Direction", &args[1])? as u8;
            status(unsafe { medius_device_lock(ctx.device, a1, a2) }, || {
                Value::Null
            })
        },
        ("Device", "unlock") => |ctx, args| {
            let a1 = lock_target_from_json(&args[0])?;
            let a2 = enum_value("Direction", &args[1])? as u8;
            status(unsafe { medius_device_unlock(ctx.device, a1, a2) }, || {
                Value::Null
            })
        },
        ("Device", "lock_all") => |ctx, args| {
            let a1 = enum_value("Blanket", &args[0])? as u8;
            let a2 = enum_value("Direction", &args[1])? as u8;
            status(
                unsafe { medius_device_lock_all(ctx.device, a1, a2) },
                || Value::Null,
            )
        },
        ("Device", "unlock_all") => |ctx, args| {
            let a1 = enum_value("Blanket", &args[0])? as u8;
            let a2 = enum_value("Direction", &args[1])? as u8;
            status(
                unsafe { medius_device_unlock_all(ctx.device, a1, a2) },
                || Value::Null,
            )
        },
        ("Device", "led") => |ctx, args| {
            let a1 = enum_value("LedTarget", &args[0])? as u8;
            let a2 = enum_value("LedMode", &args[1])? as u8;
            let a3: u8 = number(&args[2], "medius_device_led level");
            status(unsafe { medius_device_led(ctx.device, a1, a2, a3) }, || {
                Value::Null
            })
        },
        ("Device", "reset") => {
            |ctx, _args| status(unsafe { medius_device_reset(ctx.device) }, || Value::Null)
        }
        ("Device", "factory_reset") => |ctx, _args| {
            status(unsafe { medius_device_factory_reset(ctx.device) }, || {
                Value::Null
            })
        },
        ("Device", "reapply") => {
            |ctx, _args| status(unsafe { medius_device_reapply(ctx.device) }, || Value::Null)
        }
        ("Device", "reconnect") => |ctx, _args| {
            status(unsafe { medius_device_reconnect(ctx.device) }, || {
                Value::Null
            })
        },
        ("Device", "reboot") => |ctx, args| {
            let a1 = enum_value("RebootTarget", &args[0])? as u8;
            status(unsafe { medius_device_reboot(ctx.device, a1) }, || {
                Value::Null
            })
        },
        ("Device", "allow_imperfect_clones") => |ctx, args| {
            let a1 = flag(&args[0]);
            status(
                unsafe { medius_device_allow_imperfect_clones(ctx.device, a1) },
                || Value::Null,
            )
        },
        ("Device", "raw") => |ctx, args| {
            let a1: u8 = number(&args[0], "medius_device_raw ep_num");
            let a2 = enum_value("Direction", &args[1])? as u8;
            let a3 = unhex(&args[2], "medius_device_raw bytes");
            status(
                unsafe { medius_device_raw(ctx.device, a1, a2, a3.as_ptr(), a3.len()) },
                || Value::Null,
            )
        },
        ("Device", "set_rewrite") => |ctx, args| {
            let a1 = rewrite_rule_from_json(&args[0])?;
            status(
                unsafe { medius_device_set_rewrite(ctx.device, &a1) },
                || Value::Null,
            )
        },
        ("Device", "clear_rewrite") => |ctx, _args| {
            status(unsafe { medius_device_clear_rewrite(ctx.device) }, || {
                Value::Null
            })
        },
        ("Device", "query_rewrite") => |ctx, _args| {
            let mut out: MediusRewriteTable = unsafe { std::mem::zeroed() };
            status(
                unsafe { medius_device_query_rewrite(ctx.device, &mut out) },
                || rewrite_table_to_json(&out),
            )
        },
        ("Device", "query_rewrite_entry") => |ctx, args| {
            let a1: u8 = number(&args[0], "medius_device_query_rewrite_entry index");
            let mut out: MediusRewriteRule = unsafe { std::mem::zeroed() };
            status(
                unsafe { medius_device_query_rewrite_entry(ctx.device, a1, &mut out) },
                || rewrite_rule_to_json(&out),
            )
        },
        ("Device", "set_patch") => |ctx, args| {
            let a1 = patch_from_json(&args[0])?;
            status(unsafe { medius_device_set_patch(ctx.device, &a1) }, || {
                Value::Null
            })
        },
        ("Device", "apply_patch") => |ctx, _args| {
            status(unsafe { medius_device_apply_patch(ctx.device) }, || {
                Value::Null
            })
        },
        ("Device", "clear_patch") => |ctx, _args| {
            status(unsafe { medius_device_clear_patch(ctx.device) }, || {
                Value::Null
            })
        },
        ("Device", "query_patches") => |ctx, _args| {
            let mut out: MediusPatchSet = unsafe { std::mem::zeroed() };
            status(
                unsafe { medius_device_query_patches(ctx.device, &mut out) },
                || patch_set_to_json(&out),
            )
        },
        ("Device", "query_patch_entry") => |ctx, args| {
            let a1: u8 = number(&args[0], "medius_device_query_patch_entry index");
            let mut out: MediusPatch = unsafe { std::mem::zeroed() };
            status(
                unsafe { medius_device_query_patch_entry(ctx.device, a1, &mut out) },
                || patch_to_json(&out),
            )
        },
        ("Device", "transform") => |ctx, args| {
            let a1 = transform_from_json(&args[0])?;
            status(unsafe { medius_device_transform(ctx.device, &a1) }, || {
                Value::Null
            })
        },
        ("Device", "untransform") => |ctx, args| {
            let a1 = transform_from_json(&args[0])?;
            status(
                unsafe { medius_device_untransform(ctx.device, &a1) },
                || Value::Null,
            )
        },
        ("Device", "clear_transforms") => |ctx, _args| {
            status(
                unsafe { medius_device_clear_transforms(ctx.device) },
                || Value::Null,
            )
        },
        ("Device", "transform_swap") => |ctx, args| {
            let a1 = enum_value("Axis", &args[0])? as u8;
            let a2 = enum_value("Axis", &args[1])? as u8;
            status(
                unsafe { medius_device_transform_swap(ctx.device, a1, a2) },
                || Value::Null,
            )
        },
        ("Device", "transform_remap") => |ctx, args| {
            let a1 = lock_target_from_json(&args[0])?;
            let a2 = lock_target_from_json(&args[1])?;
            status(
                unsafe { medius_device_transform_remap(ctx.device, a1, a2) },
                || Value::Null,
            )
        },
        ("Device", "query_transforms") => |ctx, _args| {
            let mut out: MediusTransforms = unsafe { std::mem::zeroed() };
            status(
                unsafe { medius_device_query_transforms(ctx.device, &mut out) },
                || transforms_to_json(&out),
            )
        },
        ("Device", "set_name") => |ctx, args| {
            let a1 = c_string(&args[0]);
            status(
                unsafe { medius_device_set_name(ctx.device, a1.as_ptr()) },
                || Value::Null,
            )
        },
        ("Device", "clear_name") => |ctx, _args| {
            status(unsafe { medius_device_clear_name(ctx.device) }, || {
                Value::Null
            })
        },
        ("Device", "set_render") => |ctx, args| {
            let a1 = enum_value("RenderMode", &args[0])? as u8;
            let a2 = flag(&args[1]);
            status(
                unsafe { medius_device_set_render(ctx.device, a1, a2) },
                || Value::Null,
            )
        },
        ("Device", "set_spread") => |ctx, args| {
            let a1: u16 = number(&args[0], "medius_device_set_spread percent");
            status(unsafe { medius_device_set_spread(ctx.device, a1) }, || {
                Value::Null
            })
        },
        ("Device", "query_version") => |ctx, _args| {
            let mut out: MediusVersion = unsafe { std::mem::zeroed() };
            status(
                unsafe { medius_device_query_version(ctx.device, &mut out) },
                || version_to_json(&out),
            )
        },
        ("Device", "firmware_info") => |ctx, _args| {
            let mut out: MediusFirmwareInfo = unsafe { std::mem::zeroed() };
            status(
                unsafe { medius_device_firmware_info(ctx.device, &mut out) },
                || firmware_info_to_json(&out),
            )
        },
        ("Device", "abort_update") => |ctx, args| {
            let a1 = enum_value("UpdateTarget", &args[0])? as u8;
            status(
                unsafe { medius_device_abort_update(ctx.device, a1) },
                || Value::Null,
            )
        },
        ("Device", "activate_firmware") => |ctx, _args| {
            status(
                unsafe { medius_device_activate_firmware(ctx.device) },
                || Value::Null,
            )
        },
        ("Device", "wait_firmware_confirmed") => |ctx, _args| {
            let mut out: MediusFirmwareInfo = unsafe { std::mem::zeroed() };
            status(
                unsafe { medius_device_wait_firmware_confirmed(ctx.device, &mut out) },
                || firmware_info_to_json(&out),
            )
        },
        ("Device", "query_health") => |ctx, _args| {
            let mut out: MediusHealth = unsafe { std::mem::zeroed() };
            status(
                unsafe { medius_device_query_health(ctx.device, &mut out) },
                || health_to_json(&out),
            )
        },
        ("Device", "device_info") => |ctx, _args| {
            let mut out: MediusDeviceInfo = unsafe { std::mem::zeroed() };
            status(
                unsafe { medius_device_device_info(ctx.device, &mut out) },
                || device_info_to_json(&out),
            )
        },
        ("Device", "caps") => |ctx, _args| {
            let mut out: MediusCaps = unsafe { std::mem::zeroed() };
            status(unsafe { medius_device_caps(ctx.device, &mut out) }, || {
                caps_to_json(&out)
            })
        },
        ("Device", "query_rate") => |ctx, _args| {
            let mut out: MediusRate = unsafe { std::mem::zeroed() };
            status(
                unsafe { medius_device_query_rate(ctx.device, &mut out) },
                || rate_to_json(&out),
            )
        },
        ("Device", "query_stats") => |ctx, _args| {
            let mut out: MediusStats = unsafe { std::mem::zeroed() };
            status(
                unsafe { medius_device_query_stats(ctx.device, &mut out) },
                || stats_to_json(&out),
            )
        },
        ("Device", "query_locks") => |ctx, _args| {
            let mut out: MediusLocks = unsafe { std::mem::zeroed() };
            status(
                unsafe { medius_device_query_locks(ctx.device, &mut out) },
                || locks_to_json(&out),
            )
        },
        ("Device", "query_catch") => |ctx, _args| {
            let mut out: MediusCatchState = unsafe { std::mem::zeroed() };
            status(
                unsafe { medius_device_query_catch(ctx.device, &mut out) },
                || catch_state_to_json(&out),
            )
        },
        ("Device", "query_imperfect") => |ctx, _args| {
            let mut out: MediusImperfectStatus = unsafe { std::mem::zeroed() };
            status(
                unsafe { medius_device_query_imperfect(ctx.device, &mut out) },
                || imperfect_status_to_json(&out),
            )
        },
        ("Device", "query_emit_pace") => |ctx, _args| {
            let mut out: MediusEmitPaceStatus = unsafe { std::mem::zeroed() };
            status(
                unsafe { medius_device_query_emit_pace(ctx.device, &mut out) },
                || emit_pace_status_to_json(&out),
            )
        },
        ("Device", "query_bearing") => |ctx, _args| {
            let mut out: MediusBearing = unsafe { std::mem::zeroed() };
            status(
                unsafe { medius_device_query_bearing(ctx.device, &mut out) },
                || bearing_to_json(&out),
            )
        },
        ("Device", "query_render") => |ctx, _args| {
            let mut out: MediusRenderStatus = unsafe { std::mem::zeroed() };
            status(
                unsafe { medius_device_query_render(ctx.device, &mut out) },
                || render_status_to_json(&out),
            )
        },
        ("Device", "query_spread") => |ctx, _args| {
            let mut out: MediusSpreadStatus = unsafe { std::mem::zeroed() };
            status(
                unsafe { medius_device_query_spread(ctx.device, &mut out) },
                || spread_status_to_json(&out),
            )
        },
        ("Device", "counters") => |ctx, _args| {
            let mut out: MediusCountersSnapshot = unsafe { std::mem::zeroed() };
            status(
                unsafe { medius_device_counters(ctx.device, &mut out) },
                || counters_snapshot_to_json(&out),
            )
        },
        ("", "default_query_timeout_ms") => {
            |_ctx, _args| Ok(Value::from(unsafe { medius_default_query_timeout_ms() }))
        }
        ("", "default_transfer_timeout_ms") => {
            |_ctx, _args| Ok(Value::from(unsafe { medius_default_transfer_timeout_ms() }))
        }
        ("", "default_keepalive_cadence_ms") => |_ctx, _args| {
            Ok(Value::from(unsafe {
                medius_default_keepalive_cadence_ms()
            }))
        },
        ("", "abi_version") => |_ctx, _args| Ok(Value::from(unsafe { medius_abi_version() })),
        ("", "version") => |_ctx, _args| Ok(c_text(unsafe { medius_version_string() })),
        ("Timeline", "reset") => |ctx, args| {
            let a1 = enum_value("ClockDomain", &args[0])? as u8;
            {
                unsafe { medius_timeline_reset(ctx.timeline, a1) };
                Ok(Value::Null)
            }
        },
        ("Timeline", "samples") => |ctx, args| {
            let a1 = enum_value("ClockDomain", &args[0])? as u8;
            Ok(Value::from(unsafe {
                medius_timeline_samples(ctx.timeline, a1)
            }))
        },
        ("MockBox", "set_version") => |ctx, args| {
            let a1 = version_from_json(&args[0])?;
            {
                unsafe { medius_mock_set_version(ctx.mock, a1) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "set_health") => |ctx, args| {
            let a1 = health_from_json(&args[0])?;
            {
                unsafe { medius_mock_set_health(ctx.mock, a1) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "set_device_info") => |ctx, args| {
            let a1 = device_info_from_json(&args[0])?;
            {
                unsafe { medius_mock_set_device_info(ctx.mock, a1) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "set_caps") => |ctx, args| {
            let a1 = caps_from_json(&args[0])?;
            {
                unsafe { medius_mock_set_caps(ctx.mock, a1) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "set_mouse_caps") => |ctx, args| {
            let a1 = mouse_caps_from_json(&args[0])?;
            {
                unsafe { medius_mock_set_mouse_caps(ctx.mock, a1) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "set_kbd_caps") => |ctx, args| {
            let a1 = kbd_caps_from_json(&args[0])?;
            {
                unsafe { medius_mock_set_kbd_caps(ctx.mock, a1) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "set_rate") => |ctx, args| {
            let a1 = rate_from_json(&args[0])?;
            {
                unsafe { medius_mock_set_rate(ctx.mock, a1) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "set_stats") => |ctx, args| {
            let a1 = stats_from_json(&args[0])?;
            {
                unsafe { medius_mock_set_stats(ctx.mock, a1) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "set_locks") => |ctx, args| {
            let a1 = locks_from_json(&args[0])?;
            {
                unsafe { medius_mock_set_locks(ctx.mock, a1) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "set_catch_state") => |ctx, args| {
            let a1 = catch_state_from_json(&args[0])?;
            {
                unsafe { medius_mock_set_catch_state(ctx.mock, a1) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "set_imperfect_status") => |ctx, args| {
            let a1 = imperfect_status_from_json(&args[0])?;
            {
                unsafe { medius_mock_set_imperfect_status(ctx.mock, a1) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "set_transfer_reply") => |ctx, args| {
            let a1 = enum_value("TransferStatus", &args[0])? as u8;
            let a2 = unhex(&args[1], "medius_mock_set_transfer_reply data");
            {
                unsafe { medius_mock_set_transfer_reply(ctx.mock, a1, a2.as_ptr(), a2.len()) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "set_render") => |ctx, args| {
            let a1 = enum_value("RenderMode", &args[0])? as u8;
            let a2 = flag(&args[1]);
            let a3 = flag(&args[2]);
            {
                unsafe { medius_mock_set_render(ctx.mock, a1, a2, a3) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "set_spread_learned") => |ctx, args| {
            let a1: u32 = number(&args[0], "medius_mock_set_spread_learned period_us");
            {
                unsafe { medius_mock_set_spread_learned(ctx.mock, a1) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "set_advertised_hz") => |ctx, args| {
            let a1: u16 = number(&args[0], "medius_mock_set_advertised_hz hz");
            {
                unsafe { medius_mock_set_advertised_hz(ctx.mock, a1) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "set_clip_status") => |ctx, args| {
            let a1 = clip_status_from_json(&args[0])?;
            {
                unsafe { medius_mock_set_clip_status(ctx.mock, a1) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "set_clip_settings") => |ctx, args| {
            let a1 = clip_settings_from_json(&args[0])?;
            {
                unsafe { medius_mock_set_clip_settings(ctx.mock, a1) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "restart") => |ctx, _args| {
            unsafe { medius_mock_restart(ctx.mock) };
            Ok(Value::Null)
        },
        ("MockBox", "link_lost") => |ctx, _args| {
            unsafe { medius_mock_link_lost(ctx.mock) };
            Ok(Value::Null)
        },
        ("MockBox", "detach") => |ctx, args| {
            let a1 = flag(&args[0]);
            {
                unsafe { medius_mock_detach(ctx.mock, a1) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "attach") => |ctx, _args| {
            unsafe { medius_mock_attach(ctx.mock) };
            Ok(Value::Null)
        },
        ("MockBox", "refuse_next_activate") => |ctx, args| {
            let a1: u8 = number(&args[0], "medius_mock_refuse_next_activate status");
            {
                unsafe { medius_mock_refuse_next_activate(ctx.mock, a1) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "reboot_after_activate") => |ctx, args| {
            let a1: u32 = number(&args[0], "medius_mock_reboot_after_activate unanswered");
            let a2: u32 = number(&args[1], "medius_mock_reboot_after_activate pending");
            {
                unsafe { medius_mock_reboot_after_activate(ctx.mock, a1, a2) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "silent") => |ctx, _args| {
            unsafe { medius_mock_silent(ctx.mock) };
            Ok(Value::Null)
        },
        ("MockBox", "push_log") => |ctx, args| {
            let a1 = enum_value("LogLevel", &args[0])? as u8;
            let a2 = c_string(&args[1]);
            {
                unsafe { medius_mock_push_log(ctx.mock, a1, a2.as_ptr()) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "push_motion") => |ctx, args| {
            let a1: u8 = number(&args[0], "medius_mock_push_motion seq");
            let a2: u32 = number(&args[1], "medius_mock_push_motion ts_us");
            let a3 = motion_event_from_json(&args[2])?;
            {
                unsafe { medius_mock_push_motion(ctx.mock, a1, a2, a3) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "push_usages") => |ctx, args| {
            let a1: u8 = number(&args[0], "medius_mock_push_usages seq");
            let a2: u32 = number(&args[1], "medius_mock_push_usages ts_us");
            let a3 = usage_event_from_json(&args[2])?;
            {
                unsafe { medius_mock_push_usages(ctx.mock, a1, a2, &a3) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "push_traffic") => |ctx, args| {
            let a1: u8 = number(&args[0], "medius_mock_push_traffic seq");
            let a2: u32 = number(&args[1], "medius_mock_push_traffic ts_us");
            let a3 = enum_value("ClockDomain", &args[2])? as u8;
            let a4 = traffic_event_from_json(&args[3])?;
            {
                unsafe { medius_mock_push_traffic(ctx.mock, a1, a2, a3, &a4) };
                Ok(Value::Null)
            }
        },
        ("MockBox", "recorded") => {
            |ctx, _args| Ok(Value::from(unsafe { medius_mock_recorded(ctx.mock) }))
        }
        ("MockBox", "saw") => |ctx, args| {
            let a1 = enum_value("FrameType", &args[0])? as u8;
            Ok(Value::from(unsafe { medius_mock_saw(ctx.mock, a1) }))
        },
        ("MockBox", "clear_recorded") => |ctx, _args| {
            unsafe { medius_mock_clear_recorded(ctx.mock) };
            Ok(Value::Null)
        },
        _ => return None,
    };
    Some(run(ctx, args))
}

pub(super) fn enum_name(name: &str, value: i64) -> Value {
    let named = match (name, value) {
        ("Status", 0) => "Ok",
        ("Status", 1) => "ErrIo",
        ("Status", 2) => "ErrNotFound",
        ("Status", 3) => "ErrNoReply",
        ("Status", 4) => "ErrBadProtoVer",
        ("Status", 5) => "ErrQueryTimeout",
        ("Status", 6) => "ErrDisconnected",
        ("Status", 7) => "ErrFrameTooLong",
        ("Status", 8) => "ErrUpdate",
        ("Status", 9) => "ErrInvalidArg",
        ("Status", 10) => "ErrPanic",
        ("Status", 11) => "ErrUnknown",
        ("Status", 12) => "ErrCatchTableFull",
        ("Status", 13) => "ErrEmptySubscription",
        ("Status", 14) => "ErrCaptureNotApplicable",
        ("Status", 15) => "ErrNotAnInputFilter",
        ("Status", 16) => "ErrWildcardNotInput",
        ("Status", 17) => "ErrHalfEdgeInputFilter",
        ("Status", 18) => "ErrReservedId",
        ("Status", 19) => "ErrRelativeDirection",
        ("Status", 20) => "ErrLockScaleRange",
        ("Status", 21) => "ErrLockScaleUsage",
        ("Status", 22) => "ErrImperfectRequired",
        ("Status", 23) => "ErrRewriteMaskLength",
        ("Status", 24) => "ErrRewriteActionClass",
        ("Status", 25) => "ErrRewritePayloadTooLarge",
        ("Status", 26) => "ErrRewriteTableFull",
        ("Status", 27) => "ErrTransformOpFields",
        ("Status", 28) => "ErrTransformTableFull",
        ("Status", 29) => "ErrRawDirection",
        ("Status", 30) => "ErrClipFrameCount",
        ("Status", 31) => "ErrClipFrameTooLong",
        ("Status", 32) => "ErrClipTransferData",
        ("Status", 33) => "ErrClipPacketTrigger",
        ("Status", 34) => "ErrRewriteMatchTooLong",
        ("Status", 35) => "ErrRewritePoolFull",
        ("EmitMode", 0) => "Learned",
        ("EmitMode", 1) => "Interval",
        ("EmitMode", 2) => "Fixed",
        ("BearingMode", 0) => "PerAxis",
        ("BearingMode", 1) => "Vector",
        ("RenderMode", 0) => "Off",
        ("RenderMode", 1) => "Stock",
        ("RenderMode", 2) => "Despiked",
        ("RenderMode", 3) => "Unsmoothed",
        ("ControlStatus", 0) => "Ok",
        ("ControlStatus", 1) => "Stalled",
        ("ControlStatus", 2) => "Naked",
        ("ControlStatus", 3) => "Other",
        ("BusEventKind", 0) => "Reset",
        ("BusEventKind", 1) => "Suspend",
        ("BusEventKind", 2) => "Resume",
        ("BusEventKind", 3) => "Configured",
        ("BusEventKind", 4) => "Deconfigured",
        ("BusEventKind", 5) => "SetInterface",
        ("BusEventKind", 6) => "DeviceAttached",
        ("BusEventKind", 7) => "DeviceDetached",
        ("BusEventKind", 8) => "CloneUp",
        ("BusEventKind", 9) => "CloneDown",
        ("CatchEventKind", 0) => "Motion",
        ("CatchEventKind", 1) => "Usages",
        ("CatchEventKind", 2) => "Traffic",
        ("ClockDomain", 0) => "HostChip",
        ("ClockDomain", 1) => "DeviceChip",
        ("InputKind", 0) => "Press",
        ("InputKind", 1) => "Release",
        ("InputKind", 2) => "Motion",
        ("LogLevel", 0) => "Error",
        ("LogLevel", 1) => "Warn",
        ("LogLevel", 2) => "Info",
        ("LogLevel", 3) => "Debug",
        ("LogLevel", 4) => "Verbose",
        ("FrameType", 1) => "Move",
        ("FrameType", 3) => "Inject",
        ("FrameType", 4) => "Reset",
        ("FrameType", 5) => "Query",
        ("FrameType", 6) => "Resp",
        ("FrameType", 7) => "RebootDl",
        ("FrameType", 8) => "Log",
        ("FrameType", 9) => "Led",
        ("FrameType", 10) => "Lock",
        ("FrameType", 11) => "Catch",
        ("FrameType", 12) => "MotionEvent",
        ("FrameType", 15) => "UsageEvent",
        ("FrameType", 22) => "TrafficEvent",
        ("FrameType", 17) => "Option",
        ("FrameType", 18) => "ClipAppend",
        ("FrameType", 19) => "ClipCtrl",
        ("FrameType", 20) => "ClipSet",
        ("FrameType", 21) => "ClipTrigger",
        ("FrameType", 23) => "Update",
        ("FrameType", 24) => "UpdateResp",
        ("FrameType", 25) => "Raw",
        ("FrameType", 26) => "Transfer",
        ("FrameType", 27) => "TransferResp",
        ("FrameType", 28) => "Rewrite",
        ("FrameType", 29) => "Patch",
        ("FrameType", 30) => "Transform",
        ("Action", 0) => "SoftRelease",
        ("Action", 1) => "Press",
        ("Action", 2) => "ForceRelease",
        ("Axis", 0) => "X",
        ("Axis", 1) => "Y",
        ("Axis", 2) => "Wheel",
        ("Axis", 3) => "Pan",
        ("Blanket", 0) => "Aim",
        ("Blanket", 1) => "Wheel",
        ("Blanket", 2) => "Buttons",
        ("Blanket", 3) => "Keys",
        ("Blanket", 4) => "Media",
        ("Class", 0) => "Button",
        ("Class", 1) => "Key",
        ("Class", 2) => "Media",
        ("ClipAction", 0) => "Start",
        ("ClipAction", 1) => "Stop",
        ("ClipAction", 2) => "Pause",
        ("ClipAction", 3) => "Resume",
        ("ClipAction", 4) => "Restart",
        ("ClipAction", 5) => "Toggle",
        ("ClipState", 0) => "Idle",
        ("ClipState", 1) => "Playing",
        ("ClipState", 2) => "Paused",
        ("ClipState", 3) => "Faulted",
        ("DeviceKind", 0) => "Unknown",
        ("DeviceKind", 1) => "Keyboard",
        ("DeviceKind", 2) => "Mouse",
        ("Direction", 0) => "Both",
        ("Direction", 1) => "Positive",
        ("Direction", 2) => "Negative",
        ("Direction", 3) => "With",
        ("Direction", 4) => "Against",
        ("Edge", 0) => "Both",
        ("Edge", 1) => "Press",
        ("Edge", 2) => "Release",
        ("LedMode", 0) => "Auto",
        ("LedMode", 1) => "Off",
        ("LedMode", 2) => "Solid",
        ("LedMode", 3) => "Blink",
        ("LedTarget", 0) => "Device",
        ("LedTarget", 1) => "Host",
        ("LedTarget", 2) => "Both",
        ("LockTargetKind", 0) => "X",
        ("LockTargetKind", 1) => "Y",
        ("LockTargetKind", 2) => "Wheel",
        ("LockTargetKind", 3) => "Pan",
        ("LockTargetKind", 4) => "Usage",
        ("MotionKind", 0) => "Cursor",
        ("MotionKind", 1) => "Wheel",
        ("MotionKind", 2) => "Pan",
        ("MoveTiming", 0) => "Ride",
        ("MoveTiming", 1) => "Now",
        ("PatchSection", 0) => "Device",
        ("PatchSection", 1) => "Config",
        ("PatchSection", 2) => "Report",
        ("PatchSection", 3) => "String",
        ("PatchSection", 4) => "Bos",
        ("PendingMotion", 0) => "Keep",
        ("PendingMotion", 1) => "Flush",
        ("PendingMotion", 2) => "Discard",
        ("RebootTarget", 0) => "DeviceDownload",
        ("RebootTarget", 1) => "HostDownload",
        ("RebootTarget", 2) => "DeviceRun",
        ("RebootTarget", 3) => "HostRun",
        ("RewriteAction", 0) => "Pass",
        ("RewriteAction", 1) => "Drop",
        ("RewriteAction", 2) => "Patch",
        ("RewriteAction", 3) => "Replace",
        ("RewriteAction", 4) => "Answer",
        ("RewriteAction", 5) => "Stall",
        ("RewriteAction", 6) => "Nak",
        ("RewriteAction", 7) => "ReplyPatch",
        ("RewriteAction", 8) => "ReplyReplace",
        ("RewriteClass", 4) => "HidIn",
        ("RewriteClass", 5) => "HidOut",
        ("RewriteClass", 6) => "VendorInterrupt",
        ("RewriteClass", 7) => "VendorBulk",
        ("RewriteClass", 8) => "Control",
        ("RewriteClass", 9) => "Emit",
        ("RewriteClass", 255) => "Any",
        ("TransferStatus", 0) => "Ok",
        ("TransferStatus", 252) => "Refused",
        ("TransferStatus", 253) => "Stall",
        ("TransferStatus", 254) => "Nak",
        ("TransferStatus", 255) => "NoDevice",
        ("TransformOp", 0) => "Remap",
        ("TransformOp", 1) => "Swap",
        ("CatchClass", 0) => "Button",
        ("CatchClass", 1) => "Key",
        ("CatchClass", 2) => "Media",
        ("CatchClass", 3) => "Axis",
        ("CatchClass", 4) => "HidIn",
        ("CatchClass", 5) => "HidOut",
        ("CatchClass", 6) => "VendorInterrupt",
        ("CatchClass", 7) => "VendorBulk",
        ("CatchClass", 8) => "Control",
        ("CatchClass", 9) => "Emit",
        ("CatchClass", 10) => "Bus",
        ("CatchClass", 11) => "ClipTransfer",
        ("TrafficClass", 4) => "HidIn",
        ("TrafficClass", 5) => "HidOut",
        ("TrafficClass", 6) => "VendorInterrupt",
        ("TrafficClass", 7) => "VendorBulk",
        ("TrafficClass", 8) => "Control",
        ("TrafficClass", 9) => "Emit",
        ("TrafficClass", 10) => "Bus",
        ("TrafficClass", 11) => "ClipTransfer",
        ("ImageState", 0) => "New",
        ("ImageState", 1) => "PendingVerify",
        ("ImageState", 2) => "Valid",
        ("ImageState", 3) => "Invalid",
        ("ImageState", 4) => "Aborted",
        ("ImageState", 255) => "Unknown",
        ("UpdateTarget", 0) => "Device",
        ("UpdateTarget", 1) => "Host",
        _ => return Value::from(value),
    };
    Value::from(named)
}

pub(super) fn enum_number(name: &str, member: &str) -> i64 {
    match (name, member) {
        ("Status", "Ok") => 0,
        ("Status", "ErrIo") => 1,
        ("Status", "ErrNotFound") => 2,
        ("Status", "ErrNoReply") => 3,
        ("Status", "ErrBadProtoVer") => 4,
        ("Status", "ErrQueryTimeout") => 5,
        ("Status", "ErrDisconnected") => 6,
        ("Status", "ErrFrameTooLong") => 7,
        ("Status", "ErrUpdate") => 8,
        ("Status", "ErrInvalidArg") => 9,
        ("Status", "ErrPanic") => 10,
        ("Status", "ErrUnknown") => 11,
        ("Status", "ErrCatchTableFull") => 12,
        ("Status", "ErrEmptySubscription") => 13,
        ("Status", "ErrCaptureNotApplicable") => 14,
        ("Status", "ErrNotAnInputFilter") => 15,
        ("Status", "ErrWildcardNotInput") => 16,
        ("Status", "ErrHalfEdgeInputFilter") => 17,
        ("Status", "ErrReservedId") => 18,
        ("Status", "ErrRelativeDirection") => 19,
        ("Status", "ErrLockScaleRange") => 20,
        ("Status", "ErrLockScaleUsage") => 21,
        ("Status", "ErrImperfectRequired") => 22,
        ("Status", "ErrRewriteMaskLength") => 23,
        ("Status", "ErrRewriteActionClass") => 24,
        ("Status", "ErrRewritePayloadTooLarge") => 25,
        ("Status", "ErrRewriteTableFull") => 26,
        ("Status", "ErrTransformOpFields") => 27,
        ("Status", "ErrTransformTableFull") => 28,
        ("Status", "ErrRawDirection") => 29,
        ("Status", "ErrClipFrameCount") => 30,
        ("Status", "ErrClipFrameTooLong") => 31,
        ("Status", "ErrClipTransferData") => 32,
        ("Status", "ErrClipPacketTrigger") => 33,
        ("Status", "ErrRewriteMatchTooLong") => 34,
        ("Status", "ErrRewritePoolFull") => 35,
        ("EmitMode", "Learned") => 0,
        ("EmitMode", "Interval") => 1,
        ("EmitMode", "Fixed") => 2,
        ("BearingMode", "PerAxis") => 0,
        ("BearingMode", "Vector") => 1,
        ("RenderMode", "Off") => 0,
        ("RenderMode", "Stock") => 1,
        ("RenderMode", "Despiked") => 2,
        ("RenderMode", "Unsmoothed") => 3,
        ("ControlStatus", "Ok") => 0,
        ("ControlStatus", "Stalled") => 1,
        ("ControlStatus", "Naked") => 2,
        ("ControlStatus", "Other") => 3,
        ("BusEventKind", "Reset") => 0,
        ("BusEventKind", "Suspend") => 1,
        ("BusEventKind", "Resume") => 2,
        ("BusEventKind", "Configured") => 3,
        ("BusEventKind", "Deconfigured") => 4,
        ("BusEventKind", "SetInterface") => 5,
        ("BusEventKind", "DeviceAttached") => 6,
        ("BusEventKind", "DeviceDetached") => 7,
        ("BusEventKind", "CloneUp") => 8,
        ("BusEventKind", "CloneDown") => 9,
        ("CatchEventKind", "Motion") => 0,
        ("CatchEventKind", "Usages") => 1,
        ("CatchEventKind", "Traffic") => 2,
        ("ClockDomain", "HostChip") => 0,
        ("ClockDomain", "DeviceChip") => 1,
        ("InputKind", "Press") => 0,
        ("InputKind", "Release") => 1,
        ("InputKind", "Motion") => 2,
        ("LogLevel", "Error") => 0,
        ("LogLevel", "Warn") => 1,
        ("LogLevel", "Info") => 2,
        ("LogLevel", "Debug") => 3,
        ("LogLevel", "Verbose") => 4,
        ("FrameType", "Move") => 1,
        ("FrameType", "Inject") => 3,
        ("FrameType", "Reset") => 4,
        ("FrameType", "Query") => 5,
        ("FrameType", "Resp") => 6,
        ("FrameType", "RebootDl") => 7,
        ("FrameType", "Log") => 8,
        ("FrameType", "Led") => 9,
        ("FrameType", "Lock") => 10,
        ("FrameType", "Catch") => 11,
        ("FrameType", "MotionEvent") => 12,
        ("FrameType", "UsageEvent") => 15,
        ("FrameType", "TrafficEvent") => 22,
        ("FrameType", "Option") => 17,
        ("FrameType", "ClipAppend") => 18,
        ("FrameType", "ClipCtrl") => 19,
        ("FrameType", "ClipSet") => 20,
        ("FrameType", "ClipTrigger") => 21,
        ("FrameType", "Update") => 23,
        ("FrameType", "UpdateResp") => 24,
        ("FrameType", "Raw") => 25,
        ("FrameType", "Transfer") => 26,
        ("FrameType", "TransferResp") => 27,
        ("FrameType", "Rewrite") => 28,
        ("FrameType", "Patch") => 29,
        ("FrameType", "Transform") => 30,
        ("Action", "SoftRelease") => 0,
        ("Action", "Press") => 1,
        ("Action", "ForceRelease") => 2,
        ("Axis", "X") => 0,
        ("Axis", "Y") => 1,
        ("Axis", "Wheel") => 2,
        ("Axis", "Pan") => 3,
        ("Blanket", "Aim") => 0,
        ("Blanket", "Wheel") => 1,
        ("Blanket", "Buttons") => 2,
        ("Blanket", "Keys") => 3,
        ("Blanket", "Media") => 4,
        ("Class", "Button") => 0,
        ("Class", "Key") => 1,
        ("Class", "Media") => 2,
        ("ClipAction", "Start") => 0,
        ("ClipAction", "Stop") => 1,
        ("ClipAction", "Pause") => 2,
        ("ClipAction", "Resume") => 3,
        ("ClipAction", "Restart") => 4,
        ("ClipAction", "Toggle") => 5,
        ("ClipState", "Idle") => 0,
        ("ClipState", "Playing") => 1,
        ("ClipState", "Paused") => 2,
        ("ClipState", "Faulted") => 3,
        ("DeviceKind", "Unknown") => 0,
        ("DeviceKind", "Keyboard") => 1,
        ("DeviceKind", "Mouse") => 2,
        ("Direction", "Both") => 0,
        ("Direction", "Positive") => 1,
        ("Direction", "Negative") => 2,
        ("Direction", "With") => 3,
        ("Direction", "Against") => 4,
        ("Direction", "Press") => 1,
        ("Direction", "Release") => 2,
        ("Direction", "In") => 1,
        ("Direction", "Out") => 2,
        ("Edge", "Both") => 0,
        ("Edge", "Press") => 1,
        ("Edge", "Release") => 2,
        ("LedMode", "Auto") => 0,
        ("LedMode", "Off") => 1,
        ("LedMode", "Solid") => 2,
        ("LedMode", "Blink") => 3,
        ("LedTarget", "Device") => 0,
        ("LedTarget", "Host") => 1,
        ("LedTarget", "Both") => 2,
        ("LockTargetKind", "X") => 0,
        ("LockTargetKind", "Y") => 1,
        ("LockTargetKind", "Wheel") => 2,
        ("LockTargetKind", "Pan") => 3,
        ("LockTargetKind", "Usage") => 4,
        ("MotionKind", "Cursor") => 0,
        ("MotionKind", "Wheel") => 1,
        ("MotionKind", "Pan") => 2,
        ("MoveTiming", "Ride") => 0,
        ("MoveTiming", "Now") => 1,
        ("PatchSection", "Device") => 0,
        ("PatchSection", "Config") => 1,
        ("PatchSection", "Report") => 2,
        ("PatchSection", "String") => 3,
        ("PatchSection", "Bos") => 4,
        ("PendingMotion", "Keep") => 0,
        ("PendingMotion", "Flush") => 1,
        ("PendingMotion", "Discard") => 2,
        ("RebootTarget", "DeviceDownload") => 0,
        ("RebootTarget", "HostDownload") => 1,
        ("RebootTarget", "DeviceRun") => 2,
        ("RebootTarget", "HostRun") => 3,
        ("RewriteAction", "Pass") => 0,
        ("RewriteAction", "Drop") => 1,
        ("RewriteAction", "Patch") => 2,
        ("RewriteAction", "Replace") => 3,
        ("RewriteAction", "Answer") => 4,
        ("RewriteAction", "Stall") => 5,
        ("RewriteAction", "Nak") => 6,
        ("RewriteAction", "ReplyPatch") => 7,
        ("RewriteAction", "ReplyReplace") => 8,
        ("RewriteClass", "HidIn") => 4,
        ("RewriteClass", "HidOut") => 5,
        ("RewriteClass", "VendorInterrupt") => 6,
        ("RewriteClass", "VendorBulk") => 7,
        ("RewriteClass", "Control") => 8,
        ("RewriteClass", "Emit") => 9,
        ("RewriteClass", "Any") => 255,
        ("TransferStatus", "Ok") => 0,
        ("TransferStatus", "Refused") => 252,
        ("TransferStatus", "Stall") => 253,
        ("TransferStatus", "Nak") => 254,
        ("TransferStatus", "NoDevice") => 255,
        ("TransformOp", "Remap") => 0,
        ("TransformOp", "Swap") => 1,
        ("CatchClass", "Button") => 0,
        ("CatchClass", "Key") => 1,
        ("CatchClass", "Media") => 2,
        ("CatchClass", "Axis") => 3,
        ("CatchClass", "HidIn") => 4,
        ("CatchClass", "HidOut") => 5,
        ("CatchClass", "VendorInterrupt") => 6,
        ("CatchClass", "VendorBulk") => 7,
        ("CatchClass", "Control") => 8,
        ("CatchClass", "Emit") => 9,
        ("CatchClass", "Bus") => 10,
        ("CatchClass", "ClipTransfer") => 11,
        ("TrafficClass", "HidIn") => 4,
        ("TrafficClass", "HidOut") => 5,
        ("TrafficClass", "VendorInterrupt") => 6,
        ("TrafficClass", "VendorBulk") => 7,
        ("TrafficClass", "Control") => 8,
        ("TrafficClass", "Emit") => 9,
        ("TrafficClass", "Bus") => 10,
        ("TrafficClass", "ClipTransfer") => 11,
        ("ImageState", "New") => 0,
        ("ImageState", "PendingVerify") => 1,
        ("ImageState", "Valid") => 2,
        ("ImageState", "Invalid") => 3,
        ("ImageState", "Aborted") => 4,
        ("ImageState", "Unknown") => 255,
        ("UpdateTarget", "Device") => 0,
        ("UpdateTarget", "Host") => 1,
        _ => panic!("{name} has no member {member}"),
    }
}

// whether a binding refuses a value of this enum it has no name for, before medius sees it
pub(super) fn checked(name: &str) -> bool {
    matches!(
        name,
        "EmitMode"
            | "BearingMode"
            | "RenderMode"
            | "ControlStatus"
            | "BusEventKind"
            | "CatchEventKind"
            | "ClockDomain"
            | "InputKind"
            | "LogLevel"
            | "FrameType"
            | "Action"
            | "Axis"
            | "Blanket"
            | "Class"
            | "ClipAction"
            | "ClipState"
            | "DeviceKind"
            | "Direction"
            | "Edge"
            | "LedMode"
            | "LedTarget"
            | "LockTargetKind"
            | "MotionKind"
            | "MoveTiming"
            | "PatchSection"
            | "PendingMotion"
            | "RebootTarget"
            | "RewriteAction"
            | "RewriteClass"
            | "TransformOp"
            | "CatchClass"
            | "TrafficClass"
            | "ImageState"
            | "UpdateTarget"
    )
}

pub(super) fn bearing_mode_from(value: i64) -> Result<MediusBearingMode, Failure> {
    Ok(match value {
        0 => MediusBearingMode::PerAxis,
        1 => MediusBearingMode::Vector,
        _ => {
            return Err(Failure::Refused(format!(
                "MediusBearingMode has no {value}"
            )));
        }
    })
}

pub(super) fn bus_event_kind_from(value: i64) -> Result<MediusBusEventKind, Failure> {
    Ok(match value {
        0 => MediusBusEventKind::Reset,
        1 => MediusBusEventKind::Suspend,
        2 => MediusBusEventKind::Resume,
        3 => MediusBusEventKind::Configured,
        4 => MediusBusEventKind::Deconfigured,
        5 => MediusBusEventKind::SetInterface,
        6 => MediusBusEventKind::DeviceAttached,
        7 => MediusBusEventKind::DeviceDetached,
        8 => MediusBusEventKind::CloneUp,
        9 => MediusBusEventKind::CloneDown,
        _ => {
            return Err(Failure::Refused(format!(
                "MediusBusEventKind has no {value}"
            )));
        }
    })
}

pub(super) fn log_level_from(value: i64) -> Result<MediusLogLevel, Failure> {
    Ok(match value {
        0 => MediusLogLevel::Error,
        1 => MediusLogLevel::Warn,
        2 => MediusLogLevel::Info,
        3 => MediusLogLevel::Debug,
        4 => MediusLogLevel::Verbose,
        _ => return Err(Failure::Refused(format!("MediusLogLevel has no {value}"))),
    })
}

pub(super) fn render_mode_from(value: i64) -> Result<MediusRenderMode, Failure> {
    Ok(match value {
        0 => MediusRenderMode::Off,
        1 => MediusRenderMode::Stock,
        2 => MediusRenderMode::Despiked,
        3 => MediusRenderMode::Unsmoothed,
        _ => return Err(Failure::Refused(format!("MediusRenderMode has no {value}"))),
    })
}
