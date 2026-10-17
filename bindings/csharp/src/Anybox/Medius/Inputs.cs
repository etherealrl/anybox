using static Anybox.Medius.MediusLibrary;

namespace Anybox.Medius;

/// <summary>a momentary usage, a button, a key or a media key. a <see cref="Medius.Button"/>, <see cref="Medius.Key"/>
/// or <see cref="Medius.MediaKey"/> turns into one by itself</summary>
public readonly record struct Usage(Class Kind, ushort Id)
{
    /// <summary>any id up to the clone's button count is a button, so this takes a plain number too</summary>
    public static Usage Button(byte button)
    {
        Ensure();
        return From(MediusImports.medius_usage_button(button));
    }

    public static Usage Key(Key key)
    {
        Ensure();
        return From(MediusImports.medius_usage_key(key));
    }

    public static Usage Media(MediaKey media)
    {
        Ensure();
        return From(MediusImports.medius_usage_media(media));
    }

    public static implicit operator Usage(Button button) => Button((byte)button);
    public static implicit operator Usage(Key key) => Key(key);
    public static implicit operator Usage(MediaKey media) => Media(media);

    internal MediusUsage ToNative() => new() { kind = In(Kind, "the usage's class"), id = Id };

    internal static Usage From(MediusUsage raw) => new(Out((Class)raw.kind, "a usage class"), raw.id);
}

/// <summary>a relative axis drive, made with <see cref="Cursor"/>, <see cref="Wheel"/> or <see cref="Pan"/>. two are
/// equal when they're the same kind with the same deltas</summary>
public readonly struct Motion : IEquatable<Motion>
{
    readonly MediusMotion raw;

    Motion(MediusMotion raw) => this.raw = raw;

    public MotionKind Kind => Out((MotionKind)raw.kind, "a motion kind");

    /// <summary>the cursor's x, 0 for the wheel and the pan</summary>
    public short Dx => Kind == MotionKind.Cursor ? raw.dx : (short)0;

    /// <summary>the cursor's y, 0 for the wheel and the pan</summary>
    public short Dy => Kind == MotionKind.Cursor ? raw.dy : (short)0;

    /// <summary>the wheel's or the pan's delta, 0 for the cursor</summary>
    public short Delta => Kind switch
    {
        MotionKind.Wheel => raw.wheel,
        MotionKind.Pan => raw.pan,
        _ => 0,
    };

    public bool Equals(Motion other) => (Kind, Dx, Dy, Delta) == (other.Kind, other.Dx, other.Dy, other.Delta);

    public override bool Equals(object? other) => other is Motion motion && Equals(motion);

    public override int GetHashCode() => HashCode.Combine(Kind, Dx, Dy, Delta);

    public static bool operator ==(Motion left, Motion right) => left.Equals(right);

    public static bool operator !=(Motion left, Motion right) => !left.Equals(right);

    public static Motion Cursor(short dx, short dy)
    {
        Ensure();
        return new(MediusImports.medius_motion_cursor(dx, dy));
    }

    public static Motion Wheel(short delta)
    {
        Ensure();
        return new(MediusImports.medius_motion_wheel(delta));
    }

    public static Motion Pan(short delta)
    {
        Ensure();
        return new(MediusImports.medius_motion_pan(delta));
    }

    internal MediusMotion ToNative() => raw;
}

/// <summary>what a lock, a scale or a transform works on, an axis or a momentary usage. an <see cref="Medius.Axis"/>
/// or a usage turns into one by itself</summary>
public readonly record struct LockTarget
{
    readonly MediusUsage usage;

    LockTarget(MediusLockTarget raw)
    {
        Kind = Out((LockTargetKind)raw.kind, "a lock target");
        usage = raw.usage;
    }

    public LockTargetKind Kind { get; }

    /// <summary>the usage, when <see cref="Kind"/> is <see cref="LockTargetKind.Usage"/>, null for an axis</summary>
    public Usage? Input => Kind == LockTargetKind.Usage ? Medius.Usage.From(usage) : null;

    /// <summary>the axis, null for a usage</summary>
    public Axis? AsAxis => Kind switch
    {
        LockTargetKind.X => Medius.Axis.X,
        LockTargetKind.Y => Medius.Axis.Y,
        LockTargetKind.Wheel => Medius.Axis.Wheel,
        LockTargetKind.Pan => Medius.Axis.Pan,
        _ => null,
    };

    public static LockTarget X => Axis(Medius.Axis.X);
    public static LockTarget Y => Axis(Medius.Axis.Y);
    public static LockTarget Wheel => Axis(Medius.Axis.Wheel);
    public static LockTarget Pan => Axis(Medius.Axis.Pan);

    public static LockTarget Axis(Axis axis)
    {
        Ensure();
        return new(MediusImports.medius_lock_target_axis(In(axis, nameof(axis))));
    }

    public static LockTarget Usage(Usage usage)
    {
        Ensure();
        return new(MediusImports.medius_lock_target_usage(usage.ToNative()));
    }

    public static LockTarget Button(byte button) => Usage(Medius.Usage.Button(button));
    public static LockTarget Key(Key key) => Usage(Medius.Usage.Key(key));
    public static LockTarget Media(MediaKey media) => Usage(Medius.Usage.Media(media));

    public static implicit operator LockTarget(Axis axis) => Axis(axis);
    public static implicit operator LockTarget(Usage usage) => Usage(usage);
    public static implicit operator LockTarget(Button button) => Usage(button);
    public static implicit operator LockTarget(Key key) => Usage(key);
    public static implicit operator LockTarget(MediaKey media) => Usage(media);

    internal MediusLockTarget ToNative() => new() { kind = In(Kind, "the lock target"), usage = usage };

    internal static LockTarget From(MediusLockTarget raw) => new(raw);
}

