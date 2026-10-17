/* the C++ API over anybox, https://github.com/etherealrl/anybox. MPL-2.0

   it's header only and sits on the same library as anybox.h, so there's nothing else to build or link. C++17 and
   up. it does what the Rust crate does with the Rust names, types and structs in PascalCase, functions and
   methods in snake_case, and a call that fails throws an anybox::Error of the class for its status, with the Rust
   error's fields on it. Device and the rest free their C handles themselves, and anything here can be called from
   any thread like the C functions. build with ANYBOX_FEATURE_MOCK defined and link a library built with the mock
   to get anybox::Mock */

#ifndef ANYBOX_HPP
#define ANYBOX_HPP

// windows.h has a DELETE macro, which would turn Key::DELETE into a number in here, and an interface macro, which
// breaks a field of medius.h. both are back the way they were at the end
#pragma push_macro("DELETE")
#pragma push_macro("interface")
#undef DELETE
#undef interface

#include "anybox.h"

#include <array>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <exception>
#include <iterator>
#include <limits>
#include <map>
#include <memory>
#include <mutex>
#include <optional>
#include <set>
#include <stdexcept>
#include <string>
#include <type_traits>
#include <utility>
#include <variant>
#include <vector>

namespace anybox {

enum class Status : std::uint32_t {
    Ok = ANYBOX_STATUS_OK,
    NotFound = ANYBOX_STATUS_NOT_FOUND,
    Closed = ANYBOX_STATUS_CLOSED,
    PortBusy = ANYBOX_STATUS_PORT_BUSY,
    PermissionDenied = ANYBOX_STATUS_PERMISSION_DENIED,
    NoBoxCan = ANYBOX_STATUS_NO_BOX_CAN,
    WrongFamily = ANYBOX_STATUS_WRONG_FAMILY,
    WrongBox = ANYBOX_STATUS_WRONG_BOX,
    WrongFerrumApi = ANYBOX_STATUS_WRONG_FERRUM_API,
    FirmwareTooOld = ANYBOX_STATUS_FIRMWARE_TOO_OLD,
    LibraryTooOld = ANYBOX_STATUS_LIBRARY_TOO_OLD,
    Unsupported = ANYBOX_STATUS_UNSUPPORTED,
    InvalidArgument = ANYBOX_STATUS_INVALID_ARGUMENT,
    Rejected = ANYBOX_STATUS_REJECTED,
    Timeout = ANYBOX_STATUS_TIMEOUT,
    Disconnected = ANYBOX_STATUS_DISCONNECTED,
    Interrupted = ANYBOX_STATUS_INTERRUPTED,
    Io = ANYBOX_STATUS_IO,
    Panic = ANYBOX_STATUS_PANIC,
    Unknown = ANYBOX_STATUS_UNKNOWN,
    RecvTimeout = ANYBOX_STATUS_RECV_TIMEOUT,
    RecvClosed = ANYBOX_STATUS_RECV_CLOSED,
};

enum class Family : std::uint32_t {
    Medius = ANYBOX_FAMILY_MEDIUS,
    MakcuV4 = ANYBOX_FAMILY_MAKCU_V4,
    Ferrum = ANYBOX_FAMILY_FERRUM,
    Ysc = ANYBOX_FAMILY_YSC,
};

/// the two ways into a Ferrum, see `FerrumApi` in the Rust docs
enum class FerrumApi : std::uint32_t {
    Software = ANYBOX_FERRUM_API_SOFTWARE,
    Legacy = ANYBOX_FERRUM_API_LEGACY,
};

/// a physical axis you can lock or scale
enum class Axis : std::uint32_t {
    X = ANYBOX_AXIS_X,
    Y = ANYBOX_AXIS_Y,
    Wheel = ANYBOX_AXIS_WHEEL,
    Pan = ANYBOX_AXIS_PAN,
};

/// positive is the way the box moves for a positive value, so right for X, down for Y, up for the wheel and right
/// for pan
enum class Direction : std::uint32_t {
    Both = ANYBOX_DIRECTION_BOTH,
    Positive = ANYBOX_DIRECTION_POSITIVE,
    Negative = ANYBOX_DIRECTION_NEGATIVE,
};

/// every action a box gets asked whether it can do, and the one an error is about
enum class Action : std::uint32_t {
    Connect = ANYBOX_ACTION_CONNECT,
    Move = ANYBOX_ACTION_MOVE,
    MoveNow = ANYBOX_ACTION_MOVE_NOW,
    Press = ANYBOX_ACTION_PRESS,
    Release = ANYBOX_ACTION_RELEASE,
    Click = ANYBOX_ACTION_CLICK,
    Wheel = ANYBOX_ACTION_WHEEL,
    Pan = ANYBOX_ACTION_PAN,
    DiscardPendingMoves = ANYBOX_ACTION_DISCARD_PENDING_MOVES,
    ButtonEvents = ANYBOX_ACTION_BUTTON_EVENTS,
    MotionEvents = ANYBOX_ACTION_MOTION_EVENTS,
    KeyEvents = ANYBOX_ACTION_KEY_EVENTS,
    MediaEvents = ANYBOX_ACTION_MEDIA_EVENTS,
    KeyPress = ANYBOX_ACTION_KEY_PRESS,
    KeyRelease = ANYBOX_ACTION_KEY_RELEASE,
    KeyTap = ANYBOX_ACTION_KEY_TAP,
    MediaPress = ANYBOX_ACTION_MEDIA_PRESS,
    MediaRelease = ANYBOX_ACTION_MEDIA_RELEASE,
    MediaTap = ANYBOX_ACTION_MEDIA_TAP,
    LockButton = ANYBOX_ACTION_LOCK_BUTTON,
    LockAxis = ANYBOX_ACTION_LOCK_AXIS,
    LockKey = ANYBOX_ACTION_LOCK_KEY,
    ScaleAxis = ANYBOX_ACTION_SCALE_AXIS,
};

enum class Execution : std::uint32_t {
    /// the box does it, with its own timing
    Native = ANYBOX_EXECUTION_NATIVE,
    /// anybox builds it on the PC out of things the box can do, so it runs on the PC's timing
    Host = ANYBOX_EXECUTION_HOST,
    Unsupported = ANYBOX_EXECUTION_UNSUPPORTED,
};

enum class QuirkKind : std::uint32_t {
    ForcedUpOnRelease = ANYBOX_QUIRK_FORCED_UP_ON_RELEASE,
    PhysicalOverride = ANYBOX_QUIRK_PHYSICAL_OVERRIDE,
    WholeAxisOnly = ANYBOX_QUIRK_WHOLE_AXIS_ONLY,
    NotchesPerCommand = ANYBOX_QUIRK_NOTCHES_PER_COMMAND,
    FixedClickTiming = ANYBOX_QUIRK_FIXED_CLICK_TIMING,
    MaxHold = ANYBOX_QUIRK_MAX_HOLD,
    BoxClickTiming = ANYBOX_QUIRK_BOX_CLICK_TIMING,
    HostClickTiming = ANYBOX_QUIRK_HOST_CLICK_TIMING,
    KeepsHeld = ANYBOX_QUIRK_KEEPS_HELD,
    MaxKeysHeld = ANYBOX_QUIRK_MAX_KEYS_HELD,
    KeepsLocked = ANYBOX_QUIRK_KEEPS_LOCKED,
};

enum class FirmwareKind : std::uint32_t {
    Version = ANYBOX_FIRMWARE_VERSION,
    Build = ANYBOX_FIRMWARE_BUILD,
    Protocol = ANYBOX_FIRMWARE_PROTOCOL,
    Text = ANYBOX_FIRMWARE_TEXT,
};

enum class DropSource : std::uint32_t {
    /// the box's own counters
    Device = ANYBOX_DROP_SOURCE_DEVICE,
    /// your receiver fell behind and its queue overflowed
    Host = ANYBOX_DROP_SOURCE_HOST,
};

/// each box clock is its own counter, so only compare stamps from the same one
enum class Clock : std::uint32_t {
    MediusHostChip = ANYBOX_CLOCK_MEDIUS_HOST_CHIP,
    MediusDeviceChip = ANYBOX_CLOCK_MEDIUS_DEVICE_CHIP,
};

/// a box clock counts microseconds in 32 bits, so it rolls over every 71.6 minutes
inline std::chrono::microseconds wraps_after(Clock) noexcept {
    return std::chrono::microseconds(ANYBOX_BOX_CLOCK_WRAP_US);
}

/// what kind of I/O error an `IoError` is, `Other` for every kind this has no number for
enum class IoKind : std::uint32_t {
    Other = ANYBOX_IO_KIND_OTHER,
    NotFound = ANYBOX_IO_KIND_NOT_FOUND,
    PermissionDenied = ANYBOX_IO_KIND_PERMISSION_DENIED,
    TimedOut = ANYBOX_IO_KIND_TIMED_OUT,
    BrokenPipe = ANYBOX_IO_KIND_BROKEN_PIPE,
    WouldBlock = ANYBOX_IO_KIND_WOULD_BLOCK,
    InvalidInput = ANYBOX_IO_KIND_INVALID_INPUT,
    InvalidData = ANYBOX_IO_KIND_INVALID_DATA,
    UnexpectedEof = ANYBOX_IO_KIND_UNEXPECTED_EOF,
    Interrupted = ANYBOX_IO_KIND_INTERRUPTED,
    Unsupported = ANYBOX_IO_KIND_UNSUPPORTED,
    OutOfMemory = ANYBOX_IO_KIND_OUT_OF_MEMORY,
    ResourceBusy = ANYBOX_IO_KIND_RESOURCE_BUSY,
    WriteZero = ANYBOX_IO_KIND_WRITE_ZERO,
    NotConnected = ANYBOX_IO_KIND_NOT_CONNECTED,
};

/// which input a receiver wants, combined with `|`. the connection events and the drops reach every receiver
enum class EventKinds : std::uint32_t {
    NONE = ANYBOX_EVENTS_NONE,
    BUTTONS = ANYBOX_EVENTS_BUTTONS,
    MOTION = ANYBOX_EVENTS_MOTION,
    KEYS = ANYBOX_EVENTS_KEYS,
    MEDIA = ANYBOX_EVENTS_MEDIA,
    /// everything the box can stream, a kind it can't is left out without a `Rejected`
    ALL = ANYBOX_EVENTS_ALL,
};

constexpr EventKinds operator|(EventKinds a, EventKinds b) noexcept {
    return static_cast<EventKinds>(static_cast<std::uint32_t>(a) | static_cast<std::uint32_t>(b));
}

constexpr EventKinds operator&(EventKinds a, EventKinds b) noexcept {
    return static_cast<EventKinds>(static_cast<std::uint32_t>(a) & static_cast<std::uint32_t>(b));
}

enum class ClickTimingKind : std::uint32_t {
    Makcu = ANYBOX_CLICK_TIMING_MAKCU,
    Ferrum = ANYBOX_CLICK_TIMING_FERRUM,
    Fixed = ANYBOX_CLICK_TIMING_FIXED,
    Off = ANYBOX_CLICK_TIMING_OFF,
};

/// any button the box has, `Button::extra(n)` is one past side2, numbered from 5 like `Button::Extra(n)` in Rust
struct Button {
    std::uint32_t value;

    static const Button Left;
    static const Button Right;
    static const Button Middle;
    static const Button Side1;
    static const Button Side2;

    /// any byte like in Rust, the first real one is 5. more throws std::invalid_argument
    static Button extra(int number) {
        if (number < 0 || number > 255) {
            throw std::invalid_argument("an extra button's number is a byte, from 0 to 255, not " +
                                        std::to_string(number) + " (the first real one is 5)");
        }
        return Button{ANYBOX_BUTTON_EXTRA_BASE + static_cast<std::uint32_t>(number)};
    }

