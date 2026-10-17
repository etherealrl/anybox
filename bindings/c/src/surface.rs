// the public surface of a crate, read from its source: the functions and constants reachable from its root, and for
// each reachable type its inherent functions and constants and the traits it implements by hand. a binding mirrors
// these, so a ledger of them can be held to the crate. it follows the module tree and the `pub use`s by name, which
// holds as long as a public name is defined once, and it says so when it isn't

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use syn::{Attribute, ImplItem, Item, TraitItem, Type, UseTree, Visibility};

/// one thing a binding has to mirror: `Type::name`, `Type: Trait`, `Trait::name`, or a free function or constant
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Entry {
    pub name: String,
    /// the cargo feature it's only there with
    pub feature: Option<String>,
}

// the traits every binding gets from its language, or that only Rust has
const LEFT_OUT_TRAITS: [&str; 12] = [
    "Debug",
    "Clone",
    "Copy",
    "PartialEq",
    "Eq",
    "Hash",
    "PartialOrd",
    "Ord",
    "Drop",
    "Send",
    "Sync",
    "Unpin",
];

struct Impl {
    ty: String,
    tr: Option<String>,
    items: Vec<ImplItem>,
    feature: Option<String>,
}

#[derive(Default)]
struct Scan {
    // the names the crate's public modules define or re-export
    reachable: BTreeSet<String>,
    // every type, trait, function and constant the crate defines, with its feature and whether it says pub
    defined: BTreeMap<String, Vec<(Kind, Option<String>, bool)>>,
    // every impl block
    impls: Vec<Impl>,
    // every trait: its functions and constants
    traits: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Type,
    Trait,
    Function,
    Constant,
}

/// the `feature = "x"` a `#[cfg]` asks for, or `test` for one only built for tests
fn cfg(attrs: &[Attribute]) -> Option<String> {
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("cfg")) {
        let text = attr.meta.require_list().ok()?.tokens.to_string();
        if text == "test" {
            return Some("test".into());
        }
        if let Some(rest) = text.split("feature = \"").nth(1) {
            return Some(rest.split('"').next().unwrap().to_string());
        }
    }
    None
}

fn public(vis: &Visibility) -> bool {
    matches!(vis, Visibility::Public(_))
}

/// the name a type is known by, its last path segment without generics
fn type_name(ty: &Type) -> Option<String> {
    match ty {
        Type::Path(path) => path
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string()),
        _ => None,
    }
}

/// a type the way its source spells it, with its path and generics, `Box<std::io::Error>`
fn type_text(ty: &Type) -> String {
    match ty {
        Type::Path(path) => path
            .path
            .segments
            .iter()
            .map(|segment| match &segment.arguments {
                syn::PathArguments::AngleBracketed(arguments) => {
                    let inner: Vec<String> = arguments
                        .args
                        .iter()
                        .map(|argument| match argument {
                            syn::GenericArgument::Type(ty) => type_text(ty),
                            _ => "_".into(),
                        })
                        .collect();
                    format!("{}<{}>", segment.ident, inner.join(", "))
                }
                _ => segment.ident.to_string(),
            })
            .collect::<Vec<_>>()
            .join("::"),
        Type::Reference(reference) => format!("&{}", type_text(&reference.elem)),
        _ => "_".into(),
    }
}

/// the traits a `#[derive]` names, by their last path segment, each with the feature a `#[cfg_attr]` around it needs
fn derives(attrs: &[Attribute]) -> Vec<(String, Option<String>)> {
    let mut found = Vec::new();
    let mut read = |list: &syn::MetaList, feature: Option<String>| {
        list.parse_nested_meta(|meta| {
            found.push((
                meta.path.segments.last().unwrap().ident.to_string(),
                feature.clone(),
            ));
            Ok(())
        })
        .unwrap();
    };
    for attr in attrs {
        if attr.path().is_ident("derive") {
            read(attr.meta.require_list().unwrap(), None);
        } else if attr.path().is_ident("cfg_attr") {
            let metas = attr
                .parse_args_with(
                    syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
                )
                .unwrap();
            let mut metas = metas.into_iter();
            let condition = metas.next().unwrap();
            let derived: Vec<syn::MetaList> = metas
                .filter_map(|meta| match meta {
                    syn::Meta::List(list) if list.path.is_ident("derive") => Some(list),
                    _ => None,
                })
                .collect();
            if derived.is_empty() {
                continue;
            }
            // only a feature can be named in the ledger, so a derive on any other condition stops the scan
            let feature = match &condition {
                syn::Meta::NameValue(pair) if pair.path.is_ident("feature") => match &pair.value {
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(name),
                        ..
                    }) => name.value(),
                    _ => panic!("a cfg_attr derive on a feature the scanner can't read"),
                },
                syn::Meta::Path(path) if path.is_ident("test") => continue,
                _ => panic!(
                    "a cfg_attr derive on a condition the scanner can't read, only feature = \"x\" is"
                ),
            };
            for list in &derived {
                read(list, Some(feature.clone()));
            }
        }
    }
    found
}

