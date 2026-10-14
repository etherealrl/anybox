"""the package over the mock build. the box behavior itself is the Rust crate's, tested there, these check that
every value and handle makes it across ctypes the right way"""

import ctypes
import ctypes.util
import gc
import os
import re
import shutil
import subprocess
import sys
import threading
import time
from pathlib import Path

import pytest

import anybox
from anybox import _errors, _native, _types
from anybox._device import _event, _Handle, _info, _quirk


def defines(workspace: Path) -> dict[str, int]:
    header = (workspace / "bindings/c/include/anybox.h").read_text()
    return {name: int(value) for name, value in re.findall(r"#define (ANYBOX_\w+) (\d+)", header)}


def wait_for(condition, seconds=2.0):
    deadline = time.monotonic() + seconds
    while not condition():
        assert time.monotonic() < deadline, "timed out"
        time.sleep(0.005)


def inputs(events, count):
    """the next `count` button, key, media and motion events, skipping the connection ones"""
    seen = []
    while len(seen) < count:
        event = events.recv(2)
        assert event is not None, "no event came"
        if isinstance(event, (anybox.ButtonEvent, anybox.MotionEvent, anybox.KeyEvent, anybox.MediaEvent)):
            seen.append(event)
    return seen


# the header and the package


def test_the_abi_version_matches_the_header_and_the_library(workspace):
    assert defines(workspace)["ANYBOX_ABI_VERSION"] == anybox.ABI_VERSION
    assert _native.lib.anybox_abi_version() == anybox.ABI_VERSION


def test_a_library_with_another_abi_is_refused():
    with pytest.raises(ImportError, match=r"speaks C ABI 1 and this package needs 2"):
        _native.load(_native.lib._name, abi=2)


def test_a_library_that_isnt_anybox_is_refused():
    other = "kernel32" if sys.platform == "win32" else ctypes.util.find_library("c")
    with pytest.raises(ImportError, match="isn't an anybox library"):
        _native.load(other)


def import_error(env: dict[str, str], package: Path) -> str:
    # a clean environment, so nothing on the PATH can stand in for the library
    env = {"SYSTEMROOT": os.environ.get("SYSTEMROOT", ""), "PATH": "", **env}
    code = "import anybox"
    result = subprocess.run([sys.executable, "-c", code], cwd=package, env=env, capture_output=True, text=True)
    assert result.returncode != 0
    return result.stderr


def test_a_library_that_isnt_there_says_what_to_do(tmp_path):
    package = Path(__file__).parents[1]
    assert "ANYBOX_LIB points at" in import_error({"ANYBOX_LIB": str(tmp_path / "nothing-here.dll")}, package)
    copy = tmp_path / "copy"
    shutil.copytree(
        package / "anybox", copy / "anybox", ignore=shutil.ignore_patterns("__pycache__", "*.dll", "*.so")
    )
    assert "so reinstall anybox-rs" in import_error({}, copy)


@pytest.mark.parametrize(
    "prefix, enum",
    [
        ("ANYBOX_FAMILY_", anybox.Family),
        ("ANYBOX_FERRUM_API_", anybox.FerrumApi),
        ("ANYBOX_BUTTON_", anybox.Button),
        ("ANYBOX_AXIS_", anybox.Axis),
        ("ANYBOX_DIRECTION_", anybox.Direction),
        ("ANYBOX_ACTION_", anybox.Action),
        ("ANYBOX_EXECUTION_", anybox.Execution),
        ("ANYBOX_QUIRK_", anybox.QuirkKind),
        ("ANYBOX_FIRMWARE_", anybox.FirmwareKind),
        ("ANYBOX_DROP_SOURCE_", anybox.DropSource),
        ("ANYBOX_CLOCK_", anybox.Clock),
        ("ANYBOX_EVENTS_", anybox.EventKinds),
        ("ANYBOX_CLICK_TIMING_", _types._ClickTimingKind),
        ("ANYBOX_KEY_", anybox.Key),
        ("ANYBOX_EVENT_", _types._EventKind),
        ("ANYBOX_ID_", _types._IdKind),
        ("ANYBOX_MEDIA_", anybox.MediaKey),
    ],
)
def test_every_header_constant_has_the_same_value_here(workspace, prefix, enum):
    # the "nothing set" values are None in Python, and the extra base is `Button.extra`
    skip = {"ANYBOX_FAMILY_ANY", "ANYBOX_FERRUM_API_NONE", "ANYBOX_FIRMWARE_NONE", "ANYBOX_BUTTON_EXTRA_BASE"}
    expected = {
        name.removeprefix(prefix): value
        for name, value in defines(workspace).items()
        if name.startswith(prefix) and name not in skip
    }
    assert expected
    assert {name: member.value for name, member in enum.__members__.items()} == expected


