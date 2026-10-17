// writes every plain call of the bindings from the table in table.rs, into the marked regions of the files below.
// run it from anywhere in the repo with `cargo run -p anybox-gen`. check.sh runs it with --check, which changes
// nothing and fails when a file doesn't match the table

mod table;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use table::{Arg, CALLS, Call, On, Ty};

// the longest line each language's files keep, less the indent of the class their region sits in: the repo's Rust
// comments go to 117, ruff's line length is 115, the C# methods keep to 117. the C# imports and forwarders stay on
// one line however long, like the hand-written ones next to them in Native.cs
const C_WIDTH: usize = 117;
const PYTHON_WIDTH: usize = 115 - 4;
const CSHARP_WIDTH: usize = 117 - 4;
const CPP_WIDTH: usize = 117 - 4;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Lang {
    C,
    Python,
    CSharp,
    Cpp,
}

fn pascal(name: &str) -> String {
    name.split('_')
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first
                    .to_uppercase()
                    .chain(chars.flat_map(char::to_lowercase))
                    .collect()
            })
        })
        .collect()
}

fn click_arg(call: &Call) -> Option<&Arg> {
    call.args.iter().find(|arg| arg.ty == Ty::Click)
}

/// the call's doc in that language, with its references to arguments, to other calls and to `Click` written its way
fn doc(call: &Call, lang: Lang) -> String {
    let mut out = String::new();
    let mut rest = call.doc;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let end = start + rest[start..].find('}').expect("a { in a doc with no }");
        let name = &rest[start + 1..end];
        let argument = call.args.iter().any(|arg| arg.name == name);
        // a reference to a call is looked up whatever the language, so a wrong one stops the generator even in a
        // doc only C gets
        let other = (!argument && name != "click").then(|| find(call.on, name));
        out.push_str(&match (name, lang) {
            ("click", Lang::C | Lang::Cpp) => format!(
                "`{}` says how long to hold it and whether to fail instead of letting anybox time it from the PC",
                click_arg(call).expect("{click} in a call with no Click").name
            ),
            ("click", Lang::Python) => {
                "`hold` is in seconds, and `native_only` fails instead of letting anybox time it from the PC".to_string()
            }
            ("click", Lang::CSharp) => "<paramref name=\"hold\"/> is how long, and <paramref name=\"nativeOnly\"/> \
                                        fails instead of letting anybox time it from the PC"
                .to_string(),
            (name, Lang::CSharp) if argument => format!("<paramref name=\"{name}\"/>"),
            (name, _) if argument => format!("`{name}`"),
            (_, Lang::C) => {
                let other = other.unwrap();
                format!("`{}{}`", other.on.handle().c_prefix, other.name)
            }
            (_, Lang::Python | Lang::Cpp) => format!("`{}`", python_name(other.unwrap())),
            (_, Lang::CSharp) => format!("<see cref=\"{}\"/>", pascal(python_name(other.unwrap()))),
        });
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

/// the call of that name on that handle, two handles can each have a call of the same name
fn find(on: On, name: &str) -> &'static Call {
    CALLS
        .iter()
        .find(|call| call.on == on && call.name == name)
        .unwrap_or_else(|| panic!("there's no call {name} on {on:?} in the table"))
}

/// what a call is called in Python, a folded one is the method it folds into
fn python_name(call: &Call) -> &'static str {
    match call.folds_into {
        Some(into) => find(call.on, into).name,
        None => call.name,
    }
}

/// the text in lines of at most `width`, the first starting with `first` and the rest with `next`. a C# tag like
/// `<see cref="X"/>` stays on one line
fn wrap(text: &str, first: &str, next: &str, width: usize) -> Vec<String> {
    let mut words = Vec::new();
    let mut start = 0;
    let mut in_tag = false;
    for (at, c) in text.char_indices() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            ' ' if !in_tag => {
                words.push(&text[start..at]);
                start = at + 1;
            }
            _ => {}
        }
    }
    words.push(&text[start..]);
    let mut lines = Vec::new();
    let mut line = first.to_string();
    let mut empty = true;
    for word in words {
        if !empty && line.len() + 1 + word.len() > width {
            lines.push(line);
            line = next.to_string();
            empty = true;
        }
        if !empty {
            line.push(' ');
        }
        line.push_str(word);
        empty = false;
    }
    lines.push(line);
    lines
}

