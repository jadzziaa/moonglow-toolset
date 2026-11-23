//! The dialog pattern's looks and keys beyond each dialog's footer (M4,
//! U27): the main action's accent fill as drawn, the job window's progress
//! words in Light, the Creature Wizard's step list, nothing under a dialog
//! taking keys or clicks, and the Tab order of a floating window's bar.

use crate::common::*;
use mg_uikit::tokens::contrast;

/// The name and rect of the control holding the keys.
fn focused(h: &Harness<'_, Moonglow>) -> Option<(String, egui::Rect)> {
    let id = h.ctx.memory(|m| m.focused())?;
    let r = h.ctx.read_response(id)?;
    let name = h
        .query_all(egui_kittest::kittest::by().predicate(|n| n.is_focused()))
        .next()
        .map(|n| n.accesskit_node().label().unwrap_or_default().to_string())
        .unwrap_or_default();
    Some((name, r.rect))
}

/// The picture of `h`'s window as drawn, and the points to its pixels.
fn picture(h: &mut Harness<'static, Moonglow>) -> (image::RgbaImage, f32) {
    let ppp = h.ctx.pixels_per_point();
    (h.render().expect("drawn"), ppp)
}

fn pixel(img: &image::RgbaImage, at: egui::Pos2, ppp: f32) -> egui::Color32 {
    let p = img.get_pixel((at.x * ppp) as u32, (at.y * ppp) as u32).0;
    egui::Color32::from_rgb(p[0], p[1], p[2])
}

/// Words drawn in `rect` (points) of `img`: the contrast of their ink
/// against what is under them (the rect's commonest color; the ink, the
/// pixel farthest from it).
fn words_contrast(img: &image::RgbaImage, rect: egui::Rect, ppp: f32) -> (f32, egui::Color32) {
    let mut counts: std::collections::HashMap<[u8; 3], usize> = Default::default();
    let mut pixels = Vec::new();
    for y in (rect.top() * ppp) as u32..(rect.bottom() * ppp) as u32 {
        for x in (rect.left() * ppp) as u32..(rect.right() * ppp) as u32 {
            let p = img.get_pixel(x, y).0;
            let c = [p[0], p[1], p[2]];
            *counts.entry(c).or_default() += 1;
            pixels.push(c);
        }
    }
    let ground = *counts.iter().max_by_key(|(_, n)| **n).expect("pixels").0;
    let ground = egui::Color32::from_rgb(ground[0], ground[1], ground[2]);
    let best = pixels
        .iter()
        .map(|c| contrast(egui::Color32::from_rgb(c[0], c[1], c[2]), ground))
        .fold(1.0, f32::max);
    (best, ground)
}

/// The job window's progress words ("1 of 3") read in both themes, Light
/// above all (T7's note: egui's bar drew them in the selection's light text
/// over its light track, half unreadable): at least WCAG's 4.5 to 1 against
/// what is under them, measured from the picture drawn, and from the
/// tokens the words are drawn in. What the bar's own words had is printed
/// for the record (the numbers in the report).
#[test]
fn the_job_window_s_progress_words_read_in_light() {
    if no_adapter() {
        return;
    }
    mg_testkit::gpu::hold();
    let _shared_rs = with_shared_renderer();
    for light in [true, false] {
        let mut app = app_with(Vec::new());
        app.settings.light_theme = light;
        let mut h = harness(app, 1280.0, 720.0);
        settle(&mut h);
        h.state_mut().start_job_for_tests("Held", |progress| {
            progress.step(1, 3);
            let started = std::time::Instant::now();
            while !progress.cancelled() && started.elapsed().as_secs() < 30 {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        });
        let words = |h: &Harness<'_, Moonglow>| {
            // (The words under the bar, a label of their own.)
            h.query_all_by_label_contains("1 of 3")
                .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Label)
                .map(|n| n.rect())
                .unwrap_or(egui::Rect::NOTHING)
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !words(&h).is_positive() {
            assert!(std::time::Instant::now() < deadline, "the words shown");
            // (A job under way repaints: frames stepped, not run to rest.)
            h.step();
        }
        // (The window fades in, as egui's areas do: at rest, the fade ended
        // with it, which asks for frames until it does.)
        crate::dialog_pattern::rested(&mut h, "Held", true);
        let theme = if light { "Light" } else { "Dark" };
        let v = h.ctx.global_style().visuals.clone();
        // (What egui's bar drew its words in, over its track.)
        let before = contrast(v.selection.stroke.color, v.extreme_bg_color);
        let tokens = contrast(v.text_color(), v.window_fill);
        let (img, ppp) = picture(&mut h);
        let (drawn, ground) = words_contrast(&img, words(&h), ppp);
        eprintln!(
            "{theme}: the bar's own words over its track {before:.2}; the words now {tokens:.2} \
             by the tokens, {drawn:.2} as drawn (over {ground:?})"
        );
        assert!(tokens >= 4.5, "{theme}: the words' tokens {tokens:.2}");
        assert!(drawn >= 4.5, "{theme}: the words as drawn {drawn:.2}");
        // (Under the bar, not over it: on the window's own fill.)
        assert!(contrast(ground, v.window_fill) < 1.05, "{theme}: on the window: {ground:?}");
        h.get_all_by_label("Cancel").last().expect("its Cancel").click();
        settle(&mut h);
    }
}

