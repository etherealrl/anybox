// the emitters for the calls only one family has. each language turns its own values into the C structs with a few
// hand-written helpers next to the region (`interpolation`, `_millis`, `ExtensionValues`, `detail::raw` and the
// like), so what's made here stays one shape for every call

use crate::table::{Arg, Call, Extension, Ret, Ty};
use crate::{C_WIDTH, CPP_WIDTH, CSHARP_WIDTH, Lang, PYTHON_WIDTH, camel, doc, pascal, wrap};

pub fn of(call: &Call) -> Extension {
    call.on
        .handle()
        .extension
        .expect("a device call has no extension")
}

/// the call's name in the conformance suite, `makcu.interpolation`
pub fn identity(call: &Call) -> String {
    format!("{}.{}", of(call).accessor, call.name)
}

fn function(call: &Call) -> String {
    format!("{}{}", call.on.handle().c_prefix, call.name)
}

/// the argument's name in C, a duration is in milliseconds there
pub fn c_name(arg: &Arg) -> String {
    match arg.ty {
        Ty::Ms | Ty::MsOrOff => format!("{}_ms", arg.name),
        _ => arg.name.to_string(),
    }
}

/// the out parameters a result takes in C, name and C type
fn outs(ret: Ret) -> &'static [(&'static str, &'static str)] {
    match ret {
        Ret::Unit => &[],
        Ret::Bool => &[("out", "bool")],
        Ret::Interpolation => &[("out", "AnyboxMakcuInterpolation")],
        Ret::YscInterpolation => &[("out", "AnyboxYscInterpolation")],
        Ret::YscSerial => &[("out", "*mut AnyboxYscSerial")],
        Ret::Xy => &[("x", "i32"), ("y", "i32")],
        Ret::MsOrOff => &[("ms_out", "u32")],
    }
}

fn unsupported(ty: Ty) -> ! {
    panic!("a {ty:?} argument only goes on the device's own calls")
}

pub fn c_function(call: &Call) -> Vec<String> {
    let extension = of(call);
    let mut text = doc(call, Lang::C);
    if call.ret == Ret::YscSerial {
        text.push_str(". free it with `anybox_ysc_serial_free`");
    }
    let mut out = wrap(&text, "/// ", "/// ", C_WIDTH);
    out.push("#[unsafe(no_mangle)]".into());
    out.push(format!("pub unsafe extern \"C\" fn {}(", function(call)));
    out.push("    device: *const AnyboxDevice,".into());
    let mut reads = Vec::new();
    let mut passed = Vec::new();
    for arg in call.args {
        let name = c_name(arg);
        let (ty, value) = match arg.ty {
            Ty::Interpolation => {
                reads.push(format!(
                    "let {name} = unsafe {{ read({name}, \"{name}\") }}?;"
                ));
                (
                    "*const AnyboxMakcuInterpolation",
                    format!("interpolation({name})"),
                )
            }
            Ty::YscSettings => {
                reads.push(format!(
                    "let {name} = unsafe {{ read({name}, \"{name}\") }}?;"
                ));
                (
                    "*const AnyboxYscInterpolationSettings",
                    format!("ysc_settings({name})"),
                )
            }
            Ty::Ms => ("u32", format!("Duration::from_millis(u64::from({name}))")),
            Ty::MsOrOff => ("u32", format!("ms_or_off({name})")),
            Ty::Bool => ("bool", name.clone()),
            other => unsupported(other),
        };
        out.push(format!("    {name}: {ty},"));
        passed.push(value);
    }
    for (name, ty) in outs(call.ret) {
        out.push(format!("    {name}: *mut {ty},"));
        reads.push(format!("let {name} = write({name}, \"{name}\")?;"));
    }
    out.push(") -> AnyboxStatus {".into());
    out.push("    guard(|| {".into());
    out.push("        let device = unsafe { self::device(device) }?;".into());
    out.extend(reads.iter().map(|line| format!("        {line}")));
    let reached = format!(
        "handle(device, Family::{}, Action::{}, Device::{})?.{}({}).map_err(error)",
        extension.family,
        call.action.expect("an extension call without its action"),
        extension.accessor,
        call.name,
        passed.join(", ")
    );
    let set = |value: &str| match call.ret {
        Ret::Unit => unreachable!(),
        Ret::Bool => vec![format!("out.set({value});")],
        Ret::Interpolation => vec![format!("out.set(makcu_interpolation({value}));")],
        Ret::YscInterpolation => vec![format!("out.set(ysc_interpolation({value}));")],
        Ret::YscSerial => vec![format!("out.set(ysc_serial({value}));")],
        Ret::Xy => vec![format!("x.set({value}.0);"), format!("y.set({value}.1);")],
        Ret::MsOrOff => vec![format!("ms_out.set(ms_of({value}));")],
    };
    if call.ret == Ret::Unit {
        out.push(format!("        {reached}"));
    } else {
        out.push(format!("        let got = {reached}?;"));
        out.extend(set("got").iter().map(|line| format!("        {line}")));
        out.push("        Ok(())".into());
    }
    out.push("    })".into());
    out.push("}".into());
    out.push(String::new());
    out
}