fn c_function(call: &Call) -> Vec<String> {
    let handle = call.on.handle();
    let mut out = wrap(&doc(call, Lang::C), "/// ", "/// ", C_WIDTH);
    out.push("#[unsafe(no_mangle)]".into());
    out.push(format!(
        "pub unsafe extern \"C\" fn {}{}(",
        handle.c_prefix, call.name
    ));
    out.push(format!("    {}: *const {},", handle.name, handle.c_type));
    for arg in call.args {
        let ty = match arg.ty {
            Ty::I16 => "i16",
            Ty::Button | Ty::Axis | Ty::Direction => "u32",
            Ty::Key => "u8",
            Ty::MediaKey => "u16",
            Ty::Click => "*const AnyboxClick",
        };
        out.push(format!("    {}: {ty},", arg.name));
    }
    out.push(") -> AnyboxStatus {".into());
    let args: Vec<String> = call
        .args
        .iter()
        .map(|arg| match arg.ty {
            Ty::I16 => arg.name.to_string(),
            Ty::Button => format!("values::button({})?", arg.name),
            Ty::Key => format!("Key({})", arg.name),
            Ty::MediaKey => format!("MediaKey({})", arg.name),
            Ty::Axis => format!("values::axis({})?", arg.name),
            Ty::Direction => format!("values::direction({})?", arg.name),
            Ty::Click => format!("values::click(unsafe {{ read({0}, \"{0}\") }}?)", arg.name),
        })
        .collect();
    out.push("    guard(|| {".into());
    out.push(format!(
        "        unsafe {{ {}({}) }}?",
        handle.c_read, handle.name
    ));
    out.push(format!("            .{}({})", call.name, args.join(", ")));
    out.push("            .map_err(error)".into());
    out.push("    })".into());
    out.push("}".into());
    out.push(String::new());
    out
}

fn python_signature(call: &Call) -> String {
    let mut types = vec!["_handle"];
    types.extend(call.args.iter().map(|arg| match arg.ty {
        Ty::I16 => "c_int16",
        Ty::Button | Ty::Axis | Ty::Direction => "c_uint32",
        Ty::Key => "c_uint8",
        Ty::MediaKey => "c_uint16",
        Ty::Click => "POINTER(Click)",
    }));
    format!(
        "\"{}{}\": (_status, [{}]),",
        call.on.handle().c_prefix,
        call.name,
        types.join(", ")
    )
}

fn python_method(call: &Call) -> Vec<String> {
    let mut params = vec!["self".to_string()];
    let mut converted = Vec::new();
    for arg in call.args {
        let (ty, value) = match arg.ty {
            Ty::I16 => ("int", format!("_i16({0}, \"{0}\")", arg.name)),
            Ty::Button => ("Button", format!("_button({})", arg.name)),
            Ty::Key => ("Key", format!("_key({})", arg.name)),
            Ty::MediaKey => ("MediaKey", format!("_media({})", arg.name)),
            Ty::Axis => ("Axis", format!("_u32({}, \"an axis\")", arg.name)),
            Ty::Direction => ("Direction", format!("_u32({}, \"a direction\")", arg.name)),
            Ty::Click => {
                params.push("*, hold: float | None = None, native_only: bool = False".into());
                converted.push("ctypes.byref(_click(hold, native_only))".into());
                continue;
            }
        };
        params.push(match arg.default {
            Some(default) => format!("{}: {ty} = {ty}.{default}", arg.name),
            None => format!("{}: {ty}", arg.name),
        });
        converted.push(value);
    }
    let mut out = vec![format!(
        "def {}({}) -> None:",
        python_name(call),
        params.join(", ")
    )];
    let mut docstring = wrap(&doc(call, Lang::Python), "    \"\"\"", "    ", PYTHON_WIDTH);
    let last = docstring.len() - 1;
    if docstring[last].len() + 3 > PYTHON_WIDTH {
        docstring.push("    \"\"\"".into());
    } else {
        docstring[last].push_str("\"\"\"");
    }
    out.extend(docstring);
    let function = format!("lib.{}{}", call.on.handle().c_prefix, call.name);
    let one_line = if converted.is_empty() {
        format!("    self._call({function})")
    } else {
        format!("    self._call({function}, {})", converted.join(", "))
    };
    if one_line.len() <= PYTHON_WIDTH {
        out.push(one_line);
    } else {
        out.push("    self._call(".into());
        out.push(format!("        {function},"));
        out.extend(converted.iter().map(|value| format!("        {value},")));
        out.push("    )".into());
    }
    out.push(String::new());
    out
}

