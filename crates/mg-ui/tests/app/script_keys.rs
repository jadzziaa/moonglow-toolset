//! The script editor's own keys, pointer and history (`script_layout::code_edit`,
//! the adversarial branch's editor in place of egui's TextEdit): what
//! egui's TextEdit answered it answers, and its history and laid-out
//! lines stay those of its text (the reviews D and K of that branch).

use crate::common::*;

/// A module whose script `hello` holds `text`, its editor open in front.
fn editor(name: &str, text: &str) -> (Harness<'static, Moonglow>, ResKey) {
    let (mut h, keys) = editors(name, &[("hello", text)], |_| {});
    let key = keys[0];
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    settle(&mut h);
    (h, key)
}

/// A module with the scripts `scripts` (name and text), none open yet;
/// `set` sets the app up before the module opens.
fn editors(
    name: &str,
    scripts: &[(&str, &str)],
    set: impl FnOnce(&mut Moonglow),
) -> (Harness<'static, Moonglow>, Vec<ResKey>) {
    let dir = mg_testkit::scratch_dir(name);
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let keys: Vec<ResKey> =
        scripts.iter().map(|(n, _)| ResKey::parse(n, ResType::NSS).unwrap()).collect();
    for (key, (_, text)) in keys.iter().zip(scripts) {
        m.set(*key, text.as_bytes().to_vec());
    }
    m.save().unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    set(&mut app);
    app.open_module(&path);
    let mut h = harness(app, 1200.0, 800.0);
    h.run();
    (h, keys)
}

fn id(key: ResKey) -> egui::Id {
    egui::Id::new(("script", key))
}

/// The editor's cursor and the other end of its selection (characters).
fn cursor(h: &Harness<'_, Moonglow>, key: ResKey) -> (usize, usize) {
    let state = egui::text_edit::TextEditState::load(&h.ctx, id(key)).expect("a cursor");
    let r = state.cursor.char_range().expect("a cursor");
    (r.primary.index.into(), r.secondary.index.into())
}

/// Puts the editor's cursor at `at` (to `to` selected) and gives it the keys.
fn put(h: &mut Harness<'_, Moonglow>, key: ResKey, at: usize, to: usize) {
    let mut state = egui::text_edit::TextEditState::load(&h.ctx, id(key)).unwrap_or_default();
    let (a, b) = (egui::text::CCursor::new(at), egui::text::CCursor::new(to));
    state.cursor.set_char_range(Some(egui::text::CCursorRange::two(a, b)));
    state.store(&h.ctx, id(key));
    h.ctx.memory_mut(|m| m.request_focus(id(key)));
    h.run();
}

fn text(h: &Harness<'_, Moonglow>, key: ResKey) -> String {
    h.state().script_text(key).unwrap()
}

fn press(h: &mut Harness<'_, Moonglow>, modifiers: egui::Modifiers, key: egui::Key) {
    h.key_press_modifiers(modifiers, key);
    h.run();
}

fn undo(h: &mut Harness<'_, Moonglow>) {
    press(h, egui::Modifiers::COMMAND, egui::Key::Z);
}

/// Where the editor shows the row that begins with `start` (the
/// accessibility tree's text run of it), if it is laid out.
fn row(h: &Harness<'_, Moonglow>, start: &str) -> Option<egui::Rect> {
    let run = egui_kittest::kittest::by().role(egui::accesskit::Role::TextRun);
    h.query_all(run)
        .find(|n| n.accesskit_node().value().is_some_and(|v| v.starts_with(start)))
        .map(|n| n.rect())
}

/// Whether the editor's row that begins with `start` is in its sight (the
/// window's top 600 points, above the Bottom Dock, as `scripts.rs` has it).
fn in_sight(h: &Harness<'_, Moonglow>, start: &str) -> bool {
    let window = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
    row(h, start).is_some_and(|r| window.contains_rect(r))
}

/// Ctrl as egui has it on Windows and Linux (`Modifiers::COMMAND` alone is
/// no key's: egui's own editing keys read `ctrl`).
const CTRL: egui::Modifiers =
    egui::Modifiers { alt: false, ctrl: true, shift: false, mac_cmd: false, command: true };

