//! A table's panels (M4's U16; the Table board): the header on the kit's
//! row (every control on it or in More at every width, OK and Cancel at
//! its end), the Right Dock's Cell and References, or a floating table's
//! window's right pane; the Outliner's columns, each with its kind and its
//! eye; the Status Bar's counts and hints; and what holds them: the Cell
//! panel editing the cell, by each chooser its column's kind has, its
//! typed text kept to its own cell, References finding the row's users on
//! a thread of its own, every panel fitting the dock from its least width,
//! Undo and Cancel reaching the panels' edits, the panels following the
//! table in front, the window coming to rest, and records from before
//! reading without a fault.

use crate::common::*;
use mg_ui::Panel;

/// A table of ours (Label, the strings' Name, the flags' Mask, the list's
/// Kind, Next a row of the table itself, Cost a number from 0 to 9; and
/// More, a column no description names).
const MINE: &str = "2DA V2.0\r\n\r\n   Label   Name  Mask  Kind  Next  Cost  More\r\n\
                    0  Bread   ****  0x1   0     1     1     a\r\n\
                    1  Cheese  ****  0x3   1     0     2     b\r\n\
                    2  Ale     ****  ****  ****  ****  3     c\r\n\
                    3  Eggs    ****  0x4   0     2     4     d\r\n";

/// [`MINE`]'s description.
const MINE_TOML: &str = "[columns.Name]\nkind = \"strref\"\n\
                         [columns.Mask]\nkind = \"flags\"\nvalues = { Red = 1, Green = 2, Blue = 4 }\n\
                         [columns.Kind]\nkind = \"list\"\nvalues = { Small = 0, Large = 1 }\n\
                         [columns.Next]\nkind = \"row\"\ntable = \"mine\"\n\
                         [columns.Cost]\nkind = \"int\"\nmin = 0\nmax = 9\n";

/// The columns of [`MINE`], by their places.
const LABEL: usize = 0;
const MASK: usize = 2;
const KIND: usize = 3;
const NEXT: usize = 4;
const COST: usize = 5;

/// A module open in `mode` in a window `width` by 900, and [`MINE`] with
/// its description in a folder of its own (a project's: `nasher.cfg`),
/// opened in the 2DA editor, its description read; the table's id.
fn table_window(mode: Mode, name: &str, width: f32) -> (Harness<'static, Moonglow>, u32) {
    let dir = mg_testkit::scratch_dir(&mode.dir(name));
    let path = sample_module(&dir);
    let tables = dir.join("tables");
    std::fs::create_dir_all(&tables).unwrap();
    std::fs::write(tables.join("nasher.cfg"), "").unwrap();
    std::fs::write(tables.join("mine.2da"), MINE).unwrap();
    std::fs::write(tables.join("mine.2da.toml"), MINE_TOML).unwrap();
    let mut app = app_in(mode, Vec::new());
    app.settings.no_last_area = true;
    // (Floating in a screen of 1920: its window kept wide enough for both
    // its panes beside the grid's room.)
    if mode == Mode::Floating && width >= 1900.0 {
        app.settings.window_sizes = vec![("table".to_string(), [1640, 820])];
    }
    app.open_module(&path);
    let mut h = harness(app, width, 900.0);
    settle(&mut h);
    let id = mg_ui::table_view::open_path(h.state_mut(), &tables.join("mine.2da")).unwrap();
    settle(&mut h);
    assert!(doc(&h, id).description().is_some(), "described");
    (h, id)
}

/// The table `id`.
fn doc<'h>(h: &'h Harness<'_, Moonglow>, id: u32) -> &'h mg_ui::table_view::TableDoc {
    h.state().tables.iter().find(|d| d.id == id).expect("the table open")
}

/// Selects the cell `(row, column)` of table `id` (as Go to does), and
/// lets the window come to rest.
fn select(h: &mut Harness<'_, Moonglow>, id: u32, cell: (usize, usize)) {
    h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap().go_to(cell.0, cell.1);
    settle(h);
}

/// The cell `(row, column)` of table `id` as it is now.
fn cell(h: &Harness<'_, Moonglow>, id: u32, (row, column): (usize, usize)) -> Option<String> {
    doc(h, id).edit.table().cell(row, column).map(str::to_string)
}

/// The grid of table `id` given the keys (as a click on a cell gives
/// them: its editor the active one from the next frame), and the window
/// at rest.
fn grid_keys(h: &mut Harness<'_, Moonglow>, id: u32) {
    mg_ui::table_view::take_keys(&h.ctx, id);
    settle(h);
}

/// A click at `at`, and the window at rest.
fn click_at(h: &mut Harness<'_, Moonglow>, at: egui::Pos2) {
    h.hover_at(at);
    h.step();
    press(h, at, true, egui::Modifiers::NONE);
    h.step();
    press(h, at, false, egui::Modifiers::NONE);
    settle(h);
}

/// Types `text` in the Cell panel's Value, after what it holds (the field
/// clicked, its end), leaving it focused.
fn type_value(h: &mut Harness<'_, Moonglow>, text: &str) {
    let at = cell_value(h).rect().center();
    click_at(h, at);
    h.key_press(egui::Key::End);
    h.event(egui::Event::Text(text.into()));
    settle(h);
}

/// Whether a choice, or an eye, is on.
fn on(n: &egui_kittest::Node<'_>) -> bool {
    let a = n.accesskit_node();
    a.toggled() == Some(egui::accesskit::Toggled::True) || a.is_selected() == Some(true)
}

/// The Outliner's row of the column `name` (its accessible name the
/// column's), in the main window's Outliner or a window's.
fn column_row<'t>(h: &'t Harness<'_, Moonglow>, name: &'t str) -> egui_kittest::Node<'t> {
    let said = format!("{name}, ");
    let is = move |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.label().is_some_and(|l| l == name || l.starts_with(&said))
    };
    let mut rows = h
        .query_all_by(is)
        .filter(|n| in_group(n, mg_ui::OUTLINER))
        .filter(|n| n.accesskit_node().role() != egui::accesskit::Role::Label)
        .filter(|n| n.accesskit_node().bounding_box().is_some());
    rows.next().unwrap_or_else(|| panic!("no Outliner row {name}"))
}

/// Docked, the table's Cell and References are the Right Dock's sections
/// (Cell over References, each under its header saying the cell or the
/// row); floating, its window's right pane's, the main window's Right Dock
/// not drawn for it. The Outliner lists its columns, each with its kind's
/// words, docked in the Left Dock and floating in its window. F6 gives
/// Cell's Value the keyboard.
#[test]
fn the_cell_s_panels_are_the_right_dock_s_or_the_window_s() {
    for mode in MODES {
        let (mut h, id) = table_window(mode, "ui-table-panels", 1920.0);
        select(&mut h, id, (1, MASK));
        h.get_by_label_contains("Row 1, Mask");
        h.get_by_label_contains("Row 1, Cheese");
        h.get_by_label("Find References to Row 1");
        // (The flags' rows, Red and Green ticked.)
        let ticked = |h: &Harness<'_, Moonglow>, name: &str| on(&choice(h, name));
        assert!(ticked(&h, "Red") && ticked(&h, "Green") && !ticked(&h, "Blue"), "{mode:?}");
        h.get_by_label_contains("Mask column: any of 3 flags");
        h.get_by_label_contains("Described by mine.2da.toml: 5 of 7 columns");
        match mode {
            Mode::Tabs => {
                let dock = right_dock_rect(&h);
                assert!(dock.contains(cell_value(&h).rect().center()), "in the Right Dock");
                assert_eq!(h.state().right_dock_shows(), Some(Panel::Cell));
            }
            Mode::Floating => {
                assert!(h.state().right_dock_rect().is_none(), "no Right Dock for it");
            }
        }
        // The Outliner's columns, their kinds' words after them.
        h.get_all_by_label(mg_ui::OUTLINER)
            .find(|o| o.query_by_label_contains("Mask, ").is_some())
            .unwrap_or_else(|| panic!("{mode:?}: the columns listed"));
        for row in ["Mask, any of 3 flags", "Kind, one of 2 values", "Next, a row of mine.2da"] {
            column_row(&h, row);
        }
        // F6 to Cell: its Value has the keyboard.
        let mut found = false;
        for _ in 0..8 {
            h.key_press(egui::Key::F6);
            settle(&mut h);
            if mg_ui::palette_view::arrow_keys_are(&h.ctx) == "Cell" {
                found = true;
                break;
            }
        }
        assert!(found, "{mode:?}: F6 came to Cell");
        settle(&mut h);
        assert!(h.ctx.text_edit_focused(), "{mode:?}: Value took the keyboard");
    }
}