    friend constexpr bool operator==(Button a, Button b) noexcept { return a.value == b.value; }
    friend constexpr bool operator!=(Button a, Button b) noexcept { return a.value != b.value; }
    friend constexpr bool operator<(Button a, Button b) noexcept { return a.value < b.value; }
};

// the constants are defined out here, inside the struct it isn't complete yet
inline constexpr Button Button::Left{ANYBOX_BUTTON_LEFT};
inline constexpr Button Button::Right{ANYBOX_BUTTON_RIGHT};
inline constexpr Button Button::Middle{ANYBOX_BUTTON_MIDDLE};
inline constexpr Button Button::Side1{ANYBOX_BUTTON_SIDE1};
inline constexpr Button Button::Side2{ANYBOX_BUTTON_SIDE2};

/// a keyboard key by its HID usage (page 0x07). `Key{0x64}` gets you any other one from 0x04 to 0xE7. just know
/// that windows.h has a DELETE macro, so `Key::DELETE` needs an `#undef DELETE` after it
struct Key {
    std::uint8_t value;

    /// ctrl, shift, alt and gui on either side. a keyboard report keeps these in a byte of their own
    constexpr bool is_modifier() const noexcept {
        return value >= ANYBOX_KEY_MODIFIER_FIRST && value <= ANYBOX_KEY_MODIFIER_LAST;
    }

    friend constexpr bool operator==(Key a, Key b) noexcept { return a.value == b.value; }
    friend constexpr bool operator!=(Key a, Key b) noexcept { return a.value != b.value; }
    friend constexpr bool operator<(Key a, Key b) noexcept { return a.value < b.value; }

    static const Key A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z;
    static const Key NUM1, NUM2, NUM3, NUM4, NUM5, NUM6, NUM7, NUM8, NUM9, NUM0;
    static const Key ENTER, ESCAPE, BACKSPACE, TAB, SPACE, MINUS, EQUAL, LEFT_BRACKET, RIGHT_BRACKET, BACKSLASH;
    static const Key SEMICOLON, APOSTROPHE, GRAVE, COMMA, PERIOD, SLASH, CAPS_LOCK;
    static const Key F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12;
    static const Key PRINT_SCREEN, SCROLL_LOCK, PAUSE, INSERT, HOME, PAGE_UP, DELETE, END, PAGE_DOWN;
    static const Key RIGHT, LEFT, DOWN, UP, NUM_LOCK;
    static const Key KEYPAD_DIVIDE, KEYPAD_MULTIPLY, KEYPAD_MINUS, KEYPAD_PLUS, KEYPAD_ENTER;
    static const Key KEYPAD1, KEYPAD2, KEYPAD3, KEYPAD4, KEYPAD5, KEYPAD6, KEYPAD7, KEYPAD8, KEYPAD9, KEYPAD0;
    static const Key KEYPAD_PERIOD, MENU;
    static const Key LEFT_CTRL, LEFT_SHIFT, LEFT_ALT, LEFT_GUI, RIGHT_CTRL, RIGHT_SHIFT, RIGHT_ALT, RIGHT_GUI;
};

inline constexpr Key Key::A{ANYBOX_KEY_A};
inline constexpr Key Key::B{ANYBOX_KEY_B};
inline constexpr Key Key::C{ANYBOX_KEY_C};
inline constexpr Key Key::D{ANYBOX_KEY_D};
inline constexpr Key Key::E{ANYBOX_KEY_E};
inline constexpr Key Key::F{ANYBOX_KEY_F};
inline constexpr Key Key::G{ANYBOX_KEY_G};
inline constexpr Key Key::H{ANYBOX_KEY_H};
inline constexpr Key Key::I{ANYBOX_KEY_I};
inline constexpr Key Key::J{ANYBOX_KEY_J};
inline constexpr Key Key::K{ANYBOX_KEY_K};
inline constexpr Key Key::L{ANYBOX_KEY_L};
inline constexpr Key Key::M{ANYBOX_KEY_M};
inline constexpr Key Key::N{ANYBOX_KEY_N};
inline constexpr Key Key::O{ANYBOX_KEY_O};
inline constexpr Key Key::P{ANYBOX_KEY_P};
inline constexpr Key Key::Q{ANYBOX_KEY_Q};
inline constexpr Key Key::R{ANYBOX_KEY_R};
inline constexpr Key Key::S{ANYBOX_KEY_S};
inline constexpr Key Key::T{ANYBOX_KEY_T};
inline constexpr Key Key::U{ANYBOX_KEY_U};
inline constexpr Key Key::V{ANYBOX_KEY_V};
inline constexpr Key Key::W{ANYBOX_KEY_W};
inline constexpr Key Key::X{ANYBOX_KEY_X};
inline constexpr Key Key::Y{ANYBOX_KEY_Y};
inline constexpr Key Key::Z{ANYBOX_KEY_Z};
inline constexpr Key Key::NUM1{ANYBOX_KEY_NUM1};
inline constexpr Key Key::NUM2{ANYBOX_KEY_NUM2};
inline constexpr Key Key::NUM3{ANYBOX_KEY_NUM3};
inline constexpr Key Key::NUM4{ANYBOX_KEY_NUM4};
inline constexpr Key Key::NUM5{ANYBOX_KEY_NUM5};
inline constexpr Key Key::NUM6{ANYBOX_KEY_NUM6};
inline constexpr Key Key::NUM7{ANYBOX_KEY_NUM7};
inline constexpr Key Key::NUM8{ANYBOX_KEY_NUM8};
inline constexpr Key Key::NUM9{ANYBOX_KEY_NUM9};
inline constexpr Key Key::NUM0{ANYBOX_KEY_NUM0};
inline constexpr Key Key::ENTER{ANYBOX_KEY_ENTER};
inline constexpr Key Key::ESCAPE{ANYBOX_KEY_ESCAPE};
inline constexpr Key Key::BACKSPACE{ANYBOX_KEY_BACKSPACE};
inline constexpr Key Key::TAB{ANYBOX_KEY_TAB};
inline constexpr Key Key::SPACE{ANYBOX_KEY_SPACE};
inline constexpr Key Key::MINUS{ANYBOX_KEY_MINUS};
inline constexpr Key Key::EQUAL{ANYBOX_KEY_EQUAL};
inline constexpr Key Key::LEFT_BRACKET{ANYBOX_KEY_LEFT_BRACKET};
inline constexpr Key Key::RIGHT_BRACKET{ANYBOX_KEY_RIGHT_BRACKET};
inline constexpr Key Key::BACKSLASH{ANYBOX_KEY_BACKSLASH};
inline constexpr Key Key::SEMICOLON{ANYBOX_KEY_SEMICOLON};
inline constexpr Key Key::APOSTROPHE{ANYBOX_KEY_APOSTROPHE};
inline constexpr Key Key::GRAVE{ANYBOX_KEY_GRAVE};
inline constexpr Key Key::COMMA{ANYBOX_KEY_COMMA};
inline constexpr Key Key::PERIOD{ANYBOX_KEY_PERIOD};
inline constexpr Key Key::SLASH{ANYBOX_KEY_SLASH};
inline constexpr Key Key::CAPS_LOCK{ANYBOX_KEY_CAPS_LOCK};
inline constexpr Key Key::F1{ANYBOX_KEY_F1};
inline constexpr Key Key::F2{ANYBOX_KEY_F2};
inline constexpr Key Key::F3{ANYBOX_KEY_F3};
inline constexpr Key Key::F4{ANYBOX_KEY_F4};
inline constexpr Key Key::F5{ANYBOX_KEY_F5};
inline constexpr Key Key::F6{ANYBOX_KEY_F6};
inline constexpr Key Key::F7{ANYBOX_KEY_F7};
inline constexpr Key Key::F8{ANYBOX_KEY_F8};
inline constexpr Key Key::F9{ANYBOX_KEY_F9};
inline constexpr Key Key::F10{ANYBOX_KEY_F10};
inline constexpr Key Key::F11{ANYBOX_KEY_F11};
inline constexpr Key Key::F12{ANYBOX_KEY_F12};
inline constexpr Key Key::PRINT_SCREEN{ANYBOX_KEY_PRINT_SCREEN};
inline constexpr Key Key::SCROLL_LOCK{ANYBOX_KEY_SCROLL_LOCK};
inline constexpr Key Key::PAUSE{ANYBOX_KEY_PAUSE};
inline constexpr Key Key::INSERT{ANYBOX_KEY_INSERT};
inline constexpr Key Key::HOME{ANYBOX_KEY_HOME};
inline constexpr Key Key::PAGE_UP{ANYBOX_KEY_PAGE_UP};
inline constexpr Key Key::DELETE{ANYBOX_KEY_DELETE};
inline constexpr Key Key::END{ANYBOX_KEY_END};
inline constexpr Key Key::PAGE_DOWN{ANYBOX_KEY_PAGE_DOWN};
inline constexpr Key Key::RIGHT{ANYBOX_KEY_RIGHT};
inline constexpr Key Key::LEFT{ANYBOX_KEY_LEFT};
inline constexpr Key Key::DOWN{ANYBOX_KEY_DOWN};
inline constexpr Key Key::UP{ANYBOX_KEY_UP};
inline constexpr Key Key::NUM_LOCK{ANYBOX_KEY_NUM_LOCK};
inline constexpr Key Key::KEYPAD_DIVIDE{ANYBOX_KEY_KEYPAD_DIVIDE};
inline constexpr Key Key::KEYPAD_MULTIPLY{ANYBOX_KEY_KEYPAD_MULTIPLY};
inline constexpr Key Key::KEYPAD_MINUS{ANYBOX_KEY_KEYPAD_MINUS};
inline constexpr Key Key::KEYPAD_PLUS{ANYBOX_KEY_KEYPAD_PLUS};
inline constexpr Key Key::KEYPAD_ENTER{ANYBOX_KEY_KEYPAD_ENTER};
inline constexpr Key Key::KEYPAD1{ANYBOX_KEY_KEYPAD1};
inline constexpr Key Key::KEYPAD2{ANYBOX_KEY_KEYPAD2};
inline constexpr Key Key::KEYPAD3{ANYBOX_KEY_KEYPAD3};
inline constexpr Key Key::KEYPAD4{ANYBOX_KEY_KEYPAD4};
inline constexpr Key Key::KEYPAD5{ANYBOX_KEY_KEYPAD5};
inline constexpr Key Key::KEYPAD6{ANYBOX_KEY_KEYPAD6};
inline constexpr Key Key::KEYPAD7{ANYBOX_KEY_KEYPAD7};
inline constexpr Key Key::KEYPAD8{ANYBOX_KEY_KEYPAD8};
inline constexpr Key Key::KEYPAD9{ANYBOX_KEY_KEYPAD9};
inline constexpr Key Key::KEYPAD0{ANYBOX_KEY_KEYPAD0};
inline constexpr Key Key::KEYPAD_PERIOD{ANYBOX_KEY_KEYPAD_PERIOD};
inline constexpr Key Key::MENU{ANYBOX_KEY_MENU};
inline constexpr Key Key::LEFT_CTRL{ANYBOX_KEY_LEFT_CTRL};
inline constexpr Key Key::LEFT_SHIFT{ANYBOX_KEY_LEFT_SHIFT};
inline constexpr Key Key::LEFT_ALT{ANYBOX_KEY_LEFT_ALT};
inline constexpr Key Key::LEFT_GUI{ANYBOX_KEY_LEFT_GUI};
inline constexpr Key Key::RIGHT_CTRL{ANYBOX_KEY_RIGHT_CTRL};
inline constexpr Key Key::RIGHT_SHIFT{ANYBOX_KEY_RIGHT_SHIFT};
inline constexpr Key Key::RIGHT_ALT{ANYBOX_KEY_RIGHT_ALT};
inline constexpr Key Key::RIGHT_GUI{ANYBOX_KEY_RIGHT_GUI};

/// a media key by its HID consumer usage (page 0x0C). `MediaKey{n}` gets you any other one
struct MediaKey {
    std::uint16_t value;

    friend constexpr bool operator==(MediaKey a, MediaKey b) noexcept { return a.value == b.value; }
    friend constexpr bool operator!=(MediaKey a, MediaKey b) noexcept { return a.value != b.value; }
    friend constexpr bool operator<(MediaKey a, MediaKey b) noexcept { return a.value < b.value; }