/// The main action is drawn in `accent-fill` (the board's `.btn.primary`)
/// and the other buttons are not: the pixel just inside each button's
/// left edge, in both themes, New Module's Create against its Cancel and
/// Unsaved Changes' Save against Don't Save.
#[test]
fn the_main_action_is_drawn_in_the_accent_fill() {
    if no_adapter() {
        return;
    }
    mg_testkit::gpu::hold();
    let _shared_rs = with_shared_renderer();
    for light in [false, true] {
        for title in ["New Module", "Unsaved Changes"] {
            let mut app = app_with(Vec::new());
            app.settings.light_theme = light;
            let mut h = harness(app, 1280.0, 720.0);
            settle(&mut h);
            match title {
                "New Module" => {
                    h.state_mut().actions.push(mg_ui::Action::NewModuleDialog);
                    settle(&mut h);
                    // (Create is off with no name: a name typed.)
                    h.event(egui::Event::Text("Keep".into()));
                }
                _ => h.state_mut().confirm_discard = Some(mg_ui::Action::NewModuleDialog),
            }
            settle(&mut h);
            let tokens = mg_uikit::tokens(&h.ctx);
            let seen = mg_ui::widgets::footers_shown(&h.ctx);
            let foot = seen.iter().find(|f| f.title == title).expect("shown");
            let (img, ppp) = picture(&mut h);
            for b in &foot.buttons {
                let at = egui::pos2(b.rect.left() + 3.0, b.rect.center().y);
                let fill = pixel(&img, at, ppp);
                match b.main {
                    true => assert_eq!(fill, tokens.accent_fill, "{title}: {} drawn", b.words),
                    false => assert_ne!(fill, tokens.accent_fill, "{title}: {} drawn", b.words),
                }
            }
            assert!(foot.buttons.iter().any(|b| b.main), "{title}: a main action");
        }
    }
}

/// The same for every dialog of the table (U27's review, fault 4), Dark
/// and Light: the pixel just inside each footer button's left edge is
/// `accent-fill` for the main action, while it is enabled, and for no other
/// button. The pointer is parked off the window, so no button is hovered.
#[test]
fn every_dialog_s_main_action_is_drawn_in_the_accent_fill() {
    if no_adapter() {
        return;
    }
    mg_testkit::gpu::hold();
    let _shared_rs = with_shared_renderer();
    crate::dialog_pattern::running("-accent");
    let mut failed = Vec::new();
    let mut held = 0;
    for op in crate::dialog_pattern::all() {
        if op.bare {
            continue;
        }
        let screen = egui::vec2(1280.0, 720.0);
        let Some(mut h) = crate::dialog_pattern::opened(&op, screen, 1.0) else { continue };
        for light in [false, true] {
            let theme = if light { "Light" } else { "Dark" };
            h.state_mut().settings.light_theme = light;
            h.event(egui::Event::PointerGone);
            crate::dialog_pattern::rested(&mut h, op.title, op.job);
            let tokens = mg_uikit::tokens(&h.ctx);
            let seen = mg_ui::widgets::footers_shown(&h.ctx);
            let Some(foot) = seen.iter().find(|f| f.title == op.title) else {
                failed.push(format!("{} {theme}: not shown", op.title));
                continue;
            };
            let (img, ppp) = picture(&mut h);
            for b in &foot.buttons {
                let at = egui::pos2(b.rect.left() + 3.0, b.rect.center().y);
                let fill = pixel(&img, at, ppp);
                let accent = fill == tokens.accent_fill;
                if accent != (b.main && b.enabled) {
                    failed.push(format!(
                        "{} ({}) {theme}: {} (main {}, enabled {}) drawn {fill:?}, the accent {:?}",
                        op.title, op.key, b.words, b.main, b.enabled, tokens.accent_fill
                    ));
                }
            }
            held += 1;
        }
    }
    assert!(held > 100, "the table's dialogs pictured: {held}");
    assert!(failed.is_empty(), "{} off:\n{}", failed.len(), failed.join("\n"));
}

