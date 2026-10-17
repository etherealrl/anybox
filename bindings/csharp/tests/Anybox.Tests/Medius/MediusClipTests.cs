using Anybox.Medius;
using static MediusTests.MediusDeviceTests;
using Medius = Anybox.Medius;

namespace MediusTests;

// the medius Python package's clip tests, over the medius mock
public sealed class MediusClipTests
{
    internal static ImperfectStatus Allowed => new(true, false, false);

    static byte[] Hex(string hex) => Convert.FromHexString(hex.Replace(" ", ""));

    // the protocol's own example: HID in interface 2, in, start, consume and once per run, selector 1, match 07 20
    // under mask FF 20
    static ClipPacketTrigger Held() => new(TrafficClass.HidIn, 2, Direction.In, ClipAction.Start, Hex("07 20"), Hex("FF 20"), true, true, 1);

    // eight packet triggers that differ in every field and each of them one a packet can match
    static ClipPacketTrigger[] PacketRows()
    {
        byte[] masks = [0xFF, 0xF0, 0x0F, 0x20, 0x81, 0x7E, 0xC3, 0x01];
        (TrafficClass Class, ushort Id, Direction Direction, ClipAction Action, int Length, bool Consume, byte? Selector, ushort Hits)[] rows =
        [
            (TrafficClass.HidIn, 0x0102, Direction.In, ClipAction.Start, 1, true, null, 0),
            (TrafficClass.HidOut, 0x0001, Direction.Out, ClipAction.Stop, 2, false, 1, 1),
            (TrafficClass.VendorInterrupt, 0x0083, Direction.In, ClipAction.Pause, 3, true, 2, 0xFFFF),
            (TrafficClass.VendorBulk, ClipPacketTrigger.AnyId, Direction.Both, ClipAction.Resume, 4, false, null, 0x1234),
            (TrafficClass.Control, 0, Direction.Both, ClipAction.Restart, 5, false, null, 0x00FF),
            (TrafficClass.Emit, 0x0081, Direction.In, ClipAction.Toggle, 0, false, null, 0xFF00),
            (TrafficClass.VendorBulk, 0x0003, Direction.Out, ClipAction.Start, 7, true, 3, 65534),
            (TrafficClass.Emit, 0x0082, Direction.In, ClipAction.Stop, Constants.MaxPktMatch, false, null, 9),
        ];
        return
        [
            .. rows.Select((row, i) => new ClipPacketTrigger(
                row.Class, row.Id, row.Direction, row.Action,
                Enumerable.Range(0, row.Length).Select(j => (byte)((0x35 + 0x1B * i + 0x47 * j) & masks[(i + j) % 8])).ToArray(),
                Enumerable.Range(0, row.Length).Select(j => masks[(i + j) % 8]).ToArray(),
                row.Consume, row.Selector is not null, row.Selector ?? 0, row.Hits)),
        ];
    }

    [Fact]
    public void ClipControlFramesAreThePrototcolsBytes()
    {
        using var mock = new MockBox();
        using (var device = Device.WithMock(mock))
        using (var clip = device.Clip())
        {
            clip.SetRetain(true);
            clip.SetAutolock(Blanket.Aim, Blanket.Buttons);
            clip.SetLoop(true);
            clip.Start();
            clip.Pause();
            clip.Resume();
            clip.Restart();
            clip.Toggle();
            clip.Stop();
            clip.Clear();
            clip.FinalizeClip();
            clip.Bind(new ClipTrigger((Medius.Key)0x3A, Edge.Press, ClipAction.Start));
            clip.Bind(new ClipTrigger(Medius.Button.Right, Edge.Release, ClipAction.Toggle, Consume: true));
            clip.Unbind((Medius.Key)0x3A, Edge.Press);
            clip.ClearTriggers();
        }
        Assert.Equal([[2, 1], [0, 0x05], [1, 1]], Frames(mock, FrameType.ClipSet));
        Assert.Equal(new byte[] { 0, 2, 3, 4, 5, 1, 6, 7 }.Select(value => new[] { value }), Frames(mock, FrameType.ClipCtrl));
        Assert.Equal(
            [[1, 0x3A, 0x00, 1, 0, 1], [0, 0x01, 0x00, 2, 5, 3], [1, 0x3A, 0x00, 1, 0, 0], [0xFF, 0xFF, 0xFF, 0, 0, 0]],
            Frames(mock, FrameType.ClipTrigger));
    }

    [Fact]
    public void AnAppendEncodesAndChunks()
    {
        using var mock = new MockBox();
        using (var device = Device.WithMock(mock))
        using (var builder = new ClipBuilder())
        using (var clip = device.Clip())
        {
            for (var round = 0; round < 150; round++)
            {
                builder.Move(3, -2);
            }
            builder.Press(Medius.Button.Left).Gap(4).Release(Medius.Button.Left);
            clip.Append(builder);
        }
        var appends = Frames(mock, FrameType.ClipAppend);
        Assert.True(appends.Length >= 2, "a clip past 512 bytes has to be split");
        var joined = appends.SelectMany(part => part).ToArray();
        Assert.Equal([0x01, 0x03, 0x00, 0xFE, 0xFF], joined[..5]);
        Assert.Equal([0x04, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x04, 0x00, 0x04, 0x01, 0x00, 0x00, 0x00, 0x00], joined[^15..]);
        Assert.All(appends, part => Assert.True(part.Length <= 512));
    }

