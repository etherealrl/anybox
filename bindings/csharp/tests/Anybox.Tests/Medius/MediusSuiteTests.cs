using System.Collections;
using System.Reflection;
using System.Runtime.CompilerServices;
using System.Text.Json.Nodes;
using Anybox.Medius;
using Anybox.Tests;
using Medius = Anybox.Medius;

namespace MediusTests;

// the Medius suite's C# runner: every scenario in bindings/conformance/medius.json through Anybox.Medius and medius'
// mock, held to what the medius crate itself did with it, which the capi crate's tests write to
// target/conformance/medius-expected.json. bindings/c/src/medius_suite.rs says what a scenario is. a generated call
// finds its method by reflection, by the C++ name in PascalCase, and what C# wraps another way, or with a span that
// reflection can't pass, is in the hand table below with the helpers and the texts
public sealed class MediusSuiteTests
{
    // the handles a scenario works on, made as it needs them
    sealed class Context : IDisposable
    {
        public readonly MockBox Mock = new();
        public Device? Device;
        public ClipHandle? Clip;
        public ClipBuilder Builder = new();
        public ClipFrame Frame = new();
        public Timeline Timeline = new();
        public EventStream? Events;
        public InputStream? Inputs;
        public LogStream? Logs;

        public void Dispose()
        {
            Events?.Dispose();
            Inputs?.Dispose();
            Logs?.Dispose();
            Clip?.Dispose();
            Builder.Dispose();
            Frame.Dispose();
            Timeline.Dispose();
            Device?.Dispose();
            Mock.Dispose();
        }
    }

    public static TheoryData<string> Names() => [.. JsonNode.Parse(MockLibrary.ReadFile("bindings/conformance/medius.json"))!
        .AsArray().Select(scenario => (string)scenario!["name"]!)];

    static JsonArray Expected()
    {
        var path = Path.Combine(MockLibrary.Workspace, "target", "conformance", "medius-expected.json");
        Assert.True(File.Exists(path), $"there's no {path}, run the capi crate's tests first, they record it");
        return JsonNode.Parse(File.ReadAllText(path))!.AsArray();
    }

    static string Pascal(string snake) => string.Concat(snake.Split('_').Select(part => char.ToUpperInvariant(part[0]) + part[1..]));

    static string Hex(ReadOnlySpan<byte> bytes) => Convert.ToHexStringLower(bytes);

    static byte[] Unhex(JsonNode node) => Convert.FromHexString((string)node!);

    // a value in the suite's JSON, the same form the recorder writes
    static JsonNode? ToJson(object? value)
    {
        switch (value)
        {
            case null:
                return null;
            case bool flag:
                return flag;
            case string text:
                return text;
            case ulong big:
                return big <= long.MaxValue ? (long)big : big;
            case nuint size:
                return (long)size;
            case byte or sbyte or short or ushort or int or uint or long:
                return Convert.ToInt64(value);
            case DirectionMeaning meaning:
                return meaning.ToString();
            case Enum named:
                var number = Convert.ToInt64(named);
                return SuiteTables.Enums.TryGetValue(value.GetType(), out var names) && names.Names.TryGetValue(number, out var name)
                    ? name
                    : number;
            case ReadOnlyMemory<byte> bytes:
                return Hex(bytes.Span);
            // the runtime lets an array of a byte enum pass as a byte[], so only a real one is bytes
            case byte[] bytes when value.GetType() == typeof(byte[]):
                return Hex(bytes);
            case Usage usage:
                return new JsonObject { ["kind"] = ToJson(usage.Kind), ["id"] = usage.Id };
            case LockTarget target:
                return new JsonObject { ["kind"] = ToJson(target.Kind), ["usage"] = ToJson(target.Input) };
            case CatchFilter filter:
                return new JsonObject
                {
                    ["class_"] = ToJson(filter.Class),
                    ["id"] = ToJson(filter.Id),
                    ["direction"] = ToJson(filter.Direction),
                    ["capture"] = filter.Capture,
                };
            case EmitPace pace:
                return new JsonObject { ["mode"] = ToJson(pace.Mode), ["hz"] = pace.Hz };
            case EmitPaceStatus status:
                return new JsonObject
                {
                    ["mode"] = ToJson(status.Mode),
                    ["resolved_hz"] = status.ResolvedHz,
                    ["force_hz"] = ToJson(status.ForceHz),
                    ["advertised_hz"] = status.AdvertisedHz,
                    ["force_active"] = status.ForceActive,
                };
            case ClipSettings settings:
                return new JsonObject
                {
                    ["autolock"] = ToJson(settings.Autolock),
                    ["loop"] = settings.Loop,
                    ["retain"] = settings.Retain,
                    ["finalized"] = settings.Finalized,
                    ["ride"] = settings.Ride,
                    ["triggers"] = ToJson(settings.Triggers),
                    ["packet_triggers"] = ToJson(settings.PacketTriggers),
                };
            case CatchEvent catchEvent:
                return new JsonObject
                {
                    ["kind"] = ToJson(catchEvent.Kind),
                    ["ts_us"] = catchEvent.TsUs,
                    ["clock"] = ToJson(catchEvent.Clock),
                    ["data"] = ToJson((object?)catchEvent.Motion ?? (object?)catchEvent.Usages ?? catchEvent.Traffic),
                };
            case InputEvent input:
                return new JsonObject
                {
                    ["kind"] = ToJson(input.Kind),
                    ["ts_us"] = input.TsUs,
                    ["clock"] = ToJson(input.Clock),
                    ["usage"] = ToJson(input.Usage),
                    ["dx"] = input.Dx,
                    ["dy"] = input.Dy,
                    ["dz"] = input.Dz,
                    ["pan"] = input.Pan,
                };
            case ITuple pair:
                return new JsonArray(Enumerable.Range(0, pair.Length).Select(at => ToJson(pair[at])).ToArray());
            case IEnumerable items:
                return new JsonArray(items.Cast<object?>().Select(ToJson).ToArray());
        }
        if (SuiteTables.Records.TryGetValue(value.GetType(), out var fields))
        {
            var record = new JsonObject();
            foreach (var (json, cs) in fields)
            {
                record[json] = ToJson(value.GetType().GetProperty(cs)!.GetValue(value));
            }
            return record;
        }
        throw new InvalidOperationException($"the runner can't write a {value.GetType().Name}");
    }

