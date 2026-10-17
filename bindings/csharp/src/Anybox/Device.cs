using System.Collections;

namespace Anybox;

/// <summary>one open box. every method can be called from any thread at the same time. dispose it, or call
/// <see cref="Close"/>, so what you held gets let go of and the port gets closed</summary>
public sealed class Device : IDisposable
{
    readonly DeviceHandle handle;
    bool closed;

    internal Device(DeviceHandle handle) => this.handle = handle;

    /// <summary>connects to the first box found, in port order</summary>
    public static Device Connect() => new Connect().Open();

    /// <inheritdoc cref="Anybox.Connect.Scan"/>
    public static IReadOnlyList<Scanned> Scan() => new Connect().Scan();

    /// <inheritdoc cref="Anybox.Connect.Candidates"/>
    public static IReadOnlyList<string> Candidates() => new Connect().Candidates();

    static INative Native => Anybox.Native.Api;

    internal DeviceHandle Handle
    {
        get
        {
            ObjectDisposedException.ThrowIf(handle.IsClosed, this);
            return handle;
        }
    }

    static AnyboxClick Click(TimeSpan? hold, bool nativeOnly) => new()
    {
        hold_us = hold is { } value ? Read.Micros(value, "a hold") : 0,
        has_hold = hold is null ? (byte)0 : (byte)1,
        native_only = nativeOnly ? (byte)1 : (byte)0,
    };

    /// <summary>who the box is and what it can do, read from the current connection. it can change after
    /// <see cref="Reconnected"/> or <see cref="Changed"/>, and it still works after <see cref="Close"/></summary>
    public Snapshot Snapshot()
    {
        Library.Check(Native.anybox_device_snapshot(Handle, out var snapshot));
        return Read.Snapshot(snapshot);
    }

    // made by bindings/gen from its call table (methods), change the table and run cargo run -p anybox-gen
    /// <summary>moves by counts, going through whatever smoothing the box has on</summary>
    public void MoveBy(short dx, short dy) => Library.Check(Native.anybox_device_move_by(Handle, dx, dy));

    /// <summary>moves by counts on the box's next report, skipping its smoothing for this move</summary>
    public void MoveNow(short dx, short dy) => Library.Check(Native.anybox_device_move_now(Handle, dx, dy));

    /// <summary>holds a button down until <see cref="Release"/></summary>
    public void Press(Button button) => Library.Check(Native.anybox_device_press(Handle, (uint)button));

    /// <summary>lets go of a button</summary>
    public void Release(Button button) => Library.Check(Native.anybox_device_release(Handle, (uint)button));

    /// <summary>the box's own click, or one timed on the PC with the device's click timing if the box has none,
    /// which blocks the thread for the hold. <paramref name="hold"/> is how long, and <paramref name="nativeOnly"/>
    /// fails instead of letting anybox time it from the PC</summary>
    public void Click(Button button, TimeSpan? hold = null, bool nativeOnly = false) =>
        Library.Check(Native.anybox_device_click_with(Handle, (uint)button, Click(hold, nativeOnly)));

    /// <summary>holds a key down until <see cref="ReleaseKey"/>. a key is its HID usage, from 0x04 to 0xE7</summary>
    public void PressKey(Key key) => Library.Check(Native.anybox_device_press_key(Handle, (byte)key));

    /// <summary>lets go of a key</summary>
    public void ReleaseKey(Key key) => Library.Check(Native.anybox_device_release_key(Handle, (byte)key));

    /// <summary>presses and lets go, timed by the box, or on the PC with the device's click timing if the box can't,
    /// which blocks the thread for the hold. <paramref name="hold"/> is how long, and <paramref name="nativeOnly"/>
    /// fails instead of letting anybox time it from the PC</summary>
    public void TapKey(Key key, TimeSpan? hold = null, bool nativeOnly = false) =>
        Library.Check(Native.anybox_device_tap_key_with(Handle, (byte)key, Click(hold, nativeOnly)));

    /// <summary>holds a media key down until <see cref="ReleaseMedia"/>. a media key is its HID consumer
    /// usage</summary>
    public void PressMedia(MediaKey key) => Library.Check(Native.anybox_device_press_media(Handle, (ushort)key));