/// The Creature Wizard's step list, at each of its nine steps: "Step N of
/// 9" over the nine steps by name, the steps before the one shown done
/// (not selected, enabled), it current (selected), those after to come
/// (neither), the list the same nine rows at every step; the rows are no
/// stop of Tab (the board's rows are not clickable); the keys are the
/// footer's: Enter is Next while it is enabled (the page complete) and goes
/// one step on, Escape cancels.
#[test]
fn the_creature_wizard_s_step_list_follows_its_steps() {
    use egui::accesskit::Toggled;
    const NAMES: [&str; 9] = [
        "Creature Wizard",
        "Monster Type",
        "Class and Level",
        "Appearance and Portrait",
        "Faction",
        "Name",
        "Select Palette Category",
        "Review Statistics",
        "Finish",
    ];
    let Some(mut h) = game_harness("u27-steps") else { return };
    h.set_size(egui::vec2(1600.0, 900.0));
    for page in 0..9 {
        h.state_mut().creature_wizard = Some(mg_ui::creature_wizard::CreatureWizard {
            page,
            race: Some(6),
            classes: vec![(4, 4)],
            chosen_class: Some(4),
            portrait: Some(1),
            first_name: "Steps".into(),
            category: Some(0),
            ..Default::default()
        });
        settle(&mut h);
        assert!(h.query_by_label_contains(&format!("Step {} of 9", page + 1)).is_some());
        let mut rows = Vec::new();
        for (i, name) in NAMES.iter().enumerate() {
            let label = format!("Step {}: {name}", i + 1);
            let node = h.get_by_label(&label);
            let a = node.accesskit_node();
            let selected = a.toggled() == Some(Toggled::True);
            assert_eq!(selected, i == page, "step {i} at page {page}: current alone selected");
            assert!(!a.is_disabled(), "step {i} at page {page}: none off in this wizard");
            rows.push(node.rect());
        }
        for pair in rows.windows(2) {
            assert!((pair[1].top() - pair[0].top() - 22.0).abs() < 0.5, "rows 22 apart: {rows:?}");
        }
        // No stop of Tab among the rows.
        h.ctx.memory_mut(|m| {
            if let Some(f) = m.focused() {
                m.surrender_focus(f);
            }
        });
        let list = rows[0].union(rows[8]);
        for _ in 0..40 {
            h.key_press(egui::Key::Tab);
            h.run();
            if let Some((name, rect)) = focused(&h) {
                assert!(!list.contains_rect(rect), "Tab stopped on a step: {name}");
            }
        }
        h.state_mut().creature_wizard = None;
        settle(&mut h);
    }
    // Enter: on, step by step, while Next is enabled (everything chosen).
    h.state_mut().creature_wizard = Some(mg_ui::creature_wizard::CreatureWizard {
        page: 2,
        race: Some(6),
        classes: vec![(4, 4)],
        chosen_class: Some(4),
        portrait: Some(1),
        first_name: "Steps".into(),
        category: Some(0),
        ..Default::default()
    });
    settle(&mut h);
    h.ctx.memory_mut(|m| {
        if let Some(f) = m.focused() {
            m.surrender_focus(f);
        }
    });
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(h.state().creature_wizard.as_ref().map(|w| w.page), Some(3), "Enter is Next");
    let current = h.get_by_label("Step 4: Appearance and Portrait").accesskit_node().toggled();
    assert_eq!(current, Some(Toggled::True), "the list follows");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.state().creature_wizard.is_none(), "Escape cancels");
}

