using System.Diagnostics;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Runtime.CompilerServices;
using System.Text.RegularExpressions;
using Anybox.Medius;
using Anybox.Tests;

namespace MediusTests;

// what the C# side adds over the Python package: device.Medius() on an anybox device, the streams' waits and
// lifetimes, the update progress, and the names
public sealed class MediusBindingTests
{
    static CancellationToken Cancel => TestContext.Current.CancellationToken;

    static Task<T> OnItsOwnThread<T>(Func<T> work) =>
        Task.Factory.StartNew(work, Cancel, TaskCreationOptions.LongRunning, TaskScheduler.Default);

    [Fact]
    public void AMediusGivesItsOwnDeviceThatSharesTheConnection()
    {
        var (device, box) = Anybox.Mock.Medius();
        using var _ = box;
        using var __ = device;
        using (var medius = device.Medius())
        {
            Assert.NotNull(medius);
            Assert.True(medius.QueryVersion().ProtoVer > 0);
        }
        // freeing the Medius handle leaves the anybox device working
        device.MoveBy(1, 1);
        device.Close();
        Assert.Null(device.Medius());
    }

    [Fact]
    public void TheMediusDeviceOutlivesTheAnyboxOneOnItsHandle()
    {
        var (device, box) = Anybox.Mock.Medius();
        using var _ = box;
        var medius = device.Medius()!;
        device.Dispose();
        // it keeps the connection, so it still answers after the anybox device is gone
        Assert.True(medius.QueryVersion().ProtoVer > 0);
        medius.Dispose();
        Assert.Throws<ObjectDisposedException>(() => medius.QueryVersion());
    }

    [Fact]
    public void EveryHeaderFunctionIsBound()
    {
        var header = Regex.Replace(MockLibrary.ReadFile("bindings/c/include/medius.h"), @"//[^\n]*", "");
        var declared = Regex.Matches(header, @"\b(medius_\w+)\((?:[^()]|\([^()]*\))*\);").Select(match => match.Groups[1].Value).ToHashSet();
        var bound = typeof(MediusImports).GetMethods(BindingFlags.Static | BindingFlags.NonPublic | BindingFlags.DeclaredOnly)
            .Select(method => method.Name).Where(name => name.StartsWith("medius_")).ToHashSet();
        Assert.Equal(258, declared.Count);
        Assert.Equal(declared.Order(), bound.Order());
    }

    static readonly string MediusHeader = Regex.Replace(MockLibrary.ReadFile("bindings/c/include/medius.h"), @"//[^\n]*", "");

    static readonly Dictionary<string, string> Widths = new()
    {
        ["uint8_t"] = "u8", ["int8_t"] = "i8", ["bool"] = "u8", ["char"] = "u8", ["uint16_t"] = "u16", ["int16_t"] = "i16",
        ["uint32_t"] = "u32", ["int32_t"] = "i32", ["uint64_t"] = "u64", ["int64_t"] = "i64", ["uintptr_t"] = "usize",
        ["float"] = "f32", ["void"] = "void",
    };

    // what a C type is in width, signedness and pointers, so a binding is held to the header and not to the
    // generator's own table
    static string Shape(string type, ISet<string> structs, string function)
    {
        type = Regex.Replace(type.Replace("const ", "").Trim(), @"\s*\*", "*");
        if (type.Contains("(*"))
        {
            return "callback";
        }
        var pointers = type.Length - type.TrimEnd('*').Length;
        var name = type.TrimEnd('*').Trim();
        if (name.StartsWith("struct "))
        {
            var bare = name["struct ".Length..];
            if (!structs.Contains(bare))
            {
                // an opaque handle, freed through the raw pointer
                return pointers switch
                {
                    1 when function.EndsWith("_free") => "isize",
                    1 => $"handle {bare}",
                    _ => $"out handle {bare}",
                };
            }
            return new string('*', pointers) + bare;
        }
        var typedef = Regex.Match(MediusHeader, $@"typedef (\w+) {name};");
        return new string('*', pointers) + Widths[typedef.Success ? typedef.Groups[1].Value : name];
    }