    /// <summary>lets go of a media key</summary>
    public void ReleaseMedia(MediaKey key) => Library.Check(Native.anybox_device_release_media(Handle, (ushort)key));

    /// <summary>presses and lets go of a media key, timed like a key's tap, which blocks the thread for the hold.
    /// <paramref name="hold"/> is how long, and <paramref name="nativeOnly"/> fails instead of letting anybox time
    /// it from the PC</summary>
    public void TapMedia(MediaKey key, TimeSpan? hold = null, bool nativeOnly = false) =>
        Library.Check(Native.anybox_device_tap_media_with(Handle, (ushort)key, Click(hold, nativeOnly)));

    /// <summary>keeps the physical button from reaching the PC until <see cref="UnlockButton"/>. your own presses
    /// still go through and its events still come, so you can take the button over</summary>
    public void LockButton(Button button) => Library.Check(Native.anybox_device_lock_button(Handle, (uint)button));

    /// <summary>lets the physical button through again</summary>
    public void UnlockButton(Button button) =>
        Library.Check(Native.anybox_device_unlock_button(Handle, (uint)button));

    /// <summary>keeps the physical key from reaching the PC until <see cref="UnlockKey"/></summary>
    public void LockKey(Key key) => Library.Check(Native.anybox_device_lock_key(Handle, (byte)key));

    /// <summary>lets the physical key through again</summary>
    public void UnlockKey(Key key) => Library.Check(Native.anybox_device_unlock_key(Handle, (byte)key));

    /// <summary>keeps physical motion one way or both ways along an axis from reaching the PC until
    /// <see cref="UnlockAxis"/>. your own moves still go through</summary>
    public void LockAxis(Axis axis, Direction direction = Direction.Both) =>
        Library.Check(Native.anybox_device_lock_axis(Handle, (uint)axis, (uint)direction));

    /// <summary>unlocks that way along the axis and leaves the other way as it was. it also undoes a scale
    /// there</summary>
    public void UnlockAxis(Axis axis, Direction direction = Direction.Both) =>
        Library.Check(Native.anybox_device_unlock_axis(Handle, (uint)axis, (uint)direction));

    /// <summary>keeps <paramref name="percent"/> of the physical motion one way or both ways along an axis, from
    /// -255 to 255. 100 leaves it alone, 0 blocks it and -100 inverts it. <see cref="UnlockAxis"/> puts it back to
    /// 100</summary>
    public void ScaleAxis(Axis axis, Direction direction, short percent) =>
        Library.Check(Native.anybox_device_scale_axis(Handle, (uint)axis, (uint)direction, percent));

    /// <summary>positive scrolls up</summary>
    public void Wheel(short notches) => Library.Check(Native.anybox_device_wheel(Handle, notches));

    /// <summary>horizontal scroll, positive goes right</summary>
    public void Pan(short notches) => Library.Check(Native.anybox_device_pan(Handle, notches));

    /// <summary>throws away the moves the box took but hasn't sent yet</summary>
    public void DiscardPendingMoves() => Library.Check(Native.anybox_device_discard_pending_moves(Handle));

    // the end of what bindings/gen makes (methods)

    /// <summary>a new receiver for these kinds of input, plus the connection events. the box only streams while
    /// at least one receiver is alive. a kind this box can't stream gets one <see cref="Rejected"/> right away,
    /// except with <see cref="EventKinds.All"/>, which means everything this box has. just know that a receiver
    /// of keys sees every key typed on the keyboard behind the box, passwords included</summary>
    public Events Events(EventKinds kinds = EventKinds.All)
    {
        if ((kinds & ~EventKinds.All) != 0)
        {
            throw new ArgumentOutOfRangeException(nameof(kinds), $"{kinds} has kinds this package doesn't know");
        }
        Library.Check(Native.anybox_device_events(Handle, (uint)kinds, out var events));
        return new Events(events, this);
    }