/// At the least window the Creature Wizard leaves its step list out, so
/// that its page keeps the room for its controls (with the list, Add Class
/// was squeezed to a letter a line); the page bar still names the page.
/// At 1600 by 900 the list is there.
#[test]
fn the_creature_wizard_at_the_least_window_keeps_its_page_s_room() {
    for (size, listed) in [(mg_ui::LEAST_WINDOW_SIZE, false), (egui::vec2(1600.0, 900.0), true)] {
        // (A window of its own each: egui keeps a dialog's size.)
        let Some(mut h) = game_harness("u27-least-steps") else { return };
        h.set_size(size);
        h.state_mut().creature_wizard = Some(mg_ui::creature_wizard::CreatureWizard {
            page: 2,
            race: Some(6),
            classes: vec![(4, 4)],
            chosen_class: Some(4),
            ..Default::default()
        });
        settle(&mut h);
        assert_eq!(h.query_by_label_contains("Step 3 of 9").is_some(), listed, "{size:?}");
        let _ = h.get_by_label("Class and Level");
        let add = h.get_by_label("Add Class").rect();
        assert!(add.height() < 30.0 && add.width() > 50.0, "{size:?}: Add Class at {add:?}");
    }
}

/// Nothing under a dialog takes its keys or its clicks: an area open with
/// a placeable in hand (where a click on the area places it), the Area
/// Wizard opened over the area. Spots of the dialog with no control on
/// them (its title row, the footer left of its buttons, the body's
/// margins) are first tried with the dialog closed: those where a click
/// places one on the area under them are kept (at least one, the test's
/// own check), and with the dialog open a click there places nothing.
/// With a placeable placed and selected, the arrows, Delete and Ctrl+A
/// (on an empty selection) with no field holding the keys change nothing
/// of the area; the dialog stays through it all. The check of the keys:
/// once the dialog is closed, they do change it.
#[test]
fn nothing_under_a_dialog_takes_its_keys_or_clicks() {
    let Some((mut h, area)) = area_harness("u27-under") else { return };
    wide_window(&mut h);
    let placed = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let git = ws.doc(&ResKey::new(area, ResType::GIT)).unwrap();
        git.root.list("Placeable List").map_or(0, <[mg_gff::Struct]>::len)
    };
    {
        let app = h.state_mut();
        app.palette.tiles = false;
        app.palette.kind = mg_module::palette::BlueprintKind::Placeable;
        app.palette.selected = ResKey::parse("plc_statue1", ResType::UTP);
        app.paint = false;
    }
    settle(&mut h);
    let open = |h: &mut Harness<'static, Moonglow>| {
        h.state_mut().actions.push(mg_ui::Action::AreaWizard);
        settle(h);
        let seen = mg_ui::widgets::footers_shown(&h.ctx);
        seen.iter().find(|f| f.title == "Area Wizard").expect("shown").clone()
    };
    let wizard = open(&mut h);
    let (foot, window) = (wizard.rect.expect("a footer"), wizard.window);
    let first = wizard.buttons[0].rect;
    let mut spots = vec![egui::pos2(window.center().x, window.top() + 14.0)];
    for k in 1..4 {
        let x = foot.left() + (first.left() - foot.left()) * k as f32 / 4.0;
        spots.push(egui::pos2(x, foot.center().y));
    }
    for k in 1..6 {
        let y = window.top() + window.height() * k as f32 / 6.0;
        spots.push(egui::pos2(window.left() + 5.0, y));
        spots.push(egui::pos2(window.right() - 5.0, y));
    }
    let click = |h: &mut Harness<'static, Moonglow>, at: egui::Pos2| {
        h.hover_at(at);
        settle(h);
        press(h, at, true, egui::Modifiers::NONE);
        press(h, at, false, egui::Modifiers::NONE);
        settle(h);
    };
    // The check: with the dialog closed, which spots place one.
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.state().wizard.is_none());
    let before = placed(&mut h);
    spots.retain(|at| {
        h.state_mut().palette.selected = ResKey::parse("plc_statue1", ResType::UTP);
        settle(&mut h);
        click(&mut h, *at);
        let placing = placed(&mut h) > before;
        if placing {
            h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
            settle(&mut h);
        }
        placing
    });
    assert!(!spots.is_empty(), "a spot of the dialog over the area, where a click places one");
    assert_eq!(placed(&mut h), before);
    // One placed and selected, for the keys to have something to act on.
    // (A placing takes the blueprint out of the hand: it is taken again.)
    h.state_mut().palette.selected = ResKey::parse("plc_statue1", ResType::UTP);
    settle(&mut h);
    click(&mut h, spots[0]);
    let one = placed(&mut h);
    assert_eq!(one, before + 1);
    let selected = vec![(mg_area::ObjectKind::Placeable, before)];
    h.state_mut().area_views.get_mut(&area).unwrap().selection = selected.clone();
    settle(&mut h);
    open(&mut h);
    for at in &spots {
        // (The blueprint in hand, so a click that reached the area would
        // place one.)
        h.state_mut().palette.selected = ResKey::parse("plc_statue1", ResType::UTP);
        settle(&mut h);
        click(&mut h, *at);
        assert_eq!(placed(&mut h), one, "a click at {at:?} on the dialog reached the area");
        assert!(h.state().palette.selected.is_some(), "still in hand after a click at {at:?}");
        assert!(h.state().wizard.is_some(), "the dialog stays");
    }
    // (Nothing in hand for the keys: placing takes them otherwise.)
    h.state_mut().palette.selected = None;
    let keys_test = |h: &mut Harness<'static, Moonglow>| {
        h.ctx.memory_mut(|m| {
            if let Some(f) = m.focused() {
                m.surrender_focus(f);
            }
        });
        h.state_mut().area_views.get_mut(&area).unwrap().selection = selected.clone();
        settle(h);
        let undo = |h: &Harness<'_, Moonglow>| {
            h.state().ws.as_ref().unwrap().can_undo().map(str::to_string)
        };
        let was = undo(h);
        for key in [egui::Key::ArrowLeft, egui::Key::ArrowDown, egui::Key::Delete] {
            h.key_press(key);
            settle(h);
            if undo(h) != was {
                return Some(format!("{key:?}"));
            }
        }
        h.state_mut().area_views.get_mut(&area).unwrap().selection.clear();
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        settle(h);
        (!h.state().area_views[&area].selection.is_empty()).then(|| "Ctrl+A".to_string())
    };
    assert_eq!(keys_test(&mut h), None, "a key reached the area under the dialog");
    assert_eq!(placed(&mut h), one);
    assert!(h.state().wizard.is_some(), "the dialog stays open through it");
    // The check of the keys: with the dialog closed, they reach the area.
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(h.state().wizard.is_none());
    h.hover_at(spots[0]);
    settle(&mut h);
    assert!(keys_test(&mut h).is_some(), "with no dialog open the keys reach the area");
}

