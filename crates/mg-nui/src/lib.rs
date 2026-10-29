//! NUI windows as the NUI Creator keeps them, and the scripts built from
//! them.
//!
//! A window is the game's own document: a JUI resource, the JSON that
//! `NuiCreate` takes ([`parse`], [`window`], [`template`]). What the Creator
//! keeps beside it (the binds' first values, the handlers, the swap layout
//! variants) is a TXT resource of the same name ([`Settings`]); nothing of
//! Moonglow's is written into the JUI. [`validate`] checks both against the
//! shapes `nw_inc_nui` builds and what the client is known to refuse, and
//! [`generate`] writes the two scripts of a window: the include that opens
//! it (`NAME_o`) and its event script (`NAME_e`), whose sections keep the
//! code written in them ([`merge_events`]). [`rename()`] moves a window with
//! all of that to another name.
//!
//! Editing is lossless where the crate decides: fields it does not know
//! stay in the JUI and in every level of the settings, in the order read.

use std::collections::{BTreeMap, BTreeSet};

use mg_core::ResType;
use mg_edit::{Command, Edit};
use mg_module::Module;
use mg_resman::ResKey;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

mod actions;
mod error;
pub mod import;
pub mod properties;
mod rename;
mod schema;
mod script;
mod validate;

pub use actions::{
    Action, BEGIN, END, EVENT_TYPES, Handler, Refit, edited_outside, event_function, event_source,
    fit_returns, handler_code, handler_key, handler_offset, initial_function, insert_code,
    merge_events, rename_in_code, string_expr, variant_function, window_id_function,
};
pub use error::NuiError;
pub use rename::{Renamed, build_renamed, can_rename, rename};
pub use schema::{controls, jui_schema, settings_schema};
pub use script::{Build, Generated, generate, is_current, opener_source, rebuild_events};
pub use validate::{Diagnostic, Severity, plain_name, validate};

/// The `format` of a settings resource this version reads and writes.
pub const FORMAT: &str = "moonglow.nui/1";

/// A bind as the settings keep it: the value the opener sets first, and
/// whether the window watches it (a change is an event).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Binding {
    pub value: Value,
    #[serde(default)]
    pub watch: bool,
    /// Fields this version does not know, kept as read.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// What the Creator keeps of a window beside its JUI, in a TXT resource of
/// the same name: nothing of it goes into the game's document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// [`FORMAT`]; a TXT with another is not a window's settings.
    pub format: String,
    /// The binds by name.
    #[serde(default)]
    pub bindings: BTreeMap<String, Binding>,
    /// The opener loads the JUI on the player's client by its name
    /// (`NuiCreateFromResRef`) rather than sending the layout.
    #[serde(default)]
    pub from_resref: bool,
    /// The window's events go to the module's own event script (its
    /// OnNuiEvent, a dispatcher of the builder's) rather than to one of
    /// the window's: Build writes `NAME_e` as an include whose
    /// `NAME_OnEvent()` handles them, for that script to call. Not written
    /// while off, so settings from before it read and write as they did.
    #[serde(default, skip_serializing_if = "is_false")]
    pub event_function: bool,
    /// The opener as Build last wrote it, as a number ([`generate`]): one
    /// that differs was changed by hand, and is not written over.
    #[serde(default)]
    pub opener_hash: Option<String>,
    /// The event script as Build last compiled it, likewise: one that
    /// differs is to be built again ([`is_current`]).
    #[serde(default)]
    pub event_hash: Option<String>,
    /// The swap layout variants by name: each a layout a handler can show
    /// in a group in place of what it holds (`views` in the file).
    #[serde(default, rename = "views")]
    pub variants: BTreeMap<String, Value>,
    /// The handlers, in the order of the event script (`actions` in the
    /// file).
    #[serde(default, rename = "actions")]
    pub handlers: Vec<Handler>,
    /// The window's ID in the game, when it is not the window's name.
    #[serde(default)]
    pub window_id: Option<String>,
    /// The group each variant was made for, by the variant's name
    /// (`_window_`: the whole window; `view_targets` in the file).
    #[serde(default, rename = "view_targets", skip_serializing_if = "BTreeMap::is_empty")]
    pub variant_targets: BTreeMap<String, String>,
    /// The IDs of the groups made as swap layouts, in the order made.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub swap_hosts: Vec<String>,
    /// Fields this version does not know, kept as read.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            format: FORMAT.into(),
            bindings: BTreeMap::new(),
            from_resref: false,
            event_function: false,
            opener_hash: None,
            event_hash: None,
            variants: BTreeMap::new(),
            handlers: Vec::new(),
            window_id: None,
            variant_targets: BTreeMap::new(),
            swap_hosts: Vec::new(),
            extra: BTreeMap::new(),
        }
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl Settings {
    /// Reads a settings resource: JSON without a key twice, of this
    /// [`FORMAT`]. Fields it does not know are kept, at every level; a
    /// handler whose action it does not know is kept as [`Action::Other`].
    pub fn parse(bytes: &[u8]) -> Result<Self, NuiError> {
        let settings: Self =
            serde_json::from_value(parse(bytes)?).map_err(|e| NuiError::Settings(e.to_string()))?;
        if settings.format != FORMAT {
            return Err(NuiError::Settings(
                "This TXT is not a supported Moonglow NUI settings resource".into(),
            ));
        }
        Ok(settings)
    }

    /// The settings as the resource's bytes.
    pub fn bytes(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(self).expect("JSON settings are serializable")
    }
}

