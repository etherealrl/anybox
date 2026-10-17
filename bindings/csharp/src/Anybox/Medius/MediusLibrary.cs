using System.Runtime.InteropServices;
using System.Text;

namespace Anybox.Medius;

/// <summary>the medius side of the library: its version, its C ABI and the defaults the medius crate uses</summary>
public static unsafe class MediusLibrary
{
    /// <summary>the medius C ABI this package speaks, a library with another one gets refused before any call</summary>
    public const uint AbiVersion = Constants.AbiVersion;

    static readonly Lock Gate = new();
    static volatile bool ready;

    /// <summary>the medius crate's version inside the library</summary>
    public static string Version
    {
        get
        {
            Ensure();
            return Marshal.PtrToStringUTF8((nint)MediusImports.medius_version_string()) ?? "";
        }
    }

    /// <summary>whether the library was built with the medius mock, the one in the package never is</summary>
    public static bool HasMock => Library.HasMock;

    public static uint DefaultQueryTimeoutMs
    {
        get
        {
            Ensure();
            return MediusImports.medius_default_query_timeout_ms();
        }
    }

    public static uint DefaultTransferTimeoutMs
    {
        get
        {
            Ensure();
            return MediusImports.medius_default_transfer_timeout_ms();
        }
    }

    public static uint DefaultKeepaliveCadenceMs
    {
        get
        {
            Ensure();
            return MediusImports.medius_default_keepalive_cadence_ms();
        }
    }