/// <summary>how much of each packet a catch keeps. traffic classes only, a capture on an input class is refused.
/// it's a ceiling the box holds once per address, so another subscriber asking for more raises yours too</summary>
public static class Capture
{
    public const byte Whole = 0;

    /// <summary>keeps the first <paramref name="n"/> bytes, 0 is <see cref="Whole"/></summary>
    public static byte First(byte n) => n;

    /// <summary>how many bytes a capture keeps, null for the whole packet</summary>
    public static byte? Bytes(byte capture) => capture == Whole ? null : capture;

    /// <summary>the wider of two, the whole packet if either keeps it, otherwise the longer one</summary>
    public static byte Widest(byte a, byte b) => a == Whole || b == Whole ? Whole : Math.Max(a, b);
}

/// <summary>one catch subscription: what to watch, which way, and how much of each packet to keep. an event goes
/// to its most specific entry, an exact class and id over a class, a class over <see cref="Everything"/>, a named
/// direction over both</summary>
public readonly record struct CatchFilter
{
    readonly MediusCatchFilter raw;

    CatchFilter(MediusCatchFilter raw) => this.raw = raw;

    // a default filter never went through a factory, so the library may not be loaded yet
    MediusCatchFilter Loaded
    {
        get
        {
            Ensure();
            return raw;
        }
    }

    /// <summary>the class it watches, null for every class</summary>
    public CatchClass? Class => raw.class_ == Constants.CatchClassAny ? null : Out((CatchClass)raw.class_, "a catch class");

    /// <summary>the class specific id, null for every id</summary>
    public ushort? Id => raw.id == Constants.CatchIdAny ? null : raw.id;

    public Direction Direction => Out((Direction)raw.direction, "a direction");

    /// <summary>bytes kept per event, 0 keeps the whole packet</summary>
    public byte Capture => raw.capture;

    public static CatchFilter Watch(Usage usage)
    {
        Ensure();
        return new(MediusImports.medius_catch_filter_watch(usage.ToNative()));
    }

    public static CatchFilter WatchAxis(Axis axis)
    {
        Ensure();
        return new(MediusImports.medius_catch_filter_watch_axis(In(axis, nameof(axis))));
    }

    /// <summary>every usage of one momentary class</summary>
    public static CatchFilter WatchClass(Class inputClass)
    {
        Ensure();
        return new(MediusImports.medius_catch_filter_watch_class(In(inputClass, nameof(inputClass))));
    }

    /// <summary>every relative axis, X, Y, the wheel and pan</summary>
    public static CatchFilter WatchAxes()
    {
        Ensure();
        return new(MediusImports.medius_catch_filter_watch_axes());
    }

    /// <summary>all four input classes, everything <see cref="Device.InputEvents"/> can report</summary>
    public static unsafe IReadOnlyList<CatchFilter> AllInput()
    {
        Ensure();
        var filters = stackalloc MediusCatchFilter[4];
        MediusImports.medius_catch_filter_all_input(filters);
        return [new(filters[0]), new(filters[1]), new(filters[2]), new(filters[3])];
    }

    /// <summary>one traffic address, an endpoint, an interface or a control endpoint number</summary>
    public static CatchFilter Traffic(TrafficClass trafficClass, ushort id)
    {
        Ensure();
        return new(MediusImports.medius_catch_filter_traffic(In(trafficClass, nameof(trafficClass)), id));
    }

    /// <summary>every id of one traffic class</summary>
    public static CatchFilter TrafficOf(TrafficClass trafficClass)
    {
        Ensure();
        return new(MediusImports.medius_catch_filter_traffic_class(In(trafficClass, nameof(trafficClass))));
    }

    /// <summary>every class, every id, both ways, whole packets, as one entry. vendor bulk alone can fill the
    /// control link, so pair it with <see cref="WithCapture"/> unless you really want bulk in full</summary>
    public static CatchFilter Everything()
    {
        Ensure();
        return new(MediusImports.medius_catch_filter_everything());
    }

    public CatchFilter WithDirection(Direction direction) =>
        new(MediusImports.medius_catch_filter_with_direction(Loaded, In(direction, nameof(direction))));

    /// <summary>keeps only the first <paramref name="bytes"/> of each packet, 0 keeps the whole packet</summary>
    public CatchFilter WithCapture(byte bytes) => new(MediusImports.medius_catch_filter_with_capture(Loaded, bytes));

    public CatchFilter OnPress() => new(MediusImports.medius_catch_filter_on_press(Loaded));
    public CatchFilter OnRelease() => new(MediusImports.medius_catch_filter_on_release(Loaded));

    /// <summary>only traffic from the device to the PC</summary>
    public CatchFilter Inbound() => new(MediusImports.medius_catch_filter_inbound(Loaded));

    /// <summary>only traffic from the PC to the device</summary>
    public CatchFilter Outbound() => new(MediusImports.medius_catch_filter_outbound(Loaded));

    /// <summary>whether both are the same entry in the box's table, whatever they capture</summary>
    public bool SameAddress(CatchFilter other) => MediusImports.medius_catch_filter_same_address(Loaded, other.raw) != 0;

    public override string ToString() =>
        $"CatchFilter {{ Class = {Class?.ToString() ?? "Any"}, Id = {Id?.ToString() ?? "Any"}, Direction = {Direction}, Capture = {Capture} }}";

    internal MediusCatchFilter ToNative() => raw;

    internal static CatchFilter From(MediusCatchFilter raw) => new(raw);
}
