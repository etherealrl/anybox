/* the C ABI over anybox, https://github.com/etherealrl/anybox. MPL-2.0

   every call that can fail returns an AnyboxStatus, ANYBOX_STATUS_OK (0) or the reason it failed, and
   anybox_last_error_message() has the text. results come back through the out pointers, which are only
   written when the call worked

   the rules for every pointer, breaking them is a bug in your code like with any C library:
   - a non-null pointer has to be aligned for its type and point at a whole value of it, and out pointers
     can't overlap each other or the inputs
   - a null where a pointer is needed is ANYBOX_STATUS_INVALID_ARGUMENT and never a crash
   - strings are NUL-terminated UTF-8, anything else is ANYBOX_STATUS_INVALID_ARGUMENT
   - every handle gets freed once with its own *_free, never used after, and null frees do nothing

   a panic inside anybox comes back as ANYBOX_STATUS_PANIC and never unwinds into your code, unless the
   library was built with panic = "abort", then it aborts. compare anybox_abi_version() with
   ANYBOX_ABI_VERSION once at start and call nothing else when they differ, the same goes for
   medius_abi_version() and MEDIUS_ABI_VERSION from medius.h, which ships next to this header */

#ifndef ANYBOX_H
#define ANYBOX_H

#pragma once

/* made by scripts/gen-header.sh from anybox-capi, don't edit it by hand */

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>
#include "medius.h"

// bumped on any change to a struct, a constant or a function, so a caller compares it with `anybox_abi_version()`
// once at start and calls nothing else when they differ
#define ANYBOX_ABI_VERSION 1

// `button`, `pressed` and the stamp
#define ANYBOX_EVENT_BUTTON 0

// a physical key, `key`, `pressed` and the stamp
#define ANYBOX_EVENT_KEY 1

// `media`, `pressed` and the stamp
#define ANYBOX_EVENT_MEDIA 2

// physical motion, `dx`, `dy`, `wheel`, `pan` and the stamp. the wheel is positive going up and pan is positive
// going right
#define ANYBOX_EVENT_MOTION 3

// `count` events never reached you since the last one from the same `source`, see `Event::Dropped` in the Rust
// docs
#define ANYBOX_EVENT_DROPPED 4

// the box lost its state, so the buttons and keys you were holding got released (on a box with
// `ANYBOX_QUIRK_KEEPS_HELD` they stay held). what you knew about which physical buttons and keys were held is out
// of date too, because one held through the reset can show a second press or never show its release
#define ANYBOX_EVENT_SESSION_RESET 5

// the box refused `count` of `action` after the call already returned ok, slow down and send it again if it
// mattered
#define ANYBOX_EVENT_REJECTED 6

#define ANYBOX_EVENT_DISCONNECTED 7

// the box is back, the buttons and keys you were holding got released (on a box with `ANYBOX_QUIRK_KEEPS_HELD`
// they stay held), and the snapshot may have changed. on a MAKCU you also get this when a mouse or keyboard gets
// plugged into the box or pulled out, and on a Medius when the device it clones gets swapped. what you knew about
// which physical buttons and keys were held is out of date, same as after `ANYBOX_EVENT_SESSION_RESET`
#define ANYBOX_EVENT_RECONNECTED 8

// the snapshot changed with no reconnect, so read it again
#define ANYBOX_EVENT_CHANGED 9

// the box's own counters
#define ANYBOX_DROP_SOURCE_DEVICE 0

// your receiver fell behind and its queue overflowed
#define ANYBOX_DROP_SOURCE_HOST 1

#define ANYBOX_CLOCK_MEDIUS_HOST_CHIP 0

#define ANYBOX_CLOCK_MEDIUS_DEVICE_CHIP 1

#define ANYBOX_KEY_A 4

#define ANYBOX_KEY_B 5

#define ANYBOX_KEY_C 6

#define ANYBOX_KEY_D 7

#define ANYBOX_KEY_E 8

#define ANYBOX_KEY_F 9

#define ANYBOX_KEY_G 10

#define ANYBOX_KEY_H 11

#define ANYBOX_KEY_I 12

#define ANYBOX_KEY_J 13

#define ANYBOX_KEY_K 14

#define ANYBOX_KEY_L 15

#define ANYBOX_KEY_M 16

#define ANYBOX_KEY_N 17

#define ANYBOX_KEY_O 18

#define ANYBOX_KEY_P 19