    static string Shape(Type type, ParameterInfo? parameter = null)
    {
        if (type.IsByRef)
        {
            return $"out {Shape(type.GetElementType()!)}";
        }
        if (type.IsPointer)
        {
            return "*" + Shape(type.GetElementType()!);
        }
        if (type.IsFunctionPointer)
        {
            return "callback";
        }
        if (typeof(SafeHandle).IsAssignableFrom(type))
        {
            return $"handle {type.Name[..^"Handle".Length]}";
        }
        if (type.IsEnum)
        {
            return Shape(Enum.GetUnderlyingType(type));
        }
        return Type.GetTypeCode(type) switch
        {
            TypeCode.Byte => "u8", TypeCode.SByte => "i8", TypeCode.UInt16 => "u16", TypeCode.Int16 => "i16",
            TypeCode.UInt32 => "u32", TypeCode.Int32 => "i32", TypeCode.UInt64 => "u64", TypeCode.Int64 => "i64",
            TypeCode.Single => "f32",
            _ when type == typeof(nuint) => "usize",
            _ when type == typeof(nint) => "isize",
            _ when type == typeof(void) => "void",
            _ => type.Name,
        };
    }

    [Fact]
    public void EveryImportTakesWhatTheHeaderDeclares()
    {
        var structs = MockLibrary.ReadFile("bindings/c/tests/medius-layout.txt").ReplaceLineEndings("\n").Split('\n')
            .Where(line => line.Length > 0 && !line.StartsWith(' ')).Select(line => line.Split(' ')[0]).ToHashSet();
        var bound = typeof(MediusImports).GetMethods(BindingFlags.Static | BindingFlags.NonPublic | BindingFlags.DeclaredOnly)
            .Where(method => method.Name.StartsWith("medius_")).ToDictionary(method => method.Name);
        var declarations = Regex.Matches(MediusHeader, @"(?<return>[A-Za-z][\w ]*?\**)\s*\b(?<name>medius_\w+)\((?<parameters>(?:[^()]|\([^()]*\))*)\);");
        Assert.Equal(258, declarations.Count);
        foreach (Match declaration in declarations)
        {
            var function = declaration.Groups["name"].Value;
            var parameters = Regex.Split(declaration.Groups["parameters"].Value, @",(?![^()]*\))")
                .Select(parameter => parameter.Trim()).Where(parameter => parameter is not ("" or "void"))
                .Select(parameter => parameter.Contains("(*")
                    ? ("callback", Regex.Match(parameter, @"\(\*(\w+)\)").Groups[1].Value)
                    : (Shape(Regex.Match(parameter, @"^(.*?)\s*\w+$").Groups[1].Value, structs, function), Regex.Match(parameter, @"(\w+)$").Groups[1].Value));
            string[] expected = [Shape(declaration.Groups["return"].Value, structs, function), .. parameters.Select(parameter => $"{parameter.Item1} {parameter.Item2.TrimEnd('_')}")];
            var method = bound[function];
            string[] actual = [Shape(method.ReturnType), .. method.GetParameters().Select(parameter => $"{Shape(parameter.ParameterType)} {parameter.Name!.TrimEnd('_')}")];
            Assert.Equal(expected, actual);
        }
    }

    // a value a newer library sends that this package doesn't know is an error saying to update, like the Python
    // package, never a number passed on as if it were known
    public static TheoryData<string, Func<object>> UnknownValues => new()
    {
        { "a usage class", () => Usage.From(new MediusUsage { kind = 200 }) },
        { "a log level", () => LogLine.From(new MediusLogLine { level = (LogLevel)200 }) },
        { "a clip state", () => ClipStatus.From(new MediusClipStatus { state = 200 }) },
        { "a render mode", () => RenderStatus.From(new MediusRenderStatus { mode = (RenderMode)200 }) },
        { "a catch event", () => CatchEvent.From(new MediusCatchEvent { kind = (CatchEventKind)200 }) },
        { "a clock domain", () => CatchEvent.From(new MediusCatchEvent { kind = CatchEventKind.Motion, clock = (ClockDomain)200 }) },
    };

