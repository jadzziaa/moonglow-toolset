//! The Right Dock and its Palette panel (the redesign's M3, T5): shown
//! and folded by Tools › Palettes, View › Right Dock, their keys, its fold
//! button and its strip; the palette laid out inside it at every width;
//! F6 through the docks; settings and records of the palettes' pane read;
//! the plugins' pane; the palettes' lists' own scroll places.

use crate::common::*;
use mg_module::palette::BlueprintKind;

/// The sample module with the game's data, opened, at `width` by 800.
fn dock_harness(name: &str, width: f32) -> Option<Harness<'static, Moonglow>> {
    let root = mg_testkit::nwn_root()?;
    let dir = mg_testkit::scratch_dir(name);
    let path = sample_module(&dir);
    let mut app = game_app(&root, None);
    app.open_module(&path);
    let mut h = harness(app, width, 800.0);
    settle(&mut h);
    Some(h)
}

/// The Right Dock folded or not: what the settings say, and what is
/// drawn (its fold button, or its strip's Palette button and the button
/// that unfolds it).
fn folded(h: &Harness<'_, Moonglow>) -> bool {
    let folded = h.state().settings.hide_palettes;
    if folded {
        assert!(h.query_by_label("Fold the Right Dock").is_none());
        h.get_by_label("Show the Right Dock");
        h.get_by_label("Right Dock").get_by_label("Palette");
        assert!(h.query_by_label("Palette types").is_none(), "the palette not drawn");
    } else {
        h.get_by_label("Fold the Right Dock");
        h.get_by_label("Palette types");
        assert!(palette_shown(h.state()));
    }
    folded
}

/// The palette list's first row in sight: its name (the rows under the
/// palette's header, 140 points down the dock).
fn top_label(h: &Harness<'_, Moonglow>) -> String {
    top_node(h).and_then(|(_, label)| label).unwrap_or_default()
}

/// The palette list's first row in sight: where it is.
fn top_row(h: &Harness<'_, Moonglow>) -> egui::Rect {
    top_node(h).map_or(egui::Rect::NOTHING, |(rect, _)| rect)
}

fn top_node(h: &Harness<'_, Moonglow>) -> Option<(egui::Rect, Option<String>)> {
    let dock = right_dock_rect(h);
    let list_top = dock.top() + 140.0;
    h.query_all_by(|n| n.role() == egui::accesskit::Role::Button)
        .filter(|n| dock.contains(n.rect().center()) && n.rect().top() > list_top)
        .min_by(|a, b| a.rect().top().total_cmp(&b.rect().top()))
        .map(|n| (n.rect(), n.accesskit_node().label()))
}

/// Clicks the palette's Expand All (not the Module panel's).
fn expand_all_in_the_dock(h: &mut Harness<'_, Moonglow>) {
    let dock = right_dock_rect(h);
    let button = h.get_all_by_label("Expand All").find(|n| dock.contains(n.rect().center()));
    button.expect("the palette's Expand All").click();
    settle(h);
}

/// Tools › Palettes shows the Right Dock unfolded on the Palette panel,
/// folded or not (asked again, it stays); View › Right Dock and its key
/// (Ctrl+Alt+2, the palettes' as before: the command keeps its id) fold
/// and unfold it, the row ticked while it is shown; the strip's Palette
/// button unfolds it.
#[test]
fn tools_palettes_and_view_right_dock_show_and_fold_it() {
    let Some(mut h) = dock_harness("ui-right-dock-commands", 1400.0) else { return };
    assert!(!folded(&h), "shown with the module");
    let ticked = |h: &mut Harness<'_, Moonglow>| {
        h.get_by_label("View").click();
        h.run();
        let row = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.role() == egui::accesskit::Role::CheckBox
                && n.label().as_deref() == Some("Right Dock")
        };
        let node = h.get(egui_kittest::kittest::by().predicate(row));
        let on = node.accesskit_node().toggled() == Some(egui::accesskit::Toggled::True);
        h.key_press(egui::Key::Escape);
        settle(h);
        on
    };
    assert!(ticked(&mut h), "ticked while shown");
    // View › Right Dock folds it, and shows it again.
    menu_command(&mut h, "View", "Right Dock");
    settle(&mut h);
    assert!(folded(&h));
    assert!(!ticked(&mut h), "not ticked folded");
    menu_command(&mut h, "View", "Right Dock");
    settle(&mut h);
    assert!(!folded(&h));
    // Its key.
    let keys = egui::Modifiers::COMMAND | egui::Modifiers::ALT;
    h.key_press_modifiers(keys, egui::Key::Num2);
    settle(&mut h);
    assert!(folded(&h));
    // The strip's Palette button unfolds it on the Palette panel.
    h.get_by_label("Right Dock").get_by_label("Palette").click();
    settle(&mut h);
    assert!(!folded(&h));
    h.key_press_modifiers(keys, egui::Key::Num2);
    settle(&mut h);
    assert!(folded(&h));
    // Tools › Palettes: unfolded, and asked again it stays so.
    for _ in 0..2 {
        menu_command(&mut h, "Tools", "Palettes");
        settle(&mut h);
        assert!(!folded(&h));
    }
    // (The Command Palette finds the row by the dock's name.)
    assert!(
        mg_ui::commands::Id::all().into_iter().any(|id| id.name() == "Right Dock"),
        "View's row named Right Dock"
    );
}

