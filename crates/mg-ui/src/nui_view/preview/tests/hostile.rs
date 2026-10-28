//! Windows no editor would write, on the canvas: a JUI is text a builder
//! may edit by hand, and the preview draws whatever parses.
use super::super::*;
use std::time::{Duration, Instant};

/// Values no property expects.
fn odd_values() -> Vec<Value> {
    vec![
        Value::Null,
        json!("text"),
        json!(""),
        json!(true),
        json!(-1),
        json!(0),
        json!(1e308),
        json!(-1e308),
        json!(1_000_000_000_000_i64),
        json!([]),
        json!({}),
        json!([[]]),
        json!([1, "a", null, [2], {"x": 3}]),
        json!({"bind": "missing"}),
        json!({"bind": "rows"}),
        json!({"bind": 5}),
        json!({"strref": "x"}),
        json!({"strref": -7}),
        json!({"x": "a", "y": [], "w": -1e308, "h": 1e308, "r": "red", "a": 1e308}),
    ]
}

/// Every property a control or a window of the editor may have.
const PROPERTIES: &[&str] = &[
    "id",
    "label",
    "value",
    "children",
    "width",
    "height",
    "aspect",
    "margin",
    "padding",
    "enabled",
    "visible",
    "encouraged",
    "tooltip",
    "disabled_tooltip",
    "foreground_color",
    "font",
    "border",
    "scrollbars",
    "elements",
    "direction",
    "min",
    "max",
    "step",
    "multiline",
    "wordwrap",
    "row_template",
    "row_count",
    "row_height",
    "text_halign",
    "text_valign",
    "image_aspect",
    "image_halign",
    "image_valign",
    "image_region",
    "draw_list",
];

fn window_of(children: Value) -> Value {
    let mut doc = mg_nui::window();
    doc["geometry"] = json!({"x": -1.0, "y": -1.0, "w": 600.0, "h": 400.0});
    doc["root"]["children"] = children;
    doc
}

/// Every kind of control with one property (or each of them) made `value`.
fn controls_with(property: Option<&str>, value: &Value) -> Value {
    let controls: Vec<Value> = mg_nui::ELEMENTS
        .iter()
        .chain(&["no_such_control"])
        .map(|kind| {
            let mut control = mg_nui::template(kind);
            for name in PROPERTIES.iter().filter(|name| property.is_none_or(|p| p == **name)) {
                control[*name] = value.clone();
            }
            control
        })
        .collect();
    // Half of them in a row, half in a group, the rest in the root column.
    let (row, rest) = controls.split_at(controls.len() / 3);
    let (group, rest) = rest.split_at(rest.len() / 2);
    let mut children = vec![
        json!({"type": "row", "children": row}),
        json!({"type": "group", "children": [{"type": "col", "children": group}]}),
    ];
    children.extend(rest.iter().cloned());
    children.into()
}