#define ANYBOX_KEY_Q 20

#define ANYBOX_KEY_R 21

#define ANYBOX_KEY_S 22

#define ANYBOX_KEY_T 23

#define ANYBOX_KEY_U 24

#define ANYBOX_KEY_V 25

#define ANYBOX_KEY_W 26

#define ANYBOX_KEY_X 27

#define ANYBOX_KEY_Y 28

#define ANYBOX_KEY_Z 29

#define ANYBOX_KEY_NUM1 30

#define ANYBOX_KEY_NUM2 31

#define ANYBOX_KEY_NUM3 32

#define ANYBOX_KEY_NUM4 33

#define ANYBOX_KEY_NUM5 34

#define ANYBOX_KEY_NUM6 35

#define ANYBOX_KEY_NUM7 36

#define ANYBOX_KEY_NUM8 37

#define ANYBOX_KEY_NUM9 38

#define ANYBOX_KEY_NUM0 39

#define ANYBOX_KEY_ENTER 40

#define ANYBOX_KEY_ESCAPE 41

#define ANYBOX_KEY_BACKSPACE 42

#define ANYBOX_KEY_TAB 43

#define ANYBOX_KEY_SPACE 44

#define ANYBOX_KEY_MINUS 45

#define ANYBOX_KEY_EQUAL 46

#define ANYBOX_KEY_LEFT_BRACKET 47

#define ANYBOX_KEY_RIGHT_BRACKET 48

#define ANYBOX_KEY_BACKSLASH 49

#define ANYBOX_KEY_SEMICOLON 51

#define ANYBOX_KEY_APOSTROPHE 52

#define ANYBOX_KEY_GRAVE 53

#define ANYBOX_KEY_COMMA 54

#define ANYBOX_KEY_PERIOD 55

#define ANYBOX_KEY_SLASH 56

#define ANYBOX_KEY_CAPS_LOCK 57

#define ANYBOX_KEY_F1 58

#define ANYBOX_KEY_F2 59

#define ANYBOX_KEY_F3 60

#define ANYBOX_KEY_F4 61

#define ANYBOX_KEY_F5 62

#define ANYBOX_KEY_F6 63

#define ANYBOX_KEY_F7 64

#define ANYBOX_KEY_F8 65

#define ANYBOX_KEY_F9 66

#define ANYBOX_KEY_F10 67

#define ANYBOX_KEY_F11 68

#define ANYBOX_KEY_F12 69

#define ANYBOX_KEY_PRINT_SCREEN 70

#define ANYBOX_KEY_SCROLL_LOCK 71

#define ANYBOX_KEY_PAUSE 72

#define ANYBOX_KEY_INSERT 73

#define ANYBOX_KEY_HOME 74

#define ANYBOX_KEY_PAGE_UP 75

#define ANYBOX_KEY_DELETE 76

#define ANYBOX_KEY_END 77

#define ANYBOX_KEY_PAGE_DOWN 78

#define ANYBOX_KEY_RIGHT 79

#define ANYBOX_KEY_LEFT 80

#define ANYBOX_KEY_DOWN 81

#define ANYBOX_KEY_UP 82

#define ANYBOX_KEY_NUM_LOCK 83

#define ANYBOX_KEY_KEYPAD_DIVIDE 84

#define ANYBOX_KEY_KEYPAD_MULTIPLY 85

#define ANYBOX_KEY_KEYPAD_MINUS 86

#define ANYBOX_KEY_KEYPAD_PLUS 87

#define ANYBOX_KEY_KEYPAD_ENTER 88

#define ANYBOX_KEY_KEYPAD1 89

#define ANYBOX_KEY_KEYPAD2 90

#define ANYBOX_KEY_KEYPAD3 91

#define ANYBOX_KEY_KEYPAD4 92

#define ANYBOX_KEY_KEYPAD5 93

#define ANYBOX_KEY_KEYPAD6 94

#define ANYBOX_KEY_KEYPAD7 95

#define ANYBOX_KEY_KEYPAD8 96

#define ANYBOX_KEY_KEYPAD9 97

#define ANYBOX_KEY_KEYPAD0 98

#define ANYBOX_KEY_KEYPAD_PERIOD 99

#define ANYBOX_KEY_MENU 101

#define ANYBOX_KEY_LEFT_CTRL 224

#define ANYBOX_KEY_LEFT_SHIFT 225