    [Fact]
    public void AFramesEdgesAreEncoded()
    {
        using var mock = new MockBox();
        using (var device = Device.WithMock(mock))
        using (var builder = new ClipBuilder())
        {
            builder.Frame(1, 2, -1, edges: [(Medius.Button.Left, InjectAction.Press), ((Medius.Key)0x04, InjectAction.Press)]);
            device.Clip().Append(builder);
        }
        Assert.Equal([0x07, 0x01, 0x00, 0x02, 0x00, 0xFF, 0xFF, 0x02, 0x00, 0x00, 0x00, 0x01, 0x01, 0x04, 0x00, 0x01], Frames(mock, FrameType.ClipAppend)[0]);
    }

    [Fact]
    public void AFrameCarriesPanRawReportsAndTransfers()
    {
        var setReport = new Setup(0x21, 0x09, 0x0300, 0, 2);
        var getReport = new Setup(0xA1, 0x01, 0x0300, 0, 8);
        using var mock = new MockBox();
        using (var device = Device.WithMock(mock))
        using (var builder = new ClipBuilder())
        {
            builder.Frame(4, -2, 1, -3,
                edges: [(Medius.Button.Left, InjectAction.Press)],
                raw: [new ClipRaw(2, Direction.Out, new byte[] { 0x10, 0xFF, 0x05 }), new ClipRaw(1, Direction.In, Array.Empty<byte>())],
                transfers: [new ClipTransfer(3, setReport, new byte[] { 0x04, 0x01 }), new ClipTransfer(4, getReport)]);
            device.Clip().Append(builder);
        }
        Assert.Equal(
            [
                [
                    0x3F, 0x04, 0x00, 0xFE, 0xFF, 0x01, 0x00, 0xFD, 0xFF, 0x01, 0x00, 0x00, 0x00, 0x01,
                    0x02, 0x02, 0x02, 0x03, 0x00, 0x10, 0xFF, 0x05, 0x01, 0x01, 0x00, 0x00,
                    0x02, 0x03, 0x21, 0x09, 0x00, 0x03, 0x00, 0x00, 0x02, 0x00, 0x04, 0x01,
                    0x04, 0xA1, 0x01, 0x00, 0x03, 0x00, 0x00, 0x08, 0x00,
                ],
            ],
            Frames(mock, FrameType.ClipAppend));
    }

    [Fact]
    public void OneFieldPanRawAndTransferFrames()
    {
        using var mock = new MockBox();
        using (var device = Device.WithMock(mock))
        using (var builder = new ClipBuilder())
        {
            builder.Pan(7).Raw(2, Direction.Out, [0xAA]).Transfer(5, new Setup(0x21, 0x09, 0x0300, 0, 1), [0x05]);
            builder.Transfer(6, new Setup(0xA1, 0x01, 0x0300, 0, 8));
            Assert.Equal((nuint)(3 + 7 + 12 + 11), builder.ByteLen);
            device.Clip().Append(builder);
        }
        Assert.Equal(
            Hex("08 07 00" + "10 01 02 02 01 00 AA" + "20 01 05 21 09 00 03 00 00 01 00 05" + "20 01 06 A1 01 00 03 00 00 08 00"),
            Frames(mock, FrameType.ClipAppend).SelectMany(part => part).ToArray());
    }

    [Fact]
    public void ByteLenIsWhatAnAppendPutsInTheRing()
    {
        using var mock = new MockBox();
        nuint expected;
        using (var device = Device.WithMock(mock))
        using (var builder = new ClipBuilder())
        {
            Assert.Equal(0u, builder.ByteLen);
            builder.Frame();
            Assert.Equal(5u, builder.ByteLen);
            builder.Gap(4).Wheel(1).Press(Medius.Button.Left);
            Assert.Equal((nuint)(5 + 3 + 3 + 6), builder.ByteLen);
            for (var round = 0; round < 150; round++)
            {
                builder.Move(3, -2);
            }
            builder.Frame(1, raw: [new ClipRaw(2, Direction.Out, new byte[] { 0x10, 0xFF })], transfers: [new ClipTransfer(3, new Setup(0xA1, 0x01, 0x0300, 0, 8))]);
            expected = builder.ByteLen;
            device.Clip().Append(builder);
            builder.Clear();
            Assert.Equal(0u, builder.ByteLen);
        }
        var appends = Frames(mock, FrameType.ClipAppend);
        Assert.True(appends.Length >= 2);
        Assert.Equal((int)expected, appends.Sum(part => part.Length));
    }

