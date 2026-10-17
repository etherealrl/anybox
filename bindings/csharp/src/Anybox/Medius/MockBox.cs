using static Anybox.Medius.MediusLibrary;

namespace Anybox.Medius;

/// <summary>the medius crate's scriptable fake box, for testing with no box plugged in. it's only in a library
/// built with the capi crate's <c>mock</c> feature, the one in the package doesn't have it</summary>
public sealed unsafe class MockBox : IDisposable
{
    readonly MediusMockBoxHandle handle;

    public MockBox()
    {
        RequireMock();
        handle = MediusImports.medius_mock_new();
        if (handle.IsInvalid)
        {
            throw new MediusException(Status.ErrUnknown, "the mock couldn't be made, which only a bug in medius does, so report it", 0);
        }
    }

    MockBox(MediusMockBoxHandle handle) => this.handle = handle;

    internal MediusMockBoxHandle Handle
    {
        get
        {
            ObjectDisposedException.ThrowIf(handle.IsClosed, this);
            return handle;
        }
    }

    /// <summary>a <see cref="Device"/> over this mock, after the handshake</summary>
    public Device Open() => Device.OpenMock(this);

    /// <summary>a <see cref="Device"/> over this mock, without the handshake</summary>
    public Device WithDevice() => Device.WithMock(this);

    /// <summary>another handle sharing the same recorded state</summary>
    public MockBox Clone()
    {
        var clone = MediusImports.medius_mock_clone(Handle);
        if (clone.IsInvalid)
        {
            throw new MediusException(Status.ErrUnknown, "cloning the mock failed, which only a bug in medius does, so report it", 0);
        }
        return new(clone);
    }

    public void SetVersion(BoxVersion version) => MediusImports.medius_mock_set_version(Handle, version.ToNative());
    public void SetHealth(Health health) => MediusImports.medius_mock_set_health(Handle, health.ToNative());
    public void SetDeviceInfo(DeviceInfo info) => MediusImports.medius_mock_set_device_info(Handle, info.ToNative());
    public void SetCaps(Caps caps) => MediusImports.medius_mock_set_caps(Handle, caps.ToNative());
    public void SetMouseCaps(MouseCaps caps) => MediusImports.medius_mock_set_mouse_caps(Handle, caps.ToNative());
    public void SetKbdCaps(KbdCaps caps) => MediusImports.medius_mock_set_kbd_caps(Handle, caps.ToNative());
    public void SetRate(Rate rate) => MediusImports.medius_mock_set_rate(Handle, rate.ToNative());
    public void SetStats(Stats stats) => MediusImports.medius_mock_set_stats(Handle, stats.ToNative());
    public void SetLocks(Locks locks) => MediusImports.medius_mock_set_locks(Handle, locks.ToNative());
    public void SetCatchState(CatchState state) => MediusImports.medius_mock_set_catch_state(Handle, state.ToNative());

    /// <summary>the mock's <see cref="Device.QueryImperfect"/> answer. with the opt in off it drops its consuming
    /// clip packet triggers, like the box does</summary>
    public void SetImperfectStatus(ImperfectStatus status) => MediusImports.medius_mock_set_imperfect_status(Handle, status.ToNative());

    /// <summary>the canned answer to a transfer while the opt in is on, with it off the mock answers refused. a
    /// status the package has no name for is sent as its number</summary>
    public void SetTransferReply(TransferStatus status, ReadOnlySpan<byte> data = default)
    {
        fixed (byte* bytes = data)
        {
            MediusImports.medius_mock_set_transfer_reply(Handle, (byte)status, bytes, (nuint)data.Length);
        }
    }

    /// <summary>the rate the mock's clone advertises when nothing forces one, 0 is no clone</summary>
    public void SetAdvertisedHz(ushort hz) => MediusImports.medius_mock_set_advertised_hz(Handle, hz);

    public void SetMovementRiding(uint? windowMs) =>
        MediusImports.medius_mock_set_movement_riding(Handle, Flag(windowMs is not null), windowMs ?? 0);

    /// <summary>the mock's <see cref="Device.QueryBearing"/> answer</summary>
    public void SetBearing(Bearing bearing) =>
        MediusImports.medius_mock_set_bearing(Handle, bearing.WindowMs ?? 0, In(bearing.Mode, "the bearing's mode"));

    public void SetEmitPace(EmitPace pace, ushort? forceHz = null) =>
        MediusImports.medius_mock_set_emit_pace(Handle, (byte)pace.Mode, pace.Hz, forceHz ?? 0);

    /// <summary>the command period the mock learned, in microseconds. a real box learns it from the moves it gets,
    /// so a mock left at 0 answers with a span of 0 whatever the percent</summary>
    public void SetSpreadLearned(uint periodUs) => MediusImports.medius_mock_set_spread_learned(Handle, periodUs);

    /// <summary>the mock's <see cref="Device.QueryRender"/> answer, <paramref name="ready"/> is whether a profile
    /// is armed, which a real box needs before it renders</summary>
    public void SetRender(RenderMode mode, bool full, bool ready) =>
        MediusImports.medius_mock_set_render(Handle, In(mode, nameof(mode)), Flag(full), Flag(ready));

    public void SetClipStatus(ClipStatus status) => MediusImports.medius_mock_set_clip_status(Handle, status.ToNative());

