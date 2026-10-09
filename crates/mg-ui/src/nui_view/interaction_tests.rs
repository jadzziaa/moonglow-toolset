use super::*;
use egui_kittest::{Harness, kittest::Queryable};

struct Case {
    doc: Value,
    settings: Settings,
    state: State,
    assets: skin::Assets,
}

fn harness(children: Value, settings: Settings) -> Harness<'static, Case> {
    let mut doc = mg_nui::window();
    doc["geometry"]["h"] = 520.0.into();
    doc["root"]["children"] = children;
    Harness::builder().with_size(egui::vec2(900.0, 850.0)).build_ui_state(
        |ui, t: &mut Case| {
            preview::canvas(ui, &mut t.doc, &t.settings, &mut t.state, &mut t.assets)
        },
        Case { doc, settings, state: State::default(), assets: skin::Assets::default() },
    )
}

fn click_at(h: &mut Harness<'_, Case>, pos: egui::Pos2) {
    h.hover_at(pos);
    h.run();
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        h.run();
    }
}

#[test]
fn nui_column_preserves_native_control_widths_when_window_grows() {
    // Native Controls demo: an omitted width is 150, not the window's width.
    let mut h = harness(
        json!([
            {"type":"button","label":"Default","height":30.0},
            {"type":"button","label":"Explicit","width":260.0,"height":30.0},
            {"type":"col","children":[{"type":"button","label":"Nested","height":30.0}]}
        ]),
        Settings::default(),
    );
    h.state_mut().state.zoom = 1.0;
    let original = h.state().doc.clone();
    for width in [420.0, 680.0] {
        h.state_mut().doc["geometry"]["w"] = json!(width);
        h.run();
        for (name, expected) in [("Default", 150.0), ("Explicit", 260.0), ("Nested", 150.0)] {
            let rect = h.get_by_label(&format!("Canvas Button · {name}")).rect();
            assert!((rect.width() - expected).abs() < 0.1, "{name}: {rect:?}");
            assert!((rect.height() - 30.0).abs() < 0.1);
        }
    }
    assert_eq!(h.state().doc["root"], original["root"], "Preview must not rewrite layout");
}

#[test]
fn nui_collapse_button_obeys_window_contract_and_reset() {
    let mut h =
        harness(json!([{"type":"button","label":"Content","height":30.0}]), Settings::default());
    h.state_mut().state.preview_interactive = true;
    let original = h.state().doc.clone();
    h.run();
    h.get_by_label("Collapse preview window").click();
    h.run();
    assert!(h.query_by_label("Canvas Button · Content").is_none());
    h.get_by_label("Expand preview window").click();
    h.run();
    assert!(h.query_by_label("Canvas Button · Content").is_some());
    h.get_by_label("Collapse preview window").click();
    h.run();
    h.get_by_label("Reset").click();
    h.run();
    assert!(h.query_by_label("Canvas Button · Content").is_some());
    assert_eq!(h.state().doc, original);
    h.state_mut().doc["collapsed"] = json!(false);
    h.run();
    assert!(h.query_by_label("Collapse preview window").is_none());
}

#[test]
fn nui_whole_window_view_changes_only_root_and_reset_restores_it() {
    let mut settings = Settings::default();
    let layout = json!({"type":"label","value":"Details"});
    settings.views.insert("details".into(), layout.clone());
    settings.actions.push(mg_nui::Route {
        event: "click".into(),
        element: "details_button".into(),
        action: mg_nui::Action::View { group: "_window_".into(), view: "details".into() },
    });
    let doc = mg_nui::window();
    let mut session = interaction::Session::new(&doc, &settings);
    session.row_values.insert(("old_list".into(), 0), true.into());
    session.dispatch("click", "details_button", None, 0);
    let mut expected = doc.clone();
    expected["root"] = layout;
    assert_eq!(session.doc, expected);
    assert!(session.row_values.is_empty());
    assert_eq!(settings.views["details"]["value"], "Details");
    assert_eq!(interaction::Session::new(&doc, &settings).doc, doc);
}

#[test]
fn nui_declarative_preview_switches_views_watches_and_resets_without_authoring_changes() {
    use mg_nui::{Action as NuiAction, Route};
    let mut s = Settings::default();
    s.bindings.insert("flag".into(), Binding { value: json!(false), ..Default::default() });
    s.bindings.insert("caption".into(), Binding { value: json!("Before"), ..Default::default() });
    s.views.insert("details".into(), json!({"type":"label","value":{"bind":"caption"}}));
    s.actions = vec![
        Route { event: "click".into(), element: "mg_close".into(), action: NuiAction::Close },
        Route {
            event: "click".into(),
            element: "next".into(),
            action: NuiAction::View { group: "host".into(), view: "details".into() },
        },
        Route {
            event: "click".into(),
            element: "toggle".into(),
            action: NuiAction::Toggle { bind: "flag".into() },
        },
        Route {
            event: "watch".into(),
            element: "flag".into(),
            action: NuiAction::Set { bind: "caption".into(), value: json!("After") },
        },
    ];
    let mut h = harness(
        json!([
            {"type":"group","id":"host","children":[{"type":"label","value":"Main"}],"height":80},
            {"type":"button","id":"next","label":"Next view","height":30},
            {"type":"button","id":"toggle","label":"Toggle flag","height":30},
            {"type":"button","id":"mg_close","label":"Close","height":30}
        ]),
        s,
    );
    h.run();
    let original = h.state().doc.clone();
    let settings = h.state().settings.clone();
    h.get_by_label("Interact").click();
    h.run();
    h.get_by_label("Canvas Button · Toggle flag").click();
    h.run();
    h.get_by_label("Canvas Button · Next view").click();
    h.run();
    let r = h.state().state.runtime.as_ref().unwrap();
    assert_eq!(r.doc["root"]["children"][0]["children"][0]["value"], json!({"bind":"caption"}));
    assert_eq!(r.settings.bindings["caption"].value, "After");
    assert_eq!(h.state().doc, original);
    assert_eq!(h.state().settings, settings);
    h.get_by_label("Canvas Button · Close").click();
    h.run();
    assert!(h.state().state.runtime.as_ref().unwrap().closed);
    h.get_by_label("Reset").click();
    h.run();
    let r = h.state().state.runtime.as_ref().unwrap();
    assert!(!r.closed);
    assert_eq!(r.doc, original);
    assert_eq!(r.settings, settings);
}

