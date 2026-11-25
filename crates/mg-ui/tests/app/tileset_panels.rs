//! The tileset editor and Problems (M4's U20; the Tileset board): the
//! header (the pages' switch, Save, Make Palette, Check, Render Minimap
//! Pictures, OK and Cancel) at every width; the tile table and its keys;
//! typed text keeping to its tile; the Right Dock's Preview (the diagram,
//! the door sentence, the model in a tab of its own) and Textures; the
//! Outliner's parts and their counts; Check's findings in Problems with the
//! switch's count, a click going to the tile (the tileset closed too); the
//! hints and counts; one Undo step an edit; the window coming to rest; a
//! floating tileset's first size; records from before reading without a
//! fault.

use crate::common::*;
use mg_ui::Panel;
use mg_ui::problems::Stale;
use mg_ui::tileset_view::{self as tv, Page, TILESET_HEADER};

/// A tileset of ours (CRLF, as the game's): two terrains and a crosser, a
/// painting rule; three tiles, the first a game model with a door, the
/// second a model nowhere (an error), the third a game model; a group
/// whose first tile is -1 (an error) and a height step of 0 with a group
/// (a warning).
const SET: &str = "[GENERAL]\r\nName=ZZT01\r\nUnlocalizedName=Test Tiles\r\nTransition=0\r\n\
EnvMap=\r\nBorder=Grass\r\nDefault=Grass\r\nFloor=Grass\r\n\r\n\
[GRASS]\r\nGrass=0\r\n\r\n\
[TERRAIN TYPES]\r\nCount=2\r\n\r\n[TERRAIN0]\r\nName=Grass\r\n\r\n[TERRAIN1]\r\nName=Water\r\n\r\n\
[CROSSER TYPES]\r\nCount=1\r\n\r\n[CROSSER0]\r\nName=Road\r\n\r\n\
[PRIMARY RULES]\r\nCount=1\r\n\r\n[PRIMARY RULE0]\r\nPlaced=Grass\r\nPlacedHeight=0\r\n\
Adjacent=Water\r\nAdjacentHeight=0\r\nChanged=Grass\r\nChangedHeight=0\r\n\r\n\
[SECONDARY RULES]\r\nCount=0\r\n\r\n\
[TILES]\r\nCount=3\r\n\r\n\
[TILE0]\r\nModel=tcn01_a01_01\r\nWalkMesh=msb01\r\nTopLeft=Grass\r\nTopLeftHeight=0\r\n\
TopRight=Grass\r\nTopRightHeight=0\r\nBottomLeft=Water\r\nBottomLeftHeight=0\r\n\
BottomRight=Grass\r\nBottomRightHeight=0\r\nTop=\r\nRight=Road\r\nBottom=\r\nLeft=\r\n\
MainLight1=1\r\nMainLight2=0\r\nSourceLight1=0\r\nSourceLight2=0\r\nAnimLoop1=0\r\n\
AnimLoop2=0\r\nAnimLoop3=0\r\nDoors=1\r\nSounds=0\r\nPathNode=A\r\nOrientation=0\r\n\
ImageMap2D=MIZZT01_A01\r\n\r\n\
[TILE0DOOR0]\r\nType=0\r\nX=1.5\r\nY=-2.0\r\nZ=0.0\r\nOrientation=90.0\r\n\r\n\
[TILE1]\r\nModel=zzt01_a02_01\r\nWalkMesh=msb01\r\nTopLeft=Water\r\nTopLeftHeight=0\r\n\
TopRight=Water\r\nTopRightHeight=0\r\nBottomLeft=Water\r\nBottomLeftHeight=0\r\n\
BottomRight=Water\r\nBottomRightHeight=0\r\nDoors=0\r\nOrientation=0\r\n\r\n\
[TILE2]\r\nModel=tcn01_a02_01\r\nWalkMesh=msb01\r\nTopLeft=Grass\r\nTopLeftHeight=0\r\n\
TopRight=Grass\r\nTopRightHeight=0\r\nBottomLeft=Grass\r\nBottomLeftHeight=0\r\n\
BottomRight=Grass\r\nBottomRightHeight=0\r\nDoors=0\r\nOrientation=0\r\n\r\n\
[GROUPS]\r\nCount=1\r\n\r\n[GROUP0]\r\nName=Pair\r\nRows=1\r\nColumns=2\r\nTile0=-1\r\nTile1=0\r\n";

/// [`SET`] written in a scratch folder of its own, as `zzt01.set`.
fn set_file(name: &str) -> std::path::PathBuf {
    let dir = mg_testkit::scratch_dir(name);
    let path = dir.join("zzt01.set");
    std::fs::write(&path, SET).unwrap();
    path
}

/// [`SET`] open in the tileset editor in `mode`, in a window `width` by
/// 900, on the game installed at `game` where given and with a module of
/// ours open where `module`; on its Tiles page.
fn tileset_window(
    mode: Mode,
    name: &str,
    width: f32,
    game: Option<&std::path::Path>,
    module: bool,
) -> (Harness<'static, Moonglow>, std::path::PathBuf, u32) {
    let path = set_file(&mode.dir(name));
    let mut app = match game {
        Some(root) => game_app(root, None),
        None => app_with(Vec::new()),
    };
    app.settings = mode.settings();
    app.settings.no_last_area = true;
    if module {
        let dir = mg_testkit::scratch_dir(&mode.dir(&format!("{name}-module")));
        app.open_module(&sample_module(&dir));
    }
    let mut h = harness(app, width, 900.0);
    settle(&mut h);
    tv::open_path(h.state_mut(), path.clone());
    settle(&mut h);
    let id = h.state().tilesets[0].id;
    h.state_mut().tilesets[0].page = Page::Tiles;
    settle(&mut h);
    (h, path, id)
}

/// The tileset's document.
fn doc<'h>(h: &'h Harness<'_, Moonglow>) -> &'h tv::TilesetDoc {
    &h.state().tilesets[0]
}

/// The tileset as read.
fn set(h: &Harness<'_, Moonglow>) -> mg_set::Tileset {
    doc(h).tileset().expect("read").clone()
}

/// The tile table's row of tile `i`.
fn tile_row(h: &Harness<'_, Moonglow>, i: usize) -> egui::Rect {
    h.get_by_label(tv::TILES_TABLE).get_by_label(&format!("Tile {i}")).rect()
}

/// The Outliner's row named `name` (its words for a screen reader).
fn outline_row<'t>(h: &'t Harness<'_, Moonglow>, name: &'t str) -> egui_kittest::Node<'t> {
    h.get_by_label(mg_ui::OUTLINER).get_by_label(name)
}

/// The field named `name` in the tile's form: the text input whose value
/// is `value`.
fn field_with<'t>(h: &'t Harness<'_, Moonglow>, value: &'t str) -> egui_kittest::Node<'t> {
    let is_input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    h.get_all_by_value(value).find(is_input).unwrap_or_else(|| panic!("a field of {value}"))
}

/// Check's findings go to Problems (the plan's): Check finds the model
/// nowhere and the group's first tile -1 (errors) and the height step of
/// 0 (a warning); Problems lists them under the tileset's file, errors
/// first; the switch's badge counts them, as many as Check found, in
/// `error`; the Bottom Dock opens on Problems (closed before: Check asked
/// for them, as a search does). A second Check replaces them, never
/// appended; a change says they are of the file as it was, until Check
/// again. A Check that finds nothing opens nothing and says so.
#[test]
fn check_s_findings_are_problems_with_the_badge_s_count() {
    use mg_uikit::BadgeKind;
    let root = mg_testkit::corpus!();
    let (mut h, path, _) = tileset_window(Mode::Tabs, "ui-u20-check", 1600.0, Some(&root), false);
    h.state_mut().settings.hide_log = true;
    settle(&mut h);
    let badge = |h: &Harness<'_, Moonglow>| h.state().switch_badge(Panel::Problems);
    assert_eq!(badge(&h), None, "nothing checked, no count");
    assert!(h.state().bottom_dock_rect().is_none(), "the dock closed");
    tileset_tool(&mut h, tv::CHECK).click();
    settle(&mut h);
    let found = tileset_problems(&h, &path);
    let errors = found.iter().filter(|p| p.severity == mg_module::doctor::Severity::Error).count();
    assert!(errors >= 2 && found.len() > errors, "errors and a warning: {found:?}");
    assert!(found.iter().any(|p| p.at == "Tile 1, Model" && p.message.contains("zzt01_a02_01")));
    assert!(found.iter().any(|p| p.at.starts_with("Group 0") && p.message.contains("-1")));
    assert!(found.iter().any(|p| p.at == "General, Transition"), "{found:?}");
    let first_warning = found.iter().position(|p| p.severity != mg_module::doctor::Severity::Error);
    assert_eq!(first_warning, Some(errors), "errors first");
    assert_eq!(badge(&h), Some((found.len().to_string(), BadgeKind::Error)), "Check's own count");
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::Problems), "opened on Problems");
    let rows = h.get_by_label(mg_ui::problems::PROBLEM_ROWS);
    assert!(rows.query_by_label("Tile 1, Model").is_some(), "the finding's place, a link");
    assert!(rows.query_all_by_label_contains("zzt01.set").count() > 0, "under its file");
    // A second Check: replaced, not appended.
    tileset_tool(&mut h, tv::CHECK).click();
    settle(&mut h);
    assert_eq!(tileset_problems(&h, &path).len(), found.len(), "replaced");
    assert_eq!(h.state().problems.sets().len(), 1, "one source");
    // A change: of the text as it was; listed, not counted (the
    // coordinator's ruling at U20's review).
    h.state_mut().tilesets[0].change(|f| f.set("TILE1", "Model", "tcn01_a03_01"));
    settle(&mut h);
    let stale = h.state().problems.sets()[0].stale;
    assert_eq!(stale, Some(Stale::Changed), "said so");
    assert_eq!(tileset_problems(&h, &path).len(), found.len(), "kept until checked again");
    assert_eq!(badge(&h), None, "a stale set's findings not counted");
    let said: Vec<String> = h
        .get_by_label(mg_ui::problems::PROBLEM_ROWS)
        .query_all_by(|_| true)
        .filter_map(|n| n.accesskit_node().label().or(n.accesskit_node().value()))
        .collect();
    let words = "Changed since the check: not counted";
    assert!(said.iter().any(|l| l == words), "in the panel: {said:?}");
    // Undo back to the text checked: current again, counted again.
    tool(&h, "Undo").click();
    settle(&mut h);
    assert_eq!(h.state().problems.sets()[0].stale, None, "the text checked again");
    assert_eq!(badge(&h), Some((found.len().to_string(), BadgeKind::Error)), "counted again");
    tool(&h, "Redo").click();
    settle(&mut h);
    assert_eq!(h.state().problems.sets()[0].stale, Some(Stale::Changed));
    // Put right: Check again finds one error fewer, and is the file's again.
    tileset_tool(&mut h, tv::CHECK).click();
    settle(&mut h);
    assert_eq!(h.state().problems.sets()[0].stale, None);
    let now = tileset_problems(&h, &path);
    assert!(!now.iter().any(|p| p.message.contains("zzt01_a02_01")), "{now:?}");
    assert_eq!(badge(&h).unwrap().0, now.len().to_string());
    // Nothing found: nothing opens, the log and the panel say so.
    let clean = mg_testkit::scratch_dir("ui-u20-check-clean").join("zzc01.set");
    let text = SET
        .replace("Transition=0", "Transition=5")
        .replace("Tile0=-1", "Tile0=2")
        .replace("zzt01_a02_01", "tcn01_a03_01");
    std::fs::write(&clean, text).unwrap();
    tv::open_path(h.state_mut(), clean.clone());
    settle(&mut h);
    h.state_mut().settings.hide_log = true;
    settle(&mut h);
    let id = h.state().tilesets.iter().find(|d| d.path == clean).unwrap().id;
    assert_eq!(h.state().active_editor(), Some(&Tab::Tileset(id)));
    tileset_tool(&mut h, tv::CHECK).click();
    settle(&mut h);
    assert!(tileset_problems(&h, &clean).is_empty(), "{:?}", tileset_problems(&h, &clean));
    assert!(h.state().bottom_dock_rect().is_none(), "nothing found opens nothing");
    let said = h.state().log.entries.iter().any(|e| e.1 == "Check: zzc01.set: no problems found");
    assert!(said, "the log says so");
    // (The other file's findings stay: one source each.)
    assert_eq!(h.state().problems.sets().len(), 2);
}

