//! Generates `crates/mg-schema/src/generated/`: field descriptors for every
//! GFF resource type the toolset authors, from the schema observed in the
//! shipped game data.
//!
//!     cargo run -p mg-corpus-tests --release --example schema_codegen
//!
//! Field types come from the data (a field seen with several types gets no
//! typed descriptor, only a note). Defaults are the zero values; defaults
//! that Aurora or the engine use instead belong in hand-written code.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::PathBuf;

use mg_erf::Erf;
use mg_gff::{Gff, Struct, Value};
use mg_key::KeySet;

const TYPES: &[&str] = &[
    "ARE", "DLG", "FAC", "GIC", "GIT", "IFO", "ITP", "JRL", "PTM", "PTT", "UTC", "UTD", "UTE",
    "UTI", "UTM", "UTP", "UTS", "UTT", "UTW",
];

#[derive(Default)]
struct Node {
    fields: BTreeMap<String, FieldObs>,
}

#[derive(Default)]
struct FieldObs {
    types: BTreeSet<&'static str>,
    count: usize,
    files: usize,
    ids: BTreeSet<u32>,
    child: Option<Box<Node>>,
}

/// `seen` holds the paths already counted for the current file.
fn walk(s: &Struct, node: &mut Node, path: &str, seen: &mut BTreeSet<String>) {
    for f in &s.fields {
        let label = f.label.to_string_lossy();
        let fpath = format!("{path}/{label}");
        let obs = node.fields.entry(label).or_default();
        obs.types.insert(f.value.field_type().json_name());
        obs.count += 1;
        if seen.insert(fpath.clone()) {
            obs.files += 1;
        }
        let children: &[Struct] = match &f.value {
            Value::Struct(c) => std::slice::from_ref(c),
            Value::List(items) => items,
            _ => &[],
        };
        for c in children {
            obs.ids.insert(c.id);
            let child = obs.child.get_or_insert_with(Default::default);
            walk(c, child, &fpath, seen);
        }
    }
}

/// `Mod_XPScale` → `MOD_XP_SCALE`, `Creature List` → `CREATURE_LIST`.
fn const_name(label: &str) -> String {
    let chars: Vec<char> = label.chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if !c.is_ascii_alphanumeric() {
            out.push('_');
            continue;
        }
        if i > 0 && c.is_ascii_uppercase() {
            let prev = chars[i - 1];
            let next_lower = chars.get(i + 1).is_some_and(|n| n.is_ascii_lowercase());
            if prev.is_ascii_lowercase()
                || prev.is_ascii_digit()
                || (prev.is_ascii_uppercase() && next_lower)
            {
                out.push('_');
            }
        }
        out.push(c.to_ascii_uppercase());
    }
    let mut collapsed = String::new();
    for c in out.chars() {
        if !(c == '_' && collapsed.ends_with('_')) {
            collapsed.push(c);
        }
    }
    let s = collapsed.trim_matches('_').to_string();
    if s.is_empty() {
        "EMPTY_LABEL".into()
    } else if s.starts_with(|c: char| c.is_ascii_digit()) {
        format!("F_{s}")
    } else {
        s
    }
}

fn module_name(label: &str) -> String {
    let n = const_name(label).to_ascii_lowercase();
    match n.as_str() {
        "type" | "struct" | "mod" | "move" | "match" | "ref" | "self" | "use" | "loop" => {
            format!("{n}_")
        }
        _ => n,
    }
}

fn rust_type(t: &str) -> Option<(&'static str, &'static str, &'static str)> {
    // (Rust type, FieldType variant, default function)
    Some(match t {
        "byte" => ("u8", "Byte", "|| 0"),
        "char" => ("i8", "Char", "|| 0"),
        "word" => ("u16", "Word", "|| 0"),
        "short" => ("i16", "Short", "|| 0"),
        "dword" => ("u32", "Dword", "|| 0"),
        "int" => ("i32", "Int", "|| 0"),
        "dword64" => ("u64", "Dword64", "|| 0"),
        "int64" => ("i64", "Int64", "|| 0"),
        "float" => ("f32", "Float", "|| 0.0"),
        "double" => ("f64", "Double", "|| 0.0"),
        "cexostring" => ("ExoString", "String", "ExoString::default"),
        "resref" => ("ResRef", "ResRef", "|| ResRef::EMPTY"),
        "cexolocstring" => ("LocString", "LocString", "LocString::default"),
        "void" => ("Void", "Void", "Void::default"),
        _ => return None,
    })
}

