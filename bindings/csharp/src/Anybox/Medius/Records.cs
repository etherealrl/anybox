// made by bindings/csharp/tools/gen-medius.cs from bindings/c/include/medius.h and its record table, don't edit it
// by hand. the helpers of each record are in Types.cs, the other half of its partial record
namespace Anybox.Medius;

/// <summary>a USB setup packet. <see cref="Length"/> is the data stage, the bytes to read for IN, otherwise the length
/// of the OUT data</summary>
public sealed partial record Setup(
    byte RequestType = 0,
    byte Request = 0,
    ushort Value = 0,
    ushort Index = 0,
    ushort Length = 0)
{
    /// <summary>what Rust's Default gives</summary>
    public static Setup Default { get; } = new(0, 0, 0, 0, 0);

    internal static unsafe Setup From(MediusSetup raw) => new(
        raw.request_type,
        raw.request,
        raw.value,
        raw.index,
        raw.length);

    internal unsafe MediusSetup ToNative()
    {
        var raw = new MediusSetup();
        raw.request_type = RequestType;
        raw.request = Request;
        raw.value = Value;
        raw.index = Index;
        raw.length = Length;
        return raw;
    }
}

/// <summary>one clip input trigger. <see cref="On"/>'s <see cref="Edge"/> drives <see cref="Action"/>, and
/// <see cref="Consume"/> keeps the input from the game</summary>
public sealed partial record ClipTrigger(
    Usage On,
    Edge Edge,
    ClipAction Action,
    bool Consume = false)
{
    internal static unsafe ClipTrigger From(MediusClipTrigger raw) => new(
        Usage.From(raw.on),
        MediusLibrary.Out((Edge)raw.edge, "an edge"),
        MediusLibrary.Out((ClipAction)raw.action, "a clip action"),
        raw.consume != 0);

    internal unsafe MediusClipTrigger ToNative()
    {
        var raw = new MediusClipTrigger();
        raw.on = On.ToNative();
        raw.edge = MediusLibrary.In(Edge, nameof(Edge));
        raw.action = MediusLibrary.In(Action, nameof(Action));
        raw.consume = MediusLibrary.Flag(Consume);
        return raw;
    }
}

/// <summary>one clip packet trigger, keyed by its class, id, direction, match and mask. a packet whose head matches
/// under the mask drives <see cref="Action"/> on the box's next tick with no round trip to the PC, see
/// <c>ClipPacketTrigger</c> in the medius docs for which ones the box refuses and how they rank</summary>
public sealed partial record ClipPacketTrigger(
    TrafficClass TrafficClass,
    ushort Id,
    Direction Direction,
    ClipAction Action,
    ReadOnlyMemory<byte> MatchBytes = default,
    ReadOnlyMemory<byte> Mask = default,
    bool Consume = false,
    bool OncePerRun = false,
    byte SelectorLen = 0,
    ushort Hits = 0)
{
    public bool Equals(ClipPacketTrigger? other) => other is not null &&
        TrafficClass == other.TrafficClass &&
        Id == other.Id &&
        Direction == other.Direction &&
        Action == other.Action &&
        MediusLibrary.Same(MatchBytes, other.MatchBytes) &&
        MediusLibrary.Same(Mask, other.Mask) &&
        Consume == other.Consume &&
        OncePerRun == other.OncePerRun &&
        SelectorLen == other.SelectorLen &&
        Hits == other.Hits;

    public override int GetHashCode()
    {
        var hash = new HashCode();
        hash.Add(TrafficClass);
        hash.Add(Id);
        hash.Add(Direction);
        hash.Add(Action);
        hash.Add(MediusLibrary.Hash(MatchBytes));
        hash.Add(MediusLibrary.Hash(Mask));
        hash.Add(Consume);
        hash.Add(OncePerRun);
        hash.Add(SelectorLen);
        hash.Add(Hits);
        return hash.ToHashCode();
    }

    internal static unsafe ClipPacketTrigger From(MediusClipPacketTrigger raw) => new(
        MediusLibrary.Out((TrafficClass)raw.class_, "a traffic class"),
        raw.id,
        MediusLibrary.Out((Direction)raw.direction, "a direction"),
        MediusLibrary.Out((ClipAction)raw.action, "a clip action"),
        MediusLibrary.Bytes(raw.match_bytes, raw.match_len, Constants.MaxPktMatch),
        MediusLibrary.Bytes(raw.mask, raw.mask_len, Constants.MaxPktMatch),
        raw.consume != 0,
        raw.once_per_run != 0,
        raw.selector_len,
        raw.hits);

    internal unsafe MediusClipPacketTrigger ToNative()
    {
        var raw = new MediusClipPacketTrigger();
        raw.class_ = MediusLibrary.In(TrafficClass, nameof(TrafficClass));
        raw.id = Id;
        raw.direction = MediusLibrary.In(Direction, nameof(Direction));
        raw.action = MediusLibrary.In(Action, nameof(Action));
        raw.match_len = (ushort)MediusLibrary.Match(MatchBytes.Span, raw.match_bytes, Constants.MaxPktMatch, nameof(MatchBytes));
        raw.mask_len = (ushort)MediusLibrary.Match(Mask.Span, raw.mask, Constants.MaxPktMatch, nameof(Mask));
        raw.consume = MediusLibrary.Flag(Consume);
        raw.once_per_run = MediusLibrary.Flag(OncePerRun);
        raw.selector_len = SelectorLen;
        raw.hits = Hits;
        return raw;
    }
}

