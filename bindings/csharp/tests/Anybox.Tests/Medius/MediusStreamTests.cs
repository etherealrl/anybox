using System.Diagnostics;
using Anybox.Medius;
using Medius = Anybox.Medius;

namespace MediusTests;

// the medius Python package's stream and traffic tests, over the medius mock
public sealed class MediusStreamTests
{
    static readonly TimeSpan Wait = TimeSpan.FromSeconds(2);

    static CatchEvent PushAndRecv(MockBox mock, EventStream stream, TrafficEvent traffic, ClockDomain clock = ClockDomain.DeviceChip)
    {
        mock.PushTraffic(1, 7000, clock, traffic);
        var got = stream.RecvTimeout(Wait);
        Assert.NotNull(got);
        Assert.Equal(CatchEventKind.Traffic, got.Kind);
        return got;
    }

    [Fact]
    public void AForcedRateComesBack()
    {
        using var mock = new MockBox();
        mock.SetAdvertisedHz(125);
        mock.SetImperfectStatus(new ImperfectStatus(true, false, false));
        mock.SetEmitPace(EmitPace.Fixed(500), 400);
        using (var device = Device.WithMock(mock))
        {
            var status = device.QueryEmitPace();
            Assert.Equal(EmitPace.Fixed(500), status.Mode);
            Assert.Equal((500, (ushort?)400, 500, true), ((int)status.ResolvedHz, status.ForceHz, (int)status.AdvertisedHz, status.ForceActive));
        }
        mock.SetEmitPace(EmitPace.Fixed(500));
        using (var device = Device.WithMock(mock))
        {
            var status = device.QueryEmitPace();
            Assert.Equal(((ushort?)null, false, 125), (status.ForceHz, status.ForceActive, (int)status.AdvertisedHz));
        }
    }

    [Fact]
    public void AForcedRateNeedsTheImperfectOptIn()
    {
        using var mock = new MockBox();
        mock.SetAdvertisedHz(125);
        using var device = Device.WithMock(mock);
        device.SetEmitPace(EmitPace.Learned, forceHz: 1000);
        var status = device.QueryEmitPace();
        Assert.Equal(((ushort?)1000, false, 125), (status.ForceHz, status.ForceActive, (int)status.AdvertisedHz));
        device.AllowImperfectClones(true);
        status = device.QueryEmitPace();
        Assert.Equal((true, 1000), (status.ForceActive, (int)status.AdvertisedHz));
    }

