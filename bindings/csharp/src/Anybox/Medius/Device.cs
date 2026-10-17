using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using static Anybox.Medius.MediusLibrary;

namespace Anybox.Medius;

/// <summary>an open connection to one Medius box, with everything only Medius does. one you got from
/// <c>device.Medius()</c> shares anybox's connection, and keeping it holds the port open after the anybox device
/// is gone. anybox doesn't know about anything you press or lock through it, and <see cref="Reset"/> lets go of
/// and unlocks EVERYTHING, what anybox did too. medius.h has no close, so <see cref="Dispose"/> is the only one,
/// and the last clone of a connection waits for its background threads there</summary>
public sealed unsafe class Device : IDisposable
{
    readonly MediusDeviceHandle handle;

    internal Device(MediusDeviceHandle handle) => this.handle = handle;

    MediusDeviceHandle Handle
    {
        get
        {
            ObjectDisposedException.ThrowIf(handle.IsClosed, this);
            return handle;
        }
    }

    static Device Opened(Status status, MediusDeviceHandle handle)
    {
        Check(status);
        return new(handle);
    }

    public static Device Open(string path)
    {
        Ensure();
        fixed (byte* text = Library.Utf8(path))
        {
            return Opened(MediusImports.medius_device_open(text, out var opened), opened);
        }
    }

    public static Device Find()
    {
        Ensure();
        return Opened(MediusImports.medius_device_find(out var opened), opened);
    }

    /// <summary>the box whose id matches, its device MAC as hex or its CH343 serial. it throws
    /// <see cref="BadProtoVerException"/> when that box speaks another control protocol</summary>
    public static Device OpenById(string boxId)
    {
        Ensure();
        fixed (byte* text = Library.Utf8(boxId))
        {
            return Opened(MediusImports.medius_device_open_by_id(text, out var opened), opened);
        }
    }

    /// <summary>the first box whose clone is a mouse</summary>
    public static Device FindMouseBox()
    {
        Ensure();
        return Opened(MediusImports.medius_device_find_mouse_box(out var opened), opened);
    }

    /// <summary>the first box whose clone is a keyboard</summary>
    public static Device FindKeyboardBox()
    {
        Ensure();
        return Opened(MediusImports.medius_device_find_keyboard_box(out var opened), opened);
    }

    /// <summary>the first box <paramref name="match"/> says yes to, preferring one this package speaks to. a box on
    /// another control protocol is only picked when nothing else matches, and opening it throws
    /// <see cref="BadProtoVerException"/></summary>
    public static Device FindWhere(Func<BoxInfo, bool> match)
    {
        var boxes = List();
        // medius' own text for not finding a box, since this picks the box on this side
        var found = boxes.FirstOrDefault(box => box.Device is not null && match(box)) ?? boxes.FirstOrDefault(match) ??
            throw new NotFoundException("no medius device found", 0);
        return Open(found.Port.Path);
    }

    /// <summary>a device over a <see cref="MockBox"/>, after the version handshake</summary>
    public static Device OpenMock(MockBox mock)
    {
        RequireMock();
        return Opened(MediusImports.medius_device_open_mock(mock.Handle, out var opened), opened);
    }

    /// <summary>a device over a <see cref="MockBox"/>, without the handshake</summary>
    public static Device WithMock(MockBox mock)
    {
        RequireMock();
        return Opened(MediusImports.medius_device_with_mock(mock.Handle, out var opened), opened);
    }

    // asks with room for 16 and once more with room for all of them when there were more
    delegate nuint Listing<T>(T* entries, nuint capacity, nuint* total) where T : unmanaged;

    static T[] List<T>(Listing<T> listing) where T : unmanaged
    {
        var capacity = 16;
        while (true)
        {
            var entries = new T[capacity];
            nuint total;
            nuint written;
            fixed (T* first = entries)
            {
                written = listing(first, (nuint)capacity, &total);
            }
            if (total <= (nuint)capacity)
            {
                return entries[..(int)written];
            }
            capacity = (int)total;
        }
    }

    /// <summary>every serial port that could be a box</summary>
    public static IReadOnlyList<PortInfo> FindPorts()
    {
        Ensure();
        return [.. List<MediusPortInfo>(MediusImports.medius_find_ports).Select(PortInfo.From)];
    }

    /// <summary>every box there is with what it says it is. one on another control protocol comes with a null
    /// <see cref="BoxInfo.Device"/></summary>
    public static IReadOnlyList<BoxInfo> List()
    {
        Ensure();
        return [.. List<MediusBoxInfo>(MediusImports.medius_list).Select(BoxInfo.From)];
    }