#define ANYBOX_KEY_LEFT_ALT 226

#define ANYBOX_KEY_LEFT_GUI 227

#define ANYBOX_KEY_RIGHT_CTRL 228

#define ANYBOX_KEY_RIGHT_SHIFT 229

#define ANYBOX_KEY_RIGHT_ALT 230

#define ANYBOX_KEY_RIGHT_GUI 231

#define ANYBOX_MEDIA_PLAY 176

#define ANYBOX_MEDIA_PAUSE 177

#define ANYBOX_MEDIA_NEXT_TRACK 181

#define ANYBOX_MEDIA_PREVIOUS_TRACK 182

#define ANYBOX_MEDIA_STOP 183

#define ANYBOX_MEDIA_PLAY_PAUSE 205

#define ANYBOX_MEDIA_MUTE 226

#define ANYBOX_MEDIA_VOLUME_UP 233

#define ANYBOX_MEDIA_VOLUME_DOWN 234

#define ANYBOX_FIRMWARE_NONE 0

#define ANYBOX_FIRMWARE_VERSION 1

#define ANYBOX_FIRMWARE_BUILD 2

// the protocol number a box speaks, which only shows up in the error of a Medius that's too old or too new, never
// in `AnyboxInfo`
#define ANYBOX_FIRMWARE_PROTOCOL 3

#define ANYBOX_FIRMWARE_TEXT 4

#define ANYBOX_ID_NONE 0

#define ANYBOX_ID_MAC 1

#define ANYBOX_ID_USB_SERIAL 2

// `min_us` and `max_us`
#define ANYBOX_QUIRK_FORCED_UP_ON_RELEASE 0

#define ANYBOX_QUIRK_PHYSICAL_OVERRIDE 1

#define ANYBOX_QUIRK_WHOLE_AXIS_ONLY 2

// `count`
#define ANYBOX_QUIRK_NOTCHES_PER_COMMAND 3

#define ANYBOX_QUIRK_FIXED_CLICK_TIMING 4

// `max_us`
#define ANYBOX_QUIRK_MAX_HOLD 5

// `min_us` and `max_us`
#define ANYBOX_QUIRK_BOX_CLICK_TIMING 6

// `click_timing`
#define ANYBOX_QUIRK_HOST_CLICK_TIMING 7

#define ANYBOX_QUIRK_KEEPS_HELD 8

// `count`
#define ANYBOX_QUIRK_MAX_KEYS_HELD 9

#define ANYBOX_QUIRK_KEEPS_LOCKED 10

// no filter in the connect options
#define ANYBOX_FAMILY_ANY 0

#define ANYBOX_FAMILY_MEDIUS 1

#define ANYBOX_FAMILY_MAKCU_V4 2

#define ANYBOX_FAMILY_FERRUM 3

#define ANYBOX_FAMILY_YSC 4

// no filter in the connect options, and in `AnyboxInfo` a box that isn't a Ferrum
#define ANYBOX_FERRUM_API_NONE 0

#define ANYBOX_FERRUM_API_SOFTWARE 1

#define ANYBOX_FERRUM_API_LEGACY 2

#define ANYBOX_BUTTON_LEFT 0

#define ANYBOX_BUTTON_RIGHT 1

#define ANYBOX_BUTTON_MIDDLE 2

#define ANYBOX_BUTTON_SIDE1 3

#define ANYBOX_BUTTON_SIDE2 4

// a button past side2 is this plus its number, from 0 to 255
#define ANYBOX_BUTTON_EXTRA_BASE 256

#define ANYBOX_AXIS_X 0

#define ANYBOX_AXIS_Y 1

#define ANYBOX_AXIS_WHEEL 2

// horizontal scroll
#define ANYBOX_AXIS_PAN 3

#define ANYBOX_DIRECTION_BOTH 0

#define ANYBOX_DIRECTION_POSITIVE 1

#define ANYBOX_DIRECTION_NEGATIVE 2

#define ANYBOX_ACTION_CONNECT 0

#define ANYBOX_ACTION_MOVE 1

#define ANYBOX_ACTION_MOVE_NOW 2

#define ANYBOX_ACTION_PRESS 3

#define ANYBOX_ACTION_RELEASE 4

#define ANYBOX_ACTION_CLICK 5

#define ANYBOX_ACTION_WHEEL 6

#define ANYBOX_ACTION_PAN 7

