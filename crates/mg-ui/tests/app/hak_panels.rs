//! The hak editor (M4's U21; the Hak board): the header (Add Files…, Add
//! Folder…, Remove, Extract…, Extract All…, Find, Compile Models, Save,
//! Save As…, More, OK and Cancel) at every width; the notice strip; the
//! table, its heads' order and its keys; the Right Dock's Preview (what is
//! chosen, a sound's Play and Stop, a model's facts) and Details (its
//! Description typed, one Undo step); the Outliner's types and their
//! counts, a click finding one; the hints and counts; the window coming to
//! rest; a floating hak's first size; records from before reading without
//! a fault.

use crate::common::*;
use mg_ui::hak_view::{self as hv, HAK_CONTROLS, HAK_HEADER};

/// A 2DA of ours.
const TABLE: &[u8] = b"2DA V2.0\n\n   LABEL\n0  one\n1  two\n";

/// A model of ours, as text: a root and one triangle drawn with a texture.
pub(crate) const BOX_MDL: &str = "newmodel zz_box\nsetsupermodel zz_box NULL\n\
classification character\nbeginmodelgeom zz_box\nnode dummy zz_box\n  parent NULL\nendnode\n\
node trimesh tri\n  parent zz_box\n  bitmap zz_tex\n  verts 3\n    0 0 0\n    1 0 0\n    0 1 0\n\
  tverts 3\n    0 0 0\n    1 0 0\n    0 1 0\n  faces 1\n    0 1 2 1 0 1 2 0\nendnode\n\
endmodelgeom zz_box\ndonemodel zz_box\n";

/// A picture of ours, 2 by 1 pixels.
fn tga() -> Vec<u8> {
    let mut img = mg_image::Rgba::new(2, 1);
    img.data.copy_from_slice(&[255, 0, 0, 255, 0, 0, 255, 255]);
    mg_image::write_tga(&img)
}

/// The resources of the hak of ours: a picture, a sound, a table, a model
/// and a text.
fn resources() -> Vec<(ResKey, Vec<u8>)> {
    let key = |n: &str, t| ResKey::parse(n, t).unwrap();
    vec![
        (key("zz_tex", ResType::TGA), tga()),
        (key("zz_sound", ResType::WAV), crate::area_view::pcm_track()),
        (key("zz_table", ResType::TWODA), TABLE.to_vec()),
        (key("zz_box", ResType::MDL), BOX_MDL.as_bytes().to_vec()),
        (key("zz_notes", ResType::TXT), b"Notes of ours.".to_vec()),
        (key("zz_other", ResType::TGA), tga()),
    ]
}

/// The hak of ours written as `name`.hak in a scratch folder of its own,
/// with the description "Ours".
fn hak_file(dir: &str, name: &str) -> std::path::PathBuf {
    use mg_module::hak_edit::{Hak, Source};
    let path = mg_testkit::scratch_dir(dir).join(format!("{name}.hak"));
    let mut hak = Hak::new();
    let items = resources().into_iter().map(|(k, d)| (k, Source::Bytes(d.into()))).collect();
    hak.add_sources(items).unwrap();
    hak.set_description("Ours");
    hak.save(Some(&path)).unwrap();
    path
}

/// The hak of ours open in the hak editor in `mode`, in a window `width`
/// by 900, on the game installed at `game` where given and with a module
/// of ours open where `module`.
fn hak_window(
    mode: Mode,
    name: &str,
    width: f32,
    game: Option<&std::path::Path>,
    module: bool,
) -> (Harness<'static, Moonglow>, std::path::PathBuf, u32) {
    let path = hak_file(&mode.dir(name), "zz_ours");
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
    let id = hv::open_at(h.state_mut(), &path).expect("opened");
    settle(&mut h);
    (h, path, id)
}

/// Hak `id`'s document.
fn doc<'h>(h: &'h Harness<'_, Moonglow>, id: u32) -> &'h hv::HakDoc {
    h.state().haks.iter().find(|d| d.id == id).expect("open")
}

fn doc_mut<'h>(h: &'h mut Harness<'_, Moonglow>, id: u32) -> &'h mut hv::HakDoc {
    h.state_mut().haks.iter_mut().find(|d| d.id == id).expect("open")
}

/// A key of ours by its name and extension ("zz_tex.tga").
fn key(name: &str) -> ResKey {
    let (n, e) = name.split_once('.').unwrap();
    ResKey::parse(n, ResType::from_extension(e).unwrap()).unwrap()
}

/// The table's row named `name` ("zz_tex.tga"), the last such (a floating
/// window's is drawn after the main window's).
fn row(h: &Harness<'_, Moonglow>, name: &str) -> egui::Rect {
    h.get_all_by_label(hv::HAK_TABLE)
        .filter_map(|t| t.query_by_label(name).map(|n| n.rect()))
        .next_back()
        .unwrap_or_else(|| panic!("no row {name}"))
}

/// The rows of the table in its order, by name (those laid out).
fn rows_shown(h: &Harness<'_, Moonglow>) -> Vec<String> {
    let table = h.get_by_label(hv::HAK_TABLE);
    let mut rows: Vec<(f32, String)> = table
        .query_all(egui_kittest::kittest::by())
        .filter(|n| n.accesskit_node().bounding_box().is_some())
        .filter_map(|n| Some((n.rect().top(), n.accesskit_node().label()?)))
        .filter(|(_, l)| l.starts_with("zz_"))
        .collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    rows.dedup();
    rows.into_iter().map(|(_, l)| l).collect()
}

/// The Description's field in Details (its label is named so too).
fn description<'t>(h: &'t Harness<'_, Moonglow>) -> egui_kittest::Node<'t> {
    let input = |n: &egui_kittest::Node<'_>| {
        n.accesskit_node().role() == egui::accesskit::Role::MultilineTextInput
            || n.accesskit_node().role() == egui::accesskit::Role::TextInput
    };
    h.get_all_by_label(hv::DETAILS_DESCRIPTION).rfind(input).expect("the Description")
}

/// The hak header's More popover's name.
const HAK_MORE: &str = "More in Hak header";

/// The hak editor's header's control `name`, wherever it is at this
/// width: on the row, or in More, which is opened for it.
#[track_caller]
fn hak_tool<'t>(h: &'t mut Harness<'_, Moonglow>, name: &'t str) -> egui_kittest::Node<'t> {
    use mg_uikit::header::{self, Place};
    let rec = header::records(&h.ctx, HAK_HEADER).into_iter().next().expect("a hak header");
    let place = rec.places.iter().find(|(n, _)| n == name).map(|(_, p)| *p);
    if place == Some(Place::More) && h.query_by_label(HAK_MORE).is_none() {
        h.get_by_label(HAK_HEADER).get_by_label(header::MORE).click();
        settle(h);
    }
    let h: &'t Harness<'_, Moonglow> = h;
    match place {
        Some(Place::More) => h.get_by_label(HAK_MORE).get_by_label(name),
        Some(Place::Row | Place::Icon) => h.get_by_label(HAK_HEADER).get_by_label(name),
        other => panic!("{name} in the hak header: {other:?} ({:?})", rec.places),
    }
}

