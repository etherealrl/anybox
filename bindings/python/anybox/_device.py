"""devices, receivers and connecting, over the C handles"""

from __future__ import annotations

import ctypes
import operator
import os
import threading
import weakref
from collections.abc import Callable, Iterator, Sequence
from typing import Any
from contextlib import contextmanager
from dataclasses import dataclass
from types import MappingProxyType

from . import _errors
from . import _native as native
from ._errors import RECV_CLOSED, RECV_TIMEOUT, ClosedError, Error, check, error
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
    Firmware,
    FirmwareKind,
    Info,
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
    _ClickTimingKind,
    _EventKind,
    _IdKind,
)

# a recv with no timeout waits in slices this long, so closing the receiver from another thread is seen this fast
_RECV_SLICE_US = 100_000


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
    return _int(round(seconds * 1_000_000), 0, 2**64 - 1, what)


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

    def close(self) -> None:
        with self._lock:
            if self._closed:
                return
            self._closed = True
            idle = self._calls == 0
        if idle:
            self._release()

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
        received_qpc=event.received_qpc if event.has_received_qpc else None,
        box_time=BoxTime(event.box_time_us, Clock(event.box_clock)) if event.has_box_time else None,
    )


def _unknown(what: str, value: int) -> Error:
    return Error(f"the library sent {what} this package doesn't know ({value}), so update anybox-rs")


def _event(event: native.Event) -> Event:
    try:
        kind = _EventKind(event.kind)
    except ValueError:
        raise _unknown("an event", event.kind) from None
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


def _firmware(raw: native.Info) -> Firmware | None:
    kind = raw.firmware_kind
    if kind == 0:
        return None
    if kind == FirmwareKind.VERSION:
        return Firmware(kind, version=(raw.firmware_major, raw.firmware_minor, raw.firmware_patch))
    if kind == FirmwareKind.BUILD:
        return Firmware(kind, build=raw.firmware_build)
    if kind == FirmwareKind.PROTOCOL:
        return Firmware(kind, protocol=raw.firmware_protocol)
    if kind == FirmwareKind.TEXT:
        return Firmware(kind, text=text(raw.firmware_text))
    raise _unknown("a firmware kind", kind)


def _stable_id(raw: native.Info) -> StableId | None:
    if raw.id_kind == _IdKind.NONE:
        return None
    if raw.id_kind == _IdKind.MAC:
        return StableId(mac=bytes(raw.id_mac))
    if raw.id_kind == _IdKind.USB_SERIAL:
        return StableId(usb_serial=text(raw.id_serial))
    raise _unknown("an id kind", raw.id_kind)