def test_every_status_has_its_exception(workspace):
    statuses = {
        name.removeprefix("ANYBOX_STATUS_"): value
        for name, value in defines(workspace).items()
        if name.startswith("ANYBOX_STATUS_")
    }
    assert _errors.RECV_TIMEOUT == statuses.pop("RECV_TIMEOUT")
    assert _errors.RECV_CLOSED == statuses.pop("RECV_CLOSED")
    assert statuses.pop("OK") == 0
    names = {cls.status: cls.__name__ for cls in anybox.Error.__subclasses__()}
    for name, value in statuses.items():
        words = name.title().replace("_", "")
        assert names[value] in (f"{words}Error", f"Box{words}Error"), name


def test_the_structs_match_the_committed_layout(workspace):
    layout = (workspace / "bindings/c/tests/layout.txt").read_text().splitlines()
    struct = None
    for line in layout:
        if not line.startswith(" "):
            name, _, size, _, align = line.split()
            struct = _native.STRUCTS[name]
            assert (ctypes.sizeof(struct), ctypes.alignment(struct)) == (int(size), int(align)), name
        else:
            field, offset = line.split()
            assert getattr(struct, field).offset == int(offset), f"{struct.__name__}.{field}"
    assert len([line for line in layout if not line.startswith(" ")]) == len(_native.STRUCTS)


def test_every_header_function_is_bound_or_left_out_on_purpose(workspace):
    header = (workspace / "bindings/c/include/anybox.h").read_text()
    declared = set(re.findall(r"\b(anybox_\w+)\(", header)) - set(re.findall(r"`(anybox_\w+)`", header))
    declared |= set(re.findall(r"^\S.*\b(anybox_\w+)\(", header, re.MULTILINE))
    bound = set(_native._FUNCTIONS) | set(_native._MOCK_FUNCTIONS)
    # connect, the plain click and taps are the options versions with defaults
    left_out = {
        "anybox_connect",
        "anybox_connect_options_default",
        "anybox_device_click",
        "anybox_device_tap_key",
        "anybox_device_tap_media",
    }
    assert declared - bound == left_out
    assert bound <= declared


def test_every_version_number_is_the_same(workspace):
    def version(path):
        return re.search(r'^version = "(.+)"', (workspace / path).read_text(), re.MULTILINE).group(1)

    crate = version("Cargo.toml")
    assert version("bindings/c/Cargo.toml") == crate
    assert version("bindings/python/pyproject.toml") == crate
    assert anybox.__version__ == crate
    assert anybox.version() == crate
    assert anybox.has_mock()


# values


def test_buttons_past_side2_come_back_as_extras():
    assert anybox.Button.extra(0) == 256
    assert anybox.Button(262) is anybox.Button.extra(6)
    assert anybox.Button(262).name == "EXTRA6"
    with pytest.raises(ValueError):
        anybox.Button.extra(256)
    with pytest.raises(ValueError):
        anybox.Button(5)


def test_any_key_has_a_value_even_without_a_name():
    assert anybox.Key(0x64) == 0x64
    assert anybox.Key(0x64) is anybox.Key(0x64)
    assert anybox.Key(0x04) is anybox.Key.A
    assert anybox.MediaKey(0x1234) == 0x1234
    with pytest.raises(ValueError):
        anybox.Key(256)