/// A Mac's Command key, as egui has it there.
const MAC_CMD: egui::Modifiers =
    egui::Modifiers { alt: false, ctrl: false, shift: false, mac_cmd: true, command: true };

/// What was typed is taken back by Ctrl+Z after the script's Save, and
/// after its Compile: each cleared the editor's history, as the module's
/// text came back the editor's (review D2, K's B1).
#[test]
fn undo_takes_back_what_was_typed_after_a_save_and_a_compile() {
    let first = "void main()\n{\n}\n";
    let (mut h, key) = editor("ui-script-undo-after-save", first);
    put(&mut h, key, 13, 13);
    h.event(egui::Event::Text("int n;".into()));
    h.run();
    let typed = "void main()\n{int n;\n}\n";
    assert_eq!(text(&h, key), typed);
    press(&mut h, egui::Modifiers::COMMAND, egui::Key::S);
    settle(&mut h);
    let module = h.state().ws.as_ref().unwrap().module.get(&key).unwrap().to_vec();
    assert_eq!(module, typed.as_bytes(), "saved");
    put(&mut h, key, 19, 19);
    undo(&mut h);
    assert_eq!(text(&h, key), first, "the typing taken back after Save");
    // Compile (it saves the text first, here the text before the typing):
    // the typing can still be made again.
    script_click(&mut h, "Compile");
    settle(&mut h);
    let module = h.state().ws.as_ref().unwrap().module.get(&key).unwrap().to_vec();
    assert_eq!(module, first.as_bytes(), "saved by Compile");
    put(&mut h, key, 13, 13);
    press(&mut h, egui::Modifiers::COMMAND, egui::Key::Y);
    assert_eq!(text(&h, key), typed, "the typing made again after Compile");
}

/// A script open in a tab behind another, rewritten by a command (the
/// NUI Creator's, a plugin's, Undo) and followed there (`sync_nav`):
/// its editor was not drawn, so its laid-out lines were those of the text
/// before, and Replace and Replace All edited through them (review D1,
/// K's B2): Replace on the wrong bytes, Replace All stamping the old
/// lines as the new text's, the editor drawn from them after.
fn rewritten_behind(name: &str) -> (Harness<'static, Moonglow>, ResKey, String) {
    let scripts =
        [("back", "int a;\nint b;\nint c;\nvoid main() {}\n"), ("front", "void main() {}\n")];
    let (mut h, keys) = editors(name, &scripts, |_| {});
    let (back, front) = (keys[0], keys[1]);
    for key in [back, front] {
        h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
        settle(&mut h);
    }
    assert!(h.state().script_laid_out_for_test(back), "its lines laid out");
    assert!(row(&h, "int b;").is_none(), "behind the other tab");
    // (Shorter, a character of two bytes first.)
    let rewritten = "// é\nint c;\nvoid main() {}\n".to_string();
    let data = mg_core::Codepage::WINDOWS_1252.encode_lossy(&rewritten);
    let edit = mg_edit::Edit::SetResource { key: back, data: Some(data) };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Rewrite", vec![edit])));
    settle(&mut h);
    let _ = h.state_mut().declaration_at(front, 0);
    assert_eq!(text(&h, back), rewritten, "followed behind its tab");
    let search = &mut h.state_mut().script_tools.search;
    (search.open, search.replace_mode, search.script) = (true, true, Some(back));
    (search.find, search.replace) = ("c".into(), "total".into());
    search.options.whole_word = true;
    settle(&mut h);
    (h, back, rewritten)
}

#[test]
fn replace_all_in_a_script_rewritten_behind_its_tab() {
    let (mut h, back, _) = rewritten_behind("ui-script-replace-all-behind");
    h.get_by_label("Replace All").click();
    settle(&mut h);
    let replaced = "// é\nint total;\nvoid main() {}\n";
    assert_eq!(text(&h, back), replaced);
    assert!(h.state().script_layout_in_step_for_test(back), "its lines the new text's");
    h.state_mut().script_tools.search.open = false;
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(back)));
    settle(&mut h);
    for start in ["// é", "int total;", "void main() {}"] {
        assert!(row(&h, start).is_some(), "{start:?} drawn");
    }
}

