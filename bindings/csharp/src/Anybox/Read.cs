using System.Runtime.CompilerServices;

namespace Anybox;

// turns what the C side hands back into the public records, right away, so no record points into C memory
internal static unsafe class Read
{
    internal static ulong Micros(TimeSpan value, string what) => value < TimeSpan.Zero
        ? throw new ArgumentOutOfRangeException(what, $"{what} can't be negative")
        : (ulong)(value.Ticks / TimeSpan.TicksPerMicrosecond);

    static TimeSpan FromMicros(ulong micros) => TimeSpan.FromMicroseconds((long)micros);

    static TimeSpan FromNanos(ulong nanos) => TimeSpan.FromTicks((long)(nanos / 100));

    static AnyboxException Unknown(string what, uint value) =>
        new($"the library sent {what} this package doesn't know ({value}), so update the Anybox package");

    // a cast would take any number without a word, and a value this package has no name for means it's older
    // than the library
    internal static T Known<T>(uint value, string what) where T : struct, Enum
    {
        var known = Unsafe.BitCast<uint, T>(value);
        return Enum.IsDefined(known) ? known : throw Unknown(what, value);
    }

    static Stamp Stamp(in AnyboxEvent raw) => new(
        FromNanos(raw.since_open_ns),
        FromNanos(raw.received_since_open_ns),
        (long)raw.arrived_ns,
        (long)raw.received_ns,
        raw.has_received_qpc != 0 ? raw.received_qpc : null,
        raw.has_box_time != 0 ? new BoxTime(raw.box_time_us, Known<Clock>(raw.box_clock, "a box clock")) : null);

    internal static Event Event(in AnyboxEvent raw) => (EventKind)raw.kind switch
    {
        EventKind.Button => new ButtonEvent((Button)raw.button, raw.pressed != 0, Stamp(raw)),
        EventKind.Key => new KeyEvent((Key)raw.key, raw.pressed != 0, Stamp(raw)),
        EventKind.Media => new MediaEvent((MediaKey)raw.media, raw.pressed != 0, Stamp(raw)),
        EventKind.Motion => new MotionEvent(raw.dx, raw.dy, raw.wheel, raw.pan, Stamp(raw)),
        EventKind.Dropped => new Dropped(raw.count, Known<DropSource>(raw.source, "a drop source")),
        EventKind.SessionReset => new SessionReset(),
        EventKind.Rejected => new Rejected(Known<BoxAction>(raw.action, "an action"), raw.count),
        EventKind.Disconnected => new Disconnected(),
        EventKind.Reconnected => new Reconnected(),
        EventKind.Changed => new Changed(),
        _ => throw Unknown("an event", raw.kind),
    };

    // a firmware and an id come flattened into fields, the same way in AnyboxInfo and AnyboxErrorDetail
    static Firmware? Firmware(uint kind, ushort major, ushort minor, ushort patch, byte protocol, uint build, byte* text) =>
        kind switch
        {
            0 => null,
            (uint)FirmwareKind.Version => new(FirmwareKind.Version) { Version = new(major, minor, patch) },
            (uint)FirmwareKind.Build => new(FirmwareKind.Build) { Build = build },
            (uint)FirmwareKind.Protocol => new(FirmwareKind.Protocol) { Protocol = protocol },
            (uint)FirmwareKind.Text => new(FirmwareKind.Text) { Text = Library.Text(text) },
            _ => throw Unknown("a firmware kind", kind),
        };

    static StableId? Id(uint kind, byte* mac, byte* serial) => (IdKind)kind switch
    {
        IdKind.None => null,
        IdKind.Mac => StableId.FromMac(new ReadOnlySpan<byte>(mac, 6)),
        IdKind.UsbSerial => StableId.FromUsbSerial(Library.Text(serial) ?? ""),
        _ => throw Unknown("an id kind", kind),
    };

    internal static Info Info(in AnyboxInfo raw)
    {
        fixed (byte* mac = raw.id_mac)
        {
            return new(
                Library.Text(raw.port) ?? "",
                Known<Family>(raw.family, "a family"),
                Library.Text(raw.model),
                Firmware(
                    raw.firmware_kind,
                    raw.firmware_major,
                    raw.firmware_minor,
                    raw.firmware_patch,
                    raw.firmware_protocol,
                    raw.firmware_build,
                    raw.firmware_text),
                Id(raw.id_kind, mac, raw.id_serial),
                raw.ferrum_api == 0 ? null : Known<FerrumApi>(raw.ferrum_api, "a Ferrum API"),
                raw.has_peripheral != 0 ? new UsbIds(raw.peripheral_vid, raw.peripheral_pid) : null);
        }
    }

    static Family? OptionalFamily(uint value) => value == 0 ? null : Known<Family>(value, "a family");