/// With no module open there is no Right Dock (the palettes came with a
/// module), and View › Right Dock is grayed; Tools › Palettes shows it on
/// the game's standard palettes all the same (as it opened the palettes'
/// pane beside the start page before), and from then on View's row and
/// its key fold and unfold it.
#[test]
fn without_a_module_tools_palettes_shows_the_standard_palettes() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let mut h = harness(game_app(&root, None), 1400.0, 800.0);
    settle(&mut h);
    assert!(h.query_by_label("Right Dock").is_none(), "no Right Dock without a module");
    assert!(h.query_by_label("Fold the Right Dock").is_none());
    let row = |h: &Harness<'_, Moonglow>| menu_row(h, "Right Dock");
    h.get_by_label("View").click();
    h.run();
    assert!(row(&h).is_some());
    let grayed = h.get_by_label("Right Dock").accesskit_node().is_disabled();
    assert!(grayed, "View › Right Dock grayed with no panel to show");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    menu_command(&mut h, "Tools", "Palettes");
    settle(&mut h);
    assert!(!folded(&h));
    // The standard placeables (the custom palette is a module's).
    h.state_mut().palette.tiles = false;
    h.state_mut().palette.kind = BlueprintKind::Placeable;
    settle(&mut h);
    h.get_by_label_contains("items");
    let keys = egui::Modifiers::COMMAND | egui::Modifiers::ALT;
    h.key_press_modifiers(keys, egui::Key::Num2);
    settle(&mut h);
    assert!(folded(&h), "its key folds it");
    menu_command(&mut h, "View", "Right Dock");
    settle(&mut h);
    assert!(!folded(&h), "View's row unfolds it");
}

/// What is drawn in and over the Right Dock, as the accessibility tree
/// has it: the dock's own nodes are those under its panel (the pane over
/// its tab bar, `Right Dock`), wherever they are drawn; the others are
/// every node elsewhere in the tree.
struct DockNodes {
    /// The dock's nodes: their names, rects, and egui ids.
    own: Vec<(String, egui::Rect, u64)>,
    /// The other nodes (not those holding the dock's panel: the window's,
    /// the layout's), their names, rects and egui ids.
    others: Vec<(String, egui::Rect, u64)>,
    /// The egui ids of the dock's panel and of every node under it, with
    /// a rect or not (its layouts).
    ids: std::collections::HashSet<u64>,
}

fn dock_nodes(h: &Harness<'_, Moonglow>) -> DockNodes {
    use egui_kittest::kittest::AccessKitNode;
    let rect = |n: &AccessKitNode<'_>| {
        n.bounding_box().map(|b| {
            let ppp = h.ctx.pixels_per_point() as f64;
            egui::Rect::from_min_max(
                egui::pos2((b.x0 / ppp) as f32, (b.y0 / ppp) as f32),
                egui::pos2((b.x1 / ppp) as f32, (b.y1 / ppp) as f32),
            )
        })
    };
    let name = |n: &AccessKitNode<'_>| {
        format!("{:?} {:?}", n.role(), n.label().or_else(|| n.value()).unwrap_or_default())
    };
    // The path down to the dock's tab bar.
    fn find<'t>(
        n: AccessKitNode<'t>,
        path: &mut Vec<AccessKitNode<'t>>,
        out: &mut Option<Vec<AccessKitNode<'t>>>,
    ) {
        let tabs = n.label().as_deref() == Some(mg_ui::RIGHT_DOCK)
            && matches!(n.role(), egui::accesskit::Role::Group | egui::accesskit::Role::TabList);
        path.push(n);
        if tabs && out.is_none() {
            *out = Some(path.clone());
        }
        for k in n.children() {
            find(k, path, out);
        }
        path.pop();
    }
    let mut path = Vec::new();
    let mut above = None;
    find(h.root().accesskit_node(), &mut path, &mut above);
    let above = above.expect("the Right Dock's tab bar");
    // (The dock's panel is a pane: `shell::dock_pane`.)
    let panel = *(above.iter().rev())
        .find(|n| n.role() == egui::accesskit::Role::Pane)
        .expect("the Right Dock's panel in the tree");
    let key = |n: &AccessKitNode<'_>| n.locate().0.0;
    let holders: Vec<u64> = above.iter().take_while(|n| key(n) != key(&panel)).map(&key).collect();
    let mut own = Vec::new();
    let mut others = Vec::new();
    let mut ids = std::collections::HashSet::new();
    fn walk<'t>(
        n: AccessKitNode<'t>,
        inside: bool,
        panel: u64,
        each: &mut dyn FnMut(AccessKitNode<'t>, bool),
    ) {
        let inside = inside || n.locate().0.0 == panel;
        each(n, inside);
        for k in n.children() {
            walk(k, inside, panel, each);
        }
    }
    walk(h.root().accesskit_node(), false, key(&panel), &mut |n, inside| {
        if inside {
            ids.insert(key(&n));
        }
        let Some(r) = rect(&n) else { return };
        if key(&n) == key(&panel) || holders.contains(&key(&n)) {
            return;
        }
        if inside {
            own.push((name(&n), r, key(&n)));
        } else {
            others.push((name(&n), r, key(&n)));
        }
    });
    DockNodes { own, others, ids }
}

