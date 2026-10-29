//! The forms of a window's two resources, as a program that writes them
//! needs them: the settings (`NAME.txt`, [`FORMAT`]) as a JSON Schema
//! ([`settings_schema`]), the layout (`NAME.jui`) as a JSON Schema made of
//! the checks' tables ([`jui_schema`]), and the controls of a layout as the
//! Creator makes them ([`controls`]). `mg nui schema` prints them. The schema is written
//! here beside [`Settings`] and [`Handler`], and its tests hold it to what
//! they read and write: a key added to the settings without its place
//! here fails them.

use serde_json::{Map, Value, json};

use crate::properties::{self, Kind, Property, Takes};
use crate::{ELEMENTS, EVENT_TYPES, FORMAT, template, window};

#[cfg(doc)]
use crate::{Handler, Settings};

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
        "$id": "https://github.com/jadzziaa/moonglow-toolset/nui-settings.schema.json",
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
            "event_function": key(json!({"type": "boolean", "default": false}),
                "The window's events go to the module's own event script (OnNuiEvent): Build writes NAME_e.nss as an include whose NAME_OnEvent() handles them and returns whether the event was this window's, for that script to call; the window is created without an event script of its own"),
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
        "description": "A JUI is the JSON NuiCreate takes, as nw_inc_nui's functions build it (NuiWindow, NuiCol, NuiLabel...). `window` is a new window; `controls` each type's fields as the NUI Creator adds it. Any element also takes id, width, height, aspect, margin, padding, enabled, visible, encouraged, tooltip, disabled_tooltip, foreground_color, font and draw_list. A property may be a bind ({\"bind\": NAME}) unless nw_inc_nui makes it a static number. Widths: these templates give none, and a control without one is not stretched to its column. A button, button_image, button_select, check, combo, slider or sliderf without a width is 150 points wide (nw_inc_nui's NUI_STYLE_PRIMARY_WIDTH, the width the client gives it); options and tabbar 150 an entry; color_picker 300. A label, text, textedit, image, spacer, progress, list, chart, row, col or group without one takes its column's width. A column is as wide as its widest control (a label, text, textedit, image or spacer counts nothing toward it, a list, chart or progress 150), the window's root column too: a new window's column is 154 wide, its Close button 150, however wide the window. In a row, those that take their column's width share what the others leave; the rest keep theirs. Give the controls widths: the window's width less 20 fills its root column (8 points of inset and 2 of padding a side).",
        "window": window(),
        "controls": controls,
    })
}

/// A property's schema as the table has it: its kind's value; for one a
/// bind may give, or a bind; null where null will do.
fn property_schema(p: &Property) -> Value {
    let mut choices = vec![match p.takes {
        Takes::Modifier => json!({"type": "number"}),
        _ => p.kind.schema(),
    }];
    if p.takes == Takes::Bound {
        choices.push(json!({"$ref": "#/$defs/bind"}));
    }
    if p.nullable {
        choices.push(json!({"type": "null"}));
    }
    if p.name == "title" && p.nullable {
        choices.push(json!({"const": false}));
    }
    let mut about = p.about.to_owned();
    match p.takes {
        Takes::Bound => {}
        Takes::Static => about += " (static: nw_inc_nui takes no bind for it)",
        Takes::Modifier => about += " (a static number: no bind)",
    }
    if let Some((low, high, constants)) = p.constants {
        about += &format!(" ({constants}: {low} to {high}; another is shown all the same)");
    }
    let mut schema = if choices.len() == 1 { choices.remove(0) } else { json!({"anyOf": choices}) };
    schema["description"] = about.into();
    schema
}

/// The properties of `table` that `ty` has, as a schema's `properties`.
fn properties_of(table: &[Property], ty: &str) -> Map<String, Value> {
    table.iter().filter(|p| p.of(ty)).map(|p| (p.name.to_owned(), property_schema(p))).collect()
}