/// The Cell panel edits the cell, by each chooser its column's kind has:
/// Value typed (Enter sets it, one step; Escape lets it go), a flag
/// ticked, a list's value chosen, a row of the table named chosen, a
/// number typed; each the table's step, taken back by the Top Bar's Undo
/// and by Ctrl+Z with the pointer over the panel, and set again by Redo;
/// both ways editors open. Cancel sets back what the panel changed.
#[test]
fn the_cell_panel_edits_the_cell() {
    for mode in MODES {
        let (mut h, id) = table_window(mode, "ui-table-cell", 1920.0);
        // Value: typed, Enter sets it, in one step.
        select(&mut h, id, (0, COST));
        let steps = |h: &Harness<'_, Moonglow>| doc(h, id).edit.changes();
        let before = steps(&h);
        // (The table's own, never copied by the panel's step: a whole copy
        // was made as Enter set it while the panel held the table.)
        let held = |h: &Harness<'_, Moonglow>| std::sync::Arc::as_ptr(&doc(h, id).edit.shared());
        let table_was = held(&h);
        type_value(&mut h, "7");
        assert_eq!(cell(&h, id, (0, COST)).as_deref(), Some("1"), "{mode:?}: not set while typed");
        h.key_press(egui::Key::Enter);
        settle(&mut h);
        assert_eq!(cell(&h, id, (0, COST)).as_deref(), Some("17"), "{mode:?}: set by Enter");
        assert_eq!(steps(&h), before + 1, "{mode:?}: one step");
        assert_eq!(held(&h), table_was, "{mode:?}: the table set in place, not copied");
        assert!(doc(&h, id).typing.is_none(), "{mode:?}: Enter left the grid's cell alone");
        // Escape lets it go.
        type_value(&mut h, "9");
        h.key_press(egui::Key::Escape);
        settle(&mut h);
        assert_eq!(cell(&h, id, (0, COST)).as_deref(), Some("17"), "{mode:?}: Escape");
        assert!(h.state().dock.find_tab(&Tab::Table(id)).is_some(), "{mode:?}: still open");
        // A flag ticked, a list's value chosen, a row chosen.
        select(&mut h, id, (0, MASK));
        choice(&h, "Blue").click();
        settle(&mut h);
        assert_eq!(cell(&h, id, (0, MASK)).as_deref(), Some("0x5"), "{mode:?}: Blue ticked");
        select(&mut h, id, (0, KIND));
        choice(&h, "Large").click();
        settle(&mut h);
        assert_eq!(cell(&h, id, (0, KIND)).as_deref(), Some("1"), "{mode:?}: Large chosen");
        choice(&h, "none").click();
        settle(&mut h);
        assert_eq!(cell(&h, id, (0, KIND)), None, "{mode:?}: none chosen");
        select(&mut h, id, (2, NEXT));
        let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
        let at = h
            .query_all(combo)
            .find(|n| n.accesskit_node().value().is_some_and(|v| v == "none"))
            .map(|n| n.rect().center())
            .expect("the rows' list");
        h.hover_at(at);
        h.step();
        press(&h, at, true, egui::Modifiers::NONE);
        press(&h, at, false, egui::Modifiers::NONE);
        settle(&mut h);
        h.get_by_label_contains("3  Eggs").click();
        settle(&mut h);
        assert_eq!(cell(&h, id, (2, NEXT)).as_deref(), Some("3"), "{mode:?}: row 3 chosen");
        // Undo, by the Top Bar's and by Ctrl+Z over the panel; Redo.
        tool(&h, "Undo").click();
        settle(&mut h);
        assert_eq!(cell(&h, id, (2, NEXT)), None, "{mode:?}: the Top Bar's Undo");
        // (The grid's keys let go of, Escape: the pointer over Cell alone
        // makes Ctrl+Z the table's.)
        h.key_press(egui::Key::Escape);
        settle(&mut h);
        assert_ne!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Table", "{mode:?}");
        let over = cell_value(&h).rect().center() + egui::vec2(0.0, 60.0);
        h.hover_at(over);
        h.step();
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
        settle(&mut h);
        assert_eq!(cell(&h, id, (0, KIND)).as_deref(), Some("1"), "{mode:?}: Ctrl+Z over Cell");
        tool(&h, "Redo").click();
        settle(&mut h);
        assert_eq!(cell(&h, id, (0, KIND)), None, "{mode:?}: Redo");
        // The grid's steps too, as its own Undo and Redo took them: typed
        // in a cell, Ctrl+Z over the grid, the Top Bar's Redo.
        grid_cell(&h, "Cheese").click();
        settle(&mut h);
        h.key_press(egui::Key::F2);
        settle(&mut h);
        h.event(egui::Event::Text("s".into()));
        h.key_press(egui::Key::Enter);
        settle(&mut h);
        assert_eq!(cell(&h, id, (1, LABEL)).as_deref(), Some("Cheeses"), "{mode:?}: typed");
        let over = grid_cell(&h, "Bread").rect().center();
        h.hover_at(over);
        h.step();
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
        settle(&mut h);
        assert_eq!(cell(&h, id, (1, LABEL)).as_deref(), Some("Cheese"), "{mode:?}: Ctrl+Z, grid");
        tool(&h, "Redo").click();
        settle(&mut h);
        assert_eq!(cell(&h, id, (1, LABEL)).as_deref(), Some("Cheeses"), "{mode:?}: Redo, grid");
        // Cancel: what the panel changed set back, the table as saved.
        let mg_ui::table_view::Home::File(file) = doc(&h, id).home.clone() else {
            panic!("a file's table")
        };
        let header = mg_ui::table_view::TABLE_HEADER;
        h.get_by_label(header).get_by_label("Cancel").click();
        settle(&mut h);
        h.get_all_by_label("Don't Save").last().unwrap().click();
        settle(&mut h);
        assert!(h.state().tables.iter().all(|d| d.id != id), "{mode:?}: closed");
        let file = std::fs::read_to_string(file).unwrap();
        assert_eq!(file, MINE, "{mode:?}: nothing of the panel's kept");
    }
}

/// What is typed in Cell's Value keeps to its own cell (M4's U16: the
/// field's id is its cell's): rows numbered anew under it while it is
/// typed (a row above taken away elsewhere, its Undo) carry the text and
/// the selection with its row, Enter setting it there; where its own row
/// goes, it is let go of, never set in the row that took the number. A
/// click elsewhere sets it in its own cell first.
#[test]
fn a_value_typed_keeps_to_its_own_cell() {
    let (mut h, id) = table_window(Mode::Tabs, "ui-table-typed", 1920.0);
    select(&mut h, id, (2, LABEL));
    type_value(&mut h, "x");
    // A row above taken away, as another change made elsewhere would.
    let renumber = |h: &mut Harness<'_, Moonglow>, undo: bool| {
        let d = h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap();
        if undo {
            d.edit.undo();
        } else {
            d.edit.remove_row(0);
        }
        h.step();
        settle(h);
    };
    renumber(&mut h, false);
    assert_eq!(doc(&h, id).selected, Some((1, LABEL)), "the selection with its row");
    assert!(h.ctx.text_edit_focused(), "still typed in");
    renumber(&mut h, true);
    assert_eq!(doc(&h, id).selected, Some((2, LABEL)));
    h.event(egui::Event::Text("y".into()));
    settle(&mut h);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    let t = |h: &Harness<'_, Moonglow>, r| cell(h, id, (r, LABEL));
    assert_eq!((t(&h, 1).as_deref(), t(&h, 2).as_deref()), (Some("Cheese"), Some("Alexy")));
    // Its own row taken away while typed: let go of, not set in the row
    // that took its number.
    select(&mut h, id, (2, LABEL));
    type_value(&mut h, "z");
    let d = h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap();
    d.edit.remove_row(2);
    settle(&mut h);
    assert!(!h.ctx.text_edit_focused(), "let go of");
    assert_eq!(t(&h, 2).as_deref(), Some("Eggs"), "the row that took the number untouched");
    assert_eq!(doc(&h, id).cell_typed_text(), None, "nothing typed kept");
    // A click on another cell: set in its own first.
    select(&mut h, id, (0, LABEL));
    type_value(&mut h, "s");
    let other = column_name(&h, "Cost").rect().center() + egui::vec2(0.0, 60.0);
    click_at(&mut h, other);
    assert_eq!(t(&h, 0).as_deref(), Some("Breads"), "set in its own cell");
    assert_ne!(doc(&h, id).selected, Some((0, LABEL)), "the selection moved on");
}

/// The Outliner's eye hides a column as its menu's Hide Column does: the
/// same columns hidden, the column out of the grid, the selection let go
/// of where it was in it, Show Hidden Columns counting it (in More and on
/// the Outliner's header), the keys passing over it; the eye shows it
/// again. A click on a column's row goes to its cell in the selected row.
#[test]
fn the_eye_hides_a_column_as_hide_column_does() {
    let (mut h, id) = table_window(Mode::Tabs, "ui-table-eye", 1920.0);
    let hidden = |h: &Harness<'_, Moonglow>| doc(h, id).hidden.clone();
    // Hide Column on the column's menu.
    select(&mut h, id, (1, KIND));
    column_name(&h, "Kind").click_secondary();
    settle(&mut h);
    h.get_by_label("Hide Column").click();
    settle(&mut h);
    let by_menu = hidden(&h);
    assert_eq!(doc(&h, id).selected, None, "the menu let go of the selection in it");
    table_click(&mut h, "Show Hidden Columns");
    assert!(hidden(&h).is_empty());
    // The eye: the same.
    select(&mut h, id, (1, KIND));
    let eye = h.get_by_label("Show Kind");
    assert!(on(&eye), "shown");
    eye.click();
    settle(&mut h);
    assert_eq!(hidden(&h), by_menu, "as Hide Column hides it");
    assert_eq!(doc(&h, id).selected, None, "the eye let go of the selection in it");
    assert!(h.query_all_by_label("Kind").all(|n| in_group(&n, mg_ui::OUTLINER)), "out of the grid");
    assert!(!on(&h.get_by_label("Show Kind")), "the eye says hidden");
    let show = h.get_by_label(mg_ui::OUTLINER).get_by_label(mg_ui::table_view::SHOW_HIDDEN);
    assert!(!show.accesskit_node().is_disabled(), "Show Hidden Columns on");
    // The keys pass over it: from Mask, Right goes to Next.
    select(&mut h, id, (1, MASK));
    grid_keys(&mut h, id);
    h.key_press(egui::Key::ArrowRight);
    settle(&mut h);
    assert_eq!(doc(&h, id).selected, Some((1, NEXT)), "past the hidden column");
    // The eye again: shown.
    h.get_by_label("Show Kind").click();
    settle(&mut h);
    assert!(hidden(&h).is_empty(), "shown again");
    // The Outliner's header's eye shows them all.
    h.get_by_label("Show Cost").click();
    h.get_by_label("Show More").click();
    settle(&mut h);
    assert_eq!(hidden(&h).len(), 2);
    let header = h.get_by_label(mg_ui::OUTLINER).get_by_label(mg_ui::table_view::SHOW_HIDDEN);
    header.click();
    settle(&mut h);
    assert!(hidden(&h).is_empty(), "every one shown");
    // A click on Cost's row: its cell in the row selected.
    select(&mut h, id, (3, LABEL));
    column_row(&h, "Cost").click();
    settle(&mut h);
    assert_eq!(doc(&h, id).selected, Some((3, COST)), "gone to the column");
}

/// The Outliner's columns take the keys from a click on a row (and F6):
/// Down, Up, End and Home move along them, Enter goes to the column,
/// letters go to a column by its name, Escape hands them back; beside a
/// table the seam over the Outliner is its own, kept apart from the
/// areas' and the bookmarks'.
#[test]
fn the_outliner_s_columns_take_the_keys() {
    let (mut h, id) = table_window(Mode::Tabs, "ui-table-columns-keys", 1920.0);
    select(&mut h, id, (2, LABEL));
    column_row(&h, "Name").click();
    settle(&mut h);
    assert_eq!(doc(&h, id).selected, Some((2, 1)), "the click went to Name");
    assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Marks", "the list has the keys");
    let key = |h: &mut Harness<'_, Moonglow>, k: egui::Key| {
        h.key_press(k);
        settle(h);
    };
    key(&mut h, egui::Key::ArrowDown);
    key(&mut h, egui::Key::ArrowDown);
    key(&mut h, egui::Key::Enter);
    assert_eq!(doc(&h, id).selected, Some((2, KIND)), "Down twice from Name, Enter");
    key(&mut h, egui::Key::End);
    key(&mut h, egui::Key::Enter);
    assert_eq!(doc(&h, id).selected, Some((2, 6)), "End: More");
    key(&mut h, egui::Key::Home);
    key(&mut h, egui::Key::Enter);
    assert_eq!(doc(&h, id).selected, Some((2, LABEL)), "Home: Label");
    h.event(egui::Event::Text("co".into()));
    settle(&mut h);
    key(&mut h, egui::Key::Enter);
    assert_eq!(doc(&h, id).selected, Some((2, COST)), "typed: Cost");
    key(&mut h, egui::Key::Escape);
    assert_ne!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Marks", "Escape handed them back");
    // F6 stops at them, after the Left Dock's panel, before the grid.
    let mut stops = Vec::new();
    for _ in 0..6 {
        key(&mut h, egui::Key::F6);
        stops.push(mg_ui::palette_view::arrow_keys_are(&h.ctx));
    }
    let at = |k: &str| stops.iter().position(|s| s == k);
    assert!(at("Marks").is_some(), "F6 came to the columns: {stops:?}");
    assert_eq!(
        at("Marks").map(|m| stops.get(m + 1)),
        Some(Some(&"Table".to_string())),
        "{stops:?}"
    );
    // The seam, kept apart.
    let before = h.state().settings.module_panel_height;
    let top = column_row(&h, "Label").rect().top();
    let seam = egui::pos2(60.0, top - mg_uikit::metrics::DOCK_TABS - 5.0);
    drag(&mut h, seam, seam - egui::vec2(0.0, 60.0));
    settle(&mut h);
    let kept = h.state().settings.columns_panel_height.expect("kept beside a table");
    let moved = column_row(&h, "Label").rect().top();
    assert!((top - moved - 60.0).abs() < 8.0, "dragged up 60: {top} to {moved} ({kept})");
    assert_eq!(h.state().settings.module_panel_height, before, "the areas' left alone");
}