/// A click on a finding's place goes to the tile (the plan's): its
/// tileset brought forward on Tiles, the tile chosen and its Model field
/// given the keyboard; the tileset closed, it is opened again there; a
/// group's finding goes to the Groups page and the group.
#[test]
fn a_click_in_problems_goes_to_the_tile() {
    let root = mg_testkit::corpus!();
    let (mut h, path, id) = tileset_window(Mode::Tabs, "ui-u20-go", 1600.0, Some(&root), false);
    tileset_tool(&mut h, tv::CHECK).click();
    settle(&mut h);
    // Away first: the General page, another tile.
    tileset_tool(&mut h, "General").click();
    settle(&mut h);
    h.state_mut().tilesets[0].tile = Some(2);
    settle(&mut h);
    h.get_by_label(mg_ui::problems::PROBLEM_ROWS).get_by_label("Tile 1, Model").click();
    settle(&mut h);
    let d = doc(&h);
    assert_eq!((d.page, d.tile), (Page::Tiles, Some(1)), "the tile");
    let focused = h.ctx.memory(|m| m.focused());
    assert_eq!(focused, Some(egui::Id::new(("set", id, "TILE1", "Model"))), "its field");
    assert!(field_with(&h, "zzt01_a02_01").rect().is_positive(), "in sight");
    // A group's finding.
    let group = tileset_problems(&h, &path).into_iter().find(|p| p.at.starts_with("Group 0"));
    h.get_by_label(mg_ui::problems::PROBLEM_ROWS).get_by_label(&group.unwrap().at).click();
    settle(&mut h);
    assert_eq!((doc(&h).page, doc(&h).group), (Page::Groups, Some(0)));
    // Closed: opened again on the tile.
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Tileset(id)]));
    settle(&mut h);
    assert!(h.state().tilesets.is_empty(), "closed");
    assert!(!tileset_problems(&h, &path).is_empty(), "its findings kept");
    h.get_by_label(mg_ui::problems::PROBLEM_ROWS).get_by_label("Tile 1, Model").click();
    settle(&mut h);
    let d = doc(&h);
    assert_eq!((d.path.as_path(), d.page, d.tile), (path.as_path(), Page::Tiles, Some(1)));
    assert_eq!(h.state().active_editor(), Some(&Tab::Tileset(d.id)), "in front");
    // Opened again so, its findings are the file's: a change makes them
    // stale.
    assert_eq!(h.state().problems.sets()[0].stale, None, "reopened: the file is the text checked");
    h.state_mut().tilesets[0].change(|f| f.set("TILE2", "PathNode", "Q"));
    settle(&mut h);
    assert_eq!(h.state().problems.sets()[0].stale, Some(Stale::Changed), "reopened");
    // Closed without saving the edit: the file on disk is what was checked,
    // so its findings are current again.
    let close = |h: &mut Harness<'_, Moonglow>| {
        let id = doc(h).id;
        h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Tileset(id)]));
        settle(h);
        h.get_by_label("Don't Save").click();
        settle(h);
        assert!(h.state().tilesets.is_empty(), "closed");
    };
    close(&mut h);
    assert_eq!(h.state().problems.sets()[0].stale, None, "the file is what was checked");
    // Checked with an edit not saved, then closed without saving: said so.
    h.get_by_label(mg_ui::problems::PROBLEM_ROWS).get_by_label("Tile 1, Model").click();
    settle(&mut h);
    h.state_mut().tilesets[0].change(|f| f.set("TILE2", "PathNode", "R"));
    settle(&mut h);
    tileset_tool(&mut h, tv::CHECK).click();
    settle(&mut h);
    close(&mut h);
    assert_eq!(h.state().problems.sets()[0].stale, Some(Stale::Closed));
    // Opened again: the file is not the text checked.
    tv::open_path(h.state_mut(), path.clone());
    settle(&mut h);
    assert_eq!(
        h.state().problems.sets()[0].stale,
        Some(Stale::Changed),
        "reopened on the old file"
    );
}

/// A source that is gone, and one let go of: a finding whose file is no
/// longer found marks its set "File not found" (listed, not counted); a
/// heading's right click offers Remove from Problems, and Delete on the
/// focused heading does it; closing the module keeps a tileset's findings
/// (its file is no module's).
#[test]
fn a_gone_source_is_said_and_a_set_is_removed() {
    let (mut h, path, id) = tileset_window(Mode::Tabs, "ui-u20-gone", 1600.0, None, true);
    let finding = mg_module::doctor::Finding {
        severity: mg_module::doctor::Severity::Error,
        check: "set-model".into(),
        source: "tileset".into(),
        resource: ResKey::parse("zzt01", ResType::SET).unwrap(),
        at: "[TILE1] Model".into(),
        message: "model zzt01_a02_01 isn't in the haks or the game".into(),
    };
    tv::record_check(h.state_mut(), id, std::slice::from_ref(&finding));
    settle(&mut h);
    // Closing the module keeps it.
    h.state_mut().actions.push(mg_ui::Action::Close);
    settle(&mut h);
    assert!(h.state().ws.is_none(), "the module closed");
    assert_eq!(h.state().problems.sets().len(), 1, "kept");
    // The file gone, the tileset closed: a click says so.
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Tileset(id)]));
    settle(&mut h);
    std::fs::remove_file(&path).unwrap();
    h.state_mut().actions.push(mg_ui::Action::ShowPanel(Panel::Problems));
    settle(&mut h);
    h.get_by_label(mg_ui::problems::PROBLEM_ROWS).get_by_label("Tile 1, Model").click();
    settle(&mut h);
    assert_eq!(h.state().problems.sets()[0].stale, Some(Stale::Missing));
    assert_eq!(h.state().switch_badge(Panel::Problems), None, "not counted");
    let rows = h.get_by_label(mg_ui::problems::PROBLEM_ROWS);
    assert!(rows.query_by_label_contains("File not found").is_some());
    // Removed by its heading's menu.
    // (The heading is named by its source alone, its counts the tab bar's:
    // M4's U28 review; the link of a finding has the same words.)
    let heading = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.label().as_deref() == Some("zzt01.set") && n.role() == egui::accesskit::Role::Unknown
    };
    rows.query_all_by(heading).next().expect("the heading").click_secondary();
    settle(&mut h);
    h.get_by_label(mg_ui::problems::REMOVE).click();
    settle(&mut h);
    assert!(h.state().problems.sets().is_empty(), "removed by its menu");
    // And by Delete on the heading focused.
    tv::open_path(h.state_mut(), set_file("ui-u20-gone-2"));
    settle(&mut h);
    let id = h.state().tilesets[0].id;
    tv::record_check(h.state_mut(), id, std::slice::from_ref(&finding));
    settle(&mut h);
    let rows = h.get_by_label(mg_ui::problems::PROBLEM_ROWS);
    rows.query_all_by(heading).next().expect("the heading").focus();
    settle(&mut h);
    h.key_press(egui::Key::Delete);
    settle(&mut h);
    assert!(h.state().problems.sets().is_empty(), "removed by Delete");
}

/// A tileset changed behind another tab (or read again from disk) is said
/// stale before its tab is drawn again (U20's review: followed only where
/// drawn, the badge counted a stale set).
#[test]
fn a_tileset_changed_behind_another_tab_is_said_stale() {
    let (mut h, _path, id) = tileset_window(Mode::Tabs, "ui-u20-behind", 1600.0, None, true);
    let finding = mg_module::doctor::Finding {
        severity: mg_module::doctor::Severity::Error,
        check: "set-model".into(),
        source: "tileset".into(),
        resource: ResKey::parse("zzt01", ResType::SET).unwrap(),
        at: "[TILE1] Model".into(),
        message: "model zzt01_a02_01 isn't in the haks or the game".into(),
    };
    tv::record_check(h.state_mut(), id, std::slice::from_ref(&finding));
    tv::open_path(h.state_mut(), set_file("ui-u20-behind-2"));
    settle(&mut h);
    let other = h.state().tilesets.iter().find(|d| d.id != id).expect("the second").id;
    assert_eq!(h.state().front_tab(), Some(Tab::Tileset(other)), "the second in front");
    assert_eq!(h.state().problems.sets()[0].stale, None);
    let d = h.state_mut().tilesets.iter_mut().find(|d| d.id == id).unwrap();
    d.change(|f| f.set("TILE1", "PathNode", "Q"));
    settle(&mut h);
    assert_eq!(h.state().front_tab(), Some(Tab::Tileset(other)), "still behind");
    assert_eq!(h.state().problems.sets()[0].stale, Some(Stale::Changed));
    assert_eq!(h.state().switch_badge(Panel::Problems), None, "not counted");
}

/// A floating tileset's Check fills the main window's Problems (Problems
/// is global, the main window's): the main Bottom Dock opens on it, and the
/// tileset's window has no bottom pane of its own for it.
#[test]
fn a_floating_tileset_s_check_opens_the_main_window_s_problems() {
    let root = mg_testkit::corpus!();
    let (mut h, path, id) =
        tileset_window(Mode::Floating, "ui-u20-check-floating", 1600.0, Some(&root), false);
    let at = h.state().dock.find_tab(&Tab::Tileset(id)).expect("its tab");
    assert!(!at.surface.is_main(), "floating");
    h.state_mut().settings.hide_log = true;
    settle(&mut h);
    tileset_tool(&mut h, tv::CHECK).click();
    settle(&mut h);
    assert!(!tileset_problems(&h, &path).is_empty());
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::Problems), "the main window's dock");
    assert_eq!(h.state().window_bottom_for_test(&Tab::Tileset(id)), None, "no pane of its own");
    let dock = h.state().bottom_dock_rect().expect("open");
    let link = h.get_by_label(mg_ui::problems::PROBLEM_ROWS).get_by_label("Tile 1, Model").rect();
    assert!(dock.contains_rect(link), "drawn in the main window's dock");
}

/// A frame that changes nothing walks the tileset never: the list's and
/// the Outliner's rows are made once for the file's revision (counted), the
/// pointer moving over the table and the Outliner, with Tiles open and the
/// list's keys; a change makes them once more.
#[test]
fn a_frame_that_changes_nothing_walks_no_tileset() {
    let (mut h, _, _) = tileset_window(Mode::Tabs, "ui-u20-walks", 1600.0, None, false);
    h.state_mut().tilesets[0].outline_open = [true; 6];
    settle(&mut h);
    click_pos(&h, tile_row(&h, 0).center());
    settle(&mut h);
    let walks = |h: &Harness<'_, Moonglow>| doc(h).walks();
    let before = walks(&h);
    for at in [tile_row(&h, 1).center(), outline_row(&h, "3 tiles").rect().center()] {
        for dx in 0..6 {
            h.hover_at(at + egui::vec2(dx as f32 * 3.0, 0.0));
            h.step();
        }
    }
    assert_eq!(walks(&h), before, "no walk in frames that change nothing");
    h.state_mut().tilesets[0].change(|f| f.set("TILE2", "PathNode", "Q"));
    settle(&mut h);
    let after = walks(&h);
    assert!(after > before && after <= before + 2, "once each after a change: {before} {after}");
}

/// Problems with nothing in it (the kit's empty state, every board's
/// switch): its switch is there with no tileset open, no count; the panel
/// says nothing was found and what fills it.
#[test]
fn problems_with_nothing_in_it_says_what_fills_it() {
    let mut h = harness(app_in(Mode::Tabs, Vec::new()), 1600.0, 900.0);
    settle(&mut h);
    let bar = h.get_by_label("Status Bar");
    assert!(bar.query_by_label("Problems").is_some(), "its switch");
    assert_eq!(h.state().switch_badge(Panel::Problems), None);
    bar.get_by_label("Problems").click();
    settle(&mut h);
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::Problems));
    let dock = h.state().bottom_dock_rect().expect("open");
    let said = h.get_by_label(mg_ui::problems::NOTHING).rect();
    assert!(dock.contains_rect(said), "{said:?} in {dock:?}");
    assert!(h.query_by_label(mg_ui::problems::FILLED_BY).is_some());
    // (M4's U28: Build Module and Verify Module fill it too, and say so.)
    assert!(h.query_by_label_contains("Build Module").is_some(), "Build's named, drawn");
    assert!(h.query_by_label_contains("Verify Module").is_some(), "Verify's named, drawn");
}

