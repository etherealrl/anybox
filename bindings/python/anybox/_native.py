"""the ctypes side of the C ABI, `bindings/c/include/anybox.h` is what this has to match"""

from __future__ import annotations

import ctypes
import os
import sys
from ctypes import (
    POINTER,
    c_bool,
    c_char_p,
    c_int16,
    c_int32,
    c_int64,
    c_size_t,
    c_uint8,
    c_uint16,
    c_uint32,
    c_uint64,
    c_void_p,
)
from ctypes.util import find_library
from pathlib import Path

# the header's ANYBOX_ABI_VERSION, the tests hold it to the header and import refuses any other library
ABI_VERSION = 4

_LIBRARY = {"win32": "anybox.dll", "darwin": "libanybox.dylib"}.get(sys.platform, "libanybox.so")


class ClickTiming(ctypes.Structure):
    _fields_ = [("kind", c_uint32), ("fixed_us", c_uint64)]


class ConnectOptions(ctypes.Structure):
    _fields_ = [
        ("port", c_char_p),
        ("family", c_uint32),
        ("ferrum_api", c_uint32),
        ("can", POINTER(c_uint32)),
        ("can_len", c_size_t),
        ("id_kind", c_uint32),
        ("id_mac", c_uint8 * 6),
        ("id_serial", c_char_p),
        ("click_timing", ClickTiming),
    ]


class MakcuInterpolation(ctypes.Structure):
    _fields_ = [("automatic", c_bool), ("percent", c_uint8)]


class YscInterpolation(ctypes.Structure):
    _fields_ = [
        ("enabled", c_bool),
        ("profile", c_uint32),
        ("has_window", c_bool),
        ("window_ms", c_uint32),
        ("has_max_window", c_bool),
        ("max_window_ms", c_uint32),
        ("has_measured_interval", c_bool),
        ("measured_interval_us", c_uint64),
    ]


class YscInterpolationSettings(ctypes.Structure):
    _fields_ = [("enabled", c_bool), ("profile", c_uint32), ("window_ms", c_uint32), ("max_window_ms", c_uint32)]


class Click(ctypes.Structure):
    _fields_ = [("hold_us", c_uint64), ("has_hold", c_bool), ("native_only", c_bool)]


class Event(ctypes.Structure):
    _fields_ = [
        ("kind", c_uint32),
        ("button", c_uint32),
        ("key", c_uint8),
        ("pressed", c_bool),
        ("media", c_uint16),
        ("dx", ctypes.c_int32),
        ("dy", ctypes.c_int32),
        ("wheel", ctypes.c_int32),
        ("pan", ctypes.c_int32),
        ("source", c_uint32),
        ("action", c_uint32),
        ("count", c_uint64),
        ("since_open_ns", c_uint64),
        ("received_since_open_ns", c_uint64),
        ("received_qpc", c_int64),
        ("has_received_qpc", c_bool),
        ("has_box_time", c_bool),
        ("box_clock", c_uint32),
        ("box_time_us", c_uint32),
        ("arrived_ns", c_uint64),
        ("received_ns", c_uint64),
    ]


class Info(ctypes.Structure):
    _fields_ = [
        ("port", c_char_p),
        ("family", c_uint32),
        ("model", c_char_p),
        ("firmware_kind", c_uint32),
        ("firmware_major", c_uint16),
        ("firmware_minor", c_uint16),
        ("firmware_patch", c_uint16),
        ("firmware_protocol", c_uint8),
        ("firmware_build", c_uint32),
        ("firmware_text", c_char_p),
        ("id_kind", c_uint32),
        ("id_mac", c_uint8 * 6),
        ("id_serial", c_char_p),
        ("ferrum_api", c_uint32),
        ("has_peripheral", c_bool),
        ("peripheral_vid", c_uint16),
        ("peripheral_pid", c_uint16),
    ]


class Capability(ctypes.Structure):
    _fields_ = [("execution", c_uint32), ("reason", c_char_p), ("quirk_count", c_uint32)]


class Quirk(ctypes.Structure):
    _fields_ = [
        ("kind", c_uint32),
        ("min_us", c_uint64),
        ("max_us", c_uint64),
        ("count", c_uint32),
        ("click_timing", ClickTiming),
    ]


