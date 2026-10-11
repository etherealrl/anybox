# what each box can do

every method of `Device`, and what each box does with it. "on your PC" means the box has no way to do it by itself, so anybox does it from your PC out of things the box can do, with your PC's timing

what you get also depends on what's plugged into the box. a mouse without horizontal scroll means no `pan`, and a device with no keyboard interface at all means no keys. `device.snapshot().capabilities` tells you what the box in front of you can do right now, and why when it can't

## mouse

| method                   | Medius        | MAKCU            |
| ------------------------ | ------------- | ---------------- |
| `move_by`                | ✓             | ✓                |
| `move_now`               | ✓             | V4.086 and up    |
| `press`, `release`       | ✓             | ✓                |
| `click`, `click_with`    | on your PC    | ✓                |
| `wheel`                  | ✓             | ✓                |
| `pan`                    | ✓             | ✗                |
| `discard_pending_moves`  | ✓             | ✗                |

## keyboard

| method                         | Medius        | MAKCU            |
| ------------------------------ | ------------- | ---------------- |
| `press_key`, `release_key`     | ✓             | ✓                |
| `tap_key`, `tap_key_with`      | on your PC    | ✓                |
| `press_media`, `release_media` | ✓             | ✗                |
| `tap_media`, `tap_media_with`  | on your PC    | ✗                |

a Medius holds as many keys at once as the keyboard behind it can report, six on most of them and no limit on an NKRO one, modifiers not counted

## locks

| method                           | Medius        | MAKCU            |
| -------------------------------- | ------------- | ---------------- |
| `lock_button`, `unlock_button`   | ✓             | ✓                |
| `lock_key`, `unlock_key`         | ✓             | ✓                |
| `lock_axis`, `unlock_axis`       | ✓             | ✓                |
| `scale_axis`                     | ✓             | ✗                |

## events

| event                 | Medius        | MAKCU            |
| --------------------- | ------------- | ---------------- |
| buttons               | ✓             | ✓                |
| motion                | ✓             | ✗                |
| keys                  | ✓             | ✓                |
| media keys            | ✓             | ✗                |

on the Medius every event also comes with the box's own timestamp, the MAKCU doesn't send one
