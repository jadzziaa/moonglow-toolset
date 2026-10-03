//! Edits written out and read back: a [`Command`] as JSON, for `mg apply`,
//! for `--dry-run`, and for anything outside Moonglow that proposes changes
//! to a module.
//!
//! ```json
//! { "version": 1, "label": "Upper-case tags", "edits": [
//!   { "op": "set_field", "resource": "bandit.utc", "field": "/Tag",
//!     "value": { "type": "cexostring", "value": "BANDIT" } },
//!   { "op": "remove_field", "resource": "bandit.utc", "field": "/Comment" },
//!   { "op": "insert_item", "resource": "area.git", "list": "/Creature List",
//!     "index": 0, "item": { "__struct_id": 4 } },
//!   { "op": "remove_item", "resource": "area.git", "item": "/Creature List[0]" },
//!   { "op": "set_resource", "resource": "hello.nss", "text": "void main() {}" },
//!   { "op": "remove_resource", "resource": "old.utc" } ] }
//! ```
//!
//! Values and structs are nwn-lib JSON (`mg_gff::json`). A path is the
//! labels from a resource's root, each after a `/`, a list's item as
//! `Label[index]`; in a label, `~0`, `~1`, `~2` and `~3` stand for `~`, `/`,
//! `[` and `]`, and `~e` alone for a label that is empty. What is read is
//! taken as untrusted: every fault is an error that says where, never a
//! panic.

use std::str::FromStr;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use mg_core::Codepage;
use mg_gff::{Gff, Label};
use mg_resman::ResKey;
use serde_json::{Map, Value as Json, json};
use thiserror::Error;

use crate::{Command, Edit, GffPath, Step};

/// The version of the format written; a file of a later one is refused.
pub const VERSION: u64 = 1;

/// JSON that is not a command, or a command JSON cannot hold.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{at}: {message}")]
pub struct WireError {
    /// Where: `edits[3]`, `edits[3].value/Tag`.
    pub at: String,
    pub message: String,
}

fn fault<T>(at: &str, message: impl Into<String>) -> Result<T, WireError> {
    Err(WireError { at: at.to_string(), message: message.into() })
}

/// A label as a path writes it.
pub(crate) fn escape(label: &str) -> String {
    if label.is_empty() {
        return "~e".into();
    }
    let mut out = String::with_capacity(label.len());
    for c in label.chars() {
        match c {
            '~' => out.push_str("~0"),
            '/' => out.push_str("~1"),
            '[' => out.push_str("~2"),
            ']' => out.push_str("~3"),
            c => out.push(c),
        }
    }
    out
}

fn unescape(text: &str) -> Result<String, String> {
    if text == "~e" {
        return Ok(String::new());
    }
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            '~' => out.push(match chars.next() {
                Some('0') => '~',
                Some('1') => '/',
                Some('2') => '[',
                Some('3') => ']',
                other => {
                    let shown = other.map(String::from).unwrap_or_default();
                    return Err(format!("~{shown} is no escape (~0 ~1 ~2 ~3, or ~e alone)"));
                }
            }),
            '[' | ']' => return Err(format!("a stray {c} in {text:?} (write ~2 for [, ~3 for ])")),
            c => out.push(c),
        }
    }
    if out.is_empty() {
        return Err("an empty label is written ~e".into());
    }
    Ok(out)
}

impl FromStr for GffPath {
    type Err = String;

    /// Reads what `Display` writes: `/` for the root, else `/Label` and
    /// `/Label[index]` steps.
    fn from_str(text: &str) -> Result<GffPath, String> {
        let Some(rest) = text.strip_prefix('/') else {
            return Err(format!("a path starts with /: {text:?}"));
        };
        if rest.is_empty() {
            return Ok(GffPath::root());
        }
        let mut steps = Vec::new();
        for part in rest.split('/') {
            // An item: the label, then [index] at the very end.
            let step = match part.strip_suffix(']').and_then(|p| p.rsplit_once('[')) {
                Some((label, index)) => {
                    let index = index
                        .parse()
                        .map_err(|_| format!("{index:?} in {part:?} is not an item's number"))?;
                    Step::Item(unescape(label)?, index)
                }
                None => Step::Field(unescape(part)?),
            };
            steps.push(step);
        }
        Ok(GffPath(steps))
    }
}

