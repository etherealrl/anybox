using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

namespace Anybox.Tests;

// what comes back from C gets built here from structs filled the way the C side fills them, because the mock can
// only push input
public sealed unsafe class ValueTests
{
    static readonly Stamp At = new(
        TimeSpan.FromTicks(5),
        TimeSpan.FromTicks(6),
        10,
        11,
        7,
        new BoxTime(9, Clock.MediusDeviceChip));

    static AnyboxEvent Stamped(AnyboxEvent raw) => raw with
    {
        since_open_ns = 500,
        received_since_open_ns = 600,
        received_qpc = 7,
        has_received_qpc = 1,
        has_box_time = 1,
        box_clock = 1,
        box_time_us = 9,
        arrived_ns = 10,
        received_ns = 11,
    };

    [Fact]
    public void EveryEventKindComesAcrossWithItsFields()
    {
        (AnyboxEvent Raw, Event Event)[] cases =
        [
            (Stamped(new() { kind = 0, button = 257, pressed = 1 }), new ButtonEvent(Button.Extra(1), true, At)),
            (Stamped(new() { kind = 1, key = 0x04 }), new KeyEvent(Key.A, false, At)),
            (Stamped(new() { kind = 2, media = 0xE2, pressed = 1 }), new MediaEvent(MediaKey.Mute, true, At)),
            (Stamped(new() { kind = 3, dx = -1, dy = 2, wheel = -3, pan = 4 }), new MotionEvent(-1, 2, -3, 4, At)),
            (new() { kind = 4, count = 12, source = 1 }, new Dropped(12, DropSource.Host)),
            (new() { kind = 5 }, new SessionReset()),
            (new() { kind = 6, action = 13, count = 3 }, new Rejected(BoxAction.KeyPress, 3)),
            (new() { kind = 7 }, new Disconnected()),
            (new() { kind = 8 }, new Reconnected()),
            (new() { kind = 9 }, new Changed()),
        ];
        foreach (var (raw, expected) in cases)
        {
            Assert.Equal(expected, Read.Event(raw));
        }
        Assert.Equal(
            new Stamp(TimeSpan.FromTicks(1), TimeSpan.FromTicks(2), 0, 0, null, null),
            ((MotionEvent)Read.Event(new AnyboxEvent { kind = 3, since_open_ns = 100, received_since_open_ns = 200 })).At);
        Assert.Contains("update the Anybox package", Assert.Throws<AnyboxException>(() => Read.Event(new AnyboxEvent { kind = 10 })).Message);
        Assert.Throws<AnyboxException>(() => Read.Event(new AnyboxEvent { kind = 6, action = 23 }));
    }

    [Fact]
    public void EveryInfoFieldComesAcross()
    {
        fixed (byte* port = "COM3\0"u8, model = "V4\0"u8, serial = "SN1\0"u8, text = "Oct 10 2026\0"u8)
        {
            var raw = new AnyboxInfo
            {
                port = port,
                family = 2,
                model = model,
                firmware_kind = 2,
                firmware_build = 4094,
                id_kind = 2,
                id_serial = serial,
                ferrum_api = 1,
                has_peripheral = 1,
                peripheral_vid = 0x046D,
                peripheral_pid = 0xC547,
            };
            Assert.Equal(
                new Info(
                    "COM3",
                    Family.MakcuV4,
                    "V4",
                    new Firmware(FirmwareKind.Build) { Build = 4094 },
                    StableId.FromUsbSerial("SN1"),
                    FerrumApi.Software,
                    new UsbIds(0x046D, 0xC547)),
                Read.Info(raw));
            (AnyboxInfo Raw, string Firmware)[] firmwares =
            [
                (new() { family = 1, firmware_kind = 1, firmware_major = 3, firmware_minor = 4, firmware_patch = 5 }, "3.4.5"),
                (new() { family = 1, firmware_kind = 3, firmware_protocol = 9 }, "protocol 9"),
                (new() { family = 1, firmware_kind = 4, firmware_text = text }, "Oct 10 2026"),
            ];
            foreach (var (info, firmware) in firmwares)
            {
                Assert.Equal(firmware, Read.Info(info).Firmware!.ToString());
            }
        }
        var bare = Read.Info(new AnyboxInfo { family = 1 });
        Assert.Equal(new Info("", Family.Medius, null, null, null, null, null), bare);
        var mac = new AnyboxInfo { family = 1, id_kind = 1 };
        for (var index = 0; index < 6; index++)
        {
            mac.id_mac[index] = (byte)(index + 1);
        }
        Assert.Equal(StableId.FromMac([1, 2, 3, 4, 5, 6]), Read.Info(mac).Id);
        foreach (var unknown in new[] { new AnyboxInfo { family = 1, firmware_kind = 5 }, new AnyboxInfo { family = 1, id_kind = 3 }, new AnyboxInfo { family = 5 } })
        {
            Assert.Contains("update the Anybox package", Assert.Throws<AnyboxException>(() => Read.Info(unknown)).Message);
        }
    }