def test_a_stable_id_is_one_of_the_two():
    assert str(anybox.StableId(mac=bytes([1, 2, 3, 4, 5, 0xAB]))) == "MAC 01:02:03:04:05:ab"
    assert str(anybox.StableId(usb_serial="X1")) == "USB serial X1"
    with pytest.raises(ValueError):
        anybox.StableId()
    with pytest.raises(ValueError):
        anybox.StableId(mac=b"12345", usb_serial="x")
    with pytest.raises(ValueError):
        anybox.StableId(mac=b"12345")


def test_firmware_reads_like_the_rust_one():
    assert str(anybox.Firmware(anybox.FirmwareKind.VERSION, version=(3, 4, 2))) == "3.4.2"
    assert str(anybox.Firmware(anybox.FirmwareKind.BUILD, build=7)) == "build 7"
    assert str(anybox.Firmware(anybox.FirmwareKind.TEXT, text="Oct 10 2026")) == "Oct 10 2026"


def test_connect_options_carry_every_field():
    options = anybox.Connect(
        port="COM9",
        family=anybox.Family.FERRUM,
        ferrum_api=anybox.FerrumApi.LEGACY,
        can=[anybox.Action.MOVE, anybox.Action.LOCK_KEY],
        id=anybox.StableId(mac=bytes(range(6))),
        click_timing=anybox.ClickTiming.fixed(0.05),
    )._options()
    assert options.port == b"COM9"
    assert (options.family, options.ferrum_api) == (3, 2)
    assert options.can == (1 << 1) | (1 << 21)
    assert (options.id_kind, list(options.id_mac), options.id_serial) == (1, list(range(6)), None)
    assert (options.click_timing.kind, options.click_timing.fixed_us) == (2, 50_000)
    serial = anybox.Connect(id=anybox.StableId(usb_serial="AB12"))._options()
    assert (serial.id_kind, serial.id_serial) == (2, b"AB12")
    default = anybox.Connect()._options()
    assert ctypes.string_at(ctypes.addressof(default), ctypes.sizeof(default)) == bytes(ctypes.sizeof(default))


def test_a_port_with_a_nul_is_refused():
    with pytest.raises(ValueError, match="NUL"):
        anybox.Connect(port="CO\0M3").candidates()


def test_a_named_port_is_the_only_candidate_and_its_scan_says_why_nothing_is_there():
    connect = anybox.Connect(port="NOPE99")
    assert connect.candidates() == ["NOPE99"]
    (scanned,) = connect.scan()
    assert scanned.port == "NOPE99"
    assert scanned.found is None
    assert isinstance(scanned.error, anybox.Error)
    assert "NOPE99" in str(scanned.error)
    with pytest.raises(type(scanned.error), match=re.escape(str(scanned.error))):
        connect.open()


def test_numbers_that_dont_fit_are_refused_before_ctypes_wraps_them(mock):
    device, _ = mock
    for call in [
        lambda: device.move_by(32768, 0),
        lambda: device.move_now(0, -32769),
        lambda: device.wheel(40000),
        lambda: device.pan(-40000),
        lambda: device.scale_axis(anybox.Axis.X, anybox.Direction.BOTH, 1 << 16),
        lambda: device.press(-1),
        lambda: device.press_key(256),
        lambda: device.press_media(1 << 16),
        lambda: device.lock_axis(anybox.Axis.X, -1),
        lambda: device.click(anybox.Button.LEFT, hold=-0.1),
        lambda: device.events(16),
    ]:
        with pytest.raises(ValueError):
            call()
    with pytest.raises(TypeError):
        device.move_by(1.5, 0)


# the device