    /// <summary>another handle to the same connection</summary>
    public Device Clone()
    {
        var clone = MediusImports.medius_device_clone(Handle);
        if (clone.IsInvalid)
        {
            throw new MediusException(Status.ErrUnknown, "cloning the device failed, which only a bug in medius does, so report it", 0);
        }
        return new(clone);
    }

    public void MoveRel(short dx, short dy) => Check(MediusImports.medius_device_move_rel(Handle, dx, dy));
    public void Wheel(short delta) => Check(MediusImports.medius_device_wheel(Handle, delta));

    /// <summary>a cursor move that skips movement riding, sent on the box's next mouse report</summary>
    public void MoveRelNow(short dx, short dy) => Check(MediusImports.medius_device_move_rel_now(Handle, dx, dy));

    public void WheelNow(short delta) => Check(MediusImports.medius_device_wheel_now(Handle, delta));

    /// <summary>horizontal scroll, the whole range with no clamp</summary>
    public void Pan(short delta) => Check(MediusImports.medius_device_pan(Handle, delta));

    public void PanNow(short delta) => Check(MediusImports.medius_device_pan_now(Handle, delta));

    /// <summary>sends the motion held for a ride now, whatever the ride window says</summary>
    public void FlushMotion() => Check(MediusImports.medius_device_flush_motion(Handle));

    /// <summary>drops the motion held for a ride</summary>
    public void DiscardMotion() => Check(MediusImports.medius_device_discard_motion(Handle));

    public void MoveAxis(Motion motion, MoveTiming timing = MoveTiming.Ride, PendingMotion pending = PendingMotion.Keep) =>
        Check(MediusImports.medius_device_move_axis(Handle, motion.ToNative(), In(timing, nameof(timing)), In(pending, nameof(pending))));

    public void Inject(Usage input, InjectAction action) =>
        Check(MediusImports.medius_device_inject(Handle, input.ToNative(), In(action, nameof(action))));

    public void Press(Usage input) => Check(MediusImports.medius_device_press(Handle, input.ToNative()));
    public void SoftRelease(Usage input) => Check(MediusImports.medius_device_soft_release(Handle, input.ToNative()));
    public void ForceRelease(Usage input) => Check(MediusImports.medius_device_force_release(Handle, input.ToNative()));

    public void Lock(LockTarget target, Direction direction) =>
        Check(MediusImports.medius_device_lock(Handle, target.ToNative(), In(direction, nameof(direction))));

    public void Unlock(LockTarget target, Direction direction) =>
        Check(MediusImports.medius_device_unlock(Handle, target.ToNative(), In(direction, nameof(direction))));

    /// <summary>weighs physical input on a target and direction. <paramref name="scale"/> is the percent kept: 0
    /// blocks, 100 passes, above that amplifies up to 255, and a negative one down to
    /// <see cref="Constants.LockScaleMin"/> weighs and reverses (axes only). <see cref="Lock"/> and
    /// <see cref="Unlock"/> are its two ends. see <c>Device::scale</c> in the medius docs for how the fixed and
    /// relative scales multiply</summary>
    public void Scale(LockTarget target, Direction direction, short scale) =>
        Check(MediusImports.medius_device_scale(Handle, target.ToNative(), In(direction, nameof(direction)), scale));

    /// <summary>weighs a whole blanket, see <see cref="Scale"/> for what the number means</summary>
    public void ScaleAll(Blanket what, Direction direction, short scale) =>
        Check(MediusImports.medius_device_scale_all(Handle, In(what, nameof(what)), In(direction, nameof(direction)), scale));

    /// <summary>blocks a whole blanket. <see cref="Blanket.Keys"/> follows the direction, positive blocks the
    /// presses only and negative the releases only</summary>
    public void LockAll(Blanket what, Direction direction) =>
        Check(MediusImports.medius_device_lock_all(Handle, In(what, nameof(what)), In(direction, nameof(direction))));

    public void UnlockAll(Blanket what, Direction direction) =>
        Check(MediusImports.medius_device_unlock_all(Handle, In(what, nameof(what)), In(direction, nameof(direction))));

    public void Led(LedTarget target, LedMode mode, byte level) =>
        Check(MediusImports.medius_device_led(Handle, In(target, nameof(target)), In(mode, nameof(mode)), level));

    public void Reset() => Check(MediusImports.medius_device_reset(Handle));

