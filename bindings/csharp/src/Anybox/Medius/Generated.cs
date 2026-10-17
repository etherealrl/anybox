// made by bindings/csharp/tools/gen-medius.cs from bindings/c/include/medius.h, don't edit it by hand
#pragma warning disable CS1591
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace Anybox.Medius;

public static class Constants
{
    public const int MaxUsages = 256;
    public const int MaxLocks = 256;
    public const int MaxLogText = 512;
    public const int MaxPath = 512;
    public const int MaxProduct = 128;
    public const int MaxName = 33;
    public const int MaxSerial = 128;
    public const int MaxCatchEntries = 32;
    public const int MaxTrafficBytes = 180;
    public const int MaxRewriteEntries = 32;
    public const int MaxPatchEntries = 16;
    public const int MaxTransformEntries = 32;
    public const int MaxRewriteMatch = 16;
    public const int RewritePayloadPool = 2048;
    public const int MaxDevPayload = 512;
    public const int CatchClassBtn = 0;
    public const int CatchClassKey = 1;
    public const int CatchClassMedia = 2;
    public const int CatchClassAxis = 3;
    public const int CatchClassHidIn = 4;
    public const int CatchClassHidOut = 5;
    public const int CatchClassVendorInterrupt = 6;
    public const int CatchClassVendorBulk = 7;
    public const int CatchClassControl = 8;
    public const int CatchClassEmit = 9;
    public const int CatchClassBus = 10;
    public const int CatchClassClipTransfer = 11;
    public const int CatchClassAny = 255;
    public const int CatchIdAny = 65535;
    public const uint ClockAgeNone = uint.MaxValue;
    public const int ClockRateNone = int.MinValue;
    public const int LockScaleBlock = 0;
    public const int LockScalePass = 100;
    public const int LockScaleMax = 255;
    public const int LockScaleMin = -255;
    public const int BearingWindowDefaultMs = 20;
    public const int ClipTrigMax = 8;
    public const int ClipPktTrigMax = 8;
    public const int ClipPktMatchPool = 112;
    public const int MaxPktMatch = 16;
    public const int ClipEdgesMax = 8;
    public const int ClipRawMax = 8;
    public const int ClipEntryMax = 512;
    public const int AbiVersion = 10;
}

public enum Status : int
{
    Ok = 0,
    ErrIo = 1,
    ErrNotFound = 2,
    ErrNoReply = 3,
    ErrBadProtoVer = 4,
    ErrQueryTimeout = 5,
    ErrDisconnected = 6,
    ErrFrameTooLong = 7,
    ErrUpdate = 8,
    ErrInvalidArg = 9,
    ErrPanic = 10,
    ErrUnknown = 11,
    ErrCatchTableFull = 12,
    ErrEmptySubscription = 13,
    ErrCaptureNotApplicable = 14,
    ErrNotAnInputFilter = 15,
    ErrWildcardNotInput = 16,
    ErrHalfEdgeInputFilter = 17,
    ErrReservedId = 18,
    ErrRelativeDirection = 19,
    ErrLockScaleRange = 20,
    ErrLockScaleUsage = 21,
    ErrImperfectRequired = 22,
    ErrRewriteMaskLength = 23,
    ErrRewriteActionClass = 24,
    ErrRewritePayloadTooLarge = 25,
    ErrRewriteTableFull = 26,
    ErrTransformOpFields = 27,
    ErrTransformTableFull = 28,
    ErrRawDirection = 29,
    ErrClipFrameCount = 30,
    ErrClipFrameTooLong = 31,
    ErrClipTransferData = 32,
    ErrClipPacketTrigger = 33,
    ErrRewriteMatchTooLong = 34,
    ErrRewritePoolFull = 35,
}

public enum EmitMode : byte
{
    Learned = 0,
    Interval = 1,
    Fixed = 2,
}

public enum BearingMode : byte
{
    PerAxis = 0,
    Vector = 1,
}

public enum RenderMode : byte
{
    Off = 0,
    Stock = 1,
    Despiked = 2,
    Unsmoothed = 3,
}

public enum ControlStatus : byte
{
    Ok = 0,
    Stalled = 1,
    Naked = 2,
    Other = 3,
}

public enum BusEventKind : byte
{
    Reset = 0,
    Suspend = 1,
    Resume = 2,
    Configured = 3,
    Deconfigured = 4,
    SetInterface = 5,
    DeviceAttached = 6,
    DeviceDetached = 7,
    CloneUp = 8,
    CloneDown = 9,
}

public enum CatchEventKind : byte
{
    Motion = 0,
    Usages = 1,
    Traffic = 2,
}

public enum ClockDomain : byte
{
    HostChip = 0,
    DeviceChip = 1,
}

public enum InputKind : byte
{
    Press = 0,
    Release = 1,
    Motion = 2,
}

public enum LogLevel : byte
{
    Error = 0,
    Warn = 1,
    Info = 2,
    Debug = 3,
    Verbose = 4,
}

public enum FrameType : byte
{
    Move = 1,
    Inject = 3,
    Reset = 4,
    Query = 5,
    Resp = 6,
    RebootDl = 7,
    Log = 8,
    Led = 9,
    Lock = 10,
    Catch = 11,
    MotionEvent = 12,
    UsageEvent = 15,
    TrafficEvent = 22,
    Option = 17,
    ClipAppend = 18,
    ClipCtrl = 19,
    ClipSet = 20,
    ClipTrigger = 21,
    Update = 23,
    UpdateResp = 24,
    Raw = 25,
    Transfer = 26,
    TransferResp = 27,
    Rewrite = 28,
    Patch = 29,
    Transform = 30,
}

public enum InjectAction : byte
{
    SoftRelease = 0,
    Press = 1,
    ForceRelease = 2,
}

public enum Axis : byte
{
    X = 0,
    Y = 1,
    Wheel = 2,
    Pan = 3,
}

public enum Blanket : byte
{
    Aim = 0,
    Wheel = 1,
    Buttons = 2,
    Keys = 3,
    Media = 4,
}

public enum Button : byte
{
    Left = 0,
    Right = 1,
    Middle = 2,
    Side1 = 3,
    Side2 = 4,
}

public enum Class : byte
{
    Button = 0,
    Key = 1,
    Media = 2,
}

public enum ClipAction : byte
{
    Start = 0,
    Stop = 1,
    Pause = 2,
    Resume = 3,
    Restart = 4,
    Toggle = 5,
}

public enum ClipState : byte
{
    Idle = 0,
    Playing = 1,
    Paused = 2,
    Faulted = 3,
}