def test_a_snapshot_describes_the_mock(mock):
    device, _ = mock
    snapshot = device.snapshot()
    assert snapshot.info.family == anybox.Family.MEDIUS
    assert snapshot.info.port == "mock box"
    assert snapshot.info.ferrum_api is None
    assert snapshot.capabilities.buttons == tuple(anybox.Button(n) for n in range(5))
    assert snapshot.capabilities.axes == (anybox.Axis.X, anybox.Axis.Y, anybox.Axis.WHEEL)
    assert set(snapshot.capabilities) == set(anybox.Action)
    move = snapshot.capabilities[anybox.Action.MOVE]
    assert move.supported and move.execution == anybox.Execution.NATIVE and move.reason is None
    click = snapshot.capabilities[anybox.Action.CLICK]
    assert click.execution == anybox.Execution.HOST
    assert click.quirks == (
        anybox.Quirk(anybox.QuirkKind.HOST_CLICK_TIMING, click_timing=anybox.ClickTiming.MAKCU),
    )
    keys = snapshot.capabilities[anybox.Action.KEY_PRESS]
    assert not keys.supported and keys.reason
    assert any(
        quirk.kind == anybox.QuirkKind.KEEPS_HELD
        for capability in snapshot.capabilities.values()
        for quirk in capability.quirks
    )


def test_the_click_timing_reaches_the_snapshot():
    device, box = anybox.mock_medius(anybox.ClickTiming.fixed(0.042))
    with device:
        (quirk,) = device.snapshot().capabilities[anybox.Action.CLICK].quirks
        assert quirk.click_timing == anybox.ClickTiming.fixed(0.042)
    box.close()


def test_every_output_call_reaches_the_box(mock):
    device, _ = mock
    device.move_by(3, -4)
    device.move_now(1, 1)
    device.press(anybox.Button.RIGHT)
    device.release(anybox.Button.RIGHT)
    device.click(anybox.Button.LEFT)
    device.click(anybox.Button.LEFT, hold=0.001)
    device.lock_button(anybox.Button.MIDDLE)
    device.unlock_button(anybox.Button.MIDDLE)
    device.lock_axis(anybox.Axis.Y, anybox.Direction.POSITIVE)
    device.unlock_axis(anybox.Axis.Y, anybox.Direction.POSITIVE)
    device.lock_axis(anybox.Axis.X)
    device.unlock_axis(anybox.Axis.X)
    device.scale_axis(anybox.Axis.X, anybox.Direction.BOTH, 50)
    device.wheel(-2)
    device.discard_pending_moves()
    # a mouse with no keyboard behind it and no pan refuses the rest, that's the box answering and not ctypes
    for call in [
        lambda: device.press_key(anybox.Key.A),
        lambda: device.release_key(anybox.Key.A),
        lambda: device.tap_key(anybox.Key.A),
        lambda: device.tap_key(anybox.Key.A, hold=0.001),
        lambda: device.press_media(anybox.MediaKey.MUTE),
        lambda: device.release_media(anybox.MediaKey.MUTE),
        lambda: device.tap_media(anybox.MediaKey.MUTE),
        lambda: device.tap_media(anybox.MediaKey.MUTE, hold=0.001),
        lambda: device.lock_key(anybox.Key.A),
        lambda: device.unlock_key(anybox.Key.A),
        lambda: device.pan(1),
    ]:
        with pytest.raises(anybox.UnsupportedError, match="can't"):
            call()


def test_an_error_keeps_the_librarys_text(mock):
    device, _ = mock
    with pytest.raises(anybox.InvalidArgumentError) as raised:
        device.scale_axis(anybox.Axis.X, anybox.Direction.BOTH, 300)
    assert "300" in str(raised.value)
    assert raised.value.status == 12
    with pytest.raises(anybox.UnsupportedError) as raised:
        device.click(anybox.Button.LEFT, native_only=True)
    assert raised.value.status == 11


def test_a_closed_device_refuses_commands_and_closes_once_like_rust(mock):
    device, _ = mock
    device.close()
    with pytest.raises(anybox.ClosedError):
        device.close()
    with pytest.raises(anybox.ClosedError):
        device.move_by(1, 1)
    assert device.snapshot().info.family == anybox.Family.MEDIUS
    with device.events() as events:
        assert list(events) == []


