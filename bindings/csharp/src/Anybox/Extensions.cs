using System.Runtime.InteropServices;

namespace Anybox;

/// <summary>how long a MAKCU spreads a move, <see cref="Auto"/> or <see cref="Percent"/> from 0 to 100</summary>
public readonly record struct Interpolation(bool Automatic, byte Percentage)
{
    public static Interpolation Auto => new(true, 0);

    public static Interpolation Percent(byte percent) => new(false, percent);
}

/// <summary>the shape a YSC gives a spread move, a number the vendor doesn't name yet is kept as the box sent
/// it</summary>
public enum YscProfile : uint
{
    Linear = 0,
    EaseInOut = 1,
    MinJerk = 2,
}

/// <summary>a YSC's interpolation as the box reports it. what the box didn't send is null, the vendor's own app only
/// reads <see cref="Enabled"/>, <see cref="Profile"/> and the measured interval. a <see cref="Window"/> of zero means
/// the box adapts it to how often moves come</summary>
public sealed record YscInterpolation(
    bool Enabled,
    YscProfile Profile,
    TimeSpan? Window,
    TimeSpan? MaxWindow,
    TimeSpan? MeasuredInterval);

/// <summary>what <see cref="Ysc.SetInterpolation"/> sends, the windows in whole milliseconds up to 60 s</summary>
public sealed record YscInterpolationSettings(bool Enabled, YscProfile Profile, TimeSpan Window, TimeSpan MaxWindow);

/// <summary>the serial a YSC gives the mouse behind it, each null when the box didn't send it</summary>
public sealed record YscSerial(string? Custom, string? Physical, string? Active);

/// <summary>what only a MAKCU has. it keeps the device alive, and each call checks what the box can do right
/// now</summary>
public sealed class Makcu
{
    readonly Device device;

    internal Makcu(Device device) => this.device = device;

    static INative Native => Anybox.Native.Api;

    // made by bindings/gen from its call table (makcu_methods), change the table and run cargo run -p anybox-gen
    /// <summary>how long the box spreads a move, read fresh from the box. it's global to the box and every program
    /// on it shares it</summary>
    public Interpolation Interpolation()
    {
        Library.Check(Native.anybox_makcu_interpolation(device.Handle, out var raw));
        return ExtensionValues.Read(raw);
    }

    /// <summary>sets it and reads it back, a box that didn't take it throws <see cref="RejectedException"/>. it
    /// changes how moves land for every program on the box, and anybox leaves it set when you close</summary>
    public void SetInterpolation(Interpolation value)
    {
        var rawValue = ExtensionValues.Raw(value);
        Library.Check(Native.anybox_makcu_set_interpolation(device.Handle, rawValue));
    }

    /// <summary>whether injected motion is still queued or being spread</summary>
    public bool Moving()
    {
        Library.Check(Native.anybox_makcu_moving(device.Handle, out var raw));
        return raw != 0;
    }

    /// <summary>how long the box waits with nothing coming in before it lets go of injected mouse buttons, null when
    /// it's off</summary>
    public TimeSpan? Autorelease()
    {
        Library.Check(Native.anybox_makcu_autorelease(device.Handle, out var raw));
        return ExtensionValues.MsOrOff(raw);
    }

    /// <summary>turns autorelease on for 1 ms to 60 s (whole milliseconds), or off with null. anybox asks the box
    /// something about every 250 ms, so with more than that it only fires once your app dies or hangs, and a button
    /// it held gets let go instead of staying down. the box shares the timer with every program on it and forgets it
    /// when it loses power, so anybox sends it again after a reconnect. needs V4.108</summary>
    public void SetAutorelease(TimeSpan? after)
    {
        var rawAfter = ExtensionValues.MsOrOff(after, "autorelease");
        Library.Check(Native.anybox_makcu_set_autorelease(device.Handle, rawAfter));
    }

    // the end of what bindings/gen makes (makcu_methods)
}

/// <summary>what only a YSC8K has. it keeps the device alive, and each call checks what the box can do right
/// now</summary>
public sealed class Ysc
{
    readonly Device device;

    internal Ysc(Device device) => this.device = device;

    static INative Native => Anybox.Native.Api;

    // made by bindings/gen from its call table (ysc_methods), change the table and run cargo run -p anybox-gen
    /// <summary>how the box spreads moves. it's on by default and kept on the box, so another app may have changed
    /// it</summary>
    public YscInterpolation Interpolation()
    {
        Library.Check(Native.anybox_ysc_interpolation(device.Handle, out var raw));
        return ExtensionValues.Read(raw);
    }

    /// <summary>sets it and reads it back, a box that didn't take it throws <see cref="RejectedException"/>. the box
    /// keeps it, anybox leaves it set when you close</summary>
    public void SetInterpolation(YscInterpolationSettings settings)
    {
        var rawSettings = ExtensionValues.Raw(settings);
        Library.Check(Native.anybox_ysc_set_interpolation(device.Handle, rawSettings));
    }

