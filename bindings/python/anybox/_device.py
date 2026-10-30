"""devices, receivers and connecting, over the C handles"""

from __future__ import annotations

import ctypes
import operator
import threading
import weakref
from collections.abc import Callable, Iterator, Sequence
from contextlib import contextmanager
from dataclasses import dataclass
from types import MappingProxyType
from typing import Any

from . import _errors
from . import _native as native
from ._errors import RECV_TIMEOUT, ClosedError, Error, RecvClosedError, check, error, firmware, stable_id, unknown
from ._native import lib, text
from ._types import (
    Action,
    Axis,
    BoxTime,
    Button,
    ButtonEvent,
    Capabilities,
    Capability,
    Changed,
    ClickTiming,
    Clock,
    Direction,
    Disconnected,
    Dropped,
    DropSource,
    Event,
    EventKinds,
    Execution,
    Family,
    FerrumApi,
    Info,
    Interpolation,
    Key,
    KeyEvent,
    MediaEvent,
    MediaKey,
    MotionEvent,
    Quirk,
    QuirkKind,
    Reconnected,
    Rejected,
    SessionReset,
    Snapshot,
    StableId,
    Stamp,
    UsbIds,
    YscInterpolation,
    YscInterpolationSettings,
    YscProfile,
    YscSerial,
    _ClickTimingKind,
    _EventKind,
    _IdKind,
)


def _int(value: int, low: int, high: int, what: str) -> int:
    """ctypes wraps an int that doesn't fit without saying anything, so it gets checked here first"""
    number = operator.index(value)
    if not low <= number <= high:
        raise ValueError(f"{what} goes from {low} to {high}, not {number}")
    return number


def _i16(value: int, what: str) -> int:
    return _int(value, -32768, 32767, what)


