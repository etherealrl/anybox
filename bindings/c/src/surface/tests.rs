use super::*;
use crate::conformance::{cpp_code, root};

const CLASSES: [&str; 6] = [
    "suite",
    "generated",
    "hand",
    "hardware",
    "language",
    "upstream",
];

/// whether a row stands for an entry, `Type::*` for all of a type and `Type::CONSTANTS` for its uppercase constants
fn covers(row: &Row, entry: &Entry) -> bool {
    if row.feature != entry.feature {
        return false;
    }
    if let Some(ty) = row.rust.strip_suffix("::*") {
        return entry.name == ty
            || entry.name.starts_with(&format!("{ty}::"))
            || entry.name.starts_with(&format!("{ty}: "));
    }
    if let Some(ty) = row.rust.strip_suffix("::CONSTANTS") {
        return constant_of(ty, &entry.name).is_some();
    }
    row.rust == entry.name
}

/// the constant's own name when the entry is an uppercase constant of the type
fn constant_of<'a>(ty: &str, name: &'a str) -> Option<&'a str> {
    name.strip_prefix(&format!("{ty}::")).filter(|member| {
        member
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    })
}

/// every entry has exactly one row and every row still stands for something
fn holds(entries: &[Entry], rows: &[Row], file: &str) {
    let mut missing = Vec::new();
    for entry in entries {
        let matching: Vec<usize> = rows
            .iter()
            .filter(|row| covers(row, entry))
            .map(|row| row.line)
            .collect();
        match matching.len() {
            0 => missing.push(match &entry.feature {
                Some(feature) => format!("{} ({feature})", entry.name),
                None => entry.name.clone(),
            }),
            1 => {}
            _ => panic!(
                "{} has more than one row in {file}, lines {matching:?}",
                entry.name
            ),
        }
    }
    assert!(
        missing.is_empty(),
        "the crate has these and {file} doesn't, map each to its binding names or say why there isn't one: \
         {missing:?}"
    );
    for row in rows {
        assert!(
            entries.iter().any(|entry| covers(row, entry)),
            "{file} line {} has {}, which the crate doesn't have any more",
            row.line,
            row.rust
        );
    }
}

#[test]
fn the_ledger_is_the_medius_crates_surface() {
    let entries = surface(&package_src("medius"));
    assert!(
        entries.len() > 500,
        "only {} entries found in the medius crate",
        entries.len()
    );
    holds(&entries, &ledger(), "medius-crate.txt");
}

#[test]
fn the_core_ledger_is_the_anybox_crates_surface() {
    let entries = surface(&root().join("src"));
    assert!(
        entries.len() > 200,
        "only {} entries found in the anybox crate",
        entries.len()
    );
    holds(&entries, &core_ledger(), "anybox-crate.txt");
}

#[test]
fn every_row_of_the_ledger_says_how_its_held() {
    for row in ledger() {
        let at = format!("medius-crate.txt line {} ({})", row.line, row.rust);
        let [class, cs, cpp, note] = &row.fields[..] else {
            unreachable!()
        };
        assert!(
            CLASSES.contains(&class.as_str()),
            "{at} is held by {class}, which isn't one of {CLASSES:?}"
        );
        match class.as_str() {
            "upstream" => assert!(
                cs.is_empty() && cpp.is_empty() && !note.is_empty(),
                "{at} is upstream, so it has no binding names and says what medius-capi doesn't export"
            ),
            "suite" | "generated" | "hand" | "hardware" => assert!(
                !cs.is_empty() && !cpp.is_empty(),
                "{at} is held by {class}, so it needs its C# and C++ names"
            ),
            _ => assert!(
                !note.is_empty(),
                "{at} is covered by the language, so its note says how"
            ),
        }
        assert!(
            !note.ends_with('.'),
            "{at}'s note has no full stop at the end"
        );
    }
}