    [Fact]
    public void ARefusedFrameThrowsItsOwnExceptionAndSendsNothing()
    {
        var setReport = new Setup(0x21, 0x09, 0x0300, 0, 2);
        (Type Exception, Action<ClipBuilder> Frame)[] refusals =
        [
            (typeof(ClipFrameCountException), builder => builder.Frame(edges: Enumerable.Repeat(((Usage)Medius.Button.Left, InjectAction.Press), Constants.ClipEdgesMax + 1))),
            (typeof(ClipFrameCountException), builder => builder.Frame(raw: Enumerable.Repeat(new ClipRaw(1, Direction.Out, new byte[1]), Constants.ClipRawMax + 1))),
            (typeof(ClipFrameTooLongException), builder => builder.Frame(raw: [new ClipRaw(1, Direction.Out, new byte[Constants.ClipEntryMax])])),
            (typeof(ClipTransferDataException), builder => builder.Frame(transfers: [new ClipTransfer(0, setReport, new byte[] { 4 })])),
            (typeof(ClipTransferDataException), builder => builder.Frame(transfers: [new ClipTransfer(0, new Setup(0xA1, 0x01, 0x0300, 0, 8), new byte[] { 4 })])),
            (typeof(RawDirectionException), builder => builder.Frame(raw: [new ClipRaw(1, Direction.Both, new byte[1])])),
            (typeof(RelativeDirectionException), builder => builder.Frame(raw: [new ClipRaw(1, Direction.With, new byte[1])])),
        ];
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var clip = device.Clip();
        foreach (var (exception, frame) in refusals)
        {
            using var builder = new ClipBuilder();
            builder.Move(1, 1);
            frame(builder);
            Assert.IsType(exception, Assert.ThrowsAny<MediusException>(() => clip.Append(builder)));
        }
        Assert.Empty(Frames(mock, FrameType.ClipAppend));
        using (var builder = new ClipBuilder())
        {
            builder.Frame(
                edges: Enumerable.Repeat(((Usage)Medius.Button.Left, InjectAction.Press), Constants.ClipEdgesMax),
                raw: Enumerable.Repeat(new ClipRaw(1, Direction.Out, new byte[1]), Constants.ClipRawMax));
            clip.Append(builder);
        }
        Assert.Single(Frames(mock, FrameType.ClipAppend));
    }

    [Fact]
    public void ABadFrameFieldLeavesTheBuilderAsItWas()
    {
        using var mock = new MockBox();
        using (var device = Device.WithMock(mock))
        using (var builder = new ClipBuilder())
        {
            builder.Move(1, 1);
            Assert.Throws<ArgumentOutOfRangeException>(() => builder.Frame(2, raw: [new ClipRaw(1, (Direction)200, new byte[1])]));
            Assert.Throws<ArgumentOutOfRangeException>(() => builder.Raw(1, (Direction)200, [0]));
            device.Clip().Append(builder);
        }
        Assert.Equal([[0x01, 0x01, 0x00, 0x01, 0x00]], Frames(mock, FrameType.ClipAppend));
    }

    [Fact]
    public void AGapOfZeroDoesNothing()
    {
        using var mock = new MockBox();
        using (var device = Device.WithMock(mock))
        using (var builder = new ClipBuilder())
        {
            builder.Gap(0);
            device.Clip().Append(builder);
        }
        Assert.Empty(Frames(mock, FrameType.ClipAppend));
    }

    [Fact]
    public void TheClipStatusAndConfigComeBack()
    {
        var status = new ClipStatus(ClipState.Playing, 512, 40, 8, 99, 2, 0, 1, 7, 3, 5, [Medius.Button.Side1, Medius.Key.A]);
        var settings = new ClipSettings
        {
            Autolock = [Blanket.Aim, Blanket.Keys],
            Loop = true,
            Retain = true,
            Ride = true,
            Triggers = [new ClipTrigger(Medius.Button.Right, Edge.Both, ClipAction.Toggle), new ClipTrigger((Medius.Key)0x3A, Edge.Release, ClipAction.Stop, true)],
            PacketTriggers = PacketRows()[..3],
        };
        using var mock = new MockBox();
        mock.SetImperfectStatus(Allowed);
        mock.SetClipStatus(status);
        mock.SetClipSettings(settings);
        using var device = Device.WithMock(mock);
        using var clip = device.Clip();
        var got = clip.QueryStatus();
        var config = clip.QueryConfig();
        Assert.Equal(status with { Held = [] }, got with { Held = [] });
        Assert.Equal(status.Held, got.Held);
        Assert.True(got.IsHeld(Medius.Button.Side1));
        Assert.True(got.IsHeld(Medius.Key.A));
        Assert.False(got.IsHeld(Medius.Button.Left));
        Assert.Equal(settings.Autolock, config.Autolock);
        Assert.Equal((settings.Loop, settings.Retain, settings.Finalized, settings.Ride), (config.Loop, config.Retain, config.Finalized, config.Ride));
        Assert.Equal(settings.Triggers, config.Triggers);
        Assert.Equal(settings.PacketTriggers, config.PacketTriggers);
    }