    /// <summary>on a Medius, its own device for everything only Medius does (clips, transforms, rendering, the
    /// polling rate and so on), and null on any other box or once this device is closed. it shares this device's
    /// connection, and keeping it holds the port open after this device is gone. anybox doesn't know about anything
    /// you press or lock through it, and its <c>Reset()</c> lets go of and unlocks EVERYTHING, what anybox did
    /// too</summary>
    public Anybox.Medius.Device? Medius()
    {
        var status = Native.anybox_device_medius(Handle, out var medius);
        if (status is Library.Unsupported or Library.Closed)
        {
            medius.Dispose();
            return null;
        }
        Library.Check(status);
        // asked after the box, so another box or a closed device is null like in Rust even when the library's
        // medius side doesn't match this package
        try
        {
            Anybox.Medius.MediusLibrary.Ensure();
        }
        catch
        {
            medius.Dispose();
            throw;
        }
        return new Anybox.Medius.Device(medius);
    }

    /// <summary>lets go of what you held, unlocks, and closes the port, and throws if that didn't work. it waits
    /// up to about 2 s when the box doesn't answer. after it every call throws <see cref="ClosedException"/> and
    /// the receivers end once they're read empty, <see cref="Events"/> gives one that's already ended and
    /// <see cref="Snapshot"/> still works. a second close throws <see cref="ClosedException"/>, like in
    /// Rust</summary>
    public void Close()
    {
        closed = true;
        Library.Check(Native.anybox_device_close(Handle));
    }

    /// <summary>closes the device if you didn't, and frees it, after which every call throws
    /// <see cref="ObjectDisposedException"/>. like Rust's drop, a close that fails here doesn't throw, because it
    /// would take the place of an exception already on its way out of a <c>using</c>, so call <see cref="Close"/>
    /// when you want to know whether it worked</summary>
    public void Dispose()
    {
        try
        {
            if (!closed && !handle.IsClosed)
            {
                Close();
            }
        }
        catch (AnyboxException)
        {
        }
        finally
        {
            handle.Dispose();
        }
    }
}

/// <summary>a receiver of events from one device. it can be read from any thread, two threads reading the same
/// one split the events between them, and enumerating it goes until the device is gone and everything queued was
/// read</summary>
public sealed class Events : IDisposable, IEnumerable<Event>
{
    readonly EventsHandle handle;
    // up before the library's close, so a receive the close wakes already sees it, which the SafeHandle can't do
    // since it only closes once no call is in it
    int disposed;

#pragma warning disable CS0414, IDE0052
    // never read, it's here so the device stays reachable, otherwise `Device.Connect().Events()` would get its
    // device finalized and closed in the middle of reading
    readonly Device device;
#pragma warning restore CS0414, IDE0052

    internal Events(EventsHandle handle, Device device)
    {
        this.handle = handle;
        this.device = device;
    }

    /// <summary>the next event, waiting up to <paramref name="timeout"/> for it (forever with none) and null when
    /// nothing came. it throws <see cref="RecvClosedException"/> once the device is gone and everything queued was
    /// read, and <see cref="ClosedException"/> once this receiver was disposed</summary>
    public Event? Recv(TimeSpan? timeout = null)
    {
        // the library takes a signed count, and a wait that long is forever anyway
        var wait = timeout is { } value ? (long)Math.Min(Read.Micros(value, "a timeout"), long.MaxValue) : -1;
        uint status;
        AnyboxEvent raw;
        try
        {
            if (Volatile.Read(ref disposed) != 0)
            {
                throw Closed();
            }
            status = Imports.anybox_events_recv(handle, wait, out raw);
        }
        // disposed before the call got the handle
        catch (ObjectDisposedException)
        {
            throw Closed();
        }
        // disposed while it waited, so whatever came back belongs to a receiver that's gone
        if (Volatile.Read(ref disposed) != 0)
        {
            throw Closed();
        }
        if (status == Library.Ok)
        {
            return Read.Event(raw);
        }
        if (status == Library.RecvTimeout)
        {
            return null;
        }
        throw Library.Error(status);
    }

    static ClosedException Closed() => new("this receiver was disposed, get a new one with device.Events()");

    public Event? TryRecv() => Recv(TimeSpan.Zero);

    public IEnumerator<Event> GetEnumerator()
    {
        while (Next() is { } next)
        {
            yield return next;
        }
    }

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();

    Event? Next()
    {
        try
        {
            return Recv();
        }
        catch (Exception error) when (error is RecvClosedException or ClosedException)
        {
            return null;
        }
    }

