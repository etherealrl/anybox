# what each box can do

every method of `Device`, and what each box does with it. "on your PC" means the box has no way to do it by itself, so anybox does it from your PC out of things the box can do, with your PC's timing

what you get also depends on what's plugged into the box. a mouse without horizontal scroll means no `pan`, and a device with no keyboard interface at all means no keys. `device.snapshot().capabilities` tells you what the box in front of you can do right now, and why when it can't

Ferrum and YSC8K (the V1 that's sold and the V2 that isn't out yet) are written from vendor docs and reference code, and so far only one community tester ran them on real ones. "app" means only on the Ferrum app's virtual port, the box's own legacy port doesn't have it. YSC keys and key events start as available, and keys worked on a V1 with only a mouse behind it, though a `no_keyboard` from the box turns both off until you reconnect. anybox can't tell yet whether a mouse is behind a YSC, so its snapshot says moves, presses and button events work even when nothing is plugged in

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
| `type_text`                    | ✗             | ✓                | ✗                | ✓               |
| `press_keys`, `tap_keys`       | ✗             | ✓                | app              | ✗               |
| `release_keys`                 | one at a time | ✓                | app              | one at a time   |

`type_text` sends US layout keys, so on a PC set to another layout the text comes out as that layout reads them. it only takes printable ASCII, so tap Enter or Tab yourself after it. a chord goes out as one command of the box, up to 14 keys, and a box without them answers `Unsupported` instead of pressing them one by one, since being together is the whole point. `release_keys` still lets go of them one at a time there, so a key you hold always gets let go (a key you don't hold can still come back unsupported once the held ones went)

a Medius holds as many keys at once as the keyboard behind it can report, six on most of them and no limit on an NKRO one, modifiers not counted

## locks

| method                           | Medius        | MAKCU            | Ferrum           | YSC8K           |
| -------------------------------- | ------------- | ---------------- | ---------------- | --------------- |
| `lock_button`, `unlock_button`   | ✓             | ✓                | ✓                | ✓               |
| `lock_key`, `unlock_key`         | ✓             | ✓                | app              | ✗               |
| `lock_axis`, `unlock_axis`       | ✓             | ✓                | ✓, one way alone only on an app that has it | whole X or Y only |
| `scale_axis`                     | ✓             | ✗                | ✗                | ✗               |
| `force_release_key`, `restore_key` | ✓            | ✓                | ✗                | ✗               |

a lock blocks what the user does, your own presses and moves still go through (both vendors say so, i haven't checked it myself). the events keep showing what the user does (that one's checked on both), so you can lock something, read its events and send your own. the thing is that a lock on a key that's already held doesn't always take it away from the PC, on a Medius it stays down until the unlock. `force_release_key` is for that, the PC sees the key up while the finger stays on it, and `restore_key` gives it back, down again if it's still held. while a key is forced up, pressing, typing or locking it is refused, and so is forcing a key you hold or locked yourself, because each box would do something else with it

## events

| event                 | Medius        | MAKCU            | Ferrum           | YSC8K           |
| --------------------- | ------------- | ---------------- | ---------------- | --------------- |
| buttons               | ✓             | ✓                | ✓                | ✓               |
| motion                | ✓             | ✗                | app              | ✓               |
| keys                  | ✓             | ✓                | app              | ✓               |
| media keys            | ✓             | ✗                | ✗                | ✗               |

on the Medius every input event also comes with the box's own timestamp, MAKCU, Ferrum, and YSC don't send one

`received` is on every input event from every box. on MAKCU, Ferrum, and YSC it's the moment the port read returned, on Medius it's when anybox took the event from the medius crate, which reads the port on its own thread, so a bit later than the read

on Medius each kind of event is its own subscription, so asking for buttons only never makes the box stream motion, and the order between two kinds is only as good as their box time

Ferrum sends the whole state on every change, so anybox works out what changed. when you start listening it doesn't know what was already held, so a button or key you were holding shows up as a press with the next change

YSC buttons and motion come in one stream. anybox takes the buttons in the first report as already held, so a button that was down before it never shows its press, only its release. what a V1 puts in those reports was read off a real one, though anybox reading them hasn't run on one yet, and a report anybox can't read counts as at least one lost event, it doesn't tell how many the box sent

YSC keys come in reports of their own, which the box sends every time something changes on the keyboard behind it, whether you listen or not. anybox follows them from the moment it connects, so a key that was already down then shows its press with the next change, and if you start listening while a key is down you only get its release. the keys anybox presses never show up, only the real keyboard's. the box names 6 keys at most next to the modifiers, so with more down than that the extra ones only show up once some go up. those reports were read off a real V1's recording and the vendor's own source, though anybox reading them hasn't run on one yet

## box-specific behavior

a lot of the MAKCU and Medius notes line up with K4's [comparison of stock MAKCU firmware and Medius](https://medius.k4tech.net/comparison) (K4 makes Medius). it's a mix, some of it i measured myself, some we both found on our own, and some only K4 found, the move latency, the extra reports and what gets lost below are K4's. the comparison has a lot more, like which devices each one clones and how faithfully

### YSC8K

- the YSC8K you can buy is the V1, one chip behind a USB serial chip. a community tester ran anybox on one, so moves, buttons, the wheel and keys were seen working there. anybox read the box's events wrong in that run, and the fix for it isn't checked on a real one yet, the rest still comes from the vendor docs and the vendor's own software, which is public now
- the vendor also documents a V2 with two chips and its own USB ports. it isn't out yet, so its support only comes from their docs, and a V2 gets found on its own like a Medius. on a V2 anybox talks to its TOWMCULEFT port, the other two ports of the box never get a single byte
- a V1 can sit at any of 9 rates, so anybox looks for it the way the vendor's app does, 4M first, and never changes the rate. its firmware is the build date it gives back, and if it's in its bootloader you get told to finish the firmware update first
- clicks and key taps are timed on your PC, and the wheel takes one notch at a time
- the box types text by itself and gets the case right with Caps Lock on too, it reads Caps Lock off your PC. that comes from the vendor's docs and source, it hasn't run on a real one yet, and i don't know what it does with a key you hold while it types
- the box has no chords, it presses one key per command
- keys worked on that V1 with only a mouse behind it, though if the box ever says there's no keyboard, keys, typing and key events turn off until you connect again
- key events only show the real keyboard behind the box, the keys anybox presses aren't in them
- anybox can't tell yet whether a mouse is behind the box, so moves, presses and button events say they work even when nothing is plugged in
- if the box is in debug mode, leave it in the vendor tool first, anybox won't connect otherwise
- after being unplugged and plugged back in, that V1 kept answering but ignored every text command (moves, clicks, locks) and i don't know why yet. when you connect, anybox checks for that and tells you (a reconnect just keeps trying), turning debug mode on and off in the vendor tool brought it back once
- locks cover the five buttons and a whole X or Y axis, not one direction of it
- buttons and motion come in one stream, and anybox doesn't know what was already held when you start listening, so a button that's already down when the first report comes in never shows its press, only its release
- the box keeps its curve and interpolation settings, so another app may have changed how your moves come out. anybox never changes them by itself, though you can read and set the interpolation through `device.ysc()`. that comes from the vendor's source and hasn't run on a real one yet
- `device.ysc().serial()` gives you the serial the box shows the PC for the mouse behind it, the custom one, the physical one and the one that's active, also only from the vendor's source
- the vendor app 1.17.0 adds Physical Guard. it's off by default, but once it's on and calibrated, the box drops your moves when they change direction faster than your own hand can, which anybox can't see
- i don't know yet whether events keep coming while something is locked, or what stays held when the port closes or the box loses power

### Ferrum

- a community tester ran anybox on one through the app's port (app 2.0.5), so moves, clicks, the wheel, keys and locks were seen working there, the rest comes from the vendor docs
- in that run keys anybox sent only reached Windows once a key got pressed on the real keyboard, i don't know why yet and it was seen just once
- with the app installed, Windows refused to open the box's own port on that PC even with the app closed, and i don't know what holds it
- anybox does the most on the Ferrum app's virtual port, which needs the app running on Windows. the box's own port (the legacy one, and the only way in from Linux) only moves, clicks, scrolls, locks buttons and whole axes, and reports buttons. Mouse Proxy, controllers and the app's network APIs aren't in anybox yet
- `connect()` doesn't find it by itself since we don't know its USB ids yet, so use `Connect::port` or `.family(Family::Ferrum)`. on Windows that search only takes the app's port, add `.ferrum_api(FerrumApi::Legacy)` if you really want the box's own one. `device.snapshot().info.ferrum_api` says which one you're on
- the app has to answer `km.has_mouse()`, otherwise anybox can't tell what's plugged in and asks you to update it
- on the app's port `info.firmware` is the Ferrum API version, from `ferrum.version()`. the docs only have that since 2.0.6, so an older app gives you none, and so does the box's own port
- keep the app and the box's firmware on the same version, the app tells you when the board needs an update. the vendor says a mismatch is undefined, some combinations happen to work and some don't, and with v1 firmware under a v2 app pretty much every command fails until you click Update Board
- on the box's own port use 2.0.6 or newer. an update before it stopped `km.buttons` from reporting the state with nothing held, so i'm pretty sure the release of the last button held could go missing (the changelog only says the empty state wasn't reported, and not which version broke it)
- by default a release, and the end of a click or a tap, forces it up for 125 to 175 ms before the physical button counts again. the app's No Button Release setting drops the software press right away instead, and anybox can't read that setting
- a click on a button that's still in its press or forced release takes over from it, i don't know yet what two quick clicks in a row look like
- the box lets the physical button win, so pressing and letting go of a button anybox holds really lets it go
- clicks and taps hold for a random 75 to 125 ms, the box has no way to take a hold, so a click or a tap with one is a press and a release timed from your PC, which is what the vendor tells you to do too
- a chord goes out in one frame, the vendor says that may change though. `tap_keys` holds each key for its own random 75 to 125 ms, so `[LEFT_CTRL, A]` can let go of Ctrl before A. none of that ran on a real one yet
- the box has no way to type text, so `type_text` is unsupported there
- the vendor says to scroll one notch at a time, bigger scrolls can look emulated
- the app's Move Multiplier scales every move and anybox can't read it, so leave it at 1 if your moves have to be exact
- the box never says no, so a command it doesn't take just does nothing. anybox only sends what the box says it has
- a mouse or keyboard you set in the app after connecting shows up within a second
- unplugging the box from this PC doesn't disconnect anybox, because it talks to the app's port and that stays open. the app just loses the mouse and keyboard, so they go away from the snapshot with an `Event::Changed` and come back the same way when you plug it back in (the app doing that was seen on a real box). the box forgets its locks over that, so when it's back you also get an `Event::SessionReset`, and anybox puts your locks back and lets go of what you held. a replug quicker than a second can slip past that though
- `info.peripheral` comes from the app's `device.VID()` and `device.PID()`. with nothing plugged in the app answers a default pair that's also a real Logitech receiver, so anybox leaves it empty then
- the legacy port answers exactly like a KMBox B+, so a B+ opens as a legacy Ferrum too. anybox only promises the legacy commands there, and nobody tried it on a B+
- locking one way of an axis needs an app that has directional locks, otherwise (and on the legacy port) you can only lock a whole axis
- i don't know yet whether the events include what anybox sends or only what you do yourself
- `device.ferrum().catch_xy()` gives you how far the real mouse moved over the last bit of time (up to a second), plus what software sent if you ask for it. it only works on the app's port. the vendor's two examples disagree on which way y goes, anybox takes positive down like `move_by` and nobody checked it on a box yet

### MAKCU

- `move_by` goes through the box's own smoothing, so on V4.094 a move shows up about 64 ms after you send it with the mouse still and about 72 ms with it moving. `move_now` skips that, about 4 ms still and 12 ms with a 1 kHz mouse moving, so use it when the delay matters
- that smoothing is the box's interpolation, and `device.makcu()` reads and sets it, 0 to 100 percent or AUTO. K4 measured the delay above on AUTO and about 4 ms at 0 on V4.094. it's global to the box, so it changes `move_by` for every program on it, and anybox leaves it as you set it when you close. i don't know yet whether it survives unplugging the box. `moving()` tells you whether the box is still paying out a move
- the box refuses moves it can't keep up with and an `Event::Rejected` counts the ones it dropped. flooding it as fast as the port goes, about 1,400 a second got through here. paced every millisecond with the mouse still, K4 got all of them through up to 5,000 a second, and with the mouse moving about 2,000 a second. so if you move once per frame at a high frame rate, add up the deltas and send them every millisecond or two instead
- even paced, a few counts go missing when you move a lot while the real mouse moves (about 1 count in 1,300 at 1,000 moves a second), `move_now` didn't lose any. the box also sends extra reports with only your motion in them next to the mouse's own, and while it injects it reads a 1 kHz mouse only about 450 times a second
- before V4.102, a move or click right after the mouse behind it got plugged in is lost until the mouse sends its first report. from V4.102 an ordinary mouse takes it right away, a controller or a mouse that's also something else still waits for that first report
- a key tap without a hold lasts about one report, so some apps miss it, pass a hold with `tap_key_with` if it has to land. a click is fine, the box holds it 35 to 75 ms by itself
- a key press or a chord right after a tap gets refused, because the box is still busy with the tap for about 10 ms. you get an `Event::Rejected` for it, so leave a little gap (20 ms was fine). typing waits for it by itself
- it types about 100 ms a character, so a long text takes a while, and other calls on the device can wait while it types (moves and clicks still get through in between)
- it types the same keys whether Caps Lock is on or not, so with Caps Lock on the case comes out flipped, `aB1!` comes out as `Ab1!`. the snapshot says so with `TypedTextIgnoresCapsLock`
- typing lets go of a key you hold when the text types that key, and of Left Shift when the text needs Shift, a held key the text doesn't touch stays held. anybox takes those off what it holds for you and doesn't press them again, the snapshot says so with `TypingReleasesHeldKeys`
- how many keys it holds at once is up to the device behind it. on the mice and keyboards tried that's 6 next to the 8 modifiers, past that the box refuses the key and you get an `Event::Rejected`
- a chord tap holds its keys 8 to 12 ms
- a lock on a button that's already held still lets its release through
- `force_release_key` is the same key mask `lock_key` uses. K4 measured that mask cutting a held key from the PC in about 1 ms and giving it back about 2 ms after the unlock, on V4.094. i haven't checked it on a MAKCU myself yet
- K4 also saw a button held by the user stay held when the box lets go of its own press of it, so `release_key` most likely leaves a key the user holds alone too
- if your app gets killed, its locks stay on the box until something unlocks them, and a button it held stays down too
- on V4.108 and newer `device.makcu().set_autorelease()` fixes the button part. once nothing reaches the box for that long, it ends a click and lets go of the buttons anybox held, your own buttons and keys stay as they are. anybox reads the box about every 250 ms, so with more than that it only fires when your app dies or hangs. it's off by default, shared with every program on the box and gone when the box loses power, so anybox sends it again after a reconnect
- after a reconnect anybox lets go of what was held, press again if you still need it
- right after a reconnect the mouse behind the box can take a second or so to show up again, so until then the snapshot has no mouse and mouse calls come back unsupported. you get a second `Reconnected` once it's there, so wait for that one
- unplugging the box restarts it, so a button held at that moment stays down in Windows after the reconnect, because the box never sends it the release. anybox can't fix that without clicking for you, a real click on that button clears it

### Medius

- with the mouse still, a move shows up about 1 ms after you send it, `move_by` and `move_now` alike, and none got lost in K4's runs
- if your app gets killed, the box lets go of a button it held
- a lock on a button that's already held keeps it down until you unlock it, and an app on the medius crate saw the same for keys, so to take a held key away from the PC use `force_release_key`, the box's own force release
- checked on 3.4.7 with a keyboard: a key held through `force_release_key` stops reaching the PC, `restore_key` brings it back while it's still held, closing the device looked like it brings it back too, and the key events keep showing the finger the whole time
- `release_key` only takes your own press back, a key the user holds stays down for the PC (the docs say so, i haven't checked it)
- key events carry the box's own time like buttons and motion do
- held buttons, keys and locks come back by themselves after the box reconnects or resets
- `device.medius()` gives you the medius crate's own device, for clips, transforms, rendering, the polling rate (`query_rate()`) and the identity the box shows the PC (`device_info()`)
- it can't type text and has no chords. its clips can press up to 8 keys in one report, but a clip gets loaded first and played by the box, so that stays in `device.medius()`
- anybox doesn't know what you press or lock through `medius()`, and its `reset()` lets go of and unlocks EVERYTHING, what anybox did too
