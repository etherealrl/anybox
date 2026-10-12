<div align="center">

# anybox

one API for the most popular passthrough boxes

<p>
  <img src="https://img.shields.io/badge/rust-111111?style=flat&logo=rust" />
  <a href="https://crates.io/crates/anybox-rs"><img src="https://img.shields.io/crates/v/anybox-rs?style=flat&amp;color=2b2b2b&amp;labelColor=111111&amp;label=crates.io&amp;logo=data%3Aimage%2Fpng%3Bbase64%2CiVBORw0KGgoAAAANSUhEUgAAABwAAAAcCAIAAAD9b0jDAAAACXBIWXMAAAABAAAAAQBPJcTWAAAAAXNSR0IB2cksfwAAACBjSFJNAAB6JgAAgIQAAPoAAACA6AAAdTAAAOpgAAA6mAAAF3CculE8AAAABGdBTUEAALGPC%2FxhBQAAB5FJREFUeJyN1XdQU9kaAPBLKNJNKCLSFFGKFAkhHSkBKdIRURbUhV0LiuLqggMoiIiAoBEEpQlYlpJ7UwgQOggCK6isrlhHSsgtIdFZ%2FWf%2FfQfd55v3Rt%2FuN9%2FcuWfm3t%2F55lQI%2BkboaWg4m5ADvSz4FyJ7a4Jw6W5cugu5HFpbnhQf7Whnu%2BpbP34zjCHoCM3i7knuVF2EQhz1vmunsitBNXjo41TOQn%2FWC8nBxvOBmSm2QVwz%2FVWa%2F0g00dM8HUV9BSeo%2B3a%2F692j7tv%2F8UGuciwbHUjHuhKJjugPst1%2F9qf%2B0ZM8yI%2B4Vhjgw93gsE5fm%2FR%2FdcvVGucOMWfvbp8TBCyPZavu5xC9e1FpLNa5E%2B%2Fctdyfqh7LVA4fUcl2f%2ByPf9%2BX%2BPB2ZFdFXFqynX%2Bg4TdlK2Odwh%2FoL24xXjY7yzvjF0WhcgEPFUcCEe%2FaBV6wzjjlSLpi6ITyXoZ6MOVdT5xaFjsnjO6tiY%2BPcSaTtb6K6l5Iob1opr5qcn7Z5Py2haEQheHSOLQjhujZT%2FSlAhqTRCgkUagsCbjL49n4UMaydMeH%2FnhBbZY3g%2FM11IiUl%2BT2vJkx18pBhcEKYTjaEYX1fo%2BPnlaCoRg6gncn4NIovDsW7wY9heN9%2B9TjWYttrCUBp6Uqw4vO%2FWqlUMF%2B6qIwUiEKRzv34f0Z%2BMgpbOg4JkvGpNFERzjRGQk4rCtmSRy62M5cQnzQjhBFO03V4dd245SXt89XUHN9KCfZ7Vmz35vb3qqRTNW9TLT7O1S6EwOcJBgkJuLJYe4S7LvYxl5ooS21M5ba6aiAoZLwbvGTveje%2FyuSTQ1C2WaX0lx%2Fb%2BL%2B3uC6KIldEEW%2BbectICGoJBgFRSE%2BcgF9XuCpgLdhsD8GczCYi8McAmYRSFB5liHV2%2BQvC%2BwPZ2ebhB0O5zKYklKOQhLxrMFz5vqmp3VOszc9QL5spsoFXAXMeX2HPnnN7UmDByr0xZEAAvEhYC4Bc5QAhQP%2Fg1IohtVHGU2FIS87UxdFUWpZNNEd%2B6TO42Gl3Uy1%2FdNax6c1jrP1LgoBa76FNVzuMlTm8muVhwLxJ4SfUGQF%2FVxpWaY%2B1ZsCkTWhg9ttXkuTVbI4ojselyUujx5TjZ2Yue4yUW45XQHcjSt0jSOBcF41M3svufaXu09UeuKiAELEw4VsQhKgHj6gEAYsCcMr89yp3naQNQRluhs8vuU317bt3f0z6rGcpe7kBUn8g%2FJ1oyXmwJ3i20xdtZ2pdlAKuc8aGMN8j99uMt7cZiyLA5bFgbiQg0rC8f4f3wqCcHFIU7GlJ80coqyGcg6bTVYyntU5yDv2vG3jgZRLoscvWQ0WUiaubprk24%2BXWU5dtQNzstTOmamngc22LPEDawAXB80joQvIdgxmY%2B0MHPFrLKJ40kzAgEJnjpmOlntOX7V7UusxW79ltnbz86atU9Uuw5ccHt%2BJGa%2BiDxQYj5VZKcDwCdlKEQcTBciFgYui7XKhnwJhzbXSpm9sfd7gqRIHNV40XUFNyNDZo%2BShC06TJWsny2yn%2BSCtp6s2zdS4Pqp2nKndOlFJHbhgMVxitYD4LwkDFR2xRG%2FKXBvnzR0WLvJbaGWOVHgMX3Ebr3DHEF5TselfleYfp8gK7IYLySMXzMaKLe6XWo5f2fCwyulRhf3kZdvfqjc%2BvO704JrTPBwkh%2F1QaYSyZ4%2B8lSFvZSmFPq%2FvMGWlW%2FrKXMf47oQoqPmiGZVmBmnrQlE8fel56548g958o%2F4CULX5aKnN5JWNkmwLQab5QKHlg0qrR1UOCy3s%2BV9oGMLEYKZCwEZhDg6mrolxj%2B8%2BdYP2tIH2R2doXf5aFzfTlXXqYq3XctpFmm8yyXfvKzAD9GDRmpESGyTLvOmYSevP5gNF5lMVGxQCDgoDi4UhKyKGcMF6kLexpm54Pb1JU3dwFHDwiUQXLd1PN40%2F1bL0AKP8gP39ysSekg3iXKj%2FvOFY6brWkya1aeT6o6vFuZRf%2BetRAQdH2CBX5hrhgiREXLWUrZKy5tt4A5fDui7%2FmJ4U8nmLajiuNz2aQD9%2FmFdyOLT4EKfptF1nnvFEqYXsLEWQufrWMUNBpuFEuS1AlQjYOWxiBfVRiri4aNtsW4i4PKIkjfdzAnOs%2FsQPO3lWNoDUJEEQiWKo58OwdVhPIRsahTOdSw561WfaSfLNes7qik%2BSus8YDF9c8xlViXkfBpNQUejjxmBBUdjl9B05sZ4Fu70y4mjB9E0UY%2F3PhWqQIJCkLwfVZhvymcO87O8Di9OYjVlOnXlmffm6w0UUMKBqMft9T8JL%2BEhzrv%2FFNN%2FR2tRr6f4%2FxbjF0h1syat1NLU1vjgaEGQIQUaamrqfWkar9Hw97U%2Ft25YSzeJuWXc01uV6%2BubuvDXgIhivCWvIDju3j5URR81L8StOj%2FClbbRfb25goPuVgxkEuK60wVNLS0MDXIkkY10DHZIO6MNYWystml6S5ttWlHDleNhPcW65yeyMWNbOba7rTI20tUgrRf1taHyKL00dTdLeEM%2FzB4MnG07dPbfncAzNn7bBGhSno%2F3vj0jQP4L%2FO6zXUsIYm3P3BvK81jtYGulp%2Fw3xLxzmgcpQDPe3AAAAAElFTkSuQmCC" /></a>
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