/// The header holds every control from 420 to 1600 (the 640 and 480 test
/// it replaces, M4's U21; the board's rule: a header never overflows):
/// docked and floating, each control on the row, its icon alone or in
/// More, OK and Cancel in the end slot; at 1400 and wider, As Tabs, all
/// but More's three (Update from Folder, Rename…, Edit Table) on the row.
/// At the least width, Save As… and More's Update from Folder work from
/// wherever they are.
#[test]
fn the_hak_header_holds_every_control_from_420_to_1600() {
    use mg_uikit::header::{self, Place};
    for mode in MODES {
        let (mut h, _, id) = hak_window(mode, "ui-u21-header", 1600.0, None, false);
        doc_mut(&mut h, id).folder = Some(mg_testkit::scratch_dir(&mode.dir("ui-u21-folder")));
        let mut widths: Vec<f32> = (21..=80).rev().map(|n| n as f32 * 20.0).collect();
        widths.extend([1399.0, 1400.0, 961.0, 962.0]);
        for width in widths {
            h.set_size(egui::vec2(width, 800.0));
            settle(&mut h);
            let what = format!("{mode:?} {width}");
            let rec = header::records(&h.ctx, HAK_HEADER);
            let rec = rec.into_iter().next().unwrap_or_else(|| panic!("{what}: no header"));
            let names: Vec<&str> = rec.places.iter().map(|(n, _)| n.as_str()).collect();
            assert_eq!(names, HAK_CONTROLS, "{what}");
            let place = |n: &str| rec.places.iter().find(|(m, _)| m == n).map(|(_, p)| *p);
            if width >= 1400.0 && mode == Mode::Tabs {
                for n in &HAK_CONTROLS[..9] {
                    assert!(matches!(place(n), Some(Place::Row | Place::Icon)), "{what}: {n}");
                }
                // (Add Files… and Add Folder… their icons alone, the board's;
                // the rest their words.)
                for n in [hv::ADD_FILES, hv::ADD_FOLDER] {
                    let w = h.get_by_label(HAK_HEADER).get_by_label(n).rect().width();
                    assert!(w < 40.0, "{what}: {n} an icon, {w} wide");
                }
                // (At the board's 1600, the rest with their words.)
                for n in [hv::EXTRACT_ALL, hv::COMPILE_MODELS, hv::SAVE_AS] {
                    if width >= 1600.0 {
                        assert_eq!(place(n), Some(Place::Row), "{what}: {n} with its words");
                    }
                }
            }
            // (At the least width every command that may has gone into More,
            // the end slot kept: none left on the row but those that stay.)
            if width <= 420.0 {
                for n in [hv::COMPILE_MODELS, hv::EXTRACT_ALL] {
                    assert_eq!(place(n), Some(Place::More), "{what}: {n} into More");
                }
            }
            for n in &HAK_CONTROLS[9..] {
                assert_eq!(place(n), Some(Place::More), "{what}: {n} in More");
            }
            let end = rec.end.expect("the end slot");
            for n in ["OK", "Cancel"] {
                let r = h.get_by_label(HAK_HEADER).get_by_label(n).rect();
                assert!(end.expand(0.5).contains_rect(r), "{what}: {n} in {end:?}");
            }
            header_holds(&mut h, HAK_HEADER, &HAK_CONTROLS);
        }
        // At the larger text sizes and with "Remove (1234)" (a long count:
        // 1234 keys selected), every control still held, OK and Cancel in
        // the end slot; each on the row with a tip naming it (the 640 and
        // 480 test's: a name and a tip each, enabled or not, inside the
        // window, Save As… its icon alone where narrow).
        let many: std::collections::BTreeSet<ResKey> = (0..1234)
            .map(|i| ResKey::parse(&format!("zz_{i:04}"), ResType::TXT).unwrap())
            .collect();
        doc_mut(&mut h, id).selected = many;
        for zoom in [1.0_f32, 1.25, 1.5] {
            h.ctx.set_zoom_factor(zoom);
            for width in [1600.0, 1200.0, 900.0, 640.0, 480.0] {
                h.set_size(egui::vec2(width, 800.0));
                settle(&mut h);
                let what = format!("{mode:?} {width} at {zoom}");
                let rec = header::records(&h.ctx, HAK_HEADER).remove(0);
                let shown = (rec.places.iter()).find(|(n, _)| n == hv::REMOVE).map(|(_, p)| *p);
                // (\`header_holds\` reads AccessKit's pixels, not points: at a
                // zoom its rects are the zoom's times. The zoom lays the
                // header out as a window 1/zoom as wide does, which the sweep
                // holds at zoom 1; here every control placed, none cut, each
                // on the row inside it, in points.)
                if zoom == 1.0 {
                    header_holds(&mut h, HAK_HEADER, &HAK_CONTROLS);
                }
                let names: Vec<&str> = rec.places.iter().map(|(n, _)| n.as_str()).collect();
                assert_eq!(names, HAK_CONTROLS, "{what}");
                assert!(
                    !rec.places.iter().any(|(_, p)| *p == Place::Cut),
                    "{what}: {:?}",
                    rec.places
                );
                if shown == Some(Place::Row) {
                    let r = h.get_by_label(HAK_HEADER).get_by_label(hv::REMOVE);
                    let said = r.accesskit_node().value().or(r.accesskit_node().label());
                    assert!(
                        h.query_by_label_contains("Remove (1234)").is_some() || said.is_some(),
                        "{what}"
                    );
                }
                let screen = h.ctx.content_rect();
                for (name, place) in &rec.places {
                    // (Find is a field: its words in it say what it finds.)
                    if matches!(place, Place::Row | Place::Icon) && name != hv::FIND {
                        let r = h.get_by_label(HAK_HEADER).get_by_label(name).rect();
                        assert!(screen.expand(0.5).contains_rect(r), "{what}: {name} inside");
                        assert!(rec.row.expand(0.5).contains_rect(r), "{what}: {name} on the row");
                        let tip =
                            tip_at(&mut h, r).unwrap_or_else(|| panic!("{what}: {name} no tip"));
                        if *place == Place::Icon {
                            assert!(
                                tip.contains(name.trim_end_matches('…')),
                                "{what}: {name}'s tip {tip:?}"
                            );
                        }
                    }
                }
            }
        }
        h.ctx.set_zoom_factor(1.0);
        doc_mut(&mut h, id).selected.clear();
        h.set_size(egui::vec2(420.0, 800.0));
        settle(&mut h);
        // At the least width, from wherever they are: Update from Folder
        // reads the folder (empty: every resource goes, one step).
        let before = doc(&h, id).hak.items().len();
        hak_tool(&mut h, hv::UPDATE_FROM_FOLDER).click();
        settle(&mut h);
        assert!(doc(&h, id).hak.items().len() < before, "{mode:?}: updated at 420");
        assert_eq!(doc(&h, id).hak.undo_label(), Some("Update from Folder"));
        h.key_press(egui::Key::Escape);
        settle(&mut h);
    }
}

/// The table's heads sort it (the board's Name, Type, Size, Source with the
/// arrow): a click on Type sorts by type, again the other way; Size's first
/// click the largest first; the selection kept by resource through each;
/// the rows made once for each order, not at every frame.
#[test]
fn the_table_sorts_by_its_heads_and_keeps_the_selection() {
    let (mut h, _, id) = hak_window(Mode::Tabs, "ui-u21-sort", 1600.0, None, false);
    click_pos(&h, row(&h, "zz_table.2da").center());
    settle(&mut h);
    let chosen = doc(&h, id).selected.clone();
    assert_eq!(chosen, [key("zz_table.2da")].into());
    let names = |h: &Harness<'_, Moonglow>| rows_shown(h);
    assert_eq!(names(&h)[0], "zz_box.mdl", "by name: {:?}", names(&h));
    let head = |h: &Harness<'_, Moonglow>, words: &str| {
        h.get_by_label(hv::HAK_TABLE).get_by_label_contains(words).rect().center()
    };
    click_pos(&h, head(&h, "Type"));
    settle(&mut h);
    assert_eq!((doc(&h, id).sort, doc(&h, id).descending), (hv::Sort::Type, false));
    // (Model, Sound, Table, Text, Texture: by the type's words.)
    let by_type = names(&h);
    assert_eq!(by_type[0], "zz_box.mdl", "{by_type:?}");
    assert_eq!(by_type.last().unwrap(), "zz_tex.tga", "{by_type:?}");
    assert!(h.query_by_label("Type, sorted up").is_some(), "the arrow's words");
    let walks = doc(&h, id).walks();
    for _ in 0..3 {
        h.hover_at(row(&h, "zz_tex.tga").center());
        h.run_steps(2);
        h.hover_at(row(&h, "zz_box.mdl").center());
        h.run_steps(2);
    }
    assert_eq!(doc(&h, id).walks(), walks, "no rows made again at a frame");
    click_pos(&h, head(&h, "Type"));
    settle(&mut h);
    assert!(doc(&h, id).descending, "the other way");
    assert_eq!(names(&h)[0], "zz_tex.tga");
    assert!(h.query_by_label("Type, sorted down").is_some());
    click_pos(&h, head(&h, "Size"));
    settle(&mut h);
    assert_eq!((doc(&h, id).sort, doc(&h, id).descending), (hv::Sort::Size, true));
    assert_eq!(names(&h)[0], "zz_sound.wav", "the largest first");
    assert_eq!(doc(&h, id).selected, chosen, "the selection kept");
    // (Kept through a remove of another and an add.)
    doc_mut(&mut h, id).hak.remove(&[key("zz_notes.txt")]);
    settle(&mut h);
    assert_eq!(doc(&h, id).selected, chosen, "kept through a remove");
}

/// A frame that changes nothing walks no hak (its rows sorted, its facts
/// counted, each once for a change): frames with the pointer moving, the
/// Outliner, Preview and Details drawn; an add walks it again, once each.
#[test]
fn a_frame_that_changes_nothing_walks_no_hak() {
    let (mut h, _, id) = hak_window(Mode::Tabs, "ui-u21-walks", 1600.0, None, false);
    doc_mut(&mut h, id).choose(key("zz_tex.tga"));
    settle(&mut h);
    assert!(h.query_by_label_contains("Details, zz_ours.hak").is_some(), "Details drawn");
    assert!(h.query_by_label_contains(".tga, 2 resources").is_some(), "the Outliner drawn");
    let walks = doc(&h, id).walks();
    for n in 0..20 {
        h.hover_at(egui::pos2(300.0 + n as f32 * 30.0, 300.0 + n as f32 * 10.0));
        h.run_steps(1);
    }
    assert_eq!(doc(&h, id).walks(), walks, "frames walked the hak");
    let file = mg_testkit::scratch_dir("ui-u21-walks-file").join("zz_more.txt");
    std::fs::write(&file, "more").unwrap();
    doc_mut(&mut h, id).hak.add_files(&[file]);
    settle(&mut h);
    assert_eq!(doc(&h, id).walks(), walks + 2, "made again once: its rows and its facts");
}

