//! Fields and lists: line ends, typing, undo, Cancel, number arrows,
//! dropdowns and the arrow keys through lists.

use crate::common::*;

/// The classic clipboard keys, Aurora's and every Windows program's, in a
/// text field: Ctrl+Insert copies, Shift+Delete cuts and Shift+Insert
/// pastes, as Ctrl+C, Ctrl+X and Ctrl+V do (a builder's report).
#[test]
fn insert_and_delete_keys_copy_cut_and_paste_text() {
    let dir = mg_testkit::scratch_dir("ui-cua-text");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let is_input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    let field =
        |h: &Harness<'_, Moonglow>, text: &str| h.query_all_by_value(text).any(|n| is_input(&n));
    h.get_all_by_value("SAMPLE").find(is_input).expect("the tag's text field").click();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.run();
    use egui::{Key, Modifiers};
    // Ctrl+Insert: copied, the text as it was.
    assert_eq!(press_now(&mut h, Modifiers::COMMAND, Key::Insert), (vec!["SAMPLE".into()], false));
    assert!(field(&h, "SAMPLE"));
    // Shift+Delete: cut (copied, and taken out), not a character deleted.
    assert_eq!(press_now(&mut h, Modifiers::SHIFT, Key::Delete), (vec!["SAMPLE".into()], false));
    assert!(!field(&h, "SAMPLE") && !field(&h, "SAMPL") && !field(&h, "AMPLE"));
    // Shift+Insert: the clipboard's text asked for (the Insert key itself
    // goes nowhere); the backend sends it as a paste.
    assert_eq!(press_now(&mut h, Modifiers::SHIFT, Key::Insert), (vec![], true));
    h.event(egui::Event::Paste("PASTED".into()));
    h.run();
    assert!(field(&h, "PASTED"));
    h.key_press(Key::Tab);
    h.run();
    let ws = h.state_mut().ws.as_mut().unwrap();
    let g = ws.doc(&ResKey::parse("module", ResType::IFO).unwrap()).unwrap();
    assert_eq!(g.root.read(&ifo::MOD_TAG).as_bytes(), b"PASTED");
}

/// A color pasted into a color button (a builder's report: the picker's
/// 📋 copies "158, 151, 211", and nothing pasted one back): with the
/// picker open, the paste its Paste button asks for, or Ctrl+V; with the
/// pointer on the swatch, Ctrl+V; text that isn't a color changes nothing.
#[test]
fn a_color_is_pasted_into_a_color_button() {
    let swatch = |h: &Harness<'_, [u8; 3]>| {
        h.get_by(|n| n.role() == egui::accesskit::Role::ColorWell).rect().center()
    };
    let mut h = Harness::new_ui_state(
        |ui, rgb: &mut [u8; 3]| {
            mg_ui::color_button::edit_srgb(ui, rgb);
        },
        [10, 20, 30],
    );
    h.run();
    // Pointed at, Ctrl+V.
    h.hover_at(swatch(&h));
    h.run();
    h.event(egui::Event::Paste("158, 151, 211".into()));
    h.run();
    assert_eq!(*h.state(), [158, 151, 211]);
    // Not a color: nothing.
    h.event(egui::Event::Paste("hello".into()));
    h.run();
    assert_eq!(*h.state(), [158, 151, 211]);
    // Open: its Paste button, and the clipboard as the next frame's paste.
    h.get_by(|n| n.role() == egui::accesskit::Role::ColorWell).click();
    h.run();
    h.get_by_label("Paste Color").click();
    h.run();
    h.event(egui::Event::Paste("#204060".into()));
    h.run();
    assert_eq!(*h.state(), [0x20, 0x40, 0x60]);
}

#[test]
fn multi_line_text_keeps_its_line_ends() {
    let dir = mg_testkit::scratch_dir("ui-crlf");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    // (The module opens on its Properties, not on an area.)
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Description").click();
    h.run();
    // Shown with plain line breaks in a multi-line box.
    let is_input = |n: &egui_kittest::Node<'_>| {
        n.accesskit_node().role() == egui::accesskit::Role::MultilineTextInput
    };
    h.get_all_by_value("Line one\nLine two")
        .find(is_input)
        .expect("the description's text box")
        .click();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_all_by_value("Line one\nLine two")
        .find(is_input)
        .unwrap()
        .type_text("First\nSecond\nThird");
    h.run();
    h.key_press(egui::Key::Escape);
    h.run();
    let ws = h.state_mut().ws.as_mut().unwrap();
    let g = ws.doc(&ResKey::parse("module", ResType::IFO).unwrap()).unwrap();
    let desc: LocString = g.root.read(&ifo::MOD_DESCRIPTION);
    assert_eq!(desc.get(Language::ENGLISH, Gender::Male), Some(&b"First\r\nSecond\r\nThird"[..]));
}

/// The sample module with `description` for its description, open on its
/// Description page in a harness `size` big; and the description's box.
fn module_description_harness(
    name: &str,
    description: &str,
    size: egui::Vec2,
) -> Harness<'static, Moonglow> {
    let dir = mg_testkit::scratch_dir(name);
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let mut info = m.info().unwrap();
    info.root.write(
        &ifo::MOD_DESCRIPTION,
        LocString::from_text(Language::ENGLISH, Gender::Male, description),
    );
    m.set_info(&info).unwrap();
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h =
        Harness::builder().with_size(size).build_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    h.get_by_label("Description").click();
    h.run();
    h
}

/// The module's description as stored.
fn module_description(h: &mut Harness<'_, Moonglow>) -> String {
    let ws = h.state_mut().ws.as_mut().unwrap();
    let g = ws.doc(&ResKey::parse("module", ResType::IFO).unwrap()).unwrap();
    let desc: LocString = g.root.read(&ifo::MOD_DESCRIPTION);
    String::from_utf8(desc.get(Language::ENGLISH, Gender::Male).unwrap().to_vec()).unwrap()
}

fn multiline_input<'a>(h: &'a Harness<'_, Moonglow>, value: &'a str) -> egui_kittest::Node<'a> {
    h.get_all_by_value(value)
        .find(|n| n.accesskit_node().role() == egui::accesskit::Role::MultilineTextInput)
        .unwrap_or_else(|| panic!("a text box of {value:?}"))
}

#[test]
fn page_keys_move_through_a_description() {
    // Sacha's report: Page Up and Page Down did nothing in the text fields
    // of several lines (egui's own field has none).
    let text: String = (0..120).map(|n| format!("Line {n}\r\n")).collect();
    let shown = text.replace('\r', "");
    let mut h = module_description_harness("ui-page-desc", &text, egui::vec2(1000.0, 600.0));
    // (A click on its first lines: the box is taller than the window.)
    let at = multiline_input(&h, &shown).rect().left_top() + egui::vec2(40.0, 8.0);
    h.hover_at(at);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run();
    h.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::Home);
    h.run();
    let heading = |h: &Harness<'_, Moonglow>| h.get_by_label_contains("Description (").rect().top();
    let top = heading(&h);
    let typed = |h: &mut Harness<'_, Moonglow>, c: &str| {
        h.event(egui::Event::Text(c.into()));
        h.run();
        let n = h.get_all_by_role(egui::accesskit::Role::MultilineTextInput).find(|n| {
            n.accesskit_node().value().is_some_and(|v| v.contains(c) && v.contains("Line 119"))
        });
        let Some(n) = n else {
            let all: Vec<String> = (h.get_all_by_role(egui::accesskit::Role::MultilineTextInput))
                .map(|n| format!("{:.80?}", n.accesskit_node().value()))
                .collect();
            panic!("no box with {c}: {all:?}");
        };
        let v = n.accesskit_node().value().unwrap().to_string();
        v.lines().position(|l| l.contains(c)).unwrap()
    };
    // Two pages down: as many lines on, the box scrolled with the caret.
    h.key_press(egui::Key::PageDown);
    h.run();
    h.key_press(egui::Key::PageDown);
    h.run();
    let down = typed(&mut h, "X");
    assert!(down >= 10 && down % 2 == 0 && down < 100, "two pages down: line {down}");
    assert!(heading(&h) < top - 100.0, "scrolled: {} from {top}", heading(&h));
    // A page up, from before the X: a page back (the lines in sight now,
    // the box filling the page).
    h.key_press(egui::Key::ArrowLeft);
    h.key_press(egui::Key::PageUp);
    h.run();
    let up = typed(&mut h, "Y");
    assert!(up + 10 <= down, "a page up: line {up}");
    // With Shift, the selection: typed over, the lines go with it.
    h.key_press(egui::Key::ArrowLeft);
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::PageDown);
    h.run();
    assert_eq!(typed(&mut h, "Z"), up);
    h.key_press(egui::Key::Escape);
    h.run();
    let stored = module_description(&mut h);
    assert!(!stored.contains('Y') && stored.lines().count() <= 110, "{stored:.300}");
    // The lines kept keep their CRLF.
    assert!(stored.split_terminator('\n').all(|l| l.ends_with('\r')), "{stored:?}");
}