    static readonly Dictionary<string, LockTarget> Axes = new()
    {
        ["X"] = LockTarget.X,
        ["Y"] = LockTarget.Y,
        ["Wheel"] = LockTarget.Wheel,
        ["Pan"] = LockTarget.Pan,
    };

    // a value from the suite's JSON as the type a C# parameter or record field takes
    static object? FromJson(JsonNode? node, Type type)
    {
        if (node is null && !type.IsValueType)
        {
            return null;
        }
        if (Nullable.GetUnderlyingType(type) is { } inner)
        {
            return node is null ? null : FromJson(node, inner);
        }
        if (type.IsEnum)
        {
            var number = node is JsonValue text && text.TryGetValue<string>(out var name) ? SuiteTables.Enums[type].Values[name] : (long)node!;
            return Enum.ToObject(type, number);
        }
        if (type == typeof(bool))
        {
            return (bool)node!;
        }
        if (type == typeof(string))
        {
            return (string?)node;
        }
        if (type.IsPrimitive || type == typeof(nuint))
        {
            return type == typeof(nuint) ? (nuint)(ulong)node! : Convert.ChangeType((long)node!, type);
        }
        if (type == typeof(ReadOnlyMemory<byte>))
        {
            return new ReadOnlyMemory<byte>(Unhex(node!));
        }
        if (type == typeof(byte[]))
        {
            return Unhex(node!);
        }
        if (type == typeof(TimeSpan))
        {
            return TimeSpan.FromMilliseconds((long)node!);
        }
        if (type.IsGenericType && type.GetGenericTypeDefinition() is var generic &&
            (generic == typeof(IReadOnlyList<>) || generic == typeof(IEnumerable<>)))
        {
            var element = type.GetGenericArguments()[0];
            var list = (IList)Activator.CreateInstance(typeof(List<>).MakeGenericType(element))!;
            foreach (var item in node!.AsArray())
            {
                list.Add(FromJson(item, element));
            }
            return list;
        }
        if (type == typeof(Usage))
        {
            return new Usage((Class)FromJson(node!["kind"], typeof(Class))!, (ushort)(long)node["id"]!);
        }
        if (type == typeof(LockTarget))
        {
            var kind = (string)node!["kind"]!;
            return kind == "Usage" ? LockTarget.Usage((Usage)FromJson(node["usage"], typeof(Usage))!) : Axes[kind];
        }
        if (type == typeof(Motion))
        {
            return (string)node!["kind"]! switch
            {
                "Cursor" => Motion.Cursor((short)(long)node["dx"]!, (short)(long)node["dy"]!),
                "Wheel" => Motion.Wheel((short)(long)node["delta"]!),
                _ => Motion.Pan((short)(long)node["delta"]!),
            };
        }
        if (type == typeof(CatchFilter))
        {
            return Filter(node!);
        }
        if (type == typeof(EmitPace))
        {
            return (string)node!["mode"]! switch
            {
                "Learned" => EmitPace.Learned,
                "Interval" => EmitPace.Interval,
                _ => EmitPace.Fixed((ushort)(long)node["hz"]!),
            };
        }
        if (type == typeof(ClipSettings))
        {
            return new ClipSettings
            {
                Autolock = (IReadOnlyList<Blanket>)FromJson(node!["autolock"], typeof(IReadOnlyList<Blanket>))!,
                Loop = (bool)node["loop"]!,
                Retain = (bool)node["retain"]!,
                Finalized = (bool)node["finalized"]!,
                Ride = (bool)node["ride"]!,
                Triggers = (IReadOnlyList<ClipTrigger>)FromJson(node["triggers"], typeof(IReadOnlyList<ClipTrigger>))!,
                PacketTriggers = (IReadOnlyList<ClipPacketTrigger>)FromJson(node["packet_triggers"], typeof(IReadOnlyList<ClipPacketTrigger>))!,
            };
        }
        if (type == typeof(CatchEvent))
        {
            var kind = (CatchEventKind)FromJson(node!["kind"], typeof(CatchEventKind))!;
            var data = node["data"];
            return new CatchEvent(kind, (uint)(long)node["ts_us"]!, (ClockDomain)FromJson(node["clock"], typeof(ClockDomain))!,
                kind == CatchEventKind.Motion ? (MotionEvent)FromJson(data, typeof(MotionEvent))! : null,
                kind == CatchEventKind.Usages ? (UsageSnapshot)FromJson(data, typeof(UsageSnapshot))! : null,
                kind == CatchEventKind.Traffic ? (TrafficEvent)FromJson(data, typeof(TrafficEvent))! : null);
        }
        if (type == typeof(InputEvent))
        {
            return new InputEvent((InputKind)FromJson(node!["kind"], typeof(InputKind))!, (uint)(long)node["ts_us"]!,
                (ClockDomain)FromJson(node["clock"], typeof(ClockDomain))!, (Usage?)FromJson(node["usage"], typeof(Usage?)),
                (short)(long)node["dx"]!, (short)(long)node["dy"]!, (short)(long)node["dz"]!, (short)(long)node["pan"]!);
        }
        if (SuiteTables.Records.TryGetValue(type, out var fields))
        {
            // the positional constructor, its parameters named like the record's properties
            var constructor = type.GetConstructors().OrderByDescending(each => each.GetParameters().Length).First();
            var values = constructor.GetParameters()
                .Select(parameter => FromJson(node![fields.Single(field => field.Cs == parameter.Name).Json], parameter.ParameterType))
                .ToArray();
            return constructor.Invoke(values);
        }
        throw new InvalidOperationException($"the runner can't read a {type.Name}");
    }