def test_closing_inside_the_with_block_is_fine():
    device, box = anybox.mock_medius()
    with device:
        device.close()
    with pytest.raises(anybox.ClosedError):
        device.move_by(1, 1)
    box.close()


def test_every_output_call_hands_its_arguments_to_c(mock, monkeypatch):
    # the C side checks these against the Rust calls frame by frame, this checks Python puts them in the right
    # place on the way there
    device, _ = mock
    calls = []

    def record(name):
        def call(pointer, *args):
            values = [arg._obj if isinstance(arg, type(ctypes.byref(ctypes.c_int()))) else arg for arg in args]
            calls.append(
                (
                    name,
                    [
                        (v.hold_us, v.has_hold, v.native_only) if isinstance(v, _native.Click) else v
                        for v in values
                    ],
                )
            )
            return 0

        return call

    names = [
        name.removeprefix("anybox_device_") for name in _native._FUNCTIONS if name.startswith("anybox_device_")
    ]
    for name in names:
        monkeypatch.setattr(_native.lib, f"anybox_device_{name}", record(name))
    device.move_by(3, -4)
    device.move_now(-5, 6)
    device.press(anybox.Button.RIGHT)
    device.release(anybox.Button.extra(7))
    device.click(anybox.Button.MIDDLE)
    device.click(anybox.Button.LEFT, hold=0.0125, native_only=True)
    device.press_key(anybox.Key.B)
    device.release_key(anybox.Key(0x64))
    device.tap_key(anybox.Key.C, hold=0.002)
    device.press_media(anybox.MediaKey.MUTE)
    device.release_media(anybox.MediaKey.VOLUME_UP)
    device.tap_media(anybox.MediaKey.PLAY, native_only=True)
    device.lock_button(anybox.Button.SIDE1)
    device.unlock_button(anybox.Button.SIDE2)
    device.lock_key(anybox.Key.D)
    device.unlock_key(anybox.Key.E)
    device.lock_axis(anybox.Axis.Y, anybox.Direction.NEGATIVE)
    device.unlock_axis(anybox.Axis.WHEEL)
    device.scale_axis(anybox.Axis.PAN, anybox.Direction.POSITIVE, -100)
    device.wheel(-2)
    device.pan(9)
    device.discard_pending_moves()
    assert calls == [
        ("move_by", [3, -4]),
        ("move_now", [-5, 6]),
        ("press", [1]),
        ("release", [263]),
        ("click_with", [2, (0, False, False)]),
        ("click_with", [0, (12_500, True, True)]),
        ("press_key", [0x05]),
        ("release_key", [0x64]),
        ("tap_key_with", [0x06, (2_000, True, False)]),
        ("press_media", [0xE2]),
        ("release_media", [0xE9]),
        ("tap_media_with", [0xB0, (0, False, True)]),
        ("lock_button", [3]),
        ("unlock_button", [4]),
        ("lock_key", [0x07]),
        ("unlock_key", [0x08]),
        ("lock_axis", [1, 2]),
        ("unlock_axis", [2, 0]),
        ("scale_axis", [3, 1, -100]),
        ("wheel", [-2]),
        ("pan", [9]),
        ("discard_pending_moves", []),
    ]


def test_a_click_timing_has_a_kind_this_package_knows_and_a_hold_only_when_fixed():
    for timing in [
        anybox.ClickTiming(2**32),
        anybox.ClickTiming(_types._ClickTimingKind.FIXED),
        anybox.ClickTiming(_types._ClickTimingKind.MAKCU, 0.05),
    ]:
        with pytest.raises(ValueError):
            anybox.mock_medius(timing)


def test_the_context_manager_closes_the_device():
    device, box = anybox.mock_medius()
    with device as same:
        assert same is device
    with pytest.raises(anybox.ClosedError):
        device.move_by(1, 1)
    box.close()


def test_a_dropped_device_gets_freed():
    device, box = anybox.mock_medius()
    handle = device._handle
    del device
    gc.collect()
    assert handle.closed and handle._pointer is None
    box.close()


# events