#[test]
fn nui_screen_position_drag_commits_on_release_and_escape_cancels() {
    let mut h = harness(json!([]), Settings::default());
    h.state_mut().state.screen_preview = true;
    h.state_mut().state.screen_size = egui::vec2(1280.0, 720.0);
    h.run();
    let original = h.state().doc.clone();
    let from = h.get_by_label("Canvas window").rect().center();
    h.hover_at(from);
    h.run();
    h.drag_at(from);
    h.run();
    h.hover_at(from + egui::vec2(30.0, 20.0));
    h.run();
    assert_eq!(h.state().doc, original);
    h.drop_at(from + egui::vec2(30.0, 20.0));
    h.run();
    assert!(h.state().doc["geometry"]["x"].as_f64().unwrap() > 0.0);
    assert_eq!(h.state().doc["geometry"]["w"], original["geometry"]["w"]);
    let moved = h.state().doc.clone();
    let from = h.get_by_label("Canvas window").rect().center();
    h.hover_at(from);
    h.run();
    h.drag_at(from);
    h.run();
    h.hover_at(from + egui::vec2(20.0, 20.0));
    h.run();
    h.key_press(egui::Key::Escape);
    h.run();
    h.drop_at(from + egui::vec2(20.0, 20.0));
    h.run();
    assert_eq!(h.state().doc, moved);
}

#[test]
fn nui_wrapped_preview_toolbar_stays_compact() {
    for width in [360.0, 480.0, 540.0, 640.0, 900.0] {
        for live in [false, true] {
            let mut assets = skin::Assets::default();
            assets.loaded = true;
            let mut h = Harness::builder().with_size(egui::vec2(width, 760.0)).build_ui_state(
                |ui, t: &mut Case| {
                    preview::canvas(ui, &mut t.doc, &t.settings, &mut t.state, &mut t.assets)
                },
                Case {
                    doc: mg_nui::window(),
                    settings: Settings::default(),
                    state: State { preview_interactive: live, ..Default::default() },
                    assets,
                },
            );
            h.run();
            let top = h.get_by_label("Interact").rect().top();
            let canvas = h.state().state.preview_rect.unwrap();
            assert!(
                canvas.top() - top < 110.0,
                "width={width}, live={live}, gap={}",
                canvas.top() - top
            );
            assert!(canvas.height() > 580.0);
        }
    }
}

#[test]
fn nui_bound_window_geometry_is_not_replaced_by_authoring_handles() {
    let mut h = harness(json!([]), Settings::default());
    h.state_mut().doc["geometry"] = json!({"bind":"bounds"});
    h.state_mut().settings.bindings.insert(
        "bounds".into(),
        Binding { value: json!({"x":-1,"y":-1,"w":420,"h":240}), ..Default::default() },
    );
    h.run();
    assert!(h.query_by_label("Resize window width and height").is_none());
    assert_eq!(h.state().doc["geometry"], json!({"bind":"bounds"}));
}

#[test]
fn nui_interact_changes_bind_values_and_inputs_without_editing_the_document() {
    let mut settings = Settings::default();
    settings
        .bindings
        .insert("checked".into(), Binding { value: false.into(), ..Default::default() });
    settings.bindings.insert("name".into(), Binding { value: "".into(), ..Default::default() });
    let mut h = harness(
        json!([
            {"type":"check","label":"Flag","value":{"bind":"checked"},"height":30},
            {"type":"button_select","label":"Toggle","value":false,"height":30},
            {"type":"slider","id":"amount","value":0,"min":0,"max":100,"step":5,"height":30},
            {"type":"textedit","label":"Name","value":{"bind":"name"},"max":8,"height":30},
            {"type":"combo","id":"language","elements":[["Polski",7],["English",9]],"value":7,"height":30},
            {"type":"tabbar","elements":["First","Second"],"value":0,"height":30},
            {"type":"button","id":"run","label":"Run","height":30},
            {"type":"check","label":"Disabled","value":false,"enabled":false,"height":30},
            {"type":"check","label":"Hidden","value":false,"visible":false,"height":30}
        ]),
        settings,
    );
    h.run();
    let original = h.state().doc.clone();
    let original_settings = h.state().settings.clone();
    h.get_by_label("Interact").click();
    h.run();
    h.get_by_label("Canvas Checkbox · Flag").click();
    h.run();
    assert_eq!(h.state().state.runtime.as_ref().unwrap().settings.bindings["checked"].value, true);
    h.get_by_label("Canvas Toggle button · Toggle").click();
    h.run();
    assert_eq!(h.state().state.runtime.as_ref().unwrap().doc["root"]["children"][1]["value"], true);
    let slider = h.get_by_label("Canvas Integer slider").rect();
    click_at(&mut h, slider.left_center() + egui::vec2(slider.width() * 0.75, 0.0));
    assert_eq!(h.state().state.runtime.as_ref().unwrap().doc["root"]["children"][2]["value"], 75);
    h.get_by_role(egui::accesskit::Role::TextInput).click();
    h.run();
    h.event(egui::Event::Text("Zażółć12345".into()));
    h.run();
    assert_eq!(h.state().state.runtime.as_ref().unwrap().settings.bindings["name"].value, "Zażół");
    h.get_by_label("Canvas Dropdown").click();
    h.run();
    h.get_by_label("English").click();
    h.run();
    assert_eq!(h.state().state.runtime.as_ref().unwrap().doc["root"]["children"][4]["value"], 9);
    let tabs = h.get_by_label("Canvas Tabs").rect();
    click_at(&mut h, tabs.right_center() - egui::vec2(12.0, 0.0));
    assert_eq!(h.state().state.runtime.as_ref().unwrap().doc["root"]["children"][5]["value"], 1);
    h.get_by_label("Canvas Button · Run").click();
    h.run();
    assert_eq!(h.state().state.runtime.as_ref().unwrap().last_event, "Local click: run");
    h.get_by_label("Canvas Checkbox · Disabled").click();
    h.run();
    h.get_by_label("Canvas Checkbox · Hidden").click();
    h.run();
    for i in [7, 8] {
        assert_eq!(
            h.state().state.runtime.as_ref().unwrap().doc["root"]["children"][i]["value"],
            false
        );
    }
    assert_eq!(h.state().doc, original);
    assert_eq!(h.state().settings, original_settings);
    h.get_by_label("Reset").click();
    h.run();
    assert_eq!(h.state().state.runtime.as_ref().unwrap().settings, original_settings);
    assert_eq!(h.state().state.runtime.as_ref().unwrap().doc, original);
    h.get_by_label("Interact").click();
    h.run();
    assert!(h.state().state.runtime.is_none());
}