fn hostile_documents() -> Vec<Value> {
    let mut docs = vec![
        // Not a window at all.
        json!("text"),
        json!([1, 2, 3]),
        Value::Null,
        json!(42),
        json!({}),
        json!({"root": "nope", "geometry": "wide", "title": [1], "collapsed": "yes",
            "closable": 5, "resizable": {}, "border": [], "transparent": "no",
            "accepts_input": 0, "font": 7, "size_constraint": "small"}),
        json!({"root": [], "geometry": {"x": 1e308, "y": -1e308, "w": 1e308, "h": -1e308},
            "size_constraint": {"x": 1e308, "y": -1e308, "w": -5, "h": "h"}}),
        json!({"root": {"type": 7, "children": {"a": 1}}, "geometry": {"bind": "rows"},
            "collapsed": {"bind": "rows"}, "title": {"strref": 1e308}}),
        json!({"root": {"type": "col", "children": "x"}, "geometry": [1, 2, 3, 4]}),
    ];
    // Children that are no controls, and lists of every wrong shape.
    docs.push(window_of(json!([
        1, "x", null, [], {}, {"type": 7}, {"type": []}, {"type": "row", "children": "x"},
        {"type": "col", "children": [[[]]]}, {"type": "group", "children": 5},
        {"type": "group", "children": [1, 2]},
        {"type": "list", "row_count": 1_000_000_000, "row_height": 25.0,
            "row_template": [[{"type": "label", "value": {"bind": "rows"}}, 0.0, true]]},
        {"type": "list", "row_count": 1_000_000_000, "row_height": 0.0,
            "row_template": [[{"type": "check"}, -5.0, false], [{"type": "label"}, 1e308, false]]},
        {"type": "list", "row_count": 1e308, "row_height": -1e308, "row_template": "x"},
        {"type": "list", "row_count": [1, 2, 3], "row_height": 1e308,
            "row_template": [1, [2], [[3]], [null, "wide", "yes"], [{"type": "list",
                "row_count": 1_000_000_000, "row_template": [[{"type": "label"}, 0.0, true]]}]]},
    ])));
    // Sizes no window has.
    for size in [1e308, -1e308, 0.0, -1.0, 1e-300, 4096.0, 1e9] {
        docs.push(window_of(json!([
            {"type": "row", "height": size, "children": [
                {"type": "button", "label": "a", "width": size, "height": size},
                {"type": "label", "value": "b", "aspect": size, "width": 50.0},
                {"type": "image", "value": "gui_error", "aspect": size, "height": 50.0,
                    "image_region": {"x": size, "y": size, "w": size, "h": size}},
                {"type": "spacer", "margin": size, "padding": size}]},
            {"type": "group", "width": size, "margin": size, "children": [
                {"type": "col", "padding": size, "children": [
                    {"type": "text", "value": "long ".repeat(500), "height": size},
                    {"type": "slider", "min": size, "max": -size, "step": size, "value": size},
                    {"type": "sliderf", "min": -size, "max": size, "step": -size, "value": {"bind": "n"}},
                    {"type": "progress", "min": size, "max": size, "value": size},
                    {"type": "chart", "value": [{"type": 0, "data": [size, -size, 1.0, "a", null]},
                        {"type": 1, "data": [size, -size, 0.0, 2.0], "color": {"r": size}}]},
                    {"type": "color_picker", "value": {"r": size, "g": -size, "b": "b"}},
                    {"type": "options", "elements": [["a", 0], ["b", 1]], "value": size,
                        "direction": size, "height": size},
                    {"type": "list", "row_count": 50, "row_height": size, "height": size,
                        "row_template": [[{"type": "label", "value": "r"}, size, false]]}]}]},
        ])));
        let mut window = window_of(json!([{"type": "label", "value": "sized"}]));
        window["geometry"] = json!({"x": size, "y": -size, "w": size, "h": size});
        window["size_constraint"] = json!({"x": size, "y": size, "w": -size, "h": size});
        docs.push(window);
    }
    // Draw layers of every wrong shape.
    docs.push(window_of(json!([{"type": "spacer", "height": 200.0, "draw_list": [
        1, "a", null, [], {}, {"type": "x"}, {"type": -1}, {"type": 99},
        {"type": 0, "points": "x"}, {"type": 0, "points": [1, "a", null]},
        {"type": 0, "points": [1e308, -1e308, 5.0], "fill": true},
        {"type": 1}, {"type": 1, "a": "a", "b": [], "ctrl0": {"x": 1e308}, "ctrl1": 5},
        {"type": 2, "rect": {"x": 1e308, "y": -1e308, "w": 1e308, "h": 0}},
        {"type": 3, "radius": 1e308, "amin": -1e308, "amax": 1e308, "c": "c"},
        {"type": 3, "radius": -5, "fill": true},
        {"type": 4, "text": [], "rect": "r", "font": 5},
        {"type": 4, "text": "x".repeat(20_000), "rect": {"x": 0, "y": 0, "w": 1e308, "h": 1e308}},
        {"type": 5, "image": 5}, {"type": 5, "image": "gui_error", "rect": {"w": -1, "h": -1}},
        {"type": 6, "a": {"x": 1e308}, "b": {"y": -1e308}, "line_thickness": -1e308},
        {"type": 7, "arrayBinds": true, "rect": {"bind": "rows"}, "color": "red",
            "order": "first", "render": 1e308, "enabled": "yes"},
        {"type": 7, "arrayBinds": "true", "rect": {"x": {"bind": "rows"}}},
    ]}])));
    docs.push(window_of(json!([{"type": "label", "draw_list": "x"}])));
    // A hundred deep, of every container.
    for kind in ["col", "row", "group", "list"] {
        let mut deep = json!({"type": "label", "value": "bottom"});
        for _ in 0..100 {
            deep = if kind == "list" {
                json!({"type": "list", "row_count": 3, "row_template": [[deep, 0.0, true]]})
            } else {
                json!({"type": kind, "children": [deep.clone(), deep]})
            };
            // (Two children a level would be 2^100 controls: one branch only
            // is kept whole.)
            if let Some(first) = deep.pointer_mut("/children/0") {
                *first = json!({"type": "label", "value": "beside"});
            }
        }
        docs.push(window_of(json!([deep])));
    }
    // Every kind of control with each odd value in all its properties, and
    // in one property at a time (a third of the pairs: each property meets
    // six of the values).
    for (k, value) in odd_values().into_iter().enumerate() {
        docs.push(window_of(controls_with(None, &value)));
        for (p, property) in PROPERTIES.iter().enumerate() {
            if (p + k) % 3 == 0 {
                docs.push(window_of(controls_with(Some(property), &value)));
            }
        }
        // And the window's own.
        let mut window = window_of(json!([{"type": "label", "value": "w"}]));
        for name in [
            "title",
            "geometry",
            "collapsed",
            "closable",
            "resizable",
            "border",
            "transparent",
            "accepts_input",
            "font",
            "size_constraint",
            "id",
        ] {
            window[name] = value.clone();
        }
        docs.push(window);
    }
    docs
}