    static const MediaKey PLAY, PAUSE, NEXT_TRACK, PREVIOUS_TRACK, STOP, PLAY_PAUSE, MUTE, VOLUME_UP, VOLUME_DOWN;
};

inline constexpr MediaKey MediaKey::PLAY{ANYBOX_MEDIA_PLAY};
inline constexpr MediaKey MediaKey::PAUSE{ANYBOX_MEDIA_PAUSE};
inline constexpr MediaKey MediaKey::NEXT_TRACK{ANYBOX_MEDIA_NEXT_TRACK};
inline constexpr MediaKey MediaKey::PREVIOUS_TRACK{ANYBOX_MEDIA_PREVIOUS_TRACK};
inline constexpr MediaKey MediaKey::STOP{ANYBOX_MEDIA_STOP};
inline constexpr MediaKey MediaKey::PLAY_PAUSE{ANYBOX_MEDIA_PLAY_PAUSE};
inline constexpr MediaKey MediaKey::MUTE{ANYBOX_MEDIA_MUTE};
inline constexpr MediaKey MediaKey::VOLUME_UP{ANYBOX_MEDIA_VOLUME_UP};
inline constexpr MediaKey MediaKey::VOLUME_DOWN{ANYBOX_MEDIA_VOLUME_DOWN};

namespace detail {

inline std::uint64_t micros(std::chrono::microseconds value, const char* what) {
    if (value.count() < 0) {
        throw std::invalid_argument(std::string(what) + " can't be negative");
    }
    return static_cast<std::uint64_t>(value.count());
}

// a stamp's nanoseconds are unsigned and a std::chrono duration is signed, so the far end clamps instead of wrapping
inline std::chrono::nanoseconds clamp_ns(std::uint64_t nanos) noexcept {
    constexpr auto most = static_cast<std::uint64_t>((std::numeric_limits<std::int64_t>::max)());
    return std::chrono::nanoseconds(static_cast<std::int64_t>(nanos > most ? most : nanos));
}

inline std::string upper_hex(std::uint32_t value, std::size_t digits) {
    static const char symbols[] = "0123456789ABCDEF";
    std::string text(digits, '0');
    for (std::size_t at = digits; at > 0; at--) {
        text[at - 1] = symbols[value & 0xF];
        value >>= 4;
    }
    return text;
}

}  // namespace detail

/// like Rust's Display, "Medius" or "MAKCU V4"
inline std::string to_string(Family family) {
    switch (family) {
    case Family::Medius:
        return "Medius";
    case Family::MakcuV4:
        return "MAKCU V4";
    case Family::Ferrum:
        return "Ferrum";
    case Family::Ysc:
        return "YSC 8K";
    }
    throw std::invalid_argument("anybox has no such family");
}

/// like Rust's Display, "the Ferrum app's port" or "the box's own legacy port"
inline std::string to_string(FerrumApi api) {
    switch (api) {
    case FerrumApi::Software:
        return "the Ferrum app's port";
    case FerrumApi::Legacy:
        return "the box's own legacy port";
    }
    throw std::invalid_argument("a Ferrum has no such API");
}

/// like Rust's Display, "left" or "extra 7"
inline std::string to_string(Button button) {
    switch (button.value) {
    case ANYBOX_BUTTON_LEFT:
        return "left";
    case ANYBOX_BUTTON_RIGHT:
        return "right";
    case ANYBOX_BUTTON_MIDDLE:
        return "middle";
    case ANYBOX_BUTTON_SIDE1:
        return "side1";
    case ANYBOX_BUTTON_SIDE2:
        return "side2";
    default:
        return "extra " + std::to_string(button.value - ANYBOX_BUTTON_EXTRA_BASE);
    }
}

/// like Rust's Display, "X" or "wheel"
inline std::string to_string(Axis axis) {
    switch (axis) {
    case Axis::X:
        return "X";
    case Axis::Y:
        return "Y";
    case Axis::Wheel:
        return "wheel";
    case Axis::Pan:
        return "pan";
    }
    throw std::invalid_argument("a box has no such axis");
}

/// like Rust's Display, "move now" or "lock or unlock a key", the words anybox's errors use
inline std::string to_string(Action action) {
    switch (action) {
    case Action::Connect:
        return "connect";
    case Action::Move:
        return "move";
    case Action::MoveNow:
        return "move now";
    case Action::Press:
        return "press";
    case Action::Release:
        return "release";
    case Action::Click:
        return "click";
    case Action::Wheel:
        return "scroll";
    case Action::Pan:
        return "scroll sideways";
    case Action::DiscardPendingMoves:
        return "discard pending moves";
    case Action::ButtonEvents:
        return "report button events";
    case Action::MotionEvents:
        return "report motion events";
    case Action::KeyEvents:
        return "report key events";
    case Action::MediaEvents:
        return "report media key events";
    case Action::KeyPress:
        return "press a key";
    case Action::KeyRelease:
        return "release a key";
    case Action::KeyTap:
        return "tap a key";
    case Action::MediaPress:
        return "press a media key";
    case Action::MediaRelease:
        return "release a media key";
    case Action::MediaTap:
        return "tap a media key";
    case Action::LockButton:
        return "lock or unlock a button";
    case Action::LockAxis:
        return "lock or unlock an axis";
    case Action::LockKey:
        return "lock or unlock a key";
    case Action::ScaleAxis:
        return "scale an axis";
    }
    throw std::invalid_argument("anybox has no such action");
}

/// like Rust's Display, "key 0x04"
inline std::string to_string(Key key) { return "key 0x" + detail::upper_hex(key.value, 2); }

/// like Rust's Display, "media key 0x00CD"
inline std::string to_string(MediaKey key) { return "media key 0x" + detail::upper_hex(key.value, 4); }

/// what a click and the taps do on a box that can't time them itself. `makcu()` (a random hold from 35 to 75 ms)
/// is the default, `ferrum()` holds 75 to 125 ms, `fixed()` always the same, and with `off()` a click or tap
/// without a hold fails on a box like that
class ClickTiming {
public:
    ClickTiming() noexcept = default;

    static ClickTiming makcu() noexcept { return ClickTiming(ClickTimingKind::Makcu, {}); }
    static ClickTiming ferrum() noexcept { return ClickTiming(ClickTimingKind::Ferrum, {}); }
    static ClickTiming off() noexcept { return ClickTiming(ClickTimingKind::Off, {}); }

    /// a negative hold throws std::invalid_argument
    static ClickTiming fixed(std::chrono::microseconds hold) {
        detail::micros(hold, "a fixed click timing");
        return ClickTiming(ClickTimingKind::Fixed, hold);
    }

    ClickTimingKind kind() const noexcept { return kind_; }

    /// the hold, which only a fixed timing has
    std::optional<std::chrono::microseconds> hold() const noexcept {
        if (kind_ != ClickTimingKind::Fixed) {
            return std::nullopt;
        }
        return hold_;
    }

    AnyboxClickTiming to_c() const noexcept {
        return AnyboxClickTiming{static_cast<std::uint32_t>(kind_), static_cast<std::uint64_t>(hold_.count())};
    }

    friend bool operator==(const ClickTiming& a, const ClickTiming& b) noexcept {
        return a.kind_ == b.kind_ && a.hold_ == b.hold_;
    }
    friend bool operator!=(const ClickTiming& a, const ClickTiming& b) noexcept { return !(a == b); }

private:
    ClickTiming(ClickTimingKind kind, std::chrono::microseconds hold) noexcept : kind_(kind), hold_(hold) {}

    ClickTimingKind kind_ = ClickTimingKind::Makcu;
    std::chrono::microseconds hold_{};
};

/// how one click or tap goes. with no hold, the box uses its own timing, or the device's click timing if it has
/// none. a hold anybox times on the PC blocks the calling thread that long
class Click {
public:
    Click() noexcept = default;

    /// a negative hold throws std::invalid_argument
    static Click hold(std::chrono::microseconds hold) {
        Click click;
        click.raw_.hold_us = detail::micros(hold, "a hold");
        click.raw_.has_hold = true;
        return click;
    }

    /// fails instead of letting anybox time it from the PC
    Click native_only() const noexcept {
        Click click = *this;
        click.raw_.native_only = true;
        return click;
    }

    std::optional<std::chrono::microseconds> hold() const noexcept {
        if (!raw_.has_hold) {
            return std::nullopt;
        }
        return std::chrono::microseconds(static_cast<std::chrono::microseconds::rep>(raw_.hold_us));
    }

    bool is_native_only() const noexcept { return raw_.native_only; }

    /// the C struct, for calling anybox.h yourself
    const AnyboxClick* get() const noexcept { return &raw_; }

private:
    AnyboxClick raw_{0, false, false};
};

/// a box's own id, which stays the same whatever port it's on. it's always exactly one of the two
class StableId {
public:
    static StableId mac(const std::array<std::uint8_t, 6>& mac) {
        StableId id;
        id.mac_ = mac;
        return id;
    }

    static StableId usb_serial(std::string serial) {
        StableId id;
        id.usb_serial_ = std::move(serial);
        return id;
    }

    const std::optional<std::array<std::uint8_t, 6>>& mac() const noexcept { return mac_; }
    const std::optional<std::string>& usb_serial() const noexcept { return usb_serial_; }

    friend bool operator==(const StableId& a, const StableId& b) noexcept {
        return a.mac_ == b.mac_ && a.usb_serial_ == b.usb_serial_;
    }
    friend bool operator!=(const StableId& a, const StableId& b) noexcept { return !(a == b); }

private:
    StableId() = default;

    std::optional<std::array<std::uint8_t, 6>> mac_;
    std::optional<std::string> usb_serial_;
};

/// like Rust's Display, "MAC 01:23:45:67:89:ab" or "USB serial ..."
inline std::string to_string(const StableId& id) {
    if (const auto& mac = id.mac()) {
        static const char digits[] = "0123456789abcdef";
        std::string text = "MAC ";
        for (std::size_t index = 0; index < mac->size(); index++) {
            if (index > 0) {
                text += ':';
            }
            text += digits[(*mac)[index] >> 4];
            text += digits[(*mac)[index] & 0xF];
        }
        return text;
    }
    return "USB serial " + id.usb_serial().value_or("");
}

/// `Version` is major.minor.patch on a box that has one, otherwise `Build` or `Text` says it. `Protocol` only shows
/// up in the error of a Medius that's too old or too new, never in `Info`. only the fields of its kind are set
struct Firmware {
    FirmwareKind kind = FirmwareKind::Version;
    std::uint16_t major = 0;
    std::uint16_t minor = 0;
    std::uint16_t patch = 0;
    std::uint32_t build = 0;
    std::uint8_t protocol = 0;
    std::string text;

    friend bool operator==(const Firmware& a, const Firmware& b) noexcept {
        return a.kind == b.kind && a.major == b.major && a.minor == b.minor && a.patch == b.patch &&
               a.build == b.build && a.protocol == b.protocol && a.text == b.text;
    }
    friend bool operator!=(const Firmware& a, const Firmware& b) noexcept { return !(a == b); }
};

/// like Rust's Display, "3.4.6", "build 42", "protocol 9" or the text
inline std::string to_string(const Firmware& firmware) {
    switch (firmware.kind) {
    case FirmwareKind::Version:
        return std::to_string(firmware.major) + "." + std::to_string(firmware.minor) + "." +
               std::to_string(firmware.patch);
    case FirmwareKind::Build:
        return "build " + std::to_string(firmware.build);
    case FirmwareKind::Protocol:
        return "protocol " + std::to_string(firmware.protocol);
    default:
        return firmware.text;
    }
}

/// a USB vendor and product id
struct UsbIds {
    std::uint16_t vid = 0;
    std::uint16_t pid = 0;
};

/// who the box is, see `Info` in the Rust docs for what each one means
struct Info {
    /// the port it was opened on, so you can open the same box again with `Connect::port`
    std::string port;
    Family family = Family::Medius;
    std::optional<std::string> model;
    std::optional<Firmware> firmware;
    std::optional<StableId> id;
    /// which of Ferrum's two APIs this is, none on other boxes
    std::optional<FerrumApi> ferrum_api;
    /// the USB ids of the device plugged into the box, see `Info::peripheral` in the Rust docs for when there's none
    std::optional<UsbIds> peripheral;
};

/// one way the box behaves differently, `kind` says which, and only the values that kind has are set
struct Quirk {
    QuirkKind kind = QuirkKind::ForcedUpOnRelease;
    std::optional<std::chrono::microseconds> min;
    std::optional<std::chrono::microseconds> max;
    std::optional<std::uint32_t> count;
    std::optional<ClickTiming> click_timing;
};

/// `reason` says why the box can't, only when it's unsupported
struct Capability {
    Execution execution = Execution::Unsupported;
    std::optional<std::string> reason;
    std::vector<Quirk> quirks;