pub fn python_signature(call: &Call) -> String {
    let mut types = vec!["_handle"];
    for arg in call.args {
        types.push(match arg.ty {
            Ty::Interpolation => "POINTER(MakcuInterpolation)",
            Ty::YscSettings => "POINTER(YscInterpolationSettings)",
            Ty::Ms | Ty::MsOrOff => "c_uint32",
            Ty::Bool => "c_bool",
            other => unsupported(other),
        });
    }
    types.extend(match call.ret {
        Ret::Unit => &[][..],
        Ret::Bool => &["POINTER(c_bool)"],
        Ret::Interpolation => &["POINTER(MakcuInterpolation)"],
        Ret::YscInterpolation => &["POINTER(YscInterpolation)"],
        Ret::YscSerial => &["_out"],
        Ret::Xy => &["POINTER(c_int32)", "POINTER(c_int32)"],
        Ret::MsOrOff => &["POINTER(c_uint32)"],
    });
    format!("\"{}\": (_status, [{}]),", function(call), types.join(", "))
}

pub fn python_method(call: &Call) -> Vec<String> {
    let mut params = vec!["self".to_string()];
    let mut passed = vec![format!("lib.{}", function(call))];
    for arg in call.args {
        let name = arg.name;
        let (param, value) = match arg.ty {
            Ty::Interpolation => (
                format!("{name}: Interpolation"),
                format!("ctypes.byref(_raw_interpolation({name}))"),
            ),
            Ty::YscSettings => (
                format!("{name}: YscInterpolationSettings"),
                format!("ctypes.byref(_raw_ysc_settings({name}))"),
            ),
            Ty::Ms => (
                format!("{name}: float"),
                format!("_millis({name}, \"{}\")", arg.what),
            ),
            Ty::MsOrOff => (
                format!("{name}: float | None"),
                format!("_ms_or_off({name}, \"{}\")", arg.what),
            ),
            Ty::Bool => (
                match arg.default {
                    Some(_) => format!("{name}: bool = False"),
                    None => format!("{name}: bool"),
                },
                name.to_string(),
            ),
            other => unsupported(other),
        };
        params.push(param);
        passed.push(value);
    }
    let (ty, declared, read): (&str, &[&str], &str) = match call.ret {
        Ret::Unit => ("None", &[], ""),
        Ret::Bool => ("bool", &["out = ctypes.c_bool()"], "out.value"),
        Ret::Interpolation => (
            "Interpolation",
            &["out = native.MakcuInterpolation()"],
            "_interpolation(out)",
        ),
        Ret::YscInterpolation => (
            "YscInterpolation",
            &["out = native.YscInterpolation()"],
            "_ysc_interpolation(out)",
        ),
        Ret::YscSerial => (
            "YscSerial",
            &["out = ctypes.c_void_p()"],
            "_ysc_serial(out)",
        ),
        Ret::Xy => (
            "tuple[int, int]",
            &["x, y = ctypes.c_int32(), ctypes.c_int32()"],
            "x.value, y.value",
        ),
        Ret::MsOrOff => (
            "float | None",
            &["out = ctypes.c_uint32()"],
            "_seconds_or_off(out)",
        ),
    };
    passed.extend(match call.ret {
        Ret::Unit => vec![],
        Ret::Xy => vec!["ctypes.byref(x)".into(), "ctypes.byref(y)".into()],
        _ => vec!["ctypes.byref(out)".into()],
    });
    let mut out = vec![format!("def {}({}) -> {ty}:", call.name, params.join(", "))];
    let mut docstring = wrap(&doc(call, Lang::Python), "    \"\"\"", "    ", PYTHON_WIDTH);
    let last = docstring.len() - 1;
    if docstring[last].len() + 3 > PYTHON_WIDTH {
        docstring.push("    \"\"\"".into());
    } else {
        docstring[last].push_str("\"\"\"");
    }
    out.extend(docstring);
    out.extend(declared.iter().map(|line| format!("    {line}")));
    let one_line = format!("    self._device._out({})", passed.join(", "));
    if one_line.len() <= PYTHON_WIDTH {
        out.push(one_line);
    } else {
        out.push("    self._device._out(".into());
        out.extend(passed.iter().map(|value| format!("        {value},")));
        out.push("    )".into());
    }
    if call.ret != Ret::Unit {
        out.push(format!("    return {read}"));
    }
    out.push(String::new());
    out
}