/// A field's address as one path, `/List[2]/Label`: the path to its struct,
/// then its label.
pub fn field_to_string(path: &GffPath, label: &str) -> String {
    path.field(label).to_string()
}

/// Reads a field's address ([`field_to_string`]): the path to its struct
/// and its label. A leading `/` may be left out (`Tag` is `/Tag`).
pub fn field_from_str(text: &str) -> Result<(GffPath, String), String> {
    let full: GffPath =
        if text.starts_with('/') { text.parse()? } else { format!("/{text}").parse()? };
    let mut steps = full.0;
    match steps.pop() {
        Some(Step::Field(label)) => Ok((GffPath(steps), label)),
        Some(Step::Item(..)) => Err(format!("{text:?} is a list's item, not a field")),
        None => Err("no field named".into()),
    }
}

/// A command as JSON. Strings are converted from `codepage` to UTF-8.
pub fn command_to_json(cmd: &Command, codepage: Codepage) -> Result<Json, WireError> {
    let edits = cmd
        .edits
        .iter()
        .enumerate()
        .map(|(i, e)| edit_to_json(e, codepage, &format!("edits[{i}]")))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({ "version": VERSION, "label": cmd.label, "edits": edits }))
}

fn gff_fault(at: &str, what: &str, e: mg_gff::JsonError) -> WireError {
    WireError { at: format!("{at}.{what}"), message: e.to_string() }
}

fn edit_to_json(edit: &Edit, cp: Codepage, at: &str) -> Result<Json, WireError> {
    Ok(match edit {
        Edit::SetField { key, path, label, value: Some(value) } => json!({
            "op": "set_field",
            "resource": key.to_string(),
            "field": field_to_string(path, label),
            "value": mg_gff::value_to_json(value, cp).map_err(|e| gff_fault(at, "value", e))?,
        }),
        Edit::SetField { key, path, label, value: None } => json!({
            "op": "remove_field",
            "resource": key.to_string(),
            "field": field_to_string(path, label),
        }),
        Edit::InsertItem { key, path, list, index, item } => json!({
            "op": "insert_item",
            "resource": key.to_string(),
            "list": field_to_string(path, list),
            "index": index,
            "item": mg_gff::struct_to_json(item, cp).map_err(|e| gff_fault(at, "item", e))?,
        }),
        Edit::RemoveItem { key, path, list, index } => json!({
            "op": "remove_item",
            "resource": key.to_string(),
            "item": path.item(list, *index).to_string(),
        }),
        Edit::SetResource { key, data: Some(data) } => {
            let mut o = Map::new();
            o.insert("op".into(), json!("set_resource"));
            o.insert("resource".into(), json!(key.to_string()));
            match as_text(data, cp) {
                Some(text) => o.insert("text".into(), json!(text)),
                None => o.insert("base64".into(), json!(BASE64.encode(data))),
            };
            Json::Object(o)
        }
        Edit::SetResource { key, data: None } => json!({
            "op": "remove_resource",
            "resource": key.to_string(),
        }),
    })
}

/// Bytes as text, when they are text that reads back to the same bytes.
fn as_text(data: &[u8], cp: Codepage) -> Option<String> {
    if data.iter().any(|&b| b < 0x20 && !matches!(b, b'\t' | b'\n' | b'\r')) {
        return None;
    }
    let text = cp.decode(data);
    (cp.encode(&text).as_deref() == Some(data)).then(|| text.into_owned())
}