#[test]
fn a_description_with_both_line_ends_keeps_each_line_s() {
    // Jasperre's report: a text of several lines from a file with CRLF
    // line ends, or both kinds, keeps them where it isn't changed.
    let mixed = "One\r\nTwo\nThree\r\nFour";
    let mut h = module_description_harness("ui-crlf-mixed", mixed, egui::vec2(1000.0, 700.0));
    let shown = "One\nTwo\nThree\nFour";
    multiline_input(&h, shown).click();
    h.run();
    // A line added at the end: the others keep theirs, the new one takes
    // the text's (CRLF: most of its lines').
    h.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::End);
    h.key_press(egui::Key::Enter);
    h.event(egui::Event::Text("Five".into()));
    // Text pasted with CRLF comes with plain line ends.
    h.event(egui::Event::Paste("\r\nSix".into()));
    h.run();
    multiline_input(&h, "One\nTwo\nThree\nFour\nFive\nSix");
    h.key_press(egui::Key::Escape);
    h.run();
    assert_eq!(module_description(&mut h), "One\r\nTwo\nThree\r\nFour\r\nFive\r\nSix");
}

#[test]
fn a_description_of_one_line_gets_a_plain_line_end() {
    // As Aurora writes one typed in a localized string's text (the module's
    // description, a blueprint's, a conversation line's: docs/findings.md,
    // "Line ends"); its comments get CRLF.
    let mut h = module_description_harness("ui-crlf-new", "One", egui::vec2(1000.0, 700.0));
    multiline_input(&h, "One").click();
    h.run();
    h.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::End);
    h.key_press(egui::Key::Enter);
    h.event(egui::Event::Text("Two".into()));
    h.run();
    h.key_press(egui::Key::Escape);
    h.run();
    assert_eq!(module_description(&mut h), "One\nTwo");
}

/// A blueprint's fields take Aurora's line ends: a line typed into a
/// comment of one line gets CRLF, into a description of one line a plain
/// one; and a one-line field whose stored text has a CRLF (its Tag) keeps
/// it once typed in, as the fields of several lines keep theirs.
#[test]
fn a_blueprint_s_fields_take_aurora_s_line_ends() {
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "chest_ends", ResType::UTP) else {
        return;
    };
    let set = |label: &str, value: mg_gff::Value| mg_edit::Edit::SetField {
        key,
        path: mg_edit::GffPath::root(),
        label: label.into(),
        value: Some(value),
    };
    let desc = LocString::from_text(Language::ENGLISH, Gender::Male, "Desc");
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "fields",
        vec![
            set("Tag", mg_gff::Value::String(b"AB\r\nCD".to_vec())),
            set("Comment", mg_gff::Value::String(b"One".to_vec())),
            set("Description", mg_gff::Value::LocString(desc)),
        ],
    )));
    h.run();
    let click = |h: &mut Harness<'_, Moonglow>, role: egui::accesskit::Role, value: &str| {
        let is = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.role() == role && n.value().as_deref() == Some(value)
        };
        let at = h.get(egui_kittest::kittest::by().predicate(is)).rect().center();
        h.hover_at(at);
        press(h, at, true, egui::Modifiers::NONE);
        press(h, at, false, egui::Modifiers::NONE);
        h.run_steps(2);
    };
    let typed = |h: &mut Harness<'_, Moonglow>, text: &str| {
        h.key_press_modifiers(egui::Modifiers::CTRL, egui::Key::End);
        for (k, line) in text.split('\n').enumerate() {
            if k > 0 {
                h.key_press(egui::Key::Enter);
            }
            h.event(egui::Event::Text(line.into()));
        }
        h.run_steps(2);
    };
    // The Tag, shown with a plain line end, typed in: its CRLF kept.
    click(&mut h, egui::accesskit::Role::TextInput, "AB\nCD");
    typed(&mut h, "X");
    h.get_by_label("Comments").click();
    h.run_steps(3);
    assert_eq!(field(&mut h, &key).string("Tag"), Some(&b"AB\r\nCDX"[..]));
    // The comment: CRLF.
    click(&mut h, egui::accesskit::Role::MultilineTextInput, "One");
    typed(&mut h, "\nTwo");
    h.get_by_label("Description").click();
    h.run_steps(3);
    assert_eq!(field(&mut h, &key).string("Comment"), Some(&b"One\r\nTwo"[..]));
    // The description: plain.
    click(&mut h, egui::accesskit::Role::MultilineTextInput, "Desc");
    typed(&mut h, "\nTwo");
    h.get_by_label("Comments").click();
    h.run_steps(3);
    let desc = field(&mut h, &key).locstring("Description").cloned().unwrap();
    assert_eq!(desc.get(Language::ENGLISH, Gender::Male), Some(&b"Desc\nTwo"[..]));
}

/// A creature's statistics have Aurora's arrows after their numbers: a
/// click is one more or one less, one change each, within the range.
#[test]
fn a_creature_s_statistics_have_arrows_to_step_by_one() {
    let Some((mut h, key)) = blueprint_harness("nw_chicken", "stat_arrows", ResType::UTC) else {
        return;
    };
    h.run();
    h.get_by_label("Statistics").click();
    h.run();
    let strength = |h: &mut Harness<'_, Moonglow>| field(h, &key).integer("Str").unwrap();
    let was = strength(&mut h);
    // (The first pair on the page is Strength's.)
    // (After the module tree's own two, Collapse All and Expand All.)
    let up = h.query_all_by_label("⏶").nth(1).expect("an arrow up").rect().center();
    let down = h.query_all_by_label("⏷").nth(1).expect("an arrow down").rect().center();
    let click = |h: &mut Harness<'_, Moonglow>, at: egui::Pos2| {
        h.hover_at(at);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
        }
        h.run();
    };
    click(&mut h, up);
    click(&mut h, up);
    assert_eq!(strength(&mut h), was + 2);
    click(&mut h, down);
    assert_eq!(strength(&mut h), was + 1);
    // One change each.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(strength(&mut h), was + 2);
}

/// An arrow held down goes on stepping, after a pause, and the whole run
/// is one change, made when it is let go; Up and Down in the field step
/// too; the field's limits hold.
#[test]
fn a_held_arrow_repeats_and_the_keys_step_in_the_field() {
    let Some((mut h, key)) = blueprint_harness("nw_chicken", "stat_held", ResType::UTC) else {
        return;
    };
    h.run();
    h.get_by_label("Statistics").click();
    h.run();
    let int = |h: &mut Harness<'_, Moonglow>, label: &str| field(h, &key).integer(label).unwrap();
    let was = int(&mut h, "Str");
    // (After the module tree's own two, Collapse All and Expand All.)
    let up = h.query_all_by_label("⏶").nth(1).expect("an arrow up").rect().center();
    let button = |pressed| egui::Event::PointerButton {
        pos: up,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    h.hover_at(up);
    h.event(button(true));
    h.run_steps(5); // (A quarter of a second each.)
    assert_eq!(int(&mut h, "Str"), was, "nothing changed while it is held");
    h.event(button(false));
    h.run();
    let after = int(&mut h, "Str");
    assert!(after > was + 2, "it went on stepping: {was} to {after}");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run();
    assert_eq!(int(&mut h, "Str"), was, "the run was one change");
    // The keys, in the field.
    let shown = was.to_string();
    h.get_all_by_value(&shown).next().expect("Strength's field").click();
    h.run();
    h.key_press(egui::Key::ArrowUp);
    h.key_press(egui::Key::ArrowUp);
    h.key_press(egui::Key::ArrowDown);
    h.run();
    assert_eq!(int(&mut h, "Str"), was + 1);
    // Natural AC is 0 at the least: its arrow down is off.
    assert_eq!(int(&mut h, "NaturalAC"), 0, "the chicken has no natural armor");
    // (The tree's, Strength to Charisma's, then Natural AC's.)
    assert!(h.query_all_by_label("⏷").nth(7).is_some_and(|n| n.accesskit_node().is_disabled()));
    assert!(!h.query_all_by_label("⏶").nth(7).is_some_and(|n| n.accesskit_node().is_disabled()));
}

/// Aurora's arrows are on the other editors' number fields too: a
/// placeable's hardness, saves and hit points, a lock's DCs.
#[test]
fn a_placeable_s_numbers_have_arrows() {
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "plc_arrows", ResType::UTP) else {
        return;
    };
    h.run();
    let int = |h: &mut Harness<'_, Moonglow>, label: &str| field(h, &key).integer(label).unwrap();
    let hardness = int(&mut h, "Hardness");
    // (Hit points' arrows first, then hardness's.)
    let up = h.query_all_by_label("⏶").nth(2).expect("hardness's arrow up").rect().center();
    h.hover_at(up);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: up,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run();
    assert_eq!(int(&mut h, "Hardness"), hardness + 1);
    h.get_by_label("Lock").click();
    h.run();
    assert!(h.query_all_by_label("⏶").count() >= 3, "the lock's DCs have arrows");
}

