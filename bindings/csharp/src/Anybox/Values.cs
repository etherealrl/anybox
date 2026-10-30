using System.Collections.ObjectModel;
using System.Net.NetworkInformation;

namespace Anybox;

public enum Family : uint
{
    Medius = 1,
    MakcuV4 = 2,
    Ferrum = 3,
    Ysc = 4,
}

/// <summary>the two ways into a Ferrum, see <c>FerrumApi</c> in the Rust docs</summary>
public enum FerrumApi : uint
{
    Software = 1,
    Legacy = 2,
}

/// <summary>any button the box has, <c>Button.Extra(n)</c> is one past side2, numbered from 5 like
/// <c>Button::Extra(n)</c> in Rust</summary>
public enum Button : uint
{
    Left = 0,
    Right = 1,
    Middle = 2,
    Side1 = 3,
    Side2 = 4,
}

public static class ButtonExtensions
{
    internal const uint ExtraBase = 256;

    extension(Button)
    {
        public static Button Extra(byte number) => (Button)(ExtraBase + number);
    }
}

public enum Axis : uint
{
    X = 0,
    Y = 1,
    Wheel = 2,
    Pan = 3,
}

/// <summary>positive is the way the box moves for a positive value, so right for X, down for Y, up for the wheel
/// and right for pan</summary>
public enum Direction : uint
{
    Both = 0,
    Positive = 1,
    Negative = 2,
}

/// <summary>what a box can be asked to do. it's <c>Action</c> in Rust, the name would clash with
/// <see cref="System.Action"/> here</summary>
public enum BoxAction : uint
{
    Connect = 0,
    Move = 1,
    MoveNow = 2,
    Press = 3,
    Release = 4,
    Click = 5,
    Wheel = 6,
    Pan = 7,
    DiscardPendingMoves = 8,
    ButtonEvents = 9,
    MotionEvents = 10,
    KeyEvents = 11,
    MediaEvents = 12,
    KeyPress = 13,
    KeyRelease = 14,
    KeyTap = 15,
    MediaPress = 16,
    MediaRelease = 17,
    MediaTap = 18,
    LockButton = 19,
    LockAxis = 20,
    LockKey = 21,
    ScaleAxis = 22,
    TypeText = 23,
    KeyChord = 24,
    ForceReleaseKey = 25,
    MakcuInterpolation = 0x10000,
    MakcuMoving = 0x10001,
    MakcuAutorelease = 0x10002,
    YscInterpolation = 0x20000,
    YscSerial = 0x20001,
    FerrumCatchXy = 0x30000,
}

public enum Execution : uint
{
    /// <summary>the box does it, with its own timing</summary>
    Native = 0,
    /// <summary>anybox builds it on the PC out of things the box can do, so it runs on the PC's timing</summary>
    Host = 1,
    Unsupported = 2,
}

public enum QuirkKind : uint
{
    ForcedUpOnRelease = 0,
    PhysicalOverride = 1,
    WholeAxisOnly = 2,
    NotchesPerCommand = 3,
    FixedClickTiming = 4,
    MaxHold = 5,
    BoxClickTiming = 6,
    HostClickTiming = 7,
    KeepsHeld = 8,
    MaxKeysHeld = 9,
    KeepsLocked = 10,
    TypedTextIgnoresCapsLock = 11,
    TypingReleasesHeldKeys = 12,
}

public enum FirmwareKind : uint
{
    Version = 1,
    Build = 2,
    Protocol = 3,
    Text = 4,
}

public enum DropSource : uint
{
    /// <summary>the box's own counters</summary>
    Device = 0,
    /// <summary>your receiver fell behind and its queue overflowed</summary>
    Host = 1,
}

/// <summary>each box clock is its own counter, so only compare stamps from the same one</summary>
public enum Clock : uint
{
    MediusHostChip = 0,
    MediusDeviceChip = 1,
}