    // the settings carry the box's CLIP_LOCK bits (the crate's protocol/opcode.rs), which the C# side maps itself,
    // so each one is pinned against the bit the library puts on the wire
    [Theory]
    [InlineData(new[] { Blanket.Aim }, 0x01)]
    [InlineData(new[] { Blanket.Wheel }, 0x02)]
    [InlineData(new[] { Blanket.Buttons }, 0x04)]
    [InlineData(new[] { Blanket.Keys }, 0x08)]
    [InlineData(new[] { Blanket.Media }, 0x10)]
    [InlineData(new[] { Blanket.Aim, Blanket.Wheel, Blanket.Buttons, Blanket.Keys, Blanket.Media }, 0x1F)]
    public void TheAutolockBitsAreTheBoxs(Blanket[] blankets, byte bits)
    {
        using var mock = new MockBox();
        using (var device = Device.WithMock(mock))
        using (var clip = device.Clip())
        {
            clip.SetAutolock(blankets);
        }
        Assert.Equal([[0, bits]], Frames(mock, FrameType.ClipSet));
        Assert.Equal(bits, new ClipSettings { Autolock = blankets }.ToNative().autolock_bits);
        Assert.Equal(blankets, ClipSettings.From(new MediusClipSettings { autolock_bits = bits }).Autolock);
    }

    [Fact]
    public void APacketTriggerGoesOutAsTheBytesTheProtocolNames()
    {
        var held = Held();
        // a removal sends the key alone, whatever else is in it
        var stale = held with { Action = (ClipAction)5, Consume = true, OncePerRun = true, SelectorLen = 9, Hits = 77 };
        using var mock = new MockBox();
        using (var device = Device.WithMock(mock))
        using (var clip = device.Clip())
        {
            clip.BindPacket(held);
            clip.UnbindPacket(stale);
            clip.BindPacket(new ClipPacketTrigger(TrafficClass.VendorBulk, ClipPacketTrigger.AnyId, Direction.Both, ClipAction.Pause, Consume: true));
            clip.BindPacket(new ClipPacketTrigger(TrafficClass.HidOut, 1, Direction.Out, ClipAction.Resume, Hex("01 02 03"), Hex("FF FF 0F"), OncePerRun: true, SelectorLen: 2));
            clip.BindPacket(new ClipPacketTrigger(TrafficClass.Control, 0, Direction.Both, ClipAction.Toggle,
                Enumerable.Range(0, 16).Select(value => (byte)value).ToArray(), Enumerable.Repeat((byte)0xFF, 16).ToArray(), Hits: 500));
            clip.BindPacket(new ClipPacketTrigger(TrafficClass.Emit, 0x0181, Direction.In, ClipAction.Stop, new byte[] { 0x80 }, new byte[] { 0xF0 }));
            clip.ClearTriggers();
        }
        // [class][id u16][dir][action][flags: 1 present, 2 consume, 4 once per run][slen][mlen][match][mask]
        Assert.Equal(
            [
                Hex("04 02 00 01 00 07 01 02 07 20 FF 20"),
                Hex("04 02 00 01 00 00 00 02 07 20 FF 20"),
                Hex("07 FF FF 00 02 03 00 00"),
                Hex("05 01 00 02 03 05 02 03 01 02 03 FF FF 0F"),
                [.. Hex("08 00 00 00 05 01 00 10"), .. Enumerable.Range(0, 16).Select(value => (byte)value), .. Enumerable.Repeat((byte)0xFF, 16)],
                Hex("09 81 01 01 01 01 00 01 80 F0"),
                Hex("FF FF FF 00 00 00"),
            ],
            Frames(mock, FrameType.ClipTrigger));
    }

    [Fact]
    public void PacketTriggersReadBackFieldForField()
    {
        var rows = PacketRows();
        Assert.Equal(0xFFFF, rows[2].Hits);
        ClipTrigger[] inputs = [new(Medius.Button.Right, Edge.Press, ClipAction.Start)];
        foreach (var count in new[] { 0, 1, 8 })
        {
            using var mock = new MockBox();
            mock.SetImperfectStatus(Allowed);
            mock.SetClipSettings(new ClipSettings { Triggers = inputs, PacketTriggers = rows[..count] });
            using var device = Device.WithMock(mock);
            var config = device.Clip().QueryConfig();
            Assert.Equal(rows[..count], config.PacketTriggers);
            Assert.Equal(inputs, config.Triggers);
        }
        var spec = new ClipPacketTrigger(TrafficClass.HidIn, 0x0102, Direction.In, ClipAction.Toggle, Hex("07 20"), Hex("FF 20"), true, true, 1, 0xFFFF);
        using (var mock = new MockBox())
        {
            mock.SetImperfectStatus(Allowed);
            mock.SetClipSettings(new ClipSettings { PacketTriggers = [spec] });
            using var device = Device.WithMock(mock);
            using var clip = device.Clip();
            var got = Assert.Single(clip.QueryConfig().PacketTriggers);
            Assert.Equal(spec, got);
            // a read trigger replays as a bind, its hits left behind
            clip.BindPacket(got);
            Assert.Equal([Hex("04 02 01 01 05 07 01 02 07 20 FF 20")], Frames(mock, FrameType.ClipTrigger));
        }
    }