/// A GFF of the hak in the Preview (the read-only view's rows, `ro_gff`):
/// its shown rows listed once when it is first drawn and once for each
/// fold or unfold, none in frames that change nothing, counted with the
/// read-only tab's by the one counter (M4's U23 fix).
#[test]
fn the_preview_s_gff_lists_its_rows_once_a_fold() {
    use mg_gff::{Field, Label, Struct, Value};
    use mg_module::hak_edit::{Hak, Source};
    let mut gff = mg_gff::Gff::new(*b"UTC ");
    let item = |i: i32| Struct {
        fields: vec![Field { label: Label::new("Feat").unwrap(), value: Value::Int(i) }],
        ..Default::default()
    };
    let list = Value::List((0..3).map(item).collect());
    gff.root.fields.push(Field { label: Label::new("FeatList").unwrap(), value: list });
    let path = mg_testkit::scratch_dir("ui-u23-hak-gff").join("zz_gff.hak");
    let mut hak = Hak::new();
    let bytes = gff.to_bytes().unwrap();
    hak.add_sources(vec![(key("zz_hen.utc"), Source::Bytes(bytes.into()))]).unwrap();
    hak.save(Some(&path)).unwrap();
    let mut app = app_with(Vec::new());
    app.settings = Mode::Tabs.settings();
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    let id = hv::open_at(h.state_mut(), &path).expect("opened");
    settle(&mut h);
    let (before, _) = h.state().read_only_counts();
    doc_mut(&mut h, id).choose(key("zz_hen.utc"));
    settle(&mut h);
    let dock = right_dock_rect(&h);
    let list = (h.get_all_by_label("FeatList").map(|n| n.rect()))
        .find(|r| dock.contains_rect(*r))
        .expect("the GFF's rows in the Preview");
    let (shown, drawn) = h.state().read_only_counts();
    assert_eq!(shown, before + 1, "listed once as it is shown");
    assert!(drawn >= 1, "its rows drawn");
    for n in 0..10 {
        h.hover_at(egui::pos2(300.0 + n as f32 * 30.0, 300.0));
        h.run_steps(1);
    }
    assert_eq!(h.state().read_only_counts().0, shown, "frames listed nothing");
    click_pos(&h, list.center());
    settle(&mut h);
    let unfolded = h.state().read_only_counts().0;
    assert_eq!(unfolded, shown + 1, "listed once for the unfold");
    assert!(h.query_by_label("[2]").is_some(), "its items shown");
    h.run_steps(10);
    assert_eq!(h.state().read_only_counts().0, unfolded, "and not again");
}

/// The table's keys are the active editor's (two haks in two groups): a
/// click gives one's table the keys, Down and End choose in it; with the
/// other active, Down does nothing to the first; with a popover open,
/// nothing.
#[test]
fn the_table_s_keys_are_the_active_editor_s() {
    let (mut h, _, id) = hak_window(Mode::Tabs, "ui-u21-keys", 2400.0, None, false);
    click_pos(&h, row(&h, "zz_box.mdl").center());
    settle(&mut h);
    let chosen =
        |h: &Harness<'_, Moonglow>| doc(h, id).selected.iter().copied().collect::<Vec<_>>();
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(chosen(&h), [key("zz_notes.txt")], "Down");
    h.key_press(egui::Key::End);
    settle(&mut h);
    assert_eq!(chosen(&h), [key("zz_tex.tga")], "End");
    assert_eq!(doc(&h, id).preview_key(), Some(key("zz_tex.tga")), "the Preview follows");
    h.key_press(egui::Key::Home);
    settle(&mut h);
    // A popover open: the keys do nothing.
    h.get_by_label(HAK_HEADER).get_by_label(mg_uikit::header::MORE).click();
    settle(&mut h);
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(chosen(&h), [key("zz_box.mdl")], "nothing with More open");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    // Another hak in a group of its own at the right, pressed in.
    let path2 = hak_file("ui-u21-keys-2", "zz_two");
    let second = hv::open_at(h.state_mut(), &path2).unwrap();
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::SplitEditor);
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Hak(id)));
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Hak(second)));
    settle(&mut h);
    assert!(h.state().tab_rect(&Tab::Hak(id)).is_some(), "the first drawn beside it");
    let body = {
        let dock = &h.state().dock;
        let at = dock.find_tab(&Tab::Hak(second)).expect("open");
        dock.leaf(egui_dock::NodePath { surface: at.surface, node: at.node }).unwrap().rect
    };
    click_pos(&h, egui::pos2(body.right() - 40.0, body.bottom() - 40.0));
    settle(&mut h);
    assert_eq!(h.state().active_editor(), Some(&Tab::Hak(second)), "the second active");
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(chosen(&h), [key("zz_box.mdl")], "the first table's keys do nothing");
}

/// Down in Find goes on to the table, at its first row (the table has the
/// keys, outlined): Down and End go on from there in the table's order,
/// Enter shows the resource again where its Preview was closed.
#[test]
fn down_in_find_goes_to_the_table_s_first_row() {
    let (mut h, _, id) = hak_window(Mode::Tabs, "ui-hak-find-down", 1600.0, None, false);
    let chosen =
        |h: &Harness<'_, Moonglow>| doc(h, id).selected.iter().copied().collect::<Vec<_>>();
    hak_tool(&mut h, hv::FIND).click();
    h.get_by_label(HAK_HEADER).get_by_label(hv::FIND).type_text("zz");
    settle(&mut h);
    let order = rows_shown(&h);
    assert!(order.len() >= 3, "{order:?}");
    assert_eq!(accent_outlines(&h, 100.0), 0, "no keys yet");
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(chosen(&h), [key(&order[0])], "the first row, as the table is ordered");
    assert_eq!(accent_outlines(&h, 100.0), 1, "the table outlined");
    says(&mut h, &["Up or Down: Move in the list", "Enter: View"], &[]);
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(chosen(&h), [key(&order[1])], "Down goes on in the table");
    h.key_press(egui::Key::End);
    settle(&mut h);
    assert_eq!(chosen(&h), [key(order.last().unwrap())], "End");
    // Enter views it: its Preview, closed, is shown again.
    h.get_by_label(hv::PREVIEW_CLOSE).click();
    settle(&mut h);
    assert_eq!(doc(&h, id).preview_key(), None, "closed");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(doc(&h, id).preview_key(), Some(key(order.last().unwrap())), "Enter");
}

/// Down in Find inside the header's More closes the popover, and the table's
/// arrows work at once.
#[test]
fn down_in_find_in_more_closes_it_and_the_table_has_the_keys() {
    use mg_uikit::header::{self, Place};
    let (mut h, _, id) = hak_window(Mode::Tabs, "ui-hak-find-more", 400.0, None, false);
    let rec = header::records(&h.ctx, HAK_HEADER).into_iter().next().expect("a hak header");
    let place = rec.places.iter().find(|(n, _)| n == hv::FIND).map(|(_, p)| *p);
    assert_eq!(place, Some(Place::More), "Find in More at this width: {:?}", rec.places);
    let chosen =
        |h: &Harness<'_, Moonglow>| doc(h, id).selected.iter().copied().collect::<Vec<_>>();
    hak_tool(&mut h, hv::FIND).click();
    settle(&mut h);
    assert!(h.query_by_label(HAK_MORE).is_some(), "More open");
    h.get_by_label(HAK_MORE).get_by_label(hv::FIND).type_text("zz");
    settle(&mut h);
    let order = rows_shown(&h);
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert!(h.query_by_label(HAK_MORE).is_none(), "More closed by Down");
    assert_eq!(chosen(&h), [key(&order[0])], "the first row");
    h.key_press(egui::Key::ArrowDown);
    settle(&mut h);
    assert_eq!(chosen(&h), [key(&order[1])], "the arrows work at once");
}

/// The Outliner lists the hak's types with their counts, the most first
/// (the board's), with no module open (the Left Dock holding it alone); a
/// click on a type finds it (Find ".tga", the table that type's alone, the
/// row chosen), again shows every type; its keys: Down and Enter find the
/// next type.
#[test]
fn the_outliner_counts_the_hak_s_types_and_a_click_finds_one() {
    let (mut h, _, id) = hak_window(Mode::Tabs, "ui-u21-outliner", 1600.0, None, false);
    let outline = |h: &Harness<'_, Moonglow>| -> Vec<String> {
        let o = h.get_by_label(mg_ui::OUTLINER);
        let mut rows: Vec<(f32, String)> = o
            .query_all(egui_kittest::kittest::by())
            .filter(|n| n.accesskit_node().bounding_box().is_some())
            .filter_map(|n| Some((n.rect().top(), n.accesskit_node().label()?)))
            .filter(|(_, l)| l.starts_with('.'))
            .collect();
        rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        rows.into_iter().map(|(_, l)| l).collect()
    };
    let o = outline(&h);
    assert_eq!(o[0], ".tga, 2 resources", "the most first: {o:?}");
    assert_eq!(o.len(), 5, "{o:?}");
    assert!(o.contains(&".mdl, 1 resource".to_string()), "{o:?}");
    h.get_by_label(mg_ui::OUTLINER).get_by_label(".tga, 2 resources").click();
    settle(&mut h);
    assert_eq!(doc(&h, id).filter, ".tga");
    assert_eq!(rows_shown(&h), ["zz_other.tga", "zz_tex.tga"]);
    h.key_press(egui::Key::ArrowDown);
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(doc(&h, id).filter, outline(&h)[1].split(',').next().unwrap(), "Down, Enter");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    assert_eq!(doc(&h, id).filter, "", "found again: every type");
    assert!(rows_shown(&h).len() >= 6, "{:?}", rows_shown(&h));
}