/// the C function's parameters in C# and how a forwarder passes each on, the device first
fn csharp_c_params(call: &Call) -> (String, String) {
    let mut params = vec!["DeviceHandle device".to_string()];
    let mut passed = vec!["device".to_string()];
    for arg in call.args {
        let name = c_name(arg);
        let (ty, pass) = match arg.ty {
            Ty::Interpolation => ("in AnyboxMakcuInterpolation", format!("in {name}")),
            Ty::YscSettings => ("in AnyboxYscInterpolationSettings", format!("in {name}")),
            Ty::Ms | Ty::MsOrOff => ("uint", name.clone()),
            // C's bool is one byte, and LibraryImport would marshal a C# bool as a 4 byte BOOL
            Ty::Bool => ("byte", name.clone()),
            other => unsupported(other),
        };
        params.push(format!("{ty} {name}"));
        passed.push(pass);
    }
    for (name, ty) in outs(call.ret) {
        let ty = match *ty {
            "bool" => "byte",
            "AnyboxMakcuInterpolation" => "AnyboxMakcuInterpolation",
            "AnyboxYscInterpolation" => "AnyboxYscInterpolation",
            "*mut AnyboxYscSerial" => "YscSerialHandle",
            "i32" => "int",
            "u32" => "uint",
            other => panic!("no C# type for {other}"),
        };
        let name = if *name == "out" { "@out" } else { name };
        params.push(format!("out {ty} {name}"));
        passed.push(format!("out {name}"));
    }
    (params.join(", "), passed.join(", "))
}

pub fn csharp_import(call: &Call) -> Vec<String> {
    vec![
        "[LibraryImport(Library)]".into(),
        format!(
            "internal static partial uint {}({});",
            function(call),
            csharp_c_params(call).0
        ),
        String::new(),
    ]
}

pub fn csharp_forwarder(call: &Call) -> String {
    let (params, passed) = csharp_c_params(call);
    let function = function(call);
    format!("uint {function}({params}) => Imports.{function}({passed});")
}

