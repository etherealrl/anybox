using static Anybox.Medius.MediusLibrary;

namespace Anybox.Medius;

/// <summary>one raw report in a clip frame, as <see cref="Device.Raw"/> sends one</summary>
public sealed record ClipRaw(byte Ep, Direction Direction, ReadOnlyMemory<byte> Bytes);

/// <summary>one control transfer in a clip frame, as <see cref="Device.Transfer"/> runs one</summary>
public sealed record ClipTransfer(byte Ep, Setup Setup, ReadOnlyMemory<byte> Data = default);

/// <summary>builds the entries of a buffered clip. give it to <see cref="ClipHandle.Append"/>, then
/// <see cref="Clear"/> to use it again. every call gives the builder back, so they chain</summary>
public sealed unsafe class ClipBuilder : IDisposable
{
    readonly MediusClipBuilderHandle handle;

    // the library takes the builder as &mut, so only one call goes in at a time, an append's read included
    internal readonly Lock Gate = new();

    public ClipBuilder()
    {
        Ensure();
        handle = MediusImports.medius_clip_builder_new();
        if (handle.IsInvalid)
        {
            throw new OutOfMemoryException("the clip builder couldn't be made");
        }
    }

    internal MediusClipBuilderHandle Handle
    {
        get
        {
            ObjectDisposedException.ThrowIf(handle.IsClosed, this);
            return handle;
        }
    }

    ClipBuilder Done(Func<MediusClipBuilderHandle, Status> call)
    {
        lock (Gate)
        {
            Check(call(Handle));
        }
        return this;
    }

    public ClipBuilder Clear() => Done(MediusImports.medius_clip_builder_clear);

    /// <summary>the ring bytes the entries take, to check against <see cref="ClipStatus.Free"/> before an
    /// append</summary>
    public nuint ByteLen
    {
        get
        {
            lock (Gate)
            {
                return MediusImports.medius_clip_builder_byte_len(Handle);
            }
        }
    }

    /// <summary>nothing for that many native frames, 0 does nothing</summary>
    public ClipBuilder Gap(ushort frames) => Done(builder => MediusImports.medius_clip_builder_gap(builder, frames));

    public ClipBuilder Move(short dx, short dy) => Done(builder => MediusImports.medius_clip_builder_move(builder, dx, dy));
    public ClipBuilder Wheel(short dz) => Done(builder => MediusImports.medius_clip_builder_wheel(builder, dz));
    public ClipBuilder Pan(short dpan) => Done(builder => MediusImports.medius_clip_builder_pan(builder, dpan));
    public ClipBuilder Press(Usage usage) => Done(builder => MediusImports.medius_clip_builder_press(builder, usage.ToNative()));

    /// <summary>a soft release, which leaves a physical hold alone</summary>
    public ClipBuilder Release(Usage usage) => Done(builder => MediusImports.medius_clip_builder_release(builder, usage.ToNative()));

    /// <summary>a release that masks a physical hold too</summary>
    public ClipBuilder ForceRelease(Usage usage) => Done(builder => MediusImports.medius_clip_builder_force_release(builder, usage.ToNative()));

    public ClipBuilder Edge(Usage usage, InjectAction action = InjectAction.Press)
    {
        var raw = In(action, nameof(action));
        return Done(builder => MediusImports.medius_clip_builder_edge(builder, usage.ToNative(), raw));
    }

    public ClipBuilder Raw(byte ep, Direction direction, ReadOnlySpan<byte> data)
    {
        var raw = In(direction, nameof(direction));
        lock (Gate)
        {
            fixed (byte* bytes = data)
            {
                Check(MediusImports.medius_clip_builder_raw(Handle, ep, raw, bytes, (nuint)data.Length));
            }
        }
        return this;
    }

    /// <summary>one control transfer, its reply comes as a <see cref="TrafficClass.ClipTransfer"/> event</summary>
    public ClipBuilder Transfer(byte ep, Setup setup, ReadOnlySpan<byte> data = default)
    {
        lock (Gate)
        {
            fixed (byte* bytes = data)
            {
                Check(MediusImports.medius_clip_builder_transfer(Handle, ep, setup.ToNative(), bytes, (nuint)data.Length));
            }
        }
        return this;
    }