#[test]
fn nui_interact_list_rows_and_disabled_ancestors_are_isolated() {
    let mut settings = Settings::default();
    settings.bindings.insert(
        "rows".into(),
        Binding { value: json!([false, false, false]), ..Default::default() },
    );
    let mut h = harness(
        json!([
            {"type":"list","row_template":[[{"type":"check","label":"Row","value":{"bind":"rows"}},150,true],[{"type":"check","label":"Local","value":false},150,true]],"row_count":3,"row_height":28,"height":100},
            {"type":"col","enabled":false,"children":[{"type":"check","label":"Blocked","value":false,"height":30}]}
        ]),
        settings,
    );
    h.run();
    h.get_by_label("Interact").click();
    h.run();
    h.get_by_label("Canvas Checkbox · Row · row 1").click();
    h.run();
    h.get_by_label("Canvas Checkbox · Local · row 1").click();
    h.run();
    let runtime = h.state().state.runtime.as_ref().unwrap();
    assert_eq!(runtime.settings.bindings["rows"].value, json!([false, true, false]));
    assert_eq!(runtime.row_values.len(), 1);
    assert_eq!(runtime.row_values[&("/root/children/0/row_template/1/0".into(), 1)], true);
    h.get_by_label("Canvas Checkbox · Blocked").click();
    h.run();
    assert_eq!(
        h.state().state.runtime.as_ref().unwrap().doc["root"]["children"][1]["children"][0]["value"],
        false
    );
    // An external authoring change resets stale simulation instead of replaying old binds.
    h.state_mut().doc["title"] = "Changed".into();
    h.run();
    assert_eq!(
        h.state().state.runtime.as_ref().unwrap().settings.bindings["rows"].value,
        json!([false, false, false])
    );
}

#[test]
fn nui_list_scroll_reaches_later_rows_without_changing_source() {
    let mut h = harness(
        json!([{"type":"list","row_template":[[{"type":"label","value":"Entry"},150,true]],"row_count":40,"row_height":25,"height":120,"scrollbars":2}]),
        Settings::default(),
    );
    h.run();
    let before = h.state().doc.clone();
    let scroll = h.get_by_label("Scroll list /root/children/0").rect();
    click_at(&mut h, scroll.center_bottom() - egui::vec2(0.0, 3.0));
    assert!(h.state().state.list_scroll.values().any(|v| *v > 500.0));
    assert!(h.query_by_label("Canvas Label · Entry · row 0").is_none());
    assert!(h.query_by_label("Canvas Label · Entry · row 39").is_some());
    assert_eq!(h.state().doc, before);
}

#[test]
fn nui_group_horizontal_scroll_moves_content_with_fixed_viewport_and_resets() {
    // Native Layout: content translates, frame stays fixed. No pixel/extent parity claim.
    let mut h = harness(
        json!([{
            "type":"group", "width":220, "height":150, "scrollbars":1,
            "children":[{"type":"col","children":[
                {"type":"button","label":"Wide content","width":400,"height":30}
            ]}]
        }]),
        Settings::default(),
    );
    h.state_mut().state.zoom = 1.0;
    h.state_mut().state.preview_interactive = true;
    h.run();
    let original = h.state().doc.clone();
    let viewport = h.get_by_label("Canvas Group").rect();
    let content = h.get_by_label("Canvas Button · Wide content").rect();
    let track = h.get_by_label("Scroll group horizontally /root/children/0").rect();
    click_at(&mut h, track.center());
    let offset = *h.state().state.group_scroll_x.values().next().unwrap();
    assert!(offset > 50.0);
    assert_eq!(h.get_by_label("Canvas Group").rect(), viewport);
    let moved = h.get_by_label("Canvas Button · Wide content").rect();
    assert!((content.left() - moved.left() - offset).abs() < 0.1);
    assert_eq!(h.state().doc, original);
    h.get_by_label("Reset").click();
    h.run();
    assert!(h.state().state.group_scroll_x.values().all(|v| *v == 0.0));
    assert_eq!(h.get_by_label("Canvas Button · Wide content").rect(), content);
}

#[test]
fn nui_text_vertical_scroll_moves_galley_with_fixed_clip_and_resets() {
    // Real Creator Text demo in NWN: Y/BOTH/AUTO scroll; NONE stays fixed.
    let message = "FIRST line\n".repeat(30) + "LAST";
    for mode in [2, 3, 4] {
        let mut h = harness(
            json!([{"type":"text","value":message,"width":300,
            "height":100,"scrollbars":mode}]),
            Settings::default(),
        );
        h.state_mut().state.zoom = 1.0;
        h.state_mut().state.preview_interactive = true;
        h.run();
        let before = h.state().doc.clone();
        let text_position = |h: &Harness<'_, Case>| {
            h.output()
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text().starts_with("FIRST line") => {
                        Some((t.pos, s.clip_rect))
                    }
                    _ => None,
                })
                .unwrap()
        };
        let initial = text_position(&h);
        let track = h.get_by_label("Scroll text vertically /root/children/0").rect();
        click_at(&mut h, track.center());
        let moved = text_position(&h);
        assert!(moved.0.y < initial.0.y - 100.0);
        assert_eq!(moved.1, initial.1, "scroll must not move clipping bounds");
        assert_eq!(h.state().doc, before);
        h.get_by_label("Reset").click();
        h.run();
        assert_eq!(text_position(&h), initial);
        assert!(h.state().state.text_scroll_y.values().all(|offset| *offset == 0.0));
    }
    for (mode, message) in [(0, message.as_str()), (1, message.as_str()), (4, "Short")] {
        let mut h = harness(
            json!([{"type":"text","value":message,"width":300,
            "height":100,"scrollbars":mode}]),
            Settings::default(),
        );
        h.run();
        assert!(h.query_by_label("Scroll text vertically /root/children/0").is_none());
    }
}