    static T From<T>(JsonNode? node) => (T)FromJson(node, typeof(T))!;

    // a filter argument is the calls that build it, a constructor and then the modifiers, and a filter the box lists is
    // its fields, which C# builds through the same calls
    static CatchFilter Filter(JsonNode node)
    {
        if (node is JsonObject fields)
        {
            var catchClass = From<CatchClass?>(fields["class_"]);
            var id = From<ushort?>(fields["id"]);
            var made = catchClass switch
            {
                null => CatchFilter.Everything(),
                CatchClass.Axis => id is { } axis ? CatchFilter.WatchAxis((Axis)axis) : CatchFilter.WatchAxes(),
                CatchClass.Button or CatchClass.Key or CatchClass.Media => id is { } usage
                    ? CatchFilter.Watch(new Usage((Class)(byte)catchClass, usage))
                    : CatchFilter.WatchClass((Class)(byte)catchClass),
                _ => id is { } address ? CatchFilter.Traffic((TrafficClass)(byte)catchClass, address) : CatchFilter.TrafficOf((TrafficClass)(byte)catchClass),
            };
            made = made.WithDirection(From<Direction>(fields["direction"]));
            var capture = From<byte>(fields["capture"]);
            return capture == 0 ? made : made.WithCapture(capture);
        }
        CatchFilter? filter = null;
        foreach (var call in node.AsArray())
        {
            var args = call!.AsArray();
            filter = (string)args[0]! switch
            {
                "watch" => CatchFilter.Watch(From<Usage>(args[1])),
                "watch_axis" => CatchFilter.WatchAxis(From<Axis>(args[1])),
                "watch_class" => CatchFilter.WatchClass(From<Class>(args[1])),
                "watch_axes" => CatchFilter.WatchAxes(),
                "traffic" => CatchFilter.Traffic(From<TrafficClass>(args[1]), From<ushort>(args[2])),
                "traffic_of" => CatchFilter.TrafficOf(From<TrafficClass>(args[1])),
                "everything" => CatchFilter.Everything(),
                "with_direction" => filter!.Value.WithDirection(From<Direction>(args[1])),
                "with_capture" => filter!.Value.WithCapture(From<byte>(args[1])),
                "on_press" => filter!.Value.OnPress(),
                "on_release" => filter!.Value.OnRelease(),
                "inbound" => filter!.Value.Inbound(),
                "outbound" => filter!.Value.Outbound(),
                var name => throw new InvalidOperationException($"a filter can't take {name}"),
            };
        }
        return filter!.Value;
    }

    static uint? Window(JsonNode? node) => node is null ? null : (uint)(long)node;

    static ushort? Force(JsonNode? node) => node is null ? null : (ushort)(long)node;

