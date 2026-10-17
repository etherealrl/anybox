# what each box can do

every method of `Device`, and what each box does with it. "on your PC" means the box has no way to do it by itself, so anybox does it from your PC out of things the box can do, with your PC's timing

what you get also depends on what's plugged into the box. a mouse without horizontal scroll means no `pan`, and a device with no keyboard interface at all means no keys. `device.snapshot().capabilities` tells you what the box in front of you can do right now, and why when it can't

Ferrum and YSC8K (the V1 that's sold and the V2 that isn't out yet) are written from vendor docs and reference code, and so far only one community tester ran them on real ones. "app" means only on the Ferrum app's virtual port, the box's own legacy port doesn't have it. YSC keys start as available and worked on a V1 with only a mouse behind it, though a `no_keyboard` from the box disables them until you reconnect. anybox can't tell yet whether a mouse is behind a YSC, so its snapshot says moves, presses and button events work even when nothing is plugged in

## mouse

| method                   | Medius        | MAKCU            | Ferrum           | YSC8K           |
| ------------------------ | ------------- | ---------------- | ---------------- | --------------- |
| `move_by`                | ✓             | ✓                | ✓                | ✓               |
| `move_now`               | ✓             | V4.086 and up    | ✗                | ✗               |
| `press`, `release`       | ✓             | ✓                | ✓                | ✓               |
| `click`, `click_with`    | on your PC    | ✓                | ✓, a hold on PC  | on your PC      |
| `wheel`                  | ✓             | ✓                | ✓                | one notch       |
| `pan`                    | ✓             | ✗                | ✗                | ✗               |
| `discard_pending_moves`  | ✓             | ✗                | ✗                | ✗               |

## keyboard

| method                         | Medius        | MAKCU            | Ferrum           | YSC8K           |
| ------------------------------ | ------------- | ---------------- | ---------------- | --------------- |
| `press_key`, `release_key`     | ✓             | ✓                | app              | ✓               |
| `tap_key`, `tap_key_with`      | on your PC    | ✓                | app, hold on PC  | on your PC      |
| `press_media`, `release_media` | ✓             | ✗                | ✗                | ✗               |
| `tap_media`, `tap_media_with`  | on your PC    | ✗                | ✗                | ✗               |

a Medius holds as many keys at once as the keyboard behind it can report, six on most of them and no limit on an NKRO one, modifiers not counted

## locks

| method                           | Medius        | MAKCU            | Ferrum           | YSC8K           |
| -------------------------------- | ------------- | ---------------- | ---------------- | --------------- |
| `lock_button`, `unlock_button`   | ✓             | ✓                | ✓                | ✓               |
| `lock_key`, `unlock_key`         | ✓             | ✓                | app              | ✗               |
| `lock_axis`, `unlock_axis`       | ✓             | ✓                | ✓, one way alone only on an app that has it | whole X or Y only |
| `scale_axis`                     | ✓             | ✗                | ✗                | ✗               |

## events

| event                 | Medius        | MAKCU            | Ferrum           | YSC8K           |
| --------------------- | ------------- | ---------------- | ---------------- | --------------- |
| buttons               | ✓             | ✓                | ✓                | ✓               |
| motion                | ✓             | ✗                | app              | ✓               |
| keys                  | ✓             | ✓                | app              | ✗               |
| media keys            | ✓             | ✗                | ✗                | ✗               |

on the Medius every input event also comes with the box's own timestamp, MAKCU, Ferrum, and YSC don't send one

`received` is on every input event from every box. on MAKCU, Ferrum, and YSC it's the moment the port read returned, on Medius it's when anybox took the event from the medius crate, which reads the port on its own thread, so a bit later than the read

on Medius each kind of event is its own subscription, so asking for buttons only never makes the box stream motion, and the order between two kinds is only as good as their box time

Ferrum sends the whole state on every change, so anybox works out what changed. when you start listening it doesn't know what was already held, so a button or key you were holding shows up as a press with the next change

YSC buttons and motion come in one stream. anybox takes the buttons in the first report as already held, so a button that was down before it never shows its press, only its release. what a V1 puts in those reports was read off a real one, though anybox reading them hasn't run on one yet, and a report anybox can't read counts as at least one lost event, it doesn't tell how many the box sent

## box-specific behavior

### YSC8K

- the YSC8K you can buy is the V1, one chip behind a USB serial chip. a community tester ran anybox on one, so moves, buttons, the wheel and keys were seen working there. anybox read the box's events wrong in that run, and the fix for it isn't checked on a real one yet, the rest still comes from the vendor docs and the vendor's own app
- the vendor also documents a V2 with two chips and its own USB ports. it isn't out yet, so its support only comes from their docs, and a V2 gets found on its own like a Medius. on a V2 anybox talks to its TOWMCULEFT port, the other two ports of the box never get a single byte
- a V1 can sit at any of 9 rates, so anybox looks for it the way the vendor's app does, 4M first, and never changes the rate. its firmware is the build date it gives back, and if it's in its bootloader you get told to finish the firmware update first
- clicks and key taps are timed on your PC, and the wheel takes one notch at a time
- keys worked on that V1 with only a mouse behind it, though if the box ever says there's no keyboard, keys turn off until you connect again
- anybox can't tell yet whether a mouse is behind the box, so moves, presses and button events say they work even when nothing is plugged in
- if the box is in debug mode, leave it in the vendor tool first, anybox won't connect otherwise
- after being unplugged and plugged back in, that V1 kept answering but ignored every text command (moves, clicks, locks) and i don't know why yet. when you connect, anybox checks for that and tells you (a reconnect just keeps trying), turning debug mode on and off in the vendor tool brought it back once
- locks cover the five buttons and a whole X or Y axis, not one direction of it
- buttons and motion come in one stream, and anybox doesn't know what was already held when you start listening, so a button that's already down when the first report comes in never shows its press, only its release
- the box's curve and interpolation settings can shape your moves, anybox leaves them alone
- i don't know yet whether events keep coming while something is locked, or what stays held when the port closes or the box loses power