/// The Preview shows the resource chosen (the one clicked last of those
/// selected, the board's first of three), made once as it is chosen: a
/// picture with its size, a sound with Play, a table's rows, a text; its
/// header's Close puts it away until another is chosen; a double click
/// shows it again. Details lists File, Contents, Built from and the
/// Description.
#[test]
fn the_preview_and_details_show_the_hak() {
    let (mut h, path, id) = hak_window(Mode::Tabs, "ui-u21-preview", 1600.0, None, false);
    let viewing = |h: &Harness<'_, Moonglow>| doc(h, id).viewing();
    assert_eq!(viewing(&h), None);
    assert!(h.query_by_label("Preview, Nothing chosen").is_some(), "nothing chosen yet");
    click_pos(&h, row(&h, "zz_tex.tga").center());
    settle(&mut h);
    assert_eq!(viewing(&h), Some((key("zz_tex.tga"), "picture")));
    assert!(h.query_by_label_contains("zz_tex.tga, 2 by 1 pixels").is_some());
    let dock = right_dock_rect(&h);
    let shown = h.get_by_label_contains("zz_tex.tga, 2 by 1 pixels").rect();
    assert!(dock.contains_rect(shown), "in the Right Dock");
    // Three selected: the one clicked last.
    let shift = |h: &mut Harness<'_, Moonglow>, name: &str| {
        let at = row(h, name).center();
        h.hover_at(at);
        press(h, at, true, egui::Modifiers::SHIFT);
        h.run_steps(1);
        press(h, at, false, egui::Modifiers::SHIFT);
        settle(h);
    };
    shift(&mut h, "zz_sound.wav");
    assert_eq!(doc(&h, id).selected.len(), 3, "a run of three");
    assert_eq!(viewing(&h).map(|v| v.0), Some(key("zz_tex.tga")), "the one clicked first");
    click_pos(&h, row(&h, "zz_sound.wav").center());
    settle(&mut h);
    assert_eq!(viewing(&h), Some((key("zz_sound.wav"), "sound")));
    assert!(h.query_by_label("Play").is_some(), "Play");
    click_pos(&h, row(&h, "zz_table.2da").center());
    settle(&mut h);
    assert_eq!(viewing(&h), Some((key("zz_table.2da"), "table")));
    click_pos(&h, row(&h, "zz_notes.txt").center());
    settle(&mut h);
    assert_eq!(viewing(&h), Some((key("zz_notes.txt"), "plain")));
    // Close: put away; another chosen shows; back to it, shown again; a
    // double click too.
    h.get_by_label(hv::PREVIEW_CLOSE).click();
    settle(&mut h);
    assert_eq!(doc(&h, id).preview_key(), None, "closed");
    click_pos(&h, row(&h, "zz_tex.tga").center());
    settle(&mut h);
    assert_eq!(doc(&h, id).preview_key(), Some(key("zz_tex.tga")), "another chosen");
    click_pos(&h, row(&h, "zz_notes.txt").center());
    settle(&mut h);
    assert_eq!(doc(&h, id).preview_key(), Some(key("zz_notes.txt")), "back to it");
    h.get_by_label(hv::PREVIEW_CLOSE).click();
    settle(&mut h);
    assert_eq!(doc(&h, id).preview_key(), None, "closed again");
    let at = row(&h, "zz_notes.txt").center();
    double_click_now(&mut h, at);
    settle(&mut h);
    assert_eq!(doc(&h, id).preview_key(), Some(key("zz_notes.txt")), "a double click");
    // Details.
    assert!(h.query_by_label_contains("Details, zz_ours.hak").is_some(), "Details");
    let details = h.get_by_label("Built from").rect();
    assert!(dock.contains_rect(details), "Details in the Right Dock");
    let contents = h.get_all_by_label_contains("6 resources, ").map(|n| n.rect());
    assert!(contents.into_iter().any(|r| dock.contains_rect(r)), "Contents in Details");
    assert!(h.query_all_by_label("(not built from a folder)").next().is_some());
    // (Its name after a separator, `\\` on Windows: the path's own.)
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    let tail = format!("{}{name}", std::path::MAIN_SEPARATOR);
    let mut shown = h.get_all_by_label_contains(&tail).map(|n| n.rect());
    assert!(shown.any(|r| dock.contains_rect(r)), "the file in Details");
    let update = h.get_by_label(hv::UPDATE_FROM_FOLDER);
    assert!(update.accesskit_node().is_disabled(), "not built from a folder");
}

/// A sound played in the Preview (its Stop shown while it plays, looked at
/// again every 300 ms, which egui's harness takes for rest) stops when
/// another resource is chosen, the window at rest.
#[test]
fn a_sound_rests_once_another_is_chosen() {
    let (mut h, _, id) = hak_window(Mode::Tabs, "ui-u21-sound", 1600.0, None, false);
    click_pos(&h, row(&h, "zz_sound.wav").center());
    settle(&mut h);
    h.get_by_label("Play").click();
    settle(&mut h);
    assert!(h.query_by_label("Stop").is_some(), "Stop");
    assert!(h.state().speaker.playing(mg_ui::audio::Channel::Preview), "playing");
    click_pos(&h, row(&h, "zz_tex.tga").center());
    settle(&mut h);
    assert!(!h.state().speaker.playing(mg_ui::audio::Channel::Preview), "stopped");
    assert_eq!(doc(&h, id).preview_key(), Some(key("zz_tex.tga")));
}

/// Typed text keeps to its hak, typed after the move (the reviews' rule):
/// the Description typed in one hak, another brought to the front (its
/// Details shown): the text set on the first, the second's untouched; typed
/// in the second and Save pressed: set before it is written; typed, then
/// the Top Bar's Undo with the pointer over Details: set, then undone, one
/// step each.
#[test]
fn typed_description_keeps_to_its_hak() {
    let (mut h, _, first) = hak_window(Mode::Tabs, "ui-u21-typed", 1600.0, None, false);
    let path2 = hak_file("ui-u21-typed-2", "zz_two");
    let second = hv::open_at(h.state_mut(), &path2).unwrap();
    settle(&mut h);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Hak(first)));
    settle(&mut h);
    let field = |h: &Harness<'_, Moonglow>| description(h).rect();
    click_pos(&h, field(&h).center());
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    description(&h).type_text("Alpha");
    settle(&mut h);
    // The second hak brought to the front, then typed in.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Hak(second)));
    settle(&mut h);
    assert_eq!(doc(&h, first).hak.description, "Alpha", "set on the first as it left");
    assert_eq!(doc(&h, first).hak.undo_label(), Some("Description"));
    assert_eq!(doc(&h, second).hak.description, "Ours", "the second untouched");
    click_pos(&h, field(&h).center());
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    description(&h).type_text("Beta");
    h.run_steps(1);
    assert!(doc(&h, second).is_dirty(), "typed counts as unsaved");
    hak_tool(&mut h, hv::SAVE).click();
    settle(&mut h);
    assert_eq!(doc(&h, second).hak.description, "Beta");
    assert!(!doc(&h, second).is_dirty(), "saved with it");
    let saved = mg_module::hak_edit::Hak::open(&path2).unwrap();
    assert_eq!(saved.description, "Beta", "written");
    assert_eq!(doc(&h, first).hak.description, "Alpha", "the first as it was");
    // Typed over two frames, then the Top Bar's Undo pressed (the pointer
    // gone to it, the field left as it is pressed): set, then undone, one
    // step for the two frames' typing; its Redo: back.
    click_pos(&h, field(&h).center());
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    description(&h).type_text("Gam");
    settle(&mut h);
    description(&h).type_text("ma");
    settle(&mut h);
    let steps = doc(&h, second).hak.changes();
    h.get_by_label("Top Bar").get_by_label("Undo").click();
    settle(&mut h);
    assert_eq!(doc(&h, second).hak.description, "Beta", "Gamma set, then undone: one step");
    assert_eq!(doc(&h, first).hak.description, "Alpha", "the other hak's own");
    assert_eq!(doc(&h, second).hak.changes(), steps + 2, "one step set, one undone");
    h.get_by_label("Top Bar").get_by_label("Redo").click();
    settle(&mut h);
    assert_eq!(doc(&h, second).hak.description, "Gamma", "and redone");
}