#[test]
fn nui_group_horizontal_scroll_respects_disabled_and_no_overflow() {
    let mut h = harness(
        json!([{
            "type":"group", "width":220, "height":150, "scrollbars":1, "enabled":false,
            "children":[{"type":"button","label":"Wide","width":400,"height":30}]
        }]),
        Settings::default(),
    );
    h.state_mut().state.preview_interactive = true;
    h.run();
    let track = h.get_by_label("Scroll group horizontally /root/children/0").rect();
    click_at(&mut h, track.center());
    assert!(h.state().state.group_scroll_x.values().all(|v| *v == 0.0));
    h.state_mut().state.group_scroll_x.insert("/root/children/0/None".into(), 100.0);
    h.state_mut().doc["root"]["children"][0]["children"][0]["width"] = json!(100);
    h.run();
    assert!(h.state().state.group_scroll_x.values().all(|v| *v == 0.0));
}

/// NWN EE 8193.37 (np_list2, np_list3): a list scrolled to its end has
/// moved by whole 29-point rows: 10 rows in 100 to row 9, 40 in 200 to row
/// 35, 40 in 300 to row 32.
#[test]
fn nui_list_scrolls_to_the_rows_the_client_ends_on() {
    for (count, height, last_top_row) in [(10, 100.0, 9.0), (40, 200.0, 35.0), (40, 300.0, 32.0)] {
        let rows: Vec<_> = (0..count).map(|i| format!("Row {i}")).collect();
        let mut settings = Settings::default();
        settings
            .bindings
            .insert("rows".into(), Binding { value: json!(rows), ..Default::default() });
        let mut h = harness(
            json!([{"type":"list","row_template":[[{"type":"label","value":{"bind":"rows"}},0.0,true]],
                "row_count":{"bind":"rows"},"row_height":25.0,"border":true,"scrollbars":2,
                "width":200.0,"height":height}]),
            settings,
        );
        h.state_mut().state.zoom = 1.0;
        h.state_mut().state.preview_interactive = true;
        h.run();
        let track = h.get_by_label("Scroll list /root/children/0").rect();
        click_at(&mut h, egui::pos2(track.center().x, track.bottom() - 2.0));
        let offset = *h.state().state.list_scroll.values().next().unwrap();
        assert_eq!(offset, last_top_row * 29.0, "{count} rows in {height}");
    }
}

/// NWN EE 8193.37 (np_chart3): columns as Nuklear's nk_chart_push_column
/// draws them, quirks included. Positions as fractions of the chart's height
/// from its top: [1,2,3] all hang from the top (the first down to the middle),
/// [5,-2,0,4] stand on a zero line at 5/7.
#[test]
fn nui_chart_columns_follow_the_clients_formula() {
    let columns = |data: Value| {
        let mut h = harness(
            json!([{"type":"chart","width":300.0,"height":200.0,"value":[
                {"type":1,"legend":"","color":{"r":10,"g":20,"b":30,"a":255},"data":data}]}]),
            Settings::default(),
        );
        h.state_mut().state.zoom = 1.0;
        h.run();
        let chart = h.get_by_label("Canvas Chart").rect();
        let colour = egui::Color32::from_rgb(10, 20, 30);
        let mut rects: Vec<_> = h
            .output()
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Rect(r) if r.fill == colour => Some(r.rect),
                _ => None,
            })
            .collect();
        rects.sort_by(|a, b| a.left().total_cmp(&b.left()));
        (chart, rects)
    };
    let (chart, rects) = columns(json!([1.0, 2.0, 3.0]));
    assert_eq!(rects.len(), 3);
    // Inside the chart's own 4-point inset.
    let (top, height) = (chart.top() + 4.0, chart.height() - 8.0);
    let at = |y: f32| (y - top) / height;
    for (r, (t, b)) in rects.iter().zip([(0.0, 0.5), (-0.5, 0.5), (-1.0, 0.5)]) {
        assert!((at(r.top()) - t).abs() < 0.01 && (at(r.bottom()) - b).abs() < 0.01, "{r:?}");
    }
    let (_, rects) = columns(json!([5.0, -2.0, 0.0, 4.0]));
    let zero = 5.0 / 7.0;
    for (r, (t, b)) in rects.iter().zip([(0.0, zero), (zero, 1.0), (zero, zero), (1.0 / 7.0, zero)])
    {
        assert!((at(r.top()) - t).abs() < 0.01 && (at(r.bottom()) - b).abs() < 0.01, "{r:?}");
    }
    // Equal values: no range, nothing drawn.
    assert!(columns(json!([2.0, 2.0, 2.0])).1.is_empty());
}

