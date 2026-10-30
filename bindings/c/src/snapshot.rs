use std::ffi::{CString, c_char};
use std::ptr;

use anybox::{Capabilities, Capability, Firmware, Info, Quirk, StableId};

use crate::status::{AnyboxStatus, c_string, guard, guard_free, invalid, read, write};
use crate::values::{
    ACTIONS, AnyboxClickTiming, axis_number, button_number, click_timing_struct, execution_number,
    family_number, ferrum_api_number, micros,
};

pub const ANYBOX_FIRMWARE_NONE: u32 = 0;
pub const ANYBOX_FIRMWARE_VERSION: u32 = 1;
pub const ANYBOX_FIRMWARE_BUILD: u32 = 2;
/// the protocol number a box speaks, which only shows up in the error of a Medius that's too old or too new, never
/// in `AnyboxInfo`
pub const ANYBOX_FIRMWARE_PROTOCOL: u32 = 3;
pub const ANYBOX_FIRMWARE_TEXT: u32 = 4;

pub const ANYBOX_ID_NONE: u32 = 0;
pub const ANYBOX_ID_MAC: u32 = 1;
pub const ANYBOX_ID_USB_SERIAL: u32 = 2;

/// `min_us` and `max_us`
pub const ANYBOX_QUIRK_FORCED_UP_ON_RELEASE: u32 = 0;
pub const ANYBOX_QUIRK_PHYSICAL_OVERRIDE: u32 = 1;
pub const ANYBOX_QUIRK_WHOLE_AXIS_ONLY: u32 = 2;
/// `count`
pub const ANYBOX_QUIRK_NOTCHES_PER_COMMAND: u32 = 3;
pub const ANYBOX_QUIRK_FIXED_CLICK_TIMING: u32 = 4;
/// `max_us`
pub const ANYBOX_QUIRK_MAX_HOLD: u32 = 5;
/// `min_us` and `max_us`
pub const ANYBOX_QUIRK_BOX_CLICK_TIMING: u32 = 6;
/// `click_timing`
pub const ANYBOX_QUIRK_HOST_CLICK_TIMING: u32 = 7;
pub const ANYBOX_QUIRK_KEEPS_HELD: u32 = 8;
/// `count`
pub const ANYBOX_QUIRK_MAX_KEYS_HELD: u32 = 9;
pub const ANYBOX_QUIRK_KEEPS_LOCKED: u32 = 10;
pub const ANYBOX_QUIRK_TYPED_TEXT_IGNORES_CAPS_LOCK: u32 = 11;
pub const ANYBOX_QUIRK_TYPING_RELEASES_HELD_KEYS: u32 = 12;

/// who the box is and what it can do, read from one connection, see `anybox_device_snapshot`
pub struct AnyboxSnapshot {
    info: AnyboxInfo,
    /// by action number, with the C copy of the reason next to each
    pub(crate) capabilities: Vec<(u32, Capability, Option<CString>)>,
    buttons: Vec<u32>,
    axes: Vec<u32>,
    // the strings `info` points into
    _strings: Vec<CString>,
}

