"""the values and the plain data that go in and out of a box, the same as the Rust types"""

from __future__ import annotations

from collections.abc import Iterator, Mapping
from dataclasses import dataclass
from enum import IntEnum, IntFlag
from typing import ClassVar

EXTRA_BASE = 256
# the header's ANYBOX_KEY_MODIFIER_FIRST, ANYBOX_KEY_MODIFIER_LAST and ANYBOX_BOX_CLOCK_WRAP_US
MODIFIER_FIRST = 0xE0
MODIFIER_LAST = 0xE7
BOX_CLOCK_WRAP_US = 1 << 32


class _Open(IntEnum):
    """an enum that also takes the values it has no name for, the way the Rust newtypes do"""

    @classmethod
    def _missing_(cls, value: object) -> _Open | None:
        if not isinstance(value, int) or not cls._fits(value):
            return None
        member = int.__new__(cls, value)
        member._name_ = cls._unnamed(value)
        member._value_ = value
        return cls._value2member_map_.setdefault(value, member)

    @classmethod
    def _fits(cls, value: int) -> bool:
        return False

    @classmethod
    def _unnamed(cls, value: int) -> str:
        return f"0x{value:02X}"


class Family(IntEnum):
    MEDIUS = 1
    MAKCU_V4 = 2
    FERRUM = 3
    YSC = 4

    def __str__(self) -> str:
        return {
            Family.MEDIUS: "Medius",
            Family.MAKCU_V4: "MAKCU V4",
            Family.FERRUM: "Ferrum",
            Family.YSC: "YSC 8K",
        }[self]


class FerrumApi(IntEnum):
    """the two ways into a Ferrum, see `FerrumApi` in the Rust docs"""

    SOFTWARE = 1
    LEGACY = 2

    def __str__(self) -> str:
        return "the Ferrum app's port" if self is FerrumApi.SOFTWARE else "the box's own legacy port"


class Button(_Open):
    """`Button.extra(n)` is a button past side2, numbered from 5 like `Button::Extra(n)` in Rust"""

    LEFT = 0
    RIGHT = 1
    MIDDLE = 2
    SIDE1 = 3
    SIDE2 = 4

    @classmethod
    def extra(cls, number: int) -> Button:
        if not 0 <= number <= 255:
            raise ValueError(
                f"an extra button's number is a byte, from 0 to 255, not {number} (the first real one is 5)"
            )
        return cls(EXTRA_BASE + number)

    @classmethod
    def _fits(cls, value: int) -> bool:
        return EXTRA_BASE <= value <= EXTRA_BASE + 255

    @classmethod
    def _unnamed(cls, value: int) -> str:
        return f"EXTRA{value - EXTRA_BASE}"

    def __str__(self) -> str:
        if self >= EXTRA_BASE:
            return f"extra {self - EXTRA_BASE}"
        return self.name.lower()


class Axis(IntEnum):
    X = 0
    Y = 1
    WHEEL = 2
    PAN = 3

    def __str__(self) -> str:
        return ("X", "Y", "wheel", "pan")[self]


class Direction(IntEnum):
    """positive is the way the box moves for a positive value, so right for X, down for Y, up for the wheel and
    right for pan"""

    BOTH = 0
    POSITIVE = 1
    NEGATIVE = 2


class Action(IntEnum):
    CONNECT = 0
    MOVE = 1
    MOVE_NOW = 2
    PRESS = 3
    RELEASE = 4
    CLICK = 5
    WHEEL = 6
    PAN = 7
    DISCARD_PENDING_MOVES = 8
    BUTTON_EVENTS = 9
    MOTION_EVENTS = 10
    KEY_EVENTS = 11
    MEDIA_EVENTS = 12
    KEY_PRESS = 13
    KEY_RELEASE = 14
    KEY_TAP = 15
    MEDIA_PRESS = 16
    MEDIA_RELEASE = 17
    MEDIA_TAP = 18
    LOCK_BUTTON = 19
    LOCK_AXIS = 20
    LOCK_KEY = 21
    SCALE_AXIS = 22
    TYPE_TEXT = 23
    KEY_CHORD = 24
    FORCE_RELEASE_KEY = 25
    MAKCU_INTERPOLATION = 0x10000
    MAKCU_MOVING = 0x10001
    MAKCU_AUTORELEASE = 0x10002
    YSC_INTERPOLATION = 0x20000
    YSC_SERIAL = 0x20001
    FERRUM_CATCH_XY = 0x30000

    def __str__(self) -> str:
        return _ACTION_TEXTS[self]