/// NWN EE 8193.37 (np_margin, np_gap): a margin m moves a control m - 2
/// from where it stands with none (0 and 1 move it back), keeping its width;
/// a button with no height is 50 high.
#[test]
fn nui_margins_move_controls_by_their_excess_over_the_default() {
    let button = |label: &str, margin: Option<f64>| {
        let mut b = json!({"type":"button","label":label,"height":30.0});
        if let Some(m) = margin {
            b["margin"] = json!(m);
        }
        b
    };
    let mut h = harness(
        json!([button("none", None), button("m0", Some(0.0)), button("m1", Some(1.0)),
            button("m3", Some(3.0)), button("m10", Some(10.0)), {"type":"button","label":"auto"}]),
        Settings::default(),
    );
    h.state_mut().state.zoom = 1.0;
    h.run();
    let at =
        |h: &Harness<'_, Case>, l: &str| h.get_by_label(&format!("Canvas Button · {l}")).rect();
    let none = at(&h, "none");
    for (label, shift) in [("m0", -2.0), ("m1", -1.0), ("m3", 1.0), ("m10", 8.0)] {
        let r = at(&h, label);
        assert!((r.left() - none.left() - shift).abs() < 0.1, "{label}: {r:?} vs {none:?}");
        assert!((r.width() - none.width()).abs() < 0.1, "{label}");
    }
    // Between two controls: their default margins (4), or what a margin adds.
    assert!((at(&h, "m0").top() - at(&h, "none").bottom() - 2.0).abs() < 0.1);
    assert!((at(&h, "m10").top() - at(&h, "m3").bottom() - 13.0).abs() < 0.1);
    assert_eq!(at(&h, "auto").height(), 50.0);
}

/// NWN EE 8193.37 (nui_orient_s): the root column fills its window, and a
/// Row with nothing in it takes what the controls leave, pushing those after
/// it to the bottom.
#[test]
fn nui_an_empty_row_takes_the_room_a_column_has_left() {
    let mut h = harness(
        json!([{"type":"button","label":"top","height":30.0},
            {"type":"row","children":[]},
            {"type":"button","label":"bottom","height":30.0}]),
        Settings::default(),
    );
    h.state_mut().state.zoom = 1.0;
    h.run();
    let column = h.get_by_label("Canvas Column").rect();
    let bottom = h.get_by_label("Canvas Button · bottom").rect();
    assert!((column.bottom() - bottom.bottom()).abs() < 4.0, "{column:?} {bottom:?}");
    let row = h.get_by_label("Canvas Row").rect();
    assert!(row.height() > 300.0, "{row:?}");
}

/// NWN EE 8193.37, Text of widths 200, 360 and 560 with X scrolling: its
/// bar moves the text 36 to the left at the end, whatever the text; the
/// lines don't rewrap. A short text has nothing to scroll in Y.
#[test]
fn nui_text_scrolls_its_measured_extents() {
    let message = "FIRST 01 Lorem ipsum dolor sit amet. 02 Long text checks wrapping.";
    for width in [200.0, 360.0] {
        let mut h = harness(
            json!([{"type":"text","value":message,"width":width,"height":90,"scrollbars":1}]),
            Settings::default(),
        );
        h.state_mut().state.zoom = 1.0;
        h.state_mut().state.preview_interactive = true;
        h.run();
        let track = h.get_by_label("Scroll text horizontally /root/children/0").rect();
        // The increment arrow, until the end.
        for _ in 0..4 {
            click_at(&mut h, egui::pos2(track.right() - 4.0, track.center().y));
        }
        let offset = *h.state().state.text_scroll_x.values().next().unwrap();
        assert_eq!(offset, 36.0, "{width}");
    }
    let mut h = harness(
        json!([{"type":"text","value":"Short text, one line.","width":360,"height":90,"scrollbars":2}]),
        Settings::default(),
    );
    h.state_mut().state.zoom = 1.0;
    h.state_mut().state.preview_interactive = true;
    h.run();
    let track = h.get_by_label("Scroll text vertically /root/children/0").rect();
    click_at(&mut h, egui::pos2(track.center().x, track.bottom() - 20.0));
    assert_eq!(*h.state().state.text_scroll_y.values().next().unwrap(), 0.0);
}

#[test]
fn nui_group_scroll_translates_text_without_rewrapping_at_viewport_edge() {
    let message = "Nested group: this longer sentence must move with its content.";
    let mut h = harness(
        json!([{
            "type":"group", "width":220, "height":150, "scrollbars":1,
            "children":[{"type":"col","children":[
                {"type":"text","value":message,"width":400,"height":90,"scrollbars":0}
            ]}]
        }]),
        Settings::default(),
    );
    h.state_mut().state.zoom = 1.0;
    h.run();
    let rendered = |h: &Harness<'_, Case>| {
        h.output()
            .shapes
            .iter()
            .find_map(|shape| {
                // Lines break where the client's do: as line breaks in the job.
                if let egui::Shape::Text(text) = &shape.shape
                    && text.galley.job.text.replace('\n', "") == message
                {
                    Some((text.pos, text.galley.size(), shape.clip_rect))
                } else {
                    None
                }
            })
            .expect("Text must be painted")
    };
    let (before, size, clip) = rendered(&h);
    let viewport = h.get_by_label("Canvas Group").rect();
    let track = h.get_by_label("Scroll group horizontally /root/children/0").rect();
    click_at(&mut h, track.center());
    let offset = *h.state().state.group_scroll_x.values().next().unwrap();
    let (after, new_size, new_clip) = rendered(&h);
    assert!(offset > 50.0);
    assert!((before.x - after.x - offset).abs() < 0.1);
    assert_eq!(size, new_size, "Scrolling cannot change text wrapping");
    assert_eq!(h.get_by_label("Canvas Group").rect(), viewport);
    assert_eq!(clip.right(), new_clip.right());
    assert!(viewport.contains_rect(new_clip), "Text cannot paint outside Group");
    // The text's own left padding may scroll out of view; the parent clip cannot.
    assert!(new_clip.left() >= viewport.left());
}

#[test]
fn nui_vertical_tabs_keep_native_natural_width_and_explicit_width_override() {
    // GUI-authored nui_orient_s: two native vertical Toggles span the same
    // 304 units as the horizontal pair; the previous preview narrowed to 150.
    for width in [None, Some(220.0)] {
        let mut node = json!({"type":"tabbar", "elements":["First", "Second"],
            "direction":1, "height":90, "value":0});
        if let Some(width) = width {
            node["width"] = json!(width);
        }
        let mut h = harness(json!([node]), Settings::default());
        h.state_mut().state.zoom = 1.0;
        h.run();
        let tabs = h.get_by_label("Canvas Tabs").rect();
        assert!((tabs.width() - width.unwrap_or(304.0)).abs() < 0.1);
    }
}