/// The Outliner lists the tileset's parts with their counts (the plan's):
/// General, 2 terrains, 1 crosser, 1 primary rule, no secondary rules (a
/// quiet row), 3 tiles, 1 group; Terrains and Crossers open at first; the
/// counts follow a tile added. A click on a tile goes to the Tiles page and
/// the tile; on a group's row to its page; the keys go down and Enter goes
/// there; letters go to a row by its words; a group opened by its arrow
/// lists its parts.
#[test]
fn the_outliner_counts_the_tileset_s_parts() {
    let (mut h, _, _) = tileset_window(Mode::Tabs, "ui-u20-outliner", 1600.0, None, false);
    for name in [
        "General settings",
        "2 terrains",
        "1 crosser",
        "1 primary rule",
        "0 secondary rules",
        "3 tiles",
        "1 group",
    ] {
        assert!(h.get_by_label(mg_ui::OUTLINER).query_by_label(name).is_some(), "{name}");
    }
    assert!(outline_row(&h, "Terrain 1: Water").rect().is_positive(), "Terrains open");
    assert!(h.get_by_label(mg_ui::OUTLINER).query_by_label("Tile 2: tcn01_a02_01").is_none());
    // A tile added: counted.
    tileset_tool(&mut h, "Tiles").click();
    settle(&mut h);
    h.get_by_label(tv::ADD_TILE).click();
    settle(&mut h);
    assert!(h.get_by_label(mg_ui::OUTLINER).query_by_label("4 tiles").is_some(), "counted");
    // Tiles opened by its arrow, a tile clicked.
    tileset_tool(&mut h, "General").click();
    settle(&mut h);
    outline_row(&h, "Open Tiles").click();
    settle(&mut h);
    assert!(doc(&h).outline_open[4], "Tiles opened by its arrow");
    outline_row(&h, "Tile 1: zzt01_a02_01").click();
    settle(&mut h);
    assert_eq!((doc(&h).page, doc(&h).tile), (Page::Tiles, Some(1)), "the page and the tile");
    // The keys: Down and Enter.
    h.key_press(egui::Key::ArrowDown);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(doc(&h).tile, Some(2), "the next tile, by the keys");
    // Letters: to the Groups row by its words, Enter to its page.
    h.key_press(egui::Key::Home);
    settle(&mut h);
    h.event(egui::Event::Text("gro".into()));
    settle(&mut h);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(doc(&h).page, Page::Groups, "by its words");
}

/// The Outliner lays out its rows in sight alone: a tileset of a
/// thousand tiles with Tiles open draws a few dozen rows.
#[test]
fn the_outliner_lays_out_its_rows_in_sight_alone() {
    let path = mg_testkit::scratch_dir("ui-u20-many").join("zzm01.set");
    let mut text = SET.replace("[TILES]\r\nCount=3", "[TILES]\r\nCount=1003");
    for i in 3..1003 {
        text.push_str(&format!("\r\n[TILE{i}]\r\nModel=zzm01_a{i:04}\r\nTopLeft=Grass\r\n"));
    }
    std::fs::write(&path, text).unwrap();
    let mut h = harness(app_in(Mode::Tabs, Vec::new()), 1600.0, 900.0);
    settle(&mut h);
    tv::open_path(h.state_mut(), path);
    settle(&mut h);
    h.state_mut().tilesets[0].outline_open = [true; 6];
    settle(&mut h);
    let drawn = h.get_by_label(mg_ui::OUTLINER).query_all_by_label_contains("Tile ").count();
    assert!(drawn > 5 && drawn < 60, "{drawn} tiles' rows laid out");
    // (And the table's: its rows in sight alone too.)
    h.state_mut().tilesets[0].page = Page::Tiles;
    settle(&mut h);
    let rows = h.get_by_label(tv::TILES_TABLE).query_all_by_label_contains("Tile ").count();
    assert!(rows > 5 && rows < 60, "{rows} of the table's rows laid out");
}

/// The tile table's keys are the active editor's: a click on a row gives
/// them to the list, Down and End choose the next and the last tile; a
/// press in another tileset's group (its page, on no control) makes it the
/// active editor, and the first list's keys do nothing then, though it is
/// still drawn beside it.
#[test]
fn the_tile_table_s_keys_are_the_active_editor_s() {
    let (mut h, path, id) = tileset_window(Mode::Tabs, "ui-u20-keys", 2400.0, None, false);
    click_pos(&h, tile_row(&h, 0).center());
    settle(&mut h);
    let first = |h: &Harness<'_, Moonglow>| {
        h.state().tilesets.iter().find(|d| d.path == path).unwrap().tile
    };
    assert_eq!(first(&h), Some(0));
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(first(&h), Some(1), "Down");
    h.key_press(egui::Key::End);
    settle(&mut h);
    assert_eq!(first(&h), Some(2), "End");
    h.key_press(egui::Key::Home);
    settle(&mut h);
    // Another tileset in a group of its own at the right, pressed in.
    let path2 = mg_testkit::scratch_dir("ui-u20-keys-2").join("zzk01.set");
    std::fs::write(&path2, SET).unwrap();
    tv::open_path(h.state_mut(), path2.clone());
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::SplitEditor);
    settle(&mut h);
    let second = h.state().tilesets.iter().find(|d| d.path == path2).unwrap().id;
    h.state_mut().tilesets.iter_mut().find(|d| d.id == second).unwrap().page = Page::General;
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Tileset(id)));
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Tileset(second)));
    settle(&mut h);
    let body = {
        let dock = &h.state().dock;
        let at = dock.find_tab(&Tab::Tileset(second)).expect("open");
        dock.leaf(egui_dock::NodePath { surface: at.surface, node: at.node }).unwrap().rect
    };
    assert!(h.state().tab_rect(&Tab::Tileset(id)).is_some(), "the first drawn beside it");
    click_pos(&h, egui::pos2(body.right() - 40.0, body.bottom() - 40.0));
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::Tileset(second)), "the second active");
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(first(&h), Some(0), "the first list's keys do nothing");
}

/// Down in Find goes on to the tile table, at its first tile (the table
/// has the keys, outlined); Down and End go on from there in the table's
/// order, Enter does nothing (choosing a tile already shows it in the
/// form).
#[test]
fn down_in_find_goes_to_the_tile_table_s_first_tile() {
    let (mut h, _, _) = tileset_window(Mode::Tabs, "ui-u20-find-down", 1600.0, None, false);
    // (Find the models with "a02": tiles 1 and 2, the first of them not
    // tile 0.)
    h.state_mut().tilesets[0].tile = Some(2);
    settle(&mut h);
    let find =
        |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("Model or terrain");
    h.get(egui_kittest::kittest::by().predicate(find)).click();
    h.get(egui_kittest::kittest::by().predicate(find)).type_text("a02");
    settle(&mut h);
    assert_eq!(accent_outlines(&h, 100.0), 0, "no keys yet");
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(doc(&h).tile, Some(1), "the first tile found");
    assert_eq!(accent_outlines(&h, 100.0), 1, "the table outlined");
    says(&mut h, &["Up or Down: Move in the list"], &[]);
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(doc(&h).tile, Some(2), "Down goes on in the table");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(doc(&h).tile, Some(2), "Enter changes nothing");
    h.key_press(egui::Key::Home);
    settle(&mut h);
    assert_eq!(doc(&h).tile, Some(1), "Home");
    h.key_press(egui::Key::End);
    settle(&mut h);
    assert_eq!(doc(&h).tile, Some(2), "End");
}

/// Typed text keeps to its tile, typed after the move (the reviews'
/// rule): the last tile's Model typed in, then Remove Last Tile pressed:
/// the text is set on that tile as the press leaves the field, the tile
/// then taken away, each one Undo step (an Undo brings the tile back with
/// the text); the first tile's Walkmesh typed in, Add Tile pressed: set on
/// the first tile, the new tile untouched; typed in and the Top Bar's Undo
/// pressed: set, then undone, one step each.
#[test]
fn typed_text_keeps_to_its_tile() {
    let (mut h, _, _) = tileset_window(Mode::Tabs, "ui-u20-typed", 1600.0, None, false);
    h.state_mut().tilesets[0].tile = Some(2);
    settle(&mut h);
    field_with(&h, "tcn01_a02_01").click();
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    field_with(&h, "tcn01_a02_01").type_text("zzt01_last");
    settle(&mut h);
    h.get_by_label(tv::REMOVE_LAST_TILE).click();
    settle(&mut h);
    assert_eq!(set(&h).tiles.len(), 2, "taken away");
    tool(&h, "Undo").click();
    settle(&mut h);
    assert_eq!(set(&h).tiles.len(), 3, "back");
    assert_eq!(set(&h).tiles[2].model, "zzt01_last", "with what was typed");
    tool(&h, "Undo").click();
    settle(&mut h);
    assert_eq!(set(&h).tiles[2].model, "tcn01_a02_01", "the typing its own step");
    // The first tile's Walkmesh, then Add Tile.
    h.state_mut().tilesets[0].tile = Some(0);
    settle(&mut h);
    field_with(&h, "msb01").click();
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    field_with(&h, "msb01").type_text("msz09");
    settle(&mut h);
    h.get_by_label(tv::ADD_TILE).click();
    settle(&mut h);
    let t = set(&h);
    assert_eq!((t.tiles.len(), t.tiles[0].walkmesh.as_deref()), (4, Some("msz09")));
    assert_ne!(t.tiles[3].walkmesh.as_deref(), Some("msz09"), "the new tile untouched");
    assert_eq!(doc(&h).tile, Some(3), "the new tile chosen");
}

/// Each edit is one Undo step through the Top Bar with the pointer over a
/// docked panel (Preview), on the right tileset of two in two groups; a
/// door added in the form, then the Top Bar's Undo with the pointer over
/// the Preview, takes the door away from that tileset alone.
#[test]
fn an_edit_is_one_undo_step_on_its_own_tileset() {
    // (Wide enough for each group to keep its list and form side by side.)
    let (mut h, path, id) = tileset_window(Mode::Tabs, "ui-u20-undo", 2400.0, None, false);
    let other = mg_testkit::scratch_dir("ui-u20-undo-2").join("zzo01.set");
    std::fs::write(&other, SET).unwrap();
    tv::open_path(h.state_mut(), other.clone());
    settle(&mut h);
    // (The second in a group of its own, at the first's right.)
    h.state_mut().actions.push(mg_ui::Action::SplitEditor);
    settle(&mut h);
    let first = h.state().tilesets.iter().position(|d| d.path == path).unwrap();
    h.state_mut().tilesets[first].tile = Some(1);
    h.state_mut().tilesets[first].page = Page::Tiles;
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Tileset(id)));
    settle(&mut h);
    let body = {
        let dock = &h.state().dock;
        let at = dock.find_tab(&Tab::Tileset(id)).expect("open");
        dock.leaf(egui_dock::NodePath { surface: at.surface, node: at.node }).unwrap().rect
    };
    let add = h
        .get_all_by_label("Add Door")
        .find(|n| body.contains_rect(n.rect()))
        .unwrap_or_else(|| panic!("the first tileset's Add Door in {body:?}"));
    add.click();
    settle(&mut h);
    let doors = |h: &Harness<'_, Moonglow>, i: usize| {
        h.state().tilesets[i].tileset().unwrap().tiles[1].doors.len()
    };
    let second = 1 - first;
    assert_eq!((doors(&h, first), doors(&h, second)), (1, 0), "on its own tileset");
    // (A press elsewhere first, the Status Bar: what the Top Bar's Undo
    // takes is then what the pointer was over before it, the Preview.)
    let screen = h.ctx.content_rect();
    click_pos(&h, egui::pos2(screen.center().x, screen.bottom() - 10.0));
    settle(&mut h);
    let preview = right_dock_rect(&h);
    h.hover_at(preview.center());
    settle(&mut h);
    tool(&h, "Undo").click();
    settle(&mut h);
    assert_eq!((doors(&h, first), doors(&h, second)), (0, 0), "one step, its own");
}