_ACTION_TEXTS = {
    Action.CONNECT: "connect",
    Action.MOVE: "move",
    Action.MOVE_NOW: "move now",
    Action.PRESS: "press",
    Action.RELEASE: "release",
    Action.CLICK: "click",
    Action.WHEEL: "scroll",
    Action.PAN: "scroll sideways",
    Action.DISCARD_PENDING_MOVES: "discard pending moves",
    Action.BUTTON_EVENTS: "report button events",
    Action.MOTION_EVENTS: "report motion events",
    Action.KEY_EVENTS: "report key events",
    Action.MEDIA_EVENTS: "report media key events",
    Action.KEY_PRESS: "press a key",
    Action.KEY_RELEASE: "release a key",
    Action.KEY_TAP: "tap a key",
    Action.MEDIA_PRESS: "press a media key",
    Action.MEDIA_RELEASE: "release a media key",
    Action.MEDIA_TAP: "tap a media key",
    Action.LOCK_BUTTON: "lock or unlock a button",
    Action.LOCK_AXIS: "lock or unlock an axis",
    Action.LOCK_KEY: "lock or unlock a key",
    Action.SCALE_AXIS: "scale an axis",
    Action.TYPE_TEXT: "type text",
    Action.KEY_CHORD: "press several keys at once",
    Action.FORCE_RELEASE_KEY: "force a key up or give it back",
    Action.MAKCU_INTERPOLATION: "read or set the MAKCU's interpolation",
    Action.MAKCU_MOVING: "ask the MAKCU whether it's still moving",
    Action.MAKCU_AUTORELEASE: "read or set the MAKCU's autorelease",
    Action.YSC_INTERPOLATION: "read or set the YSC's interpolation",
    Action.YSC_SERIAL: "read the serial the YSC gives its mouse",
    Action.FERRUM_CATCH_XY: "read the Ferrum's recent mouse motion",
}


class Execution(IntEnum):
    NATIVE = 0
    """the box does it, with its own timing"""
    HOST = 1
    """anybox builds it on the PC out of things the box can do, so it runs on the PC's timing"""
    UNSUPPORTED = 2


class QuirkKind(IntEnum):
    FORCED_UP_ON_RELEASE = 0
    PHYSICAL_OVERRIDE = 1
    WHOLE_AXIS_ONLY = 2
    NOTCHES_PER_COMMAND = 3
    FIXED_CLICK_TIMING = 4
    MAX_HOLD = 5
    BOX_CLICK_TIMING = 6
    HOST_CLICK_TIMING = 7
    KEEPS_HELD = 8
    MAX_KEYS_HELD = 9
    KEEPS_LOCKED = 10
    TYPED_TEXT_IGNORES_CAPS_LOCK = 11
    TYPING_RELEASES_HELD_KEYS = 12


class FirmwareKind(IntEnum):
    VERSION = 1
    BUILD = 2
    PROTOCOL = 3
    TEXT = 4


class _EventKind(IntEnum):
    BUTTON = 0
    KEY = 1
    MEDIA = 2
    MOTION = 3
    DROPPED = 4
    SESSION_RESET = 5
    REJECTED = 6
    DISCONNECTED = 7
    RECONNECTED = 8
    CHANGED = 9


class _IdKind(IntEnum):
    NONE = 0
    MAC = 1
    USB_SERIAL = 2


class DropSource(IntEnum):
    DEVICE = 0
    """the box's own counters"""
    HOST = 1
    """your receiver fell behind and its queue overflowed"""


class Clock(IntEnum):
    """each box clock is its own counter, so only compare stamps from the same one"""

    MEDIUS_HOST_CHIP = 0
    MEDIUS_DEVICE_CHIP = 1

    def wraps_after(self) -> float:
        """in seconds, a 32 bit microsecond counter rolls over every 71.6 minutes"""
        return BOX_CLOCK_WRAP_US / 1_000_000


class IoKind(IntEnum):
    """what kind of I/O error an `IoError` is, `OTHER` for every kind this has no number for"""

    OTHER = 1
    NOT_FOUND = 2
    PERMISSION_DENIED = 3
    TIMED_OUT = 4
    BROKEN_PIPE = 5
    WOULD_BLOCK = 6
    INVALID_INPUT = 7
    INVALID_DATA = 8
    UNEXPECTED_EOF = 9
    INTERRUPTED = 10
    UNSUPPORTED = 11
    OUT_OF_MEMORY = 12
    RESOURCE_BUSY = 13
    WRITE_ZERO = 14
    NOT_CONNECTED = 15