#[test]
fn nui_native_choice_rows_do_not_stretch_and_empty_space_is_inert() {
    // Native Orientations height90/150: same39px pitch, unused lower area inert.
    for ty in ["options", "tabbar"] {
        for height in [90, 150] {
            let mut h = harness(
                json!([{"type":ty,"elements":["First","Second"],
                "direction":1,"height":height,"value":0}]),
                Settings::default(),
            );
            h.state_mut().state.zoom = 1.0;
            h.state_mut().state.preview_interactive = true;
            h.run();
            let original = h.state().doc.clone();
            let bounds = h
                .get_by_label(if ty == "options" { "Canvas Options" } else { "Canvas Tabs" })
                .rect();
            let value = |h: &Harness<'_, Case>| {
                h.state().state.runtime.as_ref().unwrap().doc["root"]["children"][0]["value"]
                    .clone()
            };
            click_at(&mut h, bounds.left_top() + egui::vec2(50.0, 85.0));
            assert_eq!(value(&h), 0, "Reserved blank height must not select an entry");
            click_at(&mut h, bounds.left_top() + egui::vec2(50.0, 37.0));
            assert_eq!(value(&h), 0, "The gap between entries is not an option");
            click_at(&mut h, bounds.left_top() + egui::vec2(50.0, 56.0));
            assert_eq!(value(&h), 1, "Native second entry stays at the same vertical position");
            click_at(&mut h, bounds.left_top() + egui::vec2(50.0, 17.0));
            assert_eq!(value(&h), 0);
            assert_eq!(h.state().doc, original);
        }
    }
}

#[test]
fn nui_native_vertical_tabs_remain_visible_and_clickable_past_short_allocation() {
    // Native height30: second tab remains visible below y+39 and changes the bind.
    let mut h = harness(
        json!([{"type":"tabbar","elements":["First","Second"],
        "direction":1,"height":30,"value":0}]),
        Settings::default(),
    );
    h.state_mut().state.zoom = 1.0;
    h.state_mut().state.preview_interactive = true;
    h.run();
    let original = h.state().doc.clone();
    let bounds = h.get_by_label("Canvas Tabs").rect();
    let painted_second = h.output().shapes.iter().any(|shape| {
        if let egui::Shape::Text(text) = &shape.shape {
            text.galley.job.text == "Second"
                && text.pos.y > bounds.top() + 39.0
                && shape.clip_rect.bottom() > bounds.top() + 60.0
        } else {
            false
        }
    });
    assert!(painted_second, "Second entry cannot be squeezed or clipped to height30");
    click_at(&mut h, bounds.left_top() + egui::vec2(60.0, 56.0));
    assert_eq!(h.state().state.runtime.as_ref().unwrap().doc["root"]["children"][0]["value"], 1);
    assert_eq!(h.state().doc, original);
}

#[test]
fn nui_native_horizontal_tabs_cap_row_height_and_preserve_reserved_space() {
    let mut h = harness(
        json!([
            {"type":"tabbar","elements":["First","Second"],"height":60,"value":0},
            {"type":"button","label":"After","height":30}
        ]),
        Settings::default(),
    );
    h.state_mut().state.zoom = 1.0;
    h.state_mut().state.preview_interactive = true;
    h.run();
    let tabs = h.get_by_label("Canvas Tabs").rect();
    let after = h.get_by_label("Canvas Button · After").rect();
    assert!(
        (after.top() - tabs.top() - 60.0).abs() < 0.1,
        "Choice block already includes row spacing"
    );
    click_at(&mut h, tabs.left_top() + egui::vec2(200.0, 50.0));
    assert_eq!(h.state().state.runtime.as_ref().unwrap().doc["root"]["children"][0]["value"], 0);
    click_at(&mut h, tabs.left_top() + egui::vec2(200.0, 17.0));
    assert_eq!(h.state().state.runtime.as_ref().unwrap().doc["root"]["children"][0]["value"], 1);
}

#[test]
fn nui_native_short_options_overlap_next_tabs_and_use_full_natural_width() {
    // NWN (370,385) updates BOTH Options and following horizontal Tabs;
    // then (370,354), beyond the old150px width, resets Options only.
    let mut h = harness(
        json!([
            {"type":"options","elements":["V first","V second"],"direction":1,"height":30,"value":0},
            {"type":"tabbar","elements":["H first","H second"],"height":60,"value":0}
        ]),
        Settings::default(),
    );
    h.state_mut().state.zoom = 1.0;
    h.state_mut().state.preview_interactive = true;
    h.run();
    let original = h.state().doc.clone();
    let options = h.get_by_label("Canvas Options").rect();
    assert!((options.width() - 304.0).abs() < 0.1);
    click_at(&mut h, options.left_top() + egui::vec2(227.0, 48.0));
    let rows = &h.state().state.runtime.as_ref().unwrap().doc["root"]["children"];
    assert_eq!(rows[0]["value"], 1);
    assert_eq!(rows[1]["value"], 1);
    click_at(&mut h, options.left_top() + egui::vec2(227.0, 17.0));
    let rows = &h.state().state.runtime.as_ref().unwrap().doc["root"]["children"];
    assert_eq!(rows[0]["value"], 0);
    assert_eq!(rows[1]["value"], 1);
    assert_eq!(h.state().doc, original);
}

