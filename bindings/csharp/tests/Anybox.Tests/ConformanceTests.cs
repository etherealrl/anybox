using System.Reflection;
using System.Text.Json.Nodes;
using System.Text.RegularExpressions;

namespace Anybox.Tests;

// the conformance suite's C# runner: every scenario in bindings/conformance/scenarios.json through this package and
// the mock, held to what the Rust crate itself did with it, which the capi crate's tests write to
// target/conformance/expected.json. bindings/c/src/conformance.rs says what a scenario is
public sealed class ConformanceTests
{
    static readonly Dictionary<string, long> Header = Regex
        .Matches(MockLibrary.ReadFile("bindings/c/include/anybox.h"), @"^#define ANYBOX_(\w+) (-?\d+)\r?$", RegexOptions.Multiline)
        .ToDictionary(match => match.Groups[1].Value, match => long.Parse(match.Groups[2].Value));

    // the Rust calls this package folds into another method, with the `Click` argument as optional parameters
    static readonly Dictionary<string, string> Folded = new()
    {
        // made by bindings/gen from its call table (folded), change the table and run cargo run -p anybox-gen
        ["click_with"] = "Click",
        ["tap_key_with"] = "TapKey",
        ["tap_media_with"] = "TapMedia",
        // the end of what bindings/gen makes (folded)
    };

    // the anybox.h prefix of each value type, a value compares by its name there, so the C ABI's number for it is
    // checked against the Rust name the recorder wrote
    static readonly Dictionary<Type, string> Prefixes = new()
    {
        [typeof(Button)] = "BUTTON",
        [typeof(Axis)] = "AXIS",
        [typeof(Direction)] = "DIRECTION",
        [typeof(BoxAction)] = "ACTION",
        [typeof(Family)] = "FAMILY",
        [typeof(FerrumApi)] = "FERRUM_API",
        [typeof(Clock)] = "CLOCK",
        [typeof(DropSource)] = "DROP_SOURCE",
        [typeof(Execution)] = "EXECUTION",
        [typeof(QuirkKind)] = "QUIRK",
        [typeof(IoKind)] = "IO_KIND",
        [typeof(ClickTimingKind)] = "CLICK_TIMING",
    };

    // what comes in after the events a step waits for, so an extra one shows up instead of being left in the queue
    static readonly TimeSpan Quiet = TimeSpan.FromMilliseconds(150);

    // what medius' own keepalive sends every 500 ms while something's held, a stats query and the subscriptions
    // again, which no call sends, so the recorder and every runner leave them out
    static bool Keepalive(byte type, byte[] payload) => (type == 0x05 && payload is [0x05]) || type == 0x0B;

    static string Name(string prefix, long number)
    {
        var names = Header.Where(entry => entry.Key.StartsWith(prefix + "_") && entry.Value == number)
            .Select(entry => entry.Key[(prefix.Length + 1)..]).ToList();
        Assert.True(names.Count == 1, $"anybox.h has [{string.Join(", ", names)}] for {prefix} {number}");
        return names[0];
    }

    static JsonNode ButtonName(Button button)
    {
        var extra = Header["BUTTON_EXTRA_BASE"];
        return (long)button >= extra ? new JsonObject { ["Extra"] = (long)button - extra } : Name("BUTTON", (long)button);
    }

    public static TheoryData<string> Names() => [.. JsonNode.Parse(MockLibrary.ReadFile("bindings/conformance/scenarios.json"))!
        .AsArray().Select(scenario => (string)scenario!["name"]!)];

    static JsonArray Expected()
    {
        var path = Path.Combine(MockLibrary.Workspace, "target", "conformance", "expected.json");
        Assert.True(File.Exists(path), $"there's no {path}, run the capi crate's tests first, they record it");
        return JsonNode.Parse(File.ReadAllText(path))!.AsArray();
    }

    static string Pascal(string snake) => string.Concat(snake.Split('_').Select(part => char.ToUpperInvariant(part[0]) + part[1..]));

