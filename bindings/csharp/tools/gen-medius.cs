// writes the medius.h side of the C# package (its constants, enums, structs, handles and imports), the generated
// parts of the C++ header anybox_medius.hpp (its enums, errors, structs and most methods, from the tables at the
// bottom) and the Rust test that pins the layout, all from bindings/c/include/medius.h. check.sh runs it and fails on
// a diff, so a medius bump that changes the header shows up as a change to look at, and on a header its tables don't
// fit. run it from the repo root with dotnet run bindings/csharp/tools/gen-medius.cs
using System.Text;
using System.Text.RegularExpressions;

// --header reads another medius.h and --dry writes nothing, which is how the generator's own tests feed it a changed
// header and see it refuse
var positional = new List<string>();
string? headerPath = null;
var dry = false;
for (var at = 0; at < args.Length; at++)
{
    if (args[at] == "--header")
    {
        headerPath = args[++at];
    }
    else if (args[at] == "--dry")
    {
        dry = true;
    }
    else
    {
        positional.Add(args[at]);
    }
}
var root = positional.Count > 0 ? positional[0] : Directory.GetCurrentDirectory();
var header = File.ReadAllText(headerPath ?? Path.Combine(root, "bindings/c/include/medius.h")).ReplaceLineEndings("\n");

// a name C# would take as a keyword
string[] keywords = ["abstract", "as", "base", "bool", "break", "byte", "case", "catch", "char", "checked", "class", "const",
    "continue", "decimal", "default", "delegate", "do", "double", "else", "enum", "event", "explicit", "extern", "false",
    "finally", "fixed", "float", "for", "foreach", "goto", "if", "implicit", "in", "int", "interface", "internal", "is",
    "lock", "long", "namespace", "new", "null", "object", "operator", "out", "override", "params", "private", "protected",
    "public", "readonly", "ref", "return", "sbyte", "sealed", "short", "sizeof", "stackalloc", "static", "string", "struct",
    "switch", "this", "throw", "true", "try", "typeof", "uint", "ulong", "unchecked", "unsafe", "ushort", "using",
    "virtual", "void", "volatile", "while"];
string Id(string name) => keywords.Contains(name) ? "@" + name : name;

string Pascal(string snake) => string.Concat(snake.ToLowerInvariant().Split('_', StringSplitOptions.RemoveEmptyEntries)
    .Select(word => char.ToUpperInvariant(word[0]) + word[1..]));
// a C# name can't start with a digit, so a key like 1 is N1, like the Python package
string Member(string snake) => Pascal(snake) is var name && char.IsDigit(name[0]) ? "N" + name : name;
string Snake(string pascal) => Regex.Replace(pascal, "(?<=[a-z0-9])([A-Z])", "_$1").ToUpperInvariant();

// the public name of a C enum, Medius dropped since the namespace says it. names the implicit usings already have
// get one that doesn't clash, like BoxAction in the main package
var renames = new Dictionary<string, string> { ["Action"] = "InjectAction" };
string EnumName(string c) => renames.GetValueOrDefault(c["Medius".Length..], c["Medius".Length..]);

var constants = new List<(string Name, string Type, string Value)>();
foreach (Match define in Regex.Matches(header, @"^#define (MEDIUS_\w+) (.+)$", RegexOptions.Multiline))
{
    var value = define.Groups[2].Value.Trim();
    (string Type, string Value) parsed = value switch
    {
        "UINT32_MAX" => ("uint", "uint.MaxValue"),
        "INT32_MIN" => ("int", "int.MinValue"),
        _ when Regex.IsMatch(value, @"^-?\d+$") => (long.Parse(value) is >= int.MinValue and <= int.MaxValue ? "int" : "long", value),
        _ when Regex.IsMatch(value, @"^-?0x[0-9A-Fa-f]+$") => ("int", value),
        _ => throw new InvalidOperationException($"no C# type for #define {define.Groups[1].Value} {value}"),
    };
    constants.Add((define.Groups[1].Value, parsed.Type, parsed.Value));
}

// enums, each with the C type cbindgen gives it
var enums = new Dictionary<string, (string Underlying, List<(string Name, string Value)> Members)>();
foreach (Match block in Regex.Matches(header, @"^enum (Medius\w+)\n#if [^\n]*\n\s*: (\w+)\n#endif[^\n]*\n \{\n(.*?)\n\};", RegexOptions.Multiline | RegexOptions.Singleline))
{
    var name = block.Groups[1].Value;
    var prefix = "MEDIUS_" + Snake(name["Medius".Length..]) + "_";
    var members = new List<(string, string)>();
    foreach (Match member in Regex.Matches(block.Groups[3].Value, @"^\s+(MEDIUS_\w+) = (-?\w+),$", RegexOptions.Multiline))
    {
        var member_name = member.Groups[1].Value;
        if (!member_name.StartsWith(prefix))
        {
            throw new InvalidOperationException($"{member_name} doesn't start with {prefix}");
        }
        members.Add((Member(member_name[prefix.Length..]), member.Groups[2].Value));
    }
    enums[name] = (block.Groups[2].Value, members);
}

// the key and media key #defines are an enum in the Python package, so they're one here too
foreach (var (prefix, name, type) in new[] { ("MEDIUS_KEY_", "MediusKey", "uint8_t"), ("MEDIUS_MEDIA_", "MediusMediaKey", "uint16_t") })
{
    var members = constants.Where(constant => constant.Name.StartsWith(prefix))
        .Select(constant => (Member(constant.Name[prefix.Length..]), constant.Value)).ToList();
    constants.RemoveAll(constant => constant.Name.StartsWith(prefix));
    enums[name] = (type, members);
}

// the names the Python package gives the two values for a usage edge and a traffic flow
enums["MediusDirection"].Members.AddRange([("Press", "Positive"), ("Release", "Negative"), ("In", "Positive"), ("Out", "Negative")]);

// plain aliases like MediusKey, which are only a number, not an enum
var aliases = Regex.Matches(header, @"^typedef (\w+) (Medius\w+);$", RegexOptions.Multiline)
    .Where(match => !enums.ContainsKey(match.Groups[2].Value))
    .ToDictionary(match => match.Groups[2].Value, match => match.Groups[1].Value);

var opaque = Regex.Matches(header, @"^typedef struct (Medius\w+) \1;$", RegexOptions.Multiline).Select(match => match.Groups[1].Value).ToList();

// structs and unions, fields in order
var records = new List<(string Kind, string Name, List<(string Type, string Name, string? Count)> Fields)>();
foreach (Match block in Regex.Matches(header, @"^typedef (struct|union) (Medius\w+) \{\n(.*?)\n\} \2;", RegexOptions.Multiline | RegexOptions.Singleline))
{
    var fields = new List<(string, string, string?)>();
    foreach (var raw in block.Groups[3].Value.Split('\n'))
    {
        var line = raw.Trim();
        if (line.Length == 0 || line.StartsWith("//") || line.StartsWith('#'))
        {
            continue;
        }
        var field = Regex.Match(line, @"^(?:(?:struct|union) )?(\w+) (\w+)(?:\[(\w+)\])?;$");
        if (!field.Success)
        {
            throw new InvalidOperationException($"can't read the field `{line}` in {block.Groups[2].Value}");
        }
        fields.Add((field.Groups[1].Value, field.Groups[2].Value, field.Groups[3].Success ? field.Groups[3].Value : null));
    }
    records.Add((block.Groups[1].Value, block.Groups[2].Value, fields));
}
var recordNames = records.Select(record => record.Name).ToHashSet();

string? Primitive(string c) => c switch
{
    "uint8_t" or "bool" or "char" => "byte",
    "uint16_t" => "ushort",
    "uint32_t" => "uint",
    "uint64_t" => "ulong",
    "int8_t" => "sbyte",
    "int16_t" => "short",
    "int32_t" => "int",
    "int64_t" => "long",
    "uintptr_t" => "nuint",
    "float" => "float",
    "double" => "double",
    "void" => "void",
    _ => null,
};

string Value(string c) => Primitive(aliases.GetValueOrDefault(c, c)) ?? (enums.ContainsKey(c) ? EnumName(c) : recordNames.Contains(c) ? c
    : throw new InvalidOperationException($"no C# type for {c}"));

int Count(string count) => int.TryParse(count, out var number) ? number
    : int.Parse(constants.Single(constant => constant.Name == count).Value);

// arrays of anything that isn't a primitive become inline arrays, one type per element and length
var inlineArrays = new SortedSet<(string Element, int Length)>();

string Handle(string c) => c + "Handle";

// one parameter or return type, `ptr` is how many stars it has
string Param(string type, int stars, bool isFree, bool isReturn)
{
    if (opaque.Contains(type))
    {
        return (stars, isFree) switch
        {
            (1, true) => "nint",
            (1, false) => Handle(type),
            (2, _) when !isReturn => "out " + Handle(type),
            _ => throw new InvalidOperationException($"no C# type for {type} with {stars} stars"),
        };
    }
    var value = type == "void" ? "void" : Value(type);
    return value + new string('*', stars);
}

var functions = new List<(string Name, string Return, List<(string Type, string Name)> Parameters)>();
// the same functions as medius.h has them, for the C++ side, which works from the C types
var cfunctions = new List<CFunction>();
var flat = Regex.Replace(header, @"//[^\n]*", "");
// what's only there with the mock, which the C++ side keeps inside the same #if
var mockOnly = Regex.Matches(flat, @"^#if defined\(MEDIUS_FEATURE_MOCK\)\n(.*?)^#endif", RegexOptions.Multiline | RegexOptions.Singleline)
    .SelectMany(block => Regex.Matches(block.Groups[1].Value, @"\b(medius_\w+)\(").Select(match => match.Groups[1].Value))
    .ToHashSet();
foreach (Match declaration in Regex.Matches(flat, @"^([A-Za-z][\w \*]*?)\s*\b(medius_\w+)\(((?:[^()]|\([^()]*\))*)\);", RegexOptions.Multiline))
{
    var name = declaration.Groups[2].Value;
    var isFree = name.EndsWith("_free");
    var ret = Regex.Match(Regex.Replace(declaration.Groups[1].Value, @"\b(const|struct|union) ", "").Trim(), @"^(\w+)\s*(\**)$");
    var parameters = new List<(string, string)>();
    var cparameters = new List<CParam>();
    var text = Regex.Replace(declaration.Groups[3].Value, @"\s+", " ").Trim();
    if (text != "void")
    {
        // split on the commas that aren't inside the callback's own parentheses
        var depth = 0;
        var start = 0;
        var parts = new List<string>();
        for (var at = 0; at <= text.Length; at++)
        {
            if (at == text.Length || (text[at] == ',' && depth == 0))
            {
                parts.Add(text[start..at].Trim());
                start = at + 1;
            }
            else if (text[at] == '(')
            {
                depth++;
            }
            else if (text[at] == ')')
            {
                depth--;
            }
        }
        foreach (var part in parts)
        {
            var callback = Regex.Match(part, @"^void \(\*(\w+)\)\((.*)\)$");
            if (callback.Success)
            {
                var types = callback.Groups[2].Value.Split(',').Select(arg => Param(arg.Trim().TrimEnd('*').Trim(), arg.Count(c => c == '*'), false, false));
                parameters.Add(($"delegate* unmanaged<{string.Join(", ", types)}, void>", callback.Groups[1].Value));
                cparameters.Add(new CParam("void", 0, false, callback.Groups[1].Value, true));
                continue;
            }
            var parameter = Regex.Match(Regex.Replace(part, @"\b(const|struct|union) ", ""), @"^(\w+)\s*(\**)\s*(\w+)$");
            if (!parameter.Success)
            {
                throw new InvalidOperationException($"can't read the parameter `{part}` of {name}");
            }
            parameters.Add((Param(parameter.Groups[1].Value, parameter.Groups[2].Length, isFree, false), parameter.Groups[3].Value));
            cparameters.Add(new CParam(parameter.Groups[1].Value, parameter.Groups[2].Length, Regex.IsMatch(part, @"\bconst\b"), parameter.Groups[3].Value, false));
        }
    }
    functions.Add((name, Param(ret.Groups[1].Value, ret.Groups[2].Length, false, true), parameters));
    var signature = Regex.Replace(Regex.Replace(declaration.Value.TrimEnd(';'), @"\s+", " "), @"\( | \)", match => match.Value.Trim());
    cfunctions.Add(new CFunction(name, ret.Groups[1].Value, ret.Groups[2].Length, cparameters, signature, mockOnly.Contains(name)));
}

var cs = new StringBuilder();
cs.AppendLine("// made by bindings/csharp/tools/gen-medius.cs from bindings/c/include/medius.h, don't edit it by hand");
cs.AppendLine("#pragma warning disable CS1591");
cs.AppendLine("using System.Runtime.CompilerServices;");
cs.AppendLine("using System.Runtime.InteropServices;");
cs.AppendLine();
cs.AppendLine("namespace Anybox.Medius;");
cs.AppendLine();
cs.AppendLine("public static class Constants");
cs.AppendLine("{");
foreach (var (name, type, value) in constants)
{
    cs.AppendLine($"    public const {type} {Pascal(name["MEDIUS_".Length..])} = {value};");
}
cs.AppendLine("}");
foreach (var (name, (underlying, members)) in enums)
{
    cs.AppendLine();
    cs.AppendLine($"public enum {EnumName(name)} : {Primitive(underlying)}");
    cs.AppendLine("{");
    foreach (var (member, value) in members)
    {
        cs.AppendLine($"    {member} = {value},");
    }
    cs.AppendLine("}");
}
foreach (var (kind, name, fields) in records)
{
    cs.AppendLine();
    cs.AppendLine(kind == "union" ? "[StructLayout(LayoutKind.Explicit)]" : "[StructLayout(LayoutKind.Sequential)]");
    cs.AppendLine($"internal unsafe struct {name}");
    cs.AppendLine("{");
    foreach (var (type, field, count) in fields)
    {
        var offset = kind == "union" ? "[FieldOffset(0)] " : "";
        var primitive = Primitive(type);
        if (count is null)
        {
            cs.AppendLine($"    {offset}public {Value(type)} {Id(field)};");
        }
        else if (primitive is not null)
        {
            cs.AppendLine($"    {offset}public fixed {primitive} {Id(field)}[{Count(count)}];");
        }
        else
        {
            var length = Count(count);
            inlineArrays.Add((Value(type), length));
            cs.AppendLine($"    {offset}public {Value(type)}Array{length} {Id(field)};");
        }
    }
    cs.AppendLine("}");
}
foreach (var (element, length) in inlineArrays)
{
    cs.AppendLine();
    cs.AppendLine($"[InlineArray({length})]");
    cs.AppendLine($"internal struct {element}Array{length}");
    cs.AppendLine("{");
    cs.AppendLine($"    public {element} first;");
    cs.AppendLine("}");
}
foreach (var handle in opaque)
{
    var free = functions.Single(function => function.Name.EndsWith("_free") && function.Parameters.Count == 1 &&
        function.Name == "medius_" + Regex.Replace(Snake(handle["Medius".Length..]), "_BOX$", "").ToLowerInvariant() + "_free");
    cs.AppendLine();
    cs.AppendLine($"internal sealed class {Handle(handle)}() : SafeHandle(0, ownsHandle: true)");
    cs.AppendLine("{");
    cs.AppendLine("    public override bool IsInvalid => handle == 0;");
    cs.AppendLine();
    cs.AppendLine("    protected override bool ReleaseHandle()");
    cs.AppendLine("    {");
    cs.AppendLine($"        MediusImports.{free.Name}(handle);");
    cs.AppendLine("        return true;");
    cs.AppendLine("    }");
    cs.AppendLine("}");
}
cs.AppendLine();
cs.AppendLine("internal static unsafe partial class MediusImports");
cs.AppendLine("{");
cs.AppendLine("    const string Library = \"anybox\";");
foreach (var (name, ret, parameters) in functions)
{
    cs.AppendLine();
    cs.AppendLine("    [LibraryImport(Library)]");
    cs.AppendLine($"    internal static partial {ret} {name}({string.Join(", ", parameters.Select(parameter => $"{parameter.Type} {Id(parameter.Name)}"))});");
}
cs.AppendLine("}");

string[] rustKeywords = ["as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for", "if", "impl",
    "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "static", "struct", "super", "trait",
    "true", "type", "unsafe", "use", "where", "while", "async", "await", "dyn"];
var rust = new StringBuilder();
rust.AppendLine("// made by bindings/csharp/tools/gen-medius.cs from bindings/c/include/medius.h, don't edit it by hand. it");
rust.AppendLine("// writes the real size, alignment and field offsets of every medius-capi struct, which the C# package is held to");
rust.AppendLine("use std::mem::{align_of, offset_of, size_of};");
rust.AppendLine();
rust.AppendLine("use medius_capi::*;");
rust.AppendLine();
rust.AppendLine("#[test]");
rust.AppendLine("fn the_medius_layout_is_the_committed_one() {");
rust.AppendLine("    let mut layout = String::new();");
foreach (var (kind, name, fields) in records)
{
    rust.AppendLine($"    layout += &format!(\"{name} size {{}} align {{}}\\n\", size_of::<{name}>(), align_of::<{name}>());");
    if (kind == "struct")
    {
        foreach (var (_, field, _) in fields)
        {
            // cbindgen puts a _ after a name C++ has as a keyword, so Rust's `class` is `class_` in the header, but
            // a Rust keyword like `loop` already has its _ in the crate
            var trimmed = field.TrimEnd('_');
            var rustField = rustKeywords.Contains(trimmed) ? field : trimmed;
            rust.AppendLine($"    layout += &format!(\"  {field} {{}}\\n\", offset_of!({name}, {rustField}));");
        }
    }
}
rust.AppendLine("    // cargo says where the crate is at run time too, and a test binary copied from another checkout has the other");
rust.AppendLine("    // one's path built in");
rust.AppendLine("    let path = std::path::PathBuf::from(std::env::var(\"CARGO_MANIFEST_DIR\").unwrap()).join(\"tests/medius-layout.txt\");");
rust.AppendLine("    if std::env::var_os(\"ANYBOX_WRITE_LAYOUT\").is_some() {");
rust.AppendLine("        std::fs::write(&path, &layout).unwrap();");
rust.AppendLine("    }");
rust.AppendLine("    let committed = std::fs::read_to_string(&path).unwrap_or_default().replace(\"\\r\\n\", \"\\n\");");
rust.AppendLine("    assert_eq!(committed, layout, \"medius-capi's layout changed, rerun with ANYBOX_WRITE_LAYOUT=1 and check the C# side\");");
rust.AppendLine("}");

// the C++ side, the marked regions of anybox_medius.hpp. every function, struct and enum medius.h has needs an
// entry in the tables at the bottom, and an entry pins what the code written by hand for it was written against,
// so a medius bump that adds or changes one stops here until someone decides what C++ does with it
var problems = new List<string>();
var regions = new Dictionary<string, List<string>>();

string? Number(string c) => aliases.GetValueOrDefault(c, c) switch
{
    "uint8_t" => "std::uint8_t",
    "uint16_t" => "std::uint16_t",
    "uint32_t" => "std::uint32_t",
    "uint64_t" => "std::uint64_t",
    "int8_t" => "std::int8_t",
    "int16_t" => "std::int16_t",
    "int32_t" => "std::int32_t",
    "int64_t" => "std::int64_t",
    "uintptr_t" => "std::size_t",
    "float" => "float",
    "double" => "double",
    _ => null,
};
bool IsByte(string c) => aliases.GetValueOrDefault(c, c) == "uint8_t";

// lines of at most 120 characters with the indent the region gives them, for the doc comments
List<string> Doc(string text, int indent = 0)
{
    var lines = new List<string>();
    if (text.Length == 0)
    {
        return lines;
    }
    var line = "///";
    foreach (var word in text.Split(' '))
    {
        if (line.Length + 1 + word.Length + indent > 120 && line != "///")
        {
            lines.Add(line);
            line = "///";
        }
        line += " " + word;
    }
    lines.Add(line);
    return lines;
}