    internal static ErrorFields Fields(in AnyboxErrorDetail raw)
    {
        fixed (byte* expectedMac = raw.expected_id_mac, foundMac = raw.found_id_mac)
        {
            var ferrum = raw.kind == 8;
            return new ErrorFields
            {
                Action = raw.has_action != 0 ? Known<BoxAction>(raw.action, "an action") : null,
                Family = OptionalFamily(raw.family),
                ExpectedFamily = OptionalFamily(raw.expected_family),
                FoundFamily = OptionalFamily(raw.found_family),
                Port = Library.Text(raw.port),
                Text = Library.Text(raw.text),
                ExpectedId = Id(raw.expected_id_kind, expectedMac, raw.expected_id_serial),
                FoundId = Id(raw.found_id_kind, foundMac, raw.found_id_serial),
                FoundFirmware = Firmware(
                    raw.found_firmware_kind,
                    raw.found_firmware_major,
                    raw.found_firmware_minor,
                    raw.found_firmware_patch,
                    raw.found_firmware_protocol,
                    raw.found_firmware_build,
                    raw.found_firmware_text),
                NeededFirmware = Firmware(
                    raw.needed_firmware_kind,
                    raw.needed_firmware_major,
                    raw.needed_firmware_minor,
                    raw.needed_firmware_patch,
                    raw.needed_firmware_protocol,
                    raw.needed_firmware_build,
                    raw.needed_firmware_text),
                ExpectedFerrumApi = ferrum ? Known<FerrumApi>(raw.expected_ferrum_api, "a Ferrum API") : null,
                FoundFerrumApi = ferrum ? Known<FerrumApi>(raw.found_ferrum_api, "a Ferrum API") : null,
                IoKind = raw.io_kind == 0 ? null : Known<IoKind>(raw.io_kind, "an I/O error kind"),
                OsError = raw.has_os_error != 0 ? raw.os_error : null,
            };
        }
    }

    internal static IReadOnlyList<string> Strings(StringListHandle list)
    {
        var count = Imports.anybox_string_list_count(list);
        var strings = new List<string>();
        for (nuint index = 0; index < count; index++)
        {
            strings.Add(Library.Text(Imports.anybox_string_list_get(list, index)) ?? "");
        }
        return strings;
    }

    // what the box on a scan list entry couldn't do
    internal static IReadOnlySet<BoxAction> Missing(ScanListHandle list, nuint index)
    {
        Library.Check(Imports.anybox_scan_list_missing(list, index, null, 0, out var length));
        var values = new uint[length];
        fixed (uint* first = values)
        {
            Library.Check(Imports.anybox_scan_list_missing(list, index, first, length, out length));
        }
        return values.Select(value => Known<BoxAction>(value, "an action")).ToHashSet();
    }

    internal static IReadOnlyList<Incapable> Incapable(ScanListHandle list)
    {
        var count = Imports.anybox_scan_list_count(list);
        var boxes = new List<Incapable>();
        for (nuint index = 0; index < count; index++)
        {
            Library.Check(Imports.anybox_scan_list_result(list, index, out var snapshot));
            boxes.Add(new Incapable(Snapshot(snapshot), Missing(list, index)));
        }
        return boxes;
    }

    internal static Quirk Quirk(in AnyboxQuirk raw)
    {
        var kind = Known<QuirkKind>(raw.kind, "a quirk");
        return kind switch
        {
            QuirkKind.ForcedUpOnRelease or QuirkKind.BoxClickTiming => new(kind)
            {
                Min = FromMicros(raw.min_us),
                Max = FromMicros(raw.max_us),
            },
            QuirkKind.MaxHold => new(kind) { Max = FromMicros(raw.max_us) },
            QuirkKind.NotchesPerCommand or QuirkKind.MaxKeysHeld => new(kind) { Count = raw.count },
            QuirkKind.HostClickTiming => new(kind) { ClickTiming = ClickTiming.From(raw.click_timing) },
            _ => new(kind),
        };
    }

    delegate uint ListCall(SnapshotHandle snapshot, uint* values, nuint cap, out nuint len);

    static uint[] List(SnapshotHandle snapshot, ListCall call)
    {
        Library.Check(call(snapshot, null, 0, out var length));
        var values = new uint[length];
        fixed (uint* first = values)
        {
            Library.Check(call(snapshot, first, length, out length));
        }
        return values;
    }

    // reads the whole snapshot handle and frees it, so nothing has to keep it alive
    internal static Snapshot Snapshot(SnapshotHandle handle)
    {
        using var snapshot = handle;
        Library.Check(Imports.anybox_snapshot_info(snapshot, out var rawInfo));
        var info = Info(rawInfo);
        var actions = new Dictionary<BoxAction, Capability>();
        foreach (var action in Enum.GetValues<BoxAction>())
        {
            Library.Check(Imports.anybox_snapshot_capability(snapshot, (uint)action, out var capability));
            var quirks = new Quirk[capability.quirk_count];
            for (uint index = 0; index < quirks.Length; index++)
            {
                Library.Check(Imports.anybox_snapshot_quirk(snapshot, (uint)action, index, out var quirk));
                quirks[index] = Quirk(quirk);
            }
            actions[action] = new Capability(
                Known<Execution>(capability.execution, "an execution"),
                Library.Text(capability.reason),
                quirks);
        }
        var buttons = Array.ConvertAll(List(snapshot, Imports.anybox_snapshot_buttons), value => (Button)value);
        var axes = Array.ConvertAll(List(snapshot, Imports.anybox_snapshot_axes), value => Known<Axis>(value, "an axis"));
        return new Snapshot(info, new Capabilities(actions, buttons, axes));
    }
}
