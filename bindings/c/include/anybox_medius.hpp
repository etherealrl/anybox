/* the Medius side of anybox.hpp, everything the medius crate can do, with its names. anybox.hpp brings it in for
   `device.medius()`, so include that and not this. most of it is made from medius.h by
   bindings/csharp/tools/gen-medius.cs, which fails on anything in a new medius.h nobody decided about. MPL-2.0 */

#ifndef ANYBOX_MEDIUS_HPP
#define ANYBOX_MEDIUS_HPP

#ifndef ANYBOX_HPP
#error "include anybox.hpp, it brings this in"
#endif

#include <algorithm>
#include <functional>

namespace anybox {
namespace medius {

namespace detail {

// what a value coming back from the library does when this header has no name for it. Strict throws, since it
// means the library is newer than the header, Unknown reads as the enum's Unknown member and Any keeps the number
enum class Read { Strict, Unknown, Any };

template <class E>
struct Enum;

}  // namespace detail

// made by gen-medius from medius.h (enums), change its tables and run dotnet run bindings/csharp/tools/gen-medius.cs
/// the status a medius.h call gave, what `Error::medius_status()` says
enum class Status : std::int32_t {
    Ok = MEDIUS_STATUS_OK,
    ErrIo = MEDIUS_STATUS_ERR_IO,
    ErrNotFound = MEDIUS_STATUS_ERR_NOT_FOUND,
    ErrNoReply = MEDIUS_STATUS_ERR_NO_REPLY,
    ErrBadProtoVer = MEDIUS_STATUS_ERR_BAD_PROTO_VER,
    ErrQueryTimeout = MEDIUS_STATUS_ERR_QUERY_TIMEOUT,
    ErrDisconnected = MEDIUS_STATUS_ERR_DISCONNECTED,
    ErrFrameTooLong = MEDIUS_STATUS_ERR_FRAME_TOO_LONG,
    ErrUpdate = MEDIUS_STATUS_ERR_UPDATE,
    ErrInvalidArg = MEDIUS_STATUS_ERR_INVALID_ARG,
    ErrPanic = MEDIUS_STATUS_ERR_PANIC,
    ErrUnknown = MEDIUS_STATUS_ERR_UNKNOWN,
    ErrCatchTableFull = MEDIUS_STATUS_ERR_CATCH_TABLE_FULL,
    ErrEmptySubscription = MEDIUS_STATUS_ERR_EMPTY_SUBSCRIPTION,
    ErrCaptureNotApplicable = MEDIUS_STATUS_ERR_CAPTURE_NOT_APPLICABLE,
    ErrNotAnInputFilter = MEDIUS_STATUS_ERR_NOT_AN_INPUT_FILTER,
    ErrWildcardNotInput = MEDIUS_STATUS_ERR_WILDCARD_NOT_INPUT,
    ErrHalfEdgeInputFilter = MEDIUS_STATUS_ERR_HALF_EDGE_INPUT_FILTER,
    ErrReservedId = MEDIUS_STATUS_ERR_RESERVED_ID,
    ErrRelativeDirection = MEDIUS_STATUS_ERR_RELATIVE_DIRECTION,
    ErrLockScaleRange = MEDIUS_STATUS_ERR_LOCK_SCALE_RANGE,
    ErrLockScaleUsage = MEDIUS_STATUS_ERR_LOCK_SCALE_USAGE,
    ErrImperfectRequired = MEDIUS_STATUS_ERR_IMPERFECT_REQUIRED,
    ErrRewriteMaskLength = MEDIUS_STATUS_ERR_REWRITE_MASK_LENGTH,
    ErrRewriteActionClass = MEDIUS_STATUS_ERR_REWRITE_ACTION_CLASS,
    ErrRewritePayloadTooLarge = MEDIUS_STATUS_ERR_REWRITE_PAYLOAD_TOO_LARGE,
    ErrRewriteTableFull = MEDIUS_STATUS_ERR_REWRITE_TABLE_FULL,
    ErrTransformOpFields = MEDIUS_STATUS_ERR_TRANSFORM_OP_FIELDS,
    ErrTransformTableFull = MEDIUS_STATUS_ERR_TRANSFORM_TABLE_FULL,
    ErrRawDirection = MEDIUS_STATUS_ERR_RAW_DIRECTION,
    ErrClipFrameCount = MEDIUS_STATUS_ERR_CLIP_FRAME_COUNT,
    ErrClipFrameTooLong = MEDIUS_STATUS_ERR_CLIP_FRAME_TOO_LONG,
    ErrClipTransferData = MEDIUS_STATUS_ERR_CLIP_TRANSFER_DATA,
    ErrClipPacketTrigger = MEDIUS_STATUS_ERR_CLIP_PACKET_TRIGGER,
    ErrRewriteMatchTooLong = MEDIUS_STATUS_ERR_REWRITE_MATCH_TOO_LONG,
    ErrRewritePoolFull = MEDIUS_STATUS_ERR_REWRITE_POOL_FULL,
};

/// what paces injected motion
enum class EmitMode : std::uint8_t {
    Learned = MEDIUS_EMIT_MODE_LEARNED,
    Interval = MEDIUS_EMIT_MODE_INTERVAL,
    Fixed = MEDIUS_EMIT_MODE_FIXED,
};

/// how the box tells whether physical motion runs with or against injected motion
enum class BearingMode : std::uint8_t {
    PerAxis = MEDIUS_BEARING_MODE_PER_AXIS,
    Vector = MEDIUS_BEARING_MODE_VECTOR,
};

/// the render texture. `Off` is the paced fill, the others render the device's own report texture and only differ in
/// the path smoother
enum class RenderMode : std::uint8_t {
    Off = MEDIUS_RENDER_MODE_OFF,
    Stock = MEDIUS_RENDER_MODE_STOCK,
    Despiked = MEDIUS_RENDER_MODE_DESPIKED,
    Unsmoothed = MEDIUS_RENDER_MODE_UNSMOOTHED,
};

/// the handshake the game PC got for a control transaction, `Other` for one this build of medius doesn't know
enum class ControlStatus : std::uint8_t {
    Ok = MEDIUS_CONTROL_STATUS_OK,
    Stalled = MEDIUS_CONTROL_STATUS_STALLED,
    Naked = MEDIUS_CONTROL_STATUS_NAKED,
    Other = MEDIUS_CONTROL_STATUS_OTHER,
};

/// what a bus event describes
enum class BusEventKind : std::uint8_t {
    Reset = MEDIUS_BUS_EVENT_KIND_RESET,
    Suspend = MEDIUS_BUS_EVENT_KIND_SUSPEND,
    Resume = MEDIUS_BUS_EVENT_KIND_RESUME,
    Configured = MEDIUS_BUS_EVENT_KIND_CONFIGURED,
    Deconfigured = MEDIUS_BUS_EVENT_KIND_DECONFIGURED,
    SetInterface = MEDIUS_BUS_EVENT_KIND_SET_INTERFACE,
    DeviceAttached = MEDIUS_BUS_EVENT_KIND_DEVICE_ATTACHED,
    DeviceDetached = MEDIUS_BUS_EVENT_KIND_DEVICE_DETACHED,
    CloneUp = MEDIUS_BUS_EVENT_KIND_CLONE_UP,
    CloneDown = MEDIUS_BUS_EVENT_KIND_CLONE_DOWN,
};

/// which kind a catch event is
enum class CatchEventKind : std::uint8_t {
    Motion = MEDIUS_CATCH_EVENT_KIND_MOTION,
    Usages = MEDIUS_CATCH_EVENT_KIND_USAGES,
    Traffic = MEDIUS_CATCH_EVENT_KIND_TRAFFIC,
};

/// which chip's clock stamped an event. the chips boot on their own and their timers have nothing to do with each
/// other, so only compare stamps from the same one
enum class ClockDomain : std::uint8_t {
    HostChip = MEDIUS_CLOCK_DOMAIN_HOST_CHIP,
    DeviceChip = MEDIUS_CLOCK_DOMAIN_DEVICE_CHIP,
};

/// which kind an input event is
enum class InputKind : std::uint8_t {
    Press = MEDIUS_INPUT_KIND_PRESS,
    Release = MEDIUS_INPUT_KIND_RELEASE,
    Motion = MEDIUS_INPUT_KIND_MOTION,
};

/// how bad a log line from the box is
enum class LogLevel : std::uint8_t {
    Error = MEDIUS_LOG_LEVEL_ERROR,
    Warn = MEDIUS_LOG_LEVEL_WARN,
    Info = MEDIUS_LOG_LEVEL_INFO,
    Debug = MEDIUS_LOG_LEVEL_DEBUG,
    Verbose = MEDIUS_LOG_LEVEL_VERBOSE,
};

/// the type byte of a frame, for what the mock got. one this header has no name for comes through as its number
enum class FrameType : std::uint8_t {
    Move = MEDIUS_FRAME_TYPE_MOVE,
    Inject = MEDIUS_FRAME_TYPE_INJECT,
    Reset = MEDIUS_FRAME_TYPE_RESET,
    Query = MEDIUS_FRAME_TYPE_QUERY,
    Resp = MEDIUS_FRAME_TYPE_RESP,
    RebootDl = MEDIUS_FRAME_TYPE_REBOOT_DL,
    Log = MEDIUS_FRAME_TYPE_LOG,
    Led = MEDIUS_FRAME_TYPE_LED,
    Lock = MEDIUS_FRAME_TYPE_LOCK,
    Catch = MEDIUS_FRAME_TYPE_CATCH,
    MotionEvent = MEDIUS_FRAME_TYPE_MOTION_EVENT,
    UsageEvent = MEDIUS_FRAME_TYPE_USAGE_EVENT,
    TrafficEvent = MEDIUS_FRAME_TYPE_TRAFFIC_EVENT,
    Option = MEDIUS_FRAME_TYPE_OPTION,
    ClipAppend = MEDIUS_FRAME_TYPE_CLIP_APPEND,
    ClipCtrl = MEDIUS_FRAME_TYPE_CLIP_CTRL,
    ClipSet = MEDIUS_FRAME_TYPE_CLIP_SET,
    ClipTrigger = MEDIUS_FRAME_TYPE_CLIP_TRIGGER,
    Update = MEDIUS_FRAME_TYPE_UPDATE,
    UpdateResp = MEDIUS_FRAME_TYPE_UPDATE_RESP,
    Raw = MEDIUS_FRAME_TYPE_RAW,
    Transfer = MEDIUS_FRAME_TYPE_TRANSFER,
    TransferResp = MEDIUS_FRAME_TYPE_TRANSFER_RESP,
    Rewrite = MEDIUS_FRAME_TYPE_REWRITE,
    Patch = MEDIUS_FRAME_TYPE_PATCH,
    Transform = MEDIUS_FRAME_TYPE_TRANSFORM,
};

/// what an injected edge does. `SoftRelease` takes your press back and leaves a physical hold alone, `ForceRelease`
/// masks a physical hold too
enum class Action : std::uint8_t {
    SoftRelease = MEDIUS_ACTION_SOFT_RELEASE,
    Press = MEDIUS_ACTION_PRESS,
    ForceRelease = MEDIUS_ACTION_FORCE_RELEASE,
};

/// a relative axis
enum class Axis : std::uint8_t {
    X = MEDIUS_AXIS_X,
    Y = MEDIUS_AXIS_Y,
    Wheel = MEDIUS_AXIS_WHEEL,
    Pan = MEDIUS_AXIS_PAN,
};

/// a whole group of inputs, for a blanket lock or a clip's autolock
enum class Blanket : std::uint8_t {
    Aim = MEDIUS_BLANKET_AIM,
    Wheel = MEDIUS_BLANKET_WHEEL,
    Buttons = MEDIUS_BLANKET_BUTTONS,
    Keys = MEDIUS_BLANKET_KEYS,
    Media = MEDIUS_BLANKET_MEDIA,
};

/// a mouse button by the box's id. any id up to the clone's button count is one, these are the five usual ones
struct Button {
    std::uint8_t value;

    friend constexpr bool operator==(Button a, Button b) noexcept { return a.value == b.value; }
    friend constexpr bool operator!=(Button a, Button b) noexcept { return a.value != b.value; }
    friend constexpr bool operator<(Button a, Button b) noexcept { return a.value < b.value; }

    static const Button LEFT;
    static const Button RIGHT;
    static const Button MIDDLE;
    static const Button SIDE1;
    static const Button SIDE2;
};

inline constexpr Button Button::LEFT{MEDIUS_BUTTON_LEFT};
inline constexpr Button Button::RIGHT{MEDIUS_BUTTON_RIGHT};
inline constexpr Button Button::MIDDLE{MEDIUS_BUTTON_MIDDLE};
inline constexpr Button Button::SIDE1{MEDIUS_BUTTON_SIDE1};
inline constexpr Button Button::SIDE2{MEDIUS_BUTTON_SIDE2};

/// what a usage is, a button, a key or a media key
enum class Class : std::uint8_t {
    Button = MEDIUS_CLASS_BUTTON,
    Key = MEDIUS_CLASS_KEY,
    Media = MEDIUS_CLASS_MEDIA,
};

/// what a clip trigger does to the clip
enum class ClipAction : std::uint8_t {
    Start = MEDIUS_CLIP_ACTION_START,
    Stop = MEDIUS_CLIP_ACTION_STOP,
    Pause = MEDIUS_CLIP_ACTION_PAUSE,
    Resume = MEDIUS_CLIP_ACTION_RESUME,
    Restart = MEDIUS_CLIP_ACTION_RESTART,
    Toggle = MEDIUS_CLIP_ACTION_TOGGLE,
};

/// where the clip is on the box
enum class ClipState : std::uint8_t {
    Idle = MEDIUS_CLIP_STATE_IDLE,
    Playing = MEDIUS_CLIP_STATE_PLAYING,
    Paused = MEDIUS_CLIP_STATE_PAUSED,
    Faulted = MEDIUS_CLIP_STATE_FAULTED,
};

/// what the cloned device mostly is, from its HID descriptors. one this header has no name for reads as `Unknown`
enum class DeviceKind : std::uint8_t {
    Unknown = MEDIUS_DEVICE_KIND_UNKNOWN,
    Keyboard = MEDIUS_DEVICE_KIND_KEYBOARD,
    Mouse = MEDIUS_DEVICE_KIND_MOUSE,
};

/// which way a lock, a scale or a filter goes. on a button or a key it's the press or the release, on traffic it's in
/// (to the PC) or out, and `With` and `Against` are measured against the bearing instead of a fixed sign
enum class Direction : std::uint8_t {
    Both = MEDIUS_DIRECTION_BOTH,
    Positive = MEDIUS_DIRECTION_POSITIVE,
    Negative = MEDIUS_DIRECTION_NEGATIVE,
    With = MEDIUS_DIRECTION_WITH,
    Against = MEDIUS_DIRECTION_AGAINST,
    Press = Positive,
    Release = Negative,
    In = Positive,
    Out = Negative,
};

/// which edge of a usage fires a clip trigger
enum class Edge : std::uint8_t {
    Both = MEDIUS_EDGE_BOTH,
    Press = MEDIUS_EDGE_PRESS,
    Release = MEDIUS_EDGE_RELEASE,
};

/// what a status LED does
enum class LedMode : std::uint8_t {
    Auto = MEDIUS_LED_MODE_AUTO,
    Off = MEDIUS_LED_MODE_OFF,
    Solid = MEDIUS_LED_MODE_SOLID,
    Blink = MEDIUS_LED_MODE_BLINK,
};

/// which status LED a command is for
enum class LedTarget : std::uint8_t {
    Device = MEDIUS_LED_TARGET_DEVICE,
    Host = MEDIUS_LED_TARGET_HOST,
    Both = MEDIUS_LED_TARGET_BOTH,
};

/// what a lock target is, one of the four axes or a usage
enum class LockTargetKind : std::uint8_t {
    X = MEDIUS_LOCK_TARGET_KIND_X,
    Y = MEDIUS_LOCK_TARGET_KIND_Y,
    Wheel = MEDIUS_LOCK_TARGET_KIND_WHEEL,
    Pan = MEDIUS_LOCK_TARGET_KIND_PAN,
    Usage = MEDIUS_LOCK_TARGET_KIND_USAGE,
};

/// which axis a motion drives
enum class MotionKind : std::uint8_t {
    Cursor = MEDIUS_MOTION_KIND_CURSOR,
    Wheel = MEDIUS_MOTION_KIND_WHEEL,
    Pan = MEDIUS_MOTION_KIND_PAN,
};

/// whether a move rides a physical report or goes out at once
enum class MoveTiming : std::uint8_t {
    Ride = MEDIUS_MOVE_TIMING_RIDE,
    Now = MEDIUS_MOVE_TIMING_NOW,
};

/// which descriptor a patch overwrites
enum class PatchSection : std::uint8_t {
    Device = MEDIUS_PATCH_SECTION_DEVICE,
    Config = MEDIUS_PATCH_SECTION_CONFIG,
    Report = MEDIUS_PATCH_SECTION_REPORT,
    String = MEDIUS_PATCH_SECTION_STRING,
    Bos = MEDIUS_PATCH_SECTION_BOS,
};

/// what a move does to the motion already held for a ride
enum class PendingMotion : std::uint8_t {
    Keep = MEDIUS_PENDING_MOTION_KEEP,
    Flush = MEDIUS_PENDING_MOTION_FLUSH,
    Discard = MEDIUS_PENDING_MOTION_DISCARD,
};

/// which chip reboots, and into what
enum class RebootTarget : std::uint8_t {
    DeviceDownload = MEDIUS_REBOOT_TARGET_DEVICE_DOWNLOAD,
    HostDownload = MEDIUS_REBOOT_TARGET_HOST_DOWNLOAD,
    DeviceRun = MEDIUS_REBOOT_TARGET_DEVICE_RUN,
    HostRun = MEDIUS_REBOOT_TARGET_HOST_RUN,
};

/// what the most specific matching rewrite rule does to the packet
enum class RewriteAction : std::uint8_t {
    Pass = MEDIUS_REWRITE_ACTION_PASS,
    Drop = MEDIUS_REWRITE_ACTION_DROP,
    Patch = MEDIUS_REWRITE_ACTION_PATCH,
    Replace = MEDIUS_REWRITE_ACTION_REPLACE,
    Answer = MEDIUS_REWRITE_ACTION_ANSWER,
    Stall = MEDIUS_REWRITE_ACTION_STALL,
    Nak = MEDIUS_REWRITE_ACTION_NAK,
    ReplyPatch = MEDIUS_REWRITE_ACTION_REPLY_PATCH,
    ReplyReplace = MEDIUS_REWRITE_ACTION_REPLY_REPLACE,
};

/// the traffic a rewrite rule works on, `Any` is every surface a packet crosses
enum class RewriteClass : std::uint8_t {
    HidIn = MEDIUS_REWRITE_CLASS_HID_IN,
    HidOut = MEDIUS_REWRITE_CLASS_HID_OUT,
    VendorInterrupt = MEDIUS_REWRITE_CLASS_VENDOR_INTERRUPT,
    VendorBulk = MEDIUS_REWRITE_CLASS_VENDOR_BULK,
    Control = MEDIUS_REWRITE_CLASS_CONTROL,
    Emit = MEDIUS_REWRITE_CLASS_EMIT,
    Any = MEDIUS_REWRITE_CLASS_ANY,
};

/// how a control transfer ended. one this header has no name for comes through as its number, both ways
enum class TransferStatus : std::uint8_t {
    Ok = MEDIUS_TRANSFER_STATUS_OK,
    Refused = MEDIUS_TRANSFER_STATUS_REFUSED,
    Stall = MEDIUS_TRANSFER_STATUS_STALL,
    Nak = MEDIUS_TRANSFER_STATUS_NAK,
    NoDevice = MEDIUS_TRANSFER_STATUS_NO_DEVICE,
};

/// what a transform does. `Remap` moves a field's value into another one and clears it, `Swap` exchanges two axes at
/// once
enum class TransformOp : std::uint8_t {
    Remap = MEDIUS_TRANSFORM_OP_REMAP,
    Swap = MEDIUS_TRANSFORM_OP_SWAP,
};

/// a keyboard key by its HID usage, `Key{0x64}` gets you any other one. just know that windows.h has a DELETE macro, so
/// `Key::DELETE` needs an `#undef DELETE` after it
struct Key {
    std::uint8_t value;

    friend constexpr bool operator==(Key a, Key b) noexcept { return a.value == b.value; }
    friend constexpr bool operator!=(Key a, Key b) noexcept { return a.value != b.value; }
    friend constexpr bool operator<(Key a, Key b) noexcept { return a.value < b.value; }

    static const Key A;
    static const Key B;
    static const Key C;
    static const Key D;
    static const Key E;
    static const Key F;
    static const Key G;
    static const Key H;
    static const Key I;
    static const Key J;
    static const Key K;
    static const Key L;
    static const Key M;
    static const Key N;
    static const Key O;
    static const Key P;
    static const Key Q;
    static const Key R;
    static const Key S;
    static const Key T;
    static const Key U;
    static const Key V;
    static const Key W;
    static const Key X;
    static const Key Y;
    static const Key Z;
    static const Key NUM1;
    static const Key NUM2;
    static const Key NUM3;
    static const Key NUM4;
    static const Key NUM5;
    static const Key NUM6;
    static const Key NUM7;
    static const Key NUM8;
    static const Key NUM9;
    static const Key NUM0;
    static const Key ENTER;
    static const Key ESCAPE;
    static const Key BACKSPACE;
    static const Key TAB;
    static const Key SPACE;
    static const Key CAPS_LOCK;
    static const Key INSERT;
    static const Key HOME;
    static const Key PAGE_UP;
    static const Key DELETE;
    static const Key END;
    static const Key PAGE_DOWN;
    static const Key RIGHT;
    static const Key LEFT;
    static const Key DOWN;
    static const Key UP;
    static const Key F1;
    static const Key F2;
    static const Key F3;
    static const Key F4;
    static const Key F5;
    static const Key F6;
    static const Key F7;
    static const Key F8;
    static const Key F9;
    static const Key F10;
    static const Key F11;
    static const Key F12;
    static const Key LEFT_CTRL;
    static const Key LEFT_SHIFT;
    static const Key LEFT_ALT;
    static const Key LEFT_GUI;
    static const Key RIGHT_CTRL;
    static const Key RIGHT_SHIFT;
    static const Key RIGHT_ALT;
    static const Key RIGHT_GUI;
};

inline constexpr Key Key::A{MEDIUS_KEY_A};
inline constexpr Key Key::B{MEDIUS_KEY_B};
inline constexpr Key Key::C{MEDIUS_KEY_C};
inline constexpr Key Key::D{MEDIUS_KEY_D};
inline constexpr Key Key::E{MEDIUS_KEY_E};
inline constexpr Key Key::F{MEDIUS_KEY_F};
inline constexpr Key Key::G{MEDIUS_KEY_G};
inline constexpr Key Key::H{MEDIUS_KEY_H};
inline constexpr Key Key::I{MEDIUS_KEY_I};
inline constexpr Key Key::J{MEDIUS_KEY_J};
inline constexpr Key Key::K{MEDIUS_KEY_K};
inline constexpr Key Key::L{MEDIUS_KEY_L};
inline constexpr Key Key::M{MEDIUS_KEY_M};
inline constexpr Key Key::N{MEDIUS_KEY_N};
inline constexpr Key Key::O{MEDIUS_KEY_O};
inline constexpr Key Key::P{MEDIUS_KEY_P};
inline constexpr Key Key::Q{MEDIUS_KEY_Q};
inline constexpr Key Key::R{MEDIUS_KEY_R};
inline constexpr Key Key::S{MEDIUS_KEY_S};
inline constexpr Key Key::T{MEDIUS_KEY_T};
inline constexpr Key Key::U{MEDIUS_KEY_U};
inline constexpr Key Key::V{MEDIUS_KEY_V};
inline constexpr Key Key::W{MEDIUS_KEY_W};
inline constexpr Key Key::X{MEDIUS_KEY_X};
inline constexpr Key Key::Y{MEDIUS_KEY_Y};
inline constexpr Key Key::Z{MEDIUS_KEY_Z};
inline constexpr Key Key::NUM1{MEDIUS_KEY_1};
inline constexpr Key Key::NUM2{MEDIUS_KEY_2};
inline constexpr Key Key::NUM3{MEDIUS_KEY_3};
inline constexpr Key Key::NUM4{MEDIUS_KEY_4};
inline constexpr Key Key::NUM5{MEDIUS_KEY_5};
inline constexpr Key Key::NUM6{MEDIUS_KEY_6};
inline constexpr Key Key::NUM7{MEDIUS_KEY_7};
inline constexpr Key Key::NUM8{MEDIUS_KEY_8};
inline constexpr Key Key::NUM9{MEDIUS_KEY_9};
inline constexpr Key Key::NUM0{MEDIUS_KEY_0};
inline constexpr Key Key::ENTER{MEDIUS_KEY_ENTER};
inline constexpr Key Key::ESCAPE{MEDIUS_KEY_ESCAPE};
inline constexpr Key Key::BACKSPACE{MEDIUS_KEY_BACKSPACE};
inline constexpr Key Key::TAB{MEDIUS_KEY_TAB};
inline constexpr Key Key::SPACE{MEDIUS_KEY_SPACE};
inline constexpr Key Key::CAPS_LOCK{MEDIUS_KEY_CAPS_LOCK};
inline constexpr Key Key::INSERT{MEDIUS_KEY_INSERT};
inline constexpr Key Key::HOME{MEDIUS_KEY_HOME};
inline constexpr Key Key::PAGE_UP{MEDIUS_KEY_PAGE_UP};
inline constexpr Key Key::DELETE{MEDIUS_KEY_DELETE};
inline constexpr Key Key::END{MEDIUS_KEY_END};
inline constexpr Key Key::PAGE_DOWN{MEDIUS_KEY_PAGE_DOWN};
inline constexpr Key Key::RIGHT{MEDIUS_KEY_RIGHT};
inline constexpr Key Key::LEFT{MEDIUS_KEY_LEFT};
inline constexpr Key Key::DOWN{MEDIUS_KEY_DOWN};
inline constexpr Key Key::UP{MEDIUS_KEY_UP};
inline constexpr Key Key::F1{MEDIUS_KEY_F1};
inline constexpr Key Key::F2{MEDIUS_KEY_F2};
inline constexpr Key Key::F3{MEDIUS_KEY_F3};
inline constexpr Key Key::F4{MEDIUS_KEY_F4};
inline constexpr Key Key::F5{MEDIUS_KEY_F5};
inline constexpr Key Key::F6{MEDIUS_KEY_F6};
inline constexpr Key Key::F7{MEDIUS_KEY_F7};
inline constexpr Key Key::F8{MEDIUS_KEY_F8};
inline constexpr Key Key::F9{MEDIUS_KEY_F9};
inline constexpr Key Key::F10{MEDIUS_KEY_F10};
inline constexpr Key Key::F11{MEDIUS_KEY_F11};
inline constexpr Key Key::F12{MEDIUS_KEY_F12};
inline constexpr Key Key::LEFT_CTRL{MEDIUS_KEY_LEFT_CTRL};
inline constexpr Key Key::LEFT_SHIFT{MEDIUS_KEY_LEFT_SHIFT};
inline constexpr Key Key::LEFT_ALT{MEDIUS_KEY_LEFT_ALT};
inline constexpr Key Key::LEFT_GUI{MEDIUS_KEY_LEFT_GUI};
inline constexpr Key Key::RIGHT_CTRL{MEDIUS_KEY_RIGHT_CTRL};
inline constexpr Key Key::RIGHT_SHIFT{MEDIUS_KEY_RIGHT_SHIFT};
inline constexpr Key Key::RIGHT_ALT{MEDIUS_KEY_RIGHT_ALT};
inline constexpr Key Key::RIGHT_GUI{MEDIUS_KEY_RIGHT_GUI};

/// a media key by its HID consumer usage, `MediaKey{n}` gets you any other one
struct MediaKey {
    std::uint16_t value;

    friend constexpr bool operator==(MediaKey a, MediaKey b) noexcept { return a.value == b.value; }
    friend constexpr bool operator!=(MediaKey a, MediaKey b) noexcept { return a.value != b.value; }
    friend constexpr bool operator<(MediaKey a, MediaKey b) noexcept { return a.value < b.value; }