/// The hints and counts as bound (the Hak board's): with the pointer over
/// the editor, Click Select, Ctrl + Click Add to selection, Shift + Click
/// Select range, Double Click View, Right Click More; with it elsewhere,
/// none; none while Find is typed in; never a Save hint (Ctrl+S is the
/// module's Save, the owner's ruling of 2026-11-18). "6 resources, …"
/// always, "N selected" where any are.
#[test]
fn the_hak_hints_are_as_bound() {
    let said = |h: &Harness<'_, Moonglow>| h.state().status_said();
    let none_of = |s: &[String], words: &[&str], what: &str| {
        for w in words {
            assert!(!s.iter().any(|l| l.contains(w)), "{what}: no {w}: {s:?}");
        }
    };
    let hints = ["Click: Select", "Ctrl, Click: Add to selection", "Shift, Click: Select range"];
    let more = ["Double Click: View", "Right Click: More"];
    let (mut h, _, id) = hak_window(Mode::Tabs, "ui-u21-hints", 1600.0, None, true);
    h.hover_at(row(&h, "zz_tex.tga").center());
    settle(&mut h);
    let s = said(&h);
    for w in hints.iter().chain(&more) {
        assert!(s.contains(&w.to_string()), "{w}: {s:?}");
    }
    assert!(s.iter().any(|l| l.starts_with("6 resources, ")), "{s:?}");
    none_of(&s, &["selected", "Save"], "nothing selected");
    click_pos(&h, row(&h, "zz_tex.tga").center());
    settle(&mut h);
    doc_mut(&mut h, id).selected.insert(key("zz_box.mdl"));
    settle(&mut h);
    assert!(said(&h).contains(&"2 selected".to_string()), "{:?}", said(&h));
    // The pointer elsewhere: the counts, no hints.
    let left = left_dock_rect(&h).center();
    h.hover_at(left);
    settle(&mut h);
    let s = said(&h);
    none_of(&s, &["Click", "Save"], "the pointer elsewhere");
    assert!(s.contains(&"2 selected".to_string()), "{s:?}");
    // More open (a popover): none.
    h.hover_at(row(&h, "zz_tex.tga").center());
    h.get_by_label(HAK_HEADER).get_by_label(mg_uikit::header::MORE).click();
    h.run_steps(1);
    h.hover_at(row(&h, "zz_tex.tga").center());
    h.run_steps(2);
    none_of(&said(&h), &["Click"], "More open");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    // Typing in Find: none.
    h.hover_at(row(&h, "zz_tex.tga").center());
    hak_tool(&mut h, hv::FIND).click();
    h.get_by_label(HAK_HEADER).get_by_label(hv::FIND).type_text("zz");
    h.run_steps(1);
    h.hover_at(row(&h, "zz_tex.tga").center());
    h.run_steps(2);
    none_of(&said(&h), &["Click", "Save"], "typing");
}

/// The window comes to rest (`settle`) with the hak's panels in front,
/// behind and folded, the docks closed, docked and floating: a click in
/// the table, its keys, a sort, Find typed, the Description typed, an add,
/// a remove and an Undo; the last check that the remove changed the hak.
#[test]
fn the_window_comes_to_rest_with_the_hak_s_panels() {
    let cases: [(Mode, &str, f32); 7] = [
        (Mode::Tabs, "stacked", 1600.0),
        (Mode::Tabs, "Details", 1300.0),
        (Mode::Tabs, "Preview", 1300.0),
        (Mode::Tabs, "folded", 1600.0),
        (Mode::Tabs, "closed", 1600.0),
        (Mode::Floating, "pane", 1600.0),
        (Mode::Floating, "small", 1280.0),
    ];
    for (mode, case, width) in cases {
        let (mut h, _, id) = hak_window(mode, "ui-u21-rest", width, None, false);
        let what = format!("{mode:?} {case}");
        match case {
            "Details" | "Preview" => {
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
        click_pos(&h, row(&h, "zz_box.mdl").center());
        settle(&mut h);
        h.key_press(egui::Key::ArrowDown);
        settle(&mut h);
        let head = h.get_by_label(hv::HAK_TABLE).get_by_label_contains("Type").rect().center();
        click_pos(&h, head);
        settle(&mut h);
        hak_tool(&mut h, hv::FIND).click();
        h.get_by_label(HAK_HEADER).get_by_label(hv::FIND).type_text("zz");
        settle(&mut h);
        let details = matches!(case, "stacked" | "Details" | "pane" | "small");
        assert_eq!(
            h.query_all_by_label(hv::DETAILS_DESCRIPTION).next().is_some(),
            details,
            "{what}: Details drawn"
        );
        if details {
            let field = description(&h).rect();
            click_pos(&h, field.center());
            settle(&mut h);
            description(&h).type_text(" more");
            settle(&mut h);
        }
        let file = mg_testkit::scratch_dir(&mode.dir("ui-u21-rest-file")).join("zz_added.txt");
        std::fs::write(&file, "added").unwrap();
        doc_mut(&mut h, id).hak.add_files(&[file]);
        settle(&mut h);
        let before = doc(&h, id).hak.items().len();
        click_pos(&h, row(&h, "zz_added.txt").center());
        settle(&mut h);
        hak_tool(&mut h, hv::REMOVE).click();
        settle(&mut h);
        assert_eq!(doc(&h, id).hak.items().len(), before - 1, "{what}: the remove changed it");
        h.state_mut().actions.push(mg_ui::Action::Undo);
        settle(&mut h);
    }
}

/// A model of the hak in the Preview (a GPU, the game's data): its 3D view
/// drawn and at rest, paused at first; Play asks for frames, Pause rests;
/// its facts listed under it.
#[test]
fn the_model_s_view_rests_until_played() {
    let root = mg_testkit::corpus!();
    let Some(rs) = gpu() else { return };
    let path = hak_file("ui-u21-3d", "zz_ours");
    let mut app = game_app(&root, None);
    app.settings = Mode::Tabs.settings();
    app.set_render_state(rs.clone());
    let mut h = gpu_harness(app, rs, 1600.0, 1200.0);
    settle(&mut h);
    let id = hv::open_at(h.state_mut(), &path).unwrap();
    settle(&mut h);
    doc_mut(&mut h, id).choose(key("zz_box.mdl"));
    settle(&mut h);
    assert_eq!(doc(&h, id).viewing(), Some((key("zz_box.mdl"), "model")));
    let revision = doc(&h, id).hak.changes();
    let source = mg_ui::model_view::Source::Hak { hak: id, revision, key: key("zz_box.mdl") };
    let view = |h: &Harness<'_, Moonglow>| {
        let v = h.state().model_views.get(&source).expect("the model's view made");
        (v.playing, v.drawn_in())
    };
    let (playing, drawn) = view(&h);
    assert!(!playing, "paused at first");
    let pass = h.ctx.cumulative_pass_nr();
    assert!(drawn.is_some_and(|d| d + 3 >= pass), "drawn: {drawn:?} at {pass}");
    let node = h.get_by_label_contains("Facts: ");
    let facts = node.accesskit_node().label().or(node.accesskit_node().value()).unwrap();
    assert!(facts.contains("Type Model, text") && facts.contains("Faces 1"), "{facts}");
    assert!(facts.contains("Textures 1") && facts.contains("Supermodel (none)"), "{facts}");
    h.get_by_label("Play").click();
    assert!(h.try_run().is_err(), "played: it asks for frames");
    h.get_by_label("Pause").click();
    settle(&mut h);
    assert!(!view(&h).0, "paused again, at rest");
    // Another model chosen, then a picture: one view of the hak's kept, then
    // none (arrowed through, each model's view was kept).
    let two = key("zz_box2.mdl");
    let src = mg_module::hak_edit::Source::Bytes(BOX_MDL.as_bytes().to_vec().into());
    doc_mut(&mut h, id).hak.add_sources(vec![(two, src)]).unwrap();
    settle(&mut h);
    doc_mut(&mut h, id).choose(two);
    settle(&mut h);
    let hak_views = |h: &Harness<'_, Moonglow>| {
        let views = h.state().model_views.keys();
        views.filter(|s| matches!(s, mg_ui::model_view::Source::Hak { .. })).count()
    };
    assert_eq!(hak_views(&h), 1, "the one shown");
    doc_mut(&mut h, id).choose(key("zz_tex.tga"));
    settle(&mut h);
    assert_eq!(hak_views(&h), 0, "a picture shown: none");
    h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Hak(id)]));
    settle(&mut h);
}