/// Find References to Row looks on a thread of its own: "Looking…" while it
/// does (the window's work under way, every frame drawn), stopped as
/// another row is chosen; then what it found: the module's creature and
/// script that name the row, each going there; for a row nothing names, it
/// says so.
#[test]
fn references_finds_the_row_s_users() {
    let dir = mg_testkit::scratch_dir("ui-table-references");
    let path = sample_module(&dir);
    let mut app = app_in(Mode::Tabs, Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let table = ResKey::parse("mg_looks", ResType::TWODA).unwrap();
    let script = ResKey::parse("mg_look", ResType::NSS).unwrap();
    let src = b"void main()\n{\n    string s = Get2DAString(\"mg_looks\", \"Label\", 2);\n}\n";
    app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A table and a script",
        vec![
            mg_edit::Edit::SetResource {
                key: table,
                data: Some(b"2DA V2.0\n\n   Label\n0  a\n1  b\n2  c\n".to_vec()),
            },
            mg_edit::Edit::SetResource { key: script, data: Some(src.to_vec()) },
        ],
    )));
    let mut h = harness(app, 1920.0, 900.0);
    settle(&mut h);
    let id = mg_ui::table_view::open_in_module(h.state_mut(), table).unwrap();
    settle(&mut h);
    select(&mut h, id, (2, 0));
    // Held at its start: looking, the window's work under way.
    let hold = h.state().hold_reference_looks.clone();
    hold.store(true, std::sync::atomic::Ordering::Relaxed);
    h.get_by_label("Find References to Row 2").click();
    h.run_steps(3);
    assert!(doc(&h, id).looking(), "under way");
    assert!(h.state().under_way().is_some());
    h.get_by_label_contains("Looking through the game's tables and the module");
    // Another row chosen: stopped, nothing shown for either.
    select(&mut h, id, (1, 0));
    assert!(!doc(&h, id).looking(), "stopped as another row was chosen");
    assert!(h.query_by_label_contains("Looking through").is_none());
    hold.store(false, std::sync::atomic::Ordering::Relaxed);
    // Nothing names row 1.
    h.get_by_label("Find References to Row 1").click();
    settle(&mut h);
    h.get_by_label_contains("Nothing names row 1");
    // Row 2: the script.
    select(&mut h, id, (2, 0));
    assert!(h.query_by_label_contains("Nothing names").is_none(), "its own row's alone");
    h.get_by_label("Find References to Row 2").click();
    settle(&mut h);
    h.get_by_label("In the module (1)");
    h.get_by_label_contains("mg_look line 3: Get2DAString").click();
    settle(&mut h);
    assert!(h.state().dock.find_tab(&Tab::Script(script)).is_some(), "the script opened");
}

/// What every panel of a table shows fits the Right Dock from its least
/// width: at 280, 340, 440 and 600 points, with each column's kind's
/// chooser shown (a list's and flags' rows, a row's list, a string's
/// Choose String… and New Line…, a number's words), nothing in sight past
/// the dock's edge and every one-line field as wide as its text; the
/// panel scrolled to its foot too.
#[test]
fn every_table_panel_fits_the_right_dock() {
    let (mut h, id) = table_window(Mode::Tabs, "ui-table-fit", 1600.0);
    for width in [280.0, 340.0, 440.0, 600.0] {
        right_dock_width(&mut h, width);
        settle(&mut h);
        for (column, own) in [
            (MASK, "Blue"),
            (KIND, "Large"),
            (NEXT, "Next column: a row of mine.2da"),
            (1, "Choose String…"),
            (COST, "Cost column: a number from 0 to 9"),
            (6, "More column"),
        ] {
            select(&mut h, id, (1, column));
            let dock = right_dock_rect(&h);
            let what = format!("{width} {column}");
            assert!((dock.width() - width - 1.0).abs() < 2.0, "{what}: {dock:?}");
            let drawn = h.query_all_by_label(own).any(|n| dock.contains(n.rect().center()));
            assert!(drawn, "{what}: {own}");
            fits(&h, dock, &what);
            if column == 1 {
                for open in ["Choose String…", "New Line…"] {
                    if !h.get_by_label(open).accesskit_node().is_disabled() {
                        h.get_by_label(open).click();
                        settle(&mut h);
                        fits(&h, dock, &format!("{what} {open}"));
                    }
                }
            }
            // (Scrolled to its foot.)
            h.get_by_label_contains("Described by").scroll_to_me();
            settle(&mut h);
            fits(&h, dock, &format!("{what} at its foot"));
        }
    }
}

/// What a table's panel shows in `dock` fits it: nothing in sight past its
/// edge, every one-line field in sight as wide as its text.
#[track_caller]
fn fits(h: &Harness<'_, Moonglow>, dock: egui::Rect, what: &str) {
    fits_dock(h, dock, what, egui::FontId::monospace(mg_uikit::metrics::BODY_SIZE), false);
}

/// The Cell panel's content's height, as drawn: from its Value to the foot
/// of its "Described by" note.
fn cell_height(h: &Harness<'_, Moonglow>) -> f32 {
    let top = cell_value(h).rect().top();
    let foot = h.get_by_label_contains("Described by").rect().bottom();
    foot - top
}

/// The Cell panel is never taller where wider, and comes to rest: the
/// Right Dock swept from 270 to 480 points a point at a time beside each
/// column's kind, its height never growing with its width; around each
/// width where its layout changes, settled at window heights a few points
/// apart (a scrolled panel taller where wider never comes to rest: U14's
/// fault, found in U15).
#[test]
fn the_cell_panel_is_never_taller_where_wider_and_comes_to_rest() {
    let (mut h, id) = table_window(Mode::Tabs, "ui-table-sweep", 1600.0);
    never_taller(&mut h, id, &[(1, MASK), (1, KIND), (1, NEXT), (1, 1), (1, COST), (1, 6)]);
}

/// [`the_cell_panel_is_never_taller_where_wider_and_comes_to_rest`]'s
/// sweep beside each of `cells` of table `id`.
#[track_caller]
fn never_taller(h: &mut Harness<'_, Moonglow>, id: u32, cells: &[(usize, usize)]) {
    for &cell in cells {
        h.state_mut().settings.right_dock_width = None;
        h.set_size(egui::vec2(1600.0, 900.0));
        select(h, id, cell);
        sweep_dock_heights(h, &format!("{cell:?}"), cell_height, true);
    }
}

/// The same sweep beside every kind a description of ours can give a
/// column that [`MINE`] has not: text said so, a float, a script, a list
/// open to values of the builder's own, and a list of 300 values (its
/// frame a few rows high); no game data.
#[test]
fn the_cell_panel_is_never_taller_beside_every_kind_we_describe() {
    let dir = mg_testkit::scratch_dir("ui-table-sweep-kinds");
    std::fs::write(dir.join("nasher.cfg"), "").unwrap();
    std::fs::write(
        dir.join("kinds.2da"),
        "2DA V2.0\r\n\r\n   Words  Ratio  Runs        Mood  Many\r\n\
         0  Some   0.5    my_script   7     12\r\n\
         1  More   1.25   ****        1     299\r\n",
    )
    .unwrap();
    let many: Vec<String> = (0..300).map(|n| format!("V{n:03} = {n}")).collect();
    let toml = format!(
        "[columns.Words]\nkind = \"text\"\n[columns.Ratio]\nkind = \"float\"\nmin = 0.0\n\
         [columns.Runs]\nkind = \"script\"\n\
         [columns.Mood]\nkind = \"list\"\nother = true\nvalues = {{ Calm = 0, Cross = 1 }}\n\
         [columns.Many]\nkind = \"list\"\nvalues = {{ {} }}\n",
        many.join(", ")
    );
    std::fs::write(dir.join("kinds.2da.toml"), toml).unwrap();
    let mut h = harness(app_in(Mode::Tabs, Vec::new()), 1600.0, 900.0);
    settle(&mut h);
    let id = mg_ui::table_view::open_path(h.state_mut(), &dir.join("kinds.2da")).unwrap();
    settle(&mut h);
    assert!(doc(&h, id).description().is_some(), "described");
    // (Mood's 7 is a value of the builder's own; row 1's Runs is empty.)
    let cells = [(0, 0), (0, 1), (0, 2), (1, 2), (0, 3), (1, 3), (0, 4), (1, 4)];
    never_taller(&mut h, id, &cells);
}

/// The same sweep beside the kinds the game's own tables give (copies of
/// them, described by a file of ours so that the panel says so at its
/// foot, and `SpellDesc` a string by it): spells.2da's longest talk-table
/// text among its first 400 rows (`SpellDesc`, a string of several
/// paragraphs), its `Name` and its `ImpactScript` (the game's own kinds);
/// appearance.2da's `RACE`, a model's name.
#[test]
fn the_cell_panel_is_never_taller_beside_the_game_s_kinds() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-table-sweep-game");
    std::fs::write(dir.join("nasher.cfg"), "").unwrap();
    let app = game_app(&root, None);
    let game = app.game.clone().unwrap();
    for name in ["spells", "appearance"] {
        let data = game.resman.get_named(name, ResType::TWODA).unwrap().into_owned();
        std::fs::write(dir.join(format!("{name}.2da")), data).unwrap();
        // (SpellDesc said a string by ours: the game's own knows Name.)
        let toml = "[columns.Label]\nkind = \"text\"\n[columns.SpellDesc]\nkind = \"strref\"\n";
        std::fs::write(dir.join(format!("{name}.2da.toml")), toml).unwrap();
    }
    let spells = game.table("spells").unwrap();
    let text = |r: usize| {
        let n: u32 = spells.get(r, "SpellDesc").and_then(|v| v.parse().ok())?;
        Some(game.tlk().get(mg_core::StrRef(n))?.text.len())
    };
    let long = (0..spells.len().min(400)).max_by_key(|r| text(*r).unwrap_or(0)).unwrap();
    assert!(text(long).unwrap_or(0) > 600, "a long string: {:?}", text(long));
    let column = |t: &mg_2da::TwoDa, c: &str| t.column(c).unwrap();
    let mut app = app;
    app.settings = Mode::Tabs.settings();
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    let id = mg_ui::table_view::open_path(h.state_mut(), &dir.join("spells.2da")).unwrap();
    settle(&mut h);
    // (The string's text is Cell's: its first words drawn.)
    select(&mut h, id, (long, column(&spells, "SpellDesc")));
    let n: u32 = spells.get(long, "SpellDesc").unwrap().parse().unwrap();
    let text =
        String::from_utf8_lossy(&game.tlk().get(mg_core::StrRef(n)).unwrap().text).to_string();
    let words: String = text.chars().take(20).collect();
    assert!(h.query_all_by_label_contains(&words).next().is_some(), "Cell shows {words:?}");
    let cells = [
        (long, column(&spells, "SpellDesc")),
        (long, column(&spells, "Name")),
        (long, column(&spells, "ImpactScript")),
    ];
    never_taller(&mut h, id, &cells);
    let appearance = game.table("appearance").unwrap();
    let id = mg_ui::table_view::open_path(h.state_mut(), &dir.join("appearance.2da")).unwrap();
    settle(&mut h);
    never_taller(&mut h, id, &[(6, column(&appearance, "RACE"))]);
}