/// <summary>the clip's ring and playback counters on the box</summary>
public sealed partial record ClipStatus(
    ClipState State,
    uint Free,
    uint Total,
    uint Played,
    uint Ticks,
    ushort Underruns,
    ushort Overruns,
    ushort SeqGaps,
    ushort Xfers,
    ushort XferErrs,
    ushort Gated,
    IReadOnlyList<Usage> Held)
{
    /// <summary>what Rust's Default gives</summary>
    public static ClipStatus Default { get; } = new(ClipState.Idle, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, []);

    public bool Equals(ClipStatus? other) => other is not null &&
        State == other.State &&
        Free == other.Free &&
        Total == other.Total &&
        Played == other.Played &&
        Ticks == other.Ticks &&
        Underruns == other.Underruns &&
        Overruns == other.Overruns &&
        SeqGaps == other.SeqGaps &&
        Xfers == other.Xfers &&
        XferErrs == other.XferErrs &&
        Gated == other.Gated &&
        MediusLibrary.Same(Held, other.Held);

    public override int GetHashCode()
    {
        var hash = new HashCode();
        hash.Add(State);
        hash.Add(Free);
        hash.Add(Total);
        hash.Add(Played);
        hash.Add(Ticks);
        hash.Add(Underruns);
        hash.Add(Overruns);
        hash.Add(SeqGaps);
        hash.Add(Xfers);
        hash.Add(XferErrs);
        hash.Add(Gated);
        hash.Add(MediusLibrary.Hash(Held));
        return hash.ToHashCode();
    }

    internal static unsafe ClipStatus From(MediusClipStatus raw) => new(
        MediusLibrary.Out((ClipState)raw.state, "a clip state"),
        raw.free,
        raw.total,
        raw.played,
        raw.ticks,
        raw.underruns,
        raw.overruns,
        raw.seq_gaps,
        raw.xfers,
        raw.xfer_errs,
        raw.gated,
        MediusLibrary.ListOf<MediusUsage, Usage>(raw.held, raw.held_n, Usage.From));

    internal unsafe MediusClipStatus ToNative()
    {
        var raw = new MediusClipStatus();
        raw.state = MediusLibrary.In(State, nameof(State));
        raw.free = Free;
        raw.total = Total;
        raw.played = Played;
        raw.ticks = Ticks;
        raw.underruns = Underruns;
        raw.overruns = Overruns;
        raw.seq_gaps = SeqGaps;
        raw.xfers = Xfers;
        raw.xfer_errs = XferErrs;
        raw.gated = Gated;
        raw.held_n = (ushort)MediusLibrary.Fill<Usage, MediusUsage>(Held, raw.held, nameof(Held), item => item.ToNative());
        return raw;
    }
}

/// <summary>a serial port that could be a box</summary>
public sealed partial record PortInfo(
    string Path = "",
    ushort Vid = 0,
    ushort Pid = 0,
    string? Serial = null)
{
    internal static unsafe PortInfo From(MediusPortInfo raw) => new(
        MediusLibrary.Text(new ReadOnlySpan<byte>(raw.path, Constants.MaxPath)),
        raw.vid,
        raw.pid,
        raw.has_serial != 0 ? MediusLibrary.Text(new ReadOnlySpan<byte>(raw.serial, Constants.MaxSerial)) : null);

    internal unsafe MediusPortInfo ToNative()
    {
        var raw = new MediusPortInfo();
        MediusLibrary.Text(Path, raw.path, Constants.MaxPath);
        raw.vid = Vid;
        raw.pid = Pid;
        raw.has_serial = MediusLibrary.Flag(Serial is not null);
        if (Serial is not null)
        {
            MediusLibrary.Text(Serial, raw.serial, Constants.MaxSerial);
        }
        return raw;
    }
}

/// <summary>what the box says it is. it's <c>Version</c> in the Python package, the name would clash with
/// <c>System.Version</c> here</summary>
public sealed partial record BoxVersion(
    byte ProtoVer,
    byte FwMajor,
    byte FwMinor,
    byte FwPatch,
    ReadOnlyMemory<byte> Mac,
    string Name = "")
{
    public bool Equals(BoxVersion? other) => other is not null &&
        ProtoVer == other.ProtoVer &&
        FwMajor == other.FwMajor &&
        FwMinor == other.FwMinor &&
        FwPatch == other.FwPatch &&
        MediusLibrary.Same(Mac, other.Mac) &&
        Name == other.Name;

    public override int GetHashCode()
    {
        var hash = new HashCode();
        hash.Add(ProtoVer);
        hash.Add(FwMajor);
        hash.Add(FwMinor);
        hash.Add(FwPatch);
        hash.Add(MediusLibrary.Hash(Mac));
        hash.Add(Name);
        return hash.ToHashCode();
    }

    internal static unsafe BoxVersion From(MediusVersion raw) => new(
        raw.proto_ver,
        raw.fw_major,
        raw.fw_minor,
        raw.fw_patch,
        MediusLibrary.Bytes(raw.mac, 6, 6),
        MediusLibrary.Text(new ReadOnlySpan<byte>(raw.name, Constants.MaxName)));

    internal unsafe MediusVersion ToNative()
    {
        var raw = new MediusVersion();
        raw.proto_ver = ProtoVer;
        raw.fw_major = FwMajor;
        raw.fw_minor = FwMinor;
        raw.fw_patch = FwPatch;
        MediusLibrary.Exact(Mac.Span, raw.mac, 6, nameof(Mac));
        MediusLibrary.Text(Name, raw.name, Constants.MaxName);
        return raw;
    }
}

/// <summary>the cloned device's USB ids, what kind it mostly is and its product string</summary>
public sealed partial record DeviceInfo(
    ushort Vid = 0,
    ushort Pid = 0,
    ushort BcdDevice = 0,
    ushort BcdUsb = 0,
    bool HasSerial = false,
    bool HasBos = false,
    DeviceKind Kind = DeviceKind.Unknown,
    string Product = "")
{
    /// <summary>what Rust's Default gives</summary>
    public static DeviceInfo Default { get; } = new(0, 0, 0, 0, false, false, DeviceKind.Unknown, "");

    internal static unsafe DeviceInfo From(MediusDeviceInfo raw) => new(
        raw.vid,
        raw.pid,
        raw.bcd_device,
        raw.bcd_usb,
        raw.has_serial != 0,
        raw.has_bos != 0,
        (Enum.IsDefined((DeviceKind)raw.kind) ? (DeviceKind)raw.kind : DeviceKind.Unknown),
        MediusLibrary.Text(new ReadOnlySpan<byte>(raw.product, Constants.MaxProduct)));

    internal unsafe MediusDeviceInfo ToNative()
    {
        var raw = new MediusDeviceInfo();
        raw.vid = Vid;
        raw.pid = Pid;
        raw.bcd_device = BcdDevice;
        raw.bcd_usb = BcdUsb;
        raw.has_serial = MediusLibrary.Flag(HasSerial);
        raw.has_bos = MediusLibrary.Flag(HasBos);
        raw.kind = MediusLibrary.In(Kind, nameof(Kind));
        MediusLibrary.Text(Product, raw.product, Constants.MaxProduct);
        return raw;
    }
}

/// <summary>a box a listing found. <see cref="Device"/> is null for a box on another control protocol, opening it
/// throws <see cref="BadProtoVerException"/></summary>
public sealed partial record BoxInfo(
    PortInfo Port,
    BoxVersion Version,
    DeviceInfo? Device = null)
{
    internal static unsafe BoxInfo From(MediusBoxInfo raw) => new(
        PortInfo.From(raw.port),
        BoxVersion.From(raw.version),
        raw.has_device != 0 ? DeviceInfo.From(raw.device) : null);

    internal unsafe MediusBoxInfo ToNative()
    {
        var raw = new MediusBoxInfo();
        raw.port = Port.ToNative();
        raw.version = Version.ToNative();
        raw.has_device = MediusLibrary.Flag(Device is not null);
        if (Device is not null)
        {
            raw.device = Device.ToNative();
        }
        return raw;
    }
}