    /// <summary>the mock's <see cref="ClipHandle.QueryConfig"/> answer. its packet triggers get bound in order like
    /// <see cref="ClipHandle.BindPacket"/> binds them, and the mock keeps only the ones the box would</summary>
    public void SetClipSettings(ClipSettings settings) => MediusImports.medius_mock_set_clip_settings(Handle, settings.ToNative());

    /// <summary>runs one packet through the packet triggers like the box does, and gives back the action the most
    /// specific matching trigger drives (null when none matches or a once per run trigger's run goes on) and
    /// whether it consumes the packet</summary>
    public (ClipAction? Action, bool Consumed) ClipPacket(TrafficClass trafficClass, ushort id, Direction direction, ReadOnlySpan<byte> head)
    {
        byte action;
        byte consumed;
        byte fired;
        fixed (byte* bytes = head)
        {
            fired = MediusImports.medius_mock_clip_packet(
                Handle, In(trafficClass, nameof(trafficClass)), id, In(direction, nameof(direction)), bytes, (nuint)head.Length, &action, &consumed);
        }
        return (fired != 0 ? Out((ClipAction)action, "a clip action") : null, consumed != 0);
    }

    /// <summary>acts like the device chip restarting: the session state goes, the stored state stays, and the
    /// clone is back 100 ms later</summary>
    public void Restart() => MediusImports.medius_mock_restart(Handle);

    /// <summary>acts like the link between the chips dropping and coming back: host set session state goes,
    /// counted in <see cref="Stats.Session"/>, and the clone stays up</summary>
    public void LinkLost() => MediusImports.medius_mock_link_lost(Handle);

    /// <summary>acts like the real device unplugging. with <paramref name="backWithinGrace"/> it's back inside the
    /// 250 ms grace and the clone stays up, otherwise the clone goes when the grace ends until
    /// <see cref="Attach"/></summary>
    public void Detach(bool backWithinGrace = false) => MediusImports.medius_mock_detach(Handle, Flag(backWithinGrace));

    public void Attach() => MediusImports.medius_mock_attach(Handle);

    /// <summary>refuses the next activate with that update status (protocol §4.16), and whatever is staged stays
    /// staged</summary>
    public void RefuseNextActivate(byte status) => MediusImports.medius_mock_refuse_next_activate(Handle, status);

    /// <summary>after each accepted activate, leaves the next <paramref name="unanswered"/> firmware queries
    /// unanswered like a rebooting box, then has the device chip on probation for the next
    /// <paramref name="pending"/></summary>
    public void RebootAfterActivate(uint unanswered, uint pending) =>
        MediusImports.medius_mock_reboot_after_activate(Handle, unanswered, pending);

    /// <summary>stops answering queries, for good, for timeout tests</summary>
    public void Silent() => MediusImports.medius_mock_silent(Handle);

    public void PushRaw(ReadOnlySpan<byte> data)
    {
        if (data.IsEmpty)
        {
            return;
        }
        fixed (byte* bytes = data)
        {
            MediusImports.medius_mock_push_raw(Handle, bytes, (nuint)data.Length);
        }
    }

    public void PushLog(LogLevel level, string text)
    {
        fixed (byte* bytes = Library.Utf8(text))
        {
            MediusImports.medius_mock_push_log(Handle, In(level, nameof(level)), bytes);
        }
    }

    public void PushMotion(byte seq, uint tsUs, MotionEvent motion) => MediusImports.medius_mock_push_motion(Handle, seq, tsUs, motion.ToNative());

    public void PushUsages(byte seq, uint tsUs, UsageSnapshot usages)
    {
        var raw = usages.ToNative();
        MediusImports.medius_mock_push_usages(Handle, seq, tsUs, &raw);
    }

    /// <summary>a traffic event as if from the box, a <see cref="TrafficEvent.TrueLen"/> above the byte count is a
    /// capture that was cut</summary>
    public void PushTraffic(byte seq, uint tsUs, ClockDomain clock, TrafficEvent traffic)
    {
        var raw = traffic.ToNative();
        MediusImports.medius_mock_push_traffic(Handle, seq, tsUs, In(clock, nameof(clock)), &raw);
    }

    public int Recorded => (int)MediusImports.medius_mock_recorded(Handle);

    public bool Saw(FrameType frameType) => MediusImports.medius_mock_saw(Handle, In(frameType, nameof(frameType))) != 0;

    public void ClearRecorded() => MediusImports.medius_mock_clear_recorded(Handle);

    /// <summary>frame number <paramref name="index"/> the mock got, null past the end</summary>
    public RecordedFrame? RecordedFrame(int index)
    {
        if (index < 0 || index >= Recorded)
        {
            return null;
        }
        var capacity = Constants.MaxDevPayload;
        while (true)
        {
            var payload = new byte[capacity];
            FrameType type;
            byte seq;
            nuint full;
            fixed (byte* first = payload)
            {
                full = MediusImports.medius_mock_recorded_frame(Handle, (nuint)index, &type, &seq, first, (nuint)capacity);
            }
            if (full <= (nuint)capacity)
            {
                return new(type, seq, payload.AsMemory(0, (int)full));
            }
            capacity = (int)full;
        }
    }

    public void Dispose() => handle.Dispose();
}