/// The window comes to rest with each of the table's panels in front,
/// behind and away, docked and floating: Cell, References (in front of
/// Cell in the one bar under 1400), the Outliner's columns; the Right
/// Dock folded, the Left Dock closed; floating, its window's panes. Each
/// case types in the grid and in Cell, moves the selection with the keys,
/// hides a column with the eye (where the Outliner is drawn), runs
/// References, undoes, and deletes the last row with the selection in it
/// (none left); each settles (an endless redraw fails as "keeps
/// repainting"), and the last shows something changed.
#[test]
fn the_window_comes_to_rest_with_each_table_panel() {
    let cases: [(Mode, &str, f32); 6] = [
        (Mode::Tabs, "Cell", 1600.0),
        (Mode::Tabs, "References", 1300.0),
        (Mode::Tabs, "folded", 1600.0),
        (Mode::Tabs, "closed", 1600.0),
        (Mode::Floating, "pane", 1920.0),
        (Mode::Floating, "strip", 1600.0),
    ];
    for (mode, case, width) in cases {
        let (mut h, id) = table_window(mode, "ui-table-rest", width);
        let what = format!("{mode:?} {case}");
        match case {
            "References" => {
                let tab = h
                    .get_all_by_label("References")
                    .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Tab);
                tab.expect("References' tab").click();
                settle(&mut h);
            }
            "folded" => {
                h.state_mut().settings.hide_palettes = true;
                settle(&mut h);
            }
            "closed" => {
                h.state_mut().settings.hide_tree = true;
                settle(&mut h);
            }
            _ => {}
        }
        select(&mut h, id, (1, LABEL));
        // Typed in the grid: F2, a letter, Enter.
        grid_keys(&mut h, id);
        h.key_press(egui::Key::F2);
        settle(&mut h);
        h.event(egui::Event::Text("q".into()));
        settle(&mut h);
        h.key_press(egui::Key::Enter);
        settle(&mut h);
        assert_eq!(cell(&h, id, (1, LABEL)).as_deref(), Some("Cheeseq"), "{what}: the grid");
        // Typed in Cell, where it is drawn.
        if h.query_all_by_label("Value")
            .any(|n| n.accesskit_node().role() == egui::accesskit::Role::TextInput)
        {
            select(&mut h, id, (0, LABEL));
            type_value(&mut h, "w");
            h.key_press(egui::Key::Enter);
            settle(&mut h);
            assert_eq!(cell(&h, id, (0, LABEL)).as_deref(), Some("Breadw"), "{what}: Cell");
        }
        // The keys.
        select(&mut h, id, (0, LABEL));
        grid_keys(&mut h, id);
        for k in [egui::Key::ArrowDown, egui::Key::ArrowRight, egui::Key::ArrowDown] {
            h.key_press(k);
            settle(&mut h);
        }
        assert_eq!(doc(&h, id).selected, Some((2, 1)), "{what}: the keys moved it");
        // The eye, where the columns are listed.
        if let Some(eye) = h.query_by_label("Show More") {
            eye.click();
            settle(&mut h);
            assert_eq!(doc(&h, id).hidden.len(), 1, "{what}: hidden");
        }
        // References, where drawn.
        if let Some(find) = h.query_by_label("Find References to Row 2") {
            find.click();
            settle(&mut h);
        }
        tool(&h, "Undo").click();
        settle(&mut h);
        // The last row deleted with the selection in it: none left.
        select(&mut h, id, (3, LABEL));
        table_click(&mut h, "Delete Row");
        settle(&mut h);
        assert_eq!(doc(&h, id).edit.table().len(), 3, "{what}: deleted");
        let d = h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap();
        d.edit.remove_row(2);
        d.selected = None;
        h.step();
        settle(&mut h);
        assert!(doc(&h, id).selected.is_none(), "{what}: no selection");
    }
}

/// The Status Bar beside a table (the Table board's): "6 cells" for six
/// cells chosen (nothing for one), its rows, columns and where it is kept
/// ("4 rows, 7 columns, in a file"; in the module, in a hak), and the
/// grid's keys while it has them: Enter Type, Shift + Click Select to,
/// Delete Empty, Ctrl + C Copy, Ctrl + V Paste, Tab Next column, Esc Leave
/// grid, each doing what it says; typing in a cell, Shift + Click alone;
/// a menu open, none, the counts kept. A table only looked at (the game's
/// copy in the resource tab) says whose it is and none of the keys that
/// change it.
#[test]
fn a_table_s_hints_are_as_bound() {
    let (mut h, id) = table_window(Mode::Tabs, "ui-table-hints", 1920.0);
    let all = [
        "Enter: Type",
        "Shift, Click: Select to",
        "Delete: Empty",
        "Ctrl, C: Copy",
        "Ctrl, V: Paste",
        "Tab: Next column",
        "Esc: Leave grid",
    ];
    select(&mut h, id, (0, LABEL));
    grid_keys(&mut h, id);
    settle(&mut h);
    says(&mut h, &all, &["6 cells"]);
    h.key_press(egui::Key::Tab);
    settle(&mut h);
    assert_eq!(doc(&h, id).selected, Some((0, 1)), "Tab: Next column");
    select(&mut h, id, (0, LABEL));
    let mut said = all.to_vec();
    said.push("4 rows, 7 columns, in a file");
    assert_eq!(h.state().status_said(), said, "in the board's order, nothing else");
    // Six cells chosen: the count.
    let d = h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap();
    d.select_to((2, 1));
    settle(&mut h);
    says(&mut h, &["6 cells", "4 rows, 7 columns, in a file"], &[]);
    // Each does what it says: Delete empties them, Ctrl+C copies, Tab goes
    // to the next column, Esc leaves the grid, Enter types.
    h.key_press(egui::Key::Delete);
    settle(&mut h);
    assert_eq!((cell(&h, id, (0, 0)), cell(&h, id, (2, 1))), (None, None), "Delete: Empty");
    tool(&h, "Undo").click();
    settle(&mut h);
    // (A click on Bread's cell: the grid has the keys, not the Top Bar's
    // Undo, which the click gave the focus.)
    grid_cell(&h, "Bread").click();
    settle(&mut h);
    h.event(egui::Event::Copy);
    h.step();
    let copied = h
        .output()
        .platform_output
        .commands
        .iter()
        .any(|c| matches!(c, egui::OutputCommand::CopyText(t) if t == "Bread"));
    assert!(copied, "Ctrl + C: Copy");
    settle(&mut h);
    h.event(egui::Event::Paste("Rye".into()));
    settle(&mut h);
    assert_eq!(cell(&h, id, (0, LABEL)).as_deref(), Some("Rye"), "Ctrl + V: Paste");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert!(doc(&h, id).typing.is_some(), "Enter: Type");
    // Typing: Shift + Click alone.
    says(
        &mut h,
        &["Shift, Click: Select to"],
        &["Enter: Type", "Delete: Empty", "Esc: Leave grid"],
    );
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert_ne!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Table", "Esc: Leave grid");
    says(&mut h, &["Shift, Click: Select to"], &["Enter: Type", "Ctrl, C: Copy"]);
    // In the module: says so.
    let key = ResKey::parse("mine", ResType::TWODA).unwrap();
    let data = doc(&h, id).edit.table().to_bytes(mg_core::Codepage::default()).unwrap();
    h.state_mut().ws.as_mut().unwrap().module.set(key, data);
    let own = mg_ui::table_view::open_in_module(h.state_mut(), key).unwrap();
    settle(&mut h);
    let rows = format!("{} rows, 7 columns, in the module", doc(&h, own).edit.table().len());
    says(&mut h, &[rows.as_str()], &[]);
    // Tab is the grid's only where it is drawn: the first table, its grid
    // given the keys, behind the second: Tab moves nothing of it, and goes
    // to the controls as egui gives it.
    select(&mut h, id, (0, LABEL));
    grid_keys(&mut h, id);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Table(own)));
    settle(&mut h);
    h.key_press(egui::Key::Tab);
    settle(&mut h);
    assert_eq!(doc(&h, id).selected, Some((0, LABEL)), "a grid out of sight keeps its cell");
    // (Nor is the key kept from the controls: it gave one the focus.)
    assert!(h.ctx.memory(|m| m.focused().is_some()), "Tab went to the controls");
}

/// Find and Go to, on the header: Ctrl+F with the pointer over the table
/// goes to Find, whose text narrows the rows; Go to with a row's number
/// and Enter goes to that row, nothing typed in its cell.
#[test]
fn find_and_go_to_from_the_header() {
    let (mut h, id) = table_window(Mode::Tabs, "ui-table-find", 1920.0);
    let over = grid_cell(&h, "Cheese").rect().center();
    h.hover_at(over);
    h.step();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::F);
    settle(&mut h);
    let find = h.get_by_label(mg_ui::table_view::TABLE_HEADER).get_by_label("Find");
    assert!(find.accesskit_node().is_focused(), "Ctrl+F went to Find");
    h.event(egui::Event::Text("ale".into()));
    settle(&mut h);
    assert_eq!(doc(&h, id).filter, "ale");
    assert!(h.query_by_label("Cheese").is_none() && h.query_by_label("Ale").is_some(), "narrowed");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(doc(&h, id).filter.is_empty(), "Escape cleared it");
    let go = h.get_by_label(mg_ui::table_view::TABLE_HEADER).get_by_label("Go to").rect();
    click_at(&mut h, go.center());
    h.event(egui::Event::Text("3".into()));
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(doc(&h, id).selected.map(|s| s.0), Some(3), "Go to went to row 3");
    assert!(doc(&h, id).typing.is_none(), "Enter typed nothing in the cell");
}