    /// <summary>a frame with all of it at once: motion, wheel and pan, the edges, raw reports and transfers</summary>
    public ClipBuilder Frame(
        short dx = 0,
        short dy = 0,
        short wheel = 0,
        short pan = 0,
        IEnumerable<(Usage Usage, InjectAction Action)>? edges = null,
        IEnumerable<ClipRaw>? raw = null,
        IEnumerable<ClipTransfer>? transfers = null)
    {
        // the frame is this call's own, so only the builder needs the lock
        using var frame = MediusImports.medius_clip_frame_new();
        if (frame.IsInvalid)
        {
            throw new OutOfMemoryException("the clip frame couldn't be made");
        }
        Check(MediusImports.medius_clip_frame_move(frame, dx, dy));
        Check(MediusImports.medius_clip_frame_wheel(frame, wheel));
        Check(MediusImports.medius_clip_frame_pan(frame, pan));
        foreach (var (usage, action) in edges ?? [])
        {
            Check(MediusImports.medius_clip_frame_edge(frame, usage.ToNative(), In(action, "an edge's action")));
        }
        foreach (var report in raw ?? [])
        {
            fixed (byte* bytes = report.Bytes.Span)
            {
                Check(MediusImports.medius_clip_frame_raw(
                    frame, report.Ep, In(report.Direction, "a raw report's direction"), bytes, (nuint)report.Bytes.Length));
            }
        }
        foreach (var transfer in transfers ?? [])
        {
            fixed (byte* bytes = transfer.Data.Span)
            {
                Check(MediusImports.medius_clip_frame_transfer(
                    frame, transfer.Ep, transfer.Setup.ToNative(), bytes, (nuint)transfer.Data.Length));
            }
        }
        return Done(builder => MediusImports.medius_clip_builder_frame(builder, frame));
    }

    /// <summary>adds a frame built on its own, which stays the caller's to use again or dispose</summary>
    public ClipBuilder Frame(ClipFrame frame)
    {
        lock (frame.Gate)
        {
            return Done(builder => MediusImports.medius_clip_builder_frame(builder, frame.Handle));
        }
    }

    public void Dispose() => handle.Dispose();
}

/// <summary>one native frame of a clip, built on its own and added with <see cref="ClipBuilder.Frame(ClipFrame)"/>.
/// every call gives the frame back, so they chain</summary>
public sealed unsafe class ClipFrame : IDisposable
{
    readonly MediusClipFrameHandle handle;

    // the library takes the frame as &mut, so only one call goes in at a time
    internal readonly Lock Gate = new();

    public ClipFrame()
    {
        Ensure();
        handle = MediusImports.medius_clip_frame_new();
        if (handle.IsInvalid)
        {
            throw new OutOfMemoryException("the clip frame couldn't be made");
        }
    }

    internal MediusClipFrameHandle Handle
    {
        get
        {
            ObjectDisposedException.ThrowIf(handle.IsClosed, this);
            return handle;
        }
    }

    ClipFrame Done(Func<MediusClipFrameHandle, Status> call)
    {
        lock (Gate)
        {
            Check(call(Handle));
        }
        return this;
    }

    public ClipFrame Clear() => Done(MediusImports.medius_clip_frame_clear);

    /// <summary>the ring bytes the frame takes</summary>
    public nuint ByteLen
    {
        get
        {
            lock (Gate)
            {
                return MediusImports.medius_clip_frame_byte_len(Handle);
            }
        }
    }

    public ClipFrame Move(short dx, short dy) => Done(frame => MediusImports.medius_clip_frame_move(frame, dx, dy));
    public ClipFrame Wheel(short dz) => Done(frame => MediusImports.medius_clip_frame_wheel(frame, dz));
    public ClipFrame Pan(short dpan) => Done(frame => MediusImports.medius_clip_frame_pan(frame, dpan));
    public ClipFrame Press(Usage usage) => Done(frame => MediusImports.medius_clip_frame_press(frame, usage.ToNative()));
    public ClipFrame Release(Usage usage) => Done(frame => MediusImports.medius_clip_frame_release(frame, usage.ToNative()));
    public ClipFrame ForceRelease(Usage usage) => Done(frame => MediusImports.medius_clip_frame_force_release(frame, usage.ToNative()));

    public ClipFrame Edge(Usage usage, InjectAction action = InjectAction.Press)
    {
        var raw = In(action, nameof(action));
        return Done(frame => MediusImports.medius_clip_frame_edge(frame, usage.ToNative(), raw));
    }

    public ClipFrame Raw(byte ep, Direction direction, ReadOnlySpan<byte> data)
    {
        var raw = In(direction, nameof(direction));
        lock (Gate)
        {
            fixed (byte* bytes = data)
            {
                Check(MediusImports.medius_clip_frame_raw(Handle, ep, raw, bytes, (nuint)data.Length));
            }
        }
        return this;
    }

    /// <summary>one control transfer, its reply comes as a <see cref="TrafficClass.ClipTransfer"/> event</summary>
    public ClipFrame Transfer(byte ep, Setup setup, ReadOnlySpan<byte> data = default)
    {
        lock (Gate)
        {
            fixed (byte* bytes = data)
            {
                Check(MediusImports.medius_clip_frame_transfer(Handle, ep, setup.ToNative(), bytes, (nuint)data.Length));
            }
        }
        return this;
    }

    public void Dispose() => handle.Dispose();
}

/// <summary>one box's buffered clip playback, from <see cref="Device.Clip"/>, keep one per clip session. a trigger
/// runs a clip verb on the box with no round trip to the PC, an input trigger (<see cref="Bind"/>) on a button,
/// key or media edge and a packet trigger (<see cref="BindPacket"/>) on a packet crossing a traffic surface</summary>
public sealed unsafe class ClipHandle : IDisposable
{
    readonly MediusClipHandle handle;

#pragma warning disable CS0414, IDE0052
    // never read, it keeps the device reachable while the clip is open, like the Python package
    readonly Device device;
#pragma warning restore CS0414, IDE0052