    bool is_supported() const noexcept { return execution != Execution::Unsupported; }
};

/// what this box, on this firmware, with this mouse behind it, can do for every action. `axes()` are the ones you
/// can lock or scale on this box
class Capabilities {
public:
    Capabilities() = default;
    Capabilities(std::map<Action, Capability> actions, std::vector<Button> buttons, std::vector<Axis> axes)
        : actions_(std::move(actions)), buttons_(std::move(buttons)), axes_(std::move(axes)) {}

    /// an action the box doesn't list is unsupported, "this box doesn't have it", like Rust's `get`
    const Capability& get(Action action) const {
        static const Capability missing{Execution::Unsupported, "this box doesn't have it", {}};
        const auto found = actions_.find(action);
        return found == actions_.end() ? missing : found->second;
    }

    const Capability& operator[](Action action) const { return get(action); }

    const std::map<Action, Capability>& actions() const noexcept { return actions_; }
    const std::vector<Button>& buttons() const noexcept { return buttons_; }
    const std::vector<Axis>& axes() const noexcept { return axes_; }

private:
    std::map<Action, Capability> actions_;
    std::vector<Button> buttons_;
    std::vector<Axis> axes_;
};

/// who the box is and what it can do, both read from the same connection
struct Snapshot {
    Info info;
    Capabilities capabilities;
};

struct BoxTime {
    std::uint32_t micros = 0;
    Clock clock = Clock::MediusHostChip;
};

/// `since_open` is the best guess at when it happened, as the time since the device was opened, and the one to
/// compare events by. `received_since_open` is when anybox got it, and `received_qpc` the raw
/// QueryPerformanceCounter value then, on Windows only. `arrived_ns` and `received_ns` are the same two moments in
/// nanoseconds on the clock `monotonic_ns()` reads, so they line up with other devices and the rest of your program
struct Stamp {
    std::chrono::nanoseconds since_open{};
    std::chrono::nanoseconds received_since_open{};
    std::uint64_t arrived_ns = 0;
    std::uint64_t received_ns = 0;
    std::optional<std::int64_t> received_qpc;
    std::optional<BoxTime> box_time;

// std::chrono::steady_clock is anybox's clock with MSVC (QueryPerformanceCounter) and with libstdc++ on Linux
// (CLOCK_MONOTONIC). libc++ on macOS reads CLOCK_MONOTONIC_RAW, which counts sleep where anybox's CLOCK_UPTIME_RAW
// doesn't, so the two would drift apart and these aren't there
#if defined(_MSC_VER) || (defined(__GLIBCXX__) && defined(__linux__))
    /// `arrived_ns` on std::chrono::steady_clock
    std::chrono::steady_clock::time_point arrived() const noexcept { return at(arrived_ns); }

    /// `received_ns` on std::chrono::steady_clock
    std::chrono::steady_clock::time_point received() const noexcept { return at(received_ns); }

private:
    static std::chrono::steady_clock::time_point at(std::uint64_t nanos) noexcept {
        return std::chrono::steady_clock::time_point(
            std::chrono::duration_cast<std::chrono::steady_clock::duration>(detail::clamp_ns(nanos)));
    }
#endif
};

struct ButtonEvent {
    Button button{0};
    bool pressed = false;
    Stamp at;
};

/// a physical key on the keyboard behind the box. just know that you see every key typed there, passwords
/// included
struct KeyEvent {
    Key key{0};
    bool pressed = false;
    Stamp at;
};

struct MediaEvent {
    MediaKey key{0};
    bool pressed = false;
    Stamp at;
};

/// physical motion. the wheel is positive going up and pan is positive going right
struct MotionEvent {
    std::int32_t dx = 0;
    std::int32_t dy = 0;
    std::int32_t wheel = 0;
    std::int32_t pan = 0;
    Stamp at;
};

/// how many events never reached you since the last one from the same source, see `Event::Dropped` in the Rust
/// docs
struct Dropped {
    std::uint64_t count = 0;
    DropSource source = DropSource::Device;
};

/// the box lost its state, so the buttons and keys you were holding got released, unless the box has
/// `QuirkKind::KeepsHeld`. what you knew about which physical ones were held is out of date too
struct SessionReset {};

/// the box refused `count` of `action` after the call already returned, so slow down and send it again if it
/// mattered
struct Rejected {
    Action action = Action::Connect;
    std::uint64_t count = 0;
};

struct Disconnected {};

/// the box is back, what you held got released (unless it has `QuirkKind::KeepsHeld`), and the snapshot may have
/// changed
struct Reconnected {};

/// the snapshot changed with no reconnect, so read it again
struct Changed {};

/// one event from a box, read it with std::visit or std::get_if
using Event = std::variant<ButtonEvent, KeyEvent, MediaEvent, MotionEvent, Dropped, SessionReset, Rejected,
                           Disconnected, Reconnected, Changed>;

/// everything anybox throws, besides std::invalid_argument for a value it refuses before the call and
/// std::bad_alloc. `status()` is the C ABI's number for it, and empty for this header's own errors, like a
/// library that doesn't go with it. each class has the fields of its Rust error
class Error : public std::runtime_error {
public:
    explicit Error(const std::string& message, std::optional<Status> status = std::nullopt)
        : std::runtime_error(message), status_(status) {}

    std::optional<Status> status() const noexcept { return status_; }

    /// throws it again as its own class, for an error that came back as a value like a scan entry's
    [[noreturn]] void raise() const {
        throw_self();
        // throw_self always throws, this only tells the compiler so, it can't tell from a virtual call
        std::terminate();
    }

private:
    virtual void throw_self() const { throw *this; }

    std::optional<Status> status_;
};

/// a box that answered, with the actions it couldn't do
struct Incapable {
    Snapshot box;
    std::set<Action> missing;
};

class NotFoundError : public Error {
public:
    NotFoundError(const std::string& message, std::vector<std::string> tried)
        : Error(message, Status::NotFound), tried_(std::move(tried)) {}
    void throw_self() const override { throw *this; }

    /// every port it looked at
    const std::vector<std::string>& tried() const noexcept { return tried_; }

private:
    std::vector<std::string> tried_;
};

class ClosedError : public Error {
public:
    explicit ClosedError(const std::string& message) : Error(message, Status::Closed) {}
    void throw_self() const override { throw *this; }
};

class PortBusyError : public Error {
public:
    PortBusyError(const std::string& message, std::string port)
        : Error(message, Status::PortBusy), port_(std::move(port)) {}
    void throw_self() const override { throw *this; }

    const std::string& port() const noexcept { return port_; }

private:
    std::string port_;
};

class PermissionDeniedError : public Error {
public:
    PermissionDeniedError(const std::string& message, std::string port)
        : Error(message, Status::PermissionDenied), port_(std::move(port)) {}
    void throw_self() const override { throw *this; }

    const std::string& port() const noexcept { return port_; }

private:
    std::string port_;
};

class NoBoxCanError : public Error {
public:
    NoBoxCanError(const std::string& message, std::vector<Incapable> boxes)
        : Error(message, Status::NoBoxCan), boxes_(std::move(boxes)) {}
    void throw_self() const override { throw *this; }

    /// each box that answered, with what it couldn't do
    const std::vector<Incapable>& boxes() const noexcept { return boxes_; }

private:
    std::vector<Incapable> boxes_;
};

class WrongFamilyError : public Error {
public:
    WrongFamilyError(const std::string& message, std::string port, Family expected, std::optional<Family> found)
        : Error(message, Status::WrongFamily), port_(std::move(port)), expected_(expected), found_(found) {}
    void throw_self() const override { throw *this; }

    const std::string& port() const noexcept { return port_; }
    Family expected() const noexcept { return expected_; }
    std::optional<Family> found() const noexcept { return found_; }

private:
    std::string port_;
    Family expected_;
    std::optional<Family> found_;
};

class WrongBoxError : public Error {
public:
    WrongBoxError(const std::string& message, std::string port, std::optional<StableId> expected,
                  std::optional<StableId> found)
        : Error(message, Status::WrongBox), port_(std::move(port)), expected_(std::move(expected)),
          found_(std::move(found)) {}
    void throw_self() const override { throw *this; }

    const std::string& port() const noexcept { return port_; }
    const std::optional<StableId>& expected() const noexcept { return expected_; }
    const std::optional<StableId>& found() const noexcept { return found_; }

private:
    std::string port_;
    std::optional<StableId> expected_;
    std::optional<StableId> found_;
};

class WrongFerrumApiError : public Error {
public:
    WrongFerrumApiError(const std::string& message, std::string port, FerrumApi expected, FerrumApi found)
        : Error(message, Status::WrongFerrumApi), port_(std::move(port)), expected_(expected), found_(found) {}
    void throw_self() const override { throw *this; }

    const std::string& port() const noexcept { return port_; }
    FerrumApi expected() const noexcept { return expected_; }
    FerrumApi found() const noexcept { return found_; }

private:
    std::string port_;
    FerrumApi expected_;
    FerrumApi found_;
};

class FirmwareTooOldError : public Error {
public:
    FirmwareTooOldError(const std::string& message, Family family, std::optional<Firmware> found,
                        std::optional<Firmware> needed)
        : Error(message, Status::FirmwareTooOld), family_(family), found_(std::move(found)),
          needed_(std::move(needed)) {}
    void throw_self() const override { throw *this; }

    Family family() const noexcept { return family_; }
    const std::optional<Firmware>& found() const noexcept { return found_; }
    const std::optional<Firmware>& needed() const noexcept { return needed_; }

private:
    Family family_;
    std::optional<Firmware> found_;
    std::optional<Firmware> needed_;
};

class LibraryTooOldError : public Error {
public:
    LibraryTooOldError(const std::string& message, Family family, std::optional<Firmware> found)
        : Error(message, Status::LibraryTooOld), family_(family), found_(std::move(found)) {}
    void throw_self() const override { throw *this; }

    Family family() const noexcept { return family_; }
    const std::optional<Firmware>& found() const noexcept { return found_; }

private:
    Family family_;
    std::optional<Firmware> found_;
};

/// `action()` and `family()` are empty when the C ABI refused it on its own
class UnsupportedError : public Error {
public:
    UnsupportedError(const std::string& message, std::optional<Action> action, std::optional<Family> family,
                     std::string reason)
        : Error(message, Status::Unsupported), action_(action), family_(family), reason_(std::move(reason)) {}
    void throw_self() const override { throw *this; }

    std::optional<Action> action() const noexcept { return action_; }
    std::optional<Family> family() const noexcept { return family_; }
    const std::string& reason() const noexcept { return reason_; }

private:
    std::optional<Action> action_;
    std::optional<Family> family_;
    std::string reason_;
};

/// `action()` is empty when the C ABI refused the value before anybox saw it
class InvalidArgumentError : public Error {
public:
    InvalidArgumentError(const std::string& message, std::optional<Action> action, std::string detail)
        : Error(message, Status::InvalidArgument), action_(action), detail_(std::move(detail)) {}
    void throw_self() const override { throw *this; }

    std::optional<Action> action() const noexcept { return action_; }
    const std::string& detail() const noexcept { return detail_; }

private:
    std::optional<Action> action_;
    std::string detail_;
};

class RejectedError : public Error {
public:
    RejectedError(const std::string& message, Action action, std::string detail)
        : Error(message, Status::Rejected), action_(action), detail_(std::move(detail)) {}
    void throw_self() const override { throw *this; }