#[test]
fn nui_label_alignment_uses_full_native_bounds_and_preserves_rgb() {
    // GUI-authored Alignment demo: top text was 2px too low and bottom
    // text 2-3px too high. Native middle alignment and RGB matched.
    for horizontal in 0..3 {
        let children: Vec<_> = (0..3)
            .map(|vertical| {
                json!({
                    "type":"label", "value":format!("Align {horizontal}/{vertical}"),
                    "width":300, "height":60,
                    "text_halign":horizontal, "text_valign":vertical,
                    "foreground_color":{"r":64,"g":128,"b":255,"a":255}
                })
            })
            .collect();
        let mut h = harness(json!(children), Settings::default());
        h.state_mut().state.zoom = 1.0;
        let source = h.state().doc.clone();
        h.run();
        for vertical in 0..3 {
            let message = format!("Align {horizontal}/{vertical}");
            let rect = h.get_by_label(&format!("Canvas Label · {message}")).rect();
            let (position, size, color) = h
                .output()
                .shapes
                .iter()
                .find_map(|shape| {
                    if let egui::Shape::Text(text) = &shape.shape
                        && text.galley.job.text == message
                    {
                        Some((
                            text.pos,
                            text.galley.size(),
                            text.galley.job.sections[0].format.color,
                        ))
                    } else {
                        None
                    }
                })
                .expect("Label must be painted");
            let x = match horizontal {
                1 => rect.left(),
                2 => rect.right() - size.x,
                _ => rect.center().x - size.x / 2.0,
            };
            let y = match vertical {
                1 => rect.top(),
                2 => rect.bottom() - size.y,
                _ => rect.center().y - size.y / 2.0,
            };
            assert!((position.x - x).abs() < 0.1, "horizontal {horizontal}: {position:?}/{rect:?}");
            assert!((position.y - y).abs() < 0.1, "vertical {vertical}: {position:?}/{rect:?}");
            assert_eq!(color, egui::Color32::from_rgb(64, 128, 255));
        }
        assert_eq!(h.state().doc, source);
    }
}

#[test]
fn nui_interactive_textedit_keeps_native_vertical_alignment() {
    for multiline in [false, true] {
        let mut h = harness(
            json!([{
                "type":"textedit","label":"Alignment","value":"Native alignment",
                "multiline":multiline,"width":360,"height":90
            }]),
            Settings::default(),
        );
        h.state_mut().state.zoom = 1.0;
        h.state_mut().state.preview_interactive = true;
        h.run();
        let rect = h.get_by_label("Canvas Text input · Alignment").rect();
        let (position, size) = h
            .output()
            .shapes
            .iter()
            .find_map(|s| {
                if let egui::Shape::Text(t) = &s.shape
                    && t.galley.job.text == "Native alignment"
                {
                    Some((t.pos, t.galley.size()))
                } else {
                    None
                }
            })
            .unwrap();
        if multiline {
            assert!((position.y - rect.top()).abs() < 8.0);
        } else {
            assert!((position.y + size.y / 2.0 - rect.center().y).abs() < 2.0);
        }
    }
}

#[test]
fn nui_textedit_limit_counts_utf8_bytes_and_rejects_partial_codepoints() {
    // Native nui_edit_s: 127 bytes rejects ą but accepts Q. Both cases must
    // preserve existing text; do not trim the suffix when inserting at Home.
    for multiline in [false, true] {
        let initial = "A".repeat(123) + "Żółć"; // 131 bytes from a server bind
        let mut settings = Settings::default();
        settings.bindings.insert(
            "value".into(),
            Binding { value: initial.clone().into(), ..Default::default() },
        );
        let mut h = harness(
            json!([{
                "type":"textedit", "label":"Byte budget", "value":{"bind":"value"},
                "max":128,"multiline":multiline,"wordwrap":false,"width":400,"height":90
            }]),
            settings,
        );
        h.state_mut().state.preview_interactive = true;
        h.run();
        let source = h.state().doc.clone();
        h.get_by_role(if multiline {
            egui::accesskit::Role::MultilineTextInput
        } else {
            egui::accesskit::Role::TextInput
        })
        .click();
        h.run();
        let value = |h: &Harness<'_, Case>| {
            h.state().state.runtime.as_ref().unwrap().settings.bindings["value"]
                .value
                .as_str()
                .unwrap()
                .to_owned()
        };
        // An oversized existing bind is preserved until edited, never silently truncated.
        assert_eq!(value(&h), initial);
        h.key_press(egui::Key::End);
        h.run();
        h.key_press(egui::Key::Backspace);
        h.run();
        h.key_press(egui::Key::Backspace);
        h.run();
        assert_eq!(value(&h).len(), 127);
        h.key_press(egui::Key::Home);
        h.run();
        let before = value(&h);
        h.event(egui::Event::Text("ą".into()));
        h.run();
        assert_eq!(value(&h), before);
        h.event(egui::Event::Text("Q".into()));
        h.run();
        assert_eq!(value(&h).len(), 128);
        assert!(value(&h).starts_with('Q'));
        assert!(value(&h).ends_with("Żó"));
        assert_eq!(h.state().doc, source);
    }
}

#[test]
fn nui_textedit_wordwrap_preserves_explicit_lines_and_clips_overflow() {
    // GUI-authored Edit demo exposed that wordwrap was ignored. This checks
    // the local property contract, not native NWN pixel/keyboard parity.
    let message = "A long sentence with several words that exceeds the field width.\nSecond line.";
    for interactive in [false, true] {
        for wrap in [false, true] {
            let mut h = harness(
                json!([{
                    "type":"textedit", "label":"Wrapping probe", "value":message,
                    "multiline":true, "wordwrap":wrap, "max":128,
                    "width":160, "height":160
                }]),
                Settings::default(),
            );
            h.state_mut().state.zoom = 1.0;
            h.state_mut().state.preview_interactive = interactive;
            let source = h.state().doc.clone();
            h.run();
            let bounds = h.get_by_label("Canvas Text input · Wrapping probe").rect();
            let (rows, clip) = h
                .output()
                .shapes
                .iter()
                .find_map(|shape| {
                    if let egui::Shape::Text(text) = &shape.shape
                        && text.galley.job.text == message
                    {
                        Some((text.galley.rows.len(), shape.clip_rect))
                    } else {
                        None
                    }
                })
                .expect("The editable text must be painted");
            if wrap {
                assert!(rows > 2, "interactive={interactive}: text must wrap");
            } else {
                assert_eq!(rows, 2, "interactive={interactive}: only explicit newline may wrap");
            }
            assert!(
                bounds.contains_rect(clip),
                "interactive={interactive}: overflow must be clipped to the control: {clip:?}/{bounds:?}"
            );
            assert_eq!(h.state().doc, source, "Preview must not rewrite the authored input");
        }
    }
}

