//! Regression expectations derived from the Creator-authored native NWN demos.
//! Reference MOD SHA256: 021f9251c71320998e0d027bfd7ebb1e18b79fb2ab8133983f0bd57a701a7587.
//! The tests check observed relationships, not universal Cassowary equivalence.
use super::*;
use egui_kittest::{Harness, kittest::Queryable};

#[test]
fn nui_native_number_precision_rounds_signed_half_away_from_zero() {
    // Creator-generated Set bind actions, NWN EE89.8193.37-17, 2026-10-08:
    // +/-42.125 renders +/-42.13 at precision 2 (not Rust's ties-to-even).
    for (value, expected) in [
        (42.125, ["42", "42.13", "42.1250"]),
        (-42.125, ["-42", "-42.13", "-42.1250"]),
        (-12.345, ["-12", "-12.35", "-12.3450"]),
    ] {
        let mut settings = Settings::default();
        settings
            .bindings
            .insert("number".into(), Binding { value: json!(value), ..Default::default() });
        let mut h = harness(
            json!([0, 2, 4].map(|precision| json!({
                "type":"label", "value":{"bind":"number","number_precision":precision},
                "width":150.0, "height":30.0
            }))),
            240.0,
            settings,
        );
        h.run();
        let texts: Vec<_> = h
            .output()
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) => Some(t.galley.text()),
                _ => None,
            })
            .collect();
        for label in expected {
            assert!(texts.contains(&label), "missing {label} for {value}: {texts:?}");
        }
    }
}

#[test]
fn nui_native_multiline_edit_text_starts_at_top_even_without_wordwrap() {
    // GUI-authored nui_edit_s in NWN EE89.8193.37-17 (2026-10-08):
    // both multiline variants start at the top; both single-line variants center.
    for multiline in [false, true] {
        for wordwrap in [false, true] {
            let mut h = harness(
                json!([{"type":"textedit","label":"Input","value":"Native edit probe",
                    "multiline":multiline,"wordwrap":wordwrap,"height":90.0,"width":400.0}]),
                240.0,
                Settings::default(),
            );
            h.run();
            let body = h.get_by_label("Canvas Text input · Input").rect();
            let text = h
                .output()
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == "Native edit probe" => Some(t),
                    _ => None,
                })
                .expect("rendered input text");
            if multiline {
                assert!(
                    text.pos.y - body.top() < 6.0,
                    "multiline text: {:?}, body: {body:?}",
                    text.pos
                );
            } else {
                assert!((text.pos.y + text.galley.size().y / 2.0 - body.center().y).abs() < 1.0);
            }
        }
    }
}

#[test]
fn nui_native_chart_uses_per_series_extents_and_zero_baseline() {
    // nui_chart_s, MOD d9f13b28…: line [-10,0,10], columns [5,-5].
    // Native line uses x=i/count; bars extend from zero, not the bottom, a point
    // apart (Nuklear moves each by its index: 14..308 then 310..604 in NWN EE 8193.37).
    let mut h = harness(
        json!([{"type":"chart","width":600.0,"height":180.0,"value":[
            {"type":0,"legend":"Lines","color":{"r":100,"g":160,"b":255,"a":255},"data":[-10.0,0.0,10.0]},
            {"type":1,"legend":"Columns","color":{"r":240,"g":64,"b":64,"a":255},"data":[5.0,-5.0]}
        ]}]),
        300.0,
        Settings::default(),
    );
    h.run();
    let body = h.get_by_label("Canvas Chart").rect();
    let plot = body.shrink(4.0);
    let bars: Vec<_> = h
        .output()
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Rect(r) if r.fill == Color32::from_rgb(240, 64, 64) => Some(r.rect),
            _ => None,
        })
        .collect();
    assert_eq!(bars.len(), 2);
    assert!((bars[0].left() - plot.left()).abs() < 0.1);
    assert!((bars[1].left() - bars[0].right() - 1.0).abs() < 0.1);
    assert!((bars[0].bottom() - plot.center().y).abs() < 0.1);
    assert!((bars[1].top() - plot.center().y).abs() < 0.1);
    let line = h
        .output()
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Path(p) if p.points.len() == 3 && !p.closed => Some(p),
            _ => None,
        })
        .expect("line series");
    assert!((line.points[2].x - plot.left() - plot.width() * 2.0 / 3.0).abs() < 0.1);
}