/// The header's controls by their names: the switch, Save, Make Palette,
/// Check, Render Minimap Pictures.
const TILESET_CONTROLS: [&str; 5] =
    [tv::PAGE, "Save", tv::MAKE_PALETTE, tv::CHECK, tv::RENDER_MINIMAPS];

/// The header holds every control from 420 to 1600 (`header_holds`),
/// docked and floating: on the row or in More, the switch and Save the
/// last to go; OK and Cancel in the end slot at every width; a page chosen
/// from More's switch at the least width.
#[test]
fn the_tileset_header_holds_every_control_from_420_to_1600() {
    use mg_uikit::header::{self, Place};
    for mode in MODES {
        let (mut h, _, _) = tileset_window(mode, "ui-u20-header", 1600.0, None, false);
        let widths = [1600.0, 1400.0, 1399.0, 1200.0, 1000.0, 962.0, 961.0, 800.0, 600.0, 420.0];
        for width in widths {
            h.set_size(egui::vec2(width, 800.0));
            settle(&mut h);
            let what = format!("{mode:?} {width}");
            let rec = header::records(&h.ctx, TILESET_HEADER);
            let rec = rec.into_iter().next().unwrap_or_else(|| panic!("{what}: no header"));
            let names: Vec<&str> = rec.places.iter().map(|(n, _)| n.as_str()).collect();
            assert_eq!(names, TILESET_CONTROLS, "{what}");
            let place = |n: &str| rec.places.iter().find(|(m, _)| m == n).map(|(_, p)| *p);
            if width >= 1400.0 && mode == Mode::Tabs {
                for n in TILESET_CONTROLS {
                    assert!(matches!(place(n), Some(Place::Row | Place::Icon)), "{what}: {n}");
                }
            }
            let end = rec.end.expect("the end slot");
            for n in ["OK", "Cancel"] {
                let r = h.get_by_label(TILESET_HEADER).get_by_label(n).rect();
                assert!(end.expand(0.5).contains_rect(r), "{what}: {n} in {end:?}");
            }
            header_holds(&mut h, TILESET_HEADER, &TILESET_CONTROLS);
        }
        // At the least width, a page chosen from wherever the switch is.
        tileset_tool(&mut h, "Groups").click();
        settle(&mut h);
        assert_eq!(doc(&h).page, Page::Groups, "{mode:?}: chosen at 420");
        h.key_press(egui::Key::Escape);
        settle(&mut h);
    }
}

/// The hints and counts as bound (the Tileset board's): on Tiles, Click
/// Choose a tile, and with the pointer over the page Ctrl + F Find (remapped,
/// as remapped); with the pointer away from the page, no Find; on the other
/// pages neither Click nor Find; none while a field is typed in; never a
/// Save hint (Ctrl+S is the module's Save, the coordinator's and the owner's
/// ruling at U20's review); "3 tiles, 1 group, 2 terrains, 1 crosser".
/// Ctrl+F does what it says: Find's own field takes the keyboard. Ctrl+S with a
/// module open saves the tileset with the module; with none it does nothing,
/// as beside a 2DA opened by Tools › 2DA File….
#[test]
fn the_tileset_hints_are_as_bound() {
    let said = |h: &Harness<'_, Moonglow>| h.state().status_said();
    let none_of = |s: &[String], words: &[&str], what: &str| {
        for w in words {
            assert!(!s.iter().any(|l| l.contains(w)), "{what}: no {w}: {s:?}");
        }
    };
    let (mut h, path, _) = tileset_window(Mode::Tabs, "ui-u20-hints", 1600.0, None, true);
    let page = tile_row(&h, 1);
    h.hover_at(page.center());
    settle(&mut h);
    let s = said(&h);
    assert!(s.contains(&"Click: Choose a tile".to_string()), "{s:?}");
    assert!(s.contains(&"Ctrl, F: Find".to_string()), "{s:?}");
    assert!(s.contains(&"3 tiles, 1 group, 2 terrains, 1 crosser".to_string()), "{s:?}");
    none_of(&s, &["Save"], "over the page");
    // The pointer away from the page (over the Right Dock): no Find.
    h.hover_at(right_dock_rect(&h).center());
    settle(&mut h);
    let s = said(&h);
    assert!(s.contains(&"Click: Choose a tile".to_string()), "{s:?}");
    none_of(&s, &["Find", "Save"], "away from the page");
    // Ctrl + F: Find's own field takes the keyboard.
    h.hover_at(page.center());
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::F);
    settle(&mut h);
    let field = egui::Id::new(("tileset-filter-field", doc(&h).id));
    assert_eq!(h.ctx.memory(|m| m.focused()), Some(field), "Find's field has the keyboard");
    // Typing: none.
    none_of(&said(&h), &["Choose a tile", "Find", "Save"], "typing");
    h.key_press(egui::Key::Escape);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    // Ctrl + S saves it with the module.
    h.state_mut().tilesets[0].change(|f| f.set("TILE0", "PathNode", "B"));
    settle(&mut h);
    h.hover_at(page.center());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    settle(&mut h);
    assert!(!doc(&h).is_dirty(), "saved");
    assert!(std::fs::read_to_string(&path).unwrap().contains("PathNode=B"));
    // Another page: no Click, no Find.
    tileset_tool(&mut h, "General").click();
    settle(&mut h);
    h.hover_at(page.center());
    settle(&mut h);
    none_of(&said(&h), &["Choose a tile", "Find", "Save"], "General");
    // Find remapped: shown so.
    let ctrl_g = egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, egui::Key::G);
    h.state_mut().keymap.set(mg_ui::keys::Cmd::Find, vec![ctrl_g]);
    tileset_tool(&mut h, "Tiles").click();
    settle(&mut h);
    h.hover_at(page.center());
    settle(&mut h);
    let s = said(&h);
    assert!(s.contains(&"Ctrl, G: Find".to_string()), "{s:?}");
    none_of(&s, &["Ctrl, F"], "remapped");
    // Without a module: no Save hint, and Ctrl+S does nothing.
    let (mut h, path, _) = tileset_window(Mode::Tabs, "ui-u20-hints-alone", 1600.0, None, false);
    let page = tile_row(&h, 1);
    h.hover_at(page.center());
    settle(&mut h);
    let s = said(&h);
    assert!(s.contains(&"Click: Choose a tile".to_string()), "{s:?}");
    none_of(&s, &["Save"], "no module");
    h.state_mut().tilesets[0].change(|f| f.set("TILE0", "PathNode", "C"));
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    settle(&mut h);
    assert!(doc(&h).is_dirty(), "no module: Ctrl+S saves nothing");
    assert!(!std::fs::read_to_string(&path).unwrap().contains("PathNode=C"));
}

/// The window comes to rest (`settle`) with the tileset's panels in front,
/// behind and folded, the docks closed, docked and floating: on each page,
/// typing in the form, moving in the table, a Check run, a click in
/// Problems, an Undo, and Remove Last Tile with the last tile chosen (the
/// real path: its button); the last check that the removal changed the
/// file.
#[test]
fn the_window_comes_to_rest_with_the_tileset_s_panels() {
    let cases: [(Mode, &str, f32); 7] = [
        (Mode::Tabs, "stacked", 1600.0),
        (Mode::Tabs, "Textures", 1300.0),
        (Mode::Tabs, "Preview", 1300.0),
        (Mode::Tabs, "folded", 1600.0),
        (Mode::Tabs, "closed", 1600.0),
        (Mode::Floating, "pane", 1600.0),
        (Mode::Floating, "small", 1280.0),
    ];
    let root = mg_testkit::nwn_root();
    for (mode, case, width) in cases {
        let (mut h, path, _) = tileset_window(mode, "ui-u20-rest", width, root.as_deref(), false);
        let what = format!("{mode:?} {case}");
        match case {
            "Textures" | "Preview" => {
                let tab = h
                    .get_all_by_label(case)
                    .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Tab);
                tab.unwrap_or_else(|| panic!("{what}: its tab")).click();
                settle(&mut h);
            }
            "folded" => {
                h.state_mut().settings.hide_palettes = true;
                settle(&mut h);
            }
            "closed" => {
                h.state_mut().settings.hide_palettes = true;
                h.state_mut().settings.hide_tree = true;
                settle(&mut h);
            }
            _ => {}
        }
        for name in ["General", "Terrains and Crossers", "Painting Rules", "Groups", "Tiles"] {
            tileset_tool(&mut h, name).click();
            settle(&mut h);
        }
        click_pos(&h, tile_row(&h, 0).center());
        settle(&mut h);
        h.key_press(egui::Key::ArrowDown);
        settle(&mut h);
        field_with(&h, "msb01").click();
        settle(&mut h);
        field_with(&h, "msb01").type_text("x");
        settle(&mut h);
        h.key_press(egui::Key::Enter);
        settle(&mut h);
        if root.is_some() {
            tileset_tool(&mut h, tv::CHECK).click();
            settle(&mut h);
            h.state_mut().tilesets[0].tile = Some(2);
            settle(&mut h);
            // (By its action: a floating tileset's window may stand over
            // the main window's Bottom Dock.)
            let rows = h.get_by_label(mg_ui::problems::PROBLEM_ROWS);
            rows.get_by_label("Tile 1, Model").click_accesskit();
            settle(&mut h);
            assert_eq!(doc(&h).tile, Some(1), "{what}: the click in Problems went there");
        }
        tool(&h, "Undo").click();
        settle(&mut h);
        h.state_mut().tilesets[0].tile = Some(2);
        settle(&mut h);
        let before = std::sync::Arc::new(doc(&h).file.text());
        h.get_by_label(tv::REMOVE_LAST_TILE).click();
        settle(&mut h);
        assert_ne!(*before, doc(&h).file.text(), "{what}: the removal changed the file");
        assert_eq!(set(&h).tiles.len(), 2, "{what}: two tiles left");
        let _ = path;
    }
}

/// A floating tileset opens at its first size within the main window (a
/// screen of 1280 by 720 and of 1600 by 900): as wide as it asks (its list
/// and form, its right pane and its Outliner's) where the screen has it;
/// the header's controls held; the Preview's diagram and the Outliner's
/// counts in its panes, or the Outliner's strip.
#[test]
fn a_floating_tileset_opens_at_its_first_size() {
    for (width, height) in [(1280.0, 720.0), (1600.0, 900.0)] {
        let what = format!("{width} by {height}");
        let path = set_file(&format!("ui-u20-first-{width}"));
        let mut h = harness(app_in(Mode::Floating, Vec::new()), width, height);
        settle(&mut h);
        tv::open_path(h.state_mut(), path);
        settle(&mut h);
        let id = h.state().tilesets[0].id;
        h.state_mut().tilesets[0].page = Page::Tiles;
        h.state_mut().tilesets[0].tile = Some(0);
        settle(&mut h);
        let at = h.state().dock.find_tab(&Tab::Tileset(id)).expect("its tab");
        assert!(!at.surface.is_main(), "{what}: a window of its own");
        let path = egui_dock::NodePath { surface: at.surface, node: at.node };
        let pane = h.state().dock.leaf(path).unwrap().rect;
        let want = (920.0_f32 + 281.0 + 261.0).min(width - 48.0);
        assert!(pane.width() >= want - 12.0, "{what}: {} wide, wants {want}", pane.width());
        let screen = h.ctx.content_rect();
        assert!(screen.contains_rect(pane), "{what}: within the main window: {pane:?}");
        header_holds(&mut h, TILESET_HEADER, &TILESET_CONTROLS);
        let diagram = h.get_by_label_contains(tv::DIAGRAM_NAME).rect();
        assert!(pane.contains_rect(diagram), "{what}: the diagram in its pane");
        // (The Outliner a pane where the window has the room beside the
        // page's 918, else its strip; the list and the form side by side
        // in both.)
        let pane_drawn = h.query_by_label("3 tiles").is_some();
        let strip = h.query_by_label(mg_ui::dialog_view::OUTLINER_STRIP).is_some();
        assert_eq!((pane_drawn, strip), (width >= 1600.0, width < 1600.0), "{what}: Outliner");
        let row = tile_row(&h, 0);
        let field = field_with(&h, "tcn01_a01_01").rect();
        assert!(field.left() > row.right() - 1.0, "{what}: the form beside the list");
    }
}

