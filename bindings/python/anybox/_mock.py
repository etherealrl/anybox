"""the Medius mock box, for testing code that uses anybox with no box plugged in. the wheels on PyPI don't have it,
it's only in a library built with the capi crate's `mock` feature"""

from __future__ import annotations

import ctypes
import weakref

from ._device import Device, _click_timing_in, _Handle, _i16, _int, _u32
from ._errors import Error, check
from ._native import lib
from ._types import ClickTiming


def has_mock() -> bool:
    """whether the library was built with the mock, ask BEFORE calling `mock_medius`"""
    return bool(lib.anybox_has_mock())


class Mock:
    """the medius crate's model of a Medius box. it proves nothing about hardware and it can only be shaped like
    a Medius"""

    def __init__(self, pointer: int) -> None:
        self._handle = _Handle(
            pointer, lib.anybox_mock_free, "this mock was closed, make a new one with mock_medius()"
        )
        self._finalizer = weakref.finalize(self, self._handle.close)

    def subscribed(self) -> bool:
        """whether anything subscribed to the mock's input yet. a receiver subscribes on its own thread, so wait
        for this before you push, otherwise the box drops what you pushed like a real one would"""
        with self._handle.use() as pointer:
            return bool(lib.anybox_mock_subscribed(pointer))

    def push_motion(self, seq: int, ts_us: int, dx: int, dy: int, wheel: int = 0, pan: int = 0) -> None:
        """physical motion as if from the box. `seq` is the box's frame counter and `ts_us` its
        clock. a gap in `seq` doesn't show up as a drop"""
        with self._handle.use() as pointer:
            check(
                lib.anybox_mock_push_motion(
                    pointer,
                    _int(seq, 0, 255, "seq"),
                    _u32(ts_us, "ts_us"),
                    _i16(dx, "dx"),
                    _i16(dy, "dy"),
                    _i16(wheel, "wheel"),
                    _i16(pan, "pan"),
                )
            )

    def push_buttons(self, seq: int, ts_us: int, pressed: bool, held: int) -> None:
        """the physical buttons held now, bit n for the box's button n. it's the whole held set, so a release is
        the set without that button, and `pressed` says whether the set grew or shrank"""
        with self._handle.use() as pointer:
            check(
                lib.anybox_mock_push_buttons(
                    pointer, _int(seq, 0, 255, "seq"), _u32(ts_us, "ts_us"), pressed, _u32(held, "held")
                )
            )

    def close(self) -> None:
        self._finalizer()


def mock_medius(click_timing: ClickTiming = ClickTiming.MAKCU) -> tuple[Device, Mock]:
    """a device over a new mock box, a five button mouse with a wheel, running the exact code a real Medius gets"""
    if not has_mock():
        raise Error("this anybox library was built without the mock, build anybox-capi with --features mock")
    mock = ctypes.c_void_p()
    device = ctypes.c_void_p()
    timing = _click_timing_in(click_timing)
    check(lib.anybox_mock_medius_open(ctypes.byref(timing), ctypes.byref(mock), ctypes.byref(device)))
    return Device(device.value), Mock(mock.value)  # type: ignore[arg-type]