/// What is wrong in and over the Right Dock: a node of the dock's outside
/// it (left or right of it, or above it: the lists are at their tops,
/// and rows below the dock's foot are the lists' scrolled out of sight),
/// or over its fold button; another node reaching into the dock (but
/// the fold button, beside its edge); two of its named widgets over each
/// other (their visible rects, clipped by their scroll areas, overlapping
/// by more than a quarter of the smaller, in any layers). `at` names the
/// case. The dock's nodes are counted into `seen`.
fn dock_wrong(h: &Harness<'_, Moonglow>, at: &str, seen: &mut usize) -> Vec<String> {
    let dock = right_dock_rect(h);
    let fold = h.get_by_label("Fold the Right Dock").rect();
    let DockNodes { own, others, ids: mine } = dock_nodes(h);
    let mut wrong = Vec::new();
    for (name, r, _) in &own {
        *seen += 1;
        let past = r.left() < dock.left() - 0.5
            || r.right() > dock.right() + 0.5
            || r.top() < dock.top() - 0.5;
        if past {
            wrong.push(format!("{at}: {name} at {r:?}, past {dock:?}"));
        }
        if r.intersects(fold) {
            wrong.push(format!("{at}: {name} at {r:?}, under the fold button {fold:?}"));
        }
    }
    // Anything drawn across the dock's edges (where it is in sight: its
    // rect clipped, as egui records it for the frame): another widget
    // reaching into it, or one of the dock's own reaching out of it (its
    // layouts, which have no node of their own, too). But the dock's
    // edge, which it is resized by, its fold button beside it, and what
    // holds the whole dock (the window's layouts).
    let inner = dock.shrink(1.0);
    let edge = egui::Id::new("right-dock").with("__resize").value();
    let names: std::collections::HashMap<u64, &String> = (others.iter())
        .map(|(name, _, id)| (*id, name))
        .chain(own.iter().map(|(name, _, id)| (*id, name)))
        .collect();
    h.ctx.viewport(|v| {
        for (_, widgets) in v.prev_pass.widgets.layers() {
            for w in widgets {
                let r = w.interact_rect;
                let id = w.id.value();
                let across = r.intersects(inner) && !dock.expand(0.5).contains_rect(r);
                let beside = fold.expand(0.5).contains_rect(r);
                if across && !beside && id != edge && !r.contains_rect(inner) && r.area() > 1.0 {
                    let mut name = names.get(&id).map_or("a widget".to_string(), |n| n.to_string());
                    if mine.contains(&id) {
                        name = format!("the dock's {name}");
                    }
                    wrong.push(format!("{at}: {name} at {r:?}, across the dock's edge {dock:?}"));
                }
            }
        }
    });
    // (Named widgets: the dock's leaves with a name.)
    let ids: std::collections::HashMap<u64, &String> = own
        .iter()
        .filter(|(name, _, _)| !name.ends_with("\"\""))
        .map(|(name, _, id)| (*id, name))
        .collect();
    h.ctx.viewport(|v| {
        let shown: Vec<(egui::Rect, &String)> = (v.prev_pass.widgets.layers())
            .flat_map(|(_, widgets)| widgets.iter())
            .filter(|w| w.interact_rect.area() > 1.0)
            .filter_map(|w| Some((w.interact_rect, *ids.get(&w.id.value())?)))
            .collect();
        for (i, (a, name_a)) in shown.iter().enumerate() {
            for (b, name_b) in &shown[i + 1..] {
                let both = a.intersect(*b);
                if both.is_positive() && both.area() > 0.25 * a.area().min(b.area()) {
                    wrong.push(format!("{at}: {name_a} {a:?} over {name_b} {b:?}"));
                }
            }
        }
    });
    wrong
}

/// A type of palette the sweeps show.
#[derive(Debug, Clone, Copy)]
enum Kind {
    Tiles,
    /// A blueprint type, in the Gallery or not.
    Blueprint(BlueprintKind, bool),
    Prefabs,
    Lights,
    Start,
}

/// Every type of palette: the tiles' (an area open), each blueprint
/// type's, in the Gallery too where the type has pictures, prefabs,
/// lights and the start location.
fn every_kind() -> Vec<Kind> {
    use BlueprintKind as K;
    let mut kinds = vec![Kind::Tiles];
    kinds.extend(BlueprintKind::ALL.map(|k| Kind::Blueprint(k, false)));
    let pictured = [K::Creature, K::Door, K::Item, K::Placeable, K::Waypoint];
    kinds.extend(pictured.map(|k| Kind::Blueprint(k, true)));
    kinds.extend([Kind::Prefabs, Kind::Lights, Kind::Start]);
    kinds
}

/// Shows the palette of `kind`, its categories all open where it has
/// them (rows to look at).
fn show_kind(h: &mut Harness<'_, Moonglow>, kind: Kind) {
    {
        let app = h.state_mut();
        let p = &mut app.palette;
        (p.tiles, p.prefabs, p.lights, p.start) = (false, false, false, false);
        app.settings.palette_gallery = false;
        match kind {
            Kind::Tiles => p.tiles = true,
            Kind::Blueprint(k, gallery) => {
                (p.kind, p.custom) = (k, false);
                app.settings.palette_gallery = gallery;
            }
            Kind::Prefabs => p.prefabs = true,
            Kind::Lights => p.lights = true,
            Kind::Start => p.start = true,
        }
    }
    settle(h);
    if matches!(kind, Kind::Blueprint(..)) {
        expand_all_in_the_dock(h);
    }
}

/// The dock's nodes below its header (the type strip, the type's name and
/// source, the search: 110 points), at least: the list's rows or
/// pictures, or the type's own controls. Fewer, the palette is empty for
/// the type.
fn body_nodes(h: &Harness<'_, Moonglow>) -> usize {
    let dock = right_dock_rect(h);
    let header = dock.top() + 110.0;
    let own = dock_nodes(h).own;
    own.iter().filter(|(name, r, _)| !name.ends_with("\"\"") && r.top() > header).count()
}

/// The fewest body nodes ([`body_nodes`]) each type shows in the sweeps
/// (at the narrowest, the most in its rows): a list's toolbar and rows,
/// the Gallery's pictures, the tiles' brushes, prefabs' and lights' and
/// the start location's controls.
fn least_body(kind: Kind) -> usize {
    match kind {
        Kind::Tiles | Kind::Blueprint(..) => 10,
        Kind::Prefabs | Kind::Lights => 3,
        Kind::Start => 1,
    }
}

