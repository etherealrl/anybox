<div align="center">

# anybox

one API for the most popular passthrough boxes

<p>
  <img src="https://img.shields.io/badge/rust-111111?style=flat&logo=rust" />
  <img src="https://img.shields.io/crates/v/anybox-rs?style=flat&color=111111&label=crates.io" />
</p>
<p>
  <img src="docs/badges/windows.svg" />
  <img src="https://img.shields.io/badge/linux-111111?style=flat&logo=linux&logoColor=FCC624" />
  <a href="#support"><img src="https://img.shields.io/badge/support_my_work-111111?style=flat&logo=githubsponsors&logoColor=EA4AAA" /></a>
</p>

</div>

## what it does

anybox talks to the passthrough box plugged into your PC, so you write your app once and it works with all the [supported boxes](#boxes) the same way. when you connect, it figures out which box it is, the firmware, and what it can do, so you don't have to

if a box can't do something, you get a clear error telling you so. nothing is faked or changed behind your back, anybox reproduces each box's native behavior as closely as it can, and the differences i found are all written down in [box-specific behavior](#box-specific-behavior)

some boxes can do things the others can't, and anybox doesn't hide those. you get that box's own API from the device, so you can use them next to everything else

## boxes

> Ferrum and YSC8K are in the works, and KMBox B Pro and Net might get added if enough people ask for it, otherwise i won't spend time on it

|            | firmware     | mouse | wheel | pan | keys | media keys | locks | scale | events        |
| ---------- | ------------ | :---: | :---: | :-: | :--: | :--------: | :---: | :---: | ------------- |
| **Medius** | 3.4.2+       |   ✓   |   ✓   |  ✓  |  ✓   |     ✓      |   ✓   |   ✓   | all           |
| **MAKCU**  | V4.085+      |   ✓   |   ✓   |  ✗  |  ✓   |     ✗      |   ✓   |   ✗   | buttons, keys |
| Ferrum     | in the works |       |       |     |      |            |       |       |               |
| YSC8K      | in the works |       |       |     |      |            |       |       |               |

not every box can do everything, and [what each box can do](docs/boxes.md) has every method with the small differences. what you actually get also depends on what's plugged into the box, a mouse without horizontal scroll means no `pan` for example, so `device.snapshot().capabilities` is what tells you for sure, at runtime

## install

```sh
cargo add anybox-rs
```

it's still `use anybox::...` in your code. by default every box is built in, if you only ever talk to one kind you can leave the others out:

```sh
cargo add anybox-rs --no-default-features --features medius
```

on Linux it needs libudev to find the boxes, that's `libudev-dev` and `pkg-config` on Debian and Ubuntu, `systemd-devel` on Fedora. your user also has to be allowed to open serial ports, the `dialout` group on most distros (`uucp` on Arch)

to test your own code without a box plugged in, turn on the `mock` feature. `Device::mock_medius` gives you a device over the medius crate's mock box, running the exact same code a real Medius gets. it only proves your code works though, not how the hardware behaves

## usage

```rust,no_run
use anybox::{Button, Key};

fn main() -> anybox::Result<()> {
    let device = anybox::connect()?;
    println!("{:?}", device.snapshot().info);

    device.move_by(100, 0)?;
    device.click(Button::Left)?;
    device.tap_key(Key::A)?;
    Ok(())
}
```

`connect()` takes the first box it finds. `anybox::find()` lists every box on the machine with what each one can do, `Connect::port("COM18")` or `Connect::auto().family(Family::Medius)` opens the one you want, and `Connect::auto().can(Action::Move)` opens the first one that can move, so a keyboard box plugged in next to it gets skipped

what happens on the box comes in as events:

```rust,no_run
use anybox::{Button, Event};

fn main() -> anybox::Result<()> {
    let device = anybox::connect()?;
    for event in device.events() {
        match event {
            Event::Button { button: Button::Side1, pressed: true, .. } => println!("side button down"),
            Event::Disconnected => println!("the box is gone, anybox keeps looking for it"),
            Event::Reconnected => println!("it's back"),
            _ => {}
        }
    }
    Ok(())
}
```

a receiver can ask for only some kinds, `device.events_of(EventKinds::BUTTONS)` for a hotkey for example, and then the box doesn't stream motion or keys just for it. the box only streams while at least one receiver wants something. just so you know, a receiver of keys sees every key typed on the keyboard behind the box, passwords included

every event has a stamp. `received` is when anybox got it, which is what you want next to a capture or your own timing. `arrived` is anybox's best guess at when it happened, and on a Medius that's the box clock mapped onto the PC's and smoothed, so it's NOT when the PC got it

when you drop the device, anybox tries to let go of everything you still hold and unlock everything you locked. if the box is unplugged right then it can't, so call `close()` instead when you want to know whether it worked. `close()` also works on a device you share between threads, and after it every command fails with `Error::Closed`

## when a box can't do something

anybox never makes up something a box can't do, that's your responsibility. if you want it anyway, put it in your own trait on `Device` and build it out of what the box can do

anybox has no `type_text` for example, so here it is made of key taps:

```rust,no_run
use anybox::{Device, Key};

trait TypeText {
    fn type_text(&self, text: &str) -> anybox::Result<()>;
}

impl TypeText for Device {
    fn type_text(&self, text: &str) -> anybox::Result<()> {
        for c in text.chars() {
            // a us layout and only the basics, add whatever your app needs
            let (key, shift) = match c {
                'a'..='z' => (Key(Key::A.0 + (c as u8 - b'a')), false),
                'A'..='Z' => (Key(Key::A.0 + (c as u8 - b'A')), true),
                '1'..='9' => (Key(Key::NUM1.0 + (c as u8 - b'1')), false),
                '0' => (Key::NUM0, false),
                ' ' => (Key::SPACE, false),
                '\n' => (Key::ENTER, false),
                _ => continue,
            };
            if shift {
                self.press_key(Key::LEFT_SHIFT)?;
            }
            let tapped = self.tap_key(key);
            if shift {
                self.release_key(Key::LEFT_SHIFT)?;
            }
            tapped?;
        }
        Ok(())
    }
}

fn main() -> anybox::Result<()> {
    let device = anybox::connect()?;
    device.type_text("Hello there\n")
}
```

## moving from another library

if your app already talks to a box directly, moving over is mostly swapping your calls for anybox's. a few things that help:

- anybox opens the port itself and a port only opens once, so your own serial code goes away completely
- let `connect()` find the box instead of a hardcoded port, `device.snapshot()` tells you which box and firmware it found and what it can do
- delete your reconnect loop and your "release everything on exit" code, anybox does both
- handle `Error::Unsupported` where a box might not have something, don't fake it
- on a Medius, whatever only the medius crate does stays reachable through `device.medius()`, so use that instead of a second dependency

<details>
<summary>a prompt to let an AI do it for you lazy bums</summary>

```text
I'm moving this project from talking to a passthrough box directly to anybox, a library that drives
these boxes through one API. Its core is the Rust crate `anybox-rs` (imported as `anybox`), and the
README lists the bindings for other languages (a C ABI for C and C++, Python and so on).

Before changing anything, read:
- the README at https://github.com/etherealrl/anybox (API overview, which languages are supported and
  how to install each, box-specific behavior, the pattern for adding things a box can't do)
- docs/boxes.md in that repo (what each box supports, method by method)
- the API docs for this project's language: https://docs.rs/anybox-rs for Rust, or the docs the README
  links for the binding

Use the Rust crate if this project is Rust, otherwise the binding for its language. If the README has
no binding for this language yet, stop and tell me instead of writing one.

Then:
1. Find every place this code talks to the box (opening the port, sending commands, reading input,
   reconnecting, cleanup on exit) and list them.
2. Replace each with its anybox equivalent. Where anybox has none, keep the box's own API (e.g. on a Medius
   through anybox's access to the medius library) or tell me it's missing. Never emulate a feature the
   box doesn't have.
3. Remove our own reconnect and release-on-exit logic, anybox handles both.
4. Build and run the tests, and tell me what couldn't be checked without the box plugged in.
```

</details>

## box-specific behavior

### MAKCU

- a key tap without a hold lasts about one report, so some apps miss it, pass a hold with `tap_key_with` if it has to land. a click is fine, the box holds it 35 to 75 ms by itself
- a key press right after a tap gets refused
- a lock on a button that's already held still lets its release through
- if your app gets killed, its locks stay on the box until something unlocks them
- after a reconnect anybox lets go of what was held, press again if you still need it

### Medius

- a lock on a button that's already held keeps it down until you unlock it
- held buttons, keys and locks come back by themselves after the box reconnects or resets
- `device.medius()` gives you the medius crate's own device, for clips, transforms, rendering, the polling rate (`query_rate()`) and the identity the box shows the PC (`device_info()`)
- anybox doesn't know what you press or lock through `medius()`, and its `reset()` lets go of and unlocks EVERYTHING, what anybox did too

## support

if anybox saves you some time, you can [sponsor my work](https://github.com/sponsors/etherealrl), it genuinely helps

> GitHub is still approving the sponsors page, so the link might not work yet. it should be up soon

## license

MPL-2.0, see [LICENSE](LICENSE). you can use anybox in any app, closed source too. if you change anybox itself and ship that, those changes have to be published under the same license with the credit kept. made by ethereal

<div align="center">
  <sub>native behavior, one API</sub>
</div>