fn settings() -> Settings {
    let mut s = Settings::default();
    for (name, value) in [
        (
            "rows",
            json!(["a", 1, null, {"strref": 5}, [1], {"x": 1.0, "y": 2.0, "w": 3.0, "h": 4.0}]),
        ),
        ("n", json!(1e308)),
        ("flag", json!("yes")),
    ] {
        s.bindings.insert(name.into(), Binding { value, ..Default::default() });
    }
    s.variants.insert("other".into(), json!("not a layout"));
    s.handlers = vec![
        mg_nui::Handler {
            event: "open".into(),
            element: String::new(),
            action: mg_nui::Action::Variant { group: "_window_".into(), variant: "other".into() },
            extra: Default::default(),
        },
        mg_nui::Handler {
            event: "click".into(),
            element: "label".into(),
            action: mg_nui::Action::Toggle { bind: "rows".into() },
            extra: Default::default(),
        },
        mg_nui::Handler {
            event: "watch".into(),
            element: "rows".into(),
            action: mg_nui::Action::Set { bind: "rows".into(), value: json!("again") },
            extra: Default::default(),
        },
    ];
    s
}

/// The states of the canvas a document is drawn in.
fn states(doc: &Value) -> Vec<State> {
    let base = || State { zoom: 1.0, ..Default::default() };
    let resize = |path: &str| interaction::Resize {
        path: path.into(),
        source: doc.clone(),
        origin: pos2(10.0, 10.0),
        scale: 1.0,
        anchor: vec2(20.0, 20.0),
        initial: vec2(100.0, 50.0),
        size: vec2(120.0, 60.0),
        horizontal: true,
        vertical: true,
    };
    vec![
        base(),
        State { preview_interactive: true, ..base() },
        State { preview_interactive: true, preview_scale: 2.0, screen_preview: true, ..base() },
        State { selected: "/root/children/0".into(), preview_state: 4, zoom: 0.0, ..base() },
        State { selected: "/root/children/1/children/0".into(), preview_state: 3, ..base() },
        State { resize: Some(resize("")), ..base() },
        State { resize: Some(resize("/root/children/0")), ..base() },
        State { resize: Some(resize("/root/children/3")), selected: "/root".into(), ..base() },
        State {
            window_move: Some(interaction::WindowMove {
                source: doc.clone(),
                origin: pos2(10.0, 10.0),
                initial: vec2(5.0, 5.0),
                position: vec2(15.0, 25.0),
                scale: 1.0,
            }),
            screen_preview: true,
            ..base()
        },
    ]
}