// the enums with their C names, the cbindgen ones and the two #define groups that are enums in Rust
var cEnums = new Dictionary<string, (string Prefix, string Underlying, List<(string Constant, string Value)> Members)>();
foreach (Match block in Regex.Matches(header, @"^enum (Medius\w+)\n#if [^\n]*\n\s*: (\w+)\n#endif[^\n]*\n \{\n(.*?)\n\};", RegexOptions.Multiline | RegexOptions.Singleline))
{
    var name = block.Groups[1].Value;
    cEnums[name] = ("MEDIUS_" + Snake(name["Medius".Length..]) + "_", block.Groups[2].Value,
        [.. Regex.Matches(block.Groups[3].Value, @"^\s+(MEDIUS_\w+) = (-?\w+),$", RegexOptions.Multiline).Select(member => (member.Groups[1].Value, member.Groups[2].Value))]);
}
List<(string Constant, string Value)> Defined(string prefix) =>
    [.. Regex.Matches(header, $@"^#define ({prefix}\w+) (\d+)$", RegexOptions.Multiline).Select(define => (define.Groups[1].Value, define.Groups[2].Value))];
cEnums["MediusKey"] = ("MEDIUS_KEY_", "uint8_t", Defined("MEDIUS_KEY_"));
cEnums["MediusMediaKey"] = ("MEDIUS_MEDIA_", "uint16_t", Defined("MEDIUS_MEDIA_"));

// every #define and enum member as a number, for the suite's name tables
long ParseNumber(string text) => text.StartsWith('-') ? -ParseNumber(text[1..]) : text.StartsWith("0x") ? Convert.ToInt64(text[2..], 16) : long.Parse(text);
var macroValues = Regex.Matches(header, @"^#define (MEDIUS_\w+) (-?(?:0x[0-9A-Fa-f]+|\d+))$", RegexOptions.Multiline)
    .ToDictionary(define => define.Groups[1].Value, define => ParseNumber(define.Groups[2].Value));
foreach (var (constant, value) in cEnums.Values.SelectMany(entry => entry.Members))
{
    macroValues[constant] = ParseNumber(value);
}
long ValueOf(string expression) => Regex.IsMatch(expression, @"^-?(0x[0-9A-Fa-f]+|\d+)$") ? ParseNumber(expression) : macroValues[expression];
// each C++ enum the suite names: its members by value (the first name for a value with two), the other names that
// read as one of them, whether a binding refuses a value with no name, and the medius.h enum it is when there is one
var suiteEnums = new List<(string Cpp, string? C, List<(string Member, long Value)> Members, Dictionary<string, string> Aliases, bool CheckIn)>();
// the C++ enums that are any number, like Key
var suiteNumbers = new HashSet<string>();

foreach (var name in cEnums.Keys.Where(name => !Tables.Enums.ContainsKey(name)))
{
    problems.Add($"{name} is new in medius.h, give it an entry in Tables.Enums");
}
foreach (var name in Tables.Enums.Keys.Where(name => !cEnums.ContainsKey(name)))
{
    problems.Add($"{name} has an entry in Tables.Enums but medius.h doesn't have it any more");
}

// the C++ name of every enum, the C++ only ones too, so the other tables can name them
var cppEnums = Tables.Enums.Values.Select(use => use.Name).Concat(Tables.ExtraEnums.Select(extra => extra.Name)).ToHashSet();
string? EnumOf(string c) => Tables.Enums.TryGetValue(c, out var use) && use.Form == "class" ? use.Name : null;

var enumLines = new List<string>();
var enumDetail = new List<string>();
void EnumClass(string name, string doc, string underlying, List<(string Member, string Value)> members, List<string> aliasLines, string read, bool checkIn)
{
    enumLines.AddRange(Doc(doc));
    enumLines.Add($"enum class {name} : {underlying} {{");
    enumLines.AddRange(members.Select(member => $"    {member.Member} = {member.Value},"));
    enumLines.AddRange(aliasLines);
    enumLines.Add("};");
    enumLines.Add("");
    enumDetail.Add("template <>");
    enumDetail.Add($"struct Enum<{name}> {{");
    enumDetail.Add($"    static constexpr const char* name = \"{name}\";");
    enumDetail.Add($"    static constexpr bool checked = {(checkIn ? "true" : "false")};");
    enumDetail.Add($"    static constexpr Read read = Read::{read};");
    if (read == "Unknown")
    {
        enumDetail.Add($"    static constexpr {name} unknown = {name}::Unknown;");
    }
    enumDetail.Add("");
    enumDetail.Add($"    static constexpr bool named({name} value) noexcept {{");
    enumDetail.Add($"        switch (static_cast<{underlying}>(value)) {{");
    enumDetail.AddRange(members.Select(member => member.Value).Distinct().Select(value => $"        case {value}:"));
    enumDetail.Add("            return true;");
    enumDetail.Add("        default:");
    enumDetail.Add("            return false;");
    enumDetail.Add("        }");
    enumDetail.Add("    }");
    enumDetail.Add("};");
    enumDetail.Add("");
}
foreach (var (cname, (prefix, underlying, members)) in cEnums)
{
    if (!Tables.Enums.TryGetValue(cname, out var use))
    {
        continue;
    }
    var pinned = string.Join(" ", members.Select(member => $"{member.Constant}={member.Value}"));
    if (use.Pinned is not null && use.Pinned != pinned)
    {
        problems.Add($"{cname}'s members changed and code written by hand switches over them, so check it and pin the new ones: {pinned}");
    }
    foreach (var member in members.Where(member => !member.Constant.StartsWith(prefix)))
    {
        problems.Add($"{member.Constant} doesn't start with {prefix}");
    }
    var cppType = Number(underlying)!;
    if (use.Form == "struct")
    {
        // Rust has these as a newtype with a constant per name, so any value is one, like anybox's own Key
        string Upper(string suffix) => char.IsDigit(suffix[0]) ? "NUM" + suffix : suffix;
        enumLines.AddRange(Doc(use.Doc));
        enumLines.Add($"struct {use.Name} {{");
        enumLines.Add($"    {cppType} value;");
        enumLines.Add("");
        enumLines.Add($"    friend constexpr bool operator==({use.Name} a, {use.Name} b) noexcept {{ return a.value == b.value; }}");
        enumLines.Add($"    friend constexpr bool operator!=({use.Name} a, {use.Name} b) noexcept {{ return a.value != b.value; }}");
        enumLines.Add($"    friend constexpr bool operator<({use.Name} a, {use.Name} b) noexcept {{ return a.value < b.value; }}");
        enumLines.Add("");
        enumLines.AddRange(members.Select(member => $"    static const {use.Name} {Upper(member.Constant[prefix.Length..])};"));
        enumLines.Add("};");
        enumLines.Add("");
        enumLines.AddRange(members.Select(member => $"inline constexpr {use.Name} {use.Name}::{Upper(member.Constant[prefix.Length..])}{{{member.Constant}}};"));
        enumLines.Add("");
        suiteNumbers.Add(use.Name);
        continue;
    }
    var cppMembers = members.Select(member => (Member(member.Constant[prefix.Length..]), member.Constant)).ToList();
    var aliasLines = use.Aliases.Select(alias => $"    {alias.Key} = {alias.Value},").ToList();
    if (use.Form == "status")
    {
        enumLines.AddRange(Doc(use.Doc));
        enumLines.Add($"enum class {use.Name} : {cppType} {{");
        enumLines.AddRange(cppMembers.Select(member => $"    {member.Item1} = {member.Constant},"));
        enumLines.Add("};");
        enumLines.Add("");
        suiteEnums.Add((use.Name, cname, [.. cppMembers.Select(member => (member.Item1, ValueOf(member.Constant)))], [], false));
        continue;
    }
    EnumClass(use.Name, use.Doc, cppType, cppMembers, aliasLines, use.Read, use.CheckIn);
    suiteEnums.Add((use.Name, cname, [.. cppMembers.Select(member => (member.Item1, ValueOf(member.Constant)))], new(use.Aliases), use.CheckIn));
}
foreach (var extra in Tables.ExtraEnums)
{
    List<(string, string)> members = extra.Members.Length > 0
        ? [.. extra.Members.Select(member => (member.Member, member.Value))]
        : [.. Defined(extra.FromDefines!)
            .Where(define => !extra.Skip.Contains(define.Constant[extra.FromDefines!.Length..]) && int.Parse(define.Value) >= extra.From)
            .Select(define => (extra.Renames.GetValueOrDefault(define.Constant[extra.FromDefines!.Length..], Member(define.Constant[extra.FromDefines!.Length..])), define.Constant))];
    EnumClass(extra.Name, extra.Doc, "std::uint8_t", members, [], extra.Read, extra.CheckIn);
    suiteEnums.Add((extra.Name, null, [.. members.Select(member => (member.Item1, ValueOf(member.Item2)))], [], extra.CheckIn));
}
regions["enums"] = [.. enumLines, "namespace detail {", "", .. enumDetail, "}  // namespace detail"];

// the plain #defines, PascalCase like the C# package's, since names like MAX_PATH are a windows.h macro
regions["constants"] = [.. constants
    .Where(constant => !constant.Name.StartsWith("MEDIUS_CATCH_CLASS_") || constant.Name.EndsWith("_ANY"))
    .Select(constant => $"inline constexpr auto {Pascal(constant.Name["MEDIUS_".Length..])} = {constant.Name};")];

// one error class per status, all but ok, unknown and update, which has fields of its own and is written by hand
var errorLines = new List<string>();
var errorCases = new List<string>();
var statusNames = new List<string>();
foreach (var (constant, _) in cEnums["MediusStatus"].Members)
{
    var status = constant["MEDIUS_STATUS_".Length..];
    statusNames.Add($"    case Status::{Member(status)}:");
    statusNames.Add($"        return \"{Member(status)}\";");
    if (status is "OK" or "ERR_UNKNOWN")
    {
        continue;
    }
    var className = Pascal(status["ERR_".Length..]) + "Error";
    if (status == "ERR_UPDATE")
    {
        errorCases.Add($"    case {constant}:");
        errorCases.Add($"        return std::make_shared<{className}>(message, proto_ver, update);");
        continue;
    }
    errorLines.Add($"class {className} : public Error {{");
    errorLines.Add("public:");
    errorLines.Add($"    {className}(const std::string& message, std::uint8_t proto_ver) : Error(message, Status::{Member(status)}, proto_ver) {{}}");
    errorLines.Add("");
    errorLines.Add("private:");
    errorLines.Add("    void throw_self() const override { throw *this; }");
    errorLines.Add("};");
    errorLines.Add("");
    errorCases.Add($"    case {constant}:");
    errorCases.Add($"        return std::make_shared<{className}>(message, proto_ver);");
}
regions["errors"] = [
    .. errorLines,
    "namespace detail {",
    "",
    "inline std::string status_text(Status status) {",
    "    switch (status) {",
    .. statusNames,
    "    }",
    "    return std::to_string(static_cast<std::int32_t>(status));",
    "}",
    "",
    "// the error of that status, the one switch every medius error goes through",
    "inline std::shared_ptr<Error> make_error(MediusStatus status, const std::string& message, std::uint8_t proto_ver,",
    "                                         const std::optional<MediusUpdateError>& update) {",
    "    switch (status) {",
    .. errorCases,
    "    default:",
    "        return std::make_shared<Error>(message, static_cast<Status>(status), proto_ver);",
    "    }",
    "}",
    "",
    "}  // namespace detail",
];

// the structs, generated from their fields and the roles the table gives them
foreach (var name in recordNames.Where(name => !Tables.Records.ContainsKey(name)))
{
    problems.Add($"{name} is new in medius.h, give it an entry in Tables.Records");
}
foreach (var name in Tables.Records.Keys.Where(name => !recordNames.Contains(name)))
{
    problems.Add($"{name} has an entry in Tables.Records but medius.h doesn't have it any more");
}
string RecordName(string c) => Tables.Records.TryGetValue(c, out var use) ? use.Name : c;

// a count that can't hold what its array holds (or a whole match length) would cut a longer one on its way in
void CountsUpTo(string cname, string count, string type, long needed, string field)
{
    var most = aliases.GetValueOrDefault(type, type) switch
    {
        "uint8_t" => byte.MaxValue,
        "uint16_t" => ushort.MaxValue,
        "uint32_t" or "uintptr_t" or "uint64_t" => uint.MaxValue,
        _ => 0L,
    };
    if (most < needed)
    {
        problems.Add($"{cname}.{count} is a {type}, which can't count the {needed} {field} can take");
    }
}

// the C# side of a record comes from the same walk over its roles, so both bindings get the same fields, length
// rules and defaults
string CsCount(string count) => int.TryParse(count, out _) ? count : "Constants." + Pascal(count["MEDIUS_".Length..]);
string CsRecordName(string c) => Tables.Records.TryGetValue(c, out var use) && use is MadeRecord { CsName: { } name } ? name : RecordName(c);
(string Cs, string Read, bool CheckIn, string? Default) EnumPolicy(string cpp)
{
    if (Tables.Enums.FirstOrDefault(entry => entry.Value.Name == cpp) is { Key: { } cname, Value: { } use })
    {
        return (EnumName(cname), use.Read, use.CheckIn, use.Default);
    }
    var extra = Tables.ExtraEnums.Single(extra => extra.Name == cpp);
    return (extra.Name, extra.Read, extra.CheckIn, extra.Default);
}
string Words(string pascal) => Regex.Replace(pascal, "(?<=[a-z0-9])([A-Z])", " $1").ToLowerInvariant();
string Article(string words) => ("aeiou".Contains(words[0]) ? "an " : "a ") + words;
// a byte coming back the package has no name for, read the way the table says
string CsRead(string cpp, string value)
{
    var (cs, read, _, _) = EnumPolicy(cpp);
    return read switch
    {
        "Unknown" => $"(Enum.IsDefined({value}) ? {value} : {cs}.Unknown)",
        "Any" => value,
        _ => $"MediusLibrary.Out({value}, \"{Article(Words(cs))}\")",
    };
}
// `x` in a doc is the field's C# name when it's one of the record's, the C# spelling of a name otherwise
string CsDocText(string doc, Dictionary<string, string> fieldNames) => Regex.Replace(doc, "`([^`]+)`", match =>
{
    var name = match.Groups[1].Value;
    if (fieldNames.TryGetValue(name, out var field))
    {
        return $"<see cref=\"{field}\"/>";
    }
    if (name.StartsWith("MEDIUS_"))
    {
        return $"<c>Constants.{Pascal(name["MEDIUS_".Length..])}</c>";
    }
    return $"<c>{Regex.Replace(name.Replace("::", "."), @"(?<=\.)([a-z])([a-z_]*)", part => Pascal(part.Value))}</c>";
});
List<string> CsDoc(string text)
{
    // a tag like <see cref="X"/> stays on one line
    var words = Regex.Split($"<summary>{text}</summary>", " (?![^<]*>)");
    var lines = new List<string>();
    var line = "///";
    foreach (var word in words)
    {
        if (line.Length + 1 + word.Length > 120 && line != "///")
        {
            lines.Add(line);
            line = "///";
        }
        line += " " + word;
    }
    lines.Add(line);
    return lines;
}
var csRecordLines = new List<string>();
void CsRecord(MadeRecord made, string cname, Dictionary<string, string> fieldNames,
    List<(string Type, string Name, string From, string To, string Equality, string? Default, string? Static)> members)
{
    var name = made.CsName ?? made.Name;
    csRecordLines.Add("");
    csRecordLines.AddRange(CsDoc(CsDocText(made.CsDoc ?? made.Doc, fieldNames)));
    // a parameter can be left out where it's zero, empty, off or its enum's Rust default, like Rust's constructors
    // leave those out, and only when every one after it can be too
    var optional = new bool[members.Count];
    for (var index = members.Count - 1; index >= 0 && members[index].Default is not null; index--)
    {
        optional[index] = true;
    }
    csRecordLines.Add($"public sealed partial record {name}(");
    for (var index = 0; index < members.Count; index++)
    {
        var (type, field, _, _, _, initial, _) = members[index];
        csRecordLines.Add($"    {type} {field}{(optional[index] ? " = " + initial : "")}{(index < members.Count - 1 ? "," : ")")}");
    }
    csRecordLines.Add("{");
    if (made.Default)
    {
        csRecordLines.Add("    /// <summary>what Rust's Default gives</summary>");
        csRecordLines.Add($"    public static {name} Default {{ get; }} = new({string.Join(", ", members.Select(member => member.Static ?? member.Default))});");
        csRecordLines.Add("");
    }
    if (members.Any(member => member.Equality != "plain"))
    {
        // bytes and lists compare what's in them, like Rust's PartialEq
        csRecordLines.Add($"    public bool Equals({name}? other) => other is not null &&");
        for (var index = 0; index < members.Count; index++)
        {
            var (_, field, _, _, equality, _, _) = members[index];
            var same = equality == "plain" ? $"{field} == other.{field}" : $"MediusLibrary.Same({field}, other.{field})";
            csRecordLines.Add($"        {same}{(index < members.Count - 1 ? " &&" : ";")}");
        }
        csRecordLines.Add("");
        csRecordLines.Add("    public override int GetHashCode()");
        csRecordLines.Add("    {");
        csRecordLines.Add("        var hash = new HashCode();");
        foreach (var (_, field, _, _, equality, _, _) in members)
        {
            csRecordLines.Add(equality == "plain" ? $"        hash.Add({field});" : $"        hash.Add(MediusLibrary.Hash({field}));");
        }
        csRecordLines.Add("        return hash.ToHashCode();");
        csRecordLines.Add("    }");
        csRecordLines.Add("");
    }
    csRecordLines.Add($"    internal static unsafe {name} From({cname} raw) => new(");
    for (var index = 0; index < members.Count; index++)
    {
        csRecordLines.Add($"        {members[index].From}{(index < members.Count - 1 ? "," : ");")}");
    }
    csRecordLines.Add("");
    csRecordLines.Add($"    internal unsafe {cname} ToNative()");
    csRecordLines.Add("    {");
    csRecordLines.Add($"        var raw = new {cname}();");
    foreach (var member in members)
    {
        csRecordLines.AddRange(member.To.Split('\n').Select(line => "        " + line));
    }
    csRecordLines.Add("        return raw;");
    csRecordLines.Add("    }");
    csRecordLines.Add("}");
}

