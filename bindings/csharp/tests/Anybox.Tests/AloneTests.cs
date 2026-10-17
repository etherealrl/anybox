using System.Reflection;

namespace Anybox.Tests;

// records what each call hands to C, the C side checks those against the Rust calls frame by frame, this checks
// C# puts them in the right place on the way there
public class Recorder : DispatchProxy
{
    public List<string> Calls { get; } = [];

    protected override object? Invoke(MethodInfo? method, object?[]? args)
    {
        var values = args!.Skip(1).Select(value => value is AnyboxClick click
            ? $"({click.hold_us}, {click.has_hold}, {click.native_only})"
            : $"{value}");
        Calls.Add(string.Join(' ', [method!.Name["anybox_device_".Length..], .. values]));
        return 0u;
    }
}

[Collection(Alone.Name)]
public sealed class AloneTests
{
    [Fact]
    public void EveryOutputCallHandsItsArgumentsToC()
    {
        var (device, box) = Mock.Medius();
        using var _ = box;
        using var __ = device;
        var real = Native.Api;
        var recording = DispatchProxy.Create<INative, Recorder>();
        Native.Api = recording;
        try
        {
            device.MoveBy(3, -4);
            device.MoveNow(-5, 6);
            device.Press(Button.Right);
            device.Release(Button.Extra(7));
            device.Click(Button.Middle);
            device.Click(Button.Left, TimeSpan.FromMicroseconds(12_500), nativeOnly: true);
            device.PressKey(Key.B);
            device.ReleaseKey((Key)0x64);
            device.TapKey(Key.C, TimeSpan.FromMilliseconds(2));
            device.PressMedia(MediaKey.Mute);
            device.ReleaseMedia(MediaKey.VolumeUp);
            device.TapMedia(MediaKey.Play, nativeOnly: true);
            device.LockButton(Button.Side1);
            device.UnlockButton(Button.Side2);
            device.LockKey(Key.D);
            device.UnlockKey(Key.E);
            device.LockAxis(Axis.Y, Direction.Negative);
            device.UnlockAxis(Axis.Wheel);
            device.ScaleAxis(Axis.Pan, Direction.Positive, -100);
            device.Wheel(-2);
            device.Pan(9);
            device.DiscardPendingMoves();
            device.Close();
        }
        finally
        {
            Native.Api = real;
        }
        Assert.Equal(
            [
                "move_by 3 -4",
                "move_now -5 6",
                "press 1",
                "release 263",
                "click_with 2 (0, 0, 0)",
                "click_with 0 (12500, 1, 1)",
                "press_key 5",
                "release_key 100",
                "tap_key_with 6 (2000, 1, 0)",
                "press_media 226",
                "release_media 233",
                "tap_media_with 176 (0, 0, 1)",
                "lock_button 3",
                "unlock_button 4",
                "lock_key 7",
                "unlock_key 8",
                "lock_axis 1 2",
                "unlock_axis 2 0",
                "scale_axis 3 1 -100",
                "wheel -2",
                "pan 9",
                "discard_pending_moves",
                "close",
            ],
            ((Recorder)recording).Calls);
    }
}