/// the C function's parameters in C# and how a forwarder passes each on, the handle first
fn csharp_c_params(call: &Call) -> (String, String) {
    let handle = call.on.handle();
    let mut params = vec![format!("{} {}", handle.csharp_type, handle.name)];
    let mut passed = vec![handle.name.to_string()];
    for arg in call.args {
        let ty = match arg.ty {
            Ty::I16 => "short",
            Ty::Button | Ty::Axis | Ty::Direction => "uint",
            Ty::Key => "byte",
            Ty::MediaKey => "ushort",
            Ty::Click => "in AnyboxClick",
        };
        params.push(format!("{ty} {}", arg.name));
        passed.push(if arg.ty == Ty::Click {
            format!("in {}", arg.name)
        } else {
            arg.name.to_string()
        });
    }
    (params.join(", "), passed.join(", "))
}

fn csharp_import(call: &Call) -> Vec<String> {
    vec![
        "[LibraryImport(Library)]".into(),
        format!(
            "internal static partial uint {}{}({});",
            call.on.handle().c_prefix,
            call.name,
            csharp_c_params(call).0
        ),
        String::new(),
    ]
}

fn csharp_forwarder(call: &Call) -> String {
    let (params, passed) = csharp_c_params(call);
    let function = format!("{}{}", call.on.handle().c_prefix, call.name);
    format!("uint {function}({params}) => Imports.{function}({passed});")
}

fn csharp_method(call: &Call) -> Vec<String> {
    let mut params = Vec::new();
    let mut converted = vec!["Handle".to_string()];
    for arg in call.args {
        let (ty, value) = match arg.ty {
            Ty::I16 => ("short", arg.name.to_string()),
            Ty::Button => ("Button", format!("(uint){}", arg.name)),
            Ty::Key => ("Key", format!("(byte){}", arg.name)),
            Ty::MediaKey => ("MediaKey", format!("(ushort){}", arg.name)),
            Ty::Axis => ("Axis", format!("(uint){}", arg.name)),
            Ty::Direction => ("Direction", format!("(uint){}", arg.name)),
            Ty::Click => {
                params.push("TimeSpan? hold = null, bool nativeOnly = false".to_string());
                converted.push("Click(hold, nativeOnly)".into());
                continue;
            }
        };
        params.push(match arg.default {
            Some(default) => format!("{ty} {} = {ty}.{}", arg.name, pascal(default)),
            None => format!("{ty} {}", arg.name),
        });
        converted.push(value);
    }
    let mut out = wrap(
        &format!("<summary>{}</summary>", doc(call, Lang::CSharp)),
        "/// ",
        "/// ",
        CSHARP_WIDTH,
    );
    let head = format!(
        "public void {}({}) =>",
        pascal(python_name(call)),
        params.join(", ")
    );
    let body = format!(
        "Library.Check(Native.{}{}({}));",
        call.on.handle().c_prefix,
        call.name,
        converted.join(", ")
    );
    if head.len() + 1 + body.len() <= CSHARP_WIDTH {
        out.push(format!("{head} {body}"));
    } else {
        out.push(head);
        out.push(format!("    {body}"));
    }
    out.push(String::new());
    out
}

/// the recorder's arm for a call, through the core and never the C ABI, with its argument readers
fn recorder_arm(call: &Call) -> String {
    let args: Vec<String> = call
        .args
        .iter()
        .enumerate()
        .map(|(index, arg)| {
            let reader = match arg.ty {
                Ty::I16 => "i16",
                Ty::Button => "button",
                Ty::Key => "key",
                Ty::MediaKey => "media",
                Ty::Axis => "axis",
                Ty::Direction => "direction",
                Ty::Click => "click",
            };
            format!("{reader}({index})")
        })
        .collect();
    format!(
        "\"{0}\" => {1}.{0}({2}),",
        call.name,
        call.on.handle().name,
        args.join(", ")
    )
}