var structLines = new List<string>();
// the suite's JSON form of every made record, for the converters each runner gets
var suiteRecords = new List<(string C, List<JsonField> Fields)>();
foreach (var (kind, cname, fields) in records)
{
    if (!Tables.Records.TryGetValue(cname, out var use))
    {
        continue;
    }
    var pinned = string.Join(", ", fields.Select(field => field.Count is null ? $"{field.Type} {field.Name}" : $"{field.Type} {field.Name}[{field.Count}]"));
    if (use is HandRecord hand)
    {
        if (hand.Fields != pinned)
        {
            problems.Add($"{cname} changed in medius.h, so check {hand.Name}, which is written by hand, and pin its new fields: {pinned}");
        }
        continue;
    }
    var made = (MadeRecord)use;
    var roles = made.Roles.ToDictionary(role => role.Key, role => role.Value);
    var fieldNames = fields.Select(field => field.Name).ToHashSet();
    foreach (var key in roles.Keys.Where(key => !fieldNames.Contains(key)))
    {
        problems.Add($"{cname} has no field {key}, which its entry in Tables.Records gives a role");
    }
    // a count or a presence flag is folded into the field it belongs to, so it has to be there and be a number or
    // a flag, otherwise the generated code would read a field medius.h doesn't have
    foreach (var (key, role) in roles)
    {
        var folds = Regex.Match(role, @"^(if|bytes|match|list)\((\w+)\)$");
        if (folds.Success && !fields.Any(field => field.Name == folds.Groups[2].Value && field.Count is null &&
                (field.Type == "bool" || Number(field.Type) is not null)))
        {
            problems.Add($"{cname}.{key}'s role {role} names {folds.Groups[2].Value}, which isn't a number or flag field of {cname}");
        }
    }
    var folded = roles.Values.Select(role => Regex.Match(role, @"^(?:if|bytes|match|list)\((\w+)\)$")).Where(match => match.Success)
        .Select(match => match.Groups[1].Value).ToHashSet();
    var members = new List<(string Type, string Name, string Default, string From, string To)>();
    // the C# side of each field, its equality kind (plain, bytes or list), the default a parameter can take, and the
    // one Default uses where a parameter can't take it
    var csMembers = new List<(string Type, string Name, string From, string To, string Equality, string? Default, string? Static)>();
    // the suite's JSON form of each field, which every runner converts the same way
    var jsonFields = new List<JsonField>();
    // what Rust's Default gives a field when the record has one, an enum's default member or a constant the table names
    string? DefaultOf(string field, string? cppEnum)
    {
        if (made.Defaults.TryGetValue(field, out var constant))
        {
            return constant;
        }
        if (cppEnum is not null && made.Default)
        {
            if (EnumPolicy(cppEnum).Default is { } member)
            {
                return $"{cppEnum}::{member}";
            }
            problems.Add($"{cname} has a Default in Rust, so {cppEnum} needs one in its table entry for {field}");
        }
        return null;
    }
    string CsDefault(string cppDefault) => cppDefault.Contains("::")
        ? EnumPolicy(cppDefault.Split("::")[0]).Cs + "." + cppDefault.Split("::")[1]
        : "Constants." + cppDefault;
    // a C# parameter can take its enum's default on any record, a constructor leaving it out
    string? CsEnumDefault(string cppEnum, string? initial) => initial is not null ? CsDefault(initial)
        : EnumPolicy(cppEnum).Default is { } member ? $"{EnumPolicy(cppEnum).Cs}.{member}" : null;
    foreach (var (type, field, count) in fields)
    {
        if (folded.Contains(field))
        {
            continue;
        }
        var role = roles.GetValueOrDefault(field);
        var counted = role is null ? null : Regex.Match(role, @"^(\w+)\((\w+)\)$");
        var argument = counted is { Success: true } ? counted.Groups[2].Value : null;
        var verb = counted is { Success: true } ? counted.Groups[1].Value : role;
        var name = made.CsNames.GetValueOrDefault(field, Member(field));
        var raw = $"raw.{Id(field)}";
        if (count is not null)
        {
            var capacity = CsCount(count);
            if (type == "char" && verb is null)
            {
                members.Add(("std::string", field, "", $"out.{field} = detail::text(raw.{field});", $"detail::text({field}, raw.{field});"));
                csMembers.Add(("string", name, $"MediusLibrary.Text(new ReadOnlySpan<byte>({raw}, {capacity}))",
                    $"MediusLibrary.Text({name}, {raw}, {capacity});", "plain", "\"\"", null));
                jsonFields.Add(new(field, "text", name));
            }
            else if (type == "char" && verb == "if")
            {
                members.Add(("std::optional<std::string>", field, "",
                    $"if (raw.{argument}) {{\n    out.{field} = detail::text(raw.{field});\n}}",
                    $"raw.{argument} = static_cast<std::uint8_t>({field}.has_value());\nif ({field}) {{\n    detail::text(*{field}, raw.{field});\n}}"));
                csMembers.Add(("string?", name, $"raw.{argument} != 0 ? MediusLibrary.Text(new ReadOnlySpan<byte>({raw}, {capacity})) : null",
                    $"raw.{argument} = MediusLibrary.Flag({name} is not null);\nif ({name} is not null)\n{{\n    MediusLibrary.Text({name}, {raw}, {capacity});\n}}",
                    "plain", "null", null));
                jsonFields.Add(new(field, "opttext", name, Count: argument));
            }
            else if (IsByte(type) && verb == "array")
            {
                members.Add(($"std::array<std::uint8_t, {count}>", field, "{}",
                    $"std::copy(std::begin(raw.{field}), std::end(raw.{field}), out.{field}.begin());",
                    $"std::copy({field}.begin(), {field}.end(), raw.{field});"));
                csMembers.Add(("ReadOnlyMemory<byte>", name, $"MediusLibrary.Bytes({raw}, {capacity}, {capacity})",
                    $"MediusLibrary.Exact({name}.Span, {raw}, {capacity}, nameof({name}));", "bytes", null, $"new byte[{capacity}]"));
                jsonFields.Add(new(field, "array", name));
            }
            else if (IsByte(type) && verb is "bytes" or "match" && argument is not null)
            {
                var countType = fields.FirstOrDefault(other => other.Name == argument).Type ?? "uint16_t";
                CountsUpTo(cname, argument, countType, verb == "match" ? 0xFFFF : Count(count), field);
                members.Add(("std::vector<std::uint8_t>", field, "", $"out.{field} = detail::bytes(raw.{field}, raw.{argument});",
                    $"raw.{argument} = static_cast<{Number(countType)}>(detail::{verb}({field}, raw.{field}, \"{field}\"));"));
                csMembers.Add(("ReadOnlyMemory<byte>", name, $"MediusLibrary.Bytes({raw}, raw.{argument}, {capacity})",
                    $"raw.{argument} = ({Primitive(aliases.GetValueOrDefault(countType, countType))})MediusLibrary.{(verb == "match" ? "Match" : "Copy")}({name}.Span, {raw}, {capacity}, nameof({name}));",
                    "bytes", "default", null));
                jsonFields.Add(new(field, verb!, name, Count: argument));
            }
            else if (recordNames.Contains(type) && verb == "list" && argument is not null)
            {
                var countType = fields.FirstOrDefault(other => other.Name == argument).Type ?? "uint16_t";
                CountsUpTo(cname, argument, countType, Count(count), field);
                members.Add(($"std::vector<{RecordName(type)}>", field, "", $"out.{field} = detail::list<{RecordName(type)}>(raw.{field}, raw.{argument});",
                    $"raw.{argument} = static_cast<{Number(countType)}>(detail::list({field}, raw.{field}, \"{field}\"));"));
                var element = CsRecordName(type);
                csMembers.Add(($"IReadOnlyList<{element}>", name, $"MediusLibrary.ListOf<{type}, {element}>({raw}, raw.{argument}, {element}.From)",
                    $"raw.{argument} = ({Primitive(aliases.GetValueOrDefault(countType, countType))})MediusLibrary.Fill<{element}, {type}>({name}, {raw}, nameof({name}), item => item.ToNative());",
                    "list", null, "[]"));
                jsonFields.Add(new(field, "list", name, Count: argument, Element: type));
            }
            else
            {
                problems.Add($"{cname}.{field} is an array that needs a role in Tables.Records: array, bytes(count), match(count) or list(count)");
            }
            continue;
        }
        if (recordNames.Contains(type))
        {
            var element = CsRecordName(type);
            if (verb == "if" && argument is not null)
            {
                members.Add(($"std::optional<{RecordName(type)}>", field, "",
                    $"if (raw.{argument}) {{\n    out.{field} = {RecordName(type)}::from_c(raw.{field});\n}}",
                    $"raw.{argument} = static_cast<std::uint8_t>({field}.has_value());\nif ({field}) {{\n    raw.{field} = {field}->to_c();\n}}"));
                csMembers.Add(($"{element}?", name, $"raw.{argument} != 0 ? {element}.From({raw}) : null",
                    $"raw.{argument} = MediusLibrary.Flag({name} is not null);\nif ({name} is not null)\n{{\n    {raw} = {name}.ToNative();\n}}",
                    "plain", "null", null));
                jsonFields.Add(new(field, "optrecord", name, Count: argument, Element: type));
            }
            else if (verb is null)
            {
                members.Add((RecordName(type), field, "", $"out.{field} = {RecordName(type)}::from_c(raw.{field});", $"raw.{field} = {field}.to_c();"));
                if (made.Default && !(Tables.Records[type] is MadeRecord { Default: true }))
                {
                    problems.Add($"{cname} has a Default in Rust, so {type}, which its {field} is, needs one too");
                }
                csMembers.Add((element, name, $"{element}.From({raw})", $"{raw} = {name}.ToNative();", "plain", null, $"{element}.Default"));
                jsonFields.Add(new(field, "record", name, Element: type));
            }
            else
            {
                problems.Add($"{cname}.{field} is a struct, its role can only be if(flag)");
            }
            continue;
        }
        if (EnumOf(type) is { } typed && verb is null)
        {
            var initial = DefaultOf(field, typed);
            members.Add((typed, field, initial is null ? "{}" : $"{{{initial}}}", $"out.{field} = detail::read<{typed}>(raw.{field});", $"raw.{field} = static_cast<{type}>(detail::write({field}));"));
            var (csEnum, _, checkIn, _) = EnumPolicy(typed);
            csMembers.Add((csEnum, name, CsRead(typed, raw), checkIn ? $"{raw} = MediusLibrary.Named({name}, nameof({name}));" : $"{raw} = {name};",
                "plain", CsEnumDefault(typed, initial), null));
            jsonFields.Add(new(field, "enum", name, Enum: typed, CType: type));
            continue;
        }
        if (type == "bool" && verb is null or "flag")
        {
            members.Add(("bool", field, " = false", $"out.{field} = raw.{field};", $"raw.{field} = {field};"));
            csMembers.Add(("bool", name, $"{raw} != 0", $"{raw} = MediusLibrary.Flag({name});", "plain", "false", null));
            jsonFields.Add(new(field, "flag", name, CType: type));
            continue;
        }
        if (Number(type) is not { } number)
        {
            problems.Add($"{cname}.{field} has the type {type}, which the C++ side doesn't know");
            continue;
        }
        if (IsByte(type) && verb is null)
        {
            problems.Add($"{cname}.{field} is a byte, so give it a role in Tables.Records: number, flag or the enum it holds");
            continue;
        }
        var primitive = Primitive(aliases.GetValueOrDefault(type, type))!;
        if (verb == "flag")
        {
            members.Add(("bool", field, " = false", $"out.{field} = raw.{field} != 0;", $"raw.{field} = static_cast<{number}>({field});"));
            csMembers.Add(("bool", name, $"{raw} != 0", $"{raw} = MediusLibrary.Flag({name});", "plain", "false", null));
            jsonFields.Add(new(field, "flag", name, CType: type));
        }
        else if (verb is null or "number")
        {
            var initial = DefaultOf(field, null);
            members.Add((number, field, $" = {initial ?? "0"}", $"out.{field} = raw.{field};", $"raw.{field} = {field};"));
            csMembers.Add((primitive, name, raw, $"{raw} = {name};", "plain", initial is null ? "0" : CsDefault(initial), null));
            jsonFields.Add(new(field, "number", name, CType: type));
        }
        else if (verb == "optional" && argument is not null)
        {
            var initial = DefaultOf(field, null);
            members.Add(($"std::optional<{number}>", field, initial is null ? "" : $" = static_cast<{number}>({initial})",
                // always set, since a record whose default isn't empty starts with it
                $"out.{field} = raw.{field} != {argument} ? std::optional<{number}>(raw.{field}) : std::nullopt;",
                $"raw.{field} = {field} ? *{field} : static_cast<{number}>({argument});"));
            var none = argument.StartsWith("MEDIUS_") ? "Constants." + Pascal(argument["MEDIUS_".Length..]) : argument;
            csMembers.Add(($"{primitive}?", name, $"{raw} == {none} ? null : {raw}", $"{raw} = ({primitive})({name} ?? {none});",
                "plain", initial is null ? "null" : CsDefault(initial), null));
            jsonFields.Add(new(field, "optional", name, CType: type, Sentinel: argument));
        }
        else if (cppEnums.Contains(verb!) && IsByte(type))
        {
            var initial = DefaultOf(field, verb);
            members.Add((verb!, field, initial is null ? "{}" : $"{{{initial}}}", $"out.{field} = detail::read<{verb}>(raw.{field});", $"raw.{field} = detail::write({field});"));
            var (csEnum, _, checkIn, _) = EnumPolicy(verb!);
            csMembers.Add((csEnum, name, CsRead(verb!, $"({csEnum}){raw}"),
                checkIn ? $"{raw} = MediusLibrary.In({name}, nameof({name}));" : $"{raw} = (byte){name};",
                "plain", CsEnumDefault(verb!, initial), null));
            jsonFields.Add(new(field, "enum", name, Enum: verb, CType: type));
        }
        else
        {
            problems.Add($"{cname}.{field} has the role {role}, which doesn't fit a {type}");
        }
    }
    // Rust's order where medius.h's isn't it, for both bindings
    if (made.Order is { } order)
    {
        if (!order.Order().SequenceEqual(members.Select(member => member.Name).Order()))
        {
            problems.Add($"{cname}'s Order in Tables.Records has to name every one of its fields: {string.Join(", ", members.Select(member => member.Name))}");
        }
        else
        {
            var cppOrder = members.ToDictionary(member => member.Name);
            var csOrder = members.Zip(csMembers).ToDictionary(pair => pair.First.Name, pair => pair.Second);
            members = [.. order.Select(field => cppOrder[field])];
            csMembers = [.. order.Select(field => csOrder[field])];
        }
    }
    suiteRecords.Add((cname, jsonFields));
    CsRecord(made, cname, fields.Where(field => !folded.Contains(field.Name)).ToDictionary(field => field.Name, field => made.CsNames.GetValueOrDefault(field.Name, Member(field.Name))), csMembers);
    structLines.AddRange(Doc(made.Doc));
    structLines.Add($"struct {made.Name} {{");
    structLines.AddRange(members.Select(member => $"    {member.Type} {member.Name}{member.Default};"));
    if (made.Members.Length > 0)
    {
        structLines.Add("");
        foreach (var member in made.Members)
        {
            structLines.AddRange(Doc(member.Doc, 4).Select(line => "    " + line));
            structLines.Add($"    {member.Declaration};");
        }
    }
    structLines.Add("");
    structLines.Add($"    static {made.Name} from_c(const {cname}& raw);");
    structLines.Add($"    {cname} to_c() const;");
    structLines.Add("");
    structLines.Add($"    friend bool operator==(const {made.Name}& a, const {made.Name}& b) noexcept {{");
    structLines.Add($"        return {string.Join(" && ", members.Select(member => $"a.{member.Name} == b.{member.Name}"))};");
    structLines.Add("    }");
    structLines.Add($"    friend bool operator!=(const {made.Name}& a, const {made.Name}& b) noexcept {{ return !(a == b); }}");
    structLines.Add("};");
    structLines.Add("");
    structLines.Add($"inline {made.Name} {made.Name}::from_c(const {cname}& raw) {{");
    structLines.Add($"    {made.Name} out;");
    structLines.AddRange(members.SelectMany(member => member.From.Split('\n')).Select(line => "    " + line));
    structLines.Add("    return out;");
    structLines.Add("}");
    structLines.Add("");
    structLines.Add($"inline {cname} {made.Name}::to_c() const {{");
    structLines.Add($"    {cname} raw{{}};");
    structLines.AddRange(members.SelectMany(member => member.To.Split('\n')).Select(line => "    " + line));
    structLines.Add("    return raw;");
    structLines.Add("}");
    structLines.Add("");
}
// declared first, so a helper can name a struct that comes later in medius.h
regions["structs"] = [
    .. Tables.Records.Values.OfType<MadeRecord>().Select(made => $"struct {made.Name};"),
    "",
    .. structLines,
];

