"""the conformance suite's Python runner: every scenario in bindings/conformance/scenarios.json through this
package and the mock, held to what the Rust crate itself did with it, which the capi crate's tests write to
target/conformance/expected.json. bindings/c/src/conformance.rs says what a scenario is"""

import json
import re
import time
from pathlib import Path

import pytest

import anybox

_ROOT = Path(__file__).resolve().parents[3]
_EXPECTED = _ROOT / "target" / "conformance" / "expected.json"
_HEADER = {
    name: int(number)
    for name, number in re.findall(
        r"^#define ANYBOX_(\w+) (-?\d+)$", (_ROOT / "bindings/c/include/anybox.h").read_text(), re.MULTILINE
    )
}

# the Rust calls this package folds into another method, with the `Click` argument as keywords
_FOLDED = {
    # made by bindings/gen from its call table (folded), change the table and run cargo run -p anybox-gen
    "click_with": "click",
    "tap_key_with": "tap_key",
    "tap_media_with": "tap_media",
    # the end of what bindings/gen makes (folded)
}

# the anybox.h prefix of each value type, a value compares by its name there, so the C ABI's number for it is
# checked against the Rust name the recorder wrote
_PREFIXES = {
    anybox.Button: "BUTTON",
    anybox.Axis: "AXIS",
    anybox.Direction: "DIRECTION",
    anybox.Action: "ACTION",
    anybox.Family: "FAMILY",
    anybox.FerrumApi: "FERRUM_API",
    anybox.Clock: "CLOCK",
    anybox.DropSource: "DROP_SOURCE",
    anybox.Execution: "EXECUTION",
    anybox.QuirkKind: "QUIRK",
    anybox.IoKind: "IO_KIND",
    type(anybox.ClickTiming.MAKCU.kind): "CLICK_TIMING",
}

# what comes in after the events a step waits for, so an extra one shows up instead of being left in the queue
_QUIET = 0.15


def _keepalive(kind: int, payload: bytes) -> bool:
    """what medius' own keepalive sends every 500 ms while something's held, a stats query and the subscriptions
    again, which no call sends, so the recorder and every runner leave them out"""
    return (kind == 0x05 and payload == b"\x05") or kind == 0x0B


def _name(prefix: str, number: int) -> str:
    names = [
        name[len(prefix) + 1 :]
        for name, value in _HEADER.items()
        if name.startswith(f"{prefix}_") and value == number
    ]
    assert len(names) == 1, f"anybox.h has {names} for {prefix} {number}"
    return names[0]


def _expected() -> list[dict]:
    if not _EXPECTED.exists():
        pytest.fail(f"there's no {_EXPECTED}, run the capi crate's tests first, they record it")
    return json.loads(_EXPECTED.read_text())


def _number(prefix: str, value) -> int:
    if isinstance(value, str):
        return _HEADER[f"{prefix}_{value}"]
    if isinstance(value, dict):
        return _HEADER["BUTTON_EXTRA_BASE"] + value["Extra"]
    return value


def _arg(value: dict):
    ((kind, raw),) = value.items()
    if kind == "i16":
        return raw
    if kind == "Button":
        return anybox.Button(_number("BUTTON", raw))
    if kind == "Key":
        return anybox.Key(_number("KEY", raw))
    if kind == "MediaKey":
        return anybox.MediaKey(_number("MEDIA", raw))
    if kind == "Text":
        return raw
    if kind == "Keys":
        return [anybox.Key(_number("KEY", key)) for key in raw]
    if kind == "Axis":
        return anybox.Axis(_number("AXIS", raw))
    if kind == "Direction":
        return anybox.Direction(_number("DIRECTION", raw))
    raise AssertionError(f"the runner doesn't know the value {value}")


def _text(value: dict) -> str:
    """the text of a value given like a call's argument, or of a firmware or an id given the way a snapshot
    writes them"""
    ((kind, raw),) = value.items()
    if kind == "Firmware":
        ((field, held),) = raw.items()
        kinds = anybox.FirmwareKind
        if field == "version":
            return str(anybox.Firmware(kinds.VERSION, version=tuple(held)))
        return str(anybox.Firmware(getattr(kinds, field.upper()), **{field: held}))
    if kind == "StableId":
        return str(anybox.StableId(mac=bytes.fromhex(raw["mac"])) if "mac" in raw else anybox.StableId(**raw))
    if kind in ("Action", "Family", "FerrumApi"):
        prefix = {"Action": "ACTION", "Family": "FAMILY", "FerrumApi": "FERRUM_API"}[kind]
        return str(getattr(anybox, kind)(_number(prefix, raw)))
    return str(_arg(value))