/// GitHub issue 18: Ctrl+Z while a text field has the keyboard is the
/// field's (its typing is taken back), not the module's Undo, which undid
/// an earlier change somewhere else; with no field in hand it is the
/// module's again.
#[test]
fn undo_in_a_text_field_is_the_field_s() {
    let dir = mg_testkit::scratch_dir("ui-undo-in-text");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 800.0);
    h.run();
    let key = ResKey::parse("mg_note", ResType::NSS).unwrap();
    let edit = mg_edit::Edit::SetResource { key, data: Some(b"void main() {}\n".to_vec()) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Earlier", vec![edit])));
    h.run();
    let undoable =
        |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().can_undo().map(str::to_owned);
    assert_eq!(undoable(&h).as_deref(), Some("Earlier"));
    // Typing in a field (the module tree's Filter), then Ctrl+Z and
    // Ctrl+Y: the module's history is as it was.
    h.get_all_by_role(egui::accesskit::Role::TextInput).next().unwrap().focus();
    h.run();
    filter_type(&mut h, "ab");
    assert!(h.ctx.egui_wants_keyboard_input());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(undoable(&h).as_deref(), Some("Earlier"), "not the module's Undo");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Y);
    h.run_steps(2);
    assert_eq!(undoable(&h).as_deref(), Some("Earlier"));
    // With no field in hand, Ctrl+Z is the module's. (Escape clears the
    // Filter; again, it leaves it.)
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    assert!(!h.ctx.egui_wants_keyboard_input());
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(undoable(&h), None, "the module's Undo");
}

/// Text typed in a Properties field is kept when the field is left by a
/// click on another page, or by closing the window: the field is gone
/// before the keyboard leaves it, and what was typed was lost.
#[test]
fn text_typed_in_a_properties_field_is_kept_when_its_page_is_left() {
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "chest_typed", ResType::UTP) else {
        return;
    };
    h.run();
    let input = |h: &Harness<'_, Moonglow>, value: &str| {
        let is = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.role() == egui::accesskit::Role::TextInput && n.value().as_deref() == Some(value)
        };
        h.get(egui_kittest::kittest::by().predicate(is)).rect().center()
    };
    let tag = field(&mut h, &key).string("Tag").map(|t| String::from_utf8_lossy(t).into_owned());
    let tag = tag.unwrap_or_default();
    // The Tag, typed in, and another page clicked.
    let at = input(&h, &tag);
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(2);
    h.key_press(egui::Key::End);
    h.event(egui::Event::Text("_X".into()));
    h.run_steps(2);
    assert_eq!(field(&mut h, &key).string("Tag"), Some(tag.as_bytes()), "not set while typed");
    h.get_by_label("Lock").click();
    h.run_steps(4);
    let now = format!("{tag}_X");
    assert_eq!(field(&mut h, &key).string("Tag"), Some(now.as_bytes()), "kept");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Tag"));
    // A script's name, typed in, and another page clicked: it was lost
    // (the script fields kept no part of the fix above).
    h.get_by_label("Scripts").click();
    h.run_steps(3);
    let is = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::TextInput
            && n.value().as_deref() == Some("nw_o2_classlow")
    };
    let at = h.get_all(egui_kittest::kittest::by().predicate(is)).next().unwrap().rect().center();
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(2);
    h.key_press(egui::Key::End);
    h.event(egui::Event::Text("_x".into()));
    h.run_steps(2);
    h.get_by_label("Lock").click();
    h.run_steps(4);
    let scripts = ["OnOpen", "OnDeath"].map(|l| field(&mut h, &key).resref(l).unwrap().to_string());
    assert!(scripts.contains(&"nw_o2_classlow_x".to_string()), "kept: {scripts:?}");
    // The description, typed in, and the window closed.
    h.get_by_label("Description").click();
    h.run_steps(3);
    let memo = egui_kittest::kittest::by().role(egui::accesskit::Role::MultilineTextInput);
    let at = h.get(memo).rect().center();
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(2);
    h.event(egui::Event::Text("Sturdy. ".into()));
    h.run_steps(2);
    close_tab(h.state_mut(), &Tab::Blueprint(key));
    h.run_steps(4);
    let description = field(&mut h, &key).locstring("Description").cloned().unwrap();
    let text: Vec<String> =
        description.strings.iter().map(|(_, t)| String::from_utf8_lossy(t).into_owned()).collect();
    assert!(text.iter().any(|t| t.contains("Sturdy. ")), "{text:?}");
    // A field left with nothing typed sets nothing.
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Description"));
}

/// Text typed in a field is kept while another program has the keyboard
/// (Alt + Tab away and back): the field took its window's loss of the
/// keyboard for its own, and showed what was stored again.
#[test]
fn text_typed_is_kept_while_another_program_has_the_keyboard() {
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "chest_away", ResType::UTP) else {
        return;
    };
    h.run();
    let tag = field(&mut h, &key).string("Tag").map(|t| String::from_utf8_lossy(t).into_owned());
    let tag = tag.unwrap_or_default();
    let is = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::TextInput && n.value().as_deref() == Some(&*tag)
    };
    let at = h.get(egui_kittest::kittest::by().predicate(is)).rect().center();
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(2);
    h.key_press(egui::Key::End);
    h.event(egui::Event::Text("_X".into()));
    h.run_steps(2);
    for focused in [false, true] {
        h.input_mut().focused = focused;
        h.event(egui::Event::WindowFocused(focused));
        h.run_steps(3);
    }
    h.get_by_label("Lock").click();
    h.run_steps(4);
    let now = format!("{tag}_X");
    assert_eq!(field(&mut h, &key).string("Tag"), Some(now.as_bytes()), "kept");
}

/// A Properties window has OK and Cancel, as Aurora's: Cancel sets back
/// what the window changed since it was opened, as one step, and closes
/// it; what was changed elsewhere meanwhile stays; OK keeps it all.
#[test]
fn a_properties_window_s_cancel_sets_back_what_it_changed() {
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "chest_cancel", ResType::UTP) else {
        return;
    };
    h.run();
    let tab = Tab::Blueprint(key);
    let tag = |h: &mut Harness<'_, Moonglow>| {
        String::from_utf8_lossy(field(h, &key).string("Tag").unwrap_or_default()).into_owned()
    };
    let type_tag = |h: &mut Harness<'_, Moonglow>, more: &str| {
        let was = tag(h);
        let is = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.role() == egui::accesskit::Role::TextInput && n.value().as_deref() == Some(&*was)
        };
        let at = h.get(egui_kittest::kittest::by().predicate(is)).rect().center();
        h.hover_at(at);
        press(h, at, true, egui::Modifiers::NONE);
        press(h, at, false, egui::Modifiers::NONE);
        h.run_steps(2);
        h.key_press(egui::Key::End);
        h.event(egui::Event::Text(more.into()));
        h.run_steps(2);
    };
    let before = field(&mut h, &key);
    let original = tag(&mut h);
    // Two changes in the window, and one made elsewhere.
    type_tag(&mut h, "_X");
    h.get_by_label("Plot").click();
    h.run_steps(3);
    assert_eq!(tag(&mut h), format!("{original}_X"));
    assert_ne!(field(&mut h, &key).integer("Plot"), before.integer("Plot"));
    let elsewhere = mg_edit::Edit::SetField {
        key,
        path: mg_edit::GffPath::root(),
        label: "Hardness".into(),
        value: Some(mg_gff::Value::Byte(77)),
    };
    let elsewhere = mg_edit::Command::new("Elsewhere", vec![elsewhere]);
    h.state_mut().actions.push(mg_ui::Action::Apply(elsewhere));
    h.run_steps(3);
    h.get_by_label("Cancel").click();
    h.run_steps(4);
    let now = field(&mut h, &key);
    assert_eq!(tag(&mut h), original, "the tag is set back");
    assert_eq!(now.integer("Plot"), before.integer("Plot"), "and Plot");
    assert_eq!(now.integer("Hardness"), Some(77), "what was changed elsewhere stays");
    assert_eq!(h.state().ws.as_ref().unwrap().can_undo(), Some("Cancel Changes"));
    assert!(h.state().dock.find_tab(&tab).is_none(), "closed");
    // Opened again: nothing of the last time is its to set back; OK keeps
    // what is changed.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
    h.run_steps(4);
    type_tag(&mut h, "_Y");
    h.get_by_label("OK").click();
    h.run_steps(4);
    assert_eq!(tag(&mut h), format!("{original}_Y"), "kept");
    assert!(h.state().dock.find_tab(&tab).is_none(), "closed");
    // Cancel with text typed and its field not left yet: the press sets
    // the text, and Cancel sets it back.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
    h.run_steps(4);
    type_tag(&mut h, "_Z");
    h.get_by_label("Cancel").click();
    h.run_steps(4);
    assert_eq!(tag(&mut h), format!("{original}_Y"), "what was typed is not kept");
    assert!(h.state().dock.find_tab(&tab).is_none(), "closed");
    // And Cancel with nothing changed closes, no more.
    h.state_mut().actions.push(mg_ui::Action::OpenTab(tab.clone()));
    h.run_steps(4);
    h.get_by_label("Cancel").click();
    h.run_steps(4);
    assert_eq!(tag(&mut h), format!("{original}_Y"));
}

