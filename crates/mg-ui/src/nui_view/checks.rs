//! The NUI designer's Checks (M4's U19, the NUI board's Bottom Dock): what
//! the checks of the window found, a row each (its kind's mark and word, the
//! control it is about as a link that chooses it, its message cut to its
//! line and whole in its tip, its fix), errors first; a window or settings
//! that do not read first of all. Its Status Bar switch counts them (the
//! board's "1"), and only new errors open the Bottom Dock on it (U13's rule:
//! a failure). The rows are made once for the findings they show and only
//! those in sight are laid out, as the list under the header had them.
use super::*;

/// The name of the panel's rows (a screen reader's, the tests').
pub const CHECKS_ROWS: &str = "Checks rows";

/// What a designer's checks count: its errors (a window or settings that do
/// not read among them) and its warnings, as last checked.
pub(crate) fn counts(state: &State) -> Option<(usize, usize)> {
    let findings = &state.checks.as_ref()?.findings;
    let source = state.source.as_ref()?;
    let errors = findings.iter().filter(|d| d.severity == Severity::Error).count()
        + usize::from(source.window.is_err())
        + usize::from(source.settings.is_err());
    let warnings = findings.iter().filter(|d| d.severity == Severity::Warning).count();
    Some((errors, warnings))
}

/// The Checks switch's badge for the designer `editor` is (or whose source
/// group it is): its errors and warnings together, in `error` where one is
/// an error, else in `warn`; none where the checks found nothing.
pub(crate) fn badge(app: &Moonglow, editor: Option<&Tab>) -> Option<mg_uikit::Badge> {
    let state = app.nui_views.get(&panels::nui_of(editor)?)?;
    let (errors, warnings) = counts(state)?;
    let kind = if errors > 0 { mg_uikit::BadgeKind::Error } else { mg_uikit::BadgeKind::Warn };
    (errors + warnings > 0).then(|| mg_uikit::Badge::count(errors + warnings).kind(kind))
}

/// What the panel's bar says after its tab (the board's "crier: 0 errors ·
/// 1 warning").
pub(crate) fn detail(app: &Moonglow, editor: Option<&Tab>) -> Option<String> {
    let key = panels::nui_of(editor)?;
    let (errors, warnings) = counts(app.nui_views.get(&key)?)?;
    Some(format!("{}: {} · {}", key.resref, count(errors, "error"), count(warnings, "warning")))
}

/// More errors than the designer's checks found last frame: its Checks
/// asked to the front, the Bottom Dock opened on it (in its window's
/// bottom pane where it floats), as a script's failed compile opens its
/// Compiler (`shell::bottom_forward`, at the next frame's start). Fewer, or
/// warnings, open nothing.
pub(super) fn note_failures(app: &mut Moonglow, key: ResKey, checked: &Checked, state: &mut State) {
    if checked.errors > state.issues_seen {
        app.nui_checks_forward.push(key);
    }
    state.issues_seen = checked.errors;
}

/// The room a finding's fix takes at its row's right: its button's words
/// and its padding (none without one), so that each is whole ("Add Close
/// Handler"), the message cut short before it.
fn fix_width(ui: &Ui, d: &mg_nui::Diagnostic) -> f32 {
    let words = if d.message.starts_with("What the player enters here is lost") {
        "Bind It"
    } else if d.path.ends_with("/draw_list_scissor") {
        "Turn It Off"
    } else if d.message.starts_with("The mg_close button has no Clicked handler") {
        "Add Close Handler"
    } else {
        return 0.0;
    };
    let font = egui::TextStyle::Small.resolve(ui.style());
    let wide =
        ui.fonts_mut(|f| f.layout_no_wrap(words.into(), font, egui::Color32::WHITE)).size().x;
    wide + 2.0 * ui.spacing().button_padding.x + 2.0 * ui.spacing().item_spacing.x + 8.0
}

