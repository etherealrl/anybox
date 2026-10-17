using System.Diagnostics;
using System.Runtime.CompilerServices;

namespace Anybox.Tests;

// the package over the mock build. the box behavior itself is the Rust crate's, tested there, these check that
// every value and handle makes it across the right way
public sealed class DeviceTests : IDisposable
{
    readonly Device device;
    readonly Mock box;

    public DeviceTests() => (device, box) = Mock.Medius();

    public void Dispose()
    {
        device.Dispose();
        box.Dispose();
    }

    static void WaitFor(Func<bool> condition) => Assert.True(SpinWait.SpinUntil(condition, TimeSpan.FromSeconds(2)), "timed out");

    // the next `count` button, key, media and motion events, skipping the connection ones
    static List<Event> Inputs(Events events, int count)
    {
        var seen = new List<Event>();
        while (seen.Count < count)
        {
            var next = events.Recv(TimeSpan.FromSeconds(2));
            Assert.NotNull(next);
            if (next is ButtonEvent or MotionEvent or KeyEvent or MediaEvent)
            {
                seen.Add(next);
            }
        }
        return seen;
    }

    [Fact]
    public void EveryErrorCarriesTheRustFields()
    {
        Assert.Equal(["COM3", "COM4"], Mock.ExampleError<NotFoundException>().Tried);
        Assert.Equal("COM3", Mock.ExampleError<PortBusyException>().Port);
        Assert.Equal("/dev/ttyACM0", Mock.ExampleError<PermissionDeniedException>().Port);
        var family = Mock.ExampleError<WrongFamilyException>();
        Assert.Equal(("COM3", Family.Ferrum, (Family?)Family.MakcuV4), (family.Port, family.Expected, family.Found));
        var box = Mock.ExampleError<WrongBoxException>();
        Assert.Equal(StableId.FromUsbSerial("ABC123"), box.Expected);
        Assert.Equal(StableId.FromMac([1, 2, 3, 4, 5, 6]), box.Found);
        var api = Mock.ExampleError<WrongFerrumApiException>();
        Assert.Equal((FerrumApi.Software, FerrumApi.Legacy), (api.Expected, api.Found));
        var old = Mock.ExampleError<FirmwareTooOldException>();
        Assert.Equal((Family.Medius, "protocol 8", "3.4.0"), (old.Family, old.Found?.ToString(), old.Needed?.ToString()));
        var newer = Mock.ExampleError<LibraryTooOldException>();
        Assert.Equal((Family.MakcuV4, "V9.9"), (newer.Family, newer.Found?.ToString()));
        var unsupported = Mock.ExampleError<UnsupportedException>();
        Assert.Equal(
            ((BoxAction?)BoxAction.KeyPress, (Family?)Family.Medius, "no keyboard is plugged into it"),
            (unsupported.Action, unsupported.Family, unsupported.Reason));
        // the C ABI's own refusal names no action or family, like Python's None
        var plain = Assert.IsType<UnsupportedException>(AnyboxException.For(11, "text"));
        Assert.Equal(((BoxAction?)null, (Family?)null), (plain.Action, plain.Family));
        var invalid = Mock.ExampleError<InvalidArgumentException>();
        Assert.Equal(((BoxAction?)BoxAction.ScaleAxis, "300 is out of range, a scale goes from -255 to 255"), (invalid.Action, invalid.Detail));
        var rejected = Mock.ExampleError<RejectedException>();
        Assert.Equal((BoxAction.Move, "its queue is full"), (rejected.Action, rejected.Detail));
        Assert.Equal(BoxAction.LockAxis, Mock.ExampleError<BoxTimeoutException>().Action);
        Assert.Equal(BoxAction.Click, Mock.ExampleError<BoxInterruptedException>().Action);
        var io = Mock.ExampleError<IoException>();
        Assert.Equal((OperatingSystem.IsWindows() ? IoKind.PermissionDenied : IoKind.Other, (int?)5), (io.Kind, io.OsError));
        Mock.ExampleError<ClosedException>();
        Mock.ExampleError<DisconnectedException>();
        Assert.Contains("no example", Assert.Throws<InvalidArgumentException>(Mock.ExampleError<PanicException>).Message);
        Assert.Throws<InvalidArgumentException>(Mock.ExampleError<RecvClosedException>);
    }