    [Fact]
    public void EveryQuirkComesAcrossWithItsFields()
    {
        static TimeSpan Ms(double value) => TimeSpan.FromMilliseconds(value);
        (AnyboxQuirk Raw, Quirk Quirk)[] cases =
        [
            (new() { kind = 0, min_us = 125_000, max_us = 175_000 }, new(QuirkKind.ForcedUpOnRelease) { Min = Ms(125), Max = Ms(175) }),
            (new() { kind = 6, min_us = 35_000, max_us = 75_000 }, new(QuirkKind.BoxClickTiming) { Min = Ms(35), Max = Ms(75) }),
            (new() { kind = 5, max_us = 1_000_000 }, new(QuirkKind.MaxHold) { Max = Ms(1000) }),
            (new() { kind = 3, count = 1 }, new(QuirkKind.NotchesPerCommand) { Count = 1 }),
            (new() { kind = 9, count = 6 }, new(QuirkKind.MaxKeysHeld) { Count = 6 }),
            (
                new() { kind = 7, click_timing = new() { kind = 2, fixed_us = 40_000 } },
                new(QuirkKind.HostClickTiming) { ClickTiming = ClickTiming.Fixed(Ms(40)) }
            ),
            (new() { kind = 7, click_timing = new() { kind = 1 } }, new(QuirkKind.HostClickTiming) { ClickTiming = ClickTiming.Ferrum }),
            (new() { kind = 1 }, new(QuirkKind.PhysicalOverride)),
            (new() { kind = 2 }, new(QuirkKind.WholeAxisOnly)),
            (new() { kind = 4 }, new(QuirkKind.FixedClickTiming)),
            (new() { kind = 8 }, new(QuirkKind.KeepsHeld)),
            (new() { kind = 10 }, new(QuirkKind.KeepsLocked)),
        ];
        foreach (var (raw, quirk) in cases)
        {
            Assert.Equal(quirk, Read.Quirk(raw));
        }
        Assert.Equal(Enum.GetValues<QuirkKind>().Order(), cases.Select(pair => pair.Quirk.Kind).Distinct().Order());
        Assert.Throws<AnyboxException>(() => Read.Quirk(new AnyboxQuirk { kind = 11 }));
        Assert.Throws<AnyboxException>(() => Read.Quirk(new AnyboxQuirk { kind = 7, click_timing = new() { kind = 4 } }));
    }

    [Fact]
    public void AStableIdIsOneOfTheTwo()
    {
        Assert.Equal("MAC 01:02:03:04:05:ab", StableId.FromMac([1, 2, 3, 4, 5, 0xAB]).ToString());
        Assert.Equal("USB serial X1", StableId.FromUsbSerial("X1").ToString());
        Assert.Throws<ArgumentException>(() => StableId.FromMac([1, 2, 3, 4, 5]));
        Assert.Throws<ArgumentNullException>(() => StableId.FromUsbSerial(null!));
    }

    [Fact]
    public void FirmwareReadsLikeTheRustOne()
    {
        Assert.Equal("3.4.2", new Firmware(FirmwareKind.Version) { Version = new(3, 4, 2) }.ToString());
        Assert.Equal("build 7", new Firmware(FirmwareKind.Build) { Build = 7 }.ToString());
        Assert.Equal("Oct 10 2026", new Firmware(FirmwareKind.Text) { Text = "Oct 10 2026" }.ToString());
    }

    [Fact]
    public void AnyKeyHasAValueEvenWithoutAName()
    {
        Assert.Equal(0x64, (byte)(Key)0x64);
        Assert.Equal(Key.A, (Key)0x04);
        Assert.Equal(0x1234, (ushort)(MediaKey)0x1234);
    }

    [Fact]
    public void ClickTimingsAreTheFourKinds()
    {
        Assert.Equal(ClickTiming.Fixed(TimeSpan.FromMilliseconds(50)), ClickTiming.Fixed(TimeSpan.FromMicroseconds(50_000)));
        Assert.NotEqual(ClickTiming.Makcu, ClickTiming.Ferrum);
        Assert.Null(ClickTiming.Off.Hold);
        Assert.Throws<ArgumentOutOfRangeException>(() => ClickTiming.Fixed(TimeSpan.FromTicks(-1)));
        var fixedTiming = ClickTiming.Fixed(TimeSpan.FromMilliseconds(50)).ToNative();
        Assert.Equal((2u, 50_000ul), (fixedTiming.kind, fixedTiming.fixed_us));
        Assert.Equal((3u, 0ul), (ClickTiming.Off.ToNative().kind, ClickTiming.Off.ToNative().fixed_us));
    }