pub fn csharp_method(call: &Call) -> Vec<String> {
    let mut params = Vec::new();
    // the arguments are checked into locals first, so a bad one is refused before the handle is read, which throws
    // on a disposed device
    let mut locals = Vec::new();
    let mut passed = vec!["device.Handle".to_string()];
    for arg in call.args {
        let name = camel(arg.name);
        let (param, value) = match arg.ty {
            Ty::Interpolation => (
                format!("Interpolation {name}"),
                format!("ExtensionValues.Raw({name})"),
            ),
            Ty::YscSettings => (
                format!("YscInterpolationSettings {name}"),
                format!("ExtensionValues.Raw({name})"),
            ),
            Ty::Ms => (
                format!("TimeSpan {name}"),
                format!("ExtensionValues.Millis({name}, \"{}\")", arg.what),
            ),
            Ty::MsOrOff => (
                format!("TimeSpan? {name}"),
                format!("ExtensionValues.MsOrOff({name}, \"{}\")", arg.what),
            ),
            Ty::Bool => (
                match arg.default {
                    Some(_) => format!("bool {name} = false"),
                    None => format!("bool {name}"),
                },
                format!("{name} ? (byte)1 : (byte)0"),
            ),
            other => unsupported(other),
        };
        params.push(param);
        if arg.ty == Ty::Bool {
            passed.push(value);
        } else {
            locals.push(format!("var raw{} = {value};", pascal(arg.name)));
            passed.push(format!("raw{}", pascal(arg.name)));
        }
    }
    let (ty, read) = match call.ret {
        Ret::Unit => ("void", ""),
        Ret::Bool => ("bool", "raw != 0"),
        Ret::Interpolation => ("Interpolation", "ExtensionValues.Read(raw)"),
        Ret::YscInterpolation => ("YscInterpolation", "ExtensionValues.Read(raw)"),
        Ret::YscSerial => ("YscSerial", "ExtensionValues.Read(raw)"),
        Ret::Xy => ("(int X, int Y)", "(x, y)"),
        Ret::MsOrOff => ("TimeSpan?", "ExtensionValues.MsOrOff(raw)"),
    };
    passed.extend(match call.ret {
        Ret::Unit => vec![],
        Ret::Xy => vec!["out var x".into(), "out var y".into()],
        _ => vec!["out var raw".into()],
    });
    let mut out = wrap(
        &format!("<summary>{}</summary>", doc(call, Lang::CSharp)),
        "/// ",
        "/// ",
        CSHARP_WIDTH,
    );
    let head = format!("public {ty} {}({})", pascal(call.name), params.join(", "));
    let check = call_lines(
        &format!("Library.Check(Native.{}(", function(call)),
        &passed,
        "));",
        CSHARP_WIDTH - 4,
    );
    let indented = |lines: Vec<String>| lines.into_iter().map(|line| format!("    {line}"));
    if call.ret == Ret::Unit && locals.is_empty() {
        if check.len() == 1 && head.len() + 4 + check[0].len() <= CSHARP_WIDTH {
            out.push(format!("{head} => {}", check[0]));
        } else {
            out.push(format!("{head} =>"));
            out.extend(indented(check));
        }
    } else {
        out.push(head);
        out.push("{".into());
        out.extend(indented(locals));
        out.extend(indented(check));
        if call.ret != Ret::Unit {
            out.push(format!("    return {read};"));
        }
        out.push("}".into());
    }
    out.push(String::new());
    out
}

/// a call on one line when it fits in `width`, otherwise one argument a line, so a string in an argument never gets
/// split
fn call_lines(open: &str, args: &[String], close: &str, width: usize) -> Vec<String> {
    let one = format!("{open}{}{close}", args.join(", "));
    if one.len() <= width {
        return vec![one];
    }
    let mut lines = vec![open.to_string()];
    let last = args.len() - 1;
    lines.extend(args.iter().enumerate().map(|(index, arg)| {
        if index == last {
            format!("    {arg}{close}")
        } else {
            format!("    {arg},")
        }
    }));
    lines
}