// every made function the suite can call through its generated dispatch
var suiteCalls = new List<SuiteCall>();
// the methods of every class, and the functions without a handle
var classes = new Dictionary<string, (string Class, string Prefix, string Region)>
{
    ["MediusDevice"] = ("Device", "medius_device_", "device"),
    ["MediusClip"] = ("Clip", "medius_clip_", "clip"),
    ["MediusClipFrame"] = ("ClipFrame", "medius_clip_frame_", "clip_frame"),
    ["MediusClipBuilder"] = ("ClipBuilder", "medius_clip_builder_", "clip_builder"),
    ["MediusTimeline"] = ("Timeline", "medius_timeline_", "timeline"),
    ["MediusMockBox"] = ("MockBox", "medius_mock_", "mock"),
};
foreach (var region in classes.Values.Select(owner => owner.Region).Append("free"))
{
    regions[region] = [];
}
foreach (var name in Tables.Functions.Keys.Where(name => !cfunctions.Any(function => function.Name == name)))
{
    problems.Add($"{name} has an entry in Tables.Functions but medius.h doesn't have it any more");
}
foreach (var function in cfunctions)
{
    if (!Tables.Functions.TryGetValue(function.Name, out var use))
    {
        problems.Add($"{function.Name} is new in medius.h, give it an entry in Tables.Functions: {function.Signature}");
        continue;
    }
    if (use is HandFunction hand)
    {
        if (hand.Signature != function.Signature)
        {
            problems.Add($"{function.Name} changed in medius.h, so check {hand.Where}, which is written by hand for it, and pin the new one: {function.Signature}");
        }
        if (hand.Mock != function.Mock)
        {
            problems.Add($"{function.Name} is {(function.Mock ? "" : "not ")}only there with the mock now, so move {hand.Where}, which is written by hand for it, {(function.Mock ? "inside" : "out of")} MEDIUS_FEATURE_MOCK and set its Mock");
        }
        continue;
    }
    var made = (MadeFunction)use;
    string? rename = null;
    // const for a method that only reads, which medius.h can't say when the C side takes the handle without it
    var reads = false;
    // a free function that skips the ABI check, for the one that has to answer when the check fails
    var skipsCheck = false;
    var roles = new Dictionary<string, string>();
    var defaults = new Dictionary<string, string>();
    foreach (var token in made.Tokens)
    {
        if (token.StartsWith("as:"))
        {
            rename = token[3..];
            continue;
        }
        if (token == "const")
        {
            reads = true;
            continue;
        }
        if (token == "unchecked")
        {
            skipsCheck = true;
            continue;
        }
        var equals = token.IndexOf('=');
        var head = equals < 0 ? token : token[..equals];
        var colon = head.IndexOf(':');
        var parameter = colon < 0 ? head : head[..colon];
        if (colon >= 0)
        {
            roles[parameter] = head[(colon + 1)..];
        }
        if (equals >= 0)
        {
            defaults[parameter] = token[(equals + 1)..];
        }
    }
    var first = function.Parameters.FirstOrDefault();
    var owned = first is { Stars: 1 } && classes.ContainsKey(first.Type);
    var (className, prefix, regionName) = owned ? classes[first!.Type] : ("", "medius_", "free");
    if (!function.Name.StartsWith(prefix))
    {
        problems.Add($"{function.Name} takes a {first!.Type} but doesn't start with {prefix}, so give it a name with as:");
    }
    var methodName = rename ?? function.Name[Math.Min(prefix.Length, function.Name.Length)..];
    var suiteRoles = new Dictionary<string, string>(roles);
    if (function.Mock != (className == "MockBox"))
    {
        problems.Add($"{function.Name} is {(function.Mock ? "" : "not ")}only there with the mock, so it can't be generated into {(className == "" ? "the free functions" : className)}");
    }
    var declared = new List<string>();
    var arguments = new List<string>();
    var before = new List<string>();
    string? returned = null;
    string? outDeclaration = null;
    for (var at = owned ? 1 : 0; at < function.Parameters.Count; at++)
    {
        var parameter = function.Parameters[at];
        roles.Remove(parameter.Name, out var role);
        defaults.Remove(parameter.Name, out var fallback);
        string? declaration = null;
        if (parameter.Callback)
        {
            problems.Add($"{function.Name} takes a callback, which only code written by hand can pass");
        }
        else if (parameter.Stars == 0 && Tables.Records.ContainsKey(parameter.Type) && role is null)
        {
            declaration = $"const {RecordName(parameter.Type)}& {parameter.Name}";
            arguments.Add($"{parameter.Name}.to_c()");
        }
        else if (parameter.Stars == 0 && parameter.Type == "bool" && role is null or "flag")
        {
            declaration = $"bool {parameter.Name}";
            arguments.Add(parameter.Name);
        }
        else if (parameter.Stars == 0 && IsByte(parameter.Type))
        {
            switch (role)
            {
                case null:
                    problems.Add($"{function.Name}'s {parameter.Name} is a byte, so give it a role in Tables.Functions: number, flag or the enum it takes");
                    break;
                case "number":
                    declaration = $"std::uint8_t {parameter.Name}";
                    arguments.Add(parameter.Name);
                    break;
                case "flag":
                    declaration = $"bool {parameter.Name}";
                    arguments.Add($"static_cast<std::uint8_t>({parameter.Name})");
                    break;
                case var enumName when cppEnums.Contains(enumName):
                    declaration = $"{enumName} {parameter.Name}";
                    arguments.Add($"detail::write({parameter.Name})");
                    break;
                default:
                    problems.Add($"{function.Name}'s {parameter.Name} has the role {role}, which isn't number, flag or an enum");
                    break;
            }
        }
        else if (parameter.Stars == 0 && Number(parameter.Type) is { } number && role is null or "number")
        {
            declaration = $"{number} {parameter.Name}";
            arguments.Add(parameter.Name);
        }
        else if (parameter.Stars == 0 && EnumOf(parameter.Type) is { } typed && role is null)
        {
            declaration = $"{typed} {parameter.Name}";
            arguments.Add($"static_cast<{parameter.Type}>(detail::write({parameter.Name}))");
        }
        else if (parameter.Stars == 1 && parameter.Const && IsByte(parameter.Type) && at + 1 < function.Parameters.Count &&
                 function.Parameters[at + 1] is { Type: "uintptr_t", Stars: 0 } && role is null)
        {
            declaration = $"const std::vector<std::uint8_t>& {parameter.Name}";
            arguments.Add($"{parameter.Name}.data(), {parameter.Name}.size()");
            at++;
        }
        else if (parameter.Stars == 1 && parameter.Const && parameter.Type == "char" && role is null)
        {
            declaration = $"const std::string& {parameter.Name}";
            arguments.Add($"anybox::detail::c_text({parameter.Name}).c_str()");
        }
        else if (parameter.Stars == 1 && parameter.Const && Tables.Records.ContainsKey(parameter.Type) && role is null)
        {
            declaration = $"const {RecordName(parameter.Type)}& {parameter.Name}";
            before.Add($"const auto {parameter.Name}_c = {parameter.Name}.to_c();");
            arguments.Add($"&{parameter.Name}_c");
        }
        else if (parameter.Stars == 1 && parameter.Const && classes.ContainsKey(parameter.Type) && role is null)
        {
            declaration = $"const {classes[parameter.Type].Class}& {parameter.Name}";
            arguments.Add($"{parameter.Name}.get()");
        }
        else if (parameter.Stars == 1 && !parameter.Const && at == function.Parameters.Count - 1 && parameter.Name.StartsWith("out") && role is null)
        {
            if (Tables.Records.ContainsKey(parameter.Type))
            {
                outDeclaration = $"{parameter.Type} out{{}};";
                returned = $"{RecordName(parameter.Type)}::from_c(out)";
                declaration = "";
            }
            else if (Number(parameter.Type) is { } outNumber)
            {
                outDeclaration = $"{outNumber} out = 0;";
                returned = "out";
                declaration = "";
            }
            else
            {
                problems.Add($"{function.Name} writes a {parameter.Type}, which the C++ side doesn't know");
            }
            arguments.Add("&out");
        }
        else
        {
            problems.Add($"{function.Name}'s {parameter.Name} ({parameter.Type}{new string('*', parameter.Stars)}{(role is null ? "" : $", role {role}")}) fits no shape the generator knows, so write it by hand");
        }
        if (declaration is { Length: > 0 })
        {
            declared.Add(fallback is null ? declaration : $"{declaration} = {fallback}");
        }
    }
    foreach (var left in roles.Keys.Concat(defaults.Keys))
    {
        problems.Add($"{function.Name} has no parameter {left} the generator could use, which its entry in Tables.Functions names");
    }
    var outReturn = returned is null ? null : Regex.Match(returned, @"^(\w+)::from_c").Groups[1].Value;
    var call = $"{function.Name}({string.Join(", ", (owned ? new[] { "get()" } : Array.Empty<string>()).Concat(arguments))})";
    var chained = className is "ClipBuilder" or "ClipFrame";
    var isConst = className is "Device" or "Clip" or "MockBox" || (first?.Const ?? false) || reads;
    var qualifier = owned && isConst ? " const" : "";
    var parameters = string.Join(", ", declared);
    var lines = new List<string>();
    string? signature;
    var body = new List<string>(before);
    if (outDeclaration is not null)
    {
        body.Add(outDeclaration);
    }
    if (function.Return == "MediusStatus" && function.ReturnStars == 0)
    {
        body.Add($"detail::check({call});");
        if (returned is not null)
        {
            signature = $"{(outReturn is { Length: > 0 } ? outReturn : Number(function.Parameters[^1].Type))} {methodName}({parameters}){qualifier}";
            body.Add($"return {returned};");
        }
        else if (chained)
        {
            signature = $"{className}& {methodName}({parameters}){qualifier}";
            body.Add("return *this;");
        }
        else
        {
            signature = $"void {methodName}({parameters}){qualifier}";
        }
    }
    else if (returned is not null)
    {
        problems.Add($"{function.Name} returns a {function.Return} and writes an out too, which only code written by hand can tell apart");
        continue;
    }
    else if (function.Return == "void" && function.ReturnStars == 0)
    {
        signature = $"void {methodName}({parameters}){qualifier}";
        body.Add($"{call};");
    }
    else if (function.Return == "bool" && function.ReturnStars == 0)
    {
        signature = $"bool {methodName}({parameters}){qualifier}";
        body.Add($"return {call};");
    }
    else if (Number(function.Return) is { } returnNumber && function.ReturnStars == 0)
    {
        signature = $"{returnNumber} {methodName}({parameters}){qualifier}";
        body.Add($"return {call};");
    }
    else if (function.Return == "char" && function.ReturnStars == 1)
    {
        signature = $"std::string {methodName}({parameters}){qualifier}";
        body.Add($"return anybox::detail::text({call});");
    }
    else
    {
        problems.Add($"{function.Name} returns a {function.Return}{new string('*', function.ReturnStars)}, which the generator can't, so write it by hand");
        continue;
    }
    if (!owned)
    {
        signature = "inline " + signature;
        if (!skipsCheck)
        {
            body.Insert(0, "detail::ready();");
        }
    }
    suiteCalls.Add(new SuiteCall(owned ? className : "", methodName, function, suiteRoles, [.. declared.Select(param => Regex.Replace(param, @" = .*$", ""))],
        returned is not null ? "out" : function.Return == "MediusStatus" ? (chained ? "chained" : "void") : function.ReturnStars == 1 ? "text" : function.Return));
    lines.AddRange(Doc(made.Doc, owned ? 4 : 0));
    if (body.Count == 1 && signature.Length + body[0].Length + 6 + (owned ? 4 : 0) <= 120)
    {
        lines.Add($"{signature} {{ {body[0]} }}");
    }
    else
    {
        lines.Add($"{signature} {{");
        lines.AddRange(body.Select(line => "    " + line));
        lines.Add("}");
    }
    lines.Add("");
    regions[regionName].AddRange(lines);
}
foreach (var region in regions.Values.Where(region => region.Count > 0 && region[^1] == ""))
{
    region.RemoveAt(region.Count - 1);
}

// the regions go between their marks, and a mark the generator doesn't know is refused like bindings/gen does
const string CppStart = "made by gen-medius from medius.h ";
const string CppEnd = "the end of what gen-medius makes ";
var cppPath = Path.Combine(root, "bindings/c/include/anybox_medius.hpp");
var cpp = File.ReadAllText(cppPath).ReplaceLineEndings("\n");
foreach (Match mark in Regex.Matches(cpp, $@"(?:{Regex.Escape(CppStart)}|{Regex.Escape(CppEnd)})\((\w+)\)"))
{
    if (!regions.ContainsKey(mark.Groups[1].Value))
    {
        problems.Add($"anybox_medius.hpp has a mark for {mark.Groups[1].Value}, a region the generator doesn't make");
    }
}
foreach (var (region, lines) in regions)
{
    var all = cpp.Split('\n').ToList();
    var starts = all.Select((line, at) => (line, at)).Where(entry => entry.line.Contains($"{CppStart}({region})")).ToList();
    var ends = all.Select((line, at) => (line, at)).Where(entry => entry.line.Contains($"{CppEnd}({region})")).ToList();
    if (starts.Count != 1 || ends.Count != 1 || ends[0].at < starts[0].at)
    {
        problems.Add($"anybox_medius.hpp needs one start mark for {region} and one end mark after it");
        continue;
    }
    var indent = new string(all[starts[0].at].TakeWhile(char.IsWhiteSpace).ToArray());
    cpp = string.Join("\n", all.Take(starts[0].at + 1).Concat(lines.Select(line => line.Length == 0 ? "" : indent + line)).Concat(all.Skip(ends[0].at)));
}

// what the table says is written by hand has to be, so an entry can't claim a function nothing calls
foreach (var function in cfunctions)
{
    if (Tables.Functions.GetValueOrDefault(function.Name) is HandFunction hand && !hand.Where.StartsWith("nothing") &&
        !Regex.IsMatch(cpp, $@"\b{function.Name}\b"))
    {
        problems.Add($"{function.Name} is written by hand in {hand.Where}, the table says, but anybox_medius.hpp never calls it");
    }
}

// the Medius suite's side of each runner, from the same walks: the Rust recorder's converters and dispatch over
// medius-capi's own structs and functions, the C++ runner's over anybox_medius.hpp, and the C# runner's names. a value
// is the same JSON in all three, a record keyed by medius.h's field names with its counts and presence flags folded in
string RustSnake(string c) => Snake(c["Medius".Length..]).ToLowerInvariant();
// medius-capi's Rust structs call class_ class, it's only C++ that needs the underscore
string RustField(string field) => field == "class_" ? "class" : field;
string? RustNumber(string c) => aliases.GetValueOrDefault(c, c) switch
{
    "uint8_t" => "u8",
    "uint16_t" => "u16",
    "uint32_t" => "u32",
    "uint64_t" => "u64",
    "int8_t" => "i8",
    "int16_t" => "i16",
    "int32_t" => "i32",
    "int64_t" => "i64",
    "uintptr_t" => "usize",
    "float" => "f32",
    _ => null,
};
var suiteRecordNames = suiteRecords.Select(record => record.C).ToHashSet();
// the Rust enums medius-capi has for medius.h's enum blocks, which a byte can't just be cast to
var rustEnums = new SortedSet<string>();
void RustEnumUsed(string c)
{
    if (cEnums.ContainsKey(c) && c is not ("MediusKey" or "MediusMediaKey"))
    {
        rustEnums.Add(c);
    }
}

var suiteRust = new List<string>
{
    "// made by bindings/csharp/tools/gen-medius.cs from bindings/c/include/medius.h and its tables, don't edit it by hand.",
    "// every record gets both directions and only some get used, and medius-capi marks some C functions unsafe and",
    "// some not, which medius.h doesn't say",
    "#![allow(dead_code, unused_unsafe)]",
    "",
    "use serde_json::{Map, Value};",
    "",
    "use super::*;",
    "",
};
// a record from medius-capi, then one going back, refusing what a binding refuses before medius sees it
foreach (var (cname, fields) in suiteRecords)
{
    var snake = RustSnake(cname);
    suiteRust.Add($"pub(super) fn {snake}_to_json(raw: &{cname}) -> Value {{");
    suiteRust.Add("    let mut out = Map::new();");
    foreach (var field in fields)
    {
        var at = $"raw.{RustField(field.Field)}";
        var value = field.Kind switch
        {
            "text" => $"text(&{at})",
            "opttext" => $"if raw.{field.Count} != 0 {{ text(&{at}) }} else {{ Value::Null }}",
            "array" => $"hex(&{at})",
            "bytes" or "match" => $"hex(&{at}[..usize::from(raw.{field.Count}).min({at}.len())])",
            "list" => $"Value::Array({at}[..usize::from(raw.{field.Count}).min({at}.len())].iter().map({RustSnake(field.Element!)}_to_json).collect())",
            "record" => $"{RustSnake(field.Element!)}_to_json(&{at})",
            "optrecord" => $"if raw.{field.Count} != 0 {{ {RustSnake(field.Element!)}_to_json(&{at}) }} else {{ Value::Null }}",
            "enum" => $"enum_name(\"{field.Enum}\", {at} as i64)",
            "flag" => field.CType == "bool" ? $"Value::from({at})" : $"Value::from({at} != 0)",
            "number" => $"Value::from({at})",
            "optional" => $"if {at} == {field.Sentinel} {{ Value::Null }} else {{ Value::from({at}) }}",
            _ => throw new InvalidOperationException(field.Kind),
        };
        suiteRust.Add($"    out.insert(\"{field.Field}\".into(), {value});");
    }
    suiteRust.Add("    Value::Object(out)");
    suiteRust.Add("}");
    suiteRust.Add("");
    suiteRust.Add($"pub(super) fn {snake}_from_json(value: &Value) -> Result<{cname}, Failure> {{");
    suiteRust.Add("    // every medius.h struct is plain data and every enum in one has a member at 0, so zero is a valid value");
    suiteRust.Add($"    let mut raw: {cname} = unsafe {{ std::mem::zeroed() }};");
    foreach (var field in fields)
    {
        var at = $"raw.{RustField(field.Field)}";
        var from = $"&value[\"{field.Field}\"]";
        var what = $"\"{cname}.{field.Field}\"";
        switch (field.Kind)
        {
            case "text":
                suiteRust.Add($"    write_text(&mut {at}, {from});");
                break;
            case "opttext":
                suiteRust.Add($"    raw.{field.Count} = u8::from(!value[\"{field.Field}\"].is_null());");
                suiteRust.Add($"    write_text(&mut {at}, {from});");
                break;
            case "array":
                suiteRust.Add($"    {at} = exact({from}, {what})?;");
                break;
            case "bytes":
                suiteRust.Add($"    raw.{field.Count} = fill(&mut {at}, {from}, {what})?.try_into().unwrap();");
                break;
            case "match":
                suiteRust.Add($"    raw.{field.Count} = fill_match(&mut {at}, {from}, {what})?;");
                break;
            case "list":
                suiteRust.Add($"    raw.{field.Count} = fill_list(&mut {at}, {from}, {what}, {RustSnake(field.Element!)}_from_json)?.try_into().unwrap();");
                break;
            case "record":
                suiteRust.Add($"    {at} = {RustSnake(field.Element!)}_from_json({from})?;");
                break;
            case "optrecord":
                suiteRust.Add($"    if !value[\"{field.Field}\"].is_null() {{");
                suiteRust.Add($"        raw.{field.Count} = 1;");
                suiteRust.Add($"        {at} = {RustSnake(field.Element!)}_from_json({from})?;");
                suiteRust.Add("    }");
                break;
            case "enum":
                if (cEnums.ContainsKey(field.CType!) && field.CType is not ("MediusKey" or "MediusMediaKey"))
                {
                    RustEnumUsed(field.CType!);
                    suiteRust.Add($"    {at} = {RustSnake(field.CType!)}_from(enum_value(\"{field.Enum}\", {from})?)?;");
                }
                else
                {
                    suiteRust.Add($"    {at} = enum_value(\"{field.Enum}\", {from})? as {RustNumber(field.CType!)};");
                }
                break;
            case "flag":
                suiteRust.Add(field.CType == "bool" ? $"    {at} = flag({from});" : $"    {at} = {RustNumber(field.CType!)}::from(flag({from}));");
                break;
            case "number":
                suiteRust.Add($"    {at} = number({from}, {what});");
                break;
            case "optional":
                suiteRust.Add($"    {at} = if value[\"{field.Field}\"].is_null() {{ {field.Sentinel} }} else {{ number({from}, {what}) }};");
                break;
        }
    }
    suiteRust.Add("    Ok(raw)");
    suiteRust.Add("}");
    suiteRust.Add("");
}

// the generated functions, each one a C call with its arguments read the way its C++ method takes them
var rustCalls = new List<string>();
var cppCalls = new List<string>();
string RustHandle(string c) => classes[c].Class switch
{
    "Device" => "ctx.device",
    "Clip" => "ctx.clip",
    "ClipFrame" => "ctx.frame",
    "ClipBuilder" => "ctx.builder",
    "Timeline" => "ctx.timeline",
    _ => "ctx.mock",
};
string CppHandle(string cls) => cls switch
{
    "Device" => "ctx.device()",
    "Clip" => "ctx.clip()",
    "ClipFrame" => "ctx.frame()",
    "ClipBuilder" => "ctx.builder()",
    "Timeline" => "ctx.timeline()",
    _ => "ctx.mock()",
};
foreach (var call in suiteCalls)
{
    var function = call.Function;
    var owned = call.Class != "";
    var setup = new List<string>();
    var cArguments = new List<string>();
    var json = 0;
    string? outType = null;
    for (var at = owned ? 1 : 0; at < function.Parameters.Count; at++)
    {
        var parameter = function.Parameters[at];
        var role = call.Roles.GetValueOrDefault(parameter.Name);
        var name = $"a{at}";
        var from = $"&args[{json}]";
        var what = $"\"{function.Name} {parameter.Name}\"";
        if (parameter.Stars == 0 && Tables.Records.ContainsKey(parameter.Type) && role is null)
        {
            setup.Add($"let {name} = {RustSnake(parameter.Type)}_from_json({from})?;");
            json++;
        }
        else if (parameter.Stars == 0 && parameter.Type == "bool")
        {
            setup.Add($"let {name} = flag({from});");
            json++;
        }
        else if (parameter.Stars == 0 && IsByte(parameter.Type) && role is not null)
        {
            setup.Add(role switch
            {
                "number" => $"let {name}: u8 = number({from}, {what});",
                "flag" => $"let {name} = u8::from(flag({from}));",
                _ => $"let {name} = enum_value(\"{role}\", {from})? as u8;",
            });
            json++;
        }
        else if (parameter.Stars == 0 && RustNumber(parameter.Type) is { } number && EnumOf(parameter.Type) is null)
        {
            setup.Add($"let {name}: {number} = number({from}, {what});");
            json++;
        }
        else if (parameter.Stars == 0 && EnumOf(parameter.Type) is { } typed)
        {
            if (cEnums.ContainsKey(parameter.Type) && parameter.Type is not ("MediusKey" or "MediusMediaKey"))
            {
                RustEnumUsed(parameter.Type);
                setup.Add($"let {name} = {RustSnake(parameter.Type)}_from(enum_value(\"{typed}\", {from})?)?;");
            }
            else
            {
                setup.Add($"let {name} = enum_value(\"{typed}\", {from})? as {RustNumber(parameter.Type)};");
            }
            json++;
        }
        else if (parameter.Stars == 1 && parameter.Const && IsByte(parameter.Type) && at + 1 < function.Parameters.Count &&
                 function.Parameters[at + 1] is { Type: "uintptr_t", Stars: 0 })
        {
            setup.Add($"let {name} = unhex({from}, {what});");
            cArguments.Add($"{name}.as_ptr()");
            cArguments.Add($"{name}.len()");
            json++;
            at++;
            continue;
        }
        else if (parameter.Stars == 1 && parameter.Const && parameter.Type == "char")
        {
            setup.Add($"let {name} = c_string({from});");
            cArguments.Add($"{name}.as_ptr()");
            json++;
            continue;
        }
        else if (parameter.Stars == 1 && parameter.Const && Tables.Records.ContainsKey(parameter.Type))
        {
            setup.Add($"let {name} = {RustSnake(parameter.Type)}_from_json({from})?;");
            cArguments.Add($"&{name}");
            json++;
            continue;
        }
        else if (parameter.Stars == 1 && parameter.Const && classes.ContainsKey(parameter.Type))
        {
            cArguments.Add($"{RustHandle(parameter.Type)} as *const _");
            continue;
        }
        else if (parameter.Stars == 1 && !parameter.Const)
        {
            outType = parameter.Type;
            setup.Add(Tables.Records.ContainsKey(parameter.Type)
                ? $"let mut out: {parameter.Type} = unsafe {{ std::mem::zeroed() }};"
                : $"let mut out: {RustNumber(parameter.Type)} = 0;");
            cArguments.Add("&mut out");
            continue;
        }
        else
        {
            problems.Add($"{function.Name}'s {parameter.Name} is a shape the suite's Rust dispatch doesn't know");
            continue;
        }
        cArguments.Add(name);
    }
    var target = owned ? RustHandle(function.Parameters[0].Type) : null;
    var cCall = $"{function.Name}({string.Join(", ", (target is null ? [] : new[] { target }).Concat(cArguments))})";
    var outValue = outType is null ? "Value::Null" : Tables.Records.ContainsKey(outType) ? $"{RustSnake(outType)}_to_json(&out)" : "Value::from(out)";
    var result = function.Return switch
    {
        "MediusStatus" => $"status(unsafe {{ {cCall} }}, || {outValue})",
        "void" => $"{{\n    unsafe {{ {cCall} }};\n    Ok(Value::Null)\n}}",
        "bool" => $"Ok(Value::from(unsafe {{ {cCall} }}))",
        "char" => $"Ok(c_text(unsafe {{ {cCall} }}))",
        _ => $"Ok(Value::from(unsafe {{ {cCall} }}))",
    };
    rustCalls.Add($"        (\"{call.Class}\", \"{call.Method}\") => |{(owned ? "ctx" : "_ctx")}, {(json > 0 ? "args" : "_args")}| {{");
    rustCalls.AddRange(setup.Select(line => "            " + line));
    rustCalls.AddRange(result.Split('\n').Select(line => "            " + line));
    rustCalls.Add("        },");

    // the C++ method with every argument read into the type it takes
    var cppArguments = new List<string>();
    var cppJson = 0;
    foreach (var parameter in call.CppParams)
    {
        var type = Regex.Replace(Regex.Replace(parameter, @"\s+\w+$", ""), @"^const |&$", "");
        if (classes.Values.Any(owner => owner.Class == type))
        {
            cppArguments.Add(CppHandle(type));
        }
        else
        {
            cppArguments.Add($"arg<{type}>(args, {cppJson++})");
        }
    }
    var cppCall = owned
        ? $"{CppHandle(call.Class)}.{call.Method}({string.Join(", ", cppArguments)})"
        : $"anybox::medius::{call.Method}({string.Join(", ", cppArguments)})";
    cppCalls.Add($"    if (cls == \"{call.Class}\" && method == \"{call.Method}\") {{");
    if (call.Returns is "void" or "chained")
    {
        cppCalls.Add($"        {cppCall};");
        cppCalls.Add("        return Value();");
    }
    else
    {
        cppCalls.Add($"        return write({cppCall});");
    }
    cppCalls.Add("    }");
}
suiteRust.Add("// every generated function, which the suite's coverage test makes the scenarios reach");
suiteRust.Add($"pub(super) const CALLS: [(&str, &str); {suiteCalls.Count}] = [");
suiteRust.AddRange(suiteCalls.Select(call => $"    (\"{call.Class}\", \"{call.Method}\"),"));
suiteRust.Add("];");
suiteRust.Add("");
suiteRust.Add("// a generated function by its C++ class and method, None for one that's written by hand");
suiteRust.Add("pub(super) fn call(ctx: &Context, class: &str, method: &str, args: &[Value]) -> Option<Outcome> {");
suiteRust.Add("    let run: fn(&Context, &[Value]) -> Outcome = match (class, method) {");
suiteRust.AddRange(rustCalls);
suiteRust.Add("        _ => return None,");
suiteRust.Add("    };");
suiteRust.Add("    Some(run(ctx, args))");
suiteRust.Add("}");
suiteRust.Add("");