/// The Checks panel's body for the designer `editor` is.
pub(crate) fn panel(app: &mut Moonglow, ui: &mut Ui, editor: Option<&Tab>) {
    let Some(key) = panels::nui_of(editor) else { return };
    let Some(ws) = app.ws.as_ref() else { return };
    mg_uikit::name_group(ui, CHECKS_ROWS);
    let keys = crate::palette_view::Keys::NuiChecks;
    crate::palette_view::keys_outline(ui, ui.max_rect(), keys);
    if ui.input(|i| i.pointer.any_pressed()) && ui.rect_contains_pointer(ui.max_rect()) {
        crate::palette_view::give_keys(ui.ctx(), keys);
    }
    let focus = app.dialog_panel_focus.take_if(|k| *k == keys).is_some();
    let mut state = app.nui_views.remove(&key).unwrap_or_default();
    let checked = match begin(&ws.module, ui.ctx(), key, &mut state) {
        Ok(checked) => checked,
        Err(why) => {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.weak(why);
            });
            app.nui_views.insert(key, state);
            return;
        }
    };
    let margin = egui::Margin { left: 8, right: 8, top: 2, bottom: 2 };
    egui::Frame::new().inner_margin(margin).show(ui, |ui| {
        rows(ui, &checked, &mut state, focus);
    });
    app.nui_views.insert(key, state);
}

/// A finding as one line of text (its row's menu's Copy, a line of Copy
/// All): its kind, the control it is about and its whole message, "Error:
/// Window › Row › OK, The mg_close button has no Clicked handler".
fn copy_line(error: bool, place: &str, message: &str) -> String {
    let word = if error { "Error" } else { "Warning" };
    match place.is_empty() {
        true => format!("{word}: {message}"),
        false => format!("{word}: {place}, {message}"),
    }
}

/// Every row the panel lists, one a line, in its order (Copy All): what
/// does not read, then the findings.
fn all_lines(checked: &Checked, rows: &ProblemRows) -> String {
    let unread = [("JUI", &checked.window_error), ("Settings", &checked.settings_error)];
    let unread = unread.into_iter().filter_map(|(w, e)| Some(copy_line(true, w, e.as_ref()?)));
    let findings = rows.rows.iter().map(|(i, (_, place))| {
        let d = &checked.findings[*i];
        copy_line(d.severity == Severity::Error, place, &d.message)
    });
    crate::row_copy::lines(unread.chain(findings))
}