    // what C# wraps by hand or takes as a span, by the C++ class and method the scenarios name
    static readonly Dictionary<string, Func<Context, JsonArray, JsonNode?>> Hand = new()
    {
        ["Device.clip"] = (ctx, _) => { ctx.Clip = ctx.Device!.Clip(); return null; },
        ["Device.catch_events"] = (ctx, args) => { ctx.Events = ctx.Device!.CatchEvents([.. From<IReadOnlyList<CatchFilter>>(args[0])]); return null; },
        ["Device.input_events"] = (ctx, args) => { ctx.Inputs = ctx.Device!.InputEvents([.. From<IReadOnlyList<CatchFilter>>(args[0])]); return null; },
        ["Device.logs"] = (ctx, _) => { ctx.Logs = ctx.Device!.Logs(); return null; },
        ["Device.raw"] = (ctx, args) => { ctx.Device!.Raw(From<byte>(args[0]), From<Direction>(args[1]), Unhex(args[2]!)); return null; },
        ["Device.transfer"] = (ctx, args) => ToJson(ctx.Device!.Transfer(From<byte>(args[0]), From<Setup>(args[1]), Unhex(args[2]!),
            args[3] is null ? null : TimeSpan.FromMilliseconds((long)args[3]!))),
        ["Device.remove_rewrite"] = (ctx, args) => { ctx.Device!.RemoveRewrite(From<RewriteRule>(args[0])); return null; },
        ["Device.set_movement_riding"] = (ctx, args) => { ctx.Device!.SetMovementRiding(Window(args[0])); return null; },
        ["Device.query_movement_riding"] = (ctx, _) => ToJson(ctx.Device!.QueryMovementRiding()),
        ["Device.set_emit_pace"] = (ctx, args) => { ctx.Device!.SetEmitPace(From<EmitPace>(args[0]), Force(args[1])); return null; },
        ["Device.set_bearing"] = (ctx, args) => { ctx.Device!.SetBearing(Window(args[0]), From<BearingMode>(args[1])); return null; },
        ["Device.stage_firmware"] = (ctx, args) => { ctx.Device!.StageFirmware(From<UpdateTarget>(args[0]), Unhex(args[1]!)); return null; },
        ["Device.update_firmware"] = (ctx, args) => { ctx.Device!.UpdateFirmware(From<UpdateTarget>(args[0]), Unhex(args[1]!)); return null; },
        ["Clip.set_autolock"] = (ctx, args) => { ctx.Clip!.SetAutolock([.. From<IReadOnlyList<Blanket>>(args[0])]); return null; },
        ["Clip.unbind_packet"] = (ctx, args) => { ctx.Clip!.UnbindPacket(From<ClipPacketTrigger>(args[0])); return null; },
        ["ClipBuilder.raw"] = (ctx, args) => { ctx.Builder.Raw(From<byte>(args[0]), From<Direction>(args[1]), Unhex(args[2]!)); return null; },
        ["ClipBuilder.transfer"] = (ctx, args) => { ctx.Builder.Transfer(From<byte>(args[0]), From<Setup>(args[1]), Unhex(args[2]!)); return null; },
        ["ClipBuilder.frame"] = (ctx, _) => { ctx.Builder.Frame(ctx.Frame); return null; },
        ["Clip.append"] = (ctx, _) => { ctx.Clip!.Append(ctx.Builder); return null; },
        ["ClipFrame.raw"] = (ctx, args) => { ctx.Frame.Raw(From<byte>(args[0]), From<Direction>(args[1]), Unhex(args[2]!)); return null; },
        ["ClipFrame.transfer"] = (ctx, args) => { ctx.Frame.Transfer(From<byte>(args[0]), From<Setup>(args[1]), Unhex(args[2]!)); return null; },
        ["MockBox.set_movement_riding"] = (ctx, args) => { ctx.Mock.SetMovementRiding(Window(args[0])); return null; },
        ["MockBox.set_emit_pace"] = (ctx, args) => { ctx.Mock.SetEmitPace(From<EmitPace>(args[0]), Force(args[1])); return null; },
        ["MockBox.clip_packet"] = (ctx, args) =>
        {
            var (action, consumed) = ctx.Mock.ClipPacket(From<TrafficClass>(args[0]), From<ushort>(args[1]), From<Direction>(args[2]), Unhex(args[3]!));
            return new JsonObject { ["action"] = ToJson(action), ["consumed"] = consumed };
        },
        ["MockBox.push_raw"] = (ctx, args) => { ctx.Mock.PushRaw(Unhex(args[0]!)); return null; },
        ["MockBox.set_transfer_reply"] = (ctx, args) => { ctx.Mock.SetTransferReply(From<TransferStatus>(args[0]), Unhex(args[1]!)); return null; },
        ["EventStream.recv"] = (ctx, args) => ToJson(ctx.Events!.RecvTimeout(TimeSpan.FromMilliseconds((long)args[0]!))),
        ["EventStream.try_recv"] = (ctx, _) => ToJson(ctx.Events!.TryRecv()),
        ["InputStream.recv"] = (ctx, args) => ToJson(ctx.Inputs!.RecvTimeout(TimeSpan.FromMilliseconds((long)args[0]!))),
        ["InputStream.try_recv"] = (ctx, _) => ToJson(ctx.Inputs!.TryRecv()),
        ["LogStream.recv"] = (ctx, args) => ToJson(ctx.Logs!.RecvTimeout(TimeSpan.FromMilliseconds((long)args[0]!))),
        ["LogStream.try_recv"] = (ctx, _) => ToJson(ctx.Logs!.TryRecv()),
        ["Timeline.observe"] = (ctx, args) => ToJson(ctx.Timeline.Observe(
            new CatchEvent(CatchEventKind.Motion, From<uint>(args[0]!["ts_us"]), From<ClockDomain>(args[0]!["clock"]), null, null, null), From<ulong>(args[1]))),
        ["Timeline.observe_stamp"] = (ctx, args) => ToJson(ctx.Timeline.ObserveStamp(From<uint>(args[0]), From<ClockDomain>(args[1]), From<ulong>(args[2]))),
        ["ClipBuilder.new"] = (ctx, _) => { ctx.Builder.Dispose(); ctx.Builder = new(); return null; },
        ["ClipFrame.new"] = (ctx, _) => { ctx.Frame.Dispose(); ctx.Frame = new(); return null; },
        ["Timeline.new"] = (ctx, _) => { ctx.Timeline.Dispose(); ctx.Timeline = new(); return null; },
    };

    static readonly Dictionary<string, Type> Classes = new()
    {
        ["Device"] = typeof(Device),
        ["Clip"] = typeof(ClipHandle),
        ["ClipBuilder"] = typeof(ClipBuilder),
        ["ClipFrame"] = typeof(ClipFrame),
        ["MockBox"] = typeof(MockBox),
        ["Timeline"] = typeof(Timeline),
        ["EventStream"] = typeof(EventStream),
        ["InputStream"] = typeof(InputStream),
        ["LogStream"] = typeof(LogStream),
        [""] = typeof(MediusLibrary),
    };

    // the C++ names whose C# name isn't the PascalCase of it
    static readonly Dictionary<string, string> Renamed = new()
    {
        ["Clip.finalize"] = "FinalizeClip",
    };

    static object? Target(Context ctx, string cls) => cls switch
    {
        "Device" => ctx.Device,
        "Clip" => ctx.Clip,
        "ClipBuilder" => ctx.Builder,
        "ClipFrame" => ctx.Frame,
        "MockBox" => ctx.Mock,
        "Timeline" => ctx.Timeline,
        "EventStream" => ctx.Events,
        "InputStream" => ctx.Inputs,
        "LogStream" => ctx.Logs,
        _ => null,
    };