// the enums by their C++ names, the first name for a value with two, and the ones a binding refuses unnamed
suiteRust.Add("pub(super) fn enum_name(name: &str, value: i64) -> Value {");
suiteRust.Add("    let named = match (name, value) {");
foreach (var (enumName, _, members, _, _) in suiteEnums)
{
    foreach (var (member, value) in members.DistinctBy(member => member.Value))
    {
        suiteRust.Add($"        (\"{enumName}\", {value}) => \"{member}\",");
    }
}
suiteRust.Add("        _ => return Value::from(value),");
suiteRust.Add("    };");
suiteRust.Add("    Value::from(named)");
suiteRust.Add("}");
suiteRust.Add("");
suiteRust.Add("pub(super) fn enum_number(name: &str, member: &str) -> i64 {");
suiteRust.Add("    match (name, member) {");
foreach (var (enumName, _, members, enumAliases, _) in suiteEnums)
{
    foreach (var (member, value) in members)
    {
        suiteRust.Add($"        (\"{enumName}\", \"{member}\") => {value},");
    }
    foreach (var (alias, target) in enumAliases)
    {
        suiteRust.Add($"        (\"{enumName}\", \"{alias}\") => {members.First(member => member.Member == target).Value},");
    }
}
suiteRust.Add("        _ => panic!(\"{name} has no member {member}\"),");
suiteRust.Add("    }");
suiteRust.Add("}");
suiteRust.Add("");
suiteRust.Add("// whether a binding refuses a value of this enum it has no name for, before medius sees it");
suiteRust.Add("pub(super) fn checked(name: &str) -> bool {");
suiteRust.Add($"    matches!(name, {string.Join(" | ", suiteEnums.Where(entry => entry.CheckIn).Select(entry => $"\"{entry.Cpp}\""))})");
suiteRust.Add("}");
suiteRust.Add("");
foreach (var c in rustEnums)
{
    var (prefix, _, members) = cEnums[c];
    suiteRust.Add($"pub(super) fn {RustSnake(c)}_from(value: i64) -> Result<{c}, Failure> {{");
    suiteRust.Add("    Ok(match value {");
    foreach (var (constant, value) in members)
    {
        suiteRust.Add($"        {ParseNumber(value)} => {c}::{Pascal(constant[prefix.Length..])},");
    }
    suiteRust.Add($"        _ => return Err(Failure::Refused(format!(\"{c} has no {{value}}\"))),");
    suiteRust.Add("    })");
    suiteRust.Add("}");
    suiteRust.Add("");
}

// C++: the records by their fields, the enums by their names, and the generated functions
var suiteCpp = new List<string>
{
    "// made by bindings/csharp/tools/gen-medius.cs from bindings/c/include/medius.h and its tables, don't edit it by hand.",
    "// it goes in the Medius suite's runner after the overloads it builds on",
    "",
};
foreach (var (enumName, c, members, enumAliases, _) in suiteEnums)
{
    suiteCpp.Add($"inline Value write({enumName} value) {{");
    suiteCpp.Add($"    switch (static_cast<std::int64_t>(value)) {{");
    foreach (var (member, value) in members.DistinctBy(member => member.Value))
    {
        suiteCpp.Add($"    case {value}:");
        suiteCpp.Add($"        return Value(\"{member}\");");
    }
    suiteCpp.Add("    default:");
    suiteCpp.Add("        return Value(static_cast<std::int64_t>(value));");
    suiteCpp.Add("    }");
    suiteCpp.Add("}");
    suiteCpp.Add("");
    suiteCpp.Add($"inline void read(const Value& value, {enumName}& out) {{");
    suiteCpp.Add("    if (!value.is_string()) {");
    suiteCpp.Add($"        out = static_cast<{enumName}>(value.as_int());");
    suiteCpp.Add("        return;");
    suiteCpp.Add("    }");
    suiteCpp.Add("    const auto& name = value.as_string();");
    foreach (var (member, _) in members)
    {
        suiteCpp.Add($"    if (name == \"{member}\") {{");
        suiteCpp.Add($"        out = {enumName}::{member};");
        suiteCpp.Add("        return;");
        suiteCpp.Add("    }");
    }
    foreach (var (alias, _) in enumAliases)
    {
        suiteCpp.Add($"    if (name == \"{alias}\") {{");
        suiteCpp.Add($"        out = {enumName}::{alias};");
        suiteCpp.Add("        return;");
        suiteCpp.Add("    }");
    }
    suiteCpp.Add($"    throw std::runtime_error(\"{enumName} has no member \" + name);");
    suiteCpp.Add("}");
    suiteCpp.Add("");
}
foreach (var enumName in suiteNumbers.Order())
{
    suiteCpp.Add($"inline Value write({enumName} value) {{ return Value(static_cast<std::int64_t>(value.value)); }}");
    suiteCpp.Add($"inline void read(const Value& value, {enumName}& out) {{ out.value = static_cast<decltype(out.value)>(value.as_int()); }}");
    suiteCpp.Add("");
}
// declared first, a record can hold one that comes later
foreach (var (cname, _) in suiteRecords)
{
    suiteCpp.Add($"Value write(const {RecordName(cname)}& value);");
    suiteCpp.Add($"void read(const Value& value, {RecordName(cname)}& out);");
}
suiteCpp.Add("");
foreach (var (cname, fields) in suiteRecords)
{
    suiteCpp.Add($"inline Value write(const {RecordName(cname)}& value) {{");
    suiteCpp.Add("    Object out;");
    suiteCpp.AddRange(fields.Select(field => $"    out.push_back({{\"{field.Field}\", write(value.{field.Field})}});"));
    suiteCpp.Add("    return Value(std::move(out));");
    suiteCpp.Add("}");
    suiteCpp.Add("");
    suiteCpp.Add($"inline void read(const Value& value, {RecordName(cname)}& out) {{");
    suiteCpp.AddRange(fields.Select(field => $"    read(value[\"{field.Field}\"], out.{field.Field});"));
    suiteCpp.Add("}");
    suiteCpp.Add("");
}
suiteCpp.Add("// a generated function by its class and method, an empty optional for one that's written by hand");
suiteCpp.Add("inline std::optional<Value> call(Context& ctx, const std::string& cls, const std::string& method, const Array& args) {");
suiteCpp.AddRange(cppCalls);
suiteCpp.Add("    return std::nullopt;");
suiteCpp.Add("}");

// C#: the enums by their names, and each record's medius.h field names with its C# ones, read by reflection
var suiteCs = new List<string>
{
    "// made by bindings/csharp/tools/gen-medius.cs from bindings/c/include/medius.h and its tables, don't edit it by hand",
    "using Anybox.Medius;",
    "",
    "namespace MediusTests;",
    "",
    "static class SuiteTables",
    "{",
    "    // each enum's members by value, the first name for a value with two, and the other names that read as one",
    "    public static readonly Dictionary<Type, (Dictionary<long, string> Names, Dictionary<string, long> Values)> Enums = new()",
    "    {",
};
foreach (var (enumName, _, members, enumAliases, _) in suiteEnums)
{
    var csEnum = EnumPolicy(enumName).Cs;
    var names = string.Join(", ", members.DistinctBy(member => member.Value).Select(member => $"[{member.Value}] = \"{member.Member}\""));
    var values = string.Join(", ", members.Select(member => $"[\"{member.Member}\"] = {member.Value}")
        .Concat(enumAliases.Select(alias => $"[\"{alias.Key}\"] = {members.First(member => member.Member == alias.Value).Value}")));
    suiteCs.Add($"        [typeof({csEnum})] = (new() {{ {names} }}, new() {{ {values} }}),");
}
suiteCs.Add("    };");
suiteCs.Add("");
suiteCs.Add("    // each record's fields, medius.h's name and the C# property");
suiteCs.Add("    public static readonly Dictionary<Type, (string Json, string Cs)[]> Records = new()");
suiteCs.Add("    {");
foreach (var (cname, fields) in suiteRecords)
{
    suiteCs.Add($"        [typeof({CsRecordName(cname)})] = [{string.Join(", ", fields.Select(field => $"(\"{field.Field}\", \"{field.Cs}\")"))}],");
}
suiteCs.Add("    };");
suiteCs.Add("}");
if (problems.Count > 0)
{
    foreach (var problem in problems)
    {
        Console.Error.WriteLine($"gen-medius: {problem}");
    }
    Environment.Exit(1);
}
if (!dry)
{
    Directory.CreateDirectory(Path.Combine(root, "bindings/csharp/src/Anybox/Medius"));
    File.WriteAllText(Path.Combine(root, "bindings/csharp/src/Anybox/Medius/Generated.cs"), cs.ToString().ReplaceLineEndings("\n"));
    File.WriteAllText(Path.Combine(root, "bindings/csharp/src/Anybox/Medius/Records.cs"), string.Join("\n", [
        "// made by bindings/csharp/tools/gen-medius.cs from bindings/c/include/medius.h and its record table, don't edit it",
        "// by hand. the helpers of each record are in Types.cs, the other half of its partial record",
        "namespace Anybox.Medius;",
        .. csRecordLines,
        "",
    ]));
    File.WriteAllText(Path.Combine(root, "bindings/c/tests/medius_layout.rs"), rust.ToString().ReplaceLineEndings("\n"));
    File.WriteAllText(cppPath, cpp);
    File.WriteAllText(Path.Combine(root, "bindings/c/src/medius_suite/generated.rs"), string.Join("\n", suiteRust) + "\n");
    File.WriteAllText(Path.Combine(root, "bindings/c/tests/cpp/medius_suite.hpp"), string.Join("\n", suiteCpp) + "\n");
    File.WriteAllText(Path.Combine(root, "bindings/csharp/tests/Anybox.Tests/Medius/SuiteTables.cs"), string.Join("\n", suiteCs) + "\n");
    // the Rust test goes through rustfmt like the rest of the crate, so check.sh's fmt check and diff both stay quiet
    using var fmt = System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo("cargo", "fmt -p anybox-capi") { WorkingDirectory = root })!;
    fmt.WaitForExit();
    if (fmt.ExitCode != 0)
    {
        throw new InvalidOperationException("cargo fmt failed on the generated Rust test");
    }
}
Console.WriteLine($"gen-medius: {constants.Count} constants, {enums.Count} enums, {records.Count} structs, {opaque.Count} handles, {functions.Count} functions");

record CParam(string Type, int Stars, bool Const, string Name, bool Callback);

// one field of a record in the suite's JSON: its medius.h name, how it's written (text, opttext, array, bytes, match, list,
// record, optrecord, enum, flag, number, optional), its C# name, and what that kind needs
record JsonField(string Field, string Kind, string Cs, string? Count = null, string? Element = null, string? Enum = null, string? CType = null, string? Sentinel = null);

// one made function the suite dispatches: its C++ class and method, the C function, its roles and its C++ parameters
record SuiteCall(string Class, string Method, CFunction Function, Dictionary<string, string> Roles, List<string> CppParams, string Returns);

record CFunction(string Name, string Return, int ReturnStars, List<CParam> Parameters, string Signature, bool Mock);

// form is class (an enum class), struct (Rust's newtype with a constant per name) or status. read is what a byte
// coming back from the library that no member names does: Strict throws, Unknown reads as the Unknown member and Any
// keeps it. check_in false lets any value go in, for the enums medius carries through as they are
// default is the member Rust's Default gives, for the records that have one
record EnumUse(string Name, string Doc, string Form = "class", string Read = "Strict", bool CheckIn = true, string? Pinned = null)
{
    public Dictionary<string, string> Aliases { get; init; } = [];
    public string? Default { get; init; }
}

// an enum the C++ side has that medius.h only gives as #defines or comments
record ExtraEnum(string Name, string Doc, (string Member, string Value)[] Members, string? FromDefines = null, int From = 0, string Read = "Strict", bool CheckIn = true)
{
    public string[] Skip { get; init; } = [];
    public Dictionary<string, string> Renames { get; init; } = [];
    public string? Default { get; init; }
}

abstract record RecordUse(string Name);

// a field's role: number, flag, an enum's C++ name, array, bytes(count), match(count), list(count), optional(none)
// or if(flag). a field of a type that says it all (bool, a C enum, a struct, a char array) needs none.
// Default says Rust's record has a Default, so both bindings get the same one: every field Rust's zero or its enum's
// default, or what Defaults gives it (a constant's name). Order is Rust's field order where it isn't medius.h's.
// CsName, CsNames and CsDoc are the C# names and doc where the C++ ones don't fit C#
record MadeRecord(string Name, string Doc, Dictionary<string, string> Roles, Helper[] Members) : RecordUse(Name)
{
    public bool Default { get; init; }
    public Dictionary<string, string> Defaults { get; init; } = [];
    public string[]? Order { get; init; }
    public string? CsName { get; init; }
    public Dictionary<string, string> CsNames { get; init; } = [];
    public string? CsDoc { get; init; }
}

record Helper(string Declaration, string Doc = "");

record HandRecord(string Name, string Fields) : RecordUse(Name);

abstract record FunctionUse;

// tokens: param:role, param:role=default, param=default and as:name. a byte needs a role (number, flag or the enum
// it takes), the rest of a parameter's shape comes from its C type
record MadeFunction(string Doc, params string[] Tokens) : FunctionUse;

record HandFunction(string Where, string Signature, bool Mock = false) : FunctionUse;

// what the C++ side does with each thing in medius.h. a doc is what the C++ comment above it says
static class Tables
{
    public static readonly Dictionary<string, EnumUse> Enums = new()
    {
        ["MediusStatus"] = new("Status", "the status a medius.h call gave, what `Error::medius_status()` says", Form: "status"),
        ["MediusEmitMode"] = new("EmitMode", "what paces injected motion",
            Pinned: "MEDIUS_EMIT_MODE_LEARNED=0 MEDIUS_EMIT_MODE_INTERVAL=1 MEDIUS_EMIT_MODE_FIXED=2"),
        ["MediusBearingMode"] = new("BearingMode", "how the box tells whether physical motion runs with or against injected motion") { Default = "PerAxis" },
        ["MediusRenderMode"] = new("RenderMode", "the render texture. `Off` is the paced fill, the others render the device's own report texture and only differ in the path smoother") { Default = "Despiked" },
        ["MediusControlStatus"] = new("ControlStatus", "the handshake the game PC got for a control transaction, `Other` for one this build of medius doesn't know"),
        ["MediusBusEventKind"] = new("BusEventKind", "what a bus event describes"),
        ["MediusCatchEventKind"] = new("CatchEventKind", "which kind a catch event is",
            Pinned: "MEDIUS_CATCH_EVENT_KIND_MOTION=0 MEDIUS_CATCH_EVENT_KIND_USAGES=1 MEDIUS_CATCH_EVENT_KIND_TRAFFIC=2"),
        ["MediusClockDomain"] = new("ClockDomain", "which chip's clock stamped an event. the chips boot on their own and their timers have nothing to do with each other, so only compare stamps from the same one"),
        ["MediusInputKind"] = new("InputKind", "which kind an input event is",
            Pinned: "MEDIUS_INPUT_KIND_PRESS=0 MEDIUS_INPUT_KIND_RELEASE=1 MEDIUS_INPUT_KIND_MOTION=2"),
        ["MediusLogLevel"] = new("LogLevel", "how bad a log line from the box is"),
        ["MediusFrameType"] = new("FrameType", "the type byte of a frame, for what the mock got. one this header has no name for comes through as its number", Read: "Any"),
        ["MediusAction"] = new("Action", "what an injected edge does. `SoftRelease` takes your press back and leaves a physical hold alone, `ForceRelease` masks a physical hold too"),
        ["MediusAxis"] = new("Axis", "a relative axis"),
        ["MediusBlanket"] = new("Blanket", "a whole group of inputs, for a blanket lock or a clip's autolock",
            Pinned: "MEDIUS_BLANKET_AIM=0 MEDIUS_BLANKET_WHEEL=1 MEDIUS_BLANKET_BUTTONS=2 MEDIUS_BLANKET_KEYS=3 MEDIUS_BLANKET_MEDIA=4"),
        ["MediusButton"] = new("Button", "a mouse button by the box's id. any id up to the clone's button count is one, these are the five usual ones", Form: "struct"),
        ["MediusClass"] = new("Class", "what a usage is, a button, a key or a media key"),
        ["MediusClipAction"] = new("ClipAction", "what a clip trigger does to the clip"),
        ["MediusClipState"] = new("ClipState", "where the clip is on the box") { Default = "Idle" },
        ["MediusDeviceKind"] = new("DeviceKind", "what the cloned device mostly is, from its HID descriptors. one this header has no name for reads as `Unknown`", Read: "Unknown") { Default = "Unknown" },
        ["MediusDirection"] = new("Direction", "which way a lock, a scale or a filter goes. on a button or a key it's the press or the release, on traffic it's in (to the PC) or out, and `With` and `Against` are measured against the bearing instead of a fixed sign")
        {
            Aliases = new() { ["Press"] = "Positive", ["Release"] = "Negative", ["In"] = "Positive", ["Out"] = "Negative" },
            Default = "Both",
        },
        ["MediusEdge"] = new("Edge", "which edge of a usage fires a clip trigger"),
        ["MediusLedMode"] = new("LedMode", "what a status LED does"),
        ["MediusLedTarget"] = new("LedTarget", "which status LED a command is for"),
        ["MediusLockTargetKind"] = new("LockTargetKind", "what a lock target is, one of the four axes or a usage",
            Pinned: "MEDIUS_LOCK_TARGET_KIND_X=0 MEDIUS_LOCK_TARGET_KIND_Y=1 MEDIUS_LOCK_TARGET_KIND_WHEEL=2 MEDIUS_LOCK_TARGET_KIND_PAN=3 MEDIUS_LOCK_TARGET_KIND_USAGE=4"),
        ["MediusMotionKind"] = new("MotionKind", "which axis a motion drives",
            Pinned: "MEDIUS_MOTION_KIND_CURSOR=0 MEDIUS_MOTION_KIND_WHEEL=1 MEDIUS_MOTION_KIND_PAN=2"),
        ["MediusMoveTiming"] = new("MoveTiming", "whether a move rides a physical report or goes out at once"),
        ["MediusPatchSection"] = new("PatchSection", "which descriptor a patch overwrites") { Default = "Device" },
        ["MediusPendingMotion"] = new("PendingMotion", "what a move does to the motion already held for a ride"),
        ["MediusRebootTarget"] = new("RebootTarget", "which chip reboots, and into what"),
        ["MediusRewriteAction"] = new("RewriteAction", "what the most specific matching rewrite rule does to the packet") { Default = "Pass" },
        ["MediusRewriteClass"] = new("RewriteClass", "the traffic a rewrite rule works on, `Any` is every surface a packet crosses") { Default = "Emit" },
        ["MediusTransferStatus"] = new("TransferStatus", "how a control transfer ended. one this header has no name for comes through as its number, both ways", Read: "Any", CheckIn: false),
        ["MediusTransformOp"] = new("TransformOp", "what a transform does. `Remap` moves a field's value into another one and clears it, `Swap` exchanges two axes at once"),
        ["MediusKey"] = new("Key", "a keyboard key by its HID usage, `Key{0x64}` gets you any other one. just know that windows.h has a DELETE macro, so `Key::DELETE` needs an `#undef DELETE` after it", Form: "struct"),
        ["MediusMediaKey"] = new("MediaKey", "a media key by its HID consumer usage, `MediaKey{n}` gets you any other one", Form: "struct"),
    };