class EventKinds(IntFlag):
    """which input a receiver wants, combined with `|`. connection events and drops reach every receiver"""

    NONE = 0
    BUTTONS = 1
    MOTION = 2
    KEYS = 4
    MEDIA = 8
    ALL = 15
    """everything the box can stream, a kind it can't is left out without a `Rejected`"""


class YscProfile(_Open):
    """the shape a YSC gives a spread move, a number the vendor doesn't name yet is kept as the box sent it"""

    LINEAR = 0
    EASE_IN_OUT = 1
    MIN_JERK = 2

    @classmethod
    def _fits(cls, value: int) -> bool:
        return 0 <= value <= 0xFFFFFFFF

    @classmethod
    def _unnamed(cls, value: int) -> str:
        return f"OTHER_{value}"


class Key(_Open):
    """a keyboard key by its HID usage (page 0x07). `Key(n)` gets you any other one from 0x04 to 0xE7"""

    A = 0x04
    B = 0x05
    C = 0x06
    D = 0x07
    E = 0x08
    F = 0x09
    G = 0x0A
    H = 0x0B
    I = 0x0C
    J = 0x0D
    K = 0x0E
    L = 0x0F
    M = 0x10
    N = 0x11
    O = 0x12
    P = 0x13
    Q = 0x14
    R = 0x15
    S = 0x16
    T = 0x17
    U = 0x18
    V = 0x19
    W = 0x1A
    X = 0x1B
    Y = 0x1C
    Z = 0x1D
    NUM1 = 0x1E
    NUM2 = 0x1F
    NUM3 = 0x20
    NUM4 = 0x21
    NUM5 = 0x22
    NUM6 = 0x23
    NUM7 = 0x24
    NUM8 = 0x25
    NUM9 = 0x26
    NUM0 = 0x27
    ENTER = 0x28
    ESCAPE = 0x29
    BACKSPACE = 0x2A
    TAB = 0x2B
    SPACE = 0x2C
    MINUS = 0x2D
    EQUAL = 0x2E
    LEFT_BRACKET = 0x2F
    RIGHT_BRACKET = 0x30
    BACKSLASH = 0x31
    SEMICOLON = 0x33
    APOSTROPHE = 0x34
    GRAVE = 0x35
    COMMA = 0x36
    PERIOD = 0x37
    SLASH = 0x38
    CAPS_LOCK = 0x39
    F1 = 0x3A
    F2 = 0x3B
    F3 = 0x3C
    F4 = 0x3D
    F5 = 0x3E
    F6 = 0x3F
    F7 = 0x40
    F8 = 0x41
    F9 = 0x42
    F10 = 0x43
    F11 = 0x44
    F12 = 0x45
    PRINT_SCREEN = 0x46
    SCROLL_LOCK = 0x47
    PAUSE = 0x48
    INSERT = 0x49
    HOME = 0x4A
    PAGE_UP = 0x4B
    DELETE = 0x4C
    END = 0x4D
    PAGE_DOWN = 0x4E
    RIGHT = 0x4F
    LEFT = 0x50
    DOWN = 0x51
    UP = 0x52
    NUM_LOCK = 0x53
    KEYPAD_DIVIDE = 0x54
    KEYPAD_MULTIPLY = 0x55
    KEYPAD_MINUS = 0x56
    KEYPAD_PLUS = 0x57
    KEYPAD_ENTER = 0x58
    KEYPAD1 = 0x59
    KEYPAD2 = 0x5A
    KEYPAD3 = 0x5B
    KEYPAD4 = 0x5C
    KEYPAD5 = 0x5D
    KEYPAD6 = 0x5E
    KEYPAD7 = 0x5F
    KEYPAD8 = 0x60
    KEYPAD9 = 0x61
    KEYPAD0 = 0x62
    KEYPAD_PERIOD = 0x63
    MENU = 0x65
    LEFT_CTRL = 0xE0
    LEFT_SHIFT = 0xE1
    LEFT_ALT = 0xE2
    LEFT_GUI = 0xE3
    RIGHT_CTRL = 0xE4
    RIGHT_SHIFT = 0xE5
    RIGHT_ALT = 0xE6
    RIGHT_GUI = 0xE7

    def is_modifier(self) -> bool:
        """ctrl, shift, alt and gui on either side. a keyboard report keeps these in a byte of their own"""
        return MODIFIER_FIRST <= self <= MODIFIER_LAST

    def __str__(self) -> str:
        return f"key 0x{int(self):02X}"

    @classmethod
    def _fits(cls, value: int) -> bool:
        return 0 <= value <= 0xFF