    [Fact]
    public void AForcedRateGoesThroughTheSetter()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        device.AllowImperfectClones(true);
        device.SetEmitPace(EmitPace.Fixed(250), forceHz: 125);
        var status = device.QueryEmitPace();
        Assert.Equal(EmitPace.Fixed(250), status.Mode);
        Assert.Equal(((ushort?)125, 125, true), (status.ForceHz, (int)status.AdvertisedHz, status.ForceActive));
        device.SetEmitPace(EmitPace.Fixed(250), forceHz: null);
        Assert.Null(device.QueryEmitPace().ForceHz);
    }

    [Fact]
    public void TheCountersCanBeRead()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        device.MoveRel(1, 0);
        Assert.True(device.Counters().FramesTx >= 1);
    }

    [Fact]
    public void ACatchDeliversMotion()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var stream = device.CatchEvents(CatchFilter.Everything());
        mock.PushMotion(1, 7000, new MotionEvent(12, -34, 1, 2));
        var got = stream.RecvTimeout(Wait);
        Assert.NotNull(got);
        Assert.Equal((CatchEventKind.Motion, 7000u), (got.Kind, got.TsUs));
        Assert.Equal(new MotionEvent(12, -34, 1, 2), got.Motion);
    }

    [Fact]
    public void ACatchDeliversAKey()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var stream = device.CatchEvents(CatchFilter.WatchClass(Class.Key));
        mock.PushUsages(1, 7000, new UsageSnapshot(Class.Key, Direction.Press, [Medius.Key.Escape]));
        var got = stream.RecvTimeout(Wait);
        Assert.NotNull(got);
        Assert.Equal(CatchEventKind.Usages, got.Kind);
        Assert.True(got.Usages!.IsHeld(Medius.Key.Escape));
        Assert.False(got.Usages.IsHeld(Medius.Key.A));
    }

    [Fact]
    public void ACatchDeliversMedia()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var stream = device.CatchEvents(CatchFilter.Everything());
        mock.PushUsages(1, 7000, new UsageSnapshot(Class.Media, Direction.Press, [MediaKey.VolumeUp]));
        var got = stream.RecvTimeout(Wait);
        Assert.NotNull(got);
        Assert.True(got.Usages!.IsHeld(MediaKey.VolumeUp));
    }

    [Fact]
    public void ACatchDeliversTraffic()
    {
        var sent = new TrafficEvent(CatchClass.HidIn, 2, Direction.Positive, 0, 6, new byte[] { 1, 2, 3, 4, 5, 6 });
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var stream = device.CatchEvents(CatchFilter.TrafficOf(TrafficClass.HidIn));
        var got = PushAndRecv(mock, stream, sent, ClockDomain.HostChip);
        Assert.Equal((7000u, ClockDomain.HostChip), (got.TsUs, got.Clock));
        Assert.Null(got.Motion);
        Assert.Null(got.Usages);
        Assert.Equal(sent, got.Traffic);
        Assert.False(got.Traffic!.Truncated);
        Assert.Equal(sent.Bytes.ToArray(), got.Traffic.Data.ToArray());
    }

    [Fact]
    public void ATrueLenAboveTheCaptureIsATruncation()
    {
        var cut = new TrafficEvent(CatchClass.VendorBulk, 3, Direction.Positive, 0x03, 512, Enumerable.Range(0, 16).Select(value => (byte)value).ToArray());
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var stream = device.CatchEvents(CatchFilter.Traffic(TrafficClass.VendorBulk, 3).WithCapture(16));
        var traffic = PushAndRecv(mock, stream, cut).Traffic!;
        Assert.Equal(512, traffic.TrueLen);
        Assert.Equal(16, traffic.Bytes.Length);
        Assert.True(traffic.Truncated);
        Assert.True(traffic.BulkEndOfTransfer);
        Assert.True(traffic.Zlp);
        Assert.False(new TrafficEvent(CatchClass.VendorBulk, 3, Direction.Positive, 0, 16, new byte[16]).Truncated);
    }

    [Fact]
    public void AControlEventsAccessorsReadItsBytes()
    {
        byte[] setup = [0x80, 0x06, 0x00, 0x01, 0x00, 0x00, 0x12, 0x00];
        var stalled = new TrafficEvent(CatchClass.Control, 0, Direction.Positive, 0x81, 8, setup);
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var stream = device.CatchEvents(CatchFilter.TrafficOf(TrafficClass.Control));
        var traffic = PushAndRecv(mock, stream, stalled).Traffic!;
        Assert.Equal(setup, traffic.SetupBytes!.Value.ToArray());
        Assert.True(traffic.Data.IsEmpty);
        Assert.Equal(ControlStatus.Stalled, traffic.Control);
        Assert.True(traffic.RuleActed);
        Assert.Null(traffic.Bus);
        var answered = new TrafficEvent(CatchClass.Control, 0, Direction.Positive, 0, 10, setup.Concat(new byte[] { 0x12, 0x01 }).ToArray());
        Assert.Equal(ControlStatus.Ok, answered.Control);
        Assert.False(answered.RuleActed);
        Assert.Equal([0x12, 0x01], answered.Data.ToArray());
        Assert.Equal(ControlStatus.Naked, new TrafficEvent(CatchClass.Control, 0, Direction.Positive, 0x02, 8, setup).Control);
    }

    [Fact]
    public void AClipTransfersAccessorsReadItsBytes()
    {
        byte[] setup = [0xA1, 0x01, 0x00, 0x03, 0x00, 0x00, 0x02, 0x00];
        var answered = new TrafficEvent(CatchClass.ClipTransfer, 0, Direction.In, (byte)TransferStatus.Ok, 10, setup.Concat(new byte[] { 4, 1 }).ToArray());
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var stream = device.CatchEvents(CatchFilter.TrafficOf(TrafficClass.ClipTransfer));
        var traffic = PushAndRecv(mock, stream, answered).Traffic!;
        Assert.Equal(answered, traffic);
        Assert.True(traffic.CatchClass.IsTraffic);
        Assert.Equal(setup, traffic.SetupBytes!.Value.ToArray());
        Assert.Equal([4, 1], traffic.Data.ToArray());
        Assert.Equal(TransferStatus.Ok, traffic.Transfer);
        Assert.True(traffic.Transfer!.Value.IsOk);
        Assert.Null(traffic.Control);
        var stalled = new TrafficEvent(CatchClass.ClipTransfer, 0, Direction.In, 0xFD, 8, setup);
        Assert.Equal(TransferStatus.Stall, stalled.Transfer);
        Assert.False(stalled.RuleActed);
        Assert.True(stalled.Data.IsEmpty);
        Assert.Equal(TransferStatus.Nak, new TrafficEvent(CatchClass.ClipTransfer, 0, Direction.In, 0xFE, 8, setup).Transfer);
        // a status nothing names stays its byte
        Assert.Equal((TransferStatus)0x42, new TrafficEvent(CatchClass.ClipTransfer, 0, Direction.In, 0x42, 8, setup).Transfer);
        Assert.Null(new TrafficEvent(CatchClass.Control, 0, Direction.In, 0, 8, setup).Transfer);
    }

    [Fact]
    public void ABusEventIsDecoded()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var stream = device.CatchEvents(CatchFilter.TrafficOf(TrafficClass.Bus));
        var traffic = PushAndRecv(mock, stream, new TrafficEvent(CatchClass.Bus, 0, Direction.Both, 5, 2, new byte[] { 3, 1 })).Traffic!;
        var bus = traffic.Bus;
        Assert.NotNull(bus);
        Assert.Equal(BusEventKind.SetInterface, bus.Kind);
        Assert.Equal((3, 1), ((int)bus.Interface, (int)bus.Alt));
        Assert.Null(traffic.Control);
    }

    [Fact]
    public void TrafficComesOnEveryWayOfReceiving()
    {
        var sent = new TrafficEvent(CatchClass.Emit, 1, Direction.Positive, 0, 3, new byte[] { 1, 2, 3 });
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var stream = device.CatchEvents(CatchFilter.TrafficOf(TrafficClass.Emit));
        mock.PushTraffic(1, 10, ClockDomain.DeviceChip, sent);
        var blocking = stream.Recv();
        mock.PushTraffic(2, 20, ClockDomain.DeviceChip, sent);
        CatchEvent? polled = null;
        var clock = Stopwatch.StartNew();
        while (polled is null && clock.Elapsed < Wait)
        {
            polled = stream.TryRecv();
        }
        mock.PushTraffic(3, 30, ClockDomain.DeviceChip, sent);
        var timed = stream.RecvTimeout(Wait);
        foreach (var got in new[] { blocking, polled, timed })
        {
            Assert.NotNull(got);
            Assert.Equal(CatchEventKind.Traffic, got.Kind);
            Assert.Equal(sent, got.Traffic);
        }
    }

    [Fact]
    public void CatchEventsNeedsAFilter()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        Assert.Throws<EmptySubscriptionException>(() => device.CatchEvents());
        Assert.Throws<EmptySubscriptionException>(() => device.InputEvents());
    }

    [Fact]
    public void TryRecvGivesNullWhenEmpty()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var stream = device.CatchEvents(CatchFilter.Everything());
        Assert.Null(stream.TryRecv());
        Assert.Equal(0ul, stream.Dropped);
    }

    [Fact]
    public void ALogLineComes()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var logs = device.Logs();
        mock.PushLog(LogLevel.Warn, "hello world");
        var line = logs.RecvTimeout(Wait);
        Assert.Equal(new LogLine(LogLevel.Warn, "hello world"), line);
    }

    [Fact]
    public void ACloneSharesTheConnection()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var clone = device.Clone();
        device.MoveRel(1, 0);
        clone.MoveRel(2, 0);
        using var mockClone = mock.Clone();
        Assert.Equal(2, mockClone.Recorded);
    }

    [Fact]
    public void AStreamCloneSharesTheSubscription()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var stream = device.CatchEvents(CatchFilter.Everything());
        using var clone = stream.Clone();
        mock.PushMotion(1, 7000, new MotionEvent(9, 0, 0));
        var got = clone.RecvTimeout(Wait);
        Assert.Equal((short?)9, got?.Motion?.Dx);
    }

    [Fact]
    public void DisposingTwiceIsFine()
    {
        var mock = new MockBox();
        var device = Device.WithMock(mock);
        device.Dispose();
        device.Dispose();
        mock.Dispose();
        mock.Dispose();
        Assert.Throws<ObjectDisposedException>(() => device.MoveRel(1, 1));
        Assert.Throws<ObjectDisposedException>(() => mock.Silent());
    }

    [Fact]
    public void ASnapshotsIsHeldTakesEveryClass()
    {
        var snapshot = new UsageSnapshot(Class.Key, Direction.Press, [Medius.Button.Right, Medius.Key.LeftCtrl, Medius.Key.A]);
        Assert.True(snapshot.IsHeld(Medius.Button.Right));
        Assert.True(snapshot.IsHeld(Medius.Key.LeftCtrl));
        Assert.True(snapshot.IsHeld(Medius.Key.A));
        Assert.False(snapshot.IsHeld(Medius.Button.Left));
        Assert.False(snapshot.IsHeld(Medius.Key.B));
    }
}