/// A table only looked at (the game's `spells.2da` in the resource tab):
/// its rows and columns and whose copy it is, read-only, and the keys it
/// has (Shift + Click; with its grid's keys Ctrl + C, Tab, Esc), none of
/// those that change a table.
#[test]
fn a_table_only_looked_at_says_whose_it_is() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let mut app = game_app(&root, None);
    app.settings = Mode::Tabs.settings();
    let mut h = harness(app, 1920.0, 900.0);
    settle(&mut h);
    let key = ResKey::parse("spells", ResType::TWODA).unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(key)));
    settle(&mut h);
    let said = h.state().status_said();
    let counts = said.iter().find(|s| s.contains("rows,")).expect("its counts");
    assert!(counts.contains("from key:nwn_retail, read-only"), "{said:?}");
    assert!(said.iter().any(|s| s == "Shift, Click: Select to"), "{said:?}");
    for not in ["Enter: Type", "Delete: Empty", "Ctrl, V: Paste"] {
        assert!(!said.iter().any(|s| s == not), "{not} beside a table only looked at");
    }
    // Its Tab: the next column, as its hint says.
    let game = h.state().game.clone().unwrap();
    let t = game.table("spells").unwrap();
    let first = t.get(0, "Label").unwrap().to_string();
    let name = t.get(0, "Name").map(str::to_string).unwrap_or_default();
    h.get_by_label(&first).click();
    settle(&mut h);
    h.get_by_label_contains(&format!("Row 0, Label: {first}"));
    // With its grid's keys: what it can do, none of what changes a table.
    says(
        &mut h,
        &["Ctrl, C: Copy", "Tab: Next column", "Esc: Leave grid"],
        &["Enter: Type", "Delete: Empty", "Ctrl, V: Paste"],
    );
    h.key_press(egui::Key::Tab);
    settle(&mut h);
    h.get_by_label_contains(&format!("Row 0, Name: {name}"));
    // Its keys are the active editor's: another view, a blueprint's, in a
    // group of its own, pressed: Tab and the arrows move nothing of
    // spells'.
    let feat = ResKey::parse("nw_it_torch001", ResType::UTI).unwrap();
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(feat)));
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::SplitEditor);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Resource(key)));
    settle(&mut h);
    let other = pane_of(&h, &Tab::Resource(feat));
    click_at(&mut h, other.center_bottom() - egui::vec2(0.0, 20.0));
    assert_eq!(h.state().active_editor(), Some(&Tab::Resource(feat)), "the blueprint pressed");
    for k in [egui::Key::Tab, egui::Key::ArrowRight, egui::Key::ArrowDown] {
        h.key_press(k);
        settle(&mut h);
    }
    h.get_by_label_contains(&format!("Row 0, Name: {name}"));
}

/// [`h`]'s Status Bar comes to say each of `what` (drawn) and none of
/// The panels follow the table in front: two tables in two groups, each
/// one's Cell shown as its group has the focus; a floating table's in its
/// window, the docked one's dock left as it was.
#[test]
fn the_panels_follow_the_table_in_front() {
    let (mut h, first) = table_window(Mode::Tabs, "ui-table-follow", 1920.0);
    let dir = mg_testkit::scratch_dir("ui-table-follow-second");
    std::fs::write(dir.join("other.2da"), MINE).unwrap();
    let second = mg_ui::table_view::open_path(h.state_mut(), &dir.join("other.2da")).unwrap();
    settle(&mut h);
    // (The second beside the first, a group of its own.)
    h.state_mut().actions.push(mg_ui::Action::SplitEditor);
    settle(&mut h);
    select(&mut h, first, (1, MASK));
    select(&mut h, second, (3, COST));
    let detail = |h: &Harness<'_, Moonglow>| {
        let shown = |d: &&str| h.query_by_label_contains(d).is_some();
        let both: Vec<&str> = ["Row 1, Mask", "Row 3, Cost"].into_iter().filter(shown).collect();
        (both.len() == 1).then(|| both[0])
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Table(first)));
    settle(&mut h);
    assert_eq!(detail(&h), Some("Row 1, Mask"), "the first's");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Table(second)));
    settle(&mut h);
    assert_eq!(detail(&h), Some("Row 3, Cost"), "the second's");
}

/// Records from before the table's cell was the Right Dock's read without
/// a fault: a layout kept for tables naming a newer one's panel, or none
/// of these, as its fronts; an open-tabs view of a table with its cell
/// (and the old inspector's height, never kept, among its fields). Each
/// passes over what it does not know: Cell in front, the cell chosen.
#[test]
fn records_naming_the_old_inspector_read_without_error() {
    for fronts in ["[\"Inspector\",\"Cell\"]", "[\"Toolbar\"]", "[]"] {
        let dir = mg_testkit::scratch_dir("ui-table-old-records");
        let path = sample_module(&dir);
        let key = ResKey::parse("mine", ResType::TWODA).unwrap();
        let mut m = Module::open(&path).unwrap();
        m.set(key, MINE.as_bytes().to_vec());
        m.save().unwrap();
        let tabs = dir.join("open-tabs");
        std::fs::create_dir_all(&tabs).unwrap();
        let record = r#"{"panes":[{"place":"middle","tabs":["table:mine.2da"]}],
            "front":"table:mine.2da",
            "views":{"table:mine.2da":{"cell":[2,3],"inspector":96,"toolbar":"rows"}}}"#;
        std::fs::write(mg_ui::open_tabs::file(&tabs, &path), record).unwrap();
        let text = format!(
            "(dock_layouts:{{\"table\":(fronts:{fronts},shares:[1200,800])}},\
             table_inspector_height:Some(120))"
        );
        let (settings, _unread) = mg_ui::Settings::read_fields(&text).expect("read");
        let mut app = app_in(Mode::Tabs, Vec::new());
        app.settings.dock_layouts = settings.dock_layouts;
        app.settings.no_last_area = true;
        app.open_tabs_dir = Some(tabs);
        app.open_module(&path);
        let mut h = harness(app, 1600.0, 900.0);
        settle(&mut h);
        let id = match h.state().active_editor() {
            Some(Tab::Table(id)) => *id,
            other => panic!("{fronts}: {other:?}"),
        };
        assert_eq!(doc(&h, id).selected, Some((2, 3)), "{fronts}: the cell kept");
        assert_eq!(h.state().right_dock_shows(), Some(Panel::Cell), "{fronts}");
        h.get_by_label_contains("Row 2, Kind");
    }
}

/// The table's header's controls, in their order.
const TABLE_CONTROLS: [&str; 15] = [
    "Save",
    "Add Row",
    "Empty Row",
    "Delete Row",
    "Add Column…",
    "Find",
    "Go to",
    "How cells show",
    "Values as names",
    "Strings as text",
    "Save As File…",
    "Take Rows…",
    "Compact",
    "Show Hidden Columns",
    "Reset All Widths",
];

/// The table header holds every control (on its row, as an icon, or in
/// More) at every window width from 420 to 1600 points, both ways editors
/// open, OK and Cancel at its end; Find and Go to stay on the row longest,
/// with Save; at the least width Strings as text and Take Rows… work from
/// More, and Cell stays in reach (the Right Dock, its strip where the
/// window folds it, or the window's pane).
#[test]
fn the_table_header_holds_every_control_from_420_to_1600() {
    use mg_uikit::header::{self, Place};
    let header_name = mg_ui::table_view::TABLE_HEADER;
    for mode in MODES {
        let (mut h, id) = table_window(mode, "ui-table-header-widths", 1600.0);
        select(&mut h, id, (1, KIND));
        // (Downward, so that a floating table's window narrows with the
        // screen: one made at 420 first would stay so.)
        let mut widths: Vec<f32> = (21..=80).rev().map(|n| n as f32 * 20.0).collect();
        widths.extend([1399.0, 1400.0, 961.0, 962.0]);
        let mut top = true;
        for width in widths {
            h.set_size(egui::vec2(width, 800.0));
            settle(&mut h);
            let rec = header::records(&h.ctx, header_name);
            let rec = rec.into_iter().next().unwrap_or_else(|| panic!("no header at {width}"));
            let names: Vec<&str> = rec.places.iter().map(|(n, _)| n.as_str()).collect();
            assert_eq!(names, TABLE_CONTROLS, "{mode:?} {width}");
            // (At the widest, the first commands on the row, both ways.)
            if std::mem::take(&mut top) {
                for n in ["Save", "Find", "Go to"] {
                    let place = rec.places.iter().find(|(m, _)| m == n).map(|(_, p)| *p);
                    let shown = matches!(place, Some(Place::Row | Place::Icon));
                    assert!(shown, "{mode:?} {width}: {n} on the row ({place:?})");
                }
            }
            let end = rec.end.expect("the end slot");
            for n in ["OK", "Cancel"] {
                let r = h.get_by_label(header_name).get_by_label(n).rect();
                assert!(end.expand(0.5).contains_rect(r), "{mode:?} {width}: {n} in {end:?}");
            }
            // (The group's toggles by their own names.)
            let held: Vec<&str> =
                names.iter().copied().filter(|n| *n != "How cells show").collect();
            header_holds(&mut h, header_name, &held);
            // (Find and Go to into More only after the rest, Save last.)
            let place = |n: &str| rec.places.iter().find(|(m, _)| m == n).map(|(_, p)| *p);
            for kept in ["Find", "Go to"] {
                if place(kept) == Some(Place::More) {
                    for n in ["Add Row", "Empty Row", "Delete Row", "Add Column…", "How cells show"]
                    {
                        let gone = place(n) == Some(Place::More);
                        assert!(gone, "{mode:?} {width}: {n} into More before {kept}");
                    }
                }
            }
            // Cell in reach.
            let reach = h.query_by_label_contains("Row 1, Kind").is_some()
                || h.query_all_by_label("Cell").next().is_some();
            assert!(reach, "{mode:?} {width}: Cell out of reach");
        }
        // At the least: Strings as text and Take Rows… from More.
        h.set_size(egui::vec2(420.0, 800.0));
        settle(&mut h);
        let was = doc(&h, id).strref_text;
        table_click(&mut h, "Strings as text");
        assert_ne!(doc(&h, id).strref_text, was, "{mode:?}: Strings as text from More");
        table_click(&mut h, "Take Rows…");
    }
}

/// A floating table whose window has not the room for its Outliner's pane
/// beside the grid's room (a screen of 1300: since M4's U17 review a
/// window's first size is clipped to the main window, and in one of 1600
/// it has the room): its columns fold to a strip
/// at the grid's left, whose icon unfolds them over the grid, a click on a
/// column there going to it, and folds them again.
#[test]
fn a_floating_table_s_columns_fold_to_a_strip() {
    let (mut h, id) = table_window(Mode::Floating, "ui-table-strip", 1300.0);
    select(&mut h, id, (1, LABEL));
    assert!(h.query_all_by_label_contains("Kind, ").next().is_none(), "no pane");
    let strip = h.get_by_label(mg_ui::dialog_view::OUTLINER_STRIP);
    strip.get_by_label(mg_ui::OUTLINER).click();
    settle(&mut h);
    column_row(&h, "Kind").click();
    settle(&mut h);
    assert_eq!(doc(&h, id).selected, Some((1, KIND)), "gone to Kind");
    let strip = h.get_by_label(mg_ui::dialog_view::OUTLINER_STRIP);
    strip.get_by_label(mg_ui::OUTLINER).click();
    settle(&mut h);
    assert!(h.query_all_by_label_contains("Kind, ").next().is_none(), "folded again");
}