/// <summary>the real device's reply to a control transfer. a status that isn't ok is the protocol's answer and not a
/// link error, and it carries no data</summary>
public sealed partial record TransferOutcome(
    TransferStatus Status,
    ReadOnlyMemory<byte> Data = default)
{
    public bool Equals(TransferOutcome? other) => other is not null &&
        Status == other.Status &&
        MediusLibrary.Same(Data, other.Data);

    public override int GetHashCode()
    {
        var hash = new HashCode();
        hash.Add(Status);
        hash.Add(MediusLibrary.Hash(Data));
        return hash.ToHashCode();
    }

    internal static unsafe TransferOutcome From(MediusTransferOutcome raw) => new(
        (TransferStatus)raw.status,
        MediusLibrary.Bytes(raw.data, raw.len, Constants.MaxDevPayload));

    internal unsafe MediusTransferOutcome ToNative()
    {
        var raw = new MediusTransferOutcome();
        raw.status = (byte)Status;
        raw.len = (ushort)MediusLibrary.Copy(Data.Span, raw.data, Constants.MaxDevPayload, nameof(Data));
        return raw;
    }
}

/// <summary>a rewrite rule, keyed by its class, id, direction, match and mask. the match and mask compare the masked
/// head and are as long as each other, an empty match takes every packet on the address. <see cref="Payload"/> is for
/// the actions that carry one, and <see cref="Offset"/> is where a patch writes</summary>
public sealed partial record RewriteRule(
    RewriteClass RewriteClass = RewriteClass.Emit,
    ushort Id = 0,
    Direction Direction = Direction.Both,
    RewriteAction Action = RewriteAction.Pass,
    ushort Offset = 0,
    ReadOnlyMemory<byte> MatchBytes = default,
    ReadOnlyMemory<byte> Mask = default,
    ReadOnlyMemory<byte> Payload = default)
{
    /// <summary>what Rust's Default gives</summary>
    public static RewriteRule Default { get; } = new(RewriteClass.Emit, 0, Direction.Both, RewriteAction.Pass, 0, default, default, default);

    public bool Equals(RewriteRule? other) => other is not null &&
        RewriteClass == other.RewriteClass &&
        Id == other.Id &&
        Direction == other.Direction &&
        Action == other.Action &&
        Offset == other.Offset &&
        MediusLibrary.Same(MatchBytes, other.MatchBytes) &&
        MediusLibrary.Same(Mask, other.Mask) &&
        MediusLibrary.Same(Payload, other.Payload);

    public override int GetHashCode()
    {
        var hash = new HashCode();
        hash.Add(RewriteClass);
        hash.Add(Id);
        hash.Add(Direction);
        hash.Add(Action);
        hash.Add(Offset);
        hash.Add(MediusLibrary.Hash(MatchBytes));
        hash.Add(MediusLibrary.Hash(Mask));
        hash.Add(MediusLibrary.Hash(Payload));
        return hash.ToHashCode();
    }

    internal static unsafe RewriteRule From(MediusRewriteRule raw) => new(
        MediusLibrary.Out((RewriteClass)raw.class_, "a rewrite class"),
        raw.id,
        MediusLibrary.Out((Direction)raw.direction, "a direction"),
        MediusLibrary.Out((RewriteAction)raw.action, "a rewrite action"),
        raw.offset,
        MediusLibrary.Bytes(raw.match_bytes, raw.match_len, Constants.MaxRewriteMatch),
        MediusLibrary.Bytes(raw.mask, raw.mask_len, Constants.MaxRewriteMatch),
        MediusLibrary.Bytes(raw.payload, raw.payload_len, Constants.MaxDevPayload));

    internal unsafe MediusRewriteRule ToNative()
    {
        var raw = new MediusRewriteRule();
        raw.class_ = MediusLibrary.In(RewriteClass, nameof(RewriteClass));
        raw.id = Id;
        raw.direction = MediusLibrary.In(Direction, nameof(Direction));
        raw.action = MediusLibrary.In(Action, nameof(Action));
        raw.offset = Offset;
        raw.match_len = (ushort)MediusLibrary.Match(MatchBytes.Span, raw.match_bytes, Constants.MaxRewriteMatch, nameof(MatchBytes));
        raw.mask_len = (ushort)MediusLibrary.Match(Mask.Span, raw.mask, Constants.MaxRewriteMatch, nameof(Mask));
        raw.payload_len = (ushort)MediusLibrary.Copy(Payload.Span, raw.payload, Constants.MaxDevPayload, nameof(Payload));
        return raw;
    }
}

/// <summary>one row of the rewrite table, a rule's address, action and live counters without its bytes</summary>
public sealed partial record RewriteEntry(
    RewriteClass RewriteClass = RewriteClass.Emit,
    ushort Id = 0,
    Direction Direction = Direction.Both,
    RewriteAction Action = RewriteAction.Pass,
    byte MatchLen = 0,
    ushort Offset = 0,
    ushort PayloadLen = 0,
    ushort Hits = 0)
{
    internal static unsafe RewriteEntry From(MediusRewriteEntry raw) => new(
        MediusLibrary.Out((RewriteClass)raw.class_, "a rewrite class"),
        raw.id,
        MediusLibrary.Out((Direction)raw.direction, "a direction"),
        MediusLibrary.Out((RewriteAction)raw.action, "a rewrite action"),
        raw.match_len,
        raw.offset,
        raw.payload_len,
        raw.hits);

    internal unsafe MediusRewriteEntry ToNative()
    {
        var raw = new MediusRewriteEntry();
        raw.class_ = MediusLibrary.In(RewriteClass, nameof(RewriteClass));
        raw.id = Id;
        raw.direction = MediusLibrary.In(Direction, nameof(Direction));
        raw.action = MediusLibrary.In(Action, nameof(Action));
        raw.match_len = MatchLen;
        raw.offset = Offset;
        raw.payload_len = PayloadLen;
        raw.hits = Hits;
        return raw;
    }
}

