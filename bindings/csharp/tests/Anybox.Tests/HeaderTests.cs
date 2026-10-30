using System.Reflection;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text.RegularExpressions;

namespace Anybox.Tests;

// the package against bindings/c/include/anybox.h, which is what it has to match
public sealed class HeaderTests
{
    static readonly string Header = MockLibrary.ReadFile("bindings/c/include/anybox.h");

    static Dictionary<string, ulong> Defines() => Regex.Matches(Header, @"#define (ANYBOX_\w+) (\d+)")
        .ToDictionary(match => match.Groups[1].Value, match => ulong.Parse(match.Groups[2].Value));

    static string Pascal(string name) =>
        string.Concat(name.Split('_').Select(word => word[..1] + word[1..].ToLowerInvariant()));

    [Fact]
    public void TheAbiVersionMatchesTheHeaderAndTheLibrary()
    {
        Assert.Equal((ulong)Library.AbiVersion, Defines()["ANYBOX_ABI_VERSION"]);
        Library.Ensure();
        Assert.Equal(Library.AbiVersion, Imports.anybox_abi_version());
    }

    [Theory]
    [InlineData("ANYBOX_FAMILY_", typeof(Family))]
    [InlineData("ANYBOX_FERRUM_API_", typeof(FerrumApi))]
    [InlineData("ANYBOX_BUTTON_", typeof(Button))]
    [InlineData("ANYBOX_AXIS_", typeof(Axis))]
    [InlineData("ANYBOX_DIRECTION_", typeof(Direction))]
    [InlineData("ANYBOX_ACTION_", typeof(BoxAction))]
    [InlineData("ANYBOX_EXECUTION_", typeof(Execution))]
    [InlineData("ANYBOX_QUIRK_", typeof(QuirkKind))]
    [InlineData("ANYBOX_FIRMWARE_", typeof(FirmwareKind))]
    [InlineData("ANYBOX_DROP_SOURCE_", typeof(DropSource))]
    [InlineData("ANYBOX_CLOCK_", typeof(Clock))]
    [InlineData("ANYBOX_EVENTS_", typeof(EventKinds))]
    [InlineData("ANYBOX_CLICK_TIMING_", typeof(ClickTimingKind))]
    [InlineData("ANYBOX_KEY_", typeof(Key))]
    [InlineData("ANYBOX_EVENT_", typeof(EventKind))]
    [InlineData("ANYBOX_ID_", typeof(IdKind))]
    [InlineData("ANYBOX_MEDIA_", typeof(MediaKey))]
    [InlineData("ANYBOX_IO_KIND_", typeof(IoKind))]
    public void EveryHeaderConstantHasTheSameValueHere(string prefix, Type type)
    {
        // the "nothing set" values are null here, and the extra base is Button.Extra
        string[] skip =
        [
            "ANYBOX_FAMILY_ANY",
            "ANYBOX_FERRUM_API_NONE",
            "ANYBOX_FIRMWARE_NONE",
            "ANYBOX_IO_KIND_NONE",
            "ANYBOX_BUTTON_EXTRA_BASE",
            "ANYBOX_KEY_MODIFIER_FIRST",
            "ANYBOX_KEY_MODIFIER_LAST",
        ];
        var expected = Defines()
            .Where(define => define.Key.StartsWith(prefix) && !skip.Contains(define.Key))
            .ToDictionary(define => Pascal(define.Key[prefix.Length..]), define => define.Value);
        Assert.NotEmpty(expected);
        var actual = Enum.GetNames(type).ToDictionary(name => name, name => Convert.ToUInt64(Enum.Parse(type, name)));
        Assert.Equal(expected.OrderBy(pair => pair.Key), actual.OrderBy(pair => pair.Key));
    }

    [Fact]
    public void TheExtraButtonsStartWhereTheHeaderSays()
    {
        Assert.Equal((ulong)ButtonExtensions.ExtraBase, Defines()["ANYBOX_BUTTON_EXTRA_BASE"]);
        Assert.Equal((Button)256, Button.Extra(0));
        Assert.Equal((Button)511, Button.Extra(255));
    }

    [Fact]
    public void TheHelpersHoldTheHeaderRanges()
    {
        Assert.Equal((ulong)KeyExtensions.ModifierFirst, Defines()["ANYBOX_KEY_MODIFIER_FIRST"]);
        Assert.Equal((ulong)KeyExtensions.ModifierLast, Defines()["ANYBOX_KEY_MODIFIER_LAST"]);
        Assert.Equal(ClockExtensions.WrapUs, Defines()["ANYBOX_BOX_CLOCK_WRAP_US"]);
    }