class MediaKey(_Open):
    """a media key by its HID consumer usage (page 0x0C). `MediaKey(n)` gets you any other one"""

    PLAY = 0xB0
    PAUSE = 0xB1
    NEXT_TRACK = 0xB5
    PREVIOUS_TRACK = 0xB6
    STOP = 0xB7
    PLAY_PAUSE = 0xCD
    MUTE = 0xE2
    VOLUME_UP = 0xE9
    VOLUME_DOWN = 0xEA

    @classmethod
    def _fits(cls, value: int) -> bool:
        return 0 <= value <= 0xFFFF

    def __str__(self) -> str:
        return f"media key 0x{int(self):04X}"


class _ClickTimingKind(IntEnum):
    MAKCU = 0
    FERRUM = 1
    FIXED = 2
    OFF = 3


@dataclass(frozen=True)
class ClickTiming:
    """what a click and the taps do on a box that can't time them itself. `ClickTiming.MAKCU` (a random hold from
    35 to 75 ms) is the default, `ClickTiming.FERRUM` holds 75 to 125 ms, `ClickTiming.fixed(seconds)` always the
    same, and with `ClickTiming.OFF` a click or tap without a hold fails on a box like that"""

    kind: _ClickTimingKind
    hold: float | None = None

    MAKCU: ClassVar[ClickTiming]
    FERRUM: ClassVar[ClickTiming]
    OFF: ClassVar[ClickTiming]

    @classmethod
    def fixed(cls, seconds: float) -> ClickTiming:
        return cls(_ClickTimingKind.FIXED, seconds)


ClickTiming.MAKCU = ClickTiming(_ClickTimingKind.MAKCU)
ClickTiming.FERRUM = ClickTiming(_ClickTimingKind.FERRUM)
ClickTiming.OFF = ClickTiming(_ClickTimingKind.OFF)


@dataclass(frozen=True)
class StableId:
    """a box's own id, which stays the same whatever port it's on. give it one of the two"""

    mac: bytes | None = None
    usb_serial: str | None = None

    def __post_init__(self) -> None:
        if (self.mac is None) == (self.usb_serial is None):
            raise ValueError("a StableId is either a mac or a usb_serial, give it exactly one")
        if self.mac is not None and len(self.mac) != 6:
            raise ValueError(f"a MAC is 6 bytes, not {len(self.mac)}")

    def __str__(self) -> str:
        if self.mac is not None:
            return "MAC " + ":".join(f"{byte:02x}" for byte in self.mac)
        return f"USB serial {self.usb_serial}"


@dataclass(frozen=True)
class Firmware:
    """`version` is (major, minor, patch) on a box that has one, otherwise `build` or `text` says it. `protocol`
    only shows up in the error of a Medius that's too old or too new, never in `Info`"""

    kind: FirmwareKind
    version: tuple[int, int, int] | None = None
    build: int | None = None
    protocol: int | None = None
    text: str | None = None

    def __str__(self) -> str:
        if self.version is not None:
            return ".".join(map(str, self.version))
        if self.build is not None:
            return f"build {self.build}"
        if self.protocol is not None:
            return f"protocol {self.protocol}"
        return self.text or ""


@dataclass(frozen=True)
class Interpolation:
    """how long a MAKCU spreads a move, `Interpolation.auto()` or `Interpolation.percent(n)` from 0 to 100"""

    automatic: bool
    percentage: int = 0

    @classmethod
    def auto(cls) -> Interpolation:
        return cls(True)

    @classmethod
    def percent(cls, n: int) -> Interpolation:
        return cls(False, n)


@dataclass(frozen=True)
class YscInterpolation:
    """a YSC's interpolation as the box reports it, in seconds. what the box didn't send is None, the vendor's own
    app only reads `enabled`, `profile` and the measured interval"""

    enabled: bool
    profile: YscProfile
    window: float | None
    """a fixed window, zero means the box adapts it to how often moves come"""
    max_window: float | None
    measured_interval: float | None


@dataclass(frozen=True)
class YscInterpolationSettings:
    """what `set_interpolation` sends, the windows in seconds and whole milliseconds up to 60 s"""

    enabled: bool
    profile: YscProfile
    window: float
    max_window: float