/// <summary>the rewrite table in the order it was installed. <see cref="Generation"/> moves on every change,
/// <see cref="TableFull"/> says the box refused the last new rule for room</summary>
public sealed partial record RewriteTable(
    bool TableFull,
    byte Generation,
    IReadOnlyList<RewriteEntry> Entries)
{
    /// <summary>what Rust's Default gives</summary>
    public static RewriteTable Default { get; } = new(false, 0, []);

    public bool Equals(RewriteTable? other) => other is not null &&
        TableFull == other.TableFull &&
        Generation == other.Generation &&
        MediusLibrary.Same(Entries, other.Entries);

    public override int GetHashCode()
    {
        var hash = new HashCode();
        hash.Add(TableFull);
        hash.Add(Generation);
        hash.Add(MediusLibrary.Hash(Entries));
        return hash.ToHashCode();
    }

    internal static unsafe RewriteTable From(MediusRewriteTable raw) => new(
        raw.table_full != 0,
        raw.generation,
        MediusLibrary.ListOf<MediusRewriteEntry, RewriteEntry>(raw.entries, raw.n, RewriteEntry.From));

    internal unsafe MediusRewriteTable ToNative()
    {
        var raw = new MediusRewriteTable();
        raw.table_full = MediusLibrary.Flag(TableFull);
        raw.generation = Generation;
        raw.n = (ushort)MediusLibrary.Fill<RewriteEntry, MediusRewriteEntry>(Entries, raw.entries, nameof(Entries), item => item.ToNative());
        return raw;
    }
}

/// <summary>a descriptor patch, keyed by its section, configuration, index and offset. <see cref="Bytes"/> overwrites
/// the descriptor from <see cref="Offset"/>, empty removes the patch at that key. <see cref="Cfg"/> is the
/// configuration's index, 0 is the first one and not bConfigurationValue</summary>
public sealed partial record Patch(
    PatchSection Section = PatchSection.Device,
    byte Cfg = 0,
    byte Index = 0,
    ushort Offset = 0,
    ReadOnlyMemory<byte> Bytes = default)
{
    /// <summary>what Rust's Default gives</summary>
    public static Patch Default { get; } = new(PatchSection.Device, 0, 0, 0, default);

    public bool Equals(Patch? other) => other is not null &&
        Section == other.Section &&
        Cfg == other.Cfg &&
        Index == other.Index &&
        Offset == other.Offset &&
        MediusLibrary.Same(Bytes, other.Bytes);

    public override int GetHashCode()
    {
        var hash = new HashCode();
        hash.Add(Section);
        hash.Add(Cfg);
        hash.Add(Index);
        hash.Add(Offset);
        hash.Add(MediusLibrary.Hash(Bytes));
        return hash.ToHashCode();
    }

    internal static unsafe Patch From(MediusPatch raw) => new(
        MediusLibrary.Out((PatchSection)raw.section, "a patch section"),
        raw.cfg,
        raw.index,
        raw.offset,
        MediusLibrary.Bytes(raw.bytes, raw.len, Constants.MaxDevPayload));

    internal unsafe MediusPatch ToNative()
    {
        var raw = new MediusPatch();
        raw.section = MediusLibrary.In(Section, nameof(Section));
        raw.cfg = Cfg;
        raw.index = Index;
        raw.offset = Offset;
        raw.len = (ushort)MediusLibrary.Copy(Bytes.Span, raw.bytes, Constants.MaxDevPayload, nameof(Bytes));
        return raw;
    }
}

/// <summary>one stored patch's key and length, without its bytes</summary>
public sealed partial record PatchEntry(
    PatchSection Section = PatchSection.Device,
    byte Cfg = 0,
    byte Index = 0,
    ushort Offset = 0,
    ushort Len = 0)
{
    internal static unsafe PatchEntry From(MediusPatchEntry raw) => new(
        MediusLibrary.Out((PatchSection)raw.section, "a patch section"),
        raw.cfg,
        raw.index,
        raw.offset,
        raw.len);

    internal unsafe MediusPatchEntry ToNative()
    {
        var raw = new MediusPatchEntry();
        raw.section = MediusLibrary.In(Section, nameof(Section));
        raw.cfg = Cfg;
        raw.index = Index;
        raw.offset = Offset;
        raw.len = Len;
        return raw;
    }
}

/// <summary>the stored patch set and how it's applied. <see cref="Applied"/> is the clone serving a patched set,
/// <see cref="Pending"/> the stored set being different from the served one, <see cref="Refused"/> the stored set
/// failing a check the plain descriptors pass so the device is served unpatched, and <see cref="TableFull"/> the box
/// refusing the last new patch for room</summary>
public sealed partial record PatchSet(
    bool Applied,
    bool Pending,
    bool Refused,
    bool TableFull,
    IReadOnlyList<PatchEntry> Entries)
{
    /// <summary>what Rust's Default gives</summary>
    public static PatchSet Default { get; } = new(false, false, false, false, []);

    public bool Equals(PatchSet? other) => other is not null &&
        Applied == other.Applied &&
        Pending == other.Pending &&
        Refused == other.Refused &&
        TableFull == other.TableFull &&
        MediusLibrary.Same(Entries, other.Entries);

    public override int GetHashCode()
    {
        var hash = new HashCode();
        hash.Add(Applied);
        hash.Add(Pending);
        hash.Add(Refused);
        hash.Add(TableFull);
        hash.Add(MediusLibrary.Hash(Entries));
        return hash.ToHashCode();
    }

    internal static unsafe PatchSet From(MediusPatchSet raw) => new(
        raw.applied != 0,
        raw.pending != 0,
        raw.refused != 0,
        raw.table_full != 0,
        MediusLibrary.ListOf<MediusPatchEntry, PatchEntry>(raw.entries, raw.n, PatchEntry.From));

    internal unsafe MediusPatchSet ToNative()
    {
        var raw = new MediusPatchSet();
        raw.applied = MediusLibrary.Flag(Applied);
        raw.pending = MediusLibrary.Flag(Pending);
        raw.refused = MediusLibrary.Flag(Refused);
        raw.table_full = MediusLibrary.Flag(TableFull);
        raw.n = (ushort)MediusLibrary.Fill<PatchEntry, MediusPatchEntry>(Entries, raw.entries, nameof(Entries), item => item.ToNative());
        return raw;
    }
}

/// <summary>one field transform, keyed by its source and destination. it swaps or remaps a field the clone already has,
/// so every report is one the real device could send and it needs no imperfect clone opt in. to weigh or reverse a
/// field use <c>Device.Scale</c>, which runs first</summary>
public sealed partial record Transform(
    TransformOp Op,
    LockTarget Source,
    LockTarget Dest)
{
    internal static unsafe Transform From(MediusTransform raw) => new(
        MediusLibrary.Out((TransformOp)raw.op, "a transform op"),
        LockTarget.From(raw.source),
        LockTarget.From(raw.dest));

    internal unsafe MediusTransform ToNative()
    {
        var raw = new MediusTransform();
        raw.op = MediusLibrary.In(Op, nameof(Op));
        raw.source = Source.ToNative();
        raw.dest = Dest.ToNative();
        return raw;
    }
}