    static const MediaKey PLAY_PAUSE;
    static const MediaKey STOP;
    static const MediaKey NEXT_TRACK;
    static const MediaKey PREV_TRACK;
    static const MediaKey MUTE;
    static const MediaKey VOLUME_UP;
    static const MediaKey VOLUME_DOWN;
    static const MediaKey PLAY;
    static const MediaKey PAUSE;
};

inline constexpr MediaKey MediaKey::PLAY_PAUSE{MEDIUS_MEDIA_PLAY_PAUSE};
inline constexpr MediaKey MediaKey::STOP{MEDIUS_MEDIA_STOP};
inline constexpr MediaKey MediaKey::NEXT_TRACK{MEDIUS_MEDIA_NEXT_TRACK};
inline constexpr MediaKey MediaKey::PREV_TRACK{MEDIUS_MEDIA_PREV_TRACK};
inline constexpr MediaKey MediaKey::MUTE{MEDIUS_MEDIA_MUTE};
inline constexpr MediaKey MediaKey::VOLUME_UP{MEDIUS_MEDIA_VOLUME_UP};
inline constexpr MediaKey MediaKey::VOLUME_DOWN{MEDIUS_MEDIA_VOLUME_DOWN};
inline constexpr MediaKey MediaKey::PLAY{MEDIUS_MEDIA_PLAY};
inline constexpr MediaKey MediaKey::PAUSE{MEDIUS_MEDIA_PAUSE};

/// what a catch filter watches. the first four are the parsed input, the rest are byte oriented traffic
enum class CatchClass : std::uint8_t {
    Button = MEDIUS_CATCH_CLASS_BTN,
    Key = MEDIUS_CATCH_CLASS_KEY,
    Media = MEDIUS_CATCH_CLASS_MEDIA,
    Axis = MEDIUS_CATCH_CLASS_AXIS,
    HidIn = MEDIUS_CATCH_CLASS_HID_IN,
    HidOut = MEDIUS_CATCH_CLASS_HID_OUT,
    VendorInterrupt = MEDIUS_CATCH_CLASS_VENDOR_INTERRUPT,
    VendorBulk = MEDIUS_CATCH_CLASS_VENDOR_BULK,
    Control = MEDIUS_CATCH_CLASS_CONTROL,
    Emit = MEDIUS_CATCH_CLASS_EMIT,
    Bus = MEDIUS_CATCH_CLASS_BUS,
    ClipTransfer = MEDIUS_CATCH_CLASS_CLIP_TRANSFER,
};

/// the byte oriented catch classes
enum class TrafficClass : std::uint8_t {
    HidIn = MEDIUS_CATCH_CLASS_HID_IN,
    HidOut = MEDIUS_CATCH_CLASS_HID_OUT,
    VendorInterrupt = MEDIUS_CATCH_CLASS_VENDOR_INTERRUPT,
    VendorBulk = MEDIUS_CATCH_CLASS_VENDOR_BULK,
    Control = MEDIUS_CATCH_CLASS_CONTROL,
    Emit = MEDIUS_CATCH_CLASS_EMIT,
    Bus = MEDIUS_CATCH_CLASS_BUS,
    ClipTransfer = MEDIUS_CATCH_CLASS_CLIP_TRANSFER,
};

/// the bootloader's state for the image a chip runs. one this header has no name for comes through as its number, like
/// Rust's `Unknown(n)`
enum class ImageState : std::uint8_t {
    New = 0,
    PendingVerify = 1,
    Valid = 2,
    Invalid = 3,
    Aborted = 4,
    Unknown = 0xFF,
};

/// which chip a firmware update goes to
enum class UpdateTarget : std::uint8_t {
    Device = 0,
    Host = 1,
};

namespace detail {

template <>
struct Enum<EmitMode> {
    static constexpr const char* name = "EmitMode";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(EmitMode value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_EMIT_MODE_LEARNED:
        case MEDIUS_EMIT_MODE_INTERVAL:
        case MEDIUS_EMIT_MODE_FIXED:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<BearingMode> {
    static constexpr const char* name = "BearingMode";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(BearingMode value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_BEARING_MODE_PER_AXIS:
        case MEDIUS_BEARING_MODE_VECTOR:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<RenderMode> {
    static constexpr const char* name = "RenderMode";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(RenderMode value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_RENDER_MODE_OFF:
        case MEDIUS_RENDER_MODE_STOCK:
        case MEDIUS_RENDER_MODE_DESPIKED:
        case MEDIUS_RENDER_MODE_UNSMOOTHED:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<ControlStatus> {
    static constexpr const char* name = "ControlStatus";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(ControlStatus value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_CONTROL_STATUS_OK:
        case MEDIUS_CONTROL_STATUS_STALLED:
        case MEDIUS_CONTROL_STATUS_NAKED:
        case MEDIUS_CONTROL_STATUS_OTHER:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<BusEventKind> {
    static constexpr const char* name = "BusEventKind";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(BusEventKind value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_BUS_EVENT_KIND_RESET:
        case MEDIUS_BUS_EVENT_KIND_SUSPEND:
        case MEDIUS_BUS_EVENT_KIND_RESUME:
        case MEDIUS_BUS_EVENT_KIND_CONFIGURED:
        case MEDIUS_BUS_EVENT_KIND_DECONFIGURED:
        case MEDIUS_BUS_EVENT_KIND_SET_INTERFACE:
        case MEDIUS_BUS_EVENT_KIND_DEVICE_ATTACHED:
        case MEDIUS_BUS_EVENT_KIND_DEVICE_DETACHED:
        case MEDIUS_BUS_EVENT_KIND_CLONE_UP:
        case MEDIUS_BUS_EVENT_KIND_CLONE_DOWN:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<CatchEventKind> {
    static constexpr const char* name = "CatchEventKind";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(CatchEventKind value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_CATCH_EVENT_KIND_MOTION:
        case MEDIUS_CATCH_EVENT_KIND_USAGES:
        case MEDIUS_CATCH_EVENT_KIND_TRAFFIC:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<ClockDomain> {
    static constexpr const char* name = "ClockDomain";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(ClockDomain value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_CLOCK_DOMAIN_HOST_CHIP:
        case MEDIUS_CLOCK_DOMAIN_DEVICE_CHIP:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<InputKind> {
    static constexpr const char* name = "InputKind";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(InputKind value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_INPUT_KIND_PRESS:
        case MEDIUS_INPUT_KIND_RELEASE:
        case MEDIUS_INPUT_KIND_MOTION:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<LogLevel> {
    static constexpr const char* name = "LogLevel";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(LogLevel value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_LOG_LEVEL_ERROR:
        case MEDIUS_LOG_LEVEL_WARN:
        case MEDIUS_LOG_LEVEL_INFO:
        case MEDIUS_LOG_LEVEL_DEBUG:
        case MEDIUS_LOG_LEVEL_VERBOSE:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<FrameType> {
    static constexpr const char* name = "FrameType";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Any;

    static constexpr bool named(FrameType value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_FRAME_TYPE_MOVE:
        case MEDIUS_FRAME_TYPE_INJECT:
        case MEDIUS_FRAME_TYPE_RESET:
        case MEDIUS_FRAME_TYPE_QUERY:
        case MEDIUS_FRAME_TYPE_RESP:
        case MEDIUS_FRAME_TYPE_REBOOT_DL:
        case MEDIUS_FRAME_TYPE_LOG:
        case MEDIUS_FRAME_TYPE_LED:
        case MEDIUS_FRAME_TYPE_LOCK:
        case MEDIUS_FRAME_TYPE_CATCH:
        case MEDIUS_FRAME_TYPE_MOTION_EVENT:
        case MEDIUS_FRAME_TYPE_USAGE_EVENT:
        case MEDIUS_FRAME_TYPE_TRAFFIC_EVENT:
        case MEDIUS_FRAME_TYPE_OPTION:
        case MEDIUS_FRAME_TYPE_CLIP_APPEND:
        case MEDIUS_FRAME_TYPE_CLIP_CTRL:
        case MEDIUS_FRAME_TYPE_CLIP_SET:
        case MEDIUS_FRAME_TYPE_CLIP_TRIGGER:
        case MEDIUS_FRAME_TYPE_UPDATE:
        case MEDIUS_FRAME_TYPE_UPDATE_RESP:
        case MEDIUS_FRAME_TYPE_RAW:
        case MEDIUS_FRAME_TYPE_TRANSFER:
        case MEDIUS_FRAME_TYPE_TRANSFER_RESP:
        case MEDIUS_FRAME_TYPE_REWRITE:
        case MEDIUS_FRAME_TYPE_PATCH:
        case MEDIUS_FRAME_TYPE_TRANSFORM:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<Action> {
    static constexpr const char* name = "Action";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(Action value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_ACTION_SOFT_RELEASE:
        case MEDIUS_ACTION_PRESS:
        case MEDIUS_ACTION_FORCE_RELEASE:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<Axis> {
    static constexpr const char* name = "Axis";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(Axis value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_AXIS_X:
        case MEDIUS_AXIS_Y:
        case MEDIUS_AXIS_WHEEL:
        case MEDIUS_AXIS_PAN:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<Blanket> {
    static constexpr const char* name = "Blanket";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(Blanket value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_BLANKET_AIM:
        case MEDIUS_BLANKET_WHEEL:
        case MEDIUS_BLANKET_BUTTONS:
        case MEDIUS_BLANKET_KEYS:
        case MEDIUS_BLANKET_MEDIA:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<Class> {
    static constexpr const char* name = "Class";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(Class value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_CLASS_BUTTON:
        case MEDIUS_CLASS_KEY:
        case MEDIUS_CLASS_MEDIA:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<ClipAction> {
    static constexpr const char* name = "ClipAction";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(ClipAction value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_CLIP_ACTION_START:
        case MEDIUS_CLIP_ACTION_STOP:
        case MEDIUS_CLIP_ACTION_PAUSE:
        case MEDIUS_CLIP_ACTION_RESUME:
        case MEDIUS_CLIP_ACTION_RESTART:
        case MEDIUS_CLIP_ACTION_TOGGLE:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<ClipState> {
    static constexpr const char* name = "ClipState";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(ClipState value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_CLIP_STATE_IDLE:
        case MEDIUS_CLIP_STATE_PLAYING:
        case MEDIUS_CLIP_STATE_PAUSED:
        case MEDIUS_CLIP_STATE_FAULTED:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<DeviceKind> {
    static constexpr const char* name = "DeviceKind";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Unknown;
    static constexpr DeviceKind unknown = DeviceKind::Unknown;

    static constexpr bool named(DeviceKind value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_DEVICE_KIND_UNKNOWN:
        case MEDIUS_DEVICE_KIND_KEYBOARD:
        case MEDIUS_DEVICE_KIND_MOUSE:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<Direction> {
    static constexpr const char* name = "Direction";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(Direction value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_DIRECTION_BOTH:
        case MEDIUS_DIRECTION_POSITIVE:
        case MEDIUS_DIRECTION_NEGATIVE:
        case MEDIUS_DIRECTION_WITH:
        case MEDIUS_DIRECTION_AGAINST:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<Edge> {
    static constexpr const char* name = "Edge";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(Edge value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_EDGE_BOTH:
        case MEDIUS_EDGE_PRESS:
        case MEDIUS_EDGE_RELEASE:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<LedMode> {
    static constexpr const char* name = "LedMode";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(LedMode value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_LED_MODE_AUTO:
        case MEDIUS_LED_MODE_OFF:
        case MEDIUS_LED_MODE_SOLID:
        case MEDIUS_LED_MODE_BLINK:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<LedTarget> {
    static constexpr const char* name = "LedTarget";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(LedTarget value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_LED_TARGET_DEVICE:
        case MEDIUS_LED_TARGET_HOST:
        case MEDIUS_LED_TARGET_BOTH:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<LockTargetKind> {
    static constexpr const char* name = "LockTargetKind";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(LockTargetKind value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_LOCK_TARGET_KIND_X:
        case MEDIUS_LOCK_TARGET_KIND_Y:
        case MEDIUS_LOCK_TARGET_KIND_WHEEL:
        case MEDIUS_LOCK_TARGET_KIND_PAN:
        case MEDIUS_LOCK_TARGET_KIND_USAGE:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<MotionKind> {
    static constexpr const char* name = "MotionKind";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(MotionKind value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_MOTION_KIND_CURSOR:
        case MEDIUS_MOTION_KIND_WHEEL:
        case MEDIUS_MOTION_KIND_PAN:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<MoveTiming> {
    static constexpr const char* name = "MoveTiming";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(MoveTiming value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_MOVE_TIMING_RIDE:
        case MEDIUS_MOVE_TIMING_NOW:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<PatchSection> {
    static constexpr const char* name = "PatchSection";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(PatchSection value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_PATCH_SECTION_DEVICE:
        case MEDIUS_PATCH_SECTION_CONFIG:
        case MEDIUS_PATCH_SECTION_REPORT:
        case MEDIUS_PATCH_SECTION_STRING:
        case MEDIUS_PATCH_SECTION_BOS:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<PendingMotion> {
    static constexpr const char* name = "PendingMotion";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(PendingMotion value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_PENDING_MOTION_KEEP:
        case MEDIUS_PENDING_MOTION_FLUSH:
        case MEDIUS_PENDING_MOTION_DISCARD:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<RebootTarget> {
    static constexpr const char* name = "RebootTarget";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(RebootTarget value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_REBOOT_TARGET_DEVICE_DOWNLOAD:
        case MEDIUS_REBOOT_TARGET_HOST_DOWNLOAD:
        case MEDIUS_REBOOT_TARGET_DEVICE_RUN:
        case MEDIUS_REBOOT_TARGET_HOST_RUN:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<RewriteAction> {
    static constexpr const char* name = "RewriteAction";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(RewriteAction value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_REWRITE_ACTION_PASS:
        case MEDIUS_REWRITE_ACTION_DROP:
        case MEDIUS_REWRITE_ACTION_PATCH:
        case MEDIUS_REWRITE_ACTION_REPLACE:
        case MEDIUS_REWRITE_ACTION_ANSWER:
        case MEDIUS_REWRITE_ACTION_STALL:
        case MEDIUS_REWRITE_ACTION_NAK:
        case MEDIUS_REWRITE_ACTION_REPLY_PATCH:
        case MEDIUS_REWRITE_ACTION_REPLY_REPLACE:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<RewriteClass> {
    static constexpr const char* name = "RewriteClass";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(RewriteClass value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_REWRITE_CLASS_HID_IN:
        case MEDIUS_REWRITE_CLASS_HID_OUT:
        case MEDIUS_REWRITE_CLASS_VENDOR_INTERRUPT:
        case MEDIUS_REWRITE_CLASS_VENDOR_BULK:
        case MEDIUS_REWRITE_CLASS_CONTROL:
        case MEDIUS_REWRITE_CLASS_EMIT:
        case MEDIUS_REWRITE_CLASS_ANY:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<TransferStatus> {
    static constexpr const char* name = "TransferStatus";
    static constexpr bool checked = false;
    static constexpr Read read = Read::Any;

    static constexpr bool named(TransferStatus value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_TRANSFER_STATUS_OK:
        case MEDIUS_TRANSFER_STATUS_REFUSED:
        case MEDIUS_TRANSFER_STATUS_STALL:
        case MEDIUS_TRANSFER_STATUS_NAK:
        case MEDIUS_TRANSFER_STATUS_NO_DEVICE:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<TransformOp> {
    static constexpr const char* name = "TransformOp";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(TransformOp value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_TRANSFORM_OP_REMAP:
        case MEDIUS_TRANSFORM_OP_SWAP:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<CatchClass> {
    static constexpr const char* name = "CatchClass";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(CatchClass value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_CATCH_CLASS_BTN:
        case MEDIUS_CATCH_CLASS_KEY:
        case MEDIUS_CATCH_CLASS_MEDIA:
        case MEDIUS_CATCH_CLASS_AXIS:
        case MEDIUS_CATCH_CLASS_HID_IN:
        case MEDIUS_CATCH_CLASS_HID_OUT:
        case MEDIUS_CATCH_CLASS_VENDOR_INTERRUPT:
        case MEDIUS_CATCH_CLASS_VENDOR_BULK:
        case MEDIUS_CATCH_CLASS_CONTROL:
        case MEDIUS_CATCH_CLASS_EMIT:
        case MEDIUS_CATCH_CLASS_BUS:
        case MEDIUS_CATCH_CLASS_CLIP_TRANSFER:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<TrafficClass> {
    static constexpr const char* name = "TrafficClass";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(TrafficClass value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case MEDIUS_CATCH_CLASS_HID_IN:
        case MEDIUS_CATCH_CLASS_HID_OUT:
        case MEDIUS_CATCH_CLASS_VENDOR_INTERRUPT:
        case MEDIUS_CATCH_CLASS_VENDOR_BULK:
        case MEDIUS_CATCH_CLASS_CONTROL:
        case MEDIUS_CATCH_CLASS_EMIT:
        case MEDIUS_CATCH_CLASS_BUS:
        case MEDIUS_CATCH_CLASS_CLIP_TRANSFER:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<ImageState> {
    static constexpr const char* name = "ImageState";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Any;

    static constexpr bool named(ImageState value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case 0:
        case 1:
        case 2:
        case 3:
        case 4:
        case 0xFF:
            return true;
        default:
            return false;
        }
    }
};

template <>
struct Enum<UpdateTarget> {
    static constexpr const char* name = "UpdateTarget";
    static constexpr bool checked = true;
    static constexpr Read read = Read::Strict;

    static constexpr bool named(UpdateTarget value) noexcept {
        switch (static_cast<std::uint8_t>(value)) {
        case 0:
        case 1:
            return true;
        default:
            return false;
        }
    }
};

}  // namespace detail
// the end of what gen-medius makes (enums)

// made by gen-medius from medius.h (constants), change its tables and run dotnet run bindings/csharp/tools/gen-medius.cs
inline constexpr auto MaxUsages = MEDIUS_MAX_USAGES;
inline constexpr auto MaxLocks = MEDIUS_MAX_LOCKS;
inline constexpr auto MaxLogText = MEDIUS_MAX_LOG_TEXT;
inline constexpr auto MaxPath = MEDIUS_MAX_PATH;
inline constexpr auto MaxProduct = MEDIUS_MAX_PRODUCT;
inline constexpr auto MaxName = MEDIUS_MAX_NAME;
inline constexpr auto MaxSerial = MEDIUS_MAX_SERIAL;
inline constexpr auto MaxCatchEntries = MEDIUS_MAX_CATCH_ENTRIES;
inline constexpr auto MaxTrafficBytes = MEDIUS_MAX_TRAFFIC_BYTES;
inline constexpr auto MaxRewriteEntries = MEDIUS_MAX_REWRITE_ENTRIES;
inline constexpr auto MaxPatchEntries = MEDIUS_MAX_PATCH_ENTRIES;
inline constexpr auto MaxTransformEntries = MEDIUS_MAX_TRANSFORM_ENTRIES;
inline constexpr auto MaxRewriteMatch = MEDIUS_MAX_REWRITE_MATCH;
inline constexpr auto RewritePayloadPool = MEDIUS_REWRITE_PAYLOAD_POOL;
inline constexpr auto MaxDevPayload = MEDIUS_MAX_DEV_PAYLOAD;
inline constexpr auto CatchClassAny = MEDIUS_CATCH_CLASS_ANY;
inline constexpr auto CatchIdAny = MEDIUS_CATCH_ID_ANY;
inline constexpr auto ClockAgeNone = MEDIUS_CLOCK_AGE_NONE;
inline constexpr auto ClockRateNone = MEDIUS_CLOCK_RATE_NONE;
inline constexpr auto LockScaleBlock = MEDIUS_LOCK_SCALE_BLOCK;
inline constexpr auto LockScalePass = MEDIUS_LOCK_SCALE_PASS;
inline constexpr auto LockScaleMax = MEDIUS_LOCK_SCALE_MAX;
inline constexpr auto LockScaleMin = MEDIUS_LOCK_SCALE_MIN;
inline constexpr auto BearingWindowDefaultMs = MEDIUS_BEARING_WINDOW_DEFAULT_MS;
inline constexpr auto ClipTrigMax = MEDIUS_CLIP_TRIG_MAX;
inline constexpr auto ClipPktTrigMax = MEDIUS_CLIP_PKT_TRIG_MAX;
inline constexpr auto ClipPktMatchPool = MEDIUS_CLIP_PKT_MATCH_POOL;
inline constexpr auto MaxPktMatch = MEDIUS_MAX_PKT_MATCH;
inline constexpr auto ClipEdgesMax = MEDIUS_CLIP_EDGES_MAX;
inline constexpr auto ClipRawMax = MEDIUS_CLIP_RAW_MAX;
inline constexpr auto ClipEntryMax = MEDIUS_CLIP_ENTRY_MAX;
inline constexpr auto AbiVersion = MEDIUS_ABI_VERSION;
// the end of what gen-medius makes (constants)

namespace detail {

// every way in that can be the first call checks both libraries before it, like anybox.hpp does
inline void ready() {
    anybox::detail::ensure();
    anybox::detail::ensure_medius();
}

// a value going in has to be one the enum names, otherwise the library reads a stray byte as whatever it happens to be
template <class E>
std::underlying_type_t<E> write(E value) {
    const auto number = static_cast<std::underlying_type_t<E>>(value);
    if constexpr (Enum<E>::checked) {
        if (!Enum<E>::named(value)) {
            throw std::invalid_argument(std::to_string(static_cast<long long>(number)) + " isn't a " + Enum<E>::name +
                                        " this header knows");
        }
    }
    return number;
}

template <class E, class Raw>
E read(Raw raw) {
    const auto value = static_cast<E>(static_cast<std::underlying_type_t<E>>(raw));
    if constexpr (Enum<E>::read == Read::Strict) {
        if (!Enum<E>::named(value)) {
            throw anybox::Error(std::string("the library sent a ") + Enum<E>::name + " this header doesn't know (" +
                                std::to_string(static_cast<long long>(raw)) +
                                "), so build against the anybox.hpp that came with the library");
        }
    } else if constexpr (Enum<E>::read == Read::Unknown) {
        if (!Enum<E>::named(value)) {
            return Enum<E>::unknown;
        }
    }
    return value;
}

// a C string in a fixed size array, up to its first NUL. what the box sends isn't checked UTF-8, so it comes as bytes
template <std::size_t N>
std::string text(const char (&raw)[N]) {
    std::size_t length = 0;
    while (length < N && raw[length] != '\0') {
        length++;
    }
    return std::string(raw, length);
}

// cut to fit with room for the NUL, the way medius cuts a name itself
template <std::size_t N>
void text(const std::string& value, char (&raw)[N]) {
    anybox::detail::c_text(value);
    const auto length = value.size() < N - 1 ? value.size() : N - 1;
    value.copy(raw, length);
    raw[length] = '\0';
}

// a count can be bigger than its array, so only what the array holds is read
template <std::size_t N, class Count>
std::vector<std::uint8_t> bytes(const std::uint8_t (&raw)[N], Count count) {
    const auto kept = static_cast<std::size_t>(count) < N ? static_cast<std::size_t>(count) : N;
    return std::vector<std::uint8_t>(raw, raw + kept);
}

// bytes that don't fit are refused, the library would cut them without a word
template <std::size_t N>
std::size_t bytes(const std::vector<std::uint8_t>& value, std::uint8_t (&raw)[N], const char* what) {
    if (value.size() > N) {
        throw std::invalid_argument(std::string(what) + " is " + std::to_string(value.size()) + " bytes, over the " +
                                    std::to_string(N) + " medius.h has room for");
    }
    std::copy(value.begin(), value.end(), raw);
    return value.size();
}

// a match or mask goes over with its whole length and as many bytes as the array holds, so one that's too long comes
// back as medius' own refusal
template <std::size_t N>
std::size_t match(const std::vector<std::uint8_t>& value, std::uint8_t (&raw)[N], const char* what) {
    if (value.size() > 0xFFFF) {
        throw std::invalid_argument(std::string(what) + " is " + std::to_string(value.size()) +
                                    " bytes, over what medius.h can say");
    }
    std::copy(value.begin(), value.begin() + static_cast<std::ptrdiff_t>(value.size() < N ? value.size() : N), raw);
    return value.size();
}

template <class T, class Raw, std::size_t N, class Count>
std::vector<T> list(const Raw (&raw)[N], Count count) {
    const auto kept = static_cast<std::size_t>(count) < N ? static_cast<std::size_t>(count) : N;
    std::vector<T> out;
    out.reserve(kept);
    for (std::size_t index = 0; index < kept; index++) {
        out.push_back(T::from_c(raw[index]));
    }
    return out;
}

template <class T, class Raw, std::size_t N>
std::size_t list(const std::vector<T>& values, Raw (&raw)[N], const char* what) {
    if (values.size() > N) {
        throw std::invalid_argument(std::string(what) + " has " + std::to_string(values.size()) + " entries, over the " +
                                    std::to_string(N) + " medius.h has room for");
    }
    for (std::size_t index = 0; index < values.size(); index++) {
        raw[index] = values[index].to_c();
    }
    return values.size();
}

}  // namespace detail

namespace detail {

// the status' own name, or its number when this header has none, what an error with no text of its own says
inline std::string status_text(Status status);

}  // namespace detail

/// a medius call that failed, with the library's text. `medius_status()` is medius.h's number for it and
/// `proto_ver()` the protocol byte a box on another protocol answered with. it's an anybox::Error, so one catch takes
/// everything anybox throws
class Error : public anybox::Error {
public:
    Error(const std::string& message, Status status, std::uint8_t proto_ver)
        : anybox::Error(message.empty() ? detail::status_text(status) : message),
          status_(status), proto_ver_(proto_ver) {}

    Status medius_status() const noexcept { return status_; }
    std::uint8_t proto_ver() const noexcept { return proto_ver_; }

private:
    void throw_self() const override { throw *this; }

    Status status_;
    std::uint8_t proto_ver_;
};

/// an update op the box refused, or a confirm that ran out of time. `op()` is the op (0 begin, 1 data, 2 end, 3 abort,
/// 4 activate), `update_status()` the box's status byte for it (0x1B is a chip still on probation) and `arg()` what
/// goes with that status. a confirm that ran out of time gives op 0 and 0x1B, the refusal the next update would get.
/// all three are empty when the library had no detail
class UpdateError : public Error {
public:
    UpdateError(const std::string& message, std::uint8_t proto_ver, const std::optional<MediusUpdateError>& update)
        : Error(message, Status::ErrUpdate, proto_ver) {
        if (update) {
            op_ = update->op;
            update_status_ = update->status;
            arg_ = update->arg;
        }
    }

    std::optional<std::uint8_t> op() const noexcept { return op_; }
    std::optional<std::uint8_t> update_status() const noexcept { return update_status_; }
    std::optional<std::uint32_t> arg() const noexcept { return arg_; }

private:
    void throw_self() const override { throw *this; }

    std::optional<std::uint8_t> op_;
    std::optional<std::uint8_t> update_status_;
    std::optional<std::uint32_t> arg_;
};

// made by gen-medius from medius.h (errors), change its tables and run dotnet run bindings/csharp/tools/gen-medius.cs
class IoError : public Error {
public:
    IoError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrIo, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class NotFoundError : public Error {
public:
    NotFoundError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrNotFound, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class NoReplyError : public Error {
public:
    NoReplyError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrNoReply, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class BadProtoVerError : public Error {
public:
    BadProtoVerError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrBadProtoVer, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class QueryTimeoutError : public Error {
public:
    QueryTimeoutError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrQueryTimeout, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class DisconnectedError : public Error {
public:
    DisconnectedError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrDisconnected, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class FrameTooLongError : public Error {
public:
    FrameTooLongError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrFrameTooLong, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class InvalidArgError : public Error {
public:
    InvalidArgError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrInvalidArg, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class PanicError : public Error {
public:
    PanicError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrPanic, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class CatchTableFullError : public Error {
public:
    CatchTableFullError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrCatchTableFull, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class EmptySubscriptionError : public Error {
public:
    EmptySubscriptionError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrEmptySubscription, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class CaptureNotApplicableError : public Error {
public:
    CaptureNotApplicableError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrCaptureNotApplicable, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class NotAnInputFilterError : public Error {
public:
    NotAnInputFilterError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrNotAnInputFilter, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class WildcardNotInputError : public Error {
public:
    WildcardNotInputError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrWildcardNotInput, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class HalfEdgeInputFilterError : public Error {
public:
    HalfEdgeInputFilterError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrHalfEdgeInputFilter, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class ReservedIdError : public Error {
public:
    ReservedIdError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrReservedId, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class RelativeDirectionError : public Error {
public:
    RelativeDirectionError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrRelativeDirection, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class LockScaleRangeError : public Error {
public:
    LockScaleRangeError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrLockScaleRange, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class LockScaleUsageError : public Error {
public:
    LockScaleUsageError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrLockScaleUsage, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class ImperfectRequiredError : public Error {
public:
    ImperfectRequiredError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrImperfectRequired, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class RewriteMaskLengthError : public Error {
public:
    RewriteMaskLengthError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrRewriteMaskLength, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class RewriteActionClassError : public Error {
public:
    RewriteActionClassError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrRewriteActionClass, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class RewritePayloadTooLargeError : public Error {
public:
    RewritePayloadTooLargeError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrRewritePayloadTooLarge, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class RewriteTableFullError : public Error {
public:
    RewriteTableFullError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrRewriteTableFull, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class TransformOpFieldsError : public Error {
public:
    TransformOpFieldsError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrTransformOpFields, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class TransformTableFullError : public Error {
public:
    TransformTableFullError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrTransformTableFull, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class RawDirectionError : public Error {
public:
    RawDirectionError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrRawDirection, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class ClipFrameCountError : public Error {
public:
    ClipFrameCountError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrClipFrameCount, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class ClipFrameTooLongError : public Error {
public:
    ClipFrameTooLongError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrClipFrameTooLong, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class ClipTransferDataError : public Error {
public:
    ClipTransferDataError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrClipTransferData, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class ClipPacketTriggerError : public Error {
public:
    ClipPacketTriggerError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrClipPacketTrigger, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class RewriteMatchTooLongError : public Error {
public:
    RewriteMatchTooLongError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrRewriteMatchTooLong, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

class RewritePoolFullError : public Error {
public:
    RewritePoolFullError(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::ErrRewritePoolFull, proto_ver) {}

private:
    void throw_self() const override { throw *this; }
};

namespace detail {

inline std::string status_text(Status status) {
    switch (status) {
    case Status::Ok:
        return "Ok";
    case Status::ErrIo:
        return "ErrIo";
    case Status::ErrNotFound:
        return "ErrNotFound";
    case Status::ErrNoReply:
        return "ErrNoReply";
    case Status::ErrBadProtoVer:
        return "ErrBadProtoVer";
    case Status::ErrQueryTimeout:
        return "ErrQueryTimeout";
    case Status::ErrDisconnected:
        return "ErrDisconnected";
    case Status::ErrFrameTooLong:
        return "ErrFrameTooLong";
    case Status::ErrUpdate:
        return "ErrUpdate";
    case Status::ErrInvalidArg:
        return "ErrInvalidArg";
    case Status::ErrPanic:
        return "ErrPanic";
    case Status::ErrUnknown:
        return "ErrUnknown";
    case Status::ErrCatchTableFull:
        return "ErrCatchTableFull";
    case Status::ErrEmptySubscription:
        return "ErrEmptySubscription";
    case Status::ErrCaptureNotApplicable:
        return "ErrCaptureNotApplicable";
    case Status::ErrNotAnInputFilter:
        return "ErrNotAnInputFilter";
    case Status::ErrWildcardNotInput:
        return "ErrWildcardNotInput";
    case Status::ErrHalfEdgeInputFilter:
        return "ErrHalfEdgeInputFilter";
    case Status::ErrReservedId:
        return "ErrReservedId";
    case Status::ErrRelativeDirection:
        return "ErrRelativeDirection";
    case Status::ErrLockScaleRange:
        return "ErrLockScaleRange";
    case Status::ErrLockScaleUsage:
        return "ErrLockScaleUsage";
    case Status::ErrImperfectRequired:
        return "ErrImperfectRequired";
    case Status::ErrRewriteMaskLength:
        return "ErrRewriteMaskLength";
    case Status::ErrRewriteActionClass:
        return "ErrRewriteActionClass";
    case Status::ErrRewritePayloadTooLarge:
        return "ErrRewritePayloadTooLarge";
    case Status::ErrRewriteTableFull:
        return "ErrRewriteTableFull";
    case Status::ErrTransformOpFields:
        return "ErrTransformOpFields";
    case Status::ErrTransformTableFull:
        return "ErrTransformTableFull";
    case Status::ErrRawDirection:
        return "ErrRawDirection";
    case Status::ErrClipFrameCount:
        return "ErrClipFrameCount";
    case Status::ErrClipFrameTooLong:
        return "ErrClipFrameTooLong";
    case Status::ErrClipTransferData:
        return "ErrClipTransferData";
    case Status::ErrClipPacketTrigger:
        return "ErrClipPacketTrigger";
    case Status::ErrRewriteMatchTooLong:
        return "ErrRewriteMatchTooLong";
    case Status::ErrRewritePoolFull:
        return "ErrRewritePoolFull";
    }
    return std::to_string(static_cast<std::int32_t>(status));
}

// the error of that status, the one switch every medius error goes through
inline std::shared_ptr<Error> make_error(MediusStatus status, const std::string& message, std::uint8_t proto_ver,
                                         const std::optional<MediusUpdateError>& update) {
    switch (status) {
    case MEDIUS_STATUS_ERR_IO:
        return std::make_shared<IoError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_NOT_FOUND:
        return std::make_shared<NotFoundError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_NO_REPLY:
        return std::make_shared<NoReplyError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_BAD_PROTO_VER:
        return std::make_shared<BadProtoVerError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_QUERY_TIMEOUT:
        return std::make_shared<QueryTimeoutError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_DISCONNECTED:
        return std::make_shared<DisconnectedError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_FRAME_TOO_LONG:
        return std::make_shared<FrameTooLongError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_UPDATE:
        return std::make_shared<UpdateError>(message, proto_ver, update);
    case MEDIUS_STATUS_ERR_INVALID_ARG:
        return std::make_shared<InvalidArgError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_PANIC:
        return std::make_shared<PanicError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_CATCH_TABLE_FULL:
        return std::make_shared<CatchTableFullError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_EMPTY_SUBSCRIPTION:
        return std::make_shared<EmptySubscriptionError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_CAPTURE_NOT_APPLICABLE:
        return std::make_shared<CaptureNotApplicableError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_NOT_AN_INPUT_FILTER:
        return std::make_shared<NotAnInputFilterError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_WILDCARD_NOT_INPUT:
        return std::make_shared<WildcardNotInputError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_HALF_EDGE_INPUT_FILTER:
        return std::make_shared<HalfEdgeInputFilterError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_RESERVED_ID:
        return std::make_shared<ReservedIdError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_RELATIVE_DIRECTION:
        return std::make_shared<RelativeDirectionError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_LOCK_SCALE_RANGE:
        return std::make_shared<LockScaleRangeError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_LOCK_SCALE_USAGE:
        return std::make_shared<LockScaleUsageError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_IMPERFECT_REQUIRED:
        return std::make_shared<ImperfectRequiredError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_REWRITE_MASK_LENGTH:
        return std::make_shared<RewriteMaskLengthError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_REWRITE_ACTION_CLASS:
        return std::make_shared<RewriteActionClassError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_REWRITE_PAYLOAD_TOO_LARGE:
        return std::make_shared<RewritePayloadTooLargeError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_REWRITE_TABLE_FULL:
        return std::make_shared<RewriteTableFullError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_TRANSFORM_OP_FIELDS:
        return std::make_shared<TransformOpFieldsError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_TRANSFORM_TABLE_FULL:
        return std::make_shared<TransformTableFullError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_RAW_DIRECTION:
        return std::make_shared<RawDirectionError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_CLIP_FRAME_COUNT:
        return std::make_shared<ClipFrameCountError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_CLIP_FRAME_TOO_LONG:
        return std::make_shared<ClipFrameTooLongError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_CLIP_TRANSFER_DATA:
        return std::make_shared<ClipTransferDataError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_CLIP_PACKET_TRIGGER:
        return std::make_shared<ClipPacketTriggerError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_REWRITE_MATCH_TOO_LONG:
        return std::make_shared<RewriteMatchTooLongError>(message, proto_ver);
    case MEDIUS_STATUS_ERR_REWRITE_POOL_FULL:
        return std::make_shared<RewritePoolFullError>(message, proto_ver);
    default:
        return std::make_shared<Error>(message, static_cast<Status>(status), proto_ver);
    }
}

}  // namespace detail
// the end of what gen-medius makes (errors)

namespace detail {

// the text is thread local on the C side, so this has to run right after the call that failed, on its thread
inline std::shared_ptr<Error> last_error(MediusStatus status) {
    std::string message(256, '\0');
    const auto full = medius_last_error_message(&message[0], message.size());
    if (full >= message.size()) {
        message.assign(full + 1, '\0');
        medius_last_error_message(&message[0], message.size());
    }
    message.resize(full < message.size() ? full : message.size());
    MediusUpdateError raw{};
    std::optional<MediusUpdateError> update;
    if (medius_last_error_update(&raw)) {
        update = raw;
    }
    return make_error(status, message, medius_last_error_proto_ver(), update);
}

inline void check(MediusStatus status) {
    if (status != MEDIUS_STATUS_OK) {
        last_error(status)->raise();
    }
}

}  // namespace detail

/// a momentary usage, a button, a key or a media key. a Button, Key or MediaKey turns into one by itself
struct Usage {
    Class kind = Class::Button;
    std::uint16_t id = 0;

    Usage() = default;
    constexpr Usage(Class usage_class, std::uint16_t usage_id) noexcept : kind(usage_class), id(usage_id) {}
    Usage(Button button) : Usage(Usage::button(button.value)) {}
    Usage(Key key) : Usage(Usage::key(key)) {}
    Usage(MediaKey media) : Usage(Usage::media(media)) {}

    /// any id up to the clone's button count is a button, so this takes a plain number too
    static Usage button(std::uint8_t button) {
        detail::ready();
        return from_c(medius_usage_button(button));
    }

    static Usage key(Key key) {
        detail::ready();
        return from_c(medius_usage_key(key.value));
    }

    static Usage media(MediaKey media) {
        detail::ready();
        return from_c(medius_usage_media(media.value));
    }

    static Usage from_c(const MediusUsage& raw) { return Usage(detail::read<Class>(raw.kind), raw.id); }
    MediusUsage to_c() const { return MediusUsage{detail::write(kind), id}; }

    friend bool operator==(const Usage& a, const Usage& b) noexcept { return a.kind == b.kind && a.id == b.id; }
    friend bool operator!=(const Usage& a, const Usage& b) noexcept { return !(a == b); }
};

/// a relative axis drive, made with `cursor()`, `wheel()` or `pan()`
class Motion {
public:
    static Motion cursor(std::int16_t dx, std::int16_t dy) {
        detail::ready();
        return Motion(medius_motion_cursor(dx, dy));
    }

    static Motion wheel(std::int16_t delta) {
        detail::ready();
        return Motion(medius_motion_wheel(delta));
    }

    static Motion pan(std::int16_t delta) {
        detail::ready();
        return Motion(medius_motion_pan(delta));
    }

    MotionKind kind() const { return detail::read<MotionKind>(raw_.kind); }

    /// the cursor's x, 0 for the wheel and the pan
    std::int16_t dx() const { return kind() == MotionKind::Cursor ? raw_.dx : std::int16_t{0}; }

    /// the cursor's y, 0 for the wheel and the pan
    std::int16_t dy() const { return kind() == MotionKind::Cursor ? raw_.dy : std::int16_t{0}; }

    /// the wheel's or the pan's delta, 0 for the cursor
    std::int16_t delta() const {
        switch (kind()) {
        case MotionKind::Wheel:
            return raw_.wheel;
        case MotionKind::Pan:
            return raw_.pan;
        default:
            return 0;
        }
    }

    MediusMotion to_c() const noexcept { return raw_; }

    friend bool operator==(const Motion& a, const Motion& b) noexcept {
        return a.raw_.kind == b.raw_.kind && a.raw_.dx == b.raw_.dx && a.raw_.dy == b.raw_.dy &&
               a.raw_.wheel == b.raw_.wheel && a.raw_.pan == b.raw_.pan;
    }
    friend bool operator!=(const Motion& a, const Motion& b) noexcept { return !(a == b); }

private:
    explicit Motion(const MediusMotion& raw) noexcept : raw_(raw) {}

    MediusMotion raw_{};
};

/// what a lock, a scale or a transform works on, an axis or a momentary usage. an Axis or anything that's a usage
/// turns into one by itself
class LockTarget {
public:
    LockTarget() = default;
    LockTarget(Axis axis) : LockTarget(LockTarget::axis(axis)) {}
    LockTarget(const Usage& usage) : LockTarget(LockTarget::usage(usage)) {}
    LockTarget(Button button) : LockTarget(Usage(button)) {}
    LockTarget(Key key) : LockTarget(Usage(key)) {}
    LockTarget(MediaKey media) : LockTarget(Usage(media)) {}

    static LockTarget axis(Axis axis) {
        detail::ready();
        return LockTarget(medius_lock_target_axis(detail::write(axis)));
    }

    static LockTarget usage(const Usage& usage) {
        detail::ready();
        return LockTarget(medius_lock_target_usage(usage.to_c()));
    }

    static LockTarget button(std::uint8_t button) { return usage(Usage::button(button)); }
    static LockTarget key(Key key) { return usage(Usage::key(key)); }
    static LockTarget media(MediaKey media) { return usage(Usage::media(media)); }

    LockTargetKind kind() const { return detail::read<LockTargetKind>(raw_.kind); }

    /// the usage, when `kind()` is `LockTargetKind::Usage`, empty for an axis
    std::optional<Usage> input() const {
        if (kind() != LockTargetKind::Usage) {
            return std::nullopt;
        }
        return Usage::from_c(raw_.usage);
    }

    /// the axis, empty for a usage
    std::optional<Axis> as_axis() const {
        switch (kind()) {
        case LockTargetKind::X:
            return Axis::X;
        case LockTargetKind::Y:
            return Axis::Y;
        case LockTargetKind::Wheel:
            return Axis::Wheel;
        case LockTargetKind::Pan:
            return Axis::Pan;
        default:
            return std::nullopt;
        }
    }

    static LockTarget from_c(const MediusLockTarget& raw) {
        detail::read<LockTargetKind>(raw.kind);
        return LockTarget(raw);
    }

    MediusLockTarget to_c() const {
        MediusLockTarget raw = raw_;
        raw.kind = detail::write(detail::read<LockTargetKind>(raw_.kind));
        return raw;
    }

    friend bool operator==(const LockTarget& a, const LockTarget& b) noexcept {
        return a.raw_.kind == b.raw_.kind && a.raw_.usage.kind == b.raw_.usage.kind && a.raw_.usage.id == b.raw_.usage.id;
    }
    friend bool operator!=(const LockTarget& a, const LockTarget& b) noexcept { return !(a == b); }

private:
    explicit LockTarget(const MediusLockTarget& raw) noexcept : raw_(raw) {}

    MediusLockTarget raw_{};
};

/// how much of each packet a catch keeps, traffic classes only, a capture on an input class is refused. it's a
/// ceiling the box holds once per address, so another subscriber asking for more raises yours too
struct Capture {
    static constexpr std::uint8_t Whole = 0;

    /// keeps the first `n` bytes, 0 is `Whole`
    static constexpr std::uint8_t first(std::uint8_t n) noexcept { return n; }

    /// how many bytes a capture keeps, empty for the whole packet
    static constexpr std::optional<std::uint8_t> bytes(std::uint8_t capture) noexcept {
        return capture == Whole ? std::nullopt : std::optional<std::uint8_t>(capture);
    }

    /// the wider of two, the whole packet if either keeps it, otherwise the longer one
    static constexpr std::uint8_t widest(std::uint8_t a, std::uint8_t b) noexcept {
        return a == Whole || b == Whole ? Whole : (a > b ? a : b);
    }
};

/// one catch subscription, what to watch, which way, and how much of each packet to keep. an event goes to its most
/// specific entry, an exact class and id over a class, a class over `everything()`, a named direction over both
class CatchFilter {
public:
    CatchFilter() = default;

    static CatchFilter watch(const Usage& usage) {
        detail::ready();
        return CatchFilter(medius_catch_filter_watch(usage.to_c()));
    }

    static CatchFilter watch_axis(Axis axis) {
        detail::ready();
        return CatchFilter(medius_catch_filter_watch_axis(detail::write(axis)));
    }

    /// every usage of one momentary class
    static CatchFilter watch_class(Class input_class) {
        detail::ready();
        return CatchFilter(medius_catch_filter_watch_class(detail::write(input_class)));
    }

    /// every relative axis, X, Y, the wheel and pan
    static CatchFilter watch_axes() {
        detail::ready();
        return CatchFilter(medius_catch_filter_watch_axes());
    }

    /// all four input classes, everything `Device::input_events` can report
    static std::vector<CatchFilter> all_input() {
        detail::ready();
        MediusCatchFilter raw[4] = {};
        medius_catch_filter_all_input(raw);
        return {CatchFilter(raw[0]), CatchFilter(raw[1]), CatchFilter(raw[2]), CatchFilter(raw[3])};
    }

    /// one traffic address, an endpoint, an interface or a control endpoint number
    static CatchFilter traffic(TrafficClass traffic_class, std::uint16_t id) {
        detail::ready();
        return CatchFilter(medius_catch_filter_traffic(detail::write(traffic_class), id));
    }

    /// every id of one traffic class
    static CatchFilter traffic_of(TrafficClass traffic_class) {
        detail::ready();
        return CatchFilter(medius_catch_filter_traffic_class(detail::write(traffic_class)));
    }

    /// every class, every id, both ways, whole packets, as one entry. vendor bulk alone can fill the control link, so
    /// pair it with `with_capture()` unless you really want bulk in full
    static CatchFilter everything() {
        detail::ready();
        return CatchFilter(medius_catch_filter_everything());
    }

    CatchFilter with_direction(Direction direction) const {
        detail::ready();
        return CatchFilter(medius_catch_filter_with_direction(raw_, detail::write(direction)));
    }

    /// keeps only the first `bytes` of each packet, 0 keeps the whole packet
    CatchFilter with_capture(std::uint8_t bytes) const {
        detail::ready();
        return CatchFilter(medius_catch_filter_with_capture(raw_, bytes));
    }

    CatchFilter on_press() const {
        detail::ready();
        return CatchFilter(medius_catch_filter_on_press(raw_));
    }

    CatchFilter on_release() const {
        detail::ready();
        return CatchFilter(medius_catch_filter_on_release(raw_));
    }

    /// only traffic from the device to the PC
    CatchFilter inbound() const {
        detail::ready();
        return CatchFilter(medius_catch_filter_inbound(raw_));
    }

    /// only traffic from the PC to the device
    CatchFilter outbound() const {
        detail::ready();
        return CatchFilter(medius_catch_filter_outbound(raw_));
    }

    /// whether both are the same entry in the box's table, whatever they capture
    bool same_address(const CatchFilter& other) const {
        detail::ready();
        return medius_catch_filter_same_address(raw_, other.raw_);
    }

    /// the class it watches, empty for every class
    std::optional<CatchClass> class_() const {
        if (raw_.class_ == MEDIUS_CATCH_CLASS_ANY) {
            return std::nullopt;
        }
        return detail::read<CatchClass>(raw_.class_);
    }

    /// the class specific id, empty for every id
    std::optional<std::uint16_t> id() const noexcept {
        if (raw_.id == MEDIUS_CATCH_ID_ANY) {
            return std::nullopt;
        }
        return raw_.id;
    }

    Direction direction() const { return detail::read<Direction>(raw_.direction); }

    /// bytes kept per event, 0 keeps the whole packet
    std::uint8_t capture() const noexcept { return raw_.capture; }

    static CatchFilter from_c(const MediusCatchFilter& raw) noexcept { return CatchFilter(raw); }
    MediusCatchFilter to_c() const noexcept { return raw_; }

    friend bool operator==(const CatchFilter& a, const CatchFilter& b) noexcept {
        return a.raw_.class_ == b.raw_.class_ && a.raw_.id == b.raw_.id && a.raw_.direction == b.raw_.direction &&
               a.raw_.capture == b.raw_.capture;
    }
    friend bool operator!=(const CatchFilter& a, const CatchFilter& b) noexcept { return !(a == b); }

private:
    explicit CatchFilter(const MediusCatchFilter& raw) noexcept : raw_(raw) {}

    MediusCatchFilter raw_{};
};

/// what paces injected motion, made with `learned()`, `interval()` or `fixed()`
class EmitPace {
public:
    EmitPace() = default;

    static EmitPace learned() noexcept { return EmitPace(EmitMode::Learned, 0); }
    static EmitPace interval() noexcept { return EmitPace(EmitMode::Interval, 0); }
    static EmitPace fixed(std::uint16_t hz) noexcept { return EmitPace(EmitMode::Fixed, hz); }

    EmitMode mode() const noexcept { return mode_; }

    /// the rate of a fixed one, 0 for the others
    std::uint16_t hz() const noexcept { return hz_; }

    friend bool operator==(const EmitPace& a, const EmitPace& b) noexcept { return a.mode_ == b.mode_ && a.hz_ == b.hz_; }
    friend bool operator!=(const EmitPace& a, const EmitPace& b) noexcept { return !(a == b); }

private:
    EmitPace(EmitMode mode, std::uint16_t hz) noexcept : mode_(mode), hz_(hz) {}

    EmitMode mode_ = EmitMode::Learned;
    std::uint16_t hz_ = 0;
};

/// how a catch class reads a `Direction`: the edge of a press or a release, the sign of a delta, or which way traffic
/// flows
enum class DirectionMeaning : std::uint8_t {
    Edge,
    Sign,
    Flow,
};

/// what makes a patch the same entry as another, a second patch with it replaces the first
struct PatchKey {
    PatchSection section{PatchSection::Device};
    std::uint8_t cfg = 0;
    std::uint8_t index = 0;
    std::uint16_t offset = 0;

    friend bool operator==(const PatchKey& a, const PatchKey& b) noexcept {
        return a.section == b.section && a.cfg == b.cfg && a.index == b.index && a.offset == b.offset;
    }
    friend bool operator!=(const PatchKey& a, const PatchKey& b) noexcept { return !(a == b); }
};

/// what makes a rewrite rule the same entry as another, a second rule with it replaces the first
struct RewriteKey {
    RewriteClass class_{RewriteClass::Emit};
    std::uint16_t id = 0;
    Direction direction{Direction::Both};
    std::vector<std::uint8_t> match_bytes;
    std::vector<std::uint8_t> mask;

    friend bool operator==(const RewriteKey& a, const RewriteKey& b) noexcept {
        return a.class_ == b.class_ && a.id == b.id && a.direction == b.direction && a.match_bytes == b.match_bytes &&
               a.mask == b.mask;
    }
    friend bool operator!=(const RewriteKey& a, const RewriteKey& b) noexcept { return !(a == b); }
};

/// what makes a transform the same entry as another, a second one with it replaces the first's op
struct TransformKey {
    LockTarget source;
    LockTarget dest;

    friend bool operator==(const TransformKey& a, const TransformKey& b) noexcept {
        return a.source == b.source && a.dest == b.dest;
    }
    friend bool operator!=(const TransformKey& a, const TransformKey& b) noexcept { return !(a == b); }
};

/// how far a staged update got, for its progress
struct UpdateProgress {
    UpdateTarget target{UpdateTarget::Device};
    std::size_t sent = 0;
    std::size_t total = 0;

    /// the share sent so far, 100 for an empty image
    std::uint8_t percent() const noexcept {
        return total == 0 ? std::uint8_t{100}
                          : static_cast<std::uint8_t>(static_cast<std::uint64_t>(sent) * 100 / total);
    }

    friend bool operator==(const UpdateProgress& a, const UpdateProgress& b) noexcept {
        return a.target == b.target && a.sent == b.sent && a.total == b.total;
    }
    friend bool operator!=(const UpdateProgress& a, const UpdateProgress& b) noexcept { return !(a == b); }
};

// made by gen-medius from medius.h (structs), change its tables and run dotnet run bindings/csharp/tools/gen-medius.cs
struct Setup;
struct ClipTrigger;
struct ClipPacketTrigger;
struct ClipStatus;
struct PortInfo;
struct Version;
struct DeviceInfo;
struct BoxInfo;
struct TransferOutcome;
struct RewriteRule;
struct RewriteEntry;
struct RewriteTable;
struct Patch;
struct PatchEntry;
struct PatchSet;
struct Transform;
struct Transforms;
struct ChipFirmware;
struct FirmwareInfo;
struct Health;
struct MouseCaps;
struct KbdCaps;
struct Caps;
struct Rate;
struct Stats;
struct LockEntry;
struct Locks;
struct ClockEstimate;
struct CatchEntry;
struct CatchState;
struct ImperfectStatus;
struct Bearing;
struct RenderStatus;
struct SpreadStatus;
struct CountersSnapshot;
struct UsageSnapshot;
struct TrafficEvent;
struct BusEvent;
struct MotionEvent;
struct Stamped;
struct LogLine;

/// a USB setup packet. `length` is the data stage, the bytes to read for IN, otherwise the length of the OUT data
struct Setup {
    std::uint8_t request_type = 0;
    std::uint8_t request = 0;
    std::uint16_t value = 0;
    std::uint16_t index = 0;
    std::uint16_t length = 0;

    /// a device to host request, by the direction bit of the request type
    bool is_in() const;
    /// the eight bytes on the wire, the numbers little endian
    std::array<std::uint8_t, 8> to_bytes() const;

    static Setup from_c(const MediusSetup& raw);
    MediusSetup to_c() const;

    friend bool operator==(const Setup& a, const Setup& b) noexcept {
        return a.request_type == b.request_type && a.request == b.request && a.value == b.value && a.index == b.index && a.length == b.length;
    }
    friend bool operator!=(const Setup& a, const Setup& b) noexcept { return !(a == b); }
};

inline Setup Setup::from_c(const MediusSetup& raw) {
    Setup out;
    out.request_type = raw.request_type;
    out.request = raw.request;
    out.value = raw.value;
    out.index = raw.index;
    out.length = raw.length;
    return out;
}

inline MediusSetup Setup::to_c() const {
    MediusSetup raw{};
    raw.request_type = request_type;
    raw.request = request;
    raw.value = value;
    raw.index = index;
    raw.length = length;
    return raw;
}

/// one clip input trigger. `on`'s `edge` drives `action`, and `consume` keeps the input from the game
struct ClipTrigger {
    Usage on;
    Edge edge{};
    ClipAction action{};
    bool consume = false;

    static ClipTrigger from_c(const MediusClipTrigger& raw);
    MediusClipTrigger to_c() const;

    friend bool operator==(const ClipTrigger& a, const ClipTrigger& b) noexcept {
        return a.on == b.on && a.edge == b.edge && a.action == b.action && a.consume == b.consume;
    }
    friend bool operator!=(const ClipTrigger& a, const ClipTrigger& b) noexcept { return !(a == b); }
};

inline ClipTrigger ClipTrigger::from_c(const MediusClipTrigger& raw) {
    ClipTrigger out;
    out.on = Usage::from_c(raw.on);
    out.edge = detail::read<Edge>(raw.edge);
    out.action = detail::read<ClipAction>(raw.action);
    out.consume = raw.consume != 0;
    return out;
}

inline MediusClipTrigger ClipTrigger::to_c() const {
    MediusClipTrigger raw{};
    raw.on = on.to_c();
    raw.edge = detail::write(edge);
    raw.action = detail::write(action);
    raw.consume = static_cast<std::uint8_t>(consume);
    return raw;
}

/// one clip packet trigger, keyed by its class, id, direction, match and mask. a packet whose head matches under the
/// mask drives `action` on the box's next tick with no round trip to the PC, see `ClipPacketTrigger` in the medius docs
/// for which ones the box refuses and how they rank
struct ClipPacketTrigger {
    TrafficClass class_{};
    std::uint16_t id = 0;
    Direction direction{};
    ClipAction action{};
    std::vector<std::uint8_t> match_bytes;
    std::vector<std::uint8_t> mask;
    bool consume = false;
    bool once_per_run = false;
    std::uint8_t selector_len = 0;
    std::uint16_t hits = 0;

    static ClipPacketTrigger from_c(const MediusClipPacketTrigger& raw);
    MediusClipPacketTrigger to_c() const;

    friend bool operator==(const ClipPacketTrigger& a, const ClipPacketTrigger& b) noexcept {
        return a.class_ == b.class_ && a.id == b.id && a.direction == b.direction && a.action == b.action && a.match_bytes == b.match_bytes && a.mask == b.mask && a.consume == b.consume && a.once_per_run == b.once_per_run && a.selector_len == b.selector_len && a.hits == b.hits;
    }
    friend bool operator!=(const ClipPacketTrigger& a, const ClipPacketTrigger& b) noexcept { return !(a == b); }
};

inline ClipPacketTrigger ClipPacketTrigger::from_c(const MediusClipPacketTrigger& raw) {
    ClipPacketTrigger out;
    out.class_ = detail::read<TrafficClass>(raw.class_);
    out.id = raw.id;
    out.direction = detail::read<Direction>(raw.direction);
    out.action = detail::read<ClipAction>(raw.action);
    out.match_bytes = detail::bytes(raw.match_bytes, raw.match_len);
    out.mask = detail::bytes(raw.mask, raw.mask_len);
    out.consume = raw.consume != 0;
    out.once_per_run = raw.once_per_run != 0;
    out.selector_len = raw.selector_len;
    out.hits = raw.hits;
    return out;
}

inline MediusClipPacketTrigger ClipPacketTrigger::to_c() const {
    MediusClipPacketTrigger raw{};
    raw.class_ = detail::write(class_);
    raw.id = id;
    raw.direction = detail::write(direction);
    raw.action = detail::write(action);
    raw.match_len = static_cast<std::uint16_t>(detail::match(match_bytes, raw.match_bytes, "match_bytes"));
    raw.mask_len = static_cast<std::uint16_t>(detail::match(mask, raw.mask, "mask"));
    raw.consume = static_cast<std::uint8_t>(consume);
    raw.once_per_run = static_cast<std::uint8_t>(once_per_run);
    raw.selector_len = selector_len;
    raw.hits = hits;
    return raw;
}

/// the clip's ring and playback counters on the box
struct ClipStatus {
    ClipState state{ClipState::Idle};
    std::uint32_t free = 0;
    std::uint32_t total = 0;
    std::uint32_t played = 0;
    std::uint32_t ticks = 0;
    std::uint16_t underruns = 0;
    std::uint16_t overruns = 0;
    std::uint16_t seq_gaps = 0;
    std::uint16_t xfers = 0;
    std::uint16_t xfer_errs = 0;
    std::uint16_t gated = 0;
    std::vector<Usage> held;

    /// whether the clip holds that usage down
    bool is_held(const Usage& usage) const;

    static ClipStatus from_c(const MediusClipStatus& raw);
    MediusClipStatus to_c() const;

    friend bool operator==(const ClipStatus& a, const ClipStatus& b) noexcept {
        return a.state == b.state && a.free == b.free && a.total == b.total && a.played == b.played && a.ticks == b.ticks && a.underruns == b.underruns && a.overruns == b.overruns && a.seq_gaps == b.seq_gaps && a.xfers == b.xfers && a.xfer_errs == b.xfer_errs && a.gated == b.gated && a.held == b.held;
    }
    friend bool operator!=(const ClipStatus& a, const ClipStatus& b) noexcept { return !(a == b); }
};

inline ClipStatus ClipStatus::from_c(const MediusClipStatus& raw) {
    ClipStatus out;
    out.state = detail::read<ClipState>(raw.state);
    out.free = raw.free;
    out.total = raw.total;
    out.played = raw.played;
    out.ticks = raw.ticks;
    out.underruns = raw.underruns;
    out.overruns = raw.overruns;
    out.seq_gaps = raw.seq_gaps;
    out.xfers = raw.xfers;
    out.xfer_errs = raw.xfer_errs;
    out.gated = raw.gated;
    out.held = detail::list<Usage>(raw.held, raw.held_n);
    return out;
}

inline MediusClipStatus ClipStatus::to_c() const {
    MediusClipStatus raw{};
    raw.state = detail::write(state);
    raw.free = free;
    raw.total = total;
    raw.played = played;
    raw.ticks = ticks;
    raw.underruns = underruns;
    raw.overruns = overruns;
    raw.seq_gaps = seq_gaps;
    raw.xfers = xfers;
    raw.xfer_errs = xfer_errs;
    raw.gated = gated;
    raw.held_n = static_cast<std::uint16_t>(detail::list(held, raw.held, "held"));
    return raw;
}

/// a serial port that could be a box
struct PortInfo {
    std::string path;
    std::uint16_t vid = 0;
    std::uint16_t pid = 0;
    std::optional<std::string> serial;

    static PortInfo from_c(const MediusPortInfo& raw);
    MediusPortInfo to_c() const;

    friend bool operator==(const PortInfo& a, const PortInfo& b) noexcept {
        return a.path == b.path && a.vid == b.vid && a.pid == b.pid && a.serial == b.serial;
    }
    friend bool operator!=(const PortInfo& a, const PortInfo& b) noexcept { return !(a == b); }
};

inline PortInfo PortInfo::from_c(const MediusPortInfo& raw) {
    PortInfo out;
    out.path = detail::text(raw.path);
    out.vid = raw.vid;
    out.pid = raw.pid;
    if (raw.has_serial) {
        out.serial = detail::text(raw.serial);
    }
    return out;
}

inline MediusPortInfo PortInfo::to_c() const {
    MediusPortInfo raw{};
    detail::text(path, raw.path);
    raw.vid = vid;
    raw.pid = pid;
    raw.has_serial = static_cast<std::uint8_t>(serial.has_value());
    if (serial) {
        detail::text(*serial, raw.serial);
    }
    return raw;
}

/// what the box says it is
struct Version {
    std::uint8_t proto_ver = 0;
    std::uint8_t fw_major = 0;
    std::uint8_t fw_minor = 0;
    std::uint8_t fw_patch = 0;
    std::array<std::uint8_t, 6> mac{};
    std::string name;

    /// the base MAC as 12 lowercase hex digits, the box's id
    std::string mac_hex() const;

    static Version from_c(const MediusVersion& raw);
    MediusVersion to_c() const;

    friend bool operator==(const Version& a, const Version& b) noexcept {
        return a.proto_ver == b.proto_ver && a.fw_major == b.fw_major && a.fw_minor == b.fw_minor && a.fw_patch == b.fw_patch && a.mac == b.mac && a.name == b.name;
    }
    friend bool operator!=(const Version& a, const Version& b) noexcept { return !(a == b); }
};

inline Version Version::from_c(const MediusVersion& raw) {
    Version out;
    out.proto_ver = raw.proto_ver;
    out.fw_major = raw.fw_major;
    out.fw_minor = raw.fw_minor;
    out.fw_patch = raw.fw_patch;
    std::copy(std::begin(raw.mac), std::end(raw.mac), out.mac.begin());
    out.name = detail::text(raw.name);
    return out;
}

inline MediusVersion Version::to_c() const {
    MediusVersion raw{};
    raw.proto_ver = proto_ver;
    raw.fw_major = fw_major;
    raw.fw_minor = fw_minor;
    raw.fw_patch = fw_patch;
    std::copy(mac.begin(), mac.end(), raw.mac);
    detail::text(name, raw.name);
    return raw;
}

/// the cloned device's USB ids, what kind it mostly is and its product string
struct DeviceInfo {
    std::uint16_t vid = 0;
    std::uint16_t pid = 0;
    std::uint16_t bcd_device = 0;
    std::uint16_t bcd_usb = 0;
    bool has_serial = false;
    bool has_bos = false;
    DeviceKind kind{DeviceKind::Unknown};
    std::string product;

    /// whether a device is cloned, one with no HID interface too
    bool is_cloned() const;

    static DeviceInfo from_c(const MediusDeviceInfo& raw);
    MediusDeviceInfo to_c() const;

    friend bool operator==(const DeviceInfo& a, const DeviceInfo& b) noexcept {
        return a.vid == b.vid && a.pid == b.pid && a.bcd_device == b.bcd_device && a.bcd_usb == b.bcd_usb && a.has_serial == b.has_serial && a.has_bos == b.has_bos && a.kind == b.kind && a.product == b.product;
    }
    friend bool operator!=(const DeviceInfo& a, const DeviceInfo& b) noexcept { return !(a == b); }
};

inline DeviceInfo DeviceInfo::from_c(const MediusDeviceInfo& raw) {
    DeviceInfo out;
    out.vid = raw.vid;
    out.pid = raw.pid;
    out.bcd_device = raw.bcd_device;
    out.bcd_usb = raw.bcd_usb;
    out.has_serial = raw.has_serial != 0;
    out.has_bos = raw.has_bos != 0;
    out.kind = detail::read<DeviceKind>(raw.kind);
    out.product = detail::text(raw.product);
    return out;
}

inline MediusDeviceInfo DeviceInfo::to_c() const {
    MediusDeviceInfo raw{};
    raw.vid = vid;
    raw.pid = pid;
    raw.bcd_device = bcd_device;
    raw.bcd_usb = bcd_usb;
    raw.has_serial = static_cast<std::uint8_t>(has_serial);
    raw.has_bos = static_cast<std::uint8_t>(has_bos);
    raw.kind = detail::write(kind);
    detail::text(product, raw.product);
    return raw;
}

/// a box a listing found. `device` is empty for a box on another control protocol, opening that one throws
/// `BadProtoVerError`
struct BoxInfo {
    PortInfo port;
    Version version;
    std::optional<DeviceInfo> device;

    /// the box's stable id, its device MAC as hex
    std::string id() const;
    /// the box's name, a made up default when nobody set one
    const std::string& name() const;
    /// the serial of the box's control adapter, when it has one
    const std::optional<std::string>& serial() const;

    static BoxInfo from_c(const MediusBoxInfo& raw);
    MediusBoxInfo to_c() const;

    friend bool operator==(const BoxInfo& a, const BoxInfo& b) noexcept {
        return a.port == b.port && a.version == b.version && a.device == b.device;
    }
    friend bool operator!=(const BoxInfo& a, const BoxInfo& b) noexcept { return !(a == b); }
};

inline BoxInfo BoxInfo::from_c(const MediusBoxInfo& raw) {
    BoxInfo out;
    out.port = PortInfo::from_c(raw.port);
    out.version = Version::from_c(raw.version);
    if (raw.has_device) {
        out.device = DeviceInfo::from_c(raw.device);
    }
    return out;
}

inline MediusBoxInfo BoxInfo::to_c() const {
    MediusBoxInfo raw{};
    raw.port = port.to_c();
    raw.version = version.to_c();
    raw.has_device = static_cast<std::uint8_t>(device.has_value());
    if (device) {
        raw.device = device->to_c();
    }
    return raw;
}

/// the real device's reply to a control transfer. a status that isn't ok is the protocol's answer and not a link error,
/// and it carries no data
struct TransferOutcome {
    TransferStatus status{};
    std::vector<std::uint8_t> data;

    bool is_ok() const;
    /// the data when the transfer went through, empty when it didn't
    std::optional<std::vector<std::uint8_t>> ok_data() const;

    static TransferOutcome from_c(const MediusTransferOutcome& raw);
    MediusTransferOutcome to_c() const;

    friend bool operator==(const TransferOutcome& a, const TransferOutcome& b) noexcept {
        return a.status == b.status && a.data == b.data;
    }
    friend bool operator!=(const TransferOutcome& a, const TransferOutcome& b) noexcept { return !(a == b); }
};

inline TransferOutcome TransferOutcome::from_c(const MediusTransferOutcome& raw) {
    TransferOutcome out;
    out.status = detail::read<TransferStatus>(raw.status);
    out.data = detail::bytes(raw.data, raw.len);
    return out;
}

inline MediusTransferOutcome TransferOutcome::to_c() const {
    MediusTransferOutcome raw{};
    raw.status = detail::write(status);
    raw.len = static_cast<std::uint16_t>(detail::bytes(data, raw.data, "data"));
    return raw;
}

/// a rewrite rule, keyed by its class, id, direction, match and mask. the match and mask compare the masked head and
/// are as long as each other, an empty match takes every packet on the address. `payload` is for the actions that carry
/// one, and `offset` is where a patch writes
struct RewriteRule {
    RewriteClass class_{RewriteClass::Emit};
    std::uint16_t id = 0;
    Direction direction{Direction::Both};
    RewriteAction action{RewriteAction::Pass};
    std::uint16_t offset = 0;
    std::vector<std::uint8_t> match_bytes;
    std::vector<std::uint8_t> mask;
    std::vector<std::uint8_t> payload;

    RewriteKey key() const;

    static RewriteRule from_c(const MediusRewriteRule& raw);
    MediusRewriteRule to_c() const;

    friend bool operator==(const RewriteRule& a, const RewriteRule& b) noexcept {
        return a.class_ == b.class_ && a.id == b.id && a.direction == b.direction && a.action == b.action && a.offset == b.offset && a.match_bytes == b.match_bytes && a.mask == b.mask && a.payload == b.payload;
    }
    friend bool operator!=(const RewriteRule& a, const RewriteRule& b) noexcept { return !(a == b); }
};

inline RewriteRule RewriteRule::from_c(const MediusRewriteRule& raw) {
    RewriteRule out;
    out.class_ = detail::read<RewriteClass>(raw.class_);
    out.id = raw.id;
    out.direction = detail::read<Direction>(raw.direction);
    out.action = detail::read<RewriteAction>(raw.action);
    out.offset = raw.offset;
    out.match_bytes = detail::bytes(raw.match_bytes, raw.match_len);
    out.mask = detail::bytes(raw.mask, raw.mask_len);
    out.payload = detail::bytes(raw.payload, raw.payload_len);
    return out;
}

inline MediusRewriteRule RewriteRule::to_c() const {
    MediusRewriteRule raw{};
    raw.class_ = detail::write(class_);
    raw.id = id;
    raw.direction = detail::write(direction);
    raw.action = detail::write(action);
    raw.offset = offset;
    raw.match_len = static_cast<std::uint16_t>(detail::match(match_bytes, raw.match_bytes, "match_bytes"));
    raw.mask_len = static_cast<std::uint16_t>(detail::match(mask, raw.mask, "mask"));
    raw.payload_len = static_cast<std::uint16_t>(detail::bytes(payload, raw.payload, "payload"));
    return raw;
}

/// one row of the rewrite table, a rule's address, action and live counters without its bytes
struct RewriteEntry {
    RewriteClass class_{};
    std::uint16_t id = 0;
    Direction direction{};
    RewriteAction action{};
    std::uint8_t match_len = 0;
    std::uint16_t offset = 0;
    std::uint16_t payload_len = 0;
    std::uint16_t hits = 0;

    static RewriteEntry from_c(const MediusRewriteEntry& raw);
    MediusRewriteEntry to_c() const;

    friend bool operator==(const RewriteEntry& a, const RewriteEntry& b) noexcept {
        return a.class_ == b.class_ && a.id == b.id && a.direction == b.direction && a.action == b.action && a.match_len == b.match_len && a.offset == b.offset && a.payload_len == b.payload_len && a.hits == b.hits;
    }
    friend bool operator!=(const RewriteEntry& a, const RewriteEntry& b) noexcept { return !(a == b); }
};

inline RewriteEntry RewriteEntry::from_c(const MediusRewriteEntry& raw) {
    RewriteEntry out;
    out.class_ = detail::read<RewriteClass>(raw.class_);
    out.id = raw.id;
    out.direction = detail::read<Direction>(raw.direction);
    out.action = detail::read<RewriteAction>(raw.action);
    out.match_len = raw.match_len;
    out.offset = raw.offset;
    out.payload_len = raw.payload_len;
    out.hits = raw.hits;
    return out;
}

inline MediusRewriteEntry RewriteEntry::to_c() const {
    MediusRewriteEntry raw{};
    raw.class_ = detail::write(class_);
    raw.id = id;
    raw.direction = detail::write(direction);
    raw.action = detail::write(action);
    raw.match_len = match_len;
    raw.offset = offset;
    raw.payload_len = payload_len;
    raw.hits = hits;
    return raw;
}

/// the rewrite table in the order it was installed. `generation` moves on every change, `table_full` says the box
/// refused the last new rule for room
struct RewriteTable {
    bool table_full = false;
    std::uint8_t generation = 0;
    std::vector<RewriteEntry> entries;

    static RewriteTable from_c(const MediusRewriteTable& raw);
    MediusRewriteTable to_c() const;

    friend bool operator==(const RewriteTable& a, const RewriteTable& b) noexcept {
        return a.table_full == b.table_full && a.generation == b.generation && a.entries == b.entries;
    }
    friend bool operator!=(const RewriteTable& a, const RewriteTable& b) noexcept { return !(a == b); }
};

inline RewriteTable RewriteTable::from_c(const MediusRewriteTable& raw) {
    RewriteTable out;
    out.table_full = raw.table_full != 0;
    out.generation = raw.generation;
    out.entries = detail::list<RewriteEntry>(raw.entries, raw.n);
    return out;
}

inline MediusRewriteTable RewriteTable::to_c() const {
    MediusRewriteTable raw{};
    raw.table_full = static_cast<std::uint8_t>(table_full);
    raw.generation = generation;
    raw.n = static_cast<std::uint16_t>(detail::list(entries, raw.entries, "entries"));
    return raw;
}

/// a descriptor patch, keyed by its section, configuration, index and offset. `bytes` overwrites the descriptor from
/// `offset`, empty removes the patch at that key. `cfg` is the configuration's index, 0 is the first one and not
/// bConfigurationValue
struct Patch {
    PatchSection section{PatchSection::Device};
    std::uint8_t cfg = 0;
    std::uint8_t index = 0;
    std::uint16_t offset = 0;
    std::vector<std::uint8_t> bytes;

    /// a patch in configuration `cfg`, 0 is the first one
    static Patch in_config(std::uint8_t cfg, std::uint16_t offset, std::vector<std::uint8_t> bytes);
    /// a report descriptor patch on `interface_` in configuration `cfg`, 0 is the first one
    static Patch in_interface(std::uint8_t cfg, std::uint8_t interface_, std::uint16_t offset, std::vector<std::uint8_t> bytes);
    /// a patch that replaces all of string `index`
    static Patch in_string(std::uint8_t index, std::vector<std::uint8_t> bytes);
    PatchKey key() const;

    static Patch from_c(const MediusPatch& raw);
    MediusPatch to_c() const;

    friend bool operator==(const Patch& a, const Patch& b) noexcept {
        return a.section == b.section && a.cfg == b.cfg && a.index == b.index && a.offset == b.offset && a.bytes == b.bytes;
    }
    friend bool operator!=(const Patch& a, const Patch& b) noexcept { return !(a == b); }
};

inline Patch Patch::from_c(const MediusPatch& raw) {
    Patch out;
    out.section = detail::read<PatchSection>(raw.section);
    out.cfg = raw.cfg;
    out.index = raw.index;
    out.offset = raw.offset;
    out.bytes = detail::bytes(raw.bytes, raw.len);
    return out;
}

inline MediusPatch Patch::to_c() const {
    MediusPatch raw{};
    raw.section = detail::write(section);
    raw.cfg = cfg;
    raw.index = index;
    raw.offset = offset;
    raw.len = static_cast<std::uint16_t>(detail::bytes(bytes, raw.bytes, "bytes"));
    return raw;
}

/// one stored patch's key and length, without its bytes
struct PatchEntry {
    PatchSection section{};
    std::uint8_t cfg = 0;
    std::uint8_t index = 0;
    std::uint16_t offset = 0;
    std::uint16_t len = 0;

    static PatchEntry from_c(const MediusPatchEntry& raw);
    MediusPatchEntry to_c() const;

    friend bool operator==(const PatchEntry& a, const PatchEntry& b) noexcept {
        return a.section == b.section && a.cfg == b.cfg && a.index == b.index && a.offset == b.offset && a.len == b.len;
    }
    friend bool operator!=(const PatchEntry& a, const PatchEntry& b) noexcept { return !(a == b); }
};

inline PatchEntry PatchEntry::from_c(const MediusPatchEntry& raw) {
    PatchEntry out;
    out.section = detail::read<PatchSection>(raw.section);
    out.cfg = raw.cfg;
    out.index = raw.index;
    out.offset = raw.offset;
    out.len = raw.len;
    return out;
}

inline MediusPatchEntry PatchEntry::to_c() const {
    MediusPatchEntry raw{};
    raw.section = detail::write(section);
    raw.cfg = cfg;
    raw.index = index;
    raw.offset = offset;
    raw.len = len;
    return raw;
}

/// the stored patch set and how it's applied. `applied` is the clone serving a patched set, `pending` the stored set
/// being different from the served one, `refused` the stored set failing a check the plain descriptors pass so the
/// device is served unpatched, and `table_full` the box refusing the last new patch for room
struct PatchSet {
    bool applied = false;
    bool pending = false;
    bool refused = false;
    bool table_full = false;
    std::vector<PatchEntry> entries;

    static PatchSet from_c(const MediusPatchSet& raw);
    MediusPatchSet to_c() const;

    friend bool operator==(const PatchSet& a, const PatchSet& b) noexcept {
        return a.applied == b.applied && a.pending == b.pending && a.refused == b.refused && a.table_full == b.table_full && a.entries == b.entries;
    }
    friend bool operator!=(const PatchSet& a, const PatchSet& b) noexcept { return !(a == b); }
};

inline PatchSet PatchSet::from_c(const MediusPatchSet& raw) {
    PatchSet out;
    out.applied = raw.applied != 0;
    out.pending = raw.pending != 0;
    out.refused = raw.refused != 0;
    out.table_full = raw.table_full != 0;
    out.entries = detail::list<PatchEntry>(raw.entries, raw.n);
    return out;
}

inline MediusPatchSet PatchSet::to_c() const {
    MediusPatchSet raw{};
    raw.applied = static_cast<std::uint8_t>(applied);
    raw.pending = static_cast<std::uint8_t>(pending);
    raw.refused = static_cast<std::uint8_t>(refused);
    raw.table_full = static_cast<std::uint8_t>(table_full);
    raw.n = static_cast<std::uint16_t>(detail::list(entries, raw.entries, "entries"));
    return raw;
}

/// one field transform, keyed by its source and destination. it swaps or remaps a field the clone already has, so every
/// report is one the real device could send and it needs no imperfect clone opt in. to weigh or reverse a field use
/// `Device::scale`, which runs first
struct Transform {
    TransformOp op{};
    LockTarget source;
    LockTarget dest;

    TransformKey key() const;
    /// exchanges two axes at once
    static Transform swap(Axis a, Axis b);
    /// moves the source's value into the destination and clears the source
    static Transform remap(const LockTarget& source, const LockTarget& dest);

    static Transform from_c(const MediusTransform& raw);
    MediusTransform to_c() const;

    friend bool operator==(const Transform& a, const Transform& b) noexcept {
        return a.op == b.op && a.source == b.source && a.dest == b.dest;
    }
    friend bool operator!=(const Transform& a, const Transform& b) noexcept { return !(a == b); }
};

inline Transform Transform::from_c(const MediusTransform& raw) {
    Transform out;
    out.op = detail::read<TransformOp>(raw.op);
    out.source = LockTarget::from_c(raw.source);
    out.dest = LockTarget::from_c(raw.dest);
    return out;
}

inline MediusTransform Transform::to_c() const {
    MediusTransform raw{};
    raw.op = detail::write(op);
    raw.source = source.to_c();
    raw.dest = dest.to_c();
    return raw;
}

/// the transform table in the order it was installed, each entry replays as a `Device::transform`
struct Transforms {
    bool table_full = false;
    std::vector<Transform> entries;

    static Transforms from_c(const MediusTransforms& raw);
    MediusTransforms to_c() const;

    friend bool operator==(const Transforms& a, const Transforms& b) noexcept {
        return a.table_full == b.table_full && a.entries == b.entries;
    }
    friend bool operator!=(const Transforms& a, const Transforms& b) noexcept { return !(a == b); }
};

inline Transforms Transforms::from_c(const MediusTransforms& raw) {
    Transforms out;
    out.table_full = raw.table_full != 0;
    out.entries = detail::list<Transform>(raw.entries, raw.n);
    return out;
}

inline MediusTransforms Transforms::to_c() const {
    MediusTransforms raw{};
    raw.table_full = static_cast<std::uint8_t>(table_full);
    raw.n = static_cast<std::uint16_t>(detail::list(entries, raw.entries, "entries"));
    return raw;
}

/// one chip's firmware version and the slot it booted
struct ChipFirmware {
    std::uint8_t major = 0;
    std::uint8_t minor = 0;
    std::uint8_t patch = 0;
    std::uint8_t slot = 0;
    ImageState state{};

    static ChipFirmware from_c(const MediusChipFirmware& raw);
    MediusChipFirmware to_c() const;

    friend bool operator==(const ChipFirmware& a, const ChipFirmware& b) noexcept {
        return a.major == b.major && a.minor == b.minor && a.patch == b.patch && a.slot == b.slot && a.state == b.state;
    }
    friend bool operator!=(const ChipFirmware& a, const ChipFirmware& b) noexcept { return !(a == b); }
};

inline ChipFirmware ChipFirmware::from_c(const MediusChipFirmware& raw) {
    ChipFirmware out;
    out.major = raw.major;
    out.minor = raw.minor;
    out.patch = raw.patch;
    out.slot = raw.slot;
    out.state = detail::read<ImageState>(raw.state);
    return out;
}

inline MediusChipFirmware ChipFirmware::to_c() const {
    MediusChipFirmware raw{};
    raw.major = major;
    raw.minor = minor;
    raw.patch = patch;
    raw.slot = slot;
    raw.state = detail::write(state);
    return raw;
}

/// both chips' firmware. `host` is empty while the host chip hasn't answered over the link between the chips
struct FirmwareInfo {
    ChipFirmware device;
    std::optional<ChipFirmware> host;
    std::uint32_t slot_size = 0;
    bool device_staged = false;
    bool host_staged = false;

    /// while either chip hasn't confirmed the image it booted, which is when an update gets refused
    bool any_pending() const;

    static FirmwareInfo from_c(const MediusFirmwareInfo& raw);
    MediusFirmwareInfo to_c() const;

    friend bool operator==(const FirmwareInfo& a, const FirmwareInfo& b) noexcept {
        return a.device == b.device && a.host == b.host && a.slot_size == b.slot_size && a.device_staged == b.device_staged && a.host_staged == b.host_staged;
    }
    friend bool operator!=(const FirmwareInfo& a, const FirmwareInfo& b) noexcept { return !(a == b); }
};

inline FirmwareInfo FirmwareInfo::from_c(const MediusFirmwareInfo& raw) {
    FirmwareInfo out;
    out.device = ChipFirmware::from_c(raw.device);
    if (raw.host_present) {
        out.host = ChipFirmware::from_c(raw.host);
    }
    out.slot_size = raw.slot_size;
    out.device_staged = raw.device_staged != 0;
    out.host_staged = raw.host_staged != 0;
    return out;
}

inline MediusFirmwareInfo FirmwareInfo::to_c() const {
    MediusFirmwareInfo raw{};
    raw.device = device.to_c();
    raw.host_present = static_cast<std::uint8_t>(host.has_value());
    if (host) {
        raw.host = host->to_c();
    }
    raw.slot_size = slot_size;
    raw.device_staged = static_cast<std::uint8_t>(device_staged);
    raw.host_staged = static_cast<std::uint8_t>(host_staged);
    return raw;
}

/// the box's health flags
struct Health {
    bool link_up = false;
    bool mouse_attached = false;
    bool clone_configured = false;
    bool injection_active = false;
    bool rate_confident = false;
    bool lock_on = false;
    bool catch_on = false;
    bool kbd_attached = false;
    bool rewrite_on = false;
    bool patch_on = false;
    bool transform_on = false;

    static Health from_c(const MediusHealth& raw);
    MediusHealth to_c() const;

    friend bool operator==(const Health& a, const Health& b) noexcept {
        return a.link_up == b.link_up && a.mouse_attached == b.mouse_attached && a.clone_configured == b.clone_configured && a.injection_active == b.injection_active && a.rate_confident == b.rate_confident && a.lock_on == b.lock_on && a.catch_on == b.catch_on && a.kbd_attached == b.kbd_attached && a.rewrite_on == b.rewrite_on && a.patch_on == b.patch_on && a.transform_on == b.transform_on;
    }
    friend bool operator!=(const Health& a, const Health& b) noexcept { return !(a == b); }
};

inline Health Health::from_c(const MediusHealth& raw) {
    Health out;
    out.link_up = raw.link_up != 0;
    out.mouse_attached = raw.mouse_attached != 0;
    out.clone_configured = raw.clone_configured != 0;
    out.injection_active = raw.injection_active != 0;
    out.rate_confident = raw.rate_confident != 0;
    out.lock_on = raw.lock_on != 0;
    out.catch_on = raw.catch_on != 0;
    out.kbd_attached = raw.kbd_attached != 0;
    out.rewrite_on = raw.rewrite_on != 0;
    out.patch_on = raw.patch_on != 0;
    out.transform_on = raw.transform_on != 0;
    return out;
}

inline MediusHealth Health::to_c() const {
    MediusHealth raw{};
    raw.link_up = static_cast<std::uint8_t>(link_up);
    raw.mouse_attached = static_cast<std::uint8_t>(mouse_attached);
    raw.clone_configured = static_cast<std::uint8_t>(clone_configured);
    raw.injection_active = static_cast<std::uint8_t>(injection_active);
    raw.rate_confident = static_cast<std::uint8_t>(rate_confident);
    raw.lock_on = static_cast<std::uint8_t>(lock_on);
    raw.catch_on = static_cast<std::uint8_t>(catch_on);
    raw.kbd_attached = static_cast<std::uint8_t>(kbd_attached);
    raw.rewrite_on = static_cast<std::uint8_t>(rewrite_on);
    raw.patch_on = static_cast<std::uint8_t>(patch_on);
    raw.transform_on = static_cast<std::uint8_t>(transform_on);
    return raw;
}

/// what the cloned mouse has
struct MouseCaps {
    std::uint8_t n_buttons = 0;
    bool has_x = false;
    bool has_y = false;
    bool has_wheel = false;
    bool pan = false;
    bool has_report_id = false;
    std::uint8_t n_hid = 0;

    /// whether the mouse has more than one HID interface
    bool is_composite() const;

    static MouseCaps from_c(const MediusMouseCaps& raw);
    MediusMouseCaps to_c() const;

    friend bool operator==(const MouseCaps& a, const MouseCaps& b) noexcept {
        return a.n_buttons == b.n_buttons && a.has_x == b.has_x && a.has_y == b.has_y && a.has_wheel == b.has_wheel && a.pan == b.pan && a.has_report_id == b.has_report_id && a.n_hid == b.n_hid;
    }
    friend bool operator!=(const MouseCaps& a, const MouseCaps& b) noexcept { return !(a == b); }
};

inline MouseCaps MouseCaps::from_c(const MediusMouseCaps& raw) {
    MouseCaps out;
    out.n_buttons = raw.n_buttons;
    out.has_x = raw.has_x != 0;
    out.has_y = raw.has_y != 0;
    out.has_wheel = raw.has_wheel != 0;
    out.pan = raw.pan != 0;
    out.has_report_id = raw.has_report_id != 0;
    out.n_hid = raw.n_hid;
    return out;
}

inline MediusMouseCaps MouseCaps::to_c() const {
    MediusMouseCaps raw{};
    raw.n_buttons = n_buttons;
    raw.has_x = static_cast<std::uint8_t>(has_x);
    raw.has_y = static_cast<std::uint8_t>(has_y);
    raw.has_wheel = static_cast<std::uint8_t>(has_wheel);
    raw.pan = static_cast<std::uint8_t>(pan);
    raw.has_report_id = static_cast<std::uint8_t>(has_report_id);
    raw.n_hid = n_hid;
    return raw;
}

/// what the cloned keyboard has, `n_keys` 0xFF is an NKRO bitmap
struct KbdCaps {
    std::uint8_t n_keys = 0;
    bool nkro = false;
    bool has_consumer = false;
    bool has_system = false;
    bool has_report_id = false;

    static KbdCaps from_c(const MediusKbdCaps& raw);
    MediusKbdCaps to_c() const;

    friend bool operator==(const KbdCaps& a, const KbdCaps& b) noexcept {
        return a.n_keys == b.n_keys && a.nkro == b.nkro && a.has_consumer == b.has_consumer && a.has_system == b.has_system && a.has_report_id == b.has_report_id;
    }
    friend bool operator!=(const KbdCaps& a, const KbdCaps& b) noexcept { return !(a == b); }
};

inline KbdCaps KbdCaps::from_c(const MediusKbdCaps& raw) {
    KbdCaps out;
    out.n_keys = raw.n_keys;
    out.nkro = raw.nkro != 0;
    out.has_consumer = raw.has_consumer != 0;
    out.has_system = raw.has_system != 0;
    out.has_report_id = raw.has_report_id != 0;
    return out;
}

inline MediusKbdCaps KbdCaps::to_c() const {
    MediusKbdCaps raw{};
    raw.n_keys = n_keys;
    raw.nkro = static_cast<std::uint8_t>(nkro);
    raw.has_consumer = static_cast<std::uint8_t>(has_consumer);
    raw.has_system = static_cast<std::uint8_t>(has_system);
    raw.has_report_id = static_cast<std::uint8_t>(has_report_id);
    return raw;
}

/// what the cloned device has
struct Caps {
    MouseCaps mouse;
    KbdCaps keyboard;
    bool mouse_change_driven = false;
    bool kbd_change_driven = false;

    /// whether a mouse interface is bound
    bool has_mouse() const;
    /// whether a keyboard interface is bound
    bool has_keyboard() const;
    /// whether the clone has more than one HID interface
    bool is_composite() const;

    static Caps from_c(const MediusCaps& raw);
    MediusCaps to_c() const;

    friend bool operator==(const Caps& a, const Caps& b) noexcept {
        return a.mouse == b.mouse && a.keyboard == b.keyboard && a.mouse_change_driven == b.mouse_change_driven && a.kbd_change_driven == b.kbd_change_driven;
    }
    friend bool operator!=(const Caps& a, const Caps& b) noexcept { return !(a == b); }
};

inline Caps Caps::from_c(const MediusCaps& raw) {
    Caps out;
    out.mouse = MouseCaps::from_c(raw.mouse);
    out.keyboard = KbdCaps::from_c(raw.keyboard);
    out.mouse_change_driven = raw.mouse_change_driven != 0;
    out.kbd_change_driven = raw.kbd_change_driven != 0;
    return out;
}

inline MediusCaps Caps::to_c() const {
    MediusCaps raw{};
    raw.mouse = mouse.to_c();
    raw.keyboard = keyboard.to_c();
    raw.mouse_change_driven = static_cast<std::uint8_t>(mouse_change_driven);
    raw.kbd_change_driven = static_cast<std::uint8_t>(kbd_change_driven);
    return raw;
}

/// the device's report rate the box measured, and the clone's poll period
struct Rate {
    std::uint16_t native_period_us = 0;
    std::uint16_t poll_period_us = 0;
    bool confident = false;
    bool change_driven = false;

    /// the device's own rate, empty while the box doesn't know it
    std::optional<float> native_hz() const;

    static Rate from_c(const MediusRate& raw);
    MediusRate to_c() const;

    friend bool operator==(const Rate& a, const Rate& b) noexcept {
        return a.native_period_us == b.native_period_us && a.poll_period_us == b.poll_period_us && a.confident == b.confident && a.change_driven == b.change_driven;
    }
    friend bool operator!=(const Rate& a, const Rate& b) noexcept { return !(a == b); }
};

inline Rate Rate::from_c(const MediusRate& raw) {
    Rate out;
    out.native_period_us = raw.native_period_us;
    out.poll_period_us = raw.poll_period_us;
    out.confident = raw.confident != 0;
    out.change_driven = raw.change_driven != 0;
    return out;
}

inline MediusRate Rate::to_c() const {
    MediusRate raw{};
    raw.native_period_us = native_period_us;
    raw.poll_period_us = poll_period_us;
    raw.confident = static_cast<std::uint8_t>(confident);
    raw.change_driven = static_cast<std::uint8_t>(change_driven);
    return raw;
}

/// the box's delivery counters. the narrow ones saturate instead of wrapping, the three drop counters don't.
/// `tx_drops`, `link_rx_drops` and `host_rx_drops` are lost physical input and should stay 0, `relay_drops` is relayed
/// traffic and commands, never physical input. `session` counts the times the box let go of what the PC set and wraps,
/// so compare it for inequality
struct Stats {
    std::uint32_t inject_emits = 0;
    std::uint16_t tx_drops = 0;
    std::uint16_t tx_merges = 0;
    std::uint8_t tx_maxdepth = 0;
    std::uint8_t tx_wedges = 0;
    std::uint16_t wakeups = 0;
    std::uint16_t reset_count = 0;
    std::uint16_t config_count = 0;
    std::uint32_t link_rx_drops = 0;
    std::uint32_t host_rx_drops = 0;
    std::uint32_t relay_drops = 0;
    std::uint16_t session = 0;

    static Stats from_c(const MediusStats& raw);
    MediusStats to_c() const;

    friend bool operator==(const Stats& a, const Stats& b) noexcept {
        return a.inject_emits == b.inject_emits && a.tx_drops == b.tx_drops && a.tx_merges == b.tx_merges && a.tx_maxdepth == b.tx_maxdepth && a.tx_wedges == b.tx_wedges && a.wakeups == b.wakeups && a.reset_count == b.reset_count && a.config_count == b.config_count && a.link_rx_drops == b.link_rx_drops && a.host_rx_drops == b.host_rx_drops && a.relay_drops == b.relay_drops && a.session == b.session;
    }
    friend bool operator!=(const Stats& a, const Stats& b) noexcept { return !(a == b); }
};

inline Stats Stats::from_c(const MediusStats& raw) {
    Stats out;
    out.inject_emits = raw.inject_emits;
    out.tx_drops = raw.tx_drops;
    out.tx_merges = raw.tx_merges;
    out.tx_maxdepth = raw.tx_maxdepth;
    out.tx_wedges = raw.tx_wedges;
    out.wakeups = raw.wakeups;
    out.reset_count = raw.reset_count;
    out.config_count = raw.config_count;
    out.link_rx_drops = raw.link_rx_drops;
    out.host_rx_drops = raw.host_rx_drops;
    out.relay_drops = raw.relay_drops;
    out.session = raw.session;
    return out;
}

inline MediusStats Stats::to_c() const {
    MediusStats raw{};
    raw.inject_emits = inject_emits;
    raw.tx_drops = tx_drops;
    raw.tx_merges = tx_merges;
    raw.tx_maxdepth = tx_maxdepth;
    raw.tx_wedges = tx_wedges;
    raw.wakeups = wakeups;
    raw.reset_count = reset_count;
    raw.config_count = config_count;
    raw.link_rx_drops = link_rx_drops;
    raw.host_rx_drops = host_rx_drops;
    raw.relay_drops = relay_drops;
    raw.session = session;
    return raw;
}

/// one weighed direction. `scale` is the percent of the physical value kept, 0 blocks, 100 passes, above 100 amplifies
/// and a negative one reverses what it keeps. it's the scale the box applies, which in `BearingMode::Vector` can differ
/// from the one sent
struct LockEntry {
    LockTarget target;
    bool is_blanket = false;
    Direction direction{};
    std::int16_t scale = 0;

    bool is_block() const;

    static LockEntry from_c(const MediusLockEntry& raw);
    MediusLockEntry to_c() const;

    friend bool operator==(const LockEntry& a, const LockEntry& b) noexcept {
        return a.target == b.target && a.is_blanket == b.is_blanket && a.direction == b.direction && a.scale == b.scale;
    }
    friend bool operator!=(const LockEntry& a, const LockEntry& b) noexcept { return !(a == b); }
};

inline LockEntry LockEntry::from_c(const MediusLockEntry& raw) {
    LockEntry out;
    out.target = LockTarget::from_c(raw.target);
    out.is_blanket = raw.is_blanket;
    out.direction = detail::read<Direction>(raw.direction);
    out.scale = raw.scale;
    return out;
}

inline MediusLockEntry LockEntry::to_c() const {
    MediusLockEntry raw{};
    raw.target = target.to_c();
    raw.is_blanket = is_blanket;
    raw.direction = detail::write(direction);
    raw.scale = scale;
    return raw;
}

/// the locks and scales the box holds
struct Locks {
    std::vector<LockEntry> entries;

    /// whether the target is blocked outright that way, a weighed direction isn't locked. `Both` asks about the two
    /// fixed signs, ask for a relative one by name
    bool is_locked(const LockTarget& target, Direction direction) const;
    /// the scale on that target and direction, of the entries covering it the one of smallest magnitude (a negative one
    /// on a tie), or 100 when none does. a delta meets the product of its fixed and relative scales, which this doesn't
    /// work out
    std::int16_t scale_of(const LockTarget& target, Direction direction) const;

    static Locks from_c(const MediusLocks& raw);
    MediusLocks to_c() const;

    friend bool operator==(const Locks& a, const Locks& b) noexcept {
        return a.entries == b.entries;
    }
    friend bool operator!=(const Locks& a, const Locks& b) noexcept { return !(a == b); }
};

inline Locks Locks::from_c(const MediusLocks& raw) {
    Locks out;
    out.entries = detail::list<LockEntry>(raw.entries, raw.n);
    return out;
}

inline MediusLocks Locks::to_c() const {
    MediusLocks raw{};
    raw.n = static_cast<std::uint16_t>(detail::list(entries, raw.entries, "entries"));
    return raw;
}

/// the measured offset between the two chips' clocks. `rate_ppb` is empty when the box fitted no rate, `age_ms` empty
/// when it never measured, which a zero offset looks like too
struct ClockEstimate {
    std::int32_t offset_us = 0;
    std::optional<std::int32_t> rate_ppb;
    std::uint16_t delay_us = 0;
    std::optional<std::uint32_t> age_ms;

    /// half the measured round trip, how wrong `offset_us` can be at most
    std::int32_t error_bound_us() const;
    /// a device chip stamp on the host chip's timeline, empty without an estimate
    std::optional<std::int64_t> to_host_domain(std::int64_t device_us) const;
    /// how far the offset drifts over `elapsed`, in microseconds. add `error_bound_us()` for the whole bound on a stamp
    /// moved that long after the estimate. it's 0 when the box fitted no rate, though the drift is unknown then and not
    /// zero
    std::int64_t drift_us_over(std::chrono::microseconds elapsed) const;

    static ClockEstimate from_c(const MediusClockEstimate& raw);
    MediusClockEstimate to_c() const;

    friend bool operator==(const ClockEstimate& a, const ClockEstimate& b) noexcept {
        return a.offset_us == b.offset_us && a.rate_ppb == b.rate_ppb && a.delay_us == b.delay_us && a.age_ms == b.age_ms;
    }
    friend bool operator!=(const ClockEstimate& a, const ClockEstimate& b) noexcept { return !(a == b); }
};

inline ClockEstimate ClockEstimate::from_c(const MediusClockEstimate& raw) {
    ClockEstimate out;
    out.offset_us = raw.offset_us;
    out.rate_ppb = raw.rate_ppb != MEDIUS_CLOCK_RATE_NONE ? std::optional<std::int32_t>(raw.rate_ppb) : std::nullopt;
    out.delay_us = raw.delay_us;
    out.age_ms = raw.age_ms != MEDIUS_CLOCK_AGE_NONE ? std::optional<std::uint32_t>(raw.age_ms) : std::nullopt;
    return out;
}

inline MediusClockEstimate ClockEstimate::to_c() const {
    MediusClockEstimate raw{};
    raw.offset_us = offset_us;
    raw.rate_ppb = rate_ppb ? *rate_ppb : static_cast<std::int32_t>(MEDIUS_CLOCK_RATE_NONE);
    raw.delay_us = delay_us;
    raw.age_ms = age_ms ? *age_ms : static_cast<std::uint32_t>(MEDIUS_CLOCK_AGE_NONE);
    return raw;
}

/// one row of the box's subscription table, a live subscription and its drop count
struct CatchEntry {
    CatchFilter filter;
    std::uint16_t dropped = 0;

    static CatchEntry from_c(const MediusCatchEntry& raw);
    MediusCatchEntry to_c() const;

    friend bool operator==(const CatchEntry& a, const CatchEntry& b) noexcept {
        return a.filter == b.filter && a.dropped == b.dropped;
    }
    friend bool operator!=(const CatchEntry& a, const CatchEntry& b) noexcept { return !(a == b); }
};

inline CatchEntry CatchEntry::from_c(const MediusCatchEntry& raw) {
    CatchEntry out;
    out.filter = CatchFilter::from_c(raw.filter);
    out.dropped = raw.dropped;
    return out;
}

inline MediusCatchEntry CatchEntry::to_c() const {
    MediusCatchEntry raw{};
    raw.filter = filter.to_c();
    raw.dropped = dropped;
    return raw;
}

/// the live subscription table, its drop counts and the clock estimate between the chips
struct CatchState {
    bool table_full = false;
    std::uint32_t dropped = 0;
    ClockEstimate clock;
    std::vector<CatchEntry> entries;

    /// whether nothing is subscribed
    bool is_empty() const;

    static CatchState from_c(const MediusCatchState& raw);
    MediusCatchState to_c() const;

    friend bool operator==(const CatchState& a, const CatchState& b) noexcept {
        return a.table_full == b.table_full && a.dropped == b.dropped && a.clock == b.clock && a.entries == b.entries;
    }
    friend bool operator!=(const CatchState& a, const CatchState& b) noexcept { return !(a == b); }
};

inline CatchState CatchState::from_c(const MediusCatchState& raw) {
    CatchState out;
    out.table_full = raw.table_full != 0;
    out.dropped = raw.dropped;
    out.clock = ClockEstimate::from_c(raw.clock);
    out.entries = detail::list<CatchEntry>(raw.entries, raw.n);
    return out;
}

inline MediusCatchState CatchState::to_c() const {
    MediusCatchState raw{};
    raw.table_full = static_cast<std::uint8_t>(table_full);
    raw.dropped = dropped;
    raw.clock = clock.to_c();
    raw.n = static_cast<std::uint16_t>(detail::list(entries, raw.entries, "entries"));
    return raw;
}

/// the imperfect clone opt in, and whether the clone needs it
struct ImperfectStatus {
    bool allowed = false;
    bool over_capacity = false;
    bool clone_imperfect = false;

    static ImperfectStatus from_c(const MediusImperfectStatus& raw);
    MediusImperfectStatus to_c() const;

    friend bool operator==(const ImperfectStatus& a, const ImperfectStatus& b) noexcept {
        return a.allowed == b.allowed && a.over_capacity == b.over_capacity && a.clone_imperfect == b.clone_imperfect;
    }
    friend bool operator!=(const ImperfectStatus& a, const ImperfectStatus& b) noexcept { return !(a == b); }
};

inline ImperfectStatus ImperfectStatus::from_c(const MediusImperfectStatus& raw) {
    ImperfectStatus out;
    out.allowed = raw.allowed != 0;
    out.over_capacity = raw.over_capacity != 0;
    out.clone_imperfect = raw.clone_imperfect != 0;
    return out;
}

inline MediusImperfectStatus ImperfectStatus::to_c() const {
    MediusImperfectStatus raw{};
    raw.allowed = static_cast<std::uint8_t>(allowed);
    raw.over_capacity = static_cast<std::uint8_t>(over_capacity);
    raw.clone_imperfect = static_cast<std::uint8_t>(clone_imperfect);
    return raw;
}

/// what `Direction::With` and `Direction::Against` are measured against. `window_ms` is how long the last injected
/// direction stays the bearing, empty is off and leaves the relative directions doing nothing
struct Bearing {
    std::optional<std::uint16_t> window_ms = static_cast<std::uint16_t>(BearingWindowDefaultMs);
    BearingMode mode{BearingMode::PerAxis};

    bool is_live() const;

    static Bearing from_c(const MediusBearing& raw);
    MediusBearing to_c() const;

    friend bool operator==(const Bearing& a, const Bearing& b) noexcept {
        return a.window_ms == b.window_ms && a.mode == b.mode;
    }
    friend bool operator!=(const Bearing& a, const Bearing& b) noexcept { return !(a == b); }
};

inline Bearing Bearing::from_c(const MediusBearing& raw) {
    Bearing out;
    out.window_ms = raw.window_ms != 0 ? std::optional<std::uint16_t>(raw.window_ms) : std::nullopt;
    out.mode = detail::read<BearingMode>(raw.mode);
    return out;
}

inline MediusBearing Bearing::to_c() const {
    MediusBearing raw{};
    raw.window_ms = window_ms ? *window_ms : static_cast<std::uint16_t>(0);
    raw.mode = static_cast<MediusBearingMode>(detail::write(mode));
    return raw;
}

/// the render mode, whether physical motion goes through it, and whether the box learned the attached device's profile
/// yet
struct RenderStatus {
    RenderMode mode{RenderMode::Despiked};
    bool full = false;
    bool ready = false;

    static RenderStatus from_c(const MediusRenderStatus& raw);
    MediusRenderStatus to_c() const;

    friend bool operator==(const RenderStatus& a, const RenderStatus& b) noexcept {
        return a.mode == b.mode && a.full == b.full && a.ready == b.ready;
    }
    friend bool operator!=(const RenderStatus& a, const RenderStatus& b) noexcept { return !(a == b); }
};

inline RenderStatus RenderStatus::from_c(const MediusRenderStatus& raw) {
    RenderStatus out;
    out.mode = detail::read<RenderMode>(raw.mode);
    out.full = raw.full != 0;
    out.ready = raw.ready != 0;
    return out;
}

inline MediusRenderStatus RenderStatus::to_c() const {
    MediusRenderStatus raw{};
    raw.mode = static_cast<MediusRenderMode>(detail::write(mode));
    raw.full = static_cast<std::uint8_t>(full);
    raw.ready = static_cast<std::uint8_t>(ready);
    return raw;
}

/// the injection spread, a percent of the host's command interval, and the span the box releases across
struct SpreadStatus {
    std::uint16_t percent = 0;
    std::uint32_t span_us = 0;

    static SpreadStatus from_c(const MediusSpreadStatus& raw);
    MediusSpreadStatus to_c() const;

    friend bool operator==(const SpreadStatus& a, const SpreadStatus& b) noexcept {
        return a.percent == b.percent && a.span_us == b.span_us;
    }
    friend bool operator!=(const SpreadStatus& a, const SpreadStatus& b) noexcept { return !(a == b); }
};

inline SpreadStatus SpreadStatus::from_c(const MediusSpreadStatus& raw) {
    SpreadStatus out;
    out.percent = raw.percent;
    out.span_us = raw.span_us;
    return out;
}

inline MediusSpreadStatus SpreadStatus::to_c() const {
    MediusSpreadStatus raw{};
    raw.percent = percent;
    raw.span_us = span_us;
    return raw;
}

/// what the library counted on this side. `restarts` is the device chip restarts it got over by sending its held state
/// again
struct CountersSnapshot {
    std::uint64_t frames_tx = 0;
    std::uint64_t frames_rx = 0;
    std::uint64_t crc_drops = 0;
    std::uint64_t reconnects = 0;
    std::uint64_t restarts = 0;

    static CountersSnapshot from_c(const MediusCountersSnapshot& raw);
    MediusCountersSnapshot to_c() const;

    friend bool operator==(const CountersSnapshot& a, const CountersSnapshot& b) noexcept {
        return a.frames_tx == b.frames_tx && a.frames_rx == b.frames_rx && a.crc_drops == b.crc_drops && a.reconnects == b.reconnects && a.restarts == b.restarts;
    }
    friend bool operator!=(const CountersSnapshot& a, const CountersSnapshot& b) noexcept { return !(a == b); }
};

inline CountersSnapshot CountersSnapshot::from_c(const MediusCountersSnapshot& raw) {
    CountersSnapshot out;
    out.frames_tx = raw.frames_tx;
    out.frames_rx = raw.frames_rx;
    out.crc_drops = raw.crc_drops;
    out.reconnects = raw.reconnects;
    out.restarts = raw.restarts;
    return out;
}

inline MediusCountersSnapshot CountersSnapshot::to_c() const {
    MediusCountersSnapshot raw{};
    raw.frames_tx = frames_tx;
    raw.frames_rx = frames_rx;
    raw.crc_drops = crc_drops;
    raw.reconnects = reconnects;
    raw.restarts = restarts;
    return raw;
}

/// every usage of one class held right now. the class and direction come from the frame, so the empty snapshot of the
/// last release still has them
struct UsageSnapshot {
    Class class_{};
    Direction direction{};
    std::vector<Usage> usages;

    bool is_held(const Usage& usage) const;

    static UsageSnapshot from_c(const MediusUsageEvent& raw);
    MediusUsageEvent to_c() const;

    friend bool operator==(const UsageSnapshot& a, const UsageSnapshot& b) noexcept {
        return a.class_ == b.class_ && a.direction == b.direction && a.usages == b.usages;
    }
    friend bool operator!=(const UsageSnapshot& a, const UsageSnapshot& b) noexcept { return !(a == b); }
};

inline UsageSnapshot UsageSnapshot::from_c(const MediusUsageEvent& raw) {
    UsageSnapshot out;
    out.class_ = detail::read<Class>(raw.class_);
    out.direction = detail::read<Direction>(raw.direction);
    out.usages = detail::list<Usage>(raw.usages, raw.n);
    return out;
}

inline MediusUsageEvent UsageSnapshot::to_c() const {
    MediusUsageEvent raw{};
    raw.class_ = detail::write(class_);
    raw.direction = detail::write(direction);
    raw.n = static_cast<std::uint16_t>(detail::list(usages, raw.usages, "usages"));
    return raw;
}

/// one byte oriented catch event, HID reports, vendor endpoints, control transactions, what the clone sent, bus
/// lifecycle, or a clip's control transfers. `bytes` is what the capture kept and `true_len` the length before it was
/// cut, so set both when you build one by hand
struct TrafficEvent {
    CatchClass class_{};
    std::uint16_t id = 0;
    Direction direction{};
    std::uint8_t flags = 0;
    std::uint16_t true_len = 0;
    std::vector<std::uint8_t> bytes;

    /// whether the capture cut the packet short, which `bytes` alone can't tell from a short packet
    bool truncated() const;
    /// the 8 setup bytes of a control or clip transfer event, empty for another class or a shorter capture
    std::optional<std::array<std::uint8_t, 8>> setup_bytes() const;
    /// the data stage of a control or clip transfer event, the whole packet for any other class
    std::vector<std::uint8_t> data() const;
    /// the handshake the game PC got, empty for any class but control
    std::optional<ControlStatus> control_status() const;
    /// how the transfer ended, empty for any class but clip transfer. `Nak` when no reply came, and a status this
    /// header has no name for stays its number
    std::optional<TransferStatus> transfer_status() const;
    /// the lifecycle event, empty for any class but bus or a kind the library doesn't know
    std::optional<BusEvent> bus_event() const;
    /// whether a rewrite rule changed, dropped, answered or refused the packet. only HID in and out, the vendor
    /// classes, control and emit carry it
    bool rule_acted() const;
    bool bulk_end_of_transfer() const;
    /// whether it's a zero length packet, or on a HID endpoint a HID in or emit report one ended
    bool zlp() const;

    static TrafficEvent from_c(const MediusTrafficEvent& raw);
    MediusTrafficEvent to_c() const;

    friend bool operator==(const TrafficEvent& a, const TrafficEvent& b) noexcept {
        return a.class_ == b.class_ && a.id == b.id && a.direction == b.direction && a.flags == b.flags && a.true_len == b.true_len && a.bytes == b.bytes;
    }
    friend bool operator!=(const TrafficEvent& a, const TrafficEvent& b) noexcept { return !(a == b); }
};

inline TrafficEvent TrafficEvent::from_c(const MediusTrafficEvent& raw) {
    TrafficEvent out;
    out.class_ = detail::read<CatchClass>(raw.class_);
    out.id = raw.id;
    out.direction = detail::read<Direction>(raw.direction);
    out.flags = raw.flags;
    out.true_len = raw.true_len;
    out.bytes = detail::bytes(raw.bytes, raw.len);
    return out;
}

inline MediusTrafficEvent TrafficEvent::to_c() const {
    MediusTrafficEvent raw{};
    raw.class_ = detail::write(class_);
    raw.id = id;
    raw.direction = detail::write(direction);
    raw.flags = flags;
    raw.true_len = true_len;
    raw.len = static_cast<std::uint16_t>(detail::bytes(bytes, raw.bytes, "bytes"));
    return raw;
}

/// a bus lifecycle event, the numbers are 0 for the kinds that have none. just know that windows.h has an interface
/// macro, so reading `interface` needs an `#undef interface` after it
struct BusEvent {
    BusEventKind kind{};
    std::uint8_t configuration = 0;
    std::uint8_t interface = 0;
    std::uint8_t alt = 0;

    static BusEvent from_c(const MediusBusEvent& raw);
    MediusBusEvent to_c() const;

    friend bool operator==(const BusEvent& a, const BusEvent& b) noexcept {
        return a.kind == b.kind && a.configuration == b.configuration && a.interface == b.interface && a.alt == b.alt;
    }
    friend bool operator!=(const BusEvent& a, const BusEvent& b) noexcept { return !(a == b); }
};

inline BusEvent BusEvent::from_c(const MediusBusEvent& raw) {
    BusEvent out;
    out.kind = detail::read<BusEventKind>(raw.kind);
    out.configuration = raw.configuration;
    out.interface = raw.interface;
    out.alt = raw.alt;
    return out;
}

inline MediusBusEvent BusEvent::to_c() const {
    MediusBusEvent raw{};
    raw.kind = static_cast<MediusBusEventKind>(detail::write(kind));
    raw.configuration = configuration;
    raw.interface = interface;
    raw.alt = alt;
    return raw;
}

/// physical motion at the merge point, pan is positive going right
struct MotionEvent {
    std::int16_t dx = 0;
    std::int16_t dy = 0;
    std::int16_t dz = 0;
    std::int16_t pan = 0;

    /// every axis and its delta, moved or not
    std::vector<std::pair<Axis, std::int16_t>> all_axes() const;
    /// the axes this report moved, with their deltas. the box never sends one that moved nothing, a mock can
    std::vector<std::pair<Axis, std::int16_t>> axes() const;

    static MotionEvent from_c(const MediusMotionEvent& raw);
    MediusMotionEvent to_c() const;

    friend bool operator==(const MotionEvent& a, const MotionEvent& b) noexcept {
        return a.dx == b.dx && a.dy == b.dy && a.dz == b.dz && a.pan == b.pan;
    }
    friend bool operator!=(const MotionEvent& a, const MotionEvent& b) noexcept { return !(a == b); }
};

inline MotionEvent MotionEvent::from_c(const MediusMotionEvent& raw) {
    MotionEvent out;
    out.dx = raw.dx;
    out.dy = raw.dy;
    out.dz = raw.dz;
    out.pan = raw.pan;
    return out;
}

inline MediusMotionEvent MotionEvent::to_c() const {
    MediusMotionEvent raw{};
    raw.dx = dx;
    raw.dy = dy;
    raw.dz = dz;
    raw.pan = pan;
    return raw;
}

/// an event a `Timeline` put on your clock, when it happened on the scale you passed in, its box stamp unwrapped past
/// the 32 bit rollover, and how much later than the measured floor it came, which is jitter and not latency
struct Stamped {
    std::uint64_t host_ns = 0;
    std::uint64_t box_us = 0;
    std::uint64_t excess_ns = 0;

    static Stamped from_c(const MediusStamped& raw);
    MediusStamped to_c() const;

    friend bool operator==(const Stamped& a, const Stamped& b) noexcept {
        return a.host_ns == b.host_ns && a.box_us == b.box_us && a.excess_ns == b.excess_ns;
    }
    friend bool operator!=(const Stamped& a, const Stamped& b) noexcept { return !(a == b); }
};

inline Stamped Stamped::from_c(const MediusStamped& raw) {
    Stamped out;
    out.host_ns = raw.host_ns;
    out.box_us = raw.box_us;
    out.excess_ns = raw.excess_ns;
    return out;
}

inline MediusStamped Stamped::to_c() const {
    MediusStamped raw{};
    raw.host_ns = host_ns;
    raw.box_us = box_us;
    raw.excess_ns = excess_ns;
    return raw;
}

/// one line of the box's log
struct LogLine {
    LogLevel level{};
    std::string text;

    static LogLine from_c(const MediusLogLine& raw);
    MediusLogLine to_c() const;

    friend bool operator==(const LogLine& a, const LogLine& b) noexcept {
        return a.level == b.level && a.text == b.text;
    }
    friend bool operator!=(const LogLine& a, const LogLine& b) noexcept { return !(a == b); }
};

inline LogLine LogLine::from_c(const MediusLogLine& raw) {
    LogLine out;
    out.level = detail::read<LogLevel>(raw.level);
    out.text = detail::text(raw.text);
    return out;
}

inline MediusLogLine LogLine::to_c() const {
    MediusLogLine raw{};
    raw.level = static_cast<MediusLogLevel>(detail::write(level));
    detail::text(text, raw.text);
    return raw;
}
// the end of what gen-medius makes (structs)

/// the clip's configuration, its autolock, loop, retain, finalized and ride, and both kinds of trigger, the packet
/// ones in box order with their hits
struct ClipSettings {
    std::vector<Blanket> autolock;
    bool loop = false;
    bool retain = false;
    bool finalized = false;
    bool ride = false;
    std::vector<ClipTrigger> triggers;
    std::vector<ClipPacketTrigger> packet_triggers;

    static ClipSettings from_c(const MediusClipSettings& raw) {
        ClipSettings out;
        for (const auto& bit : bits()) {
            if ((raw.autolock_bits & bit.first) != 0) {
                out.autolock.push_back(bit.second);
            }
        }
        out.loop = raw.loop_ != 0;
        out.retain = raw.retain != 0;
        out.finalized = raw.finalized != 0;
        out.ride = raw.ride != 0;
        out.triggers = detail::list<ClipTrigger>(raw.triggers, raw.n);
        out.packet_triggers = detail::list<ClipPacketTrigger>(raw.packet_triggers, raw.packet_n);
        return out;
    }

    MediusClipSettings to_c() const {
        MediusClipSettings raw{};
        for (const auto blanket : autolock) {
            detail::write(blanket);
            for (const auto& bit : bits()) {
                if (bit.second == blanket) {
                    raw.autolock_bits = static_cast<std::uint8_t>(raw.autolock_bits | bit.first);
                }
            }
        }
        raw.loop_ = static_cast<std::uint8_t>(loop);
        raw.retain = static_cast<std::uint8_t>(retain);
        raw.finalized = static_cast<std::uint8_t>(finalized);
        raw.ride = static_cast<std::uint8_t>(ride);
        raw.n = static_cast<std::uint8_t>(detail::list(triggers, raw.triggers, "triggers"));
        raw.packet_n = static_cast<std::uint8_t>(detail::list(packet_triggers, raw.packet_triggers, "packet_triggers"));
        return raw;
    }

    friend bool operator==(const ClipSettings& a, const ClipSettings& b) noexcept {
        return a.autolock == b.autolock && a.loop == b.loop && a.retain == b.retain && a.finalized == b.finalized &&
               a.ride == b.ride && a.triggers == b.triggers && a.packet_triggers == b.packet_triggers;
    }
    friend bool operator!=(const ClipSettings& a, const ClipSettings& b) noexcept { return !(a == b); }

private:
    // the autolock bits are the box's own CLIP_LOCK ones from the crate's protocol/opcode.rs, not the blanket
    // numbers, since medius.h names them and never defines them
    static const std::array<std::pair<std::uint8_t, Blanket>, 5>& bits() noexcept {
        static const std::array<std::pair<std::uint8_t, Blanket>, 5> table{{
            {std::uint8_t{0x01}, Blanket::Aim},
            {std::uint8_t{0x02}, Blanket::Wheel},
            {std::uint8_t{0x04}, Blanket::Buttons},
            {std::uint8_t{0x08}, Blanket::Keys},
            {std::uint8_t{0x10}, Blanket::Media},
        }};
        return table;
    }
};

/// the configured pace, the emit rate ceiling and the wire rate in use. `force_hz` is empty when nothing forces the
/// poll rate
struct EmitPaceStatus {
    EmitPace mode;
    std::uint16_t resolved_hz = 0;
    std::optional<std::uint16_t> force_hz;
    std::uint16_t advertised_hz = 0;
    bool force_active = false;

    static EmitPaceStatus from_c(const MediusEmitPaceStatus& raw) {
        EmitPaceStatus out;
        switch (detail::read<EmitMode>(raw.mode)) {
        case EmitMode::Learned:
            out.mode = EmitPace::learned();
            break;
        case EmitMode::Interval:
            out.mode = EmitPace::interval();
            break;
        case EmitMode::Fixed:
            out.mode = EmitPace::fixed(raw.fixed_hz);
            break;
        }
        out.resolved_hz = raw.resolved_hz;
        if (raw.force_hz != 0) {
            out.force_hz = raw.force_hz;
        }
        out.advertised_hz = raw.advertised_hz;
        out.force_active = raw.force_active != 0;
        return out;
    }

    friend bool operator==(const EmitPaceStatus& a, const EmitPaceStatus& b) noexcept {
        return a.mode == b.mode && a.resolved_hz == b.resolved_hz && a.force_hz == b.force_hz &&
               a.advertised_hz == b.advertised_hz && a.force_active == b.force_active;
    }
    friend bool operator!=(const EmitPaceStatus& a, const EmitPaceStatus& b) noexcept { return !(a == b); }
};

/// one catch event, read `data` with std::visit or one of the three getters. `ts_us` is in the `clock` chip's
/// microseconds, a box local clock, so only compare stamps from the same chip, and `CatchState::clock` moves one
/// across. it wraps about every 71.6 minutes and starts at 0 again when that chip reboots
struct CatchEvent {
    CatchEventKind kind = CatchEventKind::Motion;
    std::uint32_t ts_us = 0;
    ClockDomain clock = ClockDomain::HostChip;
    std::variant<MotionEvent, UsageSnapshot, TrafficEvent> data;

    const MotionEvent* motion() const noexcept { return std::get_if<MotionEvent>(&data); }
    const UsageSnapshot* usages() const noexcept { return std::get_if<UsageSnapshot>(&data); }
    const TrafficEvent* traffic() const noexcept { return std::get_if<TrafficEvent>(&data); }

    /// the class it came on, the axis class for motion
    CatchClass class_() const;

    /// the interface or endpoint of traffic, empty for motion and usages
    std::optional<std::uint16_t> id() const noexcept {
        if (const auto* event = traffic()) {
            return event->id;
        }
        return std::nullopt;
    }

    /// the edge of a usage snapshot or the flow of traffic, both for motion
    Direction direction() const noexcept {
        if (const auto* event = traffic()) {
            return event->direction;
        }
        if (const auto* event = usages()) {
            return event->direction;
        }
        return Direction::Both;
    }

    /// the packet bytes of traffic, empty for motion and usages
    std::vector<std::uint8_t> bytes() const {
        if (const auto* event = traffic()) {
            return event->bytes;
        }
        return {};
    }

    static CatchEvent from_c(const MediusCatchEvent& raw) {
        CatchEvent out;
        out.kind = detail::read<CatchEventKind>(raw.kind);
        out.ts_us = raw.ts_us;
        out.clock = detail::read<ClockDomain>(raw.clock);
        switch (out.kind) {
        case CatchEventKind::Motion:
            out.data = MotionEvent::from_c(raw.data.motion);
            break;
        case CatchEventKind::Usages:
            out.data = UsageSnapshot::from_c(raw.data.usages);
            break;
        case CatchEventKind::Traffic:
            out.data = TrafficEvent::from_c(raw.data.traffic);
            break;
        }
        return out;
    }

    friend bool operator==(const CatchEvent& a, const CatchEvent& b) noexcept {
        return a.kind == b.kind && a.ts_us == b.ts_us && a.clock == b.clock && a.data == b.data;
    }
    friend bool operator!=(const CatchEvent& a, const CatchEvent& b) noexcept { return !(a == b); }
};

/// one decoded input event, a press or release, or a motion report. the deltas are 0 unless it's motion, x positive
/// going right, y down, the wheel up and pan right
struct InputEvent {
    InputKind kind = InputKind::Press;
    std::uint32_t ts_us = 0;
    ClockDomain clock = ClockDomain::HostChip;
    /// empty for motion
    std::optional<Usage> usage;
    std::int16_t dx = 0;
    std::int16_t dy = 0;
    std::int16_t dz = 0;
    std::int16_t pan = 0;

    bool is_press() const noexcept { return kind == InputKind::Press; }
    bool is_release() const noexcept { return kind == InputKind::Release; }

    /// the edge as a direction, both for motion
    Direction direction() const noexcept {
        switch (kind) {
        case InputKind::Press:
            return Direction::Press;
        case InputKind::Release:
            return Direction::Release;
        default:
            return Direction::Both;
        }
    }

    /// the axes a motion report moved, with their deltas, empty for an edge
    std::vector<std::pair<Axis, std::int16_t>> axes() const {
        if (kind != InputKind::Motion) {
            return {};
        }
        return MotionEvent{dx, dy, dz, pan}.axes();
    }

    static InputEvent from_c(const MediusInputEvent& raw) {
        InputEvent out;
        out.kind = detail::read<InputKind>(raw.kind);
        out.ts_us = raw.ts_us;
        out.clock = detail::read<ClockDomain>(raw.clock);
        if (out.kind != InputKind::Motion) {
            out.usage = Usage::from_c(raw.usage);
        }
        out.dx = raw.dx;
        out.dy = raw.dy;
        out.dz = raw.dz;
        out.pan = raw.pan;
        return out;
    }

    friend bool operator==(const InputEvent& a, const InputEvent& b) noexcept {
        return a.kind == b.kind && a.ts_us == b.ts_us && a.clock == b.clock && a.usage == b.usage && a.dx == b.dx &&
               a.dy == b.dy && a.dz == b.dz && a.pan == b.pan;
    }
    friend bool operator!=(const InputEvent& a, const InputEvent& b) noexcept { return !(a == b); }
};

inline bool ClipStatus::is_held(const Usage& usage) const { return std::find(held.begin(), held.end(), usage) != held.end(); }

inline std::string Version::mac_hex() const {
    static const char digits[] = "0123456789abcdef";
    std::string out;
    for (const auto byte : mac) {
        out += digits[byte >> 4];
        out += digits[byte & 0xF];
    }
    return out;
}

inline bool DeviceInfo::is_cloned() const {
    detail::ready();
    return medius_device_info_is_cloned(to_c());
}

inline std::string BoxInfo::id() const { return version.mac_hex(); }
inline const std::string& BoxInfo::name() const { return version.name; }
inline const std::optional<std::string>& BoxInfo::serial() const { return port.serial; }

inline bool TransferOutcome::is_ok() const { return status == TransferStatus::Ok; }

inline Transform Transform::swap(Axis a, Axis b) { return Transform{TransformOp::Swap, LockTarget(a), LockTarget(b)}; }

inline Transform Transform::remap(const LockTarget& source, const LockTarget& dest) {
    return Transform{TransformOp::Remap, source, dest};
}

inline bool FirmwareInfo::any_pending() const {
    return device.state == ImageState::PendingVerify || (host && host->state == ImageState::PendingVerify);
}

inline CatchClass CatchEvent::class_() const {
    if (const auto* event = traffic()) {
        return event->class_;
    }
    if (const auto* event = usages()) {
        switch (event->class_) {
        case Class::Button:
            return CatchClass::Button;
        case Class::Key:
            return CatchClass::Key;
        case Class::Media:
            return CatchClass::Media;
        }
    }
    return CatchClass::Axis;
}

/// like Rust's Display, "pending-verify", and "unknown(66)" for a state with no name
inline std::string to_string(ImageState state) {
    switch (state) {
    case ImageState::New:
        return "new";
    case ImageState::PendingVerify:
        return "pending-verify";
    case ImageState::Valid:
        return "valid";
    case ImageState::Invalid:
        return "invalid";
    case ImageState::Aborted:
        return "aborted";
    default:
        return "unknown(" + std::to_string(static_cast<int>(state)) + ")";
    }
}

/// like Rust's Display, "1.2.3 on ota_1 (pending-verify)"
inline std::string to_string(const ChipFirmware& firmware) {
    return std::to_string(firmware.major) + "." + std::to_string(firmware.minor) + "." +
           std::to_string(firmware.patch) + " on ota_" + std::to_string(firmware.slot) + " (" +
           to_string(firmware.state) + ")";
}

/// like Rust's Display, "keyboard", "mouse" or "unknown"
inline std::string to_string(DeviceKind kind) {
    switch (kind) {
    case DeviceKind::Keyboard:
        return "keyboard";
    case DeviceKind::Mouse:
        return "mouse";
    default:
        return "unknown";
    }
}

/// like Rust's Display, "device" or "host"
inline std::string to_string(UpdateTarget target) {
    switch (target) {
    case UpdateTarget::Device:
        return "device";
    case UpdateTarget::Host:
        return "host";
    default:
        return std::to_string(static_cast<int>(target));
    }
}

/// like Rust's Display, "046D:C08B G502", the ids alone when there's no product string
inline std::string to_string(const DeviceInfo& info) {
    char ids[10];
    std::snprintf(ids, sizeof ids, "%04X:%04X", static_cast<unsigned>(info.vid), static_cast<unsigned>(info.pid));
    return info.product.empty() ? std::string(ids) : std::string(ids) + " " + info.product;
}

/// like Rust's Display, "fw 1.2.3"
inline std::string to_string(const Version& version) {
    return "fw " + std::to_string(version.fw_major) + "." + std::to_string(version.fw_minor) + "." +
           std::to_string(version.fw_patch);
}

/// while the chip hasn't confirmed the image it booted. it refuses another update until then, since that image could
/// still be rolled back
inline constexpr bool is_pending(ImageState state) noexcept { return state == ImageState::PendingVerify; }

inline std::optional<std::vector<std::uint8_t>> TransferOutcome::ok_data() const {
    if (!is_ok()) {
        return std::nullopt;
    }
    return data;
}

inline Patch Patch::in_config(std::uint8_t cfg, std::uint16_t offset, std::vector<std::uint8_t> bytes) {
    return Patch{PatchSection::Config, cfg, 0, offset, std::move(bytes)};
}

inline Patch Patch::in_interface(std::uint8_t cfg, std::uint8_t interface_, std::uint16_t offset,
                                 std::vector<std::uint8_t> bytes) {
    return Patch{PatchSection::Report, cfg, interface_, offset, std::move(bytes)};
}

inline Patch Patch::in_string(std::uint8_t index, std::vector<std::uint8_t> bytes) {
    return Patch{PatchSection::String, 0, index, 0, std::move(bytes)};
}

inline PatchKey Patch::key() const { return PatchKey{section, cfg, index, offset}; }

inline RewriteKey RewriteRule::key() const { return RewriteKey{class_, id, direction, match_bytes, mask}; }

inline TransformKey Transform::key() const { return TransformKey{source, dest}; }

inline bool MouseCaps::is_composite() const { return n_hid > 1; }

inline bool CatchState::is_empty() const { return entries.empty(); }

inline std::vector<std::pair<Axis, std::int16_t>> MotionEvent::all_axes() const {
    return {{Axis::X, dx}, {Axis::Y, dy}, {Axis::Wheel, dz}, {Axis::Pan, pan}};
}

inline std::vector<std::pair<Axis, std::int16_t>> MotionEvent::axes() const {
    auto moved = all_axes();
    moved.erase(std::remove_if(moved.begin(), moved.end(), [](const auto& axis) { return axis.second == 0; }),
                moved.end());
    return moved;
}

inline bool Setup::is_in() const { return (request_type & 0x80) != 0; }

inline std::array<std::uint8_t, 8> Setup::to_bytes() const {
    return {request_type,
            request,
            static_cast<std::uint8_t>(value),
            static_cast<std::uint8_t>(value >> 8),
            static_cast<std::uint8_t>(index),
            static_cast<std::uint8_t>(index >> 8),
            static_cast<std::uint8_t>(length),
            static_cast<std::uint8_t>(length >> 8)};
}

inline std::int64_t ClockEstimate::drift_us_over(std::chrono::microseconds elapsed) const {
    // never negative, like Rust's Duration
    const auto micros = static_cast<std::int64_t>(anybox::detail::micros(elapsed, "the elapsed time"));
    if (!rate_ppb) {
        return 0;
    }
    // Rust's saturating multiply, worked out before it can overflow since C++ has no wider integer to do it in
    const std::int64_t ppb = *rate_ppb;
    constexpr auto max = (std::numeric_limits<std::int64_t>::max)();
    constexpr auto min = (std::numeric_limits<std::int64_t>::min)();
    if (micros != 0 && ppb != 0) {
        const bool positive = (micros > 0) == (ppb > 0);
        const bool over = positive ? (micros > 0 ? micros > max / ppb : micros < max / ppb)
                                   : (micros > 0 ? ppb < min / micros : micros < min / ppb);
        if (over) {
            return (positive ? max : min) / 1'000'000'000;
        }
    }
    return micros * ppb / 1'000'000'000;
}

inline bool Caps::has_mouse() const {
    detail::ready();
    return medius_caps_has_mouse(to_c());
}

inline bool Caps::has_keyboard() const {
    detail::ready();
    return medius_caps_has_keyboard(to_c());
}

inline bool Caps::is_composite() const {
    detail::ready();
    return medius_caps_is_composite(to_c());
}

inline std::optional<float> Rate::native_hz() const {
    detail::ready();
    float hz = 0;
    if (!medius_rate_native_hz(to_c(), &hz)) {
        return std::nullopt;
    }
    return hz;
}

inline bool LockEntry::is_block() const { return scale == 0; }

inline bool Locks::is_locked(const LockTarget& target, Direction direction) const {
    detail::ready();
    const auto raw = to_c();
    return medius_locks_is_locked(&raw, target.to_c(), detail::write(direction));
}

inline std::int16_t Locks::scale_of(const LockTarget& target, Direction direction) const {
    detail::ready();
    const auto raw = to_c();
    return medius_locks_scale_of(&raw, target.to_c(), detail::write(direction));
}

inline std::int32_t ClockEstimate::error_bound_us() const { return delay_us / 2; }

inline std::optional<std::int64_t> ClockEstimate::to_host_domain(std::int64_t device_us) const {
    if (!age_ms) {
        return std::nullopt;
    }
    return device_us + offset_us;
}

inline bool Bearing::is_live() const { return window_ms.has_value(); }

inline bool UsageSnapshot::is_held(const Usage& usage) const {
    return std::find(usages.begin(), usages.end(), usage) != usages.end();
}

inline bool TrafficEvent::truncated() const {
    detail::ready();
    const auto raw = to_c();
    return medius_traffic_event_truncated(&raw);
}

inline std::optional<std::array<std::uint8_t, 8>> TrafficEvent::setup_bytes() const {
    detail::ready();
    const auto raw = to_c();
    // it points into raw, so the bytes get copied out while raw is still here
    const auto* setup = medius_traffic_event_setup(&raw);
    if (!setup) {
        return std::nullopt;
    }
    std::array<std::uint8_t, 8> out{};
    std::copy(setup, setup + out.size(), out.begin());
    return out;
}

inline std::vector<std::uint8_t> TrafficEvent::data() const {
    detail::ready();
    const auto raw = to_c();
    std::uintptr_t length = 0;
    const auto* stage = medius_traffic_event_data(&raw, &length);
    if (!stage) {
        return {};
    }
    return std::vector<std::uint8_t>(stage, stage + length);
}

inline std::optional<ControlStatus> TrafficEvent::control_status() const {
    detail::ready();
    const auto raw = to_c();
    MediusControlStatus status{};
    if (!medius_traffic_event_control_status(&raw, &status)) {
        return std::nullopt;
    }
    return detail::read<ControlStatus>(status);
}

inline std::optional<TransferStatus> TrafficEvent::transfer_status() const {
    detail::ready();
    const auto raw = to_c();
    std::uint8_t status = 0;
    if (!medius_traffic_event_transfer_status(&raw, &status)) {
        return std::nullopt;
    }
    return detail::read<TransferStatus>(status);
}

inline std::optional<BusEvent> TrafficEvent::bus_event() const {
    detail::ready();
    const auto raw = to_c();
    MediusBusEvent bus{};
    if (!medius_traffic_event_bus_event(&raw, &bus)) {
        return std::nullopt;
    }
    return BusEvent::from_c(bus);
}

inline bool TrafficEvent::rule_acted() const {
    detail::ready();
    const auto raw = to_c();
    return medius_traffic_event_rule_acted(&raw);
}

inline bool TrafficEvent::bulk_end_of_transfer() const {
    detail::ready();
    const auto raw = to_c();
    return medius_traffic_event_bulk_end_of_transfer(&raw);
}

inline bool TrafficEvent::zlp() const {
    detail::ready();
    const auto raw = to_c();
    return medius_traffic_event_zlp(&raw);
}

/// whether it's measured against the bearing instead of a fixed sign
inline constexpr bool is_relative(Direction direction) noexcept {
    return direction == Direction::With || direction == Direction::Against;
}

/// a parsed input class, it arrives decoded with no packet, so a capture doesn't apply
inline bool is_input(CatchClass catch_class) {
    detail::ready();
    return medius_catch_class_is_input(detail::write(catch_class));
}

/// one of the eight byte oriented traffic classes
inline bool is_traffic(CatchClass catch_class) {
    detail::ready();
    return medius_catch_class_is_traffic(detail::write(catch_class));
}

inline constexpr bool is_ok(TransferStatus status) noexcept { return status == TransferStatus::Ok; }

/// whether the two meet, which is when either one is both or they're the same
inline constexpr bool admits(Direction direction, Direction other) noexcept {
    return direction == Direction::Both || other == Direction::Both || direction == other;
}

/// the sign of a delta, both for 0
inline constexpr Direction of_delta(std::int16_t delta) noexcept {
    return delta > 0 ? Direction::Positive : delta < 0 ? Direction::Negative : Direction::Both;
}

/// the direction a clip trigger's edge stands for
inline Direction to_direction(Edge edge) {
    switch (detail::read<Edge>(detail::write(edge))) {
    case Edge::Press:
        return Direction::Press;
    case Edge::Release:
        return Direction::Release;
    default:
        return Direction::Both;
    }
}

/// how this class reads a direction
inline DirectionMeaning direction_meaning(CatchClass catch_class) noexcept {
    switch (catch_class) {
    case CatchClass::Button:
    case CatchClass::Key:
    case CatchClass::Media:
        return DirectionMeaning::Edge;
    case CatchClass::Axis:
        return DirectionMeaning::Sign;
    default:
        return DirectionMeaning::Flow;
    }
}

inline CatchClass to_catch_class(Class usage_class) {
    switch (detail::read<Class>(detail::write(usage_class))) {
    case Class::Key:
        return CatchClass::Key;
    case Class::Media:
        return CatchClass::Media;
    default:
        return CatchClass::Button;
    }
}

// each traffic class is defined as its catch class, so the cast is exact
inline constexpr CatchClass to_catch_class(TrafficClass traffic_class) noexcept {
    return static_cast<CatchClass>(traffic_class);
}

/// the traffic class of a catch class, empty for the four input classes
inline std::optional<TrafficClass> to_traffic_class(CatchClass catch_class) {
    if (!is_traffic(catch_class)) {
        return std::nullopt;
    }
    return static_cast<TrafficClass>(catch_class);
}

/// one of the eight modifiers, the left and right ctrl, shift, alt and gui
inline constexpr bool is_modifier(Key key) noexcept { return key.value >= 0xE0 && key.value <= 0xE7; }

/// the control class, the only one that can answer or rewrite a native reply
inline constexpr bool is_control(RewriteClass rewrite_class) noexcept { return rewrite_class == RewriteClass::Control; }

/// whether a rule with this action needs a payload
inline constexpr bool carries_payload(RewriteAction action) noexcept {
    return action == RewriteAction::Patch || action == RewriteAction::Replace || action == RewriteAction::Answer ||
           action == RewriteAction::ReplyPatch || action == RewriteAction::ReplyReplace;
}

/// whether the box takes this action on that class, the same check it makes itself. drop is for the report classes
/// only, answer, stall, nak and the reply rewrites for control only
inline constexpr bool is_valid_for(RewriteAction action, RewriteClass rewrite_class) noexcept {
    switch (action) {
    case RewriteAction::Pass:
    case RewriteAction::Patch:
    case RewriteAction::Replace:
        return true;
    case RewriteAction::Drop:
        return !is_control(rewrite_class) && rewrite_class != RewriteClass::Any;
    case RewriteAction::Answer:
    case RewriteAction::Stall:
    case RewriteAction::Nak:
    case RewriteAction::ReplyPatch:
    case RewriteAction::ReplyReplace:
        return is_control(rewrite_class);
    }
    return false;
}

/// whether the op takes that source and destination, the same check the box makes. one field as both ends is refused,
/// and only the box knows whether the clone declares them
inline bool admits(TransformOp op, const LockTarget& source, const LockTarget& dest) {
    if (source == dest) {
        return false;
    }
    const bool axes = source.kind() != LockTargetKind::Usage && dest.kind() != LockTargetKind::Usage;
    switch (op) {
    case TransformOp::Swap:
        return axes;
    case TransformOp::Remap:
        return axes || (source.input() && source.input()->kind == Class::Button && dest.input());
    }
    return false;
}

/// every axis, in Rust's `Axis::ALL` order
inline constexpr std::array<Axis, 4> all_axes{Axis::X, Axis::Y, Axis::Wheel, Axis::Pan};

/// every blanket, in Rust's `Blanket::ALL` order
inline constexpr std::array<Blanket, 5> all_blankets{Blanket::Aim, Blanket::Wheel, Blanket::Buttons, Blanket::Keys,
                                                     Blanket::Media};

/// both clock domains, in Rust's `ClockDomain::ALL` order
inline constexpr std::array<ClockDomain, 2> all_clock_domains{ClockDomain::HostChip, ClockDomain::DeviceChip};

/// every traffic class, in Rust's `TrafficClass::ALL` order
inline constexpr std::array<TrafficClass, 8> all_traffic_classes{
    TrafficClass::HidIn,   TrafficClass::HidOut, TrafficClass::VendorInterrupt, TrafficClass::VendorBulk,
    TrafficClass::Control, TrafficClass::Emit,   TrafficClass::Bus,             TrafficClass::ClipTransfer};

// made by gen-medius from medius.h (free), change its tables and run dotnet run bindings/csharp/tools/gen-medius.cs
/// how long a query waits for the box by default
inline std::uint32_t default_query_timeout_ms() {
    detail::ready();
    return medius_default_query_timeout_ms();
}

/// how long a control transfer waits for its reply by default
inline std::uint32_t default_transfer_timeout_ms() {
    detail::ready();
    return medius_default_transfer_timeout_ms();
}

/// how often the library sends the box what's held while something is
inline std::uint32_t default_keepalive_cadence_ms() {
    detail::ready();
    return medius_default_keepalive_cadence_ms();
}

/// the medius C ABI the library speaks, which has to be the `AbiVersion` this header was made for. it doesn't check the
/// library first, so it still answers when the two don't match
inline std::uint32_t abi_version() { return medius_abi_version(); }

/// the medius crate's version inside the library
inline std::string version() {
    detail::ready();
    return anybox::detail::text(medius_version_string());
}
// the end of what gen-medius makes (free)

namespace detail {

// the three streams wait and end the same way. a read takes a copy of the handle under the lock and reads without
// it, so `close()` can drop the stream's own copy at any time and the C handle only goes once no read is in it. a
// close from another thread wakes a blocked read with the answer a gone box gives too, so the flag goes up BEFORE the
// C close and a read checks it once the call is back, which is the only way to tell the two apart
template <class Handle, void (*Free)(Handle*), void (*Close)(Handle*)>
class Stream {
public:
    Stream(Handle* handle, std::shared_ptr<MediusDevice> device) {
        // owned first, so the handle gets freed even when the state's own allocation fails
        std::shared_ptr<Handle> owned(handle, Free);
        state_ = std::make_unique<State>();
        state_->handle = std::move(owned);
        state_->device = std::move(device);
    }

    std::shared_ptr<Handle> handle() const {
        if (!state_) {
            throw anybox::ClosedError("this stream was moved from, use the one it was moved to");
        }
        std::lock_guard<std::mutex> guard(state_->lock);
        if (state_->closed) {
            throw closed();
        }
        return state_->handle;
    }

    const std::shared_ptr<MediusDevice>& device() const { return state_->device; }

    // after a call that ran without the lock, so whatever it brought back belongs to a stream that's gone
    void still_open() const {
        std::lock_guard<std::mutex> guard(state_->lock);
        if (state_->closed) {
            throw closed();
        }
    }

    void close() const {
        if (!state_) {
            throw anybox::ClosedError("this stream was moved from, use the one it was moved to");
        }
        std::shared_ptr<Handle> handle;
        {
            std::lock_guard<std::mutex> guard(state_->lock);
            if (state_->closed) {
                return;
            }
            state_->closed = true;
            handle = std::move(state_->handle);
        }
        Close(handle.get());
    }

private:
    static anybox::ClosedError closed() {
        return anybox::ClosedError("this stream was closed, subscribe again if you need it");
    }

    struct State {
        std::mutex lock;
        bool closed = false;
        std::shared_ptr<Handle> handle;
        std::shared_ptr<MediusDevice> device;
    };

    std::unique_ptr<State> state_;
};

// rounded up, so a timeout above zero never turns into a poll
inline std::uint64_t millis(std::chrono::microseconds timeout) {
    const auto micros = anybox::detail::micros(timeout, "a timeout");
    return micros / 1000 + (micros % 1000 != 0 ? 1 : 0);
}

// a for loop over a stream reads until the box is gone and everything queued was read, or the stream gets closed.
// any other error gets thrown out of the loop
template <class Source, class Item>
class Iterator {
public:
    using iterator_category = std::input_iterator_tag;
    using value_type = Item;
    using difference_type = std::ptrdiff_t;
    using pointer = Item*;
    using reference = Item&;

    Iterator() = default;
    explicit Iterator(const Source* source) : source_(source) { next(); }

    reference operator*() { return *current_; }
    pointer operator->() { return &*current_; }

    Iterator& operator++() {
        next();
        return *this;
    }

    /// the iterator as it was, holding the item it was on, so `*it++` works like on any input iterator
    Iterator operator++(int) {
        Iterator before = *this;
        next();
        return before;
    }

    friend bool operator==(const Iterator& a, const Iterator& b) noexcept { return a.source_ == b.source_; }
    friend bool operator!=(const Iterator& a, const Iterator& b) noexcept { return !(a == b); }

private:
    void next() {
        try {
            current_ = source_->recv();
        } catch (const DisconnectedError&) {
            end();
        } catch (const anybox::ClosedError&) {
            end();
        }
    }

    void end() noexcept {
        current_.reset();
        source_ = nullptr;
    }

    const Source* source_ = nullptr;
    std::optional<Item> current_;
};

}  // namespace detail

/// a live catch stream. a for loop over it reads events until the box is gone or it's closed, and closing it from
/// another thread ends a reader at once. like every stream and the clip, it holds its device's connection, so the
/// Device it came from can go first
class EventStream {
public:
    /// takes the handle over
    EventStream(MediusEventStream* stream, std::shared_ptr<MediusDevice> device) : stream_(stream, std::move(device)) {}

    /// waits for the next event. it throws DisconnectedError once the box is gone and everything queued was read, or
    /// a clone was closed, and anybox::ClosedError once this stream was closed
    CatchEvent recv() const {
        const auto handle = stream_.handle();
        MediusCatchEvent raw{};
        const auto status = medius_event_stream_recv(handle.get(), &raw);
        stream_.still_open();
        detail::check(status);
        return CatchEvent::from_c(raw);
    }

    /// the next event, or nothing when nothing came in time or the box is gone, `is_connected()` tells which. a
    /// negative timeout throws std::invalid_argument
    std::optional<CatchEvent> recv(std::chrono::microseconds timeout) const {
        const auto millis = detail::millis(timeout);
        const auto handle = stream_.handle();
        MediusCatchEvent raw{};
        const bool came = medius_event_stream_recv_timeout(handle.get(), millis, &raw);
        stream_.still_open();
        if (!came) {
            return std::nullopt;
        }
        return CatchEvent::from_c(raw);
    }

    std::optional<CatchEvent> try_recv() const {
        const auto handle = stream_.handle();
        MediusCatchEvent raw{};
        const bool came = medius_event_stream_try_recv(handle.get(), &raw);
        stream_.still_open();
        if (!came) {
            return std::nullopt;
        }
        return CatchEvent::from_c(raw);
    }

    std::uint64_t dropped() const {
        const auto handle = stream_.handle();
        const auto dropped = medius_event_stream_dropped(handle.get());
        stream_.still_open();
        return dropped;
    }

    /// whether the box still delivers, which tells "nothing yet" from "nothing ever again" when `try_recv()` gives
    /// nothing
    bool is_connected() const {
        const auto handle = stream_.handle();
        const bool connected = medius_event_stream_is_connected(handle.get());
        stream_.still_open();
        return connected;
    }

    /// another handle to the same subscription, they share the queue. closing any of them ends it for all, one that
    /// only goes away leaves the others reading
    EventStream clone() const {
        const auto handle = stream_.handle();
        auto* raw = medius_event_stream_clone(handle.get());
        if (!raw) {
            throw Error("cloning the event stream failed, which only a bug in medius does, so report it", Status::ErrUnknown, 0);
        }
        return EventStream(raw, stream_.device());
    }

    /// ends the subscription for every clone and wakes a `recv()` blocked in another thread, so it works from any
    /// thread. a second close does nothing
    void close() const { stream_.close(); }

    using iterator = detail::Iterator<EventStream, CatchEvent>;
    iterator begin() const { return iterator(this); }
    iterator end() const noexcept { return iterator(); }

private:
    detail::Stream<MediusEventStream, medius_event_stream_free, medius_event_stream_close> stream_;
};

/// a live stream of decoded input, presses, releases and motion. the box sends held usage snapshots and this turns
/// them into edges. read it like an EventStream
class InputStream {
public:
    /// takes the handle over
    InputStream(MediusInputStream* stream, std::shared_ptr<MediusDevice> device) : stream_(stream, std::move(device)) {}

    /// waits for the next event. it throws DisconnectedError once the box is gone and everything queued was read, and
    /// anybox::ClosedError once the stream was closed
    InputEvent recv() const {
        const auto handle = stream_.handle();
        MediusInputEvent raw{};
        const auto status = medius_input_stream_recv(handle.get(), &raw);
        stream_.still_open();
        detail::check(status);
        return InputEvent::from_c(raw);
    }

    /// the next event, or nothing when nothing came in time or the box is gone, `is_connected()` tells which. a
    /// negative timeout throws std::invalid_argument
    std::optional<InputEvent> recv(std::chrono::microseconds timeout) const {
        const auto millis = detail::millis(timeout);
        const auto handle = stream_.handle();
        MediusInputEvent raw{};
        const bool came = medius_input_stream_recv_timeout(handle.get(), millis, &raw);
        stream_.still_open();
        if (!came) {
            return std::nullopt;
        }
        return InputEvent::from_c(raw);
    }

    std::optional<InputEvent> try_recv() const {
        const auto handle = stream_.handle();
        MediusInputEvent raw{};
        const bool came = medius_input_stream_try_recv(handle.get(), &raw);
        stream_.still_open();
        if (!came) {
            return std::nullopt;
        }
        return InputEvent::from_c(raw);
    }

    /// the usages of that class this stream holds now
    std::vector<Usage> held(Class input_class) const {
        const auto number = detail::write(input_class);
        const auto handle = stream_.handle();
        std::vector<MediusUsage> raw(16);
        for (;;) {
            const auto count = medius_input_stream_held(handle.get(), number, raw.data(), raw.size());
            if (count <= raw.size()) {
                raw.resize(count);
                break;
            }
            raw.resize(count);
        }
        stream_.still_open();
        std::vector<Usage> out;
        for (const auto& usage : raw) {
            out.push_back(Usage::from_c(usage));
        }
        return out;
    }

    std::uint64_t dropped() const {
        const auto handle = stream_.handle();
        const auto dropped = medius_input_stream_dropped(handle.get());
        stream_.still_open();
        return dropped;
    }

    /// whether the box still delivers, see `EventStream::is_connected`
    bool is_connected() const {
        const auto handle = stream_.handle();
        const bool connected = medius_input_stream_is_connected(handle.get());
        stream_.still_open();
        return connected;
    }

    /// ends the subscription and wakes a `recv()` blocked in another thread, so it works from any thread. a second
    /// close does nothing
    void close() const { stream_.close(); }

    using iterator = detail::Iterator<InputStream, InputEvent>;
    iterator begin() const { return iterator(this); }
    iterator end() const noexcept { return iterator(); }

private:
    detail::Stream<MediusInputStream, medius_input_stream_free, medius_input_stream_close> stream_;
};

/// the box's log. read it like an EventStream
class LogStream {
public:
    /// takes the handle over
    LogStream(MediusLogStream* stream, std::shared_ptr<MediusDevice> device) : stream_(stream, std::move(device)) {}

    /// waits for the next line, see `EventStream::recv`
    LogLine recv() const {
        const auto handle = stream_.handle();
        MediusLogLine raw{};
        const auto status = medius_log_stream_recv(handle.get(), &raw);
        stream_.still_open();
        detail::check(status);
        return LogLine::from_c(raw);
    }

    /// the next line, or nothing when nothing came in time or the box is gone. a negative timeout throws
    /// std::invalid_argument
    std::optional<LogLine> recv(std::chrono::microseconds timeout) const {
        const auto millis = detail::millis(timeout);
        const auto handle = stream_.handle();
        MediusLogLine raw{};
        const bool came = medius_log_stream_recv_timeout(handle.get(), millis, &raw);
        stream_.still_open();
        if (!came) {
            return std::nullopt;
        }
        return LogLine::from_c(raw);
    }

    std::optional<LogLine> try_recv() const {
        const auto handle = stream_.handle();
        MediusLogLine raw{};
        const bool came = medius_log_stream_try_recv(handle.get(), &raw);
        stream_.still_open();
        if (!came) {
            return std::nullopt;
        }
        return LogLine::from_c(raw);
    }

    /// whether the box still delivers, see `EventStream::is_connected`
    bool is_connected() const {
        const auto handle = stream_.handle();
        const bool connected = medius_log_stream_is_connected(handle.get());
        stream_.still_open();
        return connected;
    }

    /// another handle to the same log. closing any of them ends it for all, one that only goes away leaves the others
    /// reading. other logs from the same device keep going either way
    LogStream clone() const {
        const auto handle = stream_.handle();
        auto* raw = medius_log_stream_clone(handle.get());
        if (!raw) {
            throw Error("cloning the log stream failed, which only a bug in medius does, so report it", Status::ErrUnknown, 0);
        }
        return LogStream(raw, stream_.device());
    }

    /// ends this log for every clone and wakes a `recv()` blocked in another thread, so it works from any thread. a
    /// second close does nothing
    void close() const { stream_.close(); }

    using iterator = detail::Iterator<LogStream, LogLine>;
    iterator begin() const { return iterator(this); }
    iterator end() const noexcept { return iterator(); }

private:
    detail::Stream<MediusLogStream, medius_log_stream_free, medius_log_stream_close> stream_;
};

/// puts box stamps on your clock. a catch stamp is microseconds on a chip that booted before this process, it wraps
/// about every 71.6 minutes and has nothing to do with any clock here. feed it every event as it comes, in order.
/// each chip is tracked on its own by the smallest gap between the time here and on the box, and an event can only
/// come late, never early, so the mapping only gets better. use it from one thread at a time, like medius says
class Timeline {
public:
    Timeline() {
        detail::ready();
        handle_.reset(medius_timeline_new());
        if (!handle_) {
            throw Error("the timeline couldn't be made, which only a bug in medius does, so report it", Status::ErrUnknown, 0);
        }
    }

    /// the handle for medius.h, which throws anybox::ClosedError once this was moved from
    MediusTimeline* get() const {
        if (!handle_) {
            throw anybox::ClosedError("this timeline was moved from, use the one it was moved to");
        }
        return handle_.get();
    }

    /// puts the event on this machine's clock. `now_ns` is a monotonic time in nanoseconds, the one
    /// `anybox::monotonic_ns()` reads when you leave it out. catch and input events share one timeline, so the two
    /// kinds of stream order together
    Stamped observe(const CatchEvent& event, std::optional<std::uint64_t> now_ns = std::nullopt) {
        // only the stamp and the chip are read, so those two are all that goes over
        MediusCatchEvent raw{};
        raw.kind = static_cast<MediusCatchEventKind>(detail::write(event.kind));
        raw.ts_us = event.ts_us;
        raw.clock = static_cast<MediusClockDomain>(detail::write(event.clock));
        MediusStamped stamped{};
        if (!medius_timeline_observe(get(), &raw, now_ns ? *now_ns : anybox_monotonic_ns(), &stamped)) {
            throw InvalidArgError("the timeline couldn't place the event, which only a bug in medius does, so report it", 0);
        }
        return Stamped::from_c(stamped);
    }

    Stamped observe(const InputEvent& event, std::optional<std::uint64_t> now_ns = std::nullopt) {
        MediusInputEvent raw{};
        raw.kind = static_cast<MediusInputKind>(detail::write(event.kind));
        raw.ts_us = event.ts_us;
        raw.clock = static_cast<MediusClockDomain>(detail::write(event.clock));
        MediusStamped stamped{};
        if (!medius_timeline_observe_input(get(), &raw, now_ns ? *now_ns : anybox_monotonic_ns(), &stamped)) {
            throw InvalidArgError("the timeline couldn't place the event, which only a bug in medius does, so report it", 0);
        }
        return Stamped::from_c(stamped);
    }

    /// puts a bare stamp from `clock`'s chip on this machine's clock, like `observe`
    Stamped observe_stamp(std::uint32_t ts_us, ClockDomain clock, std::optional<std::uint64_t> now_ns = std::nullopt) {
        CatchEvent event;
        event.ts_us = ts_us;
        event.clock = clock;
        return observe(event, now_ns);
    }

    // made by gen-medius from medius.h (timeline), change its tables and run dotnet run bindings/csharp/tools/gen-medius.cs
    /// forgets one chip's rollovers and floor, for a chip that rebooted
    void reset(ClockDomain domain) { medius_timeline_reset(get(), detail::write(domain)); }

    /// how many events a chip had, the floor is the smallest of them
    std::uint64_t samples(ClockDomain domain) const { return medius_timeline_samples(get(), detail::write(domain)); }
    // the end of what gen-medius makes (timeline)

private:
    std::unique_ptr<MediusTimeline, anybox::detail::Freer<MediusTimeline, medius_timeline_free>> handle_;
};

/// everything the box does on one tick, for `ClipBuilder::frame`. every call gives the frame back, so they chain.
/// use it from one thread at a time, like medius says
class ClipFrame {
public:
    ClipFrame() {
        detail::ready();
        handle_.reset(medius_clip_frame_new());
        if (!handle_) {
            throw std::bad_alloc();
        }
    }

    /// the handle for medius.h, which throws anybox::ClosedError once this was moved from
    MediusClipFrame* get() const {
        if (!handle_) {
            throw anybox::ClosedError("this clip frame was moved from, use the one it was moved to");
        }
        return handle_.get();
    }

    // made by gen-medius from medius.h (clip_frame), change its tables and run dotnet run bindings/csharp/tools/gen-medius.cs
    /// empties the frame to use it again
    ClipFrame& clear() {
        detail::check(medius_clip_frame_clear(get()));
        return *this;
    }

    /// the frame's cursor motion
    ClipFrame& move(std::int16_t dx, std::int16_t dy) {
        detail::check(medius_clip_frame_move(get(), dx, dy));
        return *this;
    }

    /// the frame's wheel motion
    ClipFrame& wheel(std::int16_t dz) {
        detail::check(medius_clip_frame_wheel(get(), dz));
        return *this;
    }

    /// the frame's horizontal scroll
    ClipFrame& pan(std::int16_t dpan) {
        detail::check(medius_clip_frame_pan(get(), dpan));
        return *this;
    }

    /// an edge on any usage with the action you pick
    ClipFrame& edge(const Usage& usage, Action action) {
        detail::check(medius_clip_frame_edge(get(), usage.to_c(), detail::write(action)));
        return *this;
    }

    ClipFrame& press(const Usage& usage) {
        detail::check(medius_clip_frame_press(get(), usage.to_c()));
        return *this;
    }

    /// a soft release, which leaves a physical hold alone
    ClipFrame& release(const Usage& usage) {
        detail::check(medius_clip_frame_release(get(), usage.to_c()));
        return *this;
    }

    /// a release that masks a physical hold too
    ClipFrame& force_release(const Usage& usage) {
        detail::check(medius_clip_frame_force_release(get(), usage.to_c()));
        return *this;
    }

    /// one raw report, as `Device::raw` sends one
    ClipFrame& raw(std::uint8_t ep_num, Direction dir, const std::vector<std::uint8_t>& bytes) {
        detail::check(medius_clip_frame_raw(get(), ep_num, detail::write(dir), bytes.data(), bytes.size()));
        return *this;
    }

    /// one control transfer, as `Device::transfer` runs one
    ClipFrame& transfer(std::uint8_t ep, const Setup& setup, const std::vector<std::uint8_t>& out_data = {}) {
        detail::check(medius_clip_frame_transfer(get(), ep, setup.to_c(), out_data.data(), out_data.size()));
        return *this;
    }

    /// the ring bytes the frame takes, `Clip::append` takes up to `ClipEntryMax`
    std::size_t byte_len() const { return medius_clip_frame_byte_len(get()); }
    // the end of what gen-medius makes (clip_frame)

private:
    std::unique_ptr<MediusClipFrame, anybox::detail::Freer<MediusClipFrame, medius_clip_frame_free>> handle_;
};

/// builds the entries of a buffered clip. give it to `Clip::append`, then `clear()` to use it again. every call gives
/// the builder back, so they chain. use it from one thread at a time, like medius says
class ClipBuilder {
public:
    ClipBuilder() {
        detail::ready();
        handle_.reset(medius_clip_builder_new());
        if (!handle_) {
            throw std::bad_alloc();
        }
    }

    /// the handle for medius.h, which throws anybox::ClosedError once this was moved from
    MediusClipBuilder* get() const {
        if (!handle_) {
            throw anybox::ClosedError("this clip builder was moved from, use the one it was moved to");
        }
        return handle_.get();
    }

    // made by gen-medius from medius.h (clip_builder), change its tables and run dotnet run bindings/csharp/tools/gen-medius.cs
    /// empties the builder to use it again after an append
    ClipBuilder& clear() {
        detail::check(medius_clip_builder_clear(get()));
        return *this;
    }

    /// the ring bytes the entries take, to check against `ClipStatus::free` before an append
    std::size_t byte_len() const { return medius_clip_builder_byte_len(get()); }

    /// nothing for that many native frames, 0 does nothing
    ClipBuilder& gap(std::uint16_t frames) {
        detail::check(medius_clip_builder_gap(get(), frames));
        return *this;
    }

    ClipBuilder& move(std::int16_t dx, std::int16_t dy) {
        detail::check(medius_clip_builder_move(get(), dx, dy));
        return *this;
    }

    ClipBuilder& wheel(std::int16_t dz) {
        detail::check(medius_clip_builder_wheel(get(), dz));
        return *this;
    }

    ClipBuilder& pan(std::int16_t dpan) {
        detail::check(medius_clip_builder_pan(get(), dpan));
        return *this;
    }

    ClipBuilder& press(const Usage& usage) {
        detail::check(medius_clip_builder_press(get(), usage.to_c()));
        return *this;
    }

    /// a soft release, which leaves a physical hold alone
    ClipBuilder& release(const Usage& usage) {
        detail::check(medius_clip_builder_release(get(), usage.to_c()));
        return *this;
    }

    /// a release that masks a physical hold too
    ClipBuilder& force_release(const Usage& usage) {
        detail::check(medius_clip_builder_force_release(get(), usage.to_c()));
        return *this;
    }

    ClipBuilder& edge(const Usage& usage, Action action = Action::Press) {
        detail::check(medius_clip_builder_edge(get(), usage.to_c(), detail::write(action)));
        return *this;
    }

    ClipBuilder& raw(std::uint8_t ep_num, Direction dir, const std::vector<std::uint8_t>& bytes) {
        detail::check(medius_clip_builder_raw(get(), ep_num, detail::write(dir), bytes.data(), bytes.size()));
        return *this;
    }

    /// one control transfer, its reply comes as a `TrafficClass::ClipTransfer` event
    ClipBuilder& transfer(std::uint8_t ep, const Setup& setup, const std::vector<std::uint8_t>& out_data = {}) {
        detail::check(medius_clip_builder_transfer(get(), ep, setup.to_c(), out_data.data(), out_data.size()));
        return *this;
    }

    /// one frame with all of the frame in it, which stays usable
    ClipBuilder& frame(const ClipFrame& frame) {
        detail::check(medius_clip_builder_frame(get(), frame.get()));
        return *this;
    }
    // the end of what gen-medius makes (clip_builder)

private:
    std::unique_ptr<MediusClipBuilder, anybox::detail::Freer<MediusClipBuilder, medius_clip_builder_free>> handle_;
};

/// one box's buffered clip playback, from `Device::clip`, keep one per clip session. a trigger runs a clip verb on the
/// box with no round trip to the PC, an input trigger (`bind`) on a button, key or media edge and a packet trigger
/// (`bind_packet`) on a packet crossing a traffic surface
class Clip {
public:
    /// takes the handle over
    Clip(MediusClip* clip, std::shared_ptr<MediusDevice> device) : handle_(clip), device_(std::move(device)) {}

    /// the handle for medius.h, which throws anybox::ClosedError once this was moved from
    MediusClip* get() const {
        if (!handle_) {
            throw anybox::ClosedError("this clip was moved from, use the one it was moved to");
        }
        return handle_.get();
    }

    /// locks these groups while the clip plays and lets go on stop, set it before the first append
    void set_autolock(const std::vector<Blanket>& scope) const {
        std::vector<std::uint8_t> groups;
        for (const auto blanket : scope) {
            groups.push_back(detail::write(blanket));
        }
        detail::check(medius_clip_set_autolock(get(), groups.empty() ? nullptr : groups.data(), groups.size()));
    }

    /// removes the packet trigger with that trigger's key, its other fields don't matter
    void unbind_packet(const ClipPacketTrigger& trigger) const {
        ClipPacketTrigger key = trigger;
        key.action = ClipAction::Start;
        key.consume = false;
        key.once_per_run = false;
        key.selector_len = 0;
        key.hits = 0;
        const auto raw = key.to_c();
        detail::check(medius_clip_unbind_packet(get(), &raw));
    }

    // made by gen-medius from medius.h (clip), change its tables and run dotnet run bindings/csharp/tools/gen-medius.cs
    /// adds the builder's entries to the ring. every entry is checked before the first frame goes out, so a refusal
    /// sends nothing
    void append(const ClipBuilder& builder) const { detail::check(medius_clip_append(get(), builder.get())); }

    /// loops at the clip's end, retained clips only
    void set_loop(bool on) const { detail::check(medius_clip_set_loop(get(), static_cast<std::uint8_t>(on))); }

    /// keeps the loaded clip so it can rewind and play again, off is streaming. set it before the first append
    void set_retain(bool on) const { detail::check(medius_clip_set_retain(get(), static_cast<std::uint8_t>(on))); }

    /// makes the clip's motion wait to ride a physical report instead of the box's own clock
    void set_ride(bool on) const { detail::check(medius_clip_set_ride(get(), static_cast<std::uint8_t>(on))); }

    void bind(const ClipTrigger& trigger) const { detail::check(medius_clip_bind(get(), trigger.to_c())); }

    void unbind(const Usage& usage, Edge edge) const {
        detail::check(medius_clip_unbind(get(), usage.to_c(), detail::write(edge)));
    }

    /// adds or overwrites a packet trigger. one the box would refuse throws `ClipPacketTriggerError` before anything is
    /// sent, with why in the message. to be sure a bind took, compare what `query_config()` reads back
    void bind_packet(const ClipPacketTrigger& trigger) const {
        const auto trigger_c = trigger.to_c();
        detail::check(medius_clip_bind_packet(get(), &trigger_c));
    }

    void clear_triggers() const { detail::check(medius_clip_clear_triggers(get())); }

    /// rewinds and plays, or goes on after a pause
    void start() const { detail::check(medius_clip_start(get())); }

    /// stops, flushes a streaming clip or rewinds a retained one, and lets go of held input and the autolock
    void stop() const { detail::check(medius_clip_stop(get())); }

    void pause() const { detail::check(medius_clip_pause(get())); }

    void resume() const { detail::check(medius_clip_resume(get())); }

    /// rewinds and plays even in the middle of playing
    void restart() const { detail::check(medius_clip_restart(get())); }

    /// plays when idle or paused, stops when playing
    void toggle() const { detail::check(medius_clip_toggle(get())); }

    /// throws the loaded clip away, frees the ring and clears a fault
    void clear() const { detail::check(medius_clip_clear(get())); }

    /// fixes a retained clip's end so it can play again and loop
    void finalize() const { detail::check(medius_clip_finalize(get())); }

    /// whether the box dropped the clip appended since the last clear, because its device chip restarted, it let go of
    /// the session, or a reconnect found it gone
    bool lost() const { return medius_clip_lost(get()); }

    ClipStatus query_status() const {
        MediusClipStatus out{};
        detail::check(medius_clip_query_status(get(), &out));
        return ClipStatus::from_c(out);
    }

    ClipSettings query_config() const {
        MediusClipSettings out{};
        detail::check(medius_clip_query_config(get(), &out));
        return ClipSettings::from_c(out);
    }
    // the end of what gen-medius makes (clip)

private:
    std::unique_ptr<MediusClip, anybox::detail::Freer<MediusClip, medius_clip_free>> handle_;
    std::shared_ptr<MediusDevice> device_;
};

namespace detail {

// what the progress callback reaches through `user`. an exception can't go back through Rust's frames, so the first
// one gets kept here and thrown once the C call is back, and the calls after it are skipped
struct Progress {
    const std::function<void(const UpdateProgress&)>* report;
    UpdateTarget target;
    std::exception_ptr failed;
};

inline void progressed(void* user, std::uintptr_t sent, std::uintptr_t total) noexcept {
    auto* progress = static_cast<Progress*>(user);
    if (progress->failed) {
        return;
    }
    try {
        (*progress->report)(UpdateProgress{progress->target, sent, total});
    } catch (...) {
        progress->failed = std::current_exception();
    }
}

// a window in whole ms from 1 to 65535 like the Rust API, since 0 is off on the wire and a zero window isn't
inline std::uint16_t window(std::optional<std::chrono::milliseconds> window) {
    if (!window) {
        return 0;
    }
    if (window->count() < 0) {
        throw std::invalid_argument("a window can't be negative");
    }
    if (window->count() == 0) {
        return 1;
    }
    return window->count() > 0xFFFF ? 0xFFFF : static_cast<std::uint16_t>(window->count());
}

}  // namespace detail

#if defined(MEDIUS_FEATURE_MOCK)
class MockBox;
#endif

/// an open connection to one Medius box, with everything only Medius does. every method can be called from any
/// thread, and a copy shares the connection. one from `device.medius()` shares anybox's connection, and keeping it
/// holds the port open after the anybox device is gone. anybox doesn't know about anything you press or lock through
/// it, and `reset()` lets go of and unlocks EVERYTHING, what anybox did too. medius.h has no close, so the last copy
/// going away is the only one, and the last clone of a connection waits for its background threads there
class Device {
public:
    /// takes a handle from medius.h over
    explicit Device(MediusDevice* device) : handle_(device, medius_device_free) {}

    /// the handle for medius.h, which throws anybox::ClosedError once this was moved from
    MediusDevice* get() const {
        if (!handle_) {
            throw anybox::ClosedError("this medius device was moved from, use the one it was moved to");
        }
        return handle_.get();
    }

    static Device open(const std::string& path) {
        detail::ready();
        MediusDevice* raw = nullptr;
        detail::check(medius_device_open(anybox::detail::c_text(path).c_str(), &raw));
        return Device(raw);
    }

    static Device find() {
        detail::ready();
        MediusDevice* raw = nullptr;
        detail::check(medius_device_find(&raw));
        return Device(raw);
    }

    /// the box whose id matches, its device MAC as hex or its CH343 serial. it throws BadProtoVerError when that box
    /// speaks another control protocol, and it skips a port that's already open, this program's own too
    static Device open_by_id(const std::string& id) {
        detail::ready();
        MediusDevice* raw = nullptr;
        detail::check(medius_device_open_by_id(anybox::detail::c_text(id).c_str(), &raw));
        return Device(raw);
    }

    /// the first box whose clone is a mouse
    static Device find_mouse_box() {
        detail::ready();
        MediusDevice* raw = nullptr;
        detail::check(medius_device_find_mouse_box(&raw));
        return Device(raw);
    }

    /// the first box whose clone is a keyboard
    static Device find_keyboard_box() {
        detail::ready();
        MediusDevice* raw = nullptr;
        detail::check(medius_device_find_keyboard_box(&raw));
        return Device(raw);
    }

    /// the first box `match` says yes to, preferring one this header speaks to. a box on another control protocol is
    /// only picked when nothing else matches, and opening it throws `BadProtoVerError`
    static Device find_where(const std::function<bool(const BoxInfo&)>& match) {
        const auto boxes = list();
        auto found = std::find_if(boxes.begin(), boxes.end(), [&](const BoxInfo& box) { return box.device && match(box); });
        if (found == boxes.end()) {
            found = std::find_if(boxes.begin(), boxes.end(), match);
        }
        if (found == boxes.end()) {
            // medius' own text for not finding a box, since this picks the box on this side
            throw NotFoundError("no medius device found", 0);
        }
        return open(found->port.path);
    }

    /// every serial port that could be a box
    static std::vector<PortInfo> find_ports() {
        detail::ready();
        return listed<PortInfo, MediusPortInfo>(medius_find_ports);
    }

    /// every box there is with what it says it is. one on another control protocol comes with an empty
    /// `BoxInfo::device`, and a port that's already open is skipped, this program's own too
    static std::vector<BoxInfo> list() {
        detail::ready();
        return listed<BoxInfo, MediusBoxInfo>(medius_list);
    }

#if defined(MEDIUS_FEATURE_MOCK)
    /// a device over a MockBox, after the version handshake
    static Device open_mock(const MockBox& mock);

    /// a device over a MockBox, without the handshake
    static Device with_mock(const MockBox& mock);
#endif

    /// another handle to the same connection
    Device clone() const {
        auto* raw = medius_device_clone(get());
        if (!raw) {
            throw Error("cloning the device failed, which only a bug in medius does, so report it", Status::ErrUnknown, 0);
        }
        return Device(raw);
    }

    // made by gen-medius from medius.h (device), change its tables and run dotnet run bindings/csharp/tools/gen-medius.cs
    void move_rel(std::int16_t dx, std::int16_t dy) const { detail::check(medius_device_move_rel(get(), dx, dy)); }

    void wheel(std::int16_t delta) const { detail::check(medius_device_wheel(get(), delta)); }

    /// a cursor move that skips movement riding, sent on the box's next mouse report
    void move_rel_now(std::int16_t dx, std::int16_t dy) const {
        detail::check(medius_device_move_rel_now(get(), dx, dy));
    }

    void wheel_now(std::int16_t delta) const { detail::check(medius_device_wheel_now(get(), delta)); }

    /// horizontal scroll, the whole range with no clamp
    void pan(std::int16_t delta) const { detail::check(medius_device_pan(get(), delta)); }

    void pan_now(std::int16_t delta) const { detail::check(medius_device_pan_now(get(), delta)); }

    /// sends the motion held for a ride now, whatever the ride window says
    void flush_motion() const { detail::check(medius_device_flush_motion(get())); }

    /// drops the motion held for a ride
    void discard_motion() const { detail::check(medius_device_discard_motion(get())); }

    void move_axis(const Motion& motion, MoveTiming timing = MoveTiming::Ride, PendingMotion pending = PendingMotion::Keep) const {
        detail::check(medius_device_move_axis(get(), motion.to_c(), detail::write(timing), detail::write(pending)));
    }

    void inject(const Usage& input, Action action) const {
        detail::check(medius_device_inject(get(), input.to_c(), detail::write(action)));
    }

    void press(const Usage& input) const { detail::check(medius_device_press(get(), input.to_c())); }

    void soft_release(const Usage& input) const { detail::check(medius_device_soft_release(get(), input.to_c())); }

    void force_release(const Usage& input) const { detail::check(medius_device_force_release(get(), input.to_c())); }

    /// weighs physical input on a target and direction. `scale` is the percent kept, 0 blocks, 100 passes, above that
    /// amplifies up to 255, and a negative one down to `LockScaleMin` weighs and reverses (axes only). `lock` and
    /// `unlock` are its two ends, and `Device::scale` in the medius docs says how the fixed and relative scales
    /// multiply
    void scale(const LockTarget& target, Direction dir, std::int16_t scale) const {
        detail::check(medius_device_scale(get(), target.to_c(), detail::write(dir), scale));
    }

    /// weighs a whole blanket, see `scale` for what the number means
    void scale_all(Blanket what, Direction dir, std::int16_t scale) const {
        detail::check(medius_device_scale_all(get(), detail::write(what), detail::write(dir), scale));
    }

    void lock(const LockTarget& target, Direction dir) const {
        detail::check(medius_device_lock(get(), target.to_c(), detail::write(dir)));
    }

    void unlock(const LockTarget& target, Direction dir) const {
        detail::check(medius_device_unlock(get(), target.to_c(), detail::write(dir)));
    }

    /// blocks a whole blanket. `Blanket::Keys` follows the direction, positive blocks the presses only and negative the
    /// releases only
    void lock_all(Blanket what, Direction dir) const {
        detail::check(medius_device_lock_all(get(), detail::write(what), detail::write(dir)));
    }

    void unlock_all(Blanket what, Direction dir) const {
        detail::check(medius_device_unlock_all(get(), detail::write(what), detail::write(dir)));
    }

    void led(LedTarget target, LedMode mode, std::uint8_t level) const {
        detail::check(medius_device_led(get(), detail::write(target), detail::write(mode), level));
    }

    void reset() const { detail::check(medius_device_reset(get())); }

    /// a reset that also erases the box's stored settings, its name and everything it learned, and reboots it. while it
    /// reboots the port stays there but quiet, so queries time out
    void factory_reset() const { detail::check(medius_device_factory_reset(get())); }

    void reapply() const { detail::check(medius_device_reapply(get())); }

    /// finds and opens this box again and sends it the held state. it throws `BadProtoVerError` when the box answers on
    /// another control protocol, and stays disconnected
    void reconnect() const { detail::check(medius_device_reconnect(get())); }

    void reboot(RebootTarget target) const { detail::check(medius_device_reboot(get(), detail::write(target))); }

    /// opts in to cloning devices the box can't clone faithfully, or back out. a change to the served patch set
    /// presents the clone again, which lets go of the session like a replug
    void allow_imperfect_clones(bool allow) const {
        detail::check(medius_device_allow_imperfect_clones(get(), allow));
    }

    /// puts `bytes` as they are on the cloned endpoint `ep_num` (0 to 15, only its low 4 bits go over), `Direction::In`
    /// to the game PC and `Direction::Out` to the real device, without waiting for anything. with the imperfect clone
    /// opt in off the box drops it silently and this still returns, `query_imperfect()` says which it is
    void raw(std::uint8_t ep_num, Direction dir, const std::vector<std::uint8_t>& bytes) const {
        detail::check(medius_device_raw(get(), ep_num, detail::write(dir), bytes.data(), bytes.size()));
    }

    /// adds or overwrites one rewrite rule, it needs the opt in. `query_rewrite()` says what the box holds
    void set_rewrite(const RewriteRule& rule) const {
        const auto rule_c = rule.to_c();
        detail::check(medius_device_set_rewrite(get(), &rule_c));
    }

    void clear_rewrite() const { detail::check(medius_device_clear_rewrite(get())); }

    RewriteTable query_rewrite() const {
        MediusRewriteTable out{};
        detail::check(medius_device_query_rewrite(get(), &out));
        return RewriteTable::from_c(out);
    }

    /// one rule in full, in the shape `set_rewrite` takes
    RewriteRule query_rewrite_entry(std::uint8_t index) const {
        MediusRewriteRule out{};
        detail::check(medius_device_query_rewrite_entry(get(), index, &out));
        return RewriteRule::from_c(out);
    }

    /// stores one descriptor patch, empty bytes remove the one at its key. it reaches the game PC the next time the
    /// clone is presented under the opt in
    void set_patch(const Patch& patch) const {
        const auto patch_c = patch.to_c();
        detail::check(medius_device_set_patch(get(), &patch_c));
    }

    /// presents the clone again with the stored patch set, it needs the opt in, and lets go of the session like a
    /// replug
    void apply_patch() const { detail::check(medius_device_apply_patch(get())); }

    void clear_patch() const { detail::check(medius_device_clear_patch(get())); }

    PatchSet query_patches() const {
        MediusPatchSet out{};
        detail::check(medius_device_query_patches(get(), &out));
        return PatchSet::from_c(out);
    }

    Patch query_patch_entry(std::uint8_t index) const {
        MediusPatch out{};
        detail::check(medius_device_query_patch_entry(get(), index, &out));
        return Patch::from_c(out);
    }

    /// adds or overwrites one field transform, no opt in needed
    void transform(const Transform& transform) const {
        const auto transform_c = transform.to_c();
        detail::check(medius_device_transform(get(), &transform_c));
    }

    /// drops the transform with that source and destination, whatever its op
    void untransform(const Transform& transform) const {
        const auto transform_c = transform.to_c();
        detail::check(medius_device_untransform(get(), &transform_c));
    }

    void clear_transforms() const { detail::check(medius_device_clear_transforms(get())); }

    void transform_swap(Axis a, Axis b) const {
        detail::check(medius_device_transform_swap(get(), detail::write(a), detail::write(b)));
    }

    void transform_remap(const LockTarget& source, const LockTarget& dest) const {
        detail::check(medius_device_transform_remap(get(), source.to_c(), dest.to_c()));
    }

    Transforms query_transforms() const {
        MediusTransforms out{};
        detail::check(medius_device_query_transforms(get(), &out));
        return Transforms::from_c(out);
    }

    /// the box's name, kept across power cuts, an empty one clears it. the box cuts a name past 32 bytes, or one that
    /// isn't ASCII, without an error
    void set_name(const std::string& name) const {
        detail::check(medius_device_set_name(get(), anybox::detail::c_text(name).c_str()));
    }

    /// back to the made up Medius-XXXX name
    void clear_name() const { detail::check(medius_device_clear_name(get())); }

    /// the render texture, and whether physical motion goes through the model too. both are one command and kept, so
    /// `full` is always needed. nothing renders until the box learned the attached device's profile
    /// (`RenderStatus::ready`)
    void set_render(RenderMode mode, bool full) const {
        detail::check(medius_device_set_render(get(), detail::write(mode), full));
    }

    /// the percent of the host's command interval an injected delta is spread across, 0 puts it all on the next report
    /// and 100 spreads it over one interval
    void set_spread(std::uint16_t percent) const { detail::check(medius_device_set_spread(get(), percent)); }

    Version query_version() const {
        MediusVersion out{};
        detail::check(medius_device_query_version(get(), &out));
        return Version::from_c(out);
    }

    /// both chips' firmware versions and the slots they booted
    FirmwareInfo firmware_info() const {
        MediusFirmwareInfo out{};
        detail::check(medius_device_firmware_info(get(), &out));
        return FirmwareInfo::from_c(out);
    }

    /// drops whatever is staged or on its way for one chip
    void abort_update(UpdateTarget target) const {
        detail::check(medius_device_abort_update(get(), detail::write(target)));
    }

    /// commits every staged image and reboots into it, the host chip first. it waits until the host chip is back
    void activate_firmware() const { detail::check(medius_device_activate_firmware(get())); }

    /// waits until neither chip is still on probation, a chip on probation refuses an update. it keeps asking while the
    /// box reboots after an activate. after 55 s a chip still on probation is an `UpdateError`, and a box that never
    /// answered a `QueryTimeoutError`
    FirmwareInfo wait_firmware_confirmed() const {
        MediusFirmwareInfo out{};
        detail::check(medius_device_wait_firmware_confirmed(get(), &out));
        return FirmwareInfo::from_c(out);
    }

    Health query_health() const {
        MediusHealth out{};
        detail::check(medius_device_query_health(get(), &out));
        return Health::from_c(out);
    }

    DeviceInfo device_info() const {
        MediusDeviceInfo out{};
        detail::check(medius_device_device_info(get(), &out));
        return DeviceInfo::from_c(out);
    }

    Caps caps() const {
        MediusCaps out{};
        detail::check(medius_device_caps(get(), &out));
        return Caps::from_c(out);
    }

    Rate query_rate() const {
        MediusRate out{};
        detail::check(medius_device_query_rate(get(), &out));
        return Rate::from_c(out);
    }

    Stats query_stats() const {
        MediusStats out{};
        detail::check(medius_device_query_stats(get(), &out));
        return Stats::from_c(out);
    }

    Locks query_locks() const {
        MediusLocks out{};
        detail::check(medius_device_query_locks(get(), &out));
        return Locks::from_c(out);
    }

    CatchState query_catch() const {
        MediusCatchState out{};
        detail::check(medius_device_query_catch(get(), &out));
        return CatchState::from_c(out);
    }

    ImperfectStatus query_imperfect() const {
        MediusImperfectStatus out{};
        detail::check(medius_device_query_imperfect(get(), &out));
        return ImperfectStatus::from_c(out);
    }

    EmitPaceStatus query_emit_pace() const {
        MediusEmitPaceStatus out{};
        detail::check(medius_device_query_emit_pace(get(), &out));
        return EmitPaceStatus::from_c(out);
    }

    Bearing query_bearing() const {
        MediusBearing out{};
        detail::check(medius_device_query_bearing(get(), &out));
        return Bearing::from_c(out);
    }

    RenderStatus query_render() const {
        MediusRenderStatus out{};
        detail::check(medius_device_query_render(get(), &out));
        return RenderStatus::from_c(out);
    }

    SpreadStatus query_spread() const {
        MediusSpreadStatus out{};
        detail::check(medius_device_query_spread(get(), &out));
        return SpreadStatus::from_c(out);
    }

    CountersSnapshot counters() const {
        MediusCountersSnapshot out{};
        detail::check(medius_device_counters(get(), &out));
        return CountersSnapshot::from_c(out);
    }
    // the end of what gen-medius makes (device)

    /// drops the rule with that rule's key, its action and payload don't matter
    void remove_rewrite(const RewriteRule& rule) const {
        // the payload isn't read, so one past what medius.h holds still removes like in Rust
        RewriteRule key = rule;
        key.payload.clear();
        const auto raw = key.to_c();
        detail::check(medius_device_remove_rewrite(get(), &raw));
    }

    /// the movement riding window, empty turns it off. it saturates at 65535 ms like the Rust API, and a negative one
    /// throws std::invalid_argument
    void set_movement_riding(std::optional<std::chrono::milliseconds> window) const {
        detail::check(medius_device_set_movement_riding(get(), window.has_value(), detail::window(window)));
    }

    /// the movement riding window, empty when it's off
    std::optional<std::chrono::milliseconds> query_movement_riding() const {
        bool enabled = false;
        std::uint32_t window_ms = 0;
        detail::check(medius_device_query_movement_riding(get(), &enabled, &window_ms));
        if (!enabled) {
            return std::nullopt;
        }
        return std::chrono::milliseconds(window_ms);
    }

    /// how injected motion is paced, and the rate the clone advertises and the box polls the device at (`force_hz`,
    /// empty is the device's own)
    void set_emit_pace(const EmitPace& pace, std::optional<std::uint16_t> force_hz = std::nullopt) const {
        detail::check(medius_device_set_emit_pace(get(), detail::write(pace.mode()), pace.hz(), force_hz.value_or(0)));
    }

    /// what `Direction::With` and `Direction::Against` are measured against. `window` is how long the last injected
    /// direction stays the bearing, empty turns it off and it saturates at 65535 ms. both go in one frame and are
    /// kept together, so the mode is always needed
    void set_bearing(std::optional<std::chrono::milliseconds> window, BearingMode mode) const {
        detail::check(medius_device_set_bearing(get(), detail::window(window), detail::write(mode)));
    }

    /// runs one control transfer on the real device and gives back its reply. a status that isn't ok is returned, not
    /// thrown, and the box answers refused while the opt in is off. the box gives up after about 800 ms on its own,
    /// so a shorter `timeout` gives up before a slow device answers. a negative timeout throws std::invalid_argument
    TransferOutcome transfer(std::uint8_t ep, const Setup& setup, const std::vector<std::uint8_t>& data = {},
                             std::optional<std::chrono::milliseconds> timeout = std::nullopt) const {
        MediusTransferOutcome raw{};
        if (timeout) {
            if (timeout->count() < 0 || timeout->count() > 0xFFFFFFFF) {
                throw std::invalid_argument("a timeout is from 0 to 4294967295 ms");
            }
            detail::check(medius_device_transfer_timeout(get(), ep, setup.to_c(), data.data(), data.size(),
                                                         static_cast<std::uint32_t>(timeout->count()), &raw));
        } else {
            detail::check(medius_device_transfer(get(), ep, setup.to_c(), data.data(), data.size(), &raw));
        }
        return TransferOutcome::from_c(raw);
    }

    /// writes one image into that chip's spare slot, where it does nothing until `activate_firmware()`. `progress`
    /// gets the bytes sent so far and the total, on this thread, and the first call can take up to 55 s to come,
    /// since staging waits for both chips first. if it throws, the update gets aborted once the image is through and
    /// its exception comes out of here
    void stage_firmware(UpdateTarget target, const std::vector<std::uint8_t>& image,
                        const std::function<void(const UpdateProgress& progress)>& progress = {}) const {
        const auto chip = detail::write(target);
        detail::Progress reporting{&progress, target, nullptr};
        const auto status = medius_device_stage_firmware(get(), chip, image.data(), image.size(),
                                                         progress ? detail::progressed : nullptr,
                                                         progress ? &reporting : nullptr);
        if (reporting.failed) {
            // what your progress threw is what you need, a stage or an abort that failed after it isn't
            try {
                abort_update(target);
            } catch (...) {
            }
            std::rethrow_exception(reporting.failed);
        }
        detail::check(status);
    }

    /// stages one image and activates it. a refused activate aborts the staged image, so it can't go live with some
    /// later activate, like the crate
    // the same steps as the crate's own update_firmware, not medius_device_update_firmware, because a progress that
    // throws has to stop the update before the activate, and inside that one call it can't
    void update_firmware(UpdateTarget target, const std::vector<std::uint8_t>& image,
                         const std::function<void(const UpdateProgress& progress)>& progress = {}) const {
        stage_firmware(target, image, progress);
        try {
            activate_firmware();
        } catch (...) {
            // the activate's error is what you need, a failed abort on top of it isn't, like the crate
            try {
                abort_update(target);
            } catch (...) {
            }
            throw;
        }
    }

    /// this box's buffered clip playback
    Clip clip() const {
        MediusClip* raw = nullptr;
        detail::check(medius_device_clip(get(), &raw));
        return Clip(raw, handle_);
    }

    /// subscribes to the catch stream. overlapping subscriptions merge into the box's one table, and every stream
    /// still gets everything it asked for
    EventStream catch_events(const std::vector<CatchFilter>& filters) const {
        const auto raw = c_filters(filters);
        MediusEventStream* stream = nullptr;
        detail::check(medius_device_catch_events(get(), raw.data(), raw.size(), &stream));
        return EventStream(stream, handle_);
    }

    /// subscribes to decoded input, presses, releases and motion. every filter has to be an input class covering
    /// both edges, so build them with the `watch` ones or `CatchFilter::all_input()`
    InputStream input_events(const std::vector<CatchFilter>& filters) const {
        const auto raw = c_filters(filters);
        MediusInputStream* stream = nullptr;
        detail::check(medius_device_input_events(get(), raw.data(), raw.size(), &stream));
        return InputStream(stream, handle_);
    }

    LogStream logs() const {
        MediusLogStream* stream = nullptr;
        detail::check(medius_device_logs(get(), &stream));
        return LogStream(stream, handle_);
    }

private:
    // asks with room for 16 and once more with room for all of them when there were more
    template <class T, class Raw>
    static std::vector<T> listed(std::uintptr_t (*listing)(Raw*, std::uintptr_t, std::uintptr_t*)) {
        std::vector<Raw> raw(16);
        for (;;) {
            std::uintptr_t total = 0;
            const auto written = listing(raw.data(), raw.size(), &total);
            if (total <= raw.size()) {
                raw.resize(written);
                break;
            }
            raw.resize(total);
        }
        std::vector<T> out;
        for (const auto& entry : raw) {
            out.push_back(T::from_c(entry));
        }
        return out;
    }

    // an empty list goes over too, so medius answers it with its own EmptySubscriptionError
    static std::vector<MediusCatchFilter> c_filters(const std::vector<CatchFilter>& filters) {
        std::vector<MediusCatchFilter> raw;
        for (const auto& filter : filters) {
            raw.push_back(filter.to_c());
        }
        return raw;
    }

    std::shared_ptr<MediusDevice> handle_;
};

#if defined(MEDIUS_FEATURE_MOCK)

/// a frame the mock got. a frame type this header has no name for stays its number
struct RecordedFrame {
    FrameType type = FrameType::Move;
    std::uint8_t seq = 0;
    std::vector<std::uint8_t> payload;

    friend bool operator==(const RecordedFrame& a, const RecordedFrame& b) noexcept {
        return a.type == b.type && a.seq == b.seq && a.payload == b.payload;
    }
    friend bool operator!=(const RecordedFrame& a, const RecordedFrame& b) noexcept { return !(a == b); }
};

/// the medius crate's scriptable fake box, for testing with no box plugged in. a copy shares the same box. it needs a
/// library built with the capi crate's mock feature, linking one without it fails
class MockBox {
public:
    MockBox() {
        detail::ready();
        auto* raw = medius_mock_new();
        if (!raw) {
            throw Error("the mock couldn't be made, which only a bug in medius does, so report it", Status::ErrUnknown, 0);
        }
        handle_ = std::shared_ptr<MediusMockBox>(raw, medius_mock_free);
    }

    /// the handle for medius.h, which throws anybox::ClosedError once this was moved from
    MediusMockBox* get() const {
        if (!handle_) {
            throw anybox::ClosedError("this mock was moved from, use the one it was moved to");
        }
        return handle_.get();
    }

    /// a Device over this mock, after the handshake
    Device open() const { return Device::open_mock(*this); }

    /// a Device over this mock, without the handshake
    Device with_device() const { return Device::with_mock(*this); }

    /// another handle sharing the same recorded state
    MockBox clone() const {
        auto* raw = medius_mock_clone(get());
        if (!raw) {
            throw Error("cloning the mock failed, which only a bug in medius does, so report it", Status::ErrUnknown, 0);
        }
        return MockBox(raw);
    }

    // made by gen-medius from medius.h (mock), change its tables and run dotnet run bindings/csharp/tools/gen-medius.cs
    void set_version(const Version& value) const { medius_mock_set_version(get(), value.to_c()); }

    void set_health(const Health& value) const { medius_mock_set_health(get(), value.to_c()); }

    void set_device_info(const DeviceInfo& value) const { medius_mock_set_device_info(get(), value.to_c()); }

    void set_caps(const Caps& value) const { medius_mock_set_caps(get(), value.to_c()); }

    void set_mouse_caps(const MouseCaps& value) const { medius_mock_set_mouse_caps(get(), value.to_c()); }

    void set_kbd_caps(const KbdCaps& value) const { medius_mock_set_kbd_caps(get(), value.to_c()); }

    void set_rate(const Rate& value) const { medius_mock_set_rate(get(), value.to_c()); }

    void set_stats(const Stats& value) const { medius_mock_set_stats(get(), value.to_c()); }

    void set_locks(const Locks& value) const { medius_mock_set_locks(get(), value.to_c()); }

    void set_catch_state(const CatchState& value) const { medius_mock_set_catch_state(get(), value.to_c()); }

    /// the mock's `query_imperfect()` answer. with the opt in off it drops its consuming clip packet triggers, like the
    /// box does
    void set_imperfect_status(const ImperfectStatus& value) const {
        medius_mock_set_imperfect_status(get(), value.to_c());
    }

    /// the canned answer to a transfer while the opt in is on, with it off the mock answers refused. a status this
    /// header has no name for is sent as its number
    void set_transfer_reply(TransferStatus status, const std::vector<std::uint8_t>& data = {}) const {
        medius_mock_set_transfer_reply(get(), detail::write(status), data.data(), data.size());
    }

    /// the mock's `query_render()` answer, `ready` is whether a profile is armed, which a real box needs before it
    /// renders
    void set_render(RenderMode mode, bool full, bool ready) const {
        medius_mock_set_render(get(), detail::write(mode), full, ready);
    }

    /// the command period the mock learned, in microseconds. a real box learns it from the moves it gets, so a mock
    /// left at 0 answers with a span of 0 whatever the percent
    void set_spread_learned(std::uint32_t period_us) const { medius_mock_set_spread_learned(get(), period_us); }

    /// the rate the mock's clone advertises when nothing forces one, 0 is no clone
    void set_advertised_hz(std::uint16_t hz) const { medius_mock_set_advertised_hz(get(), hz); }

    void set_clip_status(const ClipStatus& value) const { medius_mock_set_clip_status(get(), value.to_c()); }

    /// the mock's `Clip::query_config()` answer. its packet triggers get bound in order like `Clip::bind_packet` binds
    /// them, and the mock keeps only the ones the box would
    void set_clip_settings(const ClipSettings& value) const { medius_mock_set_clip_settings(get(), value.to_c()); }

    /// acts like the device chip restarting, the session state goes, the stored state stays, and the clone is back 100
    /// ms later
    void restart() const { medius_mock_restart(get()); }

    /// acts like the link between the chips dropping and coming back, so what the PC set goes, counted in
    /// `Stats::session`, and the clone stays up
    void link_lost() const { medius_mock_link_lost(get()); }

    /// acts like the real device unplugging. with `back_within_grace` it's back inside the 250 ms grace and the clone
    /// stays up, otherwise the clone goes when the grace ends until `attach()`
    void detach(bool back_within_grace = false) const { medius_mock_detach(get(), back_within_grace); }

    void attach() const { medius_mock_attach(get()); }

    /// refuses the next activate with that update status (protocol §4.16), and whatever is staged stays staged
    void refuse_next_activate(std::uint8_t status) const { medius_mock_refuse_next_activate(get(), status); }

    /// after each accepted activate, leaves the next `unanswered` firmware queries unanswered like a rebooting box,
    /// then has the device chip on probation for the next `pending`
    void reboot_after_activate(std::uint32_t unanswered, std::uint32_t pending) const {
        medius_mock_reboot_after_activate(get(), unanswered, pending);
    }

    /// stops answering queries, for good, for timeout tests
    void silent() const { medius_mock_silent(get()); }

    void push_log(LogLevel level, const std::string& text) const {
        medius_mock_push_log(get(), detail::write(level), anybox::detail::c_text(text).c_str());
    }

    void push_motion(std::uint8_t seq, std::uint32_t ts_us, const MotionEvent& event) const {
        medius_mock_push_motion(get(), seq, ts_us, event.to_c());
    }

    void push_usages(std::uint8_t seq, std::uint32_t ts_us, const UsageSnapshot& event) const {
        const auto event_c = event.to_c();
        medius_mock_push_usages(get(), seq, ts_us, &event_c);
    }

    /// a traffic event as if from the box, a `true_len` above the byte count is a capture that was cut
    void push_traffic(std::uint8_t seq, std::uint32_t ts_us, ClockDomain clock, const TrafficEvent& event) const {
        const auto event_c = event.to_c();
        medius_mock_push_traffic(get(), seq, ts_us, detail::write(clock), &event_c);
    }

    /// how many frames the mock got so far
    std::size_t recorded() const { return medius_mock_recorded(get()); }

    bool saw(FrameType ty) const { return medius_mock_saw(get(), detail::write(ty)); }

    void clear_recorded() const { medius_mock_clear_recorded(get()); }
    // the end of what gen-medius makes (mock)

    void set_movement_riding(std::optional<std::chrono::milliseconds> window) const {
        if (window && window->count() < 0) {
            throw std::invalid_argument("a window can't be negative");
        }
        const auto window_ms = window ? (window->count() > 0xFFFFFFFF ? 0xFFFFFFFF : window->count()) : 0;
        medius_mock_set_movement_riding(get(), window.has_value(), static_cast<std::uint32_t>(window_ms));
    }

    /// the mock's `Device::query_bearing()` answer
    void set_bearing(const Bearing& bearing) const {
        medius_mock_set_bearing(get(), bearing.window_ms.value_or(0), detail::write(bearing.mode));
    }

    void set_emit_pace(const EmitPace& pace, std::optional<std::uint16_t> force_hz = std::nullopt) const {
        medius_mock_set_emit_pace(get(), detail::write(pace.mode()), pace.hz(), force_hz.value_or(0));
    }

    /// runs one packet through the packet triggers like the box does, and gives back the action the most specific
    /// matching trigger drives (empty when none matches or a once per run trigger's run goes on) and whether it
    /// consumes the packet
    std::pair<std::optional<ClipAction>, bool> clip_packet(TrafficClass traffic_class, std::uint16_t id,
                                                           Direction direction,
                                                           const std::vector<std::uint8_t>& head) const {
        std::uint8_t action = 0;
        bool consumed = false;
        const bool fired = medius_mock_clip_packet(get(), detail::write(traffic_class), id, detail::write(direction),
                                                   head.data(), head.size(), &action, &consumed);
        if (!fired) {
            return {std::nullopt, consumed};
        }
        return {detail::read<ClipAction>(action), consumed};
    }

    void push_raw(const std::vector<std::uint8_t>& data) const {
        if (data.empty()) {
            return;
        }
        medius_mock_push_raw(get(), data.data(), data.size());
    }

    /// frame number `index` the mock got, empty past the end
    std::optional<RecordedFrame> recorded_frame(std::size_t index) const {
        if (index >= recorded()) {
            return std::nullopt;
        }
        std::vector<std::uint8_t> payload(MEDIUS_MAX_DEV_PAYLOAD);
        for (;;) {
            MediusFrameType type{};
            std::uint8_t seq = 0;
            const auto full = medius_mock_recorded_frame(get(), index, &type, &seq, payload.data(), payload.size());
            if (full <= payload.size()) {
                payload.resize(full);
                return RecordedFrame{detail::read<FrameType>(type), seq, std::move(payload)};
            }
            payload.resize(full);
        }
    }

private:
    explicit MockBox(MediusMockBox* raw) : handle_(raw, medius_mock_free) {}

    std::shared_ptr<MediusMockBox> handle_;
};

inline Device Device::open_mock(const MockBox& mock) {
    MediusDevice* raw = nullptr;
    detail::check(medius_device_open_mock(mock.get(), &raw));
    return Device(raw);
}

inline Device Device::with_mock(const MockBox& mock) {
    MediusDevice* raw = nullptr;
    detail::check(medius_device_with_mock(mock.get(), &raw));
    return Device(raw);
}

#endif

}  // namespace medius
}  // namespace anybox

#endif