    // a generated call, by the C# method, property or constant of the C++ name
    static JsonNode? Reflected(Context ctx, string cls, string method, JsonArray args)
    {
        var type = Classes[cls];
        var name = Renamed.GetValueOrDefault($"{cls}.{method}", Pascal(method));
        var target = Target(ctx, cls);
        if (args.Count == 0 && type.GetProperty(name) is { } property)
        {
            return ToJson(property.GetValue(target));
        }
        if (args.Count == 0 && type.GetField(name, BindingFlags.Public | BindingFlags.Static) is { } field)
        {
            return ToJson(field.GetValue(null));
        }
        var methods = type.GetMethods(BindingFlags.Public | BindingFlags.Instance | BindingFlags.Static)
            .Where(each => each.Name == name && each.GetParameters() is var parameters && parameters.Length >= args.Count &&
                parameters.Skip(args.Count).All(parameter => parameter.IsOptional))
            .ToList();
        Assert.True(methods.Count > 0, $"C# has no {type.Name}.{name} for {cls}::{method}");
        foreach (var each in methods)
        {
            object?[] values;
            try
            {
                values = [.. each.GetParameters().Select((parameter, at) => at < args.Count ? FromJson(args[at], parameter.ParameterType) : parameter.DefaultValue)];
            }
            catch (Exception) when (methods.Count > 1)
            {
                continue;
            }
            var result = each.Invoke(each.IsStatic ? null : target, values);
            // a chained call gives the builder or the frame back, which the other runners don't write
            return each.ReturnType == type || each.ReturnType == typeof(void) ? null : ToJson(result);
        }
        throw new InvalidOperationException($"no {type.Name}.{name} takes {args.ToJsonString()}");
    }