/// <summary>the transform table in the order it was installed, each entry replays as a
/// <c>Device.Transform</c></summary>
public sealed partial record Transforms(
    bool TableFull,
    IReadOnlyList<Transform> Entries)
{
    /// <summary>what Rust's Default gives</summary>
    public static Transforms Default { get; } = new(false, []);

    public bool Equals(Transforms? other) => other is not null &&
        TableFull == other.TableFull &&
        MediusLibrary.Same(Entries, other.Entries);

    public override int GetHashCode()
    {
        var hash = new HashCode();
        hash.Add(TableFull);
        hash.Add(MediusLibrary.Hash(Entries));
        return hash.ToHashCode();
    }

    internal static unsafe Transforms From(MediusTransforms raw) => new(
        raw.table_full != 0,
        MediusLibrary.ListOf<MediusTransform, Transform>(raw.entries, raw.n, Transform.From));

    internal unsafe MediusTransforms ToNative()
    {
        var raw = new MediusTransforms();
        raw.table_full = MediusLibrary.Flag(TableFull);
        raw.n = (ushort)MediusLibrary.Fill<Transform, MediusTransform>(Entries, raw.entries, nameof(Entries), item => item.ToNative());
        return raw;
    }
}

/// <summary>one chip's firmware version and the slot it booted</summary>
public sealed partial record ChipFirmware(
    byte Major,
    byte Minor,
    byte Patch,
    byte Slot,
    ImageState State)
{
    internal static unsafe ChipFirmware From(MediusChipFirmware raw) => new(
        raw.major,
        raw.minor,
        raw.patch,
        raw.slot,
        (ImageState)raw.state);

    internal unsafe MediusChipFirmware ToNative()
    {
        var raw = new MediusChipFirmware();
        raw.major = Major;
        raw.minor = Minor;
        raw.patch = Patch;
        raw.slot = Slot;
        raw.state = MediusLibrary.In(State, nameof(State));
        return raw;
    }
}

/// <summary>both chips' firmware. <see cref="Host"/> is null while the host chip hasn't answered over the link between
/// the chips</summary>
public sealed partial record FirmwareInfo(
    ChipFirmware Device,
    ChipFirmware? Host = null,
    uint SlotSize = 0,
    bool DeviceStaged = false,
    bool HostStaged = false)
{
    internal static unsafe FirmwareInfo From(MediusFirmwareInfo raw) => new(
        ChipFirmware.From(raw.device),
        raw.host_present != 0 ? ChipFirmware.From(raw.host) : null,
        raw.slot_size,
        raw.device_staged != 0,
        raw.host_staged != 0);

    internal unsafe MediusFirmwareInfo ToNative()
    {
        var raw = new MediusFirmwareInfo();
        raw.device = Device.ToNative();
        raw.host_present = MediusLibrary.Flag(Host is not null);
        if (Host is not null)
        {
            raw.host = Host.ToNative();
        }
        raw.slot_size = SlotSize;
        raw.device_staged = MediusLibrary.Flag(DeviceStaged);
        raw.host_staged = MediusLibrary.Flag(HostStaged);
        return raw;
    }
}

/// <summary>the box's health flags</summary>
public sealed partial record Health(
    bool LinkUp = false,
    bool MouseAttached = false,
    bool CloneConfigured = false,
    bool InjectionActive = false,
    bool RateConfident = false,
    bool LockOn = false,
    bool CatchOn = false,
    bool KbdAttached = false,
    bool RewriteOn = false,
    bool PatchOn = false,
    bool TransformOn = false)
{
    internal static unsafe Health From(MediusHealth raw) => new(
        raw.link_up != 0,
        raw.mouse_attached != 0,
        raw.clone_configured != 0,
        raw.injection_active != 0,
        raw.rate_confident != 0,
        raw.lock_on != 0,
        raw.catch_on != 0,
        raw.kbd_attached != 0,
        raw.rewrite_on != 0,
        raw.patch_on != 0,
        raw.transform_on != 0);

    internal unsafe MediusHealth ToNative()
    {
        var raw = new MediusHealth();
        raw.link_up = MediusLibrary.Flag(LinkUp);
        raw.mouse_attached = MediusLibrary.Flag(MouseAttached);
        raw.clone_configured = MediusLibrary.Flag(CloneConfigured);
        raw.injection_active = MediusLibrary.Flag(InjectionActive);
        raw.rate_confident = MediusLibrary.Flag(RateConfident);
        raw.lock_on = MediusLibrary.Flag(LockOn);
        raw.catch_on = MediusLibrary.Flag(CatchOn);
        raw.kbd_attached = MediusLibrary.Flag(KbdAttached);
        raw.rewrite_on = MediusLibrary.Flag(RewriteOn);
        raw.patch_on = MediusLibrary.Flag(PatchOn);
        raw.transform_on = MediusLibrary.Flag(TransformOn);
        return raw;
    }
}

/// <summary>what the cloned mouse has</summary>
public sealed partial record MouseCaps(
    byte NButtons = 0,
    bool HasX = false,
    bool HasY = false,
    bool HasWheel = false,
    bool Pan = false,
    bool HasReportId = false,
    byte NHid = 0)
{
    /// <summary>what Rust's Default gives</summary>
    public static MouseCaps Default { get; } = new(0, false, false, false, false, false, 0);

    internal static unsafe MouseCaps From(MediusMouseCaps raw) => new(
        raw.n_buttons,
        raw.has_x != 0,
        raw.has_y != 0,
        raw.has_wheel != 0,
        raw.pan != 0,
        raw.has_report_id != 0,
        raw.n_hid);

    internal unsafe MediusMouseCaps ToNative()
    {
        var raw = new MediusMouseCaps();
        raw.n_buttons = NButtons;
        raw.has_x = MediusLibrary.Flag(HasX);
        raw.has_y = MediusLibrary.Flag(HasY);
        raw.has_wheel = MediusLibrary.Flag(HasWheel);
        raw.pan = MediusLibrary.Flag(Pan);
        raw.has_report_id = MediusLibrary.Flag(HasReportId);
        raw.n_hid = NHid;
        return raw;
    }
}

/// <summary>what the cloned keyboard has, <see cref="NKeys"/> 0xFF is an NKRO bitmap</summary>
public sealed partial record KbdCaps(
    byte NKeys = 0,
    bool Nkro = false,
    bool HasConsumer = false,
    bool HasSystem = false,
    bool HasReportId = false)
{
    /// <summary>what Rust's Default gives</summary>
    public static KbdCaps Default { get; } = new(0, false, false, false, false);

    internal static unsafe KbdCaps From(MediusKbdCaps raw) => new(
        raw.n_keys,
        raw.nkro != 0,
        raw.has_consumer != 0,
        raw.has_system != 0,
        raw.has_report_id != 0);

    internal unsafe MediusKbdCaps ToNative()
    {
        var raw = new MediusKbdCaps();
        raw.n_keys = NKeys;
        raw.nkro = MediusLibrary.Flag(Nkro);
        raw.has_consumer = MediusLibrary.Flag(HasConsumer);
        raw.has_system = MediusLibrary.Flag(HasSystem);
        raw.has_report_id = MediusLibrary.Flag(HasReportId);
        return raw;
    }
}