    /// <summary>puts it back to the box's default and gives you what that is</summary>
    public YscInterpolation ResetInterpolation()
    {
        Library.Check(Native.anybox_ysc_reset_interpolation(device.Handle, out var raw));
        return ExtensionValues.Read(raw);
    }

    /// <summary>the serial the box gives the mouse behind it</summary>
    public YscSerial Serial()
    {
        Library.Check(Native.anybox_ysc_serial(device.Handle, out var raw));
        return ExtensionValues.Read(raw);
    }

    // the end of what bindings/gen makes (ysc_methods)
}

/// <summary>what only a Ferrum has. it keeps the device alive, and each call checks what the box can do right
/// now</summary>
public sealed class Ferrum
{
    readonly Device device;

    internal Ferrum(Device device) => this.device = device;

    static INative Native => Anybox.Native.Api;

    // made by bindings/gen from its call table (ferrum_methods), change the table and run cargo run -p anybox-gen
    /// <summary>the mouse's own motion summed over the last <paramref name="window"/> (whole milliseconds, up to a
    /// second), and what software sent too with <paramref name="withSoftware"/>. y is positive down like
    /// <see cref="Device.MoveBy"/> per the vendor's first example, its second says the opposite and nobody checked
    /// it on a box yet. what a 0 window does isn't documented</summary>
    public (int X, int Y) CatchXy(TimeSpan window, bool withSoftware = false)
    {
        var rawWindow = ExtensionValues.Millis(window, "the window");
        Library.Check(Native.anybox_ferrum_catch_xy(
            device.Handle,
            rawWindow,
            withSoftware ? (byte)1 : (byte)0,
            out var x,
            out var y));
        return (x, y);
    }

    // the end of what bindings/gen makes (ferrum_methods)
}

// how each language's values turn into the C structs and back, for the made methods above
internal static class ExtensionValues
{
    internal static AnyboxMakcuInterpolation Raw(Interpolation value) => new()
    {
        automatic = value.Automatic ? (byte)1 : (byte)0,
        percent = value.Automatic ? (byte)0 : value.Percentage,
    };

    internal static Interpolation Read(AnyboxMakcuInterpolation raw) =>
        raw.automatic != 0 ? Interpolation.Auto : Interpolation.Percent(raw.percent);

    internal static AnyboxYscInterpolationSettings Raw(YscInterpolationSettings settings) => new()
    {
        enabled = settings.Enabled ? (byte)1 : (byte)0,
        profile = (uint)settings.Profile,
        window_ms = Anybox.Read.Millis(settings.Window, "the window"),
        max_window_ms = Anybox.Read.Millis(settings.MaxWindow, "the max window"),
    };

    internal static YscInterpolation Read(AnyboxYscInterpolation raw) => new(
        raw.enabled != 0,
        (YscProfile)raw.profile,
        raw.has_window != 0 ? TimeSpan.FromMilliseconds(raw.window_ms) : null,
        raw.has_max_window != 0 ? TimeSpan.FromMilliseconds(raw.max_window_ms) : null,
        raw.has_measured_interval != 0 ? TimeSpan.FromMicroseconds((long)raw.measured_interval_us) : null);

    internal static unsafe YscSerial Read(YscSerialHandle serial)
    {
        using (serial)
        {
            return new YscSerial(
                Library.Text(Imports.anybox_ysc_serial_custom(serial)),
                Library.Text(Imports.anybox_ysc_serial_physical(serial)),
                Library.Text(Imports.anybox_ysc_serial_active(serial)));
        }
    }

    internal static uint Millis(TimeSpan value, string what) => Anybox.Read.Millis(value, what);

    // 0 is off on the C side, so a zero asked for has to fail here like it does in Rust
    internal static uint MsOrOff(TimeSpan? value, string what)
    {
        if (value is not { } after)
        {
            return 0;
        }
        var ms = Anybox.Read.Millis(after, what);
        return ms != 0 ? ms : throw new ArgumentOutOfRangeException(what, $"{what} can't be 0, use null to turn it off");
    }

    internal static TimeSpan? MsOrOff(uint ms) => ms == 0 ? null : TimeSpan.FromMilliseconds(ms);
}

[StructLayout(LayoutKind.Sequential)]
internal struct AnyboxMakcuInterpolation
{
    public byte automatic;
    public byte percent;
}

[StructLayout(LayoutKind.Sequential)]
internal struct AnyboxYscInterpolation
{
    public byte enabled;
    public uint profile;
    public byte has_window;
    public uint window_ms;
    public byte has_max_window;
    public uint max_window_ms;
    public byte has_measured_interval;
    public ulong measured_interval_us;
}

[StructLayout(LayoutKind.Sequential)]
internal struct AnyboxYscInterpolationSettings
{
    public byte enabled;
    public uint profile;
    public uint window_ms;
    public uint max_window_ms;
}

internal sealed class YscSerialHandle : AnyboxHandle
{
    protected override bool ReleaseHandle()
    {
        Imports.anybox_ysc_serial_free(handle);
        return true;
    }
}