    [Fact]
    public unsafe void EachLengthOfAReadBackTriggerIsItsOwn()
    {
        var raw = new MediusClipPacketTrigger { class_ = (byte)TrafficClass.HidIn, id = 2, direction = (byte)Direction.In, action = (byte)ClipAction.Start };
        for (var index = 0; index < Constants.MaxPktMatch; index++)
        {
            raw.match_bytes[index] = (byte)(index + 1);
            raw.mask[index] = (byte)(0xF0 + index);
        }
        (raw.match_len, raw.mask_len) = (3, 1);
        var got = ClipPacketTrigger.From(raw);
        Assert.Equal(Hex("01 02 03"), got.MatchBytes.ToArray());
        Assert.Equal(Hex("F0"), got.Mask.ToArray());
        // a length past the array reads the array
        (raw.match_len, raw.mask_len) = (1, 40);
        got = ClipPacketTrigger.From(raw);
        Assert.Equal(Hex("01"), got.MatchBytes.ToArray());
        Assert.Equal(Enumerable.Range(0xF0, 16).Select(value => (byte)value).ToArray(), got.Mask.ToArray());
    }

    [Fact]
    public void AConsumingTriggerScriptedWithTheOptInOffIsLeftOut()
    {
        var consuming = Held() with { Hits = 7 };
        var watching = new ClipPacketTrigger(TrafficClass.HidIn, 3, Direction.In, ClipAction.Start, Hex("07 20"), Hex("FF 20"), false, true, 1, 5);
        ClipTrigger[] inputs = [new(Medius.Button.Right, Edge.Press, ClipAction.Start, true)];
        var settings = new ClipSettings { Triggers = inputs, PacketTriggers = [consuming, watching] };
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var clip = device.Clip();
        mock.SetClipSettings(settings);
        var config = clip.QueryConfig();
        Assert.Equal(inputs, config.Triggers);
        Assert.Equal([watching], config.PacketTriggers);
        mock.SetImperfectStatus(Allowed);
        mock.SetClipSettings(settings);
        Assert.Equal([consuming, watching], clip.QueryConfig().PacketTriggers);
        mock.SetImperfectStatus(new ImperfectStatus(false, false, false));
        Assert.Equal([watching], clip.QueryConfig().PacketTriggers);
    }

    [Fact]
    public void ABoundPacketTriggerReadsBackAsItWasBound()
    {
        var rows = PacketRows();
        using var mock = new MockBox();
        mock.SetImperfectStatus(Allowed);
        using var device = Device.WithMock(mock);
        using var clip = device.Clip();
        foreach (var row in rows)
        {
            clip.BindPacket(row);
        }
        var config = clip.QueryConfig();
        Assert.Equal([(true, false), (false, true)], config.PacketTriggers.Take(2).Select(row => (row.Consume, row.OncePerRun)));
        // a bind starts its count at zero
        Assert.All(config.PacketTriggers, row => Assert.Equal(0, row.Hits));
        var bound = rows.Select(row => row with { Hits = 0 }).ToArray();
        Assert.Equal(bound, config.PacketTriggers);
        clip.UnbindPacket(rows[2]);
        clip.UnbindPacket(rows[7]);
        Assert.Equal([.. bound[..2], .. bound[3..7]], clip.QueryConfig().PacketTriggers);
    }

    [Fact]
    public void SettingsPastTheArraysAreRefused()
    {
        var inputs = Enumerable.Range(0, 9).Select(index => new ClipTrigger((Medius.Key)(0x04 + index), Edge.Press, ClipAction.Start)).ToArray();
        var packets = Enumerable.Range(0, Constants.ClipPktTrigMax + 1)
            .Select(index => new ClipPacketTrigger(TrafficClass.HidIn, (ushort)index, Direction.In, ClipAction.Start, new[] { (byte)index }, new byte[] { 0xFF }, Hits: (ushort)index))
            .ToArray();
        using var mock = new MockBox();
        Assert.Contains("over the 8", Assert.Throws<ArgumentException>(() => mock.SetClipSettings(new ClipSettings { Triggers = inputs })).Message);
        Assert.Throws<ArgumentException>(() => mock.SetClipSettings(new ClipSettings { PacketTriggers = packets }));
        mock.SetClipSettings(new ClipSettings { Triggers = inputs[..8], PacketTriggers = packets[..Constants.ClipPktTrigMax] });
        using var device = Device.WithMock(mock);
        var config = device.Clip().QueryConfig();
        Assert.Equal(inputs[..8], config.Triggers);
        Assert.Equal(packets[..Constants.ClipPktTrigMax], config.PacketTriggers);
    }

    [Fact]
    public void ClearTriggersRemovesBothKinds()
    {
        var rows = PacketRows();
        using var mock = new MockBox();
        mock.SetClipSettings(new ClipSettings { Triggers = [new ClipTrigger(Medius.Button.Right, Edge.Press, ClipAction.Start)], PacketTriggers = [rows[1]] });
        using var device = Device.WithMock(mock);
        using var clip = device.Clip();
        clip.BindPacket(rows[5]);
        var held = clip.QueryConfig();
        Assert.Equal((1, 2), (held.Triggers.Count, held.PacketTriggers.Count));
        clip.ClearTriggers();
        var cleared = clip.QueryConfig();
        Assert.Empty(cleared.Triggers);
        Assert.Empty(cleared.PacketTriggers);
    }

    static ClipPacketTrigger Base() => new(TrafficClass.HidIn, 2, Direction.In, ClipAction.Start, Hex("07 20"), Hex("FF 20"));