public enum DeviceKind : byte
{
    Unknown = 0,
    Keyboard = 1,
    Mouse = 2,
}

public enum Direction : byte
{
    Both = 0,
    Positive = 1,
    Negative = 2,
    With = 3,
    Against = 4,
    Press = Positive,
    Release = Negative,
    In = Positive,
    Out = Negative,
}

public enum Edge : byte
{
    Both = 0,
    Press = 1,
    Release = 2,
}

public enum LedMode : byte
{
    Auto = 0,
    Off = 1,
    Solid = 2,
    Blink = 3,
}

public enum LedTarget : byte
{
    Device = 0,
    Host = 1,
    Both = 2,
}

public enum LockTargetKind : byte
{
    X = 0,
    Y = 1,
    Wheel = 2,
    Pan = 3,
    Usage = 4,
}

public enum MotionKind : byte
{
    Cursor = 0,
    Wheel = 1,
    Pan = 2,
}

public enum MoveTiming : byte
{
    Ride = 0,
    Now = 1,
}

public enum PatchSection : byte
{
    Device = 0,
    Config = 1,
    Report = 2,
    String = 3,
    Bos = 4,
}

public enum PendingMotion : byte
{
    Keep = 0,
    Flush = 1,
    Discard = 2,
}

public enum RebootTarget : byte
{
    DeviceDownload = 0,
    HostDownload = 1,
    DeviceRun = 2,
    HostRun = 3,
}

public enum RewriteAction : byte
{
    Pass = 0,
    Drop = 1,
    Patch = 2,
    Replace = 3,
    Answer = 4,
    Stall = 5,
    Nak = 6,
    ReplyPatch = 7,
    ReplyReplace = 8,
}

public enum RewriteClass : byte
{
    HidIn = 4,
    HidOut = 5,
    VendorInterrupt = 6,
    VendorBulk = 7,
    Control = 8,
    Emit = 9,
    Any = 255,
}

public enum TransferStatus : byte
{
    Ok = 0,
    Refused = 252,
    Stall = 253,
    Nak = 254,
    NoDevice = 255,
}

public enum TransformOp : byte
{
    Remap = 0,
    Swap = 1,
}

public enum Key : byte
{
    A = 4,
    B = 5,
    C = 6,
    D = 7,
    E = 8,
    F = 9,
    G = 10,
    H = 11,
    I = 12,
    J = 13,
    K = 14,
    L = 15,
    M = 16,
    N = 17,
    O = 18,
    P = 19,
    Q = 20,
    R = 21,
    S = 22,
    T = 23,
    U = 24,
    V = 25,
    W = 26,
    X = 27,
    Y = 28,
    Z = 29,
    N1 = 30,
    N2 = 31,
    N3 = 32,
    N4 = 33,
    N5 = 34,
    N6 = 35,
    N7 = 36,
    N8 = 37,
    N9 = 38,
    N0 = 39,
    Enter = 40,
    Escape = 41,
    Backspace = 42,
    Tab = 43,
    Space = 44,
    CapsLock = 57,
    Insert = 73,
    Home = 74,
    PageUp = 75,
    Delete = 76,
    End = 77,
    PageDown = 78,
    Right = 79,
    Left = 80,
    Down = 81,
    Up = 82,
    F1 = 58,
    F2 = 59,
    F3 = 60,
    F4 = 61,
    F5 = 62,
    F6 = 63,
    F7 = 64,
    F8 = 65,
    F9 = 66,
    F10 = 67,
    F11 = 68,
    F12 = 69,
    LeftCtrl = 224,
    LeftShift = 225,
    LeftAlt = 226,
    LeftGui = 227,
    RightCtrl = 228,
    RightShift = 229,
    RightAlt = 230,
    RightGui = 231,
}

