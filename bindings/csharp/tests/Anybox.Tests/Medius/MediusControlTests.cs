using System.Diagnostics;
using System.Text.RegularExpressions;
using Anybox.Medius;
using Anybox.Tests;
using static MediusTests.MediusClipTests;
using static MediusTests.MediusDeviceTests;
using Medius = Anybox.Medius;

namespace MediusTests;

// the medius Python package's input, timeline, transform and advanced control tests, over the medius mock
public sealed class MediusControlTests
{
    static readonly TimeSpan Wait = TimeSpan.FromSeconds(2);

    static void Until(Func<bool> condition, string what)
    {
        var clock = Stopwatch.StartNew();
        while (!condition())
        {
            Assert.True(clock.Elapsed < TimeSpan.FromSeconds(3), what);
            Thread.Sleep(5);
        }
    }

    [Fact]
    public void TheStatusCodesAreTheHeaders()
    {
        var header = MockLibrary.ReadFile("bindings/c/include/medius.h");
        var declared = Regex.Matches(header, @"^    MEDIUS_STATUS_(\w+) = (\d+),$", RegexOptions.Multiline)
            .ToDictionary(match => match.Groups[1].Value.Replace("_", ""), match => int.Parse(match.Groups[2].Value), StringComparer.OrdinalIgnoreCase);
        Assert.Equal(declared.Count, Enum.GetValues<Status>().Length);
        foreach (var status in Enum.GetValues<Status>())
        {
            Assert.Equal(declared[status.ToString()], (int)status);
        }
        Assert.Equal(33, (int)Status.ErrClipPacketTrigger);
    }

    [Fact]
    public void EveryStatusHasItsException()
    {
        foreach (var status in Enum.GetValues<Status>().Where(status => status is not (Status.Ok or Status.ErrUnknown)))
        {
            var error = MediusException.For(status, "text", 3);
            Assert.Equal($"{status.ToString()["Err".Length..]}Exception", error.GetType().Name);
            Assert.Equal((status, (byte)3, "text"), (error.MediusStatus, error.ProtoVer, error.Message));
            Assert.IsAssignableFrom<Anybox.AnyboxException>(error);
        }
        Assert.Equal(typeof(MediusException), MediusException.For(Status.ErrUnknown, "", 0).GetType());
        Assert.Equal("ErrUnknown", MediusException.For(Status.ErrUnknown, "", 0).Message);
        Assert.Equal(typeof(MediusException), MediusException.For((Status)99, "x", 0).GetType());
    }

    [Fact]
    public void InputEventsTurnSnapshotsIntoEdges()
    {
        Usage escape = Medius.Key.Escape;
        Usage a = Medius.Key.A;
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var stream = device.InputEvents([.. CatchFilter.AllInput()]);
        mock.PushUsages(1, 1000, new UsageSnapshot(Class.Key, Direction.Press, [escape]));
        mock.PushUsages(2, 2000, new UsageSnapshot(Class.Key, Direction.Press, [escape, a]));
        mock.PushUsages(3, 3000, new UsageSnapshot(Class.Key, Direction.Release, [a]));
        mock.PushMotion(4, 4000, new MotionEvent(3, -4, 0));
        var first = stream.RecvTimeout(Wait)!;
        Assert.Equal((InputKind.Press, (Usage?)escape, 1000u), (first.Kind, first.Usage, first.TsUs));
        Assert.Equal((0, 0, 0), ((int)first.Dx, (int)first.Dy, (int)first.Dz));
        Assert.Equal(a, stream.RecvTimeout(Wait)!.Usage);
        var release = stream.RecvTimeout(Wait)!;
        Assert.Equal((InputKind.Release, (Usage?)escape), (release.Kind, release.Usage));
        Assert.Equal([a], stream.Held(Class.Key));
        var motion = stream.RecvTimeout(Wait)!;
        Assert.Equal((InputKind.Motion, 3, -4, 0), (motion.Kind, (int)motion.Dx, (int)motion.Dy, (int)motion.Dz));
        Assert.Null(motion.Usage);
        Assert.Null(stream.TryRecv());
    }