    Action action() const noexcept { return action_; }
    const std::string& detail() const noexcept { return detail_; }

private:
    Action action_;
    std::string detail_;
};

/// the box didn't answer in time, so a receiver timing out is never this
class BoxTimeoutError : public Error {
public:
    BoxTimeoutError(const std::string& message, Action action) : Error(message, Status::Timeout), action_(action) {}
    void throw_self() const override { throw *this; }

    Action action() const noexcept { return action_; }

private:
    Action action_;
};

class DisconnectedError : public Error {
public:
    explicit DisconnectedError(const std::string& message) : Error(message, Status::Disconnected) {}
    void throw_self() const override { throw *this; }
};

/// the box reconnected or reset in the middle of the call, anybox already let go of it
class BoxInterruptedError : public Error {
public:
    BoxInterruptedError(const std::string& message, Action action)
        : Error(message, Status::Interrupted), action_(action) {}
    void throw_self() const override { throw *this; }

    Action action() const noexcept { return action_; }

private:
    Action action_;
};

/// `os_error()` is the operating system's own code, when it came from there
class IoError : public Error {
public:
    IoError(const std::string& message, IoKind kind, std::optional<std::int32_t> os_error)
        : Error(message, Status::Io), kind_(kind), os_error_(os_error) {}
    void throw_self() const override { throw *this; }

    IoKind kind() const noexcept { return kind_; }
    std::optional<std::int32_t> os_error() const noexcept { return os_error_; }

private:
    IoKind kind_;
    std::optional<std::int32_t> os_error_;
};

/// anybox panicked inside the call. the device still works and can be closed, but what the call already did on
/// the box isn't known
class PanicError : public Error {
public:
    explicit PanicError(const std::string& message) : Error(message, Status::Panic) {}
    void throw_self() const override { throw *this; }
};

/// an error the C ABI has no number for yet, the message still says what happened
class UnknownError : public Error {
public:
    explicit UnknownError(const std::string& message) : Error(message, Status::Unknown) {}
    void throw_self() const override { throw *this; }
};

/// a receiver has nothing left, the device is gone and every event it queued was read. not `ClosedError`, which is
/// a call on a closed device or a closed receiver, like in Rust
class RecvClosedError : public Error {
public:
    explicit RecvClosedError(const std::string& message) : Error(message, Status::RecvClosed) {}
    void throw_self() const override { throw *this; }
};

/// the medius library doesn't go with the medius.h this was built against
class MediusAbiError : public Error {
public:
    explicit MediusAbiError(const std::string& message) : Error(message) {}
    void throw_self() const override { throw *this; }
};

namespace detail {

// the order of the checks matters, a mismatch is built without calling the library again
inline void check_abi(std::uint32_t found, std::uint32_t wanted) {
    if (found != wanted) {
        throw Error("the anybox library speaks C ABI " + std::to_string(found) + " and this header needs " +
                    std::to_string(wanted) + ", so build against the anybox.h that came with this library");
    }
}

// once per program, from every way in that reaches the library first. a refusal leaves it unset, so the next call
// checks again and throws again
inline void ensure() {
    static const bool ready = (check_abi(anybox_abi_version(), ANYBOX_ABI_VERSION), true);
    (void)ready;
}

inline void check_medius_abi(std::uint32_t found, std::uint32_t wanted) {
    if (found != wanted) {
        throw MediusAbiError("the medius library speaks C ABI " + std::to_string(found) + " and medius.h needs " +
                             std::to_string(wanted) + ", so build against the medius.h that came with this library");
    }
}

inline void ensure_medius() {
    static const bool ready = (check_medius_abi(medius_abi_version(), MEDIUS_ABI_VERSION), true);
    (void)ready;
}

inline std::string text(const char* value) { return value ? std::string(value) : std::string(); }

inline std::optional<std::string> optional_text(const char* value) {
    if (!value) {
        return std::nullopt;
    }
    return std::string(value);
}

// a NUL inside would cut the text short on the C side
inline const std::string& c_text(const std::string& value) {
    if (value.find('\0') != std::string::npos) {
        throw std::invalid_argument("the text can't have a NUL character in it");
    }
    return value;
}

// a cast would take any number without a word, and a value this header has no name for means it's older than the
// library
template <class T>
T known(std::uint32_t value, std::uint32_t first, std::uint32_t last, const char* what) {
    if (value < first || value > last) {
        throw Error(std::string("the library sent ") + what + " this header doesn't know (" + std::to_string(value) +
                    "), so build against the anybox.hpp that came with the library");
    }
    return static_cast<T>(value);
}

inline Family family(std::uint32_t value) {
    return known<Family>(value, ANYBOX_FAMILY_MEDIUS, ANYBOX_FAMILY_YSC, "a family");
}

inline std::optional<Family> optional_family(std::uint32_t value) {
    if (value == ANYBOX_FAMILY_ANY) {
        return std::nullopt;
    }
    return family(value);
}

inline FerrumApi ferrum_api(std::uint32_t value) {
    return known<FerrumApi>(value, ANYBOX_FERRUM_API_SOFTWARE, ANYBOX_FERRUM_API_LEGACY, "a Ferrum API");
}

inline Action action(std::uint32_t value) {
    return known<Action>(value, ANYBOX_ACTION_CONNECT, ANYBOX_ACTION_SCALE_AXIS, "an action");
}

inline ClickTiming click_timing(const AnyboxClickTiming& raw) {
    switch (known<ClickTimingKind>(raw.kind, ANYBOX_CLICK_TIMING_MAKCU, ANYBOX_CLICK_TIMING_OFF, "a click timing")) {
    case ClickTimingKind::Makcu:
        return ClickTiming::makcu();
    case ClickTimingKind::Ferrum:
        return ClickTiming::ferrum();
    case ClickTimingKind::Off:
        return ClickTiming::off();
    default:
        return ClickTiming::fixed(
            std::chrono::microseconds(static_cast<std::chrono::microseconds::rep>(raw.fixed_us)));
    }
}

// a firmware and an id come flattened into fields, the same way in AnyboxInfo and AnyboxErrorDetail
inline std::optional<Firmware> firmware(std::uint32_t kind, std::uint16_t major, std::uint16_t minor,
                                        std::uint16_t patch, std::uint8_t protocol, std::uint32_t build,
                                        const char* text_value) {
    if (kind == ANYBOX_FIRMWARE_NONE) {
        return std::nullopt;
    }
    Firmware out;
    out.kind = known<FirmwareKind>(kind, ANYBOX_FIRMWARE_VERSION, ANYBOX_FIRMWARE_TEXT, "a firmware kind");
    switch (out.kind) {
    case FirmwareKind::Version:
        out.major = major;
        out.minor = minor;
        out.patch = patch;
        break;
    case FirmwareKind::Build:
        out.build = build;
        break;
    case FirmwareKind::Protocol:
        out.protocol = protocol;
        break;
    case FirmwareKind::Text:
        out.text = text(text_value);
        break;
    }
    return out;
}

inline std::optional<StableId> id(std::uint32_t kind, const std::uint8_t (&mac)[6], const char* serial) {
    switch (kind) {
    case ANYBOX_ID_NONE:
        return std::nullopt;
    case ANYBOX_ID_MAC:
        return StableId::mac({mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]});
    case ANYBOX_ID_USB_SERIAL:
        return StableId::usb_serial(text(serial));
    default:
        known<std::uint32_t>(kind, ANYBOX_ID_NONE, ANYBOX_ID_USB_SERIAL, "an id kind");
        return std::nullopt;
    }
}

inline Info info(const AnyboxInfo& raw) {
    Info out;
    out.port = text(raw.port);
    out.family = family(raw.family);
    out.model = optional_text(raw.model);
    out.firmware = firmware(raw.firmware_kind, raw.firmware_major, raw.firmware_minor, raw.firmware_patch,
                            raw.firmware_protocol, raw.firmware_build, raw.firmware_text);
    out.id = id(raw.id_kind, raw.id_mac, raw.id_serial);
    if (raw.ferrum_api != ANYBOX_FERRUM_API_NONE) {
        out.ferrum_api = ferrum_api(raw.ferrum_api);
    }
    if (raw.has_peripheral) {
        out.peripheral = UsbIds{raw.peripheral_vid, raw.peripheral_pid};
    }
    return out;
}

inline std::chrono::microseconds micros_of(std::uint64_t value) {
    return std::chrono::microseconds(static_cast<std::chrono::microseconds::rep>(value));
}

inline Quirk quirk(const AnyboxQuirk& raw) {
    Quirk out;
    out.kind = known<QuirkKind>(raw.kind, ANYBOX_QUIRK_FORCED_UP_ON_RELEASE, ANYBOX_QUIRK_KEEPS_LOCKED, "a quirk");
    switch (out.kind) {
    case QuirkKind::ForcedUpOnRelease:
    case QuirkKind::BoxClickTiming:
        out.min = micros_of(raw.min_us);
        out.max = micros_of(raw.max_us);
        break;
    case QuirkKind::MaxHold:
        out.max = micros_of(raw.max_us);
        break;
    case QuirkKind::NotchesPerCommand:
    case QuirkKind::MaxKeysHeld:
        out.count = raw.count;
        break;
    case QuirkKind::HostClickTiming:
        out.click_timing = click_timing(raw.click_timing);
        break;
    default:
        break;
    }
    return out;
}

inline Stamp stamp(const AnyboxEvent& raw) {
    Stamp out;
    out.since_open = clamp_ns(raw.since_open_ns);
    out.received_since_open = clamp_ns(raw.received_since_open_ns);
    out.arrived_ns = raw.arrived_ns;
    out.received_ns = raw.received_ns;
    if (raw.has_received_qpc) {
        out.received_qpc = raw.received_qpc;
    }
    if (raw.has_box_time) {
        out.box_time = BoxTime{
            raw.box_time_us,
            known<Clock>(raw.box_clock, ANYBOX_CLOCK_MEDIUS_HOST_CHIP, ANYBOX_CLOCK_MEDIUS_DEVICE_CHIP, "a box clock")};
    }
    return out;
}

inline Event event(const AnyboxEvent& raw) {
    switch (raw.kind) {
    case ANYBOX_EVENT_BUTTON:
        return ButtonEvent{Button{raw.button}, raw.pressed, stamp(raw)};
    case ANYBOX_EVENT_KEY:
        return KeyEvent{Key{raw.key}, raw.pressed, stamp(raw)};
    case ANYBOX_EVENT_MEDIA:
        return MediaEvent{MediaKey{raw.media}, raw.pressed, stamp(raw)};
    case ANYBOX_EVENT_MOTION:
        return MotionEvent{raw.dx, raw.dy, raw.wheel, raw.pan, stamp(raw)};
    case ANYBOX_EVENT_DROPPED:
        return Dropped{raw.count, known<DropSource>(raw.source, ANYBOX_DROP_SOURCE_DEVICE, ANYBOX_DROP_SOURCE_HOST,
                                                    "a drop source")};
    case ANYBOX_EVENT_SESSION_RESET:
        return SessionReset{};
    case ANYBOX_EVENT_REJECTED:
        return Rejected{action(raw.action), raw.count};
    case ANYBOX_EVENT_DISCONNECTED:
        return Disconnected{};
    case ANYBOX_EVENT_RECONNECTED:
        return Reconnected{};
    case ANYBOX_EVENT_CHANGED:
        return Changed{};
    default:
        known<std::uint32_t>(raw.kind, ANYBOX_EVENT_BUTTON, ANYBOX_EVENT_CHANGED, "an event");
        return Changed{};
    }
}

// the flat fields of a failure, copied out of the C detail before anything can replace it
struct Fields {
    std::optional<Action> action;
    std::optional<Family> family;
    std::optional<Family> expected_family;
    std::optional<Family> found_family;
    std::string port;
    std::string text;
    std::optional<StableId> expected_id;
    std::optional<StableId> found_id;
    std::optional<Firmware> found_firmware;
    std::optional<Firmware> needed_firmware;
    std::optional<FerrumApi> expected_ferrum_api;
    std::optional<FerrumApi> found_ferrum_api;
    std::optional<IoKind> io_kind;
    std::optional<std::int32_t> os_error;
};

inline Fields fields(const AnyboxErrorDetail& raw) {
    Fields out;
    if (raw.has_action) {
        out.action = action(raw.action);
    }
    out.family = optional_family(raw.family);
    out.expected_family = optional_family(raw.expected_family);
    out.found_family = optional_family(raw.found_family);
    out.port = text(raw.port);
    out.text = text(raw.text);
    out.expected_id = id(raw.expected_id_kind, raw.expected_id_mac, raw.expected_id_serial);
    out.found_id = id(raw.found_id_kind, raw.found_id_mac, raw.found_id_serial);
    out.found_firmware =
        firmware(raw.found_firmware_kind, raw.found_firmware_major, raw.found_firmware_minor, raw.found_firmware_patch,
                 raw.found_firmware_protocol, raw.found_firmware_build, raw.found_firmware_text);
    out.needed_firmware = firmware(raw.needed_firmware_kind, raw.needed_firmware_major, raw.needed_firmware_minor,
                                   raw.needed_firmware_patch, raw.needed_firmware_protocol,
                                   raw.needed_firmware_build, raw.needed_firmware_text);
    if (raw.kind == ANYBOX_STATUS_WRONG_FERRUM_API) {
        out.expected_ferrum_api = ferrum_api(raw.expected_ferrum_api);
        out.found_ferrum_api = ferrum_api(raw.found_ferrum_api);
    }
    if (raw.io_kind != ANYBOX_IO_KIND_NONE) {
        out.io_kind = known<IoKind>(raw.io_kind, ANYBOX_IO_KIND_OTHER, ANYBOX_IO_KIND_NOT_CONNECTED,
                                    "an I/O error kind");
    }
    if (raw.has_os_error) {
        out.os_error = raw.os_error;
    }
    return out;
}

// the error of that status with those fields, the one switch every way of building an error goes through
inline std::shared_ptr<Error> make_error(std::uint32_t status, const std::string& message, const Fields& f,
                                         std::vector<std::string> tried, std::vector<Incapable> boxes) {
    switch (status) {
    case ANYBOX_STATUS_NOT_FOUND:
        return std::make_shared<NotFoundError>(message, std::move(tried));
    case ANYBOX_STATUS_CLOSED:
        return std::make_shared<ClosedError>(message);
    case ANYBOX_STATUS_PORT_BUSY:
        return std::make_shared<PortBusyError>(message, f.port);
    case ANYBOX_STATUS_PERMISSION_DENIED:
        return std::make_shared<PermissionDeniedError>(message, f.port);
    case ANYBOX_STATUS_NO_BOX_CAN:
        return std::make_shared<NoBoxCanError>(message, std::move(boxes));
    case ANYBOX_STATUS_WRONG_FAMILY:
        return std::make_shared<WrongFamilyError>(message, f.port, f.expected_family.value_or(Family{}),
                                                  f.found_family);
    case ANYBOX_STATUS_WRONG_BOX:
        return std::make_shared<WrongBoxError>(message, f.port, f.expected_id, f.found_id);
    case ANYBOX_STATUS_WRONG_FERRUM_API:
        return std::make_shared<WrongFerrumApiError>(message, f.port, f.expected_ferrum_api.value_or(FerrumApi{}),
                                                     f.found_ferrum_api.value_or(FerrumApi{}));
    case ANYBOX_STATUS_FIRMWARE_TOO_OLD:
        return std::make_shared<FirmwareTooOldError>(message, f.family.value_or(Family{}), f.found_firmware,
                                                     f.needed_firmware);
    case ANYBOX_STATUS_LIBRARY_TOO_OLD:
        return std::make_shared<LibraryTooOldError>(message, f.family.value_or(Family{}), f.found_firmware);
    case ANYBOX_STATUS_UNSUPPORTED:
        return std::make_shared<UnsupportedError>(message, f.action, f.family, f.text);
    case ANYBOX_STATUS_INVALID_ARGUMENT:
        return std::make_shared<InvalidArgumentError>(message, f.action, f.text);
    case ANYBOX_STATUS_REJECTED:
        return std::make_shared<RejectedError>(message, f.action.value_or(Action{}), f.text);
    case ANYBOX_STATUS_TIMEOUT:
        return std::make_shared<BoxTimeoutError>(message, f.action.value_or(Action{}));
    case ANYBOX_STATUS_DISCONNECTED:
        return std::make_shared<DisconnectedError>(message);
    case ANYBOX_STATUS_INTERRUPTED:
        return std::make_shared<BoxInterruptedError>(message, f.action.value_or(Action{}));
    case ANYBOX_STATUS_IO:
        return std::make_shared<IoError>(message, f.io_kind.value_or(IoKind::Other), f.os_error);
    case ANYBOX_STATUS_PANIC:
        return std::make_shared<PanicError>(message);
    case ANYBOX_STATUS_RECV_CLOSED:
        return std::make_shared<RecvClosedError>(message);
    default:
        return std::make_shared<UnknownError>(message);
    }
}

inline std::shared_ptr<Error> last_error(std::uint32_t status);

inline void check(std::uint32_t status) {
    if (status != ANYBOX_STATUS_OK) {
        last_error(status)->raise();
    }
}

template <class T, void (*Free)(T*)>
struct Freer {
    void operator()(T* handle) const noexcept { Free(handle); }
};

using SnapshotOwner = std::unique_ptr<AnyboxSnapshot, Freer<AnyboxSnapshot, anybox_snapshot_free>>;
using ScanListOwner = std::unique_ptr<AnyboxScanList, Freer<AnyboxScanList, anybox_scan_list_free>>;
using StringListOwner = std::unique_ptr<AnyboxStringList, Freer<AnyboxStringList, anybox_string_list_free>>;

// the count first, then the values, the way the snapshot's and the scan list's lists are filled
template <class Fill>
std::vector<std::uint32_t> numbers(Fill fill) {
    std::uintptr_t length = 0;
    check(fill(nullptr, 0, &length));
    std::vector<std::uint32_t> values(length);
    check(fill(values.data(), values.size(), &length));
    values.resize(length);
    return values;
}

// reads the whole snapshot handle and frees it, so nothing has to keep it alive
inline Snapshot snapshot(AnyboxSnapshot* raw) {
    SnapshotOwner owned(raw);
    AnyboxInfo raw_info{};
    check(anybox_snapshot_info(owned.get(), &raw_info));
    Snapshot out;
    out.info = info(raw_info);
    std::map<Action, Capability> actions;
    for (std::uint32_t number = ANYBOX_ACTION_CONNECT; number <= ANYBOX_ACTION_SCALE_AXIS; number++) {
        AnyboxCapability raw_capability{};
        check(anybox_snapshot_capability(owned.get(), number, &raw_capability));
        Capability capability;
        capability.execution = known<Execution>(raw_capability.execution, ANYBOX_EXECUTION_NATIVE,
                                                ANYBOX_EXECUTION_UNSUPPORTED, "an execution");
        capability.reason = optional_text(raw_capability.reason);
        for (std::uint32_t index = 0; index < raw_capability.quirk_count; index++) {
            AnyboxQuirk raw_quirk{};
            check(anybox_snapshot_quirk(owned.get(), number, index, &raw_quirk));
            capability.quirks.push_back(quirk(raw_quirk));
        }
        actions.emplace(static_cast<Action>(number), std::move(capability));
    }
    std::vector<Button> buttons;
    for (const auto value : numbers([&](std::uint32_t* out_values, std::uintptr_t cap, std::uintptr_t* len) {
             return anybox_snapshot_buttons(owned.get(), out_values, cap, len);
         })) {
        buttons.push_back(Button{value});
    }
    std::vector<Axis> axes;
    for (const auto value : numbers([&](std::uint32_t* out_values, std::uintptr_t cap, std::uintptr_t* len) {
             return anybox_snapshot_axes(owned.get(), out_values, cap, len);
         })) {
        axes.push_back(known<Axis>(value, ANYBOX_AXIS_X, ANYBOX_AXIS_PAN, "an axis"));
    }
    out.capabilities = Capabilities(std::move(actions), std::move(buttons), std::move(axes));
    return out;
}

inline std::vector<std::string> strings(const AnyboxStringList* list) {
    std::vector<std::string> out;
    const auto count = anybox_string_list_count(list);
    for (std::uintptr_t index = 0; index < count; index++) {
        out.push_back(text(anybox_string_list_get(list, index)));
    }
    return out;
}

// what the box on a scan list entry couldn't do
inline std::set<Action> missing(const AnyboxScanList* list, std::uintptr_t index) {
    std::set<Action> out;
    for (const auto value : numbers([&](std::uint32_t* out_values, std::uintptr_t cap, std::uintptr_t* len) {
             return anybox_scan_list_missing(list, index, out_values, cap, len);
         })) {
        out.insert(action(value));
    }
    return out;
}

inline std::vector<Incapable> incapable(const AnyboxScanList* list) {
    std::vector<Incapable> out;
    const auto count = anybox_scan_list_count(list);
    for (std::uintptr_t index = 0; index < count; index++) {
        AnyboxSnapshot* raw = nullptr;
        check(anybox_scan_list_result(list, index, &raw));
        auto box = snapshot(raw);
        out.push_back(Incapable{std::move(box), missing(list, index)});
    }
    return out;
}

// the last error is thread local on the C side, so this has to run right after the call that failed, on its
// thread. the flat fields come first, then the lists, then the exception, so a failure on the way can't mix two
// errors. the fields only count when the library set them for this status, otherwise they'd be a stale failure's
inline std::shared_ptr<Error> last_error(std::uint32_t status) {
    std::string message = text(anybox_last_error_message());
    if (message.empty()) {
        message = "anybox failed with status " + std::to_string(status);
    }
    AnyboxErrorDetail raw{};
    anybox_last_error_detail(&raw);
    const Fields found = raw.kind == status ? fields(raw) : Fields{};
    std::vector<std::string> tried;
    std::vector<Incapable> boxes;
    if (status == ANYBOX_STATUS_NOT_FOUND) {
        AnyboxStringList* list = nullptr;
        check(anybox_last_error_tried(&list));
        StringListOwner owned(list);
        tried = strings(owned.get());
    }
    if (status == ANYBOX_STATUS_NO_BOX_CAN) {
        AnyboxScanList* list = nullptr;
        check(anybox_last_error_incapable(&list));
        ScanListOwner owned(list);
        boxes = incapable(owned.get());
    }
    return make_error(status, message, found, std::move(tried), std::move(boxes));
}

}  // namespace detail

}  // namespace anybox

