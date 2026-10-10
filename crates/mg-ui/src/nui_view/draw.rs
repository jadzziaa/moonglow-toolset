//! Stock draw-list primitives and a local coordinate editor.
use super::preview::{draw_image, flag, num, rgba, string, text, val};
use super::*;
use egui::{Color32, Pos2, Rect, Stroke, Vec2, pos2, vec2};

pub(super) const KINDS: &[&str] =
    &["Polyline", "Curve", "Circle", "Arc", "Text", "Image", "Line", "Rectangle"];

fn template(kind: usize) -> Value {
    let mut item = json!({"type":kind,"enabled":true,"color":{"r":220,"g":180,"b":80,"a":255},"fill":false,"line_thickness":2.0,"order":1,"render":0,"arrayBinds":false});
    match kind {
        0 => item["points"] = json!([10.0, 10.0, 80.0, 70.0, 140.0, 10.0]),
        1 | 6 => {
            item["a"] = json!({"x":10.0,"y":10.0});
            item["b"] = json!({"x":150.0,"y":80.0});
            if kind == 1 {
                item["ctrl0"] = json!({"x":70.0,"y":0.0});
                item["ctrl1"] = json!({"x":90.0,"y":100.0});
            }
        }
        3 => {
            item["c"] = json!({"x":80.0,"y":70.0});
            item["radius"] = json!(50.0);
            item["amin"] = json!(0.0);
            item["amax"] = json!(std::f32::consts::PI);
        }
        _ => {
            item["rect"] = json!({"x":10.0,"y":10.0,"w":140.0,"h":70.0});
        }
    }
    if kind == 4 {
        item["text"] = "Text".into();
        item["font"] = "".into();
    }
    if kind == 5 {
        // Stock NuiDrawListImage has no tint/fill/stroke arguments.
        item["color"] = Value::Null;
        item["fill"] = Value::Null;
        item["line_thickness"] = Value::Null;
        item["image"] = "".into();
        item["image_aspect"] = 0.into();
        item["image_halign"] = 0.into();
        item["image_valign"] = 0.into();
    }
    item
}

fn coord(v: &Value, s: &Settings, row: Option<usize>) -> Vec2 {
    vec2(num(&v["x"], s, row, 0.0), num(&v["y"], s, row, 0.0))
}

#[test]
fn native_draw_circle_uses_width_radius_and_outline_arc_closes_chord() {
    // Exact geometry from Creator-authored nui_draw_a, observed in NWN
    // 89.8193.37-17. Test paint output, not just a geometry helper.
    let ctx = egui::Context::default();
    let s = Settings::default();
    let assets = skin::Assets::default();
    for kind in [2, 3] {
        let item = template(kind);
        let mut output = ctx.run_ui(Default::default(), |ui| {
            paint(
                &ui.ctx().layer_painter(egui::LayerId::background()),
                &json!({"draw_list":[item]}),
                Rect::from_min_size(pos2(100.0, 100.0), vec2(200.0, 110.0)),
                1.0,
                &s,
                &assets,
                None,
                false,
                [false; 4],
            );
        });
        output.textures_delta.clear();
        let path = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::Shape::Path(path) => Some(path),
                _ => None,
            })
            .unwrap();
        assert!(path.closed, "native outline must close, kind {kind}");
        assert_eq!(path.fill, Color32::TRANSPARENT);
        let bounds = Rect::from_points(&path.points);
        if kind == 2 {
            assert_eq!(bounds.center(), pos2(180.0, 145.0));
            assert_eq!(bounds.size(), vec2(140.0, 140.0));
        } else {
            assert!((bounds.width() - 100.0).abs() < 0.01);
            assert!((bounds.height() - 50.0).abs() < 0.01);
        }
    }
}
fn rect(v: &Value, s: &Settings, row: Option<usize>, origin: Pos2, scale: f32) -> Rect {
    Rect::from_min_size(
        origin + coord(v, s, row) * scale,
        vec2(num(&v["w"], s, row, 0.0), num(&v["h"], s, row, 0.0)) * scale,
    )
}