/// The sweep: each type of palette at each of `widths` (given by `set`,
/// the dock's width then measured): nothing wrong ([`dock_wrong`]), and
/// on each its body ([`least_body`]).
fn sweep(
    h: &mut Harness<'_, Moonglow>,
    widths: &[f32],
    set: &dyn Fn(&mut Harness<'_, Moonglow>, f32),
) -> (Vec<String>, usize) {
    let (mut wrong, mut seen) = (Vec::new(), 0);
    for kind in every_kind() {
        show_kind(h, kind);
        for &width in widths {
            set(h, width);
            settle(h);
            let at = format!("{kind:?} at {:.0}", right_dock_rect(h).width());
            wrong.extend(dock_wrong(h, &at, &mut seen));
            let body = body_nodes(h);
            if body < least_body(kind) {
                wrong.push(format!("{at}: {body} nodes under the header: an empty palette"));
            }
        }
    }
    (wrong, seen)
}

/// The palette inside the Right Dock at every width from its least, 280,
/// to 600, 8 points apart, on every type of palette (the tiles' with an
/// area open, each blueprint type, in the Gallery too where it has
/// pictures, prefabs, lights, the start location), each with something
/// in it: nothing of the panel lies outside the dock, under its fold
/// button or over another of its widgets, and nothing else lies over the
/// dock ([`dock_wrong`]). (With the palettes' pane dragged to about 135
/// px, a blueprint type's list toolbar spilled left under its fold
/// button: the dock is never so narrow. Narrower than 280, in a narrow
/// window: `the_palette_fits_the_right_dock_in_the_narrowest_window`.)
#[test]
fn the_palette_fits_the_right_dock_from_its_least_width_to_600() {
    let Some((mut h, _)) = area_harness("right-dock-sweep") else { return };
    h.set_size(egui::vec2(1600.0, 900.0));
    settle(&mut h);
    let widths: Vec<f32> = (280..=600).step_by(8).map(|w| w as f32).collect();
    let set = |h: &mut Harness<'_, Moonglow>, width: f32| {
        right_dock_width(h, width);
        settle(h);
        let dock = right_dock_rect(h);
        assert!((dock.width() - width).abs() < 1.5, "{width}: {dock:?}");
    };
    let (wrong, seen) = sweep(&mut h, &widths, &set);
    assert!(seen > 20_000, "the panel's nodes looked at: {seen}");
    assert!(wrong.is_empty(), "{} found:\n{}", wrong.len(), wrong.join("\n"));
}

/// The palette in the Right Dock where the window is too narrow for its
/// least width beside the Left Dock (both unfolded): from the least
/// window, 420 points, where the dock is 190 wide, a window 8 points
/// wider at a time to where the dock has its 280, on every type of
/// palette as `the_palette_fits_the_right_dock_from_its_least_width_to_600`.
/// (The editor area is then left little: the shell's yields, M3 T14,
/// fold the Left Dock.)
#[test]
fn the_palette_fits_the_right_dock_in_the_narrowest_window() {
    let Some((mut h, _)) = area_harness("right-dock-sweep-narrow") else { return };
    let least = mg_ui::LEAST_WINDOW_SIZE.x;
    let set = |h: &mut Harness<'_, Moonglow>, window: f32| {
        h.set_size(egui::vec2(window, 800.0));
        settle(h);
        assert!(!h.state().settings.hide_tree && !h.state().settings.hide_palettes);
    };
    set(&mut h, least);
    let narrowest = right_dock_rect(&h).width();
    assert!((narrowest - 190.0).abs() < 2.0, "the dock at the least window: {narrowest}");
    // (The windows from the least to where the dock is 280.)
    let mut windows = Vec::new();
    let mut window = least;
    loop {
        set(&mut h, window);
        windows.push(window);
        if right_dock_rect(&h).width() >= mg_ui::RIGHT_DOCK_LEAST - 0.5 {
            break;
        }
        window += 8.0;
        assert!(window < 800.0, "the dock never reached its least width");
    }
    assert!(windows.len() >= 10, "{windows:?}");
    let (wrong, seen) = sweep(&mut h, &windows, &set);
    assert!(seen > 5_000, "the panel's nodes looked at: {seen}");
    assert!(wrong.is_empty(), "{} found:\n{}", wrong.len(), wrong.join("\n"));
}

/// F6 gives the arrow keys to the panes left to right and then down: the
/// Left Dock's panel, the editor area (the area's view), the Right Dock's
/// Palette panel, the Bottom Dock's log, and round; Shift+F6 the other
/// way. A folded Right Dock is passed over.
#[test]
fn f6_steps_through_the_docks_and_the_editor_area() {
    let Some((mut h, area)) = area_harness("right-dock-f6") else { return };
    h.state_mut().settings.hide_log = false;
    settle(&mut h);
    // (The pointer in the area's view: the keys are its.)
    let centre = h.state().area_views[&area].rect.center();
    h.hover_at(centre);
    settle(&mut h);
    let keys = |h: &Harness<'_, Moonglow>| mg_ui::palette_view::arrow_keys_are(&h.ctx);
    assert_eq!(keys(&h), "Area");
    let step = |h: &mut Harness<'_, Moonglow>, back: bool| {
        let m = if back { egui::Modifiers::SHIFT } else { egui::Modifiers::NONE };
        h.key_press_modifiers(m, egui::Key::F6);
        settle(h);
        keys(h)
    };
    // (Away from the view, which takes the keys back while the pointer
    // is in it.)
    h.hover_at(egui::pos2(centre.x, 20.0));
    settle(&mut h);
    let forward: Vec<String> = (0..4).map(|_| step(&mut h, false)).collect();
    assert_eq!(forward, ["Palette", "Log", "Tree", "Area"]);
    let back: Vec<String> = (0..4).map(|_| step(&mut h, true)).collect();
    assert_eq!(back, ["Tree", "Log", "Palette", "Area"]);
    // Folded, the Right Dock is passed over.
    fold_palette(h.state_mut());
    settle(&mut h);
    let forward: Vec<String> = (0..3).map(|_| step(&mut h, false)).collect();
    assert_eq!(forward, ["Log", "Tree", "Area"]);
}