    [Fact]
    public void EveryStatusHasItsException()
    {
        var statuses = Defines()
            .Where(define => define.Key.StartsWith("ANYBOX_STATUS_"))
            .ToDictionary(define => define.Key["ANYBOX_STATUS_".Length..], define => (uint)define.Value);
        Assert.Equal(0u, statuses["OK"]);
        Assert.Equal(Library.Ok, statuses["OK"]);
        Assert.Equal(Library.Closed, statuses["CLOSED"]);
        Assert.Equal(Library.RecvTimeout, statuses["RECV_TIMEOUT"]);
        Assert.Equal(Library.RecvClosed, statuses["RECV_CLOSED"]);
        foreach (var (name, status) in statuses.Where(pair => pair.Key is not ("OK" or "RECV_TIMEOUT")))
        {
            var error = AnyboxException.For(status, "text");
            Assert.Contains(error.GetType().Name, new[] { $"{Pascal(name)}Exception", $"Box{Pascal(name)}Exception" });
            Assert.Equal(status, error.Status);
            Assert.Equal("text", error.Message);
        }
        Assert.IsType<UnknownException>(AnyboxException.For(99, "text"));
    }

    [Theory]
    [InlineData("bindings/c/tests/layout.txt", "Anybox", 11)]
    // written by a test in the capi crate from the real medius-capi structs
    [InlineData("bindings/c/tests/medius-layout.txt", "Anybox.Medius", 51)]
    public void TheStructsMatchTheCommittedLayout(string file, string space, int count)
    {
        var layout = MockLibrary.ReadFile(file).ReplaceLineEndings("\n").Split('\n', StringSplitOptions.RemoveEmptyEntries);
        Type? type = null;
        var structs = 0;
        foreach (var line in layout)
        {
            var parts = line.Split(' ', StringSplitOptions.RemoveEmptyEntries);
            if (!line.StartsWith(' '))
            {
                type = typeof(Library).Assembly.GetType($"{space}.{parts[0]}", throwOnError: true)!;
                Assert.Equal(int.Parse(parts[2]), (int)typeof(Unsafe).GetMethod(nameof(Unsafe.SizeOf))!.MakeGenericMethod(type).Invoke(null, null)!);
                structs++;
            }
            else
            {
                Assert.Equal(int.Parse(parts[1]), (int)Marshal.OffsetOf(type!, parts[0]));
            }
        }
        Assert.Equal(count, structs);
    }

    record Declaration(string Name, string Return, (string Type, string Name)[] Parameters);

    static Declaration[] Declarations() => Regex.Matches(
            Header,
            @"^(?<return>[A-Za-z][\w ]*?\**)\s*\b(?<name>anybox_\w+)\((?<parameters>[^)]*)\);",
            RegexOptions.Multiline)
        .Select(match => new Declaration(
            match.Groups["name"].Value,
            Normalize(match.Groups["return"].Value),
            match.Groups["parameters"].Value.Trim() is "void" ? [] : match.Groups["parameters"].Value
                .Split(',')
                .Select(parameter => Regex.Match(Normalize(parameter), @"^(?<type>.*?)\s*(?<name>\w+)$"))
                .Select(parameter => (parameter.Groups["type"].Value, parameter.Groups["name"].Value))
                .ToArray()))
        .ToArray();

    static string Normalize(string type) => Regex.Replace(Regex.Replace(type.Trim(), @"\s+", " "), @"\s*\*", "*");

    static readonly string[] Handles = ["Device", "Events", "Mock", "ScanList", "Snapshot", "StringList", "YscSerial"];

    // the calls only one box has, which go through the seam like a device call does
    static bool Extension(string function) =>
        (function.StartsWith("anybox_makcu_") || function.StartsWith("anybox_ysc_") || function.StartsWith("anybox_ferrum_"))
        && !function.StartsWith("anybox_ysc_serial_");

    // what each C type has to be on this side, `in` and `out` for a pointer to one value
    // a device call goes through the seam the recording test swaps out, and DispatchProxy can't take a pointer, so a
    // text or a key list is an array there, which LibraryImport pins
    // an extension call writes one value through each pointer it takes, so those are `out`
    static string Parameter(string function, string type) => (function, type) switch
    {
        _ when function.StartsWith("anybox_device_") && type is "const char*" or "const uint8_t*" => "Byte[]",
        _ when Extension(function) && type == "uint32_t*" => "out UInt32",
        _ when Extension(function) && type == "bool*" => "out Byte",
        _ when Extension(function) && type == "int32_t*" => "out Int32",
        _ => Expected(type),
    };

    static string Expected(string type)
    {
        if (Regex.Match(type, @"^const struct Anybox(\w+)\*$") is { Success: true } input)
        {
            var name = input.Groups[1].Value;
            return Handles.Contains(name) ? $"{name}Handle" : $"in Anybox{name}";
        }
        if (Regex.Match(type, @"^struct Anybox(\w+)\*\*$") is { Success: true } output)
        {
            return $"out {output.Groups[1].Value}Handle";
        }
        if (Regex.Match(type, @"^struct Anybox(\w+)\*$") is { Success: true } written)
        {
            var name = written.Groups[1].Value;
            return Handles.Contains(name) ? "IntPtr" : $"out Anybox{name}";
        }
        return type switch
        {
            "uint32_t" or "AnyboxStatus" => "UInt32",
            "int16_t" => "Int16",
            "uint8_t" => "Byte",
            "uint16_t" => "UInt16",
            "int64_t" => "Int64",
            "uint64_t" => "UInt64",
            "uintptr_t" => "UIntPtr",
            "bool" => "Byte",
            "void" => "Void",
            "const char*" => "Byte*",
            "uint32_t*" => "UInt32*",
            "const uint8_t*" or "uint8_t*" => "Byte*",
            "const uint16_t*" => "UInt16*",
            "uintptr_t*" => "out UIntPtr",
            // the Medius handle, bound in Anybox.Medius
            "MediusDevice**" => "out MediusDeviceHandle",
            _ => throw new InvalidOperationException($"the test doesn't know the C type {type}"),
        };
    }