/// A floating hak opens at its first size within the main window (1280 by
/// 720 and 1600 by 900; the coordinator's ruling at U17's review), its
/// Preview and Details in its window's right pane, its types at its left
/// where there is room (else the strip), its header holding every control.
#[test]
fn a_floating_hak_opens_at_its_first_size() {
    for (width, height) in [(1280.0, 720.0), (1600.0, 900.0)] {
        let path = hak_file(&format!("ui-u21-float-{width}"), "zz_ours");
        let mut app = app_in(Mode::Floating, Vec::new());
        app.settings.no_last_area = true;
        let mut h = harness(app, width, height);
        settle(&mut h);
        let id = hv::open_at(h.state_mut(), &path).unwrap();
        settle(&mut h);
        doc_mut(&mut h, id).choose(key("zz_tex.tga"));
        settle(&mut h);
        let what = format!("{width} by {height}");
        let at = h.state().dock.find_tab(&Tab::Hak(id)).expect("open");
        assert!(!at.surface.is_main(), "{what}: a window of its own");
        let pane = (h.state().dock.iter_leaves())
            .find(|(_, leaf)| leaf.tabs.contains(&Tab::Hak(id)))
            .map(|(_, leaf)| leaf.rect)
            .unwrap();
        let screen = h.ctx.content_rect();
        assert!(screen.contains_rect(pane), "{what}: within the main window: {pane:?}");
        // (Its first size: the editor's 920, its Preview and Details (280)
        // and its types (260) beside it, held to the main window less its
        // margins: the coordinator's ruling at U17's review.)
        let want = (920.0 + mg_ui::RIGHT_DOCK_LEAST + 1.0 + 260.0 + 1.0).min(width - 48.0);
        assert!(pane.width() >= want - 12.0, "{what}: {} wide, wants {want}", pane.width());
        let picture = h.get_by_label_contains("zz_tex.tga, 2 by 1 pixels").rect();
        assert!(pane.contains_rect(picture), "{what}: the Preview in its pane");
        let details = h.get_by_label("Built from").rect();
        assert!(pane.contains_rect(details), "{what}: Details in its pane");
        let types = h.query_by_label_contains(".tga, 2 resources").is_some();
        // (Or folded to its strip at the window's left, its icon named
        // Outliner.)
        let strip = (h.query_all_by_label(mg_ui::OUTLINER))
            .filter(|n| n.accesskit_node().bounding_box().is_some())
            .any(|n| pane.contains_rect(n.rect()) && n.rect().width() < 40.0);
        assert_eq!((types, strip), (width >= 1600.0, width < 1600.0), "{what}: types or strip");
        header_holds(&mut h, HAK_HEADER, &HAK_CONTROLS);
    }
}

/// Records from before read without a fault: a settings file with the
/// hak's window size and a dock layout naming a panel the hak never had
/// (the tip wrote no preview pane or width of the hak's: its inline
/// preview kept none); an open-tabs record of a hak with its resources
/// chosen (as the tip writes it). The hak opens where it was, its chosen
/// kept, its Preview and Details shown.
#[test]
fn records_of_a_hak_read_without_error() {
    let text = "(window_sizes:[(\"hak\",(920,720))],window_sizes_scaled:true,\
                dock_layouts:{\"hak\":(fronts:[\"Palette\"],shares:[1000])},name_resrefs:true)";
    let (settings, _unread) = mg_ui::Settings::read_fields(text).expect("read");
    assert!(settings.name_resrefs, "the rest read");
    assert_eq!(settings.window_sizes, [("hak".to_string(), [920, 720])], "its size read");
    for mode in MODES {
        let hak = hak_file(&mode.dir("ui-u21-record-hak"), "zz_ours");
        let dir = mg_testkit::scratch_dir(&mode.dir("ui-u21-record"));
        let path = sample_module(&dir);
        let tabs = dir.join("open-tabs");
        std::fs::create_dir_all(&tabs).unwrap();
        let name = serde_json::to_string(&format!("hak:{}", hak.display())).unwrap();
        let record = match mode {
            Mode::Floating => format!(
                r#"{{"panes":[{{"place":"middle","tabs":["module-properties"],"active":"module-properties"}},
                {{"place":"window","tabs":[{name}],"active":{name},"rect":[300.0,60.0,920.0,720.0]}}],
                "front":{name},"views":{{{name}:{{"chosen":["zz_tex.tga"]}}}},
                "rects_scaled":true}}"#
            ),
            Mode::Tabs => format!(
                r#"{{"panes":[{{"place":"middle","tabs":["module-properties",{name}],"active":{name}}}],
                "front":{name},"views":{{{name}:{{"chosen":["zz_tex.tga"]}}}},
                "rects_scaled":true}}"#
            ),
        };
        std::fs::write(mg_ui::open_tabs::file(&tabs, &path), record).unwrap();
        let mut app = app_in(mode, Vec::new());
        app.settings.window_sizes = settings.window_sizes.clone();
        app.settings.window_sizes_scaled = settings.window_sizes_scaled;
        app.settings.dock_layouts = settings.dock_layouts.clone();
        app.settings.no_last_area = true;
        app.open_tabs_dir = Some(tabs.clone());
        app.open_module(&path);
        let mut h = harness(app, 1600.0, 900.0);
        settle(&mut h);
        let what = format!("{mode:?}");
        let d = h.state().haks.first().unwrap_or_else(|| panic!("{what}: opened"));
        assert_eq!(d.selected, [key("zz_tex.tga")].into(), "{what}: its chosen");
        let at = h.state().dock.find_tab(&Tab::Hak(d.id)).expect("its tab");
        assert_eq!(at.surface.is_main(), mode == Mode::Tabs, "{what}: where it was");
        header_holds(&mut h, HAK_HEADER, &HAK_CONTROLS);
        assert!(h.query_by_label_contains("zz_tex.tga, 2 by 1 pixels").is_some(), "{what}");
        assert!(h.query_by_label_contains("Details, zz_ours.hak").is_some(), "{what}");
    }
}

/// F6 reaches the hak's panes in order: the Outliner's types, the table
/// (its keys), the Right Dock's Preview, Details (its Description given the
/// keyboard).
#[test]
fn f6_reaches_the_hak_s_panes() {
    let (mut h, _, _) = hak_window(Mode::Tabs, "ui-u21-f6", 1600.0, None, false);
    let mut seen = Vec::new();
    for _ in 0..5 {
        h.key_press(egui::Key::F6);
        settle(&mut h);
        seen.push(mg_ui::palette_view::arrow_keys_are(&h.ctx));
    }
    assert!(seen.contains(&"Marks".to_string()), "the Outliner: {seen:?}");
    assert!(seen.contains(&"Hak".to_string()), "the table: {seen:?}");
    let at = seen.iter().position(|s| s == "Marks").unwrap();
    assert_eq!(seen[(at + 1) % seen.len()], "Hak", "the table after the Outliner: {seen:?}");
    // On to Details: its Description has the keyboard.
    for _ in 0..5 {
        if mg_ui::palette_view::arrow_keys_are(&h.ctx) == "HakDetails" {
            break;
        }
        h.key_press(egui::Key::F6);
        settle(&mut h);
    }
    assert_eq!(mg_ui::palette_view::arrow_keys_are(&h.ctx), "HakDetails", "{seen:?}");
    assert!(description(&h).is_focused(), "the Description has the keyboard");
}

/// Details is never taller where it is wider (the U14 rule: a scrolled
/// panel taller where wider never rests), its rows' height swept over the
/// Right Dock's widths at three text sizes, the window resting at each.
#[test]
fn details_is_never_taller_where_wider() {
    let (mut h, _, _) = hak_window(Mode::Tabs, "ui-u21-tall", 1600.0, None, false);
    // (Wide enough at each size for Preview and Details to stand stacked:
    // under 1400 points they share one tab bar.)
    h.set_size(egui::vec2(2200.0, 1300.0));
    for scale in [1.0_f32, 1.25, 1.5] {
        h.ctx.set_zoom_factor(scale);
        let size = scale;
        let mut seen: Vec<(f32, f32)> = Vec::new();
        for width in (280..=480).step_by(8) {
            right_dock_width(&mut h, width as f32);
            settle(&mut h);
            let top = h.get_by_label("Contents").rect().top();
            let foot = h.get_by_label("For people: the game doesn't show it.").rect().bottom();
            let high = foot - top;
            // (Against every narrower width, not the last alone.)
            for (w, was) in &seen {
                assert!(high <= was + 0.5, "{size:?}: {high} at {width} against {was} at {w}");
            }
            seen.push((width as f32, high));
        }
    }
}

/// Only the rows in sight are laid out, in the table and in the Outliner: a
/// hak of 3,000 resources of 40 types in a window 700 high.
#[test]
fn only_the_rows_in_sight_are_laid_out() {
    use mg_module::hak_edit::{Hak, Source};
    let types: Vec<ResType> =
        ResType::ALL.iter().copied().filter(|t| t.extension().is_some()).take(40).collect();
    assert_eq!(types.len(), 40);
    let mut hak = Hak::new();
    let items = (0..3000)
        .map(|i| {
            let key = ResKey::parse(&format!("zz_{i:04}"), types[i % types.len()]).unwrap();
            (key, Source::Bytes(b"x".to_vec().into()))
        })
        .collect();
    hak.add_sources(items).unwrap();
    let path = mg_testkit::scratch_dir("ui-u21-sight").join("zz_many.hak");
    hak.save(Some(&path)).unwrap();
    let mut app = app_in(Mode::Tabs, Vec::new());
    app.settings.no_last_area = true;
    let mut h = harness(app, 1600.0, 700.0);
    settle(&mut h);
    let id = hv::open_at(h.state_mut(), &path).unwrap();
    settle(&mut h);
    assert_eq!(doc(&h, id).hak.items().len(), 3000);
    let laid = |h: &Harness<'_, Moonglow>, group: &str, first: char| {
        h.get_by_label(group)
            .query_all(egui_kittest::kittest::by())
            .filter(|n| n.accesskit_node().label().is_some_and(|l| l.starts_with(first)))
            .count()
    };
    let rows = laid(&h, hv::HAK_TABLE, 'z');
    assert!(rows > 10 && rows < 60, "{rows} rows of 3,000 laid out in the table");
    let types_laid = laid(&h, mg_ui::OUTLINER, '.');
    assert!(types_laid > 3 && types_laid < 30, "{types_laid} types of 40 laid out in the Outliner");
}