/// Reads a command. Strings are converted from UTF-8 to `codepage`.
pub fn command_from_json(json: &Json, codepage: Codepage) -> Result<Command, WireError> {
    let Json::Object(o) = json else { return fault("", "expected an object with \"edits\"") };
    known(o, &["version", "label", "edits"], "")?;
    match o.get("version") {
        None => {}
        Some(v) if v.as_u64().is_some_and(|v| (1..=VERSION).contains(&v)) => {}
        Some(v) => {
            return fault("version", format!("{v} is not a version this Moonglow reads (1)"));
        }
    }
    let label = match o.get("label") {
        None => "Apply edits".to_string(),
        Some(Json::String(s)) => s.clone(),
        Some(_) => return fault("label", "expected a string"),
    };
    let Some(Json::Array(list)) = o.get("edits") else {
        return fault("edits", "expected a list of edits");
    };
    let edits = list
        .iter()
        .enumerate()
        .map(|(i, e)| edit_from_json(e, codepage, &format!("edits[{i}]")))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Command { label, edits })
}

/// Refuses keys an object should not have (a misspelled one, most likely).
fn known(o: &Map<String, Json>, keys: &[&str], at: &str) -> Result<(), WireError> {
    match o.keys().find(|k| !keys.contains(&k.as_str())) {
        Some(k) => fault(at, format!("unknown key {k:?} (expected {})", keys.join(", "))),
        None => Ok(()),
    }
}

fn text<'a>(o: &'a Map<String, Json>, key: &str, at: &str) -> Result<&'a str, WireError> {
    match o.get(key) {
        Some(Json::String(s)) => Ok(s),
        Some(_) => fault(at, format!("{key:?} must be a string")),
        None => fault(at, format!("no {key:?}")),
    }
}

fn edit_from_json(json: &Json, cp: Codepage, at: &str) -> Result<Edit, WireError> {
    let Json::Object(o) = json else { return fault(at, "expected an object") };
    let op = text(o, "op", at)?;
    let name = text(o, "resource", at)?;
    let Some(key) = ResKey::from_filename(name) else {
        return fault(at, format!("{name:?} is not a resource's name (name.ext)"));
    };
    let field = |what: &str| -> Result<(GffPath, String), WireError> {
        let (path, label) = field_from_str(text(o, what, at)?)
            .map_err(|message| WireError { at: format!("{at}.{what}"), message })?;
        if Label::new(&label).is_err() {
            return fault(&format!("{at}.{what}"), format!("label {label:?} is over 16 bytes"));
        }
        Ok((path, label))
    };
    match op {
        "set_field" => {
            known(o, &["op", "resource", "field", "value"], at)?;
            let (path, label) = field("field")?;
            let Some(value) = o.get("value") else { return fault(at, "no \"value\"") };
            let value =
                mg_gff::value_from_json(value, cp).map_err(|e| gff_fault(at, "value", e))?;
            Ok(Edit::SetField { key, path, label, value: Some(value) })
        }
        "remove_field" => {
            known(o, &["op", "resource", "field"], at)?;
            let (path, label) = field("field")?;
            Ok(Edit::SetField { key, path, label, value: None })
        }
        "insert_item" => {
            known(o, &["op", "resource", "list", "index", "item"], at)?;
            let (path, list) = field("list")?;
            let Some(index) = o.get("index").and_then(Json::as_u64) else {
                return fault(at, "\"index\" must be a position in the list (0 is first)");
            };
            let Ok(index) = usize::try_from(index) else {
                return fault(at, "\"index\" is out of range");
            };
            let Some(item) = o.get("item") else { return fault(at, "no \"item\"") };
            let item = mg_gff::struct_from_json(item, cp).map_err(|e| gff_fault(at, "item", e))?;
            Ok(Edit::InsertItem { key, path, list, index, item })
        }
        "remove_item" => {
            known(o, &["op", "resource", "item"], at)?;
            let at_item = format!("{at}.item");
            let path: GffPath = text(o, "item", at)?
                .parse()
                .map_err(|message| WireError { at: at_item.clone(), message })?;
            let mut steps = path.0;
            match steps.pop() {
                Some(Step::Item(list, index)) => {
                    Ok(Edit::RemoveItem { key, path: GffPath(steps), list, index })
                }
                _ => fault(&at_item, "expected a list's item: /List[index]"),
            }
        }
        "set_resource" => {
            known(o, &["op", "resource", "text", "base64", "gff"], at)?;
            let given = ["text", "base64", "gff"].iter().filter(|k| o.contains_key(**k)).count();
            if given != 1 {
                return fault(at, "give one of \"text\", \"base64\" or \"gff\"");
            }
            let data = if o.contains_key("text") {
                match cp.encode(text(o, "text", at)?) {
                    Some(bytes) => bytes.into_owned(),
                    None => {
                        return fault(
                            &format!("{at}.text"),
                            "text has characters the codepage cannot represent",
                        );
                    }
                }
            } else if o.contains_key("base64") {
                match BASE64.decode(text(o, "base64", at)?) {
                    Ok(bytes) => bytes,
                    Err(e) => return fault(&format!("{at}.base64"), format!("bad base64: {e}")),
                }
            } else {
                let gff: Gff =
                    mg_gff::from_json(&o["gff"], cp).map_err(|e| gff_fault(at, "gff", e))?;
                match gff.to_bytes() {
                    Ok(bytes) => bytes,
                    Err(e) => return fault(&format!("{at}.gff"), e.to_string()),
                }
            };
            Ok(Edit::SetResource { key, data: Some(data) })
        }
        "remove_resource" => {
            known(o, &["op", "resource"], at)?;
            Ok(Edit::SetResource { key, data: None })
        }
        other => fault(
            at,
            format!(
                "unknown op {other:?} (set_field, remove_field, insert_item, remove_item, \
                 set_resource, remove_resource)"
            ),
        ),
    }
}