/// A settings file of an older Moonglow, with the palettes split off the
/// middle's left (their own place, `split left 300`), a kind kept in the
/// palettes' pane (place `palettes`) and the palettes folded away: it
/// reads with no warning; the app opened with it has none of those
/// places, every other as it was, and the Right Dock folded (the
/// palettes were), its strip at the window's right; Tools › Palettes
/// unfolds it.
#[test]
fn old_settings_with_the_palettes_places_open_right() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-right-dock-old-settings");
    let old = mg_ui::Settings {
        window_places: vec![
            ("palette".into(), "split left 300".into()),
            ("factions".into(), "palettes".into()),
            ("manual".into(), "split below 330".into()),
        ],
        window_sizes: vec![("palette".into(), [400, 700]), ("script".into(), [1100, 700])],
        hide_palettes: true,
        ..Default::default()
    };
    let file = dir.join("app.ron");
    let map: std::collections::HashMap<String, String> =
        [("moonglow-settings".to_string(), old.saved_text())].into_iter().collect();
    std::fs::write(&file, ron::to_string(&map).unwrap()).unwrap();
    let read = mg_ui::Settings::read_eframe(&file, "moonglow-settings");
    assert!(read.unread.is_empty() && read.unreadable.is_none(), "{:?}", read.problem());
    assert!(read.kept.is_none(), "nothing put aside");
    let migrated = Moonglow::with_settings(read.settings, Box::new(NoDialogs::default()));
    let app = &migrated;
    assert_eq!(app.settings.window_places, [("manual".to_string(), "split below 330".to_string())]);
    assert_eq!(app.settings.window_sizes, [("script".to_string(), [1100, 700])]);
    let said: Vec<_> =
        (app.log.entries.iter()).filter(|(level, _)| *level != mg_ui::Level::Info).collect();
    assert!(said.is_empty(), "no warning: {said:?}");
    // The window with them (the game's data read as the tests read it).
    let mut app = game_app(&root, None);
    let s = &migrated.settings;
    app.settings.window_places = s.window_places.clone();
    app.settings.window_sizes = s.window_sizes.clone();
    app.settings.hide_palettes = s.hide_palettes;
    let path = sample_module(&dir);
    app.open_module(&path);
    let mut h = harness(app, 1400.0, 800.0);
    settle(&mut h);
    assert!(folded(&h), "folded, as the palettes were");
    let show = h.get_by_label("Show the Right Dock").rect();
    assert!(show.right() > 1400.0 - 60.0, "at the window's right: {show:?}");
    // The Faction Editor, kept in the palettes' pane: where its kind
    // opens, a window of its own.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Factions));
    settle(&mut h);
    let at = h.state().dock.find_tab(&Tab::Factions).unwrap();
    assert!(!at.surface.is_main(), "a window of its own");
    menu_command(&mut h, "Tools", "Palettes");
    settle(&mut h);
    assert!(!folded(&h));
}

/// The settings as JSON (a settings file is not; a plugin's or a test's
/// may be) with the palettes' places: migrated as the app takes them.
#[test]
fn json_settings_with_the_palettes_places_open_without_them() {
    let json = r#"{
        "window_places": [["palette", "split left 300"], ["journal", "palettes"], ["script", "middle"]],
        "window_sizes": [["palette", [400, 700]]],
        "hide_palettes": true
    }"#;
    let read: mg_ui::Settings = serde_json::from_str(json).unwrap();
    assert_eq!(read.window_places.len(), 3, "read as it is");
    let app = Moonglow::with_settings(read, Box::new(NoDialogs::default()));
    assert_eq!(app.settings.window_places, [("script".to_string(), "middle".to_string())]);
    assert!(app.settings.window_sizes.is_empty());
    assert!(app.settings.hide_palettes);
}

/// A press in the Palette panel makes it the plugins' "pane last clicked
/// in" (as a press in the palettes' tab did): a command from the Plugins
/// menu then runs on the blueprint chosen in the palette, not on what the
/// module tree has chosen.
#[test]
fn a_press_in_the_palette_makes_it_the_plugins_pane() {
    let Some((mut h, _)) = area_harness("right-dock-plugin-pane") else { return };
    {
        let p = &mut h.state_mut().palette;
        (p.tiles, p.kind, p.custom) = (false, BlueprintKind::Waypoint, false);
        p.filter = "nw_wp_tavern".into();
    }
    settle(&mut h);
    let tavern = h.state().game.as_deref().unwrap().string(mg_core::StrRef(69068)).unwrap();
    // (The tree's choice, and its pane the one clicked in last.)
    let start = ResKey::parse("start", ResType::ARE).unwrap();
    h.state_mut().tree_selected = [start].into_iter().collect();
    h.state_mut().plugins.pane = Some(mg_ui::plugins::Pane::Tree);
    h.get_by_label(&tavern).click();
    settle(&mut h);
    assert_eq!(h.state().plugins.pane, Some(mg_ui::plugins::Pane::Palette));
    let chosen = h.state().palette.selected.expect("the waypoint chosen");
    assert_eq!(h.state().plugin_selection(), [mg_plugin::Selected::Blueprint(chosen)]);
    // A press in the panel away from its rows: the palette's still.
    h.state_mut().plugins.pane = Some(mg_ui::plugins::Pane::Tree);
    let types = h.get_by_label("Palette types").get_by_label("Waypoints").rect();
    click_pos(&h, types.center());
    settle(&mut h);
    assert_eq!(h.state().plugins.pane, Some(mg_ui::plugins::Pane::Palette));
}