    public static readonly ExtraEnum[] ExtraEnums =
    [
        new("CatchClass", "what a catch filter watches. the first four are the parsed input, the rest are byte oriented traffic", [], FromDefines: "MEDIUS_CATCH_CLASS_")
        {
            Skip = ["ANY"],
            Renames = new() { ["BTN"] = "Button" },
        },
        new("TrafficClass", "the byte oriented catch classes", [], FromDefines: "MEDIUS_CATCH_CLASS_", From: 4)
        {
            Skip = ["ANY"],
        },
        new("ImageState", "the bootloader's state for the image a chip runs. one this header has no name for comes through as its number, like Rust's `Unknown(n)`",
            [("New", "0"), ("PendingVerify", "1"), ("Valid", "2"), ("Invalid", "3"), ("Aborted", "4"), ("Unknown", "0xFF")], Read: "Any"),
        new("UpdateTarget", "which chip a firmware update goes to", [("Device", "0"), ("Host", "1")]),
    ];

    public static readonly Dictionary<string, RecordUse> Records = new()
    {
        ["MediusUsage"] = new HandRecord("Usage", "uint8_t kind, uint16_t id"),
        ["MediusSetup"] = new MadeRecord("Setup", "a USB setup packet. `length` is the data stage, the bytes to read for IN, otherwise the length of the OUT data",
            new() { ["request_type"] = "number", ["request"] = "number" },
            [
                new("bool is_in() const", "a device to host request, by the direction bit of the request type"),
                new("std::array<std::uint8_t, 8> to_bytes() const", "the eight bytes on the wire, the numbers little endian"),
            ]) { Default = true },
        ["MediusClipTrigger"] = new MadeRecord("ClipTrigger", "one clip input trigger. `on`'s `edge` drives `action`, and `consume` keeps the input from the game",
            new() { ["edge"] = "Edge", ["action"] = "ClipAction", ["consume"] = "flag" }, []),
        ["MediusClipPacketTrigger"] = new MadeRecord("ClipPacketTrigger",
            "one clip packet trigger, keyed by its class, id, direction, match and mask. a packet whose head matches under the mask drives `action` on the box's next tick with no round trip to the PC, see `ClipPacketTrigger` in the medius docs for which ones the box refuses and how they rank",
            new()
            {
                ["class_"] = "TrafficClass", ["direction"] = "Direction", ["action"] = "ClipAction", ["consume"] = "flag", ["once_per_run"] = "flag",
                ["selector_len"] = "number", ["match_bytes"] = "match(match_len)", ["mask"] = "match(mask_len)",
            }, [])
        {
            CsNames = new() { ["class_"] = "TrafficClass" },
            Order = ["class_", "id", "direction", "action", "match_bytes", "mask", "consume", "once_per_run", "selector_len", "hits"],
        },
        ["MediusClipStatus"] = new MadeRecord("ClipStatus", "the clip's ring and playback counters on the box",
            new() { ["state"] = "ClipState", ["held"] = "list(held_n)" },
            [new("bool is_held(const Usage& usage) const", "whether the clip holds that usage down")]) { Default = true },
        ["MediusClipSettings"] = new HandRecord("ClipSettings",
            "uint8_t autolock_bits, uint8_t loop_, uint8_t retain, uint8_t finalized, uint8_t ride, MediusClipTrigger triggers[MEDIUS_CLIP_TRIG_MAX], uint8_t n, MediusClipPacketTrigger packet_triggers[MEDIUS_CLIP_PKT_TRIG_MAX], uint8_t packet_n"),
        ["MediusPortInfo"] = new MadeRecord("PortInfo", "a serial port that could be a box", new() { ["serial"] = "if(has_serial)" }, []),
        ["MediusVersion"] = new MadeRecord("Version", "what the box says it is",
            new() { ["proto_ver"] = "number", ["fw_major"] = "number", ["fw_minor"] = "number", ["fw_patch"] = "number", ["mac"] = "array" },
            [new("std::string mac_hex() const", "the base MAC as 12 lowercase hex digits, the box's id")])
        {
            CsName = "BoxVersion",
            CsDoc = "what the box says it is. it's `Version` in the Python package, the name would clash with `System.Version` here",
        },
        ["MediusDeviceInfo"] = new MadeRecord("DeviceInfo", "the cloned device's USB ids, what kind it mostly is and its product string",
            new() { ["has_serial"] = "flag", ["has_bos"] = "flag", ["kind"] = "DeviceKind" },
            [new("bool is_cloned() const", "whether a device is cloned, one with no HID interface too")]) { Default = true },
        ["MediusBoxInfo"] = new MadeRecord("BoxInfo", "a box a listing found. `device` is empty for a box on another control protocol, opening that one throws `BadProtoVerError`",
            new() { ["device"] = "if(has_device)" },
            [
                new("std::string id() const", "the box's stable id, its device MAC as hex"),
                new("const std::string& name() const", "the box's name, a made up default when nobody set one"),
                new("const std::optional<std::string>& serial() const", "the serial of the box's control adapter, when it has one"),
            ])
        {
            CsDoc = "a box a listing found. `device` is null for a box on another control protocol, opening it throws <see cref=\"BadProtoVerException\"/>",
        },
        ["MediusMotion"] = new HandRecord("Motion", "uint8_t kind, int16_t dx, int16_t dy, int16_t wheel, int16_t pan"),
        ["MediusLockTarget"] = new HandRecord("LockTarget", "uint8_t kind, MediusUsage usage"),
        ["MediusTransferOutcome"] = new MadeRecord("TransferOutcome",
            "the real device's reply to a control transfer. a status that isn't ok is the protocol's answer and not a link error, and it carries no data",
            new() { ["status"] = "TransferStatus", ["data"] = "bytes(len)" },
            [new("bool is_ok() const"), new("std::optional<std::vector<std::uint8_t>> ok_data() const", "the data when the transfer went through, empty when it didn't")]),
        ["MediusRewriteRule"] = new MadeRecord("RewriteRule",
            "a rewrite rule, keyed by its class, id, direction, match and mask. the match and mask compare the masked head and are as long as each other, an empty match takes every packet on the address. `payload` is for the actions that carry one, and `offset` is where a patch writes",
            new()
            {
                ["class_"] = "RewriteClass", ["direction"] = "Direction", ["action"] = "RewriteAction", ["match_bytes"] = "match(match_len)",
                ["mask"] = "match(mask_len)", ["payload"] = "bytes(payload_len)",
            }, [new("RewriteKey key() const")])
        {
            Default = true,
            CsNames = new() { ["class_"] = "RewriteClass" },
        },
        ["MediusRewriteEntry"] = new MadeRecord("RewriteEntry", "one row of the rewrite table, a rule's address, action and live counters without its bytes",
            new() { ["class_"] = "RewriteClass", ["direction"] = "Direction", ["action"] = "RewriteAction", ["match_len"] = "number" }, [])
            { CsNames = new() { ["class_"] = "RewriteClass" } },
        ["MediusRewriteTable"] = new MadeRecord("RewriteTable",
            "the rewrite table in the order it was installed. `generation` moves on every change, `table_full` says the box refused the last new rule for room",
            new() { ["table_full"] = "flag", ["generation"] = "number", ["entries"] = "list(n)" }, []) { Default = true },
        ["MediusPatch"] = new MadeRecord("Patch",
            "a descriptor patch, keyed by its section, configuration, index and offset. `bytes` overwrites the descriptor from `offset`, empty removes the patch at that key. `cfg` is the configuration's index, 0 is the first one and not bConfigurationValue",
            new() { ["section"] = "PatchSection", ["cfg"] = "number", ["index"] = "number", ["bytes"] = "bytes(len)" },
            [
                new("static Patch in_config(std::uint8_t cfg, std::uint16_t offset, std::vector<std::uint8_t> bytes)", "a patch in configuration `cfg`, 0 is the first one"),
                new("static Patch in_interface(std::uint8_t cfg, std::uint8_t interface_, std::uint16_t offset, std::vector<std::uint8_t> bytes)",
                    "a report descriptor patch on `interface_` in configuration `cfg`, 0 is the first one"),
                new("static Patch in_string(std::uint8_t index, std::vector<std::uint8_t> bytes)", "a patch that replaces all of string `index`"),
                new("PatchKey key() const"),
            ]) { Default = true },
        ["MediusPatchEntry"] = new MadeRecord("PatchEntry", "one stored patch's key and length, without its bytes",
            new() { ["section"] = "PatchSection", ["cfg"] = "number", ["index"] = "number" }, []),
        ["MediusPatchSet"] = new MadeRecord("PatchSet",
            "the stored patch set and how it's applied. `applied` is the clone serving a patched set, `pending` the stored set being different from the served one, `refused` the stored set failing a check the plain descriptors pass so the device is served unpatched, and `table_full` the box refusing the last new patch for room",
            new() { ["applied"] = "flag", ["pending"] = "flag", ["refused"] = "flag", ["table_full"] = "flag", ["entries"] = "list(n)" }, []) { Default = true },
        ["MediusTransform"] = new MadeRecord("Transform",
            "one field transform, keyed by its source and destination. it swaps or remaps a field the clone already has, so every report is one the real device could send and it needs no imperfect clone opt in. to weigh or reverse a field use `Device::scale`, which runs first",
            new() { ["op"] = "TransformOp" },
            [
                new("TransformKey key() const"),
                new("static Transform swap(Axis a, Axis b)", "exchanges two axes at once"),
                new("static Transform remap(const LockTarget& source, const LockTarget& dest)", "moves the source's value into the destination and clears the source"),
            ]),
        ["MediusTransforms"] = new MadeRecord("Transforms", "the transform table in the order it was installed, each entry replays as a `Device::transform`",
            new() { ["table_full"] = "flag", ["entries"] = "list(n)" }, []) { Default = true },
        ["MediusChipFirmware"] = new MadeRecord("ChipFirmware", "one chip's firmware version and the slot it booted",
            new() { ["major"] = "number", ["minor"] = "number", ["patch"] = "number", ["slot"] = "number", ["state"] = "ImageState" }, []),
        ["MediusFirmwareInfo"] = new MadeRecord("FirmwareInfo", "both chips' firmware. `host` is empty while the host chip hasn't answered over the link between the chips",
            new() { ["host"] = "if(host_present)", ["device_staged"] = "flag", ["host_staged"] = "flag" },
            [new("bool any_pending() const", "while either chip hasn't confirmed the image it booted, which is when an update gets refused")])
        {
            CsDoc = "both chips' firmware. `host` is null while the host chip hasn't answered over the link between the chips",
        },
        ["MediusHealth"] = new MadeRecord("Health", "the box's health flags",
            new()
            {
                ["link_up"] = "flag", ["mouse_attached"] = "flag", ["clone_configured"] = "flag", ["injection_active"] = "flag", ["rate_confident"] = "flag",
                ["lock_on"] = "flag", ["catch_on"] = "flag", ["kbd_attached"] = "flag", ["rewrite_on"] = "flag", ["patch_on"] = "flag", ["transform_on"] = "flag",
            }, []),
        ["MediusMouseCaps"] = new MadeRecord("MouseCaps", "what the cloned mouse has",
            new()
            {
                ["n_buttons"] = "number", ["has_x"] = "flag", ["has_y"] = "flag", ["has_wheel"] = "flag", ["pan"] = "flag", ["has_report_id"] = "flag",
                ["n_hid"] = "number",
            }, [new("bool is_composite() const", "whether the mouse has more than one HID interface")]) { Default = true },
        ["MediusKbdCaps"] = new MadeRecord("KbdCaps", "what the cloned keyboard has, `n_keys` 0xFF is an NKRO bitmap",
            new() { ["n_keys"] = "number", ["nkro"] = "flag", ["has_consumer"] = "flag", ["has_system"] = "flag", ["has_report_id"] = "flag" }, [])
        { Default = true },
        ["MediusCaps"] = new MadeRecord("Caps", "what the cloned device has",
            new() { ["mouse_change_driven"] = "flag", ["kbd_change_driven"] = "flag" },
            [
                new("bool has_mouse() const", "whether a mouse interface is bound"),
                new("bool has_keyboard() const", "whether a keyboard interface is bound"),
                new("bool is_composite() const", "whether the clone has more than one HID interface"),
            ]) { Default = true },
        ["MediusRate"] = new MadeRecord("Rate", "the device's report rate the box measured, and the clone's poll period",
            new() { ["confident"] = "flag", ["change_driven"] = "flag" },
            [new("std::optional<float> native_hz() const", "the device's own rate, empty while the box doesn't know it")]),
        ["MediusStats"] = new MadeRecord("Stats",
            "the box's delivery counters. the narrow ones saturate instead of wrapping, the three drop counters don't. `tx_drops`, `link_rx_drops` and `host_rx_drops` are lost physical input and should stay 0, `relay_drops` is relayed traffic and commands, never physical input. `session` counts the times the box let go of what the PC set and wraps, so compare it for inequality",
            new() { ["tx_maxdepth"] = "number", ["tx_wedges"] = "number" }, []),
        ["MediusLockEntry"] = new MadeRecord("LockEntry",
            "one weighed direction. `scale` is the percent of the physical value kept, 0 blocks, 100 passes, above 100 amplifies and a negative one reverses what it keeps. it's the scale the box applies, which in `BearingMode::Vector` can differ from the one sent",
            new() { ["direction"] = "Direction" }, [new("bool is_block() const")]),
        ["MediusLocks"] = new MadeRecord("Locks", "the locks and scales the box holds", new() { ["entries"] = "list(n)" },
            [
                new("bool is_locked(const LockTarget& target, Direction direction) const",
                    "whether the target is blocked outright that way, a weighed direction isn't locked. `Both` asks about the two fixed signs, ask for a relative one by name"),
                new("std::int16_t scale_of(const LockTarget& target, Direction direction) const",
                    "the scale on that target and direction, of the entries covering it the one of smallest magnitude (a negative one on a tie), or 100 when none does. a delta meets the product of its fixed and relative scales, which this doesn't work out"),
            ]),
        ["MediusClockEstimate"] = new MadeRecord("ClockEstimate",
            "the measured offset between the two chips' clocks. `rate_ppb` is empty when the box fitted no rate, `age_ms` empty when it never measured, which a zero offset looks like too",
            new() { ["rate_ppb"] = "optional(MEDIUS_CLOCK_RATE_NONE)", ["age_ms"] = "optional(MEDIUS_CLOCK_AGE_NONE)" },
            [
                new("std::int32_t error_bound_us() const", "half the measured round trip, how wrong `offset_us` can be at most"),
                new("std::optional<std::int64_t> to_host_domain(std::int64_t device_us) const", "a device chip stamp on the host chip's timeline, empty without an estimate"),
                new("std::int64_t drift_us_over(std::chrono::microseconds elapsed) const",
                    "how far the offset drifts over `elapsed`, in microseconds. add `error_bound_us()` for the whole bound on a stamp moved that long after the estimate. it's 0 when the box fitted no rate, though the drift is unknown then and not zero"),
            ])
        {
            CsDoc = "the measured offset between the two chips' clocks. `rate_ppb` is null when the box fitted no rate, `age_ms` null when it never measured, which a zero offset looks like too",
        },
        ["MediusCatchFilter"] = new HandRecord("CatchFilter", "MediusCatchClass class_, uint16_t id, uint8_t direction, uint8_t capture"),
        ["MediusCatchEntry"] = new MadeRecord("CatchEntry", "one row of the box's subscription table, a live subscription and its drop count", new(), []),
        ["MediusCatchState"] = new MadeRecord("CatchState", "the live subscription table, its drop counts and the clock estimate between the chips",
            new() { ["table_full"] = "flag", ["entries"] = "list(n)" }, [new("bool is_empty() const", "whether nothing is subscribed")]),
        ["MediusImperfectStatus"] = new MadeRecord("ImperfectStatus", "the imperfect clone opt in, and whether the clone needs it",
            new() { ["allowed"] = "flag", ["over_capacity"] = "flag", ["clone_imperfect"] = "flag" }, []) { Default = true },
        ["MediusEmitPaceStatus"] = new HandRecord("EmitPaceStatus",
            "MediusEmitMode mode, uint16_t fixed_hz, uint16_t resolved_hz, uint16_t force_hz, uint16_t advertised_hz, uint8_t force_active"),
        ["MediusBearing"] = new MadeRecord("Bearing",
            "what `Direction::With` and `Direction::Against` are measured against. `window_ms` is how long the last injected direction stays the bearing, empty is off and leaves the relative directions doing nothing",
            new() { ["window_ms"] = "optional(0)" }, [new("bool is_live() const")])
        {
            Default = true,
            Defaults = new() { ["window_ms"] = "BearingWindowDefaultMs" },
            CsDoc = "what `Direction::With` and `Direction::Against` are measured against. `window_ms` is how long the last injected direction stays the bearing, null turns it off and leaves the relative directions doing nothing",
        },
        ["MediusRenderStatus"] = new MadeRecord("RenderStatus",
            "the render mode, whether physical motion goes through it, and whether the box learned the attached device's profile yet",
            new() { ["full"] = "flag", ["ready"] = "flag" }, []) { Default = true },
        ["MediusSpreadStatus"] = new MadeRecord("SpreadStatus", "the injection spread, a percent of the host's command interval, and the span the box releases across", new(), [])
        { Default = true },
        ["MediusCountersSnapshot"] = new MadeRecord("CountersSnapshot",
            "what the library counted on this side. `restarts` is the device chip restarts it got over by sending its held state again", new(), [])
        {
            Default = true,
            CsName = "Counters",
        },
        ["MediusUpdateError"] = new HandRecord("UpdateError", "uint8_t op, uint8_t status, uint32_t arg"),
        ["MediusUsageEvent"] = new MadeRecord("UsageSnapshot",
            "every usage of one class held right now. the class and direction come from the frame, so the empty snapshot of the last release still has them",
            new() { ["class_"] = "Class", ["direction"] = "Direction", ["usages"] = "list(n)" },
            [new("bool is_held(const Usage& usage) const")]) { CsNames = new() { ["class_"] = "Class" } },
        ["MediusTrafficEvent"] = new MadeRecord("TrafficEvent",
            "one byte oriented catch event, HID reports, vendor endpoints, control transactions, what the clone sent, bus lifecycle, or a clip's control transfers. `bytes` is what the capture kept and `true_len` the length before it was cut, so set both when you build one by hand",
            new() { ["class_"] = "CatchClass", ["direction"] = "Direction", ["flags"] = "number", ["bytes"] = "bytes(len)" },
            [
                new("bool truncated() const", "whether the capture cut the packet short, which `bytes` alone can't tell from a short packet"),
                new("std::optional<std::array<std::uint8_t, 8>> setup_bytes() const", "the 8 setup bytes of a control or clip transfer event, empty for another class or a shorter capture"),
                new("std::vector<std::uint8_t> data() const", "the data stage of a control or clip transfer event, the whole packet for any other class"),
                new("std::optional<ControlStatus> control_status() const", "the handshake the game PC got, empty for any class but control"),
                new("std::optional<TransferStatus> transfer_status() const",
                    "how the transfer ended, empty for any class but clip transfer. `Nak` when no reply came, and a status this header has no name for stays its number"),
                new("std::optional<BusEvent> bus_event() const", "the lifecycle event, empty for any class but bus or a kind the library doesn't know"),
                new("bool rule_acted() const",
                    "whether a rewrite rule changed, dropped, answered or refused the packet. only HID in and out, the vendor classes, control and emit carry it"),
                new("bool bulk_end_of_transfer() const"),
                new("bool zlp() const", "whether it's a zero length packet, or on a HID endpoint a HID in or emit report one ended"),
            ]) { CsNames = new() { ["class_"] = "CatchClass" } },
        ["MediusBusEvent"] = new MadeRecord("BusEvent", "a bus lifecycle event, the numbers are 0 for the kinds that have none. just know that windows.h has an interface macro, so reading `interface` needs an `#undef interface` after it",
            new() { ["configuration"] = "number", ["interface"] = "number", ["alt"] = "number" }, [])
        { CsDoc = "a bus lifecycle event, the numbers are 0 for the kinds that have none" },
        ["MediusMotionEvent"] = new MadeRecord("MotionEvent", "physical motion at the merge point, pan is positive going right", new(),
            [
                new("std::vector<std::pair<Axis, std::int16_t>> all_axes() const", "every axis and its delta, moved or not"),
                new("std::vector<std::pair<Axis, std::int16_t>> axes() const", "the axes this report moved, with their deltas. the box never sends one that moved nothing, a mock can"),
            ]),
        ["MediusCatchEventData"] = new HandRecord("CatchEvent", "MediusMotionEvent motion, MediusUsageEvent usages, MediusTrafficEvent traffic"),
        ["MediusCatchEvent"] = new HandRecord("CatchEvent", "MediusCatchEventKind kind, uint32_t ts_us, MediusClockDomain clock, MediusCatchEventData data"),
        ["MediusInputEvent"] = new HandRecord("InputEvent",
            "MediusInputKind kind, uint32_t ts_us, MediusClockDomain clock, MediusUsage usage, int16_t dx, int16_t dy, int16_t dz, int16_t pan"),
        ["MediusStamped"] = new MadeRecord("Stamped",
            "an event a `Timeline` put on your clock, when it happened on the scale you passed in, its box stamp unwrapped past the 32 bit rollover, and how much later than the measured floor it came, which is jitter and not latency",
            new(), []),
        ["MediusLogLine"] = new MadeRecord("LogLine", "one line of the box's log", new(), []),
    };