    static JsonNode? Helper(string name, JsonArray args)
    {
        T A<T>(int at) => From<T>(args[at]);
        return name switch
        {
            "Bearing::is_live" => A<Bearing>(0).IsLive,
            "Caps::has_keyboard" => A<Caps>(0).HasKeyboard,
            "Caps::has_mouse" => A<Caps>(0).HasMouse,
            "Caps::is_composite" => A<Caps>(0).IsComposite,
            "Capture::bytes" => ToJson(Capture.Bytes(A<byte>(0))),
            "Capture::widest" => ToJson(Capture.Widest(A<byte>(0), A<byte>(1))),
            "CatchClass: From<Class>" => ToJson(CatchClass.From(A<Class>(0))),
            "CatchClass: From<TrafficClass>" => ToJson(CatchClass.From(A<TrafficClass>(0))),
            "CatchClass::direction_meaning" => ToJson(A<CatchClass>(0).DirectionMeaning),
            "CatchClass::is_input" => A<CatchClass>(0).IsInput,
            "CatchClass::is_traffic" => A<CatchClass>(0).IsTraffic,
            "CatchEvent::bytes" => ToJson(A<CatchEvent>(0).Bytes),
            "CatchEvent::class" => ToJson(A<CatchEvent>(0).Class),
            "CatchEvent::clock" => ToJson(A<CatchEvent>(0).Clock),
            "CatchEvent::direction" => ToJson(A<CatchEvent>(0).Direction),
            "CatchEvent::id" => ToJson(A<CatchEvent>(0).Id),
            "CatchEvent::ts_us" => A<CatchEvent>(0).TsUs,
            "CatchFilter::all_input" => ToJson(CatchFilter.AllInput()),
            "CatchFilter::capture" => A<CatchFilter>(0).Capture,
            "CatchFilter::class" => ToJson(A<CatchFilter>(0).Class),
            "CatchFilter::direction" => ToJson(A<CatchFilter>(0).Direction),
            "CatchFilter::id" => ToJson(A<CatchFilter>(0).Id),
            "CatchFilter::same_address" => A<CatchFilter>(0).SameAddress(A<CatchFilter>(1)),
            "CatchFilter::everything" => ToJson(CatchFilter.Everything()),
            "CatchFilter::watch" => ToJson(CatchFilter.Watch(A<Usage>(0))),
            "CatchFilter::watch_axes" => ToJson(CatchFilter.WatchAxes()),
            "CatchFilter::watch_axis" => ToJson(CatchFilter.WatchAxis(A<Axis>(0))),
            "CatchFilter::watch_class" => ToJson(CatchFilter.WatchClass(A<Class>(0))),
            "CatchFilter::traffic" => ToJson(CatchFilter.Traffic(A<TrafficClass>(0), A<ushort>(1))),
            "CatchFilter::traffic_class" => ToJson(CatchFilter.TrafficOf(A<TrafficClass>(0))),
            "CatchFilter::inbound" => ToJson(A<CatchFilter>(0).Inbound()),
            "CatchFilter::outbound" => ToJson(A<CatchFilter>(0).Outbound()),
            "CatchFilter::on_press" => ToJson(A<CatchFilter>(0).OnPress()),
            "CatchFilter::on_release" => ToJson(A<CatchFilter>(0).OnRelease()),
            "CatchFilter::with_capture" => ToJson(A<CatchFilter>(0).WithCapture(A<byte>(1))),
            "CatchFilter::with_direction" => ToJson(A<CatchFilter>(0).WithDirection(A<Direction>(1))),
            "CatchState::is_empty" => A<CatchState>(0).IsEmpty,
            "ClipStatus::is_held" => A<ClipStatus>(0).IsHeld(A<Usage>(1)),
            "ClockEstimate::drift_us_over" => A<ClockEstimate>(0).DriftUsOver(TimeSpan.FromMicroseconds(A<long>(1))),
            "ClockEstimate::error_bound_us" => A<ClockEstimate>(0).ErrorBoundUs,
            "ClockEstimate::to_host_domain" => ToJson(A<ClockEstimate>(0).ToHostDomain(A<long>(1))),
            "DeviceInfo::is_cloned" => A<DeviceInfo>(0).IsCloned,
            "Direction: From<Edge>" => ToJson(Direction.From(A<Edge>(0))),
            "Direction::admits" => A<Direction>(0).Admits(A<Direction>(1)),
            "Direction::is_relative" => A<Direction>(0).IsRelative,
            "Direction::of_delta" => ToJson(Direction.OfDelta(A<short>(0))),
            "FirmwareInfo::any_pending" => A<FirmwareInfo>(0).AnyPending,
            "ImageState::is_pending" => A<ImageState>(0).IsPending,
            "Input::axes" => ToJson(A<InputEvent>(0).Axes),
            "Input::direction" => ToJson(A<InputEvent>(0).Direction),
            "Input::is_press" => A<InputEvent>(0).IsPress,
            "Input::is_release" => A<InputEvent>(0).IsRelease,
            "Input::usage" => ToJson(A<InputEvent>(0).Usage),
            "Key::is_modifier" => A<Medius.Key>(0).IsModifier,
            "LockEntry::is_block" => A<LockEntry>(0).IsBlock,
            "LockTarget::as_axis" => ToJson(A<LockTarget>(0).AsAxis),
            "Locks::is_locked" => A<Locks>(0).IsLocked(A<LockTarget>(1), A<Direction>(2)),
            "Locks::scale_of" => A<Locks>(0).ScaleOf(A<LockTarget>(1), A<Direction>(2)),
            "MotionEvent::all_axes" => ToJson(A<MotionEvent>(0).AllAxes),
            "MotionEvent::axes" => ToJson(A<MotionEvent>(0).Axes),
            "MouseCaps::is_composite" => A<MouseCaps>(0).IsComposite,
            "Patch::in_config" => ToJson(Patch.InConfig(A<byte>(0), A<ushort>(1), A<ReadOnlyMemory<byte>>(2))),
            "Patch::in_interface" => ToJson(Patch.InInterface(A<byte>(0), A<byte>(1), A<ushort>(2), A<ReadOnlyMemory<byte>>(3))),
            "Patch::in_string" => ToJson(Patch.InString(A<byte>(0), A<ReadOnlyMemory<byte>>(1))),
            "Patch::key" => A<Patch>(0).Key() is var key
                ? new JsonObject { ["section"] = ToJson(key.Section), ["cfg"] = key.Cfg, ["index"] = key.Index, ["offset"] = key.Offset }
                : null,
            // the bits, since the JSON has no floats and the bits compare exactly
            "Rate::native_hz" => A<Rate>(0).NativeHz is { } hz ? BitConverter.SingleToUInt32Bits(hz) : null,
            "RewriteAction::carries_payload" => A<RewriteAction>(0).CarriesPayload,
            "RewriteAction::is_valid_for" => A<RewriteAction>(0).IsValidFor(A<RewriteClass>(1)),
            "RewriteClass::is_control" => A<RewriteClass>(0).IsControl,
            "RewriteRule::key" => A<RewriteRule>(0).Key() is var rule
                ? new JsonObject
                {
                    ["class_"] = ToJson(rule.RewriteClass),
                    ["id"] = rule.Id,
                    ["direction"] = ToJson(rule.Direction),
                    ["match_bytes"] = ToJson(rule.MatchBytes),
                    ["mask"] = ToJson(rule.Mask),
                }
                : null,
            "Setup::is_in" => A<Setup>(0).IsIn,
            "Setup::to_bytes" => ToJson(A<Setup>(0).ToBytes()),
            "TrafficClass: TryFrom<CatchClass>" => ToJson(TrafficClass.From(A<CatchClass>(0))),
            "TrafficEvent::bulk_end_of_transfer" => A<TrafficEvent>(0).BulkEndOfTransfer,
            "TrafficEvent::bus_event" => ToJson(A<TrafficEvent>(0).Bus),
            "TrafficEvent::control_status" => ToJson(A<TrafficEvent>(0).Control),
            "TrafficEvent::data" => ToJson(A<TrafficEvent>(0).Data),
            "TrafficEvent::rule_acted" => A<TrafficEvent>(0).RuleActed,
            "TrafficEvent::setup" => ToJson(A<TrafficEvent>(0).SetupBytes),
            "TrafficEvent::transfer_status" => ToJson(A<TrafficEvent>(0).Transfer),
            "TrafficEvent::truncated" => A<TrafficEvent>(0).Truncated,
            "TrafficEvent::zlp" => A<TrafficEvent>(0).Zlp,
            "TransferOutcome::is_ok" => A<TransferOutcome>(0).IsOk,
            "TransferOutcome::ok_data" => ToJson(A<TransferOutcome>(0).OkData),
            "TransferStatus::is_ok" => A<TransferStatus>(0).IsOk,
            "Transform::key" => A<Transform>(0).Key() is var transform
                ? new JsonObject { ["source"] = ToJson(transform.Source), ["dest"] = ToJson(transform.Dest) }
                : null,
            "Transform::remap" => ToJson(Transform.Remap(A<LockTarget>(0), A<LockTarget>(1))),
            "Transform::swap" => ToJson(Transform.Swap(A<Axis>(0), A<Axis>(1))),
            "TransformOp::admits" => A<TransformOp>(0).Admits(A<LockTarget>(1), A<LockTarget>(2)),
            "UpdateProgress::percent" => new UpdateProgress(From<UpdateTarget>(args[0]!["target"]), From<nuint>(args[0]!["sent"]),
                From<nuint>(args[0]!["total"])).Percent,
            "UsageSnapshot::is_held" => A<UsageSnapshot>(0).IsHeld(A<Usage>(1)),
            "Version::mac_hex" => A<BoxVersion>(0).MacHex,
            _ => throw new InvalidOperationException($"the runner has no helper {name}"),
        };
    }

    static JsonNode? Text(string name, JsonNode? value) => name switch
    {
        "ChipFirmware" => From<ChipFirmware>(value).ToString(),
        "DeviceInfo" => From<DeviceInfo>(value).ToString(),
        "DeviceKind" => From<DeviceKind>(value).Text,
        "ImageState" => From<ImageState>(value).Text,
        "UpdateTarget" => From<UpdateTarget>(value).Text,
        "Version" => From<BoxVersion>(value).ToString(),
        _ => throw new InvalidOperationException($"the runner has no text for {name}"),
    };

