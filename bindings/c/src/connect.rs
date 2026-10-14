use std::ffi::{CString, c_char};
use std::ptr;

use anybox::{Connect, Found, StableId};

use crate::device::AnyboxDevice;
use crate::snapshot::{ANYBOX_ID_MAC, ANYBOX_ID_NONE, ANYBOX_ID_USB_SERIAL, AnyboxSnapshot};
use crate::status::{
    AnyboxStatus, c_string, error, fail, guard, guard_free, invalid, read, status_of, text, write,
};
use crate::values::{
    ANYBOX_FAMILY_ANY, ANYBOX_FERRUM_API_NONE, AnyboxClickTiming, actions, click_timing,
    click_timing_struct, family_filter, ferrum_api_filter,
};

/// what `anybox_open`, `anybox_scan` and `anybox_candidates` look for. a struct of zeros is the default too, the
/// same as what `anybox_connect_options_default` fills in
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct AnyboxConnectOptions {
    /// only this port, whatever its USB id is. null lets anybox look on every port a box could be on
    pub port: *const c_char,
    /// only this family, so the port never gets anything meant for another box
    pub family: u32,
    /// only a Ferrum on this API, see `Connect::ferrum_api` in the Rust docs
    pub ferrum_api: u32,
    /// skips a box that can't do these, one bit per action (bit n for action n). `ANYBOX_ACTION_CONNECT` matches no
    /// box
    pub can: u64,
    /// only the box with this id, whatever port it's on
    pub id_kind: u32,
    pub id_mac: [u8; 6],
    pub id_serial: *const c_char,
    /// what a click does on a box with no click of its own
    pub click_timing: AnyboxClickTiming,
}

impl Default for AnyboxConnectOptions {
    fn default() -> Self {
        Self {
            port: ptr::null(),
            family: ANYBOX_FAMILY_ANY,
            ferrum_api: ANYBOX_FERRUM_API_NONE,
            can: 0,
            id_kind: ANYBOX_ID_NONE,
            id_mac: [0; 6],
            id_serial: ptr::null(),
            click_timing: click_timing_struct(anybox::ClickTiming::default()),
        }
    }
}

/// the box on a port, or the status and text of the error a search of just that port gives
type Outcome = Result<Found, (AnyboxStatus, String)>;

/// every port a scan looked at, in port order
pub struct AnyboxScanList(Vec<(CString, Outcome)>);

/// a list of strings, like the ports `anybox_candidates` gives
pub struct AnyboxStringList(Vec<CString>);

/// null options are the defaults
pub(crate) unsafe fn builder(
    options: *const AnyboxConnectOptions,
) -> Result<Connect, AnyboxStatus> {
    // SAFETY: the header says non-null options point at a whole `AnyboxConnectOptions`
    let Some(options) = (unsafe { options.as_ref() }) else {
        return Ok(Connect::auto());
    };
    let mut connect = if options.port.is_null() {
        Connect::auto()
    } else {
        Connect::port(unsafe { text(options.port, "options.port") }?)
    };
    if let Some(family) = family_filter(options.family)? {
        connect = connect.family(family);
    }
    if let Some(api) = ferrum_api_filter(options.ferrum_api)? {
        connect = connect.ferrum_api(api);
    }
    for action in actions(options.can)? {
        connect = connect.can(action);
    }
    connect = match options.id_kind {
        ANYBOX_ID_NONE => connect,
        ANYBOX_ID_MAC => connect.id(StableId::Mac(options.id_mac)),
        ANYBOX_ID_USB_SERIAL => connect.id(StableId::UsbSerial(
            unsafe { text(options.id_serial, "options.id_serial") }?.to_string(),
        )),
        other => return Err(invalid(format!("{other} isn't an id kind anybox knows"))),
    };
    Ok(connect.click_timing(click_timing(&options.click_timing)?))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_connect_options_default(
    out: *mut AnyboxConnectOptions,
) -> AnyboxStatus {
    guard(|| {
        write(out, "out")?.set(AnyboxConnectOptions::default());
        Ok(())
    })
}

/// connects to the first box found, in port order
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_connect(out: *mut *mut AnyboxDevice) -> AnyboxStatus {
    unsafe { anybox_open(ptr::null(), out) }
}

/// opens the first box that fits the options, null options are the defaults. free it with `anybox_device_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_open(
    options: *const AnyboxConnectOptions,
    out: *mut *mut AnyboxDevice,
) -> AnyboxStatus {
    guard(|| {
        let connect = unsafe { builder(options) }?;
        let out = write(out, "out")?;
        let device = connect.open().map_err(error)?;
        out.set(AnyboxDevice::boxed(device));
        Ok(())
    })
}