/// Keep bind formatting metadata while resolving one repeated drawing's array values.
fn instance_settings(item: &Value, settings: &Settings, index: usize) -> Settings {
    let bindings = mg_nui::bind_names(item)
        .into_iter()
        .filter_map(|name| {
            let binding = settings.bindings.get(&name)?;
            let value = if binding.value.is_array() {
                binding.value.get(index).cloned().unwrap_or(Value::Null)
            } else {
                binding.value.clone()
            };
            Some((name, Binding { value, ..Default::default() }))
        })
        .collect();
    Settings { bindings, ..Default::default() }
}

#[test]
fn repeated_draw_text_preserves_bind_formatting() {
    let mut settings = Settings::default();
    for (name, value) in [
        ("values", json!([15, 255])),
        ("decimals", json!([1.25, 2.75])),
        ("labels", json!(["Mixed", "Case"])),
    ] {
        settings.bindings.insert(name.into(), Binding { value, ..Default::default() });
    }
    for (bind, flags, expected) in [
        ("values", json!({"number_flags":1,"text_flags":2}), ["F", "FF"]),
        ("decimals", json!({"number_precision":2}), ["1.25", "2.75"]),
        ("labels", json!({"text_flags":1}), ["mixed", "case"]),
    ] {
        let mut text = flags;
        text["bind"] = bind.into();
        let item = json!({"type":4,"arrayBinds":true,"text":text});
        for (index, expected) in expected.iter().enumerate() {
            let instance = instance_settings(&item, &settings, index);
            assert_eq!(string(&item["text"], &instance, None), *expected);
        }
    }
    assert_eq!(settings.bindings["values"].value, json!([15, 255]));
}

