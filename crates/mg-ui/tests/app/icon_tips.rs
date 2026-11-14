//! Every control drawn as an icon alone says what it is: a name for a
//! screen reader and a tooltip under the pointer (the redesign's rule,
//! `docs/ui-redesign-proposal.md` section 7), in the main windows with
//! Options › General › Editors' toolbar buttons at Icons only.

use crate::common::*;

/// The buttons drawn as an icon alone (a button no wider than an icon
/// button with an icon drawn in it), and a dock's tabs showing their icons
/// alone (a tab no wider than an icon and its padding), each once: its
/// name, and where.
fn icon_buttons(h: &Harness<'_, Moonglow>) -> Vec<(String, egui::Rect)> {
    use egui::accesskit::Role;
    let screen = h.ctx.content_rect();
    let drawn = mg_uikit::icons_drawn(&h.ctx);
    let side = mg_uikit::metrics::ICON_BUTTON + 4.0;
    let tab = mg_uikit::metrics::ICON + 2.0 * mg_uikit::shell::DOCK_TAB_ICON_PADDING + 2.0;
    let mut out: Vec<(String, egui::Rect)> = Vec::new();
    for n in h.query_all_by(|n| matches!(n.role(), Role::Button | Role::Tab)) {
        let r = n.rect();
        let wide = if n.accesskit_node().role() == Role::Tab { tab } else { side };
        if r.width() > wide || r.height() > side + 4.0 || !screen.contains_rect(r) {
            continue;
        }
        if !drawn.iter().any(|(_, at)| r.expand(1.0).contains_rect(*at)) {
            continue;
        }
        let name = n.accesskit_node().label().unwrap_or_default();
        out.push((name, r));
    }
    out
}

/// Each icon button in sight not seen before (`seen`, by name and place)
/// has a name and a tooltip; what lacks one is added to `wrong`.
fn walk(
    h: &mut Harness<'_, Moonglow>,
    window: &str,
    seen: &mut Vec<(String, egui::Rect)>,
    wrong: &mut Vec<String>,
) -> usize {
    let mut walked = 0;
    for (name, rect) in icon_buttons(h) {
        if seen.iter().any(|(n, r)| *n == name && r.center().distance(rect.center()) < 2.0) {
            continue;
        }
        seen.push((name.clone(), rect));
        walked += 1;
        if name.trim().is_empty() {
            wrong.push(format!("{window}: a button at {rect:?} has no name"));
            continue;
        }
        match tip_at(h, rect) {
            Some(tip) if !tip.trim().is_empty() => {}
            _ => wrong.push(format!("{window}: {name} at {rect:?} has no tooltip")),
        }
    }
    walked
}

