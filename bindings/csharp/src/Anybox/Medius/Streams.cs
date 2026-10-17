using System.Collections;
using System.Runtime.InteropServices;
using static Anybox.Medius.MediusLibrary;

namespace Anybox.Medius;

// the three streams wait and end the same way. a close from another thread wakes a blocked receive with the
// answer a gone box gives too, so whether the stream was disposed comes from its own flag once the call is back.
// the flag goes up before the close, the SafeHandle only after it, so a reader the close woke still sees it
internal static unsafe class Ending
{
    internal static ClosedException Closed() => new("this stream was disposed, subscribe again if you need it");

    internal static bool Disposed(ref int disposed) => Volatile.Read(ref disposed) != 0;

    // true for the one call that gets to close, so a second Dispose, or two at once, does nothing
    internal static bool First(ref int disposed) => Interlocked.Exchange(ref disposed, 1) == 0;

    internal static TRaw Recv<THandle, TRaw>(THandle handle, ref int disposed, delegate*<THandle, TRaw*, Status> recv)
        where THandle : SafeHandle where TRaw : unmanaged
    {
        TRaw raw;
        Status status;
        try
        {
            status = recv(handle, &raw);
        }
        // disposed before the call got the handle
        catch (ObjectDisposedException)
        {
            throw Closed();
        }
        if (Disposed(ref disposed))
        {
            throw Closed();
        }
        Check(status);
        return raw;
    }

    // null when nothing came in time or the box is gone, like the crate
    internal static TRaw? RecvTimeout<THandle, TRaw>(THandle handle, ref int disposed, TimeSpan timeout, delegate*<THandle, ulong, TRaw*, byte> recv)
        where THandle : SafeHandle where TRaw : unmanaged
    {
        var ms = Ms(timeout);
        TRaw raw;
        byte came;
        try
        {
            came = recv(handle, ms, &raw);
        }
        catch (ObjectDisposedException)
        {
            throw Closed();
        }
        // whatever it brought back belongs to a stream that's gone
        if (Disposed(ref disposed))
        {
            throw Closed();
        }
        return came != 0 ? raw : null;
    }

    internal static TRaw? TryRecv<THandle, TRaw>(THandle handle, ref int disposed, delegate*<THandle, TRaw*, byte> recv)
        where THandle : SafeHandle where TRaw : unmanaged
    {
        if (Disposed(ref disposed))
        {
            throw Closed();
        }
        TRaw raw;
        byte came;
        try
        {
            came = recv(handle, &raw);
        }
        catch (ObjectDisposedException)
        {
            throw Closed();
        }
        if (Disposed(ref disposed))
        {
            throw Closed();
        }
        return came != 0 ? raw : null;
    }

    // every other call on a disposed stream says so the same way a receive does
    internal static T Call<THandle, T>(THandle handle, ref int disposed, Func<THandle, T> call) where THandle : SafeHandle
    {
        if (Disposed(ref disposed))
        {
            throw Closed();
        }
        try
        {
            return call(handle);
        }
        catch (ObjectDisposedException)
        {
            throw Closed();
        }
    }

    // rounded up from the ticks, so a timeout above zero never turns into a poll, like the Python package's ceil
    internal static ulong Ms(TimeSpan timeout)
    {
        if (timeout < TimeSpan.Zero)
        {
            throw new ArgumentOutOfRangeException(nameof(timeout), "a timeout can't be negative");
        }
        var whole = (ulong)(timeout.Ticks / TimeSpan.TicksPerMillisecond);
        return timeout.Ticks % TimeSpan.TicksPerMillisecond == 0 ? whole : whole + 1;
    }

    internal static IEnumerator<T> Until<T>(Func<T> recv)
    {
        while (Next(recv) is { } next)
        {
            yield return next;
        }
    }

    static T? Next<T>(Func<T> recv)
    {
        try
        {
            return recv();
        }
        catch (Exception error) when (error is DisconnectedException or ClosedException)
        {
            return default;
        }
    }
}