def test_pushed_input_comes_out_as_events(mock):
    device, box = mock
    with device.events() as events:
        wait_for(box.subscribed)
        box.push_motion(1, 1000, 3, -4, 1, 0)
        box.push_buttons(2, 2000, True, 1 | (1 << 6))
        box.push_buttons(3, 3000, False, 0)
        seen = inputs(events, 5)
    motion = next(event for event in seen if isinstance(event, anybox.MotionEvent))
    assert (motion.dx, motion.dy, motion.wheel, motion.pan) == (3, -4, 1, 0)
    assert motion.at.box_time == anybox.BoxTime(1000, anybox.Clock.MEDIUS_HOST_CHIP)
    assert motion.at.received_since_open_ns >= motion.at.since_open_ns > 0
    assert (motion.at.received_qpc is not None) == (sys.platform == "win32")
    buttons = [(event.button, event.pressed) for event in seen if isinstance(event, anybox.ButtonEvent)]
    assert sorted(buttons) == sorted(
        [
            (anybox.Button.LEFT, True),
            (anybox.Button.extra(6), True),
            (anybox.Button.LEFT, False),
            (anybox.Button.extra(6), False),
        ]
    )


def test_every_event_kind_comes_across_with_its_fields():
    # the mock can only push input, so the rest get built the way the C side fills them
    stamp = dict(since_open_ns=5, received_since_open_ns=6, received_qpc=7, has_received_qpc=True)
    clock = dict(has_box_time=True, box_clock=1, box_time_us=9)
    at = anybox.Stamp(5, 6, 7, anybox.BoxTime(9, anybox.Clock.MEDIUS_DEVICE_CHIP))
    for raw, event in [
        (
            dict(kind=0, button=257, pressed=True, **stamp, **clock),
            anybox.ButtonEvent(anybox.Button.extra(1), True, at),
        ),
        (dict(kind=1, key=0x04, pressed=False, **stamp, **clock), anybox.KeyEvent(anybox.Key.A, False, at)),
        (
            dict(kind=2, media=0xE2, pressed=True, **stamp, **clock),
            anybox.MediaEvent(anybox.MediaKey.MUTE, True, at),
        ),
        (dict(kind=3, dx=-1, dy=2, wheel=-3, pan=4, **stamp, **clock), anybox.MotionEvent(-1, 2, -3, 4, at)),
        (dict(kind=4, count=12, source=1), anybox.Dropped(12, anybox.DropSource.HOST)),
        (dict(kind=5), anybox.SessionReset()),
        (dict(kind=6, action=13, count=3), anybox.Rejected(anybox.Action.KEY_PRESS, 3)),
        (dict(kind=7), anybox.Disconnected()),
        (dict(kind=8), anybox.Reconnected()),
        (dict(kind=9), anybox.Changed()),
    ]:
        assert _event(_native.Event(**raw)) == event
    assert _event(_native.Event(kind=3, since_open_ns=1, received_since_open_ns=2)).at == anybox.Stamp(1, 2)
    with pytest.raises(anybox.Error, match="update anybox-rs"):
        _event(_native.Event(kind=10))


def test_every_info_field_comes_across():
    raw = _native.Info(
        port=b"COM3",
        family=2,
        model=b"V4",
        firmware_kind=2,
        firmware_build=4094,
        id_kind=2,
        id_serial=b"SN1",
        ferrum_api=1,
        has_peripheral=True,
        peripheral_vid=0x046D,
        peripheral_pid=0xC547,
    )
    assert _info(raw) == anybox.Info(
        "COM3",
        anybox.Family.MAKCU_V4,
        "V4",
        anybox.Firmware(anybox.FirmwareKind.BUILD, build=4094),
        anybox.StableId(usb_serial="SN1"),
        anybox.FerrumApi.SOFTWARE,
        anybox.UsbIds(0x046D, 0xC547),
    )
    for fields, firmware in [
        (dict(firmware_kind=1, firmware_major=3, firmware_minor=4, firmware_patch=5), "3.4.5"),
        (dict(firmware_kind=3, firmware_protocol=9), "protocol 9"),
        (dict(firmware_kind=4, firmware_text=b"Oct 10 2026"), "Oct 10 2026"),
    ]:
        assert str(_info(_native.Info(family=1, **fields)).firmware) == firmware
    bare = _info(_native.Info(family=1))
    assert (bare.firmware, bare.id, bare.model, bare.ferrum_api, bare.peripheral) == (None,) * 5
    mac = _info(_native.Info(family=1, id_kind=1, id_mac=(1, 2, 3, 4, 5, 6)))
    assert mac.id == anybox.StableId(mac=bytes([1, 2, 3, 4, 5, 6]))
    for fields in [dict(firmware_kind=5), dict(id_kind=3)]:
        with pytest.raises(anybox.Error, match="update anybox-rs"):
            _info(_native.Info(family=1, **fields))