/// The other editors have OK and Cancel too: Cancel sets back what the
/// window changed (a file it made is gone again), and closes it.
#[test]
fn the_editors_cancel_sets_back_what_they_changed() {
    use mg_module::dialog::{Parent, add_node, new_dialog};
    let dir = mg_testkit::scratch_dir("ui-editors-cancel");
    let path = sample_module(&dir);
    let talk = ResKey::parse("talk", ResType::DLG).unwrap();
    let mut g = new_dialog();
    add_node(&mut g, Parent::Root, "Hello there.");
    let mut m = Module::open(&path).unwrap();
    m.set_gff(talk, &g).unwrap();
    m.set(ResKey::parse("mine", ResType::TWODA).unwrap(), b"2DA V2.0\n\n  A\n0 one\n".to_vec());
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    // (One window with OK and Cancel at a time.)
    close_tab(&mut app, &Tab::ModuleProperties);
    let mut h = harness(app, 1200.0, 900.0);
    h.run();
    let open = |h: &mut Harness<'_, Moonglow>, tab: Tab| {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(tab));
        h.run_steps(4);
    };
    let cancel = |h: &mut Harness<'_, Moonglow>, tab: &Tab| {
        h.get_by_label("Cancel").click();
        h.run_steps(4);
        assert!(h.state().dock.find_tab(tab).is_none(), "{tab:?} closed");
    };
    let undoable =
        |h: &Harness<'_, Moonglow>| h.state().ws.as_ref().unwrap().can_undo().map(str::to_owned);

    // The journal: a category added makes module.jrl; Cancel takes the
    // file away again.
    let jrl = ResKey::parse("module", ResType::JRL).unwrap();
    let has = |h: &mut Harness<'_, Moonglow>, key: &ResKey| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.flush().unwrap();
        ws.module.contains(key)
    };
    let had_journal = has(&mut h, &jrl);
    open(&mut h, Tab::Journal);
    h.get_by_label("Add").click();
    h.run_steps(3);
    assert!(has(&mut h, &jrl));
    cancel(&mut h, &Tab::Journal);
    assert_eq!(has(&mut h, &jrl), had_journal, "as it was");
    assert_eq!(undoable(&h).as_deref(), Some("Cancel Changes"));

    // A conversation: a line deleted, and back.
    let lines = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let g = ws.doc(&talk).unwrap();
        (
            g.root.list("EntryList").map_or(0, <[_]>::len),
            g.root.list("StartingList").map_or(0, <[_]>::len),
        )
    };
    open(&mut h, Tab::Dialog(talk));
    assert_eq!(lines(&mut h), (1, 1));
    h.get_by_label_contains("Hello there.").click();
    h.run_steps(2);
    h.get_by_label("Delete").click();
    h.run_steps(3);
    assert_eq!(lines(&mut h).1, 0);
    cancel(&mut h, &Tab::Dialog(talk));
    assert_eq!(lines(&mut h), (1, 1), "the line is back");

    // Factions: one added (repute.fac made), and gone.
    let factions =
        |app: &Moonglow| mg_module::factions::Factions::of_module(&app.ws.as_ref().unwrap().module);
    open(&mut h, Tab::Factions);
    h.get_by_label("Add Faction…").click();
    h.run();
    h.state_mut().faction_view.adding.as_mut().unwrap().name = "Guards".into();
    h.run();
    h.get_all_by_label("OK").last().unwrap().click();
    h.run_steps(3);
    assert_eq!(factions(h.state()).factions.len(), 6);
    cancel(&mut h, &Tab::Factions);
    assert_eq!(factions(h.state()).factions.len(), 5, "the faction is gone");

    // Area Properties: a box ticked, and unticked; OK keeps it.
    let area = ResRef::from_str("start").unwrap();
    let are = ResKey::new(area, ResType::ARE);
    let night = |h: &mut Harness<'_, Moonglow>| {
        h.state_mut().ws.as_mut().unwrap().doc(&are).unwrap().root.integer("IsNight")
    };
    let was = night(&mut h);
    open(&mut h, Tab::AreaProperties(area));
    h.get_by_label("Visual").click();
    h.run_steps(2);
    h.get_by_label("Always Dark").click();
    h.run_steps(3);
    assert_ne!(night(&mut h), was);
    cancel(&mut h, &Tab::AreaProperties(area));
    assert_eq!(night(&mut h), was, "as it was");
    open(&mut h, Tab::AreaProperties(area));
    h.get_by_label("Visual").click();
    h.run_steps(2);
    h.get_by_label("Always Dark").click();
    h.run_steps(3);
    h.get_by_label("OK").click();
    h.run_steps(4);
    assert_ne!(night(&mut h), was, "kept");
    assert!(h.state().dock.find_tab(&Tab::AreaProperties(area)).is_none());

    // Module Properties: its tag typed in, and Cancel pressed with the
    // field not left yet.
    let info = ResKey::parse("module", ResType::IFO).unwrap();
    let module_tag = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        let root = &ws.doc(&info).unwrap().root;
        String::from_utf8_lossy(root.string("Mod_Tag").unwrap_or_default()).into_owned()
    };
    open(&mut h, Tab::ModuleProperties);
    let is = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::TextInput && n.value().as_deref() == Some("SAMPLE")
    };
    let at = h.get(egui_kittest::kittest::by().predicate(is)).rect().center();
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(2);
    h.key_press(egui::Key::End);
    h.event(egui::Event::Text("_X".into()));
    h.run_steps(2);
    h.get_by_label("Basic").click();
    h.run_steps(3);
    assert_eq!(module_tag(&mut h), "SAMPLE_X");
    cancel(&mut h, &Tab::ModuleProperties);
    assert_eq!(module_tag(&mut h), "SAMPLE", "as it was");
    assert_eq!(undoable(&h).as_deref(), Some("Cancel Changes"));

    // A 2DA is saved when asked: OK saves it; Cancel closes it as last
    // saved, after the question its window's close asks of unsaved work.
    let mine = ResKey::parse("mine", ResType::TWODA).unwrap();
    let cell = |h: &Harness<'_, Moonglow>| {
        let data = h.state().ws.as_ref().unwrap().module.get(&mine).unwrap().to_vec();
        mg_2da::TwoDa::parse(&data, mg_core::Codepage::default())
            .unwrap()
            .get(0, "A")
            .map(str::to_string)
    };
    for (press, kept) in [("Cancel", "one"), ("OK", "two")] {
        let id = mg_ui::table_view::open_in_module(h.state_mut(), mine).unwrap();
        h.run_steps(4);
        let doc = h.state_mut().tables.iter_mut().find(|d| d.id == id).unwrap();
        doc.edit.set(0, 0, Some("two".into())).unwrap();
        h.run_steps(2);
        h.get_by_label(press).click();
        h.run_steps(4);
        if press == "Cancel" {
            h.get_by_label("Don't Save").click();
            h.run_steps(4);
        }
        assert!(h.state().dock.find_tab(&Tab::Table(id)).is_none(), "closed by {press}");
        assert!(h.state().tables.is_empty());
        assert_eq!(cell(&h).as_deref(), Some(kept), "{press}");
    }
}

