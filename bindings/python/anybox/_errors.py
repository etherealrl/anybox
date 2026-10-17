"""one exception per status the C ABI gives back, the message is always the library's own text and the fields are
the Rust error's"""

from __future__ import annotations

import ctypes
from typing import Any, Generic, TypeVar, overload

from . import _native as native
from ._native import lib, text
from ._types import Action, Family, FerrumApi, Firmware, FirmwareKind, IoKind, Snapshot, StableId, _IdKind

T = TypeVar("T")


class _Field(Generic[T]):
    """a field of the Rust error, read only so it can't drift from the message it came with"""

    def __init__(self, default: T) -> None:
        self._default = default

    def __set_name__(self, owner: type, name: str) -> None:
        self._name = name

    @overload
    def __get__(self, instance: None, owner: type) -> _Field[T]: ...
    @overload
    def __get__(self, instance: Error, owner: type) -> T: ...
    def __get__(self, instance: Error | None, owner: type) -> T | _Field[T]:
        if instance is None:
            return self
        return instance._fields.get(self._name, self._default)

    def __set__(self, instance: Error, value: T) -> None:
        raise AttributeError(f"{self._name} comes from the library and can't be changed")


class Error(Exception):
    """everything anybox raises. `status` is the C ABI's number for it"""

    status: int = -1

    def __init__(self, message: str, **fields: Any) -> None:
        super().__init__(message)
        self._fields = fields


class NotFoundError(Error):
    """`tried` is every port it looked at"""

    status = 1
    tried = _Field[tuple[str, ...]](())


class ClosedError(Error):
    status = 2


class PortBusyError(Error):
    status = 3
    port = _Field[str]("")


class PermissionDeniedError(Error):
    status = 4
    port = _Field[str]("")


class NoBoxCanError(Error):
    """`boxes` is each box that answered, with the actions it couldn't do"""

    status = 5
    boxes = _Field[tuple[tuple[Snapshot, frozenset[Action]], ...]](())


class WrongFamilyError(Error):
    status = 6
    port = _Field[str]("")
    expected = _Field[Family | None](None)
    found = _Field[Family | None](None)


class WrongBoxError(Error):
    status = 7
    port = _Field[str]("")
    expected = _Field[StableId | None](None)
    found = _Field[StableId | None](None)


class WrongFerrumApiError(Error):
    status = 8
    port = _Field[str]("")
    expected = _Field[FerrumApi | None](None)
    found = _Field[FerrumApi | None](None)


class FirmwareTooOldError(Error):
    status = 9
    family = _Field[Family | None](None)
    found = _Field[Firmware | None](None)
    needed = _Field[Firmware | None](None)


class LibraryTooOldError(Error):
    status = 10
    family = _Field[Family | None](None)
    found = _Field[Firmware | None](None)


class UnsupportedError(Error):
    """`action` and `family` are None when the C ABI refused it on its own, like `device.medius()` on another
    box"""

    status = 11
    action = _Field[Action | None](None)
    family = _Field[Family | None](None)
    reason = _Field[str]("")


class InvalidArgumentError(Error):
    """`action` is None when the package or the C ABI refused the value before anybox saw it"""

    status = 12
    action = _Field[Action | None](None)
    detail = _Field[str]("")


class RejectedError(Error):
    status = 13
    action = _Field[Action | None](None)
    detail = _Field[str]("")


class BoxTimeoutError(Error):
    """the box didn't answer in time, so `recv` timing out is never this"""

    status = 14
    action = _Field[Action | None](None)


class DisconnectedError(Error):
    status = 15


class BoxInterruptedError(Error):
    """the box reconnected or reset in the middle of the call, anybox already let go of it"""

    status = 16
    action = _Field[Action | None](None)


class IoError(Error):
    """`os_error` is the operating system's own code, when it came from there"""

    status = 17
    kind = _Field[IoKind](IoKind.OTHER)
    os_error = _Field[int | None](None)


class PanicError(Error):
    """anybox panicked inside the call. the device still works and can be closed, but what the call already did
    on the box isn't known"""

    status = 18


class UnknownError(Error):
    """an error the C ABI has no number for yet, the message still says what happened"""

    status = 19


class RecvClosedError(Error):
    """a receiver has nothing left, the device is gone and every event it queued was read. not `ClosedError`,
    which is a call on a closed device or receiver, like in Rust"""

    status = 21


_BY_STATUS = {cls.status: cls for cls in Error.__subclasses__()}

CLOSED = ClosedError.status
UNSUPPORTED = UnsupportedError.status
RECV_TIMEOUT = 20
RECV_CLOSED = RecvClosedError.status


def unknown(what: str, value: int) -> Error:
    return Error(f"the library sent {what} this package doesn't know ({value}), so update anybox-rs")


