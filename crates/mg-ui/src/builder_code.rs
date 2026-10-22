//! Edit › Builder Code…: the builder's own code, which the ResRefs
//! Moonglow proposes for new resources begin with (the area and blueprint
//! wizards, Add to Palette, Setup Store, Copy…, the Script Wizard). It is
//! kept in the project's `.moonglow/user.toml`, the builder's own file
//! that version control leaves out; the team's `moonglow.toml` may say
//! how it goes into a ResRef (`[resrefs] pattern`).

use std::path::PathBuf;

use mg_module::project_file::{DEFAULT_PATTERN, MAX_CODE, Naming, ProjectFile, code_problem};

use crate::Moonglow;

/// The window's draft: the code as typed, and the project's root it is
/// written in.
#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    pub code: String,
    pub root: PathBuf,
}

/// The project's root of the open module: none for a module kept in one
/// file (`.mod`), whose folder holds other modules too.
pub(crate) fn root(app: &Moonglow) -> Option<PathBuf> {
    let ws = app.ws.as_ref()?;
    ws.module.location.as_ref().and_then(ProjectFile::root_of).map(PathBuf::from)
}

/// Opens the window on the code the builder has now.
pub(crate) fn open(app: &mut Moonglow) {
    let Some(root) = root(app) else {
        app.log.warn(
            "A module kept in one file (.mod) has no project to keep a builder code in: open \
             its folder or nasher project",
        );
        return;
    };
    let code = app.ws.as_ref().and_then(|ws| ws.module.project_file.user.builder_code.clone());
    app.builder_code = Some(Draft { code: code.unwrap_or_default(), root });
}

/// Writes the code (or, empty, takes it out) and reads the project's
/// files again, so that what is proposed next follows it.
pub(crate) fn set(app: &mut Moonglow, root: &std::path::Path, code: Option<&str>) {
    match ProjectFile::set_builder_code(root, code) {
        Ok(()) => {
            match code {
                Some(c) => app.log.info(format!(
                    "Your builder code is {c}: new ResRefs begin with it (kept in \
                     .moonglow/user.toml)"
                )),
                None => app.log.info("No builder code: new ResRefs are made from names alone"),
            }
            if let Some(ws) = app.ws.as_mut() {
                let had = ws.module.project_file.found;
                if ws.module.reread_project_file() {
                    app.log_project_file(true, had);
                }
            }
        }
        Err(e) => app.log.error(format!("The builder code was not written: {e}")),
    }
}

pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(mut draft) = app.builder_code.take() else { return };
    let pattern = app
        .ws
        .as_ref()
        .and_then(|ws| ws.module.project_file.resref_pattern.clone())
        .unwrap_or_else(|| DEFAULT_PATTERN.to_string());
    let mut open = true;
    let (mut ok, mut clear, mut cancel) = (false, false, false);
    crate::widgets::dialog(ctx, "Builder Code", &mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.set_max_width(420.0);
            ui.label(
                "Your own code: the ResRefs Moonglow proposes for new areas, blueprints, copies \
                 and scripts begin with it, so that two builders' new resources never share a \
                 name.",
            );
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                crate::widgets::field_label(ui, "Code");
                let field = ui.add(
                    egui::TextEdit::singleline(&mut draft.code)
                        .char_limit(MAX_CODE + 4)
                        .desired_width(120.0)
                        .hint_text("ab"),
                );
                crate::widgets::autofocus(ui, &field);
            });
            let code = draft.code.trim().to_ascii_lowercase();
            let problem = (!code.is_empty()).then(|| code_problem(&code)).flatten();
            match &problem {
                Some(p) => {
                    ui.colored_label(ui.visuals().warn_fg_color, p);
                }
                None if code.is_empty() => {
                    ui.weak("None: ResRefs are made from names alone, as Aurora makes them.");
                }
                None => {
                    let naming = Naming::new(&code, &pattern);
                    ui.weak(format!(
                        "A new area named Market: {}. A copy of nw_chicken: {}.",
                        naming.fit("market", ""),
                        naming.fit("nw_chicken", "001")
                    ));
                }
            }
            ui.weak(format!(
                "Kept in this project's .moonglow/user.toml, which version control leaves out. \
                 The team's moonglow.toml says where it goes ([resrefs] pattern: {pattern})."
            ));
            ui.separator();
            ui.horizontal(|ui| {
                let can = problem.is_none();
                ok = ui.add_enabled(can, egui::Button::new("OK")).clicked()
                    || (can && crate::widgets::enter(ui));
                clear = ui.button("No Code").on_hover_text("Take your code out").clicked();
                cancel = crate::widgets::cancel(ui);
            });
        });
    if ok {
        let code = draft.code.trim().to_ascii_lowercase();
        set(app, &draft.root, (!code.is_empty()).then_some(code.as_str()));
    } else if clear {
        set(app, &draft.root, None);
    } else if open && !cancel {
        app.builder_code = Some(draft);
    }
}
