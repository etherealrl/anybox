using static Anybox.Medius.MediusLibrary;

namespace Anybox.Medius;

// the helpers of the records Records.cs makes from the table, and the records no role fits, which are written here
// whole

public sealed partial record ChipFirmware
{
    /// <summary>like Rust's Display, "1.2.3 on ota_1 (pending-verify)", and "unknown(66)" for a state with no
    /// name</summary>
    public override string ToString() => $"{Major}.{Minor}.{Patch} on ota_{Slot} ({State.Text})";
}

public sealed partial record FirmwareInfo
{
    /// <summary>while either chip hasn't confirmed the image it booted, which is when an update gets refused</summary>
    public bool AnyPending => Device.State.IsPending || Host?.State.IsPending == true;
}

public sealed partial record BoxVersion
{
    /// <summary>the base MAC as 12 lowercase hex digits, the box's id</summary>
    public string MacHex => Convert.ToHexStringLower(Mac.Span);

    /// <summary>like Rust's Display, "fw 1.2.3"</summary>
    public override string ToString() => $"fw {FwMajor}.{FwMinor}.{FwPatch}";
}

public sealed partial record DeviceInfo
{
    /// <summary>like Rust's Display, "046D:C08B G502", the ids alone when there's no product string</summary>
    public override string ToString() => Product == "" ? $"{Vid:X4}:{Pid:X4}" : $"{Vid:X4}:{Pid:X4} {Product}";

    /// <summary>whether a device is cloned, one with no HID interface too</summary>
    public bool IsCloned
    {
        get
        {
            Ensure();
            return MediusImports.medius_device_info_is_cloned(ToNative()) != 0;
        }
    }
}

public sealed partial record Caps
{
    /// <summary>whether a mouse interface is bound</summary>
    public bool HasMouse
    {
        get
        {
            Ensure();
            return MediusImports.medius_caps_has_mouse(ToNative()) != 0;
        }
    }

    /// <summary>whether a keyboard interface is bound</summary>
    public bool HasKeyboard
    {
        get
        {
            Ensure();
            return MediusImports.medius_caps_has_keyboard(ToNative()) != 0;
        }
    }

    /// <summary>whether the clone has more than one HID interface</summary>
    public bool IsComposite
    {
        get
        {
            Ensure();
            return MediusImports.medius_caps_is_composite(ToNative()) != 0;
        }
    }
}

public sealed partial record Rate
{
    /// <summary>the device's own rate, null while the box doesn't know it</summary>
    public unsafe float? NativeHz
    {
        get
        {
            Ensure();
            float hz;
            return MediusImports.medius_rate_native_hz(ToNative(), &hz) != 0 ? hz : null;
        }
    }
}

public sealed partial record LockEntry
{
    public bool IsBlock => Scale == 0;
}

public sealed partial record Locks
{
    /// <summary>whether the target is blocked outright that way, a weighed direction isn't locked. both asks
    /// about the two fixed signs, ask for a relative one by name</summary>
    public unsafe bool IsLocked(LockTarget target, Direction direction)
    {
        Ensure();
        var raw = ToNative();
        return MediusImports.medius_locks_is_locked(&raw, target.ToNative(), In(direction, nameof(direction))) != 0;
    }

    /// <summary>the scale on that target and direction, of the entries covering it the one of smallest magnitude
    /// (a negative one on a tie), or 100 when none does. a delta meets the product of its fixed and relative
    /// scales, which this doesn't work out</summary>
    public unsafe short ScaleOf(LockTarget target, Direction direction)
    {
        Ensure();
        var raw = ToNative();
        return MediusImports.medius_locks_scale_of(&raw, target.ToNative(), In(direction, nameof(direction)));
    }
}

public sealed partial record ClockEstimate
{
    /// <summary>half the measured round trip, how wrong <see cref="OffsetUs"/> can be at most</summary>
    public int ErrorBoundUs => DelayUs / 2;

    /// <summary>a device chip stamp on the host chip's timeline, null without an estimate</summary>
    public long? ToHostDomain(long deviceUs) => AgeMs is null ? null : deviceUs + OffsetUs;

    /// <summary>how far the offset drifts over <paramref name="elapsed"/>, in microseconds. add
    /// <see cref="ErrorBoundUs"/> for the whole bound on a stamp moved that long after the estimate. it's 0 when the
    /// box fitted no rate, though the drift is unknown then and not zero</summary>
    public long DriftUsOver(TimeSpan elapsed)
    {
        // a whole microsecond count like Rust's as_micros, and never negative like its Duration
        var micros = (long)Read.Micros(elapsed, "the elapsed time");
        if (RatePpb is not { } ppb)
        {
            return 0;
        }
        // Rust's saturating multiply
        var product = Int128.Clamp((Int128)micros * ppb, long.MinValue, long.MaxValue);
        return (long)product / 1_000_000_000;
    }
}