    [Theory]
    [MemberData(nameof(UnknownValues))]
    public void AnUnknownValueFromTheLibraryIsAnError(string what, Func<object> decode)
    {
        var error = Assert.Throws<Anybox.AnyboxException>(decode);
        Assert.Equal($"the library sent {what} this package doesn't know (200), so update the Anybox package", error.Message);
    }

    // the header has these two only in comments, so the values are read from there
    [Fact]
    public void TheImageStatesAndUpdateTargetsAreTheHeaders()
    {
        var states = Regex.Match(MockLibrary.ReadFile("bindings/c/include/medius.h"), @"// (0 new, 1 pending-verify[^.]*)\.").Groups[1].Value;
        var named = states.Split(", ").Select(part => part.Split(' ', 2)).ToDictionary(
            part => Enum.Parse<ImageState>(part[1].Replace("-", ""), ignoreCase: true),
            part => Convert.ToByte(part[0], part[0].StartsWith("0x") ? 16 : 10));
        Assert.Equal(Enum.GetValues<ImageState>().ToDictionary(state => state, state => (byte)state), named);
        Assert.Contains("(0 = device chip, 1 = host chip)", MockLibrary.ReadFile("bindings/c/include/medius.h"));
        Assert.Equal((0, 1), ((int)UpdateTarget.Device, (int)UpdateTarget.Host));
    }

    [Fact]
    public void TheFirmwareInfoReadsBothChips()
    {
        var raw = new MediusFirmwareInfo
        {
            device = new MediusChipFirmware { major = 1, minor = 2, patch = 3, slot = 1, state = 1 },
            host_present = 1,
            host = new MediusChipFirmware { major = 4, minor = 5, patch = 6, slot = 0, state = 0x42 },
            slot_size = 0x1F0000,
            host_staged = 1,
        };
        var info = FirmwareInfo.From(raw);
        Assert.Equal(new FirmwareInfo(new(1, 2, 3, 1, ImageState.PendingVerify), new(4, 5, 6, 0, (ImageState)0x42), 0x1F0000, false, true), info);
        Assert.True(info.AnyPending);
        Assert.Equal("1.2.3 on ota_1 (pending-verify)", info.Device.ToString());
        Assert.Equal("4.5.6 on ota_0 (unknown(66))", info.Host!.ToString());
        Assert.Equal("0.0.0 on ota_0 (unknown(255))", new ChipFirmware(0, 0, 0, 0, ImageState.Unknown).ToString());
        Assert.Null(FirmwareInfo.From(raw with { host_present = 0 }).Host);
        Assert.False(FirmwareInfo.From(raw with { device = raw.device with { state = 2 } }).AnyPending);
    }

    [Fact]
    public void TheKeysAndCatchClassesAreTheHeaders()
    {
        var header = MockLibrary.ReadFile("bindings/c/include/medius.h");
        var defines = Regex.Matches(header, @"^#define (MEDIUS_\w+) (\d+)$", RegexOptions.Multiline).ToDictionary(match => match.Groups[1].Value, match => int.Parse(match.Groups[2].Value));
        Assert.Equal(defines.Count(define => define.Key.StartsWith("MEDIUS_KEY_")), Enum.GetValues<Key>().Length);
        Assert.Equal((int)Key.A, defines["MEDIUS_KEY_A"]);
        Assert.Equal((int)Key.N1, defines["MEDIUS_KEY_1"]);
        Assert.Equal(defines.Count(define => define.Key.StartsWith("MEDIUS_MEDIA_")), Enum.GetValues<MediaKey>().Length);
        Assert.Equal((int)CatchClass.ClipTransfer, defines["MEDIUS_CATCH_CLASS_CLIP_TRANSFER"]);
        Assert.Equal((int)CatchClass.ClipTransfer, (int)TrafficClass.ClipTransfer);
        Assert.Equal(Constants.ClipPktTrigMax, defines["MEDIUS_CLIP_PKT_TRIG_MAX"]);
        Assert.Equal(Constants.MaxPktMatch, defines["MEDIUS_MAX_PKT_MATCH"]);
    }