def _call(device: anybox.Device, name: str, args: list[dict]) -> None:
    if name in _FOLDED:
        click = args[1]["Click"]
        hold = click["hold_us"] / 1_000_000 if "hold_us" in click else None
        getattr(device, _FOLDED[name])(_arg(args[0]), hold=hold, native_only=click.get("native_only", False))
    elif args and "Keys" in args[0]:
        getattr(device, name)(*_arg(args[0]))
    else:
        getattr(device, name)(*(_arg(arg) for arg in args))


def _firmware(firmware: anybox.Firmware):
    if firmware.version is not None:
        return {"version": list(firmware.version)}
    for field in ("build", "protocol", "text"):
        if getattr(firmware, field) is not None:
            return {field: getattr(firmware, field)}
    raise AssertionError(f"a firmware with nothing in it: {firmware}")


def _id(id: anybox.StableId):
    return {"mac": id.mac.hex()} if id.mac is not None else {"usb_serial": id.usb_serial}


def _button(button: anybox.Button):
    extra = _HEADER["BUTTON_EXTRA_BASE"]
    return {"Extra": int(button) - extra} if button >= extra else _name("BUTTON", button)


def _value(value):
    """one field's value in the suite's form"""
    if value is None or isinstance(value, (bool, str)):
        return value
    if isinstance(value, anybox.Button):
        return _button(value)
    if type(value) in _PREFIXES:
        return _name(_PREFIXES[type(value)], value)
    if isinstance(value, int):
        return int(value)
    if isinstance(value, anybox.Firmware):
        return _firmware(value)
    if isinstance(value, anybox.StableId):
        return _id(value)
    if isinstance(value, anybox.Stamp):
        box = value.box_time
        return {"box_time": None if box is None else {"micros": box.micros, "clock": _value(box.clock)}}
    if isinstance(value, tuple) and value and isinstance(value[0], tuple):
        # NoBoxCanError's boxes, each the box's snapshot and what it's missing
        return [
            {"port": box.info.port, "missing": sorted(_value(action) for action in missing)}
            for box, missing in value
        ]
    if isinstance(value, tuple):
        return [_value(item) for item in value]
    raise AssertionError(f"the runner doesn't know the value {value!r}")


def _error(error: anybox.Error, expected: dict) -> dict:
    fields = {name: _value(getattr(error, name)) for name in expected["error"]["fields"]}
    return {"error": {"status": _name("STATUS", error.status), "text": str(error), "fields": fields}}


def _event(event, expected: dict) -> dict:
    name = type(event).__name__.removesuffix("Event")
    return {"event": name, "fields": {field: _value(getattr(event, field)) for field in expected["fields"]}}


def _seconds_us(seconds: float | None) -> int:
    return 0 if seconds is None else round(seconds * 1_000_000)


def _snapshot(snapshot: anybox.Snapshot, expected: dict) -> dict:
    info = snapshot.info
    capabilities = []
    for want in expected["snapshot"]["capabilities"]:
        capability = snapshot.capabilities[anybox.Action(_HEADER[f"ACTION_{want['action']}"])]
        quirks = [
            {
                "kind": _value(quirk.kind),
                "min_us": _seconds_us(quirk.min),
                "max_us": _seconds_us(quirk.max),
                "count": quirk.count or 0,
                "click_timing": {
                    "kind": None if quirk.click_timing is None else _value(quirk.click_timing.kind),
                    "fixed_us": 0 if quirk.click_timing is None else _seconds_us(quirk.click_timing.hold),
                },
            }
            for quirk in capability.quirks
        ]
        capabilities.append(
            {
                "action": want["action"],
                "execution": _value(capability.execution),
                "reason": capability.reason,
                "quirks": quirks,
            }
        )
    return {
        "snapshot": {
            "info": {
                "port": info.port,
                "family": _value(info.family),
                "model": info.model,
                "firmware": None if info.firmware is None else _firmware(info.firmware),
                "id": None if info.id is None else _id(info.id),
                "ferrum_api": _value(info.ferrum_api),
                "peripheral": None
                if info.peripheral is None
                else {"vid": info.peripheral.vid, "pid": info.peripheral.pid},
            },
            "capabilities": capabilities,
            "buttons": [_button(button) for button in snapshot.capabilities.buttons],
            "axes": [_value(axis) for axis in snapshot.capabilities.axes],
        }
    }