/// A model's chooser (with the game's data: the cell's model found or not,
/// and Choose Model… open with its list) fits the Right Dock from its
/// least width, as the other kinds' do.
#[test]
fn the_model_chooser_fits_the_right_dock() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-table-model-fit");
    let file = dir.join("placeables.2da");
    std::fs::write(
        &file,
        "2DA V2.0\n\n   Label   ModelName\n0  Chest   plc_c01\n1  Nothing mg_none\n",
    )
    .unwrap();
    let mut app = game_app(&root, None);
    app.settings = Mode::Tabs.settings();
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    let id = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    settle(&mut h);
    for width in [280.0, 340.0, 440.0, 600.0] {
        right_dock_width(&mut h, width);
        for row in [0, 1] {
            select(&mut h, id, (row, 1));
            let dock = right_dock_rect(&h);
            fits(&h, dock, &format!("{width} row {row}"));
        }
        // (Opened once, the models found by "plc_c0": it stays open, and
        // keeps what was typed, from one width to the next.)
        if !on(&h.get_by_label("Choose Model…")) {
            h.get_by_label("Choose Model…").click();
            settle(&mut h);
            type_into_hint(&mut h, "Part of a model's name", "plc_c0");
            settle(&mut h);
        }
        h.get_by_label("Choose Model…").scroll_to_me();
        settle(&mut h);
        let dock = right_dock_rect(&h);
        assert!(h.query_by_label("plc_c02").is_some(), "{width}: the list");
        fits(&h, dock, &format!("{width} Choose Model…"));
    }
}

/// Choose Model… lays out only the models in sight of its 200 (all 200
/// were laid out each frame it was open), and one scrolled to at the
/// list's end is chosen with a click: the cell is set to it.
#[test]
fn the_model_chooser_lays_out_the_models_in_sight() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-table-model-rows");
    let file = dir.join("placeables.2da");
    std::fs::write(&file, "2DA V2.0\n\n   Label   ModelName\n0  Chest   plc_c01\n").unwrap();
    let mut app = game_app(&root, None);
    app.settings = Mode::Tabs.settings();
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    let id = mg_ui::table_view::open_path(h.state_mut(), &file).unwrap();
    settle(&mut h);
    select(&mut h, id, (0, 1));
    h.get_by_label("Choose Model…").click();
    settle(&mut h);
    // (The 200 first of the game's models by name, as the list has them.)
    let mut names: Vec<String> = (h.state().game.as_ref().unwrap().resman.layers().iter())
        .flat_map(|l| l.container.keys().filter(|k| k.restype == ResType::MDL).map(|k| k.resref))
        .map(|r| r.to_string())
        .collect();
    names.sort();
    names.dedup();
    names.truncate(200);
    assert_eq!(names.len(), 200, "the install has hundreds of models");
    let (first, last) = (&names[0], &names[199]);
    assert!(h.query_by_label(first).is_some(), "the first in sight");
    assert!(h.query_by_label(last).is_none(), "the last out of sight, not laid out");
    let shown = names.iter().filter(|n| h.query_by_label(n.as_str()).is_some()).count();
    assert!((5..60).contains(&shown), "the rows in sight alone: {shown}");
    // Scrolled to its end: the last, chosen.
    let at = h.get_by_label(first).rect().center();
    h.hover_at(at);
    for _ in 0..40 {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -400.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        h.run();
        if h.query_by_label(last).is_some() {
            break;
        }
    }
    settle(&mut h);
    h.get_by_label(last).click();
    settle(&mut h);
    assert_eq!(cell(&h, id, (0, 1)).as_deref(), Some(last.as_str()));
}

/// The leaf `tab` is drawn in, its rectangle (its group's or its window's).
fn pane_of(h: &Harness<'_, Moonglow>, tab: &Tab) -> egui::Rect {
    let at = h.state().dock.find_tab(tab).expect("its tab");
    let path = egui_dock::NodePath { surface: at.surface, node: at.node };
    h.state().dock.leaf(path).unwrap().rect
}

/// Presses the keys a grid takes (Tab, Shift+Tab, Down, Delete) and
/// checks they did nothing to table `id`: its selection and cells as they
/// were, none of the grid's hints said.
#[track_caller]
fn keys_do_nothing(h: &mut Harness<'_, Moonglow>, id: u32, what: &str) {
    let (before, cells) = (doc(h, id).selected, doc(h, id).edit.table().clone());
    for (m, k) in [
        (egui::Modifiers::NONE, egui::Key::Tab),
        (egui::Modifiers::SHIFT, egui::Key::Tab),
        (egui::Modifiers::NONE, egui::Key::ArrowDown),
        (egui::Modifiers::NONE, egui::Key::Delete),
    ] {
        h.key_press_modifiers(m, k);
        settle(h);
    }
    assert_eq!(doc(h, id).selected, before, "{what}: the selection as it was");
    assert_eq!(doc(h, id).edit.table().rows, cells.rows, "{what}: the cells as they were");
    let said = h.state().status_said();
    assert!(!said.iter().any(|s| s == "Delete: Empty"), "{what}: {said:?}");
}

/// The grid's keys (Tab, Shift+Tab, the arrows, Delete...) are the active
/// editor's (the coordinator's ruling at U16's review): a press in
/// another editor's group, or in the main window beside a floating table,
/// ends them, a click on a cell gives them back; String Edit open over
/// another editor, they act on nothing.
#[test]
fn the_grid_s_keys_are_the_active_editor_s() {
    // Docked: the table in its group, Module Properties in another.
    let (mut h, id) = table_window(Mode::Tabs, "ui-table-active", 1920.0);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::ModuleProperties));
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::SplitEditor);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Table(id)));
    settle(&mut h);
    select(&mut h, id, (1, LABEL));
    let cheese = grid_cell(&h, "Cheese").rect().center();
    click_at(&mut h, cheese);
    assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Table");
    // A press in Module Properties' group: the grid's keys there do nothing.
    let other = pane_of(&h, &Tab::ModuleProperties);
    assert!(!other.contains(cheese), "two groups");
    click_at(&mut h, other.center_bottom() - egui::vec2(0.0, 40.0));
    keys_do_nothing(&mut h, id, "docked, the other group pressed");
    // A click on a cell gives them back: Tab to the next column.
    click_at(&mut h, cheese);
    h.key_press(egui::Key::Tab);
    settle(&mut h);
    assert_eq!(doc(&h, id).selected, Some((1, 1)), "the grid's again");
    // The header's More open over the grid (a popover): the keys act on
    // nothing of the table's while it is open.
    let header = h.get_by_label(mg_ui::table_view::TABLE_HEADER);
    header.get_by_label(mg_uikit::header::MORE).click();
    settle(&mut h);
    assert!(h.query_by_label(TABLE_MORE).is_some(), "More open");
    keys_do_nothing(&mut h, id, "More's popover open");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    // String Edit open (Module Properties' name), the table active: its
    // keys act on nothing while it is open.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::ModuleProperties));
    settle(&mut h);
    in_page(&h, "Basic").get_by_label("…").click();
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Table(id)));
    settle(&mut h);
    grid_keys(&mut h, id);
    keys_do_nothing(&mut h, id, "String Edit open");
    // Floating: the table in its window, Module Properties docked; a press
    // in the main window's editor ends the grid's keys.
    // (A screen of 2400: the window, as wide as it asks since M4's U17
    // review, leaves Module Properties in sight beside it.)
    let (mut h, id) = table_window(Mode::Floating, "ui-table-active", 2400.0);
    h.state_mut().settings.open_editors = mg_ui::OpenEditors::Tabs;
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::ModuleProperties));
    settle(&mut h);
    select(&mut h, id, (1, LABEL));
    let cheese = grid_cell(&h, "Cheese").rect().center();
    click_at(&mut h, cheese);
    h.key_press(egui::Key::Tab);
    settle(&mut h);
    assert_eq!(doc(&h, id).selected, Some((1, 1)), "floating, the grid's");
    let main = pane_of(&h, &Tab::ModuleProperties);
    let window = pane_of(&h, &Tab::Table(id)).expand(8.0);
    let corners = [main.left_bottom(), main.right_bottom(), main.left_top(), main.right_top()];
    let spot = corners
        .into_iter()
        .map(|c| c + (main.center() - c).normalized() * 30.0)
        .find(|p| !window.contains(*p))
        .expect("Module Properties in sight beside the window");
    click_at(&mut h, spot);
    keys_do_nothing(&mut h, id, "floating, the main window pressed");
}

/// A list of 300 values (a description of ours): one list, one stop of
/// Tab, in a frame of its own a few rows high (the panel stays short);
/// with its keys, End goes to the last value (brought into sight), Space
/// chooses it, letters go to a value by its name, Page Down a page on,
/// Enter chooses, Home goes to none; Tab leaves it at once (the review's
/// ruling: the old dropdown's keys kept).
#[test]
fn a_long_list_is_one_list_with_its_keys() {
    let dir = mg_testkit::scratch_dir("ui-table-long-list");
    std::fs::write(dir.join("nasher.cfg"), "").unwrap();
    let table = "2DA V2.0\r\n\r\n   Label  Pick\r\n0  One    5\r\n1  Two    240\r\n";
    std::fs::write(dir.join("many.2da"), table).unwrap();
    let values: Vec<String> = (0..300).map(|n| format!("V{n:03} = {n}")).collect();
    let toml = format!("[columns.Pick]\nkind = \"list\"\nvalues = {{ {} }}\n", values.join(", "));
    std::fs::write(dir.join("many.2da.toml"), toml).unwrap();
    let mut h = harness(app_in(Mode::Tabs, Vec::new()), 1600.0, 900.0);
    settle(&mut h);
    let id = mg_ui::table_view::open_path(h.state_mut(), &dir.join("many.2da")).unwrap();
    settle(&mut h);
    select(&mut h, id, (0, 1));
    let list =
        |h: &Harness<'_, Moonglow>| h.get_all_by_label("Values").next().expect("the list").rect();
    let pitch = 26.0;
    assert!(list(&h).height() < 9.0 * pitch, "a few rows high: {:?}", list(&h));
    let focused = |h: &Harness<'_, Moonglow>| {
        h.query_all_by_label("Values").any(|n| n.accesskit_node().is_focused())
    };
    cell_value(&h).click();
    settle(&mut h);
    h.key_press(egui::Key::Tab);
    settle(&mut h);
    assert!(focused(&h), "one Tab to the list");
    let key = |h: &mut Harness<'_, Moonglow>, k: egui::Key| {
        h.key_press(k);
        settle(h);
    };
    key(&mut h, egui::Key::End);
    let last = choice(&h, "V299").rect();
    assert!(list(&h).contains(last.center()), "the last brought into sight: {last:?}");
    key(&mut h, egui::Key::Space);
    assert_eq!(cell(&h, id, (0, 1)).as_deref(), Some("299"), "Space chose V299");
    h.event(egui::Event::Text("v15".into()));
    settle(&mut h);
    key(&mut h, egui::Key::Enter);
    assert_eq!(cell(&h, id, (0, 1)).as_deref(), Some("150"), "letters went to V150");
    key(&mut h, egui::Key::PageDown);
    key(&mut h, egui::Key::Enter);
    assert_eq!(cell(&h, id, (0, 1)).as_deref(), Some("158"), "a page on");
    key(&mut h, egui::Key::Home);
    key(&mut h, egui::Key::Enter);
    assert_eq!(cell(&h, id, (0, 1)), None, "Home: none");
    key(&mut h, egui::Key::Tab);
    assert!(!focused(&h), "Tab left the list at once");
    // Another row's cell: its value brought into sight in the list.
    select(&mut h, id, (1, 1));
    let on = choice(&h, "V240").rect();
    assert!(list(&h).contains(on.center()), "the value chosen in sight: {on:?}");
}