public static class ClockExtensions
{
    // the header's ANYBOX_BOX_CLOCK_WRAP_US
    internal const ulong WrapUs = 1UL << 32;

    extension(Clock clock)
    {
        /// <summary>a 32 bit microsecond counter rolls over every 71.6 minutes</summary>
        public TimeSpan WrapsAfter => TimeSpan.FromMicroseconds(WrapUs);
    }
}

/// <summary>what kind of I/O error an <see cref="IoException"/> is, <see cref="Other"/> for every kind this has
/// no number for</summary>
public enum IoKind : uint
{
    Other = 1,
    NotFound = 2,
    PermissionDenied = 3,
    TimedOut = 4,
    BrokenPipe = 5,
    WouldBlock = 6,
    InvalidInput = 7,
    InvalidData = 8,
    UnexpectedEof = 9,
    Interrupted = 10,
    Unsupported = 11,
    OutOfMemory = 12,
    ResourceBusy = 13,
    WriteZero = 14,
    NotConnected = 15,
}

/// <summary>which input a receiver wants, combined with <c>|</c>. connection events and drops reach every
/// receiver</summary>
[Flags]
public enum EventKinds : uint
{
    None = 0,
    Buttons = 1,
    Motion = 2,
    Keys = 4,
    Media = 8,
    /// <summary>everything the box can stream, a kind it can't is left out without a <see cref="Rejected"/></summary>
    All = 15,
}

public enum ClickTimingKind : uint
{
    Makcu = 0,
    Ferrum = 1,
    Fixed = 2,
    Off = 3,
}

internal enum EventKind : uint
{
    Button = 0,
    Key = 1,
    Media = 2,
    Motion = 3,
    Dropped = 4,
    SessionReset = 5,
    Rejected = 6,
    Disconnected = 7,
    Reconnected = 8,
    Changed = 9,
}

internal enum IdKind : uint
{
    None = 0,
    Mac = 1,
    UsbSerial = 2,
}

/// <summary>a keyboard key by its HID usage (page 0x07). <c>(Key)0x64</c> gets you any other one from 0x04 to
/// 0xE7</summary>
public enum Key : byte
{
    A = 0x04,
    B = 0x05,
    C = 0x06,
    D = 0x07,
    E = 0x08,
    F = 0x09,
    G = 0x0A,
    H = 0x0B,
    I = 0x0C,
    J = 0x0D,
    K = 0x0E,
    L = 0x0F,
    M = 0x10,
    N = 0x11,
    O = 0x12,
    P = 0x13,
    Q = 0x14,
    R = 0x15,
    S = 0x16,
    T = 0x17,
    U = 0x18,
    V = 0x19,
    W = 0x1A,
    X = 0x1B,
    Y = 0x1C,
    Z = 0x1D,
    Num1 = 0x1E,
    Num2 = 0x1F,
    Num3 = 0x20,
    Num4 = 0x21,
    Num5 = 0x22,
    Num6 = 0x23,
    Num7 = 0x24,
    Num8 = 0x25,
    Num9 = 0x26,
    Num0 = 0x27,
    Enter = 0x28,
    Escape = 0x29,
    Backspace = 0x2A,
    Tab = 0x2B,
    Space = 0x2C,
    Minus = 0x2D,
    Equal = 0x2E,
    LeftBracket = 0x2F,
    RightBracket = 0x30,
    Backslash = 0x31,
    Semicolon = 0x33,
    Apostrophe = 0x34,
    Grave = 0x35,
    Comma = 0x36,
    Period = 0x37,
    Slash = 0x38,
    CapsLock = 0x39,
    F1 = 0x3A,
    F2 = 0x3B,
    F3 = 0x3C,
    F4 = 0x3D,
    F5 = 0x3E,
    F6 = 0x3F,
    F7 = 0x40,
    F8 = 0x41,
    F9 = 0x42,
    F10 = 0x43,
    F11 = 0x44,
    F12 = 0x45,
    PrintScreen = 0x46,
    ScrollLock = 0x47,
    Pause = 0x48,
    Insert = 0x49,
    Home = 0x4A,
    PageUp = 0x4B,
    Delete = 0x4C,
    End = 0x4D,
    PageDown = 0x4E,
    Right = 0x4F,
    Left = 0x50,
    Down = 0x51,
    Up = 0x52,
    NumLock = 0x53,
    KeypadDivide = 0x54,
    KeypadMultiply = 0x55,
    KeypadMinus = 0x56,
    KeypadPlus = 0x57,
    KeypadEnter = 0x58,
    Keypad1 = 0x59,
    Keypad2 = 0x5A,
    Keypad3 = 0x5B,
    Keypad4 = 0x5C,
    Keypad5 = 0x5D,
    Keypad6 = 0x5E,
    Keypad7 = 0x5F,
    Keypad8 = 0x60,
    Keypad9 = 0x61,
    Keypad0 = 0x62,
    KeypadPeriod = 0x63,
    Menu = 0x65,
    LeftCtrl = 0xE0,
    LeftShift = 0xE1,
    LeftAlt = 0xE2,
    LeftGui = 0xE3,
    RightCtrl = 0xE4,
    RightShift = 0xE5,
    RightAlt = 0xE6,
    RightGui = 0xE7,
}