def _info(raw: native.Info) -> Info:
    return Info(
        port=text(raw.port) or "",
        family=Family(raw.family),
        model=text(raw.model),
        firmware=_firmware(raw),
        id=_stable_id(raw),
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


def _list(read: Callable, pointer: int) -> list[int]:
    length = ctypes.c_size_t()
    check(read(pointer, None, 0, ctypes.byref(length)))
    values = (ctypes.c_uint32 * length.value)()
    check(read(pointer, values, length.value, ctypes.byref(length)))
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
            tuple(Button(value) for value in _list(lib.anybox_snapshot_buttons, pointer)),
            tuple(Axis(value) for value in _list(lib.anybox_snapshot_axes, pointer)),
        )
        return Snapshot(info, capabilities)
    finally:
        lib.anybox_snapshot_free(pointer)


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
        it raises `ClosedError` once the device is gone and everything queued was read"""
        if timeout is not None and timeout < 0:
            raise ValueError("a timeout can't be negative")
        left = None if timeout is None else _micros(timeout, "a timeout")
        out = native.Event()
        while True:
            wait = _RECV_SLICE_US if left is None else min(left, _RECV_SLICE_US)
            with self._handle.use() as pointer:
                status = lib.anybox_events_recv(pointer, wait, ctypes.byref(out))
            if status == 0:
                return _event(out)
            if status == RECV_CLOSED:
                raise ClosedError("the device is gone and every event it queued was read")
            if status != RECV_TIMEOUT:
                raise error(status)
            if left is not None:
                left -= wait
                if left <= 0:
                    return None

    def try_recv(self) -> Event | None:
        return self.recv(0)

    def __iter__(self) -> Iterator[Event]:
        while True:
            try:
                event = self.recv()
            except ClosedError:
                return
            if event is not None:
                yield event

    def close(self) -> None:
        """stops this receiver, a `recv` waiting on it from another thread returns within about 100 ms"""
        self._finalizer()

    def __enter__(self) -> Events:
        return self

    def __exit__(self, *_: object) -> None:
        self.close()


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
        self._call(lib.anybox_device_release, _button(button))

    def click(self, button: Button, *, hold: float | None = None, native_only: bool = False) -> None:
        """the box's own click, or one timed on the PC with the device's click timing if the box has none. `hold`
        is in seconds, and `native_only` fails instead of letting anybox time it from the PC"""
        self._call(lib.anybox_device_click_with, _button(button), ctypes.byref(_click(hold, native_only)))

    def press_key(self, key: Key) -> None:
        """holds a key down until `release_key`"""
        self._call(lib.anybox_device_press_key, _key(key))

    def release_key(self, key: Key) -> None:
        self._call(lib.anybox_device_release_key, _key(key))

    def tap_key(self, key: Key, *, hold: float | None = None, native_only: bool = False) -> None:
        """presses and lets go, timed by the box, or on the PC with the device's click timing if the box can't"""
        self._call(lib.anybox_device_tap_key_with, _key(key), ctypes.byref(_click(hold, native_only)))

    def press_media(self, key: MediaKey) -> None:
        self._call(lib.anybox_device_press_media, _media(key))

    def release_media(self, key: MediaKey) -> None:
        self._call(lib.anybox_device_release_media, _media(key))

    def tap_media(self, key: MediaKey, *, hold: float | None = None, native_only: bool = False) -> None:
        self._call(lib.anybox_device_tap_media_with, _media(key), ctypes.byref(_click(hold, native_only)))

    def lock_button(self, button: Button) -> None:
        """keeps the physical button from reaching the PC until `unlock_button`. your own presses still go
        through and its events still come, so you can take the button over"""
        self._call(lib.anybox_device_lock_button, _button(button))

    def unlock_button(self, button: Button) -> None:
        self._call(lib.anybox_device_unlock_button, _button(button))

    def lock_key(self, key: Key) -> None:
        self._call(lib.anybox_device_lock_key, _key(key))

    def unlock_key(self, key: Key) -> None:
        self._call(lib.anybox_device_unlock_key, _key(key))

    def lock_axis(self, axis: Axis, direction: Direction = Direction.BOTH) -> None:
        """keeps physical motion one way or both ways along an axis from reaching the PC, your own moves still go
        through"""
        self._call(lib.anybox_device_lock_axis, _u32(axis, "an axis"), _u32(direction, "a direction"))

    def unlock_axis(self, axis: Axis, direction: Direction = Direction.BOTH) -> None:
        """unlocks that way along the axis and leaves the other way as it was. it also undoes a scale there"""
        self._call(lib.anybox_device_unlock_axis, _u32(axis, "an axis"), _u32(direction, "a direction"))

    def scale_axis(self, axis: Axis, direction: Direction, percent: int) -> None:
        """keeps `percent` of the physical motion along an axis, from -255 to 255. 100 leaves it alone, 0 blocks
        it and -100 inverts it"""
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

    def events(self, kinds: EventKinds = EventKinds.ALL) -> Events:
        """a new receiver for these kinds of input, plus the connection events. the box only streams while at
        least one receiver is alive. a kind this box can't stream gets one `Rejected` right away, except with
        `EventKinds.ALL`, which means everything this box has. just know that a receiver of keys sees every key
        typed on the keyboard behind the box, passwords included"""
        out = ctypes.c_void_p()
        with self._handle.use() as pointer:
            check(lib.anybox_device_events(pointer, _int(kinds, 0, 15, "kinds"), ctypes.byref(out)))
        return Events(out.value, self)  # type: ignore[arg-type]

    def medius(self) -> Any:
        """on a Medius, the medius package's own `Device` for everything only Medius does (clips, transforms,
        rendering, the polling rate and so on), and None on any other box or once this device is closed. it needs
        `pip install anybox-rs[medius]`. it shares this device's connection, and keeping it holds the port open
        after this device is gone. anybox doesn't know about anything you press or lock through it, and its
        `reset()` lets go of and unlocks EVERYTHING, what anybox did too"""
        medius = _medius_package()
        out = ctypes.c_void_p()
        with self._handle.use() as pointer:
            status = lib.anybox_device_medius(pointer, ctypes.byref(out))
        if status in (_errors.UNSUPPORTED, _errors.CLOSED):
            return None
        check(status)
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

    def __exit__(self, *_: object) -> None:
        # a device you already closed inside the block is fine, only the first close says how it went
        if not self._closed:
            self.close()


def _medius_package() -> Any:
    """the medius package, which has to run on this package's library, because a handle only works with the
    library it came from"""
    try:
        import medius
    except ImportError as error:
        raise ImportError(
            f"device.medius() needs the medius package, install it with pip install anybox-rs[medius] ({error})"
        ) from error
    if not os.path.samefile(medius._native.lib._name, lib._name):
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
        for action in self.can:
            options.can |= 1 << _int(action, 0, 63, "an action")
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