    internal ClipHandle(MediusClipHandle handle, Device device)
    {
        this.handle = handle;
        this.device = device;
    }

    MediusClipHandle Handle
    {
        get
        {
            ObjectDisposedException.ThrowIf(handle.IsClosed, this);
            return handle;
        }
    }

    /// <summary>adds the builder's entries to the ring. every entry is checked before the first frame goes out,
    /// so a refusal sends nothing</summary>
    public void Append(ClipBuilder builder)
    {
        lock (builder.Gate)
        {
            Check(MediusImports.medius_clip_append(Handle, builder.Handle));
        }
    }

    /// <summary>locks these groups while the clip plays and lets go on stop, set it before the first append</summary>
    public void SetAutolock(params ReadOnlySpan<Blanket> scope)
    {
        var groups = new byte[scope.Length];
        for (var index = 0; index < scope.Length; index++)
        {
            groups[index] = In(scope[index], nameof(scope));
        }
        fixed (byte* first = groups)
        {
            Check(MediusImports.medius_clip_set_autolock(Handle, groups.Length == 0 ? null : first, (nuint)groups.Length));
        }
    }

    /// <summary>loops at the clip's end, retained clips only</summary>
    public void SetLoop(bool on) => Check(MediusImports.medius_clip_set_loop(Handle, Flag(on)));

    /// <summary>keeps the loaded clip so it can rewind and play again, off is streaming. set it before the first
    /// append</summary>
    public void SetRetain(bool on) => Check(MediusImports.medius_clip_set_retain(Handle, Flag(on)));

    /// <summary>makes the clip's motion wait to ride a native report instead of the box's own clock</summary>
    public void SetRide(bool on) => Check(MediusImports.medius_clip_set_ride(Handle, Flag(on)));

    public void Bind(ClipTrigger trigger) => Check(MediusImports.medius_clip_bind(Handle, trigger.ToNative()));

    public void Unbind(Usage usage, Edge edge) => Check(MediusImports.medius_clip_unbind(Handle, usage.ToNative(), In(edge, nameof(edge))));

    /// <summary>adds or overwrites a packet trigger. one the box would refuse throws
    /// <see cref="ClipPacketTriggerException"/> before anything is sent, with why in the message. to be sure a
    /// bind took, compare what <see cref="QueryConfig"/> reads back</summary>
    public void BindPacket(ClipPacketTrigger trigger)
    {
        var raw = trigger.ToNative();
        Check(MediusImports.medius_clip_bind_packet(Handle, &raw));
    }

    /// <summary>removes the packet trigger with that trigger's key, its other fields don't matter</summary>
    public void UnbindPacket(ClipPacketTrigger trigger)
    {
        var raw = trigger.KeyOnly();
        Check(MediusImports.medius_clip_unbind_packet(Handle, &raw));
    }

    public void ClearTriggers() => Check(MediusImports.medius_clip_clear_triggers(Handle));

    /// <summary>rewinds and plays, or goes on after a pause</summary>
    public void Start() => Check(MediusImports.medius_clip_start(Handle));

    /// <summary>stops, flushes a streaming clip or rewinds a retained one, and lets go of held input and the
    /// autolock</summary>
    public void Stop() => Check(MediusImports.medius_clip_stop(Handle));

    public void Pause() => Check(MediusImports.medius_clip_pause(Handle));
    public void Resume() => Check(MediusImports.medius_clip_resume(Handle));

    /// <summary>rewinds and plays even in the middle of playing</summary>
    public void Restart() => Check(MediusImports.medius_clip_restart(Handle));

    /// <summary>plays when idle or paused, stops when playing</summary>
    public void Toggle() => Check(MediusImports.medius_clip_toggle(Handle));

    /// <summary>throws the loaded clip away, frees the ring and clears a fault</summary>
    public void Clear() => Check(MediusImports.medius_clip_clear(Handle));

    /// <summary>fixes a retained clip's end so it can play again and loop</summary>
    public void FinalizeClip() => Check(MediusImports.medius_clip_finalize(Handle));

    /// <summary>whether the box dropped the clip appended since the last clear, because its device chip
    /// restarted, it let go of the session, or a reconnect found it gone</summary>
    public bool Lost => MediusImports.medius_clip_lost(Handle) != 0;

    public ClipStatus QueryStatus()
    {
        MediusClipStatus raw;
        Check(MediusImports.medius_clip_query_status(Handle, &raw));
        return ClipStatus.From(raw);
    }

    public ClipSettings QueryConfig()
    {
        MediusClipSettings raw;
        Check(MediusImports.medius_clip_query_config(Handle, &raw));
        return ClipSettings.From(raw);
    }

    public void Dispose() => handle.Dispose();
}