public sealed partial record TransferOutcome
{
    public bool IsOk => Status == TransferStatus.Ok;

    /// <summary>the data when the transfer went through, null when it didn't</summary>
    // a bare null here goes through byte[] and comes out as empty memory, not as no value
    public ReadOnlyMemory<byte>? OkData => IsOk ? Data : (ReadOnlyMemory<byte>?)null;
}

/// <summary>what makes a patch the same entry as another, a second patch with it replaces the first</summary>
public sealed record PatchKey(PatchSection Section, byte Cfg, byte Index, ushort Offset);

public sealed partial record Patch
{
    /// <summary>a patch in configuration <paramref name="cfg"/>, 0 is the first one</summary>
    public static Patch InConfig(byte cfg, ushort offset, ReadOnlyMemory<byte> bytes) => new(PatchSection.Config, cfg, 0, offset, bytes);

    /// <summary>a report descriptor patch on <paramref name="interface"/> in configuration <paramref name="cfg"/>, 0 is
    /// the first one</summary>
    public static Patch InInterface(byte cfg, byte @interface, ushort offset, ReadOnlyMemory<byte> bytes) =>
        new(PatchSection.Report, cfg, @interface, offset, bytes);

    /// <summary>a patch that replaces all of string <paramref name="index"/></summary>
    public static Patch InString(byte index, ReadOnlyMemory<byte> bytes) => new(PatchSection.String, 0, index, 0, bytes);

    public PatchKey Key() => new(Section, Cfg, Index, Offset);
}

/// <summary>what makes a rewrite rule the same entry as another, a second rule with it replaces the first</summary>
public sealed record RewriteKey(RewriteClass RewriteClass, ushort Id, Direction Direction, ReadOnlyMemory<byte> MatchBytes, ReadOnlyMemory<byte> Mask)
{
    public bool Equals(RewriteKey? other) => other is not null && (RewriteClass, Id, Direction) == (other.RewriteClass, other.Id, other.Direction) &&
        Same(MatchBytes, other.MatchBytes) && Same(Mask, other.Mask);

    public override int GetHashCode() => HashCode.Combine(RewriteClass, Id, Direction, Hash(MatchBytes), Hash(Mask));
}

public sealed partial record RewriteRule
{
    public RewriteKey Key() => new(RewriteClass, Id, Direction, MatchBytes, Mask);
}

/// <summary>what makes a transform the same entry as another, a second one with it replaces the first's op</summary>
public sealed record TransformKey(LockTarget Source, LockTarget Dest);

public sealed partial record Transform
{
    public TransformKey Key() => new(Source, Dest);

    /// <summary>exchanges two axes at once</summary>
    public static Transform Swap(Axis a, Axis b) => new(TransformOp.Swap, LockTarget.Axis(a), LockTarget.Axis(b));

    /// <summary>moves the source's value into the destination and clears the source</summary>
    public static Transform Remap(LockTarget source, LockTarget dest) => new(TransformOp.Remap, source, dest);
}

public sealed partial record Bearing
{
    public bool IsLive => WindowMs is not null;
}

public sealed partial record BoxInfo
{
    /// <summary>the box's stable id, its device MAC as hex</summary>
    public string Id => Version.MacHex;

    /// <summary>the box's name, a made up default when nobody set one</summary>
    public string Name => Version.Name;

    /// <summary>the serial of the box's control adapter, when it has one</summary>
    public string? Serial => Port.Serial;
}

public sealed partial record UsageSnapshot
{
    public bool IsHeld(Usage usage) => Usages.Contains(usage);
}

public sealed partial record CatchState
{
    /// <summary>whether nothing is subscribed</summary>
    public bool IsEmpty => Entries.Count == 0;
}

public sealed partial record MotionEvent
{
    /// <summary>every axis and its delta, moved or not</summary>
    public IReadOnlyList<(Axis Axis, short Delta)> AllAxes => [(Axis.X, Dx), (Axis.Y, Dy), (Axis.Wheel, Dz), (Axis.Pan, Pan)];

    /// <summary>the axes this report moved, with their deltas. the box never sends one that moved nothing, a mock
    /// can</summary>
    public IReadOnlyList<(Axis Axis, short Delta)> Axes => [.. AllAxes.Where(axis => axis.Delta != 0)];
}