#define ANYBOX_ACTION_DISCARD_PENDING_MOVES 8

#define ANYBOX_ACTION_BUTTON_EVENTS 9

#define ANYBOX_ACTION_MOTION_EVENTS 10

#define ANYBOX_ACTION_KEY_EVENTS 11

#define ANYBOX_ACTION_MEDIA_EVENTS 12

#define ANYBOX_ACTION_KEY_PRESS 13

#define ANYBOX_ACTION_KEY_RELEASE 14

#define ANYBOX_ACTION_KEY_TAP 15

#define ANYBOX_ACTION_MEDIA_PRESS 16

#define ANYBOX_ACTION_MEDIA_RELEASE 17

#define ANYBOX_ACTION_MEDIA_TAP 18

#define ANYBOX_ACTION_LOCK_BUTTON 19

#define ANYBOX_ACTION_LOCK_AXIS 20

#define ANYBOX_ACTION_LOCK_KEY 21

#define ANYBOX_ACTION_SCALE_AXIS 22

#define ANYBOX_EXECUTION_NATIVE 0

#define ANYBOX_EXECUTION_HOST 1

#define ANYBOX_EXECUTION_UNSUPPORTED 2

#define ANYBOX_CLICK_TIMING_MAKCU 0

#define ANYBOX_CLICK_TIMING_FERRUM 1

#define ANYBOX_CLICK_TIMING_FIXED 2

#define ANYBOX_CLICK_TIMING_OFF 3

// combined with `|` for `anybox_device_events`
#define ANYBOX_EVENTS_BUTTONS 1

#define ANYBOX_EVENTS_MOTION 2

#define ANYBOX_EVENTS_KEYS 4

#define ANYBOX_EVENTS_MEDIA 8

// everything the box can stream, a kind it can't is left out without a `Rejected`
#define ANYBOX_EVENTS_ALL 15

// one open box. every call on it can come from any thread at the same time, only `anybox_device_free` must not
// run while another call on it is still going
typedef struct AnyboxDevice AnyboxDevice;

// a receiver of events from one device. it can be read from any thread, and two threads reading the same one
// split the events between them
typedef struct AnyboxEvents AnyboxEvents;

#if defined(ANYBOX_FEATURE_MOCK)
// the medius crate's model of a Medius box, for testing code that uses anybox with no box plugged in. it proves
// nothing about hardware and it can only be shaped like a Medius
typedef struct AnyboxMock AnyboxMock;
#endif

// every port a scan looked at, in port order
typedef struct AnyboxScanList AnyboxScanList;

// who the box is and what it can do, read from one connection, see `anybox_device_snapshot`
typedef struct AnyboxSnapshot AnyboxSnapshot;

// a list of strings, like the ports `anybox_candidates` gives
typedef struct AnyboxStringList AnyboxStringList;

// what a call did, `ANYBOX_STATUS_OK` or the reason it failed. `anybox_last_error_message` has the text
typedef uint32_t AnyboxStatus;

// how `anybox_device_click` and the taps are timed on a box that can't time them itself. `fixed_us` is only read for
// `ANYBOX_CLICK_TIMING_FIXED`
typedef struct AnyboxClickTiming {
    uint32_t kind;
    uint64_t fixed_us;
} AnyboxClickTiming;

// what `anybox_open`, `anybox_scan` and `anybox_candidates` look for. a struct of zeros is the default too, the
// same as what `anybox_connect_options_default` fills in
typedef struct AnyboxConnectOptions {
    // only this port, whatever its USB id is. null lets anybox look on every port a box could be on
    const char *port;
    // only this family, so the port never gets anything meant for another box
    uint32_t family;
    // only a Ferrum on this API, see `Connect::ferrum_api` in the Rust docs
    uint32_t ferrum_api;
    // skips a box that can't do these, one bit per action (bit n for action n). `ANYBOX_ACTION_CONNECT` matches no
    // box
    uint64_t can;
    // only the box with this id, whatever port it's on
    uint32_t id_kind;
    uint8_t id_mac[6];
    const char *id_serial;
    // what a click does on a box with no click of its own
    struct AnyboxClickTiming click_timing;
} AnyboxConnectOptions;

// how one click or tap goes. with no hold, the box uses its own timing, or the device's click timing if it has
// none
typedef struct AnyboxClick {
    // when anybox times the click on the PC, the call waits this long, so a huge one blocks the thread that long
    uint64_t hold_us;
    bool has_hold;
    // fail instead of letting anybox time it from the PC
    bool native_only;
} AnyboxClick;