/// the leaf names a `use` tree brings in, after any `as`
fn use_names(tree: &UseTree, out: &mut Vec<String>) {
    match tree {
        UseTree::Path(path) => use_names(&path.tree, out),
        UseTree::Name(name) => out.push(name.ident.to_string()),
        UseTree::Rename(rename) => out.push(rename.rename.to_string()),
        UseTree::Group(group) => group.items.iter().for_each(|item| use_names(item, out)),
        UseTree::Glob(_) => {
            panic!("a glob `pub use` can't be followed by name, list what it brings in")
        }
    }
}

impl Scan {
    fn module(&mut self, items: &[Item], dir: &Path, reachable: bool, feature: Option<String>) {
        for item in items {
            let (attrs, kind, name, vis) = match item {
                Item::Struct(it) => (&it.attrs, Some(Kind::Type), it.ident.to_string(), &it.vis),
                Item::Enum(it) => (&it.attrs, Some(Kind::Type), it.ident.to_string(), &it.vis),
                Item::Type(it) => (&it.attrs, Some(Kind::Type), it.ident.to_string(), &it.vis),
                Item::Trait(it) => (&it.attrs, Some(Kind::Trait), it.ident.to_string(), &it.vis),
                Item::Fn(it) => (
                    &it.attrs,
                    Some(Kind::Function),
                    it.sig.ident.to_string(),
                    &it.vis,
                ),
                Item::Const(it) => (
                    &it.attrs,
                    Some(Kind::Constant),
                    it.ident.to_string(),
                    &it.vis,
                ),
                Item::Static(it) => (
                    &it.attrs,
                    Some(Kind::Constant),
                    it.ident.to_string(),
                    &it.vis,
                ),
                Item::Mod(it) => (&it.attrs, None, it.ident.to_string(), &it.vis),
                Item::Use(it) => (&it.attrs, None, String::new(), &it.vis),
                Item::Impl(it) => (&it.attrs, None, String::new(), &Visibility::Inherited),
                _ => continue,
            };
            let own = cfg(attrs);
            if own.as_deref() == Some("test") {
                continue;
            }
            let feature = own.or(feature.clone());
            // a derived trait is an impl like a written one, a derived Default is a value every binding has to match
            if let Item::Struct(_) | Item::Enum(_) = item {
                let mut traits = derives(attrs);
                // thiserror's derive writes the Display from #[error] and a From for each #[from] field too
                if let Some((_, derived)) = traits.iter().find(|(tr, _)| tr == "Error").cloned() {
                    traits.push(("Display".into(), derived.clone()));
                    let fields: Vec<&syn::Field> = match item {
                        Item::Enum(it) => it
                            .variants
                            .iter()
                            .flat_map(|variant| variant.fields.iter())
                            .collect(),
                        Item::Struct(it) => it.fields.iter().collect(),
                        _ => Vec::new(),
                    };
                    for field in fields {
                        if field.attrs.iter().any(|attr| attr.path().is_ident("from")) {
                            traits
                                .push((format!("From<{}>", type_text(&field.ty)), derived.clone()));
                        }
                    }
                }
                for (tr, derived) in traits {
                    self.impls.push(Impl {
                        ty: name.clone(),
                        tr: Some(tr),
                        items: Vec::new(),
                        feature: derived.or(feature.clone()),
                    });
                }
            }
            match item {
                Item::Mod(it) => {
                    let inner = reachable && public(vis);
                    match &it.content {
                        Some((_, items)) => self.module(items, &dir.join(&name), inner, feature),
                        None => {
                            let (file, below) = if dir.join(format!("{name}.rs")).exists() {
                                (dir.join(format!("{name}.rs")), dir.join(&name))
                            } else {
                                (dir.join(&name).join("mod.rs"), dir.join(&name))
                            };
                            let text = std::fs::read_to_string(&file).unwrap_or_else(|error| {
                                panic!("can't read {}: {error}", file.display())
                            });
                            let parsed = syn::parse_file(&text).unwrap_or_else(|error| {
                                panic!("can't parse {}: {error}", file.display())
                            });
                            self.module(&parsed.items, &below, inner, feature);
                        }
                    }
                }
                Item::Use(it) => {
                    // a bare `pub use name;` hands out another crate whole, and that crate gets its own ledger
                    let whole_crate = matches!(it.tree, UseTree::Name(_));
                    if reachable && public(vis) && !whole_crate {
                        let mut names = Vec::new();
                        use_names(&it.tree, &mut names);
                        self.reachable.extend(names);
                    }
                }
                Item::Impl(it) => {
                    let Some(ty) = type_name(&it.self_ty) else {
                        continue;
                    };
                    let tr = it.trait_.as_ref().map(|(path, _)| {
                        let last = path.segments.last().unwrap();
                        let arguments = match &last.arguments {
                            syn::PathArguments::AngleBracketed(arguments) => {
                                let inner: Vec<String> = arguments
                                    .args
                                    .iter()
                                    .map(|argument| match argument {
                                        syn::GenericArgument::Type(ty) => {
                                            type_name(ty).unwrap_or_else(|| "_".into())
                                        }
                                        _ => "_".into(),
                                    })
                                    .collect();
                                format!("<{}>", inner.join(", "))
                            }
                            _ => String::new(),
                        };
                        format!("{}{arguments}", last.ident)
                    });
                    self.impls.push(Impl {
                        ty,
                        tr,
                        items: it.items.clone(),
                        feature,
                    });
                }
                Item::Trait(it) => {
                    let members = it
                        .items
                        .iter()
                        .filter_map(|member| match member {
                            TraitItem::Fn(f) => Some(f.sig.ident.to_string()),
                            TraitItem::Const(c) => Some(c.ident.to_string()),
                            _ => None,
                        })
                        .collect();
                    self.traits.insert(name.clone(), members);
                    self.define(
                        name,
                        Kind::Trait,
                        feature,
                        public(vis),
                        reachable && public(vis),
                    );
                }
                _ => self.define(
                    name,
                    kind.unwrap(),
                    feature,
                    public(vis),
                    reachable && public(vis),
                ),
            }
        }
    }