class ErrorDetail(ctypes.Structure):
    _fields_ = [
        ("kind", c_uint32),
        ("has_action", c_bool),
        ("action", c_uint32),
        ("family", c_uint32),
        ("expected_family", c_uint32),
        ("found_family", c_uint32),
        ("port", c_char_p),
        ("text", c_char_p),
        ("expected_id_kind", c_uint32),
        ("expected_id_mac", c_uint8 * 6),
        ("expected_id_serial", c_char_p),
        ("found_id_kind", c_uint32),
        ("found_id_mac", c_uint8 * 6),
        ("found_id_serial", c_char_p),
        ("found_firmware_kind", c_uint32),
        ("found_firmware_major", c_uint16),
        ("found_firmware_minor", c_uint16),
        ("found_firmware_patch", c_uint16),
        ("found_firmware_protocol", c_uint8),
        ("found_firmware_build", c_uint32),
        ("found_firmware_text", c_char_p),
        ("needed_firmware_kind", c_uint32),
        ("needed_firmware_major", c_uint16),
        ("needed_firmware_minor", c_uint16),
        ("needed_firmware_patch", c_uint16),
        ("needed_firmware_protocol", c_uint8),
        ("needed_firmware_build", c_uint32),
        ("needed_firmware_text", c_char_p),
        ("expected_ferrum_api", c_uint32),
        ("found_ferrum_api", c_uint32),
        ("io_kind", c_uint32),
        ("has_os_error", c_bool),
        ("os_error", ctypes.c_int32),
    ]


STRUCTS = {
    "AnyboxClickTiming": ClickTiming,
    "AnyboxClick": Click,
    "AnyboxConnectOptions": ConnectOptions,
    "AnyboxInfo": Info,
    "AnyboxCapability": Capability,
    "AnyboxQuirk": Quirk,
    "AnyboxEvent": Event,
    "AnyboxErrorDetail": ErrorDetail,
    "AnyboxMakcuInterpolation": MakcuInterpolation,
    "AnyboxYscInterpolation": YscInterpolation,
    "AnyboxYscInterpolationSettings": YscInterpolationSettings,
}

_status = c_uint32
_handle = c_void_p
_out = POINTER(c_void_p)
_device = [_handle]