    static string Describe(ParameterInfo parameter) => parameter.ParameterType.IsByRef
        ? $"{(parameter.IsOut ? "out" : parameter.IsIn ? "in" : "ref")} {parameter.ParameterType.GetElementType()!.Name}"
        : parameter.ParameterType.Name;

    static string[] Signature(MethodInfo method) =>
        [method.ReturnType.Name, .. method.GetParameters().Select(parameter => $"{Describe(parameter)} {parameter.Name}")];

    static readonly MethodInfo[] Bound = typeof(Imports)
        .GetMethods(BindingFlags.Static | BindingFlags.NonPublic | BindingFlags.DeclaredOnly)
        .Where(method => method.Name.StartsWith("anybox_"))
        .ToArray();

    [Fact]
    public void EveryHeaderFunctionIsBoundOrLeftOutOnPurpose()
    {
        var declared = Declarations().Select(declaration => declaration.Name).ToHashSet();
        // connect and the plain click and taps are the options versions with defaults
        string[] leftOut =
        [
            "anybox_connect",
            "anybox_connect_options_default",
            "anybox_device_click",
            "anybox_device_tap_key",
            "anybox_device_tap_media",
        ];
        Assert.Equal(
            declared.Except(leftOut).Order(),
            Bound.Select(method => method.Name).Order());
    }

    [Fact]
    public void EveryBindingTakesWhatTheHeaderDeclares()
    {
        var declarations = Declarations().ToDictionary(declaration => declaration.Name);
        foreach (var method in Bound)
        {
            var declaration = declarations[method.Name];
            string[] expected =
            [
                Expected(declaration.Return),
                .. declaration.Parameters.Select(parameter => $"{Parameter(method.Name, parameter.Type)} {parameter.Name}"),
            ];
            Assert.Equal(expected, Signature(method));
        }
    }

    [Fact]
    public void EveryDeviceCallGoesThroughTheSeamWithTheSameSignature()
    {
        var seam = typeof(INative).GetMethods();
        Assert.Equal(
            Bound.Where(method => (method.Name.StartsWith("anybox_device_") && method.Name != "anybox_device_free") || Extension(method.Name)).Select(method => method.Name).Order(),
            seam.Select(method => method.Name).Order());
        foreach (var method in seam)
        {
            Assert.Equal(Signature(Bound.Single(bound => bound.Name == method.Name)), Signature(method));
        }
    }

    // the recording test swaps the seam out, so this reads the real forwarders' IL: every argument in order, into
    // the import with the same name
    [Fact]
    public void EveryForwarderHandsItsArgumentsOnInOrder()
    {
        foreach (var method in typeof(INative).GetMethods())
        {
            var il = method.GetMethodBody()!.GetILAsByteArray()!;
            var arguments = new List<int>();
            MethodBase? called = null;
            for (var at = 0; at < il.Length;)
            {
                switch (il[at])
                {
                    // nop, stloc.0, ldloc.0 and ret, a debug build stores the result and jumps to the ret
                    case 0x00 or 0x0A or 0x06 or 0x2A:
                        at++;
                        break;
                    case >= 0x03 and <= 0x05:
                        arguments.Add(il[at] - 0x02);
                        at++;
                        break;
                    case 0x0E:
                        arguments.Add(il[at + 1]);
                        at += 2;
                        break;
                    case 0x2B:
                        at += 2;
                        break;
                    case 0x28:
                        called = method.Module.ResolveMethod(BitConverter.ToInt32(il, at + 1));
                        at += 5;
                        break;
                    default:
                        throw new InvalidOperationException($"{method.Name} has IL this test doesn't read ({il[at]:X2})");
                }
            }
            Assert.Equal(Enumerable.Range(1, method.GetParameters().Length), arguments);
            Assert.Equal(typeof(Imports), called!.DeclaringType);
            Assert.Equal(method.Name, called.Name);
        }
    }

    [Fact]
    public void EveryVersionNumberIsTheSame()
    {
        static string Version(string path) =>
            Regex.Match(MockLibrary.ReadFile(path), "^version = \"(.+)\"", RegexOptions.Multiline).Groups[1].Value;

        var crate = Version("Cargo.toml");
        Assert.Equal(crate, Version("bindings/c/Cargo.toml"));
        Assert.Equal(crate, Regex.Match(MockLibrary.ReadFile("bindings/csharp/src/Anybox/Anybox.csproj"), "<Version>(.+)</Version>").Groups[1].Value);
        Assert.Equal(crate, typeof(Library).Assembly.GetName().Version!.ToString(3));
        Assert.Equal(crate, Library.Version);
        Assert.True(Library.HasMock);
    }
}