### Ferrum

- a community tester ran anybox on one through the app's port (app 2.0.5), so moves, clicks, the wheel, keys and locks were seen working there, the rest comes from the vendor docs
- in that run keys anybox sent only reached Windows once a key got pressed on the real keyboard, i don't know why yet and it was seen just once
- with the app installed, Windows refused to open the box's own port on that PC even with the app closed, and i don't know what holds it
- anybox does the most on the Ferrum app's virtual port, which needs the app running on Windows. the box's own port (the legacy one, and the only way in from Linux) only moves, clicks, scrolls, locks buttons and whole axes, and reports buttons. Mouse Proxy, controllers and the app's network APIs aren't in anybox yet
- `connect()` doesn't find it by itself since we don't know its USB ids yet, so use `Connect::port` or `.family(Family::Ferrum)`. on Windows that search only takes the app's port, add `.ferrum_api(FerrumApi::Legacy)` if you really want the box's own one. `device.snapshot().info.ferrum_api` says which one you're on
- the app has to answer `km.has_mouse()`, otherwise anybox can't tell what's plugged in and asks you to update it
- by default a release, and the end of a click or a tap, forces it up for 125 to 175 ms before the physical button counts again. the app's No Button Release setting drops the software press right away instead, and anybox can't read that setting
- a click on a button that's still in its press or forced release takes over from it, i don't know yet what two quick clicks in a row look like
- the box lets the physical button win, so pressing and letting go of a button anybox holds really lets it go
- clicks and taps hold for a random 75 to 125 ms, the box has no way to take a hold, so a click or a tap with one is a press and a release timed from your PC, which is what the vendor tells you to do too
- the vendor says to scroll one notch at a time, bigger scrolls can look emulated
- the app's Move Multiplier scales every move and anybox can't read it, so leave it at 1 if your moves have to be exact
- the box never says no, so a command it doesn't take just does nothing. anybox only sends what the box says it has
- a mouse or keyboard you set in the app after connecting shows up within a second
- unplugging the box from this PC doesn't disconnect anybox, because it talks to the app's port and that stays open. the app just loses the mouse and keyboard, so they go away from the snapshot with an `Event::Changed` and come back the same way when you plug it back in (the app doing that was seen on a real box). the box forgets its locks over that, so when it's back you also get an `Event::SessionReset`, and anybox puts your locks back and lets go of what you held. a replug quicker than a second can slip past that though
- `info.peripheral` comes from the app's `device.VID()` and `device.PID()`. with nothing plugged in the app answers a default pair that's also a real Logitech receiver, so anybox leaves it empty then
- the legacy port answers exactly like a KMBox B+, so a B+ opens as a legacy Ferrum too. anybox only promises the legacy commands there, and nobody tried it on a B+
- locking one way of an axis needs an app that has directional locks, otherwise (and on the legacy port) you can only lock a whole axis
- i don't know yet whether the events include what anybox sends or only what you do yourself

### MAKCU

- on a MAKCU on V4.094 (measured with a USB capture, nobody touching the mouse) about 1,400 moves a second got through and the rest were refused, and an `Event::Rejected` counts the ones it dropped. that's one box and one firmware, so take it with a pinch of salt. if you move once per frame at a high frame rate, add up the deltas and send them every millisecond or two instead. moving the real mouse at the same time probably lowers that, i haven't measured it yet
- a key tap without a hold lasts about one report, so some apps miss it, pass a hold with `tap_key_with` if it has to land. a click is fine, the box holds it 35 to 75 ms by itself
- a key press right after a tap gets refused
- a lock on a button that's already held still lets its release through
- if your app gets killed, its locks stay on the box until something unlocks them
- after a reconnect anybox lets go of what was held, press again if you still need it
- right after a reconnect the mouse behind the box can take a second or so to show up again, so until then the snapshot has no mouse and mouse calls come back unsupported. you get a second `Reconnected` once it's there, so wait for that one
- unplugging the box restarts it, so a button held at that moment stays down in Windows after the reconnect, because the box never sends it the release. anybox can't fix that without clicking for you, a real click on that button clears it

### Medius

- a lock on a button that's already held keeps it down until you unlock it
- held buttons, keys and locks come back by themselves after the box reconnects or resets
- `device.medius()` gives you the medius crate's own device, for clips, transforms, rendering, the polling rate (`query_rate()`) and the identity the box shows the PC (`device_info()`)
- anybox doesn't know what you press or lock through `medius()`, and its `reset()` lets go of and unlocks EVERYTHING, what anybox did too