def firmware(raw: ctypes.Structure, prefix: str) -> Firmware | None:
    """a firmware the C side flattened into fields starting with `prefix`"""
    kind = getattr(raw, prefix + "kind")
    if kind == 0:
        return None
    if kind == FirmwareKind.VERSION:
        version = (getattr(raw, prefix + "major"), getattr(raw, prefix + "minor"), getattr(raw, prefix + "patch"))
        return Firmware(kind, version=version)
    if kind == FirmwareKind.BUILD:
        return Firmware(kind, build=getattr(raw, prefix + "build"))
    if kind == FirmwareKind.PROTOCOL:
        return Firmware(kind, protocol=getattr(raw, prefix + "protocol"))
    if kind == FirmwareKind.TEXT:
        return Firmware(kind, text=text(getattr(raw, prefix + "text")))
    raise unknown("a firmware kind", kind)


def stable_id(raw: ctypes.Structure, prefix: str) -> StableId | None:
    """an id the C side flattened into fields starting with `prefix`"""
    kind = getattr(raw, prefix + "kind")
    if kind == _IdKind.NONE:
        return None
    if kind == _IdKind.MAC:
        return StableId(mac=bytes(getattr(raw, prefix + "mac")))
    if kind == _IdKind.USB_SERIAL:
        return StableId(usb_serial=text(getattr(raw, prefix + "serial")))
    raise unknown("an id kind", kind)


def fields(detail: native.ErrorDetail) -> dict[str, Any]:
    """the Rust error's fields, copied out of the detail before anything else can replace it"""
    status = detail.kind
    port = text(detail.port) or ""
    action = Action(detail.action) if detail.has_action else None
    family = Family(detail.family) if detail.family else None
    if status in (PortBusyError.status, PermissionDeniedError.status):
        return {"port": port}
    if status == WrongFamilyError.status:
        found = Family(detail.found_family) if detail.found_family else None
        return {"port": port, "expected": Family(detail.expected_family), "found": found}
    if status == WrongBoxError.status:
        expected = stable_id(detail, "expected_id_")
        return {"port": port, "expected": expected, "found": stable_id(detail, "found_id_")}
    if status == WrongFerrumApiError.status:
        expected = FerrumApi(detail.expected_ferrum_api)
        return {"port": port, "expected": expected, "found": FerrumApi(detail.found_ferrum_api)}
    if status == FirmwareTooOldError.status:
        found = firmware(detail, "found_firmware_")
        return {"family": family, "found": found, "needed": firmware(detail, "needed_firmware_")}
    if status == LibraryTooOldError.status:
        return {"family": family, "found": firmware(detail, "found_firmware_")}
    if status == UnsupportedError.status:
        return {"action": action, "family": family, "reason": text(detail.text) or ""}
    if status in (InvalidArgumentError.status, RejectedError.status):
        return {"action": action, "detail": text(detail.text) or ""}
    if status in (BoxTimeoutError.status, BoxInterruptedError.status):
        return {"action": action}
    if status == IoError.status:
        os_error = detail.os_error if detail.has_os_error else None
        return {"kind": IoKind(detail.io_kind), "os_error": os_error}
    return {}


def _tried() -> tuple[str, ...]:
    out = ctypes.c_void_p()
    check(lib.anybox_last_error_tried(ctypes.byref(out)))
    try:
        return tuple(
            text(lib.anybox_string_list_get(out, index)) or ""
            for index in range(lib.anybox_string_list_count(out))
        )
    finally:
        lib.anybox_string_list_free(out)


def _incapable() -> tuple[tuple[Snapshot, frozenset[Action]], ...]:
    from ._device import missing, snapshot_of

    out = ctypes.c_void_p()
    check(lib.anybox_last_error_incapable(ctypes.byref(out)))
    try:
        return tuple(
            (snapshot_of(out, index), missing(out, index)) for index in range(lib.anybox_scan_list_count(out))
        )
    finally:
        lib.anybox_scan_list_free(out)


def error(status: int) -> Error:
    """the exception for a status that just came back, with the text and the fields the library left for this
    thread. the order is fixed, the flat fields first, then the lists, then the exception, so a failure on the way
    can't mix two errors"""
    message = text(lib.anybox_last_error_message()) or f"anybox failed with status {status}"
    detail = native.ErrorDetail()
    lib.anybox_last_error_detail(ctypes.byref(detail))
    found = fields(detail) if detail.kind == status else {}
    if status == NotFoundError.status:
        found["tried"] = _tried()
    if status == NoBoxCanError.status:
        found["boxes"] = _incapable()
    return _BY_STATUS.get(status, UnknownError)(message, **found)


def check(status: int) -> None:
    if status != 0:
        raise error(status)