    [Fact]
    public void ABoxThatCantComesWithWhatItCouldntDo()
    {
        var incapable = Assert.Single(Mock.ExampleError<NoBoxCanException>().Boxes);
        Assert.Equal(("COM5", Family.MakcuV4), (incapable.Box.Info.Port, incapable.Box.Info.Family));
        Assert.Equal([BoxAction.KeyTap, BoxAction.LockKey], incapable.Missing.Order());
    }

    [Fact]
    public void AScanEntryKeepsItsWholeError()
    {
        var scanned = Assert.Single(new Connect { Port = "NOPE97" }.Scan());
        Assert.Equal(["NOPE97"], Assert.IsType<NotFoundException>(scanned.Error).Tried);
    }

    [Fact]
    public void TheMockPushesKeysMediaALinkLossAndARestart()
    {
        var (keyboard, keyboardBox) = Mock.Medius(keyboard: true);
        using (keyboard)
        using (keyboardBox)
        using (var events = keyboard.Events())
        {
            WaitFor(() => keyboardBox.Subscribed(EventKinds.All));
            var before = Monotonic.NowNs;
            keyboardBox.PushKeys(1, 100, true, [Key.LeftShift, Key.A]);
            var pressed = Inputs(events, 2).Cast<KeyEvent>().ToArray();
            Assert.Equal((Key.LeftShift, true, Key.A), (pressed[0].Key, pressed[0].Pressed, pressed[1].Key));
            Assert.True(pressed[0].Key.IsModifier && !pressed[1].Key.IsModifier);
            Assert.InRange(pressed[0].At.ReceivedNs, before, Monotonic.NowNs);
            keyboardBox.PushKeys(2, 200, false, []);
            Assert.All(Inputs(events, 2).Cast<KeyEvent>(), key => Assert.False(key.Pressed));
            keyboardBox.PushMedia(3, 300, true, [MediaKey.VolumeUp]);
            var media = Assert.IsType<MediaEvent>(Assert.Single(Inputs(events, 1)));
            Assert.Equal((MediaKey.VolumeUp, true), (media.Key, media.Pressed));

            void Until<T>()
            {
                var clock = Stopwatch.StartNew();
                while (events.Recv(TimeSpan.FromMilliseconds(500)) is not T)
                {
                    Assert.True(clock.Elapsed < TimeSpan.FromSeconds(10), $"no {typeof(T).Name}");
                }
            }

            keyboardBox.LinkLost();
            Until<SessionReset>();
            keyboardBox.Restart();
            Until<Disconnected>();
            Until<Reconnected>();
        }
    }

    [Fact]
    public void ASnapshotDescribesTheMock()
    {
        var snapshot = device.Snapshot();
        Assert.Equal(Family.Medius, snapshot.Info.Family);
        Assert.Equal("mock box", snapshot.Info.Port);
        Assert.Null(snapshot.Info.FerrumApi);
        Assert.Equal([Button.Left, Button.Right, Button.Middle, Button.Side1, Button.Side2], snapshot.Capabilities.Buttons);
        Assert.Equal([Axis.X, Axis.Y, Axis.Wheel], snapshot.Capabilities.Axes);
        Assert.Equal(Enum.GetValues<BoxAction>(), snapshot.Capabilities.Keys.Order());
        var move = snapshot.Capabilities[BoxAction.Move];
        Assert.True(move.Supported);
        Assert.Equal((Execution.Native, (string?)null), (move.Execution, move.Reason));
        var click = snapshot.Capabilities[BoxAction.Click];
        Assert.Equal(Execution.Host, click.Execution);
        Assert.Equal([new Quirk(QuirkKind.HostClickTiming) { ClickTiming = ClickTiming.Makcu }], click.Quirks);
        var keys = snapshot.Capabilities[BoxAction.KeyPress];
        Assert.False(keys.Supported);
        Assert.False(string.IsNullOrEmpty(keys.Reason));
        Assert.Contains(snapshot.Capabilities.Values, capability => capability.Quirks.Any(quirk => quirk.Kind == QuirkKind.KeepsHeld));
    }