public sealed partial record MouseCaps
{
    /// <summary>whether the mouse has more than one HID interface</summary>
    public bool IsComposite => NHid > 1;
}

public sealed partial record Setup
{
    /// <summary>a device to host request, by the direction bit of the request type</summary>
    public bool IsIn => (RequestType & 0x80) != 0;

    /// <summary>the eight bytes on the wire, the numbers little endian</summary>
    public byte[] ToBytes() =>
        [RequestType, Request, (byte)Value, (byte)(Value >> 8), (byte)Index, (byte)(Index >> 8), (byte)Length, (byte)(Length >> 8)];
}

/// <summary>how far a staged update got, for its progress</summary>
public sealed record UpdateProgress(UpdateTarget Target, nuint Sent, nuint Total)
{
    /// <summary>the share sent so far, 100 for an empty image</summary>
    public byte Percent => Total == 0 ? (byte)100 : (byte)((ulong)Sent * 100 / Total);
}

public sealed partial record ClipStatus
{
    /// <summary>whether the clip holds that usage down</summary>
    public bool IsHeld(Usage usage) => Held.Contains(usage);
}

public sealed partial record ClipPacketTrigger
{
    /// <summary>the id that addresses every interface or endpoint of the class</summary>
    public const ushort AnyId = Constants.CatchIdAny;

    // just the key an unbind reads, the rest zero and not looked at
    internal unsafe MediusClipPacketTrigger KeyOnly()
    {
        var raw = new MediusClipPacketTrigger
        {
            class_ = In(TrafficClass, nameof(TrafficClass)),
            id = Id,
            direction = In(Direction, nameof(Direction)),
        };
        raw.match_len = Match(MatchBytes.Span, raw.match_bytes, Constants.MaxPktMatch, nameof(MatchBytes));
        raw.mask_len = Match(Mask.Span, raw.mask, Constants.MaxPktMatch, nameof(Mask));
        return raw;
    }
}

public sealed partial record TrafficEvent
{
    /// <summary>whether the capture cut the packet short, which <see cref="Bytes"/> alone can't tell from a short
    /// packet</summary>
    public unsafe bool Truncated
    {
        get
        {
            Ensure();
            var raw = ToNative();
            return MediusImports.medius_traffic_event_truncated(&raw) != 0;
        }
    }

    /// <summary>the 8 setup bytes of a control or clip transfer event, null for another class or a shorter
    /// capture</summary>
    public unsafe ReadOnlyMemory<byte>? SetupBytes
    {
        get
        {
            Ensure();
            var raw = ToNative();
            // it points into raw, so the bytes get copied out while raw is still here
            var setup = MediusImports.medius_traffic_event_setup(&raw);
            // the cast keeps a missing setup null, a bare null would turn into empty memory through byte[]
            return setup == null ? (ReadOnlyMemory<byte>?)null : new ReadOnlySpan<byte>(setup, 8).ToArray();
        }
    }

    /// <summary>the data stage of a control or clip transfer event, the whole packet for any other class</summary>
    public unsafe ReadOnlyMemory<byte> Data
    {
        get
        {
            Ensure();
            var raw = ToNative();
            nuint length;
            var data = MediusImports.medius_traffic_event_data(&raw, &length);
            return data == null ? ReadOnlyMemory<byte>.Empty : new ReadOnlySpan<byte>(data, (int)length).ToArray();
        }
    }

    /// <summary>the handshake the game PC got, null for any class but control</summary>
    public unsafe ControlStatus? Control
    {
        get
        {
            Ensure();
            var raw = ToNative();
            ControlStatus status;
            return MediusImports.medius_traffic_event_control_status(&raw, &status) != 0 ? Out(status, "a control status") : null;
        }
    }

    /// <summary>how the transfer ended, null for any class but clip transfer. nak when no reply came, and a status
    /// the package has no name for stays its number</summary>
    public unsafe TransferStatus? Transfer
    {
        get
        {
            Ensure();
            var raw = ToNative();
            byte status;
            return MediusImports.medius_traffic_event_transfer_status(&raw, &status) != 0 ? (TransferStatus)status : null;
        }
    }

    /// <summary>the lifecycle event, null for any class but bus or a kind the library doesn't know</summary>
    public unsafe BusEvent? Bus
    {
        get
        {
            Ensure();
            var raw = ToNative();
            MediusBusEvent bus;
            return MediusImports.medius_traffic_event_bus_event(&raw, &bus) != 0 ? BusEvent.From(bus) : null;
        }
    }