#[test]
fn hostile_documents_draw_without_panicking_in_bounded_time() {
    let s = settings();
    let docs = hostile_documents();
    assert!(docs.len() > 250, "{}", docs.len());
    let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1100.0, 900.0));
    // A skin and the picture the documents name, so that pictures are drawn.
    let mut module = mg_module::Module::new();
    module.set(
        ResKey::parse("nui_skin", ResType::TML).unwrap(),
        b"[window]\nborder_image = \"gui_error\"\n[button]\nnormal = \"gui_error\"\n\
          [checkbox]\nnormal = \"gui_error\"\ncursor_normal = \"gui_error\"\n\
          [scrollv]\nnormal = \"#202020ff\"\ncursor_normal = \"gui_error\"\n"
            .to_vec(),
    );
    let mut picture = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 1, 0, 24, 0];
    picture.extend([255, 0, 0, 0, 0, 255]);
    module.set(ResKey::parse("gui_error", ResType::TGA).unwrap(), picture);
    let mut slowest = Duration::ZERO;
    for (n, doc) in docs.iter().enumerate() {
        // Every state for the hand-made documents; the generated ones as
        // they are edited and in a simulation.
        let states = states(doc);
        let take = if n < 30 { states.len() } else { 2 };
        for (i, mut state) in states.into_iter().enumerate().take(take) {
            let ctx = egui::Context::default();
            let mut doc = doc.clone();
            let mut assets = skin::Assets::default();
            let start = Instant::now();
            // Frames with the pointer moving over the window, pressing and
            // letting go: clicks, drags and the release that applies a gesture.
            for frame in 0..5 {
                let at = pos2(300.0 + 40.0 * frame as f32, 250.0 + 30.0 * frame as f32);
                let mut events = vec![egui::Event::PointerMoved(at)];
                if frame == 1 || frame == 3 {
                    events.push(egui::Event::PointerButton {
                        pos: at,
                        button: egui::PointerButton::Primary,
                        pressed: frame == 1,
                        modifiers: Default::default(),
                    });
                }
                let input =
                    egui::RawInput { screen_rect: Some(screen), events, ..Default::default() };
                let mut out = ctx.run_ui(input, |ui| {
                    assets.prepare(ui.ctx(), None, &module, 0, &doc, &s);
                    assert!(assets.loaded && assets.picture("gui_error").is_some());
                    assets.prepare_strings(None, &doc, &s);
                    canvas(ui, &mut doc, &s, &mut state, &mut assets);
                });
                // (This test draws; no backend takes the textures.)
                out.textures_delta.clear();
                let shapes = ctx.tessellate(out.shapes, out.pixels_per_point);
                assert!(shapes.len() < 200_000, "document {n}, state {i}: {} shapes", shapes.len());
            }
            let took = start.elapsed();
            slowest = slowest.max(took);
            assert!(
                took < Duration::from_secs(5),
                "document {n}, state {i}: five frames took {took:?}\n{doc}"
            );
        }
    }
    println!("{} documents, the slowest five frames {slowest:?}", docs.len());
}

/// The gestures and the simulation's actions on windows that are not what
/// they take them for.
#[test]
fn gestures_and_actions_leave_odd_windows_alone() {
    let s = settings();
    for doc in hostile_documents().into_iter().take(40) {
        for path in ["", "/root", "/root/children/0", "/root/children/5", "/geometry", "/x/y"] {
            let mut target = doc.clone();
            let mut resize = states(&doc).remove(5).resize.unwrap();
            resize.path = path.into();
            resize.move_to(pos2(500.0, -500.0));
            resize.apply(&mut target);
            let _ = interaction::cross_room(&doc, path);
        }
        let mut target = doc.clone();
        states(&doc).remove(8).window_move.unwrap().apply(&mut target);
        let mut session = interaction::Session::new(&doc, &s);
        session.constrain_geometry();
        session.collapse(true);
        session.collapse(false);
        for event in ["open", "close", "click", "watch", "no such event"] {
            for element in ["", "label", "rows", "missing"] {
                for row in [None, Some(0), Some(99)] {
                    session.dispatch(event, element, row, 0);
                }
            }
        }
        for path in ["", "/root", "/root/children/0", "/root/children/2/row_template/0/0", "/no"] {
            for value in [None, Some(json!(true)), Some(json!([1]))] {
                for row in [None, Some(0), Some(7)] {
                    session.apply(interaction::Change {
                        path: path.into(),
                        row,
                        value: value.clone(),
                        normalization: false,
                    });
                }
            }
        }
    }
}