@dataclass(frozen=True)
class YscSerial:
    """the serial a YSC gives the mouse behind it, each None when the box didn't send it"""

    custom: str | None
    physical: str | None
    active: str | None


@dataclass(frozen=True)
class UsbIds:
    vid: int
    pid: int


@dataclass(frozen=True)
class Info:
    """who the box is, see `Info` in the Rust docs for what each one means"""

    port: str
    family: Family
    model: str | None
    firmware: Firmware | None
    id: StableId | None
    ferrum_api: FerrumApi | None
    peripheral: UsbIds | None


@dataclass(frozen=True)
class Quirk:
    """one way the box behaves differently, `kind` says which. `min` and `max` are seconds, and only the fields
    that kind has are set"""

    kind: QuirkKind
    min: float | None = None
    max: float | None = None
    count: int | None = None
    click_timing: ClickTiming | None = None


@dataclass(frozen=True)
class Capability:
    execution: Execution
    reason: str | None = None
    """why the box can't, only when it's unsupported"""
    quirks: tuple[Quirk, ...] = ()

    @property
    def supported(self) -> bool:
        return self.execution != Execution.UNSUPPORTED


@dataclass(frozen=True)
class Capabilities(Mapping[Action, Capability]):
    """what this box, on this firmware, with this mouse behind it, can do for every action"""

    actions: Mapping[Action, Capability]
    buttons: tuple[Button, ...]
    axes: tuple[Axis, ...]
    """the axes you can lock or scale on this box"""

    def __getitem__(self, action: Action) -> Capability:
        return self.actions[action]

    def __iter__(self) -> Iterator[Action]:
        return iter(self.actions)

    def __len__(self) -> int:
        return len(self.actions)


@dataclass(frozen=True)
class Snapshot:
    """who the box is and what it can do, both read from the same connection"""

    info: Info
    capabilities: Capabilities


@dataclass(frozen=True)
class BoxTime:
    micros: int
    clock: Clock


@dataclass(frozen=True)
class Stamp:
    """`since_open_ns` is the best guess at when it happened, as the time since the device was opened, and the one
    to compare events by. `received_since_open_ns` is when anybox got it, and `received_qpc` the raw
    QueryPerformanceCounter value then, on Windows only. `arrived_ns` and `received_ns` are the same two moments on
    the clock `monotonic_ns()` reads, which is the one `time.perf_counter_ns()` reads on Windows and Linux, so they
    line up with other devices and the rest of your program"""

    since_open_ns: int
    received_since_open_ns: int
    arrived_ns: int
    received_ns: int
    received_qpc: int | None = None
    box_time: BoxTime | None = None


@dataclass(frozen=True)
class ButtonEvent:
    button: Button
    pressed: bool
    at: Stamp


@dataclass(frozen=True)
class KeyEvent:
    """a physical key on the keyboard behind the box. just know that you see every key typed there, passwords
    included"""

    key: Key
    pressed: bool
    at: Stamp


@dataclass(frozen=True)
class MediaEvent:
    key: MediaKey
    pressed: bool
    at: Stamp


@dataclass(frozen=True)
class MotionEvent:
    """physical motion. the wheel is positive going up and pan is positive going right"""

    dx: int
    dy: int
    wheel: int
    pan: int
    at: Stamp


@dataclass(frozen=True)
class Dropped:
    """how many events never reached you since the last one from the same source, see `Event::Dropped` in the Rust
    docs"""

    count: int
    source: DropSource


@dataclass(frozen=True)
class SessionReset:
    """the box lost its state, so the buttons and keys you were holding got released, unless the box has
    `QuirkKind.KEEPS_HELD`. what you knew about which physical ones were held is out of date too"""


@dataclass(frozen=True)
class Rejected:
    """the box refused `count` of `action` after the call already returned, so slow down and send it again if it
    mattered"""

    action: Action
    count: int


@dataclass(frozen=True)
class Disconnected:
    pass


@dataclass(frozen=True)
class Reconnected:
    """the box is back, what you held got released (unless it has `QuirkKind.KEEPS_HELD`), and the snapshot may
    have changed"""


@dataclass(frozen=True)
class Changed:
    """the snapshot changed with no reconnect, so read it again"""


Event = (
    ButtonEvent
    | KeyEvent
    | MediaEvent
    | MotionEvent
    | Dropped
    | SessionReset
    | Rejected
    | Disconnected
    | Reconnected
    | Changed
)