/// A window maximized over the whole window (the docks too) is put back
/// by a double click on its bar wherever the bar is: over the Left Dock,
/// the editor area or the Right Dock. (Its bar's clicks were cut to the
/// editor area: over a dock, a double click did nothing.)
#[test]
fn a_maximized_window_s_bar_puts_it_back_over_the_docks() {
    let Some((mut h, area)) = area_harness("right-dock-maximized") else { return };
    let props = Tab::AreaProperties(area);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(props.clone()));
    settle(&mut h);
    let pane = |h: &Harness<'_, Moonglow>| {
        let path = h.state().dock.find_tab(&props).unwrap();
        h.state().dock.iter_leaves().find(|(p, _)| p.surface == path.surface).unwrap().1.rect
    };
    let first = pane(&h);
    let left = h.get_by_label("Fold the Left Dock").rect().left() - 40.0;
    let middle = h.state().area_views[&area].rect.center().x;
    let right = right_dock_rect(&h).center().x;
    for x in [left, middle, right] {
        h.state_mut().actions.push(mg_ui::Action::ToggleMaximize(props.clone()));
        settle(&mut h);
        let big = pane(&h);
        assert!(big.width() > first.width() + 200.0, "maximized: {big:?}");
        let at = egui::pos2(x, big.top() + 10.0);
        h.hover_at(at);
        settle(&mut h);
        for _ in 0..2 {
            press(&h, at, true, egui::Modifiers::NONE);
            press(&h, at, false, egui::Modifiers::NONE);
        }
        let clicked = h.ctx.input(|i| i.time);
        settle(&mut h);
        assert!((pane(&h).size() - first.size()).length() < 3.0, "put back from {x}");
        h.hover_at(at + egui::vec2(0.0, 300.0));
        settle(&mut h);
        // (Until these clicks are too long ago to make a triple click with
        // the next two: egui counts clicks together by the time between.)
        let apart = 2.0 * h.ctx.options(|o| o.input_options.max_double_click_delay);
        let started = std::time::Instant::now();
        while h.ctx.input(|i| i.time) - clicked <= apart {
            h.step();
            assert!(started.elapsed() < SETTLE_TIME, "the window's time stands still");
        }
    }
}

/// The palette's scroll bar, at the height of the Right Dock's fold
/// button (the button stands beside the dock's inner edge, the bar at its
/// outer one): a press on it scrolls the list and folds nothing. (The
/// edge dragged above and below the button, and a press beside it:
/// `windows::each_pane_folds_by_one_button_on_its_inner_edge`.)
#[test]
fn a_press_on_the_palette_s_scroll_bar_scrolls_it() {
    let Some((mut h, _)) = area_harness("right-dock-scroll-bar") else { return };
    {
        let p = &mut h.state_mut().palette;
        (p.tiles, p.kind, p.custom) = (false, BlueprintKind::Placeable, false);
    }
    settle(&mut h);
    // Every category open: a list longer than the dock.
    expand_all_in_the_dock(&mut h);
    let dock = right_dock_rect(&h);
    let fold = h.get_by_label("Fold the Right Dock").rect();
    let before = top_label(&h);
    // The bar inside the panel's 8 px margin, at the dock's outer edge,
    // at the button's height (the kit's bar: 4 wide, 2 from the list's
    // side).
    let at = egui::pos2(dock.right() - 12.0, fold.center().y);
    drag(&mut h, at, at + egui::vec2(0.0, 120.0));
    settle(&mut h);
    assert!(!h.state().settings.hide_palettes, "the dock left open");
    assert_ne!(top_label(&h), before, "the list scrolled");
}

/// The palette's lists keep their own scroll places: the placeables'
/// scrolled down stays so through the tiles palette shown and back, the
/// tiles' list at its top meanwhile; and the Left Dock's tree keeps its
/// place too (each scroll area under an id of its own).
#[test]
fn the_palette_s_lists_keep_their_own_scroll_places() {
    let Some((mut h, _)) = area_harness("right-dock-scroll-places") else { return };
    let set = |h: &mut Harness<'_, Moonglow>, tiles: bool| {
        let p = &mut h.state_mut().palette;
        (p.tiles, p.kind, p.custom) = (tiles, BlueprintKind::Placeable, false);
        settle(h);
    };
    set(&mut h, false);
    expand_all_in_the_dock(&mut h);
    let dock = right_dock_rect(&h);
    let at_top = top_label(&h);
    let over = egui::pos2(dock.center().x, dock.bottom() - 100.0);
    h.hover_at(over);
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -2000.0),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::NONE,
    });
    // (egui's scrolling is animated: until the list is still.)
    at_rest(&mut h, "the scrolled list", &top_row);
    let scrolled = top_label(&h);
    assert_ne!(scrolled, at_top, "scrolled");
    let tree = h.state().tree_cursor;
    set(&mut h, true);
    let tiles_top = top_label(&h);
    set(&mut h, false);
    at_rest(&mut h, "the placeables' list", &top_row);
    assert_eq!(top_label(&h), scrolled, "the placeables' list where it was");
    set(&mut h, true);
    assert_eq!(top_label(&h), tiles_top, "the tiles' list where it was");
    assert_eq!(h.state().tree_cursor, tree);
}

/// View › Reset Layout puts the Right Dock back as it is at first:
/// unfolded, 320 points wide, however it was left.
#[test]
fn reset_layout_puts_the_right_dock_back_at_320() {
    let Some(mut h) = dock_harness("ui-right-dock-reset", 1400.0) else { return };
    right_dock_width(&mut h, 470.0);
    settle(&mut h);
    assert!((right_dock_rect(&h).width() - 470.0).abs() < 1.5);
    fold_palette(h.state_mut());
    settle(&mut h);
    menu_command(&mut h, "View", "Reset Layout");
    settle(&mut h);
    assert!(!folded(&h));
    let width = right_dock_rect(&h).width();
    let want = mg_uikit::metrics::RIGHT_DOCK;
    assert!((width - want).abs() < 1.5, "{width}, not {want}");
}

