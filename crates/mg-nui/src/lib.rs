//! Native NUI authoring. JUI is the game's document, not a translated DOM.
//! Unknown fields survive edits. The companion TXT is editor metadata only.
//! Diagnostics describe the nw_inc_nui API contract, not a simulated engine.

use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit};
use mg_module::Module;
use mg_resman::ResKey;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

mod validate;
pub use validate::{Diagnostic, Severity, plain_name, validate};
mod actions;
mod script;
pub use actions::{
    Action, BEGIN, END, EVENT_TYPES, Route, edited_outside, element_ids, event_source, group_ids,
    handler_code, handler_key, handler_offset, initial_function, insert_code, merge_events,
    string_expr, variant_function,
};
pub use script::{generate, is_current, opener_source};

pub const FORMAT: &str = "moonglow.nui/1";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Binding {
    pub value: Value,
    #[serde(default)]
    pub watch: bool,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// One window per JUI; its bind defaults and generation settings are in a
/// standard TXT resource. No private metadata is inserted into the game's JUI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub format: String,
    #[serde(default)]
    pub bindings: BTreeMap<String, Binding>,
    #[serde(default)]
    pub from_resref: bool,
    /// Detect edits to the opener before regenerating it. Not a security hash.
    #[serde(default)]
    pub opener_hash: Option<String>,
    #[serde(default)]
    pub event_hash: Option<String>,
    #[serde(default)]
    pub compiled_event_hash: Option<String>,
    #[serde(default)]
    pub views: BTreeMap<String, Value>,
    #[serde(default)]
    pub actions: Vec<Route>,
    #[serde(default)]
    pub window_id: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            format: FORMAT.into(),
            bindings: BTreeMap::new(),
            from_resref: false,
            opener_hash: None,
            event_hash: None,
            compiled_event_hash: None,
            views: BTreeMap::new(),
            actions: Vec::new(),
            window_id: None,
            extra: BTreeMap::new(),
        }
    }
}

impl Settings {
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        let settings: Self = serde_json::from_value(parse(bytes)?).map_err(|e| e.to_string())?;
        if settings.format != FORMAT {
            return Err("This TXT is not a supported Moonglow NUI settings resource".into());
        }
        Ok(settings)
    }
    pub fn bytes(&self) -> Vec<u8> {
        serde_json::to_vec_pretty(self).expect("JSON settings are serializable")
    }
}

/// Reject duplicate keys instead of silently discarding authored values. The
/// raw bytes remain with the caller, even when structured editing is unavailable.
pub fn parse(bytes: &[u8]) -> Result<Value, String> {
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
    serde_json::from_slice::<Unique>(bytes).map(|v| v.0).map_err(|e| e.to_string())
}

/// ResRefs are case insensitive and 16 bytes. Two bytes are reserved for
/// _o (opener) and _e (events). Never truncate or silently rename resources.
pub fn check_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || name.len() > 14
        || !name.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'_')
    {
        return Err(
            "Use 1–14 lowercase ASCII letters, digits or underscores (room for _o and _e).".into(),
        );
    }
    Ok(())
}

/// Resource key for a previously checked ResRef.
pub fn key(name: &str, ty: ResType) -> ResKey {
    ResKey::new(ResRef::from_str(name).expect("checked resource name"), ty)
}

pub fn create(module: &Module, name: &str) -> Result<Command, String> {
    check_name(name)?;
    let resources = [
        (name.to_owned(), ResType::JUI),
        (name.to_owned(), ResType::TXT),
        (format!("{name}_o"), ResType::NSS),
        (format!("{name}_o"), ResType::NCS),
        (format!("{name}_e"), ResType::NSS),
        (format!("{name}_e"), ResType::NCS),
    ];
    for (name, ty) in resources {
        let k = key(&name, ty);
        if module.contains(&k) {
            return Err(format!("Resource already exists: {k}"));
        }
    }
    let settings = Settings {
        actions: vec![Route {
            event: "click".into(),
            element: "mg_close".into(),
            action: Action::Close,
        }],
        ..Default::default()
    };
    Ok(Command::new(
        "Create NUI window",
        vec![
            Edit::SetResource {
                key: key(name, ResType::JUI),
                data: Some(serde_json::to_vec_pretty(&window()).unwrap()),
            },
            Edit::SetResource { key: key(name, ResType::TXT), data: Some(settings.bytes()) },
        ],
    ))
}

pub fn window() -> Value {
    let mut close = template("button");
    close["id"] = "mg_close".into();
    close["label"] = "Close".into();
    json!({"version":1,"title":"New window","geometry":{"x":-1.0,"y":-1.0,"w":420.0,"h":240.0},
        "resizable":true,"collapsed":null,"closable":true,"transparent":false,"border":true,"accepts_input":true,
        "size_constraint":null,"edge_constraint":null,"font":"",
        "root":{"type":"col","label":null,"value":null,"children":[template("label"),close]}})
}

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

/// Templates follow stock nw_inc_nui.nss. These are author-created examples;
/// no stock artwork or game script source is shipped with the editor.
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

pub fn bind_names(value: &Value) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    fn visit(v: &Value, found: &mut BTreeSet<String>) {
        if let Some(name) = v.get("bind").and_then(Value::as_str) {
            found.insert(name.into());
        }
        match v {
            Value::Object(m) => {
                for v in m.values() {
                    visit(v, found);
                }
            }
            Value::Array(a) => {
                for v in a {
                    visit(v, found);
                }
            }
            _ => {}
        }
    }
    visit(value, &mut found);
    found
}

/// Literal and initially bound artwork used by the main layout and all views.
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
    for view in settings.views.values() {
        visit(view, settings, &mut out);
    }
    out
}