    /// <summary>whether a rewrite rule changed, dropped, answered or refused the packet. only HID in and out, the
    /// vendor classes, control and emit carry it</summary>
    public unsafe bool RuleActed
    {
        get
        {
            Ensure();
            var raw = ToNative();
            return MediusImports.medius_traffic_event_rule_acted(&raw) != 0;
        }
    }

    public unsafe bool BulkEndOfTransfer
    {
        get
        {
            Ensure();
            var raw = ToNative();
            return MediusImports.medius_traffic_event_bulk_end_of_transfer(&raw) != 0;
        }
    }

    /// <summary>whether it's a zero length packet, or on a HID endpoint a HID in or emit report one ended</summary>
    public unsafe bool Zlp
    {
        get
        {
            Ensure();
            var raw = ToNative();
            return MediusImports.medius_traffic_event_zlp(&raw) != 0;
        }
    }
}

/// <summary>what paces injected motion, made with <see cref="Learned"/>, <see cref="Interval"/> or
/// <see cref="Fixed"/>, so only a fixed one has a rate, like Rust's enum</summary>
public sealed record EmitPace
{
    EmitPace(EmitMode mode, ushort hz)
    {
        Mode = mode;
        Hz = hz;
    }

    public EmitMode Mode { get; }

    /// <summary>the rate of a fixed one, 0 for the others</summary>
    public ushort Hz { get; }

    public static EmitPace Learned { get; } = new(EmitMode.Learned, 0);
    public static EmitPace Interval { get; } = new(EmitMode.Interval, 0);
    public static EmitPace Fixed(ushort hz) => new(EmitMode.Fixed, hz);
}

/// <summary>the configured pace, the emit rate ceiling and the wire rate in use</summary>
public sealed record EmitPaceStatus(EmitPace Mode, ushort ResolvedHz, ushort? ForceHz, ushort AdvertisedHz, bool ForceActive)
{
    /// <summary>Rust's default, a learned pace and nothing else set</summary>
    public static EmitPaceStatus Default { get; } = new(EmitPace.Learned, 0, null, 0, false);

    // the fixed rate only means something on a fixed pace, so a learned or interval one leaves it behind
    internal static EmitPaceStatus From(MediusEmitPaceStatus raw) => new(
        Out(raw.mode, "an emit mode") switch
        {
            EmitMode.Learned => EmitPace.Learned,
            EmitMode.Interval => EmitPace.Interval,
            _ => EmitPace.Fixed(raw.fixed_hz),
        },
        raw.resolved_hz, raw.force_hz == 0 ? null : raw.force_hz, raw.advertised_hz, raw.force_active != 0);
}

/// <summary>one catch event. <see cref="TsUs"/> is in the <see cref="Clock"/> chip's microseconds, a box local
/// clock, so only compare stamps from the same chip, and <see cref="CatchState.Clock"/> moves one across. it wraps
/// about every 71.6 minutes and starts at 0 again when that chip reboots</summary>
public sealed record CatchEvent(CatchEventKind Kind, uint TsUs, ClockDomain Clock, MotionEvent? Motion, UsageSnapshot? Usages, TrafficEvent? Traffic)
{
    /// <summary>the class it came on, the axis class for motion</summary>
    public CatchClass Class => Traffic?.CatchClass ?? (Usages is { } usages ? CatchClass.From(usages.Class) : CatchClass.Axis);

    /// <summary>the interface or endpoint of traffic, null for motion and usages</summary>
    public ushort? Id => Traffic?.Id;

    /// <summary>the edge of a usage snapshot or the flow of traffic, both for motion</summary>
    public Direction Direction => Traffic?.Direction ?? Usages?.Direction ?? Direction.Both;

    /// <summary>the packet bytes of traffic, empty for motion and usages</summary>
    public ReadOnlyMemory<byte> Bytes => Traffic?.Bytes ?? ReadOnlyMemory<byte>.Empty;

    internal static CatchEvent From(MediusCatchEvent raw)
    {
        var kind = Out(raw.kind, "a catch event");
        var clock = Out(raw.clock, "a clock domain");
        return kind switch
        {
            CatchEventKind.Motion => new(kind, raw.ts_us, clock, MotionEvent.From(raw.data.motion), null, null),
            CatchEventKind.Traffic => new(kind, raw.ts_us, clock, null, null, TrafficEvent.From(raw.data.traffic)),
            _ => new(kind, raw.ts_us, clock, null, UsageSnapshot.From(raw.data.usages), null),
        };
    }
}

/// <summary>a frame the mock got. a frame type the package has no name for stays its number</summary>
public sealed record RecordedFrame(FrameType Type, byte Seq, ReadOnlyMemory<byte> Payload)
{
    public bool Equals(RecordedFrame? other) => other is not null && (Type, Seq) == (other.Type, other.Seq) && Same(Payload, other.Payload);