    static long Number(string prefix, JsonNode value) => value switch
    {
        JsonValue text when text.TryGetValue<string>(out var name) => Header[$"{prefix}_{name}"],
        JsonObject extra => Header["BUTTON_EXTRA_BASE"] + (long)extra["Extra"]!,
        _ => (long)value,
    };

    static object Arg(JsonNode node)
    {
        var (kind, raw) = node.AsObject().Single();
        return kind switch
        {
            "i16" => (short)(long)raw!,
            "Button" => (Button)Number("BUTTON", raw!),
            "Key" => (Key)Number("KEY", raw!),
            "MediaKey" => (MediaKey)Number("MEDIA", raw!),
            "Axis" => (Axis)Number("AXIS", raw!),
            "Direction" => (Direction)Number("DIRECTION", raw!),
            "Text" => (string)raw!,
            "Keys" => raw!.AsArray().Select(key => (Key)Number("KEY", key!)).ToArray(),
            _ => throw new InvalidOperationException($"the runner doesn't know the value {node}"),
        };
    }

    // the text of a value given like a call's argument, or of a firmware or an id given the way a snapshot writes them
    static string Text(JsonNode node)
    {
        var (kind, raw) = node.AsObject().Single();
        return kind switch
        {
            "Action" => ((BoxAction)Number("ACTION", raw!)).Text,
            "Family" => ((Family)Number("FAMILY", raw!)).Text,
            "FerrumApi" => ((FerrumApi)Number("FERRUM_API", raw!)).Text,
            "Button" => ((Button)Number("BUTTON", raw!)).Text,
            "Key" => ((Key)Number("KEY", raw!)).Text,
            "MediaKey" => ((MediaKey)Number("MEDIA", raw!)).Text,
            "Axis" => ((Axis)Number("AXIS", raw!)).Text,
            "Firmware" => FirmwareOf(raw!.AsObject()).ToString(),
            "StableId" => (raw!["mac"] is { } mac
                ? StableId.FromMac(Convert.FromHexString((string)mac!))
                : StableId.FromUsbSerial((string)raw["usb_serial"]!)).ToString(),
            _ => throw new InvalidOperationException($"the runner has no text for {node}"),
        };
    }

    static Firmware FirmwareOf(JsonObject raw)
    {
        var (field, held) = raw.Single();
        return field switch
        {
            "version" => new Firmware(FirmwareKind.Version) { Version = new Version((int)held![0]!, (int)held[1]!, (int)held[2]!) },
            "build" => new Firmware(FirmwareKind.Build) { Build = (uint)held! },
            "protocol" => new Firmware(FirmwareKind.Protocol) { Protocol = (byte)held! },
            _ => new Firmware(FirmwareKind.Text) { Text = (string)held! },
        };
    }

    delegate void KeysCall(ReadOnlySpan<Key> keys);

    static void Call(Device device, string name, JsonArray args)
    {
        object?[] values;
        MethodInfo method;
        if (Folded.TryGetValue(name, out var folded))
        {
            var click = args[1]!["Click"]!.AsObject();
            TimeSpan? hold = click["hold_us"] is { } us ? TimeSpan.FromMicroseconds((long)us) : null;
            values = [Arg(args[0]!), hold, click["native_only"] is { } only && (bool)only];
            method = typeof(Device).GetMethod(folded)!;
        }
        else
        {
            method = typeof(Device).GetMethod(Pascal(name))!;
            // reflection can't hand over a span, a delegate can
            if (method.GetParameters() is [{ ParameterType: var type }] && type == typeof(ReadOnlySpan<Key>))
            {
                method.CreateDelegate<KeysCall>(device)((Key[])Arg(args[0]!)!);
                return;
            }
            var given = args.Select(arg => Arg(arg!)).ToList();
            // a parameter the scenario leaves out keeps its default, like the Rust call that has none
            values = [.. method.GetParameters().Select((parameter, index) => index < given.Count ? given[index] : parameter.DefaultValue)];
        }
        try
        {
            method.Invoke(device, values);
        }
        catch (TargetInvocationException error) when (error.InnerException is not null)
        {
            System.Runtime.ExceptionServices.ExceptionDispatchInfo.Throw(error.InnerException);
        }
    }