#include "anybox_medius.hpp"

namespace anybox {

/// a receiver of events from one device. it can be read from any thread, two threads reading the same one split
/// the events between them, and a for loop over it goes until the device is gone and everything queued was read.
/// like in Rust it doesn't keep its device open
class Events {
    // a read takes a copy of the handle under the lock and reads without it, so `close()` can drop the receiver's
    // own copy at any time and the C handle only goes once no read is in it
    struct State {
        std::mutex lock;
        bool closed = false;
        std::shared_ptr<AnyboxEvents> handle;
    };

public:
    /// takes the handle over
    explicit Events(AnyboxEvents* events) {
        // owned first, so the handle gets freed even when the state's own allocation fails
        std::shared_ptr<AnyboxEvents> handle(events, anybox_events_free);
        state_ = std::make_unique<State>();
        state_->handle = std::move(handle);
    }

    /// the next event, waiting as long as it takes. it throws RecvClosedError once the device is gone and everything
    /// queued was read, and ClosedError once this receiver was closed
    Event recv() const {
        for (;;) {
            if (auto next = wait(-1)) {
                return std::move(*next);
            }
        }
    }

    /// the next event, waiting up to `timeout`, and nothing when nothing came. a negative one throws
    /// std::invalid_argument
    std::optional<Event> recv(std::chrono::microseconds timeout) const {
        return wait(static_cast<std::int64_t>(detail::micros(timeout, "a timeout")));
    }