#[test]
fn replace_in_a_script_rewritten_behind_its_tab() {
    let (mut h, back, _) = rewritten_behind("ui-script-replace-behind");
    // The match selected ("c": after "// é\nint ").
    let mut state = egui::text_edit::TextEditState::load(&h.ctx, id(back)).unwrap_or_default();
    let (a, b) = (egui::text::CCursor::new(9), egui::text::CCursor::new(10));
    state.cursor.set_char_range(Some(egui::text::CCursorRange::two(a, b)));
    state.store(&h.ctx, id(back));
    h.get_by_label("Replace").click();
    settle(&mut h);
    assert_eq!(text(&h, back), "// é\nint total;\nvoid main() {}\n");
    assert!(h.state().script_layout_in_step_for_test(back));
}

/// Text composed (IME) is one step of the history, from the text before
/// the composition to what it commits; a composition given up leaves no
/// step (each text composed was a step: review D5, K's M1).
#[test]
fn a_composition_is_one_step_of_the_history() {
    let first = "int n;\n";
    let (mut h, key) = editor("ui-script-ime-undo", first);
    put(&mut h, key, 4, 5);
    for pre in ["k", "ka", "kan"] {
        let p = egui::ImeEvent::Preedit { text: pre.into(), active_range_chars: None };
        h.event(egui::Event::Ime(p));
        h.run();
    }
    assert_eq!(text(&h, key), "int kan;\n");
    h.event(egui::Event::Ime(egui::ImeEvent::Commit("かん".into())));
    h.run();
    assert_eq!(text(&h, key), "int かん;\n");
    assert_eq!(cursor(&h, key), (6, 6));
    undo(&mut h);
    assert_eq!(text(&h, key), first, "the whole composition at once");
    assert_eq!(cursor(&h, key), (5, 4), "the selection before it");
    press(&mut h, egui::Modifiers::COMMAND, egui::Key::Y);
    assert_eq!(text(&h, key), "int かん;\n");
    // Given up: as before it, nothing to take back.
    put(&mut h, key, 0, 0);
    let p = egui::ImeEvent::Preedit { text: "x".into(), active_range_chars: None };
    h.event(egui::Event::Ime(p));
    h.run();
    let p = egui::ImeEvent::Preedit { text: String::new(), active_range_chars: None };
    h.event(egui::Event::Ime(p));
    h.run();
    assert_eq!(text(&h, key), "int かん;\n");
    undo(&mut h);
    assert_eq!(text(&h, key), first, "the step before, the composition's");
}

/// A composition goes where the cursor is, as in egui: its range kept
/// from before the keys left the editor put what it committed back over
/// its own old text, wherever the cursor was by then (review K's M2).
#[test]
fn a_composition_goes_where_the_cursor_is() {
    let (mut h, key) = editor("ui-script-ime-cursor", "int n;\nint m;\n");
    put(&mut h, key, 4, 4);
    let p = egui::ImeEvent::Preedit { text: "ka".into(), active_range_chars: None };
    h.event(egui::Event::Ime(p));
    h.run();
    assert_eq!(text(&h, key), "int kan;\nint m;\n");
    // The keys elsewhere and back, the cursor on the next line.
    h.ctx.memory_mut(|m| m.surrender_focus(id(key)));
    h.run();
    put(&mut h, key, 13, 13);
    h.event(egui::Event::Ime(egui::ImeEvent::Commit("か".into())));
    h.run();
    assert_eq!(text(&h, key), "int kan;\nint かm;\n");
    assert_eq!(cursor(&h, key), (14, 14));
}