    [Fact]
    public void TheClickTimingReachesTheSnapshot()
    {
        var timing = ClickTiming.Fixed(TimeSpan.FromMilliseconds(42));
        var (other, otherBox) = Mock.Medius(timing);
        using (other)
        using (otherBox)
        {
            var quirk = Assert.Single(other.Snapshot().Capabilities[BoxAction.Click].Quirks);
            Assert.Equal(timing, quirk.ClickTiming);
        }
    }

    [Fact]
    public void EveryOutputCallReachesTheBox()
    {
        device.MoveBy(3, -4);
        device.MoveNow(1, 1);
        device.Press(Button.Right);
        device.Release(Button.Right);
        device.Click(Button.Left);
        device.Click(Button.Left, TimeSpan.FromMilliseconds(1));
        device.LockButton(Button.Middle);
        device.UnlockButton(Button.Middle);
        device.LockAxis(Axis.Y, Direction.Positive);
        device.UnlockAxis(Axis.Y, Direction.Positive);
        device.LockAxis(Axis.X);
        device.UnlockAxis(Axis.X);
        device.ScaleAxis(Axis.X, Direction.Both, 50);
        device.Wheel(-2);
        device.DiscardPendingMoves();
        // a mouse with no keyboard behind it and no pan refuses the rest, that's the box answering
        Action[] refused =
        [
            () => device.PressKey(Key.A),
            () => device.ReleaseKey(Key.A),
            () => device.TapKey(Key.A),
            () => device.TapKey(Key.A, TimeSpan.FromMilliseconds(1)),
            () => device.PressMedia(MediaKey.Mute),
            () => device.ReleaseMedia(MediaKey.Mute),
            () => device.TapMedia(MediaKey.Mute),
            () => device.TapMedia(MediaKey.Mute, TimeSpan.FromMilliseconds(1)),
            () => device.LockKey(Key.A),
            () => device.UnlockKey(Key.A),
            () => device.Pan(1),
        ];
        foreach (var call in refused)
        {
            Assert.Contains("can't", Assert.Throws<UnsupportedException>(call).Message);
        }
    }

    [Fact]
    public void AnErrorKeepsTheLibrarysText()
    {
        var invalid = Assert.Throws<InvalidArgumentException>(() => device.ScaleAxis(Axis.X, Direction.Both, 300));
        Assert.Contains("300", invalid.Message);
        Assert.Equal(12u, invalid.Status);
        Assert.Equal(11u, Assert.Throws<UnsupportedException>(() => device.Click(Button.Left, nativeOnly: true)).Status);
    }