/// The same in the other windows: Module Properties' description, typed
/// in, is kept when another of its pages is clicked.
#[test]
fn text_typed_in_module_properties_is_kept_when_its_page_is_left() {
    let dir = mg_testkit::scratch_dir("ui-ifo-typed");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 800.0);
    h.run_steps(3);
    h.get_by_label("Description").click();
    h.run_steps(3);
    let memo = egui_kittest::kittest::by().role(egui::accesskit::Role::MultilineTextInput);
    let at = h.get(memo.clone()).rect().center();
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run_steps(2);
    h.event(egui::Event::Text("A quiet valley. ".into()));
    h.run_steps(2);
    let described = |h: &mut Harness<'_, Moonglow>| {
        let ifo = ResKey::parse("module", ResType::IFO).unwrap();
        let root = h.state_mut().ws.as_mut().unwrap().doc(&ifo).unwrap().root.clone();
        let text = root.locstring("Mod_Description").cloned().unwrap_or_default();
        text.strings.iter().any(|(_, t)| String::from_utf8_lossy(t).contains("A quiet valley. "))
    };
    assert!(!described(&mut h), "not set while it is typed");
    // (A click as the pointer makes one: pressed, then let go a frame on.)
    let basic = h.get_by_label("Basic").rect().center();
    h.hover_at(basic);
    press(&h, basic, true, egui::Modifiers::NONE);
    h.run_steps(1);
    press(&h, basic, false, egui::Modifiers::NONE);
    h.run_steps(4);
    assert!(h.query(memo.clone()).is_none(), "the other page is shown");
    assert!(described(&mut h), "what was typed is kept");

    // And when a key does it: Save, with more typed in and the field not
    // left, saves that too; then the key that closes the tab keeps it.
    let type_more = |h: &mut Harness<'_, Moonglow>, text: &str| {
        h.get_by_label("Description").click();
        h.run_steps(3);
        let at = h.get(memo.clone()).rect().center();
        h.hover_at(at);
        press(h, at, true, egui::Modifiers::NONE);
        press(h, at, false, egui::Modifiers::NONE);
        h.run_steps(2);
        h.event(egui::Event::Text(text.into()));
        h.run_steps(2);
    };
    let has = |h: &mut Harness<'_, Moonglow>, text: &str| {
        let ifo = ResKey::parse("module", ResType::IFO).unwrap();
        let root = h.state_mut().ws.as_mut().unwrap().doc(&ifo).unwrap().root.clone();
        let d = root.locstring("Mod_Description").cloned().unwrap_or_default();
        d.strings.iter().any(|(_, t)| String::from_utf8_lossy(t).contains(text))
    };
    type_more(&mut h, "Saved by a key. ");
    assert!(!has(&mut h, "Saved by a key. "));
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::S);
    h.run_steps(5);
    assert!(has(&mut h, "Saved by a key. "));
    assert!(!h.state().ws.as_ref().unwrap().is_modified(), "and saved with it");
    type_more(&mut h, "Closed by a key. ");
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::W);
    h.run_steps(5);
    assert!(h.state().dock.find_tab(&Tab::ModuleProperties).is_none(), "the tab is closed");
    assert!(has(&mut h, "Closed by a key. "));
}

/// A long dropdown opens at the one chosen, not at its top, and whole:
/// what was typed in its Filter field the last time is gone.
#[test]
fn a_dropdown_opens_at_what_is_chosen() {
    let Some((mut h, _)) = blueprint_harness("plc_chest1", "chest_opens_at", ResType::UTP) else {
        return;
    };
    h.run();
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let appearance = |h: &Harness<'_, Moonglow>| {
        h.query_all(combo.clone())
            .find(|n| n.accesskit_node().value().is_some_and(|v| v.starts_with("Chest")))
            .map(|n| (n.rect().center(), n.accesskit_node().value().unwrap()))
            .expect("the Appearance Type dropdown")
    };
    let (at, chosen) = appearance(&h);
    let open = |h: &mut Harness<'_, Moonglow>| {
        h.hover_at(at);
        press(h, at, true, egui::Modifiers::NONE);
        press(h, at, false, egui::Modifiers::NONE);
        h.run();
    };
    open(&mut h);
    // The chosen row is in the list's window, a little under its Filter
    // field (the list has hundreds of rows before "Chest").
    let hint = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("Filter");
    let in_sight = |h: &Harness<'_, Moonglow>| {
        let top = h.get(egui_kittest::kittest::by().predicate(hint)).rect().bottom();
        let row = h
            .query_all_by_label(&chosen)
            .map(|n| n.rect().center().y)
            .find(|y| *y > top)
            .expect("the chosen row");
        assert!(row - top < 260.0, "the chosen row is {} under the Filter field", row - top);
    };
    in_sight(&h);
    // Filtered to something else and left; opened again, it is whole.
    type_into_hint(&mut h, "Filter", "Armoire");
    h.key_press(egui::Key::Escape);
    h.run();
    if h.query(egui_kittest::kittest::by().predicate(hint)).is_some() {
        // (Escape went to the field: a click outside closes the list.)
        let away = egui::pos2(5.0, 5.0);
        h.hover_at(away);
        press(&h, away, true, egui::Modifiers::NONE);
        press(&h, away, false, egui::Modifiers::NONE);
        h.run();
    }
    open(&mut h);
    in_sight(&h);
}

/// A long dropdown, left open, keeps its width from the first frame it
/// is drawn: its rows grew wider frame after frame until the list reached
/// the window's right edge (a builder's report, Area Properties › Audio ›
/// Ambient Sound, Day). Narrowing it with its Filter field leaves it so.
#[test]
fn an_open_dropdown_keeps_its_width() {
    let Some((mut h, _)) = blueprint_harness("plc_chest1", "chest_width", ResType::UTP) else {
        return;
    };
    h.run();
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let (at, chosen) = h
        .query_all(combo)
        .find(|n| n.accesskit_node().value().is_some_and(|v| v.starts_with("Chest")))
        .map(|n| (n.rect().center(), n.accesskit_node().value().unwrap()))
        .expect("the Appearance Type dropdown");
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    // (Measured from the first frame that draws it: `run` would step past
    // a frame that grew.)
    h.step();
    let hint = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("Filter");
    // The Filter field's width, and a row's (its highlight) under it.
    let widths = |h: &Harness<'_, Moonglow>, row: &str| {
        let filter = h.get(egui_kittest::kittest::by().predicate(hint)).rect();
        let under = |r: &egui::Rect| r.top() > filter.bottom() && r.left() >= filter.left() - 1.0;
        let row = h.query_all_by_label(row).map(|n| n.rect()).find(under).expect("the row");
        (filter.width(), row.width())
    };
    let first = widths(&h, &chosen);
    let mut seen = vec![first];
    for i in 0..300 {
        // (The pointer moving over the list, as a builder's does.)
        h.hover_at(egui::pos2(at.x + (i % 7) as f32, at.y + 60.0));
        h.step();
        seen.push(widths(&h, &chosen));
    }
    assert!(seen.iter().all(|w| *w == first), "the list keeps its width: {seen:?}");
    // Narrowed to a few: as wide as before.
    type_into_hint(&mut h, "Filter", "Armoire");
    let input = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == egui::accesskit::Role::TextInput && n.value().as_deref() == Some("Armoire")
    };
    let filter = h.get(egui_kittest::kittest::by().predicate(input)).rect();
    assert_eq!(filter.width(), first.0, "the list keeps its width, narrowed");
    // (Its rows still within it: with no scroll bar now, as wide as it.)
    let row = h.get_all_by_label_contains("Armoire").next().unwrap().rect();
    assert!(row.right() <= filter.right() + 1.0, "{row:?} within the list beside {filter:?}");
}

/// A dropdown with a Filter field stays open while the field is clicked
/// and typed in, and closes when a choice is made.
#[test]
fn a_dropdown_s_filter_field_can_be_typed_in() {
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "chest_filter", ResType::UTP) else {
        return;
    };
    h.run();
    let before = field(&mut h, &key).integer("Appearance").unwrap();
    // The Appearance Type dropdown: it shows "Chest".
    let combo = egui_kittest::kittest::by().role(egui::accesskit::Role::ComboBox);
    let appearance = |h: &Harness<'_, Moonglow>| {
        h.query_all(combo.clone())
            .find(|n| n.accesskit_node().value().is_some_and(|v| v.starts_with("Chest")))
            .map(|n| n.rect().center())
            .expect("the Appearance Type dropdown")
    };
    let at = appearance(&h);
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    press(&h, at, false, egui::Modifiers::NONE);
    h.run();
    // Its Filter field, clicked and typed in: the list narrows, still open.
    type_into_hint(&mut h, "Filter", "Armoire");
    let found = h.query_all_by_label_contains("Armoire").count();
    assert!(found > 0, "the dropdown is still open, and filtered");
    assert!(h.query_by_label("Altar").is_none(), "what doesn't match is left out");
    // A choice closes it and sets the appearance.
    h.get_all_by_label_contains("Armoire").next().unwrap().click();
    h.run();
    assert_ne!(field(&mut h, &key).integer("Appearance"), Some(before));
    let hint = |n: &egui_kittest::kittest::AccessKitNode<'_>| n.placeholder() == Some("Filter");
    assert!(h.query(egui_kittest::kittest::by().predicate(hint)).is_none(), "closed");
}