pub fn cpp_method(call: &Call) -> Vec<String> {
    let mut params = Vec::new();
    let mut body = Vec::new();
    let mut passed = vec!["device_.get()".to_string()];
    for arg in call.args {
        let name = arg.name;
        let (param, value) = match arg.ty {
            Ty::Interpolation => {
                body.push(format!("const auto raw_{name} = detail::raw({name});"));
                (
                    format!("const Interpolation& {name}"),
                    format!("&raw_{name}"),
                )
            }
            Ty::YscSettings => {
                body.push(format!("const auto raw_{name} = detail::raw({name});"));
                (
                    format!("const YscInterpolationSettings& {name}"),
                    format!("&raw_{name}"),
                )
            }
            Ty::Ms => (
                format!("std::chrono::milliseconds {name}"),
                format!("detail::millis({name}, \"{}\")", arg.what),
            ),
            Ty::MsOrOff => (
                format!("std::optional<std::chrono::milliseconds> {name}"),
                format!("detail::ms_or_off({name}, \"{}\")", arg.what),
            ),
            Ty::Bool => (
                match arg.default {
                    Some(_) => format!("bool {name} = false"),
                    None => format!("bool {name}"),
                },
                name.to_string(),
            ),
            other => unsupported(other),
        };
        params.push(param);
        passed.push(value);
    }
    let (ty, declared, outs, read): (&str, &[&str], &[&str], &str) = match call.ret {
        Ret::Unit => ("void", &[], &[], ""),
        Ret::Bool => ("bool", &["bool out = false;"], &["&out"], "out"),
        Ret::Interpolation => (
            "Interpolation",
            &["AnyboxMakcuInterpolation out{};"],
            &["&out"],
            "detail::interpolation(out)",
        ),
        Ret::YscInterpolation => (
            "YscInterpolation",
            &["AnyboxYscInterpolation out{};"],
            &["&out"],
            "detail::ysc_interpolation(out)",
        ),
        Ret::YscSerial => (
            "YscSerial",
            &["AnyboxYscSerial* out = nullptr;"],
            &["&out"],
            "detail::ysc_serial(out)",
        ),
        Ret::Xy => (
            "std::pair<std::int32_t, std::int32_t>",
            &["std::int32_t x = 0;", "std::int32_t y = 0;"],
            &["&x", "&y"],
            "{x, y}",
        ),
        Ret::MsOrOff => (
            "std::optional<std::chrono::milliseconds>",
            &["std::uint32_t ms = 0;"],
            &["&ms"],
            "detail::optional_ms(ms)",
        ),
    };
    body.extend(declared.iter().map(|line| line.to_string()));
    passed.extend(outs.iter().map(|out| out.to_string()));
    body.extend(call_lines(
        &format!("detail::check({}(", function(call)),
        &passed,
        "));",
        CPP_WIDTH - 4,
    ));
    if call.ret != Ret::Unit {
        body.push(format!("return {read};"));
    }
    let mut out = wrap(&doc(call, Lang::Cpp), "/// ", "/// ", CPP_WIDTH);
    let head = format!("{ty} {}({}) const {{", call.name, params.join(", "));
    if head.len() <= CPP_WIDTH {
        out.push(head);
    } else {
        out.push(format!("{ty} {}(", call.name));
        out.push(format!("    {}) const {{", params.join(", ")));
    }
    out.extend(body.iter().map(|line| format!("    {line}")));
    out.push("}".into());
    out.push(String::new());
    out
}

/// the readers the recorder and the C++ runner have for each argument, by its index
fn reader(ty: Ty) -> &'static str {
    match ty {
        Ty::Interpolation => "interpolation",
        Ty::YscSettings => "ysc_settings",
        Ty::Ms => "ms",
        Ty::MsOrOff => "ms_or_off",
        Ty::Bool => "flag",
        other => unsupported(other),
    }
}

/// how the recorder and the C++ runner write a result in the suite's form
fn writer(ret: Ret) -> &'static str {
    match ret {
        Ret::Unit => unreachable!(),
        Ret::Bool => "Value::Bool",
        Ret::Interpolation => "interpolation_json",
        Ret::YscInterpolation => "ysc_json",
        Ret::YscSerial => "serial_json",
        Ret::Xy => "xy_json",
        Ret::MsOrOff => "ms_json",
    }
}