    /// <summary>a reset that also erases the box's stored settings, its name and everything it learned, and
    /// reboots it. while it reboots the port stays there but quiet, so queries time out</summary>
    public void FactoryReset() => Check(MediusImports.medius_device_factory_reset(Handle));

    public void Reapply() => Check(MediusImports.medius_device_reapply(Handle));

    /// <summary>finds and opens this box again and sends it the held state. it throws
    /// <see cref="BadProtoVerException"/> when the box answers on another control protocol, and stays
    /// disconnected</summary>
    public void Reconnect() => Check(MediusImports.medius_device_reconnect(Handle));

    public void Reboot(RebootTarget target) => Check(MediusImports.medius_device_reboot(Handle, In(target, nameof(target))));

    /// <summary>opts in to cloning devices the box can't clone faithfully, or back out. a change to the served
    /// patch set presents the clone again, which lets go of the session like a replug</summary>
    public void AllowImperfectClones(bool allow) => Check(MediusImports.medius_device_allow_imperfect_clones(Handle, Flag(allow)));

    /// <summary>the movement riding window in ms, null turns it off</summary>
    public void SetMovementRiding(uint? windowMs) =>
        Check(MediusImports.medius_device_set_movement_riding(Handle, Flag(windowMs is not null), Window(windowMs)));

    // a window in whole ms from 1 to 65535 like the Rust API, since 0 is off on the wire and a zero window isn't
    static ushort Window(uint? windowMs) => windowMs is { } ms ? (ushort)Math.Clamp(ms, 1u, ushort.MaxValue) : (ushort)0;

    /// <summary>how injected motion is paced, and the rate the clone advertises and the box polls the device at
    /// (<paramref name="forceHz"/>, null is the device's own)</summary>
    public void SetEmitPace(EmitPace pace, ushort? forceHz = null) =>
        Check(MediusImports.medius_device_set_emit_pace(Handle, (byte)pace.Mode, pace.Hz, forceHz ?? 0));

    /// <summary>the box's name, kept across power cuts, an empty one clears it</summary>
    public void SetName(string name)
    {
        fixed (byte* text = Library.Utf8(name))
        {
            Check(MediusImports.medius_device_set_name(Handle, text));
        }
    }

    /// <summary>back to the made up Medius-XXXX name</summary>
    public void ClearName() => Check(MediusImports.medius_device_clear_name(Handle));

    /// <summary>what <see cref="Direction.With"/> and <see cref="Direction.Against"/> are measured against.
    /// <paramref name="windowMs"/> is how long the last injected direction stays the bearing, null turns it off and
    /// it saturates at 65535 ms. both go in one frame and are kept together, so the mode is always needed</summary>
    public void SetBearing(uint? windowMs, BearingMode mode) =>
        Check(MediusImports.medius_device_set_bearing(Handle, Window(windowMs), In(mode, nameof(mode))));

    /// <summary>the render texture, and whether native motion goes through the model too. both are one command
    /// and kept, so <paramref name="full"/> is always needed. nothing renders until the box learned the attached
    /// device's profile (<see cref="RenderStatus.Ready"/>)</summary>
    public void SetRender(RenderMode mode, bool full) =>
        Check(MediusImports.medius_device_set_render(Handle, In(mode, nameof(mode)), Flag(full)));

    /// <summary>the percent of the host's command interval an injected delta is spread across, 0 puts it all on
    /// the next report and 100 spreads it over one interval</summary>
    public void SetSpread(ushort percent) => Check(MediusImports.medius_device_set_spread(Handle, percent));

    public BoxVersion QueryVersion()
    {
        MediusVersion raw;
        Check(MediusImports.medius_device_query_version(Handle, &raw));
        return BoxVersion.From(raw);
    }

    /// <summary>both chips' firmware versions and the slots they booted</summary>
    public FirmwareInfo FirmwareInfo()
    {
        MediusFirmwareInfo raw;
        Check(MediusImports.medius_device_firmware_info(Handle, &raw));
        return Medius.FirmwareInfo.From(raw);
    }

    /// <summary>waits until neither chip is still on probation, a chip on probation refuses an update. it keeps
    /// asking while the box reboots after an activate. after 55 s a chip still on probation is an
    /// <see cref="UpdateException"/>, and a box that never answered a <see cref="QueryTimeoutException"/></summary>
    public FirmwareInfo WaitFirmwareConfirmed()
    {
        MediusFirmwareInfo raw;
        Check(MediusImports.medius_device_wait_firmware_confirmed(Handle, &raw));
        return Medius.FirmwareInfo.From(raw);
    }