fn cpp_method(call: &Call) -> Vec<String> {
    let mut params = Vec::new();
    let mut converted = vec!["get()".to_string()];
    for arg in call.args {
        let (ty, value) = match arg.ty {
            Ty::I16 => ("std::int16_t", arg.name.to_string()),
            Ty::Button | Ty::Key | Ty::MediaKey => (
                match arg.ty {
                    Ty::Button => "Button",
                    Ty::Key => "Key",
                    _ => "MediaKey",
                },
                format!("{}.value", arg.name),
            ),
            Ty::Axis => ("Axis", format!("static_cast<std::uint32_t>({})", arg.name)),
            Ty::Direction => (
                "Direction",
                format!("static_cast<std::uint32_t>({})", arg.name),
            ),
            Ty::Click => {
                params.push(format!("const Click& {} = {{}}", arg.name));
                converted.push(format!("{}.get()", arg.name));
                continue;
            }
        };
        params.push(match arg.default {
            Some(default) => format!("{ty} {} = {ty}::{}", arg.name, pascal(default)),
            None => format!("{ty} {}", arg.name),
        });
        converted.push(value);
    }
    let mut out = wrap(&doc(call, Lang::Cpp), "/// ", "/// ", CPP_WIDTH);
    let head = format!("void {}({}) const", python_name(call), params.join(", "));
    let body = format!(
        "detail::check({}{}({}));",
        call.on.handle().c_prefix,
        call.name,
        converted.join(", ")
    );
    if head.len() + body.len() + 5 <= CPP_WIDTH {
        out.push(format!("{head} {{ {body} }}"));
    } else {
        out.push(format!("{head} {{"));
        out.push(format!("    {body}"));
        out.push("}".into());
    }
    out.push(String::new());
    out
}

/// the C++ runner's arm for a call, through the wrapper's method, so a C-only call goes through the folded method's
/// default `Click`
fn cpp_arm(call: &Call) -> String {
    let args: Vec<String> = call
        .args
        .iter()
        .enumerate()
        .map(|(index, arg)| {
            let reader = match arg.ty {
                Ty::I16 => "i16",
                Ty::Button => "button",
                Ty::Key => "key",
                Ty::MediaKey => "media",
                Ty::Axis => "axis",
                Ty::Direction => "direction",
                Ty::Click => "click",
            };
            format!("{reader}({index})")
        })
        .collect();
    format!(
        "if (name == \"{}\") return {}.{}({});",
        call.name,
        call.on.handle().name,
        python_name(call),
        args.join(", ")
    )
}

/// the fake C function of the C++ forwarding test, which writes down its name and every number it was handed
fn cpp_fake(call: &Call) -> Vec<String> {
    let handle = call.on.handle();
    let mut params = vec![format!("const {}*", handle.c_type)];
    let mut recorded = vec![format!("\"{}\"", call.name)];
    for arg in call.args {
        let ty = match arg.ty {
            Ty::I16 => "int16_t",
            Ty::Button | Ty::Axis | Ty::Direction => "uint32_t",
            Ty::Key => "uint8_t",
            Ty::MediaKey => "uint16_t",
            Ty::Click => "const AnyboxClick*",
        };
        params.push(format!("{ty} {}", arg.name));
        if arg.ty == Ty::Click {
            recorded.push(format!(
                "{0}->hold_us, {0}->has_hold, {0}->native_only",
                arg.name
            ));
        } else {
            recorded.push(arg.name.to_string());
        }
    }
    vec![
        format!(
            "AnyboxStatus {}{}({}) {{",
            handle.c_prefix,
            call.name,
            params.join(", ")
        ),
        format!("    return record({});", recorded.join(", ")),
        "}".into(),
        String::new(),
    ]
}

const START: &str = "made by bindings/gen from its call table ";
const END: &str = "the end of what bindings/gen makes ";

/// a file and the name of a region in it
type Place = (&'static str, &'static str);

/// where a handle's calls go. a region holds one handle's, so a handle sharing a file with another (`Native.cs`, the
/// recorder) has regions of its own names there
struct Places {
    c: Place,
    python_signatures: Place,
    python_methods: Place,
    csharp_imports: Place,
    csharp_forwarders: Place,
    csharp_methods: Place,
    recorder_calls: Place,
    recorder_dispatch: Place,
    python_folded: Place,
    csharp_folded: Place,
    cpp_methods: Place,
    cpp_dispatch: Place,
    cpp_fake: Place,
}

fn places(on: On) -> Places {
    match on {
        On::Device => Places {
            c: ("bindings/c/src/device.rs", "calls"),
            python_signatures: ("bindings/python/anybox/_native.py", "signatures"),
            python_methods: ("bindings/python/anybox/_device.py", "methods"),
            csharp_imports: ("bindings/csharp/src/Anybox/Native.cs", "imports"),
            csharp_forwarders: ("bindings/csharp/src/Anybox/Native.cs", "forwarders"),
            csharp_methods: ("bindings/csharp/src/Anybox/Device.cs", "methods"),
            recorder_calls: ("bindings/c/src/conformance.rs", "calls"),
            recorder_dispatch: ("bindings/c/src/conformance.rs", "dispatch"),
            python_folded: ("bindings/python/tests/test_conformance.py", "folded"),
            csharp_folded: (
                "bindings/csharp/tests/Anybox.Tests/ConformanceTests.cs",
                "folded",
            ),
            cpp_methods: ("bindings/c/include/anybox.hpp", "methods"),
            cpp_dispatch: ("bindings/c/tests/cpp/conformance.cpp", "dispatch"),
            cpp_fake: ("bindings/c/tests/cpp/fake.cpp", "calls"),
        },
    }
}