    static JsonNode Firmware(Firmware firmware) => firmware.Kind switch
    {
        FirmwareKind.Version => new JsonObject { ["version"] = new JsonArray(firmware.Version!.Major, firmware.Version.Minor, firmware.Version.Build) },
        FirmwareKind.Build => new JsonObject { ["build"] = firmware.Build },
        FirmwareKind.Protocol => new JsonObject { ["protocol"] = firmware.Protocol },
        _ => new JsonObject { ["text"] = firmware.Text },
    };

    static JsonNode Id(StableId id) => id.Mac is { } mac
        ? new JsonObject { ["mac"] = Convert.ToHexStringLower(mac.GetAddressBytes()) }
        : new JsonObject { ["usb_serial"] = id.UsbSerial };

    // one field's value in the suite's form
    static JsonNode? Value(object? value) => value switch
    {
        null => null,
        bool flag => flag,
        string text => text,
        Button button => ButtonName(button),
        Enum named when Prefixes.TryGetValue(named.GetType(), out var prefix) => Name(prefix, Convert.ToInt64(named)),
        // a key and a media key are their HID usage, the number is the value
        Enum number => Convert.ToInt64(number),
        byte or ushort or short or int or uint or long or ulong => JsonValue.Create(Convert.ToDecimal(value)),
        Firmware firmware => Firmware(firmware),
        StableId id => Id(id),
        Stamp stamp => new JsonObject
        {
            ["box_time"] = stamp.BoxTime is { } box ? new JsonObject { ["micros"] = box.Micros, ["clock"] = Value(box.Clock) } : null,
        },
        IEnumerable<Incapable> boxes => new JsonArray([.. boxes.Select(box => new JsonObject
        {
            ["port"] = box.Box.Info.Port,
            ["missing"] = new JsonArray([.. box.Missing.Select(action => Name("ACTION", (long)action)).Order(StringComparer.Ordinal).Select(name => (JsonNode)name)]),
        })]),
        IEnumerable<string> texts => new JsonArray([.. texts.Select(text => (JsonNode)text)]),
        _ => throw new InvalidOperationException($"the runner doesn't know the value {value}"),
    };

    static JsonObject Fields(object source, JsonObject expected) => new([.. expected.Select(field =>
        KeyValuePair.Create(field.Key, Value(source.GetType().GetProperty(Pascal(field.Key))!.GetValue(source))))]);

    static JsonObject Error(AnyboxException error, JsonNode expected) => new()
    {
        ["error"] = new JsonObject
        {
            ["status"] = Name("STATUS", error.Status!.Value),
            ["text"] = error.Message,
            ["fields"] = Fields(error, expected["error"]!["fields"]!.AsObject()),
        },
    };

    static JsonObject Event(Event @event, JsonNode expected)
    {
        var name = @event.GetType().Name;
        return new()
        {
            ["event"] = name.EndsWith("Event") ? name[..^"Event".Length] : name,
            ["fields"] = Fields(@event, expected["fields"]!.AsObject()),
        };
    }

    static ulong Us(TimeSpan? span) => span is { } value ? (ulong)(value.Ticks / TimeSpan.TicksPerMicrosecond) : 0;