    // what the progress callback reaches through `user`. an exception can't go back through Rust's frames, so the
    // first one gets kept here and thrown once the native call returned
    sealed class Reporting(IProgress<UpdateProgress> progress, UpdateTarget target)
    {
        internal Exception? Failed;

        internal void Report(nuint sent, nuint total)
        {
            if (Failed is not null)
            {
                return;
            }
            try
            {
                progress.Report(new UpdateProgress(target, sent, total));
            }
            catch (Exception error)
            {
                Failed = error;
            }
        }
    }

    // runs on the calling thread while stage_firmware is going
    [UnmanagedCallersOnly]
    static void Progressed(void* user, nuint sent, nuint total) => ((Reporting)GCHandle.FromIntPtr((nint)user).Target!).Report(sent, total);

    /// <summary>writes one image into that chip's spare slot, where it does nothing until
    /// <see cref="ActivateFirmware"/>. <paramref name="progress"/> gets the bytes sent so far and the total, on
    /// this thread. if it throws, the update gets aborted once the image is through and its exception comes out
    /// of here</summary>
    public void StageFirmware(UpdateTarget target, ReadOnlySpan<byte> image, IProgress<UpdateProgress>? progress = null)
    {
        var targetByte = In(target, nameof(target));
        if (progress is null)
        {
            fixed (byte* bytes = image)
            {
                Check(MediusImports.medius_device_stage_firmware(Handle, targetByte, bytes, (nuint)image.Length, null, null));
            }
            return;
        }
        var reporting = new Reporting(progress, target);
        var user = GCHandle.Alloc(reporting);
        Status status;
        try
        {
            fixed (byte* bytes = image)
            {
                status = MediusImports.medius_device_stage_firmware(
                    Handle, targetByte, bytes, (nuint)image.Length, &Progressed, (void*)GCHandle.ToIntPtr(user));
            }
        }
        finally
        {
            user.Free();
        }
        if (reporting.Failed is { } failed)
        {
            // what your progress threw is what you need, a stage or an abort that failed after it isn't
            try
            {
                AbortUpdate(target);
            }
            catch
            {
            }
            System.Runtime.ExceptionServices.ExceptionDispatchInfo.Throw(failed);
        }
        Check(status);
    }

    /// <summary>drops whatever is staged or on its way for one chip</summary>
    public void AbortUpdate(UpdateTarget target) => Check(MediusImports.medius_device_abort_update(Handle, In(target, nameof(target))));

    /// <summary>commits every staged image and reboots into it, the host chip first. it waits until the host chip
    /// is back</summary>
    public void ActivateFirmware() => Check(MediusImports.medius_device_activate_firmware(Handle));

    /// <summary>stages one image and activates it. a refused activate aborts the staged image, so it can't go
    /// live with some later activate, like the crate</summary>
    // the same steps as the crate's own update_firmware, not medius_device_update_firmware, because a progress
    // that throws has to stop the update before the activate, and inside that one call it can't
    public void UpdateFirmware(UpdateTarget target, ReadOnlySpan<byte> image, IProgress<UpdateProgress>? progress = null)
    {
        StageFirmware(target, image, progress);
        try
        {
            ActivateFirmware();
        }
        catch
        {
            // the activate's error is what the caller needs, a failed abort on top of it isn't, like the crate
            try
            {
                AbortUpdate(target);
            }
            catch
            {
            }
            throw;
        }
    }

    public Health QueryHealth()
    {
        MediusHealth raw;
        Check(MediusImports.medius_device_query_health(Handle, &raw));
        return Health.From(raw);
    }

    public DeviceInfo DeviceInfo()
    {
        MediusDeviceInfo raw;
        Check(MediusImports.medius_device_device_info(Handle, &raw));
        return Medius.DeviceInfo.From(raw);
    }

    public Caps Caps()
    {
        MediusCaps raw;
        Check(MediusImports.medius_device_caps(Handle, &raw));
        return Medius.Caps.From(raw);
    }

    public Rate QueryRate()
    {
        MediusRate raw;
        Check(MediusImports.medius_device_query_rate(Handle, &raw));
        return Rate.From(raw);
    }

    public Stats QueryStats()
    {
        MediusStats raw;
        Check(MediusImports.medius_device_query_stats(Handle, &raw));
        return Stats.From(raw);
    }

    public Locks QueryLocks()
    {
        MediusLocks raw;
        Check(MediusImports.medius_device_query_locks(Handle, &raw));
        return Locks.From(raw);
    }