    // a type's default the way C# gives it, its Default, a new one, its default value, or the record field that holds it
    static JsonNode? Default(string name) => name switch
    {
        "Bearing" => ToJson(Bearing.Default),
        "Caps" => ToJson(Caps.Default),
        "CountersSnapshot" => ToJson(Counters.Default),
        "DeviceInfo" => ToJson(DeviceInfo.Default),
        "EmitPaceStatus" => ToJson(EmitPaceStatus.Default),
        "ImperfectStatus" => ToJson(ImperfectStatus.Default),
        "KbdCaps" => ToJson(KbdCaps.Default),
        "MouseCaps" => ToJson(MouseCaps.Default),
        "Patch" => ToJson(Patch.Default),
        "PatchSet" => ToJson(PatchSet.Default),
        "RenderStatus" => ToJson(RenderStatus.Default),
        "RewriteRule" => ToJson(RewriteRule.Default),
        "RewriteTable" => ToJson(RewriteTable.Default),
        "SpreadStatus" => ToJson(SpreadStatus.Default),
        "Transforms" => ToJson(Transforms.Default),
        "ClipStatus" => ToJson(ClipStatus.Default),
        "Setup" => ToJson(Setup.Default),
        "ClipSettings" => ToJson(new ClipSettings()),
        "EmitPace" => ToJson(EmitPace.Learned),
        "ClipBuilder" => ToJson(new ClipBuilder().ByteLen),
        "ClipFrame" => ToJson(new ClipFrame().ByteLen),
        "Capture" => ToJson(Capture.Whole),
        "BearingMode" => ToJson(default(BearingMode)),
        "ClipState" => ToJson(default(ClipState)),
        "DeviceKind" => ToJson(default(DeviceKind)),
        "Direction" => ToJson(default(Direction)),
        "MoveTiming" => ToJson(default(MoveTiming)),
        "PatchSection" => ToJson(default(PatchSection)),
        "PendingMotion" => ToJson(default(PendingMotion)),
        "RenderMode" => ToJson(RenderStatus.Default.Mode),
        "RewriteAction" => ToJson(default(RewriteAction)),
        "RewriteClass" => ToJson(RewriteRule.Default.RewriteClass),
        _ => throw new InvalidOperationException($"the runner has no default for {name}"),
    };

    static JsonNode? Step(Context ctx, JsonNode step)
    {
        var args = step["args"]?.AsArray() ?? [];
        if (step["helper"] is { } helper)
        {
            return Helper((string)helper!, args);
        }
        if (step["text"] is { } text)
        {
            return Text((string)text!, step["value"]);
        }
        if (step["default"] is { } type)
        {
            return Default((string)type!);
        }
        var cls = (string?)step["on"] ?? "Device";
        var method = (string)step["call"]!;
        if (OnHardware.Contains($"{cls}::{method}"))
        {
            throw new InvalidOperationException($"{cls}::{method} only runs on a real box, on the mock it would reach whatever box is plugged in");
        }
        return Hand.TryGetValue($"{cls}.{method}", out var hand) ? hand(ctx, args) : Reflected(ctx, cls, method, args);
    }

    // the calls the ledger holds on hardware, by the C++ name the scenarios use. a mock device's reconnect still
    // rescans the real serial ports and takes the first box that opens, so none of them runs here, like in the recorder
    static readonly HashSet<string> OnHardware = [.. MockLibrary.ReadFile("bindings/conformance/medius-crate.txt")
        .Split('\n')
        .Select(line => line.Split('|').Select(field => field.Trim()).ToArray())
        .Where(fields => fields.Length >= 4 && !fields[0].StartsWith('#') && fields[1] == "hardware")
        .Select(fields => fields[3])];

    [Fact]
    public void ACallHeldOnHardwareNeverReachesTheMock()
    {
        Assert.Contains("Device::reconnect", OnHardware);
        using var ctx = new Context();
        // open stands in for reconnect, so this can't reach a box even with the check gone
        var refused = Assert.Throws<InvalidOperationException>(() => Step(ctx, JsonNode.Parse("""{"call": "open"}""")!));
        Assert.Contains("only runs on a real box", refused.Message);
    }

    // how a step ended, a value, medius' error, or a refusal before medius
    static JsonObject Outcome(Func<JsonNode?> step)
    {
        try
        {
            return new JsonObject { ["value"] = step() };
        }
        catch (Exception thrown)
        {
            var error = thrown is TargetInvocationException { InnerException: { } inner } ? inner : thrown;
            switch (error)
            {
                case MediusException medius:
                    var detail = new JsonObject
                    {
                        ["status"] = ToJson(medius.MediusStatus),
                        ["text"] = medius.Message,
                        ["proto_ver"] = medius.ProtoVer,
                    };
                    if (medius is UpdateException { Op: { } op } update)
                    {
                        detail["update"] = new JsonObject { ["op"] = op, ["status"] = update.UpdateStatus, ["arg"] = update.Arg };
                    }
                    return new JsonObject { ["error"] = detail };
                // only the package refusing counts, an argument the runner itself got wrong has to fail the test
                case ArgumentException when error.TargetSite?.DeclaringType?.Assembly == typeof(Device).Assembly:
                    return new JsonObject { ["refused"] = true };
                default:
                    throw error;
            }
        }
    }