/// every port a search looks at and what's on it, in port order, so a picker can show the boxes you can use and
/// why you can't use the others. it opens, describes and closes every box, so it's slow (a YSC search takes about
/// 2 s a port), `anybox_candidates` is the cheap one to call often. free it with `anybox_scan_list_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_scan(
    options: *const AnyboxConnectOptions,
    out: *mut *mut AnyboxScanList,
) -> AnyboxStatus {
    guard(|| {
        let connect = unsafe { builder(options) }?;
        let out = write(out, "out")?;
        let scanned = connect
            .scan()
            .into_iter()
            .map(|scanned| {
                let result = scanned
                    .result
                    .map_err(|error| (status_of(&error), error.to_string()));
                (c_string(scanned.port), result)
            })
            .collect();
        out.set(Box::into_raw(Box::new(AnyboxScanList(scanned))));
        Ok(())
    })
}

/// how many ports the scan looked at, 0 for a null list
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_scan_list_count(list: *const AnyboxScanList) -> usize {
    // SAFETY: the header says a non-null list came from `anybox_scan` and wasn't freed
    unsafe { list.as_ref() }.map_or(0, |list| list.0.len())
}

/// the port of entry `index`, null past the end. it lives as long as the list
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_scan_list_port(
    list: *const AnyboxScanList,
    index: usize,
) -> *const c_char {
    unsafe { list.as_ref() }
        .and_then(|list| list.0.get(index))
        .map_or(ptr::null(), |(port, _)| port.as_ptr())
}

/// the box on entry `index` as a snapshot you free with `anybox_snapshot_free`, or the error a search of just that
/// port gives, with its text in `anybox_last_error_message`. mostly that's busy (the box you already have open
/// shows up as busy too, so leave its port out), not allowed, too old, `ANYBOX_STATUS_NO_BOX_CAN` for a box that
/// can't do what `can` asked, or `ANYBOX_STATUS_NOT_FOUND` when nothing answered
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_scan_list_result(
    list: *const AnyboxScanList,
    index: usize,
    out: *mut *mut AnyboxSnapshot,
) -> AnyboxStatus {
    guard(|| {
        let list = unsafe { read(list, "list") }?;
        let out = write(out, "out")?;
        let (_, result) = list.0.get(index).ok_or_else(|| {
            invalid(format!(
                "there's no entry {index}, the list has {}",
                list.0.len()
            ))
        })?;
        match result {
            Ok(found) => {
                let snapshot = AnyboxSnapshot::new(&found.info, &found.capabilities);
                out.set(Box::into_raw(Box::new(snapshot)));
                Ok(())
            }
            Err((status, message)) => Err(fail(*status, message.clone())),
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_scan_list_free(list: *mut AnyboxScanList) {
    guard_free(|| {
        if !list.is_null() {
            // SAFETY: it came from `Box::into_raw` in this library and the header says it's freed once
            drop(unsafe { Box::from_raw(list) });
        }
    });
}

/// the ports a search would look at, in port order, without opening any of them or sending them a byte. it can't
/// tell whether a box is on one, only where one could be, so it's cheap enough to call every second and run
/// `anybox_scan` when the list changes. null options are the defaults. free it with `anybox_string_list_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_candidates(
    options: *const AnyboxConnectOptions,
    out: *mut *mut AnyboxStringList,
) -> AnyboxStatus {
    guard(|| {
        let connect = unsafe { builder(options) }?;
        let out = write(out, "out")?;
        let ports = connect.candidates().into_iter().map(c_string).collect();
        out.set(Box::into_raw(Box::new(AnyboxStringList(ports))));
        Ok(())
    })
}

/// 0 for a null list
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_string_list_count(list: *const AnyboxStringList) -> usize {
    // SAFETY: the header says a non-null list came from this library and wasn't freed
    unsafe { list.as_ref() }.map_or(0, |list| list.0.len())
}

/// string `index`, null past the end. it lives as long as the list
#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_string_list_get(
    list: *const AnyboxStringList,
    index: usize,
) -> *const c_char {
    unsafe { list.as_ref() }
        .and_then(|list| list.0.get(index))
        .map_or(ptr::null(), |text| text.as_ptr())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn anybox_string_list_free(list: *mut AnyboxStringList) {
    guard_free(|| {
        if !list.is_null() {
            // SAFETY: it came from `Box::into_raw` in this library and the header says it's freed once
            drop(unsafe { Box::from_raw(list) });
        }
    });
}