    std::optional<Event> try_recv() const { return recv(std::chrono::microseconds(0)); }

    /// ends this receiver now, a `recv()` waiting on it from another thread throws ClosedError right away, what
    /// was queued is thrown out, and the box stops streaming what only this receiver wanted. it gets freed once no
    /// read is in it, and a second close does nothing
    void close() const {
        if (!state_) {
            throw ClosedError("this receiver was moved from, use the one it was moved to");
        }
        std::shared_ptr<AnyboxEvents> handle;
        {
            std::lock_guard<std::mutex> guard(state_->lock);
            if (state_->closed) {
                return;
            }
            state_->closed = true;
            handle = std::move(state_->handle);
        }
        // after the flag, so a reader the close wakes already sees it. a close that fails still lets this copy go,
        // and the C handle goes with the last read holding one
        detail::check(anybox_events_close(handle.get()));
    }

    /// reads until the device is gone and everything queued was read, or this receiver gets closed. any other
    /// error gets thrown out of the loop
    class iterator {
    public:
        using iterator_category = std::input_iterator_tag;
        using value_type = Event;
        using difference_type = std::ptrdiff_t;
        using pointer = Event*;
        using reference = Event&;

        iterator() = default;

        reference operator*() { return *current_; }
        pointer operator->() { return &*current_; }

        iterator& operator++() {
            next();
            return *this;
        }

        /// the iterator as it was, holding the event it was on, so `*it++` works like on any input iterator
        iterator operator++(int) {
            iterator before = *this;
            next();
            return before;
        }

        friend bool operator==(const iterator& a, const iterator& b) noexcept { return a.events_ == b.events_; }
        friend bool operator!=(const iterator& a, const iterator& b) noexcept { return !(a == b); }

    private:
        friend class Events;

        explicit iterator(const Events* events) : events_(events) { next(); }

        void next() {
            try {
                current_ = events_->recv();
            } catch (const RecvClosedError&) {
                end();
            } catch (const ClosedError&) {
                end();
            }
        }

        void end() noexcept {
            current_.reset();
            events_ = nullptr;
        }

        const Events* events_ = nullptr;
        std::optional<Event> current_;
    };

    iterator begin() const { return iterator(this); }
    iterator end() const noexcept { return iterator(); }

private:
    std::optional<Event> wait(std::int64_t timeout_us) const {
        std::shared_ptr<AnyboxEvents> handle;
        if (state_) {
            std::lock_guard<std::mutex> guard(state_->lock);
            handle = state_->handle;
        }
        if (!handle) {
            throw ClosedError("this receiver was closed, get a new one with device.events()");
        }
        AnyboxEvent raw{};
        const auto status = anybox_events_recv(handle.get(), timeout_us, &raw);
        {
            // closed while it waited, so whatever came back belongs to a receiver that's gone
            std::lock_guard<std::mutex> guard(state_->lock);
            if (state_->closed) {
                throw ClosedError("this receiver was closed, get a new one with device.events()");
            }
        }
        if (status == ANYBOX_STATUS_OK) {
            return detail::event(raw);
        }
        if (status != ANYBOX_STATUS_RECV_TIMEOUT) {
            detail::check(status);
        }
        return std::nullopt;
    }

    std::unique_ptr<State> state_;
};

/// one open box. every method can be called from any thread at the same time, and a copy shares the box. the last
/// copy to go lets go of what you held, unlocks and closes the port, `close()` does it now and throws when that
/// didn't work
class Device {
public:
    /// takes a handle from anybox.h over
    explicit Device(AnyboxDevice* device) : handle_(device, anybox_device_free) {}

    /// the handle for anybox.h, which throws ClosedError once this was moved from
    const AnyboxDevice* get() const {
        if (!handle_) {
            throw ClosedError("this device was moved from, use the one it was moved to");
        }
        return handle_.get();
    }

    /// who the box is and what it can do, read from the current connection. it can change after `Reconnected` or
    /// `Changed`, and it still works after `close()`
    Snapshot snapshot() const {
        AnyboxSnapshot* raw = nullptr;
        detail::check(anybox_device_snapshot(get(), &raw));
        return detail::snapshot(raw);
    }

    // made by bindings/gen from its call table (methods), change the table and run cargo run -p anybox-gen
    /// moves by counts, going through whatever smoothing the box has on
    void move_by(std::int16_t dx, std::int16_t dy) const { detail::check(anybox_device_move_by(get(), dx, dy)); }

    /// moves by counts on the box's next report, skipping its smoothing for this move
    void move_now(std::int16_t dx, std::int16_t dy) const { detail::check(anybox_device_move_now(get(), dx, dy)); }

    /// holds a button down until `release`
    void press(Button button) const { detail::check(anybox_device_press(get(), button.value)); }

    /// lets go of a button
    void release(Button button) const { detail::check(anybox_device_release(get(), button.value)); }

    /// the box's own click, or one timed on the PC with the device's click timing if the box has none, which blocks
    /// the thread for the hold. `click` says how long to hold it and whether to fail instead of letting anybox time
    /// it from the PC
    void click(Button button, const Click& click = {}) const {
        detail::check(anybox_device_click_with(get(), button.value, click.get()));
    }

    /// holds a key down until `release_key`. a key is its HID usage, from 0x04 to 0xE7
    void press_key(Key key) const { detail::check(anybox_device_press_key(get(), key.value)); }

    /// lets go of a key
    void release_key(Key key) const { detail::check(anybox_device_release_key(get(), key.value)); }

    /// presses and lets go, timed by the box, or on the PC with the device's click timing if the box can't, which
    /// blocks the thread for the hold. `tap` says how long to hold it and whether to fail instead of letting anybox
    /// time it from the PC
    void tap_key(Key key, const Click& tap = {}) const {
        detail::check(anybox_device_tap_key_with(get(), key.value, tap.get()));
    }

    /// holds a media key down until `release_media`. a media key is its HID consumer usage
    void press_media(MediaKey key) const { detail::check(anybox_device_press_media(get(), key.value)); }

    /// lets go of a media key
    void release_media(MediaKey key) const { detail::check(anybox_device_release_media(get(), key.value)); }

    /// presses and lets go of a media key, timed like a key's tap, which blocks the thread for the hold. `tap` says
    /// how long to hold it and whether to fail instead of letting anybox time it from the PC
    void tap_media(MediaKey key, const Click& tap = {}) const {
        detail::check(anybox_device_tap_media_with(get(), key.value, tap.get()));
    }

    /// keeps the physical button from reaching the PC until `unlock_button`. your own presses still go through and
    /// its events still come, so you can take the button over
    void lock_button(Button button) const { detail::check(anybox_device_lock_button(get(), button.value)); }

    /// lets the physical button through again
    void unlock_button(Button button) const { detail::check(anybox_device_unlock_button(get(), button.value)); }

    /// keeps the physical key from reaching the PC until `unlock_key`
    void lock_key(Key key) const { detail::check(anybox_device_lock_key(get(), key.value)); }

    /// lets the physical key through again
    void unlock_key(Key key) const { detail::check(anybox_device_unlock_key(get(), key.value)); }

    /// keeps physical motion one way or both ways along an axis from reaching the PC until `unlock_axis`. your own
    /// moves still go through
    void lock_axis(Axis axis, Direction direction = Direction::Both) const {
        detail::check(anybox_device_lock_axis(get(), static_cast<std::uint32_t>(axis), static_cast<std::uint32_t>(direction)));
    }

    /// unlocks that way along the axis and leaves the other way as it was. it also undoes a scale there
    void unlock_axis(Axis axis, Direction direction = Direction::Both) const {
        detail::check(anybox_device_unlock_axis(get(), static_cast<std::uint32_t>(axis), static_cast<std::uint32_t>(direction)));
    }

    /// keeps `percent` of the physical motion one way or both ways along an axis, from -255 to 255. 100 leaves it
    /// alone, 0 blocks it and -100 inverts it. `unlock_axis` puts it back to 100
    void scale_axis(Axis axis, Direction direction, std::int16_t percent) const {
        detail::check(anybox_device_scale_axis(get(), static_cast<std::uint32_t>(axis), static_cast<std::uint32_t>(direction), percent));
    }

    /// positive scrolls up
    void wheel(std::int16_t notches) const { detail::check(anybox_device_wheel(get(), notches)); }

    /// horizontal scroll, positive goes right
    void pan(std::int16_t notches) const { detail::check(anybox_device_pan(get(), notches)); }

    /// throws away the moves the box took but hasn't sent yet
    void discard_pending_moves() const { detail::check(anybox_device_discard_pending_moves(get())); }

    // the end of what bindings/gen makes (methods)

    /// a new receiver for these kinds of input, plus the connection events. the box only streams while at least one
    /// receiver is alive. a kind this box can't stream gets one `Rejected` right away, except with
    /// `EventKinds::ALL`, which means everything this box has. just know that a receiver of keys sees every key
    /// typed on the keyboard behind the box, passwords included
    Events events(EventKinds kinds = EventKinds::ALL) const {
        if ((static_cast<std::uint32_t>(kinds) & ~static_cast<std::uint32_t>(EventKinds::ALL)) != 0) {
            throw std::invalid_argument("these event kinds have one this header doesn't know");
        }
        AnyboxEvents* raw = nullptr;
        detail::check(anybox_device_events(get(), static_cast<std::uint32_t>(kinds), &raw));
        return Events(raw);
    }

    /// on a Medius, its own device for everything only Medius does (clips, transforms, rendering, the polling rate
    /// and so on), and nothing on any other box or once this device is closed. it shares this connection, anybox
    /// doesn't know about anything you press or lock through it, and its reset lets go of and unlocks EVERYTHING,
    /// what anybox did too
    std::optional<::anybox::medius::Device> medius() const {
        MediusDevice* raw = nullptr;
        const auto status = anybox_device_medius(get(), &raw);
        if (status == ANYBOX_STATUS_UNSUPPORTED || status == ANYBOX_STATUS_CLOSED) {
            return std::nullopt;
        }
        detail::check(status);
        // asked after the box, so another box or a closed device is none like in Rust even when the library's medius
        // side doesn't match this header. the device frees the handle if the check throws
        ::anybox::medius::Device box(raw);
        detail::ensure_medius();
        return box;
    }

    /// lets go of what you held, unlocks, and closes the port, and throws if that didn't work. it waits up to about
    /// 2 s when the box doesn't answer. after it every command throws ClosedError and the receivers end once they're
    /// read empty, `events()` gives one that's already ended, `medius()` nothing, and `snapshot()` still works. a
    /// second close throws ClosedError, like in Rust
    void close() const { detail::check(anybox_device_close(get())); }

private:
    std::shared_ptr<AnyboxDevice> handle_;
};

/// one port a scan looked at, with the box on it in `found`, or in `error` why a search of just that port fails.
/// mostly that's busy (the box you already have open shows up as busy too), not allowed, too old, a NoBoxCanError
/// for a box that can't do what `can` asked, or a NotFoundError when nothing answered. `error->raise()` throws it
struct Scanned {
    std::string port;
    std::optional<Snapshot> found;
    std::shared_ptr<const Error> error;
};

class Connect;

namespace detail {

// the C options and the strings and the list they point into, built in place, since a move would leave the
// pointers on the old strings
class Options {
public:
    explicit Options(const Connect& connect);
    Options(const Options&) = delete;
    Options& operator=(const Options&) = delete;

    const AnyboxConnectOptions* get() const noexcept { return &raw_; }

private:
    std::string port_;
    std::string serial_;
    std::vector<std::uint32_t> can_;
    AnyboxConnectOptions raw_{};
};

}  // namespace detail

/// what to look for, `Connect::auto()` in Rust when it's made with nothing. `port()` only asks that port,
/// `family()` only that kind of box, `ferrum_api()` only a Ferrum on that API, `can()` skips a box that can't do
/// that action (call it again for each one), `id()` only takes the box with that id, and `click_timing()` is what
/// a click does on a box with no click of its own
class Connect {
public:
    Connect() = default;

    static Connect port(std::string port) {
        Connect connect;
        connect.port_ = std::move(port);
        return connect;
    }