/// In the least window (and at 460, 480, 500 and 600), both side docks
/// unfolded: nothing panics, the two fold buttons stand apart, the Left
/// Dock's first, the Right Dock inside the window (narrower than its least
/// width where the window has not the room: never past its edge), nothing
/// in the Right Dock outside it or over another of its widgets, nothing
/// else drawn across its edges ([`dock_wrong`]), and at 600 nothing in the
/// window overlaps. (The editor area is left little room under about 760,
/// and the Bottom Dock's tab is over its buttons under about 500: the
/// shell's yields, M3 T14, fold the Left Dock in narrow windows.)
#[test]
fn the_least_window_holds_both_docks() {
    let Some(mut h) = dock_harness("ui-right-dock-least-window", 1400.0) else { return };
    let sizes = [
        mg_ui::LEAST_WINDOW_SIZE,
        egui::vec2(460.0, 400.0),
        egui::vec2(480.0, 400.0),
        egui::vec2(500.0, 450.0),
        egui::vec2(600.0, 500.0),
    ];
    let mut seen = 0;
    for size in sizes {
        h.set_size(size);
        settle(&mut h);
        assert!(!h.state().settings.hide_tree && !folded(&h));
        let window = h.ctx.content_rect();
        let left = h.get_by_label("Fold the Left Dock").rect();
        let right = h.get_by_label("Fold the Right Dock").rect();
        let both = left.intersect(right);
        assert!(!both.is_positive(), "at {size:?}: {left:?} {right:?}");
        let dock = right_dock_rect(&h);
        assert!(window.expand(0.5).contains_rect(dock), "at {size:?}: {dock:?} in {window:?}");
        assert!(left.right() <= right.left() + 0.5, "the Left Dock's first: {left:?} {right:?}");
        let wrong = dock_wrong(&h, &format!("{size:?}"), &mut seen);
        assert!(wrong.is_empty(), "in the Right Dock:\n{}", wrong.join("\n"));
        if size.x >= 600.0 {
            let found = overlapping(&h);
            assert!(found.is_empty(), "at {size:?}:\n{}", found.join("\n"));
        }
    }
    assert!(seen > 100, "the dock's nodes looked at: {seen}");
}

/// The Right Dock's inner edge dragged by `by` (to the left, wider), above
/// its fold button.
fn drag_edge(h: &mut Harness<'_, Moonglow>, by: f32) {
    let dock = right_dock_rect(h);
    let from = egui::pos2(dock.left() + 1.0, dock.top() + 150.0);
    drag(h, from, from - egui::vec2(by, 0.0));
    settle(h);
}

/// The Right Dock's width now (it must be shown).
fn right_width(h: &Harness<'_, Moonglow>) -> f32 {
    right_dock_rect(h).width()
}

/// The width the Right Dock is given is kept apart from the width it is
/// drawn at: dragged to 470, it is narrower in a window too narrow for it
/// (900 wide, then 700) and 470 again when the window is wide again (it
/// stayed as narrow as the narrowest window, for good: egui kept the
/// width drawn as the panel's); dragged in a narrow window, the width
/// dragged to is kept when the window widens; the width chosen is saved
/// and read back by the next run, which opens in a narrow window without
/// losing it.
#[test]
fn the_right_dock_keeps_its_width_through_a_narrow_window() {
    let Some(mut h) = dock_harness("ui-right-dock-width-kept", 1400.0) else { return };
    let size = |h: &mut Harness<'_, Moonglow>, width: f32| {
        h.set_size(egui::vec2(width, 800.0));
        settle(h);
    };
    let first = right_width(&h);
    drag_edge(&mut h, 150.0);
    let wide = right_width(&h);
    assert!((wide - first - 150.0).abs() < 2.0, "dragged: {first} -> {wide}");
    assert_eq!(h.state().settings.right_dock_width, Some(wide.round() as u32));
    size(&mut h, 900.0);
    let at_900 = right_width(&h);
    assert!(at_900 < wide - 60.0, "narrower at 900: {at_900}");
    size(&mut h, 700.0);
    let at_700 = right_width(&h);
    assert!(at_700 < at_900 - 30.0, "narrower still at 700: {at_700}");
    size(&mut h, 1400.0);
    assert!((right_width(&h) - wide).abs() < 1.0, "{wide} again, not {}", right_width(&h));
    // Dragged while the window is narrow: the width dragged to, kept.
    size(&mut h, 900.0);
    drag_edge(&mut h, -40.0);
    let dragged = right_width(&h);
    assert!((dragged - (at_900 - 40.0)).abs() < 2.0, "dragged narrow: {at_900} -> {dragged}");
    size(&mut h, 1400.0);
    assert!((right_width(&h) - dragged).abs() < 1.0, "{dragged} kept, not {}", right_width(&h));
    // Saved, and read back by the next run, in a window too narrow for it.
    let dir = mg_testkit::scratch_dir("ui-right-dock-width-kept-again");
    let file = dir.join("app.ron");
    let map: std::collections::HashMap<String, String> =
        [("moonglow-settings".to_string(), h.state().settings.saved_text())].into_iter().collect();
    std::fs::write(&file, ron::to_string(&map).unwrap()).unwrap();
    let read = mg_ui::Settings::read_eframe(&file, "moonglow-settings");
    assert!(read.unread.is_empty() && read.unreadable.is_none(), "{:?}", read.problem());
    let migrated = Moonglow::with_settings(read.settings, Box::new(NoDialogs::default()));
    let root = mg_testkit::nwn_root().unwrap();
    let mut app = game_app(&root, None);
    app.settings = migrated.settings;
    app.open_module(&sample_module(&dir));
    let mut h = harness(app, 700.0, 800.0);
    settle(&mut h);
    assert!(right_width(&h) < dragged - 20.0, "held to the narrow window");
    assert_eq!(h.state().settings.right_dock_width, Some(dragged.round() as u32), "not lost");
    size(&mut h, 1400.0);
    assert!((right_width(&h) - dragged).abs() < 1.0, "{dragged} again, not {}", right_width(&h));
}