/// Records from before read without a fault, as the tip (3229ef5c) wrote
/// them: the module's open-tabs record names the tileset by its path, and
/// its view its page by the page's words ("Painting Rules"), its tile and
/// group; the settings keep the tileset window's size (`window_sizes`'
/// "tileset", 920 by 720, as a builder left it). Opened again: on that
/// page, that tile and group, its Preview drawn, the header's controls
/// held.
#[test]
fn records_of_a_tileset_read_without_error() {
    let text =
        "(window_sizes:[(\"tileset\",(920,720))],window_sizes_scaled:true,name_resrefs:true)";
    let (settings, _unread) = mg_ui::Settings::read_fields(text).expect("read");
    assert!(settings.name_resrefs, "the rest read");
    assert_eq!(settings.window_sizes, [("tileset".to_string(), [920, 720])], "its size read");
    for mode in MODES {
        let set = set_file(&mode.dir("ui-u20-record-set"));
        let dir = mg_testkit::scratch_dir(&mode.dir("ui-u20-record"));
        let path = sample_module(&dir);
        let tabs = dir.join("open-tabs");
        std::fs::create_dir_all(&tabs).unwrap();
        let name = serde_json::to_string(&format!("tileset:{}", set.display())).unwrap();
        let record = match mode {
            Mode::Floating => format!(
                r#"{{"panes":[{{"place":"middle","tabs":["module-properties"],"active":"module-properties"}},
                {{"place":"window","tabs":[{name}],"active":{name},"rect":[300.0,60.0,920.0,720.0]}}],
                "front":{name},"views":{{{name}:{{"page":"Painting Rules","tile":2,"group":0}}}},
                "rects_scaled":true}}"#
            ),
            Mode::Tabs => format!(
                r#"{{"panes":[{{"place":"middle","tabs":["module-properties",{name}],"active":{name}}}],
                "front":{name},"views":{{{name}:{{"page":"Painting Rules","tile":2,"group":0}}}},
                "rects_scaled":true}}"#
            ),
        };
        std::fs::write(mg_ui::open_tabs::file(&tabs, &path), record).unwrap();
        let mut app = app_in(mode, Vec::new());
        app.settings.window_sizes = settings.window_sizes.clone();
        app.settings.window_sizes_scaled = settings.window_sizes_scaled;
        app.settings.no_last_area = true;
        app.open_tabs_dir = Some(tabs.clone());
        app.open_module(&path);
        let mut h = harness(app, 1600.0, 900.0);
        settle(&mut h);
        let what = format!("{mode:?}");
        let d = h.state().tilesets.first().unwrap_or_else(|| panic!("{what}: opened"));
        assert_eq!((d.page, d.tile, d.group), (Page::Rules, Some(2), Some(0)), "{what}");
        let at = h.state().dock.find_tab(&Tab::Tileset(d.id)).expect("its tab");
        assert_eq!(at.surface.is_main(), mode == Mode::Tabs, "{what}: where it was");
        header_holds(&mut h, TILESET_HEADER, &TILESET_CONTROLS);
        assert!(h.query_by_label_contains(tv::DIAGRAM_NAME).is_some(), "{what}: Preview");
    }
}

/// The Preview (the board's): the chosen tile's diagram, its corners'
/// terrains and heights and its edges' crossers, its model, orientation
/// and doors in the middle; the door sentence under it; Open the Model in
/// a Tab opening the model's tab; a tile whose model the game has not:
/// said in the middle, the button off with its tip. Textures lists the
/// model's textures, or says why not.
#[test]
fn the_preview_draws_the_tile_and_its_doors() {
    let root = mg_testkit::corpus!();
    let (mut h, _, _) = tileset_window(Mode::Tabs, "ui-u20-preview", 1600.0, Some(&root), false);
    h.state_mut().tilesets[0].tile = Some(0);
    settle(&mut h);
    let dock = right_dock_rect(&h);
    let diagram = h.get_by_label_contains(tv::DIAGRAM_NAME);
    assert!(dock.contains_rect(diagram.rect()), "in the Right Dock");
    let sentence = "Door 1: doortypes.2da row 0, at (1.50, -2.00, 0.00), turned 90°";
    assert!(h.query_by_label(sentence).is_some(), "the door sentence");
    assert!(h.query_by_label_contains("Preview, Tile 0, tcn01_a01_01").is_some(), "its header");
    // Open the Model in a Tab.
    let open = h.get_by_label(tv::OPEN_MODEL);
    assert!(!open.accesskit_node().is_disabled());
    open.click();
    settle(&mut h);
    let key = ResKey::parse("tcn01_a01_01", ResType::MDL).unwrap();
    assert!(h.state().dock.find_tab(&Tab::Model(key)).is_some(), "the model's tab");
    // Textures: the model's.
    // (Copy on the chosen row alone, the board's: none before a choice,
    // one after.)
    let copies = |h: &Harness<'_, Moonglow>| {
        h.get_by_label(tv::TEXTURE_ROWS).query_all_by_label_contains("Copy").count()
    };
    assert_eq!(copies(&h), 0, "no row chosen");
    let first = h
        .get_by_label(tv::TEXTURE_ROWS)
        .query_all_by(|n| n.role() == egui::accesskit::Role::Label)
        .next()
        .expect("a texture's row")
        .rect();
    click_pos(&h, first.center());
    settle(&mut h);
    assert_eq!(copies(&h), 1, "the chosen row's Copy");
    // A model nowhere.
    let id = doc(&h).id;
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Tileset(id)));
    h.state_mut().tilesets[0].tile = Some(1);
    settle(&mut h);
    assert!(h.get_by_label(tv::OPEN_MODEL).accesskit_node().is_disabled(), "off");
    assert!(h.query_by_label("No doors: a door can't be placed on this tile.").is_some());
    assert!(h.query_by_label_contains("isn't in the game data").is_some(), "Textures says why");
}

/// The tile's form is never taller where wider (each width against every
/// narrower one, the reviews' sweep) at three text sizes, the editor's
/// room swept from its least to the board's; and comes to rest at each.
#[test]
fn the_tile_form_is_never_taller_where_wider() {
    let (mut h, _, _) = tileset_window(Mode::Tabs, "ui-u20-taller", 1600.0, None, false);
    h.state_mut().tilesets[0].tile = Some(0);
    h.state_mut().settings.hide_palettes = true;
    h.state_mut().settings.hide_tree = true;
    settle(&mut h);
    let add = |h: &Harness<'_, Moonglow>| h.get_by_label("Add Door").rect().bottom();
    for scale in [1.0_f32, 1.25, 1.5] {
        h.ctx.set_zoom_factor(scale);
        // (By the form's own width, the Model field's: the page stands the
        // form beside the list or under it by the editor's width, and the
        // form under the list is the wider.)
        let mut seen: Vec<(f32, f32, usize)> = Vec::new();
        for width in (720..=1600).step_by(64).chain(std::iter::once(1600)) {
            h.set_size(egui::vec2(width as f32, 1400.0));
            settle(&mut h);
            let field = field_with(&h, "tcn01_a01_01").rect();
            let high = add(&h) - field.top();
            for (w, was, at) in &seen {
                let narrower = *w < field.width() - 0.5;
                assert!(
                    !narrower || high <= was + 0.5,
                    "{scale}: {high} at {width} ({}) over {was} at {at} ({w})",
                    field.width()
                );
            }
            seen.push((field.width(), high, width));
        }
        h.ctx.set_zoom_factor(1.0);
    }
}

/// F6 goes through the tileset's panes, docked: the Outliner's parts
/// (their rows the keys'), the tile list (Down then chooses the next
/// tile), Textures (the first Copy given the keyboard), Problems (its
/// first link given the keyboard, Check having found something).
#[test]
fn f6_reaches_the_tileset_s_panes() {
    let root = mg_testkit::corpus!();
    let (mut h, _, _) = tileset_window(Mode::Tabs, "ui-u20-f6", 1600.0, Some(&root), false);
    h.state_mut().tilesets[0].tile = Some(0);
    settle(&mut h);
    tileset_tool(&mut h, tv::CHECK).click();
    settle(&mut h);
    let mut seen = Vec::new();
    for _ in 0..8 {
        h.key_press(egui::Key::F6);
        settle(&mut h);
        let keys = mg_ui::palette_view::arrow_keys_are(&h.ctx);
        if keys == "Problems" {
            let link = h.get_by_label(mg_ui::problems::PROBLEM_ROWS).get_by_label("Tile 1, Model");
            assert!(link.accesskit_node().is_focused(), "Problems' first link focused");
        }
        seen.push(keys);
    }
    for k in ["Marks", "Tiles", "Textures", "Problems"] {
        assert!(seen.iter().any(|s| s == k), "{k}: {seen:?}");
    }
    while mg_ui::palette_view::arrow_keys_are(&h.ctx) != "Tiles" {
        h.key_press(egui::Key::F6);
        settle(&mut h);
    }
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(doc(&h).tile, Some(1), "the list's Down after F6");
}

/// A floating tileset's edit is one Undo step too, the Top Bar's Undo
/// pressed with the pointer over its window's right pane after a press
/// elsewhere (the Status Bar): the pane says whose Undo it is.
#[test]
fn a_floating_tileset_s_edit_is_one_undo_step() {
    let (mut h, _, id) =
        tileset_window(Mode::Floating, "ui-u20-undo-floating", 1600.0, None, false);
    h.state_mut().tilesets[0].tile = Some(1);
    settle(&mut h);
    h.get_by_label("Add Door").click();
    settle(&mut h);
    let doors = |h: &Harness<'_, Moonglow>| set(h).tiles[1].doors.len();
    assert_eq!(doors(&h), 1);
    let screen = h.ctx.content_rect();
    click_pos(&h, egui::pos2(screen.center().x, screen.bottom() - 10.0));
    settle(&mut h);
    let diagram = h.get_by_label_contains(tv::DIAGRAM_NAME).rect();
    let at = h.state().dock.find_tab(&Tab::Tileset(id)).unwrap();
    assert!(!at.surface.is_main(), "floating");
    h.hover_at(diagram.center());
    settle(&mut h);
    tool(&h, "Undo").click();
    settle(&mut h);
    assert_eq!(doors(&h), 0, "the tileset's step undone");
}

/// Points the fixture's tile 1 at no model (a section with no `Model=`).
fn hole_set(name: &str) -> std::path::PathBuf {
    let path = mg_testkit::scratch_dir(name).join("zzt01.set");
    std::fs::write(&path, SET.replace("[TILE1]\r\nModel=zzt01_a02_01\r\n", "[TILE1]\r\n")).unwrap();
    path
}

/// A tile with no model (a hole): Preview's header says its number alone,
/// Textures' nothing after its title (no "Tile 1, " nor "of "); the
/// diagram's middle says "(no model)".
#[test]
fn a_hole_s_headers_say_no_model() {
    let mut h = harness(app_in(Mode::Tabs, Vec::new()), 1600.0, 900.0);
    settle(&mut h);
    tv::open_path(h.state_mut(), hole_set("ui-u20-hole"));
    settle(&mut h);
    h.state_mut().tilesets[0].page = Page::Tiles;
    h.state_mut().tilesets[0].tile = Some(1);
    settle(&mut h);
    assert!(h.query_by_label("Preview, Tile 1").is_some(), "its number alone");
    assert!(h.query_by_label_contains("Preview, Tile 1,").is_none());
    assert!(h.query_by_label_contains("Textures, of").is_none(), "no \"of\" of nothing");
    assert!(h.query_by_label_contains(tv::DIAGRAM_NAME).is_some());
}