/// A composition (IME) is shown underlined, not selected: the selection's
/// ground is not behind it, as egui shows one (it was drawn as a selection:
/// review K's N5). With `MG_SHOT` set, the pictures.
#[test]
fn a_composition_is_underlined_not_selected() {
    if no_adapter() {
        return;
    }
    mg_testkit::gpu::hold();
    let _shared_rs = with_shared_renderer();
    let (mut h, key) = editor("ui-script-ime-looks", "int n;\nint m;\n");
    let dir = shot_dir("ui-script-ime-looks");
    let ppp = h.ctx.pixels_per_point();
    let look = |h: &mut Harness<'_, Moonglow>, start: &str, name: &str| {
        let line = row(h, start).expect("the line");
        let img = h.render().expect("render");
        if let Some(dir) = &dir {
            img.save(dir.join(name)).unwrap();
        }
        crop_rect(&img, line, ppp, 0.0)
    };
    // The selection's ground: the most of a line selected whole.
    put(&mut h, key, 0, 6);
    let selected = look(&mut h, "int n;", "selected.png");
    let mut counts = std::collections::HashMap::new();
    for p in selected.pixels() {
        *counts.entry(p.0).or_insert(0usize) += 1;
    }
    let (ground, n) = counts.into_iter().max_by_key(|(_, n)| *n).unwrap();
    assert!(n * 2 > selected.pixels().len(), "a ground: {n} of {}", selected.pixels().len());
    let near = |p: &image::Rgba<u8>| p.0.iter().zip(ground).all(|(a, b)| a.abs_diff(b) <= 3);
    // A composition in the line: none of it.
    put(&mut h, key, 4, 4);
    let p = egui::ImeEvent::Preedit { text: "kana".into(), active_range_chars: None };
    h.event(egui::Event::Ime(p));
    h.run();
    let composed = look(&mut h, "int kanan;", "composition.png");
    let on = composed.pixels().filter(|p| near(p)).count();
    assert!(on * 20 < composed.pixels().len(), "the selection's ground behind it: {on}");
}

/// The input method's DeleteSurrounding takes the characters around the
/// selection and keeps the selection, as egui does (it took the
/// selection too: review K's N5).
#[test]
fn delete_surrounding_keeps_the_selection() {
    let (mut h, key) = editor("ui-script-ime-surrounding", "abcdef\n");
    put(&mut h, key, 2, 4);
    let d = egui::ImeEvent::DeleteSurrounding { before_chars: 1, after_chars: 1 };
    h.event(egui::Event::Ime(d));
    h.run();
    assert_eq!(text(&h, key), "acdf\n");
    assert_eq!(cursor(&h, key), (3, 1), "\"cd\" still selected");
}

/// Ctrl+Enter is no click on the editor (egui's TextEdit drops the click
/// a focused widget is given for Space and Enter): it went to the
/// definition of the name at the cursor, as Ctrl+click does (review K's N1).
#[test]
fn ctrl_enter_does_not_go_to_the_definition() {
    let source = "void Foo() {}\nvoid main() { Foo(); }\n";
    let (mut h, key) = editor("ui-script-ctrl-enter", source);
    let at = source.rfind("Foo").unwrap() + 1;
    put(&mut h, key, at, at);
    press(&mut h, egui::Modifiers::COMMAND, egui::Key::Enter);
    settle(&mut h);
    assert_eq!(cursor(&h, key), (at, at), "the cursor where it was");
    assert_eq!(text(&h, key), source);
}

/// Undo of Backspace and Delete puts the cursor back where it was, not
/// the characters they took selected (review K's N2); Delete key after
/// key is one step, as Backspace is (N11).
#[test]
fn undo_of_backspace_and_delete_gives_the_cursor_back() {
    let first = "int abcdef;\n";
    let (mut h, key) = editor("ui-script-delete-undo", first);
    put(&mut h, key, 7, 7);
    press(&mut h, egui::Modifiers::NONE, egui::Key::Backspace);
    assert_eq!(text(&h, key), "int abdef;\n");
    undo(&mut h);
    assert_eq!(text(&h, key), first);
    assert_eq!(cursor(&h, key), (7, 7), "the cursor, nothing selected");
    // (A frame a key, as a key held repeats, within a second: a pause of
    // one ends a step, as in egui; a press here runs frames enough to take
    // one.)
    for _ in 0..3 {
        h.key_press(egui::Key::Delete);
        h.step();
    }
    h.run();
    assert_eq!(text(&h, key), "int abc;\n");
    undo(&mut h);
    assert_eq!(text(&h, key), first, "the three at once");
    assert_eq!(cursor(&h, key), (7, 7));
}