/// <summary>a live catch stream. enumerate it to read events until the box is gone or it's disposed, and
/// disposing it from another thread ends a reader at once</summary>
public sealed unsafe class EventStream : IDisposable, IEnumerable<CatchEvent>
{
    readonly MediusEventStreamHandle handle;
    int disposed;

#pragma warning disable CS0414, IDE0052
    // never read, it keeps the device reachable while the stream is open, like the Python package
    readonly Device device;
#pragma warning restore CS0414, IDE0052

    internal EventStream(MediusEventStreamHandle handle, Device device)
    {
        this.handle = handle;
        this.device = device;
    }

    /// <summary>waits for the next event. it throws <see cref="DisconnectedException"/> once the box is gone and
    /// everything queued was read, or another clone was disposed, and <see cref="ClosedException"/> once this
    /// stream was disposed</summary>
    public CatchEvent Recv() => CatchEvent.From(Ending.Recv<MediusEventStreamHandle, MediusCatchEvent>(handle, ref disposed, &MediusImports.medius_event_stream_recv));

    public CatchEvent? TryRecv() =>
        Ending.TryRecv<MediusEventStreamHandle, MediusCatchEvent>(handle, ref disposed, &MediusImports.medius_event_stream_try_recv) is { } raw ? CatchEvent.From(raw) : null;

    /// <summary>the next event, or null when nothing came in time or the box is gone, <see cref="IsConnected"/> tells
    /// which</summary>
    public CatchEvent? RecvTimeout(TimeSpan timeout) =>
        Ending.RecvTimeout<MediusEventStreamHandle, MediusCatchEvent>(handle, ref disposed, timeout, &MediusImports.medius_event_stream_recv_timeout) is { } raw ? CatchEvent.From(raw) : null;

    public ulong Dropped => Ending.Call(handle, ref disposed, MediusImports.medius_event_stream_dropped);

    /// <summary>whether the box still delivers, which tells "nothing yet" from "nothing ever again" when
    /// <see cref="TryRecv"/> gives null</summary>
    public bool IsConnected => Ending.Call(handle, ref disposed, MediusImports.medius_event_stream_is_connected) != 0;

    /// <summary>another handle to the same subscription, they share the queue. disposing any of them ends it for
    /// all, one that only gets collected leaves the others reading</summary>
    public EventStream Clone()
    {
        var clone = Ending.Call(handle, ref disposed, MediusImports.medius_event_stream_clone);
        if (clone.IsInvalid)
        {
            throw new MediusException(Status.ErrUnknown, "cloning the event stream failed, which only a bug in medius does, so report it", 0);
        }
        return new(clone, device);
    }

    public IEnumerator<CatchEvent> GetEnumerator() => Ending.Until(Recv);

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();

    /// <summary>ends the subscription for every clone and wakes a <see cref="Recv"/> blocked in another thread, so
    /// it works from any thread</summary>
    public void Dispose()
    {
        if (Ending.First(ref disposed))
        {
            MediusImports.medius_event_stream_close(handle);
            handle.Dispose();
        }
    }
}

/// <summary>a live stream of decoded input, presses, releases and motion. the box sends held usage snapshots and
/// this turns them into edges. enumerate it like an <see cref="EventStream"/></summary>
public sealed unsafe class InputStream : IDisposable, IEnumerable<InputEvent>
{
    readonly MediusInputStreamHandle handle;
    int disposed;

#pragma warning disable CS0414, IDE0052
    // never read, it keeps the device reachable while the stream is open, like the Python package
    readonly Device device;
#pragma warning restore CS0414, IDE0052

    internal InputStream(MediusInputStreamHandle handle, Device device)
    {
        this.handle = handle;
        this.device = device;
    }

    /// <summary>waits for the next event. it throws <see cref="DisconnectedException"/> once the box is gone and
    /// everything queued was read, and <see cref="ClosedException"/> once the stream was disposed</summary>
    public InputEvent Recv() => InputEvent.From(Ending.Recv<MediusInputStreamHandle, MediusInputEvent>(handle, ref disposed, &MediusImports.medius_input_stream_recv));

