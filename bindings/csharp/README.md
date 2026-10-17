# anybox

one API for the most popular passthrough boxes, from C#. it's the [anybox](https://github.com/etherealrl/anybox) Rust library through its C ABI, so it does exactly what the Rust one does, and the [README there](https://github.com/etherealrl/anybox#readme) has which boxes it supports and how each one behaves

```sh
dotnet add package Anybox
```

the package has the library inside, so you don't need Rust. it's Windows and Linux on x64 and .NET 10 or newer, so no .NET Framework and no Unity for now, if you need something else open an issue. on Linux the library needs `libudev.so.1` from the system (`libudev1` on Debian and Ubuntu), which most distros have but Alpine and some minimal images don't, and your user has to be allowed to open serial ports, the `dialout` group on most distros (`uucp` on Arch)

```csharp
using Anybox;

using var device = Device.Connect();
Console.WriteLine(device.Snapshot().Info);
device.MoveBy(100, 0);
device.Click(Button.Left);
device.TapKey(Key.A);
```

`Device.Connect()` takes the first box it finds, `new Connect { Port = "COM18" }.Open()` or `new Connect { Family = Family.Medius }.Open()` the one you want, and `Device.Scan()` gives every port with what's on it, which is what a box picker wants. `Device.Candidates()` is the cheap one to call every second. dispose the device or call `Close()` so anybox lets go of what you held and unlocks what you locked

if a box can't do something you get an `UnsupportedException` saying why, and `device.Snapshot().Capabilities[BoxAction.KeyTap]` tells you before you try. every error is an `AnyboxException` with the library's own text, and it has the Rust error's fields too, like `Port`, `Action` or the boxes a `NoBoxCanException` found and what each one couldn't do. the actions are `BoxAction` here and not `Action` like in Rust, because that one is already taken by `System.Action`

what happens on the box comes in as events:

```csharp
using Anybox;

using var device = Device.Connect();
using var events = device.Events(EventKinds.Buttons);
foreach (var happened in events)
{
    switch (happened)
    {
        case ButtonEvent { Button: Button.Side1, Pressed: true }:
            Console.WriteLine("side button down");
            break;
        case Disconnected:
            Console.WriteLine("the box is gone, anybox keeps looking for it");
            break;
        case Reconnected:
            Console.WriteLine("it's back");
            break;
    }
}
```

`events.Recv(timeout)` gives you one at a time instead, null when nothing came in time. the loop ends once the device is closed and everything queued was read, or once you dispose the receiver from another thread, and a receiver keeps its device open while you have it, unlike in Rust, since the garbage collector would close it under you otherwise. just so you know, a receiver of keys sees every key typed on the keyboard behind the box, passwords included

durations like a click's hold are `TimeSpan`s, and so is an event's stamp, the time since the device was opened. `At.ArrivedNs` and `At.ReceivedNs` are nanoseconds on the clock `Stopwatch` reads, and `Monotonic.FromStopwatchTicks` turns a `Stopwatch.GetTimestamp()` into the same thing, so you can line them up with your own timing and with other boxes. the numbers go in as the types the box takes (`short` for a move for example), so nothing gets cut short on its way

what only one box can do stays reachable through that box's own API. on a Medius, `device.Medius()` gives you its own device for clips, transforms, rendering and the rest, the same things the [medius](https://github.com/K4HVH/medius) crate does, and null on every other box:

```csharp
using Anybox;
using Medius = Anybox.Medius;

using var device = Device.Connect();
using var medius = device.Medius();
medius?.SetRender(Medius.RenderMode.Despiked, false);
```

it's all in `Anybox.Medius`, and the alias is the way to use it, because it has its own `Device`, `Button`, `Key` and so on, and importing both namespaces makes those ambiguous. it shares anybox's connection, so keeping it holds the port open after the anybox device is gone, and anybox doesn't know about anything you press or lock through it. disposing one of its streams from another thread ends a reader waiting on it right away, and its clones end with it, like `close()` in Rust. a Medius exception has its status, its text, the protocol byte and an update's detail, because that's all medius.h hands over, the other fields Rust's errors have aren't exposed in C yet

## license

MPL-2.0, the package has the license, the notice and the licenses of everything built into the library. made by ethereal