/// The panel's rows: what does not read, then the findings, errors first,
/// each with the control it is about, its message cut short (whole in its
/// tip) and the fixes there are for it. A row's menu (a right click)
/// copies it or every row (no row is chosen, so no Ctrl+C).
fn rows(ui: &mut Ui, checked: &Checked, state: &mut State, focus: bool) {
    let Checked { window_error, settings_error, findings, errors, warnings, .. } = checked;
    let t = mg_uikit::tokens(ui.ctx());
    let line = mg_uikit::metrics::CONTROL_HEIGHT + 6.0;
    let mark = |ui: &mut Ui, error: bool| {
        let (icon, color, word) = match error {
            true => (mg_uikit::Icon::Error, t.error, "Error"),
            false => (mg_uikit::Icon::Warning, t.warn, "Warning"),
        };
        let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
        icon.paint(ui.painter(), rect, color);
        // (The board's kind word in its column, 54 wide.)
        let size = egui::vec2(54.0, line);
        ui.allocate_ui_with_layout(size, egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.set_min_size(size);
            ui.colored_label(color, word);
        });
    };
    // (A row's menu's Copy or Copy All: its text made after the rows.)
    let mut copy: Option<(crate::row_copy::Asked, String)> = None;
    // (The row's own response first, under its link and fixes: a right
    // click on it opens its menu. Not a Tab stop: its place is, a link or
    // a label taking the keyboard.)
    let row_at = |ui: &mut Ui, id: egui::Id| {
        let room =
            egui::Rect::from_min_size(ui.cursor().min, egui::vec2(ui.available_width(), line));
        ui.interact(room, id, egui::Sense::CLICK)
    };
    // (A place that is no link: a Tab stop all the same, for the row's
    // keys; not selectable, a right click on it the row's.)
    let place_label = |ui: &mut Ui, words: egui::RichText| {
        ui.add(egui::Label::new(words).selectable(false).sense(egui::Sense::click()))
    };
    // (Its menu, after what is in it: by a right click on it or its link,
    // or Shift+F10 where the keyboard is on either; Ctrl+C there copies
    // it.)
    let row_menu =
        |ui: &mut Ui, row: egui::Response, copy: &mut Option<_>, words: &dyn Fn() -> String| {
            let (keyed, ctrl_c) = crate::row_copy::focused_keys(ui, &row);
            if ctrl_c {
                *copy = Some((crate::row_copy::Asked::Row, words()));
            }
            if let Some(asked) = crate::row_copy::row_menu(ui, &row, keyed, |_| false) {
                *copy = Some((asked, words()));
            }
        };
    for (what, error) in [("JUI", window_error), ("Settings", settings_error)] {
        if let Some(e) = error {
            ui.horizontal(|ui| {
                let id = ui.id().with(("unread-row", what));
                let row = row_at(ui, id);
                ui.set_min_height(line);
                mark(ui, true);
                let held = place_label(ui, egui::RichText::new(what).color(t.accent)) | row;
                // (Not selectable: a right click on its words is the row's.)
                ui.add(egui::Label::new(e).truncate().selectable(false)).on_hover_text(e);
                row_menu(ui, held, &mut copy, &|| copy_line(true, what, e));
            });
        }
    }
    // The findings in the order shown (errors first: they stop a Build),
    // each with the control it is about, made once for the findings they
    // are: a list of two thousand (a window written by a script, one mistake
    // a control) was sorted, every place looked up level by level and every
    // row laid out in every frame.
    let rows = state.problem_rows.take().filter(|r| std::rc::Rc::ptr_eq(&r.of, findings));
    let rows = rows.unwrap_or_else(|| {
        let mut sorted: Vec<usize> = (0..findings.len()).collect();
        sorted.sort_by_key(|i| findings[*i].severity != Severity::Error);
        let parsed = state.source.as_ref().and_then(|s| s.window.as_ref().ok());
        let place = |i: usize| match parsed {
            Some(doc) => describe_path(doc, &findings[i].path),
            None => (None, String::new()),
        };
        let rows = sorted.into_iter().map(|i| (i, place(i))).collect();
        ProblemRows { of: findings.clone(), rows }
    });
    // (Nothing found: said in the first row, before the rows' scroll, which
    // takes the rest of the panel.)
    if errors + warnings == 0 {
        ui.horizontal(|ui| {
            ui.set_min_height(line);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
            mg_uikit::Icon::CheckCircle.paint(ui.painter(), rect, t.ok);
            ui.weak("The checks found nothing.");
        });
    }
    // The fixes work on the window itself, not a variant edited in its
    // place: it is put back for them.
    let mut work = state.work.take();
    if let Some(w) = &mut work {
        main_root(w, state, true);
    }
    let area = egui::ScrollArea::vertical().id_salt("nui-checks").auto_shrink([false, false]);
    let mut first = focus;
    mg_uikit::show_rows(area, ui, rows.rows.len(), |ui, shown| {
        for (i, (target, place)) in &rows.rows[shown] {
            let d = &findings[*i];
            ui.push_id(i, |ui| {
                ui.horizontal(|ui| {
                    let error = d.severity == Severity::Error;
                    let id = ui.id().with("check-row");
                    let as_line = || copy_line(error, place, &d.message);
                    let row = row_at(ui, id);
                    let held;
                    ui.set_min_height(line);
                    mark(ui, error);
                    let words = egui::RichText::new(place).color(t.accent);
                    match target {
                        Some(path) => {
                            let link = ui.link(words).on_hover_text("Select it");
                            if std::mem::take(&mut first) {
                                link.request_focus();
                            }
                            if link.clicked() {
                                state.select_next = Some(path.clone());
                                ui.ctx().request_repaint();
                            }
                            // (Its menu and keys the link's too: its id
                            // the link's, which has the keyboard.)
                            held = link | row;
                        }
                        None => held = place_label(ui, words) | row,
                    }
                    // (The fix at the row's right, the message the room
                    // between, cut where it ends and whole in its tip.)
                    let fix = fix_width(ui, d);
                    let room = (ui.available_width() - fix).max(60.0);
                    ui.scope(|ui| {
                        ui.set_max_width(room);
                        // (Not selectable: a right click on its words
                        // is the row's, whose menu copies it whole. Cut
                        // short, it is whole in egui's tip of a label cut.)
                        let words = egui::Label::new(&d.message).truncate().selectable(false);
                        ui.add(words);
                    });
                    if let Some(w) = &mut work {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            fixes(ui, &mut w.window, &mut w.settings, d, target.as_deref());
                        });
                    }
                    row_menu(ui, held, &mut copy, &as_line);
                })
            });
        }
    });
    if let Some(w) = &mut work {
        main_root(w, state, false);
    }
    state.work = work;
    match copy {
        Some((crate::row_copy::Asked::Row, line)) => ui.ctx().copy_text(line),
        Some((crate::row_copy::Asked::All, _)) => ui.ctx().copy_text(all_lines(checked, &rows)),
        None => {}
    }
    state.problem_rows = Some(rows);
}
