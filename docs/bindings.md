# bindings

anybox's core is the Rust crate, and every other language goes through its C library. each binding is the same API with the names of its language, and it's meant to do exactly what the Rust crate does, with the same boxes, the same errors with the same fields, the same events and stamps, and the same `device.medius()` on a Medius. if one of them acts differently from Rust, that's a bug, so open an issue

they're only Windows and Linux on x64 for now. if you need another platform or another language, open an issue too

## C and C++

every release has an `anybox-capi-<target>.tar.gz` for Windows and Linux with `anybox.h`, `anybox.hpp` and the `anybox_medius.hpp` it brings in, `medius.h`, the library, shared and static (the static one also needs the libraries in `native-static-libs.txt`, and the one that isn't part of Windows is in there too), and the licenses of everything built into it. you can also build it yourself with `cargo build -p anybox-capi --release`

```c
#include <stdio.h>
#include "anybox.h"

int main(void) {
    AnyboxDevice *device;
    if (anybox_abi_version() != ANYBOX_ABI_VERSION) {
        printf("this library doesn't go with this anybox.h\n");
        return 1;
    }
    if (anybox_connect(&device) != ANYBOX_STATUS_OK) {
        printf("%s\n", anybox_last_error_message());
        return 1;
    }
    anybox_device_move_by(device, 100, 0);
    anybox_device_click(device, ANYBOX_BUTTON_LEFT);
    anybox_device_free(device);
    return 0;
}
```

it's the same API with C names. every call that can fail returns a status, the text is in `anybox_last_error_message()` and the fields of the Rust error (the port, the action, the firmware it found and so on) are in `anybox_last_error_detail()`, so you never have to parse the text. the header explains the rest. on a Medius, `anybox_device_medius` gives you a handle for `medius.h`, so everything only Medius does works from C too. the mock is in there when you build it with `--features mock`

for C++ there's `anybox.hpp`, which sits on the same library, so there's nothing else to build or link:

```cpp
#include <cstdio>
#include "anybox.hpp"

int main() {
    try {
        const auto device = anybox::connect();
        device.move_by(100, 0);
        device.click(anybox::Button::Left);
    } catch (const anybox::Error& error) {
        std::printf("%s\n", error.what());
        return 1;
    }
}
```

it's the same API with the Rust names, C++17 or newer. a call that fails throws the `anybox::Error` class for its status (`NotFoundError`, `UnsupportedError` and so on) with the Rust error's fields on it, and `Device`, `Events` and the rest free themselves. like in Rust a receiver doesn't keep its device open. define `ANYBOX_FEATURE_MOCK` and link a library built with the mock to get `anybox::Mock`

on a Medius, `device.medius()` gives you its own device for clips, transforms, rendering and the rest, the same things the [medius](https://github.com/K4HVH/medius) crate does, and nothing on any other box:

```cpp
if (const auto medius = device.medius()) {
    medius->set_render(anybox::medius::RenderMode::Despiked, false);
}
```

it's all in `anybox::medius`, which has its own `Device`, `Button`, `Key` and so on, so write the namespace out instead of a `using namespace` for both. it shares anybox's connection, so keeping it holds the port open after the anybox device is gone, and anybox doesn't know about anything you press or lock through it. closing one of its streams from another thread ends a reader waiting on it right away, and its clones end with it, like `close()` in Rust. a Medius error has its status, its text, the protocol byte and an update's detail, because that's all medius.h hands over, the other fields Rust's errors have aren't exposed in C yet. a few more things the crate has aren't in medius.h yet either, so they aren't here, like a timeline's raw box clock, a clip builder's frame count, the names of the update statuses, what the mock replied, and the crate's frame size, protocol version and emit ceiling. [medius-crate.txt](../bindings/conformance/medius-crate.txt) lists everything the crate has and where it went. define `MEDIUS_FEATURE_MOCK` too to get `anybox::medius::MockBox`

## Python

```sh
pip install anybox-rs
```

```python
import anybox

with anybox.connect() as device:
    device.move_by(100, 0)
    device.click(anybox.Button.LEFT)
```

it's the same API with Python names, on top of the C library, which comes inside the wheel so you don't need Rust. Python 3.11 or newer, and the [package's readme](../bindings/python/README.md) has the rest. one thing works differently from Rust on purpose, a receiver keeps its device open, because Python closes a device whenever the garbage collector gets to it, so `anybox.connect().events()` would lose the box right away otherwise. `pip install anybox-rs[medius]` also gets you `device.medius()`, the medius package's own device

## C#

```sh
dotnet add package Anybox
```

```csharp
using Anybox;

using var device = Device.Connect();
device.MoveBy(100, 0);
device.Click(Button.Left);
```

it's the same API with C# names, on top of the C library, which comes inside the package so you don't need Rust. .NET 10 or newer, and the [package's readme](../bindings/csharp/README.md) has the rest. like in Python, a receiver keeps its device open, because .NET finalizes a device whenever the garbage collector gets to it, so `Device.Connect().Events()` would lose the box right away otherwise. `device.Medius()` gives you everything only Medius does, in `Anybox.Medius`, except the same few things C++ doesn't have yet because medius.h doesn't expose them

## when a box can't do something

same as in Rust, anybox never makes up something a box can't do, so you build it yourself out of what the box can do. here's the `type_text` from the [readme](../README.md#when-a-box-cant-do-something) in each language, made of key taps

Python, put on `anybox.Device` itself so every device gets `device.type_text()`, like a trait in Rust:

```python
import anybox
from anybox import Key


def type_text(device: anybox.Device, text: str) -> None:
    for c in text:
        # a us layout and only the basics, add whatever your app needs
        if c.isascii() and c.isalpha():
            key, shift = Key(Key.A + ord(c.lower()) - ord("a")), c.isupper()
        elif "1" <= c <= "9":
            key, shift = Key(Key.NUM1 + ord(c) - ord("1")), False
        elif c in ("0", " ", "\n"):
            key, shift = {"0": Key.NUM0, " ": Key.SPACE, "\n": Key.ENTER}[c], False
        else:
            continue
        if shift:
            device.press_key(Key.LEFT_SHIFT)
        try:
            device.tap_key(key)
        finally:
            if shift:
                device.release_key(Key.LEFT_SHIFT)


anybox.Device.type_text = type_text

with anybox.connect() as device:
    device.type_text("Hello there\n")
```

C#, as an extension on `Device` so it reads like the rest of the API:

```csharp
using Anybox;

using var device = Device.Connect();
device.TypeText("Hello there\n");

static class TypeTextExtension
{
    extension(Device device)
    {
        public void TypeText(string text)
        {
            foreach (var c in text)
            {
                // a us layout and only the basics, add whatever your app needs
                (Key Key, bool Shift)? typed = c switch
                {
                    >= 'a' and <= 'z' => ((Key)((byte)Key.A + (c - 'a')), false),
                    >= 'A' and <= 'Z' => ((Key)((byte)Key.A + (c - 'A')), true),
                    >= '1' and <= '9' => ((Key)((byte)Key.Num1 + (c - '1')), false),
                    '0' => (Key.Num0, false),
                    ' ' => (Key.Space, false),
                    '\n' => (Key.Enter, false),
                    _ => null,
                };
                if (typed is not (Key tap, bool shift))
                {
                    continue;
                }
                if (shift)
                {
                    device.PressKey(Key.LeftShift);
                }
                try
                {
                    device.TapKey(tap);
                }
                finally
                {
                    if (shift)
                    {
                        device.ReleaseKey(Key.LeftShift);
                    }
                }
            }
        }
    }
}
```

C has no methods, so it's a function named like the API's own, `device_type_text(device, text)` next to `anybox_device_tap_key(device, key)`. shift gets let go of even when the tap failed, and the error you get back is the one Rust would give:

```c
#include "anybox.h"

static AnyboxStatus device_type_text(const AnyboxDevice *device, const char *text) {
    for (; *text != '\0'; text++) {
        char c = *text;
        uint8_t key;
        bool shift = false;
        AnyboxStatus tapped;
        /* a us layout and only the basics, add whatever your app needs */
        if (c >= 'a' && c <= 'z') {
            key = (uint8_t)(ANYBOX_KEY_A + (c - 'a'));
        } else if (c >= 'A' && c <= 'Z') {
            key = (uint8_t)(ANYBOX_KEY_A + (c - 'A'));
            shift = true;
        } else if (c >= '1' && c <= '9') {
            key = (uint8_t)(ANYBOX_KEY_NUM1 + (c - '1'));
        } else if (c == '0') {
            key = ANYBOX_KEY_NUM0;
        } else if (c == ' ') {
            key = ANYBOX_KEY_SPACE;
        } else if (c == '\n') {
            key = ANYBOX_KEY_ENTER;
        } else {
            continue;
        }
        if (shift) {
            AnyboxStatus pressed = anybox_device_press_key(device, ANYBOX_KEY_LEFT_SHIFT);
            if (pressed != ANYBOX_STATUS_OK) {
                return pressed;
            }
        }
        tapped = anybox_device_tap_key(device, key);
        if (shift) {
            AnyboxStatus released = anybox_device_release_key(device, ANYBOX_KEY_LEFT_SHIFT);
            if (released != ANYBOX_STATUS_OK) {
                return released;
            }
        }
        if (tapped != ANYBOX_STATUS_OK) {
            return tapped;
        }
    }
    return ANYBOX_STATUS_OK;
}
```

C++ can't add a method to a class from outside it either, so it's a free function that takes the device, and it throws like the rest of the API:

```cpp
#include <string>
#include "anybox.hpp"

void type_text(const anybox::Device& device, const std::string& text) {
    using anybox::Key;
    for (const char c : text) {
        // a us layout and only the basics, add whatever your app needs
        Key key{0};
        bool shift = false;
        if (c >= 'a' && c <= 'z') {
            key = Key{static_cast<std::uint8_t>(Key::A.value + (c - 'a'))};
        } else if (c >= 'A' && c <= 'Z') {
            key = Key{static_cast<std::uint8_t>(Key::A.value + (c - 'A'))};
            shift = true;
        } else if (c >= '1' && c <= '9') {
            key = Key{static_cast<std::uint8_t>(Key::NUM1.value + (c - '1'))};
        } else if (c == '0') {
            key = Key::NUM0;
        } else if (c == ' ') {
            key = Key::SPACE;
        } else if (c == '\n') {
            key = Key::ENTER;
        } else {
            continue;
        }
        if (shift) {
            device.press_key(Key::LEFT_SHIFT);
        }
        try {
            device.tap_key(key);
        } catch (...) {
            if (shift) {
                device.release_key(Key::LEFT_SHIFT);
            }
            throw;
        }
        if (shift) {
            device.release_key(Key::LEFT_SHIFT);
        }
    }
}
```