/// Reads JSON (a JUI, or any of the Creator's resources), keeping the order
/// of every object's keys. A key twice in one object is an error rather
/// than one value silently lost; the bytes stay with the caller, so a
/// document that cannot be read can still be edited as text.
pub fn parse(bytes: &[u8]) -> Result<Value, NuiError> {
    struct Unique(Value);
    impl<'de> Deserialize<'de> for Unique {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            struct Visitor;
            impl<'de> serde::de::Visitor<'de> for Visitor {
                type Value = Unique;
                fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.write_str("JSON without duplicate object keys")
                }
                fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Unique, E> {
                    Ok(Unique(v.into()))
                }
                fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Unique, E> {
                    Ok(Unique(v.into()))
                }
                fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Unique, E> {
                    Ok(Unique(v.into()))
                }
                fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Unique, E> {
                    Ok(Unique(json!(v)))
                }
                fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Unique, E> {
                    Ok(Unique(v.into()))
                }
                fn visit_unit<E: serde::de::Error>(self) -> Result<Unique, E> {
                    Ok(Unique(Value::Null))
                }
                fn visit_seq<A: serde::de::SeqAccess<'de>>(
                    self,
                    mut a: A,
                ) -> Result<Unique, A::Error> {
                    let mut out = Vec::new();
                    while let Some(Unique(v)) = a.next_element()? {
                        out.push(v);
                    }
                    Ok(Unique(Value::Array(out)))
                }
                fn visit_map<A: serde::de::MapAccess<'de>>(
                    self,
                    mut a: A,
                ) -> Result<Unique, A::Error> {
                    let mut out = serde_json::Map::new();
                    while let Some((k, Unique(v))) = a.next_entry::<String, Unique>()? {
                        if out.contains_key(&k) {
                            return Err(serde::de::Error::custom(format!("duplicate key: {k}")));
                        }
                        out.insert(k, v);
                    }
                    Ok(Unique(Value::Object(out)))
                }
            }
            d.deserialize_any(Visitor)
        }
    }
    serde_json::from_slice::<Unique>(bytes).map(|v| v.0).map_err(|e| NuiError::Json(e.to_string()))
}

/// Whether `name` can be a window's: a ResRef (16 bytes, lower case here)
/// with two bytes left for the `_o` and `_e` of its scripts. A name is
/// never shortened or changed to fit.
pub fn check_name(name: &str) -> Result<(), NuiError> {
    let fits = !name.is_empty()
        && name.len() <= 14
        && name.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_');
    if fits { Ok(()) } else { Err(NuiError::Name { name: name.into() }) }
}

/// The resources of the window `name`: what [`create`] makes and
/// [`generate`] writes.
pub(crate) struct Resources {
    /// `NAME.jui`: the layout.
    pub layout: ResKey,
    /// `NAME.txt`: the settings.
    pub settings: ResKey,
    /// `NAME_o`: the include that opens the window.
    pub opener: String,
    /// `NAME_e`: the event script.
    pub events: String,
}

