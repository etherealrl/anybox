namespace Anybox;

/// <summary>the medius crate's model of a Medius box, for testing code that uses anybox with no box plugged in. it
/// proves nothing about hardware and it can only be shaped like a Medius. the library in the package doesn't have
/// it, only one built with the capi crate's <c>mock</c> feature</summary>
public sealed class Mock : IDisposable
{
    readonly MockHandle handle;

    Mock(MockHandle handle) => this.handle = handle;

    /// <summary>a device over a new mock box, a five button mouse with a wheel, and with
    /// <paramref name="keyboard"/> a six key keyboard with media keys behind it too, running the exact code a real
    /// Medius gets and checking the link every 250 ms like a real one</summary>
    public static (Device Device, Mock Mock) Medius(ClickTiming? clickTiming = null, bool keyboard = false)
    {
        NeedMock();
        var timing = (clickTiming ?? ClickTiming.Makcu).ToNative();
        Library.Check(Imports.anybox_mock_medius_open(timing, keyboard ? (byte)1 : (byte)0, out var mock, out var device));
        return (new Device(device), new Mock(mock));
    }

    /// <summary>a device over anybox's own model of a MAKCU V4.108 with a mouse and a keyboard behind it, running the
    /// exact code a real one gets over a fake port, for testing the <see cref="Device.Makcu"/> calls with no
    /// box</summary>
    public static Device Makcu(ClickTiming? clickTiming = null)
    {
        NeedMock();
        Library.Check(Imports.anybox_mock_makcu_open((clickTiming ?? ClickTiming.Makcu).ToNative(), out var device));
        return new Device(device);
    }

    /// <summary>a device over anybox's own model of a YSC8K V1, for testing the <see cref="Device.Ysc"/> calls with no
    /// box</summary>
    public static Device Ysc(ClickTiming? clickTiming = null)
    {
        NeedMock();
        Library.Check(Imports.anybox_mock_ysc_open((clickTiming ?? ClickTiming.Makcu).ToNative(), out var device));
        return new Device(device);
    }

    /// <summary>a device over anybox's own model of a Ferrum on the app's port, for testing the
    /// <see cref="Device.Ferrum"/> calls with no box</summary>
    public static Device Ferrum(ClickTiming? clickTiming = null)
    {
        NeedMock();
        Library.Check(Imports.anybox_mock_ferrum_open((clickTiming ?? ClickTiming.Makcu).ToNative(), out var device));
        return new Device(device);
    }

    /// <summary>the same as <see cref="Medius"/>, except that the link only gets checked on <see cref="Tick"/>, so a
    /// test decides when a link event gets noticed and nothing lands between the frames it records</summary>
    public static (Device Device, Mock Mock) MediusManual(ClickTiming? clickTiming = null, bool keyboard = false)
    {
        NeedMock();
        var timing = (clickTiming ?? ClickTiming.Makcu).ToNative();
        Library.Check(Imports.anybox_mock_medius_open_manual(timing, keyboard ? (byte)1 : (byte)0, out var mock, out var device));
        return (new Device(device), new Mock(mock));
    }

    /// <summary>anybox's example of that exception, with every field it has filled in, so you can
    /// test how your code handles it with no box. the ones only this package makes (<see cref="PanicException"/>,
    /// <see cref="UnknownException"/>, <see cref="RecvClosedException"/>) have none</summary>
    public static T ExampleError<T>() where T : AnyboxException
    {
        NeedMock();
        // the status that makes a T, every status there is fits in a byte
        var status = Enumerable.Range(0, 256).Select(number => (uint)number)
            .FirstOrDefault(number => AnyboxException.For(number, "") is T);
        var made = Imports.anybox_mock_fail(status);
        return made == status && Library.Error(made) is T example ? example : throw Library.Error(made);
    }

    static void NeedMock()
    {
        if (!Library.HasMock)
        {
            throw new AnyboxException(
                "this anybox library was built without the mock, build anybox-capi with --features mock");
        }
    }

    MockHandle Handle
    {
        get
        {
            ObjectDisposedException.ThrowIf(handle.IsClosed, this);
            return handle;
        }
    }

    /// <summary>whether the mock box got the subscription of every one of these kinds yet. a receiver subscribes
    /// each kind on the device's own threads, the keys and media a moment after the buttons, and the box drops
    /// what's pushed before its kind is in like a real one would, so wait for this before you push</summary>
    public bool Subscribed(EventKinds kinds) => Imports.anybox_mock_subscribed(Handle, (uint)kinds) != 0;

