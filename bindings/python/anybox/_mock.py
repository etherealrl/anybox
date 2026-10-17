"""the Medius mock box, for testing code that uses anybox with no box plugged in. the wheels on PyPI don't have it,
it's only in a library built with the capi crate's `mock` feature"""

from __future__ import annotations

import ctypes
import weakref
from collections.abc import Sequence
from typing import TypeVar

from ._device import Device, _click_timing_in, _Handle, _i16, _int, _u32
from ._errors import Error, check, error
from ._native import lib
from ._types import ClickTiming, EventKinds, Key, MediaKey

E = TypeVar("E", bound=Error)


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

    def subscribed(self, kinds: EventKinds) -> bool:
        """whether the mock box got the subscription of every one of these kinds yet. a receiver subscribes each
        kind on the device's own threads, the keys and media a moment after the buttons, and the box drops what's
        pushed before its kind is in like a real one would, so wait for this before you push"""
        with self._handle.use() as pointer:
            return bool(lib.anybox_mock_subscribed(pointer, _int(kinds, 0, 15, "kinds")))

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

    def push_keys(self, seq: int, ts_us: int, pressed: bool, held: Sequence[Key]) -> None:
        """the physical keys held now, modifiers included. it's the whole held set like `push_buttons`, so the last
        release is pushing no keys at all"""
        keys = (ctypes.c_uint8 * len(held))(*(_int(key, 0, 0xFF, "a key") for key in held))
        with self._handle.use() as pointer:
            check(
                lib.anybox_mock_push_keys(
                    pointer, _int(seq, 0, 255, "seq"), _u32(ts_us, "ts_us"), pressed, keys, len(held)
                )
            )

    def push_media(self, seq: int, ts_us: int, pressed: bool, held: Sequence[MediaKey]) -> None:
        """the media keys held now, the whole held set like the keys"""
        media = (ctypes.c_uint16 * len(held))(*(_int(key, 0, 0xFFFF, "a media key") for key in held))
        with self._handle.use() as pointer:
            check(
                lib.anybox_mock_push_media(
                    pointer, _int(seq, 0, 255, "seq"), _u32(ts_us, "ts_us"), pressed, media, len(held)
                )
            )

    def link_lost(self) -> None:
        """the link between the box's two chips drops and comes back, so the box lets go of what it held and the
        device sends `SessionReset` once it checks the link (within 250 ms, or on the next `tick()` of a manual
        mock)"""
        with self._handle.use() as pointer:
            check(lib.anybox_mock_link_lost(pointer))

    def restart(self) -> None:
        """the box restarts, so the device sends `Disconnected` and then `Reconnected`, noticed the way
        `link_lost()` is"""
        with self._handle.use() as pointer:
            check(lib.anybox_mock_restart(pointer))

    def tick(self) -> None:
        """one pass of what a real device checks on the link every 250 ms, on a mock from `mock_medius_manual`,
        which is where it notices `link_lost()` and `restart()`. the mock keeps its own timers (a restarted clone
        is back 100 ms later), so a test still waits for those"""
        with self._handle.use() as pointer:
            check(lib.anybox_mock_tick(pointer))

    def recorded(self) -> list[tuple[int, bytes]]:
        """every frame the box got since it opened or since `clear_recorded()`, as its type byte (the protocol's,
        `MOVE` is 1) and its payload"""
        frames = []
        with self._handle.use() as pointer:
            for index in range(lib.anybox_mock_recorded(pointer)):
                kind = ctypes.c_uint8()
                size = ctypes.c_size_t()
                check(
                    lib.anybox_mock_recorded_frame(pointer, index, ctypes.byref(kind), None, 0, ctypes.byref(size))
                )
                payload = (ctypes.c_uint8 * size.value)()
                check(
                    lib.anybox_mock_recorded_frame(
                        pointer, index, ctypes.byref(kind), payload, size.value, ctypes.byref(size)
                    )
                )
                frames.append((kind.value, bytes(payload)))
        return frames

    def clear_recorded(self) -> None:
        """forgets every frame the box got so far"""
        with self._handle.use() as pointer:
            check(lib.anybox_mock_clear_recorded(pointer))

    def close(self) -> None:
        self._finalizer()


def mock_medius(click_timing: ClickTiming = ClickTiming.MAKCU, *, keyboard: bool = False) -> tuple[Device, Mock]:
    """a device over a new mock box, a five button mouse with a wheel, and with `keyboard` a six key keyboard with
    media keys behind it too, running the exact code a real Medius gets and checking the link every 250 ms like a
    real one"""
    return _open("anybox_mock_medius_open", click_timing, keyboard)


def mock_medius_manual(
    click_timing: ClickTiming = ClickTiming.MAKCU, *, keyboard: bool = False
) -> tuple[Device, Mock]:
    """the same as `mock_medius`, except that the link only gets checked on `Mock.tick()`, so a test decides when a
    link event gets noticed and nothing lands between the frames it records"""
    return _open("anybox_mock_medius_open_manual", click_timing, keyboard)


def _open(function: str, click_timing: ClickTiming, keyboard: bool) -> tuple[Device, Mock]:
    # a library without the mock has no such function, so the check that says so goes first
    _need_mock()
    mock = ctypes.c_void_p()
    device = ctypes.c_void_p()
    timing = _click_timing_in(click_timing)
    check(getattr(lib, function)(ctypes.byref(timing), keyboard, ctypes.byref(mock), ctypes.byref(device)))
    return Device(device.value), Mock(mock.value)  # type: ignore[arg-type]


def example_error(kind: type[E]) -> E:
    """anybox's example of that error with every field it has filled in, so you can test how your code handles it
    with no box. the errors only this package makes (`PanicError`, `UnknownError`, `RecvClosedError`) have none"""
    _need_mock()
    status = lib.anybox_mock_fail(kind.status)
    if status != kind.status:
        raise error(status)
    made = error(status)
    assert isinstance(made, kind)
    return made


def example_events(device: Device) -> None:
    """sends every receiver of the device one event of every kind, with every field filled in, through the same
    queue as real ones, so you can test how your code reads each kind with no box. none of them does to the device
    what the real one would, so it stays connected through the example `Disconnected`"""
    _need_mock()
    device._call(lib.anybox_mock_example_events)


def _need_mock() -> None:
    if not has_mock():
        raise Error("this anybox library was built without the mock, build anybox-capi with --features mock")