    Connect& can(Action action) {
        can_.push_back(action);
        return *this;
    }

    Connect& family(Family family) {
        family_ = family;
        return *this;
    }

    Connect& ferrum_api(FerrumApi api) {
        ferrum_api_ = api;
        return *this;
    }

    Connect& id(StableId id) {
        id_ = std::move(id);
        return *this;
    }

    Connect& click_timing(ClickTiming click_timing) {
        click_timing_ = click_timing;
        return *this;
    }

    /// opens the first box that fits, in port order
    Device open() const {
        detail::ensure();
        const detail::Options options(*this);
        AnyboxDevice* raw = nullptr;
        detail::check(anybox_open(options.get(), &raw));
        return Device(raw);
    }

    /// every port a search looks at and what's on it, in port order. it opens, describes and closes every box, so
    /// it's slow (a YSC search takes about 2 s a port), `candidates()` is the cheap one to call often
    std::vector<Scanned> scan() const {
        detail::ensure();
        const detail::Options options(*this);
        AnyboxScanList* raw = nullptr;
        detail::check(anybox_scan(options.get(), &raw));
        const detail::ScanListOwner list(raw);
        std::vector<Scanned> out;
        const auto count = anybox_scan_list_count(list.get());
        for (std::uintptr_t index = 0; index < count; index++) {
            Scanned entry;
            entry.port = detail::text(anybox_scan_list_port(list.get(), index));
            AnyboxSnapshot* box = nullptr;
            const auto status = anybox_scan_list_result(list.get(), index, &box);
            if (status == ANYBOX_STATUS_OK) {
                entry.found = detail::snapshot(box);
            } else {
                // the result call made this entry's failure the last error, so it's read from there right away
                entry.error = detail::last_error(status);
            }
            out.push_back(std::move(entry));
        }
        return out;
    }

    /// the ports a search would look at, without opening any of them or sending them a byte, so it's cheap enough
    /// to call every second and scan when the list changes
    std::vector<std::string> candidates() const {
        detail::ensure();
        const detail::Options options(*this);
        AnyboxStringList* raw = nullptr;
        detail::check(anybox_candidates(options.get(), &raw));
        const detail::StringListOwner list(raw);
        return detail::strings(list.get());
    }

private:
    friend class detail::Options;

    std::optional<std::string> port_;
    std::optional<Family> family_;
    std::optional<FerrumApi> ferrum_api_;
    std::vector<Action> can_;
    std::optional<StableId> id_;
    ClickTiming click_timing_;
};

inline detail::Options::Options(const Connect& connect) {
    if (connect.port_) {
        port_ = c_text(*connect.port_);
        raw_.port = port_.c_str();
    }
    raw_.family = connect.family_ ? static_cast<std::uint32_t>(*connect.family_) : ANYBOX_FAMILY_ANY;
    raw_.ferrum_api = connect.ferrum_api_ ? static_cast<std::uint32_t>(*connect.ferrum_api_) : ANYBOX_FERRUM_API_NONE;
    for (const auto action : connect.can_) {
        can_.push_back(static_cast<std::uint32_t>(action));
    }
    raw_.can = can_.empty() ? nullptr : can_.data();
    raw_.can_len = can_.size();
    if (connect.id_) {
        if (const auto& mac = connect.id_->mac()) {
            raw_.id_kind = ANYBOX_ID_MAC;
            for (std::size_t index = 0; index < mac->size(); index++) {
                raw_.id_mac[index] = (*mac)[index];
            }
        } else {
            serial_ = c_text(connect.id_->usb_serial().value_or(""));
            raw_.id_kind = ANYBOX_ID_USB_SERIAL;
            raw_.id_serial = serial_.c_str();
        }
    }
    raw_.click_timing = connect.click_timing_.to_c();
}

/// the anybox version the library was built from
inline std::string version() {
    detail::ensure();
    return detail::text(anybox_version());
}

/// whether the library was built with the mock
inline bool has_mock() {
    detail::ensure();
    return anybox_has_mock();
}

/// now, in nanoseconds, on the OS monotonic clock every stamp's `arrived_ns` and `received_ns` are on
/// (QueryPerformanceCounter on Windows, CLOCK_MONOTONIC on Linux, CLOCK_UPTIME_RAW on macOS). it only compares on
/// the same machine within one boot
inline std::uint64_t monotonic_ns() {
    detail::ensure();
    return anybox_monotonic_ns();
}

/// connects to the first box found, in port order
inline Device connect() { return Connect().open(); }

/// every port a search looks at and what's on it, see `Connect::scan`
inline std::vector<Scanned> scan() { return Connect().scan(); }

/// the ports a search would look at, see `Connect::candidates`
inline std::vector<std::string> candidates() { return Connect().candidates(); }

#if defined(ANYBOX_FEATURE_MOCK)

/// one frame the mock box got, its type byte (the protocol's, MOVE is 1) and its payload
struct Frame {
    std::uint8_t type = 0;
    std::vector<std::uint8_t> payload;
};

/// the medius crate's model of a Medius box, for testing code that uses anybox with no box plugged in. it proves
/// nothing about hardware and it can only be shaped like a Medius. it needs a library built with the capi crate's
/// mock feature, linking one without it fails
class Mock {
public:
    /// a device over a new mock box, a five button mouse with a wheel, and with `keyboard` a six key keyboard with
    /// media keys behind it too, running the exact code a real Medius gets and checking the link every 250 ms like
    /// a real one
    static std::pair<Device, Mock> medius(const ClickTiming& click_timing = ClickTiming::makcu(),
                                          bool keyboard = false) {
        return open(anybox_mock_medius_open, click_timing, keyboard);
    }

    /// the same as `medius()`, except that the link only gets checked on `tick()`, so a test decides when a link
    /// event gets noticed and nothing lands between the frames it records
    static std::pair<Device, Mock> medius_manual(const ClickTiming& click_timing = ClickTiming::makcu(),
                                                 bool keyboard = false) {
        return open(anybox_mock_medius_open_manual, click_timing, keyboard);
    }

    /// the handle for anybox.h, which throws ClosedError once this was moved from
    const AnyboxMock* get() const {
        if (!handle_) {
            throw ClosedError("this mock was moved from, use the one it was moved to");
        }
        return handle_.get();
    }

    /// whether the mock box got the subscription of every one of these kinds yet. a receiver subscribes each kind on
    /// the device's own threads, the keys and media a moment after the buttons, and the box drops what's pushed
    /// before its kind is in like a real one would, so wait for this before you push
    bool subscribed(EventKinds kinds) const { return anybox_mock_subscribed(get(), static_cast<std::uint32_t>(kinds)); }

    /// physical motion as if from the box. `seq` is the box's frame counter and `ts_us` its clock. a gap in `seq`
    /// doesn't show up as a drop
    void push_motion(std::uint8_t seq, std::uint32_t ts_us, std::int16_t dx, std::int16_t dy, std::int16_t wheel = 0,
                     std::int16_t pan = 0) const {
        detail::check(anybox_mock_push_motion(get(), seq, ts_us, dx, dy, wheel, pan));
    }

    /// the physical buttons held now, bit n for the box's button n. it's the whole held set, so a release is the set
    /// without that button, and `pressed` says whether the set grew or shrank
    void push_buttons(std::uint8_t seq, std::uint32_t ts_us, bool pressed, std::uint32_t held) const {
        detail::check(anybox_mock_push_buttons(get(), seq, ts_us, pressed, held));
    }

    /// the physical keys held now, modifiers included. it's the whole held set like `push_buttons()`, so the last
    /// release is pushing no keys at all
    void push_keys(std::uint8_t seq, std::uint32_t ts_us, bool pressed, const std::vector<Key>& held) const {
        std::vector<std::uint8_t> keys;
        for (const auto key : held) {
            keys.push_back(key.value);
        }
        detail::check(anybox_mock_push_keys(get(), seq, ts_us, pressed, keys.data(), keys.size()));
    }

    /// the media keys held now, the whole held set like the keys
    void push_media(std::uint8_t seq, std::uint32_t ts_us, bool pressed, const std::vector<MediaKey>& held) const {
        std::vector<std::uint16_t> media;
        for (const auto key : held) {
            media.push_back(key.value);
        }
        detail::check(anybox_mock_push_media(get(), seq, ts_us, pressed, media.data(), media.size()));
    }

    /// the link between the box's two chips drops and comes back, so the box lets go of what it held and the device
    /// sends `SessionReset` once it checks the link (within 250 ms, or on the next `tick()` of a manual mock)
    void link_lost() const { detail::check(anybox_mock_link_lost(get())); }

    /// the box restarts, so the device sends `Disconnected` and then `Reconnected`, noticed the way `link_lost()` is
    void restart() const { detail::check(anybox_mock_restart(get())); }

    /// one pass of what a real device checks on the link every 250 ms, on a mock from `medius_manual()`, which is
    /// where it notices `link_lost()` and `restart()`. the mock keeps its own timers (a restarted clone is back
    /// 100 ms later), so a test still waits for those
    void tick() const { detail::check(anybox_mock_tick(get())); }

    /// every frame the box got since it opened or since `clear_recorded()`
    std::vector<Frame> recorded() const {
        std::vector<Frame> frames;
        const auto count = anybox_mock_recorded(get());
        for (std::uintptr_t index = 0; index < count; index++) {
            Frame frame;
            std::uintptr_t size = 0;
            detail::check(anybox_mock_recorded_frame(get(), index, &frame.type, nullptr, 0, &size));
            frame.payload.resize(size);
            detail::check(
                anybox_mock_recorded_frame(get(), index, &frame.type, frame.payload.data(), frame.payload.size(), &size));
            frame.payload.resize(size);
            frames.push_back(std::move(frame));
        }
        return frames;
    }

    /// forgets every frame the box got so far
    void clear_recorded() const { detail::check(anybox_mock_clear_recorded(get())); }

private:
    using Owner = std::unique_ptr<AnyboxMock, detail::Freer<AnyboxMock, anybox_mock_free>>;
    using Open = AnyboxStatus (*)(const AnyboxClickTiming*, bool, AnyboxMock**, AnyboxDevice**);

    explicit Mock(Owner handle) noexcept : handle_(std::move(handle)) {}

    static std::pair<Device, Mock> open(Open call, const ClickTiming& click_timing, bool keyboard) {
        detail::ensure();
        const auto timing = click_timing.to_c();
        AnyboxMock* raw_mock = nullptr;
        AnyboxDevice* raw_device = nullptr;
        detail::check(call(&timing, keyboard, &raw_mock, &raw_device));
        // both get their owner before anything else can throw, the device's frees it if its own allocation fails
        Owner mock(raw_mock);
        Device device(raw_device);
        return {std::move(device), Mock(std::move(mock))};
    }

    Owner handle_;
};

/// sends every receiver of `device` one event of every kind, with every field filled in, through the same queue as
/// real ones, so you can test how your code reads each kind with no box. none of them does to the device what the
/// real one would, so it stays connected through the example `Disconnected`
inline void example_events(const Device& device) {
    detail::ensure();
    detail::check(anybox_mock_example_events(device.get()));
}

/// anybox's example of that error, with every field it has filled in, so you can test how your code handles it
/// with no box. the ones only this C ABI or this header makes (PanicError, UnknownError, RecvClosedError, Error
/// itself) have none and throw the InvalidArgumentError the mock gives
template <class T>
T example_error() {
    static_assert(std::is_base_of<Error, T>::value, "example_error is for anybox's errors");
    detail::ensure();
    // the status that makes a T, every status there is fits in a byte
    std::uint32_t status = 0;
    for (; status < 256; status++) {
        if (dynamic_cast<const T*>(detail::make_error(status, "", detail::Fields{}, {}, {}).get())) {
            break;
        }
    }
    const auto made = anybox_mock_fail(status);
    const auto error = detail::last_error(made);
    if (made == status) {
        if (const auto* example = dynamic_cast<const T*>(error.get())) {
            return *example;
        }
    }
    error->raise();
}

#endif

}  // namespace anybox

#pragma pop_macro("interface")
#pragma pop_macro("DELETE")

#endif