#[test]
fn text_is_edited_in_the_chosen_language() {
    use mg_core::{Gender, Language};
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_german", ResType::UTC)
    else {
        return;
    };
    // Options › Language: German (the UI thread's; this test's own).
    mg_ui::set_edit_language(Language::GERMAN);
    h.run();
    let english = |h: &mut Harness<'_, Moonglow>| {
        field(h, &key)
            .locstring("FirstName")
            .and_then(|l| l.text(Language::ENGLISH, Gender::Male).map(std::borrow::Cow::into_owned))
    };
    let before = english(&mut h);
    assert!(h.query_by_label_contains("(German)").is_some());
    h.get_all_by_label("🎲").next().unwrap().click();
    h.run();
    mg_ui::set_edit_language(Language::ENGLISH);
    let name = field(&mut h, &key).locstring("FirstName").cloned().unwrap();
    assert!(name.text(Language::GERMAN, Gender::Male).is_some(), "{name:?}");
    assert_eq!(english(&mut h), before);
}

#[test]
fn enter_does_what_a_dialog_s_main_button_does() {
    let root = mg_testkit::corpus!();
    let app = game_app(&root, None);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    let is_input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    // Enter on Cancel presses Cancel, not Create.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::N);
    h.run();
    h.get_by_label("Cancel").focus();
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(h.state().ws.is_none() && h.state().wizard.is_none(), "cancelled");
    // A name typed, then Enter: the module, as Create makes it.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::N);
    h.run();
    h.get_all_by_value("module000").find(is_input).expect("the name field").click();
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.get_all_by_value("module000").find(is_input).unwrap().type_text("moonglowTEST");
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(h.state().ws.is_some(), "{:?}", h.state().log.entries);
    assert!(h.state().log.entries.iter().any(|(_, m)| m == "Created module moonglowTEST"));
}

#[test]
fn dialogs_open_with_their_main_field_ready_to_type_in() {
    let root = mg_testkit::corpus!();
    let app = game_app(&root, None);
    let mut h = Harness::new_ui_state(|ui, app: &mut Moonglow| app.ui(ui), app);
    h.run();
    // New Module: typed straight away, no click, the name is replaced...
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::N);
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("Typed".into()));
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    assert!(h.state().log.entries.iter().any(|(_, m)| m == "Created module Typed"));
    // ...and the Area Wizard that follows takes its name the same way.
    h.run();
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("Glade".into()));
    h.run();
    let is_input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    assert!(h.get_all_by_value("Glade").any(|n| is_input(&n)), "the area's name typed in");
}

/// A number field dragged left and right changes its number (a builder
/// found the drag did nothing: the value was read again from the file each
/// frame of it).
#[test]
fn a_number_field_is_dragged_to_a_new_value() {
    let Some((mut h, key)) = blueprint_harness("nw_door_ttr_01", "door_drag", ResType::UTD) else {
        return;
    };
    h.run();
    h.get_by_label("Lock").click();
    h.run();
    let was = field(&mut h, &key).integer("OpenLockDC").unwrap();
    let spin =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::SpinButton;
    let at = h.get_all_by_value(&was.to_string()).find(spin).expect("the DC field").rect().center();
    h.hover_at(at);
    press(&h, at, true, egui::Modifiers::NONE);
    for k in 1..=8 {
        h.hover_at(at + egui::vec2(5.0 * k as f32, 0.0));
        h.run_steps(1);
    }
    press(&h, at + egui::vec2(40.0, 0.0), false, egui::Modifiers::NONE);
    h.run_steps(3);
    let now = field(&mut h, &key).integer("OpenLockDC").unwrap();
    assert!(now > was, "dragged from {was} to {now}");
    // One command for the whole drag.
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    h.run_steps(2);
    assert_eq!(field(&mut h, &key).integer("OpenLockDC"), Some(was));
}

/// The arrow keys step through a list of choices once it is clicked, as
/// in Aurora: here a placeable's Appearance.
#[test]
fn arrow_keys_step_through_a_list_of_choices() {
    let Some((mut h, key)) = blueprint_harness("plc_chest1", "arrow_copy", ResType::UTP) else {
        return;
    };
    h.run();
    let appearance = |h: &mut Harness<'_, Moonglow>| field(h, &key).integer("Appearance");
    let before = appearance(&mut h);
    let combo = egui::accesskit::Role::ComboBox;
    h.get_all_by_role(combo)
        .find(|n| n.accesskit_node().value().as_deref() == Some("Chest"))
        .expect("the Appearance list")
        .click();
    h.run();
    h.key_press(egui::Key::ArrowDown);
    h.run();
    let after = appearance(&mut h);
    assert_ne!(after, before, "the next appearance");
    h.key_press(egui::Key::ArrowUp);
    h.run();
    assert_eq!(appearance(&mut h), before, "and back");
    // The pointer resting on the list's box shows the chosen one's picture.
    h.key_press(egui::Key::Escape);
    h.run();
    let pictures = |h: &Harness<'_, Moonglow>| {
        h.query_all_by_role(egui::accesskit::Role::Image).count()
            + h.query_all_by_label("(no picture)").count()
    };
    let without = pictures(&h);
    let at = h
        .get_all_by_role(combo)
        .find(|n| n.accesskit_node().value().as_deref() == Some("Chest"))
        .unwrap()
        .rect()
        .center();
    h.hover_at(at);
    h.run_steps(90);
    assert_eq!(pictures(&h), without + 1, "the chosen appearance's picture");
}

/// The lists outside the blueprint editors take the arrow keys as theirs
/// do once clicked (GitHub issue 7): the area's toolbar (Fade Geometry,
/// Snap, Turn), without the camera moving, and Area Properties' (a
/// table's rows, and the grass's surface material).
#[test]
fn arrow_keys_step_through_the_area_s_lists() {
    use mg_area::grass::{AreaGrass, GrassOverride};
    let Some((mut h, area)) = area_harness("area-list-arrows") else { return };
    h.run_steps(3);
    let combo = egui::accesskit::Role::ComboBox;
    let click = |h: &mut Harness<'_, Moonglow>, value: &str| {
        h.get_all_by_role(combo)
            .find(|n| n.accesskit_node().value().as_deref() == Some(value))
            .unwrap_or_else(|| panic!("the list showing {value:?}"))
            .click();
        h.run_steps(2);
    };
    let key = |h: &mut Harness<'_, Moonglow>, key: egui::Key| {
        h.key_press(key);
        h.run_steps(2);
    };
    let camera = |h: &Harness<'_, Moonglow>| h.state().area_views[&area].orbit;
    let before = camera(&h);
    click(&mut h, "Fade Geometry: Never");
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(h.state().settings.fade_geometry, 1, "the next, while working on objects");
    key(&mut h, egui::Key::Escape);
    click(&mut h, "Snap: off");
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(h.state().settings.snap_grid, Some(25));
    key(&mut h, egui::Key::Escape);
    click(&mut h, "Turn: free");
    key(&mut h, egui::Key::ArrowDown);
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(h.state().settings.snap_angle, Some(15));
    key(&mut h, egui::Key::ArrowUp);
    assert_eq!(h.state().settings.snap_angle, Some(5));
    key(&mut h, egui::Key::Escape);
    assert_eq!(camera(&h), before, "the arrows were the lists', not the camera's");

    // Area Properties › Audio: a table's rows (soundeax.2da).
    let git = ResKey::new(area, ResType::GIT);
    let props = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&git).unwrap().root.child("AreaProperties").unwrap().clone()
    };
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::AreaProperties(area)));
    h.run_steps(5);
    h.get_all_by_label("Audio").last().unwrap().click();
    h.run_steps(3);
    let eax = |h: &mut Harness<'_, Moonglow>| props(h).integer("EnvAudio");
    let was = eax(&mut h);
    let line = h.get_by_label("Environmental Audio Effects").rect().center().y;
    h.get_all_by_role(combo).find(|n| (n.rect().center().y - line).abs() < 6.0).unwrap().click();
    h.run_steps(2);
    // (The area's may be the last row: then Up is the one to try.)
    key(&mut h, egui::Key::ArrowDown);
    if eax(&mut h) == was {
        key(&mut h, egui::Key::ArrowUp);
    }
    assert_ne!(eax(&mut h), was, "another audio environment");
    key(&mut h, egui::Key::Escape);

    // Area Properties › Grass: the surface material of the area's own.
    let root = h.state_mut().ws.as_mut().unwrap().doc(&git).unwrap().root.clone();
    let edits = mg_area::grass::add_override(git, &root, &GrassOverride::new(1, None));
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Grass", edits)));
    h.run_steps(2);
    h.get_all_by_label("Grass").last().unwrap().click();
    h.run_steps(3);
    let material = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        AreaGrass::read(&ws.doc(&git).unwrap().root).overrides[0].material
    };
    assert_eq!(material(&mut h), 1);
    click(&mut h, "Dirt (1)");
    key(&mut h, egui::Key::ArrowDown);
    if material(&mut h) == 1 {
        key(&mut h, egui::Key::ArrowUp);
    }
    assert_ne!(material(&mut h), 1, "another surface material");
}