/// A finding of the whole file with no place (`at` empty) is named by the
/// file, so its link has words (never an empty link to focus), goes to the
/// tileset, and F6 lands on it.
#[test]
fn a_finding_of_the_whole_file_names_the_file() {
    let (mut h, path, id) = tileset_window(Mode::Tabs, "ui-u20-whole", 1600.0, None, false);
    let finding = mg_module::doctor::Finding {
        severity: mg_module::doctor::Severity::Warning,
        check: "set-data".into(),
        source: "tileset".into(),
        resource: ResKey::parse("zzt01", ResType::SET).unwrap(),
        at: String::new(),
        message: "a line the reader passed over".into(),
    };
    tv::record_check(h.state_mut(), id, &[finding]);
    settle(&mut h);
    assert_eq!(tileset_problems(&h, &path)[0].at, "zzt01.set");
    let link = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.data().supports_action(egui::accesskit::Action::Focus)
            && n.role() != egui::accesskit::Role::Unknown
            && n.label().as_deref() == Some("zzt01.set")
    };
    let rows = h.get_by_label(mg_ui::problems::PROBLEM_ROWS);
    let links: Vec<String> =
        rows.query_all_by(link).filter_map(|n| n.accesskit_node().label()).collect();
    assert_eq!(links, ["zzt01.set"], "a link of words");
    // F6 to Problems: its first link focused, not an empty one.
    let mut found = false;
    for _ in 0..8 {
        h.key_press(egui::Key::F6);
        settle(&mut h);
        if mg_ui::palette_view::arrow_keys_are(&h.ctx) == "Problems" {
            let focused = h.get_by_label(mg_ui::problems::PROBLEM_ROWS).query_all_by(link).next();
            assert!(focused.unwrap().accesskit_node().is_focused(), "the file's link focused");
            found = true;
            break;
        }
    }
    assert!(found, "F6 reached Problems");
}

/// The Problems rows are made once for what they show: frames that change
/// nothing make none (the pointer moving over them), a Check made again
/// makes them once.
#[test]
fn problems_rows_are_made_once_for_what_they_show() {
    let (mut h, _, id) = tileset_window(Mode::Tabs, "ui-u20-rows", 1600.0, None, false);
    let finding = mg_module::doctor::Finding {
        severity: mg_module::doctor::Severity::Error,
        check: "set-model".into(),
        source: "tileset".into(),
        resource: ResKey::parse("zzt01", ResType::SET).unwrap(),
        at: "[TILE1] Model".into(),
        message: "model zzt01_a02_01 isn't in the haks or the game".into(),
    };
    tv::record_check(h.state_mut(), id, std::slice::from_ref(&finding));
    settle(&mut h);
    let built = h.state().problems.row_builds();
    let rows = h.get_by_label(mg_ui::problems::PROBLEM_ROWS).get_by_label("Tile 1, Model").rect();
    for dx in 0..8 {
        h.hover_at(rows.center() + egui::vec2(dx as f32 * 5.0, 0.0));
        h.step();
    }
    assert_eq!(h.state().problems.row_builds(), built, "none in frames that change nothing");
    tv::record_check(h.state_mut(), id, &[finding]);
    settle(&mut h);
    assert_eq!(h.state().problems.row_builds(), built + 1, "once for a Check made again");
}

/// Typed text keeps to its tile when the press on Remove Last Tile and its
/// release come in one frame (a quick click on a slow frame): the text is
/// set on the last tile before it is taken away, so an Undo brings it back
/// with the text.
#[test]
fn typed_text_is_set_before_a_one_frame_click_removes_its_tile() {
    let (mut h, _, _) = tileset_window(Mode::Tabs, "ui-u20-typed-one", 1600.0, None, false);
    h.state_mut().tilesets[0].tile = Some(2);
    settle(&mut h);
    field_with(&h, "tcn01_a02_01").click();
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("zzt01_one".into()));
    settle(&mut h);
    // (Typed, not set: the field keeps the keyboard until the press.)
    let field = egui::Id::new(("set", doc(&h).id, "TILE2", "Model"));
    assert_eq!(h.ctx.memory(|m| m.focused()), Some(field), "the field has the keyboard");
    assert_eq!(set(&h).tiles[2].model, "tcn01_a02_01", "not set yet");
    let at = h.get_by_label(tv::REMOVE_LAST_TILE).rect().center();
    let button = |pressed| egui::Event::PointerButton {
        pos: at,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    h.event(egui::Event::PointerMoved(at));
    h.run();
    h.event(button(true));
    h.event(button(false));
    h.run();
    settle(&mut h);
    assert_eq!(set(&h).tiles.len(), 2, "taken away");
    tool(&h, "Undo").click();
    settle(&mut h);
    assert_eq!(set(&h).tiles[2].model, "zzt01_one", "the text set on it first");
}

/// The keys of the tile table stay its own: with the Outliner's rows
/// holding the keys, Down, Delete and letters move nothing in the table
/// nor take a tile away; with a header popover (More) open, the table's
/// keys act not.
#[test]
fn the_tile_table_takes_no_keys_it_has_not() {
    let (mut h, _, _) = tileset_window(Mode::Tabs, "ui-u20-keys-others", 1600.0, None, false);
    click_pos(&h, tile_row(&h, 1).center());
    settle(&mut h);
    // (Its Tiles group's row: the Tiles page stays, the keys the
    // Outliner's.)
    outline_row(&h, "3 tiles").click();
    settle(&mut h);
    assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Marks");
    for key in [egui::Key::ArrowDown, egui::Key::Delete] {
        h.key_press(key);
        settle(&mut h);
    }
    h.event(egui::Event::Text("t".into()));
    settle(&mut h);
    assert_eq!((doc(&h).tile, set(&h).tiles.len()), (Some(1), 3), "the table untouched");
    // A popover open: the header narrowed, More open.
    h.set_size(egui::vec2(640.0, 900.0));
    settle(&mut h);
    click_pos(&h, tile_row(&h, 1).center());
    settle(&mut h);
    h.get_by_label(TILESET_HEADER).get_by_label(mg_uikit::header::MORE).click();
    settle(&mut h);
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(doc(&h).tile, Some(1), "no keys under a popover");
}

/// What is typed in the form is set when the Right Dock is folded while it
/// is typed in, and OK and Cancel do what they say: OK saves and closes,
/// Cancel closes as last saved after Don't Save.
#[test]
fn typing_through_a_fold_and_ok_and_cancel() {
    let (mut h, path, _) = tileset_window(Mode::Tabs, "ui-u20-ok", 1600.0, None, false);
    h.state_mut().tilesets[0].tile = Some(0);
    settle(&mut h);
    field_with(&h, "msb01").click();
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    field_with(&h, "msb01").type_text("msf01");
    settle(&mut h);
    h.state_mut().settings.hide_palettes = true;
    settle(&mut h);
    click_pos(&h, tile_row(&h, 2).center());
    settle(&mut h);
    assert_eq!(set(&h).tiles[0].walkmesh.as_deref(), Some("msf01"), "set through the fold");
    // OK: saved and closed.
    h.get_by_label(TILESET_HEADER).get_by_label("OK").click();
    settle(&mut h);
    assert!(h.state().tilesets.is_empty(), "closed");
    assert!(std::fs::read_to_string(&path).unwrap().contains("WalkMesh=msf01"), "saved");
    // Cancel: as last saved.
    tv::open_path(h.state_mut(), path.clone());
    settle(&mut h);
    h.state_mut().tilesets[0].change(|f| f.set("TILE0", "PathNode", "Z"));
    settle(&mut h);
    h.get_by_label(TILESET_HEADER).get_by_label("Cancel").click();
    settle(&mut h);
    h.get_by_label("Don't Save").click();
    settle(&mut h);
    assert!(h.state().tilesets.is_empty(), "closed");
    assert!(!std::fs::read_to_string(&path).unwrap().contains("PathNode=Z"), "as last saved");
}

/// The tileset's panels fit the Right Dock from its least, 280, to 480:
/// the diagram and the door sentence inside the dock, the textures' rows
/// cut to it, nothing wider than it.
#[test]
fn the_tileset_panels_fit_the_right_dock_from_280() {
    let root = mg_testkit::corpus!();
    let (mut h, _, _) = tileset_window(Mode::Tabs, "ui-u20-fit", 1600.0, Some(&root), false);
    h.state_mut().tilesets[0].tile = Some(0);
    settle(&mut h);
    for width in (280..=480).step_by(8) {
        right_dock_width(&mut h, width as f32);
        settle(&mut h);
        let dock = right_dock_rect(&h).expand(0.5);
        let diagram = h.get_by_label_contains(tv::DIAGRAM_NAME).rect();
        assert!(dock.contains_rect(diagram), "{width}: the diagram {diagram:?} in {dock:?}");
        let sentence = h.get_by_label_contains("Door 1: doortypes.2da row 0").rect();
        assert!(dock.contains_rect(sentence), "{width}: the door sentence");
        for row in h
            .get_by_label(tv::TEXTURE_ROWS)
            .query_all_by(|n| n.role() == egui::accesskit::Role::Label)
        {
            let r = row.rect();
            assert!(r.right() <= dock.right(), "{width}: a texture's row {r:?}");
        }
    }
}

/// A tileset the game has, only looked at (the resource view): its text,
/// no tileset editor, no Problems source of it.
#[test]
fn a_game_tileset_is_only_looked_at() {
    let root = mg_testkit::corpus!();
    let mut app = game_app(&root, None);
    app.settings = Mode::Tabs.settings();
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    let key = ResKey::parse("tcn01", ResType::SET).unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(key)));
    settle(&mut h);
    assert!(h.state().tilesets.is_empty(), "no editor");
    assert!(mg_uikit::header::records(&h.ctx, TILESET_HEADER).is_empty(), "no header");
    assert!(h.state().problems.sets().is_empty());
}

/// The Preview's 3D view of a tile rests (the coordinator's ruling at U20's
/// review): a tile's view begins paused, the window coming to rest with it
/// drawn (with a GPU, in a window tall enough for it under the diagram);
/// Play animates it (the game's programs that read the clock ask a frame
/// each step); Pause rests again.
#[test]
fn the_tile_s_3d_view_rests_until_played() {
    let root = mg_testkit::corpus!();
    let Some(rs) = gpu() else { return };
    let path = set_file("ui-u20-3d");
    let mut app = game_app(&root, None);
    app.settings = Mode::Tabs.settings();
    app.set_render_state(rs.clone());
    let mut h = gpu_harness(app, rs, 1600.0, 1500.0);
    settle(&mut h);
    tv::open_path(h.state_mut(), path);
    settle(&mut h);
    h.state_mut().tilesets[0].page = Page::Tiles;
    h.state_mut().tilesets[0].tile = Some(0);
    settle(&mut h);
    let key = ResKey::parse("tcn01_a01_01", ResType::MDL).unwrap();
    let source = mg_ui::model_view::Source::Resource(key);
    let view = |h: &Harness<'_, Moonglow>| {
        let v = h.state().model_views.get(&source).expect("the tile's view made");
        (v.playing, v.drawn_in())
    };
    let (playing, drawn) = view(&h);
    assert!(!playing, "paused at first");
    let pass = h.ctx.cumulative_pass_nr();
    assert!(drawn.is_some_and(|d| d + 3 >= pass), "drawn: {drawn:?} at {pass}");
    h.get_by_label("Play").click();
    assert!(h.try_run().is_err(), "played: it asks for frames");
    assert!(view(&h).0);
    h.get_by_label("Pause").click();
    settle(&mut h);
    assert!(!view(&h).0, "paused again, at rest");
}