#[test]
fn nui_picker_hue_preserves_brightness_and_writes_opaque_alpha() {
    // A hue-only edit must not progressively darken unmultiplied RGB.
    // Exercise the actual canvas/runtime, including fully transparent colors.
    for alpha in [128, 64, 0, 255] {
        let initial = json!({"r":255,"g":64,"b":64,"a":alpha});
        let mut settings = Settings::default();
        settings
            .bindings
            .insert("color".into(), Binding { value: initial.clone(), ..Default::default() });
        let mut h = harness(
            json!([
                {"type":"color_picker","id":"picker","width":360,"height":100,"value":{"bind":"color"}}
            ]),
            settings,
        );
        h.state_mut().state.preview_interactive = true;
        h.run();
        let source = h.state().doc.clone();
        let source_settings = h.state().settings.clone();
        let hue = h.get_by_label("Color picker hue").rect();
        for fraction in [0.5, 0.25, 0.75] {
            click_at(&mut h, egui::pos2(hue.center().x, hue.top() + fraction * hue.height()));
            let color = &h.state().state.runtime.as_ref().unwrap().settings.bindings["color"].value;
            let rgb = [
                color["r"].as_u64().unwrap(),
                color["g"].as_u64().unwrap(),
                color["b"].as_u64().unwrap(),
            ];
            assert_eq!(
                *rgb.iter().max().unwrap(),
                255,
                "Hue changed brightness: alpha={alpha}, color={color}"
            );
            assert!(rgb.iter().min().unwrap().abs_diff(64) <= 1, "Hue changed saturation: {color}");
            // NWN EE 8193.37: the picker has no alpha and writes 255.
            assert_eq!(color["a"], 255);
        }
        assert_eq!(h.state().doc, source);
        assert_eq!(h.state().settings, source_settings);
        h.get_by_label("Reset").click();
        h.run();
        // Opened anew: the authored colour, made opaque as the client's picker does.
        let mut opaque = initial.clone();
        opaque["a"] = json!(255);
        assert_eq!(
            h.state().state.runtime.as_ref().unwrap().settings.bindings["color"].value,
            opaque
        );
    }
}

#[test]
fn nui_close_button_requires_explicit_action_and_removal_stops_closing() {
    let mut h = harness(
        json!([{"type":"button","id":"mg_close","label":"Close","height":30}]),
        Settings::default(),
    );
    h.state_mut().state.preview_interactive = true;
    h.run();
    h.get_by_label("Canvas Button · Close").click();
    h.run();
    assert!(!h.state().state.runtime.as_ref().unwrap().closed);
    h.state_mut().settings.actions.push(mg_nui::Route {
        event: "click".into(),
        element: "mg_close".into(),
        action: mg_nui::Action::Close,
    });
    h.run();
    h.get_by_label("Canvas Button · Close").click();
    h.run();
    assert!(h.state().state.runtime.as_ref().unwrap().closed);
    h.state_mut().settings.actions.clear();
    h.run();
    h.get_by_label("Canvas Button · Close").click();
    h.run();
    assert!(!h.state().state.runtime.as_ref().unwrap().closed);
}

#[test]
fn nui_native_dropdown_corrects_missing_selection_without_clicking_or_editing_source() {
    // GUI-created nui_choices_s, NWN EE89.8193.37-17: empty retains -1,
    // singleton selects 42, multiple entries select the first ID. Clear/restore
    // retains a valid ID. A correction is not a synthetic click.
    for (entries, expected) in [
        (json!([]), json!(-1)),
        (json!([["Only choice 42", 42]]), json!(42)),
        (json!([["First", 0], ["Second", 1]]), json!(0)),
    ] {
        let mut settings = Settings::default();
        for (name, value) in [("options", entries.clone()), ("choice", json!(-1))] {
            settings.bindings.insert(name.into(), Binding { value, ..Default::default() });
        }
        settings.actions.push(mg_nui::Route {
            event: "click".into(),
            element: "dropdown".into(),
            action: mg_nui::Action::Close,
        });
        let mut h = harness(
            json!([{
                "type":"combo", "id":"dropdown", "height":30,
                "elements":{"bind":"options"}, "value":{"bind":"choice"}
            }]),
            settings,
        );
        let source = h.state().doc.clone();
        let settings = h.state().settings.clone();
        h.state_mut().state.preview_interactive = true;
        h.run();
        let runtime = h.state().state.runtime.as_ref().unwrap();
        assert_eq!(runtime.settings.bindings["choice"].value, expected);
        assert!(!runtime.closed, "normalization must not run the click route");
        if entries.as_array().unwrap().len() == 2 {
            let runtime = h.state_mut().state.runtime.as_mut().unwrap();
            runtime.settings.bindings.get_mut("choice").unwrap().value = json!(1);
            runtime.settings.bindings.get_mut("options").unwrap().value = json!([]);
            h.run();
            assert_eq!(
                h.state().state.runtime.as_ref().unwrap().settings.bindings["choice"].value,
                1
            );
            h.state_mut()
                .state
                .runtime
                .as_mut()
                .unwrap()
                .settings
                .bindings
                .get_mut("options")
                .unwrap()
                .value = entries;
            h.run();
            assert_eq!(
                h.state().state.runtime.as_ref().unwrap().settings.bindings["choice"].value,
                1
            );
            h.state_mut()
                .state
                .runtime
                .as_mut()
                .unwrap()
                .settings
                .bindings
                .get_mut("choice")
                .unwrap()
                .value = json!(-1);
            h.run();
            assert_eq!(
                h.state().state.runtime.as_ref().unwrap().settings.bindings["choice"].value,
                0
            );
        }
        assert!(!h.state().state.runtime.as_ref().unwrap().closed);
        assert_eq!(h.state().doc, source);
        assert_eq!(h.state().settings, settings);
    }
}