/// More's Rename… and Edit Table act on the resource selected: Rename… puts
/// its field in its row (brought into sight), a name typed and Enter renames
/// it, one step; Edit Table opens a 2DA selected in the 2DA editor, the
/// hak its home.
#[test]
fn more_s_rename_and_edit_table_act() {
    let (mut h, _, id) = hak_window(Mode::Tabs, "ui-u21-more", 1600.0, None, false);
    doc_mut(&mut h, id).choose(key("zz_notes.txt"));
    settle(&mut h);
    hak_tool(&mut h, hv::RENAME).click();
    settle(&mut h);
    assert_eq!(doc(&h, id).rename.as_ref().map(|r| r.0), Some(key("zz_notes.txt")));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    let input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    let table = h.get_by_label(hv::HAK_TABLE);
    table.get_all_by_value("zz_notes").find(input).expect("its field").type_text("zz_said");
    h.key_press(egui::Key::Enter);
    settle(&mut h);
    let names: Vec<String> = doc(&h, id).hak.items().iter().map(|i| i.key.to_string()).collect();
    assert!(names.contains(&"zz_said.txt".to_string()), "{names:?}");
    assert_eq!(doc(&h, id).selected, [key("zz_said.txt")].into(), "chosen as renamed");
    doc_mut(&mut h, id).choose(key("zz_table.2da"));
    settle(&mut h);
    hak_tool(&mut h, hv::EDIT_TABLE).click();
    settle(&mut h);
    let table = h.state().tables.first().expect("the 2DA editor open");
    assert_eq!(table.key, key("zz_table.2da"));
    assert!(h.state().dock.find_tab(&Tab::Table(table.id)).is_some(), "its tab");
}

/// The notice strip says a hak of the game's is the game's (the board's
/// words, read by a screen reader), and nothing for a hak of ours.
#[test]
fn the_notice_says_a_hak_is_the_game_s() {
    let (h, _, _) = hak_window(Mode::Tabs, "ui-u21-notice", 1600.0, None, false);
    assert!(h.query_by_label_contains("part of the game").is_none(), "ours: no notice");
    let root = mg_testkit::corpus!();
    let mut app = game_app(&root, None);
    app.settings = Mode::Tabs.settings();
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    hv::open_at(h.state_mut(), &root.join("data/hk/id_resources.hak")).expect("the game's hak");
    settle(&mut h);
    let notice = h.get_by_label(hv::GAME_NOTICE).rect();
    let header = mg_uikit::header::records(&h.ctx, HAK_HEADER).remove(0).row;
    assert!(notice.top() >= header.bottom() - 0.5, "under the header");
    // Changed, the game's hak still has no Save; Save As… keeps a copy.
    let file = mg_testkit::scratch_dir("ui-u21-notice-file").join("zz_added.txt");
    std::fs::write(&file, "added").unwrap();
    h.state_mut().haks[0].hak.add_files(&[file]);
    settle(&mut h);
    assert!(h.state().haks[0].is_dirty(), "changed");
    assert!(hak_tool(&mut h, hv::SAVE).accesskit_node().is_disabled(), "Save off");
    assert!(!hak_tool(&mut h, hv::SAVE_AS).accesskit_node().is_disabled(), "Save As… on");
}

/// The past-2 GiB notice (the board's Not Drawn warning, kept in the strip):
/// a file of 2.1 GB (sparse: no room taken) and one after it, which would
/// start past the 2 GiB the game reads: the strip says how many.
#[test]
fn the_notice_says_what_starts_past_2_gib() {
    let (mut h, _, id) = hak_window(Mode::Tabs, "ui-u21-past", 1600.0, None, false);
    let past = |h: &Harness<'_, Moonglow>| h.query_by_label_contains("past 2 GiB").is_some();
    assert!(!past(&h), "ours: no warning");
    let dir = mg_testkit::scratch_dir("ui-u21-past-files");
    let big = dir.join("zz_big.txt");
    std::fs::File::create(&big).unwrap().set_len(2_200_000_000).unwrap();
    let after = dir.join("zz_zafter.txt");
    std::fs::write(&after, "after").unwrap();
    doc_mut(&mut h, id).hak.add_files(&[big, after]);
    settle(&mut h);
    let node = h.get_by_label_contains("past 2 GiB");
    let said = node.accesskit_node().label().or(node.accesskit_node().value()).unwrap_or_default();
    assert!(said.contains("resources would start past 2 GiB into the hak"), "{said}");
    assert!(!said.starts_with("0 "), "{said}");
    let at = row(&h, "zz_tex.tga").center();
    h.hover_at(at);
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    settle(&mut h);
    assert!(!past(&h), "undone: gone");
}

/// The Outliner's type keeps what Find held (the coordinator's ruling at
/// U21's review): Find typed, a type clicked (Find ".tga", its row marked),
/// clicked again: what was typed back; a type, another type, the second
/// again: what was typed back; a type, then Find typed over: the builder's
/// own text, nothing put back, no row marked.
#[test]
fn a_type_chosen_keeps_what_find_held() {
    let (mut h, _, id) = hak_window(Mode::Tabs, "ui-u21-find-kept", 1600.0, None, false);
    let marked = |h: &Harness<'_, Moonglow>, name: &str| {
        let n = h.get_by_label(mg_ui::OUTLINER).get_by_label(name);
        n.accesskit_node().toggled() == Some(egui::accesskit::Toggled::True)
            || n.accesskit_node().is_selected() == Some(true)
    };
    let tga = ".tga, 2 resources";
    let mdl = ".mdl, 1 resource";
    doc_mut(&mut h, id).filter = "zz_t".into();
    settle(&mut h);
    let click = |h: &mut Harness<'_, Moonglow>, name: &str| {
        h.get_by_label(mg_ui::OUTLINER).get_by_label(name).click();
        settle(h);
    };
    click(&mut h, tga);
    assert_eq!(doc(&h, id).filter, ".tga");
    assert!(marked(&h, tga), "its row marked");
    click(&mut h, tga);
    assert_eq!(doc(&h, id).filter, "zz_t", "what was typed back");
    // A type, another, the second again.
    click(&mut h, tga);
    click(&mut h, mdl);
    assert_eq!(doc(&h, id).filter, ".mdl");
    click(&mut h, mdl);
    assert_eq!(doc(&h, id).filter, "zz_t", "what was typed back after two types");
    // A type, then Find typed over: kept as typed.
    click(&mut h, tga);
    hak_tool(&mut h, hv::FIND).click();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_by_label(HAK_HEADER).get_by_label(hv::FIND).type_text("zz_box");
    settle(&mut h);
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    assert!(!marked(&h, tga), "no row marked: Find is not a type's");
    click(&mut h, mdl);
    click(&mut h, mdl);
    assert_eq!(doc(&h, id).filter, "zz_box", "the builder's text, not the old one");
}

/// Equal counts in the Outliner go by the type's number, as the board's
/// .are before .gic, .dlg before .utw, .utt before .utm.
#[test]
fn the_outliner_s_equal_counts_go_by_type() {
    use mg_module::hak_edit::{Hak, Source};
    let names = ["zz_a.gic", "zz_a.are", "zz_a.utw", "zz_a.dlg", "zz_a.utm", "zz_a.utt"];
    let mut hak = Hak::new();
    let items = names.iter().map(|n| (key(n), Source::Bytes(b"x".to_vec().into()))).collect();
    hak.add_sources(items).unwrap();
    let path = mg_testkit::scratch_dir("ui-u21-ties").join("zz_ties.hak");
    hak.save(Some(&path)).unwrap();
    let mut app = app_in(Mode::Tabs, Vec::new());
    app.settings.no_last_area = true;
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    hv::open_at(h.state_mut(), &path).unwrap();
    settle(&mut h);
    let o = h.get_by_label(mg_ui::OUTLINER);
    let top = |name: &str| o.get_by_label(&format!("{name}, 1 resource")).rect().top();
    assert!(top(".are") < top(".gic"), ".are before .gic");
    assert!(top(".dlg") < top(".utw"), ".dlg before .utw");
    assert!(top(".utt") < top(".utm"), ".utt before .utm");
}

/// Find's clear × stands inside the field's box, as the board draws it.
#[test]
fn find_s_clear_button_is_inside_its_field() {
    let (mut h, _, id) = hak_window(Mode::Tabs, "ui-u21-clear", 1600.0, None, false);
    doc_mut(&mut h, id).filter = ".tga".into();
    settle(&mut h);
    let header = h.get_by_label(HAK_HEADER);
    let field = header.get_by_label(hv::FIND).rect();
    let clear = header.get_by_label("Clear").rect();
    assert!(field.contains_rect(clear), "{clear:?} in {field:?}");
    assert!(field.width() <= 196.5, "the board's 196: {}", field.width());
    header.get_by_label("Clear").click();
    settle(&mut h);
    assert_eq!(doc(&h, id).filter, "", "cleared");
}

