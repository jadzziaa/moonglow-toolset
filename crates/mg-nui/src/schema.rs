//! The forms of a window's two resources, as a program that writes them
//! needs them: the settings (`NAME.txt`, [`FORMAT`]) as a JSON Schema
//! ([`settings_schema`]), and the controls of a layout as the Creator makes
//! them ([`controls`]). `mg nui schema` prints them. The schema is written
//! here beside [`Settings`] and [`Route`], and its tests hold it to what
//! they read and write: a key added to the settings without its place
//! here fails them.

use serde_json::{Map, Value, json};

use crate::{ELEMENTS, EVENT_TYPES, FORMAT, template, window};

#[cfg(doc)]
use crate::{Route, Settings};

/// What a settings key holds, for the schema: its JSON Schema and what it
/// is for.
fn key(schema: Value, about: &str) -> Value {
    let mut schema = schema;
    schema["description"] = about.into();
    schema
}

/// The settings resource (`NAME.txt`) as a JSON Schema (draft 2020-12):
/// every key [`Settings`] reads and writes, its type, its default (a key
/// left out takes it) and what it does; and each handler's form, by its
/// `action`. Keys it does not know are kept as they are, at every level.
pub fn settings_schema() -> Value {
    let layout = json!({"type": "object", "description":
        "A layout, as the JUI's root is: an element with its `type` (see `mg nui schema --layout`)"});
    let handler = |action: &str, about: &str, fields: Value| {
        let mut properties = json!({
            "event": {"enum": EVENT_TYPES, "description":
                "The event, as NuiGetEventType reads it"},
            "element": {"type": "string", "description":
                "The control's ID; for `watch`, the bind's name; empty for the window's `open` and `close`"},
            "action": {"const": action, "description": about},
        });
        let mut required = vec![json!("event"), json!("element"), json!("action")];
        if let Value::Object(fields) = fields {
            for (k, v) in fields {
                required.push(k.clone().into());
                properties[&k] = v;
            }
        }
        json!({"type": "object", "properties": properties, "required": required})
    };
    let bind = json!({"type": "string", "description": "The bind's name"});
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://github.com/moonglow-toolset/moonglow/nui-settings",
        "title": "Moonglow NUI window settings (NAME.txt)",
        "description": "What the NUI Creator keeps beside a window's layout (NAME.jui): the binds' first values, the handlers, the swap layout variants. Nothing of it goes into the game's document. UTF-8 JSON, no key twice in one object; keys this version does not know are kept.",
        "type": "object",
        "required": ["format"],
        "properties": {
            "format": key(json!({"const": FORMAT}),
                "Marks the file as a window's settings; a TXT with another format is not one"),
            "bindings": key(json!({"type": "object", "default": {},
                "additionalProperties": {"type": "object", "required": ["value"], "properties": {
                    "value": {"description": "The value the opener sets first: a string, number, boolean, color {r,g,b,a}, rectangle {x,y,w,h}, or an array of one of them a list row each"},
                    "watch": {"type": "boolean", "default": false, "description": "The window watches it: a change is a `watch` event (a `watch` handler watches it too)"},
                }}}),
                "The binds by name: every bind the layout reads needs one"),
            "from_resref": key(json!({"type": "boolean", "default": false}),
                "The opener loads the JUI on the player's client by its name (NuiCreateFromResRef) rather than sending the layout: the JUI must reach every client, in a hak"),
            "opener_hash": key(json!({"type": ["string", "null"], "default": null}),
                "Written by Build: NAME_o.nss as it last wrote it, to tell an opener changed by hand. Leave it out of a new file"),
            "event_hash": key(json!({"type": ["string", "null"], "default": null}),
                "Written by Build: NAME_e.nss as it last compiled it. Leave it out of a new file"),
            "views": key(json!({"type": "object", "default": {}, "additionalProperties": layout}),
                "The swap layout variants by name: each a layout a handler can show in a group in place of what it holds"),
            "actions": key(json!({"type": "array", "default": [], "items": {"oneOf": [
                handler("code", "Its section starts with a note where your code goes", json!({})),
                handler("close", "Its section starts by closing the window", json!({})),
                handler("toggle", "Its section starts by switching a bind between true and false (in a list, its row's)", json!({"bind": bind})),
                handler("set", "Its section starts by setting a bind", json!({"bind": bind, "value": {"description": "The value to set it to"}})),
                handler("view", "Its section starts by showing a variant in a group", json!({
                    "group": {"type": "string", "description": "The group's ID; `_window_` for the whole window"},
                    "view": {"type": "string", "description": "The variant's name, a key of `views`"},
                })),
            ]}}),
                "The handlers, in the order of the event script: each an event, whose it is, and the code its section starts with. After that the code in NAME_e.nss is yours"),
            "window_id": key(json!({"type": ["string", "null"], "default": null}),
                "The window's ID in the game (NuiFindWindow), when it is not the window's name"),
            "view_targets": key(json!({"type": "object", "default": {}, "additionalProperties": {"type": "string"}}),
                "The group each variant was made for, by the variant's name (`_window_`: the whole window)"),
            "swap_hosts": key(json!({"type": "array", "default": [], "items": {"type": "string"}}),
                "The IDs of the groups made as swap layouts, in the order made"),
        },
    })
}