internal static unsafe partial class Imports
{
    // made by bindings/gen from its call table (makcu_imports), change the table and run cargo run -p anybox-gen
    [LibraryImport(Library)]
    internal static partial uint anybox_makcu_interpolation(DeviceHandle device, out AnyboxMakcuInterpolation @out);

    [LibraryImport(Library)]
    internal static partial uint anybox_makcu_set_interpolation(DeviceHandle device, in AnyboxMakcuInterpolation value);

    [LibraryImport(Library)]
    internal static partial uint anybox_makcu_moving(DeviceHandle device, out byte @out);

    [LibraryImport(Library)]
    internal static partial uint anybox_makcu_autorelease(DeviceHandle device, out uint ms_out);

    [LibraryImport(Library)]
    internal static partial uint anybox_makcu_set_autorelease(DeviceHandle device, uint after_ms);

    // the end of what bindings/gen makes (makcu_imports)

    // made by bindings/gen from its call table (ysc_imports), change the table and run cargo run -p anybox-gen
    [LibraryImport(Library)]
    internal static partial uint anybox_ysc_interpolation(DeviceHandle device, out AnyboxYscInterpolation @out);

    [LibraryImport(Library)]
    internal static partial uint anybox_ysc_set_interpolation(DeviceHandle device, in AnyboxYscInterpolationSettings settings);

    [LibraryImport(Library)]
    internal static partial uint anybox_ysc_reset_interpolation(DeviceHandle device, out AnyboxYscInterpolation @out);

    [LibraryImport(Library)]
    internal static partial uint anybox_ysc_serial(DeviceHandle device, out YscSerialHandle @out);

    // the end of what bindings/gen makes (ysc_imports)

    // made by bindings/gen from its call table (ferrum_imports), change the table and run cargo run -p anybox-gen
    [LibraryImport(Library)]
    internal static partial uint anybox_ferrum_catch_xy(DeviceHandle device, uint window_ms, byte with_software, out int x, out int y);

    // the end of what bindings/gen makes (ferrum_imports)

    [LibraryImport(Library)]
    internal static partial byte* anybox_ysc_serial_custom(YscSerialHandle serial);

    [LibraryImport(Library)]
    internal static partial byte* anybox_ysc_serial_physical(YscSerialHandle serial);

    [LibraryImport(Library)]
    internal static partial byte* anybox_ysc_serial_active(YscSerialHandle serial);

    [LibraryImport(Library)]
    internal static partial void anybox_ysc_serial_free(nint serial);

    [LibraryImport(Library)]
    internal static partial uint anybox_mock_makcu_open(in AnyboxClickTiming click_timing, out DeviceHandle @out);

    [LibraryImport(Library)]
    internal static partial uint anybox_mock_ysc_open(in AnyboxClickTiming click_timing, out DeviceHandle @out);

    [LibraryImport(Library)]
    internal static partial uint anybox_mock_ferrum_open(in AnyboxClickTiming click_timing, out DeviceHandle @out);
}

internal partial interface INative
{
    // made by bindings/gen from its call table (makcu_forwarders), change the table and run cargo run -p anybox-gen
    uint anybox_makcu_interpolation(DeviceHandle device, out AnyboxMakcuInterpolation @out) => Imports.anybox_makcu_interpolation(device, out @out);
    uint anybox_makcu_set_interpolation(DeviceHandle device, in AnyboxMakcuInterpolation value) => Imports.anybox_makcu_set_interpolation(device, in value);
    uint anybox_makcu_moving(DeviceHandle device, out byte @out) => Imports.anybox_makcu_moving(device, out @out);
    uint anybox_makcu_autorelease(DeviceHandle device, out uint ms_out) => Imports.anybox_makcu_autorelease(device, out ms_out);
    uint anybox_makcu_set_autorelease(DeviceHandle device, uint after_ms) => Imports.anybox_makcu_set_autorelease(device, after_ms);
    // the end of what bindings/gen makes (makcu_forwarders)
    // made by bindings/gen from its call table (ysc_forwarders), change the table and run cargo run -p anybox-gen
    uint anybox_ysc_interpolation(DeviceHandle device, out AnyboxYscInterpolation @out) => Imports.anybox_ysc_interpolation(device, out @out);
    uint anybox_ysc_set_interpolation(DeviceHandle device, in AnyboxYscInterpolationSettings settings) => Imports.anybox_ysc_set_interpolation(device, in settings);
    uint anybox_ysc_reset_interpolation(DeviceHandle device, out AnyboxYscInterpolation @out) => Imports.anybox_ysc_reset_interpolation(device, out @out);
    uint anybox_ysc_serial(DeviceHandle device, out YscSerialHandle @out) => Imports.anybox_ysc_serial(device, out @out);
    // the end of what bindings/gen makes (ysc_forwarders)
    // made by bindings/gen from its call table (ferrum_forwarders), change the table and run cargo run -p anybox-gen
    uint anybox_ferrum_catch_xy(DeviceHandle device, uint window_ms, byte with_software, out int x, out int y) => Imports.anybox_ferrum_catch_xy(device, window_ms, with_software, out x, out y);
    // the end of what bindings/gen makes (ferrum_forwarders)
}