impl Resources {
    /// Those of a name [`check_name`] takes.
    pub(crate) fn of(name: &str) -> Result<Resources, NuiError> {
        check_name(name)?;
        Ok(Resources {
            layout: key(name, ResType::JUI)?,
            settings: key(name, ResType::TXT)?,
            opener: format!("{name}_o"),
            events: format!("{name}_e"),
        })
    }

    pub(crate) fn opener(&self, ty: ResType) -> ResKey {
        key(&self.opener, ty).expect("a checked name with _o is a ResRef")
    }

    pub(crate) fn events(&self, ty: ResType) -> ResKey {
        key(&self.events, ty).expect("a checked name with _e is a ResRef")
    }
}

/// The key of a resource of a window; an error for a name no resource has.
fn key(name: &str, ty: ResType) -> Result<ResKey, NuiError> {
    ResKey::parse(name, ty).ok_or_else(|| NuiError::Name { name: name.into() })
}

/// A new window `name` as one command: its layout ([`window`]) and its
/// settings, with a handler that closes it from its Close button. Refused
/// when the module has any resource the window or its scripts would take.
pub fn create(module: &Module, name: &str) -> Result<Command, NuiError> {
    let r = Resources::of(name)?;
    let taken = [
        r.layout,
        r.settings,
        r.opener(ResType::NSS),
        r.opener(ResType::NCS),
        r.events(ResType::NSS),
        r.events(ResType::NCS),
    ];
    if let Some(k) = taken.into_iter().find(|k| module.contains(k)) {
        return Err(NuiError::Exists(k));
    }
    let settings = Settings {
        handlers: vec![Handler::new("click", "mg_close", Action::Close)],
        ..Default::default()
    };
    Ok(Command::new(
        "Create NUI window",
        vec![
            Edit::SetResource {
                key: r.layout,
                data: Some(serde_json::to_vec_pretty(&window()).expect("JSON is serializable")),
            },
            Edit::SetResource { key: r.settings, data: Some(settings.bytes()) },
        ],
    ))
}

/// A new window's layout: a title, a label and a Close button (`mg_close`).
pub fn window() -> Value {
    let mut close = template("button");
    close["id"] = "mg_close".into();
    close["label"] = "Close".into();
    json!({"version":1,"title":"New window","geometry":{"x":-1.0,"y":-1.0,"w":420.0,"h":240.0},
        "resizable":true,"collapsed":null,"closable":true,"transparent":false,"border":true,"accepts_input":true,
        "size_constraint":null,"edge_constraint":null,"font":"",
        "root":{"type":"col","label":null,"value":null,"children":[template("label"),close]}})
}

/// The widgets of `nw_inc_nui` by the `type` their JSON carries: what
/// [`template`] makes and [`validate`] knows the shape of.
pub const ELEMENTS: &[&str] = &[
    "col",
    "row",
    "group",
    "spacer",
    "label",
    "text",
    "button",
    "button_image",
    "button_select",
    "check",
    "image",
    "combo",
    "slider",
    "sliderf",
    "progress",
    "textedit",
    "list",
    "color_picker",
    "options",
    "tabbar",
    "chart",
];

