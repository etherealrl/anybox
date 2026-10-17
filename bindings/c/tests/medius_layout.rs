// made by bindings/csharp/tools/gen-medius.cs from bindings/c/include/medius.h, don't edit it by hand. it
// writes the real size, alignment and field offsets of every medius-capi struct, which the C# package is held to
use std::mem::{align_of, offset_of, size_of};

use medius_capi::*;

#[test]
fn the_medius_layout_is_the_committed_one() {
    let mut layout = String::new();
    layout += &format!(
        "MediusUsage size {} align {}\n",
        size_of::<MediusUsage>(),
        align_of::<MediusUsage>()
    );
    layout += &format!("  kind {}\n", offset_of!(MediusUsage, kind));
    layout += &format!("  id {}\n", offset_of!(MediusUsage, id));
    layout += &format!(
        "MediusSetup size {} align {}\n",
        size_of::<MediusSetup>(),
        align_of::<MediusSetup>()
    );
    layout += &format!("  request_type {}\n", offset_of!(MediusSetup, request_type));
    layout += &format!("  request {}\n", offset_of!(MediusSetup, request));
    layout += &format!("  value {}\n", offset_of!(MediusSetup, value));
    layout += &format!("  index {}\n", offset_of!(MediusSetup, index));
    layout += &format!("  length {}\n", offset_of!(MediusSetup, length));
    layout += &format!(
        "MediusClipTrigger size {} align {}\n",
        size_of::<MediusClipTrigger>(),
        align_of::<MediusClipTrigger>()
    );
    layout += &format!("  on {}\n", offset_of!(MediusClipTrigger, on));
    layout += &format!("  edge {}\n", offset_of!(MediusClipTrigger, edge));
    layout += &format!("  action {}\n", offset_of!(MediusClipTrigger, action));
    layout += &format!("  consume {}\n", offset_of!(MediusClipTrigger, consume));
    layout += &format!(
        "MediusClipPacketTrigger size {} align {}\n",
        size_of::<MediusClipPacketTrigger>(),
        align_of::<MediusClipPacketTrigger>()
    );
    layout += &format!("  class_ {}\n", offset_of!(MediusClipPacketTrigger, class));
    layout += &format!("  id {}\n", offset_of!(MediusClipPacketTrigger, id));
    layout += &format!(
        "  direction {}\n",
        offset_of!(MediusClipPacketTrigger, direction)
    );
    layout += &format!("  action {}\n", offset_of!(MediusClipPacketTrigger, action));
    layout += &format!(
        "  consume {}\n",
        offset_of!(MediusClipPacketTrigger, consume)
    );
    layout += &format!(
        "  once_per_run {}\n",
        offset_of!(MediusClipPacketTrigger, once_per_run)
    );
    layout += &format!(
        "  selector_len {}\n",
        offset_of!(MediusClipPacketTrigger, selector_len)
    );
    layout += &format!(
        "  match_len {}\n",
        offset_of!(MediusClipPacketTrigger, match_len)
    );
    layout += &format!(
        "  mask_len {}\n",
        offset_of!(MediusClipPacketTrigger, mask_len)
    );
    layout += &format!(
        "  match_bytes {}\n",
        offset_of!(MediusClipPacketTrigger, match_bytes)
    );
    layout += &format!("  mask {}\n", offset_of!(MediusClipPacketTrigger, mask));
    layout += &format!("  hits {}\n", offset_of!(MediusClipPacketTrigger, hits));
    layout += &format!(
        "MediusClipStatus size {} align {}\n",
        size_of::<MediusClipStatus>(),
        align_of::<MediusClipStatus>()
    );
    layout += &format!("  state {}\n", offset_of!(MediusClipStatus, state));
    layout += &format!("  free {}\n", offset_of!(MediusClipStatus, free));
    layout += &format!("  total {}\n", offset_of!(MediusClipStatus, total));
    layout += &format!("  played {}\n", offset_of!(MediusClipStatus, played));
    layout += &format!("  ticks {}\n", offset_of!(MediusClipStatus, ticks));
    layout += &format!("  underruns {}\n", offset_of!(MediusClipStatus, underruns));
    layout += &format!("  overruns {}\n", offset_of!(MediusClipStatus, overruns));
    layout += &format!("  seq_gaps {}\n", offset_of!(MediusClipStatus, seq_gaps));
    layout += &format!("  xfers {}\n", offset_of!(MediusClipStatus, xfers));
    layout += &format!("  xfer_errs {}\n", offset_of!(MediusClipStatus, xfer_errs));
    layout += &format!("  gated {}\n", offset_of!(MediusClipStatus, gated));
    layout += &format!("  held_n {}\n", offset_of!(MediusClipStatus, held_n));
    layout += &format!("  held {}\n", offset_of!(MediusClipStatus, held));
    layout += &format!(
        "MediusClipSettings size {} align {}\n",
        size_of::<MediusClipSettings>(),
        align_of::<MediusClipSettings>()
    );
    layout += &format!(
        "  autolock_bits {}\n",
        offset_of!(MediusClipSettings, autolock_bits)
    );
    layout += &format!("  loop_ {}\n", offset_of!(MediusClipSettings, loop_));
    layout += &format!("  retain {}\n", offset_of!(MediusClipSettings, retain));
    layout += &format!(
        "  finalized {}\n",
        offset_of!(MediusClipSettings, finalized)
    );
    layout += &format!("  ride {}\n", offset_of!(MediusClipSettings, ride));
    layout += &format!("  triggers {}\n", offset_of!(MediusClipSettings, triggers));
    layout += &format!("  n {}\n", offset_of!(MediusClipSettings, n));
    layout += &format!(
        "  packet_triggers {}\n",
        offset_of!(MediusClipSettings, packet_triggers)
    );
    layout += &format!("  packet_n {}\n", offset_of!(MediusClipSettings, packet_n));
    layout += &format!(
        "MediusPortInfo size {} align {}\n",
        size_of::<MediusPortInfo>(),
        align_of::<MediusPortInfo>()
    );
    layout += &format!("  path {}\n", offset_of!(MediusPortInfo, path));
    layout += &format!("  vid {}\n", offset_of!(MediusPortInfo, vid));
    layout += &format!("  pid {}\n", offset_of!(MediusPortInfo, pid));
    layout += &format!("  serial {}\n", offset_of!(MediusPortInfo, serial));
    layout += &format!("  has_serial {}\n", offset_of!(MediusPortInfo, has_serial));
    layout += &format!(
        "MediusVersion size {} align {}\n",
        size_of::<MediusVersion>(),
        align_of::<MediusVersion>()
    );
    layout += &format!("  proto_ver {}\n", offset_of!(MediusVersion, proto_ver));
    layout += &format!("  fw_major {}\n", offset_of!(MediusVersion, fw_major));
    layout += &format!("  fw_minor {}\n", offset_of!(MediusVersion, fw_minor));
    layout += &format!("  fw_patch {}\n", offset_of!(MediusVersion, fw_patch));
    layout += &format!("  mac {}\n", offset_of!(MediusVersion, mac));
    layout += &format!("  name {}\n", offset_of!(MediusVersion, name));
    layout += &format!(
        "MediusDeviceInfo size {} align {}\n",
        size_of::<MediusDeviceInfo>(),
        align_of::<MediusDeviceInfo>()
    );
    layout += &format!("  vid {}\n", offset_of!(MediusDeviceInfo, vid));
    layout += &format!("  pid {}\n", offset_of!(MediusDeviceInfo, pid));
    layout += &format!(
        "  bcd_device {}\n",
        offset_of!(MediusDeviceInfo, bcd_device)
    );
    layout += &format!("  bcd_usb {}\n", offset_of!(MediusDeviceInfo, bcd_usb));
    layout += &format!(
        "  has_serial {}\n",
        offset_of!(MediusDeviceInfo, has_serial)
    );
    layout += &format!("  has_bos {}\n", offset_of!(MediusDeviceInfo, has_bos));
    layout += &format!("  kind {}\n", offset_of!(MediusDeviceInfo, kind));
    layout += &format!("  product {}\n", offset_of!(MediusDeviceInfo, product));
    layout += &format!(
        "MediusBoxInfo size {} align {}\n",
        size_of::<MediusBoxInfo>(),
        align_of::<MediusBoxInfo>()
    );
    layout += &format!("  port {}\n", offset_of!(MediusBoxInfo, port));
    layout += &format!("  version {}\n", offset_of!(MediusBoxInfo, version));
    layout += &format!("  device {}\n", offset_of!(MediusBoxInfo, device));
    layout += &format!("  has_device {}\n", offset_of!(MediusBoxInfo, has_device));
    layout += &format!(
        "MediusMotion size {} align {}\n",
        size_of::<MediusMotion>(),
        align_of::<MediusMotion>()
    );
    layout += &format!("  kind {}\n", offset_of!(MediusMotion, kind));
    layout += &format!("  dx {}\n", offset_of!(MediusMotion, dx));
    layout += &format!("  dy {}\n", offset_of!(MediusMotion, dy));
    layout += &format!("  wheel {}\n", offset_of!(MediusMotion, wheel));
    layout += &format!("  pan {}\n", offset_of!(MediusMotion, pan));
    layout += &format!(
        "MediusLockTarget size {} align {}\n",
        size_of::<MediusLockTarget>(),
        align_of::<MediusLockTarget>()
    );
    layout += &format!("  kind {}\n", offset_of!(MediusLockTarget, kind));
    layout += &format!("  usage {}\n", offset_of!(MediusLockTarget, usage));
    layout += &format!(
        "MediusTransferOutcome size {} align {}\n",
        size_of::<MediusTransferOutcome>(),
        align_of::<MediusTransferOutcome>()
    );
    layout += &format!("  status {}\n", offset_of!(MediusTransferOutcome, status));
    layout += &format!("  len {}\n", offset_of!(MediusTransferOutcome, len));
    layout += &format!("  data {}\n", offset_of!(MediusTransferOutcome, data));
    layout += &format!(
        "MediusRewriteRule size {} align {}\n",
        size_of::<MediusRewriteRule>(),
        align_of::<MediusRewriteRule>()
    );
    layout += &format!("  class_ {}\n", offset_of!(MediusRewriteRule, class));
    layout += &format!("  id {}\n", offset_of!(MediusRewriteRule, id));
    layout += &format!("  direction {}\n", offset_of!(MediusRewriteRule, direction));
    layout += &format!("  action {}\n", offset_of!(MediusRewriteRule, action));
    layout += &format!("  offset {}\n", offset_of!(MediusRewriteRule, offset));
    layout += &format!("  match_len {}\n", offset_of!(MediusRewriteRule, match_len));
    layout += &format!("  mask_len {}\n", offset_of!(MediusRewriteRule, mask_len));
    layout += &format!(
        "  payload_len {}\n",
        offset_of!(MediusRewriteRule, payload_len)
    );
    layout += &format!(
        "  match_bytes {}\n",
        offset_of!(MediusRewriteRule, match_bytes)
    );
    layout += &format!("  mask {}\n", offset_of!(MediusRewriteRule, mask));
    layout += &format!("  payload {}\n", offset_of!(MediusRewriteRule, payload));
    layout += &format!(
        "MediusRewriteEntry size {} align {}\n",
        size_of::<MediusRewriteEntry>(),
        align_of::<MediusRewriteEntry>()
    );
    layout += &format!("  class_ {}\n", offset_of!(MediusRewriteEntry, class));
    layout += &format!("  id {}\n", offset_of!(MediusRewriteEntry, id));
    layout += &format!(
        "  direction {}\n",
        offset_of!(MediusRewriteEntry, direction)
    );
    layout += &format!("  action {}\n", offset_of!(MediusRewriteEntry, action));
    layout += &format!(
        "  match_len {}\n",
        offset_of!(MediusRewriteEntry, match_len)
    );
    layout += &format!("  offset {}\n", offset_of!(MediusRewriteEntry, offset));
    layout += &format!(
        "  payload_len {}\n",
        offset_of!(MediusRewriteEntry, payload_len)
    );
    layout += &format!("  hits {}\n", offset_of!(MediusRewriteEntry, hits));
    layout += &format!(
        "MediusRewriteTable size {} align {}\n",
        size_of::<MediusRewriteTable>(),
        align_of::<MediusRewriteTable>()
    );
    layout += &format!(
        "  table_full {}\n",
        offset_of!(MediusRewriteTable, table_full)
    );
    layout += &format!(
        "  generation {}\n",
        offset_of!(MediusRewriteTable, generation)
    );
    layout += &format!("  n {}\n", offset_of!(MediusRewriteTable, n));
    layout += &format!("  entries {}\n", offset_of!(MediusRewriteTable, entries));
    layout += &format!(
        "MediusPatch size {} align {}\n",
        size_of::<MediusPatch>(),
        align_of::<MediusPatch>()
    );
    layout += &format!("  section {}\n", offset_of!(MediusPatch, section));
    layout += &format!("  cfg {}\n", offset_of!(MediusPatch, cfg));
    layout += &format!("  index {}\n", offset_of!(MediusPatch, index));
    layout += &format!("  offset {}\n", offset_of!(MediusPatch, offset));
    layout += &format!("  len {}\n", offset_of!(MediusPatch, len));
    layout += &format!("  bytes {}\n", offset_of!(MediusPatch, bytes));
    layout += &format!(
        "MediusPatchEntry size {} align {}\n",
        size_of::<MediusPatchEntry>(),
        align_of::<MediusPatchEntry>()
    );
    layout += &format!("  section {}\n", offset_of!(MediusPatchEntry, section));
    layout += &format!("  cfg {}\n", offset_of!(MediusPatchEntry, cfg));
    layout += &format!("  index {}\n", offset_of!(MediusPatchEntry, index));
    layout += &format!("  offset {}\n", offset_of!(MediusPatchEntry, offset));
    layout += &format!("  len {}\n", offset_of!(MediusPatchEntry, len));
    layout += &format!(
        "MediusPatchSet size {} align {}\n",
        size_of::<MediusPatchSet>(),
        align_of::<MediusPatchSet>()
    );
    layout += &format!("  applied {}\n", offset_of!(MediusPatchSet, applied));
    layout += &format!("  pending {}\n", offset_of!(MediusPatchSet, pending));
    layout += &format!("  refused {}\n", offset_of!(MediusPatchSet, refused));
    layout += &format!("  table_full {}\n", offset_of!(MediusPatchSet, table_full));
    layout += &format!("  n {}\n", offset_of!(MediusPatchSet, n));
    layout += &format!("  entries {}\n", offset_of!(MediusPatchSet, entries));
    layout += &format!(
        "MediusTransform size {} align {}\n",
        size_of::<MediusTransform>(),
        align_of::<MediusTransform>()
    );
    layout += &format!("  op {}\n", offset_of!(MediusTransform, op));
    layout += &format!("  source {}\n", offset_of!(MediusTransform, source));
    layout += &format!("  dest {}\n", offset_of!(MediusTransform, dest));
    layout += &format!(
        "MediusTransforms size {} align {}\n",
        size_of::<MediusTransforms>(),
        align_of::<MediusTransforms>()
    );
    layout += &format!(
        "  table_full {}\n",
        offset_of!(MediusTransforms, table_full)
    );
    layout += &format!("  n {}\n", offset_of!(MediusTransforms, n));
    layout += &format!("  entries {}\n", offset_of!(MediusTransforms, entries));
    layout += &format!(
        "MediusChipFirmware size {} align {}\n",
        size_of::<MediusChipFirmware>(),
        align_of::<MediusChipFirmware>()
    );
    layout += &format!("  major {}\n", offset_of!(MediusChipFirmware, major));
    layout += &format!("  minor {}\n", offset_of!(MediusChipFirmware, minor));
    layout += &format!("  patch {}\n", offset_of!(MediusChipFirmware, patch));
    layout += &format!("  slot {}\n", offset_of!(MediusChipFirmware, slot));
    layout += &format!("  state {}\n", offset_of!(MediusChipFirmware, state));
    layout += &format!(
        "MediusFirmwareInfo size {} align {}\n",
        size_of::<MediusFirmwareInfo>(),
        align_of::<MediusFirmwareInfo>()
    );
    layout += &format!("  device {}\n", offset_of!(MediusFirmwareInfo, device));
    layout += &format!(
        "  host_present {}\n",
        offset_of!(MediusFirmwareInfo, host_present)
    );
    layout += &format!("  host {}\n", offset_of!(MediusFirmwareInfo, host));
    layout += &format!(
        "  slot_size {}\n",
        offset_of!(MediusFirmwareInfo, slot_size)
    );
    layout += &format!(
        "  device_staged {}\n",
        offset_of!(MediusFirmwareInfo, device_staged)
    );
    layout += &format!(
        "  host_staged {}\n",
        offset_of!(MediusFirmwareInfo, host_staged)
    );
    layout += &format!(
        "MediusHealth size {} align {}\n",
        size_of::<MediusHealth>(),
        align_of::<MediusHealth>()
    );
    layout += &format!("  link_up {}\n", offset_of!(MediusHealth, link_up));
    layout += &format!(
        "  mouse_attached {}\n",
        offset_of!(MediusHealth, mouse_attached)
    );
    layout += &format!(
        "  clone_configured {}\n",
        offset_of!(MediusHealth, clone_configured)
    );
    layout += &format!(
        "  injection_active {}\n",
        offset_of!(MediusHealth, injection_active)
    );
    layout += &format!(
        "  rate_confident {}\n",
        offset_of!(MediusHealth, rate_confident)
    );
    layout += &format!("  lock_on {}\n", offset_of!(MediusHealth, lock_on));
    layout += &format!("  catch_on {}\n", offset_of!(MediusHealth, catch_on));
    layout += &format!(
        "  kbd_attached {}\n",
        offset_of!(MediusHealth, kbd_attached)
    );
    layout += &format!("  rewrite_on {}\n", offset_of!(MediusHealth, rewrite_on));
    layout += &format!("  patch_on {}\n", offset_of!(MediusHealth, patch_on));
    layout += &format!(
        "  transform_on {}\n",
        offset_of!(MediusHealth, transform_on)
    );
    layout += &format!(
        "MediusMouseCaps size {} align {}\n",
        size_of::<MediusMouseCaps>(),
        align_of::<MediusMouseCaps>()
    );
    layout += &format!("  n_buttons {}\n", offset_of!(MediusMouseCaps, n_buttons));
    layout += &format!("  has_x {}\n", offset_of!(MediusMouseCaps, has_x));
    layout += &format!("  has_y {}\n", offset_of!(MediusMouseCaps, has_y));
    layout += &format!("  has_wheel {}\n", offset_of!(MediusMouseCaps, has_wheel));
    layout += &format!("  pan {}\n", offset_of!(MediusMouseCaps, pan));
    layout += &format!(
        "  has_report_id {}\n",
        offset_of!(MediusMouseCaps, has_report_id)
    );
    layout += &format!("  n_hid {}\n", offset_of!(MediusMouseCaps, n_hid));
    layout += &format!(
        "MediusKbdCaps size {} align {}\n",
        size_of::<MediusKbdCaps>(),
        align_of::<MediusKbdCaps>()
    );
    layout += &format!("  n_keys {}\n", offset_of!(MediusKbdCaps, n_keys));
    layout += &format!("  nkro {}\n", offset_of!(MediusKbdCaps, nkro));
    layout += &format!(
        "  has_consumer {}\n",
        offset_of!(MediusKbdCaps, has_consumer)
    );
    layout += &format!("  has_system {}\n", offset_of!(MediusKbdCaps, has_system));
    layout += &format!(
        "  has_report_id {}\n",
        offset_of!(MediusKbdCaps, has_report_id)
    );
    layout += &format!(
        "MediusCaps size {} align {}\n",
        size_of::<MediusCaps>(),
        align_of::<MediusCaps>()
    );
    layout += &format!("  mouse {}\n", offset_of!(MediusCaps, mouse));
    layout += &format!("  keyboard {}\n", offset_of!(MediusCaps, keyboard));
    layout += &format!(
        "  mouse_change_driven {}\n",
        offset_of!(MediusCaps, mouse_change_driven)
    );
    layout += &format!(
        "  kbd_change_driven {}\n",
        offset_of!(MediusCaps, kbd_change_driven)
    );
    layout += &format!(
        "MediusRate size {} align {}\n",
        size_of::<MediusRate>(),
        align_of::<MediusRate>()
    );
    layout += &format!(
        "  native_period_us {}\n",
        offset_of!(MediusRate, native_period_us)
    );
    layout += &format!(
        "  poll_period_us {}\n",
        offset_of!(MediusRate, poll_period_us)
    );
    layout += &format!("  confident {}\n", offset_of!(MediusRate, confident));
    layout += &format!(
        "  change_driven {}\n",
        offset_of!(MediusRate, change_driven)
    );
    layout += &format!(
        "MediusStats size {} align {}\n",
        size_of::<MediusStats>(),
        align_of::<MediusStats>()
    );
    layout += &format!("  inject_emits {}\n", offset_of!(MediusStats, inject_emits));
    layout += &format!("  tx_drops {}\n", offset_of!(MediusStats, tx_drops));
    layout += &format!("  tx_merges {}\n", offset_of!(MediusStats, tx_merges));
    layout += &format!("  tx_maxdepth {}\n", offset_of!(MediusStats, tx_maxdepth));
    layout += &format!("  tx_wedges {}\n", offset_of!(MediusStats, tx_wedges));
    layout += &format!("  wakeups {}\n", offset_of!(MediusStats, wakeups));
    layout += &format!("  reset_count {}\n", offset_of!(MediusStats, reset_count));
    layout += &format!("  config_count {}\n", offset_of!(MediusStats, config_count));
    layout += &format!(
        "  link_rx_drops {}\n",
        offset_of!(MediusStats, link_rx_drops)
    );
    layout += &format!(
        "  host_rx_drops {}\n",
        offset_of!(MediusStats, host_rx_drops)
    );
    layout += &format!("  relay_drops {}\n", offset_of!(MediusStats, relay_drops));
    layout += &format!("  session {}\n", offset_of!(MediusStats, session));
    layout += &format!(
        "MediusLockEntry size {} align {}\n",
        size_of::<MediusLockEntry>(),
        align_of::<MediusLockEntry>()
    );
    layout += &format!("  target {}\n", offset_of!(MediusLockEntry, target));
    layout += &format!("  is_blanket {}\n", offset_of!(MediusLockEntry, is_blanket));
    layout += &format!("  direction {}\n", offset_of!(MediusLockEntry, direction));
    layout += &format!("  scale {}\n", offset_of!(MediusLockEntry, scale));
    layout += &format!(
        "MediusLocks size {} align {}\n",
        size_of::<MediusLocks>(),
        align_of::<MediusLocks>()
    );
    layout += &format!("  n {}\n", offset_of!(MediusLocks, n));
    layout += &format!("  entries {}\n", offset_of!(MediusLocks, entries));
    layout += &format!(
        "MediusClockEstimate size {} align {}\n",
        size_of::<MediusClockEstimate>(),
        align_of::<MediusClockEstimate>()
    );
    layout += &format!(
        "  offset_us {}\n",
        offset_of!(MediusClockEstimate, offset_us)
    );
    layout += &format!("  rate_ppb {}\n", offset_of!(MediusClockEstimate, rate_ppb));
    layout += &format!("  delay_us {}\n", offset_of!(MediusClockEstimate, delay_us));
    layout += &format!("  age_ms {}\n", offset_of!(MediusClockEstimate, age_ms));
    layout += &format!(
        "MediusCatchFilter size {} align {}\n",
        size_of::<MediusCatchFilter>(),
        align_of::<MediusCatchFilter>()
    );
    layout += &format!("  class_ {}\n", offset_of!(MediusCatchFilter, class));
    layout += &format!("  id {}\n", offset_of!(MediusCatchFilter, id));
    layout += &format!("  direction {}\n", offset_of!(MediusCatchFilter, direction));
    layout += &format!("  capture {}\n", offset_of!(MediusCatchFilter, capture));
    layout += &format!(
        "MediusCatchEntry size {} align {}\n",
        size_of::<MediusCatchEntry>(),
        align_of::<MediusCatchEntry>()
    );
    layout += &format!("  filter {}\n", offset_of!(MediusCatchEntry, filter));
    layout += &format!("  dropped {}\n", offset_of!(MediusCatchEntry, dropped));
    layout += &format!(
        "MediusCatchState size {} align {}\n",
        size_of::<MediusCatchState>(),
        align_of::<MediusCatchState>()
    );
    layout += &format!(
        "  table_full {}\n",
        offset_of!(MediusCatchState, table_full)
    );
    layout += &format!("  dropped {}\n", offset_of!(MediusCatchState, dropped));
    layout += &format!("  clock {}\n", offset_of!(MediusCatchState, clock));
    layout += &format!("  n {}\n", offset_of!(MediusCatchState, n));
    layout += &format!("  entries {}\n", offset_of!(MediusCatchState, entries));
    layout += &format!(
        "MediusImperfectStatus size {} align {}\n",
        size_of::<MediusImperfectStatus>(),
        align_of::<MediusImperfectStatus>()
    );
    layout += &format!("  allowed {}\n", offset_of!(MediusImperfectStatus, allowed));
    layout += &format!(
        "  over_capacity {}\n",
        offset_of!(MediusImperfectStatus, over_capacity)
    );
    layout += &format!(
        "  clone_imperfect {}\n",
        offset_of!(MediusImperfectStatus, clone_imperfect)
    );
    layout += &format!(
        "MediusEmitPaceStatus size {} align {}\n",
        size_of::<MediusEmitPaceStatus>(),
        align_of::<MediusEmitPaceStatus>()
    );
    layout += &format!("  mode {}\n", offset_of!(MediusEmitPaceStatus, mode));
    layout += &format!(
        "  fixed_hz {}\n",
        offset_of!(MediusEmitPaceStatus, fixed_hz)
    );
    layout += &format!(
        "  resolved_hz {}\n",
        offset_of!(MediusEmitPaceStatus, resolved_hz)
    );
    layout += &format!(
        "  force_hz {}\n",
        offset_of!(MediusEmitPaceStatus, force_hz)
    );
    layout += &format!(
        "  advertised_hz {}\n",
        offset_of!(MediusEmitPaceStatus, advertised_hz)
    );
    layout += &format!(
        "  force_active {}\n",
        offset_of!(MediusEmitPaceStatus, force_active)
    );
    layout += &format!(
        "MediusBearing size {} align {}\n",
        size_of::<MediusBearing>(),
        align_of::<MediusBearing>()
    );
    layout += &format!("  window_ms {}\n", offset_of!(MediusBearing, window_ms));
    layout += &format!("  mode {}\n", offset_of!(MediusBearing, mode));
    layout += &format!(
        "MediusRenderStatus size {} align {}\n",
        size_of::<MediusRenderStatus>(),
        align_of::<MediusRenderStatus>()
    );
    layout += &format!("  mode {}\n", offset_of!(MediusRenderStatus, mode));
    layout += &format!("  full {}\n", offset_of!(MediusRenderStatus, full));
    layout += &format!("  ready {}\n", offset_of!(MediusRenderStatus, ready));
    layout += &format!(
        "MediusSpreadStatus size {} align {}\n",
        size_of::<MediusSpreadStatus>(),
        align_of::<MediusSpreadStatus>()
    );
    layout += &format!("  percent {}\n", offset_of!(MediusSpreadStatus, percent));
    layout += &format!("  span_us {}\n", offset_of!(MediusSpreadStatus, span_us));
    layout += &format!(
        "MediusCountersSnapshot size {} align {}\n",
        size_of::<MediusCountersSnapshot>(),
        align_of::<MediusCountersSnapshot>()
    );
    layout += &format!(
        "  frames_tx {}\n",
        offset_of!(MediusCountersSnapshot, frames_tx)
    );
    layout += &format!(
        "  frames_rx {}\n",
        offset_of!(MediusCountersSnapshot, frames_rx)
    );
    layout += &format!(
        "  crc_drops {}\n",
        offset_of!(MediusCountersSnapshot, crc_drops)
    );
    layout += &format!(
        "  reconnects {}\n",
        offset_of!(MediusCountersSnapshot, reconnects)
    );
    layout += &format!(
        "  restarts {}\n",
        offset_of!(MediusCountersSnapshot, restarts)
    );
    layout += &format!(
        "MediusUpdateError size {} align {}\n",
        size_of::<MediusUpdateError>(),
        align_of::<MediusUpdateError>()
    );
    layout += &format!("  op {}\n", offset_of!(MediusUpdateError, op));
    layout += &format!("  status {}\n", offset_of!(MediusUpdateError, status));
    layout += &format!("  arg {}\n", offset_of!(MediusUpdateError, arg));
    layout += &format!(
        "MediusUsageEvent size {} align {}\n",
        size_of::<MediusUsageEvent>(),
        align_of::<MediusUsageEvent>()
    );
    layout += &format!("  class_ {}\n", offset_of!(MediusUsageEvent, class));
    layout += &format!("  direction {}\n", offset_of!(MediusUsageEvent, direction));
    layout += &format!("  n {}\n", offset_of!(MediusUsageEvent, n));
    layout += &format!("  usages {}\n", offset_of!(MediusUsageEvent, usages));
    layout += &format!(
        "MediusTrafficEvent size {} align {}\n",
        size_of::<MediusTrafficEvent>(),
        align_of::<MediusTrafficEvent>()
    );
    layout += &format!("  class_ {}\n", offset_of!(MediusTrafficEvent, class));
    layout += &format!("  id {}\n", offset_of!(MediusTrafficEvent, id));
    layout += &format!(
        "  direction {}\n",
        offset_of!(MediusTrafficEvent, direction)
    );
    layout += &format!("  flags {}\n", offset_of!(MediusTrafficEvent, flags));
    layout += &format!("  true_len {}\n", offset_of!(MediusTrafficEvent, true_len));
    layout += &format!("  len {}\n", offset_of!(MediusTrafficEvent, len));
    layout += &format!("  bytes {}\n", offset_of!(MediusTrafficEvent, bytes));
    layout += &format!(
        "MediusBusEvent size {} align {}\n",
        size_of::<MediusBusEvent>(),
        align_of::<MediusBusEvent>()
    );
    layout += &format!("  kind {}\n", offset_of!(MediusBusEvent, kind));
    layout += &format!(
        "  configuration {}\n",
        offset_of!(MediusBusEvent, configuration)
    );
    layout += &format!("  interface {}\n", offset_of!(MediusBusEvent, interface));
    layout += &format!("  alt {}\n", offset_of!(MediusBusEvent, alt));
    layout += &format!(
        "MediusMotionEvent size {} align {}\n",
        size_of::<MediusMotionEvent>(),
        align_of::<MediusMotionEvent>()
    );
    layout += &format!("  dx {}\n", offset_of!(MediusMotionEvent, dx));
    layout += &format!("  dy {}\n", offset_of!(MediusMotionEvent, dy));
    layout += &format!("  dz {}\n", offset_of!(MediusMotionEvent, dz));
    layout += &format!("  pan {}\n", offset_of!(MediusMotionEvent, pan));
    layout += &format!(
        "MediusCatchEventData size {} align {}\n",
        size_of::<MediusCatchEventData>(),
        align_of::<MediusCatchEventData>()
    );
    layout += &format!(
        "MediusCatchEvent size {} align {}\n",
        size_of::<MediusCatchEvent>(),
        align_of::<MediusCatchEvent>()
    );
    layout += &format!("  kind {}\n", offset_of!(MediusCatchEvent, kind));
    layout += &format!("  ts_us {}\n", offset_of!(MediusCatchEvent, ts_us));
    layout += &format!("  clock {}\n", offset_of!(MediusCatchEvent, clock));
    layout += &format!("  data {}\n", offset_of!(MediusCatchEvent, data));
    layout += &format!(
        "MediusInputEvent size {} align {}\n",
        size_of::<MediusInputEvent>(),
        align_of::<MediusInputEvent>()
    );
    layout += &format!("  kind {}\n", offset_of!(MediusInputEvent, kind));
    layout += &format!("  ts_us {}\n", offset_of!(MediusInputEvent, ts_us));
    layout += &format!("  clock {}\n", offset_of!(MediusInputEvent, clock));
    layout += &format!("  usage {}\n", offset_of!(MediusInputEvent, usage));
    layout += &format!("  dx {}\n", offset_of!(MediusInputEvent, dx));
    layout += &format!("  dy {}\n", offset_of!(MediusInputEvent, dy));
    layout += &format!("  dz {}\n", offset_of!(MediusInputEvent, dz));
    layout += &format!("  pan {}\n", offset_of!(MediusInputEvent, pan));
    layout += &format!(
        "MediusStamped size {} align {}\n",
        size_of::<MediusStamped>(),
        align_of::<MediusStamped>()
    );
    layout += &format!("  host_ns {}\n", offset_of!(MediusStamped, host_ns));
    layout += &format!("  box_us {}\n", offset_of!(MediusStamped, box_us));
    layout += &format!("  excess_ns {}\n", offset_of!(MediusStamped, excess_ns));
    layout += &format!(
        "MediusLogLine size {} align {}\n",
        size_of::<MediusLogLine>(),
        align_of::<MediusLogLine>()
    );
    layout += &format!("  level {}\n", offset_of!(MediusLogLine, level));
    layout += &format!("  text {}\n", offset_of!(MediusLogLine, text));
    // cargo says where the crate is at run time too, and a test binary copied from another checkout has the other
    // one's path built in
    let path = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("tests/medius-layout.txt");
    if std::env::var_os("ANYBOX_WRITE_LAYOUT").is_some() {
        std::fs::write(&path, &layout).unwrap();
    }
    let committed = std::fs::read_to_string(&path)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert_eq!(
        committed, layout,
        "medius-capi's layout changed, rerun with ANYBOX_WRITE_LAYOUT=1 and check the C# side"
    );
}