    static void RefusedFor(Action call, string reason)
    {
        var error = Assert.Throws<ClipPacketTriggerException>(call);
        Assert.Equal(Status.ErrClipPacketTrigger, error.MediusStatus);
        Assert.StartsWith("a clip packet trigger ", error.Message);
        Assert.Contains(reason, error.Message);
    }

    [Fact]
    public void APacketTriggerTheBoxWouldRefuseHasItsOwnException()
    {
        (ClipPacketTrigger Trigger, string Reason, bool KeyFault)[] refused =
        [
            (Base() with { TrafficClass = TrafficClass.Bus }, "names a surface packets cross", true),
            (Base() with { TrafficClass = TrafficClass.ClipTransfer }, "names a surface packets cross", true),
            (Base() with { Mask = Hex("FF") }, "a match and a mask of one length", true),
            (Base() with { MatchBytes = Hex("07") }, "a match and a mask of one length", true),
            (Base() with { MatchBytes = new byte[17], Mask = new byte[17] }, "at most 16 match bytes", true),
            (Base() with { MatchBytes = new byte[0xFFFF], Mask = new byte[0xFFFF] }, "at most 16 match bytes", true),
            (Base() with { TrafficClass = TrafficClass.Control, Consume = true }, "takes consume on a report or vendor class", false),
            (Base() with { SelectorLen = 1 }, "a selector length only with once_per_run", false),
            (Base() with { TrafficClass = TrafficClass.Control, OncePerRun = true }, "needs one stream", false),
            (Base() with { Id = ClipPacketTrigger.AnyId, OncePerRun = true }, "needs one stream", false),
            (Base() with { Direction = Direction.Both, OncePerRun = true }, "needs one stream", false),
            (Base() with { OncePerRun = true, SelectorLen = 2 }, "needs match bytes past its selector", false),
            (Base() with { MatchBytes = default, Mask = default, OncePerRun = true }, "needs match bytes past its selector", false),
        ];
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var clip = device.Clip();
        foreach (var (trigger, reason, keyFault) in refused)
        {
            RefusedFor(() => clip.BindPacket(trigger), reason);
            // a removal reads the key alone, so it refuses a bad key and takes anything else
            if (keyFault)
            {
                RefusedFor(() => clip.UnbindPacket(trigger), reason);
            }
            else
            {
                clip.UnbindPacket(trigger);
            }
        }
        foreach (var relative in new[] { Direction.With, Direction.Against })
        {
            Assert.Throws<RelativeDirectionException>(() => clip.BindPacket(Base() with { Direction = relative }));
            Assert.Throws<RelativeDirectionException>(() => clip.UnbindPacket(Base() with { Direction = relative }));
        }
        var sent = Frames(mock, FrameType.ClipTrigger);
        Assert.Equal(refused.Count(row => !row.KeyFault), sent.Length);
        Assert.All(sent, frame => Assert.Equal(new byte[3], frame[4..7]));
        Assert.Empty(clip.QueryConfig().PacketTriggers);
        var full = Base() with { MatchBytes = Enumerable.Repeat((byte)0x11, Constants.MaxPktMatch).ToArray(), Mask = Enumerable.Repeat((byte)0xFF, Constants.MaxPktMatch).ToArray() };
        clip.BindPacket(full);
        Assert.Equal([full], clip.QueryConfig().PacketTriggers);
    }

    [Fact]
    public void ATriggerOnADirectionItsClassNeverCarriesIsRefused()
    {
        const string Reason = "names a direction its class never carries: HidIn and Emit flow IN, HidOut flows OUT";
        static ClipPacketTrigger On(TrafficClass trafficClass, Direction direction) => new(trafficClass, 1, direction, ClipAction.Start, Hex("07"), Hex("FF"));
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var clip = device.Clip();
        foreach (var (trafficClass, direction) in new[] { (TrafficClass.HidIn, Direction.Out), (TrafficClass.Emit, Direction.Out), (TrafficClass.HidOut, Direction.In) })
        {
            RefusedFor(() => clip.BindPacket(On(trafficClass, direction)), Reason);
            RefusedFor(() => clip.UnbindPacket(On(trafficClass, direction)), Reason);
        }
        Assert.Empty(Frames(mock, FrameType.ClipTrigger));
        ClipPacketTrigger[] carried =
        [
            On(TrafficClass.HidIn, Direction.In), On(TrafficClass.HidIn, Direction.Both), On(TrafficClass.HidOut, Direction.Out),
            On(TrafficClass.HidOut, Direction.Both), On(TrafficClass.VendorInterrupt, Direction.In), On(TrafficClass.VendorInterrupt, Direction.Out),
            On(TrafficClass.VendorBulk, Direction.Both), On(TrafficClass.Emit, Direction.In),
        ];
        foreach (var trigger in carried)
        {
            clip.BindPacket(trigger);
        }
        Assert.Equal(carried, clip.QueryConfig().PacketTriggers);
    }