    // the frames the mock got since `seen`, without the keepalive's and nothing after the box restarts, the same way
    // the recorder leaves them out
    static JsonArray Frames(Context ctx, ref int seen, List<JsonNode> sent, ref bool restarted)
    {
        var frames = new JsonArray();
        var total = ctx.Mock.Recorded;
        // clear_recorded empties the list, so the count starts over
        if (total < seen)
        {
            seen = 0;
        }
        for (; seen < total && !restarted; seen++)
        {
            var frame = ctx.Mock.RecordedFrame(seen)!;
            JsonNode value = new JsonObject { ["type"] = ToJson(frame.Type), ["payload"] = Hex(frame.Payload.Span) };
            var keepalive = frame.Type switch
            {
                FrameType.Query => frame.Payload.Length == 1 && frame.Payload.Span[0] is 5 or 10,
                FrameType.Catch or FrameType.Rewrite or FrameType.Transform => sent.Any(each => JsonNode.DeepEquals(each, value)),
                _ => false,
            };
            if (!keepalive)
            {
                sent.Add(value.DeepClone());
                frames.Add(value);
            }
            // the chip boots on a RESET with the NVS flag (0x01) and on a reboot into the device chip's firmware
            restarted = frame.Payload.Length > 0 && frame.Type switch
            {
                FrameType.Reset => (frame.Payload.Span[0] & 0x01) != 0,
                FrameType.RebootDl => frame.Payload.Span[0] == (byte)RebootTarget.DeviceRun,
                _ => false,
            };
        }
        return frames;
    }

    [Fact]
    public void OnlyThePackageRefusingCountsAsARefusal()
    {
        var estimate = From<ClockEstimate>(JsonNode.Parse("""{"offset_us": 0, "rate_ppb": 1, "delay_us": 0, "age_ms": 1}"""));
        Assert.True((bool)Outcome(() => estimate.DriftUsOver(TimeSpan.FromSeconds(-1)))["refused"]!);
        Assert.Throws<ArgumentException>(() => Outcome(() => throw new ArgumentException("the runner got its own argument wrong")));
    }

    [Theory]
    [MemberData(nameof(Names))]
    public void TheScenarioDoesWhatTheMediusCrateDoes(string name)
    {
        var scenario = JsonNode.Parse(MockLibrary.ReadFile("bindings/conformance/medius.json"))!.AsArray()
            .Single(each => (string)each!["name"]! == name)!;
        var recorded = Expected().SingleOrDefault(each => (string)each!["name"]! == name);
        Assert.True(recorded is not null, $"medius-expected.json has no scenario called {name}");
        using var ctx = new Context();
        foreach (var setup in scenario["setup"]?.AsArray() ?? [])
        {
            var step = new JsonObject { ["on"] = "MockBox", ["call"] = setup!["call"]!.DeepClone(), ["args"] = setup["args"]?.DeepClone() };
            Step(ctx, step);
        }
        var seen = 0;
        var sent = new List<JsonNode>();
        var restarted = false;
        var opened = Outcome(() =>
        {
            ctx.Device = (bool?)scenario["bare"] == true ? Device.WithMock(ctx.Mock) : Device.OpenMock(ctx.Mock);
            return null;
        });
        var open = new JsonObject { ["outcome"] = opened, ["frames"] = Frames(ctx, ref seen, sent, ref restarted) };
        Assert.True(JsonNode.DeepEquals(recorded!["open"], open), $"the open differs from Rust\nwant {recorded["open"]!.ToJsonString()}\ngot  {open.ToJsonString()}");
        var want = recorded["steps"]!.AsArray();
        var steps = scenario["steps"]!.AsArray();
        Assert.Equal(want.Count, steps.Count);
        for (var index = 0; index < steps.Count; index++)
        {
            JsonObject got;
            try
            {
                got = Run(ctx, steps[index]!, ref seen, sent, ref restarted);
            }
            catch (Exception thrown)
            {
                throw new InvalidOperationException($"step {index} ({steps[index]!.ToJsonString()}) threw", thrown);
            }
            Assert.True(JsonNode.DeepEquals(want[index], got),
                $"step {index} ({steps[index]!.ToJsonString()}) differs from Rust\nwant {want[index]!.ToJsonString()}\ngot  {got.ToJsonString()}");
        }
    }

    // one step's outcome and the frames the box got during it
    static JsonObject Run(Context ctx, JsonNode step, ref int seen, List<JsonNode> sent, ref bool restarted)
    {
        var outcome = Outcome(() => Step(ctx, step));
        // the mock's own restart boots the chip without a frame
        restarted |= (string?)step["on"] == "MockBox" && (string?)step["call"] == "restart";
        return new JsonObject { ["outcome"] = outcome, ["frames"] = Frames(ctx, ref seen, sent, ref restarted) };
    }

    [Fact]
    public void NothingAfterARestartCounts()
    {
        using var ctx = new Context();
        ctx.Device = Device.OpenMock(ctx.Mock);
        var (seen, sent, restarted) = (ctx.Mock.Recorded, new List<JsonNode>(), false);
        Step(ctx, JsonNode.Parse("""{"call": "factory_reset"}""")!);
        Step(ctx, JsonNode.Parse("""{"call": "query_version"}""")!);
        // the query and whatever medius sends to recover come after the restart
        var frames = Frames(ctx, ref seen, sent, ref restarted);
        var want = JsonNode.Parse("""[{"type": "Catch", "payload": "ffffff000000"}, {"type": "Reset", "payload": "01"}]""");
        Assert.True(JsonNode.DeepEquals(want, frames) && restarted, frames.ToJsonString());

        using var other = new Context();
        other.Device = Device.OpenMock(other.Mock);
        (seen, sent, restarted) = (other.Mock.Recorded, new List<JsonNode>(), false);
        Run(other, JsonNode.Parse("""{"on": "MockBox", "call": "restart"}""")!, ref seen, sent, ref restarted);
        var after = Run(other, JsonNode.Parse("""{"call": "query_version"}""")!, ref seen, sent, ref restarted);
        Assert.True(after["frames"]!.AsArray().Count == 0, after.ToJsonString());
    }
}
