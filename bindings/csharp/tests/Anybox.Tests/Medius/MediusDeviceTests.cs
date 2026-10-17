using System.Text.RegularExpressions;
using Anybox.Medius;
using Anybox.Tests;
using AnyboxException = Anybox.AnyboxException;
using Medius = Anybox.Medius;

namespace MediusTests;

// the medius Python package's own tests (bindings/python/tests in the medius repo), over the medius mock in the same
// library. the payload bytes are the wire format from the firmware's ctrl_proto.h, so they pin what each call
// actually sends, not just that it returned
public sealed class MediusDeviceTests
{
    internal static byte[][] Frames(MockBox mock, FrameType type) =>
    [
        .. Enumerable.Range(0, mock.Recorded)
            .Select(index => mock.RecordedFrame(index)!)
            .Where(frame => frame.Type == type)
            .Select(frame => frame.Payload.ToArray()),
    ];

    [Fact]
    public void TheMockIsThere() => Assert.True(MediusLibrary.HasMock);

    [Fact]
    public void TheMetaFunctionsAnswer()
    {
        Assert.Equal(10u, MediusLibrary.AbiVersion);
        Assert.NotEmpty(MediusLibrary.Version);
        Assert.True(MediusLibrary.DefaultQueryTimeoutMs > 0);
        Assert.True(MediusLibrary.DefaultKeepaliveCadenceMs > 0);
        Assert.True(MediusLibrary.DefaultTransferTimeoutMs > 0);
    }

    [Fact]
    public void TheAbiVersionIsTheHeaders()
    {
        var header = MockLibrary.ReadFile("bindings/c/include/medius.h");
        Assert.Equal(MediusLibrary.AbiVersion, uint.Parse(Regex.Match(header, @"^#define MEDIUS_ABI_VERSION (\d+)$", RegexOptions.Multiline).Groups[1].Value));
    }

    [Fact]
    public void ALibraryWithAnotherMediusAbiIsRefused()
    {
        MediusLibrary.Ensure();
        MediusLibrary.Load(MediusLibrary.AbiVersion);
        foreach (var other in new[] { MediusLibrary.AbiVersion - 1, MediusLibrary.AbiVersion + 1 })
        {
            var error = Assert.Throws<AnyboxException>(() => MediusLibrary.Load(other));
            Assert.Contains($"speaks medius C ABI {MediusLibrary.AbiVersion} and this package needs {other}", error.Message);
        }
    }

    [Fact]
    public void ASetVersionComesBackOnOpen()
    {
        using var mock = new MockBox();
        byte proto;
        using (var device = Device.WithMock(mock))
        {
            proto = device.QueryVersion().ProtoVer;
        }
        byte[] mac = [0x5A, 0x4E, 0x00, 0x11, 0x1E, 0x28];
        mock.SetVersion(new BoxVersion(proto, 9, 8, 7, mac, "Left PC"));
        using var opened = mock.Open();
        var version = opened.QueryVersion();
        Assert.Equal((proto, (byte)9, (byte)8, (byte)7, "Left PC"), (version.ProtoVer, version.FwMajor, version.FwMinor, version.FwPatch, version.Name));
        Assert.Equal(mac, version.Mac.ToArray());
        Assert.Equal("5a4e00111e28", version.MacHex);
    }

    [Fact]
    public void ANameCanBeSetAndCleared()
    {
        using var mock = new MockBox();
        using var device = mock.Open();
        // nothing is checked on the host, like the other setters, the box cleans it up
        device.SetName("Left PC");
        device.SetName(new string('x', 40));
        device.ClearName();
    }

    [Fact]
    public void ASilentMockFailsTheOpen()
    {
        using var mock = new MockBox();
        mock.Silent();
        Assert.ThrowsAny<MediusException>(() => Device.OpenMock(mock));
    }