#[allow(clippy::too_many_arguments)]
pub(super) fn paint(
    p: &egui::Painter,
    host: &Value,
    bounds: Rect,
    scale: f32,
    s: &Settings,
    assets: &skin::Assets,
    row: Option<usize>,
    before: bool,
    mouse: [bool; 4],
) {
    let Some(items) = host["draw_list"].as_array() else { return };
    // Clip to control clips nothing in the client (NWN EE 8193.37: a scissored
    // rectangle draws past its spacer); the window it blanks is an error.
    let p = p.clone();
    for item in items {
        if (item["order"].as_i64().unwrap_or(1) < 0) != before {
            continue;
        }
        if item["arrayBinds"] == true {
            // Array binds hold one value per list row, as other binds in a
            // list do: the client draws the item once, with this row's values
            // (the first ones outside a list), not once per value (NWN EE
            // 8193.37, np_drawidx: three rectangles bound, the first drawn).
            let instance = instance_settings(item, s, row.unwrap_or(0));
            let mut copy = item.clone();
            copy["arrayBinds"] = false.into();
            paint(
                &p,
                &json!({"draw_list":[copy]}),
                bounds,
                scale,
                &instance,
                assets,
                None,
                before,
                mouse,
            );
            continue;
        }
        if !flag(&item["enabled"], s, row, true) {
            continue;
        }
        let visible = match item["render"].as_i64().unwrap_or(0) {
            0 => true,
            1 => !mouse[0],
            2 => mouse[0],
            3 => mouse[0] && mouse[1],
            4 => mouse[0] && mouse[2],
            5 => mouse[0] && mouse[3],
            _ => false,
        };
        if !visible {
            continue;
        }
        let color = rgba(val(&item["color"], s, row), Color32::WHITE);
        let stroke = Stroke::new(num(&item["line_thickness"], s, row, 1.0).max(0.0) * scale, color);
        let fill = if flag(&item["fill"], s, row, false) { color } else { Color32::TRANSPARENT };
        let point = |key: &str| bounds.min + coord(val(&item[key], s, row), s, None) * scale;
        let r = rect(val(&item["rect"], s, row), s, None, bounds.min, scale);
        let shape = |points: Vec<Pos2>, closed: bool| {
            p.add(egui::epaint::PathShape {
                points,
                closed,
                fill: if closed { fill } else { Color32::TRANSPARENT },
                stroke: stroke.into(),
            });
        };
        match item["type"].as_u64().unwrap_or(99) {
            0 => {
                if let Some(points) = val(&item["points"], s, row).as_array() {
                    shape(
                        points
                            .as_chunks::<2>()
                            .0
                            .iter()
                            .map(|xy| {
                                bounds.min
                                    + vec2(
                                        xy[0].as_f64().unwrap_or(0.0) as f32,
                                        xy[1].as_f64().unwrap_or(0.0) as f32,
                                    ) * scale
                            })
                            .collect(),
                        fill != Color32::TRANSPARENT,
                    );
                }
            }
            1 => {
                let a = point("a");
                let b = point("b");
                let c = point("ctrl0");
                let d = point("ctrl1");
                shape(
                    (0..=48)
                        .map(|i| {
                            let t = i as f32 / 48.0;
                            let u = 1.0 - t;
                            pos2(
                                u * u * u * a.x
                                    + 3.0 * u * u * t * c.x
                                    + 3.0 * u * t * t * d.x
                                    + t * t * t * b.x,
                                u * u * u * a.y
                                    + 3.0 * u * u * t * c.y
                                    + 3.0 * u * t * t * d.y
                                    + t * t * t * b.y,
                            )
                        })
                        .collect(),
                    false,
                );
            }
            2 => shape(
                (0..=64)
                    .map(|i| {
                        let t = i as f32 / 64.0 * std::f32::consts::TAU;
                        // Stock Circle uses the rectangle center and width
                        // as diameter, including non-square rects (native DrawA).
                        r.center() + vec2(t.cos(), t.sin()) * r.width() / 2.0
                    })
                    .collect(),
                true,
            ),
            3 => {
                let a = num(&item["amin"], s, row, 0.0);
                let b = num(&item["amax"], s, row, std::f32::consts::TAU);
                let radius = num(&item["radius"], s, row, 20.0) * scale;
                let center = point("c");
                let mut pts: Vec<_> = (0..=64)
                    .map(|i| {
                        let t = a + (b - a) * i as f32 / 64.0;
                        center + vec2(t.cos(), t.sin()) * radius
                    })
                    .collect();
                if fill != Color32::TRANSPARENT {
                    pts.insert(0, center);
                }
                // Native outlined arcs close their endpoints with a chord too.
                shape(pts, true);
            }
            4 => text(
                &p,
                assets,
                r,
                &super::preview::localized_string(&item["text"], s, row, assets),
                &string(&item["font"], s, row),
                scale,
                color,
                egui::Align2::LEFT_TOP,
                true,
            ),
            5 => {
                draw_image(
                    &p,
                    assets,
                    &string(&item["image"], s, row),
                    r,
                    item,
                    s,
                    row,
                    scale,
                    // Native Draw B ignores a retained legacy color field.
                    Color32::WHITE,
                );
            }
            6 => {
                p.line_segment([point("a"), point("b")], stroke);
            }
            7 => {
                p.rect(r, 0, fill, stroke, egui::StrokeKind::Inside);
            }
            _ => {}
        }
    }
}

#[derive(Clone)]
struct Gesture {
    source: Value,
    draft: Value,
    origin: Pos2,
    field: String,
    size: bool,
}