def _micros(seconds: float, what: str) -> int:
    if seconds < 0:
        raise ValueError(f"{what} can't be negative")
    # whole microseconds cut short like Rust's `as_micros` and the C# package, after rounding to nanoseconds so a
    # float like 0.0015 doesn't land a hair under 1500
    return _int(round(seconds * 1_000_000_000) // 1000, 0, 2**64 - 1, what)


def _seconds(micros: int) -> float:
    return micros / 1_000_000


class _Handle:
    """one C handle. a call counts itself in while it runs, and the handle only gets freed once it's closed and no
    call is left on it, because the C side must never free something another thread is still using"""

    def __init__(self, pointer: int, free: Callable[[int], None], closed: str) -> None:
        self._pointer: int | None = pointer
        self._closed_text = closed
        self._free = free
        self._lock = threading.Lock()
        self._calls = 0
        self._closed = False

    @contextmanager
    def use(self) -> Iterator[int]:
        with self._lock:
            if self._closed:
                raise ClosedError(self._closed_text)
            self._calls += 1
            pointer = self._pointer
        try:
            yield pointer  # type: ignore[misc]
        finally:
            with self._lock:
                self._calls -= 1
                last = self._closed and self._calls == 0
            if last:
                self._release()

    @property
    def closed(self) -> bool:
        return self._closed

    def close(self, first: Callable[[int], int] | None = None) -> int:
        """`first` runs on the handle before it can be freed, counted in like any call, so a receiver can wake
        whoever waits on it. it's what `first` gave back, 0 when there was nothing to do"""
        with self._lock:
            if self._closed:
                return 0
            # the flag goes up first, so a call that comes back after this already sees it
            self._closed = True
            pointer = self._pointer
            self._calls += 1
        try:
            return first(pointer) if first and pointer else 0
        finally:
            with self._lock:
                self._calls -= 1
                idle = self._calls == 0
            if idle:
                self._release()

    @property
    def closed_text(self) -> str:
        return self._closed_text

    def _release(self) -> None:
        with self._lock:
            pointer, self._pointer = self._pointer, None
        if pointer:
            self._free(pointer)


def _click(hold: float | None, native_only: bool) -> native.Click:
    click = native.Click(native_only=native_only)
    if hold is not None:
        click.hold_us = _micros(hold, "a hold")
        click.has_hold = True
    return click


def _click_timing_in(timing: ClickTiming) -> native.ClickTiming:
    kind = _ClickTimingKind(timing.kind)
    if (kind == _ClickTimingKind.FIXED) != (timing.hold is not None):
        raise ValueError("only a fixed click timing has a hold, use ClickTiming.fixed(seconds) for one")
    fixed = 0 if timing.hold is None else _micros(timing.hold, "a fixed click timing")
    return native.ClickTiming(kind=kind, fixed_us=fixed)


def _click_timing_out(timing: native.ClickTiming) -> ClickTiming:
    kind = _ClickTimingKind(timing.kind)
    if kind == _ClickTimingKind.FIXED:
        return ClickTiming.fixed(_seconds(timing.fixed_us))
    return ClickTiming(kind)


def _stamp(event: native.Event) -> Stamp:
    return Stamp(
        since_open_ns=event.since_open_ns,
        received_since_open_ns=event.received_since_open_ns,
        arrived_ns=event.arrived_ns,
        received_ns=event.received_ns,
        received_qpc=event.received_qpc if event.has_received_qpc else None,
        box_time=BoxTime(event.box_time_us, Clock(event.box_clock)) if event.has_box_time else None,
    )


def _event(event: native.Event) -> Event:
    try:
        kind = _EventKind(event.kind)
    except ValueError:
        raise unknown("an event", event.kind) from None
    if kind == _EventKind.BUTTON:
        return ButtonEvent(Button(event.button), event.pressed, _stamp(event))
    if kind == _EventKind.KEY:
        return KeyEvent(Key(event.key), event.pressed, _stamp(event))
    if kind == _EventKind.MEDIA:
        return MediaEvent(MediaKey(event.media), event.pressed, _stamp(event))
    if kind == _EventKind.MOTION:
        return MotionEvent(event.dx, event.dy, event.wheel, event.pan, _stamp(event))
    if kind == _EventKind.DROPPED:
        return Dropped(event.count, DropSource(event.source))
    if kind == _EventKind.REJECTED:
        return Rejected(Action(event.action), event.count)
    return {
        _EventKind.SESSION_RESET: SessionReset,
        _EventKind.DISCONNECTED: Disconnected,
        _EventKind.RECONNECTED: Reconnected,
        _EventKind.CHANGED: Changed,
    }[kind]()


def _info(raw: native.Info) -> Info:
    return Info(
        port=text(raw.port) or "",
        family=Family(raw.family),
        model=text(raw.model),
        firmware=firmware(raw, "firmware_"),
        id=stable_id(raw, "id_"),
        ferrum_api=FerrumApi(raw.ferrum_api) if raw.ferrum_api else None,
        peripheral=UsbIds(raw.peripheral_vid, raw.peripheral_pid) if raw.has_peripheral else None,
    )


def _quirk(raw: native.Quirk) -> Quirk:
    kind = QuirkKind(raw.kind)
    if kind in (QuirkKind.FORCED_UP_ON_RELEASE, QuirkKind.BOX_CLICK_TIMING):
        return Quirk(kind, min=_seconds(raw.min_us), max=_seconds(raw.max_us))
    if kind == QuirkKind.MAX_HOLD:
        return Quirk(kind, max=_seconds(raw.max_us))
    if kind in (QuirkKind.NOTCHES_PER_COMMAND, QuirkKind.MAX_KEYS_HELD):
        return Quirk(kind, count=raw.count)
    if kind == QuirkKind.HOST_CLICK_TIMING:
        return Quirk(kind, click_timing=_click_timing_out(raw.click_timing))
    return Quirk(kind)


def _list(read: Callable[..., int]) -> list[int]:
    """a list the C side fills count first, `read` takes the buffer, its size and where the length goes"""
    length = ctypes.c_size_t()
    check(read(None, 0, ctypes.byref(length)))
    values = (ctypes.c_uint32 * length.value)()
    check(read(values, length.value, ctypes.byref(length)))
    return list(values)


def _snapshot(pointer: int) -> Snapshot:
    """reads the whole snapshot handle into Python values and frees it, so nothing has to keep it alive"""
    try:
        raw = native.Info()
        check(lib.anybox_snapshot_info(pointer, ctypes.byref(raw)))
        info = _info(raw)
        actions = {}
        for action in Action:
            capability = native.Capability()
            check(lib.anybox_snapshot_capability(pointer, action, ctypes.byref(capability)))
            quirks = []
            for index in range(capability.quirk_count):
                quirk = native.Quirk()
                check(lib.anybox_snapshot_quirk(pointer, action, index, ctypes.byref(quirk)))
                quirks.append(_quirk(quirk))
            actions[action] = Capability(Execution(capability.execution), text(capability.reason), tuple(quirks))
        capabilities = Capabilities(
            MappingProxyType(actions),
            tuple(Button(value) for value in _list(lambda *out: lib.anybox_snapshot_buttons(pointer, *out))),
            tuple(Axis(value) for value in _list(lambda *out: lib.anybox_snapshot_axes(pointer, *out))),
        )
        return Snapshot(info, capabilities)
    finally:
        lib.anybox_snapshot_free(pointer)


def snapshot_of(scan_list: ctypes.c_void_p, index: int) -> Snapshot:
    """the box on a scan list entry, raising its error when the entry is one"""
    out = ctypes.c_void_p()
    check(lib.anybox_scan_list_result(scan_list, index, ctypes.byref(out)))
    return _snapshot(out.value)  # type: ignore[arg-type]


def missing(scan_list: ctypes.c_void_p, index: int) -> frozenset[Action]:
    """what the box on a scan list entry couldn't do"""
    values = _list(lambda *out: lib.anybox_scan_list_missing(scan_list, index, *out))
    return frozenset(Action(value) for value in values)


def monotonic_ns() -> int:
    """now on the clock the `arrived_ns` and `received_ns` of every stamp are on, the one `time.perf_counter_ns()`
    reads on Windows and Linux (QueryPerformanceCounter and CLOCK_MONOTONIC, CLOCK_UPTIME_RAW on macOS). it only
    compares on the same machine within one boot"""
    return int(lib.anybox_monotonic_ns())


class Events:
    """a receiver of events from one device. it can be read from any thread, two threads reading the same one
    split the events between them, and iterating it goes until the device is gone and everything queued was
    read"""

    def __init__(self, pointer: int, device: Device) -> None:
        self._handle = _Handle(
            pointer, lib.anybox_events_free, "this receiver was closed, get a new one with device.events()"
        )
        # the receiver keeps its device alive, otherwise `connect().events()` would close the box right away
        self._device = device
        self._finalizer = weakref.finalize(self, self._handle.close)

    def recv(self, timeout: float | None = None) -> Event | None:
        """the next event, waiting up to `timeout` seconds for it (forever with none) and None when nothing came.
        it raises `RecvClosedError` once the device is gone and everything queued was read, and `ClosedError` once
        this receiver was closed"""
        if timeout is not None and timeout < 0:
            raise ValueError("a timeout can't be negative")
        # the library takes a signed count, and a wait that long is forever anyway
        wait = -1 if timeout is None else min(_micros(timeout, "a timeout"), 2**63 - 1)
        out = native.Event()
        with self._handle.use() as pointer:
            status = lib.anybox_events_recv(pointer, wait, ctypes.byref(out))
        # closed while it waited, so whatever came back belongs to a receiver that's gone
        if self._handle.closed:
            raise ClosedError(self._handle.closed_text)
        if status == 0:
            return _event(out)
        if status == RECV_TIMEOUT:
            return None
        raise error(status)

    def try_recv(self) -> Event | None:
        return self.recv(0)

    def __iter__(self) -> Iterator[Event]:
        while True:
            try:
                event = self.recv()
            except (RecvClosedError, ClosedError):
                return
            if event is not None:
                yield event

    def close(self) -> None:
        """ends this receiver now, a `recv` waiting on it from another thread raises `ClosedError` right away, what
        was queued is thrown out, and the box stops streaming what only this receiver wanted. a second close does
        nothing"""
        self._finalizer.detach()
        check(self._handle.close(lib.anybox_events_close))

    def __enter__(self) -> Events:
        return self

    def __exit__(self, _: object, raised: BaseException | None, __: object) -> None:
        if raised is None:
            self.close()
            return
        # like the device's, a close that fails after the block raised doesn't take the place of what it raised
        try:
            self.close()
        except Error as error:
            raised.add_note(f"closing the receiver failed too: {error}")


class Device:
    """one open box. every method can be called from any thread at the same time. use it as a context manager,
    or call `close()`, so what you held gets let go of and the port gets closed"""

    def __init__(self, pointer: int) -> None:
        self._handle = _Handle(
            pointer, lib.anybox_device_free, "this device was closed, open the box again if you need it"
        )
        self._closed = False
        # the C device outlives close, so the snapshot still works after it, and only gets freed with this object
        self._finalizer = weakref.finalize(self, self._handle.close)

    def _call(self, function: Callable[..., int], *args: object) -> None:
        with self._handle.use() as pointer:
            check(function(pointer, *args))

    def snapshot(self) -> Snapshot:
        """who the box is and what it can do, read from the current connection. it can change after `Reconnected`
        or `Changed`"""
        out = ctypes.c_void_p()
        with self._handle.use() as pointer:
            check(lib.anybox_device_snapshot(pointer, ctypes.byref(out)))
        return _snapshot(out.value)  # type: ignore[arg-type]

    # made by bindings/gen from its call table (methods), change the table and run cargo run -p anybox-gen
    def move_by(self, dx: int, dy: int) -> None:
        """moves by counts, going through whatever smoothing the box has on"""
        self._call(lib.anybox_device_move_by, _i16(dx, "dx"), _i16(dy, "dy"))

    def move_now(self, dx: int, dy: int) -> None:
        """moves by counts on the box's next report, skipping its smoothing for this move"""
        self._call(lib.anybox_device_move_now, _i16(dx, "dx"), _i16(dy, "dy"))

    def press(self, button: Button) -> None:
        """holds a button down until `release`"""
        self._call(lib.anybox_device_press, _button(button))

    def release(self, button: Button) -> None:
        """lets go of a button"""
        self._call(lib.anybox_device_release, _button(button))

    def click(self, button: Button, *, hold: float | None = None, native_only: bool = False) -> None:
        """the box's own click, or one timed on the PC with the device's click timing if the box has none, which
        blocks the thread for the hold. `hold` is in seconds, and `native_only` fails instead of letting anybox
        time it from the PC"""
        self._call(lib.anybox_device_click_with, _button(button), ctypes.byref(_click(hold, native_only)))

    def press_key(self, key: Key) -> None:
        """holds a key down until `release_key`. a key is its HID usage, from 0x04 to 0xE7"""
        self._call(lib.anybox_device_press_key, _key(key))

    def release_key(self, key: Key) -> None:
        """lets go of a key"""
        self._call(lib.anybox_device_release_key, _key(key))

    def tap_key(self, key: Key, *, hold: float | None = None, native_only: bool = False) -> None:
        """presses and lets go, timed by the box, or on the PC with the device's click timing if the box can't,
        which blocks the thread for the hold. `hold` is in seconds, and `native_only` fails instead of letting
        anybox time it from the PC"""
        self._call(lib.anybox_device_tap_key_with, _key(key), ctypes.byref(_click(hold, native_only)))

    def type_text(self, text: str) -> None:
        """types the text on the box with the box's own timing, as US layout key presses, and returns once the box
        has typed all of it. it takes printable ASCII only, so tap keys like Enter yourself after it. other calls
        on the device can wait while it types, and a close stops it after the few characters going out. after an
        error part of it may already be typed"""
        self._call(lib.anybox_device_type_text, native.char_array(text))

    def press_keys(self, *keys: Key) -> None:
        """holds the keys down with one command of the box, up to 14 of them, until `release_keys`"""
        self._call(lib.anybox_device_press_keys, _keys(keys), len(keys))

    def release_keys(self, *keys: Key) -> None:
        """lets go of the keys with one command of the box, or one at a time like `release_key` on a box that
        can't, so a key you hold always gets let go. a key you don't hold still has to be one the box can let go
        of, so it can fail as unsupported after the held ones already went"""
        self._call(lib.anybox_device_release_keys, _keys(keys), len(keys))

    def tap_keys(self, *keys: Key) -> None:
        """presses and lets go of the keys with one command of the box, timed by the box"""
        self._call(lib.anybox_device_tap_keys, _keys(keys), len(keys))

    def press_media(self, key: MediaKey) -> None:
        """holds a media key down until `release_media`. a media key is its HID consumer usage"""
        self._call(lib.anybox_device_press_media, _media(key))

    def release_media(self, key: MediaKey) -> None:
        """lets go of a media key"""
        self._call(lib.anybox_device_release_media, _media(key))

    def tap_media(self, key: MediaKey, *, hold: float | None = None, native_only: bool = False) -> None:
        """presses and lets go of a media key, timed like a key's tap, which blocks the thread for the hold. `hold`
        is in seconds, and `native_only` fails instead of letting anybox time it from the PC"""
        self._call(lib.anybox_device_tap_media_with, _media(key), ctypes.byref(_click(hold, native_only)))

    def lock_button(self, button: Button) -> None:
        """keeps the physical button from reaching the PC until `unlock_button`. your own presses still go through
        and its events still come, so you can take the button over"""
        self._call(lib.anybox_device_lock_button, _button(button))

    def unlock_button(self, button: Button) -> None:
        """lets the physical button through again"""
        self._call(lib.anybox_device_unlock_button, _button(button))

    def force_release_key(self, key: Key) -> None:
        """the PC sees the key up from now on, whatever the finger does, until `restore_key`. key events keep
        reporting the finger, so you can tell whether it's still held. a key anybox holds or has locked itself, or
        one a text needs while it types, is refused, since each box would do something else with it"""
        self._call(lib.anybox_device_force_release_key, _key(key))

    def restore_key(self, key: Key) -> None:
        """gives the PC the finger's state of the key again, so a key that's still held goes down for it. it also
        clears a force another program left, except on a key anybox holds or has locked"""
        self._call(lib.anybox_device_restore_key, _key(key))

    def lock_key(self, key: Key) -> None:
        """keeps the physical key from reaching the PC until `unlock_key`"""
        self._call(lib.anybox_device_lock_key, _key(key))

    def unlock_key(self, key: Key) -> None:
        """lets the physical key through again"""
        self._call(lib.anybox_device_unlock_key, _key(key))

    def lock_axis(self, axis: Axis, direction: Direction = Direction.BOTH) -> None:
        """keeps physical motion one way or both ways along an axis from reaching the PC until `unlock_axis`. your
        own moves still go through"""
        self._call(lib.anybox_device_lock_axis, _u32(axis, "an axis"), _u32(direction, "a direction"))

    def unlock_axis(self, axis: Axis, direction: Direction = Direction.BOTH) -> None:
        """unlocks that way along the axis and leaves the other way as it was. it also undoes a scale there"""
        self._call(lib.anybox_device_unlock_axis, _u32(axis, "an axis"), _u32(direction, "a direction"))

    def scale_axis(self, axis: Axis, direction: Direction, percent: int) -> None:
        """keeps `percent` of the physical motion one way or both ways along an axis, from -255 to 255. 100 leaves
        it alone, 0 blocks it and -100 inverts it. `unlock_axis` puts it back to 100"""
        self._call(
            lib.anybox_device_scale_axis,
            _u32(axis, "an axis"),
            _u32(direction, "a direction"),
            _i16(percent, "percent"),
        )

    def wheel(self, notches: int) -> None:
        """positive scrolls up"""
        self._call(lib.anybox_device_wheel, _i16(notches, "notches"))

    def pan(self, notches: int) -> None:
        """horizontal scroll, positive goes right"""
        self._call(lib.anybox_device_pan, _i16(notches, "notches"))

    def discard_pending_moves(self) -> None:
        """throws away the moves the box took but hasn't sent yet"""
        self._call(lib.anybox_device_discard_pending_moves)

    # the end of what bindings/gen makes (methods)

    def events(self, kinds: EventKinds = EventKinds.ALL) -> Events:
        """a new receiver for these kinds of input, plus the connection events. the box only streams while at
        least one receiver is alive. a kind this box can't stream gets one `Rejected` right away, except with
        `EventKinds.ALL`, which means everything this box has. just know that a receiver of keys sees every key
        typed on the keyboard behind the box, passwords included"""
        out = ctypes.c_void_p()
        with self._handle.use() as pointer:
            check(lib.anybox_device_events(pointer, _int(kinds, 0, 15, "kinds"), ctypes.byref(out)))
        return Events(out.value, self)  # type: ignore[arg-type]

    def makcu(self) -> Makcu | None:
        """what only a MAKCU has, None on another box and once this device is closed"""
        return Makcu(self) if self._is(Family.MAKCU_V4) else None

    def ysc(self) -> Ysc | None:
        """what only a YSC8K has, None on another box and once this device is closed"""
        return Ysc(self) if self._is(Family.YSC) else None

    def ferrum(self) -> Ferrum | None:
        """what only a Ferrum has, None on another box and once this device is closed"""
        return Ferrum(self) if self._is(Family.FERRUM) else None

    def _is(self, family: Family) -> bool:
        return not self._closed and self.snapshot().info.family == family

    def _out(self, function: Callable[..., int], *args: object) -> None:
        with self._handle.use() as pointer:
            check(function(pointer, *args))

    def medius(self) -> Any:
        """on a Medius, the medius package's own `Device` for everything only Medius does (clips, transforms,
        rendering, the polling rate and so on), and None on any other box or once this device is closed. it needs
        `pip install anybox-rs[medius]`. it shares this device's connection, and keeping it holds the port open
        after this device is gone. anybox doesn't know about anything you press or lock through it, and its
        `reset()` lets go of and unlocks EVERYTHING, what anybox did too"""
        out = ctypes.c_void_p()
        with self._handle.use() as pointer:
            status = lib.anybox_device_medius(pointer, ctypes.byref(out))
        if status in (_errors.UNSUPPORTED, _errors.CLOSED):
            return None
        check(status)
        # asked first, so another box or a closed device is None like in Rust even without the medius package
        try:
            medius = _medius_package()
        except BaseException:
            free = lib.medius_device_free
            free.argtypes = [ctypes.c_void_p]
            free.restype = None
            free(out.value)
            raise
        return medius.Device(out.value)

    def close(self) -> None:
        """lets go of what you held, unlocks, and closes the port, and raises if that didn't work. it waits up to
        about 2 s when the box doesn't answer. after it every call raises `ClosedError` and the receivers end
        once they're read empty, `events()` gives one that's already ended and `snapshot()` still works. a second
        `close()` waits for the first one and then raises `ClosedError`, like in Rust"""
        self._closed = True
        self._call(lib.anybox_device_close)

    def __enter__(self) -> Device:
        return self

    def __exit__(self, _: object, raised: BaseException | None, __: object) -> None:
        # a device you already closed inside the block is fine, only the first close says how it went
        if self._closed:
            return
        if raised is None:
            self.close()
            return
        # what the block raised is what you need, so like Rust's drop, a close that fails after it doesn't take its
        # place
        try:
            self.close()
        except Error as error:
            raised.add_note(f"closing the device failed too: {error}")


def _medius_package() -> Any:
    """the medius package, which has to run on this package's library, because a handle only works with the
    library it came from"""
    try:
        import medius
    except ImportError as error:
        raise ImportError(
            f"device.medius() needs the medius package, install it with pip install anybox-rs[medius] ({error})"
        ) from error
    # the loader hands back the same handle for a library it already has, whatever path or name it was asked by,
    # and a name from find_library isn't a path samefile could look at
    if medius._native.lib._handle != lib._handle:
        raise Error(
            f"the medius package loaded its own library ({medius._native.lib._name}), so it can't use anybox's "
            "connection. import anybox before medius and leave MEDIUS_LIB unset"
        )
    return medius


def _u32(value: int, what: str) -> int:
    return _int(value, 0, 2**32 - 1, what)


def _button(button: Button) -> int:
    return _u32(button, "a button")


def _key(key: Key) -> int:
    return _int(key, 0, 0xFF, "a key")


def _keys(keys: tuple[Key, ...]) -> ctypes.Array:
    return (ctypes.c_uint8 * len(keys))(*(_key(key) for key in keys))


def _media(key: MediaKey) -> int:
    return _int(key, 0, 0xFFFF, "a media key")


@dataclass(frozen=True)
class Scanned:
    """one port a scan looked at, with the box on it in `found`, or in `error` why a search of just that port
    fails. mostly that's busy (the box you already have open shows up as busy too), not allowed, too old,
    `NoBoxCanError` for a box that can't do what `can` asked, or `NotFoundError` when nothing answered"""

    port: str
    found: Snapshot | None
    error: Error | None


@dataclass(frozen=True)
class Connect:
    """what to look for. `port` only asks that port, `family` only that kind of box, `ferrum_api` only a Ferrum on
    that API, `can` skips a box that can't do those actions, `id` only takes the box with that id, and
    `click_timing` is what a click does on a box with no click of its own"""

    port: str | None = None
    family: Family | None = None
    ferrum_api: FerrumApi | None = None
    can: Sequence[Action] = ()
    id: StableId | None = None
    click_timing: ClickTiming = ClickTiming.MAKCU

    def _options(self) -> native.ConnectOptions:
        options = native.ConnectOptions()
        if self.port is not None:
            options.port = native.char_array(self.port)
        options.family = 0 if self.family is None else _u32(self.family, "a family")
        options.ferrum_api = 0 if self.ferrum_api is None else _u32(self.ferrum_api, "a Ferrum API")
        if self.can:
            can = (ctypes.c_uint32 * len(self.can))(*(_u32(action, "an action") for action in self.can))
            # the options only point at the array, so they keep it alive for as long as they're used
            options._can = can
            options.can = ctypes.cast(can, ctypes.POINTER(ctypes.c_uint32))
            options.can_len = len(self.can)
        if self.id is not None and self.id.mac is not None:
            options.id_kind = _IdKind.MAC
            options.id_mac[:] = list(self.id.mac)
        elif self.id is not None:
            options.id_kind = _IdKind.USB_SERIAL
            options.id_serial = native.char_array(self.id.usb_serial or "")
        options.click_timing = _click_timing_in(self.click_timing)
        return options

    def open(self) -> Device:
        """opens the first box that fits, in port order"""
        out = ctypes.c_void_p()
        check(lib.anybox_open(ctypes.byref(self._options()), ctypes.byref(out)))
        return Device(out.value)  # type: ignore[arg-type]

    def scan(self) -> list[Scanned]:
        """every port a search looks at and what's on it, in port order. it opens, describes and closes every
        box, so it's slow (a YSC search takes about 2 s a port), `candidates()` is the cheap one to call often"""
        out = ctypes.c_void_p()
        check(lib.anybox_scan(ctypes.byref(self._options()), ctypes.byref(out)))
        scanned = []
        try:
            for index in range(lib.anybox_scan_list_count(out)):
                port = text(lib.anybox_scan_list_port(out, index)) or ""
                snapshot = ctypes.c_void_p()
                status = lib.anybox_scan_list_result(out, index, ctypes.byref(snapshot))
                if status == 0:
                    scanned.append(Scanned(port, _snapshot(snapshot.value), None))  # type: ignore[arg-type]
                else:
                    scanned.append(Scanned(port, None, error(status)))
        finally:
            lib.anybox_scan_list_free(out)
        return scanned

    def candidates(self) -> list[str]:
        """the ports a search would look at, without opening any of them or sending them a byte, so it's cheap
        enough to call every second and scan when the list changes"""
        out = ctypes.c_void_p()
        check(lib.anybox_candidates(ctypes.byref(self._options()), ctypes.byref(out)))
        try:
            return [
                text(lib.anybox_string_list_get(out, index)) or ""
                for index in range(lib.anybox_string_list_count(out))
            ]
        finally:
            lib.anybox_string_list_free(out)


def connect() -> Device:
    """connects to the first box found, in port order"""
    return Connect().open()


def scan() -> list[Scanned]:
    return Connect().scan()


def candidates() -> list[str]:
    return Connect().candidates()


def _millis(seconds: float, what: str) -> int:
    """the boxes take whole milliseconds, so anything finer is refused instead of cut"""
    if seconds < 0:
        raise ValueError(f"{what} can't be negative")
    nanos = round(seconds * 1_000_000_000)
    if nanos % 1_000_000:
        raise ValueError(f"{what} has to be whole milliseconds, {seconds} s isn't")
    return _int(nanos // 1_000_000, 0, 0xFFFFFFFF, what)


def _ysc_interpolation(raw: native.YscInterpolation) -> YscInterpolation:
    return YscInterpolation(
        enabled=raw.enabled,
        profile=YscProfile(raw.profile),
        window=raw.window_ms / 1000 if raw.has_window else None,
        max_window=raw.max_window_ms / 1000 if raw.has_max_window else None,
        measured_interval=_seconds(raw.measured_interval_us) if raw.has_measured_interval else None,
    )


def _ms_or_off(seconds: float | None, what: str) -> int:
    """0 is off on the C side, so a zero asked for has to fail here like it does in Rust"""
    if seconds is None:
        return 0
    ms = _millis(seconds, what)
    if ms == 0:
        raise ValueError(f"{what} can't be 0, use None to turn it off")
    return ms


def _seconds_or_off(ms: ctypes.c_uint32) -> float | None:
    return ms.value / 1000 if ms.value else None


def _raw_interpolation(value: Interpolation) -> native.MakcuInterpolation:
    return native.MakcuInterpolation(
        value.automatic, 0 if value.automatic else _int(value.percentage, 0, 255, "the percent")
    )


def _interpolation(raw: native.MakcuInterpolation) -> Interpolation:
    return Interpolation.auto() if raw.automatic else Interpolation.percent(raw.percent)


def _raw_ysc_settings(settings: YscInterpolationSettings) -> native.YscInterpolationSettings:
    return native.YscInterpolationSettings(
        settings.enabled,
        _u32(settings.profile, "the profile"),
        _millis(settings.window, "the window"),
        _millis(settings.max_window, "the max window"),
    )


def _ysc_serial(out: ctypes.c_void_p) -> YscSerial:
    """reads the serial and frees it"""
    try:
        return YscSerial(
            custom=text(lib.anybox_ysc_serial_custom(out)),
            physical=text(lib.anybox_ysc_serial_physical(out)),
            active=text(lib.anybox_ysc_serial_active(out)),
        )
    finally:
        lib.anybox_ysc_serial_free(out)


class Makcu:
    """what only a MAKCU has. it keeps the device alive, and each call checks what the box can do right now"""

    def __init__(self, device: Device) -> None:
        self._device = device

    # made by bindings/gen from its call table (makcu_methods), change the table and run cargo run -p anybox-gen
    def interpolation(self) -> Interpolation:
        """how long the box spreads a move, read fresh from the box. it's global to the box and every program on it
        shares it"""
        out = native.MakcuInterpolation()
        self._device._out(lib.anybox_makcu_interpolation, ctypes.byref(out))
        return _interpolation(out)

    def set_interpolation(self, value: Interpolation) -> None:
        """sets it and reads it back, a box that didn't take it raises `RejectedError`. it changes how moves land
        for every program on the box, and anybox leaves it set when you close"""
        self._device._out(lib.anybox_makcu_set_interpolation, ctypes.byref(_raw_interpolation(value)))

    def moving(self) -> bool:
        """whether injected motion is still queued or being spread"""
        out = ctypes.c_bool()
        self._device._out(lib.anybox_makcu_moving, ctypes.byref(out))
        return out.value

    def autorelease(self) -> float | None:
        """how long in seconds the box waits with nothing coming in before it lets go of injected mouse buttons,
        None when it's off"""
        out = ctypes.c_uint32()
        self._device._out(lib.anybox_makcu_autorelease, ctypes.byref(out))
        return _seconds_or_off(out)

    def set_autorelease(self, after: float | None) -> None:
        """turns autorelease on for 1 ms to 60 s (in seconds, whole milliseconds only), or off with None. anybox
        asks the box something about every 250 ms, so with more than that it only fires once your app dies or
        hangs, and a button it held gets let go instead of staying down. the box shares the timer with every
        program on it and forgets it when it loses power, so anybox sends it again after a reconnect. needs V4.108
        """
        self._device._out(lib.anybox_makcu_set_autorelease, _ms_or_off(after, "autorelease"))

    # the end of what bindings/gen makes (makcu_methods)


class Ysc:
    """what only a YSC8K has. it keeps the device alive, and each call checks what the box can do right now"""

    def __init__(self, device: Device) -> None:
        self._device = device

    # made by bindings/gen from its call table (ysc_methods), change the table and run cargo run -p anybox-gen
    def interpolation(self) -> YscInterpolation:
        """how the box spreads moves. it's on by default and kept on the box, so another app may have changed it"""
        out = native.YscInterpolation()
        self._device._out(lib.anybox_ysc_interpolation, ctypes.byref(out))
        return _ysc_interpolation(out)

    def set_interpolation(self, settings: YscInterpolationSettings) -> None:
        """sets it and reads it back, a box that didn't take it raises `RejectedError`. the box keeps it, anybox
        leaves it set when you close"""
        self._device._out(lib.anybox_ysc_set_interpolation, ctypes.byref(_raw_ysc_settings(settings)))

    def reset_interpolation(self) -> YscInterpolation:
        """puts it back to the box's default and gives you what that is"""
        out = native.YscInterpolation()
        self._device._out(lib.anybox_ysc_reset_interpolation, ctypes.byref(out))
        return _ysc_interpolation(out)

    def serial(self) -> YscSerial:
        """the serial the box gives the mouse behind it"""
        out = ctypes.c_void_p()
        self._device._out(lib.anybox_ysc_serial, ctypes.byref(out))
        return _ysc_serial(out)

    # the end of what bindings/gen makes (ysc_methods)


class Ferrum:
    """what only a Ferrum has. it keeps the device alive, and each call checks what the box can do right now"""

    def __init__(self, device: Device) -> None:
        self._device = device

    # made by bindings/gen from its call table (ferrum_methods), change the table and run cargo run -p anybox-gen
    def catch_xy(self, window: float, with_software: bool = False) -> tuple[int, int]:
        """the mouse's own motion summed over the last `window` (in seconds, whole milliseconds only, up to a
        second), and what software sent too with `with_software`. y is positive down like `move_by` per the
        vendor's first example, its second says the opposite and nobody checked it on a box yet. what a 0 window
        does isn't documented"""
        x, y = ctypes.c_int32(), ctypes.c_int32()
        self._device._out(
            lib.anybox_ferrum_catch_xy,
            _millis(window, "the window"),
            with_software,
            ctypes.byref(x),
            ctypes.byref(y),
        )
        return x.value, y.value

    # the end of what bindings/gen makes (ferrum_methods)
