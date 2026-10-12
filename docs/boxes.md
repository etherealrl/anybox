# what each box can do

every method of `Device`, and what each box does with it. "on your PC" means the box has no way to do it by itself, so anybox does it from your PC out of things the box can do, with your PC's timing

what you get also depends on what's plugged into the box. a mouse without horizontal scroll means no `pan`, and a device with no keyboard interface at all means no keys. `device.snapshot().capabilities` tells you what the box in front of you can do right now, and why when it can't

Ferrum is written from the vendor docs alone, nobody has run it on a real one yet. "app" means only on the Ferrum app's virtual port, the box's own legacy port doesn't have it

## mouse

| method                   | Medius        | MAKCU            | Ferrum           |
| ------------------------ | ------------- | ---------------- | ---------------- |
| `move_by`                | ✓             | ✓                | ✓                |
| `move_now`               | ✓             | V4.086 and up    | ✗                |
| `press`, `release`       | ✓             | ✓                | ✓                |
| `click`, `click_with`    | on your PC    | ✓                | ✓, no hold       |
| `wheel`                  | ✓             | ✓                | ✓                |
| `pan`                    | ✓             | ✗                | ✗                |
| `discard_pending_moves`  | ✓             | ✗                | ✗                |

## keyboard

| method                         | Medius        | MAKCU            | Ferrum           |
| ------------------------------ | ------------- | ---------------- | ---------------- |
| `press_key`, `release_key`     | ✓             | ✓                | app              |
| `tap_key`, `tap_key_with`      | on your PC    | ✓                | app, no hold     |
| `press_media`, `release_media` | ✓             | ✗                | ✗                |
| `tap_media`, `tap_media_with`  | on your PC    | ✗                | ✗                |

a Medius holds as many keys at once as the keyboard behind it can report, six on most of them and no limit on an NKRO one, modifiers not counted

## locks

| method                           | Medius        | MAKCU            | Ferrum           |
| -------------------------------- | ------------- | ---------------- | ---------------- |
| `lock_button`, `unlock_button`   | ✓             | ✓                | ✓                |
| `lock_key`, `unlock_key`         | ✓             | ✓                | app              |
| `lock_axis`, `unlock_axis`       | ✓             | ✓                | ✓, one way alone only on an app that has it |
| `scale_axis`                     | ✓             | ✗                | ✗                |

## events

| event                 | Medius        | MAKCU            | Ferrum           |
| --------------------- | ------------- | ---------------- | ---------------- |
| buttons               | ✓             | ✓                | ✓                |
| motion                | ✓             | ✗                | app              |
| keys                  | ✓             | ✓                | app              |
| media keys            | ✓             | ✗                | ✗                |

on the Medius every event also comes with the box's own timestamp, MAKCU and Ferrum don't send one

`received` is on every event from every box. on MAKCU and Ferrum it's the moment the port read returned, on Medius it's when anybox took the event from the medius crate, which reads the port on its own thread, so a bit later than the read

on Medius each kind of event is its own subscription, so asking for buttons only never makes the box stream motion, and the order between two kinds is only as good as their box time

Ferrum sends the whole state on every change, so anybox works out what changed. when you start listening it doesn't know what was already held, so a button or key you were holding shows up as a press with the next change