_FUNCTIONS = {
    "anybox_abi_version": (c_uint32, []),
    "anybox_version": (c_char_p, []),
    "anybox_has_mock": (c_bool, []),
    "anybox_monotonic_ns": (c_uint64, []),
    "anybox_last_error_message": (c_char_p, []),
    "anybox_last_error_detail": (_status, [POINTER(ErrorDetail)]),
    "anybox_last_error_tried": (_status, [_out]),
    "anybox_last_error_incapable": (_status, [_out]),
    "anybox_open": (_status, [POINTER(ConnectOptions), _out]),
    "anybox_scan": (_status, [POINTER(ConnectOptions), _out]),
    "anybox_scan_list_count": (c_size_t, [_handle]),
    "anybox_scan_list_port": (c_char_p, [_handle, c_size_t]),
    "anybox_scan_list_result": (_status, [_handle, c_size_t, _out]),
    "anybox_scan_list_error_detail": (_status, [_handle, c_size_t, POINTER(ErrorDetail)]),
    "anybox_scan_list_missing": (_status, [_handle, c_size_t, POINTER(c_uint32), c_size_t, POINTER(c_size_t)]),
    "anybox_scan_list_free": (None, [_handle]),
    "anybox_candidates": (_status, [POINTER(ConnectOptions), _out]),
    "anybox_string_list_count": (c_size_t, [_handle]),
    "anybox_string_list_get": (c_char_p, [_handle, c_size_t]),
    "anybox_string_list_free": (None, [_handle]),
    "anybox_device_snapshot": (_status, [_handle, _out]),
    # made by bindings/gen from its call table (signatures), change the table and run cargo run -p anybox-gen
    "anybox_device_move_by": (_status, [_handle, c_int16, c_int16]),
    "anybox_device_move_now": (_status, [_handle, c_int16, c_int16]),
    "anybox_device_press": (_status, [_handle, c_uint32]),
    "anybox_device_release": (_status, [_handle, c_uint32]),
    "anybox_device_click_with": (_status, [_handle, c_uint32, POINTER(Click)]),
    "anybox_device_press_key": (_status, [_handle, c_uint8]),
    "anybox_device_release_key": (_status, [_handle, c_uint8]),
    "anybox_device_tap_key_with": (_status, [_handle, c_uint8, POINTER(Click)]),
    "anybox_device_type_text": (_status, [_handle, c_char_p]),
    "anybox_device_press_keys": (_status, [_handle, POINTER(c_uint8), c_size_t]),
    "anybox_device_release_keys": (_status, [_handle, POINTER(c_uint8), c_size_t]),
    "anybox_device_tap_keys": (_status, [_handle, POINTER(c_uint8), c_size_t]),
    "anybox_device_press_media": (_status, [_handle, c_uint16]),
    "anybox_device_release_media": (_status, [_handle, c_uint16]),
    "anybox_device_tap_media_with": (_status, [_handle, c_uint16, POINTER(Click)]),
    "anybox_device_lock_button": (_status, [_handle, c_uint32]),
    "anybox_device_unlock_button": (_status, [_handle, c_uint32]),
    "anybox_device_force_release_key": (_status, [_handle, c_uint8]),
    "anybox_device_restore_key": (_status, [_handle, c_uint8]),
    "anybox_device_lock_key": (_status, [_handle, c_uint8]),
    "anybox_device_unlock_key": (_status, [_handle, c_uint8]),
    "anybox_device_lock_axis": (_status, [_handle, c_uint32, c_uint32]),
    "anybox_device_unlock_axis": (_status, [_handle, c_uint32, c_uint32]),
    "anybox_device_scale_axis": (_status, [_handle, c_uint32, c_uint32, c_int16]),
    "anybox_device_wheel": (_status, [_handle, c_int16]),
    "anybox_device_pan": (_status, [_handle, c_int16]),
    "anybox_device_discard_pending_moves": (_status, [_handle]),
    # the end of what bindings/gen makes (signatures)
    "anybox_device_events": (_status, [_handle, c_uint32, _out]),
    "anybox_device_medius": (_status, [_handle, _out]),
    "anybox_device_close": (_status, [_handle]),
    "anybox_device_free": (None, [_handle]),
    "anybox_events_recv": (_status, [_handle, c_int64, POINTER(Event)]),
    "anybox_events_close": (_status, [_handle]),
    "anybox_events_free": (None, [_handle]),
    "anybox_snapshot_info": (_status, [_handle, POINTER(Info)]),
    "anybox_snapshot_capability": (_status, [_handle, c_uint32, POINTER(Capability)]),
    "anybox_snapshot_quirk": (_status, [_handle, c_uint32, c_uint32, POINTER(Quirk)]),
    "anybox_snapshot_buttons": (_status, [_handle, POINTER(c_uint32), c_size_t, POINTER(c_size_t)]),
    "anybox_snapshot_axes": (_status, [_handle, POINTER(c_uint32), c_size_t, POINTER(c_size_t)]),
    "anybox_snapshot_free": (None, [_handle]),
    # made by bindings/gen from its call table (makcu_signatures), change the table and run cargo run -p anybox-gen
    "anybox_makcu_interpolation": (_status, [_handle, POINTER(MakcuInterpolation)]),
    "anybox_makcu_set_interpolation": (_status, [_handle, POINTER(MakcuInterpolation)]),
    "anybox_makcu_moving": (_status, [_handle, POINTER(c_bool)]),
    "anybox_makcu_autorelease": (_status, [_handle, POINTER(c_uint32)]),
    "anybox_makcu_set_autorelease": (_status, [_handle, c_uint32]),
    # the end of what bindings/gen makes (makcu_signatures)
    # made by bindings/gen from its call table (ysc_signatures), change the table and run cargo run -p anybox-gen
    "anybox_ysc_interpolation": (_status, [_handle, POINTER(YscInterpolation)]),
    "anybox_ysc_set_interpolation": (_status, [_handle, POINTER(YscInterpolationSettings)]),
    "anybox_ysc_reset_interpolation": (_status, [_handle, POINTER(YscInterpolation)]),
    "anybox_ysc_serial": (_status, [_handle, _out]),
    # the end of what bindings/gen makes (ysc_signatures)
    "anybox_ysc_serial_custom": (c_char_p, [_handle]),
    "anybox_ysc_serial_physical": (c_char_p, [_handle]),
    "anybox_ysc_serial_active": (c_char_p, [_handle]),
    "anybox_ysc_serial_free": (None, [_handle]),
    # made by bindings/gen from its call table (ferrum_signatures), change the table, then cargo run -p anybox-gen
    "anybox_ferrum_catch_xy": (_status, [_handle, c_uint32, c_bool, POINTER(c_int32), POINTER(c_int32)]),
    # the end of what bindings/gen makes (ferrum_signatures)
}