def _push(box: anybox.Mock, step: dict) -> None:
    kind = step["push"]
    if kind == "buttons":
        box.push_buttons(step["seq"], step["ts_us"], step["pressed"], step["held"])
    elif kind == "keys":
        box.push_keys(
            step["seq"], step["ts_us"], step["pressed"], [anybox.Key(_number("KEY", k)) for k in step["held"]]
        )
    elif kind == "media":
        held = [anybox.MediaKey(_number("MEDIA", key)) for key in step["held"]]
        box.push_media(step["seq"], step["ts_us"], step["pressed"], held)
    elif kind == "motion":
        box.push_motion(step["seq"], step["ts_us"], step["dx"], step["dy"], step["wheel"], step["pan"])
    elif kind == "link_lost":
        box.link_lost()
    elif kind == "restart":
        box.restart()
    else:
        raise AssertionError(f"the mock can't push {kind}")


def _subscribed(box: anybox.Mock, kinds: anybox.EventKinds) -> None:
    deadline = time.monotonic() + 5
    while not box.subscribed(kinds):
        assert time.monotonic() < deadline, "the receiver never subscribed"
        time.sleep(0.01)


def _recv(events, box: anybox.Mock, expected: dict, tick: bool) -> dict:
    wanted = expected["events"]
    got = []
    deadline = time.monotonic() + 10
    quiet_from = None
    while True:
        if len(got) >= len(wanted):
            quiet_from = quiet_from or time.monotonic()
            if time.monotonic() - quiet_from >= _QUIET:
                break
        assert time.monotonic() < deadline, f"only {len(got)} of {len(wanted)} events came"
        # no ticks while it waits for extras, so only what the scenario set off can come
        if tick and len(got) < len(wanted):
            box.tick()
        event = events.recv(0.05)
        if event is not None:
            # one Rust didn't send is only its kind, it has nothing to read its fields by
            got.append(
                _event(event, wanted[len(got)]) if len(got) < len(wanted) else {"extra": type(event).__name__}
            )
    return {"events": got}


def _ms(seconds: float | None) -> int | None:
    return None if seconds is None else round(seconds * 1000)


def _interpolation(value: anybox.Interpolation) -> dict:
    return {"Interpolation": "AUTO" if value.automatic else value.percentage}


def _ysc(value: anybox.YscInterpolation) -> dict:
    interval = value.measured_interval
    return {
        "enabled": value.enabled,
        "profile": int(value.profile),
        "window_ms": _ms(value.window),
        "max_window_ms": _ms(value.max_window),
        "measured_interval_us": None if interval is None else round(interval * 1_000_000),
    }


def _extension(device: anybox.Device, step: dict) -> dict:
    """one call only one family has, `{"none": null}` when the device isn't that family"""
    family, call = step["extension"].split(".")
    handle = getattr(device, family)()
    if handle is None:
        return {"none": None}
    args = [next(iter(arg.values())) for arg in step["args"]]
    try:
        if call == "set_interpolation" and family == "makcu":
            (value,) = args
            handle.set_interpolation(
                anybox.Interpolation.auto() if value == "AUTO" else anybox.Interpolation.percent(value)
            )
            value = None
        elif call == "set_interpolation":
            (settings,) = args
            handle.set_interpolation(
                anybox.YscInterpolationSettings(
                    settings["enabled"],
                    anybox.YscProfile(settings["profile"]),
                    settings["window_ms"] / 1000,
                    settings["max_window_ms"] / 1000,
                )
            )
            value = None
        elif call == "set_autorelease":
            (ms,) = args
            value = handle.set_autorelease(None if ms is None else ms / 1000)
        elif call == "catch_xy":
            ms, with_software = args
            value = list(handle.catch_xy(ms / 1000, with_software))
        else:
            value = getattr(handle, call)()
            if isinstance(value, anybox.Interpolation):
                value = _interpolation(value)
            elif isinstance(value, anybox.YscInterpolation):
                value = _ysc(value)
            elif isinstance(value, anybox.YscSerial):
                value = {"custom": value.custom, "physical": value.physical, "active": value.active}
            elif call == "autorelease":
                value = _ms(value)
    except anybox.Error as error:
        expected = step["outcome"]
        return _error(error, expected) if "error" in expected else {"error": str(error)}
    return {"ok": value}