    public CatchState QueryCatch()
    {
        MediusCatchState raw;
        Check(MediusImports.medius_device_query_catch(Handle, &raw));
        return CatchState.From(raw);
    }

    public ImperfectStatus QueryImperfect()
    {
        MediusImperfectStatus raw;
        Check(MediusImports.medius_device_query_imperfect(Handle, &raw));
        return ImperfectStatus.From(raw);
    }

    /// <summary>the movement riding window in whole ms, null when it's off</summary>
    public uint? QueryMovementRiding()
    {
        byte enabled;
        uint window;
        Check(MediusImports.medius_device_query_movement_riding(Handle, &enabled, &window));
        return enabled != 0 ? window : null;
    }

    public EmitPaceStatus QueryEmitPace()
    {
        MediusEmitPaceStatus raw;
        Check(MediusImports.medius_device_query_emit_pace(Handle, &raw));
        return EmitPaceStatus.From(raw);
    }

    public Bearing QueryBearing()
    {
        MediusBearing raw;
        Check(MediusImports.medius_device_query_bearing(Handle, &raw));
        return Bearing.From(raw);
    }

    public RenderStatus QueryRender()
    {
        MediusRenderStatus raw;
        Check(MediusImports.medius_device_query_render(Handle, &raw));
        return RenderStatus.From(raw);
    }

    public SpreadStatus QuerySpread()
    {
        MediusSpreadStatus raw;
        Check(MediusImports.medius_device_query_spread(Handle, &raw));
        return SpreadStatus.From(raw);
    }

    public Counters Counters()
    {
        MediusCountersSnapshot raw;
        Check(MediusImports.medius_device_counters(Handle, &raw));
        return Medius.Counters.From(raw);
    }

    /// <summary>puts <paramref name="data"/> as it is on the cloned endpoint <paramref name="ep"/> (0 to 15),
    /// <see cref="Direction.In"/> to the game PC and <see cref="Direction.Out"/> to the real device, without
    /// waiting for anything. with the imperfect clone opt in off the box drops it silently and this still returns,
    /// <see cref="QueryImperfect"/> says which it is</summary>
    public void Raw(byte ep, Direction direction, ReadOnlySpan<byte> data)
    {
        fixed (byte* bytes = data)
        {
            Check(MediusImports.medius_device_raw(Handle, ep, In(direction, nameof(direction)), bytes, (nuint)data.Length));
        }
    }

    /// <summary>runs one control transfer on the real device and gives back its reply. a status that isn't ok is
    /// returned, not thrown, and the box answers refused while the opt in is off. the box gives up after about
    /// 800 ms on its own, so a shorter <paramref name="timeout"/> gives up before a slow device answers</summary>
    public TransferOutcome Transfer(byte ep, Setup setup, ReadOnlySpan<byte> data = default, TimeSpan? timeout = null)
    {
        MediusTransferOutcome raw;
        fixed (byte* bytes = data)
        {
            Check(timeout is { } wait
                ? MediusImports.medius_device_transfer_timeout(
                    Handle, ep, setup.ToNative(), bytes, (nuint)data.Length, TransferMs(wait), &raw)
                : MediusImports.medius_device_transfer(Handle, ep, setup.ToNative(), bytes, (nuint)data.Length, &raw));
        }
        return TransferOutcome.From(raw);
    }

    // rounded up like a stream's, so a timeout above zero never turns into no wait at all
    static uint TransferMs(TimeSpan timeout)
    {
        var ms = Ending.Ms(timeout);
        return ms <= uint.MaxValue
            ? (uint)ms
            : throw new ArgumentOutOfRangeException(nameof(timeout), "a timeout is from 0 to 4294967295 ms");
    }

    /// <summary>adds or overwrites one rewrite rule, it needs the opt in. <see cref="QueryRewrite"/> says what the
    /// box holds</summary>
    public void SetRewrite(RewriteRule rule)
    {
        var raw = rule.ToNative();
        Check(MediusImports.medius_device_set_rewrite(Handle, &raw));
    }

    /// <summary>drops the rule with that rule's key, its action and payload don't matter</summary>
    public void RemoveRewrite(RewriteRule rule)
    {
        // the payload isn't read, so one past what medius.h holds still removes like in Rust
        var raw = (rule with { Payload = default }).ToNative();
        Check(MediusImports.medius_device_remove_rewrite(Handle, &raw));
    }

    public void ClearRewrite() => Check(MediusImports.medius_device_clear_rewrite(Handle));