// one event. which fields mean something depends on `kind`, the rest are 0. the stamp fields are set on button,
// key, media and motion events
typedef struct AnyboxEvent {
    uint32_t kind;
    uint32_t button;
    uint8_t key;
    bool pressed;
    uint16_t media;
    int32_t dx;
    int32_t dy;
    int32_t wheel;
    int32_t pan;
    uint32_t source;
    uint32_t action;
    uint64_t count;
    // the best guess at when it happened, as the time since the device was opened. it's the one to compare events
    // by
    uint64_t since_open_ns;
    // when anybox got it, as the time since the device was opened
    uint64_t received_since_open_ns;
    // the raw QueryPerformanceCounter value when anybox got it, Windows only
    int64_t received_qpc;
    bool has_received_qpc;
    // the box's own clock, only compare the ones from the same `box_clock`
    bool has_box_time;
    uint32_t box_clock;
    uint32_t box_time_us;
} AnyboxEvent;

// who the box is. every string lives as long as the snapshot it came from
typedef struct AnyboxInfo {
    // the port it was opened on, so you can open the same box again with `port` in the connect options
    const char *port;
    uint32_t family;
    // null when the box doesn't say
    const char *model;
    uint32_t firmware_kind;
    uint16_t firmware_major;
    uint16_t firmware_minor;
    uint16_t firmware_patch;
    uint8_t firmware_protocol;
    uint32_t firmware_build;
    // null unless `firmware_kind` is `ANYBOX_FIRMWARE_TEXT`
    const char *firmware_text;
    uint32_t id_kind;
    uint8_t id_mac[6];
    // null unless `id_kind` is `ANYBOX_ID_USB_SERIAL`
    const char *id_serial;
    // which of Ferrum's two APIs this is, `ANYBOX_FERRUM_API_NONE` on other boxes
    uint32_t ferrum_api;
    // the USB ids of the device plugged into the box, see `Info::peripheral` in the Rust docs for when there's
    // none
    bool has_peripheral;
    uint16_t peripheral_vid;
    uint16_t peripheral_pid;
} AnyboxInfo;

typedef struct AnyboxCapability {
    uint32_t execution;
    // why the box can't, null unless `execution` is `ANYBOX_EXECUTION_UNSUPPORTED`
    const char *reason;
    uint32_t quirk_count;
} AnyboxCapability;

// one way the box behaves differently. which fields mean something depends on `kind`, the rest are 0
typedef struct AnyboxQuirk {
    uint32_t kind;
    uint64_t min_us;
    uint64_t max_us;
    uint32_t count;
    struct AnyboxClickTiming click_timing;
} AnyboxQuirk;

#define ANYBOX_STATUS_OK 0

#define ANYBOX_STATUS_NOT_FOUND 1

#define ANYBOX_STATUS_CLOSED 2

#define ANYBOX_STATUS_PORT_BUSY 3

#define ANYBOX_STATUS_PERMISSION_DENIED 4

#define ANYBOX_STATUS_NO_BOX_CAN 5

#define ANYBOX_STATUS_WRONG_FAMILY 6

#define ANYBOX_STATUS_WRONG_BOX 7

#define ANYBOX_STATUS_WRONG_FERRUM_API 8

#define ANYBOX_STATUS_FIRMWARE_TOO_OLD 9

#define ANYBOX_STATUS_LIBRARY_TOO_OLD 10

#define ANYBOX_STATUS_UNSUPPORTED 11

#define ANYBOX_STATUS_INVALID_ARGUMENT 12

#define ANYBOX_STATUS_REJECTED 13

#define ANYBOX_STATUS_TIMEOUT 14

#define ANYBOX_STATUS_DISCONNECTED 15

#define ANYBOX_STATUS_INTERRUPTED 16

#define ANYBOX_STATUS_IO 17

// anybox panicked inside the call. the handle still works and can be freed, but what the call already did on the
// box isn't known
#define ANYBOX_STATUS_PANIC 18

// an error this C ABI has no status for yet, the message still says what happened
#define ANYBOX_STATUS_UNKNOWN 19

// `anybox_events_recv` got nothing in time
#define ANYBOX_STATUS_RECV_TIMEOUT 20