/// Undo of Replace All puts the cursor back where it was, not at the
/// text's start (review K's N3).
#[test]
fn undo_of_replace_all_gives_the_cursor_back() {
    let first = "int a;\nint b = a + a;\n";
    let (mut h, key) = editor("ui-script-replace-all-cursor", first);
    put(&mut h, key, 12, 12);
    let search = &mut h.state_mut().script_tools.search;
    (search.open, search.replace_mode, search.script) = (true, true, Some(key));
    (search.find, search.replace) = ("a".into(), "x".into());
    search.options.whole_word = true;
    settle(&mut h);
    h.get_by_label("Replace All").click();
    settle(&mut h);
    assert_eq!(text(&h, key), "int x;\nint b = x + x;\n");
    put(&mut h, key, 3, 3);
    undo(&mut h);
    assert_eq!(text(&h, key), first);
    assert_eq!(cursor(&h, key), (12, 12));
}

/// The text changed before the editor is drawn (here the toolbar's Undo)
/// brings the cursor into sight, as typing does (review K's N4).
#[test]
fn the_toolbar_s_undo_brings_the_cursor_into_sight() {
    let source: String = (0..300).map(|n| format!("int nLine{n} = {n};\n")).collect();
    let (mut h, key) = editor("ui-script-undo-in-sight", &source);
    let at = source.find("int nLine250 ").unwrap();
    h.state_mut().script_tools.jump = Some((key, at));
    settle(&mut h);
    h.event(egui::Event::Text("x".into()));
    settle(&mut h);
    assert!(in_sight(&h, "xint nLine250 "));
    // Scrolled back to the top, the cursor where it was.
    let editor = row(&h, "xint nLine250 ").unwrap();
    h.hover_at(editor.center());
    for _ in 0..10 {
        h.event(egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Page,
            delta: egui::vec2(0.0, 5.0),
            phase: egui::TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        });
        h.step();
    }
    settle(&mut h);
    assert!(in_sight(&h, "int nLine0 ") && !in_sight(&h, "xint nLine250 "), "scrolled away");
    tool(&h, "Undo").click();
    settle(&mut h);
    assert_eq!(text(&h, key), source);
    assert!(in_sight(&h, "int nLine250 "), "the cursor's line brought into sight");
}

/// With smooth scrolling off (Options, 5.2.0), the lines a paste adds at
/// the text's end are in the room the editor is given that frame, and its
/// cursor scrolled into sight in it (the room was taken before the keys,
/// the new lines left out of it for a frame, and the scroll an animation
/// of the frame after: review K's N6).
#[test]
fn a_paste_at_the_end_is_in_sight_with_smooth_scrolling_off() {
    let source: String = (0..200).map(|n| format!("int nLine{n} = {n};\n")).collect();
    let (mut h, keys) = editors("ui-script-paste-end", &[("long", &source)], |app| {
        app.settings.no_smooth_scrolling = true;
    });
    let key = keys[0];
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    settle(&mut h);
    let end = source.chars().count();
    h.state_mut().script_tools.jump = Some((key, end));
    settle(&mut h);
    assert!(in_sight(&h, "int nLine199 "));
    let pasted: String = (0..40).map(|n| format!("// pasted {n}\n")).collect();
    h.event(egui::Event::Paste(pasted.clone()));
    h.step();
    let last = row(&h, "// pasted 39").expect("laid out");
    let field = h
        .get_all_by_role(egui::accesskit::Role::MultilineTextInput)
        .map(|n| n.rect())
        .find(|r| r.x_range().contains(last.center().x) && r.top() <= last.top())
        .expect("the editor");
    assert!(field.bottom() >= last.bottom(), "the paste in the editor's room: {field:?} {last:?}");
    // (Scrolled in the paste's frame; the rows are seen where they are
    // drawn, the frame after.)
    h.step();
    assert!(in_sight(&h, "// pasted 39"), "the paste's last line in sight at once");
    settle(&mut h);
    assert!(in_sight(&h, "// pasted 39"));
    assert_eq!(text(&h, key), format!("{source}{pasted}"));
}