    [Fact]
    public void ATriggerWithAMatchBitOutsideItsMaskIsRefused()
    {
        const string Reason = "has a match bit outside its mask, which no packet can equal";
        static ClipPacketTrigger HidIn(byte[] match, byte[] mask) => new(TrafficClass.HidIn, 2, Direction.In, ClipAction.Start, match, mask);
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var clip = device.Clip();
        foreach (var trigger in new[]
        {
            HidIn(Hex("07 21"), Hex("FF 20")),
            HidIn(Hex("01"), Hex("00")),
            HidIn([.. new byte[Constants.MaxPktMatch - 1], 0x80], [.. Enumerable.Repeat((byte)0xFF, Constants.MaxPktMatch - 1), 0x7F]),
        })
        {
            RefusedFor(() => clip.BindPacket(trigger), Reason);
            RefusedFor(() => clip.UnbindPacket(trigger), Reason);
        }
        Assert.Empty(Frames(mock, FrameType.ClipTrigger));
        var held = HidIn(Hex("07 20"), Hex("FF 20"));
        clip.BindPacket(held);
        Assert.Equal([Hex("04 02 00 01 00 01 00 02 07 20 FF 20")], Frames(mock, FrameType.ClipTrigger));
        Assert.Equal([held], clip.QueryConfig().PacketTriggers);
    }

    [Fact]
    public void AOncePerRunTriggerWithNoMaskedBitPastItsSelectorIsRefused()
    {
        const string Reason = "needs a masked bit past its selector";
        static ClipPacketTrigger Run(byte selector, byte[] match, byte[] mask, bool oncePerRun = true) =>
            new(TrafficClass.HidIn, 2, Direction.In, ClipAction.Start, match, mask, OncePerRun: oncePerRun, SelectorLen: oncePerRun ? selector : (byte)0);
        (byte, byte[], byte[])[] refused = [(1, Hex("07 00"), Hex("FF 00")), (0, Hex("00 00"), Hex("00 00")), (2, Hex("07 01 00 00"), Hex("FF FF 00 00"))];
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var clip = device.Clip();
        foreach (var (selector, match, mask) in refused)
        {
            RefusedFor(() => clip.BindPacket(Run(selector, match, mask)), Reason);
        }
        Assert.Empty(Frames(mock, FrameType.ClipTrigger));
        foreach (var (selector, match, mask) in refused)
        {
            clip.UnbindPacket(Run(selector, match, mask));
            clip.BindPacket(Run(selector, match, mask, false));
        }
        var oneBit = Run(1, Hex("07 00 00"), Hex("FF 00 01"));
        clip.BindPacket(oneBit);
        Assert.Equal([.. refused.Select(row => Run(row.Item1, row.Item2, row.Item3, false)), oneBit], clip.QueryConfig().PacketTriggers);
    }

    [Fact]
    public void APacketThatCantExistFiresNothingInTheMock()
    {
        TrafficClass[] surfaces = [TrafficClass.HidIn, TrafficClass.HidOut, TrafficClass.VendorInterrupt, TrafficClass.VendorBulk, TrafficClass.Control, TrafficClass.Emit];
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var clip = device.Clip();
        foreach (var surface in surfaces)
        {
            clip.BindPacket(new ClipPacketTrigger(surface, ClipPacketTrigger.AnyId, Direction.Both, ClipAction.Toggle));
        }
        var noPacket = new[] { (TrafficClass.HidIn, Direction.Out), (TrafficClass.Emit, Direction.Out), (TrafficClass.HidOut, Direction.In), (TrafficClass.Bus, Direction.In), (TrafficClass.ClipTransfer, Direction.In) }
            .Concat(surfaces.SelectMany(surface => new[] { Direction.Both, Direction.With, Direction.Against }.Select(direction => (surface, direction))));
        foreach (var (trafficClass, direction) in noPacket)
        {
            Assert.Equal(((ClipAction?)null, false), mock.ClipPacket(trafficClass, 1, direction, [0x07]));
        }
        Assert.All(clip.QueryConfig().PacketTriggers, row => Assert.Equal(0, row.Hits));
        foreach (var (trafficClass, direction) in new[]
        {
            (TrafficClass.HidIn, Direction.In), (TrafficClass.HidOut, Direction.Out), (TrafficClass.VendorInterrupt, Direction.In),
            (TrafficClass.VendorInterrupt, Direction.Out), (TrafficClass.VendorBulk, Direction.In), (TrafficClass.VendorBulk, Direction.Out),
            (TrafficClass.Control, Direction.In), (TrafficClass.Control, Direction.Out), (TrafficClass.Emit, Direction.In),
        })
        {
            Assert.Equal(((ClipAction?)ClipAction.Toggle, false), mock.ClipPacket(trafficClass, 1, direction, [0x07]));
        }
        Assert.Equal([1, 1, 2, 2, 2, 1], clip.QueryConfig().PacketTriggers.Select(row => (int)row.Hits));
    }

    [Fact]
    public void AConsumingTriggerNeedsTheOptInAndGoesWhenItsTurnedOff()
    {
        var consuming = Held();
        var watching = new ClipPacketTrigger(TrafficClass.HidIn, 3, Direction.In, ClipAction.Start, Hex("07 20"), Hex("FF 20"));
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var clip = device.Clip();
        clip.BindPacket(consuming);
        clip.BindPacket(watching);
        Assert.Equal([watching], clip.QueryConfig().PacketTriggers);
        device.AllowImperfectClones(true);
        clip.BindPacket(consuming);
        Assert.Equal([watching, consuming], clip.QueryConfig().PacketTriggers);
        device.AllowImperfectClones(false);
        Assert.Equal([watching], clip.QueryConfig().PacketTriggers);
    }