    [Fact]
    public void InputEventsRefuseWhatTheyCantDecode()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        Assert.Throws<NotAnInputFilterException>(() => device.InputEvents(CatchFilter.TrafficOf(TrafficClass.VendorBulk)));
        Assert.Throws<WildcardNotInputException>(() => device.InputEvents(CatchFilter.Everything()));
        Assert.Throws<HalfEdgeInputFilterException>(() => device.InputEvents(CatchFilter.Watch(Medius.Key.A).OnPress()));
        Assert.Throws<CaptureNotApplicableException>(() => device.CatchEvents(CatchFilter.WatchClass(Class.Key).WithCapture(8)));
        // across this ABI a media usage of 0xFFFF is the class blanket
        Assert.Null(CatchFilter.Watch(Usage.Media((MediaKey)0xFFFF)).Id);
        Assert.Equal(CatchFilter.WatchClass(Class.Media), CatchFilter.Watch(Usage.Media((MediaKey)0xFFFF)));
        using var stream = device.InputEvents([.. CatchFilter.AllInput()]);
        Assert.Equal(0ul, stream.Dropped);
    }

    [Fact]
    public void TheFilterFactoriesAddressInputsLikeLockDoes()
    {
        Assert.Equal(CatchFilter.Watch(Usage.Key(Medius.Key.A)), CatchFilter.Watch(Medius.Key.A));
        Assert.Equal(CatchFilter.Watch(Usage.Button(0)), CatchFilter.Watch(Medius.Button.Left));
        Assert.Equal(CatchFilter.Watch(Usage.Media(MediaKey.VolumeUp)), CatchFilter.Watch(MediaKey.VolumeUp));
        var key = CatchFilter.Watch(Medius.Key.A);
        Assert.Equal((CatchClass.Key, (ushort)Medius.Key.A), (key.Class!.Value, key.Id!.Value));
        Assert.Equal((CatchClass.Button, (ushort)0), (CatchFilter.Watch(Medius.Button.Left).Class!.Value, CatchFilter.Watch(Medius.Button.Left).Id!.Value));
        Assert.Equal((ushort)Axis.Wheel, CatchFilter.WatchAxis(Axis.Wheel).Id);
        Assert.Null(CatchFilter.WatchAxes().Id);
        Assert.Equal([CatchClass.Button, CatchClass.Key, CatchClass.Media, CatchClass.Axis], CatchFilter.AllInput().Select(filter => filter.Class!.Value));
        var bulk = CatchFilter.Traffic(TrafficClass.VendorBulk, 3);
        Assert.True(bulk.SameAddress(bulk.WithCapture(16)));
        Assert.False(bulk.SameAddress(bulk.Outbound()));
        Assert.NotEqual(bulk, bulk.WithCapture(16));
        Assert.Equal(Medius.Capture.Whole, CatchFilter.Everything().Capture);
        Assert.True(CatchClass.Key.IsInput);
        Assert.True(CatchClass.VendorBulk.IsTraffic);
    }

    [Fact]
    public void AnUnknownControlStatusDoesntThrow()
    {
        var unknown = new TrafficEvent(CatchClass.Control, 0, Direction.In, 0x03, 8, new byte[8]);
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var stream = device.CatchEvents(CatchFilter.TrafficOf(TrafficClass.Control));
        mock.PushTraffic(1, 7000, ClockDomain.DeviceChip, unknown);
        var traffic = stream.RecvTimeout(Wait)!.Traffic!;
        Assert.Equal(ControlStatus.Other, traffic.Control);
        Assert.Equal(0x03, traffic.Flags);
    }

    [Fact]
    public void ATimelineUnwrapsTheRolloverOntoYourClock()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var stream = device.CatchEvents(CatchFilter.WatchAxes());
        using var timeline = new Timeline();
        mock.PushMotion(1, 0xFFFFFE0C, new MotionEvent(1, 0, 0));
        mock.PushMotion(2, 500, new MotionEvent(1, 0, 0));
        var a = timeline.Observe(stream.RecvTimeout(Wait)!, 50_000_000);
        var b = timeline.Observe(stream.RecvTimeout(Wait)!, 51_001_000);
        Assert.Equal(0xFFFFFE0Cul, a.BoxUs);
        Assert.Equal((1ul << 32) + 500, b.BoxUs);
        Assert.Equal(1_000_000ul, b.HostNs - a.HostNs);
        Assert.Equal(1000ul, b.ExcessNs);
        Assert.Equal(2ul, timeline.Samples(ClockDomain.HostChip));
        timeline.Reset(ClockDomain.HostChip);
        Assert.Equal(0ul, timeline.Samples(ClockDomain.HostChip));
    }

    [Fact]
    public void ATimelinePlacesInputEventsToo()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var stream = device.InputEvents(CatchFilter.WatchAxes());
        using var timeline = new Timeline();
        mock.PushMotion(1, 1000, new MotionEvent(1, 0, 0));
        var stamped = timeline.Observe(stream.RecvTimeout(Wait)!);
        Assert.Equal(1000ul, stamped.BoxUs);
        Assert.Equal(1ul, timeline.Samples(ClockDomain.HostChip));
    }

    [Fact]
    public void EveryEnumGoingInIsCheckedBeforeItCrosses()
    {
        using var mock = new MockBox();
        using (var device = Device.WithMock(mock))
        {
            Assert.Throws<ArgumentOutOfRangeException>(() => device.Led(LedTarget.Device, (LedMode)77, 5));
            Assert.Throws<ArgumentOutOfRangeException>(() => device.Led((LedTarget)200, LedMode.Solid, 55));
            Assert.Throws<ArgumentOutOfRangeException>(() => device.Reboot((RebootTarget)200));
            Assert.Throws<ArgumentOutOfRangeException>(() => device.Inject(Medius.Button.Left, (InjectAction)200));
            Assert.Throws<ArgumentOutOfRangeException>(() => device.MoveAxis(Motion.Cursor(1, 2), (MoveTiming)9));
            Assert.Throws<ArgumentOutOfRangeException>(() => device.MoveAxis(Motion.Cursor(1, 2), MoveTiming.Ride, (PendingMotion)9));
            Assert.Equal(0, mock.Recorded);
            device.Led(LedTarget.Both, LedMode.Blink, 128);
        }
        Assert.Equal([[2, 3, 128]], Frames(mock, FrameType.Led));
    }

    [Fact]
    public void TheMockAndStreamEnumsAreChecked()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        Assert.Throws<ArgumentOutOfRangeException>(() => mock.PushLog((LogLevel)200, "hi"));
        Assert.Throws<ArgumentOutOfRangeException>(() => mock.Saw((FrameType)200));
        Assert.Throws<ArgumentOutOfRangeException>(() => mock.PushTraffic(0, 0, (ClockDomain)200, new TrafficEvent(CatchClass.HidIn, 1, Direction.In)));
        Assert.Throws<ArgumentOutOfRangeException>(() => mock.SetDeviceInfo(new DeviceInfo(1, 2, 3, 4, false, false, (DeviceKind)200, "")));
        Assert.Throws<ArgumentOutOfRangeException>(() => mock.SetClipStatus(new ClipStatus((ClipState)200, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, [])));
        Assert.Throws<ArgumentOutOfRangeException>(() => mock.PushUsages(0, 0, new UsageSnapshot((Class)200, Direction.Positive, [])));
        using (var timeline = new Timeline())
        {
            Assert.Throws<ArgumentOutOfRangeException>(() => timeline.Reset((ClockDomain)200));
            Assert.Throws<ArgumentOutOfRangeException>(() => timeline.Samples((ClockDomain)200));
        }
        using var stream = device.InputEvents([.. CatchFilter.AllInput()]);
        Assert.Throws<ArgumentOutOfRangeException>(() => stream.Held((Class)200));
    }

    [Fact]
    public void TheNewHealthBitsComeBack()
    {
        var health = new Health(true, false, false, false, false, false, false, false, RewriteOn: true, PatchOn: false, TransformOn: true);
        using var mock = new MockBox();
        mock.SetHealth(health);
        using var device = Device.WithMock(mock);
        Assert.Equal(health, device.QueryHealth());
    }

    [Fact]
    public void TheAdvancedFramesCarryTheirType()
    {
        using var mock = new MockBox();
        mock.SetImperfectStatus(Allowed);
        using (var device = Device.WithMock(mock))
        {
            device.Raw(1, Direction.In, [0, 1, 2, 3]);
            device.SetRewrite(new RewriteRule(RewriteClass.Emit, 1, Direction.In, RewriteAction.Drop));
            device.SetPatch(new Patch(PatchSection.Device, 0, 0, 8, new byte[] { 0x34, 0x12 }));
            device.Transfer(0, new Setup(0x80, 0x06, 0x0100, 0, 18));
        }
        Assert.True(mock.Saw(FrameType.Raw));
        Assert.True(mock.Saw(FrameType.Rewrite));
        Assert.True(mock.Saw(FrameType.Patch));
        Assert.True(mock.Saw(FrameType.Transfer));
    }

    [Fact]
    public void RawGoesOutAsItIs()
    {
        using var mock = new MockBox();
        mock.SetImperfectStatus(Allowed);
        using (var device = Device.WithMock(mock))
        {
            device.Raw(1, Direction.In, [0x00, 0x01, 0x00, 0x00]);
        }
        // [ep][dir][bytes]
        Assert.Equal([[0x01, 0x01, 0x00, 0x01, 0x00, 0x00]], Frames(mock, FrameType.Raw));
    }

    [Fact]
    public void TheGatedCallsNeedTheOptIn()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        Assert.Throws<ImperfectRequiredException>(() => device.SetRewrite(new RewriteRule(RewriteClass.Emit, 1, Direction.In, RewriteAction.Drop)));
        Assert.Throws<ImperfectRequiredException>(device.ApplyPatch);
        // raw doesn't read the opt in, the box drops it there
        device.Raw(1, Direction.In, [0x00, 0x01]);
        Assert.True(mock.Saw(FrameType.Raw));
    }

    [Fact]
    public void RawRefusesADirectionThatIsntAFlow()
    {
        using var mock = new MockBox();
        mock.SetImperfectStatus(Allowed);
        using var device = Device.WithMock(mock);
        Assert.Throws<RawDirectionException>(() => device.Raw(1, Direction.Both, [0]));
        Assert.Throws<RelativeDirectionException>(() => device.Raw(1, Direction.With, [0]));
    }

    [Fact]
    public void ATransferBringsBackItsAnswer()
    {
        byte[] reply = [0x12, 0x01, 0x10, 0x02];
        using var mock = new MockBox();
        mock.SetImperfectStatus(Allowed);
        mock.SetTransferReply(TransferStatus.Ok, reply);
        using var device = Device.WithMock(mock);
        var outcome = device.Transfer(0, new Setup(0x80, 0x06, 0x0100, 0, 18));
        Assert.Equal(new TransferOutcome(TransferStatus.Ok, reply), outcome);
        Assert.True(outcome.IsOk);
        var timed = device.Transfer(0, new Setup(0x80, 0x06, 0x0100, 0, 2), timeout: TimeSpan.FromMilliseconds(MediusLibrary.DefaultTransferTimeoutMs));
        Assert.Equal(TransferStatus.Ok, timed.Status);
        Assert.True(MediusLibrary.DefaultTransferTimeoutMs >= 800);
        var tooLong = Assert.Throws<ArgumentOutOfRangeException>(
            () => device.Transfer(0, new Setup(0x80, 0x06, 0x0100, 0, 2), timeout: TimeSpan.FromMilliseconds(4294967296.0)));
        Assert.Contains("a timeout is from 0 to 4294967295 ms", tooLong.Message);
    }

    [Fact]
    public void ATransferIsRefusedWithoutTheOptIn()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        var outcome = device.Transfer(0, new Setup(0x80, 0x06, 0x0100, 0, 18));
        Assert.Equal(TransferStatus.Refused, outcome.Status);
        Assert.False(outcome.IsOk);
        Assert.True(outcome.Data.IsEmpty);
    }

    [Fact]
    public void ATransferThatIsntOkCarriesNoData()
    {
        using var mock = new MockBox();
        mock.SetImperfectStatus(Allowed);
        mock.SetTransferReply(TransferStatus.Stall, [0x12, 0x01, 0x00, 0x02]);
        using var device = Device.WithMock(mock);
        var outcome = device.Transfer(0, new Setup(0x80, 0x06, 0x0100, 0, 18));
        Assert.Equal(TransferStatus.Stall, outcome.Status);
        Assert.True(outcome.Data.IsEmpty);
    }

    [Fact]
    public void ARewriteSurvivesTheQuery()
    {
        var payload = Enumerable.Repeat((byte)0xAB, 40).ToArray();
        var rule = new RewriteRule(RewriteClass.Control, 0, Direction.Both, RewriteAction.ReplyPatch, 258, new byte[] { 0x80, 0x06 }, new byte[] { 0xFF, 0xFF }, payload);
        using var mock = new MockBox();
        mock.SetImperfectStatus(Allowed);
        using var device = Device.WithMock(mock);
        device.SetRewrite(rule);
        var entry = Assert.Single(device.QueryRewrite().Entries);
        Assert.Equal((RewriteClass.Control, RewriteAction.ReplyPatch, (ushort)258, (ushort)40), (entry.RewriteClass, entry.Action, entry.Offset, entry.PayloadLen));
        Assert.Equal(rule, device.QueryRewriteEntry(0));
        device.ClearRewrite();
        Assert.Empty(device.QueryRewrite().Entries);
    }

    [Fact]
    public void ARewritesRefusalsHaveTheirOwnException()
    {
        using var mock = new MockBox();
        mock.SetImperfectStatus(Allowed);
        using var device = Device.WithMock(mock);
        Assert.Throws<RewriteMaskLengthException>(() => device.SetRewrite(new RewriteRule(RewriteClass.Emit, 1, Direction.In, RewriteAction.Drop, MatchBytes: new byte[] { 1, 2 }, Mask: new byte[] { 0xFF })));
        Assert.Throws<RewriteActionClassException>(() => device.SetRewrite(new RewriteRule(RewriteClass.Control, 0, Direction.Both, RewriteAction.Drop)));
        Assert.Throws<ArgumentOutOfRangeException>(() => device.SetRewrite(new RewriteRule(RewriteClass.HidIn, 2, Direction.In, RewriteAction.ReplyReplace + 1)));
        Assert.Throws<RelativeDirectionException>(() => device.SetRewrite(new RewriteRule(RewriteClass.Emit, 1, Direction.With, RewriteAction.Drop)));
        Assert.Throws<RewritePayloadTooLargeException>(() => device.SetRewrite(new RewriteRule(RewriteClass.Emit, 1, Direction.In, RewriteAction.Replace, Payload: new byte[100])));
        // the struct holds 512 payload bytes, a longer one is refused instead of cut
        Assert.Throws<ArgumentException>(() => device.SetRewrite(new RewriteRule(RewriteClass.Control, 0, Direction.Both, RewriteAction.Answer, Payload: new byte[513])));
        Assert.Throws<ArgumentException>(() => device.SetPatch(new Patch(PatchSection.Device, 0, 0, 0, new byte[513])));
        // a removal only reads the key, so its payload can be anything, like in Rust
        device.RemoveRewrite(new RewriteRule(RewriteClass.Control, 0, Direction.Both, RewriteAction.Answer, Payload: new byte[513]));
    }

    [Fact]
    public void ARewriteMatchPastTheLimitHasItsOwnException()
    {
        static RewriteRule Rule(int match, int mask) => new(RewriteClass.HidIn, 2, Direction.In, RewriteAction.Pass,
            MatchBytes: Enumerable.Repeat((byte)0x11, match).ToArray(), Mask: Enumerable.Repeat((byte)0xFF, mask).ToArray());
        using var mock = new MockBox();
        mock.SetImperfectStatus(Allowed);
        using var device = Device.WithMock(mock);
        foreach (var length in new[] { 17, 40, 0xFFFF })
        {
            Assert.Equal(Status.ErrRewriteMatchTooLong, Assert.Throws<RewriteMatchTooLongException>(() => device.SetRewrite(Rule(length, length))).MediusStatus);
            Assert.Throws<RewriteMatchTooLongException>(() => device.RemoveRewrite(Rule(length, length)));
        }
        Assert.Throws<RewriteMaskLengthException>(() => device.SetRewrite(Rule(17, 16)));
        Assert.Throws<RewriteMaskLengthException>(() => device.SetRewrite(Rule(16, 17)));
        Assert.Empty(device.QueryRewrite().Entries);
        device.SetRewrite(Rule(16, 16));
        Assert.Equal(Rule(16, 16), device.QueryRewriteEntry(0));
    }

    [Fact]
    public void ARewritePastThePayloadPoolHasItsOwnException()
    {
        static RewriteRule Answer(int id, int length) => new(RewriteClass.Control, (ushort)id, Direction.Both, RewriteAction.Answer, Payload: Enumerable.Repeat((byte)0x5A, length).ToArray());
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        mock.SetImperfectStatus(Allowed);
        for (var id = 0; id < 4; id++)
        {
            device.SetRewrite(Answer(id, 501));
        }
        Assert.Throws<RewritePoolFullException>(() => device.SetRewrite(Answer(4, 45)));
        device.SetRewrite(Answer(4, 44));
        Assert.Equal(5, device.QueryRewrite().Entries.Count);
    }

    [Fact]
    public void APatchSurvivesTheQuery()
    {
        var data = Enumerable.Repeat((byte)0xCD, 40).ToArray();
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        device.SetPatch(new Patch(PatchSection.Report, 1, 2, 258, data));
        var entry = Assert.Single(device.QueryPatches().Entries);
        Assert.Equal(new PatchEntry(PatchSection.Report, 1, 2, 258, 40), entry);
        Assert.Equal(new Patch(PatchSection.Report, 1, 2, 258, data), device.QueryPatchEntry(0));
    }

    [Fact]
    public void ApplyAndClearPatchReachTheBox()
    {
        using var mock = new MockBox();
        mock.SetImperfectStatus(Allowed);
        using var device = Device.WithMock(mock);
        device.SetPatch(new Patch(PatchSection.Device, 0, 0, 8, new byte[] { 0x34, 0x12 }));
        Assert.True(device.QueryPatches().Pending);
        device.ApplyPatch();
        Assert.True(device.QueryPatches().Applied);
        device.ClearPatch();
        Assert.Empty(device.QueryPatches().Entries);
    }

    [Fact]
    public void AnEmptyPatchRemovesTheStoredOne()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        device.SetPatch(new Patch(PatchSection.Device, 0, 0, 8, new byte[] { 0x34, 0x12 }));
        Assert.Single(device.QueryPatches().Entries);
        device.SetPatch(new Patch(PatchSection.Device, 0, 0, 8));
        Assert.Empty(device.QueryPatches().Entries);
    }

    [Fact]
    public void PanFramesCarryTheMotionTag()
    {
        using var mock = new MockBox();
        using (var device = Device.WithMock(mock))
        {
            device.Pan(3);
            device.PanNow(-2);
            device.MoveAxis(Motion.Pan(4), MoveTiming.Now, PendingMotion.Keep);
        }
        // [motion=2][dpan i16 LE][flags], ride and keep is 0, now is 1
        Assert.Equal([[2, 3, 0, 0x00], [2, 0xFE, 0xFF, 0x01], [2, 4, 0, 0x01]], Frames(mock, FrameType.Move));
    }

    [Fact]
    public void ButtonsPastFiveGoByTheirId()
    {
        var usage = Usage.Button(7);
        Assert.Equal((Class.Button, (ushort)7), (usage.Kind, usage.Id));
        using var mock = new MockBox();
        using (var device = Device.WithMock(mock))
        {
            device.Press(Usage.Button(7));
            device.Lock(LockTarget.Button(9), Direction.Press);
        }
        // [class=0][id u16 LE][action=press]
        var inject = mock.RecordedFrame(0)!;
        Assert.Equal(FrameType.Inject, inject.Type);
        Assert.Equal([0, 7, 0, 1], inject.Payload.ToArray());
    }

    [Fact]
    public void TheTransformCallsReachTheBoxWithoutTheOptIn()
    {
        using var mock = new MockBox();
        using (var device = Device.WithMock(mock))
        {
            device.TransformSwap(Axis.X, Axis.Y);
            device.TransformRemap(Axis.X, Axis.Wheel);
            device.Transform(Transform.Remap(Axis.Wheel, Axis.Y));
            device.Untransform(Transform.Remap(Axis.Wheel, Axis.Y));
            device.ClearTransforms();
        }
        Assert.Equal(5, Frames(mock, FrameType.Transform).Length);
    }

    [Fact]
    public void ATransformSurvivesTheQuery()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        device.TransformSwap(Axis.X, Axis.Y);
        device.TransformRemap(Axis.Wheel, Axis.Y);
        device.TransformRemap(Axis.Y, Axis.X);
        var got = device.QueryTransforms();
        Assert.False(got.TableFull);
        Assert.Equal(
            [(TransformOp.Swap, LockTargetKind.X, LockTargetKind.Y), (TransformOp.Remap, LockTargetKind.Wheel, LockTargetKind.Y), (TransformOp.Remap, LockTargetKind.Y, LockTargetKind.X)],
            got.Entries.Select(entry => (entry.Op, entry.Source.Kind, entry.Dest.Kind)));
        Assert.Equal((0, 1), ((int)TransformOp.Remap, (int)TransformOp.Swap));
    }

    [Fact]
    public void APanAxisTransformWorksLikeAnyOther()
    {
        using var mock = new MockBox();
        mock.SetMouseCaps(new MouseCaps(5, true, true, true, true, false, 1));
        using var device = Device.WithMock(mock);
        Assert.True(device.Caps().Mouse.Pan);
        device.TransformRemap(Axis.Wheel, Axis.Pan);
        var entry = Assert.Single(device.QueryTransforms().Entries);
        Assert.Equal((TransformOp.Remap, LockTargetKind.Pan), (entry.Op, entry.Dest.Kind));
    }

    [Fact]
    public void ATransformsRefusalsHaveTheirOwnException()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        Assert.Throws<TransformOpFieldsException>(() => device.Transform(new Transform(TransformOp.Remap, LockTarget.X, LockTarget.Button(0))));
        Assert.Throws<TransformOpFieldsException>(() => device.Transform(new Transform(TransformOp.Swap, LockTarget.Y, LockTarget.Y)));
        Assert.Throws<TransformOpFieldsException>(() => device.Transform(new Transform(TransformOp.Remap, LockTarget.Button(0), LockTarget.Button(0))));
    }

    [Fact]
    public void ANegativeScaleReversesAndIsRefusedWhereItCant()
    {
        var x = LockTarget.X;
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        device.Scale(x, Direction.Both, -100);
        var locks = device.QueryLocks();
        Assert.Equal((-100, -100), ((int)locks.ScaleOf(x, Direction.Positive), (int)locks.ScaleOf(x, Direction.Negative)));
        Assert.False(locks.IsLocked(x, Direction.Both));
        var before = mock.Recorded;
        Assert.Throws<LockScaleUsageException>(() => device.Scale(LockTarget.Button(0), Direction.Positive, -100));
        Assert.Throws<LockScaleRangeException>(() => device.Scale(x, Direction.Both, Constants.LockScaleMin - 1));
        Assert.Throws<LockScaleRangeException>(() => device.Scale(x, Direction.Both, Constants.LockScaleMax + 1));
        Assert.Equal(before, mock.Recorded);
    }

    [Fact]
    public void AnInputEventCarriesPan()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var stream = device.InputEvents([.. CatchFilter.AllInput()]);
        mock.PushMotion(1, 4000, new MotionEvent(3, -4, 0, 5));
        var motion = stream.RecvTimeout(Wait)!;
        Assert.Equal((InputKind.Motion, 3, -4, 0, 5), (motion.Kind, (int)motion.Dx, (int)motion.Dy, (int)motion.Dz, (int)motion.Pan));
    }

    [Fact]
    public void AMockRestartIsRecoveredAndTheClipSaysItWasLost()
    {
        using var mock = new MockBox();
        using var device = mock.Open();
        using var clip = device.Clip();
        device.Press(Medius.Button.Side1);
        clip.SetRetain(true);
        using (var builder = new ClipBuilder())
        {
            clip.Append(builder.Move(1, 0));
        }
        Assert.False(clip.Lost);
        mock.ClearRecorded();
        mock.Restart();
        Until(() => device.Counters().Restarts >= 1, "the restart was never seen");
        Assert.True(clip.Lost);
        // the held press and the retain setting went back to the restarted box
        Assert.Equal([[0, 3, 0, 1]], Frames(mock, FrameType.Inject));
        Assert.Equal([[2, 1]], Frames(mock, FrameType.ClipSet));
        using (var builder = new ClipBuilder())
        {
            clip.Append(builder.Move(1, 0));
        }
        Assert.False(clip.Lost);
    }

    [Fact]
    public void AClonePresentedAgainGetsBackWhatTheLibraryHeld()
    {
        var x = LockTarget.X;
        using var mock = new MockBox();
        mock.SetImperfectStatus(Allowed);
        using var device = mock.Open();
        device.SetPatch(new Patch(PatchSection.Device, 0, 0, 12, new byte[] { 0x00, 0x01 }));
        device.Scale(x, Direction.Both, 40);
        mock.ClearRecorded();
        device.ApplyPatch();
        Until(() => Frames(mock, FrameType.Lock).Length > 0, "the scale never went back");
        Assert.True(device.QueryPatches().Applied);
        Assert.Equal(40, device.QueryLocks().ScaleOf(x, Direction.Positive));
        Assert.Equal(0ul, device.Counters().Restarts);
    }

    [Fact]
    public void AReleaseTheBoxCountsIsRecovered()
    {
        var x = LockTarget.X;
        using var mock = new MockBox();
        using var device = mock.Open();
        device.Scale(x, Direction.Both, 40);
        mock.LinkLost();
        Assert.Equal(1, device.QueryStats().Session);
        Until(() => device.QueryLocks().ScaleOf(x, Direction.Positive) == 40, "the scale never went back");
        mock.Detach(backWithinGrace: false);
        mock.Attach();
        Until(() => device.QueryLocks().ScaleOf(x, Direction.Positive) == 40, "the scale never went back after a replug");
        Assert.Equal(0ul, device.Counters().Restarts);
    }
}