#[test]
fn nui_native_chart_empty_message_and_constant_series() {
    for data in [json!([]), json!([{"type":0,"legend":"Equal -5","data":[-5.0,-5.0,-5.0]}])] {
        let empty = data.as_array().unwrap().is_empty();
        let mut h = harness(
            json!([{"type":"chart","width":600.0,"height":180.0,"value":data}]),
            300.0,
            Settings::default(),
        );
        h.run();
        assert!(
            !h.output().shapes.iter().any(
                |s| matches!(&s.shape, egui::Shape::Path(p) if p.points.len()==3 && !p.closed)
            )
        );
        let expected = if empty { "No chart data." } else { "Equal -5" };
        assert!(
            h.output()
                .shapes
                .iter()
                .any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text()==expected))
        );
    }
}

struct Case {
    doc: Value,
    settings: Settings,
    state: State,
    assets: skin::Assets,
}

fn harness(children: Value, height: f32, settings: Settings) -> Harness<'static, Case> {
    let mut doc = mg_nui::window();
    doc["geometry"] = json!({"x":-1.0,"y":-1.0,"w":680.0,"h":height});
    doc["root"]["children"] = children;
    Harness::builder().with_size(vec2(1100.0, 1000.0)).build_ui_state(
        |ui, t: &mut Case| canvas(ui, &mut t.doc, &t.settings, &mut t.state, &mut t.assets),
        Case {
            doc,
            settings,
            state: State { zoom: 1.0, ..Default::default() },
            assets: skin::Assets::default(),
        },
    )
}

fn assert_width(h: &Harness<'_, Case>, name: &str, expected: f32) -> Rect {
    let rect = h.get_by_label(name).rect();
    assert!((rect.width() - expected).abs() < 0.2, "{name}: {rect:?}; expected width {expected}");
    rect
}

#[test]
fn nui_native_column_spacer_does_not_add_a_second_row_gap() {
    // nui_audit_s, MOD 1258a5cd...: native gold frame rows after a 30px
    // spacer are 4px closer than the old preview (37 vs 41px between edges).
    let mut h = harness(
        json!([
            {"type":"button","label":"Before","height":30.0},
            {"type":"spacer","height":30.0},
            {"type":"button_image","label":"icon","height":64.0}
        ]),
        420.0,
        Settings::default(),
    );
    h.run();
    let before = h.get_by_label("Canvas Button · Before").rect();
    let after = h.get_by_label("Canvas Image button · icon").rect();
    assert!((after.top() - before.bottom() - 34.0).abs() < 0.1);
    assert_eq!(after.height(), 64.0);
}

#[test]
fn nui_native_widths_keep_intrinsic_row_button_and_layout_insets() {
    // Creator-authored nui_widths_s, MOD 40bfae1d...df: 120/320 fixed,
    // row auto=150, row fixed=100, 4px gap and 2px layout inset in NWN.
    let mut h = harness(
        json!([
            {"type":"label","value":"Auto label","height":30.0},
            {"type":"button","label":"Fixed 120","width":120.0,"height":30.0},
            {"type":"button","label":"Fixed 320","width":320.0,"height":30.0},
            {"type":"row","children":[
                {"type":"button","label":"Row auto","height":30.0},
                {"type":"button","label":"Row 100","width":100.0,"height":30.0}
            ]}
        ]),
        520.0,
        Settings::default(),
    );
    h.run();
    let original = h.state().doc.clone();
    assert_width(&h, "Canvas Label · Auto label", 320.0);
    assert_width(&h, "Canvas Button · Fixed 120", 120.0);
    let wide = assert_width(&h, "Canvas Button · Fixed 320", 320.0);
    let auto = assert_width(&h, "Canvas Button · Row auto", 150.0);
    let fixed = assert_width(&h, "Canvas Button · Row 100", 100.0);
    assert!((auto.left() - wide.left() - 2.0).abs() < 0.1);
    assert!((auto.top() - wide.bottom() - 6.0).abs() < 0.1);
    assert!((fixed.left() - auto.right() - 4.0).abs() < 0.1);
    assert_eq!(h.state().doc, original);
}