/// A list of choices, shown in a combo box as `from`, clicked: then Down
/// (and `back`: Up) steps through it, and the box shows `to`.
fn arrow_step_list(h: &mut Harness<'_, Moonglow>, from: &str, to: &str) {
    let combo = egui::accesskit::Role::ComboBox;
    let value = |h: &Harness<'_, Moonglow>, v: &str| {
        h.query_all_by_role(combo).any(|n| n.accesskit_node().value().as_deref() == Some(v))
    };
    h.get_all_by_role(combo)
        .find(|n| n.accesskit_node().value().as_deref() == Some(from))
        .unwrap_or_else(|| panic!("the list showing {from:?}"))
        .click();
    h.run_steps(4);
    h.key_press(egui::Key::ArrowDown);
    h.run_steps(2);
    let shown: Vec<_> =
        h.query_all_by_role(combo).filter_map(|n| n.accesskit_node().value()).collect();
    assert!(value(h, to), "Down chose {to:?} after {from:?}: {shown:?}");
    h.key_press(egui::Key::ArrowUp);
    h.run_steps(2);
    assert!(value(h, from), "and Up {from:?} again");
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
}

/// The area's dialogs' lists take the arrow keys once clicked: Resize
/// Area's edges, Find Instance's area, Edit Areas Together's tileset and
/// flags.
#[test]
fn arrow_keys_step_through_the_area_dialogs_lists() {
    let Some((mut h, area)) = area_harness("area-dialog-arrows") else { return };
    h.run_steps(3);
    let menu = |h: &mut Harness<'_, Moonglow>, item: &str| {
        h.get_by_label("Edit").click();
        h.run_steps(2);
        h.get_by_label(item).click();
        h.run_steps(6);
    };
    menu(&mut h, "Resize Area…");
    arrow_step_list(&mut h, "North and east", "East and south");
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    h.state_mut().resize_area = None;
    h.run_steps(2);

    menu(&mut h, "Find Instance…");
    let combo = egui::accesskit::Role::ComboBox;
    h.get_all_by_role(combo)
        .find(|n| n.accesskit_node().value().as_deref() == Some("(all areas)"))
        .expect("Find Instance's area")
        .click();
    h.run_steps(2);
    h.key_press(egui::Key::ArrowDown);
    h.run_steps(2);
    let found = |h: &Harness<'_, Moonglow>| h.state().find_instance.as_ref().unwrap().area;
    assert_eq!(found(&h), Some(area), "the area, after all areas");
    h.key_press(egui::Key::ArrowUp);
    h.run_steps(2);
    assert_eq!(found(&h), None);
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    h.state_mut().find_instance = None;
    h.run_steps(2);

    menu(&mut h, "Edit Areas Together…");
    arrow_step_list(&mut h, "Above or under ground", "Underground");
    let any = h
        .get_all_by_role(combo)
        .find(|n| n.accesskit_node().value().as_deref() == Some("Any"))
        .expect("the chooser's tileset");
    let tileset = any.rect().center();
    any.click();
    h.run_steps(2);
    h.key_press(egui::Key::ArrowDown);
    h.run_steps(2);
    let shown = h
        .get_all_by_role(combo)
        .find(|n| (n.rect().center().y - tileset.y).abs() < 4.0)
        .and_then(|n| n.accesskit_node().value());
    assert!(shown.as_deref().is_some_and(|v| v != "Any"), "a tileset, after any: {shown:?}");
}

/// The lists of the rest of the program take the arrow keys as the
/// blueprint editors' do (GitHub issue 7): here Options' Interface size and
/// Advanced Search's kinds and variable types.
#[test]
fn arrow_keys_step_through_every_window_s_lists() {
    let dir = mg_testkit::scratch_dir("ui-every-list-arrows");
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 900.0);
    h.run();
    h.state_mut().actions.push(mg_ui::Action::OptionsDialog);
    h.run_steps(2);
    h.get_by_label("General").click();
    h.run_steps(2);
    let scale = h.state().options.as_ref().unwrap().ui_scale;
    let combo = egui::accesskit::Role::ComboBox;
    h.get_all_by_role(combo)
        .find(|n| n.accesskit_node().value().is_some_and(|v| v == format!("{scale}%")))
        .expect("Interface size")
        .click();
    h.run_steps(2);
    h.key_press(egui::Key::ArrowDown);
    h.run_steps(2);
    let now = h.state().options.as_ref().unwrap().ui_scale;
    assert!(now > scale, "a larger interface: {scale}% then {now}%");
    h.key_press(egui::Key::Escape);
    h.run_steps(2);
    h.state_mut().options = None;
    h.run_steps(2);

    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::AdvancedSearch));
    h.run_steps(2);
    let kinds = mg_ui::search::Kind::ALL;
    arrow_step_list(&mut h, kinds[0].label(), kinds[1].label());
    h.state_mut().advanced_search.first.kind = mg_ui::search::Kind::Variable;
    h.run_steps(2);
    arrow_step_list(&mut h, "Any", mg_module::vars::VarType::ALL[0].name());
}

/// A creature's Alignment, a list of presets that is not a table's rows,
/// takes the arrow keys as the editor's other lists do.
#[test]
fn arrow_keys_step_through_a_creature_s_alignments() {
    let Some((mut h, key)) = blueprint_harness("nw_chicken", "align_arrows", ResType::UTC) else {
        return;
    };
    h.run();
    h.get_by_label("Classes").click();
    h.run_steps(2);
    let axes = |h: &mut Harness<'_, Moonglow>| {
        let s = field(h, &key);
        (s.integer("GoodEvil").unwrap(), s.integer("LawfulChaotic").unwrap())
    };
    // (Made a preset, so that the arrows step from it.)
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "Alignment",
        vec![
            mg_edit::Edit::SetField {
                key,
                path: mg_edit::GffPath::root(),
                label: "GoodEvil".into(),
                value: Some(mg_gff::Value::Byte(50)),
            },
            mg_edit::Edit::SetField {
                key,
                path: mg_edit::GffPath::root(),
                label: "LawfulChaotic".into(),
                value: Some(mg_gff::Value::Byte(50)),
            },
        ],
    )));
    h.run_steps(2);
    arrow_step_list(&mut h, "True Neutral", "Chaotic Neutral");
    assert_eq!(axes(&mut h), (50, 50), "True Neutral again");
}

/// Area Properties' Lighting Scheme, a grid of the schemes filled a column
/// at a time, takes the arrow keys once a scheme is clicked: Up and Down
/// down the columns, Left and Right to the column beside.
#[test]
fn arrow_keys_move_through_the_lighting_schemes() {
    let Some((mut h, area)) = area_harness("scheme-arrows") else { return };
    h.run_steps(3);
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::AreaProperties(area)));
    h.run_steps(5);
    h.get_all_by_label("Visual").last().unwrap().click();
    h.run_steps(3);
    let are = ResKey::new(area, ResType::ARE);
    let scheme = |h: &mut Harness<'_, Moonglow>| {
        let ws = h.state_mut().ws.as_mut().unwrap();
        ws.doc(&are).unwrap().root.integer("LightingScheme").unwrap_or(0)
    };
    let schemes = h
        .state()
        .game
        .as_ref()
        .unwrap()
        .choices(
            "environment",
            mg_rules::ChoiceColumns { name: Some("STRREF"), label: Some("LABEL") },
        )
        .unwrap();
    assert!(schemes.len() > 8, "the game's schemes");
    let key = |h: &mut Harness<'_, Moonglow>, key: egui::Key| {
        h.key_press(key);
        h.run_steps(3);
    };
    h.get_all_by_label(&schemes[0].text).last().unwrap().click();
    h.run_steps(3);
    assert_eq!(scheme(&mut h), schemes[0].row as i64);
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(scheme(&mut h), schemes[1].row as i64, "the next, below");
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(scheme(&mut h), schemes[2].row as i64, "and the next");
    key(&mut h, egui::Key::ArrowUp);
    assert_eq!(scheme(&mut h), schemes[1].row as i64);
    // To the column beside: as many on as a column holds.
    let column = |h: &Harness<'_, Moonglow>, text: &str| {
        h.get_all_by_label(text).last().unwrap().rect().left()
    };
    key(&mut h, egui::Key::ArrowRight);
    let beside = schemes.iter().position(|c| c.row as i64 == scheme(&mut h)).unwrap();
    assert!(beside > 1, "further on");
    assert!(
        column(&h, &schemes[beside].text) > column(&h, &schemes[1].text),
        "in the column to the right"
    );
    key(&mut h, egui::Key::ArrowLeft);
    assert_eq!(scheme(&mut h), schemes[1].row as i64, "and back");
}