#[test]
fn every_row_of_the_core_ledger_says_why_a_name_is_missing() {
    for row in core_ledger() {
        let at = format!("anybox-crate.txt line {} ({})", row.line, row.rust);
        assert!(
            row.fields[..4].iter().all(|name| !name.is_empty()) || !row.note().is_empty(),
            "{at} leaves a name out, so its note says why"
        );
        assert!(
            !row.note().ends_with('.'),
            "{at}'s note has no full stop at the end"
        );
    }
}

/// whether anybox.h has a C name, which is a function, a constant, a group of constants (`ANYBOX_KEY_*`), a type or
/// a struct's field (`AnyboxInfo.port`)
fn in_header(header: &str, name: &str) -> bool {
    if let Some(prefix) = name.strip_suffix('*') {
        return header.contains(&format!("#define {prefix}"));
    }
    if let Some((strukt, field)) = name.split_once('.') {
        return header
            .split(&format!("typedef struct {strukt} {{"))
            .nth(1)
            .and_then(|body| body.split('}').next())
            .is_some_and(|body| {
                body.lines().any(|line| {
                    line.trim_end_matches(';')
                        .rsplit([' ', '*'])
                        .next()
                        .is_some_and(|last| last.split('[').next() == Some(field))
                })
            });
    }
    if name.starts_with("anybox_") {
        return header.contains(&format!("{name}("));
    }
    if name.starts_with("ANYBOX_") {
        return header.contains(&format!("#define {name} "));
    }
    header.contains(&format!("typedef struct {name} ")) || header.contains(&format!("}} {name};"))
}

#[test]
fn every_c_name_in_the_core_ledger_is_in_the_header() {
    let header = std::fs::read_to_string(root().join("bindings/c/include/anybox.h")).unwrap();
    let entries = surface(&root().join("src"));
    let mut missing = Vec::new();
    for row in core_ledger() {
        let c = &row.fields[0];
        if !c.is_empty() && !in_header(&header, c) {
            missing.push(format!("line {} {c}", row.line));
        }
        // each constant of the type has its own one in the group, by the same name
        if let (Some(ty), Some(prefix)) =
            (row.rust.strip_suffix("::CONSTANTS"), c.strip_suffix('*'))
        {
            for entry in &entries {
                if let Some(constant) = constant_of(ty, &entry.name) {
                    let name = format!("{prefix}{constant}");
                    if !in_header(&header, &name) {
                        missing.push(format!("line {} {name}", row.line));
                    }
                }
            }
        }
    }
    assert!(
        missing.is_empty(),
        "anybox-crate.txt names these for C and anybox.h doesn't have them: {missing:?}"
    );
}

#[test]
fn the_header_lookup_finds_each_kind_of_name_and_refuses_what_isnt_there() {
    let header = std::fs::read_to_string(root().join("bindings/c/include/anybox.h")).unwrap();
    for name in [
        "anybox_device_click",
        "ANYBOX_KEY_A",
        "ANYBOX_KEY_*",
        "AnyboxInfo",
        "AnyboxInfo.port",
        "AnyboxInfo.id_mac",
    ] {
        assert!(in_header(&header, name), "{name}");
    }
    for name in [
        "anybox_device_fly",
        "ANYBOX_KEY_NOPE",
        "ANYBOX_NOPE_*",
        "AnyboxNope",
        "AnyboxInfo.nope",
        "AnyboxInfo.por",
    ] {
        assert!(!in_header(&header, name), "{name}");
    }
}

/// every identifier in C++ code
fn words(code: &str) -> std::collections::BTreeSet<&str> {
    code.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .filter(|word| !word.is_empty())
        .collect()
}

/// the body of a struct, class or enum class the header defines at the start of a line, up to its `};`
fn body<'a>(header: &'a str, ty: &str) -> Option<&'a str> {
    let start = ["struct ", "class ", "enum class "]
        .iter()
        .find_map(|kind| {
            header
                .match_indices(&format!("\n{kind}{ty}"))
                .find_map(|(at, found)| {
                    let rest = &header[at + found.len()..];
                    rest.starts_with([' ', '{', ':']).then_some(at + 1)
                })
        })?;
    let end = header[start..].find("\n};")?;
    Some(&header[start..start + end])
}