/// Steps the window until `done` holds (the watch of the open tilesets'
/// files hears another program's write off the window's thread, and a
/// burst once it is quiet a moment): at most ten seconds.
#[track_caller]
fn outside_wait(h: &mut Harness<'_, Moonglow>, what: &str, done: impl Fn(&Moonglow) -> bool) {
    let started = std::time::Instant::now();
    while !done(h.state()) {
        assert!(started.elapsed().as_secs() < 10, "never: {what}: {:?}", h.state().log.entries);
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    settle(h);
}

/// Steps the window for `ms` milliseconds, as time goes by with nothing
/// asked of it (what a watch would hear by then, it has).
fn outside_quiet(h: &mut Harness<'_, Moonglow>, ms: u64) {
    let started = std::time::Instant::now();
    while started.elapsed() < std::time::Duration::from_millis(ms) {
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    settle(h);
}

/// How many log lines have `what`.
fn logged(h: &Harness<'_, Moonglow>, what: &str) -> usize {
    h.state().log.entries.iter().filter(|(_, m)| m.contains(what)).count()
}

/// The watch begun on the tileset open, and its first hearing over.
fn watched(h: &mut Harness<'_, Moonglow>) {
    outside_quiet(h, 600);
}

/// A tileset's file changed by another program while it is open, with no
/// unsaved change here (M5's W5, the owner's ruling of 2026-11-21: "reread
/// on change"): the editor shows the file as it is now, as saved, its
/// history gone, the page and the tile chosen kept; five writes in a burst
/// are read once; a file written whole and moved into place, twice, is read
/// each time; a tile chosen that the file no longer has is let go of; with
/// Options' reloading off nothing is read.
#[test]
fn a_tileset_changed_outside_is_read_again() {
    let (mut h, path, _) = tileset_window(Mode::Tabs, "ui-w5-reread", 1600.0, None, false);
    h.state_mut().tilesets[0].tile = Some(2);
    // (A step to undo, of a change since saved: not unsaved.)
    h.state_mut().tilesets[0].change(|f| f.set("TILE0", "PathNode", "B"));
    h.state_mut().tilesets[0].save().unwrap();
    settle(&mut h);
    assert!(doc(&h).has_undo() && !doc(&h).is_dirty());
    watched(&mut h);
    let model = |app: &Moonglow, n: usize| {
        app.tilesets[0].tileset().map(|t| t.tiles[n].model.clone()).unwrap_or_default()
    };
    let theirs = |m: &str| {
        SET.replace("Model=tcn01_a02_01", &format!("Model={m}")).replace("PathNode=A", "PathNode=B")
    };
    std::fs::write(&path, theirs("zzt01_b01_01")).unwrap();
    outside_wait(&mut h, "read again", |app| model(app, 2) == "zzt01_b01_01");
    let d = doc(&h);
    assert!(!d.is_dirty() && !d.has_undo() && !d.has_redo(), "as saved, no history");
    assert_eq!((d.page, d.tile), (Page::Tiles, Some(2)), "the page and the tile kept");
    // (The form's field, not the table's row of the same words: the field
    // showed the old model, the window resting on it.)
    field_with(&h, "zzt01_b01_01");
    assert_eq!(logged(&h, "read again (changed outside Moonglow)"), 1);
    // A burst: read once.
    for n in 0..5 {
        std::fs::write(&path, theirs(&format!("zzt01_c0{n}_01"))).unwrap();
    }
    outside_wait(&mut h, "the burst's last", |app| model(app, 2) == "zzt01_c04_01");
    outside_quiet(&mut h, 800);
    assert_eq!(logged(&h, "read again (changed outside Moonglow)"), 2, "the burst read once");
    // Written whole and moved into place, twice.
    for m in ["zzt01_d01_01", "zzt01_d02_01"] {
        let part = path.with_extension("part");
        std::fs::write(&part, theirs(m)).unwrap();
        std::fs::rename(&part, &path).unwrap();
        outside_wait(&mut h, m, |app| model(app, 2) == m);
    }
    // The tile chosen gone from the file.
    let two = theirs("x").replace("[TILES]\r\nCount=3", "[TILES]\r\nCount=2");
    let two = &two[..two.find("[TILE2]").unwrap()];
    std::fs::write(&path, format!("{two}[GROUPS]\r\nCount=0\r\n")).unwrap();
    outside_wait(&mut h, "two tiles", |app| {
        app.tilesets[0].tileset().is_some_and(|t| t.tiles.len() == 2)
    });
    assert_eq!(doc(&h).tile, None, "the tile chosen let go of");
    // Reloading off: nothing read.
    h.state_mut().settings.no_auto_reload = true;
    settle(&mut h);
    std::fs::write(&path, theirs("zzt01_e01_01")).unwrap();
    outside_quiet(&mut h, 1200);
    assert_eq!(set(&h).tiles.len(), 2, "not read with reloading off");
}

/// A tileset's file changed by another program while it has unsaved
/// changes here: Moonglow's are kept and Changed Outside Moonglow asks
/// which to keep, as for a script; meanwhile Save does not write over the
/// file, and refused it asks again after Later. Keep Moonglow's: Save
/// writes ours over theirs. Take the Files': the editor shows theirs.
#[test]
fn a_tileset_changed_outside_with_unsaved_edits_asks_which_to_keep() {
    let (mut h, path, id) = tileset_window(Mode::Tabs, "ui-w5-conflict", 1600.0, None, false);
    watched(&mut h);
    let asked = |h: &Harness<'_, Moonglow>| h.query_by_label("Changed Outside Moonglow").is_some();
    let theirs = SET.replace("Model=tcn01_a02_01", "Model=zzt01_f01_01");
    let round = |h: &mut Harness<'_, Moonglow>, theirs: &str, ours: &str| {
        h.state_mut().tilesets[0].change(|f| f.set("GENERAL", "Transition", ours));
        settle(h);
        std::fs::write(&path, theirs).unwrap();
        outside_wait(h, "asked", |app| !app.outside_conflicts().is_empty());
        assert_eq!(h.state().outside_conflicts(), std::slice::from_ref(&path));
        assert!(asked(h), "Changed Outside Moonglow on screen");
        assert!(doc(h).file.text().contains(&format!("Transition={ours}")), "ours kept");
        assert_eq!(set(h).tiles[2].model, "tcn01_a02_01", "theirs not taken");
    };
    round(&mut h, &theirs, "5");
    assert_eq!(logged(&h, "has unsaved changes here: Moonglow's is kept"), 1);
    // Later, then Save: refused, asked again.
    h.get_by_label("Later").click();
    settle(&mut h);
    assert!(!asked(&h), "Later");
    h.get_by_label(TILESET_HEADER).get_by_label("Save").click();
    settle(&mut h);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), theirs, "not written over");
    assert!(asked(&h), "asked again");
    // Keep Moonglow's: Save writes ours.
    h.get_by_label("Keep Moonglow's").click();
    settle(&mut h);
    assert!(!asked(&h) && h.state().outside_conflicts().is_empty());
    assert!(doc(&h).is_dirty(), "not saved over theirs yet");
    tool(&h, "Undo").click();
    settle(&mut h);
    assert!(doc(&h).is_dirty(), "undone to the text read: still not the file's");
    tool(&h, "Redo").click();
    settle(&mut h);
    outside_quiet(&mut h, 800);
    assert!(!asked(&h), "not asked again of the same file");
    // (The file looked at again, as the watch's walk does: the change
    // answered is not asked about again.)
    std::fs::write(&path, &theirs).unwrap();
    outside_quiet(&mut h, 900);
    assert!(!asked(&h) && h.state().outside_conflicts().is_empty(), "answered once");
    h.get_by_label(TILESET_HEADER).get_by_label("Save").click();
    settle(&mut h);
    let ours = std::fs::read_to_string(&path).unwrap();
    assert!(ours.contains("Transition=5") && !ours.contains("zzt01_f01_01"), "ours written");
    assert!(!doc(&h).is_dirty());
    // (Changed again at once, before the watch hears the save: its own
    // write, not another program's, asks nothing of the unsaved change.)
    h.state_mut().tilesets[0].change(|f| f.set("TILE0", "PathNode", "C"));
    outside_quiet(&mut h, 800);
    assert!(!asked(&h) && doc(&h).tileset().is_some(), "our own save heard as no change");
    assert!(doc(&h).is_dirty(), "the change after it kept");
    h.state_mut().tilesets[0].undo();
    settle(&mut h);
    assert!(!doc(&h).is_dirty());
    // Take the Files'.
    round(&mut h, &theirs, "6");
    h.get_by_label("Take the Files'").click();
    settle(&mut h);
    assert!(!asked(&h) && h.state().outside_conflicts().is_empty());
    assert_eq!(set(&h).tiles[2].model, "zzt01_f01_01", "theirs taken");
    assert!(!doc(&h).is_dirty() && !doc(&h).has_undo());
    assert_eq!(h.state().tilesets[0].id, id);
}

/// A tileset's file emptied, cut off before its tiles or deleted by
/// another program keeps the editor as it was (the last state that read),
/// and the log says so once for each (what Check found stays current: it
/// is of the editor's text, which is as checked); written whole again, it
/// is read. What Check found follows the text read: said changed, then
/// current again with the text checked put back.
#[test]
fn a_tileset_file_half_written_or_gone_keeps_the_last_good_state() {
    let (mut h, path, id) = tileset_window(Mode::Tabs, "ui-w5-broken", 1600.0, None, false);
    let finding = mg_module::doctor::Finding {
        severity: mg_module::doctor::Severity::Error,
        check: "set-model".into(),
        source: "tileset".into(),
        resource: ResKey::parse("zzt01", ResType::SET).unwrap(),
        at: "[TILE1] Model".into(),
        message: "model zzt01_a02_01 isn't in the haks or the game".into(),
    };
    tv::record_check(h.state_mut(), id, std::slice::from_ref(&finding));
    watched(&mut h);
    let before = doc(&h).file.text().to_string();
    let cut = &SET[..SET.find("[TILES]").unwrap()];
    for (n, broken) in ["", cut].into_iter().enumerate() {
        std::fs::write(&path, broken).unwrap();
        outside_wait(&mut h, "said", |app| {
            app.log.entries.iter().filter(|(_, m)| m.contains("can't be read as a tileset")).count()
                == n + 1
        });
        assert_eq!(doc(&h).file.text(), before, "the last good state kept");
        assert!(!doc(&h).is_dirty());
    }
    std::fs::remove_file(&path).unwrap();
    outside_wait(&mut h, "said gone", |app| {
        app.log.entries.iter().any(|(_, m)| m.contains("can't be read now"))
    });
    assert_eq!(doc(&h).file.text(), before, "kept, the file gone");
    assert_eq!(h.state().problems.sets()[0].stale, None, "of the editor's text, as checked");
    // (Made and deleted again in a moment: still unreadable, not said
    // again.)
    std::fs::write(&path, SET).unwrap();
    std::fs::remove_file(&path).unwrap();
    outside_quiet(&mut h, 900);
    assert_eq!(logged(&h, "can't be read now"), 1, "said once");
    let theirs = SET.replace("Model=tcn01_a02_01", "Model=zzt01_g01_01");
    std::fs::write(&path, &theirs).unwrap();
    outside_wait(&mut h, "read again", |app| {
        app.tilesets[0].tileset().is_some_and(|t| t.tiles[2].model == "zzt01_g01_01")
    });
    assert_eq!(h.state().problems.sets()[0].stale, Some(Stale::Changed), "not what was checked");
    std::fs::write(&path, SET).unwrap();
    outside_wait(&mut h, "the checked text back", |app| {
        app.tilesets[0].tileset().is_some_and(|t| t.tiles[2].model == "tcn01_a02_01")
    });
    assert_eq!(h.state().problems.sets()[0].stale, None, "current again");
}