> KMBox B Pro and Net might get added if enough people ask for it, otherwise i won't spend time on it

|            | firmware     | mouse | wheel | pan | keys | media keys | locks | scale | events        |
| ---------- | ------------ | :---: | :---: | :-: | :--: | :--------: | :---: | :---: | ------------- |
| **Medius** | 3.4.2+       |   ✓   |   ✓   |  ✓  |  ✓   |     ✓      |   ✓   |   ✓   | all           |
| **MAKCU**  | V4.085+      |   ✓   |   ✓   |  ✗  |  ✓   |     ✗      |   ✓   |   ✗   | buttons, keys |
| Ferrum     | docs only    |   ✓   |   ✓   |  ✗  | app  |     ✗      |   ✓   |   ✗   | all but media |
| YSC8K V2   | docs only    |   ✓   |   ✓   |  ✗  |  ✓   |     ✗      |   ✓   |   ✗   | buttons, motion |

Ferrum and YSC8K are written from the vendor docs and reference code, nobody has run these backends on real ones yet, so expect surprises and tell me about them. "app" means only through the Ferrum app's port

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

if a box acts weird and you want to see what actually goes over the wire, turn on the `trace` feature and set `ANYBOX_TRACE` to a file path. every byte anybox sends and gets back ends up in that file with a timestamp, the Medius is the one exception since it talks through the medius crate

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

