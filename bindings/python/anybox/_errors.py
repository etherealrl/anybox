"""one exception per status the C ABI gives back, the message is always the library's own text"""

from __future__ import annotations

from ._native import lib, text


class Error(Exception):
    """everything anybox raises. `status` is the C ABI's number for it"""

    status: int = -1


class NotFoundError(Error):
    status = 1


class ClosedError(Error):
    status = 2


class PortBusyError(Error):
    status = 3


class PermissionDeniedError(Error):
    status = 4


class NoBoxCanError(Error):
    status = 5


class WrongFamilyError(Error):
    status = 6


class WrongBoxError(Error):
    status = 7


class WrongFerrumApiError(Error):
    status = 8


class FirmwareTooOldError(Error):
    status = 9


class LibraryTooOldError(Error):
    status = 10


class UnsupportedError(Error):
    status = 11


class InvalidArgumentError(Error):
    status = 12


class RejectedError(Error):
    status = 13


class BoxTimeoutError(Error):
    """the box didn't answer in time, so `recv` timing out is never this"""

    status = 14


class DisconnectedError(Error):
    status = 15


class BoxInterruptedError(Error):
    """the box reconnected or reset in the middle of the call, anybox already let go of it"""

    status = 16


class IoError(Error):
    status = 17


class PanicError(Error):
    """anybox panicked inside the call. the device still works and can be closed, but what the call already did
    on the box isn't known"""

    status = 18


class UnknownError(Error):
    """an error the C ABI has no number for yet, the message still says what happened"""

    status = 19


_BY_STATUS = {cls.status: cls for cls in Error.__subclasses__()}

CLOSED = ClosedError.status
UNSUPPORTED = UnsupportedError.status
RECV_TIMEOUT = 20
RECV_CLOSED = 21


def error(status: int) -> Error:
    """the exception for a status that just came back, with the text the library left for this thread"""
    message = text(lib.anybox_last_error_message()) or f"anybox failed with status {status}"
    return _BY_STATUS.get(status, UnknownError)(message)


def check(status: int) -> None:
    if status != 0:
        raise error(status)