    public InputEvent? TryRecv() =>
        Ending.TryRecv<MediusInputStreamHandle, MediusInputEvent>(handle, ref disposed, &MediusImports.medius_input_stream_try_recv) is { } raw ? InputEvent.From(raw) : null;

    /// <inheritdoc cref="EventStream.RecvTimeout"/>
    public InputEvent? RecvTimeout(TimeSpan timeout) =>
        Ending.RecvTimeout<MediusInputStreamHandle, MediusInputEvent>(handle, ref disposed, timeout, &MediusImports.medius_input_stream_recv_timeout) is { } raw ? InputEvent.From(raw) : null;

    /// <summary>the usages of that class this stream holds now</summary>
    public IReadOnlyList<Usage> Held(Class inputClass)
    {
        var capacity = 16;
        while (true)
        {
            var held = new MediusUsage[capacity];
            var count = Ending.Call(handle, ref disposed, stream =>
            {
                fixed (MediusUsage* first = held)
                {
                    return MediusImports.medius_input_stream_held(stream, In(inputClass, nameof(inputClass)), first, (nuint)capacity);
                }
            });
            if (count <= (nuint)capacity)
            {
                return [.. held[..(int)count].Select(Usage.From)];
            }
            capacity = (int)count;
        }
    }

    public ulong Dropped => Ending.Call(handle, ref disposed, MediusImports.medius_input_stream_dropped);

    /// <inheritdoc cref="EventStream.IsConnected"/>
    public bool IsConnected => Ending.Call(handle, ref disposed, MediusImports.medius_input_stream_is_connected) != 0;

    public IEnumerator<InputEvent> GetEnumerator() => Ending.Until(Recv);

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();

    /// <summary>ends the subscription and wakes a <see cref="Recv"/> blocked in another thread, so it works from
    /// any thread</summary>
    public void Dispose()
    {
        if (Ending.First(ref disposed))
        {
            MediusImports.medius_input_stream_close(handle);
            handle.Dispose();
        }
    }
}

/// <summary>the box's log. enumerate it like an <see cref="EventStream"/></summary>
public sealed unsafe class LogStream : IDisposable, IEnumerable<LogLine>
{
    readonly MediusLogStreamHandle handle;
    int disposed;

#pragma warning disable CS0414, IDE0052
    // never read, it keeps the device reachable while the stream is open, like the Python package
    readonly Device device;
#pragma warning restore CS0414, IDE0052

    internal LogStream(MediusLogStreamHandle handle, Device device)
    {
        this.handle = handle;
        this.device = device;
    }

    /// <inheritdoc cref="EventStream.Recv"/>
    public LogLine Recv() => LogLine.From(Ending.Recv<MediusLogStreamHandle, MediusLogLine>(handle, ref disposed, &MediusImports.medius_log_stream_recv));

    public LogLine? TryRecv() =>
        Ending.TryRecv<MediusLogStreamHandle, MediusLogLine>(handle, ref disposed, &MediusImports.medius_log_stream_try_recv) is { } raw ? LogLine.From(raw) : null;

    /// <inheritdoc cref="EventStream.RecvTimeout"/>
    public LogLine? RecvTimeout(TimeSpan timeout) =>
        Ending.RecvTimeout<MediusLogStreamHandle, MediusLogLine>(handle, ref disposed, timeout, &MediusImports.medius_log_stream_recv_timeout) is { } raw ? LogLine.From(raw) : null;

    /// <inheritdoc cref="EventStream.IsConnected"/>
    public bool IsConnected => Ending.Call(handle, ref disposed, MediusImports.medius_log_stream_is_connected) != 0;

    /// <summary>another handle to the same log. disposing any of them ends it for all, one that only gets
    /// collected leaves the others reading. other logs from the same device keep going either way</summary>
    public LogStream Clone()
    {
        var clone = Ending.Call(handle, ref disposed, MediusImports.medius_log_stream_clone);
        if (clone.IsInvalid)
        {
            throw new MediusException(Status.ErrUnknown, "cloning the log stream failed, which only a bug in medius does, so report it", 0);
        }
        return new(clone, device);
    }

    public IEnumerator<LogLine> GetEnumerator() => Ending.Until(Recv);