// `anybox_events_recv` has nothing left, the device is gone and everything queued was read
#define ANYBOX_STATUS_RECV_CLOSED 21

#ifdef __cplusplus
extern "C" {
#endif // __cplusplus

uint32_t anybox_abi_version(void);

// the anybox version this library was built from
const char *anybox_version(void);

// whether this library was built with the mock functions, ask BEFORE calling any of them
bool anybox_has_mock(void);

AnyboxStatus anybox_connect_options_default(struct AnyboxConnectOptions *out);

// connects to the first box found, in port order
AnyboxStatus anybox_connect(struct AnyboxDevice **out);

// opens the first box that fits the options, null options are the defaults. free it with `anybox_device_free`
AnyboxStatus anybox_open(const struct AnyboxConnectOptions *options,
                         struct AnyboxDevice **out);

// every port a search looks at and what's on it, in port order, so a picker can show the boxes you can use and
// why you can't use the others. it opens, describes and closes every box, so it's slow (a YSC search takes about
// 2 s a port), `anybox_candidates` is the cheap one to call often. free it with `anybox_scan_list_free`
AnyboxStatus anybox_scan(const struct AnyboxConnectOptions *options,
                         struct AnyboxScanList **out);

// how many ports the scan looked at, 0 for a null list
uintptr_t anybox_scan_list_count(const struct AnyboxScanList *list);

// the port of entry `index`, null past the end. it lives as long as the list
const char *anybox_scan_list_port(const struct AnyboxScanList *list, uintptr_t index);

// the box on entry `index` as a snapshot you free with `anybox_snapshot_free`, or the error a search of just that
// port gives, with its text in `anybox_last_error_message`. mostly that's busy (the box you already have open
// shows up as busy too, so leave its port out), not allowed, too old, `ANYBOX_STATUS_NO_BOX_CAN` for a box that
// can't do what `can` asked, or `ANYBOX_STATUS_NOT_FOUND` when nothing answered
AnyboxStatus anybox_scan_list_result(const struct AnyboxScanList *list,
                                     uintptr_t index,
                                     struct AnyboxSnapshot **out);

void anybox_scan_list_free(struct AnyboxScanList *list);

// the ports a search would look at, in port order, without opening any of them or sending them a byte. it can't
// tell whether a box is on one, only where one could be, so it's cheap enough to call every second and run
// `anybox_scan` when the list changes. null options are the defaults. free it with `anybox_string_list_free`
AnyboxStatus anybox_candidates(const struct AnyboxConnectOptions *options,
                               struct AnyboxStringList **out);

// 0 for a null list
uintptr_t anybox_string_list_count(const struct AnyboxStringList *list);

// string `index`, null past the end. it lives as long as the list
const char *anybox_string_list_get(const struct AnyboxStringList *list, uintptr_t index);

void anybox_string_list_free(struct AnyboxStringList *list);

// who the box is and what it can do, both read from the current connection. it can change after
// `ANYBOX_EVENT_RECONNECTED` or `ANYBOX_EVENT_CHANGED`. free it with `anybox_snapshot_free`
AnyboxStatus anybox_device_snapshot(const struct AnyboxDevice *device, struct AnyboxSnapshot **out);

// moves by counts, going through whatever smoothing the box has on
AnyboxStatus anybox_device_move_by(const struct AnyboxDevice *device, int16_t dx, int16_t dy);

// moves by counts on the box's next report, skipping its smoothing for this move
AnyboxStatus anybox_device_move_now(const struct AnyboxDevice *device, int16_t dx, int16_t dy);

// holds a button down until `anybox_device_release`
AnyboxStatus anybox_device_press(const struct AnyboxDevice *device, uint32_t button);

AnyboxStatus anybox_device_release(const struct AnyboxDevice *device, uint32_t button);

// the box's own click, or a click timed on the PC with the device's click timing if the box doesn't have one
AnyboxStatus anybox_device_click(const struct AnyboxDevice *device,
                                 uint32_t button);

AnyboxStatus anybox_device_click_with(const struct AnyboxDevice *device,
                                      uint32_t button,
                                      const struct AnyboxClick *click);

// holds a key down until `anybox_device_release_key`. a key is its HID usage, from 0x04 to 0xE7
AnyboxStatus anybox_device_press_key(const struct AnyboxDevice *device, uint8_t key);

AnyboxStatus anybox_device_release_key(const struct AnyboxDevice *device, uint8_t key);

// presses and lets go, timed by the box, or on the PC with the device's click timing if the box can't
AnyboxStatus anybox_device_tap_key(const struct AnyboxDevice *device,
                                   uint8_t key);

AnyboxStatus anybox_device_tap_key_with(const struct AnyboxDevice *device,
                                        uint8_t key,
                                        const struct AnyboxClick *tap);

// holds a media key down until `anybox_device_release_media`. a media key is its HID consumer usage
AnyboxStatus anybox_device_press_media(const struct AnyboxDevice *device, uint16_t key);

AnyboxStatus anybox_device_release_media(const struct AnyboxDevice *device, uint16_t key);

AnyboxStatus anybox_device_tap_media(const struct AnyboxDevice *device, uint16_t key);

AnyboxStatus anybox_device_tap_media_with(const struct AnyboxDevice *device,
                                          uint16_t key,
                                          const struct AnyboxClick *tap);

// keeps the physical button from reaching the PC until `anybox_device_unlock_button`. your own presses still go
// through and its events still come, so you can take the button over
AnyboxStatus anybox_device_lock_button(const struct AnyboxDevice *device,
                                       uint32_t button);

AnyboxStatus anybox_device_unlock_button(const struct AnyboxDevice *device, uint32_t button);

// keeps the physical key from reaching the PC until `anybox_device_unlock_key`
AnyboxStatus anybox_device_lock_key(const struct AnyboxDevice *device, uint8_t key);

AnyboxStatus anybox_device_unlock_key(const struct AnyboxDevice *device, uint8_t key);

// keeps physical motion one way or both ways along an axis from reaching the PC until
// `anybox_device_unlock_axis`. your own moves still go through
AnyboxStatus anybox_device_lock_axis(const struct AnyboxDevice *device,
                                     uint32_t axis,
                                     uint32_t direction);

// unlocks that way along the axis and leaves the other way as it was. it also undoes a scale there
AnyboxStatus anybox_device_unlock_axis(const struct AnyboxDevice *device,
                                       uint32_t axis,
                                       uint32_t direction);

// keeps `percent` of the physical motion one way or both ways along an axis, from -255 to 255. 100 leaves it
// alone, 0 blocks it and -100 inverts it. `anybox_device_unlock_axis` puts it back to 100
AnyboxStatus anybox_device_scale_axis(const struct AnyboxDevice *device,
                                      uint32_t axis,
                                      uint32_t direction,
                                      int16_t percent);

// positive scrolls up
AnyboxStatus anybox_device_wheel(const struct AnyboxDevice *device, int16_t notches);

// horizontal scroll, positive goes right
AnyboxStatus anybox_device_pan(const struct AnyboxDevice *device, int16_t notches);

// throws away the moves the box took but hasn't sent yet
AnyboxStatus anybox_device_discard_pending_moves(const struct AnyboxDevice *device);

// a new receiver for these kinds of input, `ANYBOX_EVENTS_*` combined with `|`, plus the connection events. the
// box only streams while at least one receiver is alive, and it streams what any receiver wants. a kind this box
// can't stream gets one `ANYBOX_EVENT_REJECTED` right away, except with `ANYBOX_EVENTS_ALL`, which means
// everything this box has. just know that a receiver of keys sees every key typed on the keyboard behind the box,
// passwords included. free it with `anybox_events_free`
AnyboxStatus anybox_device_events(const struct AnyboxDevice *device,
                                  uint32_t kinds,
                                  struct AnyboxEvents **out);

// a medius-capi handle for everything only Medius does, used with medius.h and freed with `medius_device_free`.
// it shares this device's connection, and keeping it holds the port open after this device is gone. anybox
// doesn't know about anything you press or lock through it, see `Device::medius` in the Rust docs for the rest
AnyboxStatus anybox_device_medius(const struct AnyboxDevice *device,
                                  MediusDevice **out);

// lets go of what you held, unlocks, and closes the port, and tells you whether that worked. it waits up to about
// 2 s when the box doesn't answer. after it every call fails with `ANYBOX_STATUS_CLOSED`, the receivers end once
// they're read empty, and the snapshot still works. you still free the device after
AnyboxStatus anybox_device_close(const struct AnyboxDevice *device);

// closes the device like `anybox_device_close` if it isn't yet, and frees it. null does nothing
void anybox_device_free(struct AnyboxDevice *device);

// the next event. a negative `timeout_us` waits as long as it takes, 0 doesn't wait at all, and anything else
// waits up to that many microseconds. it's `ANYBOX_STATUS_RECV_TIMEOUT` when nothing came, and
// `ANYBOX_STATUS_RECV_CLOSED` once the device is gone and everything queued was read, so a reader that loops until
// then gets every event. closing the device from another thread wakes it
AnyboxStatus anybox_events_recv(const struct AnyboxEvents *events,
                                int64_t timeout_us,
                                struct AnyboxEvent *out);

// null does nothing. it must not run while another thread is in `anybox_events_recv` on it, close the device to
// wake that one first
void anybox_events_free(struct AnyboxEvents *events);

#if defined(ANYBOX_FEATURE_MOCK)
// a device over a new mock box, a five button mouse with a wheel. it runs the exact code a real Medius gets. null
// `click_timing` is the default. free the mock with `anybox_mock_free` and the device with `anybox_device_free`
AnyboxStatus anybox_mock_medius_open(const struct AnyboxClickTiming *click_timing,
                                     struct AnyboxMock **mock,
                                     struct AnyboxDevice **out);
#endif

#if defined(ANYBOX_FEATURE_MOCK)
// whether anything subscribed to the mock's input yet. a receiver subscribes on its own thread, so wait for this
// before you push, otherwise the box drops what you pushed like a real one would
bool anybox_mock_subscribed(const struct AnyboxMock *mock);
#endif

#if defined(ANYBOX_FEATURE_MOCK)
// physical motion as if from the box. `seq` is the box's frame counter and `ts_us` its clock.
// a gap in `seq` doesn't show up as a drop
AnyboxStatus anybox_mock_push_motion(const struct AnyboxMock *mock,
                                     uint8_t seq,
                                     uint32_t ts_us,
                                     int16_t dx,
                                     int16_t dy,
                                     int16_t wheel,
                                     int16_t pan);
#endif

#if defined(ANYBOX_FEATURE_MOCK)
// the physical buttons held now, as if from the box, bit n for the box's button n, so bits 0 to 4 are left to side2
// and bit n from 5 up comes out as `ANYBOX_BUTTON_EXTRA_BASE` + n. it's the whole held set,
// so a release is pushing the set without that button, and `pressed` says whether the set grew or shrank
AnyboxStatus anybox_mock_push_buttons(const struct AnyboxMock *mock,
                                      uint8_t seq,
                                      uint32_t ts_us,
                                      bool pressed,
                                      uint32_t held);
#endif

#if defined(ANYBOX_FEATURE_MOCK)
// null does nothing
void anybox_mock_free(struct AnyboxMock *mock);
#endif

AnyboxStatus anybox_snapshot_info(const struct AnyboxSnapshot *snapshot, struct AnyboxInfo *out);

// what the box can do for an `ANYBOX_ACTION_*`
AnyboxStatus anybox_snapshot_capability(const struct AnyboxSnapshot *snapshot,
                                        uint32_t action,
                                        struct AnyboxCapability *out);

// quirk number `index` of an action, from 0 up to but not including its `quirk_count`
AnyboxStatus anybox_snapshot_quirk(const struct AnyboxSnapshot *snapshot,
                                   uint32_t action,
                                   uint32_t index,
                                   struct AnyboxQuirk *out);

// the buttons this box has, as `ANYBOX_BUTTON_*`. `len` always gets how many there are and `out` the first
// `cap` of them, so pass a null `out` and a `cap` of 0 to ask for the count first
AnyboxStatus anybox_snapshot_buttons(const struct AnyboxSnapshot *snapshot,
                                     uint32_t *out,
                                     uintptr_t cap,
                                     uintptr_t *len);

// the axes you can lock or scale on this box, as `ANYBOX_AXIS_*`, filled the same way as the buttons
AnyboxStatus anybox_snapshot_axes(const struct AnyboxSnapshot *snapshot,
                                  uint32_t *out,
                                  uintptr_t cap,
                                  uintptr_t *len);

void anybox_snapshot_free(struct AnyboxSnapshot *snapshot);

// what went wrong in the last call that failed on this thread, in UTF-8. it stays valid until the next anybox call
// on this thread that fails, and it's empty when nothing failed yet
const char *anybox_last_error_message(void);

#ifdef __cplusplus
}  // extern "C"
#endif  // __cplusplus

#endif  /* ANYBOX_H */