/// A floating window's bar is reached by Tab left to right: its collapse
/// arrow (at the bar's left), its tab, then its Close Window (at the
/// right), then the window's body; Shift+Tab goes back the same way.
/// (egui_dock made the arrow after the tabs and the close all, so Tab came
/// to it last, after the Close at the bar's other end: the vendored
/// crate's patch makes it first.)
#[test]
fn a_floating_window_s_bar_is_tabbed_left_to_right() {
    let Some((mut h, _)) = blueprint_harness("plc_chest1", "u27_bar", ResType::UTP) else {
        return;
    };
    settle(&mut h);
    let tab = h
        .get_all_by_label("u27_bar.utp")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Tab)
        .expect("the window's tab")
        .rect();
    h.get_all_by_label("u27_bar.utp")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Tab)
        .unwrap()
        .focus();
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Tab);
    settle(&mut h);
    let (name, arrow) = focused(&h).expect("a stop before the tab");
    assert_eq!(name, "Collapse", "the arrow before the tab");
    assert!(arrow.right() <= tab.left() + 0.5, "at the bar's left: {arrow:?} {tab:?}");
    let mut order = vec![name];
    for _ in 0..3 {
        h.key_press(egui::Key::Tab);
        settle(&mut h);
        order.push(focused(&h).expect("a stop").0);
    }
    assert_eq!(order[..3], ["Collapse", "u27_bar.utp", "Close Window"], "{order:?}");
    // (Then the body: below the bar.)
    let (next, rect) = focused(&h).unwrap();
    assert!(rect.top() >= tab.bottom() - 0.5, "the body after the bar: {next} {rect:?}");
    for want in ["Close Window", "u27_bar.utp", "Collapse"] {
        h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Tab);
        settle(&mut h);
        assert_eq!(focused(&h).expect("a stop").0, want, "Shift+Tab back");
    }
}