    [Fact]
    public void ConnectOptionsCarryEveryField()
    {
        var connect = new Connect
        {
            Port = "COM9",
            Family = Family.Ferrum,
            FerrumApi = FerrumApi.Legacy,
            Can = [BoxAction.Move, BoxAction.LockKey],
            Id = StableId.FromMac([0, 1, 2, 3, 4, 5]),
            ClickTiming = ClickTiming.Fixed(TimeSpan.FromMilliseconds(50)),
        };
        connect.WithOptions(options =>
        {
            Assert.Equal("COM9", Library.Text(options.port));
            Assert.Equal((3u, 2u), (options.family, options.ferrum_api));
            Assert.Equal([1u, 21u], new ReadOnlySpan<uint>(options.can, (int)options.can_len).ToArray());
            Assert.Equal(1u, options.id_kind);
            Assert.Equal(new byte[] { 0, 1, 2, 3, 4, 5 }, new ReadOnlySpan<byte>(options.id_mac, 6).ToArray());
            Assert.True(options.id_serial == null);
            Assert.Equal((2u, 50_000ul), (options.click_timing.kind, options.click_timing.fixed_us));
            return 0;
        });
        new Connect { Id = StableId.FromUsbSerial("AB12") }.WithOptions(options =>
        {
            Assert.Equal((2u, "AB12"), (options.id_kind, Library.Text(options.id_serial)));
            return 0;
        });
        new Connect().WithOptions(options =>
        {
            var bytes = MemoryMarshal.AsBytes(new ReadOnlySpan<AnyboxConnectOptions>(in options));
            Assert.All(bytes.ToArray(), value => Assert.Equal(0, value));
            return 0;
        });
    }

    [Fact]
    public void TextThatCantCrossIsRefused()
    {
        Assert.Contains("NUL", Assert.Throws<ArgumentException>(() => new Connect { Port = "CO\0M3" }.Candidates()).Message);
        Assert.ThrowsAny<ArgumentException>(() => new Connect { Port = "COM\uD800" }.Candidates());
        Assert.ThrowsAny<ArgumentException>(() => new Connect { Id = StableId.FromUsbSerial("\uDC00") }.Candidates());
    }

    [Fact]
    public void AnActionTheLibraryDoesntKnowIsRefusedWithNoActionNamed()
    {
        foreach (var unknown in new[] { (BoxAction)64, (BoxAction)23 })
        {
            var refused = Assert.Throws<InvalidArgumentException>(() => new Connect { Can = [unknown] }.Candidates());
            Assert.Null(refused.Action);
            Assert.Equal("", refused.Detail);
        }
    }

    [Fact]
    public void TheHelpersMatchRust()
    {
        Assert.Equal(
            Enumerable.Range(0xE0, 8).Select(value => (Key)value),
            Enumerable.Range(0, 256).Select(value => (Key)value).Where(key => key.IsModifier));
        Assert.Equal(TimeSpan.FromMicroseconds(4_294_967_296), Clock.MediusHostChip.WrapsAfter);
        Assert.Equal(TimeSpan.FromMicroseconds(4_294_967_296), Clock.MediusDeviceChip.WrapsAfter);
    }

    [Fact]
    public void TheMonotonicClockIsTheOneStopwatchReads()
    {
        var before = Monotonic.FromStopwatchTicks(System.Diagnostics.Stopwatch.GetTimestamp());
        var now = Monotonic.NowNs;
        var after = Monotonic.FromStopwatchTicks(System.Diagnostics.Stopwatch.GetTimestamp());
        // both round the raw counter down to nanoseconds, a microsecond covers any difference in how
        Assert.InRange(now, before - 1_000, after + 1_000);
        // a second of ticks is a second whatever the frequency, and a round trip only loses what a tick can't hold
        var frequency = System.Diagnostics.Stopwatch.Frequency;
        Assert.Equal(1_000_000_000L, Monotonic.FromStopwatchTicks(frequency));
        Assert.Equal(frequency, Monotonic.ToStopwatchTicks(1_000_000_000L));
        foreach (var nanos in new[] { 1L, 12_345_678_901L, 86_400_000_000_123L })
        {
            var back = Monotonic.FromStopwatchTicks(Monotonic.ToStopwatchTicks(nanos));
            Assert.InRange(nanos - back, 0, 1_000_000_000L / frequency + 1);
        }
    }

    [Fact]
    public void ANamedPortIsTheOnlyCandidateAndItsScanSaysWhyNothingIsThere()
    {
        var connect = new Connect { Port = "NOPE99" };
        Assert.Equal(["NOPE99"], connect.Candidates());
        var scanned = Assert.Single(connect.Scan());
        Assert.Equal("NOPE99", scanned.Port);
        Assert.Null(scanned.Found);
        Assert.Contains("NOPE99", scanned.Error!.Message);
        var opened = Assert.ThrowsAny<AnyboxException>(() => connect.Open());
        Assert.Equal(scanned.Error.GetType(), opened.GetType());
        Assert.Equal(scanned.Error.Message, opened.Message);
    }
}