fn emit(node: &Node, total_files: usize, depth: usize, out: &mut String) {
    let ind = "    ".repeat(depth);
    let mut infos = Vec::new();
    let mut used = BTreeSet::new();
    for (label, obs) in &node.fields {
        let mut name = const_name(label);
        if !used.insert(name.clone()) {
            name = format!("{name}_{}", used.len());
            used.insert(name.clone());
        }
        let pct = 100.0 * obs.files as f64 / total_files.max(1) as f64;
        let types: Vec<&str> = obs.types.iter().copied().collect();
        let _ = writeln!(
            out,
            "{ind}/// `{label}`: {}, in {pct:.0}% of files ({} occurrences).",
            types.join(" or "),
            obs.count
        );
        if types.len() != 1 {
            let _ =
                writeln!(out, "{ind}/// Seen with several types, so it has no typed descriptor;");
            let _ = writeln!(out, "{ind}/// read it through the raw struct.");
            let _ = writeln!(out, "{ind}pub const {name}_LABEL: &str = {label:?};\n");
            continue;
        }
        let t = types[0];
        let id = obs.ids.iter().next().copied().unwrap_or(0);
        let id_note = if obs.ids.len() > 1 {
            format!(" Struct ids vary ({} seen); new items get {id}.", obs.ids.len())
        } else {
            String::new()
        };
        match t {
            "list" => {
                if !id_note.is_empty() {
                    let _ = writeln!(out, "{ind}///{id_note}");
                }
                let _ = writeln!(
                    out,
                    "{ind}pub const {name}: ListField = ListField {{ label: {label:?}, item_id: {id} }};\n"
                );
                infos.push((label.clone(), "List"));
            }
            "struct" => {
                if !id_note.is_empty() {
                    let _ = writeln!(out, "{ind}///{id_note}");
                }
                let _ = writeln!(
                    out,
                    "{ind}pub const {name}: StructField = StructField {{ label: {label:?}, struct_id: {id} }};\n"
                );
                infos.push((label.clone(), "Struct"));
            }
            _ => {
                let (rt, variant, default) = rust_type(t).expect("known type");
                let _ = writeln!(
                    out,
                    "{ind}pub const {name}: Field<{rt}> = Field::new({label:?}, {default});\n"
                );
                infos.push((label.clone(), variant));
            }
        }
        if let Some(child) = &obs.child {
            let m = module_name(label);
            let _ = writeln!(out, "{ind}/// Fields of `{label}` items.");
            let _ = writeln!(out, "{ind}pub mod {m} {{");
            let _ = writeln!(out, "{ind}    #[allow(unused_imports)]");
            let _ = writeln!(out, "{ind}    use crate::*;");
            let _ = writeln!(out, "{ind}    #[allow(unused_imports)]");
            let _ = writeln!(out, "{ind}    use mg_core::{{LocString, ResRef}};");
            let _ = writeln!(out, "{ind}    #[allow(unused_imports)]");
            let _ = writeln!(out, "{ind}    use mg_gff::FieldType;\n");
            emit(child, total_files, depth + 1, out);
            let _ = writeln!(out, "{ind}}}\n");
        }
    }
    let _ = writeln!(out, "{ind}/// Every field of this struct seen in the shipped data.");
    let _ = writeln!(out, "{ind}pub const FIELDS: &[FieldInfo] = &[");
    for (label, variant) in infos {
        let _ = writeln!(
            out,
            "{ind}    FieldInfo {{ label: {label:?}, field_type: FieldType::{variant} }},"
        );
    }
    let _ = writeln!(out, "{ind}];");
}

fn main() {
    let root = mg_testkit::nwn_root().expect("no game install (set NWN_ROOT)");
    let mut by_type: BTreeMap<String, (usize, Node)> = BTreeMap::new();
    let mut add = |bytes: &[u8]| {
        let Ok(g) = Gff::read(bytes) else { return };
        let ty = g.file_type_str();
        if !TYPES.contains(&ty.as_str()) {
            return;
        }
        let (n, node) = by_type.entry(ty).or_default();
        *n += 1;
        walk(&g.root, node, "", &mut BTreeSet::new());
    };
    for key in ["nwn_base.key", "nwn_retail.key"] {
        let ks = KeySet::open(&root.join("data").join(key), &root).unwrap();
        for e in ks.table.entries.iter().filter(|e| e.restype.is_gff()) {
            add(ks.data(e).unwrap());
        }
    }
    let mut archives = mg_testkit::bundled_modules(&root);
    archives.extend(mg_testkit::bundled_haks(&root));
    for a in &archives {
        let data = std::fs::read(a).unwrap();
        let erf = Erf::read(&data).unwrap();
        for e in erf.entries.iter().filter(|e| e.restype.is_gff()) {
            add(&erf.data(e).unwrap());
        }
    }

    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../mg-schema/src/generated");
    std::fs::create_dir_all(&dir).unwrap();
    let mut modrs = String::from(
        "//! Generated by `crates/mg-corpus-tests/examples/schema_codegen.rs` from the\n\
         //! shipped game data. Do not edit; regenerate.\n\n",
    );
    for (ty, (files, node)) in &by_type {
        let m = ty.to_ascii_lowercase();
        let mut out = format!(
            "//! `{ty}` fields, as seen in {files} shipped files.\n\
             //! Generated by `crates/mg-corpus-tests/examples/schema_codegen.rs`; do not edit.\n\n\
             #[allow(unused_imports)]\nuse crate::*;\n#[allow(unused_imports)]\nuse mg_core::{{LocString, ResRef}};\n\
             #[allow(unused_imports)]\nuse mg_gff::FieldType;\n\n"
        );
        emit(node, *files, 0, &mut out);
        std::fs::write(dir.join(format!("{m}.rs")), out).unwrap();
        let _ = writeln!(modrs, "pub mod {m};");
    }
    std::fs::write(dir.join("mod.rs"), modrs).unwrap();
    eprintln!("generated {} modules in {}", by_type.len(), dir.display());
}
