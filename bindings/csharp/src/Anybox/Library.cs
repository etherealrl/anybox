using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;

namespace Anybox;

/// <summary>the native anybox library this package runs on</summary>
public static unsafe class Library
{
    /// <summary>the C ABI this package speaks, a library with another one gets refused before any call</summary>
    public const uint AbiVersion = 3;

    internal const uint Ok = 0;
    internal const uint NotFound = 1;
    internal const uint Closed = 2;
    internal const uint NoBoxCan = 5;
    internal const uint Unsupported = 11;
    internal const uint RecvTimeout = 20;
    internal const uint RecvClosed = 21;

    static readonly string FileName =
        OperatingSystem.IsWindows() ? "anybox.dll" : OperatingSystem.IsMacOS() ? "libanybox.dylib" : "libanybox.so";
    static readonly UTF8Encoding StrictUtf8 = new(encoderShouldEmitUTF8Identifier: false, throwOnInvalidBytes: true);
    static readonly Lock Gate = new();
    static bool resolving;
    static volatile bool ready;
    static string loaded = FileName;

    /// <summary>the anybox version the library was built from</summary>
    public static string Version
    {
        get
        {
            Ensure();
            return Marshal.PtrToStringUTF8((nint)Imports.anybox_version()) ?? "";
        }
    }

    /// <summary>whether the library was built with the mock, the one in the package never is</summary>
    public static bool HasMock
    {
        get
        {
            Ensure();
            return Imports.anybox_has_mock() != 0;
        }
    }

    // every public way in calls this first. it isn't a static constructor, because a refusal there would reach
    // the caller as a TypeInitializationException and stick to the type for the rest of the process
    internal static void Ensure()
    {
        if (!ready)
        {
            Load(AbiVersion);
            ready = true;
        }
    }

    // takes the ABI it wants, so the tests can see a mismatch refused on the library they already have
    internal static void Load(uint wanted)
    {
        lock (Gate)
        {
            if (!resolving)
            {
                NativeLibrary.SetDllImportResolver(typeof(Library).Assembly, Resolve);
                resolving = true;
            }
            uint found;
            try
            {
                found = Imports.anybox_abi_version();
            }
            catch (EntryPointNotFoundException)
            {
                throw new AnyboxException($"{loaded} isn't an anybox library, point ANYBOX_LIB at the right one");
            }
            if (found != wanted)
            {
                throw new AnyboxException(
                    $"{loaded} speaks C ABI {found} and this package needs {wanted}, so install the Anybox package " +
                    "that matches it, or build the library from the same release as this package");
            }
        }
    }

    static nint Resolve(string name, Assembly assembly, DllImportSearchPath? searchPath)
    {
        if (Environment.GetEnvironmentVariable("ANYBOX_LIB") is { Length: > 0 } path)
        {
            if (!File.Exists(path))
            {
                throw new AnyboxException($"ANYBOX_LIB points at {path}, which doesn't exist");
            }
            loaded = path;
            return NativeLibrary.Load(path);
        }
        if (NativeLibrary.TryLoad(name, assembly, searchPath, out var handle))
        {
            return handle;
        }
        throw new AnyboxException(
            $"couldn't find {FileName}, it ships inside the Anybox package, so reinstall it. if you built it " +
            "yourself, point ANYBOX_LIB at it");
    }

    internal static void Check(uint status)
    {
        if (status != Ok)
        {
            throw Error(status);
        }
    }

    // the last error is thread local on the C side, so this has to run right after the call that failed, on its
    // thread. the order is fixed, the flat fields first, then the lists, then the exception, so a failure on the way
    // can't mix two errors
    internal static AnyboxException Error(uint status)
    {
        var message = Text(Imports.anybox_last_error_message()) is { Length: > 0 } text
            ? text
            : $"anybox failed with status {status}";
        Imports.anybox_last_error_detail(out var detail);
        var fields = detail.kind == status ? Read.Fields(detail) : null;
        IReadOnlyList<string>? tried = null;
        IReadOnlyList<Incapable>? boxes = null;
        if (status == NotFound)
        {
            Check(Imports.anybox_last_error_tried(out var list));
            using (list)
            {
                tried = Read.Strings(list);
            }
        }
        if (status == NoBoxCan)
        {
            Check(Imports.anybox_last_error_incapable(out var list));
            using (list)
            {
                boxes = Read.Incapable(list);
            }
        }
        return AnyboxException.For(status, message, fields, tried, boxes);
    }

    internal static string? Text(byte* value) => Marshal.PtrToStringUTF8((nint)value);

    internal static byte[] KeyBytes(ReadOnlySpan<Key> keys) => MemoryMarshal.AsBytes(keys).ToArray();

    // NUL terminated for a `const char *`, and a NUL inside would cut it short on the C side
    internal static byte[] Utf8(string value)
    {
        if (value.Contains('\0'))
        {
            throw new ArgumentException("the text can't have a NUL character in it", nameof(value));
        }
        var bytes = new byte[StrictUtf8.GetByteCount(value) + 1];
        StrictUtf8.GetBytes(value, bytes);
        return bytes;
    }
}