    public override int GetHashCode() => HashCode.Combine(Type, Seq, Hash(Payload));
}

/// <summary>one decoded input event, a press or release, or a motion report. the deltas are 0 unless it's
/// motion, x positive going right, y down, the wheel up and pan right</summary>
public sealed record InputEvent(InputKind Kind, uint TsUs, ClockDomain Clock, Usage? Usage = null, short Dx = 0, short Dy = 0, short Dz = 0, short Pan = 0)
{
    public bool IsPress => Kind == InputKind.Press;
    public bool IsRelease => Kind == InputKind.Release;

    /// <summary>the edge as a direction, both for motion</summary>
    public Direction Direction => Kind switch
    {
        InputKind.Press => Direction.Press,
        InputKind.Release => Direction.Release,
        _ => Direction.Both,
    };

    /// <summary>the axes a motion report moved, with their deltas, empty for an edge</summary>
    public IReadOnlyList<(Axis Axis, short Delta)> Axes => Kind == InputKind.Motion
        ? new MotionEvent(Dx, Dy, Dz, Pan).Axes
        : [];

    internal static InputEvent From(MediusInputEvent raw)
    {
        var kind = Out(raw.kind, "an input event");
        return new(kind, raw.ts_us, Out(raw.clock, "a clock domain"),
            kind == InputKind.Motion ? null : Medius.Usage.From(raw.usage), raw.dx, raw.dy, raw.dz, raw.pan);
    }
}

/// <summary>the clip's configuration, its autolock, loop, retain, finalized and ride, and both kinds of trigger,
/// the packet ones in box order with their hits</summary>
public sealed record ClipSettings
{
    public IReadOnlyList<Blanket> Autolock { get; init; } = [];
    public bool Loop { get; init; }
    public bool Retain { get; init; }
    public bool Finalized { get; init; }
    public bool Ride { get; init; }
    public IReadOnlyList<ClipTrigger> Triggers { get; init; } = [];
    public IReadOnlyList<ClipPacketTrigger> PacketTriggers { get; init; } = [];

    // the autolock bits are the box's own, not the blanket numbers
    static readonly (byte Bit, Blanket Blanket)[] BlanketBits =
        [(0x01, Blanket.Aim), (0x02, Blanket.Wheel), (0x04, Blanket.Buttons), (0x08, Blanket.Keys), (0x10, Blanket.Media)];

    public bool Equals(ClipSettings? other) => other is not null && (Loop, Retain, Finalized, Ride) == (other.Loop, other.Retain, other.Finalized, other.Ride) &&
        Same(Autolock, other.Autolock) && Same(Triggers, other.Triggers) && Same(PacketTriggers, other.PacketTriggers);

    public override int GetHashCode() => HashCode.Combine(Loop, Retain, Finalized, Ride, Hash(Autolock), Hash(Triggers), Hash(PacketTriggers));

    internal static ClipSettings From(MediusClipSettings raw) => new()
    {
        Autolock = [.. BlanketBits.Where(bit => (raw.autolock_bits & bit.Bit) != 0).Select(bit => bit.Blanket)],
        Loop = raw.loop_ != 0,
        Retain = raw.retain != 0,
        Finalized = raw.finalized != 0,
        Ride = raw.ride != 0,
        Triggers = ListOf<MediusClipTrigger, ClipTrigger>(raw.triggers, raw.n, ClipTrigger.From),
        PacketTriggers = ListOf<MediusClipPacketTrigger, ClipPacketTrigger>(raw.packet_triggers, raw.packet_n, ClipPacketTrigger.From),
    };

    internal MediusClipSettings ToNative()
    {
        var raw = new MediusClipSettings
        {
            loop_ = Flag(Loop),
            retain = Flag(Retain),
            finalized = Flag(Finalized),
            ride = Flag(Ride),
        };
        foreach (var blanket in Autolock)
        {
            In(blanket, nameof(Autolock));
            raw.autolock_bits |= BlanketBits.Single(bit => bit.Blanket == blanket).Bit;
        }
        raw.n = (byte)Fill<ClipTrigger, MediusClipTrigger>(Triggers, raw.triggers, nameof(Triggers), trigger => trigger.ToNative());
        raw.packet_n = (byte)Fill<ClipPacketTrigger, MediusClipPacketTrigger>(PacketTriggers, raw.packet_triggers, nameof(PacketTriggers), trigger => trigger.ToNative());
        return raw;
    }
}