    /// <summary>ends this receiver now, a <see cref="Recv"/> waiting on it from another thread throws
    /// <see cref="ClosedException"/> right away, what was queued is thrown out, and the box stops streaming what
    /// only this receiver wanted. a second dispose does nothing, and a close that fails doesn't throw, since Rust's close can't fail</summary>
    public void Dispose()
    {
        if (Interlocked.Exchange(ref disposed, 1) != 0)
        {
            return;
        }
        try
        {
            Library.Check(Imports.anybox_events_close(handle));
        }
        // Rust's close can't fail, and a failure here would take the place of an exception already on its way out
        // of a using, like the device's
        catch (AnyboxException)
        {
        }
        finally
        {
            handle.Dispose();
        }
    }
}

/// <summary>what to look for. <see cref="Port"/> only asks that port, <see cref="Family"/> only that kind of box,
/// <see cref="FerrumApi"/> only a Ferrum on that API, <see cref="Can"/> skips a box that can't do those actions,
/// <see cref="Id"/> only takes the box with that id, and <see cref="ClickTiming"/> is what a click does on a box
/// with no click of its own</summary>
public sealed record Connect
{
    public string? Port { get; init; }
    public Family? Family { get; init; }
    public FerrumApi? FerrumApi { get; init; }
    public IReadOnlyList<BoxAction> Can { get; init; } = [];
    public StableId? Id { get; init; }
    public ClickTiming ClickTiming { get; init; } = ClickTiming.Makcu;

    internal unsafe T WithOptions<T>(Func<AnyboxConnectOptions, T> call)
    {
        var can = Can.Select(action => (uint)action).ToArray();
        var port = Port is null ? null : Library.Utf8(Port);
        var serial = Id?.UsbSerial is { } text ? Library.Utf8(text) : null;
        fixed (byte* portText = port, serialText = serial)
        fixed (uint* canList = can)
        {
            var options = new AnyboxConnectOptions
            {
                port = portText,
                family = (uint?)Family ?? 0,
                ferrum_api = (uint?)FerrumApi ?? 0,
                can = can.Length == 0 ? null : canList,
                can_len = (nuint)can.Length,
                click_timing = ClickTiming.ToNative(),
            };
            if (Id?.Mac is { } mac)
            {
                options.id_kind = (uint)IdKind.Mac;
                mac.GetAddressBytes().CopyTo(new Span<byte>(options.id_mac, 6));
            }
            else if (serial is not null)
            {
                options.id_kind = (uint)IdKind.UsbSerial;
                options.id_serial = serialText;
            }
            return call(options);
        }
    }

    /// <summary>opens the first box that fits, in port order</summary>
    public Device Open()
    {
        Library.Ensure();
        return new Device(WithOptions(options =>
        {
            Library.Check(Imports.anybox_open(options, out var device));
            return device;
        }));
    }

    /// <summary>every port a search looks at and what's on it, in port order. it opens, describes and closes
    /// every box, so it's slow (a YSC search takes about 2 s a port), <see cref="Candidates"/> is the cheap one to
    /// call often</summary>
    public unsafe IReadOnlyList<Scanned> Scan()
    {
        Library.Ensure();
        using var list = WithOptions(options =>
        {
            Library.Check(Imports.anybox_scan(options, out var list));
            return list;
        });
        var count = Imports.anybox_scan_list_count(list);
        var scanned = new List<Scanned>();
        for (nuint index = 0; index < count; index++)
        {
            var port = Library.Text(Imports.anybox_scan_list_port(list, index)) ?? "";
            var status = Imports.anybox_scan_list_result(list, index, out var snapshot);
            scanned.Add(status == Library.Ok
                ? new Scanned(port, Read.Snapshot(snapshot), null)
                : new Scanned(port, null, Library.Error(status)));
        }
        return scanned;
    }

    /// <summary>the ports a search would look at, without opening any of them or sending them a byte, so it's
    /// cheap enough to call every second and scan when the list changes</summary>
    public unsafe IReadOnlyList<string> Candidates()
    {
        Library.Ensure();
        using var list = WithOptions(options =>
        {
            Library.Check(Imports.anybox_candidates(options, out var list));
            return list;
        });
        var count = Imports.anybox_string_list_count(list);
        var ports = new List<string>();
        for (nuint index = 0; index < count; index++)
        {
            ports.Add(Library.Text(Imports.anybox_string_list_get(list, index)) ?? "");
        }
        return ports;
    }
}