#[test]
fn nui_native_geometry_actions_obey_bound_limits_without_editing_source() {
    let mut settings = Settings::default();
    settings.bindings.insert(
        "bounds".into(),
        Binding { value: json!({"x":-1.0,"y":-1.0,"w":680.0,"h":520.0}), ..Default::default() },
    );
    settings.bindings.insert(
        "limits".into(),
        Binding { value: json!({"x":360.0,"y":300.0,"w":780.0,"h":700.0}), ..Default::default() },
    );
    for (id, w, height) in [("small", 100.0, 200.0), ("large", 1000.0, 900.0)] {
        settings.actions.push(mg_nui::Route {
            event: "click".into(),
            element: id.into(),
            action: mg_nui::Action::Set {
                bind: "bounds".into(),
                value: json!({"x":-1.0,"y":-1.0,"w":w,"h":height}),
            },
        });
    }
    let mut h = harness(
        json!([
            {"type":"button","id":"small","label":"Small","height":30.0},
            {"type":"button","id":"large","label":"Large","height":30.0}
        ]),
        520.0,
        settings,
    );
    h.state_mut().doc["geometry"] = json!({"bind":"bounds"});
    h.state_mut().doc["size_constraint"] = json!({"bind":"limits"});
    h.state_mut().state.preview_interactive = true;
    h.run();
    let original = h.state().doc.clone();
    let initial = h.state().settings.clone();
    for (button, w, height) in [("Small", 360.0, 300.0), ("Large", 780.0, 700.0)] {
        h.get_by_label(&format!("Canvas Button · {button}")).click();
        h.run();
        let runtime = h.state().state.runtime.as_ref().unwrap();
        let geometry = &runtime.settings.bindings["bounds"].value;
        assert_eq!(geometry["w"], json!(w));
        assert_eq!(geometry["h"], json!(height));
        let title = h.get_by_label("Canvas window").rect();
        assert!((title.width() + 60.0 - w).abs() < 0.1, "{title:?}");
        assert_eq!(h.state().doc, original);
        assert_eq!(h.state().settings, initial);
    }
}

#[test]
fn nui_native_display_uses_per_option_width_and_column_spans() {
    let mut h = harness(
        json!([
            {"type":"label","value":"Label","height":30.0},
            {"type":"button","label":"Close","height":30.0},
            {"type":"chart","value":[{"type":0,"legend":"Values","color":{"r":100,"g":160,"b":255,"a":255},"data":[0.0,0.5,1.0]}],"height":120.0},
            {"type":"color_picker","value":{"r":255,"g":255,"b":255,"a":255},"height":90.0},
            {"type":"tabbar","elements":["Inventory","Statistics"],"direction":0,"value":0,"height":30.0},
            {"type":"options","elements":["First","Second"],"direction":0,"value":0,"height":30.0},
            {"type":"image","value":"iit_gold_001","height":48.0},
            {"type":"list","row_template":[[{"type":"label","value":"Item","height":30.0},150.0,true]],"row_count":3,"row_height":30.0,"height":110.0,"scrollbars":2}
        ]),
        700.0,
        Settings::default(),
    );
    h.run();
    let original = h.state().doc.clone();
    let label = assert_width(&h, "Canvas Label · Label", 304.0);
    assert_width(&h, "Canvas Button · Close", 150.0);
    let picker = assert_width(&h, "Canvas Color picker", 304.0);
    let square = h.get_by_label("Color picker saturation-value").rect();
    let hue = h.get_by_label("Color picker hue").rect();
    assert!((square.width() - 270.0).abs() < 0.1);
    assert!((hue.right() - picker.left() - 287.0).abs() < 0.1);
    assert_width(&h, "Canvas Tabs", 304.0);
    assert_width(&h, "Canvas Options", 304.0);
    let image = assert_width(&h, "Canvas Image · iit_gold_001", 304.0);
    assert!((label.center().x - image.center().x).abs() < 0.1);
    assert_width(&h, "Canvas List", 304.0);
    h.get_by_label("Scroll list /root/children/7");
    assert_eq!(h.state().doc, original, "Preview sizing must not change authored JUI");
}

#[test]
fn nui_native_group_ignores_child_width_and_fills_remaining_height() {
    let mut h = harness(
        json!([
            {"type":"label","value":"Label","height":30.0},
            {"type":"button","label":"Close","height":30.0},
            {"type":"row","children":[{"type":"label","value":"Auto","height":30.0},{"type":"button","label":"Fixed","width":220.0,"height":30.0}]},
            {"type":"group","border":true,"scrollbars":4,"children":[{"type":"col","children":[{"type":"text","value":"Long grouped text","width":300.0,"height":90.0,"border":true,"scrollbars":4}]}]}
        ]),
        540.0,
        Settings::default(),
    );
    h.run();
    let label = h.get_by_label("Canvas Label · Label").rect();
    assert!(
        (220.0..=232.0).contains(&label.width()),
        "Native span is ~228, independent of the 300-wide group child: {label:?}"
    );
    assert_width(&h, "Canvas Button · Fixed", 220.0);
    assert!(h.query_by_label("Canvas Label · Auto").is_none());
    let group = h.get_by_label("Canvas Group").rect();
    assert!((group.width() - label.width()).abs() < 0.1);
    assert!(group.height() > 350.0, "Group must occupy the remaining height: {group:?}");
    assert!((group.right() - label.right()).abs() < 0.1);
}