    static JsonObject Snapshot(Snapshot snapshot, JsonNode expected)
    {
        var info = snapshot.Info;
        var capabilities = new JsonArray();
        foreach (var want in expected["snapshot"]!["capabilities"]!.AsArray())
        {
            var action = (string)want!["action"]!;
            var capability = snapshot.Capabilities[(BoxAction)Header[$"ACTION_{action}"]];
            capabilities.Add(new JsonObject
            {
                ["action"] = action,
                ["execution"] = Value(capability.Execution),
                ["reason"] = capability.Reason,
                ["quirks"] = new JsonArray([.. capability.Quirks.Select(quirk => new JsonObject
                {
                    ["kind"] = Value(quirk.Kind),
                    ["min_us"] = Us(quirk.Min),
                    ["max_us"] = Us(quirk.Max),
                    ["count"] = quirk.Count ?? 0,
                    ["click_timing"] = new JsonObject
                    {
                        ["kind"] = quirk.ClickTiming is { } timing ? Value(timing.Kind) : null,
                        ["fixed_us"] = Us(quirk.ClickTiming?.Hold),
                    },
                })]),
            });
        }
        return new()
        {
            ["snapshot"] = new JsonObject
            {
                ["info"] = new JsonObject
                {
                    ["port"] = info.Port,
                    ["family"] = Value(info.Family),
                    ["model"] = info.Model,
                    ["firmware"] = info.Firmware is { } firmware ? Firmware(firmware) : null,
                    ["id"] = info.Id is { } id ? Id(id) : null,
                    ["ferrum_api"] = Value(info.FerrumApi),
                    ["peripheral"] = info.Peripheral is { } ids ? new JsonObject { ["vid"] = ids.Vid, ["pid"] = ids.Pid } : null,
                },
                ["capabilities"] = capabilities,
                ["buttons"] = new JsonArray([.. snapshot.Capabilities.Buttons.Select(ButtonName)]),
                ["axes"] = new JsonArray([.. snapshot.Capabilities.Axes.Select(axis => Value(axis))]),
            },
        };
    }

    static void Push(Mock box, JsonNode step)
    {
        var seq = step["seq"] is { } frame ? (byte)(long)frame : (byte)0;
        var ts = step["ts_us"] is { } us ? (uint)(long)us : 0;
        var pressed = step["pressed"] is { } flag && (bool)flag;
        switch ((string)step["push"]!)
        {
            case "buttons":
                box.PushButtons(seq, ts, pressed, (uint)(long)step["held"]!);
                break;
            case "keys":
                box.PushKeys(seq, ts, pressed, [.. step["held"]!.AsArray().Select(key => (Key)Number("KEY", key!))]);
                break;
            case "media":
                box.PushMedia(seq, ts, pressed, [.. step["held"]!.AsArray().Select(key => (MediaKey)Number("MEDIA", key!))]);
                break;
            case "motion":
                box.PushMotion(seq, ts, (short)(long)step["dx"]!, (short)(long)step["dy"]!, (short)(long)step["wheel"]!, (short)(long)step["pan"]!);
                break;
            case "link_lost":
                box.LinkLost();
                break;
            case "restart":
                box.Restart();
                break;
            default:
                throw new InvalidOperationException($"the mock can't push {step["push"]}");
        }
    }

    static JsonObject Recv(Events events, Mock box, JsonNode expected, bool tick)
    {
        var wanted = expected["events"]!.AsArray();
        var got = new JsonArray();
        var clock = System.Diagnostics.Stopwatch.StartNew();
        System.Diagnostics.Stopwatch? quiet = null;
        while (true)
        {
            if (got.Count >= wanted.Count && (quiet ??= System.Diagnostics.Stopwatch.StartNew()).Elapsed >= Quiet)
            {
                break;
            }
            Assert.True(clock.Elapsed < TimeSpan.FromSeconds(10), $"only {got.Count} of {wanted.Count} events came");
            // no ticks while it waits for extras, so only what the scenario set off can come
            if (tick && got.Count < wanted.Count)
            {
                box.Tick();
            }
            if (events.Recv(TimeSpan.FromMilliseconds(50)) is { } next)
            {
                // one Rust didn't send is only its kind, it has nothing to read its fields by
                got.Add(got.Count < wanted.Count ? Event(next, wanted[got.Count]!) : new JsonObject { ["extra"] = next.GetType().Name });
            }
        }
        return new() { ["events"] = got };
    }

    static long? Ms(TimeSpan? span) => span is { } value ? value.Ticks / TimeSpan.TicksPerMillisecond : null;

    static JsonObject Interpolation(Interpolation value) =>
        new() { ["Interpolation"] = value.Automatic ? "AUTO" : value.Percentage };

    static JsonObject Ysc(YscInterpolation value) => new()
    {
        ["enabled"] = value.Enabled,
        ["profile"] = (uint)value.Profile,
        ["window_ms"] = Ms(value.Window),
        ["max_window_ms"] = Ms(value.MaxWindow),
        ["measured_interval_us"] = value.MeasuredInterval is { } interval ? interval.Ticks / TimeSpan.TicksPerMicrosecond : null,
    };