def test_every_quirk_comes_across_with_its_fields():
    kinds = anybox.QuirkKind
    fixed = _native.ClickTiming(kind=2, fixed_us=40_000)
    seen = set()
    for raw, quirk in [
        (
            dict(kind=0, min_us=125_000, max_us=175_000),
            anybox.Quirk(kinds.FORCED_UP_ON_RELEASE, min=0.125, max=0.175),
        ),
        (dict(kind=6, min_us=35_000, max_us=75_000), anybox.Quirk(kinds.BOX_CLICK_TIMING, min=0.035, max=0.075)),
        (dict(kind=5, max_us=1_000_000), anybox.Quirk(kinds.MAX_HOLD, max=1.0)),
        (dict(kind=3, count=1), anybox.Quirk(kinds.NOTCHES_PER_COMMAND, count=1)),
        (dict(kind=9, count=6), anybox.Quirk(kinds.MAX_KEYS_HELD, count=6)),
        (
            dict(kind=7, click_timing=fixed),
            anybox.Quirk(kinds.HOST_CLICK_TIMING, click_timing=anybox.ClickTiming.fixed(0.04)),
        ),
        (dict(kind=1), anybox.Quirk(kinds.PHYSICAL_OVERRIDE)),
        (dict(kind=2), anybox.Quirk(kinds.WHOLE_AXIS_ONLY)),
        (dict(kind=4), anybox.Quirk(kinds.FIXED_CLICK_TIMING)),
        (dict(kind=8), anybox.Quirk(kinds.KEEPS_HELD)),
        (dict(kind=10), anybox.Quirk(kinds.KEEPS_LOCKED)),
    ]:
        assert _quirk(_native.Quirk(**raw)) == quirk
        seen.add(quirk.kind)
    assert seen == set(kinds)


def test_a_kind_the_box_cant_stream_is_rejected_on_its_own_receiver(mock):
    device, _ = mock
    with device.events(anybox.EventKinds.KEYS) as events:
        assert events.recv(2) == anybox.Rejected(anybox.Action.KEY_EVENTS, 1)


def test_recv_times_out_tries_and_ends(mock):
    device, _ = mock
    events = device.events(anybox.EventKinds.BUTTONS)
    assert events.try_recv() is None
    started = time.monotonic()
    assert events.recv(0.15) is None
    assert time.monotonic() - started >= 0.14
    device.close()
    assert list(events) == []
    with pytest.raises(anybox.ClosedError):
        events.recv()
    with pytest.raises(ValueError):
        events.recv(-1)


def test_closing_the_device_from_another_thread_ends_a_reader(mock):
    device, _ = mock
    events = device.events(anybox.EventKinds.BUTTONS)
    done = []
    reader = threading.Thread(target=lambda: done.append(list(events)))
    reader.start()
    time.sleep(0.2)
    device.close()
    reader.join(3)
    assert not reader.is_alive()
    assert done == [[]]