pub(super) fn editor(ui: &mut Ui, node: &mut Value, s: &mut Settings, assets: &skin::Assets) {
    // A control written as something else (by hand, in the JSON) has no layers.
    if !node.is_object() {
        return;
    }
    egui::CollapsingHeader::new("Draw layers").show(ui, |ui| {
        ui.menu_button("+ Draw primitive", |ui| {
            for (kind, name) in KINDS.iter().enumerate() {
                if ui.button(*name).clicked() {
                    if !node["draw_list"].is_array() {
                        node["draw_list"] = json!([]);
                        // Clip to control clips nothing in the 8193.37 client and
                        // blanks a window whose last draw list has it on: written
                        // off and not offered. One authored on is flagged.
                        if node.get("draw_list_scissor").is_none() {
                            node["draw_list_scissor"] = false.into();
                        }
                    }
                    node["draw_list"].as_array_mut().unwrap().push(template(kind));
                    ui.close();
                }
            }
        });
        let Some(items) = node.get_mut("draw_list").and_then(Value::as_array_mut) else { return };
        let mut remove = None;
        let mut move_item = None;
        let total = items.len();
        for (i, item) in items.iter_mut().enumerate() {
            let name =
                KINDS.get(item["type"].as_u64().unwrap_or(99) as usize).unwrap_or(&"Unknown");
            egui::CollapsingHeader::new(format!("{} · {name}", i + 1)).id_salt(i).show(ui, |ui| {
                let id = ui.id().with("draw-gesture");
                let mut drag = ui.ctx().data_mut(|d| d.get_temp::<Gesture>(id));
                if drag.as_ref().is_some_and(|g| g.source != *item)
                    || ui.input(|i| i.key_pressed(egui::Key::Escape))
                {
                    drag = None;
                }
                let width = ui.available_width().max(100.0);
                let scale = width / 300.0;
                let (area, _) = ui.allocate_exact_size(vec2(width, 160.0), egui::Sense::hover());
                ui.painter().rect_filled(area, 0, Color32::from_gray(18));
                if let Some(g) = &mut drag
                    && let Some(p) = ui.input(|i| i.pointer.latest_pos())
                {
                    let delta = (p - g.origin) / scale;
                    for (key, delta) in [
                        (if g.size { "w" } else { "x" }, delta.x),
                        (if g.size { "h" } else { "y" }, delta.y),
                    ] {
                        g.draft[&g.field][key] = json!(
                            (g.source[&g.field][key].as_f64().unwrap_or(0.0) + f64::from(delta))
                                .round()
                        );
                    }
                }
                let shown = drag.as_ref().map_or(&*item, |g| &g.draft);
                let host = json!({"draw_list":[shown]});
                // The editor's own thumbnail stays inside its area.
                let painter = ui.painter().with_clip_rect(area.intersect(ui.clip_rect()));
                for before in [false, true] {
                    paint(
                        &painter,
                        &host,
                        area,
                        scale,
                        s,
                        assets,
                        None,
                        before,
                        [true, true, true, true],
                    );
                }
                let resize_rect = (shown["rect"]["w"].is_number()
                    && shown["rect"]["h"].is_number())
                .then(|| rect(&shown["rect"], s, None, area.min, scale));
                for field in ["rect", "a", "b", "ctrl0", "ctrl1", "c"] {
                    let v = &shown[field];
                    if !v["x"].is_number() || !v["y"].is_number() {
                        continue;
                    }
                    let p = area.min + coord(v, s, None) * scale;
                    let r = ui
                        .interact(
                            Rect::from_center_size(p, Vec2::splat(12.0)),
                            id.with(field),
                            egui::Sense::drag(),
                        )
                        .on_hover_cursor(egui::CursorIcon::Grab);
                    ui.painter().circle_filled(p, 4.0, Color32::LIGHT_BLUE);
                    if r.drag_started()
                        && let Some(origin) = ui.input(|i| i.pointer.press_origin())
                    {
                        drag = Some(Gesture {
                            source: item.clone(),
                            draft: item.clone(),
                            origin,
                            field: field.into(),
                            size: false,
                        });
                        break;
                    }
                }
                if let Some(rect) = resize_rect {
                    let r = ui
                        .interact(
                            Rect::from_center_size(rect.max, Vec2::splat(12.0)),
                            id.with("rect-size"),
                            egui::Sense::drag(),
                        )
                        .on_hover_cursor(egui::CursorIcon::ResizeNwSe);
                    ui.painter().rect_filled(r.rect.shrink(2.0), 0, Color32::LIGHT_BLUE);
                    if r.drag_started()
                        && let Some(origin) = ui.input(|i| i.pointer.press_origin())
                    {
                        drag = Some(Gesture {
                            source: item.clone(),
                            draft: item.clone(),
                            origin,
                            field: "rect".into(),
                            size: true,
                        });
                    }
                }
                if ui.input(|i| i.pointer.any_released())
                    && let Some(g) = drag.take()
                {
                    *item = g.draft;
                }
                ui.ctx().data_mut(|d| {
                    if let Some(g) = drag {
                        d.insert_temp(id, g);
                    } else {
                        d.remove::<Gesture>(id);
                    }
                });
                let mut remove_field = None;
                if let Some(obj) = item.as_object_mut() {
                    const ORDER: [&str; 23] = [
                        "rect",
                        "points",
                        "a",
                        "b",
                        "ctrl0",
                        "ctrl1",
                        "c",
                        "radius",
                        "amin",
                        "amax",
                        "text",
                        "font",
                        "image",
                        "image_aspect",
                        "image_halign",
                        "image_valign",
                        "color",
                        "fill",
                        "line_thickness",
                        "enabled",
                        "order",
                        "render",
                        "arrayBinds",
                    ];
                    let rank = |k: &str| ORDER.iter().position(|o| *o == k).unwrap_or(ORDER.len());
                    let mut fields: Vec<_> = obj.iter_mut().collect();
                    fields.sort_by_key(|(k, _)| rank(k));
                    for (key, value) in fields {
                        if *name == "Image"
                            && matches!(key.as_str(), "color" | "fill" | "line_thickness")
                        {
                            continue;
                        }
                        ui.push_id(key, |ui| match key.as_str() {
                            "type" => {}
                            "order" => {
                                let mut n = value.as_i64().unwrap_or(1);
                                egui::ComboBox::from_label("Paint order")
                                    .selected_text(if n < 0 {
                                        "Before control"
                                    } else {
                                        "After control"
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut n, -1, "Before control");
                                        ui.selectable_value(&mut n, 1, "After control");
                                    });
                                *value = json!(n);
                            }
                            "render" => {
                                let mut n = value.as_u64().unwrap_or(0) as usize;
                                let choices = [
                                    "Always",
                                    "Mouse outside",
                                    "Hover",
                                    "Left held",
                                    "Right held",
                                    "Middle held",
                                ];
                                egui::ComboBox::from_label("Show when")
                                    .selected_text(*choices.get(n).unwrap_or(&"Unknown"))
                                    .show_ui(ui, |ui| {
                                        for (i, label) in choices.iter().enumerate() {
                                            ui.selectable_value(&mut n, i, *label);
                                        }
                                    });
                                *value = json!(n);
                            }
                            "image" if value.is_string() => {
                                ui.label("Image");
                                let Value::String(picture) = value else { return };
                                ui.horizontal(|ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(picture)
                                            .desired_width(140.0)
                                            .hint_text("Picture name"),
                                    );
                                    if let Some(name) = images::from_disk(ui) {
                                        *picture = name;
                                    }
                                    // The pictures of the module and the game, by what is typed.
                                    ui.menu_button("Choose…", |ui| {
                                        egui::ScrollArea::vertical().max_height(240.0).show(
                                            ui,
                                            |ui| {
                                                let typed = picture.to_lowercase();
                                                for name in assets
                                                    .catalog
                                                    .iter()
                                                    .filter(|n| n.contains(&typed))
                                                    .take(40)
                                                {
                                                    if ui.button(name).clicked() {
                                                        *picture = name.clone();
                                                        ui.close();
                                                    }
                                                }
                                            },
                                        );
                                    });
                                });
                            }
                            "arrayBinds" => {
                                // A bound array gives each list row its own value.
                                if let Value::Bool(b) = value {
                                    ui.checkbox(b, "Per-row values (in a list)");
                                }
                            }
                            _ => property(ui, key, "draw", value, s, &mut remove_field),
                        });
                    }
                    if let Some(key) = remove_field {
                        obj.remove(&key);
                    }
                }
                ui.horizontal(|ui| {
                    if ui.add_enabled(i > 0, egui::Button::new("Up").small()).clicked() {
                        move_item = Some((i, i - 1));
                    }
                    if ui.add_enabled(i + 1 < total, egui::Button::new("Down").small()).clicked() {
                        move_item = Some((i, i + 1));
                    }
                    if ui.small_button("Remove drawing").clicked() {
                        remove = Some(i);
                    }
                });
            });
        }
        if let Some(i) = remove {
            items.remove(i);
        } else if let Some((a, b)) = move_item
            && b < items.len()
        {
            items.swap(a, b);
        }
    });
}