    [Fact]
    public void ABadProtocolVersionSaysItsStatusAndProtocol()
    {
        using var mock = new MockBox();
        mock.SetVersion(new BoxVersion(99, 1, 0, 0, new byte[6], ""));
        var error = Assert.Throws<BadProtoVerException>(() => Device.OpenMock(mock));
        Assert.Equal(Status.ErrBadProtoVer, error.MediusStatus);
        Assert.Equal(99, error.ProtoVer);
        Assert.Null(error.Status);
    }

    [Fact]
    public unsafe void AListedBoxOnAnotherProtocolHasNoDevice()
    {
        var raw = new MediusBoxInfo();
        raw.version.proto_ver = 7;
        byte[] mac = [0x5A, 0x4E, 0x00, 0x00, 0x00, 0x01];
        mac.CopyTo(new Span<byte>(raw.version.mac, 6));
        var old = BoxInfo.From(raw);
        Assert.Null(old.Device);
        Assert.Equal(7, old.Version.ProtoVer);
        Assert.Equal("5a4e00000001", old.Id);
        raw.version.proto_ver = 8;
        raw.device.vid = 0x046D;
        raw.device.kind = (byte)DeviceKind.Mouse;
        raw.has_device = 1;
        Assert.Equal((DeviceKind.Mouse, (ushort)0x046D), (BoxInfo.From(raw).Device!.Kind, BoxInfo.From(raw).Device!.Vid));
    }