public enum MediaKey : ushort
{
    PlayPause = 205,
    Stop = 183,
    NextTrack = 181,
    PrevTrack = 182,
    Mute = 226,
    VolumeUp = 233,
    VolumeDown = 234,
    Play = 176,
    Pause = 177,
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusUsage
{
    public byte kind;
    public ushort id;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusSetup
{
    public byte request_type;
    public byte request;
    public ushort value;
    public ushort index;
    public ushort length;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusClipTrigger
{
    public MediusUsage on;
    public byte edge;
    public byte action;
    public byte consume;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusClipPacketTrigger
{
    public byte class_;
    public ushort id;
    public byte direction;
    public byte action;
    public byte consume;
    public byte once_per_run;
    public byte selector_len;
    public ushort match_len;
    public ushort mask_len;
    public fixed byte match_bytes[16];
    public fixed byte mask[16];
    public ushort hits;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusClipStatus
{
    public byte state;
    public uint free;
    public uint total;
    public uint played;
    public uint ticks;
    public ushort underruns;
    public ushort overruns;
    public ushort seq_gaps;
    public ushort xfers;
    public ushort xfer_errs;
    public ushort gated;
    public ushort held_n;
    public MediusUsageArray256 held;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusClipSettings
{
    public byte autolock_bits;
    public byte loop_;
    public byte retain;
    public byte finalized;
    public byte ride;
    public MediusClipTriggerArray8 triggers;
    public byte n;
    public MediusClipPacketTriggerArray8 packet_triggers;
    public byte packet_n;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusPortInfo
{
    public fixed byte path[512];
    public ushort vid;
    public ushort pid;
    public fixed byte serial[128];
    public byte has_serial;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusVersion
{
    public byte proto_ver;
    public byte fw_major;
    public byte fw_minor;
    public byte fw_patch;
    public fixed byte mac[6];
    public fixed byte name[33];
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusDeviceInfo
{
    public ushort vid;
    public ushort pid;
    public ushort bcd_device;
    public ushort bcd_usb;
    public byte has_serial;
    public byte has_bos;
    public byte kind;
    public fixed byte product[128];
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusBoxInfo
{
    public MediusPortInfo port;
    public MediusVersion version;
    public MediusDeviceInfo device;
    public byte has_device;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusMotion
{
    public byte kind;
    public short dx;
    public short dy;
    public short wheel;
    public short pan;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusLockTarget
{
    public byte kind;
    public MediusUsage usage;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusTransferOutcome
{
    public byte status;
    public ushort len;
    public fixed byte data[512];
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusRewriteRule
{
    public byte class_;
    public ushort id;
    public byte direction;
    public byte action;
    public ushort offset;
    public ushort match_len;
    public ushort mask_len;
    public ushort payload_len;
    public fixed byte match_bytes[16];
    public fixed byte mask[16];
    public fixed byte payload[512];
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusRewriteEntry
{
    public byte class_;
    public ushort id;
    public byte direction;
    public byte action;
    public byte match_len;
    public ushort offset;
    public ushort payload_len;
    public ushort hits;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusRewriteTable
{
    public byte table_full;
    public byte generation;
    public ushort n;
    public MediusRewriteEntryArray32 entries;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusPatch
{
    public byte section;
    public byte cfg;
    public byte index;
    public ushort offset;
    public ushort len;
    public fixed byte bytes[512];
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusPatchEntry
{
    public byte section;
    public byte cfg;
    public byte index;
    public ushort offset;
    public ushort len;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusPatchSet
{
    public byte applied;
    public byte pending;
    public byte refused;
    public byte table_full;
    public ushort n;
    public MediusPatchEntryArray16 entries;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusTransform
{
    public byte op;
    public MediusLockTarget source;
    public MediusLockTarget dest;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusTransforms
{
    public byte table_full;
    public ushort n;
    public MediusTransformArray32 entries;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusChipFirmware
{
    public byte major;
    public byte minor;
    public byte patch;
    public byte slot;
    public byte state;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusFirmwareInfo
{
    public MediusChipFirmware device;
    public byte host_present;
    public MediusChipFirmware host;
    public uint slot_size;
    public byte device_staged;
    public byte host_staged;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusHealth
{
    public byte link_up;
    public byte mouse_attached;
    public byte clone_configured;
    public byte injection_active;
    public byte rate_confident;
    public byte lock_on;
    public byte catch_on;
    public byte kbd_attached;
    public byte rewrite_on;
    public byte patch_on;
    public byte transform_on;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusMouseCaps
{
    public byte n_buttons;
    public byte has_x;
    public byte has_y;
    public byte has_wheel;
    public byte pan;
    public byte has_report_id;
    public byte n_hid;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusKbdCaps
{
    public byte n_keys;
    public byte nkro;
    public byte has_consumer;
    public byte has_system;
    public byte has_report_id;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusCaps
{
    public MediusMouseCaps mouse;
    public MediusKbdCaps keyboard;
    public byte mouse_change_driven;
    public byte kbd_change_driven;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusRate
{
    public ushort native_period_us;
    public ushort poll_period_us;
    public byte confident;
    public byte change_driven;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusStats
{
    public uint inject_emits;
    public ushort tx_drops;
    public ushort tx_merges;
    public byte tx_maxdepth;
    public byte tx_wedges;
    public ushort wakeups;
    public ushort reset_count;
    public ushort config_count;
    public uint link_rx_drops;
    public uint host_rx_drops;
    public uint relay_drops;
    public ushort session;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusLockEntry
{
    public MediusLockTarget target;
    public byte is_blanket;
    public byte direction;
    public short scale;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusLocks
{
    public ushort n;
    public MediusLockEntryArray256 entries;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusClockEstimate
{
    public int offset_us;
    public int rate_ppb;
    public ushort delay_us;
    public uint age_ms;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusCatchFilter
{
    public byte class_;
    public ushort id;
    public byte direction;
    public byte capture;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusCatchEntry
{
    public MediusCatchFilter filter;
    public ushort dropped;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusCatchState
{
    public byte table_full;
    public uint dropped;
    public MediusClockEstimate clock;
    public ushort n;
    public MediusCatchEntryArray32 entries;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusImperfectStatus
{
    public byte allowed;
    public byte over_capacity;
    public byte clone_imperfect;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusEmitPaceStatus
{
    public EmitMode mode;
    public ushort fixed_hz;
    public ushort resolved_hz;
    public ushort force_hz;
    public ushort advertised_hz;
    public byte force_active;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusBearing
{
    public ushort window_ms;
    public BearingMode mode;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusRenderStatus
{
    public RenderMode mode;
    public byte full;
    public byte ready;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusSpreadStatus
{
    public ushort percent;
    public uint span_us;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusCountersSnapshot
{
    public ulong frames_tx;
    public ulong frames_rx;
    public ulong crc_drops;
    public ulong reconnects;
    public ulong restarts;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusUpdateError
{
    public byte op;
    public byte status;
    public uint arg;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusUsageEvent
{
    public byte class_;
    public byte direction;
    public ushort n;
    public MediusUsageArray256 usages;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusTrafficEvent
{
    public byte class_;
    public ushort id;
    public byte direction;
    public byte flags;
    public ushort true_len;
    public ushort len;
    public fixed byte bytes[180];
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusBusEvent
{
    public BusEventKind kind;
    public byte configuration;
    public byte @interface;
    public byte alt;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusMotionEvent
{
    public short dx;
    public short dy;
    public short dz;
    public short pan;
}

[StructLayout(LayoutKind.Explicit)]
internal unsafe struct MediusCatchEventData
{
    [FieldOffset(0)] public MediusMotionEvent motion;
    [FieldOffset(0)] public MediusUsageEvent usages;
    [FieldOffset(0)] public MediusTrafficEvent traffic;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusCatchEvent
{
    public CatchEventKind kind;
    public uint ts_us;
    public ClockDomain clock;
    public MediusCatchEventData data;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusInputEvent
{
    public InputKind kind;
    public uint ts_us;
    public ClockDomain clock;
    public MediusUsage usage;
    public short dx;
    public short dy;
    public short dz;
    public short pan;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusStamped
{
    public ulong host_ns;
    public ulong box_us;
    public ulong excess_ns;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct MediusLogLine
{
    public LogLevel level;
    public fixed byte text[512];
}

[InlineArray(32)]
internal struct MediusCatchEntryArray32
{
    public MediusCatchEntry first;
}

[InlineArray(8)]
internal struct MediusClipPacketTriggerArray8
{
    public MediusClipPacketTrigger first;
}

[InlineArray(8)]
internal struct MediusClipTriggerArray8
{
    public MediusClipTrigger first;
}

[InlineArray(256)]
internal struct MediusLockEntryArray256
{
    public MediusLockEntry first;
}

[InlineArray(16)]
internal struct MediusPatchEntryArray16
{
    public MediusPatchEntry first;
}

[InlineArray(32)]
internal struct MediusRewriteEntryArray32
{
    public MediusRewriteEntry first;
}

[InlineArray(32)]
internal struct MediusTransformArray32
{
    public MediusTransform first;
}

[InlineArray(256)]
internal struct MediusUsageArray256
{
    public MediusUsage first;
}

internal sealed class MediusClipHandle() : SafeHandle(0, ownsHandle: true)
{
    public override bool IsInvalid => handle == 0;

    protected override bool ReleaseHandle()
    {
        MediusImports.medius_clip_free(handle);
        return true;
    }
}

internal sealed class MediusClipBuilderHandle() : SafeHandle(0, ownsHandle: true)
{
    public override bool IsInvalid => handle == 0;

    protected override bool ReleaseHandle()
    {
        MediusImports.medius_clip_builder_free(handle);
        return true;
    }
}

internal sealed class MediusClipFrameHandle() : SafeHandle(0, ownsHandle: true)
{
    public override bool IsInvalid => handle == 0;

    protected override bool ReleaseHandle()
    {
        MediusImports.medius_clip_frame_free(handle);
        return true;
    }
}

internal sealed class MediusDeviceHandle() : SafeHandle(0, ownsHandle: true)
{
    public override bool IsInvalid => handle == 0;

    protected override bool ReleaseHandle()
    {
        MediusImports.medius_device_free(handle);
        return true;
    }
}

internal sealed class MediusEventStreamHandle() : SafeHandle(0, ownsHandle: true)
{
    public override bool IsInvalid => handle == 0;

    protected override bool ReleaseHandle()
    {
        MediusImports.medius_event_stream_free(handle);
        return true;
    }
}

internal sealed class MediusInputStreamHandle() : SafeHandle(0, ownsHandle: true)
{
    public override bool IsInvalid => handle == 0;

    protected override bool ReleaseHandle()
    {
        MediusImports.medius_input_stream_free(handle);
        return true;
    }
}

internal sealed class MediusLogStreamHandle() : SafeHandle(0, ownsHandle: true)
{
    public override bool IsInvalid => handle == 0;

    protected override bool ReleaseHandle()
    {
        MediusImports.medius_log_stream_free(handle);
        return true;
    }
}

internal sealed class MediusMockBoxHandle() : SafeHandle(0, ownsHandle: true)
{
    public override bool IsInvalid => handle == 0;

    protected override bool ReleaseHandle()
    {
        MediusImports.medius_mock_free(handle);
        return true;
    }
}

internal sealed class MediusTimelineHandle() : SafeHandle(0, ownsHandle: true)
{
    public override bool IsInvalid => handle == 0;

    protected override bool ReleaseHandle()
    {
        MediusImports.medius_timeline_free(handle);
        return true;
    }
}

internal static unsafe partial class MediusImports
{
    const string Library = "anybox";

    [LibraryImport(Library)]
    internal static partial MediusClipFrameHandle medius_clip_frame_new();

    [LibraryImport(Library)]
    internal static partial void medius_clip_frame_free(nint f);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_frame_clear(MediusClipFrameHandle f);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_frame_move(MediusClipFrameHandle f, short dx, short dy);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_frame_wheel(MediusClipFrameHandle f, short dz);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_frame_pan(MediusClipFrameHandle f, short dpan);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_frame_edge(MediusClipFrameHandle f, MediusUsage usage, byte action);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_frame_press(MediusClipFrameHandle f, MediusUsage usage);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_frame_release(MediusClipFrameHandle f, MediusUsage usage);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_frame_force_release(MediusClipFrameHandle f, MediusUsage usage);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_frame_raw(MediusClipFrameHandle f, byte ep_num, byte dir, byte* bytes, nuint len);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_frame_transfer(MediusClipFrameHandle f, byte ep, MediusSetup setup, byte* out_data, nuint out_len);

    [LibraryImport(Library)]
    internal static partial nuint medius_clip_frame_byte_len(MediusClipFrameHandle f);

    [LibraryImport(Library)]
    internal static partial MediusClipBuilderHandle medius_clip_builder_new();

    [LibraryImport(Library)]
    internal static partial void medius_clip_builder_free(nint b);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_builder_clear(MediusClipBuilderHandle b);

    [LibraryImport(Library)]
    internal static partial nuint medius_clip_builder_byte_len(MediusClipBuilderHandle b);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_builder_gap(MediusClipBuilderHandle b, ushort frames);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_builder_move(MediusClipBuilderHandle b, short dx, short dy);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_builder_wheel(MediusClipBuilderHandle b, short dz);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_builder_pan(MediusClipBuilderHandle b, short dpan);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_builder_press(MediusClipBuilderHandle b, MediusUsage usage);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_builder_release(MediusClipBuilderHandle b, MediusUsage usage);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_builder_force_release(MediusClipBuilderHandle b, MediusUsage usage);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_builder_edge(MediusClipBuilderHandle b, MediusUsage usage, byte action);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_builder_raw(MediusClipBuilderHandle b, byte ep_num, byte dir, byte* bytes, nuint len);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_builder_transfer(MediusClipBuilderHandle b, byte ep, MediusSetup setup, byte* out_data, nuint out_len);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_builder_frame(MediusClipBuilderHandle b, MediusClipFrameHandle frame);

    [LibraryImport(Library)]
    internal static partial Status medius_device_clip(MediusDeviceHandle dev, out MediusClipHandle @out);

    [LibraryImport(Library)]
    internal static partial void medius_clip_free(nint clip);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_append(MediusClipHandle clip, MediusClipBuilderHandle builder);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_set_autolock(MediusClipHandle clip, byte* scope, nuint scope_len);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_set_loop(MediusClipHandle clip, byte on);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_set_retain(MediusClipHandle clip, byte on);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_set_ride(MediusClipHandle clip, byte on);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_bind(MediusClipHandle clip, MediusClipTrigger trigger);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_unbind(MediusClipHandle clip, MediusUsage usage, byte edge);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_bind_packet(MediusClipHandle clip, MediusClipPacketTrigger* trigger);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_unbind_packet(MediusClipHandle clip, MediusClipPacketTrigger* trigger);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_clear_triggers(MediusClipHandle clip);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_start(MediusClipHandle clip);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_stop(MediusClipHandle clip);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_pause(MediusClipHandle clip);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_resume(MediusClipHandle clip);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_restart(MediusClipHandle clip);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_toggle(MediusClipHandle clip);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_clear(MediusClipHandle clip);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_finalize(MediusClipHandle clip);

    [LibraryImport(Library)]
    internal static partial byte medius_clip_lost(MediusClipHandle clip);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_query_status(MediusClipHandle clip, MediusClipStatus* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_clip_query_config(MediusClipHandle clip, MediusClipSettings* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_open(byte* path, out MediusDeviceHandle @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_find(out MediusDeviceHandle @out);

    [LibraryImport(Library)]
    internal static partial MediusDeviceHandle medius_device_clone(MediusDeviceHandle dev);

    [LibraryImport(Library)]
    internal static partial void medius_device_free(nint dev);

    [LibraryImport(Library)]
    internal static partial nuint medius_find_ports(MediusPortInfo* @out, nuint cap, nuint* out_total);

    [LibraryImport(Library)]
    internal static partial nuint medius_list(MediusBoxInfo* @out, nuint cap, nuint* out_total);

    [LibraryImport(Library)]
    internal static partial Status medius_device_open_by_id(byte* id, out MediusDeviceHandle @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_find_mouse_box(out MediusDeviceHandle @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_find_keyboard_box(out MediusDeviceHandle @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_move_rel(MediusDeviceHandle dev, short dx, short dy);

    [LibraryImport(Library)]
    internal static partial Status medius_device_wheel(MediusDeviceHandle dev, short delta);

    [LibraryImport(Library)]
    internal static partial Status medius_device_move_rel_now(MediusDeviceHandle dev, short dx, short dy);

    [LibraryImport(Library)]
    internal static partial Status medius_device_wheel_now(MediusDeviceHandle dev, short delta);

    [LibraryImport(Library)]
    internal static partial Status medius_device_pan(MediusDeviceHandle dev, short delta);

    [LibraryImport(Library)]
    internal static partial Status medius_device_pan_now(MediusDeviceHandle dev, short delta);

    [LibraryImport(Library)]
    internal static partial Status medius_device_flush_motion(MediusDeviceHandle dev);

    [LibraryImport(Library)]
    internal static partial Status medius_device_discard_motion(MediusDeviceHandle dev);

    [LibraryImport(Library)]
    internal static partial Status medius_device_move_axis(MediusDeviceHandle dev, MediusMotion motion, byte timing, byte pending);

    [LibraryImport(Library)]
    internal static partial Status medius_device_inject(MediusDeviceHandle dev, MediusUsage input, byte action);

    [LibraryImport(Library)]
    internal static partial Status medius_device_press(MediusDeviceHandle dev, MediusUsage input);

    [LibraryImport(Library)]
    internal static partial Status medius_device_soft_release(MediusDeviceHandle dev, MediusUsage input);

    [LibraryImport(Library)]
    internal static partial Status medius_device_force_release(MediusDeviceHandle dev, MediusUsage input);

    [LibraryImport(Library)]
    internal static partial Status medius_device_scale(MediusDeviceHandle dev, MediusLockTarget target, byte dir, short scale);

    [LibraryImport(Library)]
    internal static partial Status medius_device_scale_all(MediusDeviceHandle dev, byte what, byte dir, short scale);

    [LibraryImport(Library)]
    internal static partial Status medius_device_lock(MediusDeviceHandle dev, MediusLockTarget target, byte dir);

    [LibraryImport(Library)]
    internal static partial Status medius_device_unlock(MediusDeviceHandle dev, MediusLockTarget target, byte dir);

    [LibraryImport(Library)]
    internal static partial Status medius_device_lock_all(MediusDeviceHandle dev, byte what, byte dir);

    [LibraryImport(Library)]
    internal static partial Status medius_device_unlock_all(MediusDeviceHandle dev, byte what, byte dir);

    [LibraryImport(Library)]
    internal static partial Status medius_device_led(MediusDeviceHandle dev, byte target, byte mode, byte level);

    [LibraryImport(Library)]
    internal static partial Status medius_device_reset(MediusDeviceHandle dev);

    [LibraryImport(Library)]
    internal static partial Status medius_device_factory_reset(MediusDeviceHandle dev);

    [LibraryImport(Library)]
    internal static partial Status medius_device_reapply(MediusDeviceHandle dev);

    [LibraryImport(Library)]
    internal static partial Status medius_device_reconnect(MediusDeviceHandle dev);

    [LibraryImport(Library)]
    internal static partial Status medius_device_reboot(MediusDeviceHandle dev, byte target);

    [LibraryImport(Library)]
    internal static partial Status medius_device_allow_imperfect_clones(MediusDeviceHandle dev, byte allow);

    [LibraryImport(Library)]
    internal static partial Status medius_device_raw(MediusDeviceHandle dev, byte ep_num, byte dir, byte* bytes, nuint len);

    [LibraryImport(Library)]
    internal static partial Status medius_device_transfer(MediusDeviceHandle dev, byte ep, MediusSetup setup, byte* out_data, nuint out_len, MediusTransferOutcome* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_transfer_timeout(MediusDeviceHandle dev, byte ep, MediusSetup setup, byte* out_data, nuint out_len, uint timeout_ms, MediusTransferOutcome* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_set_rewrite(MediusDeviceHandle dev, MediusRewriteRule* rule);

    [LibraryImport(Library)]
    internal static partial Status medius_device_remove_rewrite(MediusDeviceHandle dev, MediusRewriteRule* rule);

    [LibraryImport(Library)]
    internal static partial Status medius_device_clear_rewrite(MediusDeviceHandle dev);

    [LibraryImport(Library)]
    internal static partial Status medius_device_query_rewrite(MediusDeviceHandle dev, MediusRewriteTable* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_query_rewrite_entry(MediusDeviceHandle dev, byte index, MediusRewriteRule* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_set_patch(MediusDeviceHandle dev, MediusPatch* patch);

    [LibraryImport(Library)]
    internal static partial Status medius_device_apply_patch(MediusDeviceHandle dev);

    [LibraryImport(Library)]
    internal static partial Status medius_device_clear_patch(MediusDeviceHandle dev);

    [LibraryImport(Library)]
    internal static partial Status medius_device_query_patches(MediusDeviceHandle dev, MediusPatchSet* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_query_patch_entry(MediusDeviceHandle dev, byte index, MediusPatch* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_transform(MediusDeviceHandle dev, MediusTransform* transform);

    [LibraryImport(Library)]
    internal static partial Status medius_device_untransform(MediusDeviceHandle dev, MediusTransform* transform);

    [LibraryImport(Library)]
    internal static partial Status medius_device_clear_transforms(MediusDeviceHandle dev);

    [LibraryImport(Library)]
    internal static partial Status medius_device_transform_swap(MediusDeviceHandle dev, byte a, byte b);

    [LibraryImport(Library)]
    internal static partial Status medius_device_transform_remap(MediusDeviceHandle dev, MediusLockTarget source, MediusLockTarget dest);

    [LibraryImport(Library)]
    internal static partial Status medius_device_query_transforms(MediusDeviceHandle dev, MediusTransforms* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_set_movement_riding(MediusDeviceHandle dev, byte enabled, uint window_ms);

    [LibraryImport(Library)]
    internal static partial Status medius_device_set_emit_pace(MediusDeviceHandle dev, byte mode, ushort hz, ushort force_hz);

    [LibraryImport(Library)]
    internal static partial Status medius_device_set_name(MediusDeviceHandle dev, byte* name);

    [LibraryImport(Library)]
    internal static partial Status medius_device_clear_name(MediusDeviceHandle dev);

    [LibraryImport(Library)]
    internal static partial Status medius_device_set_bearing(MediusDeviceHandle dev, ushort window_ms, byte mode);

    [LibraryImport(Library)]
    internal static partial Status medius_device_set_render(MediusDeviceHandle dev, byte mode, byte full);

    [LibraryImport(Library)]
    internal static partial Status medius_device_set_spread(MediusDeviceHandle dev, ushort percent);

    [LibraryImport(Library)]
    internal static partial Status medius_device_query_version(MediusDeviceHandle dev, MediusVersion* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_firmware_info(MediusDeviceHandle dev, MediusFirmwareInfo* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_stage_firmware(MediusDeviceHandle dev, byte target, byte* image, nuint len, delegate* unmanaged<void*, nuint, nuint, void> progress, void* user);

    [LibraryImport(Library)]
    internal static partial Status medius_device_abort_update(MediusDeviceHandle dev, byte target);

    [LibraryImport(Library)]
    internal static partial Status medius_device_activate_firmware(MediusDeviceHandle dev);

    [LibraryImport(Library)]
    internal static partial Status medius_device_update_firmware(MediusDeviceHandle dev, byte target, byte* image, nuint len, delegate* unmanaged<void*, nuint, nuint, void> progress, void* user);

    [LibraryImport(Library)]
    internal static partial Status medius_device_wait_firmware_confirmed(MediusDeviceHandle dev, MediusFirmwareInfo* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_query_health(MediusDeviceHandle dev, MediusHealth* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_device_info(MediusDeviceHandle dev, MediusDeviceInfo* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_caps(MediusDeviceHandle dev, MediusCaps* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_query_rate(MediusDeviceHandle dev, MediusRate* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_query_stats(MediusDeviceHandle dev, MediusStats* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_query_locks(MediusDeviceHandle dev, MediusLocks* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_query_catch(MediusDeviceHandle dev, MediusCatchState* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_query_imperfect(MediusDeviceHandle dev, MediusImperfectStatus* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_query_movement_riding(MediusDeviceHandle dev, byte* out_enabled, uint* out_window_ms);

    [LibraryImport(Library)]
    internal static partial Status medius_device_query_emit_pace(MediusDeviceHandle dev, MediusEmitPaceStatus* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_query_bearing(MediusDeviceHandle dev, MediusBearing* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_query_render(MediusDeviceHandle dev, MediusRenderStatus* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_query_spread(MediusDeviceHandle dev, MediusSpreadStatus* @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_counters(MediusDeviceHandle dev, MediusCountersSnapshot* @out);

    [LibraryImport(Library)]
    internal static partial uint medius_default_query_timeout_ms();

    [LibraryImport(Library)]
    internal static partial uint medius_default_transfer_timeout_ms();

    [LibraryImport(Library)]
    internal static partial uint medius_default_keepalive_cadence_ms();

    [LibraryImport(Library)]
    internal static partial uint medius_abi_version();

    [LibraryImport(Library)]
    internal static partial byte* medius_version_string();

    [LibraryImport(Library)]
    internal static partial nuint medius_last_error_message(byte* buf, nuint cap);

    [LibraryImport(Library)]
    internal static partial byte medius_last_error_proto_ver();

    [LibraryImport(Library)]
    internal static partial byte medius_last_error_update(MediusUpdateError* @out);

    [LibraryImport(Library)]
    internal static partial MediusUsage medius_usage_button(byte button);

    [LibraryImport(Library)]
    internal static partial MediusUsage medius_usage_key(Key key);

    [LibraryImport(Library)]
    internal static partial MediusUsage medius_usage_media(MediaKey media);

    [LibraryImport(Library)]
    internal static partial MediusMotion medius_motion_cursor(short dx, short dy);

    [LibraryImport(Library)]
    internal static partial MediusMotion medius_motion_wheel(short delta);

    [LibraryImport(Library)]
    internal static partial MediusMotion medius_motion_pan(short delta);

    [LibraryImport(Library)]
    internal static partial MediusLockTarget medius_lock_target_axis(byte kind);

    [LibraryImport(Library)]
    internal static partial MediusLockTarget medius_lock_target_usage(MediusUsage usage);

    [LibraryImport(Library)]
    internal static partial short medius_locks_scale_of(MediusLocks* locks, MediusLockTarget target, byte dir);

    [LibraryImport(Library)]
    internal static partial byte medius_locks_is_locked(MediusLocks* locks, MediusLockTarget target, byte dir);

    [LibraryImport(Library)]
    internal static partial byte medius_rate_native_hz(MediusRate rate, float* out_hz);

    [LibraryImport(Library)]
    internal static partial byte medius_usage_event_is_held(MediusUsageEvent* @event, MediusUsage usage);

    [LibraryImport(Library)]
    internal static partial MediusCatchFilter medius_catch_filter_watch(MediusUsage usage);

    [LibraryImport(Library)]
    internal static partial MediusCatchFilter medius_catch_filter_watch_axis(byte axis);

    [LibraryImport(Library)]
    internal static partial MediusCatchFilter medius_catch_filter_watch_class(byte class_);

    [LibraryImport(Library)]
    internal static partial MediusCatchFilter medius_catch_filter_watch_axes();

    [LibraryImport(Library)]
    internal static partial void medius_catch_filter_all_input(MediusCatchFilter* @out);

    [LibraryImport(Library)]
    internal static partial MediusCatchFilter medius_catch_filter_traffic(byte class_, ushort id);

    [LibraryImport(Library)]
    internal static partial MediusCatchFilter medius_catch_filter_traffic_class(byte class_);

    [LibraryImport(Library)]
    internal static partial MediusCatchFilter medius_catch_filter_everything();

    [LibraryImport(Library)]
    internal static partial MediusCatchFilter medius_catch_filter_with_direction(MediusCatchFilter f, byte direction);

    [LibraryImport(Library)]
    internal static partial MediusCatchFilter medius_catch_filter_with_capture(MediusCatchFilter f, byte bytes);

    [LibraryImport(Library)]
    internal static partial MediusCatchFilter medius_catch_filter_on_press(MediusCatchFilter f);

    [LibraryImport(Library)]
    internal static partial MediusCatchFilter medius_catch_filter_on_release(MediusCatchFilter f);

    [LibraryImport(Library)]
    internal static partial MediusCatchFilter medius_catch_filter_inbound(MediusCatchFilter f);

    [LibraryImport(Library)]
    internal static partial MediusCatchFilter medius_catch_filter_outbound(MediusCatchFilter f);

    [LibraryImport(Library)]
    internal static partial byte medius_catch_filter_same_address(MediusCatchFilter a, MediusCatchFilter b);

    [LibraryImport(Library)]
    internal static partial byte medius_catch_class_is_input(byte class_);

    [LibraryImport(Library)]
    internal static partial byte medius_catch_class_is_traffic(byte class_);

    [LibraryImport(Library)]
    internal static partial byte medius_traffic_event_truncated(MediusTrafficEvent* @event);

    [LibraryImport(Library)]
    internal static partial byte* medius_traffic_event_setup(MediusTrafficEvent* @event);

    [LibraryImport(Library)]
    internal static partial byte* medius_traffic_event_data(MediusTrafficEvent* @event, nuint* out_len);

    [LibraryImport(Library)]
    internal static partial byte medius_traffic_event_control_status(MediusTrafficEvent* @event, ControlStatus* @out);

    [LibraryImport(Library)]
    internal static partial byte medius_traffic_event_transfer_status(MediusTrafficEvent* @event, byte* @out);

    [LibraryImport(Library)]
    internal static partial byte medius_traffic_event_bus_event(MediusTrafficEvent* @event, MediusBusEvent* @out);

    [LibraryImport(Library)]
    internal static partial byte medius_traffic_event_rule_acted(MediusTrafficEvent* @event);

    [LibraryImport(Library)]
    internal static partial byte medius_traffic_event_bulk_end_of_transfer(MediusTrafficEvent* @event);

    [LibraryImport(Library)]
    internal static partial byte medius_traffic_event_zlp(MediusTrafficEvent* @event);

    [LibraryImport(Library)]
    internal static partial byte medius_clip_status_is_held(MediusClipStatus* status, MediusUsage usage);

    [LibraryImport(Library)]
    internal static partial byte medius_caps_has_mouse(MediusCaps caps);

    [LibraryImport(Library)]
    internal static partial byte medius_caps_has_keyboard(MediusCaps caps);

    [LibraryImport(Library)]
    internal static partial byte medius_caps_is_composite(MediusCaps caps);

    [LibraryImport(Library)]
    internal static partial byte medius_device_info_is_cloned(MediusDeviceInfo info);

    [LibraryImport(Library)]
    internal static partial Status medius_device_catch_events(MediusDeviceHandle dev, MediusCatchFilter* filters, nuint n, out MediusEventStreamHandle @out);

    [LibraryImport(Library)]
    internal static partial MediusEventStreamHandle medius_event_stream_clone(MediusEventStreamHandle stream);

    [LibraryImport(Library)]
    internal static partial void medius_event_stream_free(nint stream);

    [LibraryImport(Library)]
    internal static partial void medius_event_stream_close(MediusEventStreamHandle stream);

    [LibraryImport(Library)]
    internal static partial Status medius_event_stream_recv(MediusEventStreamHandle stream, MediusCatchEvent* @out);

    [LibraryImport(Library)]
    internal static partial byte medius_event_stream_try_recv(MediusEventStreamHandle stream, MediusCatchEvent* @out);

    [LibraryImport(Library)]
    internal static partial byte medius_event_stream_recv_timeout(MediusEventStreamHandle stream, ulong timeout_ms, MediusCatchEvent* @out);

    [LibraryImport(Library)]
    internal static partial ulong medius_event_stream_dropped(MediusEventStreamHandle stream);

    [LibraryImport(Library)]
    internal static partial Status medius_device_input_events(MediusDeviceHandle dev, MediusCatchFilter* filters, nuint n, out MediusInputStreamHandle @out);

    [LibraryImport(Library)]
    internal static partial void medius_input_stream_free(nint stream);

    [LibraryImport(Library)]
    internal static partial void medius_input_stream_close(MediusInputStreamHandle stream);

    [LibraryImport(Library)]
    internal static partial Status medius_input_stream_recv(MediusInputStreamHandle stream, MediusInputEvent* @out);

    [LibraryImport(Library)]
    internal static partial byte medius_input_stream_try_recv(MediusInputStreamHandle stream, MediusInputEvent* @out);

    [LibraryImport(Library)]
    internal static partial byte medius_input_stream_recv_timeout(MediusInputStreamHandle stream, ulong timeout_ms, MediusInputEvent* @out);

    [LibraryImport(Library)]
    internal static partial ulong medius_input_stream_dropped(MediusInputStreamHandle stream);

    [LibraryImport(Library)]
    internal static partial nuint medius_input_stream_held(MediusInputStreamHandle stream, byte class_, MediusUsage* @out, nuint cap);

    [LibraryImport(Library)]
    internal static partial MediusTimelineHandle medius_timeline_new();

    [LibraryImport(Library)]
    internal static partial void medius_timeline_free(nint t);

    [LibraryImport(Library)]
    internal static partial byte medius_timeline_observe(MediusTimelineHandle t, MediusCatchEvent* ev, ulong now_ns, MediusStamped* @out);

    [LibraryImport(Library)]
    internal static partial byte medius_timeline_observe_input(MediusTimelineHandle t, MediusInputEvent* ev, ulong now_ns, MediusStamped* @out);

    [LibraryImport(Library)]
    internal static partial void medius_timeline_reset(MediusTimelineHandle t, byte domain);

    [LibraryImport(Library)]
    internal static partial ulong medius_timeline_samples(MediusTimelineHandle t, byte domain);

    [LibraryImport(Library)]
    internal static partial byte medius_event_stream_is_connected(MediusEventStreamHandle stream);

    [LibraryImport(Library)]
    internal static partial byte medius_input_stream_is_connected(MediusInputStreamHandle stream);

    [LibraryImport(Library)]
    internal static partial Status medius_device_logs(MediusDeviceHandle dev, out MediusLogStreamHandle @out);

    [LibraryImport(Library)]
    internal static partial MediusLogStreamHandle medius_log_stream_clone(MediusLogStreamHandle stream);

    [LibraryImport(Library)]
    internal static partial void medius_log_stream_free(nint stream);

    [LibraryImport(Library)]
    internal static partial void medius_log_stream_close(MediusLogStreamHandle stream);

    [LibraryImport(Library)]
    internal static partial Status medius_log_stream_recv(MediusLogStreamHandle stream, MediusLogLine* @out);

    [LibraryImport(Library)]
    internal static partial byte medius_log_stream_try_recv(MediusLogStreamHandle stream, MediusLogLine* @out);

    [LibraryImport(Library)]
    internal static partial byte medius_log_stream_is_connected(MediusLogStreamHandle stream);

    [LibraryImport(Library)]
    internal static partial byte medius_log_stream_recv_timeout(MediusLogStreamHandle stream, ulong timeout_ms, MediusLogLine* @out);

    [LibraryImport(Library)]
    internal static partial MediusMockBoxHandle medius_mock_new();

    [LibraryImport(Library)]
    internal static partial MediusMockBoxHandle medius_mock_clone(MediusMockBoxHandle mock);

    [LibraryImport(Library)]
    internal static partial void medius_mock_free(nint mock);

    [LibraryImport(Library)]
    internal static partial void medius_mock_set_version(MediusMockBoxHandle mock, MediusVersion value);

    [LibraryImport(Library)]
    internal static partial void medius_mock_set_health(MediusMockBoxHandle mock, MediusHealth value);

    [LibraryImport(Library)]
    internal static partial void medius_mock_set_device_info(MediusMockBoxHandle mock, MediusDeviceInfo value);

    [LibraryImport(Library)]
    internal static partial void medius_mock_set_caps(MediusMockBoxHandle mock, MediusCaps value);

    [LibraryImport(Library)]
    internal static partial void medius_mock_set_mouse_caps(MediusMockBoxHandle mock, MediusMouseCaps value);

    [LibraryImport(Library)]
    internal static partial void medius_mock_set_kbd_caps(MediusMockBoxHandle mock, MediusKbdCaps value);

    [LibraryImport(Library)]
    internal static partial void medius_mock_set_rate(MediusMockBoxHandle mock, MediusRate value);

    [LibraryImport(Library)]
    internal static partial void medius_mock_set_stats(MediusMockBoxHandle mock, MediusStats value);

    [LibraryImport(Library)]
    internal static partial void medius_mock_set_locks(MediusMockBoxHandle mock, MediusLocks value);

    [LibraryImport(Library)]
    internal static partial void medius_mock_set_catch_state(MediusMockBoxHandle mock, MediusCatchState value);

    [LibraryImport(Library)]
    internal static partial void medius_mock_set_imperfect_status(MediusMockBoxHandle mock, MediusImperfectStatus value);

    [LibraryImport(Library)]
    internal static partial void medius_mock_set_transfer_reply(MediusMockBoxHandle mock, byte status, byte* data, nuint len);

    [LibraryImport(Library)]
    internal static partial void medius_mock_set_movement_riding(MediusMockBoxHandle mock, byte enabled, uint window_ms);

    [LibraryImport(Library)]
    internal static partial void medius_mock_set_bearing(MediusMockBoxHandle mock, ushort window_ms, byte mode);

    [LibraryImport(Library)]
    internal static partial void medius_mock_set_emit_pace(MediusMockBoxHandle mock, byte mode, ushort hz, ushort force_hz);

    [LibraryImport(Library)]
    internal static partial void medius_mock_set_render(MediusMockBoxHandle mock, byte mode, byte full, byte ready);

    [LibraryImport(Library)]
    internal static partial void medius_mock_set_spread_learned(MediusMockBoxHandle mock, uint period_us);

    [LibraryImport(Library)]
    internal static partial void medius_mock_set_advertised_hz(MediusMockBoxHandle mock, ushort hz);

    [LibraryImport(Library)]
    internal static partial void medius_mock_set_clip_status(MediusMockBoxHandle mock, MediusClipStatus value);

    [LibraryImport(Library)]
    internal static partial void medius_mock_set_clip_settings(MediusMockBoxHandle mock, MediusClipSettings value);

    [LibraryImport(Library)]
    internal static partial byte medius_mock_clip_packet(MediusMockBoxHandle mock, byte class_, ushort id, byte direction, byte* head, nuint head_len, byte* out_action, byte* out_consumed);

    [LibraryImport(Library)]
    internal static partial void medius_mock_restart(MediusMockBoxHandle mock);

    [LibraryImport(Library)]
    internal static partial void medius_mock_link_lost(MediusMockBoxHandle mock);

    [LibraryImport(Library)]
    internal static partial void medius_mock_detach(MediusMockBoxHandle mock, byte back_within_grace);

    [LibraryImport(Library)]
    internal static partial void medius_mock_attach(MediusMockBoxHandle mock);

    [LibraryImport(Library)]
    internal static partial void medius_mock_refuse_next_activate(MediusMockBoxHandle mock, byte status);

    [LibraryImport(Library)]
    internal static partial void medius_mock_reboot_after_activate(MediusMockBoxHandle mock, uint unanswered, uint pending);

    [LibraryImport(Library)]
    internal static partial void medius_mock_silent(MediusMockBoxHandle mock);

    [LibraryImport(Library)]
    internal static partial void medius_mock_push_raw(MediusMockBoxHandle mock, byte* bytes, nuint len);

    [LibraryImport(Library)]
    internal static partial void medius_mock_push_log(MediusMockBoxHandle mock, byte level, byte* text);

    [LibraryImport(Library)]
    internal static partial void medius_mock_push_motion(MediusMockBoxHandle mock, byte seq, uint ts_us, MediusMotionEvent @event);

    [LibraryImport(Library)]
    internal static partial void medius_mock_push_usages(MediusMockBoxHandle mock, byte seq, uint ts_us, MediusUsageEvent* @event);

    [LibraryImport(Library)]
    internal static partial void medius_mock_push_traffic(MediusMockBoxHandle mock, byte seq, uint ts_us, byte clock, MediusTrafficEvent* @event);

    [LibraryImport(Library)]
    internal static partial nuint medius_mock_recorded(MediusMockBoxHandle mock);

    [LibraryImport(Library)]
    internal static partial byte medius_mock_saw(MediusMockBoxHandle mock, byte ty);

    [LibraryImport(Library)]
    internal static partial void medius_mock_clear_recorded(MediusMockBoxHandle mock);

    [LibraryImport(Library)]
    internal static partial nuint medius_mock_recorded_frame(MediusMockBoxHandle mock, nuint idx, FrameType* out_ty, byte* out_seq, byte* payload_buf, nuint cap);

    [LibraryImport(Library)]
    internal static partial Status medius_device_with_mock(MediusMockBoxHandle mock, out MediusDeviceHandle @out);

    [LibraryImport(Library)]
    internal static partial Status medius_device_open_mock(MediusMockBoxHandle mock, out MediusDeviceHandle @out);
}