    // every public way into Anybox.Medius calls this first. anybox's own check goes first, it's what finds the
    // library at all
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
        Library.Ensure();
        lock (Gate)
        {
            var found = MediusImports.medius_abi_version();
            if (found != wanted)
            {
                throw new AnyboxException(
                    $"the library's medius side speaks medius C ABI {found} and this package needs {wanted}, so " +
                    "install the Anybox package that matches it, or build the library from the same release as this " +
                    "package");
            }
        }
    }

    // the medius mock functions are only in a library built with the mock, and calling one that isn't there
    // would only fail at the call
    internal static void RequireMock()
    {
        Ensure();
        if (!Library.HasMock)
        {
            throw new AnyboxException(
                "this anybox library was built without the mock, build anybox-capi with --features mock");
        }
    }

    internal static void Check(Status status)
    {
        if (status != Status.Ok)
        {
            throw Error(status);
        }
    }

    // the text is thread local on the C side, so this has to run right after the call that failed, on its thread
    internal static MediusException Error(Status status)
    {
        const int First = 256;
        var buffer = stackalloc byte[First];
        var full = MediusImports.medius_last_error_message(buffer, First);
        string text;
        if (full < First)
        {
            text = Text(new ReadOnlySpan<byte>(buffer, First));
        }
        else
        {
            var whole = new byte[full + 1];
            fixed (byte* start = whole)
            {
                MediusImports.medius_last_error_message(start, (nuint)whole.Length);
            }
            text = Text(whole);
        }
        MediusUpdateError update;
        var detailed = MediusImports.medius_last_error_update(&update) != 0;
        return MediusException.For(status, text, MediusImports.medius_last_error_proto_ver(), detailed ? update : null);
    }

    // a C string in a fixed size buffer, up to its first NUL. what the box sends isn't checked UTF-8, so a bad
    // byte becomes U+FFFD like the Python package does
    internal static string Text(ReadOnlySpan<byte> buffer)
    {
        var end = buffer.IndexOf((byte)0);
        return Encoding.UTF8.GetString(end < 0 ? buffer : buffer[..end]);
    }

    // a string for a fixed size char array, cut to fit with room for the NUL, like the Python package
    internal static void Text(string value, byte* buffer, int capacity)
    {
        var bytes = Library.Utf8(value);
        var length = Math.Min(bytes.Length - 1, capacity - 1);
        new ReadOnlySpan<byte>(bytes, 0, length).CopyTo(new Span<byte>(buffer, capacity));
        buffer[length] = 0;
    }

    // a value going in has to be one the enum names, otherwise the library reads a stray byte as whichever member
    // its low bits happen to be
    internal static byte In<T>(T value, string what) where T : struct, Enum
    {
        if (!Enum.IsDefined(value))
        {
            throw new ArgumentOutOfRangeException(what, $"{value} isn't a {typeof(T).Name} this package knows");
        }
        return Convert.ToByte(value);
    }

    // a value coming back the package has no name for means it's older than the library
    internal static T Out<T>(T value, string what) where T : struct, Enum => Enum.IsDefined(value)
        ? value
        : throw new AnyboxException(
            $"the library sent {what} this package doesn't know ({Convert.ToInt64(value)}), so update the Anybox package");

    internal static byte Flag(bool value) => value ? (byte)1 : (byte)0;

    // bytes into a fixed array, refused when they don't fit
    internal static ushort Copy(ReadOnlySpan<byte> source, byte* destination, int capacity, string what)
    {
        if (source.Length > capacity)
        {
            throw new ArgumentException($"{what} is {source.Length} bytes, over the {capacity} the library takes", what);
        }
        source.CopyTo(new Span<byte>(destination, capacity));
        return (ushort)source.Length;
    }

    // bytes into an array that holds exactly that many, like a MAC, refused when they're any other length
    internal static void Exact(ReadOnlySpan<byte> source, byte* destination, int length, string what)
    {
        if (source.Length != length)
        {
            throw new ArgumentException($"{what} is {source.Length} bytes, it has to be {length}", what);
        }
        source.CopyTo(new Span<byte>(destination, length));
    }

    // the entries a count says are there, never more than the array holds
    internal static T[] ListOf<TRaw, T>(ReadOnlySpan<TRaw> raw, int count, Func<TRaw, T> from)
    {
        var entries = new T[Math.Min(count, raw.Length)];
        for (var index = 0; index < entries.Length; index++)
        {
            entries[index] = from(raw[index]);
        }
        return entries;
    }

    // a list into an array, refused when it doesn't fit, like C++ and unlike cutting it without a word
    internal static int Fill<T, TRaw>(IReadOnlyList<T> items, Span<TRaw> raw, string what, Func<T, TRaw> to)
    {
        if (items.Count > raw.Length)
        {
            throw new ArgumentException($"{what} has {items.Count} entries, over the {raw.Length} the library takes", what);
        }
        for (var index = 0; index < items.Count; index++)
        {
            raw[index] = to(items[index]);
        }
        return items.Count;
    }

    // a value going into an enum field has to be one the enum names, like In for the byte fields
    internal static T Named<T>(T value, string what) where T : struct, Enum => Enum.IsDefined(value)
        ? value
        : throw new ArgumentOutOfRangeException(what, $"{value} isn't a {typeof(T).Name} this package knows");

    // a match or mask goes over with its whole length and as many bytes as the array holds, so one too long comes
    // back as the library's own refusal, like the Python package
    internal static ushort Match(ReadOnlySpan<byte> source, byte* destination, int capacity, string what)
    {
        if (source.Length > ushort.MaxValue)
        {
            throw new ArgumentException($"{what} is {source.Length} bytes, over what the library can be told", what);
        }
        source[..Math.Min(source.Length, capacity)].CopyTo(new Span<byte>(destination, capacity));
        return (ushort)source.Length;
    }

    // records holding bytes compare what's in them, like the Python package's dataclasses, so a trigger read
    // back equals the one that was bound
    internal static bool Same(ReadOnlyMemory<byte> left, ReadOnlyMemory<byte> right) => left.Span.SequenceEqual(right.Span);

    internal static int Hash(ReadOnlyMemory<byte> bytes)
    {
        var hash = new HashCode();
        hash.AddBytes(bytes.Span);
        return hash.ToHashCode();
    }

    // and the ones holding lists compare their entries, like Rust's PartialEq
    internal static bool Same<T>(IReadOnlyList<T> left, IReadOnlyList<T> right) => left.SequenceEqual(right);

    internal static int Hash<T>(IReadOnlyList<T> entries)
    {
        var hash = new HashCode();
        foreach (var entry in entries)
        {
            hash.Add(entry);
        }
        return hash.ToHashCode();
    }

    internal static byte[] Bytes(byte* source, int length, int capacity) =>
        new ReadOnlySpan<byte>(source, Math.Min(length, capacity)).ToArray();
}