/// who the box is. every string lives as long as the snapshot it came from
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AnyboxInfo {
    /// the port it was opened on, so you can open the same box again with `port` in the connect options
    pub port: *const c_char,
    pub family: u32,
    /// null when the box doesn't say
    pub model: *const c_char,
    pub firmware_kind: u32,
    pub firmware_major: u16,
    pub firmware_minor: u16,
    pub firmware_patch: u16,
    pub firmware_protocol: u8,
    pub firmware_build: u32,
    /// null unless `firmware_kind` is `ANYBOX_FIRMWARE_TEXT`
    pub firmware_text: *const c_char,
    pub id_kind: u32,
    pub id_mac: [u8; 6],
    /// null unless `id_kind` is `ANYBOX_ID_USB_SERIAL`
    pub id_serial: *const c_char,
    /// which of Ferrum's two APIs this is, `ANYBOX_FERRUM_API_NONE` on other boxes
    pub ferrum_api: u32,
    /// the USB ids of the device plugged into the box, see `Info::peripheral` in the Rust docs for when there's
    /// none
    pub has_peripheral: bool,
    pub peripheral_vid: u16,
    pub peripheral_pid: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AnyboxCapability {
    pub execution: u32,
    /// why the box can't, null unless `execution` is `ANYBOX_EXECUTION_UNSUPPORTED`
    pub reason: *const c_char,
    pub quirk_count: u32,
}

/// one way the box behaves differently. which fields mean something depends on `kind`, the rest are 0
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnyboxQuirk {
    pub kind: u32,
    pub min_us: u64,
    pub max_us: u64,
    pub count: u32,
    pub click_timing: AnyboxClickTiming,
}

impl AnyboxSnapshot {
    pub(crate) fn new(info: &Info, capabilities: &Capabilities) -> Self {
        let mut strings = Vec::new();
        let mut keep = |text: &str| {
            let text = c_string(text);
            let pointer = text.as_ptr();
            strings.push(text);
            pointer
        };
        let port = keep(&info.port);
        let model = info.model.as_deref().map_or(ptr::null(), &mut keep);
        let firmware = firmware_fields(info.firmware.as_ref(), &mut keep);
        let (id_kind, id_mac, id_serial) = id_fields(info.id.as_ref(), &mut keep);
        let info = AnyboxInfo {
            port,
            family: family_number(info.family),
            model,
            firmware_kind: firmware.kind,
            firmware_major: firmware.major,
            firmware_minor: firmware.minor,
            firmware_patch: firmware.patch,
            firmware_protocol: firmware.protocol,
            firmware_build: firmware.build,
            firmware_text: firmware.text,
            id_kind,
            id_mac,
            id_serial,
            ferrum_api: ferrum_api_number(info.ferrum_api),
            has_peripheral: info.peripheral.is_some(),
            peripheral_vid: info.peripheral.map_or(0, |ids| ids.vid),
            peripheral_pid: info.peripheral.map_or(0, |ids| ids.pid),
        };
        Self {
            info,
            capabilities: ACTIONS
                .iter()
                .map(|(number, action)| {
                    let capability = capabilities.get(*action).clone();
                    let reason =
                        (!capability.is_supported()).then(|| match &capability.execution {
                            anybox::Execution::Unsupported { reason } => {
                                c_string(reason.as_bytes())
                            }
                            _ => CString::default(),
                        });
                    (*number, capability, reason)
                })
                .collect(),
            buttons: capabilities
                .buttons()
                .iter()
                .map(|b| button_number(*b))
                .collect(),
            axes: capabilities
                .axes()
                .iter()
                .map(|a| axis_number(*a))
                .collect(),
            _strings: strings,
        }
    }

    fn capability(&self, value: u32) -> Result<(&Capability, &Option<CString>), AnyboxStatus> {
        self.capabilities
            .iter()
            .find(|(number, ..)| *number == value)
            .map(|(_, capability, reason)| (capability, reason))
            .ok_or_else(|| invalid(format!("{value} isn't an action anybox knows")))
    }
}

/// a firmware flattened the way `AnyboxInfo` and `AnyboxErrorDetail` carry it
pub(crate) struct FirmwareFields {
    pub(crate) kind: u32,
    pub(crate) major: u16,
    pub(crate) minor: u16,
    pub(crate) patch: u16,
    pub(crate) protocol: u8,
    pub(crate) build: u32,
    pub(crate) text: *const c_char,
}

/// `keep` holds on to a string and gives back its pointer
pub(crate) fn firmware_fields(
    firmware: Option<&Firmware>,
    keep: &mut impl FnMut(&str) -> *const c_char,
) -> FirmwareFields {
    let plain = |kind| FirmwareFields {
        kind,
        major: 0,
        minor: 0,
        patch: 0,
        protocol: 0,
        build: 0,
        text: ptr::null(),
    };
    match firmware {
        None => plain(ANYBOX_FIRMWARE_NONE),
        Some(Firmware::Version {
            major,
            minor,
            patch,
        }) => FirmwareFields {
            major: *major,
            minor: *minor,
            patch: *patch,
            ..plain(ANYBOX_FIRMWARE_VERSION)
        },
        Some(Firmware::Build(build)) => FirmwareFields {
            build: *build,
            ..plain(ANYBOX_FIRMWARE_BUILD)
        },
        Some(Firmware::Protocol(protocol)) => FirmwareFields {
            protocol: *protocol,
            ..plain(ANYBOX_FIRMWARE_PROTOCOL)
        },
        Some(Firmware::Text(text)) => FirmwareFields {
            text: keep(text),
            ..plain(ANYBOX_FIRMWARE_TEXT)
        },
        Some(other) => panic!("the C ABI has no number for {other:?} yet"),
    }
}

/// an id flattened into its kind, MAC and serial
pub(crate) fn id_fields(
    id: Option<&StableId>,
    keep: &mut impl FnMut(&str) -> *const c_char,
) -> (u32, [u8; 6], *const c_char) {
    match id {
        None => (ANYBOX_ID_NONE, [0; 6], ptr::null()),
        Some(StableId::Mac(mac)) => (ANYBOX_ID_MAC, *mac, ptr::null()),
        Some(StableId::UsbSerial(serial)) => (ANYBOX_ID_USB_SERIAL, [0; 6], keep(serial)),
        Some(other) => panic!("the C ABI has no number for {other:?} yet"),
    }
}

pub(crate) fn quirk_struct(quirk: &Quirk) -> AnyboxQuirk {
    let plain = |kind| AnyboxQuirk {
        kind,
        ..AnyboxQuirk::default()
    };
    match quirk {
        Quirk::ForcedUpOnRelease { min, max } => AnyboxQuirk {
            min_us: micros(*min),
            max_us: micros(*max),
            ..plain(ANYBOX_QUIRK_FORCED_UP_ON_RELEASE)
        },
        Quirk::PhysicalOverride => plain(ANYBOX_QUIRK_PHYSICAL_OVERRIDE),
        Quirk::WholeAxisOnly => plain(ANYBOX_QUIRK_WHOLE_AXIS_ONLY),
        Quirk::NotchesPerCommand(notches) => AnyboxQuirk {
            count: u32::from(*notches),
            ..plain(ANYBOX_QUIRK_NOTCHES_PER_COMMAND)
        },
        Quirk::FixedClickTiming => plain(ANYBOX_QUIRK_FIXED_CLICK_TIMING),
        Quirk::MaxHold(hold) => AnyboxQuirk {
            max_us: micros(*hold),
            ..plain(ANYBOX_QUIRK_MAX_HOLD)
        },
        Quirk::BoxClickTiming { min, max } => AnyboxQuirk {
            min_us: micros(*min),
            max_us: micros(*max),
            ..plain(ANYBOX_QUIRK_BOX_CLICK_TIMING)
        },
        Quirk::HostClickTiming(timing) => AnyboxQuirk {
            click_timing: click_timing_struct(*timing),
            ..plain(ANYBOX_QUIRK_HOST_CLICK_TIMING)
        },
        Quirk::KeepsHeld => plain(ANYBOX_QUIRK_KEEPS_HELD),
        Quirk::MaxKeysHeld(keys) => AnyboxQuirk {
            count: u32::from(*keys),
            ..plain(ANYBOX_QUIRK_MAX_KEYS_HELD)
        },
        Quirk::KeepsLocked => plain(ANYBOX_QUIRK_KEEPS_LOCKED),
        Quirk::TypedTextIgnoresCapsLock => plain(ANYBOX_QUIRK_TYPED_TEXT_IGNORES_CAPS_LOCK),
        Quirk::TypingReleasesHeldKeys => plain(ANYBOX_QUIRK_TYPING_RELEASES_HELD_KEYS),
        other => panic!("the C ABI has no number for {other:?} yet"),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_snapshot_info(
    snapshot: *const AnyboxSnapshot,
    out: *mut AnyboxInfo,
) -> AnyboxStatus {
    guard(|| {
        let snapshot = unsafe { read(snapshot, "snapshot") }?;
        write(out, "out")?.set(snapshot.info);
        Ok(())
    })
}

/// what the box can do for an `ANYBOX_ACTION_*`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_snapshot_capability(
    snapshot: *const AnyboxSnapshot,
    action: u32,
    out: *mut AnyboxCapability,
) -> AnyboxStatus {
    guard(|| {
        let snapshot = unsafe { read(snapshot, "snapshot") }?;
        let out = write(out, "out")?;
        let (capability, reason) = snapshot.capability(action)?;
        out.set(AnyboxCapability {
            execution: execution_number(&capability.execution),
            reason: reason
                .as_ref()
                .map_or(ptr::null(), |reason| reason.as_ptr()),
            quirk_count: capability.quirks.len() as u32,
        });
        Ok(())
    })
}

/// quirk number `index` of an action, from 0 up to but not including its `quirk_count`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_snapshot_quirk(
    snapshot: *const AnyboxSnapshot,
    action: u32,
    index: u32,
    out: *mut AnyboxQuirk,
) -> AnyboxStatus {
    guard(|| {
        let snapshot = unsafe { read(snapshot, "snapshot") }?;
        let out = write(out, "out")?;
        let (capability, _) = snapshot.capability(action)?;
        let quirk = capability.quirks.get(index as usize).ok_or_else(|| {
            invalid(format!(
                "there's no quirk {index}, that action has {}",
                capability.quirks.len()
            ))
        })?;
        out.set(quirk_struct(quirk));
        Ok(())
    })
}

/// the buttons this box has, as `ANYBOX_BUTTON_*`. `len` always gets how many there are and `out` the first
/// `cap` of them, so pass a null `out` and a `cap` of 0 to ask for the count first
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_snapshot_buttons(
    snapshot: *const AnyboxSnapshot,
    out: *mut u32,
    cap: usize,
    len: *mut usize,
) -> AnyboxStatus {
    guard(|| {
        let snapshot = unsafe { read(snapshot, "snapshot") }?;
        unsafe { copy_out(&snapshot.buttons, out, cap, len) }
    })
}

/// the axes you can lock or scale on this box, as `ANYBOX_AXIS_*`, filled the same way as the buttons
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_snapshot_axes(
    snapshot: *const AnyboxSnapshot,
    out: *mut u32,
    cap: usize,
    len: *mut usize,
) -> AnyboxStatus {
    guard(|| {
        let snapshot = unsafe { read(snapshot, "snapshot") }?;
        unsafe { copy_out(&snapshot.axes, out, cap, len) }
    })
}

pub(crate) unsafe fn copy_out<T: Copy>(
    items: &[T],
    out: *mut T,
    cap: usize,
    len: *mut usize,
) -> Result<(), AnyboxStatus> {
    let len = write(len, "len")?;
    if out.is_null() && cap > 0 {
        return Err(invalid("out is null but cap isn't 0"));
    }
    let count = items.len().min(cap);
    if count > 0 {
        // SAFETY: the header says `out` has room for `cap` values, and `count` is no more than that
        unsafe { ptr::copy_nonoverlapping(items.as_ptr(), out, count) };
    }
    len.set(items.len());
    Ok(())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_snapshot_free(snapshot: *mut AnyboxSnapshot) {
    guard_free(|| {
        if !snapshot.is_null() {
            // SAFETY: it came from `Box::into_raw` in this library and the header says it's freed once
            drop(unsafe { Box::from_raw(snapshot) });
        }
    });
}