    // a public type named like one the implicit usings bring in is ambiguous in every file that uses both,
    // which is why Action is BoxAction and InjectAction, and Version is BoxVersion
    [Fact]
    public void NoPublicNameClashesWithTheImplicitUsings()
    {
        string[] implicitUsings = ["System", "System.Collections.Generic", "System.IO", "System.Linq", "System.Net.Http", "System.Threading", "System.Threading.Tasks"];
        Assembly[] framework = [typeof(object).Assembly, typeof(Enumerable).Assembly, typeof(HttpClient).Assembly, typeof(Barrier).Assembly, typeof(Queue<>).Assembly];
        var taken = framework.SelectMany(assembly => assembly.GetExportedTypes())
            .Where(type => implicitUsings.Contains(type.Namespace))
            .Select(type => type.Name.Split('`')[0])
            .ToHashSet();
        var ours = typeof(Anybox.Library).Assembly.GetExportedTypes().Where(type => !type.IsNested).Select(type => type.Name);
        var clashes = ours.Where(taken.Contains).ToList();
        Assert.True(clashes.Count == 0, $"these clash with the implicit usings: {string.Join(", ", clashes)}");
    }

    [Fact]
    public async Task DisposingAStreamFromAnotherThreadEndsItsReader()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        var events = device.CatchEvents(CatchFilter.Everything());
        var inputs = device.InputEvents([.. CatchFilter.AllInput()]);
        var logs = device.Logs();
        var timedEvents = device.CatchEvents(CatchFilter.Everything());
        var timedInputs = device.InputEvents([.. CatchFilter.AllInput()]);
        var timedLogs = device.Logs();
        var forever = TimeSpan.FromSeconds(10);
        var readers = new Task<Exception>[]
        {
            OnItsOwnThread(() => (Exception)Assert.Throws<Anybox.ClosedException>(() => events.Recv())),
            OnItsOwnThread(() => (Exception)Assert.Throws<Anybox.ClosedException>(() => inputs.Recv())),
            OnItsOwnThread(() => (Exception)Assert.Throws<Anybox.ClosedException>(() => logs.Recv())),
            OnItsOwnThread(() => (Exception)Assert.Throws<Anybox.ClosedException>(() => timedEvents.RecvTimeout(forever))),
            OnItsOwnThread(() => (Exception)Assert.Throws<Anybox.ClosedException>(() => timedInputs.RecvTimeout(forever))),
            OnItsOwnThread(() => (Exception)Assert.Throws<Anybox.ClosedException>(() => timedLogs.RecvTimeout(forever))),
        };
        // a reader that isn't waiting yet just finds it disposed, which ends it too, the wait below is what fails a
        // receive that never ends
        await Task.Delay(200, Cancel);
        IDisposable[] streams = [events, inputs, logs, timedEvents, timedInputs, timedLogs];
        foreach (var stream in streams)
        {
            stream.Dispose();
        }
        await Task.WhenAll(readers).WaitAsync(TimeSpan.FromSeconds(3), Cancel);
        Assert.Empty(events);
        foreach (var stream in streams)
        {
            stream.Dispose();
        }
    }

    // like the crate, a close ends the subscription for every clone on it, while a clone that only gets collected
    // frees its own handle and leaves the others reading
    [Fact]
    public async Task DisposingOneCloneEndsTheOthersAndCollectingOneDoesnt()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        var events = device.CatchEvents(CatchFilter.Everything());
        var logs = device.Logs();
        var other = device.Logs();
        var eventsClone = events.Clone();
        var logsClone = logs.Clone();
        var readers = new Task<Exception>[]
        {
            OnItsOwnThread(() => (Exception)Assert.Throws<DisconnectedException>(() => eventsClone.Recv())),
            OnItsOwnThread(() => (Exception)Assert.Throws<DisconnectedException>(() => logsClone.Recv())),
        };
        await Task.Delay(200, Cancel);
        events.Dispose();
        logs.Dispose();
        await Task.WhenAll(readers).WaitAsync(TimeSpan.FromSeconds(3), Cancel);
        Assert.False(eventsClone.IsConnected);
        Assert.False(logsClone.IsConnected);
        Assert.Null(eventsClone.TryRecv());
        // well under the 5 s it waits for a line that could still come
        var clock = Stopwatch.StartNew();
        Assert.Null(logsClone.RecvTimeout(TimeSpan.FromSeconds(5)));
        Assert.True(clock.Elapsed < TimeSpan.FromSeconds(2), $"a closed stream's timed wait took {clock.Elapsed}");
        // another log of the same device keeps going
        Assert.True(other.IsConnected);
        mock.PushLog(LogLevel.Info, "still here");
        Assert.Equal("still here", other.RecvTimeout(TimeSpan.FromSeconds(2))?.Text);
        eventsClone.Dispose();
        logsClone.Dispose();

        var kept = device.Logs();
        Collected(kept);
        GC.Collect();
        GC.WaitForPendingFinalizers();
        Assert.True(kept.IsConnected);
        mock.PushLog(LogLevel.Info, "after the collect");
        Assert.Equal("after the collect", kept.RecvTimeout(TimeSpan.FromSeconds(2))?.Text);
        kept.Dispose();
        other.Dispose();
    }

    // the clone is made and dropped in here, so nothing in the test keeps it reachable
    [MethodImpl(MethodImplOptions.NoInlining)]
    static void Collected(LogStream stream) => stream.Clone();

    // the library takes the input stream shared, so its other calls go in while a receive is blocked on it
    [Fact]
    public async Task AnInputStreamAnswersWhileAnotherThreadWaitsOnIt()
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        var inputs = device.InputEvents([.. CatchFilter.AllInput()]);
        var reader = OnItsOwnThread(() => (Exception)Assert.Throws<Anybox.ClosedException>(() => inputs.Recv()));
        await Task.Delay(200, Cancel);
        var asked = Task.Run(() => (inputs.Held(Class.Button).Count, inputs.Dropped, inputs.IsConnected), Cancel);
        Assert.Equal((0, 0ul, true), await asked.WaitAsync(TimeSpan.FromSeconds(2), Cancel));
        inputs.Dispose();
        await reader.WaitAsync(TimeSpan.FromSeconds(3), Cancel);
    }

    // the handle is freed too, so every other call says the stream is gone
    [Theory]
    [InlineData("events")]
    [InlineData("inputs")]
    [InlineData("logs")]
    public void ADisposedStreamSaysSo(string kind)
    {
        using var mock = new MockBox();
        using var device = Device.WithMock(mock);
        switch (kind)
        {
            case "events":
                var events = device.CatchEvents(CatchFilter.Everything());
                events.Dispose();
                Assert.Throws<Anybox.ClosedException>(() => events.IsConnected);
                Assert.Throws<Anybox.ClosedException>(() => events.Dropped);
                Assert.Throws<Anybox.ClosedException>(() => events.TryRecv());
                Assert.Throws<Anybox.ClosedException>(events.Clone);
                Assert.Throws<Anybox.ClosedException>(() => events.Recv());
                Assert.Throws<Anybox.ClosedException>(() => events.RecvTimeout(TimeSpan.FromSeconds(1)));
                Assert.Empty(events);
                break;
            case "inputs":
                var inputs = device.InputEvents([.. CatchFilter.AllInput()]);
                inputs.Dispose();
                Assert.Throws<Anybox.ClosedException>(() => inputs.IsConnected);
                Assert.Throws<Anybox.ClosedException>(() => inputs.Dropped);
                Assert.Throws<Anybox.ClosedException>(() => inputs.TryRecv());
                Assert.Throws<Anybox.ClosedException>(() => inputs.Held(Class.Key));
                Assert.Throws<Anybox.ClosedException>(() => inputs.Recv());
                Assert.Throws<Anybox.ClosedException>(() => inputs.RecvTimeout(TimeSpan.FromSeconds(1)));
                Assert.Empty(inputs);
                break;
            default:
                var logs = device.Logs();
                logs.Dispose();
                Assert.Throws<Anybox.ClosedException>(() => logs.IsConnected);
                Assert.Throws<Anybox.ClosedException>(() => logs.TryRecv());
                Assert.Throws<Anybox.ClosedException>(logs.Clone);
                Assert.Throws<Anybox.ClosedException>(() => logs.Recv());
                Assert.Throws<Anybox.ClosedException>(() => logs.RecvTimeout(TimeSpan.FromSeconds(1)));
                Assert.Empty(logs);
                break;
        }
    }

    // the holder only ever sits in the list, so no hidden local of the test keeps it or the device reachable
    [MethodImpl(MethodImplOptions.NoInlining)]
    static List<IDisposable> Holder(MockBox mock, string kind, out WeakReference weak)
    {
        var device = Device.WithMock(mock);
        weak = new WeakReference(device);
        IDisposable holder = kind switch
        {
            "events" => device.CatchEvents(CatchFilter.Everything()),
            "inputs" => device.InputEvents([.. CatchFilter.AllInput()]),
            "logs" => device.Logs(),
            "clip" => device.Clip(),
            "events clone" => Cloned(device.CatchEvents(CatchFilter.Everything()), stream => stream.Clone()),
            _ => Cloned(device.Logs(), stream => stream.Clone()),
        };
        return [holder];
    }

    // the clone outlives its original, so it has to hold the device on its own
    static IDisposable Cloned<T>(T original, Func<T, IDisposable> clone) where T : IDisposable
    {
        var copy = clone(original);
        original.Dispose();
        return copy;
    }

    [MethodImpl(MethodImplOptions.NoInlining)]
    static void Drop(List<IDisposable> held)
    {
        held[0].Dispose();
        held.Clear();
    }

    static void Collect()
    {
        GC.Collect();
        GC.WaitForPendingFinalizers();
        GC.Collect();
    }

    [Theory]
    [InlineData("events")]
    [InlineData("inputs")]
    [InlineData("logs")]
    [InlineData("clip")]
    [InlineData("events clone")]
    [InlineData("logs clone")]
    public void EveryStreamAndClipKeepsItsDeviceAlive(string kind)
    {
        using var mock = new MockBox();
        var held = Holder(mock, kind, out var weak);
        Collect();
        Assert.True(weak.IsAlive, $"the device went while its {kind} was still held");
        Drop(held);
        Collect();
        Assert.False(weak.IsAlive);
    }

    sealed class Recorded : IProgress<UpdateProgress>
    {
        public List<UpdateProgress> Seen { get; } = [];

        public void Report(UpdateProgress value) => Seen.Add(value);
    }

    [Fact]
    public void AStagedUpdateReportsItsProgress()
    {
        var image = Enumerable.Range(0, 64 * 1024).Select(value => (byte)value).ToArray();
        var progress = new Recorded();
        using var mock = new MockBox();
        using var device = mock.Open();
        device.StageFirmware(UpdateTarget.Device, image, progress);
        Assert.NotEmpty(progress.Seen);
        Assert.Equal((nuint)image.Length, progress.Seen[^1].Total);
        Assert.Equal(progress.Seen[^1].Total, progress.Seen[^1].Sent);
        Assert.Equal((UpdateTarget.Device, (byte)100), (progress.Seen[^1].Target, progress.Seen[^1].Percent));
        Assert.True(progress.Seen.Zip(progress.Seen.Skip(1)).All(pair => pair.First.Sent <= pair.Second.Sent));
        Assert.True(device.FirmwareInfo().DeviceStaged);
        device.AbortUpdate(UpdateTarget.Device);
        // no progress is fine too
        device.StageFirmware(UpdateTarget.Device, image);
        // medius refuses it itself, like every other value it can't take
        Assert.Throws<InvalidArgException>(() => device.StageFirmware(UpdateTarget.Device, []));
    }

    // a box rebooting into its new image doesn't answer for a while, so the wait asks again like the crate's
    // instead of failing on the first query that timed out
    [Fact]
    public void WaitingForTheFirmwareRidesOutTheReboot()
    {
        var image = Enumerable.Range(0, 4096).Select(value => (byte)value).ToArray();
        using var mock = new MockBox();
        using var device = mock.Open();
        Assert.False(device.WaitFirmwareConfirmed().AnyPending);
        mock.RebootAfterActivate(1, 2);
        device.UpdateFirmware(UpdateTarget.Device, image);
        var before = mock.Recorded;
        Assert.False(device.WaitFirmwareConfirmed().AnyPending);
        // one left unanswered by the reboot, two with the chip on probation, then the confirmed one, like the crate
        const byte FirmwareQuery = 11;
        var asked = Enumerable.Range(before, mock.Recorded - before).Select(mock.RecordedFrame)
            .Count(frame => frame!.Type == FrameType.Query && frame.Payload.Span[0] == FirmwareQuery);
        Assert.Equal(4, asked);
    }

    [Theory]
    [InlineData(0L, 0ul)]
    [InlineData(1L, 1ul)]
    [InlineData(TimeSpan.TicksPerMillisecond, 1ul)]
    [InlineData(TimeSpan.TicksPerMillisecond + 1, 2ul)]
    [InlineData(TimeSpan.TicksPerSecond, 1000ul)]
    [InlineData(long.MaxValue, (ulong)(long.MaxValue / TimeSpan.TicksPerMillisecond) + 1)]
    public void ATimeoutGoesToTheLibraryRoundedUp(long ticks, ulong ms) => Assert.Equal(ms, Ending.Ms(TimeSpan.FromTicks(ticks)));

    [Fact]
    public void ANegativeTimeoutIsRefused() => Assert.Throws<ArgumentOutOfRangeException>(() => Ending.Ms(TimeSpan.FromTicks(-1)));

    // after the crate's own update tests
    [Fact]
    public void ARefusedUpdateSaysWhichOpAndWhy()
    {
        var image = Enumerable.Range(0, 4096).Select(value => (byte)value).ToArray();
        using var mock = new MockBox();
        using var device = mock.Open();
        var nothingStaged = Assert.Throws<UpdateException>(device.ActivateFirmware);
        Assert.Equal(((byte?)4, (byte?)0x19), (nothingStaged.Op, nothingStaged.UpdateStatus));
        Assert.NotNull(nothingStaged.Arg);

        // a refused activate aborts what it staged, so nothing can boot it later
        const byte LinkDown = 0x17;
        mock.RefuseNextActivate(LinkDown);
        var refused = Assert.Throws<UpdateException>(() => device.UpdateFirmware(UpdateTarget.Host, image));
        Assert.Equal(((byte?)4, (byte?)LinkDown), (refused.Op, refused.UpdateStatus));
        var info = device.FirmwareInfo();
        Assert.False(info.HostStaged || info.DeviceStaged);

        // only an update error has the detail, and one made without it has none
        Assert.Throws<InvalidArgException>(() => device.StageFirmware(UpdateTarget.Device, []));
        var bare = new UpdateException("text", 0);
        Assert.Equal(((byte?)null, (byte?)null, (uint?)null), (bare.Op, bare.UpdateStatus, bare.Arg));
    }

    sealed class Throwing : IProgress<UpdateProgress>
    {
        public int Calls { get; private set; }

        public void Report(UpdateProgress value)
        {
            Calls++;
            throw new InvalidOperationException("the handler broke");
        }
    }

    [Fact]
    public void AProgressThatThrowsAbortsTheUpdateAndComesOut()
    {
        var image = Enumerable.Range(0, 64 * 1024).Select(value => (byte)value).ToArray();
        var progress = new Throwing();
        using var mock = new MockBox();
        using var device = mock.Open();
        var error = Assert.Throws<InvalidOperationException>(() => device.StageFirmware(UpdateTarget.Device, image, progress));
        Assert.Equal("the handler broke", error.Message);
        Assert.Equal(1, progress.Calls);
        Assert.False(device.FirmwareInfo().DeviceStaged);
    }

    sealed class Breaking(Action before) : IProgress<UpdateProgress>
    {
        public void Report(UpdateProgress value)
        {
            before();
            throw new InvalidOperationException("the handler broke");
        }
    }

    // what the progress threw is what you need, a stage that failed after it isn't
    [Fact]
    public void AProgressThatThrowsWinsOverAStageThatFailsAfterIt()
    {
        var image = Enumerable.Range(0, 64 * 1024).Select(value => (byte)value).ToArray();
        using var mock = new MockBox();
        using var device = mock.Open();
        // dropping the update under the stage makes the box refuse the next part
        var progress = new Breaking(() => device.AbortUpdate(UpdateTarget.Device));
        var error = Assert.Throws<InvalidOperationException>(() => device.StageFirmware(UpdateTarget.Device, image, progress));
        Assert.Equal("the handler broke", error.Message);
        Assert.False(device.FirmwareInfo().DeviceStaged);
    }

    // and an abort that fails after it isn't either
    [Fact]
    public void AProgressThatThrowsWinsOverAnAbortThatFails()
    {
        var image = Enumerable.Range(0, 64 * 1024).Select(value => (byte)value).ToArray();
        using var mock = new MockBox();
        var device = mock.Open();
        // the stage holds the handle while it runs, so only the abort after it finds the device gone
        var progress = new Breaking(device.Dispose);
        var error = Assert.Throws<InvalidOperationException>(() => device.StageFirmware(UpdateTarget.Device, image, progress));
        Assert.Equal("the handler broke", error.Message);
    }

    [Fact]
    public void AMacThatIsntSixBytesIsRefused()
    {
        using var mock = new MockBox();
        var error = Assert.Throws<ArgumentException>(() => mock.SetVersion(new BoxVersion(1, 0, 0, 0, new byte[] { 1, 2, 3 }, "")));
        Assert.Contains("is 3 bytes, it has to be 6", error.Message);
        Assert.Throws<ArgumentException>(() => mock.SetVersion(new BoxVersion(1, 0, 0, 0, new byte[7], "")));
    }

    // the medius crate's own Default, read in ref/medius
    [Fact]
    public void TheDefaultsAreTheCratesOwn()
    {
        Assert.Equal(new RewriteRule(), RewriteRule.Default);
        Assert.Equal((RewriteClass.Emit, Direction.Both, RewriteAction.Pass), (RewriteRule.Default.RewriteClass, RewriteRule.Default.Direction, RewriteRule.Default.Action));
        Assert.Equal(new Bearing(), Bearing.Default);
        Assert.Equal(((ushort?)Constants.BearingWindowDefaultMs, BearingMode.PerAxis), (Bearing.Default.WindowMs, Bearing.Default.Mode));
        Assert.Equal(new RenderStatus(), RenderStatus.Default);
        Assert.Equal(RenderMode.Despiked, RenderStatus.Default.Mode);
        Assert.Equal(new Patch(), Patch.Default);
        Assert.Equal(PatchSection.Device, Patch.Default.Section);
        Assert.Equal(new DeviceInfo(), DeviceInfo.Default);
        Assert.Equal(DeviceKind.Unknown, DeviceInfo.Default.Kind);
        Assert.Equal(ClipState.Idle, ClipStatus.Default.State);
    }

    [Fact]
    public void RecordsHoldingListsCompareTheirEntries()
    {
        Usage[] held = [Key.A, Key.B];
        List<Usage> same = [Key.A, Key.B];
        var one = new UsageSnapshot(Class.Key, Direction.Press, held);
        var two = new UsageSnapshot(Class.Key, Direction.Press, same);
        Assert.Equal(one, two);
        Assert.Equal(one.GetHashCode(), two.GetHashCode());
        Assert.NotEqual(one, two with { Usages = [Key.B, Key.A] });
        Assert.NotEqual(one, two with { Direction = Direction.Release });
    }
}