/// <summary>what the cloned device has</summary>
public sealed partial record Caps(
    MouseCaps Mouse,
    KbdCaps Keyboard,
    bool MouseChangeDriven = false,
    bool KbdChangeDriven = false)
{
    /// <summary>what Rust's Default gives</summary>
    public static Caps Default { get; } = new(MouseCaps.Default, KbdCaps.Default, false, false);

    internal static unsafe Caps From(MediusCaps raw) => new(
        MouseCaps.From(raw.mouse),
        KbdCaps.From(raw.keyboard),
        raw.mouse_change_driven != 0,
        raw.kbd_change_driven != 0);

    internal unsafe MediusCaps ToNative()
    {
        var raw = new MediusCaps();
        raw.mouse = Mouse.ToNative();
        raw.keyboard = Keyboard.ToNative();
        raw.mouse_change_driven = MediusLibrary.Flag(MouseChangeDriven);
        raw.kbd_change_driven = MediusLibrary.Flag(KbdChangeDriven);
        return raw;
    }
}

/// <summary>the device's report rate the box measured, and the clone's poll period</summary>
public sealed partial record Rate(
    ushort NativePeriodUs = 0,
    ushort PollPeriodUs = 0,
    bool Confident = false,
    bool ChangeDriven = false)
{
    internal static unsafe Rate From(MediusRate raw) => new(
        raw.native_period_us,
        raw.poll_period_us,
        raw.confident != 0,
        raw.change_driven != 0);

    internal unsafe MediusRate ToNative()
    {
        var raw = new MediusRate();
        raw.native_period_us = NativePeriodUs;
        raw.poll_period_us = PollPeriodUs;
        raw.confident = MediusLibrary.Flag(Confident);
        raw.change_driven = MediusLibrary.Flag(ChangeDriven);
        return raw;
    }
}

/// <summary>the box's delivery counters. the narrow ones saturate instead of wrapping, the three drop counters don't.
/// <see cref="TxDrops"/>, <see cref="LinkRxDrops"/> and <see cref="HostRxDrops"/> are lost physical input and should
/// stay 0, <see cref="RelayDrops"/> is relayed traffic and commands, never physical input. <see cref="Session"/> counts
/// the times the box let go of what the PC set and wraps, so compare it for inequality</summary>
public sealed partial record Stats(
    uint InjectEmits = 0,
    ushort TxDrops = 0,
    ushort TxMerges = 0,
    byte TxMaxdepth = 0,
    byte TxWedges = 0,
    ushort Wakeups = 0,
    ushort ResetCount = 0,
    ushort ConfigCount = 0,
    uint LinkRxDrops = 0,
    uint HostRxDrops = 0,
    uint RelayDrops = 0,
    ushort Session = 0)
{
    internal static unsafe Stats From(MediusStats raw) => new(
        raw.inject_emits,
        raw.tx_drops,
        raw.tx_merges,
        raw.tx_maxdepth,
        raw.tx_wedges,
        raw.wakeups,
        raw.reset_count,
        raw.config_count,
        raw.link_rx_drops,
        raw.host_rx_drops,
        raw.relay_drops,
        raw.session);

    internal unsafe MediusStats ToNative()
    {
        var raw = new MediusStats();
        raw.inject_emits = InjectEmits;
        raw.tx_drops = TxDrops;
        raw.tx_merges = TxMerges;
        raw.tx_maxdepth = TxMaxdepth;
        raw.tx_wedges = TxWedges;
        raw.wakeups = Wakeups;
        raw.reset_count = ResetCount;
        raw.config_count = ConfigCount;
        raw.link_rx_drops = LinkRxDrops;
        raw.host_rx_drops = HostRxDrops;
        raw.relay_drops = RelayDrops;
        raw.session = Session;
        return raw;
    }
}

/// <summary>one weighed direction. <see cref="Scale"/> is the percent of the physical value kept, 0 blocks, 100 passes,
/// above 100 amplifies and a negative one reverses what it keeps. it's the scale the box applies, which in
/// <c>BearingMode.Vector</c> can differ from the one sent</summary>
public sealed partial record LockEntry(
    LockTarget Target,
    bool IsBlanket = false,
    Direction Direction = Direction.Both,
    short Scale = 0)
{
    internal static unsafe LockEntry From(MediusLockEntry raw) => new(
        LockTarget.From(raw.target),
        raw.is_blanket != 0,
        MediusLibrary.Out((Direction)raw.direction, "a direction"),
        raw.scale);

    internal unsafe MediusLockEntry ToNative()
    {
        var raw = new MediusLockEntry();
        raw.target = Target.ToNative();
        raw.is_blanket = MediusLibrary.Flag(IsBlanket);
        raw.direction = MediusLibrary.In(Direction, nameof(Direction));
        raw.scale = Scale;
        return raw;
    }
}

/// <summary>the locks and scales the box holds</summary>
public sealed partial record Locks(
    IReadOnlyList<LockEntry> Entries)
{
    public bool Equals(Locks? other) => other is not null &&
        MediusLibrary.Same(Entries, other.Entries);

    public override int GetHashCode()
    {
        var hash = new HashCode();
        hash.Add(MediusLibrary.Hash(Entries));
        return hash.ToHashCode();
    }

    internal static unsafe Locks From(MediusLocks raw) => new(
        MediusLibrary.ListOf<MediusLockEntry, LockEntry>(raw.entries, raw.n, LockEntry.From));

    internal unsafe MediusLocks ToNative()
    {
        var raw = new MediusLocks();
        raw.n = (ushort)MediusLibrary.Fill<LockEntry, MediusLockEntry>(Entries, raw.entries, nameof(Entries), item => item.ToNative());
        return raw;
    }
}

/// <summary>the measured offset between the two chips' clocks. <see cref="RatePpb"/> is null when the box fitted no
/// rate, <see cref="AgeMs"/> null when it never measured, which a zero offset looks like too</summary>
public sealed partial record ClockEstimate(
    int OffsetUs = 0,
    int? RatePpb = null,
    ushort DelayUs = 0,
    uint? AgeMs = null)
{
    internal static unsafe ClockEstimate From(MediusClockEstimate raw) => new(
        raw.offset_us,
        raw.rate_ppb == Constants.ClockRateNone ? null : raw.rate_ppb,
        raw.delay_us,
        raw.age_ms == Constants.ClockAgeNone ? null : raw.age_ms);

    internal unsafe MediusClockEstimate ToNative()
    {
        var raw = new MediusClockEstimate();
        raw.offset_us = OffsetUs;
        raw.rate_ppb = (int)(RatePpb ?? Constants.ClockRateNone);
        raw.delay_us = DelayUs;
        raw.age_ms = (uint)(AgeMs ?? Constants.ClockAgeNone);
        return raw;
    }
}