/// A new widget of type `ty` with the fields its `nw_inc_nui` constructor
/// writes, filled with values of our own (no game text or art); for a type
/// not in [`ELEMENTS`], the three fields every element has.
pub fn template(ty: &str) -> Value {
    let mut v = json!({"type":ty,"label":null,"value":null});
    match ty {
        "col" | "row" => v["children"] = json!([]),
        "group" => {
            v["children"] = json!([template("col")]);
            v["border"] = true.into();
            v["scrollbars"] = 4.into();
        }
        "label" => {
            v["value"] = "Label".into();
            v["text_halign"] = 0.into();
            v["text_valign"] = 0.into();
        }
        "text" => {
            v["value"] = "Text".into();
            v["border"] = true.into();
            v["scrollbars"] = 4.into();
        }
        "button" | "button_image" => {
            v["label"] = if ty == "button" { "Button" } else { "" }.into();
        }
        "check" | "button_select" => {
            v["label"] = "Option".into();
            v["value"] = false.into();
        }
        "image" => {
            v["value"] = "".into();
            v["image_aspect"] = 0.into();
            v["image_halign"] = 0.into();
            v["image_valign"] = 0.into();
        }
        "combo" | "options" | "tabbar" => {
            v["value"] = 0.into();
            v["elements"] = if ty == "combo" {
                json!([["First", 0], ["Second", 1]])
            } else {
                json!(["First", "Second"])
            };
            if ty != "combo" {
                v["direction"] = 0.into();
            }
        }
        "slider" => {
            v["value"] = 0.into();
            v["min"] = 0.into();
            v["max"] = 100.into();
            v["step"] = 1.into();
        }
        "sliderf" => {
            v["value"] = json!(0.0);
            v["min"] = json!(0.0);
            v["max"] = json!(1.0);
            v["step"] = json!(0.01);
        }
        "progress" => v["value"] = json!(0.5),
        "textedit" => {
            v["label"] = "Enter text".into();
            v["value"] = "".into();
            v["max"] = 255.into();
            v["multiline"] = false.into();
            v["wordwrap"] = true.into();
        }
        "list" => {
            v["row_template"] = json!([]);
            v["row_count"] = 1.into();
            v["row_height"] = json!(25.0);
            v["border"] = true.into();
            v["scrollbars"] = 2.into();
        }
        "color_picker" => v["value"] = json!({"r":255,"g":255,"b":255,"a":255}),
        "chart" => {
            v["value"] = json!([{ "type":0,"legend":"Values","color":{"r":100,"g":160,"b":255,"a":255},"data":[0.0,0.5,1.0]}])
        }
        _ => {}
    }
    if !matches!(ty, "col" | "row" | "group" | "spacer") {
        v["height"] = json!(if ty == "list" { 150.0 } else { 30.0 });
    }
    v
}

/// Calls `f` with `v` and everything in it, objects and arrays alike:
/// names are found wherever a document has them, in fields this version
/// knows or not.
fn walk<'a>(v: &'a Value, f: &mut impl FnMut(&'a Value)) {
    f(v);
    match v {
        Value::Object(o) => o.values().for_each(|v| walk(v, f)),
        Value::Array(a) => a.iter().for_each(|v| walk(v, f)),
        _ => {}
    }
}

/// The names a document uses, found in one pass over it.
#[derive(Debug, Default)]
pub(crate) struct Names {
    pub binds: BTreeSet<String>,
    pub ids: BTreeSet<String>,
    pub groups: BTreeSet<String>,
}

impl Names {
    pub(crate) fn of(v: &Value) -> Names {
        let mut names = Names::default();
        walk(v, &mut |v| {
            if let Some(bind) = bind_of(v) {
                names.binds.insert(bind.into());
            }
            if let Some(id) = id_of(v) {
                names.ids.insert(id.into());
                if v["type"] == "group" {
                    names.groups.insert(id.into());
                }
            }
        });
        names
    }
}

/// The bind a value is read from: `{"bind": NAME}`.
fn bind_of(v: &Value) -> Option<&str> {
    v.get("bind").and_then(Value::as_str)
}

/// An element's ID.
fn id_of(v: &Value) -> Option<&str> {
    v.as_object()?.get("id").and_then(Value::as_str)
}

/// The binds `value` reads, anywhere in it.
pub fn bind_names(value: &Value) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    walk(value, &mut |v| found.extend(bind_of(v).map(str::to_owned)));
    found
}

/// The IDs of the elements in `v`, anywhere in it: what an event names.
pub fn element_ids(v: &Value) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    walk(v, &mut |v| ids.extend(id_of(v).map(str::to_owned)));
    ids
}

/// The IDs of the groups in `v`: where a swap layout variant can be shown.
pub fn group_ids(v: &Value) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    walk(v, &mut |v| {
        if v["type"] == "group" {
            ids.extend(id_of(v).map(str::to_owned));
        }
    });
    ids
}