/// A window's layout (`NAME.jui`, the JSON `NuiCreate` takes) as a JSON
/// Schema (draft 2020-12), made of the tables the checks hold a layout to
/// ([`crate::properties`]): the window's properties, and each control's by
/// its `type`, the kind of value each takes and whether a bind may give
/// it; a chart's series and the draw layers'. Keys it does not name are
/// kept, and the game is the judge of them. What the checks hold in code
/// (a group's one child, a row's fit across, the game's refusals) is not
/// in it: `mg nui validate` says those.
pub fn jui_schema() -> Value {
    let mut defs: Map<String, Value> =
        Kind::ALL.iter().map(|k| k.definition()).map(|(n, s)| (n.to_owned(), s)).collect();
    defs.insert(
        "bind".into(),
        json!({"type": "object", "required": ["bind"], "description":
            "A bind (NuiBind): the value is the bind's, set by the opener and by scripts \
             (NuiSetBind); in a list's row, an array of a value a row",
            "properties": {
                "bind": {"type": "string", "description": "The bind's name"},
                "number_flags": {"type": "integer", "description": "1: a whole number shown in hexadecimal"},
                "number_precision": {"type": "integer", "description": "A number's decimal places"},
                "text_flags": {"type": "integer", "description": "1: lower case, 2: upper case (ASCII letters)"},
            }}),
    );
    let mut types = Vec::new();
    for ty in ELEMENTS {
        let mut properties = properties_of(properties::ELEMENT, ty);
        properties.insert("type".into(), json!({"const": ty}));
        properties.insert(
            "id".into(),
            json!({"type": "string", "description":
            "Its ID: whose events are (NuiGetEventElement), and NuiFindWindow's by it"}),
        );
        // (The constructor writes label and value for every type: null where
        // the type reads none.)
        for k in ["label", "value"] {
            properties.entry(k).or_insert_with(|| {
                json!({"description": "Written by the constructor (null); this type reads none"})
            });
        }
        let element = json!({"$ref": "#/$defs/element"});
        match *ty {
            "row" | "col" => {
                properties.insert(
                    "children".into(),
                    json!({"type": "array", "items": element,
                    "description": "Its controls, side by side (row) or one under another (col)"}),
                );
            }
            "group" => {
                properties.insert(
                    "children".into(),
                    json!({"type": "array", "items": element,
                    "minItems": 1, "maxItems": 1, "description":
                    "Its one layout; a swap layout's is replaced by NuiSetGroupLayout"}),
                );
            }
            "list" => {
                properties.insert(
                    "row_template".into(),
                    json!({"type": "array", "maxItems": 16,
                    "description": "A row's cells (NuiListTemplateCell): the control, its width \
                                    (0: automatic) and whether it takes what the others leave",
                    "items": {"type": "array", "minItems": 3, "maxItems": 3, "prefixItems":
                        [element, {"type": "number", "minimum": 0}, {"type": "boolean"}]}}),
                );
            }
            _ => {}
        }
        properties.insert(
            "draw_list".into(),
            json!({"type": "array", "items":
            {"$ref": "#/$defs/draw"}, "description": "Draw layers painted with it (NuiDrawList)"}),
        );
        let def = format!("element_{ty}");
        defs.insert(
            def.clone(),
            json!({"type": "object", "properties": properties,
            "required": ["type"]}),
        );
        types.push(json!({"if": {"properties": {"type": {"const": ty}}, "required": ["type"]},
            "then": {"$ref": format!("#/$defs/{def}")}}));
    }
    defs.insert(
        "element".into(),
        json!({"type": "object", "required": ["type"],
        "description": format!("A control, by its type: {}; another type is kept, and the game \
                                is the judge of it", ELEMENTS.join(", ")),
        "properties": {"type": {"type": "string"}},
        "allOf": types}),
    );
    let mut slot = properties_of(properties::CHART_SLOT, "");
    slot.insert("type".into(), property_schema(&properties::CHART_SLOT[0]));
    defs.insert(
        "chart_slot".into(),
        json!({"type": "object", "properties": slot,
        "description": "A chart's series (NuiChartSlot)"}),
    );
    if let Some(value) = defs["element_chart"]["properties"].get_mut("value") {
        value["items"] = json!({"$ref": "#/$defs/chart_slot"});
    }
    let mut draw: Map<String, Value> = properties::DRAW
        .iter()
        .map(|p| {
            let mut s = property_schema(p);
            if !p.of.is_empty() {
                let names: Vec<&str> =
                    p.of.iter()
                        .filter_map(|n| n.parse::<usize>().ok())
                        .filter_map(|n| properties::DRAW_TYPES.get(n).copied())
                        .collect();
                s["description"] = format!("{} (a {}'s)", p.about, names.join(", ")).into();
            }
            (p.name.to_owned(), s)
        })
        .collect();
    let named: Vec<String> =
        properties::DRAW_TYPES.iter().enumerate().map(|(i, n)| format!("{i} {n}")).collect();
    draw.insert(
        "type".into(),
        json!({"type": "integer", "minimum": 0, "maximum": 7,
        "description": format!("NUI_DRAW_LIST_ITEM_TYPE_*: {}", named.join(", "))}),
    );
    defs.insert(
        "draw".into(),
        json!({"type": "object", "required": ["type"], "properties": draw,
        "description": "A draw layer (NuiDrawListPolyLine, NuiDrawListText...)"}),
    );
    let mut window = properties_of(properties::WINDOW, "");
    window.insert("version".into(), json!({"const": 1, "description": "1, the stock NuiWindow's"}));
    window.insert(
        "root".into(),
        json!({"$ref": "#/$defs/element", "description":
        "Its layout: a row, column or group as nw_inc_nui makes them (the game shows another \
         control there too)"}),
    );
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://github.com/jadzziaa/moonglow-toolset/nui-layout.schema.json",
        "title": "NUI window layout (NAME.jui)",
        "description": "The JSON NuiCreate takes, as nw_inc_nui's functions build it: the window (NuiWindow) and its controls by type, the kind of value each property takes and whether a bind ({\"bind\": NAME}) may give it. Made of the tables the NUI Creator's checks hold a layout to; what they hold in code (a group's one child, a child that doesn't fit across its row, a text field's max, what the game refuses) `mg nui validate` says. Keys it doesn't name are kept.",
        "type": "object",
        "required": ["version", "root"],
        "properties": window,
        "$defs": defs,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use crate::{Action, Binding, Handler, Settings};

    /// Settings with every key set: what [`Settings`] writes at most.
    fn full() -> Settings {
        let mut s = Settings {
            from_resref: true,
            event_function: true,
            opener_hash: Some("0".into()),
            event_hash: Some("1".into()),
            window_id: Some("id".into()),
            swap_hosts: vec!["pages".into()],
            ..Default::default()
        };
        s.bindings
            .insert("b".into(), Binding { value: json!(1), watch: true, ..Default::default() });
        s.variants.insert("v".into(), template("col"));
        s.variant_targets.insert("v".into(), "pages".into());
        s.handlers.push(Handler::new("click", "ok", Action::Close));
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
            // (Empty lists and maps, and switches off, aren't written: they read
            // as the default.)
            let had = written.get(k).cloned().unwrap_or_else(|| match default {
                Value::Array(_) => json!([]),
                Value::Bool(_) => json!(false),
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
            Action::Variant { group: "g".into(), variant: "v".into() },
        ];
        assert_eq!(by_action.len(), actions.len());
        for action in actions {
            let written = serde_json::to_value(Handler::new("click", "ok", action)).unwrap();
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
        // (A Windows checkout may give these files CRLF line ends.)
        let lf = |t: String| t.replace("\r\n", "\n");
        let manual =
            lf(std::fs::read_to_string(root.join("docs/manual/18-nui-creator.md")).unwrap());
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
        let crier = lf(std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data/crier.txt"),
        )
        .unwrap());
        assert_eq!(example, crier);
        Settings::parse(example.as_bytes()).unwrap();
    }

    /// The crier window, written by hand from the files' forms alone (the
    /// test fixture the command line and the preview's tests draw), passes
    /// the checks.
    #[test]
    fn a_window_written_from_the_forms_passes_the_checks() {
        let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");
        // (And the crier's list of cries, with a swap layout variant.)
        for name in ["crier", "cries"] {
            let read = |ext: &str| std::fs::read(data.join(format!("{name}.{ext}"))).unwrap();
            let window = crate::parse(&read("jui")).unwrap();
            let settings = Settings::parse(&read("txt")).unwrap();
            assert_eq!(crate::validate(&window, &settings), [], "{name}");
            // Its binds as a script sets them name binds it has.
            let binds: Map<String, Value> = serde_json::from_slice(&read("binds.json")).unwrap();
            for bind in binds.keys() {
                assert!(settings.bindings.contains_key(bind), "{name}: {bind}");
            }
        }
    }

    #[test]
    fn the_controls_are_the_creator_s_templates() {
        let c = controls();
        assert_eq!(c["controls"].as_object().unwrap().len(), ELEMENTS.len());
        assert_eq!(c["controls"]["slider"], template("slider"));
        assert_eq!(c["window"], window());
        // What a control without a width is: said, as the preview lays it out.
        let about = c["description"].as_str().unwrap();
        assert!(about.contains("is 150 points wide (nw_inc_nui's NUI_STYLE_PRIMARY_WIDTH"));
        assert!(about.contains("A column is as wide as its widest control"));
    }

    /// The layout's schema is the checks' table: every property of every
    /// type is in its type's schema, a bind allowed where the table takes
    /// one and refused where it makes the property static; and every key
    /// the Creator's templates and new window write is named.
    #[test]
    fn the_layout_schema_is_the_checks_table() {
        let s = jui_schema();
        let defs = &s["$defs"];
        for ty in ELEMENTS {
            let def = &defs[format!("element_{ty}")]["properties"];
            for p in properties::ELEMENT.iter().filter(|p| p.of(ty)) {
                let schema = def[p.name].to_string();
                assert_eq!(
                    schema.contains("#/$defs/bind"),
                    p.takes == Takes::Bound,
                    "{ty}.{}: {schema}",
                    p.name
                );
            }
            for k in keys(&template(ty)) {
                assert!(def.get(&k).is_some(), "{ty}: {k} is not in the schema");
            }
        }
        for k in keys(&window()) {
            assert!(s["properties"].get(&k).is_some(), "the window's {k}");
        }
        for p in properties::DRAW {
            assert!(defs["draw"]["properties"].get(p.name).is_some(), "{}", p.name);
        }
        // Every reference is to a definition there.
        let text = s.to_string();
        for reference in text.split("\"$ref\":\"#/$defs/").skip(1) {
            let name = reference.split('"').next().unwrap();
            assert!(defs.get(name).is_some(), "{name}");
        }
        assert_eq!(
            defs["element_slider"]["properties"]["max"]["anyOf"][0]["$ref"],
            "#/$defs/integer"
        );
        assert_eq!(
            defs["element_sliderf"]["properties"]["max"]["anyOf"][0]["$ref"],
            "#/$defs/number"
        );
        assert_eq!(defs["element_options"]["properties"]["elements"]["$ref"], "#/$defs/labels");
    }

    /// A wrong value, of any property of the table, of any type that has
    /// it, is found where it is; a bind is refused where the table makes
    /// the property static, and taken (with a first value of its kind)
    /// where it doesn't: the checks read the table the schema is made of.
    #[test]
    fn the_checks_hold_every_property_of_the_table() {
        let wrong = |k: Kind| if k == Kind::Bool { json!("x") } else { json!(true) };
        let first = |k: Kind| match k {
            Kind::Bool => json!(true),
            Kind::Text => json!("a"),
            Kind::Int | Kind::RowCount => json!(1),
            Kind::Number => json!(0.5),
            Kind::Rect => json!({"x": 0, "y": 0, "w": 1, "h": 1}),
            Kind::Color => json!({"r": 1, "g": 2, "b": 3, "a": 4}),
            Kind::Array | Kind::Numbers | Kind::FloatPairs => json!([]),
            Kind::Labels => json!(["a"]),
            Kind::Entries => json!([["a", 0]]),
            Kind::Vec2 => json!({"x": 0, "y": 0}),
        };
        let errors_at = |w: &Value, s: &Settings, at: &str| {
            crate::validate(w, s)
                .into_iter()
                .any(|d| d.severity == crate::Severity::Error && d.path == at)
        };
        let checked = |doc: &mut Value, put: &dyn Fn(&mut Value, Value), at: &str, p: &Property| {
            let s = Settings::default();
            put(doc, wrong(p.kind));
            assert!(errors_at(doc, &s, at), "{at}: {} taken", wrong(p.kind));
            put(doc, json!({"bind": "b"}));
            let mut s = Settings::default();
            s.bindings
                .insert("b".into(), crate::Binding { value: first(p.kind), ..Default::default() });
            assert_eq!(errors_at(doc, &s, at), p.takes != Takes::Bound, "{at}: a bind");
        };
        for ty in ELEMENTS {
            for p in properties::ELEMENT.iter().filter(|p| p.of(ty)) {
                let mut doc = window();
                doc["root"]["children"] = json!([template(ty)]);
                let put = |d: &mut Value, v: Value| d["root"]["children"][0][p.name] = v;
                checked(&mut doc, &put, &format!("/root/children/0/{}", p.name), p);
            }
        }
        for p in properties::WINDOW.iter().filter(|p| p.name != "version") {
            let put = |d: &mut Value, v: Value| d[p.name] = v;
            checked(&mut window(), &put, &format!("/{}", p.name), p);
        }
        for (i, kind) in properties::DRAW_TYPES.iter().enumerate() {
            for p in properties::DRAW.iter().filter(|p| p.of(&i.to_string())) {
                let mut doc = window();
                doc["root"]["draw_list"] = json!([{"type": i, "rect": first(Kind::Rect)}]);
                let put = |d: &mut Value, v: Value| d["root"]["draw_list"][0][p.name] = v;
                checked(&mut doc, &put, &format!("/root/draw_list/0/{}", p.name), p);
                assert!(!kind.is_empty());
            }
        }
        for p in properties::CHART_SLOT {
            let mut doc = window();
            doc["root"]["children"] = json!([template("chart")]);
            let put = |d: &mut Value, v: Value| d["root"]["children"][0]["value"][0][p.name] = v;
            checked(&mut doc, &put, &format!("/root/children/0/value/0/{}", p.name), p);
        }
    }
}