#[test]
fn nui_native_list_shares_variable_cells_equally_and_spaces_rows() {
    let mut h = harness(
        json!([
            {"type":"label","value":"Label","height":30.0},
            {"type":"button","label":"Close","height":30.0},
            {"type":"list","row_template":[
                [{"type":"label","value":"Repeated item","height":30.0},150.0,true],
                [{"type":"button","label":"Use item","height":30.0},120.0,true],
                [{"type":"progress","value":0.5,"height":30.0},150.0,true]
            ],"row_count":12,"row_height":30.0,"border":true,"scrollbars":2,"height":180.0,"width":560.0}
        ]),
        480.0,
        Settings::default(),
    );
    h.run();
    assert_width(&h, "Canvas Label · Label", 560.0);
    let button = h.get_by_label("Canvas Button · Use item · row 0").rect();
    let progress = h.get_by_label("Canvas Progress bar · row 0").rect();
    let first = h.get_by_label("Canvas Label · Repeated item · row 0").rect();
    assert!((button.width() - progress.width()).abs() < 0.1);
    assert!((first.width() - button.width()).abs() < 0.1);
    assert!(
        button.width() > 170.0 && button.width() < 180.0,
        "Native slot is ~175 pixels: {button:?}"
    );
    let second = h.get_by_label("Canvas Button · Use item · row 1").rect();
    assert!((second.top() - button.top() - 34.0).abs() < 0.1);
}

#[test]
fn nui_native_color_gradient_updates_simulation_without_editing_source() {
    let mut settings = Settings::default();
    settings.bindings.insert(
        "color".into(),
        Binding { value: json!({"r":255,"g":255,"b":255,"a":91}), ..Default::default() },
    );
    let mut h = harness(
        json!([{"type":"color_picker","value":{"bind":"color"},"height":90.0}]),
        240.0,
        settings,
    );
    h.state_mut().state.preview_interactive = true;
    h.run();
    let before = h.state().doc.clone();
    let square = h.get_by_label("Color picker saturation-value").rect();
    let point = pos2(square.right() - 1.0, square.top() + 1.0);
    h.hover_at(point);
    h.run();
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: point,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        h.run();
    }
    let value = &h.state().state.runtime.as_ref().unwrap().settings.bindings["color"].value;
    assert!(
        value["r"].as_u64().unwrap() > 240
            && value["g"].as_u64().unwrap() < 40
            && value["b"].as_u64().unwrap() < 40,
        "Expected red at HSV corner: {value}"
    );
    // NWN EE 8193.37 (nui_picker_s, authored 128): the bind reads 255 once
    // the window is open, and after every pick.
    assert_eq!(value["a"], 255, "The client's RGB picker writes an opaque colour");
    assert_eq!(h.state().doc, before);
}

#[test]
fn nui_native_rgb_picker_reserves_alpha_at_varied_widths_and_scales() {
    // Native Display has a ~270px matrix next to a ~17px hue bar in a 304px
    // column. These are the RGB formulas in nk_do_color_picker, not a fixed
    // visible width fitted to that screenshot:
    // https://github.com/Immediate-Mode-UI/Nuklear/blob/9f7750296f176e506c2b24ff55bc24495e2db750/src/nuklear_color_picker.c
    for width in [150.0, 304.0, 420.0] {
        for scale in [0.5, 1.0, 1.5, 2.0] {
            let mut h = harness(
                json!([{"type":"color_picker","value":{"r":255,"g":255,"b":255,"a":255},"width":width,"height":90.0}]),
                240.0,
                Settings::default(),
            );
            h.state_mut().doc["geometry"]["w"] = json!(480.0);
            h.state_mut().state.preview_scale = scale;
            h.run();
            let picker = h.get_by_label("Canvas Color picker").rect();
            let square = h.get_by_label("Color picker saturation-value").rect();
            let hue = h.get_by_label("Color picker hue").rect();
            let bar = h.state().assets.font_height("") * scale;
            assert!((picker.width() - width * scale).abs() < 0.1);
            assert!((square.width() - (width * scale - 2.0 * bar)).abs() < 0.1);
            assert!((square.right() - hue.left()).abs() < 0.1, "No internal spacing");
            assert!((hue.width() - bar).abs() < 0.1, "Hue width follows font height");
            assert!((picker.right() - hue.right() - bar).abs() < 0.1, "Reserve hidden alpha");
        }
    }
}