/// Shift+click in an editor that never had a cursor puts it at the click,
/// as egui's TextEdit does (it selected from the text's start: review
/// K's N7).
#[test]
fn shift_click_in_a_fresh_editor_puts_the_cursor_there() {
    let (mut h, key) = editor("ui-script-shift-click", "int a;\nint b;\nint c;\n");
    assert!(
        egui::text_edit::TextEditState::load(&h.ctx, id(key))
            .and_then(|s| s.cursor.char_range())
            .is_none()
    );
    let at = row(&h, "int c;").unwrap().left_center() + egui::vec2(1.0, 0.0);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::SHIFT));
    h.event(egui::Event::PointerMoved(at));
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::SHIFT,
        });
    }
    h.step();
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run();
    assert_eq!(cursor(&h, key), (14, 14), "at the click, nothing selected");
}

/// A Mac's Ctrl+A, E, B, F, P and N move the cursor (line start and end,
/// a character, a line), and Ctrl+K, Ctrl+U, Ctrl+W and Ctrl+H delete to
/// the line's end, to its start, the word before and the character before,
/// as egui's TextEdit has them (review K's N8, S's 3); Cmd+Backspace at a
/// line's start and Cmd+Delete at its end take the line end (N9).
#[test]
fn a_mac_s_control_keys_move_and_delete() {
    use egui::Key;
    let first = "int abc;\nint de;\n";
    let (mut h, key) = editor("ui-script-mac-keys", first);
    h.ctx.set_os(egui::os::OperatingSystem::Mac);
    let ctrl = egui::Modifiers::CTRL;
    put(&mut h, key, 6, 6);
    for (k, at) in [(Key::A, 0), (Key::E, 8), (Key::B, 7), (Key::F, 8), (Key::N, 16)] {
        press(&mut h, ctrl, k);
        assert_eq!(cursor(&h, key), (at, at), "Ctrl+{k:?}");
    }
    press(&mut h, ctrl, Key::P);
    assert_eq!(cursor(&h, key), (8, 8), "Ctrl+P");
    put(&mut h, key, 13, 13);
    press(&mut h, ctrl, Key::K);
    assert_eq!(text(&h, key), "int abc;\nint \n", "Ctrl+K");
    press(&mut h, ctrl, Key::U);
    assert_eq!(text(&h, key), "int abc;\n\n", "Ctrl+U");
    put(&mut h, key, 7, 7);
    press(&mut h, ctrl, Key::W);
    assert_eq!(text(&h, key), "int ;\n\n", "Ctrl+W");
    // Cmd+Backspace at the second line's start, Cmd+Delete at the first's end.
    put(&mut h, key, 6, 6);
    press(&mut h, MAC_CMD, Key::Backspace);
    assert_eq!(text(&h, key), "int ;\n", "Cmd+Backspace took the line end");
    put(&mut h, key, 5, 5);
    press(&mut h, MAC_CMD, Key::Delete);
    assert_eq!(text(&h, key), "int ;", "Cmd+Delete took the line end");
    assert_eq!(cursor(&h, key), (5, 5));
    press(&mut h, ctrl, Key::H);
    assert_eq!(text(&h, key), "int ", "Ctrl+H took the character before");
    assert_eq!(cursor(&h, key), (4, 4));
}

/// Ctrl+K and Ctrl+U on Windows and Linux too, as egui has them (Ctrl+H
/// is Replace Text's, Ctrl+W Close Tab's there).
#[test]
fn ctrl_k_and_ctrl_u_delete_to_the_line_s_end_and_start() {
    let (mut h, key) = editor("ui-script-ctrl-k-u", "int abc;\nint de;\n");
    put(&mut h, key, 6, 6);
    press(&mut h, CTRL, egui::Key::K);
    assert_eq!(text(&h, key), "int ab\nint de;\n");
    press(&mut h, CTRL, egui::Key::U);
    assert_eq!(text(&h, key), "\nint de;\n");
    // Ctrl+W is Close Tab's: no word taken.
    put(&mut h, key, 5, 5);
    press(&mut h, CTRL, egui::Key::W);
    assert_eq!(text(&h, key), "\nint de;\n", "Ctrl+W took no word");
    // Nor Ctrl+H, Replace Text's, a character.
    press(&mut h, CTRL, egui::Key::H);
    assert_eq!(text(&h, key), "\nint de;\n", "Ctrl+H took no character");
}