#[cfg(test)]
mod tests {
    use mg_core::{LocString, ResRef, ResType, StrRef};
    use mg_gff::{Struct, Value};
    use proptest::prelude::*;

    use super::*;

    fn key(name: &str) -> ResKey {
        ResKey::from_filename(name).unwrap()
    }

    #[test]
    fn paths_read_back_what_they_write() {
        let root = GffPath::root();
        for (path, text) in [
            (root.clone(), "/"),
            (root.field("Tag"), "/Tag"),
            (root.item("Creature List", 3).field("Tint"), "/Creature List[3]/Tint"),
            (root.field(""), "/~e"),
            (root.item("", 0), "/~e[0]"),
            (root.field("a/b[1]~"), "/a~1b~21~3~0"),
            (root.item("x]", 12).item("[y", 0), "/x~3[12]/~2y[0]"),
        ] {
            assert_eq!(path.to_string(), text);
            assert_eq!(text.parse::<GffPath>().as_ref(), Ok(&path), "{text}");
        }
        for bad in
            ["", "Tag", "//", "/a//b", "/a[x]", "/a[1", "/a]", "/a~9", "/a~", "/a[1]b", "/[1]"]
        {
            assert!(bad.parse::<GffPath>().is_err(), "{bad:?} should not parse");
        }
        assert_eq!(field_from_str("Tag"), Ok((GffPath::root(), "Tag".into())));
        assert_eq!(
            field_from_str("/ClassList[0]/ClassLevel"),
            Ok((GffPath::root().item("ClassList", 0), "ClassLevel".into()))
        );
        assert!(field_from_str("/ClassList[0]").is_err());
        assert!(field_from_str("/").is_err());
    }

    fn sample() -> Command {
        let mut item = Struct::new(4);
        item.set("Tag", Value::String(b"GOBLIN".to_vec()));
        item.set("XPosition", Value::Float(1.5));
        let mut name = LocString::from_strref(StrRef(77));
        name.strings.push((mg_core::LocStringKey(0), b"Caf\xe9".to_vec()));
        Command::new(
            "Sample",
            vec![
                Edit::SetField {
                    key: key("bandit.utc"),
                    path: GffPath::root(),
                    label: "FirstName".into(),
                    value: Some(Value::LocString(name)),
                },
                Edit::SetField {
                    key: key("bandit.utc"),
                    path: GffPath::root().item("ClassList", 0),
                    label: "ClassLevel".into(),
                    value: Some(Value::Short(5)),
                },
                Edit::SetField {
                    key: key("bandit.utc"),
                    path: GffPath::root(),
                    label: "Comment".into(),
                    value: None,
                },
                Edit::InsertItem {
                    key: key("area.git"),
                    path: GffPath::root(),
                    list: "Creature List".into(),
                    index: 2,
                    item,
                },
                Edit::RemoveItem {
                    key: key("area.git"),
                    path: GffPath::root().field("Deep"),
                    list: "List".into(),
                    index: 7,
                },
                Edit::SetResource {
                    key: key("hello.nss"),
                    data: Some(b"void main() { }\r\n// caf\xe9\n".to_vec()),
                },
                Edit::SetResource { key: key("blob.tga"), data: Some(vec![0, 1, 2, 255]) },
                Edit::SetResource { key: key("old.utc"), data: None },
            ],
        )
    }