/// what goes in each marked region, by file and region name
fn regions() -> Vec<(&'static str, &'static str, Vec<String>)> {
    let mut handles: Vec<On> = Vec::new();
    for call in CALLS {
        if !handles.contains(&call.on) {
            handles.push(call.on);
        }
    }
    let mut out = Vec::new();
    for on in handles {
        let all: Vec<&Call> = CALLS.iter().filter(|call| call.on == on).collect();
        let bound: Vec<&Call> = all.iter().copied().filter(|call| !call.only_in_c).collect();
        let folded: Vec<&Call> = all
            .iter()
            .copied()
            .filter(|call| call.folds_into.is_some())
            .collect();
        let places = places(on);
        let made: [(Place, Vec<String>); 13] = [
            (
                places.c,
                all.iter().flat_map(|call| c_function(call)).collect(),
            ),
            (
                places.python_signatures,
                bound.iter().map(|call| python_signature(call)).collect(),
            ),
            (
                places.python_methods,
                bound.iter().flat_map(|call| python_method(call)).collect(),
            ),
            (
                places.csharp_imports,
                bound.iter().flat_map(|call| csharp_import(call)).collect(),
            ),
            (
                places.csharp_forwarders,
                bound.iter().map(|call| csharp_forwarder(call)).collect(),
            ),
            (
                places.csharp_methods,
                bound.iter().flat_map(|call| csharp_method(call)).collect(),
            ),
            (
                places.recorder_calls,
                all.iter()
                    .map(|call| format!("\"{}\",", call.name))
                    .collect(),
            ),
            (
                places.recorder_dispatch,
                all.iter().map(|call| recorder_arm(call)).collect(),
            ),
            (
                places.python_folded,
                folded
                    .iter()
                    .map(|call| format!("\"{}\": \"{}\",", call.name, python_name(call)))
                    .collect(),
            ),
            (
                places.csharp_folded,
                folded
                    .iter()
                    .map(|call| format!("[\"{}\"] = \"{}\",", call.name, pascal(python_name(call))))
                    .collect(),
            ),
            (
                places.cpp_methods,
                bound.iter().flat_map(|call| cpp_method(call)).collect(),
            ),
            (
                places.cpp_dispatch,
                all.iter().map(|call| cpp_arm(call)).collect(),
            ),
            (
                places.cpp_fake,
                all.iter().flat_map(|call| cpp_fake(call)).collect(),
            ),
        ];
        out.extend(made.map(|((file, region), lines)| (file, region, lines)));
    }
    for (index, (file, region, _)) in out.iter().enumerate() {
        assert!(
            !out[..index]
                .iter()
                .any(|(other, name, _)| other == file && name == region),
            "two handles write the {region} region of {file}, give the second one its own"
        );
    }
    out
}

/// the file with what's between its two marks of `region` replaced, each line indented like the first mark
fn replace(text: &str, region: &str, lines: &[String]) -> Result<String, String> {
    let all: Vec<&str> = text.lines().collect();
    // a second copy of a region would be left as it is and checked by nothing
    let only = |mark: String| {
        let mut found = all
            .iter()
            .enumerate()
            .filter(|(_, line)| line.contains(&mark));
        match (found.next(), found.next()) {
            (Some((at, _)), None) => Ok(at),
            (None, _) => Err(format!("no `{mark}`")),
            (Some(_), Some(_)) => Err(format!("`{mark}` more than once")),
        }
    };
    let start = only(format!("{START}({region})"))?;
    let end = only(format!("{END}({region})"))?;
    if end < start {
        return Err(format!("the end mark of {region} comes before its start"));
    }
    let indent: String = all[start]
        .chars()
        .take_while(|c| c.is_whitespace())
        .collect();
    let mut out: Vec<String> = all[..=start].iter().map(|line| line.to_string()).collect();
    out.extend(lines.iter().map(|line| {
        if line.is_empty() {
            String::new()
        } else {
            format!("{indent}{line}")
        }
    }));
    out.extend(all[end..].iter().map(|line| line.to_string()));
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    Ok(out.join(newline) + newline)
}