/// <summary>one row of the box's subscription table, a live subscription and its drop count</summary>
public sealed partial record CatchEntry(
    CatchFilter Filter,
    ushort Dropped = 0)
{
    internal static unsafe CatchEntry From(MediusCatchEntry raw) => new(
        CatchFilter.From(raw.filter),
        raw.dropped);

    internal unsafe MediusCatchEntry ToNative()
    {
        var raw = new MediusCatchEntry();
        raw.filter = Filter.ToNative();
        raw.dropped = Dropped;
        return raw;
    }
}

/// <summary>the live subscription table, its drop counts and the clock estimate between the chips</summary>
public sealed partial record CatchState(
    bool TableFull,
    uint Dropped,
    ClockEstimate Clock,
    IReadOnlyList<CatchEntry> Entries)
{
    public bool Equals(CatchState? other) => other is not null &&
        TableFull == other.TableFull &&
        Dropped == other.Dropped &&
        Clock == other.Clock &&
        MediusLibrary.Same(Entries, other.Entries);

    public override int GetHashCode()
    {
        var hash = new HashCode();
        hash.Add(TableFull);
        hash.Add(Dropped);
        hash.Add(Clock);
        hash.Add(MediusLibrary.Hash(Entries));
        return hash.ToHashCode();
    }

    internal static unsafe CatchState From(MediusCatchState raw) => new(
        raw.table_full != 0,
        raw.dropped,
        ClockEstimate.From(raw.clock),
        MediusLibrary.ListOf<MediusCatchEntry, CatchEntry>(raw.entries, raw.n, CatchEntry.From));

    internal unsafe MediusCatchState ToNative()
    {
        var raw = new MediusCatchState();
        raw.table_full = MediusLibrary.Flag(TableFull);
        raw.dropped = Dropped;
        raw.clock = Clock.ToNative();
        raw.n = (ushort)MediusLibrary.Fill<CatchEntry, MediusCatchEntry>(Entries, raw.entries, nameof(Entries), item => item.ToNative());
        return raw;
    }
}

/// <summary>the imperfect clone opt in, and whether the clone needs it</summary>
public sealed partial record ImperfectStatus(
    bool Allowed = false,
    bool OverCapacity = false,
    bool CloneImperfect = false)
{
    /// <summary>what Rust's Default gives</summary>
    public static ImperfectStatus Default { get; } = new(false, false, false);

    internal static unsafe ImperfectStatus From(MediusImperfectStatus raw) => new(
        raw.allowed != 0,
        raw.over_capacity != 0,
        raw.clone_imperfect != 0);

    internal unsafe MediusImperfectStatus ToNative()
    {
        var raw = new MediusImperfectStatus();
        raw.allowed = MediusLibrary.Flag(Allowed);
        raw.over_capacity = MediusLibrary.Flag(OverCapacity);
        raw.clone_imperfect = MediusLibrary.Flag(CloneImperfect);
        return raw;
    }
}

/// <summary>what <c>Direction.With</c> and <c>Direction.Against</c> are measured against. <see cref="WindowMs"/> is how
/// long the last injected direction stays the bearing, null turns it off and leaves the relative directions doing
/// nothing</summary>
public sealed partial record Bearing(
    ushort? WindowMs = Constants.BearingWindowDefaultMs,
    BearingMode Mode = BearingMode.PerAxis)
{
    /// <summary>what Rust's Default gives</summary>
    public static Bearing Default { get; } = new(Constants.BearingWindowDefaultMs, BearingMode.PerAxis);

    internal static unsafe Bearing From(MediusBearing raw) => new(
        raw.window_ms == 0 ? null : raw.window_ms,
        MediusLibrary.Out(raw.mode, "a bearing mode"));

    internal unsafe MediusBearing ToNative()
    {
        var raw = new MediusBearing();
        raw.window_ms = (ushort)(WindowMs ?? 0);
        raw.mode = MediusLibrary.Named(Mode, nameof(Mode));
        return raw;
    }
}

/// <summary>the render mode, whether physical motion goes through it, and whether the box learned the attached device's
/// profile yet</summary>
public sealed partial record RenderStatus(
    RenderMode Mode = RenderMode.Despiked,
    bool Full = false,
    bool Ready = false)
{
    /// <summary>what Rust's Default gives</summary>
    public static RenderStatus Default { get; } = new(RenderMode.Despiked, false, false);

    internal static unsafe RenderStatus From(MediusRenderStatus raw) => new(
        MediusLibrary.Out(raw.mode, "a render mode"),
        raw.full != 0,
        raw.ready != 0);

    internal unsafe MediusRenderStatus ToNative()
    {
        var raw = new MediusRenderStatus();
        raw.mode = MediusLibrary.Named(Mode, nameof(Mode));
        raw.full = MediusLibrary.Flag(Full);
        raw.ready = MediusLibrary.Flag(Ready);
        return raw;
    }
}

/// <summary>the injection spread, a percent of the host's command interval, and the span the box releases
/// across</summary>
public sealed partial record SpreadStatus(
    ushort Percent = 0,
    uint SpanUs = 0)
{
    /// <summary>what Rust's Default gives</summary>
    public static SpreadStatus Default { get; } = new(0, 0);

    internal static unsafe SpreadStatus From(MediusSpreadStatus raw) => new(
        raw.percent,
        raw.span_us);

    internal unsafe MediusSpreadStatus ToNative()
    {
        var raw = new MediusSpreadStatus();
        raw.percent = Percent;
        raw.span_us = SpanUs;
        return raw;
    }
}

/// <summary>what the library counted on this side. <see cref="Restarts"/> is the device chip restarts it got over by
/// sending its held state again</summary>
public sealed partial record Counters(
    ulong FramesTx = 0,
    ulong FramesRx = 0,
    ulong CrcDrops = 0,
    ulong Reconnects = 0,
    ulong Restarts = 0)
{
    /// <summary>what Rust's Default gives</summary>
    public static Counters Default { get; } = new(0, 0, 0, 0, 0);

    internal static unsafe Counters From(MediusCountersSnapshot raw) => new(
        raw.frames_tx,
        raw.frames_rx,
        raw.crc_drops,
        raw.reconnects,
        raw.restarts);

    internal unsafe MediusCountersSnapshot ToNative()
    {
        var raw = new MediusCountersSnapshot();
        raw.frames_tx = FramesTx;
        raw.frames_rx = FramesRx;
        raw.crc_drops = CrcDrops;
        raw.reconnects = Reconnects;
        raw.restarts = Restarts;
        return raw;
    }
}