    [Fact]
    public void ValuesThatCantCrossAreRefused()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => device.Click(Button.Left, TimeSpan.FromTicks(-1)));
        Assert.Throws<ArgumentOutOfRangeException>(() => device.TapKey(Key.A, TimeSpan.FromMilliseconds(-1)));
        Assert.Throws<ArgumentOutOfRangeException>(() => device.Events((EventKinds)16));
    }

    [Fact]
    public void AClosedDeviceRefusesCommandsAndClosesOnceLikeRust()
    {
        device.Close();
        Assert.Throws<ClosedException>(device.Close);
        Assert.Throws<ClosedException>(() => device.MoveBy(1, 1));
        Assert.Equal(Family.Medius, device.Snapshot().Info.Family);
        using var events = device.Events();
        Assert.Empty(events.OfType<ButtonEvent>());
    }

    [Fact]
    public void ADisposedDeviceThrowsObjectDisposed()
    {
        device.Dispose();
        Assert.Throws<ObjectDisposedException>(() => device.MoveBy(1, 1));
        Assert.Throws<ObjectDisposedException>(() => device.Snapshot());
        device.Dispose();
    }

    [Fact]
    public void DisposingClosesTheDeviceWhenYouDidnt()
    {
        var (other, otherBox) = Mock.Medius();
        using var _ = otherBox;
        var events = other.Events(EventKinds.Buttons);
        other.Dispose();
        Assert.Empty(events.OfType<ButtonEvent>());
        events.Dispose();
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    static (WeakReference Device, Mock Box) Dropped()
    {
        var (other, otherBox) = Mock.Medius();
        return (new WeakReference(other), otherBox);
    }

    static void Collect()
    {
        GC.Collect();
        GC.WaitForPendingFinalizers();
        GC.Collect();
    }

    [Fact]
    public void ADroppedDeviceGetsFreed()
    {
        var (dropped, otherBox) = Dropped();
        using var _ = otherBox;
        Collect();
        Assert.False(dropped.IsAlive);
    }

    // the receiver only ever sits in the list, so no hidden local of the test keeps it reachable
    [MethodImpl(MethodImplOptions.NoInlining)]
    static List<Events> Receiver(out WeakReference weak, out Mock otherBox)
    {
        (var other, otherBox) = Mock.Medius();
        weak = new WeakReference(other);
        return [other.Events()];
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    static void Drop(List<Events> held)
    {
        held[0].Dispose();
        held.Clear();
    }

    [Fact]
    public void AReceiverKeepsItsDeviceOpen()
    {
        var held = Receiver(out var weak, out var otherBox);
        using var _ = otherBox;
        Collect();
        Assert.True(weak.IsAlive);
        Drop(held);
        Collect();
        Assert.False(weak.IsAlive);
    }

    [Fact]
    public void PushedInputComesOutAsEvents()
    {
        List<Event> seen;
        using (var events = device.Events())
        {
            WaitFor(() => box.Subscribed(EventKinds.Buttons | EventKinds.Motion));
            box.PushMotion(1, 1000, 3, -4, 1, 0);
            box.PushButtons(2, 2000, true, 1 | (1 << 6));
            box.PushButtons(3, 3000, false, 0);
            seen = Inputs(events, 5);
        }
        var motion = seen.OfType<MotionEvent>().First();
        Assert.Equal((3, -4, 1, 0), (motion.Dx, motion.Dy, motion.Wheel, motion.Pan));
        Assert.Equal(new BoxTime(1000, Clock.MediusHostChip), motion.At.BoxTime);
        // on a Medius the event time is the box clock mapped onto the PC's, so it can come out after the receive
        Assert.True(motion.At.ReceivedSinceOpen > TimeSpan.Zero);
        Assert.Equal(OperatingSystem.IsWindows(), motion.At.ReceivedQpc is not null);
        Assert.Equal(
            new[] { (Button.Left, true), (Button.Extra(6), true), (Button.Left, false), (Button.Extra(6), false) }.Order(),
            seen.OfType<ButtonEvent>().Select(button => (button.Button, button.Pressed)).Order());
    }

    [Fact]
    public void AKindTheBoxCantStreamIsRejectedOnItsOwnReceiver()
    {
        using var events = device.Events(EventKinds.Keys);
        Event? next;
        // a fresh receiver can get a connection event first, like in the C tests
        while ((next = events.Recv(TimeSpan.FromSeconds(2))) is not Rejected)
        {
            Assert.NotNull(next);
        }
        Assert.Equal(new Rejected(BoxAction.KeyEvents, 1), next);
    }

    [Fact]
    public void RecvTimesOutTriesAndEnds()
    {
        using var events = device.Events(EventKinds.Buttons);
        while (events.TryRecv() is not null)
        {
        }
        var clock = Stopwatch.StartNew();
        Assert.Null(events.Recv(TimeSpan.FromMilliseconds(150)));
        Assert.True(clock.Elapsed >= TimeSpan.FromMilliseconds(140));
        Assert.Throws<ArgumentOutOfRangeException>(() => events.Recv(TimeSpan.FromMilliseconds(-1)));
        device.Close();
        Assert.Empty(events.OfType<ButtonEvent>());
        // Rust's own text and its own error, not the closed device's
        var ended = Assert.Throws<RecvClosedException>(() => events.Recv());
        Assert.Equal("the device is gone, no more events will come", ended.Message);
    }

    static CancellationToken Cancel => TestContext.Current.CancellationToken;

    static Task<T> OnItsOwnThread<T>(Func<T> work) =>
        Task.Factory.StartNew(work, Cancel, TaskCreationOptions.LongRunning, TaskScheduler.Default);

    [Fact]
    public async Task ClosingTheDeviceFromAnotherThreadEndsAReader()
    {
        var events = device.Events(EventKinds.Buttons);
        var reader = OnItsOwnThread(() => events.OfType<ButtonEvent>().ToList());
        await Task.Delay(200, Cancel);
        device.Close();
        Assert.Empty(await reader.WaitAsync(TimeSpan.FromSeconds(3), Cancel));
    }

    [Fact]
    public async Task DisposingAReceiverFromAnotherThreadEndsItsReader()
    {
        var events = device.Events(EventKinds.Buttons);
        var reader = OnItsOwnThread(() => Assert.Throws<ClosedException>(() => events.Recv()));
        // a reader that isn't waiting yet just finds it disposed, which ends it too, the wait below is what fails a
        // receive that never ends
        await Task.Delay(200, Cancel);
        events.Dispose();
        var error = await reader.WaitAsync(TimeSpan.FromSeconds(3), Cancel);
        Assert.Contains("disposed", error.Message);
        foreach (var _ in events)
        {
            Assert.Fail("a disposed receiver gave an event");
        }
        // a second dispose does nothing
        events.Dispose();
        device.MoveBy(1, 1);
    }

    [Fact]
    public async Task DisposingAReceiverDuringATimedWaitEndsItToo()
    {
        var events = device.Events(EventKinds.Buttons);
        // the wait is far longer than the test waits for the reader, so only the dispose waking it ends it in time
        var reader = OnItsOwnThread(() => Assert.Throws<ClosedException>(() => events.Recv(TimeSpan.FromSeconds(30))));
        await Task.Delay(200, Cancel);
        events.Dispose();
        Assert.Contains("disposed", (await reader.WaitAsync(TimeSpan.FromSeconds(3), Cancel)).Message);
    }

    [Fact]
    public async Task TwoThreadsReadingOneReceiverSplitTheEvents()
    {
        var events = device.Events(EventKinds.Motion);
        WaitFor(() => box.Subscribed(EventKinds.Motion));
        var got = new List<int>();
        var readers = Enumerable.Range(0, 2).Select(_ => OnItsOwnThread(() =>
        {
            foreach (var motion in events.OfType<MotionEvent>())
            {
                lock (got)
                {
                    got.Add(motion.Dx);
                }
            }
            return 0;
        })).ToArray();
        for (var seq = 1; seq <= 40; seq++)
        {
            box.PushMotion((byte)seq, (uint)seq * 1000, (short)seq, 0);
        }
        WaitFor(() =>
        {
            lock (got)
            {
                return got.Count == 40;
            }
        });
        device.Close();
        await Task.WhenAll(readers).WaitAsync(TimeSpan.FromSeconds(3), Cancel);
        Assert.Equal(Enumerable.Range(1, 40), got.Order());
    }

    [Fact]
    public void CallsFromManyThreadsAtOnceWork()
    {
        Parallel.For(0, 4, _ =>
        {
            for (var round = 0; round < 50; round++)
            {
                device.MoveBy(1, 0);
                device.Snapshot();
            }
        });
    }

    [Fact]
    public async Task AnErrorOnOneThreadKeepsItsOwnText()
    {
        var scale = OnItsOwnThread(() => Assert.ThrowsAny<AnyboxException>(() => device.ScaleAxis(Axis.X, Direction.Both, 300)).Message);
        var key = OnItsOwnThread(() => Assert.ThrowsAny<AnyboxException>(() => device.PressKey(Key.A)).Message);
        Assert.Contains("300", await scale);
        Assert.Contains("key", await key);
    }

    [Fact]
    public void TheMockRefusesAfterItsDisposed()
    {
        box.Dispose();
        Assert.Throws<ObjectDisposedException>(() => box.PushMotion(1, 1, 1, 1));
    }
}