public static class KeyExtensions
{
    // the header's ANYBOX_KEY_MODIFIER_FIRST and ANYBOX_KEY_MODIFIER_LAST
    internal const byte ModifierFirst = 0xE0;
    internal const byte ModifierLast = 0xE7;

    extension(Key key)
    {
        /// <summary>ctrl, shift, alt and gui on either side. a keyboard report keeps these in a byte of their
        /// own</summary>
        public bool IsModifier => key is >= (Key)ModifierFirst and <= (Key)ModifierLast;
    }
}

/// <summary>a media key by its HID consumer usage (page 0x0C). <c>(MediaKey)n</c> gets you any other one</summary>
public enum MediaKey : ushort
{
    Play = 0xB0,
    Pause = 0xB1,
    NextTrack = 0xB5,
    PreviousTrack = 0xB6,
    Stop = 0xB7,
    PlayPause = 0xCD,
    Mute = 0xE2,
    VolumeUp = 0xE9,
    VolumeDown = 0xEA,
}

/// <summary>each value in words, the same text Rust's Display gives and anybox's errors use</summary>
public static class ValueTexts
{
    extension(Family family)
    {
        public string Text => family switch
        {
            Family.Medius => "Medius",
            Family.MakcuV4 => "MAKCU V4",
            Family.Ferrum => "Ferrum",
            Family.Ysc => "YSC 8K",
            _ => throw new ArgumentOutOfRangeException(nameof(family), family, "anybox has no such family"),
        };
    }

    extension(FerrumApi api)
    {
        public string Text => api switch
        {
            FerrumApi.Software => "the Ferrum app's port",
            FerrumApi.Legacy => "the box's own legacy port",
            _ => throw new ArgumentOutOfRangeException(nameof(api), api, "a Ferrum has no such API"),
        };
    }

    extension(Button button)
    {
        public string Text => button switch
        {
            Button.Left => "left",
            Button.Right => "right",
            Button.Middle => "middle",
            Button.Side1 => "side1",
            Button.Side2 => "side2",
            _ => $"extra {(uint)button - ButtonExtensions.ExtraBase}",
        };
    }

    extension(Axis axis)
    {
        public string Text => axis switch
        {
            Axis.X => "X",
            Axis.Y => "Y",
            Axis.Wheel => "wheel",
            Axis.Pan => "pan",
            _ => throw new ArgumentOutOfRangeException(nameof(axis), axis, "a box has no such axis"),
        };
    }

