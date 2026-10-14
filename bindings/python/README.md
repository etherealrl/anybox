# anybox

one API for the most popular passthrough boxes, from Python. it's the [anybox](https://github.com/etherealrl/anybox) Rust library through its C ABI, so it does exactly what the Rust one does, and the [README there](https://github.com/etherealrl/anybox#readme) has which boxes it supports and how each one behaves

```sh
pip install anybox-rs
```

it's still `import anybox` in your code. the wheel has the library inside, so you don't need Rust. it's Windows and Linux on x64 and Python 3.11 or newer for now, if you need something else open an issue. on Linux your user has to be allowed to open serial ports, the `dialout` group on most distros (`uucp` on Arch)

```python
import anybox

with anybox.connect() as device:
    print(device.snapshot().info)
    device.move_by(100, 0)
    device.click(anybox.Button.LEFT)
    device.tap_key(anybox.Key.A)
```

`anybox.connect()` takes the first box it finds, `anybox.Connect(port="COM18").open()` or `anybox.Connect(family=anybox.Family.MEDIUS).open()` the one you want, and `anybox.scan()` gives every port with what's on it, which is what a box picker wants. `anybox.candidates()` is the cheap one to call every second. leave the `with` or call `close()` so anybox lets go of what you held and unlocks what you locked

if a box can't do something you get an `anybox.UnsupportedError` saying why, and `device.snapshot().capabilities[anybox.Action.KEY_TAP]` tells you before you try. every error is an `anybox.Error` with the library's own text

what happens on the box comes in as events:

```python
import anybox

with anybox.connect() as device:
    for event in device.events(anybox.EventKinds.BUTTONS):
        match event:
            case anybox.ButtonEvent(button=anybox.Button.SIDE1, pressed=True):
                print("side button down")
            case anybox.Disconnected():
                print("the box is gone, anybox keeps looking for it")
            case anybox.Reconnected():
                print("it's back")
```

`events.recv(timeout)` gives you one at a time instead, `None` when nothing came in time. iterating ends once the device is closed and everything queued was read. just so you know, a receiver of keys sees every key typed on the keyboard behind the box, passwords included

durations like a click's `hold` are seconds, and an event's stamp is nanoseconds since the device was opened. a number that doesn't fit what the box takes (a move of 40000 for example) is a `ValueError` and never gets cut short on its way

what only one box can do stays reachable through that box's own API. on a Medius, `device.medius()` gives you the [medius](https://pypi.org/project/medius/) package's own device for clips, transforms, rendering and the rest, and None on every other box. it needs the extra:

```sh
pip install anybox-rs[medius]
```

```python
import anybox
from medius import RenderMode

with anybox.connect() as device:
    if (medius := device.medius()) is not None:
        medius.set_render(RenderMode.DESPIKED, False)
```

it shares anybox's connection, so import anybox BEFORE medius, otherwise medius loads its own library and can't use it. anybox doesn't know about anything you press or lock through it

## license

MPL-2.0, the wheel has the license, the notice and the licenses of everything built into the library. made by ethereal