def test_closing_a_receiver_from_another_thread_ends_its_reader_and_frees_it_after(mock):
    device, _ = mock
    events = device.events(anybox.EventKinds.BUTTONS)
    raised = []

    def read():
        try:
            events.recv()
        except anybox.ClosedError as error:
            raised.append(error)

    reader = threading.Thread(target=read)
    reader.start()
    time.sleep(0.2)
    started = time.monotonic()
    events.close()
    reader.join(3)
    assert not reader.is_alive() and raised
    assert time.monotonic() - started < 0.5
    assert events._handle._pointer is None
    device.move_by(1, 1)


def test_a_receiver_keeps_its_device_open():
    device, box = anybox.mock_medius()
    events = device.events()
    handle = device._handle
    del device
    gc.collect()
    assert not handle.closed
    events.close()
    del events
    gc.collect()
    assert handle.closed
    box.close()


def test_two_threads_reading_one_receiver_split_the_events(mock):
    device, box = mock
    events = device.events(anybox.EventKinds.MOTION)
    wait_for(box.subscribed)
    got = []
    lock = threading.Lock()

    def read():
        for event in events:
            if isinstance(event, anybox.MotionEvent):
                with lock:
                    got.append(event.dx)

    readers = [threading.Thread(target=read) for _ in range(2)]
    for reader in readers:
        reader.start()
    for seq in range(1, 41):
        box.push_motion(seq, seq * 1000, seq, 0)
    wait_for(lambda: len(got) == 40)
    device.close()
    for reader in readers:
        reader.join(3)
    assert sorted(got) == list(range(1, 41))


# the medius package


def test_on_a_medius_the_medius_package_shares_the_connection(mock):
    device, _ = mock
    box = device.medius()
    import medius

    assert isinstance(box, medius.Device)
    assert os.path.samefile(medius._native.lib._name, _native.lib._name)
    assert box.query_version() is not None
    box.close()
    device.close()
    assert device.medius() is None


def test_the_medius_extra_pins_the_medius_this_library_has_inside(workspace):
    cargo = (workspace / "Cargo.toml").read_text()
    pyproject = (workspace / "bindings/python/pyproject.toml").read_text()
    pinned = re.search(r'^medius = \{ version = "=(.+?)"', cargo, re.MULTILINE).group(1)
    assert f'medius = ["medius=={pinned}"]' in pyproject


def test_a_medius_package_with_its_own_library_is_refused():
    code = "import medius, anybox; device, _ = anybox.mock_medius(); device.medius()"
    env = {**os.environ}
    env.pop("MEDIUS_LIB", None)
    result = subprocess.run(
        [sys.executable, "-c", code], cwd=Path(__file__).parents[1], env=env, capture_output=True, text=True
    )
    assert result.returncode != 0
    assert "import anybox before medius" in result.stderr


# handles


def test_a_handle_is_freed_by_the_last_call_after_it_closed():
    freed = []
    handle = _Handle(7, freed.append, "gone")
    with handle.use() as pointer:
        assert pointer == 7
        handle.close()
        assert freed == []
        with pytest.raises(anybox.ClosedError):
            with handle.use():
                pass
    assert freed == [7]
    handle.close()
    assert freed == [7]


def test_calls_from_many_threads_at_once_work(mock):
    device, _ = mock
    failures = []

    def work():
        try:
            for _ in range(50):
                device.move_by(1, 0)
                device.snapshot()
        except Exception as error:
            failures.append(error)

    threads = [threading.Thread(target=work) for _ in range(4)]
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join()
    assert failures == []


def test_an_error_on_one_thread_keeps_its_own_text(mock):
    device, _ = mock
    texts = {}

    def fail(name, call):
        try:
            call()
        except anybox.Error as error:
            texts[name] = str(error)

    one = threading.Thread(target=fail, args=("scale", lambda: device.scale_axis(0, 0, 300)))
    two = threading.Thread(target=fail, args=("key", lambda: device.press_key(anybox.Key.A)))
    one.start()
    two.start()
    one.join()
    two.join()
    assert "300" in texts["scale"] and "key" in texts["key"]


def test_the_mock_refuses_after_it_closed():
    device, box = anybox.mock_medius()
    box.close()
    with pytest.raises(anybox.ClosedError):
        box.push_motion(1, 1, 1, 1)
    device.close()