/// A table changed where it was read from (another program writes its
/// file) is read again as the grid is drawn, after the Cell panel: the
/// panel shows the cell as it is now once the window comes to rest.
#[test]
fn the_cell_panel_follows_a_table_read_again() {
    let (mut h, id) = table_window(Mode::Tabs, "ui-table-read-again", 1920.0);
    select(&mut h, id, (0, COST));
    assert_eq!(cell_value(&h).value().as_deref(), Some("1"));
    let mg_ui::table_view::Home::File(path) = doc(&h, id).home.clone() else { panic!("a file") };
    let changed = MINE.replacen("1     1     a", "1     8     a", 1);
    assert_ne!(changed, MINE);
    // (Its time a second on: looked at once a second.)
    std::thread::sleep(std::time::Duration::from_millis(1100));
    std::fs::write(&path, changed).unwrap();
    let start = std::time::Instant::now();
    while cell(&h, id, (0, COST)).as_deref() != Some("8") {
        assert!(start.elapsed().as_secs() < 10, "never read again");
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    settle(&mut h);
    assert_eq!(cell_value(&h).value().as_deref(), Some("8"), "Cell shows it as it is now");
}

/// What is typed in Cell's Value and left there unset is set in its own
/// cell where Cell is no longer drawn, after rows were numbered anew under
/// it: the Right Dock folded, then Save; Cell behind References (the one
/// bar under 1400), then a click in the grid. Never in the row that took
/// its number.
#[test]
fn typed_text_is_set_in_its_cell_where_cell_is_not_drawn() {
    for (case, width) in [("folded, Save", 1600.0), ("behind References, a click", 1300.0)] {
        let (mut h, id) = table_window(Mode::Tabs, "ui-table-typed-away", width);
        select(&mut h, id, (2, LABEL));
        type_value(&mut h, "x");
        // A row above taken away elsewhere, Cell not drawn since.
        match case {
            "folded, Save" => h.state_mut().settings.hide_palettes = true,
            _ => {
                let tab = h
                    .get_all_by_label("References")
                    .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Tab)
                    .expect("References' tab");
                tab.click();
            }
        }
        settle(&mut h);
        assert!(
            h.query_all_by_label("Value")
                .all(|n| n.accesskit_node().role() != egui::accesskit::Role::TextInput),
            "{case}: Cell not drawn"
        );
        let d = h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap();
        d.edit.remove_row(0);
        settle(&mut h);
        match case {
            "folded, Save" => table_click(&mut h, "Save"),
            _ => {
                let cheese = grid_cell(&h, "Cheese").rect().center();
                click_at(&mut h, cheese);
            }
        }
        let t = |h: &Harness<'_, Moonglow>, r| cell(h, id, (r, LABEL));
        assert_eq!(t(&h, 1).as_deref(), Some("Alex"), "{case}: set in Ale's row, now row 1");
        assert_eq!(
            t(&h, 2).as_deref(),
            Some("Eggs"),
            "{case}: not in the row that took its number"
        );
    }
}

/// Whether the Outliner's header is drawn in the main window.
fn outliner_drawn(h: &Harness<'_, Moonglow>) -> bool {
    h.query_by_label("Outliner Header").is_some()
}

/// No module open, a table in front: the Left Dock holds the Outliner
/// alone, its columns the whole height, the dock's fold button at the end
/// of the Outliner's header; folded, the strip of the Outliner's icon,
/// which brings it back; the table closed, no dock (the coordinator's
/// ruling at U16's review: before it, no dock was drawn until Resources
/// was asked for).
#[test]
fn a_table_without_a_module_lists_its_columns_in_the_left_dock() {
    let dir = mg_testkit::scratch_dir("ui-table-no-module");
    std::fs::write(dir.join("nasher.cfg"), "").unwrap();
    std::fs::write(dir.join("mine.2da"), MINE).unwrap();
    std::fs::write(dir.join("mine.2da.toml"), MINE_TOML).unwrap();
    let mut h = harness(app_in(Mode::Tabs, Vec::new()), 1600.0, 900.0);
    settle(&mut h);
    assert_eq!(h.state().left_dock_shows(), None, "nothing open: no dock");
    assert!(h.query_by_label("Left Dock").is_none());
    let id = mg_ui::table_view::open_path(h.state_mut(), &dir.join("mine.2da")).unwrap();
    settle(&mut h);
    assert_eq!(h.state().left_dock_shows(), Some(Panel::Outliner));
    assert!(outliner_drawn(&h), "the Outliner's header");
    for name in ["Label", "Mask", "Kind", "Cost"] {
        column_row(&h, name);
    }
    // (The whole height: from the top of the dock to its foot.)
    let dock = h.state().left_dock_rect().expect("the dock drawn");
    let fold = h.get_by_label("Outliner Header").get_by_label("Fold the Left Dock");
    let at = fold.rect();
    assert!(at.top() - dock.top() < 8.0, "{at:?} in the header at the top of {dock:?}");
    // Folded by the button at the end of its header, unfolded by the strip.
    fold.click();
    settle(&mut h);
    assert_eq!(h.state().left_dock_shows(), None, "folded");
    h.get_by_label("Show the Left Dock");
    let icon = h
        .query_all_by_label("Outliner")
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::Button)
        .expect("the Outliner's icon on the strip");
    icon.click();
    settle(&mut h);
    assert_eq!(h.state().left_dock_shows(), Some(Panel::Outliner), "unfolded from the strip");
    // F6 stops there once, before the grid: Shift+F6 from the grid gives
    // the Outliner the keys, Down and Enter go to the next column.
    select(&mut h, id, (1, LABEL));
    grid_keys(&mut h, id);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::F6);
    settle(&mut h);
    assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Marks", "the Outliner's");
    // (Its one stop: Shift+F6 again leaves the Left Dock, F6 comes back.)
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::F6);
    settle(&mut h);
    let before = mg_ui::palette_view::arrow_keys_are(&h.ctx);
    assert!(
        !["Marks", "Outliner"].contains(&before.as_ref()),
        "one stop in the Left Dock: {before}"
    );
    h.key_press(egui::Key::F6);
    settle(&mut h);
    assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Marks", "back to the Outliner");
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(doc(&h, id).selected.map(|s| s.1), Some(1), "Down and Enter: the next column");
    h.key_press(egui::Key::F6);
    settle(&mut h);
    assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "Table", "F6 on: the grid's");
    // The table closed: nothing the Outliner lists, no dock.
    h.state_mut().actions.push(mg_ui::Action::CloseTab(Tab::Table(id)));
    settle(&mut h);
    assert_eq!(h.state().left_dock_shows(), None, "the table closed");
    assert!(
        h.query_by_label("Left Dock").is_none() && h.query_by_label("Show the Left Dock").is_none()
    );
}

/// The other editors' Left Dock as before the Outliner was drawn alone
/// (U16's review): with a module, an area, a script, a conversation and a
/// blueprint each beside the Module panel, the Outliner under it for an
/// area and a conversation; without one, a game script and blueprint
/// opened with no dock at all (an area and a conversation open only from
/// a module: the game has none loose). A table there: its columns docked.
#[test]
fn the_left_dock_of_other_editors_is_as_before_with_or_without_a_module() {
    let Some((mut h, blueprint)) =
        blueprint_harness_in(Mode::Tabs, "nw_it_torch001", "ld_torch", ResType::UTI, None)
    else {
        return;
    };
    wide_window(&mut h);
    settle(&mut h);
    let script = ResKey::parse("ld_script", ResType::NSS).unwrap();
    let talk = ResKey::parse("ld_talk", ResType::DLG).unwrap();
    let edits = vec![
        mg_edit::Edit::SetResource { key: script, data: Some(b"void main()\n{\n}\n".to_vec()) },
        mg_edit::Edit::SetResource {
            key: talk,
            data: Some(crate::conversation_panels::talk().to_bytes().unwrap()),
        },
    ];
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("ld", edits)));
    settle(&mut h);
    let area = Tab::Area(ResRef::from_str("start").unwrap());
    let with = [
        (area, true),
        (Tab::Script(script), false),
        (Tab::Dialog(talk), true),
        (Tab::Blueprint(blueprint), false),
    ];
    for (tab, outliner) in with {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
        settle(&mut h);
        assert!(h.state().dock.find_tab(&tab).is_some(), "{tab:?} open");
        assert_eq!(h.state().left_dock_shows(), Some(Panel::Module), "{tab:?}");
        assert_eq!(outliner_drawn(&h), outliner, "{tab:?}: the Outliner");
    }
    // No module: the game's own script and blueprint, looked at.
    h.state_mut().actions.push(mg_ui::Action::Close);
    settle(&mut h);
    h.get_by_label("Don't Save").click();
    settle(&mut h);
    assert!(h.state().ws.is_none(), "the module closed");
    for (name, t) in [("nw_c2_default1", ResType::NSS), ("nw_it_torch001", ResType::UTI)] {
        let key = ResKey::parse(name, t).unwrap();
        h.state_mut().actions.push(mg_ui::Action::OpenResource(key));
        settle(&mut h);
        assert!(h.state().front_tab().is_some(), "{name} open");
        assert_eq!(h.state().left_dock_shows(), None, "{name}: no dock");
        assert!(!outliner_drawn(&h) && h.query_by_label("Left Dock").is_none(), "{name}");
    }
}