    extension(BoxAction action)
    {
        public string Text => action switch
        {
            BoxAction.Connect => "connect",
            BoxAction.Move => "move",
            BoxAction.MoveNow => "move now",
            BoxAction.Press => "press",
            BoxAction.Release => "release",
            BoxAction.Click => "click",
            BoxAction.Wheel => "scroll",
            BoxAction.Pan => "scroll sideways",
            BoxAction.DiscardPendingMoves => "discard pending moves",
            BoxAction.ButtonEvents => "report button events",
            BoxAction.MotionEvents => "report motion events",
            BoxAction.KeyEvents => "report key events",
            BoxAction.MediaEvents => "report media key events",
            BoxAction.KeyPress => "press a key",
            BoxAction.KeyRelease => "release a key",
            BoxAction.KeyTap => "tap a key",
            BoxAction.MediaPress => "press a media key",
            BoxAction.MediaRelease => "release a media key",
            BoxAction.MediaTap => "tap a media key",
            BoxAction.LockButton => "lock or unlock a button",
            BoxAction.LockAxis => "lock or unlock an axis",
            BoxAction.LockKey => "lock or unlock a key",
            BoxAction.ScaleAxis => "scale an axis",
            BoxAction.TypeText => "type text",
            BoxAction.KeyChord => "press several keys at once",
            BoxAction.ForceReleaseKey => "force a key up or give it back",
            BoxAction.MakcuInterpolation => "read or set the MAKCU's interpolation",
            BoxAction.MakcuMoving => "ask the MAKCU whether it's still moving",
            BoxAction.MakcuAutorelease => "read or set the MAKCU's autorelease",
            BoxAction.YscInterpolation => "read or set the YSC's interpolation",
            BoxAction.YscSerial => "read the serial the YSC gives its mouse",
            BoxAction.FerrumCatchXy => "read the Ferrum's recent mouse motion",
            _ => throw new ArgumentOutOfRangeException(nameof(action), action, "anybox has no such action"),
        };
    }

    extension(Key key)
    {
        public string Text => $"key 0x{(byte)key:X2}";
    }

    extension(MediaKey key)
    {
        public string Text => $"media key 0x{(ushort)key:X4}";
    }
}

/// <summary>what a click and the taps do on a box that can't time them itself. <see cref="Makcu"/> (a random hold
/// from 35 to 75 ms) is the default, <see cref="Ferrum"/> holds 75 to 125 ms, <see cref="Fixed"/> always the same,
/// and with <see cref="Off"/> a click or tap without a hold fails on a box like that</summary>
public sealed record ClickTiming
{
    ClickTiming(ClickTimingKind kind, TimeSpan? hold)
    {
        Kind = kind;
        Hold = hold;
    }

    public ClickTimingKind Kind { get; }

    /// <summary>only set on a fixed one</summary>
    public TimeSpan? Hold { get; }

    public static ClickTiming Makcu { get; } = new(ClickTimingKind.Makcu, null);
    public static ClickTiming Ferrum { get; } = new(ClickTimingKind.Ferrum, null);
    public static ClickTiming Off { get; } = new(ClickTimingKind.Off, null);

    public static ClickTiming Fixed(TimeSpan hold) => hold < TimeSpan.Zero
        ? throw new ArgumentOutOfRangeException(nameof(hold), "a fixed click timing can't be negative")
        : new(ClickTimingKind.Fixed, hold);

    public override string ToString() => Hold is { } hold ? $"Fixed {hold.TotalMilliseconds} ms" : Kind.ToString();

    internal AnyboxClickTiming ToNative() => new()
    {
        kind = (uint)Kind,
        fixed_us = Hold is { } hold ? Read.Micros(hold, "a fixed click timing") : 0,
    };

    internal static ClickTiming From(in AnyboxClickTiming raw) => Read.Known<ClickTimingKind>(raw.kind, "a click timing") switch
    {
        ClickTimingKind.Makcu => Makcu,
        ClickTimingKind.Ferrum => Ferrum,
        ClickTimingKind.Off => Off,
        _ => Fixed(TimeSpan.FromMicroseconds((long)raw.fixed_us)),
    };
}