    // one call only one family has, {"none": null} when the device isn't that family
    static JsonObject Extension(Device device, JsonNode step)
    {
        var parts = ((string)step["extension"]!).Split('.');
        var args = step["args"]!.AsArray().Select(arg => arg!.AsObject().Single().Value).ToArray();
        TimeSpan? Window(int index) => args[index] is { } ms ? TimeSpan.FromMilliseconds((long)ms) : null;
        try
        {
            JsonNode? value = parts switch
            {
                ["makcu", var call] when device.Makcu() is { } makcu => call switch
                {
                    "interpolation" => Interpolation(makcu.Interpolation()),
                    "set_interpolation" => Done(() => makcu.SetInterpolation(args[0] is JsonValue auto && auto.TryGetValue<string>(out _)
                        ? Anybox.Interpolation.Auto
                        : Anybox.Interpolation.Percent((byte)(long)args[0]!))),
                    "moving" => makcu.Moving(),
                    "autorelease" => Ms(makcu.Autorelease()),
                    "set_autorelease" => Done(() => makcu.SetAutorelease(Window(0))),
                    _ => throw new InvalidOperationException($"there's no MAKCU call {call}"),
                },
                ["ysc", var call] when device.Ysc() is { } ysc => call switch
                {
                    "interpolation" => Ysc(ysc.Interpolation()),
                    "set_interpolation" => Done(() => ysc.SetInterpolation(new YscInterpolationSettings(
                        (bool)args[0]!["enabled"]!,
                        (YscProfile)(uint)(long)args[0]!["profile"]!,
                        TimeSpan.FromMilliseconds((long)args[0]!["window_ms"]!),
                        TimeSpan.FromMilliseconds((long)args[0]!["max_window_ms"]!)))),
                    "reset_interpolation" => Ysc(ysc.ResetInterpolation()),
                    "serial" => ysc.Serial() is var serial
                        ? new JsonObject { ["custom"] = serial.Custom, ["physical"] = serial.Physical, ["active"] = serial.Active }
                        : null,
                    _ => throw new InvalidOperationException($"there's no YSC call {call}"),
                },
                ["ferrum", "catch_xy"] when device.Ferrum() is { } ferrum =>
                    ferrum.CatchXy(Window(0)!.Value, (bool)args[1]!) is var (x, y) ? new JsonArray(x, y) : null,
                _ => throw new NoHandle(),
            };
            return new() { ["ok"] = value };
        }
        catch (NoHandle)
        {
            return new() { ["none"] = null };
        }
        catch (AnyboxException error)
        {
            var expected = step["outcome"];
            return expected?["error"] is not null ? Error(error, expected) : new JsonObject { ["error"] = error.Message };
        }
    }

    static JsonNode? Done(Action call)
    {
        call();
        return null;
    }

    sealed class NoHandle : Exception;