/// The Outliner lays out only the rows of the columns in sight (a table of
/// 160 columns, no module: the Outliner alone in the Left Dock); its keys
/// still go through all of them, each row they come to brought into sight:
/// End to the last, letters typed to a column by its name, Home back, a
/// click on a row taking the grid there.
#[test]
fn the_outliner_lays_out_only_the_columns_in_sight() {
    let dir = mg_testkit::scratch_dir("ui-table-wide-outliner");
    let columns: Vec<String> = (0..160).map(|c| format!("C{c:03}")).collect();
    let mut text = format!("2DA V2.0\r\n\r\n   {}\r\n", columns.join(" "));
    for r in 0..3 {
        let cells: Vec<String> = (0..160).map(|c| format!("v{r}x{c}")).collect();
        text.push_str(&format!("{r} {}\r\n", cells.join(" ")));
    }
    std::fs::write(dir.join("wide.2da"), text).unwrap();
    let mut h = harness(app_in(Mode::Tabs, Vec::new()), 1600.0, 900.0);
    settle(&mut h);
    let id = mg_ui::table_view::open_path(h.state_mut(), &dir.join("wide.2da")).unwrap();
    settle(&mut h);
    let dock = h.state().left_dock_rect().expect("the Outliner docked");
    let drawn = |h: &Harness<'_, Moonglow>| -> Vec<String> {
        let row = |n: &egui_kittest::Node<'_>| {
            in_group(n, mg_ui::OUTLINER)
                && n.accesskit_node().role() != egui::accesskit::Role::Label
                && n.accesskit_node().label().is_some_and(|l| l.len() == 4 && l.starts_with('C'))
        };
        h.query_all_by(|_| true).filter(row).filter_map(|n| n.accesskit_node().label()).collect()
    };
    let first = drawn(&h);
    // (900 high: some 37 rows of 22 in sight, and one either side.)
    assert!(first.len() < 45 && first.contains(&"C000".to_string()), "{first:?}");
    assert!(!first.contains(&"C159".to_string()));
    let in_sight = |h: &Harness<'_, Moonglow>, name: &str| {
        let r = column_row(h, name).rect();
        assert!(dock.contains(r.center()), "{name} at {r:?}, in sight in {dock:?}");
    };
    column_row(&h, "C002").click();
    settle(&mut h);
    assert_eq!(doc(&h, id).selected.map(|s| s.1), Some(2), "a click takes the grid there");
    h.key_press(egui::Key::End);
    settle(&mut h);
    in_sight(&h, "C159");
    assert!(!drawn(&h).contains(&"C000".to_string()), "the first out of sight, not laid out");
    // (Typed at once: a pause would start over.)
    h.event(egui::Event::Text("c08".into()));
    settle(&mut h);
    in_sight(&h, "C080");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(doc(&h, id).selected.map(|s| s.1), Some(80), "Enter takes the grid there");
    h.key_press(egui::Key::Home);
    settle(&mut h);
    in_sight(&h, "C000");
}

/// A floating table's header beside its window's panes (the pattern of
/// `a_floating_conversation_s_header_yields_no_sooner_than_before`): a new
/// window in a 1280 by 720 screen, and a window the kind kept at 1100 by
/// 700. In both the Outliner gives way first, folded to its strip; the
/// right pane (Cell, References) stays, and the header beside it keeps
/// every item where U16 put it: in the new window Add Row in its compact
/// form (its icon and its count), in the kept one with its words; the
/// row's three icons as the board draws them; Find, Go to and Save with
/// their words; More holding its five. (The tip, `f670d2e0`, had no kit
/// header: its three rows of buttons never yielded, and its window had
/// no right pane.)
#[test]
fn a_floating_table_s_header_beside_its_panes() {
    use mg_uikit::header::Place;
    let cases = [
        // (Add Row with its words in the new window too since M4's U17
        // review: its first size is the main window's, less a margin.)
        ("new", egui::vec2(1280.0, 720.0), None, Place::Row),
        ("kept", egui::vec2(1600.0, 900.0), Some([1100u32, 700]), Place::Row),
    ];
    for (case, screen, kept, add_row) in cases {
        let dir = mg_testkit::scratch_dir(&format!("ui-table-header-{case}"));
        let path = sample_module(&dir);
        std::fs::write(dir.join("nasher.cfg"), "").unwrap();
        std::fs::write(dir.join("mine.2da"), MINE).unwrap();
        std::fs::write(dir.join("mine.2da.toml"), MINE_TOML).unwrap();
        let mut app = app_in(Mode::Floating, Vec::new());
        app.settings.no_last_area = true;
        if let Some(size) = kept {
            app.settings.window_sizes = vec![("table".to_string(), size)];
        }
        app.open_module(&path);
        let mut h = harness(app, screen.x, screen.y);
        settle(&mut h);
        let id = mg_ui::table_view::open_path(h.state_mut(), &dir.join("mine.2da")).unwrap();
        settle(&mut h);
        select(&mut h, id, (1, LABEL));
        assert!(h.query_by_label(mg_ui::dialog_view::OUTLINER_STRIP).is_some(), "{case}: strip");
        assert!(h.query_all_by_label_contains("Kind, ").next().is_none(), "{case}: no pane");
        let value = cell_value(&h).rect();
        let records = mg_uikit::header::records(&h.ctx, mg_ui::table_view::TABLE_HEADER);
        let rec = records.into_iter().next().expect("its header");
        assert!(rec.row.right() <= value.left(), "{case}: the header beside Cell");
        let place = |name: &str| {
            let found = rec.places.iter().find(|(n, _)| n == name).map(|(_, p)| *p);
            found.unwrap_or_else(|| panic!("{case}: {name} not in the header"))
        };
        assert_eq!(place("Add Row"), add_row, "{case}: Add Row");
        for name in ["Save", "Find", "Go to", "How cells show"] {
            assert_eq!(place(name), Place::Row, "{case}: {name}");
        }
        for name in ["Empty Row", "Delete Row", "Add Column…"] {
            assert_eq!(place(name), Place::Icon, "{case}: {name}");
        }
        for name in ["Save As File…", "Take Rows…", "Compact", "Show Hidden Columns"] {
            assert_eq!(place(name), Place::More, "{case}: {name}");
        }
    }
}

/// A table only looked at, floating (As Floating Windows: the game's
/// spells.2da in a window of its own), lets go of its grid's keys at a
/// press in the main window, as an editor's grid does: Tab, Shift+Tab, the
/// arrows and Delete move nothing of it then; a click on its cell gives
/// them back.
#[test]
fn a_floating_table_only_looked_at_lets_go_of_its_keys() {
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let mut app = game_app(&root, None);
    app.settings = Mode::Floating.settings();
    let mut h = harness(app, 1920.0, 1080.0);
    settle(&mut h);
    let key = ResKey::parse("spells", ResType::TWODA).unwrap();
    let tab = Tab::Resource(key);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
    settle(&mut h);
    assert_ne!(h.state().dock.find_tab(&tab).unwrap().surface.0, 0, "in a window of its own");
    let game = h.state().game.clone().unwrap();
    let t = game.table("spells").unwrap();
    let first = t.get(0, "Label").unwrap().to_string();
    let name = t.get(0, "Name").map(str::to_string).unwrap_or_default();
    let at = h.get_by_label(&first).rect().center();
    click_at(&mut h, at);
    settle(&mut h);
    h.key_press(egui::Key::Tab);
    settle(&mut h);
    let on_name = format!("Row 0, Name: {name}");
    h.get_by_label_contains(&on_name);
    // A press in the main window, clear of the view's window.
    let main = pane_of(&h, &Tab::Welcome);
    let window = pane_of(&h, &tab).expand(8.0);
    let corners = [main.left_bottom(), main.right_bottom(), main.left_top(), main.right_top()];
    let spot = corners
        .into_iter()
        .map(|c| c + (main.center() - c).normalized() * 30.0)
        .find(|p| !window.contains(*p))
        .expect("the Welcome page in sight beside the window");
    click_at(&mut h, spot);
    for (m, k) in [
        (egui::Modifiers::NONE, egui::Key::Tab),
        (egui::Modifiers::SHIFT, egui::Key::Tab),
        (egui::Modifiers::NONE, egui::Key::ArrowDown),
        (egui::Modifiers::NONE, egui::Key::ArrowRight),
    ] {
        h.key_press_modifiers(m, k);
        settle(&mut h);
    }
    h.get_by_label_contains(&on_name);
    // A click on its cell gives them back.
    let at = h.get_by_label(&first).rect().center();
    click_at(&mut h, at);
    settle(&mut h);
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    let second = t.get(1, "Label").unwrap().to_string();
    h.get_by_label_contains(&format!("Row 1, Label: {second}"));
}

/// The Outliner's columns follow their description read again (kept by
/// the table's mark, `panels::columns_mark`, not made at every frame): a
/// column made a list of flags in its file says so in its row.
#[test]
fn the_outliner_follows_its_description_read_again() {
    let (mut h, id) = table_window(Mode::Tabs, "ui-table-outliner-desc", 1920.0);
    let words = |h: &Harness<'_, Moonglow>| {
        column_row(h, "Kind").accesskit_node().label().unwrap_or_default().to_string()
    };
    // (Waited for by a deadline: on a slow runner the row's words came a
    // frame after the description, and the test failed on its first line.)
    let wait = |h: &mut Harness<'_, Moonglow>, want: &str| {
        let start = std::time::Instant::now();
        while words(h) != want {
            assert!(start.elapsed().as_secs() < 10, "never {want}: {}", words(h));
            h.step();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    };
    wait(&mut h, "Kind, one of 2 values");
    let mg_ui::table_view::Home::File(path) = doc(&h, id).home.clone() else { panic!("a file") };
    let toml =
        MINE_TOML.replace("[columns.Kind]\nkind = \"list\"", "[columns.Kind]\nkind = \"flags\"");
    assert_ne!(toml, MINE_TOML);
    // (Its time a second on: looked at once a second.)
    std::thread::sleep(std::time::Duration::from_millis(1100));
    std::fs::write(path.with_extension("2da.toml"), toml).unwrap();
    wait(&mut h, "Kind, any of 2 flags");
}

/// The Outliner's columns follow the game's data read again (kept by the
/// table's mark and the load order's count, `Outliner::columns_seen`): a
/// module kept as a folder given an `itempropdef.2da` from outside (read
/// into the module's layer of the game's data), naming a table of ours as
/// an item property's subtypes, its `Name` a talk-table string as the
/// game's kinds go: its row says so.
#[test]
fn the_outliner_follows_the_game_s_data_read_again() {
    use mg_module::{Module, ModuleLocation};
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let dir = mg_testkit::scratch_dir("ui-table-outliner-game");
    let folder = dir.join("module");
    let mut m = Module::open(&sample_module(&dir)).unwrap();
    m.save_as(&ModuleLocation::Folder(folder.clone())).unwrap();
    std::fs::write(dir.join("mg_subtypes.2da"), "2DA V2.0\n\n   Label  Name\n0  One    5\n")
        .unwrap();
    let mut app = game_app(&root, None);
    app.settings = Mode::Tabs.settings();
    app.settings.no_last_area = true;
    app.open_module(&folder);
    let mut h = harness(app, 1920.0, 900.0);
    settle(&mut h);
    mg_ui::table_view::open_path(h.state_mut(), &dir.join("mg_subtypes.2da")).unwrap();
    settle(&mut h);
    let words = |h: &Harness<'_, Moonglow>| {
        column_row(h, "Name").accesskit_node().label().unwrap_or_default().to_string()
    };
    assert_eq!(words(&h), "Name", "no kind known");
    let data = {
        let game = h.state().game.as_deref().unwrap();
        let mut t = (*game.table("itempropdef").unwrap()).clone();
        let column = t.column("SubTypeResRef").unwrap();
        t.set(0, column, Some("mg_subtypes".into())).unwrap();
        t.to_bytes(game.codepage()).unwrap()
    };
    std::fs::write(folder.join("itempropdef.2da"), data).unwrap();
    h.state_mut().reload_project_files();
    settle(&mut h);
    assert_eq!(words(&h), "Name, a talk-table string's number", "the game's data read again");
}