    public static readonly Dictionary<string, FunctionUse> Functions = new()
    {
        ["medius_clip_frame_new"] = new HandFunction("ClipFrame()", "struct MediusClipFrame *medius_clip_frame_new(void)"),
        ["medius_clip_frame_free"] = new HandFunction("ClipFrame's destructor", "void medius_clip_frame_free(struct MediusClipFrame *f)"),
        ["medius_clip_frame_clear"] = new MadeFunction("empties the frame to use it again"),
        ["medius_clip_frame_move"] = new MadeFunction("the frame's cursor motion"),
        ["medius_clip_frame_wheel"] = new MadeFunction("the frame's wheel motion"),
        ["medius_clip_frame_pan"] = new MadeFunction("the frame's horizontal scroll"),
        ["medius_clip_frame_edge"] = new MadeFunction("an edge on any usage with the action you pick", "action:Action"),
        ["medius_clip_frame_press"] = new MadeFunction(""),
        ["medius_clip_frame_release"] = new MadeFunction("a soft release, which leaves a physical hold alone"),
        ["medius_clip_frame_force_release"] = new MadeFunction("a release that masks a physical hold too"),
        ["medius_clip_frame_raw"] = new MadeFunction("one raw report, as `Device::raw` sends one", "ep_num:number", "dir:Direction"),
        ["medius_clip_frame_transfer"] = new MadeFunction("one control transfer, as `Device::transfer` runs one", "ep:number", "out_data={}"),
        ["medius_clip_frame_byte_len"] = new MadeFunction("the ring bytes the frame takes, `Clip::append` takes up to `ClipEntryMax`"),
        ["medius_clip_builder_new"] = new HandFunction("ClipBuilder()", "struct MediusClipBuilder *medius_clip_builder_new(void)"),
        ["medius_clip_builder_free"] = new HandFunction("ClipBuilder's destructor", "void medius_clip_builder_free(struct MediusClipBuilder *b)"),
        ["medius_clip_builder_clear"] = new MadeFunction("empties the builder to use it again after an append"),
        ["medius_clip_builder_byte_len"] = new MadeFunction("the ring bytes the entries take, to check against `ClipStatus::free` before an append"),
        ["medius_clip_builder_gap"] = new MadeFunction("nothing for that many native frames, 0 does nothing"),
        ["medius_clip_builder_move"] = new MadeFunction(""),
        ["medius_clip_builder_wheel"] = new MadeFunction(""),
        ["medius_clip_builder_pan"] = new MadeFunction(""),
        ["medius_clip_builder_press"] = new MadeFunction(""),
        ["medius_clip_builder_release"] = new MadeFunction("a soft release, which leaves a physical hold alone"),
        ["medius_clip_builder_force_release"] = new MadeFunction("a release that masks a physical hold too"),
        ["medius_clip_builder_edge"] = new MadeFunction("", "action:Action=Action::Press"),
        ["medius_clip_builder_raw"] = new MadeFunction("", "ep_num:number", "dir:Direction"),
        ["medius_clip_builder_transfer"] = new MadeFunction("one control transfer, its reply comes as a `TrafficClass::ClipTransfer` event", "ep:number", "out_data={}"),
        ["medius_clip_builder_frame"] = new MadeFunction("one frame with all of the frame in it, which stays usable"),
        ["medius_device_clip"] = new HandFunction("Device::clip", "MediusStatus medius_device_clip(struct MediusDevice *dev, struct MediusClip **out)"),
        ["medius_clip_free"] = new HandFunction("Clip's destructor", "void medius_clip_free(struct MediusClip *clip)"),
        ["medius_clip_append"] = new MadeFunction("adds the builder's entries to the ring. every entry is checked before the first frame goes out, so a refusal sends nothing"),
        ["medius_clip_set_autolock"] = new HandFunction("Clip::set_autolock", "MediusStatus medius_clip_set_autolock(struct MediusClip *clip, const uint8_t *scope, uintptr_t scope_len)"),
        ["medius_clip_set_loop"] = new MadeFunction("loops at the clip's end, retained clips only", "on:flag"),
        ["medius_clip_set_retain"] = new MadeFunction("keeps the loaded clip so it can rewind and play again, off is streaming. set it before the first append", "on:flag"),
        ["medius_clip_set_ride"] = new MadeFunction("makes the clip's motion wait to ride a physical report instead of the box's own clock", "on:flag"),
        ["medius_clip_bind"] = new MadeFunction(""),
        ["medius_clip_unbind"] = new MadeFunction("", "edge:Edge"),
        ["medius_clip_bind_packet"] = new MadeFunction(
            "adds or overwrites a packet trigger. one the box would refuse throws `ClipPacketTriggerError` before anything is sent, with why in the message. to be sure a bind took, compare what `query_config()` reads back"),
        ["medius_clip_unbind_packet"] = new HandFunction("Clip::unbind_packet", "MediusStatus medius_clip_unbind_packet(struct MediusClip *clip, const struct MediusClipPacketTrigger *trigger)"),
        ["medius_clip_clear_triggers"] = new MadeFunction(""),
        ["medius_clip_start"] = new MadeFunction("rewinds and plays, or goes on after a pause"),
        ["medius_clip_stop"] = new MadeFunction("stops, flushes a streaming clip or rewinds a retained one, and lets go of held input and the autolock"),
        ["medius_clip_pause"] = new MadeFunction(""),
        ["medius_clip_resume"] = new MadeFunction(""),
        ["medius_clip_restart"] = new MadeFunction("rewinds and plays even in the middle of playing"),
        ["medius_clip_toggle"] = new MadeFunction("plays when idle or paused, stops when playing"),
        ["medius_clip_clear"] = new MadeFunction("throws the loaded clip away, frees the ring and clears a fault"),
        ["medius_clip_finalize"] = new MadeFunction("fixes a retained clip's end so it can play again and loop"),
        ["medius_clip_lost"] = new MadeFunction(
            "whether the box dropped the clip appended since the last clear, because its device chip restarted, it let go of the session, or a reconnect found it gone"),
        ["medius_clip_query_status"] = new MadeFunction(""),
        ["medius_clip_query_config"] = new MadeFunction(""),
        ["medius_device_open"] = new HandFunction("Device::open", "MediusStatus medius_device_open(const char *path, struct MediusDevice **out)"),
        ["medius_device_find"] = new HandFunction("Device::find", "MediusStatus medius_device_find(struct MediusDevice **out)"),
        ["medius_device_clone"] = new HandFunction("Device::clone", "struct MediusDevice *medius_device_clone(const struct MediusDevice *dev)"),
        ["medius_device_free"] = new HandFunction("Device's destructor", "void medius_device_free(struct MediusDevice *dev)"),
        ["medius_find_ports"] = new HandFunction("Device::find_ports", "uintptr_t medius_find_ports(struct MediusPortInfo *out, uintptr_t cap, uintptr_t *out_total)"),
        ["medius_list"] = new HandFunction("Device::list", "uintptr_t medius_list(struct MediusBoxInfo *out, uintptr_t cap, uintptr_t *out_total)"),
        ["medius_device_open_by_id"] = new HandFunction("Device::open_by_id", "MediusStatus medius_device_open_by_id(const char *id, struct MediusDevice **out)"),
        ["medius_device_find_mouse_box"] = new HandFunction("Device::find_mouse_box", "MediusStatus medius_device_find_mouse_box(struct MediusDevice **out)"),
        ["medius_device_find_keyboard_box"] = new HandFunction("Device::find_keyboard_box", "MediusStatus medius_device_find_keyboard_box(struct MediusDevice **out)"),
        ["medius_device_move_rel"] = new MadeFunction(""),
        ["medius_device_wheel"] = new MadeFunction(""),
        ["medius_device_move_rel_now"] = new MadeFunction("a cursor move that skips movement riding, sent on the box's next mouse report"),
        ["medius_device_wheel_now"] = new MadeFunction(""),
        ["medius_device_pan"] = new MadeFunction("horizontal scroll, the whole range with no clamp"),
        ["medius_device_pan_now"] = new MadeFunction(""),
        ["medius_device_flush_motion"] = new MadeFunction("sends the motion held for a ride now, whatever the ride window says"),
        ["medius_device_discard_motion"] = new MadeFunction("drops the motion held for a ride"),
        ["medius_device_move_axis"] = new MadeFunction("", "timing:MoveTiming=MoveTiming::Ride", "pending:PendingMotion=PendingMotion::Keep"),
        ["medius_device_inject"] = new MadeFunction("", "action:Action"),
        ["medius_device_press"] = new MadeFunction(""),
        ["medius_device_soft_release"] = new MadeFunction(""),
        ["medius_device_force_release"] = new MadeFunction(""),
        ["medius_device_scale"] = new MadeFunction(
            "weighs physical input on a target and direction. `scale` is the percent kept, 0 blocks, 100 passes, above that amplifies up to 255, and a negative one down to `LockScaleMin` weighs and reverses (axes only). `lock` and `unlock` are its two ends, and `Device::scale` in the medius docs says how the fixed and relative scales multiply",
            "dir:Direction"),
        ["medius_device_scale_all"] = new MadeFunction("weighs a whole blanket, see `scale` for what the number means", "what:Blanket", "dir:Direction"),
        ["medius_device_lock"] = new MadeFunction("", "dir:Direction"),
        ["medius_device_unlock"] = new MadeFunction("", "dir:Direction"),
        ["medius_device_lock_all"] = new MadeFunction(
            "blocks a whole blanket. `Blanket::Keys` follows the direction, positive blocks the presses only and negative the releases only", "what:Blanket", "dir:Direction"),
        ["medius_device_unlock_all"] = new MadeFunction("", "what:Blanket", "dir:Direction"),
        ["medius_device_led"] = new MadeFunction("", "target:LedTarget", "mode:LedMode", "level:number"),
        ["medius_device_reset"] = new MadeFunction(""),
        ["medius_device_factory_reset"] = new MadeFunction(
            "a reset that also erases the box's stored settings, its name and everything it learned, and reboots it. while it reboots the port stays there but quiet, so queries time out"),
        ["medius_device_reapply"] = new MadeFunction(""),
        ["medius_device_reconnect"] = new MadeFunction(
            "finds and opens this box again and sends it the held state. it throws `BadProtoVerError` when the box answers on another control protocol, and stays disconnected"),
        ["medius_device_reboot"] = new MadeFunction("", "target:RebootTarget"),
        ["medius_device_allow_imperfect_clones"] = new MadeFunction(
            "opts in to cloning devices the box can't clone faithfully, or back out. a change to the served patch set presents the clone again, which lets go of the session like a replug"),
        ["medius_device_raw"] = new MadeFunction(
            "puts `bytes` as they are on the cloned endpoint `ep_num` (0 to 15, only its low 4 bits go over), `Direction::In` to the game PC and `Direction::Out` to the real device, without waiting for anything. with the imperfect clone opt in off the box drops it silently and this still returns, `query_imperfect()` says which it is",
            "ep_num:number", "dir:Direction"),
        ["medius_device_transfer"] = new HandFunction("Device::transfer",
            "MediusStatus medius_device_transfer(struct MediusDevice *dev, uint8_t ep, struct MediusSetup setup, const uint8_t *out_data, uintptr_t out_len, struct MediusTransferOutcome *out)"),
        ["medius_device_transfer_timeout"] = new HandFunction("Device::transfer",
            "MediusStatus medius_device_transfer_timeout(struct MediusDevice *dev, uint8_t ep, struct MediusSetup setup, const uint8_t *out_data, uintptr_t out_len, uint32_t timeout_ms, struct MediusTransferOutcome *out)"),
        ["medius_device_set_rewrite"] = new MadeFunction("adds or overwrites one rewrite rule, it needs the opt in. `query_rewrite()` says what the box holds"),
        ["medius_device_remove_rewrite"] = new HandFunction("Device::remove_rewrite", "MediusStatus medius_device_remove_rewrite(struct MediusDevice *dev, const struct MediusRewriteRule *rule)"),
        ["medius_device_clear_rewrite"] = new MadeFunction(""),
        ["medius_device_query_rewrite"] = new MadeFunction(""),
        ["medius_device_query_rewrite_entry"] = new MadeFunction("one rule in full, in the shape `set_rewrite` takes", "index:number"),
        ["medius_device_set_patch"] = new MadeFunction(
            "stores one descriptor patch, empty bytes remove the one at its key. it reaches the game PC the next time the clone is presented under the opt in"),
        ["medius_device_apply_patch"] = new MadeFunction("presents the clone again with the stored patch set, it needs the opt in, and lets go of the session like a replug"),
        ["medius_device_clear_patch"] = new MadeFunction(""),
        ["medius_device_query_patches"] = new MadeFunction(""),
        ["medius_device_query_patch_entry"] = new MadeFunction("", "index:number"),
        ["medius_device_transform"] = new MadeFunction("adds or overwrites one field transform, no opt in needed"),
        ["medius_device_untransform"] = new MadeFunction("drops the transform with that source and destination, whatever its op"),
        ["medius_device_clear_transforms"] = new MadeFunction(""),
        ["medius_device_transform_swap"] = new MadeFunction("", "a:Axis", "b:Axis"),
        ["medius_device_transform_remap"] = new MadeFunction(""),
        ["medius_device_query_transforms"] = new MadeFunction(""),
        ["medius_device_set_movement_riding"] = new HandFunction("Device::set_movement_riding",
            "MediusStatus medius_device_set_movement_riding(struct MediusDevice *dev, bool enabled, uint32_t window_ms)"),
        ["medius_device_set_emit_pace"] = new HandFunction("Device::set_emit_pace",
            "MediusStatus medius_device_set_emit_pace(struct MediusDevice *dev, uint8_t mode, uint16_t hz, uint16_t force_hz)"),
        ["medius_device_set_name"] = new MadeFunction("the box's name, kept across power cuts, an empty one clears it. the box cuts a name past 32 bytes, or one that isn't ASCII, without an error"),
        ["medius_device_clear_name"] = new MadeFunction("back to the made up Medius-XXXX name"),
        ["medius_device_set_bearing"] = new HandFunction("Device::set_bearing", "MediusStatus medius_device_set_bearing(struct MediusDevice *dev, uint16_t window_ms, uint8_t mode)"),
        ["medius_device_set_render"] = new MadeFunction(
            "the render texture, and whether physical motion goes through the model too. both are one command and kept, so `full` is always needed. nothing renders until the box learned the attached device's profile (`RenderStatus::ready`)",
            "mode:RenderMode"),
        ["medius_device_set_spread"] = new MadeFunction(
            "the percent of the host's command interval an injected delta is spread across, 0 puts it all on the next report and 100 spreads it over one interval"),
        ["medius_device_query_version"] = new MadeFunction(""),
        ["medius_device_firmware_info"] = new MadeFunction("both chips' firmware versions and the slots they booted"),
        ["medius_device_stage_firmware"] = new HandFunction("Device::stage_firmware",
            "MediusStatus medius_device_stage_firmware(struct MediusDevice *dev, uint8_t target, const uint8_t *image, uintptr_t len, void (*progress)(void*, uintptr_t, uintptr_t), void *user)"),
        ["medius_device_abort_update"] = new MadeFunction("drops whatever is staged or on its way for one chip", "target:UpdateTarget"),
        ["medius_device_activate_firmware"] = new MadeFunction("commits every staged image and reboots into it, the host chip first. it waits until the host chip is back"),
        // a progress that throws has to stop the update before the activate, and inside this one call it can't, so
        // Device::update_firmware is stage, activate and abort on an error like the crate's own
        ["medius_device_update_firmware"] = new HandFunction("nothing, Device::update_firmware is composed",
            "MediusStatus medius_device_update_firmware(struct MediusDevice *dev, uint8_t target, const uint8_t *image, uintptr_t len, void (*progress)(void*, uintptr_t, uintptr_t), void *user)"),
        ["medius_device_wait_firmware_confirmed"] = new MadeFunction(
            "waits until neither chip is still on probation, a chip on probation refuses an update. it keeps asking while the box reboots after an activate. after 55 s a chip still on probation is an `UpdateError`, and a box that never answered a `QueryTimeoutError`"),
        ["medius_device_query_health"] = new MadeFunction(""),
        ["medius_device_device_info"] = new MadeFunction(""),
        ["medius_device_caps"] = new MadeFunction(""),
        ["medius_device_query_rate"] = new MadeFunction(""),
        ["medius_device_query_stats"] = new MadeFunction(""),
        ["medius_device_query_locks"] = new MadeFunction(""),
        ["medius_device_query_catch"] = new MadeFunction(""),
        ["medius_device_query_imperfect"] = new MadeFunction(""),
        ["medius_device_query_movement_riding"] = new HandFunction("Device::query_movement_riding",
            "MediusStatus medius_device_query_movement_riding(struct MediusDevice *dev, bool *out_enabled, uint32_t *out_window_ms)"),
        ["medius_device_query_emit_pace"] = new MadeFunction(""),
        ["medius_device_query_bearing"] = new MadeFunction(""),
        ["medius_device_query_render"] = new MadeFunction(""),
        ["medius_device_query_spread"] = new MadeFunction(""),
        ["medius_device_counters"] = new MadeFunction(""),
        ["medius_default_query_timeout_ms"] = new MadeFunction("how long a query waits for the box by default"),
        ["medius_default_transfer_timeout_ms"] = new MadeFunction("how long a control transfer waits for its reply by default"),
        ["medius_default_keepalive_cadence_ms"] = new MadeFunction("how often the library sends the box what's held while something is"),
        ["medius_abi_version"] = new MadeFunction(
            "the medius C ABI the library speaks, which has to be the `AbiVersion` this header was made for. it doesn't check the library first, so it still answers when the two don't match", "unchecked"),
        ["medius_version_string"] = new MadeFunction("the medius crate's version inside the library", "as:version"),
        ["medius_last_error_message"] = new HandFunction("detail::last_error", "uintptr_t medius_last_error_message(char *buf, uintptr_t cap)"),
        ["medius_last_error_proto_ver"] = new HandFunction("detail::last_error", "uint8_t medius_last_error_proto_ver(void)"),
        ["medius_last_error_update"] = new HandFunction("detail::last_error", "bool medius_last_error_update(struct MediusUpdateError *out)"),
        ["medius_usage_button"] = new HandFunction("Usage::button", "struct MediusUsage medius_usage_button(uint8_t button)"),
        ["medius_usage_key"] = new HandFunction("Usage::key", "struct MediusUsage medius_usage_key(MediusKey key)"),
        ["medius_usage_media"] = new HandFunction("Usage::media", "struct MediusUsage medius_usage_media(MediusMediaKey media)"),
        ["medius_motion_cursor"] = new HandFunction("Motion::cursor", "struct MediusMotion medius_motion_cursor(int16_t dx, int16_t dy)"),
        ["medius_motion_wheel"] = new HandFunction("Motion::wheel", "struct MediusMotion medius_motion_wheel(int16_t delta)"),
        ["medius_motion_pan"] = new HandFunction("Motion::pan", "struct MediusMotion medius_motion_pan(int16_t delta)"),
        ["medius_lock_target_axis"] = new HandFunction("LockTarget::axis", "struct MediusLockTarget medius_lock_target_axis(uint8_t kind)"),
        ["medius_lock_target_usage"] = new HandFunction("LockTarget::usage", "struct MediusLockTarget medius_lock_target_usage(struct MediusUsage usage)"),
        ["medius_locks_scale_of"] = new HandFunction("Locks::scale_of", "int16_t medius_locks_scale_of(const struct MediusLocks *locks, struct MediusLockTarget target, uint8_t dir)"),
        ["medius_locks_is_locked"] = new HandFunction("Locks::is_locked", "bool medius_locks_is_locked(const struct MediusLocks *locks, struct MediusLockTarget target, uint8_t dir)"),
        ["medius_rate_native_hz"] = new HandFunction("Rate::native_hz", "bool medius_rate_native_hz(struct MediusRate rate, float *out_hz)"),
        ["medius_usage_event_is_held"] = new HandFunction("nothing, UsageSnapshot::is_held looks through its usages itself like C# does", "bool medius_usage_event_is_held(const struct MediusUsageEvent *event, struct MediusUsage usage)"),
        ["medius_catch_filter_watch"] = new HandFunction("CatchFilter::watch", "struct MediusCatchFilter medius_catch_filter_watch(struct MediusUsage usage)"),
        ["medius_catch_filter_watch_axis"] = new HandFunction("CatchFilter::watch_axis", "struct MediusCatchFilter medius_catch_filter_watch_axis(uint8_t axis)"),
        ["medius_catch_filter_watch_class"] = new HandFunction("CatchFilter::watch_class", "struct MediusCatchFilter medius_catch_filter_watch_class(uint8_t class_)"),
        ["medius_catch_filter_watch_axes"] = new HandFunction("CatchFilter::watch_axes", "struct MediusCatchFilter medius_catch_filter_watch_axes(void)"),
        ["medius_catch_filter_all_input"] = new HandFunction("CatchFilter::all_input", "void medius_catch_filter_all_input(struct MediusCatchFilter *out)"),
        ["medius_catch_filter_traffic"] = new HandFunction("CatchFilter::traffic", "struct MediusCatchFilter medius_catch_filter_traffic(MediusCatchClass class_, uint16_t id)"),
        ["medius_catch_filter_traffic_class"] = new HandFunction("CatchFilter::traffic_of", "struct MediusCatchFilter medius_catch_filter_traffic_class(MediusCatchClass class_)"),
        ["medius_catch_filter_everything"] = new HandFunction("CatchFilter::everything", "struct MediusCatchFilter medius_catch_filter_everything(void)"),
        ["medius_catch_filter_with_direction"] = new HandFunction("CatchFilter::with_direction",
            "struct MediusCatchFilter medius_catch_filter_with_direction(struct MediusCatchFilter f, uint8_t direction)"),
        ["medius_catch_filter_with_capture"] = new HandFunction("CatchFilter::with_capture", "struct MediusCatchFilter medius_catch_filter_with_capture(struct MediusCatchFilter f, uint8_t bytes)"),
        ["medius_catch_filter_on_press"] = new HandFunction("CatchFilter::on_press", "struct MediusCatchFilter medius_catch_filter_on_press(struct MediusCatchFilter f)"),
        ["medius_catch_filter_on_release"] = new HandFunction("CatchFilter::on_release", "struct MediusCatchFilter medius_catch_filter_on_release(struct MediusCatchFilter f)"),
        ["medius_catch_filter_inbound"] = new HandFunction("CatchFilter::inbound", "struct MediusCatchFilter medius_catch_filter_inbound(struct MediusCatchFilter f)"),
        ["medius_catch_filter_outbound"] = new HandFunction("CatchFilter::outbound", "struct MediusCatchFilter medius_catch_filter_outbound(struct MediusCatchFilter f)"),
        ["medius_catch_filter_same_address"] = new HandFunction("CatchFilter::same_address",
            "bool medius_catch_filter_same_address(struct MediusCatchFilter a, struct MediusCatchFilter b)"),
        ["medius_catch_class_is_input"] = new HandFunction("is_input(CatchClass)", "bool medius_catch_class_is_input(MediusCatchClass class_)"),
        ["medius_catch_class_is_traffic"] = new HandFunction("is_traffic(CatchClass)", "bool medius_catch_class_is_traffic(MediusCatchClass class_)"),
        ["medius_traffic_event_truncated"] = new HandFunction("TrafficEvent::truncated", "bool medius_traffic_event_truncated(const struct MediusTrafficEvent *event)"),
        ["medius_traffic_event_setup"] = new HandFunction("TrafficEvent::setup_bytes", "const uint8_t *medius_traffic_event_setup(const struct MediusTrafficEvent *event)"),
        ["medius_traffic_event_data"] = new HandFunction("TrafficEvent::data", "const uint8_t *medius_traffic_event_data(const struct MediusTrafficEvent *event, uintptr_t *out_len)"),
        ["medius_traffic_event_control_status"] = new HandFunction("TrafficEvent::control_status",
            "bool medius_traffic_event_control_status(const struct MediusTrafficEvent *event, MediusControlStatus *out)"),
        ["medius_traffic_event_transfer_status"] = new HandFunction("TrafficEvent::transfer_status",
            "bool medius_traffic_event_transfer_status(const struct MediusTrafficEvent *event, uint8_t *out)"),
        ["medius_traffic_event_bus_event"] = new HandFunction("TrafficEvent::bus_event",
            "bool medius_traffic_event_bus_event(const struct MediusTrafficEvent *event, struct MediusBusEvent *out)"),
        ["medius_traffic_event_rule_acted"] = new HandFunction("TrafficEvent::rule_acted", "bool medius_traffic_event_rule_acted(const struct MediusTrafficEvent *event)"),
        ["medius_traffic_event_bulk_end_of_transfer"] = new HandFunction("TrafficEvent::bulk_end_of_transfer",
            "bool medius_traffic_event_bulk_end_of_transfer(const struct MediusTrafficEvent *event)"),
        ["medius_traffic_event_zlp"] = new HandFunction("TrafficEvent::zlp", "bool medius_traffic_event_zlp(const struct MediusTrafficEvent *event)"),
        ["medius_clip_status_is_held"] = new HandFunction("nothing, ClipStatus::is_held looks through held itself like C# does", "bool medius_clip_status_is_held(const struct MediusClipStatus *status, struct MediusUsage usage)"),
        ["medius_caps_has_mouse"] = new HandFunction("Caps::has_mouse", "bool medius_caps_has_mouse(struct MediusCaps caps)"),
        ["medius_caps_has_keyboard"] = new HandFunction("Caps::has_keyboard", "bool medius_caps_has_keyboard(struct MediusCaps caps)"),
        ["medius_caps_is_composite"] = new HandFunction("Caps::is_composite", "bool medius_caps_is_composite(struct MediusCaps caps)"),
        ["medius_device_info_is_cloned"] = new HandFunction("DeviceInfo::is_cloned", "bool medius_device_info_is_cloned(struct MediusDeviceInfo info)"),
        ["medius_device_catch_events"] = new HandFunction("Device::catch_events",
            "MediusStatus medius_device_catch_events(struct MediusDevice *dev, const struct MediusCatchFilter *filters, uintptr_t n, struct MediusEventStream **out)"),
        ["medius_event_stream_clone"] = new HandFunction("EventStream::clone", "struct MediusEventStream *medius_event_stream_clone(const struct MediusEventStream *stream)"),
        ["medius_event_stream_free"] = new HandFunction("EventStream's destructor", "void medius_event_stream_free(struct MediusEventStream *stream)"),
        ["medius_event_stream_close"] = new HandFunction("EventStream::close", "void medius_event_stream_close(struct MediusEventStream *stream)"),
        ["medius_event_stream_recv"] = new HandFunction("EventStream::recv", "MediusStatus medius_event_stream_recv(struct MediusEventStream *stream, struct MediusCatchEvent *out)"),
        ["medius_event_stream_try_recv"] = new HandFunction("EventStream::try_recv", "bool medius_event_stream_try_recv(struct MediusEventStream *stream, struct MediusCatchEvent *out)"),
        ["medius_event_stream_recv_timeout"] = new HandFunction("EventStream::recv",
            "bool medius_event_stream_recv_timeout(struct MediusEventStream *stream, uint64_t timeout_ms, struct MediusCatchEvent *out)"),
        ["medius_event_stream_dropped"] = new HandFunction("EventStream::dropped", "uint64_t medius_event_stream_dropped(struct MediusEventStream *stream)"),
        ["medius_device_input_events"] = new HandFunction("Device::input_events",
            "MediusStatus medius_device_input_events(struct MediusDevice *dev, const struct MediusCatchFilter *filters, uintptr_t n, struct MediusInputStream **out)"),
        ["medius_input_stream_free"] = new HandFunction("InputStream's destructor", "void medius_input_stream_free(struct MediusInputStream *stream)"),
        ["medius_input_stream_close"] = new HandFunction("InputStream::close", "void medius_input_stream_close(struct MediusInputStream *stream)"),
        ["medius_input_stream_recv"] = new HandFunction("InputStream::recv", "MediusStatus medius_input_stream_recv(struct MediusInputStream *stream, struct MediusInputEvent *out)"),
        ["medius_input_stream_try_recv"] = new HandFunction("InputStream::try_recv", "bool medius_input_stream_try_recv(struct MediusInputStream *stream, struct MediusInputEvent *out)"),
        ["medius_input_stream_recv_timeout"] = new HandFunction("InputStream::recv",
            "bool medius_input_stream_recv_timeout(struct MediusInputStream *stream, uint64_t timeout_ms, struct MediusInputEvent *out)"),
        ["medius_input_stream_dropped"] = new HandFunction("InputStream::dropped", "uint64_t medius_input_stream_dropped(struct MediusInputStream *stream)"),
        ["medius_input_stream_held"] = new HandFunction("InputStream::held",
            "uintptr_t medius_input_stream_held(struct MediusInputStream *stream, uint8_t class_, struct MediusUsage *out, uintptr_t cap)"),
        ["medius_timeline_new"] = new HandFunction("Timeline()", "struct MediusTimeline *medius_timeline_new(void)"),
        ["medius_timeline_free"] = new HandFunction("Timeline's destructor", "void medius_timeline_free(struct MediusTimeline *t)"),
        ["medius_timeline_observe"] = new HandFunction("Timeline::observe",
            "bool medius_timeline_observe(struct MediusTimeline *t, const struct MediusCatchEvent *ev, uint64_t now_ns, struct MediusStamped *out)"),
        ["medius_timeline_observe_input"] = new HandFunction("Timeline::observe",
            "bool medius_timeline_observe_input(struct MediusTimeline *t, const struct MediusInputEvent *ev, uint64_t now_ns, struct MediusStamped *out)"),
        ["medius_timeline_reset"] = new MadeFunction("forgets one chip's rollovers and floor, for a chip that rebooted", "domain:ClockDomain"),
        ["medius_timeline_samples"] = new MadeFunction("how many events a chip had, the floor is the smallest of them", "domain:ClockDomain", "const"),
        ["medius_event_stream_is_connected"] = new HandFunction("EventStream::is_connected", "bool medius_event_stream_is_connected(struct MediusEventStream *stream)"),
        ["medius_input_stream_is_connected"] = new HandFunction("InputStream::is_connected", "bool medius_input_stream_is_connected(struct MediusInputStream *stream)"),
        ["medius_device_logs"] = new HandFunction("Device::logs", "MediusStatus medius_device_logs(struct MediusDevice *dev, struct MediusLogStream **out)"),
        ["medius_log_stream_clone"] = new HandFunction("LogStream::clone", "struct MediusLogStream *medius_log_stream_clone(const struct MediusLogStream *stream)"),
        ["medius_log_stream_free"] = new HandFunction("LogStream's destructor", "void medius_log_stream_free(struct MediusLogStream *stream)"),
        ["medius_log_stream_close"] = new HandFunction("LogStream::close", "void medius_log_stream_close(struct MediusLogStream *stream)"),
        ["medius_log_stream_recv"] = new HandFunction("LogStream::recv", "MediusStatus medius_log_stream_recv(struct MediusLogStream *stream, struct MediusLogLine *out)"),
        ["medius_log_stream_try_recv"] = new HandFunction("LogStream::try_recv", "bool medius_log_stream_try_recv(struct MediusLogStream *stream, struct MediusLogLine *out)"),
        ["medius_log_stream_is_connected"] = new HandFunction("LogStream::is_connected", "bool medius_log_stream_is_connected(struct MediusLogStream *stream)"),
        ["medius_log_stream_recv_timeout"] = new HandFunction("LogStream::recv",
            "bool medius_log_stream_recv_timeout(struct MediusLogStream *stream, uint64_t timeout_ms, struct MediusLogLine *out)"),
        ["medius_mock_new"] = new HandFunction("MockBox()", "struct MediusMockBox *medius_mock_new(void)", Mock: true),
        ["medius_mock_clone"] = new HandFunction("MockBox::clone", "struct MediusMockBox *medius_mock_clone(const struct MediusMockBox *mock)", Mock: true),
        ["medius_mock_free"] = new HandFunction("MockBox's destructor", "void medius_mock_free(struct MediusMockBox *mock)", Mock: true),
        ["medius_mock_set_version"] = new MadeFunction(""),
        ["medius_mock_set_health"] = new MadeFunction(""),
        ["medius_mock_set_device_info"] = new MadeFunction(""),
        ["medius_mock_set_caps"] = new MadeFunction(""),
        ["medius_mock_set_mouse_caps"] = new MadeFunction(""),
        ["medius_mock_set_kbd_caps"] = new MadeFunction(""),
        ["medius_mock_set_rate"] = new MadeFunction(""),
        ["medius_mock_set_stats"] = new MadeFunction(""),
        ["medius_mock_set_locks"] = new MadeFunction(""),
        ["medius_mock_set_catch_state"] = new MadeFunction(""),
        ["medius_mock_set_imperfect_status"] = new MadeFunction(
            "the mock's `query_imperfect()` answer. with the opt in off it drops its consuming clip packet triggers, like the box does"),
        ["medius_mock_set_transfer_reply"] = new MadeFunction(
            "the canned answer to a transfer while the opt in is on, with it off the mock answers refused. a status this header has no name for is sent as its number",
            "status:TransferStatus", "data={}"),
        ["medius_mock_set_movement_riding"] = new HandFunction("MockBox::set_movement_riding",
            "void medius_mock_set_movement_riding(struct MediusMockBox *mock, bool enabled, uint32_t window_ms)", Mock: true),
        ["medius_mock_set_bearing"] = new HandFunction("MockBox::set_bearing", "void medius_mock_set_bearing(struct MediusMockBox *mock, uint16_t window_ms, uint8_t mode)", Mock: true),
        ["medius_mock_set_emit_pace"] = new HandFunction("MockBox::set_emit_pace",
            "void medius_mock_set_emit_pace(struct MediusMockBox *mock, uint8_t mode, uint16_t hz, uint16_t force_hz)", Mock: true),
        ["medius_mock_set_render"] = new MadeFunction(
            "the mock's `query_render()` answer, `ready` is whether a profile is armed, which a real box needs before it renders", "mode:RenderMode"),
        ["medius_mock_set_spread_learned"] = new MadeFunction(
            "the command period the mock learned, in microseconds. a real box learns it from the moves it gets, so a mock left at 0 answers with a span of 0 whatever the percent"),
        ["medius_mock_set_advertised_hz"] = new MadeFunction("the rate the mock's clone advertises when nothing forces one, 0 is no clone"),
        ["medius_mock_set_clip_status"] = new MadeFunction(""),
        ["medius_mock_set_clip_settings"] = new MadeFunction(
            "the mock's `Clip::query_config()` answer. its packet triggers get bound in order like `Clip::bind_packet` binds them, and the mock keeps only the ones the box would"),
        ["medius_mock_clip_packet"] = new HandFunction("MockBox::clip_packet",
            "bool medius_mock_clip_packet(struct MediusMockBox *mock, uint8_t class_, uint16_t id, uint8_t direction, const uint8_t *head, uintptr_t head_len, uint8_t *out_action, bool *out_consumed)", Mock: true),
        ["medius_mock_restart"] = new MadeFunction("acts like the device chip restarting, the session state goes, the stored state stays, and the clone is back 100 ms later"),
        ["medius_mock_link_lost"] = new MadeFunction(
            "acts like the link between the chips dropping and coming back, so what the PC set goes, counted in `Stats::session`, and the clone stays up"),
        ["medius_mock_detach"] = new MadeFunction(
            "acts like the real device unplugging. with `back_within_grace` it's back inside the 250 ms grace and the clone stays up, otherwise the clone goes when the grace ends until `attach()`",
            "back_within_grace=false"),
        ["medius_mock_attach"] = new MadeFunction(""),
        ["medius_mock_refuse_next_activate"] = new MadeFunction(
            "refuses the next activate with that update status (protocol §4.16), and whatever is staged stays staged", "status:number"),
        ["medius_mock_reboot_after_activate"] = new MadeFunction(
            "after each accepted activate, leaves the next `unanswered` firmware queries unanswered like a rebooting box, then has the device chip on probation for the next `pending`"),
        ["medius_mock_silent"] = new MadeFunction("stops answering queries, for good, for timeout tests"),
        ["medius_mock_push_raw"] = new HandFunction("MockBox::push_raw", "void medius_mock_push_raw(struct MediusMockBox *mock, const uint8_t *bytes, uintptr_t len)", Mock: true),
        ["medius_mock_push_log"] = new MadeFunction("", "level:LogLevel"),
        ["medius_mock_push_motion"] = new MadeFunction("", "seq:number"),
        ["medius_mock_push_usages"] = new MadeFunction("", "seq:number"),
        ["medius_mock_push_traffic"] = new MadeFunction(
            "a traffic event as if from the box, a `true_len` above the byte count is a capture that was cut", "seq:number", "clock:ClockDomain"),
        ["medius_mock_recorded"] = new MadeFunction("how many frames the mock got so far"),
        ["medius_mock_saw"] = new MadeFunction("", "ty:FrameType"),
        ["medius_mock_clear_recorded"] = new MadeFunction(""),
        ["medius_mock_recorded_frame"] = new HandFunction("MockBox::recorded_frame",
            "uintptr_t medius_mock_recorded_frame(struct MediusMockBox *mock, uintptr_t idx, MediusFrameType *out_ty, uint8_t *out_seq, uint8_t *payload_buf, uintptr_t cap)", Mock: true),
        ["medius_device_with_mock"] = new HandFunction("Device::with_mock", "MediusStatus medius_device_with_mock(const struct MediusMockBox *mock, struct MediusDevice **out)", Mock: true),
        ["medius_device_open_mock"] = new HandFunction("Device::open_mock", "MediusStatus medius_device_open_mock(const struct MediusMockBox *mock, struct MediusDevice **out)", Mock: true),
    };
}