    #[test]
    fn a_command_reads_back_what_it_writes() {
        let cmd = sample();
        let json = command_to_json(&cmd, Codepage::WINDOWS_1252).unwrap();
        // Through text, as a file holds it.
        let json: Json =
            serde_json::from_str(&serde_json::to_string_pretty(&json).unwrap()).unwrap();
        assert_eq!(command_from_json(&json, Codepage::WINDOWS_1252).unwrap(), cmd);
        // As documented.
        let edits = json["edits"].as_array().unwrap();
        assert_eq!(json["version"], 1);
        assert_eq!(edits[1]["field"], "/ClassList[0]/ClassLevel");
        assert_eq!(edits[1]["value"], json!({"type": "short", "value": 5}));
        assert_eq!(edits[2]["op"], "remove_field");
        assert_eq!(edits[3]["list"], "/Creature List");
        assert_eq!(edits[4]["item"], "/Deep/List[7]");
        // Text that reads back to the same bytes is written as text.
        assert_eq!(edits[5]["text"], "void main() { }\r\n// café\n");
        assert_eq!(edits[6]["base64"], "AAEC/w==");
        assert_eq!(edits[7], json!({"op": "remove_resource", "resource": "old.utc"}));
    }

    #[test]
    fn a_resource_is_given_as_text_bytes_or_fields() {
        let cp = Codepage::WINDOWS_1252;
        let mut gff = Gff::new(*b"UTC ");
        gff.root.set("Tag", Value::String(b"X".to_vec()));
        let doc = mg_gff::to_json(&gff, cp).unwrap();
        let cmd = command_from_json(
            &json!({"edits": [{"op": "set_resource", "resource": "x.utc", "gff": doc}]}),
            cp,
        )
        .unwrap();
        assert_eq!(cmd.label, "Apply edits");
        let Edit::SetResource { data: Some(data), .. } = &cmd.edits[0] else { panic!() };
        assert_eq!(Gff::read(data).unwrap(), gff);
    }

    #[test]
    fn faults_say_where() {
        let cp = Codepage::WINDOWS_1252;
        let fails = |json: Json| command_from_json(&json, cp).unwrap_err().to_string();
        assert_eq!(fails(json!([])), ": expected an object with \"edits\"");
        assert_eq!(fails(json!({"edits": 3})), "edits: expected a list of edits");
        assert!(fails(json!({"version": 2, "edits": []})).starts_with("version: 2 is not"));
        assert!(fails(json!({"edit": []})).contains("unknown key \"edit\""));
        let one = |edit: Json| fails(json!({"edits": [edit]}));
        assert_eq!(one(json!({"resource": "a.utc"})), "edits[0]: no \"op\"");
        assert!(one(json!({"op": "set_field", "resource": "a"})).contains("not a resource's name"));
        assert!(one(json!({"op": "fly", "resource": "a.utc"})).contains("unknown op \"fly\""));
        assert_eq!(
            one(json!({"op": "set_field", "resource": "a.utc", "field": "/A[0]", "value": {}})),
            "edits[0].field: \"/A[0]\" is a list's item, not a field"
        );
        assert_eq!(
            one(json!({"op": "set_field", "resource": "a.utc", "field": "Tag",
                "value": {"type": "byte", "value": 300}})),
            "edits[0].value: : 300 out of range"
        );
        assert!(
            one(json!({"op": "set_field", "resource": "a.utc", "field": "ALabelOfSeventeen",
                "value": {"type": "byte", "value": 1}}))
            .contains("over 16 bytes")
        );
        assert!(
            one(json!({"op": "set_field", "resource": "a.utc", "feild": "Tag"}))
                .contains("unknown key \"feild\"")
        );
        assert!(
            one(json!({"op": "remove_item", "resource": "a.git", "item": "/List"}))
                .contains("expected a list's item")
        );
        assert!(
            one(json!({"op": "insert_item", "resource": "a.git", "list": "/L", "index": -1,
                "item": {}}))
            .contains("\"index\" must be")
        );
        assert!(
            one(json!({"op": "set_resource", "resource": "a.nss", "text": "a", "base64": "YQ=="}))
                .contains("give one of")
        );
        assert!(
            one(json!({"op": "set_resource", "resource": "a.nss", "base64": "!"}))
                .contains("bad base64")
        );
    }