/// A tileset the game data reads from the override, open in the tileset
/// editor and drawn by an open area: changed by another program, the
/// editor shows it and the area's view and the tiles palette read it again
/// (the watch of the folders the game data reads).
#[test]
fn a_tileset_in_the_override_changed_outside_is_redrawn() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let dir = mg_testkit::scratch_dir("ui-w5-override");
    let user = user_folder(&dir, &["override"]);
    let set = {
        let install = mg_resman::GameInstall::new(&root, None, "en");
        let game = mg_resman::ResMan::for_game(&install).unwrap();
        game.get(&ResKey::parse("ttr01", ResType::SET).unwrap()).unwrap().into_owned()
    };
    let path = user.join("override/ttr01.set");
    std::fs::write(&path, &set).unwrap();
    let Some((mut h, area)) =
        area_harness_with("ui-w5-override", "ttr01", None, 4, Some(user), &|_, _, _| {})
    else {
        return;
    };
    settle(&mut h);
    tv::open_path(h.state_mut(), path.clone());
    settle(&mut h);
    watched(&mut h);
    let tileset = mg_core::ResRef::from_str("ttr01").unwrap();
    // (The tileset's tile the area's first tile is, as its view drew it.)
    let drawn = |app: &Moonglow| {
        let view = app.area_views.get(&area)?;
        let tile = view.model.as_ref()?.tiles.first()?;
        Some((tile.id, tile.model.clone()))
    };
    outside_wait(&mut h, "the area drawn", |app| drawn(app).is_some());
    let (id, _) = drawn(h.state()).unwrap();
    let parsed = mg_set::Tileset::parse(&set, mg_core::Codepage::default()).unwrap();
    let model = parsed.tiles[id as usize].model.clone();
    let text = String::from_utf8_lossy(&set).into_owned();
    let section = text.find(&format!("[TILE{id}]")).unwrap();
    let at = section + text[section..].find(&format!("Model={model}")).unwrap();
    let mut theirs = text.clone();
    theirs.replace_range(at..at + 6 + model.len(), "Model=ttr01_w5_00");
    std::fs::write(&path, theirs).unwrap();
    outside_wait(&mut h, "the editor's", |app| {
        app.tilesets[0].tileset().is_some_and(|t| t.tiles[id as usize].model == "ttr01_w5_00")
    });
    outside_wait(&mut h, "the game data's", |app| {
        app.tileset(tileset).is_some_and(|t| t.tiles[id as usize].model == "ttr01_w5_00")
    });
    outside_wait(&mut h, "the area's view", |app| {
        drawn(app).is_some_and(|(_, m)| m.as_deref() == Some("ttr01_w5_00"))
    });
    assert!(logged(&h, "Reloaded override") >= 1, "the override read again");
}

/// A look at every file (the watch's start, its walk every few seconds
/// where the system does not tell it, or after its events overflowed)
/// reads only the tilesets whose file's time or length moved: one made
/// unreadable without changing them (its mode) is not read as the watch
/// starts again, so nothing is said of it; rewritten, the walk finds it and
/// it is read again.
#[cfg(unix)]
#[test]
fn a_look_at_every_file_reads_only_a_tileset_changed() {
    use std::os::unix::fs::PermissionsExt;
    let (mut h, path, _) = tileset_window(Mode::Tabs, "ui-w5-walk", 1600.0, None, false);
    watched(&mut h);
    struct RestorePermissions<'a>(&'a std::path::Path);
    impl Drop for RestorePermissions<'_> {
        fn drop(&mut self) {
            let _ = std::fs::set_permissions(self.0, std::fs::Permissions::from_mode(0o644));
        }
    }
    let _restore = RestorePermissions(&path);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
    h.state_mut().walk_tileset_files();
    outside_quiet(&mut h, 900);
    drop(_restore);
    assert_eq!(logged(&h, "can't be read now"), 0, "the untouched file not read");
    std::fs::write(&path, SET.replace("Model=tcn01_a01_01", "Model=zzt01_walked")).unwrap();
    outside_wait(&mut h, "read again by the walk", |app| {
        app.tilesets[0].tileset().is_some_and(|t| t.tiles[0].model == "zzt01_walked")
    });
    assert_eq!(logged(&h, "read again (changed outside Moonglow)"), 1);
}

/// Text typed into a tileset's field and not yet set when its file changes
/// outside is set first, so that it counts as an unsaved change: kept, and
/// asked about (Changed Outside Moonglow), never lost to the file's text.
#[test]
fn text_typed_when_the_file_changes_outside_is_kept_and_asked_about() {
    let (mut h, path, _) = tileset_window(Mode::Tabs, "ui-w5-typed", 1600.0, None, false);
    h.state_mut().tilesets[0].tile = Some(2);
    settle(&mut h);
    watched(&mut h);
    field_with(&h, "tcn01_a02_01").click();
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    field_with(&h, "tcn01_a02_01").type_text("zzt01_typed");
    settle(&mut h);
    assert!(!doc(&h).is_dirty(), "typed, not yet set");
    std::fs::write(&path, SET.replace("Model=tcn01_a01_01", "Model=zzt01_theirs")).unwrap();
    outside_wait(&mut h, "asked", |app| !app.outside_conflicts().is_empty());
    let t = set(&h);
    assert_eq!(t.tiles[2].model, "zzt01_typed", "what was typed set and kept");
    assert_eq!(t.tiles[0].model, "tcn01_a01_01", "theirs not taken");
}

/// Text typed into another tileset's field (its tab in front of the one
/// whose file changed) keeps the keyboard and stays unset: the keyboard is
/// taken only from a field of the editor whose file changed.
#[test]
fn text_typed_in_another_editor_keeps_the_keyboard_when_a_file_changes_outside() {
    let (mut h, path, first) = tileset_window(Mode::Tabs, "ui-w5-typed-other", 1600.0, None, false);
    let other = set_file(&Mode::Tabs.dir("ui-w5-typed-other-2"));
    tv::open_path(h.state_mut(), other);
    settle(&mut h);
    let second = h.state().tilesets.iter().position(|d| d.id != first).unwrap();
    h.state_mut().tilesets[second].page = Page::Tiles;
    h.state_mut().tilesets[second].tile = Some(2);
    settle(&mut h);
    watched(&mut h);
    field_with(&h, "tcn01_a02_01").click();
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    field_with(&h, "tcn01_a02_01").type_text("zzt01_typed");
    settle(&mut h);
    std::fs::write(&path, SET.replace("Model=tcn01_a01_01", "Model=zzt01_theirs")).unwrap();
    outside_wait(&mut h, "the first read again", |app| {
        app.tilesets.iter().any(|d| {
            d.id == first && d.tileset().is_some_and(|t| t.tiles[0].model == "zzt01_theirs")
        })
    });
    assert!(h.ctx.memory(|m| m.focused()).is_some(), "the other's field keeps the keyboard");
    assert!(!h.state().tilesets[second].is_dirty(), "its text not set by the change");
}

/// A floating tileset's file changed outside (review D of M5's fix rounds
/// had not looked at one): read again as a docked one is, with nothing
/// typed; with text typed into one of its fields (in its own window, of
/// a layer of its own), that text set and kept, and Changed Outside
/// Moonglow asking which to keep.
#[test]
fn a_floating_tileset_changed_outside_is_read_again_and_typed_text_kept() {
    let (mut h, path, _) =
        tileset_window(Mode::Floating, "ui-w5-floating-reread", 1600.0, None, false);
    let at = h.state().dock.find_tab(&Tab::Tileset(h.state().tilesets[0].id)).unwrap();
    assert!(!at.surface.is_main(), "floating");
    h.state_mut().tilesets[0].tile = Some(2);
    settle(&mut h);
    watched(&mut h);
    let model = |app: &Moonglow, n: usize| {
        app.tilesets[0].tileset().map(|t| t.tiles[n].model.clone()).unwrap_or_default()
    };
    std::fs::write(&path, SET.replace("Model=tcn01_a02_01", "Model=zzt01_b01_01")).unwrap();
    outside_wait(&mut h, "read again", |app| model(app, 2) == "zzt01_b01_01");
    // Typed into its field (which shows the model read): set, kept and asked about.
    field_with(&h, "zzt01_b01_01").click();
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    field_with(&h, "zzt01_b01_01").type_text("zzt01_typed");
    settle(&mut h);
    assert!(!doc(&h).is_dirty(), "typed, not yet set");
    std::fs::write(&path, SET.replace("Model=tcn01_a01_01", "Model=zzt01_theirs")).unwrap();
    outside_wait(&mut h, "asked", |app| !app.outside_conflicts().is_empty());
    let t = set(&h);
    assert_eq!(t.tiles[2].model, "zzt01_typed", "what was typed set and kept");
    assert_eq!(t.tiles[0].model, "tcn01_a01_01", "theirs not taken");
}

/// A tileset kept in the module and drawn by an open area: another `.set`
/// imported over it is read again by the area's view and the game data,
/// as one changed in a folder the game data reads is (the deferred entry
/// "A tileset kept in the module, changed, not redrawn in open areas").
/// The area's first tile is given another tile's model; `MG_SHOT` keeps
/// the area before and after. Undo takes it back; Redo while a job
/// holds the game data is drawn once the job is done.
#[test]
fn a_tileset_kept_in_the_module_changed_is_redrawn() {
    let Some(root) = mg_testkit::nwn_root() else { return };
    let key = ResKey::parse("ttr01", ResType::SET).unwrap();
    let set = {
        let install = mg_resman::GameInstall::new(&root, None, "en");
        let game = mg_resman::ResMan::for_game(&install).unwrap();
        game.get(&key).unwrap().into_owned()
    };
    let kept = set.clone();
    let prepare = move |m: &mut mg_module::Module, _: &mg_rules::GameData, _: ResRef| {
        m.set(key, kept.clone());
    };
    let Some((mut h, area)) = area_harness_with("ui-module-set", "ttr01", None, 4, None, &prepare)
    else {
        return;
    };
    settle(&mut h);
    let drawn = |app: &Moonglow| {
        let view = app.area_views.get(&area)?;
        let tile = view.model.as_ref()?.tiles.first()?;
        Some((tile.id, tile.model.clone()))
    };
    outside_wait(&mut h, "the area drawn", |app| drawn(app).is_some());
    let (id, _) = drawn(h.state()).unwrap();
    let parsed = mg_set::Tileset::parse(&set, mg_core::Codepage::default()).unwrap();
    let model = parsed.tiles[id as usize].model.clone();
    let other = (parsed.tiles.iter().map(|t| &t.model))
        .find(|m| m[..m.len() - 3] != model[..model.len() - 3])
        .expect("a tile of another kind")
        .clone();
    let text = String::from_utf8_lossy(&set).into_owned();
    let section = text.find(&format!("[TILE{id}]")).unwrap();
    let at = section + text[section..].find(&format!("Model={model}")).unwrap();
    let mut changed = text.clone();
    changed.replace_range(at..at + 6 + model.len(), &format!("Model={other}"));
    let shots = shot_dir("ui-module-set");
    if let Some(dir) = &shots {
        h.render().expect("render").save(dir.join("before.png")).unwrap();
    }
    let files = mg_testkit::scratch_dir("ui-module-set-files");
    std::fs::write(files.join("ttr01.set"), changed).unwrap();
    let tileset = mg_core::ResRef::from_str("ttr01").unwrap();
    h.state_mut().dialogs = Box::new(mg_ui::NoDialogs {
        open_many: vec![vec![files.join("ttr01.set")]],
        ..Default::default()
    });
    h.state_mut().actions.push(mg_ui::Action::ImportFilesDialog);
    settle(&mut h);
    h.get_by_label("Select All").click();
    settle(&mut h);
    h.get_by_label("Import").click();
    settle(&mut h);
    for _ in 0..10 {
        h.step();
    }
    if let Some(dir) = &shots {
        h.render().expect("render").save(dir.join("imported.png")).unwrap();
    }
    outside_wait(&mut h, "the game data's", |app| {
        app.tileset(tileset).is_some_and(|t| t.tiles[id as usize].model == other)
    });
    outside_wait(&mut h, "the area's view", |app| {
        drawn(app).is_some_and(|(_, m)| m.as_deref() == Some(other.as_str()))
    });
    if let Some(dir) = &shots {
        h.render().expect("render").save(dir.join("after.png")).unwrap();
    }
    // Undone: the tile as it was.
    let is = |m: &str| {
        let m = m.to_string();
        move |app: &Moonglow| drawn(app).is_some_and(|(_, d)| d.as_deref() == Some(m.as_str()))
    };
    h.state_mut().run(mg_ui::Action::Undo);
    outside_wait(&mut h, "the import undone", is(&model));
    // Redone while a job holds the game data: drawn once it is free.
    let (go, held) = std::sync::mpsc::channel::<()>();
    h.state_mut().start_job_for_tests("Held", move |_| {
        let _ = held.recv();
    });
    h.state_mut().run(mg_ui::Action::Redo);
    h.run_steps(3);
    assert!(is(&model)(h.state()), "not while the game data is held");
    go.send(()).unwrap();
    outside_wait(&mut h, "the import redone, once free", is(&other));
}