/// <summary>a box's own id, which stays the same whatever port it's on. it's always exactly one of the two</summary>
public sealed record StableId
{
    StableId(PhysicalAddress? mac, string? usbSerial)
    {
        Mac = mac;
        UsbSerial = usbSerial;
    }

    public PhysicalAddress? Mac { get; }
    public string? UsbSerial { get; }

    public static StableId FromMac(ReadOnlySpan<byte> mac) => mac.Length == 6
        ? new(new PhysicalAddress(mac.ToArray()), null)
        : throw new ArgumentException($"a MAC is 6 bytes, not {mac.Length}", nameof(mac));

    public static StableId FromUsbSerial(string serial)
    {
        ArgumentNullException.ThrowIfNull(serial);
        return new(null, serial);
    }

    public override string ToString() => Mac is { } mac
        ? "MAC " + string.Join(':', mac.GetAddressBytes().Select(part => part.ToString("x2")))
        : $"USB serial {UsbSerial}";
}

/// <summary><see cref="Version"/> is major.minor.patch on a box that has one, otherwise <see cref="Build"/> or
/// <see cref="Text"/> says it. <see cref="Protocol"/> only shows up in the error of a Medius that's too old or too
/// new, never in <see cref="Info"/></summary>
public sealed record Firmware(FirmwareKind Kind)
{
    public Version? Version { get; init; }
    public uint? Build { get; init; }
    public byte? Protocol { get; init; }
    public string? Text { get; init; }

    public override string ToString() => Kind switch
    {
        FirmwareKind.Version => Version?.ToString() ?? "",
        FirmwareKind.Build => $"build {Build}",
        FirmwareKind.Protocol => $"protocol {Protocol}",
        _ => Text ?? "",
    };
}

public sealed record UsbIds(ushort Vid, ushort Pid);

/// <summary>who the box is, see <c>Info</c> in the Rust docs for what each one means</summary>
public sealed record Info(
    string Port,
    Family Family,
    string? Model,
    Firmware? Firmware,
    StableId? Id,
    FerrumApi? FerrumApi,
    UsbIds? Peripheral);

/// <summary>one way the box behaves differently, <see cref="Kind"/> says which, and only the values that kind has
/// are set</summary>
public sealed record Quirk(QuirkKind Kind)
{
    public TimeSpan? Min { get; init; }
    public TimeSpan? Max { get; init; }
    public uint? Count { get; init; }
    public ClickTiming? ClickTiming { get; init; }
}

/// <summary><see cref="Reason"/> says why the box can't, only when it's unsupported</summary>
public sealed record Capability(Execution Execution, string? Reason, IReadOnlyList<Quirk> Quirks)
{
    public bool Supported => Execution != Execution.Unsupported;
}

/// <summary>what this box, on this firmware, with this mouse behind it, can do for every action.
/// <see cref="Axes"/> are the ones you can lock or scale on this box</summary>
public sealed class Capabilities(
    IDictionary<BoxAction, Capability> actions,
    IReadOnlyList<Button> buttons,
    IReadOnlyList<Axis> axes) : ReadOnlyDictionary<BoxAction, Capability>(actions)
{
    public IReadOnlyList<Button> Buttons => buttons;
    public IReadOnlyList<Axis> Axes => axes;
}

/// <summary>who the box is and what it can do, both read from the same connection</summary>
public sealed record Snapshot(Info Info, Capabilities Capabilities);

/// <summary>one port a scan looked at, with the box on it in <see cref="Found"/>, or in <see cref="Error"/> why a
/// search of just that port fails. mostly that's busy (the box you already have open shows up as busy too), not
/// allowed, too old, a <see cref="NoBoxCanException"/> for a box that can't do what <c>Can</c> asked, or a
/// <see cref="NotFoundException"/> when nothing answered</summary>
public sealed record Scanned(string Port, Snapshot? Found, AnyboxException? Error);