/// The pictures a window shows, lower case: those named in its layout and
/// its swap layout variants, and those its binds name at first.
pub fn image_names(window: &Value, settings: &Settings) -> BTreeSet<String> {
    fn add(v: &Value, s: &Settings, out: &mut BTreeSet<String>) {
        let v = v["bind"].as_str().and_then(|n| s.bindings.get(n)).map_or(v, |b| &b.value);
        match v {
            Value::String(n) if !n.is_empty() => {
                out.insert(n.to_lowercase());
            }
            Value::Array(a) => {
                for v in a {
                    add(v, s, out)
                }
            }
            _ => {}
        }
    }
    fn visit(v: &Value, s: &Settings, out: &mut BTreeSet<String>) {
        match v["type"].as_str() {
            Some("image") => add(&v["value"], s, out),
            Some("button_image") => add(&v["label"], s, out),
            _ => {}
        }
        if v.get("image").is_some() {
            add(&v["image"], s, out);
        }
        match v {
            Value::Object(o) => {
                for v in o.values() {
                    visit(v, s, out)
                }
            }
            Value::Array(a) => {
                for v in a {
                    visit(v, s, out)
                }
            }
            _ => {}
        }
    }
    let mut out = BTreeSet::new();
    visit(window, settings, &mut out);
    for view in settings.variants.values() {
        visit(view, settings, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use mg_edit::Workspace;
    use mg_module::ModuleLocation;

    use super::*;

    fn key(name: &str, ty: ResType) -> ResKey {
        ResKey::parse(name, ty).unwrap()
    }

    /// A module with one new window, `test_nui`.
    fn project() -> Workspace {
        let mut ws = Workspace::new(Module::new());
        ws.apply(create(&ws.module, "test_nui").unwrap()).unwrap();
        ws
    }

    #[test]
    fn a_name_is_a_resref_with_room_for_the_scripts_suffixes() {
        for name in ["", "longer_than_fourteen", "Upper", "a/b", "ą", "a\0"] {
            assert_eq!(check_name(name), Err(NuiError::Name { name: name.into() }), "{name}");
        }
        assert!(check_name("abcdefghijklm1").is_ok());
    }

    #[test]
    fn a_refused_name_is_named_in_the_message() {
        let said = check_name("a_name_of_15_ch").unwrap_err().to_string();
        assert!(said.contains("a_name_of_15_ch") && said.contains("1–14 lowercase"), "{said}");
    }

    #[test]
    fn create_refuses_a_name_whose_resources_the_module_has() {
        for ty in [ResType::JUI, ResType::TXT, ResType::NSS, ResType::NCS] {
            let mut m = Module::new();
            let name =
                if matches!(ty, ResType::NSS | ResType::NCS) { "test_nui_e" } else { "test_nui" };
            m.set(key(name, ty), vec![1, 2, 3]);
            assert_eq!(create(&m, "test_nui").unwrap_err(), NuiError::Exists(key(name, ty)));
            assert_eq!(m.get(&key(name, ty)).unwrap(), [1, 2, 3]);
        }
    }

    #[test]
    fn a_new_window_has_a_handler_that_closes_it() {
        let ws = project();
        let made = Settings::parse(ws.module.get(&key("test_nui", ResType::TXT)).unwrap()).unwrap();
        assert_eq!(made.handlers, vec![Handler::new("click", "mg_close", Action::Close)]);
        let layout = parse(ws.module.get(&key("test_nui", ResType::JUI)).unwrap()).unwrap();
        assert!(element_ids(&layout).contains("mg_close"));
    }

    #[test]
    fn fields_the_crate_does_not_know_stay_in_a_layout() {
        let raw = br#"{"version":1,"root":{"type":"future","payload":[2.5,{"a":1}]},"vendor":{"x":true}}"#;
        let parsed = parse(raw).unwrap();
        assert_eq!(parsed["vendor"], json!({"x":true}));
        assert_eq!(parsed["root"]["payload"], json!([2.5, {"a":1}]));
    }

    #[test]
    fn an_untouched_layout_comes_back_with_every_field_in_its_order() {
        // Keys out of alphabetical order, unknown ones among them, nested.
        let raw = r#"{"zeta":1,"version":1,"alpha":{"y":[3,{"q":null,"b":"ż"}],"x":2.5},"root":{"type":"col","value":null,"label":null,"children":[{"type":"label","value":"a","mine":{"k":[true,false]},"id":"l"}]},"title":"T"}"#;
        let parsed = parse(raw.as_bytes()).unwrap();
        assert_eq!(serde_json::to_string(&parsed).unwrap(), raw);
        // The layout a new window gets, written as the Creator writes it.
        let made = serde_json::to_vec_pretty(&window()).unwrap();
        assert_eq!(serde_json::to_vec_pretty(&parse(&made).unwrap()).unwrap(), made);
    }

    #[test]
    fn a_key_twice_in_one_object_is_an_error() {
        for bad in [br#"{"a":1,"a":2}"#.as_slice(), br#"{"root":{"x":1,"x":2}}"#] {
            assert!(matches!(parse(bad), Err(NuiError::Json(m)) if m.contains("duplicate key")));
        }
    }

    #[test]
    fn text_that_is_not_json_is_an_error() {
        for bad in [b"[] junk".as_slice(), &[0xff], b"{", b"1e999", b"", b"\0\0\0\0", b"nul"] {
            assert!(matches!(parse(bad), Err(NuiError::Json(_))), "{bad:?}");
        }
    }

    #[test]
    fn settings_of_another_format_are_refused() {
        let other = Settings::parse(br#"{"format":"another-author"}"#).unwrap_err();
        assert!(matches!(&other, NuiError::Settings(m) if m.contains("not a supported")));
    }

    #[test]
    fn garbage_and_truncated_settings_are_errors() {
        let mut s = Settings::default();
        s.bindings.insert("ż".into(), Binding { value: json!([1, "two"]), ..Default::default() });
        s.handlers.push(Handler::new(
            "click",
            "ok",
            Action::Set { bind: "ż".into(), value: json!(1) },
        ));
        s.variants.insert("v".into(), template("col"));
        let whole = s.bytes();
        assert_eq!(Settings::parse(&whole).unwrap(), s);
        for end in 0..whole.len() - 1 {
            assert!(Settings::parse(&whole[..end]).is_err(), "cut at {end}");
        }
        for bad in [
            b"".as_slice(),
            b"unrelated text",
            &[0xff, 0xfe, 0x00],
            b"[]",
            b"null",
            b"{}",
            br#"{"format":7}"#,
            br#"{"format":"moonglow.nui/1","bindings":[]}"#,
            br#"{"format":"moonglow.nui/1","bindings":{"x":3}}"#,
            br#"{"format":"moonglow.nui/1","actions":{}}"#,
            br#"{"format":"moonglow.nui/1","actions":[7]}"#,
            br#"{"format":"moonglow.nui/1","actions":[{"event":"click"}]}"#,
            br#"{"format":"moonglow.nui/1","actions":[{"event":"click","element":"a"}]}"#,
            br#"{"format":"moonglow.nui/1","actions":[{"event":"click","element":"a","action":"toggle"}]}"#,
            br#"{"format":"moonglow.nui/1","view_targets":[]}"#,
            br#"{"format":"moonglow.nui/1","swap_hosts":{"a":1}}"#,
            br#"{"format":"moonglow.nui/1","format":"moonglow.nui/1"}"#,
        ] {
            assert!(Settings::parse(bad).is_err(), "{}", String::from_utf8_lossy(bad));
        }
    }

    #[test]
    fn fields_the_crate_does_not_know_come_back_at_every_level_of_the_settings() {
        let raw = json!({
            "format": FORMAT,
            "later": {"keep": [1, 2]},
            "bindings": {"x": {"value": true, "watch": true, "custom": {"keep": 3}}},
            "actions": [
                {"event": "click", "element": "a", "action": "close", "note": "mine", "bind": 4},
                {"event": "click", "element": "b", "action": "set", "bind": "x", "value": false,
                 "when": {"keep": null}},
                {"event": "click", "element": "c", "action": "teleport", "to": "elsewhere"},
                {"event": "open", "element": "", "action": {"kind": "later"}}
            ],
            "views": {"v": {"type": "col", "children": [], "unknown": 1}}
        });
        let parsed = Settings::parse(raw.to_string().as_bytes()).unwrap();
        assert_eq!(parsed.handlers[0].action, Action::Close);
        assert_eq!(parsed.handlers[2].action, Action::Other(json!("teleport")));
        assert_eq!(parsed.handlers[3].action, Action::Other(json!({"kind": "later"})));
        let written: Value = serde_json::from_slice(&parsed.bytes()).unwrap();
        for at in [
            "/later",
            "/bindings/x",
            "/actions/0",
            "/actions/1",
            "/actions/2",
            "/actions/3",
            "/views/v",
        ] {
            assert_eq!(written.pointer(at), raw.pointer(at), "{at}");
        }
        assert_eq!(Settings::parse(&parsed.bytes()).unwrap(), parsed);
    }

    #[test]
    fn swap_layouts_are_read_from_settings_written_before_they_were_fields() {
        let raw = br#"{"format":"moonglow.nui/1","swap_hosts":["pages","side"],"view_targets":{"details":"pages","cover":"_window_"}}"#;
        let parsed = Settings::parse(raw).unwrap();
        assert_eq!(parsed.swap_hosts, ["pages", "side"]);
        assert_eq!(parsed.variant_targets["cover"], "_window_");
        assert!(parsed.extra.is_empty());
        let written: Value = serde_json::from_slice(&parsed.bytes()).unwrap();
        assert_eq!(written["swap_hosts"], json!(["pages", "side"]));
        assert_eq!(written["view_targets"], json!({"cover": "_window_", "details": "pages"}));
    }

    #[test]
    fn settings_without_swap_layouts_write_no_fields_for_them() {
        let written: Value = serde_json::from_slice(&Settings::default().bytes()).unwrap();
        assert!(written.get("swap_hosts").is_none() && written.get("view_targets").is_none());
    }

    #[test]
    fn a_draft_that_is_not_json_is_kept_as_typed_and_undone_exactly() {
        let mut ws = project();
        let jui = key("test_nui", ResType::JUI);
        let raw = b"{\"version\":1,\n\"unknown\":false,\"unfinished\":".to_vec();
        ws.apply(Command::new(
            "draft",
            vec![Edit::SetResource { key: jui, data: Some(raw.clone()) }],
        ))
        .unwrap();
        ws.undo().unwrap();
        assert!(parse(ws.module.get(&jui).unwrap()).is_ok());
        ws.redo().unwrap();
        assert_eq!(ws.module.get(&jui).unwrap(), raw);
    }

    #[test]
    fn a_windows_resources_come_back_from_a_saved_module() {
        let mut ws = project();
        let dir = mg_testkit::scratch_dir("nui-roundtrip");
        let path = dir.join("fixture.mod");
        ws.module.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
        let reopened = Module::open(&path).unwrap();
        assert_eq!(reopened.keys().count(), 2);
        for k in ws.module.keys() {
            assert_eq!(reopened.get(k), ws.module.get(k));
        }
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn the_names_of_a_document_are_found_in_fields_the_crate_does_not_know_too() {
        let doc = json!({"root": {"type": "col", "children": [
            {"type": "group", "id": "pages", "children": [{"type": "label", "id": "l",
                "value": {"bind": "caption"}}]},
            {"type": "future", "inside": [{"id": "deep", "x": {"bind": "far"}}]}
        ]}});
        let names = Names::of(&doc);
        assert_eq!(names.binds, bind_names(&doc));
        assert_eq!(names.ids, element_ids(&doc));
        assert_eq!(names.groups, group_ids(&doc));
        assert_eq!(names.binds.iter().collect::<Vec<_>>(), ["caption", "far"]);
        assert_eq!(names.ids.iter().collect::<Vec<_>>(), ["deep", "l", "pages"]);
        assert_eq!(names.groups.iter().collect::<Vec<_>>(), ["pages"]);
    }

    #[test]
    fn a_windows_pictures_are_those_it_names_and_those_its_binds_start_with() {
        let mut w = window();
        w["root"]["children"] = json!([
            {"type": "image", "value": "Portrait"},
            {"type": "button_image", "label": {"bind": "icon"}},
            {"type": "spacer", "draw_list": [{"type": 5, "image": "frame"}]}
        ]);
        let mut s = Settings::default();
        s.bindings
            .insert("icon".into(), Binding { value: json!("ir_attack"), ..Default::default() });
        s.variants.insert("v".into(), json!({"type": "image", "value": "other"}));
        let names: Vec<String> = image_names(&w, &s).into_iter().collect();
        assert_eq!(names, ["frame", "ir_attack", "other", "portrait"]);
    }
}