/// A Right Dock width egui kept from an earlier run (under the panel's
/// id, before the width chosen was kept apart) is taken as the width
/// chosen, once: drawn so where the window has the room, and kept as it
/// is in a window without it.
#[test]
fn a_width_kept_by_an_earlier_run_is_the_right_dock_s() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-right-dock-width-before");
    let path = sample_module(&dir);
    for window in [1400.0, 700.0] {
        let mut app = game_app(&root, None);
        assert_eq!(app.settings.right_dock_width, None);
        app.open_module(&path);
        let mut h = harness(app, window, 800.0);
        // (Settings with no width chosen, and the width egui kept for the
        // panel: as an earlier run left them. The harness has drawn a
        // first frame, which took the first width: given up.)
        h.state_mut().settings.right_dock_width = None;
        let rect = egui::Rect::from_min_size(egui::pos2(900.0, 40.0), egui::vec2(430.0, 600.0));
        let state = egui::containers::panel::PanelState { outer_rect: rect };
        h.ctx.data_mut(|d| d.insert_persisted(egui::Id::new("right-dock"), state));
        settle(&mut h);
        assert_eq!(h.state().settings.right_dock_width, Some(430), "at {window}");
        let width = right_width(&h);
        if window > 1000.0 {
            assert!((width - 430.0).abs() < 1.0, "{width}");
        } else {
            assert!(width < 400.0, "held to the window at {window}: {width}");
            h.set_size(egui::vec2(1400.0, 800.0));
            settle(&mut h);
            assert!((right_width(&h) - 430.0).abs() < 1.0, "{}", right_width(&h));
        }
    }
}

/// With no module open and the Right Dock the only dock shown (Tools ›
/// Palettes; no Resources panel asked for), F6 and Shift+F6 still go round
/// the panes: from the editor area to the Right Dock's Palette panel, the
/// Bottom Dock's log and back. (They were grayed: Next Pane counted the
/// Left Dock's panels only.) No game data is needed for it.
#[test]
fn f6_reaches_the_right_dock_with_no_module() {
    let mut h = harness(app_with(Vec::new()), 1400.0, 800.0);
    h.state_mut().settings.hide_log = false;
    settle(&mut h);
    assert!(h.query_by_label("Fold the Left Dock").is_none(), "no Left Dock");
    menu_command(&mut h, "Tools", "Palettes");
    settle(&mut h);
    assert!(palette_shown(h.state()));
    h.get_by_label("Fold the Right Dock");
    let keys = |h: &Harness<'_, Moonglow>| mg_ui::palette_view::arrow_keys_are(&h.ctx);
    let step = |h: &mut Harness<'_, Moonglow>, back: bool| {
        let m = if back { egui::Modifiers::SHIFT } else { egui::Modifiers::NONE };
        h.key_press_modifiers(m, egui::Key::F6);
        settle(h);
        keys(h)
    };
    let forward: Vec<String> = (0..3).map(|_| step(&mut h, false)).collect();
    assert_eq!(forward, ["Palette", "Log", "Area"], "F6");
    let back: Vec<String> = (0..3).map(|_| step(&mut h, true)).collect();
    assert_eq!(back, ["Log", "Palette", "Area"], "Shift+F6");
    // The Log closed, the Right Dock and the editor area alone.
    h.state_mut().settings.hide_log = true;
    settle(&mut h);
    let forward: Vec<String> = (0..2).map(|_| step(&mut h, false)).collect();
    assert_eq!(forward, ["Palette", "Area"], "F6 with the log closed");
}

/// Tools › Palettes with no game data read shows the Right Dock all the
/// same, its Palette panel saying so (as the palettes' pane did; it did
/// nothing), with no module open and with one; View › Right Dock is then
/// enabled, ticked, and folds it to its strip, whose Palette button and
/// the button beside it bring it back.
#[test]
fn tools_palettes_without_game_data_says_so() {
    let dir = mg_testkit::scratch_dir("ui-right-dock-no-game");
    let path = sample_module(&dir);
    for module in [false, true] {
        let mut app = app_with(Vec::new());
        app.settings.no_last_area = true;
        if module {
            app.open_module(&path);
        }
        let mut h = harness(app, 1400.0, 800.0);
        settle(&mut h);
        assert!(h.state().game.is_none());
        assert!(h.query_by_label("Fold the Right Dock").is_none(), "{module}: not at first");
        h.get_by_label("View").click();
        h.run();
        let grayed = h.get_by_label("Right Dock").accesskit_node().is_disabled();
        assert!(grayed, "{module}: View › Right Dock grayed with no panel to show");
        h.key_press(egui::Key::Escape);
        settle(&mut h);
        menu_command(&mut h, "Tools", "Palettes");
        settle(&mut h);
        assert!(palette_shown(h.state()), "{module}: shown");
        h.get_by_label("Fold the Right Dock");
        let said = h.get_by_label("No game data.").rect();
        let dock = right_dock_rect(&h);
        assert!(dock.contains_rect(said), "{module}: said in the dock: {said:?} {dock:?}");
        // View › Right Dock: enabled, ticked, folds it.
        h.get_by_label("View").click();
        h.run();
        let checkbox = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.role() == egui::accesskit::Role::CheckBox
                && n.label().as_deref() == Some("Right Dock")
        };
        let row = h.get(egui_kittest::kittest::by().predicate(checkbox));
        let row = row.accesskit_node();
        assert!(!row.is_disabled(), "{module}: View › Right Dock enabled");
        assert_eq!(row.toggled(), Some(egui::accesskit::Toggled::True), "{module}: ticked");
        h.key_press(egui::Key::Escape);
        settle(&mut h);
        menu_command(&mut h, "View", "Right Dock");
        settle(&mut h);
        assert!(h.state().settings.hide_palettes, "{module}: folded");
        assert!(h.query_by_label("No game data.").is_none());
        h.get_by_label("Right Dock").get_by_label("Palette").click();
        settle(&mut h);
        assert!(palette_shown(h.state()), "{module}: the strip's Palette button");
        h.get_by_label("No game data.");
        h.get_by_label("Fold the Right Dock").click();
        settle(&mut h);
        h.get_by_label("Show the Right Dock").click();
        settle(&mut h);
        assert!(palette_shown(h.state()), "{module}: the button beside the strip");
    }
}