/// whether the code has an overload of the free function taking the type first, `to_string(Action)` in a ledger
fn takes(code: &str, function: &str, ty: &str) -> bool {
    code.match_indices(&format!("{function}("))
        .any(|(at, found)| {
            let rest = code[at + found.len()..].trim_start();
            let rest = rest.strip_prefix("const ").unwrap_or(rest);
            rest.strip_prefix(ty)
                .is_some_and(|after| after.starts_with([' ', '&', ')']))
        })
}

/// the names a ledger gives for C++ that the header doesn't have, a member inside its type's body, a free name
/// anywhere. C++ has no reflection, so the suites calling them are what holds them to Rust
fn missing_in(header: &str, rows: &[Row], column: usize) -> Vec<String> {
    let header = cpp_code(header);
    let found = words(&header);
    let has = |name: &str| {
        if let Some((function, ty)) = name.strip_suffix(')').and_then(|name| name.split_once('(')) {
            return takes(&header, function, ty);
        }
        match name.split_once("::") {
            Some((ty, member)) => {
                body(&header, ty).is_some_and(|body| words(body).contains(member))
            }
            None => found.contains(name),
        }
    };
    rows.iter()
        .filter(|row| !row.fields[column].is_empty() && !has(&row.fields[column]))
        .map(|row| format!("line {} {}", row.line, row.fields[column]))
        .collect()
}

#[test]
fn every_cpp_name_in_the_ledger_is_in_the_header() {
    let header =
        std::fs::read_to_string(root().join("bindings/c/include/anybox_medius.hpp")).unwrap();
    let missing = missing_in(&header, &ledger(), 2);
    assert!(
        missing.is_empty(),
        "medius-crate.txt names these for C++ and anybox_medius.hpp doesn't have them: {missing:?}"
    );
}

#[test]
fn the_overload_lookup_takes_the_type_and_nothing_longer() {
    let code = "inline std::string to_string(const StableId& id);
std::string to_string(Key key);";
    assert!(takes(code, "to_string", "StableId"));
    assert!(takes(code, "to_string", "Key"));
    assert!(!takes(code, "to_string", "Stable"));
    assert!(!takes(code, "to_string", "Action"));
}

#[test]
fn every_cpp_name_in_the_core_ledger_is_in_the_header() {
    let header = std::fs::read_to_string(root().join("bindings/c/include/anybox.hpp")).unwrap();
    let missing = missing_in(&header, &core_ledger(), 3);
    assert!(
        missing.is_empty(),
        "anybox-crate.txt names these for C++ and anybox.hpp doesn't have them: {missing:?}"
    );
}

#[test]
fn the_scanner_sees_derived_traits_and_leaves_a_whole_crate_out() {
    let src = std::env::temp_dir().join(format!("anybox-surface-{}", std::process::id()));
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("lib.rs"),
        r#"
pub use other;
pub use inner::{Error, Gated, Value, Wrapped};
mod inner {
    #[derive(Debug, Clone, Default)]
    pub struct Value;
    #[cfg_attr(feature = "extra", derive(Default))]
    pub struct Gated;
    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error("io")]
        Io(#[from] Box<std::io::Error>),
        #[error("fmt")]
        Fmt(#[from] Box<std::fmt::Error>),
    }
    #[derive(Debug, thiserror::Error)]
    #[error(transparent)]
    pub struct Wrapped(#[from] std::io::Error);
}
"#,
    )
    .unwrap();
    let found: Vec<String> = surface(&src)
        .into_iter()
        .map(|entry| match entry.feature {
            Some(feature) => format!("{} ({feature})", entry.name),
            None => entry.name,
        })
        .collect();
    std::fs::remove_dir_all(&src).unwrap();
    assert_eq!(
        found,
        [
            "Error",
            "Error: Display",
            "Error: Error",
            "Error: From<Box<std::fmt::Error>>",
            "Error: From<Box<std::io::Error>>",
            "Gated",
            "Gated: Default (extra)",
            "Value",
            "Value: Default",
            "Wrapped",
            "Wrapped: Display",
            "Wrapped: Error",
            "Wrapped: From<std::io::Error>",
        ]
    );
}