    fn label() -> impl Strategy<Value = String> {
        // Labels as files have them, and the characters paths give meaning.
        prop_oneof![
            "[A-Za-z0-9_ ]{0,16}",
            "[/~\\[\\]a0e]{0,8}",
            any::<String>().prop_map(|s| s.chars().take(8).collect()),
        ]
    }

    fn step() -> impl Strategy<Value = Step> {
        prop_oneof![
            label().prop_map(Step::Field),
            (label(), 0usize..100_000).prop_map(|(l, i)| Step::Item(l, i)),
        ]
    }

    fn any_json() -> impl Strategy<Value = Json> {
        let leaf = prop_oneof![
            Just(Json::Null),
            any::<bool>().prop_map(Json::from),
            any::<i64>().prop_map(Json::from),
            any::<f64>().prop_map(|f| json!(f)),
            "[a-z_/\\[\\]~0-9 .]{0,12}".prop_map(Json::from),
            prop::sample::select(vec![
                "op",
                "resource",
                "field",
                "value",
                "list",
                "index",
                "item",
                "text",
                "base64",
                "gff",
                "set_field",
                "remove_field",
                "insert_item",
                "remove_item",
                "set_resource",
                "remove_resource",
                "a.utc",
                "type",
                "byte",
                "list",
                "struct",
                "cexolocstring",
                "__struct_id",
                "__data_type",
            ])
            .prop_map(Json::from),
        ];
        leaf.prop_recursive(5, 48, 6, |inner| {
            prop_oneof![
                prop::collection::vec(inner.clone(), 0..6).prop_map(Json::Array),
                prop::collection::vec(
                    (
                        prop::sample::select(vec![
                            "op",
                            "resource",
                            "field",
                            "value",
                            "list",
                            "index",
                            "item",
                            "text",
                            "base64",
                            "gff",
                            "edits",
                            "label",
                            "version",
                            "type",
                            "__struct_id",
                            "__data_type",
                            "id",
                            "0",
                        ]),
                        inner
                    ),
                    0..6
                )
                .prop_map(|pairs| {
                    Json::Object(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
                }),
            ]
        })
    }

    proptest! {
        #[test]
        fn any_path_reads_back(steps in prop::collection::vec(step(), 0..6)) {
            let path = GffPath(steps);
            prop_assert_eq!(path.to_string().parse::<GffPath>(), Ok(path));
        }

        #[test]
        fn text_that_is_no_path_is_an_error_not_a_panic(text in "[/~\\[\\]a-c0-9 ]{0,20}") {
            let _ = text.parse::<GffPath>();
            let _ = field_from_str(&text);
        }

        #[test]
        fn json_that_is_no_command_is_an_error_not_a_panic(json in any_json()) {
            let _ = command_from_json(&json, Codepage::WINDOWS_1252);
            let _ = command_from_json(&json!({"edits": [json.clone()]}), Codepage::WINDOWS_1252);
            let _ = command_from_json(&json!({"edits": json}), Codepage::WINDOWS_1252);
        }
    }

    #[test]
    fn resource_names_keep_their_type() {
        let k = ResKey::new(ResRef::from_str("Area001").unwrap(), ResType::GIT);
        assert_eq!(ResKey::from_filename(&k.to_string()), Some(key("area001.git")));
    }
}