    fn define(
        &mut self,
        name: String,
        kind: Kind,
        feature: Option<String>,
        public: bool,
        reachable: bool,
    ) {
        if reachable {
            self.reachable.insert(name.clone());
        }
        self.defined
            .entry(name)
            .or_default()
            .push((kind, feature, public));
    }
}

/// every entry of the crate whose `src` folder this is, sorted
pub fn surface(src: &Path) -> Vec<Entry> {
    let text = std::fs::read_to_string(src.join("lib.rs")).unwrap();
    let mut scan = Scan::default();
    scan.module(&syn::parse_file(&text).unwrap().items, src, true, None);
    let mut entries = BTreeSet::new();
    for name in &scan.reachable {
        let Some(definitions) = scan.defined.get(name) else {
            panic!(
                "{name} is re-exported but the scan found no definition of it, it may come from another crate"
            );
        };
        // a private constant of the same name elsewhere is fine, a second type isn't, its impls would be mixed in
        let said_pub: Vec<_> = definitions.iter().filter(|(.., said)| *said).collect();
        let types = definitions
            .iter()
            .filter(|(kind, ..)| *kind == Kind::Type)
            .count();
        assert!(
            said_pub.len() == 1 && types <= 1,
            "{name} is defined {} times, so following it by name can't tell which one is public",
            definitions.len()
        );
        let (kind, feature, _) = said_pub[0].clone();
        match kind {
            Kind::Function | Kind::Constant => {
                entries.insert(Entry {
                    name: name.clone(),
                    feature,
                });
            }
            Kind::Trait => {
                entries.insert(Entry {
                    name: name.clone(),
                    feature: feature.clone(),
                });
                for member in &scan.traits[name] {
                    entries.insert(Entry {
                        name: format!("{name}::{member}"),
                        feature: feature.clone(),
                    });
                }
            }
            Kind::Type => {
                entries.insert(Entry {
                    name: name.clone(),
                    feature: feature.clone(),
                });
                for Impl {
                    ty,
                    tr,
                    items,
                    feature: impl_feature,
                } in scan.impls.iter().filter(|found| &found.ty == name)
                {
                    let feature = impl_feature.clone().or(feature.clone());
                    match tr {
                        Some(tr) => {
                            let base = tr.split('<').next().unwrap();
                            let private =
                                scan.defined.contains_key(base) && !scan.reachable.contains(base);
                            if !LEFT_OUT_TRAITS.contains(&base) && !private {
                                entries.insert(Entry {
                                    name: format!("{ty}: {tr}"),
                                    feature,
                                });
                            }
                        }
                        None => {
                            for item in items {
                                let member = match item {
                                    ImplItem::Fn(f)
                                        if public(&f.vis)
                                            && cfg(&f.attrs).as_deref() != Some("test") =>
                                    {
                                        Some((f.sig.ident.to_string(), cfg(&f.attrs)))
                                    }
                                    ImplItem::Const(c) if public(&c.vis) => {
                                        Some((c.ident.to_string(), cfg(&c.attrs)))
                                    }
                                    _ => None,
                                };
                                if let Some((member, own)) = member {
                                    entries.insert(Entry {
                                        name: format!("{ty}::{member}"),
                                        feature: own.or(feature.clone()),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    entries.into_iter().collect()
}

/// the `src` folder of a package in this workspace's dependency graph, through cargo metadata like gen-header.sh
pub fn package_src(package: &str) -> PathBuf {
    let output =
        std::process::Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
            .args([
                "metadata",
                "--format-version",
                "1",
                "--locked",
                "--manifest-path",
            ])
            .arg(PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("Cargo.toml"))
            .output()
            .unwrap();
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let manifest = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|found| found["name"] == package)
        .unwrap_or_else(|| panic!("cargo metadata has no {package}"))["manifest_path"]
        .as_str()
        .unwrap()
        .to_string();
    PathBuf::from(manifest).parent().unwrap().join("src")
}

/// one row of a ledger in bindings/conformance, the Rust name, the cargo feature it needs and the fields after it,
/// with an empty note when it has none
#[derive(Debug)]
pub(crate) struct Row {
    pub(crate) line: usize,
    pub(crate) rust: String,
    pub(crate) feature: Option<String>,
    pub(crate) fields: Vec<String>,
}

impl Row {
    pub(crate) fn note(&self) -> &str {
        self.fields.last().unwrap()
    }
}

/// a ledger whose rows have `fields` after the Rust name, the last one a note that can be left out
pub(crate) fn rows(file: &str, fields: usize) -> Vec<Row> {
    let text = std::fs::read_to_string(
        crate::conformance::root()
            .join("bindings/conformance")
            .join(file),
    )
    .unwrap();
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|(index, line)| {
            let mut split: Vec<String> = line
                .split('|')
                .map(|field| field.trim().to_string())
                .collect();
            assert!(
                (fields..=fields + 1).contains(&split.len()),
                "{file} line {} needs {fields} or {} fields split by |: {line}",
                index + 1,
                fields + 1
            );
            split.resize(fields + 1, String::new());
            let first = split.remove(0);
            let (rust, feature) = match first
                .strip_suffix(')')
                .and_then(|rest| rest.rsplit_once(" ("))
            {
                Some((rust, feature)) => (rust.to_string(), Some(feature.to_string())),
                None => (first, None),
            };
            Row {
                line: index + 1,
                rust,
                feature,
                fields: split,
            }
        })
        .collect()
}

/// medius-crate.txt, each row saying how it's held, its C# name, its C++ name and a note
pub(crate) fn ledger() -> Vec<Row> {
    rows("medius-crate.txt", 4)
}

/// anybox-crate.txt, each row with its C name, its Python name, its C# name, its C++ name and a note
pub(crate) fn core_ledger() -> Vec<Row> {
    rows("anybox-crate.txt", 5)
}

mod tests;