/// With Icons only, every icon-only button of the main window with an
/// area open, a creature's editor, the conversation editor, the script
/// editor, the NUI Creator, Options and the hak editor has a name and a
/// tooltip, enabled or not (an icon alone, grayed, says least).
#[test]
fn every_icon_only_button_has_a_name_and_a_tooltip() {
    let Some((mut h, _area)) = area_harness("icon-tips") else { return };
    // (The builder unfolds the Left Dock, folded at this width: M3 T14.)
    unfold_left_dock(&mut h);
    h.state_mut().settings.toolbar_icons_only = true;
    settle(&mut h);
    let (mut seen, mut wrong) = (Vec::new(), Vec::new());
    let mut counts = Vec::new();
    counts.push(("main window", walk(&mut h, "main window", &mut seen, &mut wrong)));
    // The Left Dock at its least width: its tabs as icons, on both tabs.
    left_dock_width(&mut h, 190.0);
    settle(&mut h);
    // (Walked afresh, not passed over as seen in the main window's walk:
    // since M3 T14's review the fold button stands on the tab bar, and at
    // the dock's first 260 its tabs are icons already, where they are at
    // 190.)
    let mut at_least = Vec::new();
    counts.push(("Left Dock", walk(&mut h, "Left Dock (Module)", &mut at_least, &mut wrong)));
    show_resources(h.state_mut());
    settle(&mut h);
    counts.push(("Resources", walk(&mut h, "Left Dock (Resources)", &mut at_least, &mut wrong)));
    let tabs: Vec<&String> =
        (at_least.iter()).map(|(n, _)| n).filter(|n| *n == "Module" || *n == "Resources").collect();
    assert_eq!(tabs.len(), 2, "the icon tabs walked at 190: {at_least:?}");
    seen.extend(at_least);
    h.state_mut().actions.push(mg_ui::Action::ShowPanel(mg_ui::Panel::Module));
    left_dock_width(&mut h, 260.0);
    settle(&mut h);
    // The editors, each opened in turn (and closed after: one at a time
    // in sight, none over another's buttons).
    let game = h.state().game.clone().expect("the game's data");
    let chicken = game.resman.get_named("nw_chicken", ResType::UTC).unwrap().into_owned();
    let creature = ResKey::parse("walk_chicken", ResType::UTC).unwrap();
    let mut talk = mg_module::dialog::new_dialog();
    let hello = mg_module::dialog::add_node(&mut talk, mg_module::dialog::Parent::Root, "Hello.");
    let parent = mg_module::dialog::Parent::Node(mg_module::dialog::Kind::Entry, hello);
    mg_module::dialog::add_node(&mut talk, parent, "Bye.");
    let dialog = ResKey::parse("walk_talk", ResType::DLG).unwrap();
    let script = ResKey::parse("walk_script", ResType::NSS).unwrap();
    let made = vec![
        (creature, chicken),
        (dialog, talk.to_bytes().unwrap()),
        (script, b"void main()\n{\n    int n = 1;\n}\n".to_vec()),
    ];
    let edits = made
        .into_iter()
        .map(|(key, data)| mg_edit::Edit::SetResource { key, data: Some(data) })
        .collect();
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("walk", edits)));
    settle(&mut h);
    for (tab, window) in [
        (Tab::Blueprint(creature), "creature"),
        (Tab::Dialog(dialog), "conversation"),
        (Tab::Script(script), "script"),
    ] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
        settle(&mut h);
        counts.push((window, walk(&mut h, window, &mut seen, &mut wrong)));
        let at = h.state().dock.find_tab(&tab).unwrap();
        h.state_mut().dock.remove_tab(at);
        settle(&mut h);
    }
    h.state_mut().actions.push(mg_ui::Action::NewNui("walk_win".into()));
    settle(&mut h);
    counts.push(("NUI Creator", walk(&mut h, "NUI Creator", &mut seen, &mut wrong)));
    let nui = h.state().dock.iter_all_tabs().find_map(|(_, t)| match t {
        Tab::Nui(k) => Some(Tab::Nui(*k)),
        _ => None,
    });
    if let Some(nui) = nui {
        let at = h.state().dock.find_tab(&nui).unwrap();
        h.state_mut().dock.remove_tab(at);
        settle(&mut h);
    }
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    settle(&mut h);
    for page in ["General", "Keyboard"] {
        h.get_by_label(page).click();
        settle(&mut h);
        counts.push(("Options", walk(&mut h, page, &mut seen, &mut wrong)));
    }
    drop(h);
    // The hak editor, in a window of its own.
    let dir = mg_testkit::scratch_dir("ui-icon-tips-hak");
    let hak_path = dir.join("stuff.hak");
    let mut hak = mg_module::hak_edit::Hak::new();
    let key = ResKey::parse("mystore", ResType::TWODA).unwrap();
    let table = b"2DA V2.0\n\n   LABEL\n0  one\n".to_vec();
    hak.add_sources(vec![(key, mg_module::hak_edit::Source::Bytes(table.into()))]).unwrap();
    hak.save(Some(&hak_path)).unwrap();
    let dialogs = NoDialogs { open: vec![hak_path], ..Default::default() };
    let mut app = Moonglow::new(None, Box::new(dialogs));
    app.settings.toolbar_icons_only = true;
    let mut h = harness(app, 1000.0, 700.0);
    h.run();
    mg_ui::hak_view::open_hak(h.state_mut());
    settle(&mut h);
    counts.push(("hak editor", walk(&mut h, "hak editor", &mut seen, &mut wrong)));
    assert!(wrong.is_empty(), "{wrong:#?}");
    // (Each window had icon buttons to walk: the walk can fail.)
    for (window, n) in &counts {
        assert!(*n > 0 || *window == "Options", "{window}: no icon-only buttons found");
    }
    let total: usize = counts.iter().map(|c| c.1).sum();
    assert!(total > 40, "{total} icon-only buttons walked: {counts:?}");
}