/// The Filter fields find names case and accents aside, as the lists are
/// put in order (Sacha's report, October 2026): `arbol` finds `Árbol
/// viejo` in the module tree and the palette, and `ÁRMOIRE` a list's
/// Armoire.
#[test]
fn filters_find_names_case_and_accents_aside() {
    use mg_edit::{Command, Edit};
    let Some(root) = mg_testkit::nwn_root() else {
        eprintln!("skipped: no game install");
        return;
    };
    let app = |name: &str| {
        let dir = mg_testkit::scratch_dir(name);
        let path = sample_module(&dir);
        let mut app = game_app(&root, None);
        app.open_module(&path);
        let game = app.game.as_deref().unwrap();
        let base =
            Gff::read(&game.resman.get_named("nw_waypoint001", ResType::UTW).unwrap()).unwrap();
        let edits = [("mg_wp_one", "Árbol viejo"), ("mg_wp_two", "Roble")]
            .map(|(resref, name)| {
                let key = ResKey::parse(resref, ResType::UTW).unwrap();
                let mut g = base.clone();
                g.root.set("TemplateResRef", mg_gff::Value::resref(key.resref));
                // (In the game's codepage.)
                let mut ls = LocString::default();
                ls.set_text(Language::ENGLISH, Gender::Male, name).unwrap();
                g.root.set("LocalizedName", mg_gff::Value::LocString(ls));
                g.root.set("PaletteID", mg_gff::Value::Byte(5));
                Edit::SetResource { key, data: Some(g.to_bytes().unwrap()) }
            })
            .to_vec();
        app.ws.as_mut().unwrap().apply(Command::new("setup", edits)).unwrap();
        close_tab(&mut app, &Tab::ModuleProperties);
        app.open_palette = false;
        // (The tree by name, Options › General.)
        app.settings.area_names = true;
        harness(app, 900.0, 700.0)
    };

    // The module tree's filter, its first text field.
    let mut h = app("ui-filter-accents-tree");
    h.run();
    let field = egui::accesskit::Role::TextInput;
    h.get_all_by_role(field).next().unwrap().click();
    h.run();
    h.get_all_by_role(field).next().unwrap().type_text("arbol");
    h.run_steps(2);
    assert!(h.query_by_label_contains("Árbol viejo").is_some(), "the tree finds Árbol");
    assert!(h.query_by_label_contains("Roble").is_none());

    // The palette's search.
    let mut h = app("ui-filter-accents-palette");
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Palette));
    h.state_mut().palette.kind = mg_module::palette::BlueprintKind::Waypoint;
    h.state_mut().palette.tiles = false;
    h.state_mut().palette.custom = true;
    h.run();
    h.state_mut().palette.filter = "ARBOL".into();
    h.run_steps(2);
    assert!(h.query_by_label_contains("Árbol viejo").is_some(), "the palette finds Árbol");
    assert!(h.query_by_label_contains("Roble").is_none());

    // A list of a table's rows (a placeable's Appearance).
    let Some((mut h, _)) = blueprint_harness("plc_chest1", "filter_accents", ResType::UTP) else {
        return;
    };
    h.run();
    h.get_all_by_role(egui::accesskit::Role::ComboBox)
        .find(|n| n.accesskit_node().value().as_deref() == Some("Chest"))
        .expect("the Appearance list")
        .click();
    h.run_steps(2);
    type_into_hint(&mut h, "Filter", "ÁRMOIRE");
    h.run_steps(2);
    assert!(h.query_by_label_contains("Armoire").is_some(), "the list finds Armoire");
}

/// Where a text has none in the language edited, its text in another
/// shows in its field, with the language named: in Module Properties and
/// in a blueprint's fields alike.
#[test]
fn text_in_another_language_shows_where_the_one_edited_has_none() {
    use mg_core::{Gender, Language, LocString};
    let Some((mut h, key)) = blueprint_harness("nw_bandit001", "bandit_text", ResType::UTC) else {
        return;
    };
    // The module's name and the bandit's first name, in English alone.
    let mut name = LocString::default();
    name.set(Language::ENGLISH, Gender::Male, b"Only English".to_vec());
    let ifo = ResKey::parse("module", ResType::IFO).unwrap();
    let edits = vec![
        mg_edit::Edit::SetField {
            key: ifo,
            path: mg_edit::GffPath::root(),
            label: "Mod_Name".into(),
            value: Some(mg_gff::Value::LocString(name.clone())),
        },
        mg_edit::Edit::SetField {
            key,
            path: mg_edit::GffPath::root(),
            label: "FirstName".into(),
            value: Some(mg_gff::Value::LocString(name)),
        },
    ];
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Setup", edits)));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::ModuleProperties));
    h.run_steps(4);
    // Edited in English: its own text, unmarked.
    let own = h.get_all_by_value("Only English").count();
    assert!(own >= 2, "both fields have it");
    assert_eq!(h.query_all_by_label("English").count(), 0);
    // Edited in German: the English text still shows, named as English.
    mg_ui::set_edit_language(Language::GERMAN);
    h.run_steps(3);
    let shown = h.get_all_by_value("Only English").count();
    let named = h.query_all_by_label("English").count();
    mg_ui::set_edit_language(Language::ENGLISH);
    assert_eq!((shown, named), (own, 2));
}

/// Escape in a text field of a Properties window (a number's, a tag's)
/// leaves the field and no more: what was typed is kept and the window
/// stays, on its page. A second Escape, with nothing typed in, closes it.
#[test]
fn escape_in_a_field_leaves_the_field_and_then_cancels_the_window() {
    let Some((mut h, key)) = blueprint_harness("nw_chicken", "esc_field", ResType::UTC) else {
        return;
    };
    let is_input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    let open = |h: &Harness<'_, Moonglow>| h.state().dock.find_tab(&Tab::Blueprint(key)).is_some();
    h.run();
    // A tag typed into.
    let tag = String::from_utf8_lossy(field(&mut h, &key).string("Tag").unwrap()).into_owned();
    h.get_all_by_value(&tag).find(is_input).expect("the tag's field").click();
    h.run();
    h.key_press(egui::Key::End);
    h.get_all_by_value(&tag).find(is_input).unwrap().type_text("X");
    h.run();
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(open(&h), "the window stays");
    let now = String::from_utf8_lossy(field(&mut h, &key).string("Tag").unwrap()).into_owned();
    assert_eq!(now, format!("{tag}X"), "what was typed is kept");
    // A number typed into, on the Statistics page.
    h.get_by_label("Statistics").click();
    h.run();
    let was = field(&mut h, &key).integer("Str").unwrap();
    h.get_all_by_value(&was.to_string()).next().expect("Strength's field").click();
    h.run();
    h.get_all_by_value(&was.to_string()).next().unwrap().type_text("12");
    h.run();
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(open(&h), "the window stays");
    assert!(h.query_by_label("Ability Scores").is_some(), "on its page");
    assert_eq!(field(&mut h, &key).integer("Str"), Some(12), "what was typed is kept");
    // Nothing typed in now: Escape is the window's Cancel, as Aurora's
    // is: it closes, and what was changed in it is set back.
    h.key_press(egui::Key::Escape);
    h.run();
    h.run();
    assert!(!open(&h));
    let now = String::from_utf8_lossy(field(&mut h, &key).string("Tag").unwrap()).into_owned();
    assert_eq!(now, tag, "the tag set back");
    assert_eq!(field(&mut h, &key).integer("Str"), Some(was), "Strength set back");
}