public sealed record BoxTime(uint Micros, Clock Clock);

/// <summary><see cref="SinceOpen"/> is the best guess at when it happened, as the time since the device was
/// opened, and the one to compare events by. <see cref="ReceivedSinceOpen"/> is when anybox got it, and
/// <see cref="ReceivedQpc"/> the raw QueryPerformanceCounter value then, on Windows only. <see cref="ArrivedNs"/>
/// and <see cref="ReceivedNs"/> are the same two moments in nanoseconds on the clock <see cref="Monotonic"/> reads,
/// so they line up with other devices and the rest of your program</summary>
public sealed record Stamp(
    TimeSpan SinceOpen,
    TimeSpan ReceivedSinceOpen,
    long ArrivedNs,
    long ReceivedNs,
    long? ReceivedQpc,
    BoxTime? BoxTime);

/// <summary>the OS monotonic clock every stamp's <see cref="Stamp.ArrivedNs"/> and <see cref="Stamp.ReceivedNs"/>
/// are on, the one <see cref="System.Diagnostics.Stopwatch"/> reads on Windows and Linux (QueryPerformanceCounter
/// and CLOCK_MONOTONIC, CLOCK_UPTIME_RAW on macOS). it only compares on the same machine within one boot</summary>
public static class Monotonic
{
    const long NanosPerSecond = 1_000_000_000;

    /// <summary>now, in nanoseconds</summary>
    public static long NowNs
    {
        get
        {
            Library.Ensure();
            return (long)Imports.anybox_monotonic_ns();
        }
    }

    /// <summary>a <see cref="System.Diagnostics.Stopwatch.GetTimestamp"/> value in nanoseconds, rounded down like
    /// anybox does it</summary>
    public static long FromStopwatchTicks(long ticks) =>
        (long)((Int128)ticks * NanosPerSecond / System.Diagnostics.Stopwatch.Frequency);

    /// <summary>nanoseconds as a <see cref="System.Diagnostics.Stopwatch.GetTimestamp"/> value, rounded
    /// down</summary>
    public static long ToStopwatchTicks(long nanos) =>
        (long)((Int128)nanos * System.Diagnostics.Stopwatch.Frequency / NanosPerSecond);
}

/// <summary>one event from a box, match on the kind with a switch</summary>
public abstract record Event;

public sealed record ButtonEvent(Button Button, bool Pressed, Stamp At) : Event;

/// <summary>a physical key on the keyboard behind the box. just know that you see every key typed there,
/// passwords included</summary>
public sealed record KeyEvent(Key Key, bool Pressed, Stamp At) : Event;

public sealed record MediaEvent(MediaKey Key, bool Pressed, Stamp At) : Event;

/// <summary>physical motion. the wheel is positive going up and pan is positive going right</summary>
public sealed record MotionEvent(int Dx, int Dy, int Wheel, int Pan, Stamp At) : Event;

/// <summary>how many events never reached you since the last one from the same source, see
/// <c>Event::Dropped</c> in the Rust docs</summary>
public sealed record Dropped(ulong Count, DropSource Source) : Event;

/// <summary>the box lost its state, so the buttons and keys you were holding got released, unless the box has
/// <see cref="QuirkKind.KeepsHeld"/>. what you knew about which physical ones were held is out of date too</summary>
public sealed record SessionReset : Event;

/// <summary>the box refused <see cref="Count"/> of <see cref="Action"/> after the call already returned, so slow
/// down and send it again if it mattered</summary>
public sealed record Rejected(BoxAction Action, ulong Count) : Event;

public sealed record Disconnected : Event;

/// <summary>the box is back, what you held got released (unless it has <see cref="QuirkKind.KeepsHeld"/>), and
/// the snapshot may have changed</summary>
public sealed record Reconnected : Event;

/// <summary>the snapshot changed with no reconnect, so read it again</summary>
public sealed record Changed : Event;