    IEnumerator IEnumerable.GetEnumerator() => GetEnumerator();

    /// <inheritdoc cref="EventStream.Dispose"/>
    public void Dispose()
    {
        if (Ending.First(ref disposed))
        {
            MediusImports.medius_log_stream_close(handle);
            handle.Dispose();
        }
    }
}

/// <summary>puts box stamps on your clock. a catch stamp is microseconds on a chip that booted before this
/// process, it wraps about every 71.6 minutes and has nothing to do with any clock here. feed it every event as it
/// comes, in order. each chip is tracked on its own by the smallest gap between the time here and on the box, and
/// an event can only come late, never early, so the mapping only gets better</summary>
public sealed unsafe class Timeline : IDisposable
{
    readonly MediusTimelineHandle handle;

    // the library takes the timeline as &mut, so only one call goes in at a time
    readonly Lock gate = new();

    public Timeline()
    {
        Ensure();
        handle = MediusImports.medius_timeline_new();
        if (handle.IsInvalid)
        {
            throw new MediusException(Status.ErrUnknown, "the timeline couldn't be made, which only a bug in medius does, so report it", 0);
        }
    }

    MediusTimelineHandle Handle
    {
        get
        {
            ObjectDisposedException.ThrowIf(handle.IsClosed, this);
            return handle;
        }
    }

    static ulong Now() => (ulong)Monotonic.NowNs;

    /// <summary>puts the event on this machine's clock. <paramref name="nowNs"/> is a monotonic time in
    /// nanoseconds, the one <see cref="Monotonic.NowNs"/> reads when you leave it out. catch and input events share
    /// one timeline, so the two kinds of stream order together</summary>
    public Stamped Observe(CatchEvent catchEvent, ulong? nowNs = null)
    {
        // only the stamp and the chip are read, so those two are all that goes back
        var raw = new MediusCatchEvent { kind = catchEvent.Kind, ts_us = catchEvent.TsUs, clock = Out(catchEvent.Clock, "a clock domain") };
        MediusStamped stamped;
        byte placed;
        lock (gate)
        {
            placed = MediusImports.medius_timeline_observe(Handle, &raw, nowNs ?? Now(), &stamped);
        }
        if (placed == 0)
        {
            throw new InvalidArgException("the timeline couldn't place the event, which only a bug in medius does, so report it", 0);
        }
        return new(stamped.host_ns, stamped.box_us, stamped.excess_ns);
    }

    public Stamped Observe(InputEvent inputEvent, ulong? nowNs = null)
    {
        var raw = new MediusInputEvent { kind = inputEvent.Kind, ts_us = inputEvent.TsUs, clock = Out(inputEvent.Clock, "a clock domain") };
        MediusStamped stamped;
        byte placed;
        lock (gate)
        {
            placed = MediusImports.medius_timeline_observe_input(Handle, &raw, nowNs ?? Now(), &stamped);
        }
        if (placed == 0)
        {
            throw new InvalidArgException("the timeline couldn't place the event, which only a bug in medius does, so report it", 0);
        }
        return new(stamped.host_ns, stamped.box_us, stamped.excess_ns);
    }

    /// <summary>puts a bare stamp from <paramref name="clock"/>'s chip on this machine's clock, like
    /// <see cref="Observe(CatchEvent, ulong?)"/></summary>
    public Stamped ObserveStamp(uint tsUs, ClockDomain clock, ulong? nowNs = null) =>
        Observe(new CatchEvent(CatchEventKind.Motion, tsUs, clock, null, null, null), nowNs);

    /// <summary>forgets one chip's rollovers and floor, for a chip that rebooted</summary>
    public void Reset(ClockDomain domain)
    {
        lock (gate)
        {
            MediusImports.medius_timeline_reset(Handle, In(domain, nameof(domain)));
        }
    }

    /// <summary>how many events a chip had, the floor is the smallest of them</summary>
    public ulong Samples(ClockDomain domain)
    {
        lock (gate)
        {
            return MediusImports.medius_timeline_samples(Handle, In(domain, nameof(domain)));
        }
    }

    public void Dispose() => handle.Dispose();
}