fn read_args(call: &Call) -> String {
    call.args
        .iter()
        .enumerate()
        .map(|(index, arg)| format!("{}({index})", reader(arg.ty)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// the recorder's arm, through the crate's handle and never the C ABI, none when the device isn't that family
pub fn recorder_arm(call: &Call) -> String {
    let accessor = of(call).accessor;
    let result = match call.ret {
        Ret::Unit => ".map(|()| Value::Null)".to_string(),
        ret => format!(".map({})", writer(ret)),
    };
    format!(
        "\"{}\" => device.{accessor}().map(|{accessor}| {accessor}.{}({}){result}),",
        identity(call),
        call.name,
        read_args(call)
    )
}

/// the C++ runner's arm, through the wrapper's class
pub fn cpp_arm(call: &Call) -> String {
    let extension = of(call);
    let accessor = extension.accessor;
    let reached = format!("{accessor}.{}({})", call.name, read_args(call));
    let body = match call.ret {
        Ret::Unit => format!("{reached}; return Value();"),
        Ret::Bool => format!("return Value({reached});"),
        ret => format!("return {}({reached});", writer(ret)),
    };
    format!(
        "if (name == \"{}\") return on(device.{accessor}(), [&](const anybox::{}& {accessor}) {{ {body} }});",
        identity(call),
        extension.class
    )
}

/// the fake C function of the C++ forwarding test, which writes down what it was handed and fills its outs with
/// values the test knows
pub fn cpp_fake(call: &Call) -> Vec<String> {
    let mut params = vec!["const AnyboxDevice*".to_string()];
    let mut recorded = vec![format!("\"{}\"", identity(call))];
    for arg in call.args {
        let name = c_name(arg);
        let ty = match arg.ty {
            Ty::Interpolation => {
                recorded.push(format!("{name}->automatic, {name}->percent"));
                "const AnyboxMakcuInterpolation*"
            }
            Ty::YscSettings => {
                recorded.push(format!(
                    "{name}->enabled, {name}->profile, {name}->window_ms, {name}->max_window_ms"
                ));
                "const AnyboxYscInterpolationSettings*"
            }
            Ty::Ms | Ty::MsOrOff => {
                recorded.push(name.clone());
                "uint32_t"
            }
            Ty::Bool => {
                recorded.push(name.clone());
                "bool"
            }
            other => unsupported(other),
        };
        params.push(format!("{ty} {name}"));
    }
    let (outs, filled): (&[&str], &[&str]) = match call.ret {
        Ret::Unit => (&[], &[]),
        Ret::Bool => (&["bool* out"], &["*out = true;"]),
        Ret::Interpolation => (
            &["AnyboxMakcuInterpolation* out"],
            &["*out = AnyboxMakcuInterpolation{false, 42};"],
        ),
        Ret::YscInterpolation => (
            &["AnyboxYscInterpolation* out"],
            &["*out = fake_ysc_interpolation();"],
        ),
        Ret::YscSerial => (&["AnyboxYscSerial** out"], &["*out = fake_serial;"]),
        Ret::Xy => (&["int32_t* x", "int32_t* y"], &["*x = -3;", "*y = 10;"]),
        Ret::MsOrOff => (&["uint32_t* ms_out"], &["*ms_out = 1500;"]),
    };
    params.extend(outs.iter().map(|out| out.to_string()));
    let mut out = vec![format!(
        "AnyboxStatus {}({}) {{",
        function(call),
        params.join(", ")
    )];
    out.extend(filled.iter().map(|line| format!("    {line}")));
    out.push(format!("    return record({});", recorded.join(", ")));
    out.push("}".into());
    out.push(String::new());
    out
}

/// the C function's name for the docs of another language, a call on another handle gets its class
pub fn cross(from: &Call, to: &Call, lang: Lang) -> String {
    let name = crate::python_name(to);
    match (lang, from.on == to.on) {
        (Lang::CSharp, true) => format!("<see cref=\"{}\"/>", pascal(name)),
        (Lang::CSharp, false) => match &to.on.handle().extension {
            Some(extension) => format!("<see cref=\"{}.{}\"/>", extension.class, pascal(name)),
            None => format!("<see cref=\"Device.{}\"/>", pascal(name)),
        },
        _ => format!("`{name}`"),
    }
}