/// A sound the Preview plays stops when its Preview is drawn no more (the
/// Right Dock folded; another tab in front; its hak's tab closed; another
/// module opened), and as it ends the window stops asking for frames
/// (while it plays it asks, every 300 ms).
#[test]
fn a_hak_s_sound_never_plays_on_unseen() {
    use mg_ui::audio::Channel;
    let asks = |h: &Harness<'_, Moonglow>| {
        // (The sound viewer's, shared with the read-only tab since M5's W6.)
        h.ctx.repaint_causes().iter().any(|c| c.file.ends_with("viewers.rs"))
    };
    let playing = |h: &Harness<'_, Moonglow>| h.state().speaker.playing(Channel::Preview);
    for way in ["fold", "behind", "close", "module", "ends"] {
        let (mut h, _, id) =
            hak_window(Mode::Tabs, &format!("ui-u21-sound-{way}"), 1600.0, None, true);
        click_pos(&h, row(&h, "zz_sound.wav").center());
        settle(&mut h);
        h.get_by_label("Play").click();
        h.run_steps(2);
        assert!(playing(&h), "{way}: playing");
        assert!(asks(&h), "{way}: it asks to look again while it plays");
        match way {
            "fold" => h.state_mut().settings.hide_palettes = true,
            "behind" => {
                h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::ModuleProperties));
            }
            "close" => h.state_mut().actions.push(mg_ui::Action::CloseTabs(vec![Tab::Hak(id)])),
            "module" => {
                let dir = mg_testkit::scratch_dir(&format!("ui-u21-sound-other-{way}"));
                h.state_mut().open_module(&sample_module(&dir));
            }
            _ => h.state_mut().speaker.stop(Channel::Preview),
        }
        settle(&mut h);
        assert!(!playing(&h), "{way}: stopped");
        h.run_steps(2);
        assert!(!asks(&h), "{way}: no more asks");
    }
}

/// Undo and Redo with a Description typed and not yet set (the
/// coordinator's ruling): nothing to redo while it is pending (the Top
/// Bar's Redo off); the Top Bar's Undo sets it and undoes it, one step; with
/// the field holding the keyboard, Ctrl+Z is the field's own and sets
/// nothing in the hak.
#[test]
fn undo_and_redo_with_a_description_pending() {
    let (mut h, _, id) = hak_window(Mode::Tabs, "ui-u21-pending", 1600.0, None, false);
    let set = |h: &mut Harness<'_, Moonglow>, text: &str| {
        let at = description(h).rect().center();
        click_pos(h, at);
        settle(h);
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        description(h).type_text(text);
        // (A frame more: the Top Bar, drawn first, sees it typed.)
        h.run_steps(2);
    };
    set(&mut h, "One");
    click_pos(&h, h.get_by_label("Built from").rect().center());
    settle(&mut h);
    assert_eq!(doc(&h, id).hak.description, "One");
    h.get_by_label("Top Bar").get_by_label("Undo").click();
    settle(&mut h);
    assert_eq!(doc(&h, id).hak.description, "Ours");
    assert_eq!(doc(&h, id).hak.redo_label(), Some("Description"), "a step to redo");
    // Typed and pending: Redo off.
    set(&mut h, "Two");
    let redo = h.get_by_label("Top Bar").get_by_label("Redo");
    assert!(redo.accesskit_node().is_disabled(), "nothing to redo while typing is pending");
    // The field's own Ctrl+Z: the hak untouched.
    let steps = doc(&h, id).hak.changes();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(1);
    assert_eq!(doc(&h, id).hak.changes(), steps, "the field's own undo");
    // The Top Bar's Undo: set, then undone.
    set(&mut h, "Three");
    h.get_by_label("Top Bar").get_by_label("Undo").click();
    settle(&mut h);
    assert_eq!(doc(&h, id).hak.description, "Ours", "set and undone, one step");
    assert_eq!(doc(&h, id).hak.redo_label(), Some("Description"));
}

/// Delete and Backspace while Find, the Description or the rename field
/// holds the keyboard, or with the Outliner holding the keys, take no
/// resource away.
#[test]
fn delete_while_typing_takes_nothing_away() {
    let (mut h, _, id) = hak_window(Mode::Tabs, "ui-u21-delete", 1600.0, None, false);
    let count = |h: &Harness<'_, Moonglow>| doc(h, id).hak.items().len();
    click_pos(&h, row(&h, "zz_box.mdl").center());
    settle(&mut h);
    let press = |h: &mut Harness<'_, Moonglow>| {
        h.key_press(egui::Key::Delete);
        h.key_press(egui::Key::Backspace);
        settle(h);
    };
    press(&mut h);
    assert_eq!(count(&h), 6, "the table's keys");
    hak_tool(&mut h, hv::FIND).click();
    h.get_by_label(HAK_HEADER).get_by_label(hv::FIND).type_text("zz");
    press(&mut h);
    assert_eq!(count(&h), 6, "Find");
    h.key_press(egui::Key::Escape);
    let at = description(&h).rect().center();
    click_pos(&h, at);
    settle(&mut h);
    press(&mut h);
    assert_eq!(count(&h), 6, "the Description");
    doc_mut(&mut h, id).filter.clear();
    doc_mut(&mut h, id).choose(key("zz_box.mdl"));
    settle(&mut h);
    hak_tool(&mut h, hv::RENAME).click();
    settle(&mut h);
    press(&mut h);
    assert_eq!(count(&h), 6, "the rename field");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    h.get_by_label(mg_ui::OUTLINER).get_by_label(".tga, 2 resources").click();
    settle(&mut h);
    press(&mut h);
    assert_eq!(count(&h), 6, "the Outliner");
}

/// A hak's model view outlives no Preview (the coordinator's ruling: as
/// other unseen views, after 2 s): put away by Close, and with the Right
/// Dock folded.
#[test]
fn a_hak_s_model_view_goes_when_unseen() {
    let root = mg_testkit::corpus!();
    let Some(rs) = gpu() else { return };
    let path = hak_file("ui-u21-unseen", "zz_ours");
    let mut app = game_app(&root, None);
    app.settings = Mode::Tabs.settings();
    app.set_render_state(rs.clone());
    let mut h = gpu_harness(app, rs, 1600.0, 1200.0);
    settle(&mut h);
    let id = hv::open_at(h.state_mut(), &path).unwrap();
    settle(&mut h);
    let views = |h: &Harness<'_, Moonglow>| {
        let v = h.state().model_views.keys();
        v.filter(|s| matches!(s, mg_ui::model_view::Source::Hak { .. })).count()
    };
    let gone = |h: &mut Harness<'_, Moonglow>, what: &str| {
        let started = std::time::Instant::now();
        while views(h) > 0 {
            h.run_steps(30);
            assert!(started.elapsed() < std::time::Duration::from_secs(30), "{what}: kept");
        }
    };
    for way in ["close", "fold"] {
        doc_mut(&mut h, id).choose(key("zz_box.mdl"));
        h.state_mut().settings.hide_palettes = false;
        settle(&mut h);
        assert_eq!(views(&h), 1, "{way}: its view");
        match way {
            "close" => h.get_by_label(hv::PREVIEW_CLOSE).click(),
            _ => h.state_mut().settings.hide_palettes = true,
        }
        settle(&mut h);
        gone(&mut h, way);
    }
}

/// A long path is cut cleanly in Details (whole folders dropped from its
/// start, "…" and the separator before what is left, the path's own, which
/// on Windows CI mixes `\\` and `/`; whole in its tip), never in the middle
/// of a folder's name.
#[test]
fn details_cuts_a_long_path_by_its_folders() {
    let dir = format!(
        "ui-u21-long/{}",
        "a folder with a long name to make the path wider".replace(' ', "_")
    );
    let path = hak_file(&dir, "zz_ours");
    let mut app = app_in(Mode::Tabs, Vec::new());
    app.settings.no_last_area = true;
    let mut h = harness(app, 1600.0, 900.0);
    settle(&mut h);
    hv::open_at(h.state_mut(), &path).unwrap();
    settle(&mut h);
    let dock = right_dock_rect(&h);
    let shown: Vec<String> = h
        .get_all_by_label_contains("zz_ours.hak")
        .filter(|n| n.accesskit_node().bounding_box().is_some())
        .filter(|n| dock.contains_rect(n.rect()))
        .filter_map(|n| n.accesskit_node().label().or(n.accesskit_node().value()))
        .filter(|l| l.starts_with('…'))
        .collect();
    let cut = shown.first().unwrap_or_else(|| panic!("the File cut: {shown:?}"));
    let after = cut["…".len()..].chars().next();
    assert!(after.is_some_and(std::path::is_separator), "cut by its folders: {cut}");
    let whole = path.display().to_string();
    assert!(whole.ends_with(&cut["…".len()..]), "{cut} the end of {whole}");
}