/// A layout's elements as the Creator makes them: a new window, and for
/// each `type` the game's `nw_inc_nui` builds, the fields its constructor
/// writes (with values of the Creator's own). The game's `nw_inc_nui.nss`
/// is the reference for what each means.
pub fn controls() -> Value {
    let controls: Map<String, Value> =
        ELEMENTS.iter().map(|ty| ((*ty).to_owned(), template(ty))).collect();
    json!({
        "description": "A JUI is the JSON NuiCreate takes, as nw_inc_nui's functions build it (NuiWindow, NuiCol, NuiLabel...). `window` is a new window; `controls` each type's fields as the NUI Creator adds it. Any element also takes id, width, height, aspect, margin, padding, enabled, visible, encouraged, tooltip, disabled_tooltip, foreground_color, font and draw_list. A property may be a bind ({\"bind\": NAME}) unless nw_inc_nui makes it a static number.",
        "window": window(),
        "controls": controls,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use crate::{Action, Binding, Route, Settings};

    /// Settings with every key set: what [`Settings`] writes at most.
    fn full() -> Settings {
        let mut s = Settings {
            from_resref: true,
            opener_hash: Some("0".into()),
            event_hash: Some("1".into()),
            window_id: Some("id".into()),
            swap_hosts: vec!["pages".into()],
            ..Default::default()
        };
        s.bindings
            .insert("b".into(), Binding { value: json!(1), watch: true, ..Default::default() });
        s.views.insert("v".into(), template("col"));
        s.view_targets.insert("v".into(), "pages".into());
        s.actions.push(Route::new("click", "ok", Action::Close));
        s
    }

    fn properties(schema: &Value) -> BTreeSet<String> {
        schema["properties"].as_object().unwrap().keys().cloned().collect()
    }

    fn keys(v: &Value) -> BTreeSet<String> {
        v.as_object().unwrap().keys().cloned().collect()
    }

    #[test]
    fn the_schema_names_every_key_the_settings_write() {
        let written: Value = serde_json::from_slice(&full().bytes()).unwrap();
        assert_eq!(keys(&written), properties(&settings_schema()));
        let bind = &settings_schema()["properties"]["bindings"]["additionalProperties"];
        assert_eq!(keys(&written["bindings"]["b"]), properties(bind));
    }

    #[test]
    fn the_schema_s_defaults_are_what_a_key_left_out_reads_as() {
        let read = Settings::parse(format!("{{\"format\":\"{FORMAT}\"}}").as_bytes()).unwrap();
        let written: Value = serde_json::from_slice(&read.bytes()).unwrap();
        let schema = settings_schema();
        for (k, p) in schema["properties"].as_object().unwrap() {
            let Some(default) = p.get("default") else { continue };
            // (Empty lists and maps aren't written: they read as empty.)
            let had = written.get(k).cloned().unwrap_or_else(|| match default {
                Value::Array(_) => json!([]),
                _ => json!({}),
            });
            assert_eq!(&had, default, "{k}");
        }
        assert_eq!(read, Settings::default());
    }

    #[test]
    fn each_handler_s_form_is_what_its_action_writes() {
        let schema = settings_schema();
        let forms = schema["properties"]["actions"]["items"]["oneOf"].as_array().unwrap();
        let by_action: BTreeMap<&str, &Value> = forms
            .iter()
            .map(|f| (f["properties"]["action"]["const"].as_str().unwrap(), f))
            .collect();
        let actions = [
            Action::Code,
            Action::Close,
            Action::Toggle { bind: "b".into() },
            Action::Set { bind: "b".into(), value: json!(1) },
            Action::View { group: "g".into(), view: "v".into() },
        ];
        assert_eq!(by_action.len(), actions.len());
        for action in actions {
            let written = serde_json::to_value(Route::new("click", "ok", action)).unwrap();
            let form = by_action[written["action"].as_str().unwrap()];
            assert_eq!(keys(&written), properties(form), "{written}");
            let required: BTreeSet<String> = form["required"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r.to_string().replace('"', ""))
                .collect();
            assert_eq!(required, properties(form), "{written}");
        }
    }

    /// The manual's chapter documents every key and action, and its example
    /// is the crier window's settings (`tests/data/crier.txt`).
    #[test]
    fn the_manual_documents_the_settings_and_its_example_reads() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let manual = std::fs::read_to_string(root.join("docs/manual/18-nui-creator.md")).unwrap();
        let section = manual.split("## The settings file").nth(1).expect("the section");
        let section = section.split("\n## ").next().unwrap();
        let schema = settings_schema();
        for k in properties(&schema) {
            assert!(section.contains(&format!("`{k}`")), "{k} is not in the manual");
        }
        for f in schema["properties"]["actions"]["items"]["oneOf"].as_array().unwrap() {
            let action = f["properties"]["action"]["const"].as_str().unwrap();
            assert!(section.contains(&format!("`{action}`")), "{action} is not in the manual");
        }
        let example = section.split("```json\n").nth(1).unwrap().split("```").next().unwrap();
        let crier = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/crier.txt"),
        )
        .unwrap();
        assert_eq!(example, crier);
        Settings::parse(example.as_bytes()).unwrap();
    }

    /// The crier window, written by hand from the files' forms alone (the
    /// test fixture the command line and the preview's tests draw), passes
    /// the checks.
    #[test]
    fn a_window_written_from_the_forms_passes_the_checks() {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");
        let window = crate::parse(&std::fs::read(data.join("crier.jui")).unwrap()).unwrap();
        let settings = Settings::parse(&std::fs::read(data.join("crier.txt")).unwrap()).unwrap();
        assert_eq!(crate::validate(&window, &settings), []);
    }

    #[test]
    fn the_controls_are_the_creator_s_templates() {
        let c = controls();
        assert_eq!(c["controls"].as_object().unwrap().len(), ELEMENTS.len());
        assert_eq!(c["controls"]["slider"], template("slider"));
        assert_eq!(c["window"], window());
    }
}