/// Word moves and word deletes go over line ends, as egui's (they stopped
/// at each line's start and end: review K's N10).
#[test]
fn word_moves_and_deletes_go_over_line_ends() {
    use egui::Key;
    use egui::text::CCursor;
    use egui::text_selection::text_cursor_state::{ccursor_next_word, ccursor_previous_word};
    let first = "int abc;\n\n  int de;\n\n\n  x\n";
    let total = first.chars().count();
    let (mut h, key) = editor("ui-script-word-lines", first);
    // From the start to the end and back, each step where egui's lands in
    // the whole text, over the line ends and blank lines.
    put(&mut h, key, 0, 0);
    let mut at = 0;
    while at < total {
        press(&mut h, CTRL, Key::ArrowRight);
        let egui_s = usize::from(ccursor_next_word(first, CCursor::new(at)).index).min(total);
        assert_eq!(cursor(&h, key), (egui_s, egui_s), "Ctrl+Right from {at}");
        at = egui_s;
    }
    while at > 0 {
        press(&mut h, CTRL, Key::ArrowLeft);
        let egui_s = usize::from(ccursor_previous_word(first, CCursor::new(at)).index);
        assert_eq!(cursor(&h, key), (egui_s, egui_s), "Ctrl+Left from {at}");
        at = egui_s;
    }
    // A line start's word delete takes the line end before it, as egui's.
    let line = first.find("  x").unwrap();
    put(&mut h, key, line, line);
    press(&mut h, CTRL, Key::Backspace);
    assert_eq!(text(&h, key), "int abc;\n\n  int de;\n\n  x\n", "Ctrl+Backspace");
    put(&mut h, key, 8, 8);
    press(&mut h, CTRL, Key::Delete);
    assert_eq!(text(&h, key), "int abc;\n  int de;\n\n  x\n", "Ctrl+Delete");
}

/// The threaded check as you type and symbols of a long script (over 16
/// KiB) with the window's background jobs on, as in the app: each on its
/// worker, and their results applied (UI tests run them inline otherwise,
/// and these paths ran in none: review A on 3f116b9d, D10).
#[test]
fn a_long_script_is_checked_and_read_on_workers() {
    let filler: String = (0..1200).map(|n| format!("int nFill{n} = {n};\n")).collect();
    let source = format!("{filler}void main() {{}}\n");
    assert!(source.len() > 16_384);
    let (mut h, keys) = editors("ui-script-workers", &[("long", &source)], |app| {
        app.background_jobs = true;
    });
    let key = keys[0];
    assert!(h.state().script_error_for_test(key).is_none());
    assert_eq!(h.state().script_workers_for_test().1, (0, 0));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Script(key)));
    let wait = |h: &mut Harness<'_, Moonglow>, what: &str, done: &dyn Fn(&Moonglow) -> bool| {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
        while !done(h.state()) {
            assert!(std::time::Instant::now() < deadline, "{what}");
            h.step();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    };
    wait(&mut h, "the script read", &|s| s.script_has_symbol_for_test(key, "nFill5"));
    // Checked at once on opening, on a worker, its result applied. (Without
    // the game's nwscript.nss the check finds that missing.)
    wait(&mut h, "the first check's result", &|s| s.script_error_for_test(key).is_some());
    assert_eq!(h.state().script_workers_for_test().1.0, 1, "the first check on a worker");
    let end = source.chars().count();
    put(&mut h, key, end, end);
    let (_, started) = h.state().script_workers_for_test();
    h.event(egui::Event::Paste("void Zed() { int }\n".into()));
    h.step();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    // (The check waits for a pause in the typing, half a second.)
    loop {
        let ((checking, reading), now) = h.state().script_workers_for_test();
        let found = h.state().script_has_symbol_for_test(key, "Zed");
        if found && now.0 > started.0 && now.1 > started.1 && !checking && !reading {
            break;
        }
        assert!(std::time::Instant::now() < deadline, "{found} {started:?} then {now:?}");
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}