    /// <summary>physical motion as if from the box. <paramref name="seq"/> is the box's frame counter and
    /// <paramref name="tsUs"/> its clock. a gap in <paramref name="seq"/> doesn't show up as a drop</summary>
    public void PushMotion(byte seq, uint tsUs, short dx, short dy, short wheel = 0, short pan = 0) =>
        Library.Check(Imports.anybox_mock_push_motion(Handle, seq, tsUs, dx, dy, wheel, pan));

    /// <summary>the physical buttons held now, bit n for the box's button n. it's the whole held set, so a release
    /// is the set without that button, and <paramref name="pressed"/> says whether the set grew or shrank</summary>
    public void PushButtons(byte seq, uint tsUs, bool pressed, uint held) =>
        Library.Check(Imports.anybox_mock_push_buttons(Handle, seq, tsUs, pressed ? (byte)1 : (byte)0, held));

    /// <summary>the physical keys held now, modifiers included. it's the whole held set like
    /// <see cref="PushButtons"/>, so the last release is pushing no keys at all</summary>
    public unsafe void PushKeys(byte seq, uint tsUs, bool pressed, ReadOnlySpan<Key> held)
    {
        fixed (Key* keys = held)
        {
            Library.Check(Imports.anybox_mock_push_keys(
                Handle, seq, tsUs, pressed ? (byte)1 : (byte)0, (byte*)keys, (nuint)held.Length));
        }
    }

    /// <summary>the media keys held now, the whole held set like the keys</summary>
    public unsafe void PushMedia(byte seq, uint tsUs, bool pressed, ReadOnlySpan<MediaKey> held)
    {
        fixed (MediaKey* media = held)
        {
            Library.Check(Imports.anybox_mock_push_media(
                Handle, seq, tsUs, pressed ? (byte)1 : (byte)0, (ushort*)media, (nuint)held.Length));
        }
    }

    /// <summary>the link between the box's two chips drops and comes back, so the box lets go of what it held and
    /// the device sends <see cref="SessionReset"/> once it checks the link (within 250 ms, or on the next
    /// <see cref="Tick"/> of a manual mock)</summary>
    public void LinkLost() => Library.Check(Imports.anybox_mock_link_lost(Handle));

    /// <summary>the box restarts, so the device sends <see cref="Disconnected"/> and then
    /// <see cref="Reconnected"/>, noticed the way <see cref="LinkLost"/> is</summary>
    public void Restart() => Library.Check(Imports.anybox_mock_restart(Handle));

    /// <summary>one pass of what a real device checks on the link every 250 ms, on a mock from
    /// <see cref="MediusManual"/>, which is where it notices <see cref="LinkLost"/> and <see cref="Restart"/>. the
    /// mock keeps its own timers (a restarted clone is back 100 ms later), so a test still waits for those</summary>
    public void Tick() => Library.Check(Imports.anybox_mock_tick(Handle));

    /// <summary>every frame the box got since it opened or since <see cref="ClearRecorded"/>, as its type byte (the
    /// protocol's, <c>MOVE</c> is 1) and its payload</summary>
    public unsafe IReadOnlyList<(byte Type, byte[] Payload)> Recorded()
    {
        var count = Imports.anybox_mock_recorded(Handle);
        var frames = new List<(byte, byte[])>();
        for (nuint index = 0; index < count; index++)
        {
            byte type;
            Library.Check(Imports.anybox_mock_recorded_frame(Handle, index, &type, null, 0, out var size));
            var payload = new byte[size];
            fixed (byte* bytes = payload)
            {
                Library.Check(Imports.anybox_mock_recorded_frame(Handle, index, &type, bytes, size, out size));
            }
            frames.Add((type, payload));
        }
        return frames;
    }

    /// <summary>forgets every frame the box got so far</summary>
    public void ClearRecorded() => Library.Check(Imports.anybox_mock_clear_recorded(Handle));

    /// <summary>sends every receiver of <paramref name="device"/> one event of every kind, with every field filled
    /// in, through the same queue as real ones, so you can test how your code reads each kind with no box. none of
    /// them does to the device what the real one would, so it stays connected through the example
    /// <see cref="Disconnected"/></summary>
    public static void ExampleEvents(Device device)
    {
        NeedMock();
        Library.Check(Imports.anybox_mock_example_events(device.Handle));
    }

    public void Dispose() => handle.Dispose();
}
