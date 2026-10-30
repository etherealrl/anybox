using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;

[assembly: DisableRuntimeMarshalling]

namespace Anybox;

// everything here mirrors bindings/c/include/anybox.h, and the tests hold every name, width and offset to it. a C
// bool is a byte everywhere because it's one byte in C and the runtime marshalling is off

[StructLayout(LayoutKind.Sequential)]
internal struct AnyboxClickTiming
{
    public uint kind;
    public ulong fixed_us;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct AnyboxConnectOptions
{
    public byte* port;
    public uint family;
    public uint ferrum_api;
    public uint* can;
    public nuint can_len;
    public uint id_kind;
    public fixed byte id_mac[6];
    public byte* id_serial;
    public AnyboxClickTiming click_timing;
}

[StructLayout(LayoutKind.Sequential)]
internal struct AnyboxClick
{
    public ulong hold_us;
    public byte has_hold;
    public byte native_only;
}

[StructLayout(LayoutKind.Sequential)]
internal struct AnyboxEvent
{
    public uint kind;
    public uint button;
    public byte key;
    public byte pressed;
    public ushort media;
    public int dx;
    public int dy;
    public int wheel;
    public int pan;
    public uint source;
    public uint action;
    public ulong count;
    public ulong since_open_ns;
    public ulong received_since_open_ns;
    public long received_qpc;
    public byte has_received_qpc;
    public byte has_box_time;
    public uint box_clock;
    public uint box_time_us;
    public ulong arrived_ns;
    public ulong received_ns;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct AnyboxInfo
{
    public byte* port;
    public uint family;
    public byte* model;
    public uint firmware_kind;
    public ushort firmware_major;
    public ushort firmware_minor;
    public ushort firmware_patch;
    public byte firmware_protocol;
    public uint firmware_build;
    public byte* firmware_text;
    public uint id_kind;
    public fixed byte id_mac[6];
    public byte* id_serial;
    public uint ferrum_api;
    public byte has_peripheral;
    public ushort peripheral_vid;
    public ushort peripheral_pid;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct AnyboxCapability
{
    public uint execution;
    public byte* reason;
    public uint quirk_count;
}

[StructLayout(LayoutKind.Sequential)]
internal struct AnyboxQuirk
{
    public uint kind;
    public ulong min_us;
    public ulong max_us;
    public uint count;
    public AnyboxClickTiming click_timing;
}

[StructLayout(LayoutKind.Sequential)]
internal unsafe struct AnyboxErrorDetail
{
    public uint kind;
    public byte has_action;
    public uint action;
    public uint family;
    public uint expected_family;
    public uint found_family;
    public byte* port;
    public byte* text;
    public uint expected_id_kind;
    public fixed byte expected_id_mac[6];
    public byte* expected_id_serial;
    public uint found_id_kind;
    public fixed byte found_id_mac[6];
    public byte* found_id_serial;
    public uint found_firmware_kind;
    public ushort found_firmware_major;
    public ushort found_firmware_minor;
    public ushort found_firmware_patch;
    public byte found_firmware_protocol;
    public uint found_firmware_build;
    public byte* found_firmware_text;
    public uint needed_firmware_kind;
    public ushort needed_firmware_major;
    public ushort needed_firmware_minor;
    public ushort needed_firmware_patch;
    public byte needed_firmware_protocol;
    public uint needed_firmware_build;
    public byte* needed_firmware_text;
    public uint expected_ferrum_api;
    public uint found_ferrum_api;
    public uint io_kind;
    public byte has_os_error;
    public int os_error;
}

// the runtime counts a SafeHandle up for every call it goes into and only frees it once none is left, which is
// exactly the "never free while another call is still going" the header asks for
internal abstract class AnyboxHandle() : SafeHandle(0, ownsHandle: true)
{
    public override bool IsInvalid => handle == 0;
}

internal sealed class DeviceHandle : AnyboxHandle
{
    protected override bool ReleaseHandle()
    {
        Imports.anybox_device_free(handle);
        return true;
    }
}

internal sealed class EventsHandle : AnyboxHandle
{
    protected override bool ReleaseHandle()
    {
        Imports.anybox_events_free(handle);
        return true;
    }
}

internal sealed class MockHandle : AnyboxHandle
{
    protected override bool ReleaseHandle()
    {
        Imports.anybox_mock_free(handle);
        return true;
    }
}

internal sealed class SnapshotHandle : AnyboxHandle
{
    protected override bool ReleaseHandle()
    {
        Imports.anybox_snapshot_free(handle);
        return true;
    }
}

internal sealed class ScanListHandle : AnyboxHandle
{
    protected override bool ReleaseHandle()
    {
        Imports.anybox_scan_list_free(handle);
        return true;
    }
}

internal sealed class StringListHandle : AnyboxHandle
{
    protected override bool ReleaseHandle()
    {
        Imports.anybox_string_list_free(handle);
        return true;
    }
}

// the names are the C ones, so the tests can hold each one to its declaration in the header. the frees take the
// raw pointer, because they run from ReleaseHandle, when the SafeHandle itself can't be passed any more
internal static unsafe partial class Imports
{
    const string Library = "anybox";

    [LibraryImport(Library)]
    internal static partial uint anybox_abi_version();

    [LibraryImport(Library)]
    internal static partial byte* anybox_version();

    [LibraryImport(Library)]
    internal static partial byte anybox_has_mock();

    [LibraryImport(Library)]
    internal static partial ulong anybox_monotonic_ns();

    [LibraryImport(Library)]
    internal static partial byte* anybox_last_error_message();

    [LibraryImport(Library)]
    internal static partial uint anybox_last_error_detail(out AnyboxErrorDetail @out);

    [LibraryImport(Library)]
    internal static partial uint anybox_last_error_tried(out StringListHandle @out);

    [LibraryImport(Library)]
    internal static partial uint anybox_last_error_incapable(out ScanListHandle @out);

    [LibraryImport(Library)]
    internal static partial uint anybox_open(in AnyboxConnectOptions options, out DeviceHandle @out);

    [LibraryImport(Library)]
    internal static partial uint anybox_scan(in AnyboxConnectOptions options, out ScanListHandle @out);

    [LibraryImport(Library)]
    internal static partial nuint anybox_scan_list_count(ScanListHandle list);

    [LibraryImport(Library)]
    internal static partial byte* anybox_scan_list_port(ScanListHandle list, nuint index);

    [LibraryImport(Library)]
    internal static partial uint anybox_scan_list_result(ScanListHandle list, nuint index, out SnapshotHandle @out);

    [LibraryImport(Library)]
    internal static partial uint anybox_scan_list_error_detail(ScanListHandle list, nuint index, out AnyboxErrorDetail @out);

    [LibraryImport(Library)]
    internal static partial uint anybox_scan_list_missing(ScanListHandle list, nuint index, uint* @out, nuint cap, out nuint len);

    [LibraryImport(Library)]
    internal static partial void anybox_scan_list_free(nint list);

    [LibraryImport(Library)]
    internal static partial uint anybox_candidates(in AnyboxConnectOptions options, out StringListHandle @out);

    [LibraryImport(Library)]
    internal static partial nuint anybox_string_list_count(StringListHandle list);

    [LibraryImport(Library)]
    internal static partial byte* anybox_string_list_get(StringListHandle list, nuint index);

    [LibraryImport(Library)]
    internal static partial void anybox_string_list_free(nint list);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_snapshot(DeviceHandle device, out SnapshotHandle @out);

    // made by bindings/gen from its call table (imports), change the table and run cargo run -p anybox-gen
    [LibraryImport(Library)]
    internal static partial uint anybox_device_move_by(DeviceHandle device, short dx, short dy);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_move_now(DeviceHandle device, short dx, short dy);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_press(DeviceHandle device, uint button);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_release(DeviceHandle device, uint button);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_click_with(DeviceHandle device, uint button, in AnyboxClick click);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_press_key(DeviceHandle device, byte key);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_release_key(DeviceHandle device, byte key);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_tap_key_with(DeviceHandle device, byte key, in AnyboxClick tap);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_type_text(DeviceHandle device, byte[] text);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_press_keys(DeviceHandle device, byte[] keys, nuint len);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_release_keys(DeviceHandle device, byte[] keys, nuint len);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_tap_keys(DeviceHandle device, byte[] keys, nuint len);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_press_media(DeviceHandle device, ushort key);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_release_media(DeviceHandle device, ushort key);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_tap_media_with(DeviceHandle device, ushort key, in AnyboxClick tap);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_lock_button(DeviceHandle device, uint button);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_unlock_button(DeviceHandle device, uint button);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_force_release_key(DeviceHandle device, byte key);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_restore_key(DeviceHandle device, byte key);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_lock_key(DeviceHandle device, byte key);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_unlock_key(DeviceHandle device, byte key);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_lock_axis(DeviceHandle device, uint axis, uint direction);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_unlock_axis(DeviceHandle device, uint axis, uint direction);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_scale_axis(DeviceHandle device, uint axis, uint direction, short percent);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_wheel(DeviceHandle device, short notches);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_pan(DeviceHandle device, short notches);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_discard_pending_moves(DeviceHandle device);

    // the end of what bindings/gen makes (imports)

    [LibraryImport(Library)]
    internal static partial uint anybox_device_events(DeviceHandle device, uint kinds, out EventsHandle @out);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_medius(DeviceHandle device, out Medius.MediusDeviceHandle @out);

    [LibraryImport(Library)]
    internal static partial uint anybox_device_close(DeviceHandle device);

    [LibraryImport(Library)]
    internal static partial void anybox_device_free(nint device);

    [LibraryImport(Library)]
    internal static partial uint anybox_events_recv(EventsHandle events, long timeout_us, out AnyboxEvent @out);

    [LibraryImport(Library)]
    internal static partial uint anybox_events_close(EventsHandle events);

    [LibraryImport(Library)]
    internal static partial void anybox_events_free(nint events);

    [LibraryImport(Library)]
    internal static partial uint anybox_mock_medius_open(in AnyboxClickTiming click_timing, byte keyboard, out MockHandle mock, out DeviceHandle @out);

    [LibraryImport(Library)]
    internal static partial uint anybox_mock_medius_open_manual(in AnyboxClickTiming click_timing, byte keyboard, out MockHandle mock, out DeviceHandle @out);

    [LibraryImport(Library)]
    internal static partial byte anybox_mock_subscribed(MockHandle mock, uint kinds);

    [LibraryImport(Library)]
    internal static partial uint anybox_mock_push_motion(MockHandle mock, byte seq, uint ts_us, short dx, short dy, short wheel, short pan);

    [LibraryImport(Library)]
    internal static partial uint anybox_mock_push_buttons(MockHandle mock, byte seq, uint ts_us, byte pressed, uint held);

    [LibraryImport(Library)]
    internal static partial uint anybox_mock_push_keys(MockHandle mock, byte seq, uint ts_us, byte pressed, byte* keys, nuint len);

    [LibraryImport(Library)]
    internal static partial uint anybox_mock_push_media(MockHandle mock, byte seq, uint ts_us, byte pressed, ushort* media, nuint len);

    [LibraryImport(Library)]
    internal static partial uint anybox_mock_link_lost(MockHandle mock);

    [LibraryImport(Library)]
    internal static partial uint anybox_mock_restart(MockHandle mock);

    [LibraryImport(Library)]
    internal static partial uint anybox_mock_tick(MockHandle mock);

    [LibraryImport(Library)]
    internal static partial nuint anybox_mock_recorded(MockHandle mock);

    [LibraryImport(Library)]
    internal static partial uint anybox_mock_recorded_frame(MockHandle mock, nuint index, byte* type, byte* payload, nuint cap, out nuint len);

    [LibraryImport(Library)]
    internal static partial uint anybox_mock_clear_recorded(MockHandle mock);

    [LibraryImport(Library)]
    internal static partial uint anybox_mock_example_events(DeviceHandle device);

    [LibraryImport(Library)]
    internal static partial uint anybox_mock_fail(uint status);

    [LibraryImport(Library)]
    internal static partial void anybox_mock_free(nint mock);

    [LibraryImport(Library)]
    internal static partial uint anybox_snapshot_info(SnapshotHandle snapshot, out AnyboxInfo @out);

    [LibraryImport(Library)]
    internal static partial uint anybox_snapshot_capability(SnapshotHandle snapshot, uint action, out AnyboxCapability @out);

    [LibraryImport(Library)]
    internal static partial uint anybox_snapshot_quirk(SnapshotHandle snapshot, uint action, uint index, out AnyboxQuirk @out);

    [LibraryImport(Library)]
    internal static partial uint anybox_snapshot_buttons(SnapshotHandle snapshot, uint* @out, nuint cap, out nuint len);

    [LibraryImport(Library)]
    internal static partial uint anybox_snapshot_axes(SnapshotHandle snapshot, uint* @out, nuint cap, out nuint len);

    [LibraryImport(Library)]
    internal static partial void anybox_snapshot_free(nint snapshot);
}

// every call a Device makes goes through here. it's an interface with one real implementation, which the repo
// otherwise avoids, but a LibraryImport can't be swapped out in a test the way a ctypes function can, and the
// tests need to see what each call hands to C
internal partial interface INative
{
    uint anybox_device_snapshot(DeviceHandle device, out SnapshotHandle @out) => Imports.anybox_device_snapshot(device, out @out);
    // made by bindings/gen from its call table (forwarders), change the table and run cargo run -p anybox-gen
    uint anybox_device_move_by(DeviceHandle device, short dx, short dy) => Imports.anybox_device_move_by(device, dx, dy);
    uint anybox_device_move_now(DeviceHandle device, short dx, short dy) => Imports.anybox_device_move_now(device, dx, dy);
    uint anybox_device_press(DeviceHandle device, uint button) => Imports.anybox_device_press(device, button);
    uint anybox_device_release(DeviceHandle device, uint button) => Imports.anybox_device_release(device, button);
    uint anybox_device_click_with(DeviceHandle device, uint button, in AnyboxClick click) => Imports.anybox_device_click_with(device, button, in click);
    uint anybox_device_press_key(DeviceHandle device, byte key) => Imports.anybox_device_press_key(device, key);
    uint anybox_device_release_key(DeviceHandle device, byte key) => Imports.anybox_device_release_key(device, key);
    uint anybox_device_tap_key_with(DeviceHandle device, byte key, in AnyboxClick tap) => Imports.anybox_device_tap_key_with(device, key, in tap);
    uint anybox_device_type_text(DeviceHandle device, byte[] text) => Imports.anybox_device_type_text(device, text);
    uint anybox_device_press_keys(DeviceHandle device, byte[] keys, nuint len) => Imports.anybox_device_press_keys(device, keys, len);
    uint anybox_device_release_keys(DeviceHandle device, byte[] keys, nuint len) => Imports.anybox_device_release_keys(device, keys, len);
    uint anybox_device_tap_keys(DeviceHandle device, byte[] keys, nuint len) => Imports.anybox_device_tap_keys(device, keys, len);
    uint anybox_device_press_media(DeviceHandle device, ushort key) => Imports.anybox_device_press_media(device, key);
    uint anybox_device_release_media(DeviceHandle device, ushort key) => Imports.anybox_device_release_media(device, key);
    uint anybox_device_tap_media_with(DeviceHandle device, ushort key, in AnyboxClick tap) => Imports.anybox_device_tap_media_with(device, key, in tap);
    uint anybox_device_lock_button(DeviceHandle device, uint button) => Imports.anybox_device_lock_button(device, button);
    uint anybox_device_unlock_button(DeviceHandle device, uint button) => Imports.anybox_device_unlock_button(device, button);
    uint anybox_device_force_release_key(DeviceHandle device, byte key) => Imports.anybox_device_force_release_key(device, key);
    uint anybox_device_restore_key(DeviceHandle device, byte key) => Imports.anybox_device_restore_key(device, key);
    uint anybox_device_lock_key(DeviceHandle device, byte key) => Imports.anybox_device_lock_key(device, key);
    uint anybox_device_unlock_key(DeviceHandle device, byte key) => Imports.anybox_device_unlock_key(device, key);
    uint anybox_device_lock_axis(DeviceHandle device, uint axis, uint direction) => Imports.anybox_device_lock_axis(device, axis, direction);
    uint anybox_device_unlock_axis(DeviceHandle device, uint axis, uint direction) => Imports.anybox_device_unlock_axis(device, axis, direction);
    uint anybox_device_scale_axis(DeviceHandle device, uint axis, uint direction, short percent) => Imports.anybox_device_scale_axis(device, axis, direction, percent);
    uint anybox_device_wheel(DeviceHandle device, short notches) => Imports.anybox_device_wheel(device, notches);
    uint anybox_device_pan(DeviceHandle device, short notches) => Imports.anybox_device_pan(device, notches);
    uint anybox_device_discard_pending_moves(DeviceHandle device) => Imports.anybox_device_discard_pending_moves(device);
    // the end of what bindings/gen makes (forwarders)
    uint anybox_device_events(DeviceHandle device, uint kinds, out EventsHandle @out) => Imports.anybox_device_events(device, kinds, out @out);
    uint anybox_device_medius(DeviceHandle device, out Medius.MediusDeviceHandle @out) => Imports.anybox_device_medius(device, out @out);
    uint anybox_device_close(DeviceHandle device) => Imports.anybox_device_close(device);
}

internal static class Native
{
    internal static INative Api { get; set; } = new Calls();

    sealed class Calls : INative;
}