    static List<JsonNode> Run(JsonNode scenario)
    {
        if (scenario["box"] is { } family)
        {
            var timing = ClickTiming.Fixed(TimeSpan.FromMilliseconds(5));
            using var mock = (string)family! switch
            {
                "makcu" => Mock.Makcu(timing),
                "ysc" => Mock.Ysc(timing),
                "ferrum" => Mock.Ferrum(timing),
                var other => throw new InvalidOperationException($"there's no mock {other}"),
            };
            return [.. scenario["steps"]!.AsArray().Select(step =>
            {
                var out_ = step!.DeepClone().AsObject();
                out_["outcome"] = Extension(mock, step);
                return (JsonNode)out_;
            })];
        }
        var (device, box) = Mock.MediusManual(ClickTiming.Fixed(TimeSpan.FromMilliseconds(5)), (bool)scenario["keyboard"]!);
        Events? events = null;
        try
        {
            if (scenario["events"] is { } kinds)
            {
                var wanted = (EventKinds)Header[$"EVENTS_{(string)kinds!}"];
                events = device.Events(wanted);
                // a mock with no keyboard has no keys or media to subscribe
                var streamed = (bool)scenario["keyboard"]! ? wanted : wanted & (EventKinds.Buttons | EventKinds.Motion);
                Assert.True(SpinWait.SpinUntil(() => box.Subscribed(streamed), TimeSpan.FromSeconds(5)), "the receiver never subscribed");
            }
            var steps = new List<JsonNode>();
            foreach (var step in scenario["steps"]!.AsArray())
            {
                var expected = step!["outcome"];
                var out_ = step.DeepClone().AsObject();
                out_.Remove("outcome");
                out_.Remove("frames");
                JsonNode? outcome;
                if (step["call"] is { } call)
                {
                    box.ClearRecorded();
                    var started = System.Diagnostics.Stopwatch.StartNew();
                    try
                    {
                        Call(device, (string)call!, step["args"]!.AsArray());
                        outcome = new JsonObject { ["ok"] = null };
                    }
                    catch (AnyboxException error)
                    {
                        outcome = expected?["error"] is not null ? Error(error, expected) : new JsonObject { ["error"] = error.Message };
                    }
                    // a hold on a box that can't time it is a wait on the PC, which no frame shows
                    if (step["takes_at_least_us"] is { } atLeast)
                    {
                        Assert.True(started.Elapsed >= TimeSpan.FromMicroseconds((long)atLeast), $"{step.ToJsonString()} only took {started.Elapsed}");
                    }
                    out_["frames"] = new JsonArray([.. box.Recorded()
                        .Where(frame => !Keepalive(frame.Type, frame.Payload))
                        .Select(frame => new JsonArray(frame.Type, Convert.ToHexStringLower(frame.Payload)))]);
                }
                else if (step["extension"] is not null)
                {
                    outcome = Extension(device, step);
                }
                else if (step["snapshot"] is not null)
                {
                    outcome = Snapshot(device.Snapshot(), expected!);
                }
                else if (step["push"] is not null)
                {
                    Push(box, step);
                    outcome = null;
                }
                else if (step["wait_us"] is { } wait)
                {
                    Thread.Sleep(TimeSpan.FromMicroseconds((long)wait));
                    outcome = null;
                }
                else if (step["recv"] is not null)
                {
                    outcome = Recv(events!, box, expected!, tick: false);
                }
                else if (step["tick_until"] is not null)
                {
                    outcome = Recv(events!, box, expected!, tick: true);
                }
                else if (step["example_events"] is not null)
                {
                    Mock.ExampleEvents(device);
                    outcome = null;
                }
                else if (step["text"] is { } value)
                {
                    outcome = new JsonObject { ["text"] = Text(value) };
                }
                else if (step["example"] is { } example)
                {
                    var status = (uint)Header[$"STATUS_{(string)example!}"];
                    var type = AnyboxException.For(status, "").GetType();
                    var made = (AnyboxException)typeof(Mock).GetMethod(nameof(Mock.ExampleError))!.MakeGenericMethod(type).Invoke(null, null)!;
                    outcome = Error(made, expected!);
                }
                else
                {
                    throw new InvalidOperationException($"the runner doesn't know the step {step}");
                }
                out_["outcome"] = outcome;
                steps.Add(out_);
            }
            return steps;
        }
        finally
        {
            events?.Dispose();
            device.Dispose();
            box.Dispose();
        }
    }

    [Theory]
    [MemberData(nameof(Names))]
    public void ThePackageDoesWhatTheRustCrateDoes(string name)
    {
        var expected = Expected().Single(scenario => (string)scenario!["name"]! == name)!;
        var actual = Run(expected);
        var steps = expected["steps"]!.AsArray();
        Assert.Equal(steps.Count, actual.Count);
        for (var index = 0; index < steps.Count; index++)
        {
            Assert.True(JsonNode.DeepEquals(steps[index], actual[index]),
                $"step {index} of '{name}' differs from Rust\nwant {steps[index]!.ToJsonString()}\ngot  {actual[index].ToJsonString()}");
        }
    }
}