/// <summary>every usage of one class held right now. the class and direction come from the frame, so the empty snapshot
/// of the last release still has them</summary>
public sealed partial record UsageSnapshot(
    Class Class,
    Direction Direction,
    IReadOnlyList<Usage> Usages)
{
    public bool Equals(UsageSnapshot? other) => other is not null &&
        Class == other.Class &&
        Direction == other.Direction &&
        MediusLibrary.Same(Usages, other.Usages);

    public override int GetHashCode()
    {
        var hash = new HashCode();
        hash.Add(Class);
        hash.Add(Direction);
        hash.Add(MediusLibrary.Hash(Usages));
        return hash.ToHashCode();
    }

    internal static unsafe UsageSnapshot From(MediusUsageEvent raw) => new(
        MediusLibrary.Out((Class)raw.class_, "a class"),
        MediusLibrary.Out((Direction)raw.direction, "a direction"),
        MediusLibrary.ListOf<MediusUsage, Usage>(raw.usages, raw.n, Usage.From));

    internal unsafe MediusUsageEvent ToNative()
    {
        var raw = new MediusUsageEvent();
        raw.class_ = MediusLibrary.In(Class, nameof(Class));
        raw.direction = MediusLibrary.In(Direction, nameof(Direction));
        raw.n = (ushort)MediusLibrary.Fill<Usage, MediusUsage>(Usages, raw.usages, nameof(Usages), item => item.ToNative());
        return raw;
    }
}

/// <summary>one byte oriented catch event, HID reports, vendor endpoints, control transactions, what the clone sent,
/// bus lifecycle, or a clip's control transfers. <see cref="Bytes"/> is what the capture kept and <see cref="TrueLen"/>
/// the length before it was cut, so set both when you build one by hand</summary>
public sealed partial record TrafficEvent(
    CatchClass CatchClass,
    ushort Id = 0,
    Direction Direction = Direction.Both,
    byte Flags = 0,
    ushort TrueLen = 0,
    ReadOnlyMemory<byte> Bytes = default)
{
    public bool Equals(TrafficEvent? other) => other is not null &&
        CatchClass == other.CatchClass &&
        Id == other.Id &&
        Direction == other.Direction &&
        Flags == other.Flags &&
        TrueLen == other.TrueLen &&
        MediusLibrary.Same(Bytes, other.Bytes);

    public override int GetHashCode()
    {
        var hash = new HashCode();
        hash.Add(CatchClass);
        hash.Add(Id);
        hash.Add(Direction);
        hash.Add(Flags);
        hash.Add(TrueLen);
        hash.Add(MediusLibrary.Hash(Bytes));
        return hash.ToHashCode();
    }

    internal static unsafe TrafficEvent From(MediusTrafficEvent raw) => new(
        MediusLibrary.Out((CatchClass)raw.class_, "a catch class"),
        raw.id,
        MediusLibrary.Out((Direction)raw.direction, "a direction"),
        raw.flags,
        raw.true_len,
        MediusLibrary.Bytes(raw.bytes, raw.len, Constants.MaxTrafficBytes));

    internal unsafe MediusTrafficEvent ToNative()
    {
        var raw = new MediusTrafficEvent();
        raw.class_ = MediusLibrary.In(CatchClass, nameof(CatchClass));
        raw.id = Id;
        raw.direction = MediusLibrary.In(Direction, nameof(Direction));
        raw.flags = Flags;
        raw.true_len = TrueLen;
        raw.len = (ushort)MediusLibrary.Copy(Bytes.Span, raw.bytes, Constants.MaxTrafficBytes, nameof(Bytes));
        return raw;
    }
}

/// <summary>a bus lifecycle event, the numbers are 0 for the kinds that have none</summary>
public sealed partial record BusEvent(
    BusEventKind Kind,
    byte Configuration = 0,
    byte Interface = 0,
    byte Alt = 0)
{
    internal static unsafe BusEvent From(MediusBusEvent raw) => new(
        MediusLibrary.Out(raw.kind, "a bus event kind"),
        raw.configuration,
        raw.@interface,
        raw.alt);

    internal unsafe MediusBusEvent ToNative()
    {
        var raw = new MediusBusEvent();
        raw.kind = MediusLibrary.Named(Kind, nameof(Kind));
        raw.configuration = Configuration;
        raw.@interface = Interface;
        raw.alt = Alt;
        return raw;
    }
}

/// <summary>physical motion at the merge point, pan is positive going right</summary>
public sealed partial record MotionEvent(
    short Dx = 0,
    short Dy = 0,
    short Dz = 0,
    short Pan = 0)
{
    internal static unsafe MotionEvent From(MediusMotionEvent raw) => new(
        raw.dx,
        raw.dy,
        raw.dz,
        raw.pan);

    internal unsafe MediusMotionEvent ToNative()
    {
        var raw = new MediusMotionEvent();
        raw.dx = Dx;
        raw.dy = Dy;
        raw.dz = Dz;
        raw.pan = Pan;
        return raw;
    }
}

/// <summary>an event a <c>Timeline</c> put on your clock, when it happened on the scale you passed in, its box stamp
/// unwrapped past the 32 bit rollover, and how much later than the measured floor it came, which is jitter and not
/// latency</summary>
public sealed partial record Stamped(
    ulong HostNs = 0,
    ulong BoxUs = 0,
    ulong ExcessNs = 0)
{
    internal static unsafe Stamped From(MediusStamped raw) => new(
        raw.host_ns,
        raw.box_us,
        raw.excess_ns);

    internal unsafe MediusStamped ToNative()
    {
        var raw = new MediusStamped();
        raw.host_ns = HostNs;
        raw.box_us = BoxUs;
        raw.excess_ns = ExcessNs;
        return raw;
    }
}

/// <summary>one line of the box's log</summary>
public sealed partial record LogLine(
    LogLevel Level,
    string Text = "")
{
    internal static unsafe LogLine From(MediusLogLine raw) => new(
        MediusLibrary.Out(raw.level, "a log level"),
        MediusLibrary.Text(new ReadOnlySpan<byte>(raw.text, Constants.MaxLogText)));

    internal unsafe MediusLogLine ToNative()
    {
        var raw = new MediusLogLine();
        raw.level = MediusLibrary.Named(Level, nameof(Level));
        MediusLibrary.Text(Text, raw.text, Constants.MaxLogText);
        return raw;
    }
}