    [Fact]
    public void ARecordedFramesPayloadCanBeRead()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        device.MoveRel(1, 2);
        var frame = mock.RecordedFrame(0);
        Assert.NotNull(frame);
        Assert.Equal(FrameType.Move, frame.Type);
        Assert.False(frame.Payload.IsEmpty);
        Assert.Null(mock.RecordedFrame(99));
    }

    [Fact]
    public void MoveFramesCarryTheirRidingFlags()
    {
        using var mock = new MockBox();
        using (var device = Device.WithMock(mock))
        {
            device.MoveRel(7, -2);
            device.MoveRelNow(7, -2);
            device.WheelNow(3);
            device.FlushMotion();
            device.DiscardMotion();
            device.MoveAxis(Motion.Cursor(5, 5), MoveTiming.Now, PendingMotion.Flush);
        }
        Assert.Equal(
            [
                [0, 7, 0, 0xFE, 0xFF, 0x00],
                [0, 7, 0, 0xFE, 0xFF, 0x01],
                [1, 3, 0, 0x01],
                [0, 0, 0, 0, 0, 0x02],
                [0, 0, 0, 0, 0, 0x04],
                [0, 5, 0, 5, 0, 0x03],
            ],
            Frames(mock, FrameType.Move));
    }

    [Fact]
    public void TheOtherMotionCallsSendTheirFrames()
    {
        using var mock = new MockBox();
        using (var device = Device.WithMock(mock))
        {
            device.Wheel(-2);
            device.Pan(4);
            device.PanNow(-5);
            device.MoveAxis(Motion.Wheel(6));
            device.MoveAxis(Motion.Pan(-7));
        }
        var moves = Frames(mock, FrameType.Move);
        Assert.Equal(5, moves.Length);
        // the axis byte comes first, the delta after it in little endian, like the cursor's
        Assert.Equal([1, 0xFE, 0xFF], moves[0][..3]);
        Assert.Equal([1, 6, 0], moves[3][..3]);
        Assert.NotEqual(moves[1], moves[2]);
    }

    [Fact]
    public void CapsComeBackAsTheyWereSet()
    {
        var caps = new Caps(
            new MouseCaps(5, true, true, true, true, false, 2),
            new KbdCaps(6, false, true, false, true),
            false,
            true);
        using var mock = new MockBox();
        mock.SetCaps(caps);
        using var device = Device.WithMock(mock);
        var got = device.Caps();
        Assert.Equal(caps, got);
        Assert.True(got.HasMouse);
        Assert.True(got.HasKeyboard);
        Assert.True(got.IsComposite);
    }

    [Fact]
    public void TheRateComesBackWithItsNativeHz()
    {
        using var mock = new MockBox();
        mock.SetRate(new Rate(1000, 1000, true, false));
        using var device = Device.WithMock(mock);
        var got = device.QueryRate();
        Assert.Equal(((ushort)1000, (ushort)1000, true), (got.NativePeriodUs, got.PollPeriodUs, got.Confident));
        Assert.InRange(got.NativeHz!.Value, 999.5f, 1000.5f);
        Assert.Null(new Rate(0, 1000, false, true).NativeHz);
    }

    [Fact]
    public void LocksComeBackAndSayWhatsLocked()
    {
        var x = LockTarget.X;
        using var mock = new MockBox();
        mock.SetLocks(new Locks([new LockEntry(x, false, Direction.Positive, 0), new LockEntry(x, false, Direction.Negative, 0)]));
        using var device = Device.WithMock(mock);
        var locks = device.QueryLocks();
        Assert.Equal(2, locks.Entries.Count);
        Assert.True(locks.IsLocked(x, Direction.Both));
        Assert.False(locks.IsLocked(LockTarget.Y, Direction.Both));
    }

    [Fact]
    public void LocksCarryAScaleNotJustALock()
    {
        var x = LockTarget.X;
        using var mock = new MockBox();
        mock.SetLocks(new Locks([new LockEntry(x, false, Direction.Against, 40), new LockEntry(x, false, Direction.With, 130)]));
        using var device = Device.WithMock(mock);
        var locks = device.QueryLocks();
        Assert.Equal(40, locks.ScaleOf(x, Direction.Against));
        Assert.Equal(130, locks.ScaleOf(x, Direction.With));
        Assert.Equal(Constants.LockScalePass, locks.ScaleOf(x, Direction.Positive));
        Assert.False(locks.IsLocked(x, Direction.Against));
        Assert.False(locks.Entries[0].IsBlock);
    }

    [Fact]
    public void ScaleAndBearingReachTheDevice()
    {
        var x = LockTarget.X;
        using var mock = new MockBox();
        using (var device = Device.WithMock(mock))
        {
            device.Scale(x, Direction.Against, 40);
            device.ScaleAll(Blanket.Aim, Direction.With, 130);
            device.Lock(x, Direction.Positive);
            device.Unlock(x, Direction.Positive);
            device.SetBearing(35, BearingMode.Vector);
            device.SetBearing(null, BearingMode.PerAxis);
        }
        // LOCK is [class][id u16 LE][direction][scale i16 LE] and OPTION(BEARING) is [4][window u16 LE][mode]
        Assert.Equal(
            [[3, 0, 0, 4, 40, 0], [3, 0, 0, 3, 130, 0], [3, 1, 0, 3, 130, 0], [3, 0, 0, 1, 0, 0], [3, 0, 0, 1, 100, 0]],
            Frames(mock, FrameType.Lock));
        Assert.Equal([[4, 35, 0, 1], [4, 0, 0, 0]], Frames(mock, FrameType.Option));
    }

    [Fact]
    public void TheBoxsLockTableAnswersQueryLocks()
    {
        var (x, y) = (LockTarget.X, LockTarget.Y);
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        device.Scale(x, Direction.Both, 50);
        var locks = device.QueryLocks();
        // both is only the fixed pair, the relative pair keeps passing
        Assert.Equal(2, locks.Entries.Count);
        Assert.Equal(50, locks.ScaleOf(x, Direction.Positive));
        Assert.Equal(50, locks.ScaleOf(x, Direction.Negative));
        Assert.Equal(Constants.LockScalePass, locks.ScaleOf(x, Direction.With));
        Assert.Equal(Constants.LockScalePass, locks.ScaleOf(x, Direction.Against));
        // in vector mode one relative scale, the lower one, governs both axes
        device.Unlock(x, Direction.Both);
        device.Scale(x, Direction.With, 130);
        device.Scale(y, Direction.With, 60);
        device.SetBearing(20, BearingMode.PerAxis);
        locks = device.QueryLocks();
        Assert.Equal((130, 60), ((int)locks.ScaleOf(x, Direction.With), (int)locks.ScaleOf(y, Direction.With)));
        device.SetBearing(20, BearingMode.Vector);
        locks = device.QueryLocks();
        Assert.Equal((60, 60), ((int)locks.ScaleOf(x, Direction.With), (int)locks.ScaleOf(y, Direction.With)));
    }

    [Fact]
    public void AKeyBlanketReportsTheEdgesItBlocks()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        device.LockAll(Blanket.Keys, Direction.Positive);
        Assert.Equal([(true, Direction.Positive, (short)0)], device.QueryLocks().Entries.Select(entry => (entry.IsBlanket, entry.Direction, entry.Scale)));
        device.LockAll(Blanket.Keys, Direction.Negative);
        Assert.Equal([Direction.Positive, Direction.Negative], device.QueryLocks().Entries.Select(entry => entry.Direction));
    }

    [Fact]
    public void AOneBitClassStoresTheBlockOrPassItAmountsTo()
    {
        LockTarget left = Medius.Button.Left;
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        device.Scale(left, Direction.Positive, 50);
        Assert.Equal(Constants.LockScaleBlock, device.QueryLocks().ScaleOf(left, Direction.Positive));
        device.Scale(left, Direction.Positive, 150);
        Assert.Empty(device.QueryLocks().Entries);
    }

    [Fact]
    public void AMediaLockIsSentAndReportedAsBoth()
    {
        var mute = LockTarget.Media((MediaKey)0xE2);
        using var mock = new MockBox();
        IReadOnlyList<LockEntry> entries;
        using (var device = Device.WithMock(mock))
        {
            device.Lock(mute, Direction.Press);
            entries = device.QueryLocks().Entries;
        }
        Assert.Equal([[2, 0xE2, 0x00, 0, 0, 0]], Frames(mock, FrameType.Lock));
        Assert.Equal([Direction.Both], entries.Select(entry => entry.Direction));
    }

    [Fact]
    public void ARelativeDirectionNeedsABearingAndOnlyAnAxisHasOne()
    {
        using var mock = new MockBox();
        using (var device = Device.WithMock(mock))
        {
            foreach (var direction in new[] { Direction.With, Direction.Against })
            {
                Assert.Throws<RelativeDirectionException>(() => device.Lock(Medius.Button.Left, direction));
                Assert.Throws<RelativeDirectionException>(() => device.Scale(Medius.Key.A, direction, 40));
                Assert.Throws<RelativeDirectionException>(() => device.LockAll(Blanket.Keys, direction));
                Assert.Throws<RelativeDirectionException>(() => device.LockAll(Blanket.Media, direction));
                device.Scale(LockTarget.X, direction, 130);
            }
        }
        Assert.Equal(2, Frames(mock, FrameType.Lock).Length);
    }

    [Fact]
    public void ValuesTheEnumsDontNameAreRefusedBeforeTheyCross()
    {
        var x = LockTarget.X;
        using var mock = new MockBox();
        using (var device = Device.WithMock(mock))
        {
            Assert.Throws<ArgumentOutOfRangeException>(() => device.Scale(x, (Direction)40, 50));
            Assert.Throws<ArgumentOutOfRangeException>(() => device.LockAll((Blanket)99, Direction.Both));
            Assert.Throws<ArgumentOutOfRangeException>(() => device.SetBearing(20, (BearingMode)7));
            Assert.Throws<ArgumentOutOfRangeException>(() => device.SetRender((RenderMode)4, false));
            // the window saturates instead of wrapping, like the Rust API, 70000 would be 4464 through a u16
            device.SetBearing(70_000, BearingMode.Vector);
            // and a zero one is 1 ms like there, 0 on the wire would turn it off
            device.SetBearing(0, BearingMode.Vector);
        }
        Assert.Equal([[4, 0xFF, 0xFF, 1], [4, 1, 0, 1]], Frames(mock, FrameType.Option));
        Assert.Empty(Frames(mock, FrameType.Lock));
    }

    [Fact]
    public void TheMocksLocksAreCheckedEntryByEntry()
    {
        var x = LockTarget.X;
        using var mock = new MockBox();
        Assert.Throws<ArgumentOutOfRangeException>(() => mock.SetLocks(new Locks([new LockEntry(x, false, (Direction)40, 0)])));
        var good = new Locks([new LockEntry(x, false, Direction.Positive, 0)]);
        Assert.Throws<ArgumentOutOfRangeException>(() => good.IsLocked(x, (Direction)40));
        Assert.Throws<ArgumentOutOfRangeException>(() => good.ScaleOf(x, (Direction)40));
        mock.SetLocks(good);
        using var device = Device.WithMock(mock);
        Assert.True(device.QueryLocks().IsLocked(x, Direction.Positive));
    }

    [Fact]
    public void ACatchFiltersValuesAreChecked()
    {
        var filter = CatchFilter.Watch(Medius.Button.Left);
        Assert.Throws<ArgumentOutOfRangeException>(() => filter.WithDirection((Direction)40));
        Assert.Throws<ArgumentOutOfRangeException>(() => CatchFilter.WatchAxis((Axis)9));
        Assert.Throws<ArgumentOutOfRangeException>(() => CatchFilter.WatchClass((Class)9));
        Assert.Throws<ArgumentOutOfRangeException>(() => CatchFilter.TrafficOf((TrafficClass)99));
        Assert.Equal(Direction.Positive, filter.WithDirection(Direction.Press).Direction);
        Assert.Equal(16, filter.WithCapture(16).Capture);
    }

    [Fact]
    public void TheMocksBearingAnswersQueryBearing()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        Assert.Equal(new Bearing(Constants.BearingWindowDefaultMs, BearingMode.PerAxis), device.QueryBearing());
        mock.SetBearing(new Bearing(250, BearingMode.Vector));
        Assert.Equal(new Bearing(250, BearingMode.Vector), device.QueryBearing());
        mock.SetBearing(new Bearing(null, BearingMode.PerAxis));
        var off = device.QueryBearing();
        Assert.Null(off.WindowMs);
        Assert.False(off.IsLive);
    }

    [Fact]
    public void RelativeDirectionsSaySo()
    {
        Assert.True(Direction.With.IsRelative);
        Assert.True(Direction.Against.IsRelative);
        Assert.False(Direction.Both.IsRelative);
        Assert.False(Direction.Positive.IsRelative);
        Assert.Equal((3, 4), ((int)Direction.With, (int)Direction.Against));
        Assert.Equal((0, 100, 255, -255), (Constants.LockScaleBlock, Constants.LockScalePass, Constants.LockScaleMax, Constants.LockScaleMin));
    }

    [Fact]
    public void HealthComesBack()
    {
        var health = new Health(true, false, true, false, true, false, true, false);
        using var mock = new MockBox();
        mock.SetHealth(health);
        using var device = Device.WithMock(mock);
        Assert.Equal(health, device.QueryHealth());
    }

    [Fact]
    public void DeviceInfoComesBack()
    {
        var info = new DeviceInfo(0x046D, 0xC08B, 0x0111, 0x0200, true, false, DeviceKind.Mouse, "Logitech G502");
        using var mock = new MockBox();
        mock.SetDeviceInfo(info);
        using var device = Device.WithMock(mock);
        var got = device.DeviceInfo();
        Assert.Equal(info, got);
        Assert.True(got.IsCloned);
    }

    [Fact]
    public void ACloneWithNoHidInterfaceIsCloned()
    {
        var pad = new DeviceInfo(0x045E, 0x028E, 0x0114, 0x0200, true, false, DeviceKind.Unknown, "Controller");
        using var mock = new MockBox();
        mock.SetDeviceInfo(pad);
        using var device = Device.WithMock(mock);
        var got = device.DeviceInfo();
        Assert.Equal(DeviceKind.Unknown, got.Kind);
        Assert.True(got.IsCloned);
        Assert.False(new DeviceInfo(0, 0, 0, 0, false, false, DeviceKind.Unknown, "").IsCloned);
    }

    [Fact]
    public void StatsComeBack()
    {
        var stats = new Stats(123456, 12, 34, 7, 2, 900, 3, 4, 0xDEADBEEF, 0, 0x0A0B0C0D, 0xBEEF);
        using var mock = new MockBox();
        mock.SetStats(stats);
        using var device = Device.WithMock(mock);
        Assert.Equal(stats, device.QueryStats());
    }

    static CatchState QueryCatch(CatchState state)
    {
        using var mock = new MockBox();
        mock.SetCatchState(state);
        using var device = Device.WithMock(mock);
        return device.QueryCatch();
    }

    [Fact]
    public void TheCatchStateComesBack()
    {
        var state = new CatchState(true, 42, new ClockEstimate(-1234, 57, 90, 1500),
        [
            new CatchEntry(CatchFilter.Everything().WithCapture(16), 3),
            new CatchEntry(CatchFilter.Traffic(TrafficClass.VendorBulk, 3).WithDirection(Direction.Positive), 7),
        ]);
        var got = QueryCatch(state);
        Assert.Equal((state.TableFull, state.Dropped, state.Clock), (got.TableFull, got.Dropped, got.Clock));
        Assert.Equal(state.Entries, got.Entries);
        Assert.Equal(45, got.Clock.ErrorBoundUs);
        Assert.Equal(CatchClass.VendorBulk, got.Entries[1].Filter.Class);
        Assert.Equal((ushort)3, got.Entries[1].Filter.Id);
    }

    [Fact]
    public void ANeverMeasuredClockIsntAZeroAge()
    {
        var never = QueryCatch(new CatchState(false, 0, new ClockEstimate(500, AgeMs: null), []));
        var fresh = QueryCatch(new CatchState(false, 0, new ClockEstimate(500, AgeMs: 0), []));
        Assert.Null(never.Clock.AgeMs);
        Assert.Equal(0u, fresh.Clock.AgeMs);
        Assert.NotEqual(never.Clock, fresh.Clock);
        Assert.Null(never.Clock.ToHostDomain(1000));
        Assert.Equal(1500, fresh.Clock.ToHostDomain(1000));
    }

    [Fact]
    public void CatchFilterWildcardsSurviveTheRoundTrip()
    {
        var every = CatchFilter.Everything();
        var blanket = CatchFilter.TrafficOf(TrafficClass.HidIn);
        var exact = CatchFilter.Traffic(TrafficClass.Control, 0);
        Assert.Equal(((CatchClass?)null, (ushort?)null), (every.Class, every.Id));
        Assert.Equal(((CatchClass?)CatchClass.HidIn, (ushort?)null), (blanket.Class, blanket.Id));
        Assert.Equal(((CatchClass?)CatchClass.Control, (ushort?)0), (exact.Class, exact.Id));
        var got = QueryCatch(new CatchState(false, 0, new ClockEstimate(), [new(every), new(blanket), new(exact)]));
        Assert.Equal([every, blanket, exact], got.Entries.Select(entry => entry.Filter));
    }

    [Fact]
    public void ImperfectStatusComesBack()
    {
        var status = new ImperfectStatus(true, true, false);
        using var mock = new MockBox();
        mock.SetImperfectStatus(status);
        using var device = Device.WithMock(mock);
        Assert.Equal(status, device.QueryImperfect());
    }

    [Fact]
    public void MovementRidingComesBack()
    {
        using var mock = new MockBox();
        mock.SetMovementRiding(8);
        using (var device = Device.WithMock(mock))
        {
            Assert.Equal(8u, device.QueryMovementRiding());
        }
        mock.SetMovementRiding(null);
        using (var device = Device.WithMock(mock))
        {
            Assert.Null(device.QueryMovementRiding());
        }
    }

    [Fact]
    public void TheEmitPaceComesBack()
    {
        using var mock = new MockBox();
        mock.SetEmitPace(EmitPace.Fixed(500));
        using (var device = Device.WithMock(mock))
        {
            var status = device.QueryEmitPace();
            Assert.Equal(EmitPace.Fixed(500), status.Mode);
            Assert.Equal(500, status.ResolvedHz);
        }
        mock.SetEmitPace(EmitPace.Learned);
        EmitPaceStatus learned;
        using (var device = Device.WithMock(mock))
        {
            learned = device.QueryEmitPace();
            Assert.Equal(EmitPace.Learned, learned.Mode);
            Assert.Equal(0, learned.ResolvedHz);
        }
        // armed, the rendered stream paces itself every millisecond, and disarming puts it back
        mock.SetRender(RenderMode.Despiked, false, true);
        using (var device = Device.WithMock(mock))
        {
            Assert.Equal(1000, device.QueryEmitPace().ResolvedHz);
        }
        mock.SetRender(RenderMode.Despiked, false, false);
        using (var device = Device.WithMock(mock))
        {
            Assert.Equal(0, device.QueryEmitPace().ResolvedHz);
        }
        Assert.Null(learned.ForceHz);
        Assert.False(learned.ForceActive);
        Assert.Equal(0, learned.AdvertisedHz);
    }

    [Fact]
    public void TheSpreadComesBack()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        Assert.Equal(new SpreadStatus(100, 0), device.QuerySpread());
        mock.SetSpreadLearned(8000);
        Assert.Equal(new SpreadStatus(100, 8000), device.QuerySpread());
        device.SetSpread(50);
        Assert.Equal(new SpreadStatus(50, 4000), device.QuerySpread());
        // above 100 overlaps instead of being clamped, and it's past a byte
        device.SetSpread(1000);
        Assert.Equal(new SpreadStatus(1000, 80000), device.QuerySpread());
        device.SetSpread(0);
        Assert.Equal(new SpreadStatus(0, 0), device.QuerySpread());
    }

    [Fact]
    public void TheRenderModeComesBack()
    {
        using (var mock = new MockBox())
        using (var device = Device.WithMock(mock))
        {
            Assert.Equal(new RenderStatus(RenderMode.Despiked, false, false), device.QueryRender());
            foreach (var mode in Enum.GetValues<RenderMode>())
            {
                foreach (var full in new[] { false, true })
                {
                    device.SetRender(mode, full);
                    var status = device.QueryRender();
                    Assert.Equal((mode, full), (status.Mode, status.Full));
                }
            }
            device.SetRender(RenderMode.Stock, false);
            device.SetEmitPace(EmitPace.Learned);
            Assert.Equal(0, device.QueryEmitPace().ResolvedHz);
            device.SetEmitPace(EmitPace.Fixed(250));
            Assert.Equal(250, device.QueryEmitPace().ResolvedHz);
        }
        using (var mock = new MockBox())
        using (var device = Device.WithMock(mock))
        {
            mock.SetRender(RenderMode.Stock, true, true);
            Assert.Equal(new RenderStatus(RenderMode.Stock, true, true), device.QueryRender());
            device.SetEmitPace(EmitPace.Learned);
            Assert.Equal(1000, device.QueryEmitPace().ResolvedHz);
        }
    }
}