    public RewriteTable QueryRewrite()
    {
        MediusRewriteTable raw;
        Check(MediusImports.medius_device_query_rewrite(Handle, &raw));
        return RewriteTable.From(raw);
    }

    /// <summary>one rule in full, in the shape <see cref="SetRewrite"/> takes</summary>
    public RewriteRule QueryRewriteEntry(byte index)
    {
        MediusRewriteRule raw;
        Check(MediusImports.medius_device_query_rewrite_entry(Handle, index, &raw));
        return RewriteRule.From(raw);
    }

    /// <summary>stores one descriptor patch, empty bytes remove the one at its key. it reaches the game PC the
    /// next time the clone is presented under the opt in</summary>
    public void SetPatch(Patch patch)
    {
        var raw = patch.ToNative();
        Check(MediusImports.medius_device_set_patch(Handle, &raw));
    }

    /// <summary>presents the clone again with the stored patch set, it needs the opt in, and lets go of the session
    /// like a replug</summary>
    public void ApplyPatch() => Check(MediusImports.medius_device_apply_patch(Handle));

    public void ClearPatch() => Check(MediusImports.medius_device_clear_patch(Handle));

    public PatchSet QueryPatches()
    {
        MediusPatchSet raw;
        Check(MediusImports.medius_device_query_patches(Handle, &raw));
        return PatchSet.From(raw);
    }

    public Patch QueryPatchEntry(byte index)
    {
        MediusPatch raw;
        Check(MediusImports.medius_device_query_patch_entry(Handle, index, &raw));
        return Medius.Patch.From(raw);
    }

    /// <summary>adds or overwrites one field transform, no opt in needed</summary>
    public void Transform(Transform transform)
    {
        var raw = transform.ToNative();
        Check(MediusImports.medius_device_transform(Handle, &raw));
    }

    /// <summary>drops the transform with that source and destination, whatever its op</summary>
    public void Untransform(Transform transform)
    {
        var raw = transform.ToNative();
        Check(MediusImports.medius_device_untransform(Handle, &raw));
    }

    public void ClearTransforms() => Check(MediusImports.medius_device_clear_transforms(Handle));

    public void TransformSwap(Axis a, Axis b) => Check(MediusImports.medius_device_transform_swap(Handle, In(a, nameof(a)), In(b, nameof(b))));

    public void TransformRemap(LockTarget source, LockTarget dest) =>
        Check(MediusImports.medius_device_transform_remap(Handle, source.ToNative(), dest.ToNative()));

    public Transforms QueryTransforms()
    {
        MediusTransforms raw;
        Check(MediusImports.medius_device_query_transforms(Handle, &raw));
        return Transforms.From(raw);
    }

    /// <summary>this box's buffered clip playback</summary>
    public ClipHandle Clip()
    {
        Check(MediusImports.medius_device_clip(Handle, out var clip));
        return new(clip, this);
    }

    // an empty list goes over too, so medius answers it with its own EmptySubscriptionException
    static MediusCatchFilter[] Filters(ReadOnlySpan<CatchFilter> filters)
    {
        var raw = new MediusCatchFilter[filters.Length];
        for (var index = 0; index < filters.Length; index++)
        {
            raw[index] = filters[index].ToNative();
        }
        return raw;
    }

    /// <summary>subscribes to the catch stream. overlapping subscriptions merge into the box's one table, and
    /// every stream still gets everything it asked for</summary>
    public EventStream CatchEvents(params ReadOnlySpan<CatchFilter> filters)
    {
        var raw = Filters(filters);
        fixed (MediusCatchFilter* first = raw)
        {
            Check(MediusImports.medius_device_catch_events(Handle, first, (nuint)raw.Length, out var stream));
            return new(stream, this);
        }
    }

    /// <summary>subscribes to decoded input, presses, releases and motion. every filter has to be an input class
    /// covering both edges, so build them with the <c>Watch</c> ones or <see cref="CatchFilter.AllInput"/></summary>
    public InputStream InputEvents(params ReadOnlySpan<CatchFilter> filters)
    {
        var raw = Filters(filters);
        fixed (MediusCatchFilter* first = raw)
        {
            Check(MediusImports.medius_device_input_events(Handle, first, (nuint)raw.Length, out var stream));
            return new(stream, this);
        }
    }

    public LogStream Logs()
    {
        Check(MediusImports.medius_device_logs(Handle, out var stream));
        return new(stream, this);
    }

    public void Dispose() => handle.Dispose();
}