`connect()` takes the first box it finds. `anybox::find()` lists every box on the machine with what each one can do, `Connect::port("COM18")` or `Connect::auto().family(Family::Medius)` opens the one you want, and `Connect::auto().can(Action::Move)` opens the first one that can move, so a keyboard box plugged in next to it gets skipped. a Ferrum isn't found on its own yet, open it with `Connect::port` or `.family(Family::Ferrum)`

a YSC8K V2 is found on its own like a Medius or a MAKCU. a YSC or a Ferrum only comes back on the same port after a reconnect, so if its port name changes, connect again

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

### YSC8K

- only the V2, and none of this was checked on a real one yet, it all comes from the vendor docs and the vendor's own app
- anybox talks to its TOWMCULEFT port, the other two ports of the box never get a single byte
- clicks and key taps are timed on your PC, and the wheel takes one notch at a time
- keys need a keyboard behind the box. if the box says there's none, keys turn off until you connect again
- if the box is in debug mode, leave it in the vendor tool first, anybox won't connect otherwise
- locks cover the five buttons and a whole X or Y axis, not one direction of it
- buttons and motion come in one stream, and anybox doesn't know what was already held when you start listening, so a button that's already down when the first report comes in never shows its press, only its release
- the box's curve and interpolation settings can shape your moves, anybox leaves them alone
- i don't know yet whether events keep coming while something is locked, or what stays held when the port closes or the box loses power

### Ferrum

- none of this was checked on a real Ferrum yet, it all comes from the vendor docs
- anybox does the most on the Ferrum app's virtual port, which needs the app running on Windows. the box's own port (the legacy one, and the only way in from Linux) only moves, clicks, scrolls, locks buttons and whole axes, and reports buttons. Mouse Proxy, controllers and the app's network APIs aren't in anybox yet
- `connect()` doesn't find it by itself since we don't know its USB ids yet, so use `Connect::port` or `.family(Family::Ferrum)`. on Windows that search only takes the app's port, add `.ferrum_api(FerrumApi::Legacy)` if you really want the box's own one. `device.snapshot().info.ferrum_api` says which one you're on
- the app has to answer `km.has_mouse()`, otherwise anybox can't tell what's plugged in and asks you to update it
- by default a release, and the end of a click or a tap, forces it up for 125 to 175 ms before the physical button counts again. the app's No Button Release setting drops the software press right away instead, and anybox can't read that setting
- a click on a button that's still in its press or forced release takes over from it, i don't know yet what two quick clicks in a row look like
- the box lets the physical button win, so pressing and letting go of a button anybox holds really lets it go
- clicks and taps hold for a random 75 to 125 ms and don't take a hold, press and release yourself if you need one
- the vendor says to scroll one notch at a time, bigger scrolls can look emulated
- the app's Move Multiplier scales every move and anybox can't read it, so leave it at 1 if your moves have to be exact
- the box never says no, so a command it doesn't take just does nothing. anybox only sends what the box says it has
- a mouse or keyboard you set in the app after connecting shows up within a second
- the legacy port answers exactly like a KMBox B+, so a B+ opens as a legacy Ferrum too. anybox only promises the legacy commands there, and nobody tried it on a B+
- locking one way of an axis needs an app that has directional locks, otherwise (and on the legacy port) you can only lock a whole axis
- i don't know yet whether the events include what anybox sends or only what you do yourself

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
