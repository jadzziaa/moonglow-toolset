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

/// The panel's rows: what does not read, then the findings, errors first,
/// each with the control it is about and the fixes there are for it.
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
    for (what, error) in [("JUI", window_error), ("Settings", settings_error)] {
        if let Some(e) = error {
            ui.horizontal(|ui| {
                ui.set_min_height(line);
                mark(ui, true);
                ui.label(egui::RichText::new(what).color(t.accent));
                ui.add(egui::Label::new(e).truncate()).on_hover_text(e);
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
                    ui.set_min_height(line);
                    mark(ui, d.severity == Severity::Error);
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
                        }
                        None => {
                            ui.label(words);
                        }
                    }
                    // (The fix at the row's right, the message the room
                    // between, cut where it ends and whole in its tip.)
                    let fix = fix_width(ui, d);
                    let room = (ui.available_width() - fix).max(60.0);
                    ui.scope(|ui| {
                        ui.set_max_width(room);
                        ui.add(egui::Label::new(&d.message).truncate());
                    });
                    if let Some(w) = &mut work {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            fixes(ui, &mut w.window, &mut w.settings, d, target.as_deref());
                        });
                    }
                })
            });
        }
    });
    if let Some(w) = &mut work {
        main_root(w, state, false);
    }
    state.work = work;
    state.problem_rows = Some(rows);
}