/// the region each bindings/gen mark in the text names, the start and the end marks both
fn marked(text: &str) -> Vec<&str> {
    text.lines()
        .filter_map(|line| {
            let at = line.find(START).or_else(|| line.find(END))?;
            let (_, rest) = line[at..].split_once('(')?;
            Some(rest.split_once(')').map_or(rest, |(region, _)| region))
        })
        .collect()
}

/// the repo it runs in, found from where it runs and not from where it was built, because a copy of the repo with
/// a copied target/ would run the old binary, which would check and write the original
fn root() -> PathBuf {
    let here = std::env::current_dir().unwrap();
    here.ancestors()
        .find(|dir| dir.join("bindings/gen/Cargo.toml").is_file())
        .unwrap_or_else(|| {
            panic!(
                "{} isn't in the anybox repo, run the generator from inside it",
                here.display()
            )
        })
        .to_path_buf()
}

/// every file in bindings/ with a bindings/gen mark, untracked ones too, so a region file the generator doesn't
/// know about fails instead of going unchecked
fn marked_files(root: &Path) -> Vec<String> {
    let out = Command::new("git")
        .args([
            "grep",
            "--untracked",
            "--files-with-matches",
            "--fixed-strings",
        ])
        .args([START.trim_end(), "--", "bindings", ":(exclude)bindings/gen"])
        .current_dir(root)
        .output()
        .expect("git isn't installed");
    // git grep says 1 when nothing matches
    assert!(
        matches!(out.status.code(), Some(0 | 1)),
        "git grep failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(String::from)
        .collect()
}

/// the Rust in rustfmt's shape, so the formatting check never fights the generator. it runs in the repo so it picks
/// up rustfmt.toml
fn rustfmt(text: String, root: &Path) -> String {
    let mut child = Command::new("rustfmt")
        .args(["--edition", "2024"])
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("rustfmt isn't installed");
    let mut stdin = child.stdin.take().unwrap();
    // a file bigger than the pipe would block both sides if the writing waited for the reading
    let writer = std::thread::spawn(move || stdin.write_all(text.as_bytes()));
    let out = child.wait_with_output().unwrap();
    writer.join().unwrap().unwrap();
    assert!(out.status.success(), "rustfmt failed");
    String::from_utf8(out.stdout).unwrap()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // a misspelled --check would otherwise write the files and pass
    let check = match args.as_slice() {
        [] => false,
        [flag] if flag == "--check" => true,
        _ => {
            eprintln!(
                "bindings/gen: the only option is --check, got {}",
                args.join(" ")
            );
            std::process::exit(2);
        }
    };
    let root = root();
    let regions = regions();
    for file in marked_files(&root) {
        assert!(
            regions.iter().any(|(known, ..)| *known == file),
            "{file} has bindings/gen marks but the generator doesn't write it, add its places in main.rs"
        );
    }
    let mut files: Vec<(&str, String, String)> = Vec::new();
    for (file, region, lines) in &regions {
        let index = files
            .iter()
            .position(|(name, ..)| name == file)
            .unwrap_or_else(|| {
                let text = std::fs::read_to_string(root.join(file))
                    .unwrap_or_else(|error| panic!("{file}: {error}"));
                files.push((file, text.clone(), text));
                files.len() - 1
            });
        let made = replace(&files[index].2, region, lines)
            .unwrap_or_else(|error| panic!("{file}: {error}"));
        files[index].2 = made;
    }
    let mut stale = Vec::new();
    for (file, old, mut made) in files {
        // a region the generator doesn't write would never be checked either
        for region in marked(&made) {
            assert!(
                regions
                    .iter()
                    .any(|(other, name, _)| *other == file && *name == region),
                "{file} has a bindings/gen mark for a region called {region}, which the generator doesn't write"
            );
        }
        if file.ends_with(".rs") {
            made = rustfmt(made, &root);
        }
        if made == old {
            continue;
        }
        if check {
            stale.push(file);
        } else {
            std::fs::write(root.join(file), made).unwrap();
        }
    }
    if !stale.is_empty() {
        eprintln!(
            "bindings/gen: what the call table makes isn't in {} yet, run cargo run -p anybox-gen and commit what it \
             changes",
            stale.join(", ")
        );
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests;