def _run_family(scenario: dict) -> list[dict]:
    open_mock = {"makcu": anybox.mock_makcu, "ysc": anybox.mock_ysc, "ferrum": anybox.mock_ferrum}
    device = open_mock[scenario["box"]](anybox.ClickTiming.fixed(0.005))
    try:
        return [{**step, "outcome": _extension(device, step)} for step in scenario["steps"]]
    finally:
        device.close()


def _run(scenario: dict) -> list[dict]:
    if "box" in scenario:
        return _run_family(scenario)
    device, box = anybox.mock_medius_manual(anybox.ClickTiming.fixed(0.005), keyboard=scenario["keyboard"])
    events = None
    try:
        if "events" in scenario:
            kinds = anybox.EventKinds(_HEADER[f"EVENTS_{scenario['events']}"])
            events = device.events(kinds)
            # a mock with no keyboard has no keys or media to subscribe
            if not scenario["keyboard"]:
                kinds &= anybox.EventKinds.BUTTONS | anybox.EventKinds.MOTION
            _subscribed(box, kinds)
        steps = []
        for step in scenario["steps"]:
            out = {key: value for key, value in step.items() if key not in ("outcome", "frames")}
            expected = step["outcome"]
            if "call" in step:
                box.clear_recorded()
                started = time.perf_counter()
                try:
                    _call(device, step["call"], step["args"])
                    outcome = {"ok": None}
                except anybox.Error as error:
                    outcome = _error(error, expected) if "error" in expected else {"error": str(error)}
                # a hold on a box that can't time it is a wait on the PC, which no frame shows
                if "takes_at_least_us" in step:
                    took = time.perf_counter() - started
                    assert took >= step["takes_at_least_us"] / 1_000_000, f"{step} only took {took} s"
                out["frames"] = [
                    [kind, payload.hex()] for kind, payload in box.recorded() if not _keepalive(kind, payload)
                ]
            elif "extension" in step:
                outcome = _extension(device, step)
            elif "snapshot" in step:
                outcome = _snapshot(device.snapshot(), expected)
            elif "push" in step:
                _push(box, step)
                outcome = None
            elif "wait_us" in step:
                time.sleep(step["wait_us"] / 1_000_000)
                outcome = None
            elif "recv" in step:
                outcome = _recv(events, box, expected, tick=False)
            elif "tick_until" in step:
                outcome = _recv(events, box, expected, tick=True)
            elif "example_events" in step:
                anybox.example_events(device)
                outcome = None
            elif "text" in step:
                outcome = {"text": _text(step["text"])}
            elif "example" in step:
                status = _HEADER[f"STATUS_{step['example']}"]
                kind = next(kind for kind in _errors() if kind.status == status)
                outcome = _error(anybox.example_error(kind), expected)
            else:
                raise AssertionError(f"the runner doesn't know the step {step}")
            out["outcome"] = outcome
            steps.append(out)
        return steps
    finally:
        if events is not None:
            events.close()
        device.close()
        box.close()


def _errors() -> list[type[anybox.Error]]:
    found = []
    pending = [anybox.Error]
    while pending:
        kind = pending.pop()
        found.append(kind)
        pending.extend(kind.__subclasses__())
    return found


def _names() -> list[str]:
    return [
        scenario["name"] for scenario in json.loads((_ROOT / "bindings/conformance/scenarios.json").read_text())
    ]


@pytest.mark.parametrize("name", _names())
def test_the_package_does_what_the_rust_crate_does(name):
    (expected,) = [scenario for scenario in _expected() if scenario["name"] == name]
    actual = _run(expected)
    for index, (want, got) in enumerate(zip(expected["steps"], actual, strict=True)):
        assert got == want, f"step {index} of {name!r} differs from Rust"