_MOCK_FUNCTIONS = {
    "anybox_mock_medius_open": (_status, [POINTER(ClickTiming), c_bool, _out, _out]),
    "anybox_mock_medius_open_manual": (_status, [POINTER(ClickTiming), c_bool, _out, _out]),
    "anybox_mock_makcu_open": (_status, [POINTER(ClickTiming), _out]),
    "anybox_mock_ysc_open": (_status, [POINTER(ClickTiming), _out]),
    "anybox_mock_ferrum_open": (_status, [POINTER(ClickTiming), _out]),
    "anybox_mock_subscribed": (c_bool, [_handle, c_uint32]),
    "anybox_mock_push_motion": (
        _status,
        [_handle, c_uint8, c_uint32, c_int16, c_int16, c_int16, c_int16],
    ),
    "anybox_mock_push_buttons": (_status, [_handle, c_uint8, c_uint32, c_bool, c_uint32]),
    "anybox_mock_push_keys": (_status, [_handle, c_uint8, c_uint32, c_bool, POINTER(c_uint8), c_size_t]),
    "anybox_mock_push_media": (_status, [_handle, c_uint8, c_uint32, c_bool, POINTER(c_uint16), c_size_t]),
    "anybox_mock_link_lost": (_status, [_handle]),
    "anybox_mock_restart": (_status, [_handle]),
    "anybox_mock_tick": (_status, [_handle]),
    "anybox_mock_recorded": (c_size_t, [_handle]),
    "anybox_mock_recorded_frame": (
        _status,
        [_handle, c_size_t, POINTER(c_uint8), POINTER(c_uint8), c_size_t, POINTER(c_size_t)],
    ),
    "anybox_mock_clear_recorded": (_status, [_handle]),
    "anybox_mock_example_events": (_status, [_handle]),
    "anybox_mock_fail": (_status, [c_uint32]),
    "anybox_mock_free": (None, [_handle]),
}


def _find() -> str:
    override = os.environ.get("ANYBOX_LIB")
    if override:
        if not Path(override).exists():
            raise ImportError(f"ANYBOX_LIB points at {override}, which doesn't exist")
        return override
    bundled = Path(__file__).with_name(_LIBRARY)
    if bundled.exists():
        return str(bundled)
    found = find_library("anybox")
    if found:
        return found
    raise ImportError(
        f"couldn't find {_LIBRARY}, it ships inside the wheel, so reinstall anybox-rs. if you built it "
        "yourself, point ANYBOX_LIB at it"
    )


def load(path: str, abi: int = ABI_VERSION) -> ctypes.CDLL:
    # CDLL and not PyDLL, so every call lets go of the GIL and a thread waiting on events doesn't stall the rest
    lib = ctypes.CDLL(path)
    try:
        found = lib.anybox_abi_version()
    except AttributeError:
        raise ImportError(f"{path} isn't an anybox library, point ANYBOX_LIB at the right one") from None
    if found != abi:
        raise ImportError(
            f"{path} speaks C ABI {found} and this package needs {abi}, so install the anybox-rs "
            "wheel that matches it, or build the library from the same release as this package"
        )
    functions = dict(_FUNCTIONS)
    lib.anybox_has_mock.restype = c_bool
    if lib.anybox_has_mock():
        functions.update(_MOCK_FUNCTIONS)
    for name, (restype, argtypes) in functions.items():
        function = getattr(lib, name)
        function.restype = restype
        function.argtypes = argtypes
    lib.path = path
    return lib


lib = load(_find())

# the medius package loads whatever MEDIUS_LIB names, and this library has every medius function in it, so with
# anybox imported first, device.medius() can hand the medius package a handle from here
if "medius._native" not in sys.modules:
    os.environ.setdefault("MEDIUS_LIB", lib._name)


def text(value: bytes | None) -> str | None:
    return None if value is None else value.decode("utf-8")


def char_array(value: str) -> bytes:
    """a NUL-free UTF-8 string for a `const char *`, a NUL inside would cut it short on the C side"""
    encoded = value.encode("utf-8")
    if b"\0" in encoded:
        raise ValueError("the text can't have a NUL character in it")
    return encoded