    [Fact]
    public void TheMockRunsAPacketThroughItsTriggers()
    {
        var held = Held();
        var letGo = new ClipPacketTrigger(TrafficClass.HidIn, 2, Direction.In, ClipAction.Stop, Hex("07 00"), Hex("FF 20"), OncePerRun: true, SelectorLen: 1);
        var wide = new ClipPacketTrigger(TrafficClass.HidIn, ClipPacketTrigger.AnyId, Direction.Both, ClipAction.Toggle);
        using var mock = new MockBox();
        mock.SetImperfectStatus(Allowed);
        using var device = Device.WithMock(mock);
        using var clip = device.Clip();
        foreach (var trigger in new[] { wide, held, letGo })
        {
            clip.BindPacket(trigger);
        }
        (ClipAction? Action, bool Consumed) Run(params byte[] head) => mock.ClipPacket(TrafficClass.HidIn, 2, Direction.In, head);
        Assert.Equal((ClipAction.Start, true), Run(0x07, 0x20, 0x55));
        Assert.Equal(((ClipAction?)null, true), Run(0x07, 0x20, 0x56));
        Assert.Equal((ClipAction.Toggle, false), Run(0x09, 0x20));
        Assert.Equal(((ClipAction?)null, true), Run(0x07, 0x20));
        Assert.Equal((ClipAction.Stop, false), Run(0x07, 0x00));
        Assert.Equal((ClipAction.Start, true), Run(0x07, 0x20));
        Assert.Equal((ClipAction.Toggle, false), mock.ClipPacket(TrafficClass.HidIn, 5, Direction.In, [0x07, 0x20]));
        Assert.Equal((ClipAction.Toggle, false), mock.ClipPacket(TrafficClass.HidIn, 5, Direction.In, []));
        Assert.Equal(((ClipAction?)null, false), mock.ClipPacket(TrafficClass.HidOut, 2, Direction.Out, [0x07, 0x20]));
        Assert.Equal([3, 4, 1], clip.QueryConfig().PacketTriggers.Select(row => (int)row.Hits));
        Assert.Throws<ArgumentOutOfRangeException>(() => mock.ClipPacket((TrafficClass)200, 2, Direction.In, []));
        Assert.Throws<ArgumentOutOfRangeException>(() => mock.ClipPacket((TrafficClass)CatchClass.Key, 2, Direction.In, []));
        Assert.Throws<ArgumentOutOfRangeException>(() => mock.ClipPacket(TrafficClass.HidIn, 2, (Direction)200, []));
    }

    [Fact]
    public void APacketTriggersValuesAreChecked()
    {
        ClipPacketTrigger[] bad =
        [
            new((TrafficClass)200, 2, Direction.In, ClipAction.Start),
            new((TrafficClass)CatchClass.Key, 2, Direction.In, ClipAction.Start),
            new(TrafficClass.HidIn, 2, (Direction)200, ClipAction.Start),
        ];
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        using var clip = device.Clip();
        foreach (var trigger in bad)
        {
            Assert.Throws<ArgumentOutOfRangeException>(() => clip.BindPacket(trigger));
            Assert.Throws<ArgumentOutOfRangeException>(() => clip.UnbindPacket(trigger));
            Assert.Throws<ArgumentOutOfRangeException>(() => mock.SetClipSettings(new ClipSettings { PacketTriggers = [trigger] }));
        }
        Assert.Throws<ArgumentOutOfRangeException>(() => clip.BindPacket(new ClipPacketTrigger(TrafficClass.HidIn, 2, Direction.In, (ClipAction)200)));
        Assert.Empty(Frames(mock, FrameType.ClipTrigger));
    }

    [Fact]
    public void TheClipEnumsAreChecked()
    {
        using var mock = new MockBox();
        using (var device = Device.WithMock(mock))
        using (var clip = device.Clip())
        using (var builder = new ClipBuilder())
        {
            Assert.Throws<ArgumentOutOfRangeException>(() => clip.Unbind(Medius.Button.Left, (Edge)200));
            Assert.Throws<ArgumentOutOfRangeException>(() => clip.Bind(new ClipTrigger(Medius.Button.Left, (Edge)200, ClipAction.Start)));
            Assert.Throws<ArgumentOutOfRangeException>(() => clip.Bind(new ClipTrigger(Medius.Button.Left, Edge.Press, (ClipAction)200)));
            Assert.Throws<ArgumentOutOfRangeException>(() => clip.SetAutolock(Blanket.Aim, (Blanket)99));
            Assert.Throws<ArgumentOutOfRangeException>(() => builder.Edge(Medius.Button.Left, (InjectAction)200));
            Assert.Throws<ArgumentOutOfRangeException>(() => builder.Frame(edges: [(Medius.Button.Left, (InjectAction)200)]));
        }
        Assert.Empty(Frames(mock, FrameType.ClipTrigger));
        Assert.Empty(Frames(mock, FrameType.ClipSet));
    }
}
