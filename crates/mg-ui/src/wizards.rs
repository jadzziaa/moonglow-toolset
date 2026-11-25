//! The Module and Area wizards (Aurora's File > New and Wizards > Area
//! Wizard), and the unsaved-changes question.

use egui::Ui;
use mg_module::new::{AREA_SIZE_RANGE, AREA_SIZES, AreaSpec, TilesetChoice};
use mg_uikit::Controls as _;

use crate::{Action, Moonglow};

/// An open wizard window.
#[derive(Debug, Clone, PartialEq)]
pub enum Wizard {
    NewModule { name: String },
    NewArea(AreaWizard),
}

/// The Area Wizard's choices.
#[derive(Debug, Clone, PartialEq)]
pub struct AreaWizard {
    pub name: String,
    pub tilesets: Vec<TilesetChoice>,
    pub selected: Option<usize>,
    pub width: u32,
    pub height: u32,
    /// Launch Area Properties Dialog (Aurora's default: off).
    pub launch_properties: bool,
    /// Open Area in the Area Viewer (Aurora's default: on).
    pub open_viewer: bool,
    /// Not yet shown: its tileset list starts at the top (not where the
    /// last one was left).
    fresh: bool,
    /// The tilesets the project hides (`[tilesets] hidden`), left out of
    /// the list unless `show_hidden`.
    pub hidden: Vec<mg_core::ResRef>,
    /// Show Hidden Tilesets: off as the wizard opens.
    pub show_hidden: bool,
    /// The page shown: 0 Name and Tileset, 1 Size, 2 Finish.
    pub page: usize,
}

impl AreaWizard {
    /// Aurora's defaults, "Area NNN" after the module's areas and Medium
    /// (8×8), with the first tileset chosen (Aurora chooses none).
    pub fn new(tilesets: Vec<TilesetChoice>, areas: usize) -> AreaWizard {
        AreaWizard {
            name: format!("Area {:03}", areas + 1),
            selected: (!tilesets.is_empty()).then_some(0),
            tilesets,
            width: 8,
            height: 8,
            launch_properties: false,
            open_viewer: true,
            fresh: true,
            hidden: Vec::new(),
            show_hidden: false,
            page: 0,
        }
    }

    /// The wizard with the tilesets the project hides (`patterns`, its
    /// `[tilesets] hidden`) left out of its list; the first shown chosen.
    pub fn hiding(mut self, patterns: &mg_module::project_file::Patterns) -> AreaWizard {
        self.hidden = (self.tilesets.iter())
            .map(|t| t.resref)
            .filter(|r| patterns.matches(&r.to_string()))
            .collect();
        self.selected = self.tilesets.iter().position(|t| self.shows(t));
        self
    }

    /// Whether the list shows `t`: a tileset not hidden, or every one
    /// with Show Hidden Tilesets on.
    fn shows(&self, t: &TilesetChoice) -> bool {
        self.show_hidden || !self.hidden.contains(&t.resref)
    }

    /// Whether the page shown is complete, so Next is on: a name and a
    /// tileset on the first, a size the game takes on the second.
    pub fn complete(&self) -> bool {
        match self.page {
            0 => !self.name.trim().is_empty() && self.selected.is_some(),
            1 => [self.width, self.height].iter().all(|n| AREA_SIZE_RANGE.contains(n)),
            _ => self.spec().is_some_and(|s| !s.name.is_empty()),
        }
    }

    fn spec(&self) -> Option<AreaSpec> {
        let t = self.tilesets.get(self.selected?)?;
        Some(AreaSpec {
            name: self.name.trim().to_string(),
            tileset: t.resref,
            width: self.width,
            height: self.height,
        })
    }
}

/// The Area Wizard's Size page (the Dialogs board's; its bar says Size):
/// Tiny, Small, Medium and Large as a segmented control across the dialog, each setting the rows
/// and the columns, none chosen where the two are another size; under it
/// the rows × the columns, any size the game takes, and what a tile is.
fn area_size(ui: &mut Ui, w: &mut AreaWizard) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        let mut chosen =
            AREA_SIZES.iter().find(|(_, n)| (w.width, w.height) == (*n, *n)).map(|c| c.1);
        let choices: Vec<(Option<u32>, &str, &str)> =
            AREA_SIZES.iter().map(|(label, n)| (Some(*n), *label, "")).collect();
        // (In a row a control high: the control is laid out in the middle
        // of the height it is given, and the body's scroll gives it all.)
        // (The group named "Area size".)
        let r = ui
            .horizontal(|ui| mg_uikit::segmented_fill(ui, "Area size", &mut chosen, &choices))
            .inner;
        if r.changed()
            && let Some(n) = chosen
        {
            (w.width, w.height) = (n, n);
        }
    });
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        crate::widgets::spin_draft(ui, &mut w.width, AREA_SIZE_RANGE, 1.0, |d| d);
        ui.label(egui::RichText::new("×").color(mg_uikit::tokens(ui.ctx()).text_2));
        crate::widgets::spin_draft(ui, &mut w.height, AREA_SIZE_RANGE, 1.0, |d| d);
        ui.weak("tiles, 10 m each");
    });
}

/// The Area Wizard's pages and their bars' icons (Aurora's `TdlgAreaWizard`:
/// Name and Tileset, Size, Finish).
const AREA_PAGES: [(mg_uikit::Icon, &str); 3] = [
    (mg_uikit::Icon::Tile, "Name and Tileset"),
    (mg_uikit::Icon::Grid, "Size"),
    (mg_uikit::Icon::CheckCircle, "Finish"),
];

/// The Area Wizard's step list: the pages before the one shown done, it
/// current, those after to come.
fn area_steps(ui: &mut Ui, w: &AreaWizard) {
    use mg_uikit::dialog::{StepList, StepState};
    let mut list = StepList::new(format!("Step {} of {}", w.page + 1, AREA_PAGES.len()));
    for (i, (_, name)) in AREA_PAGES.iter().enumerate() {
        let state = match i.cmp(&w.page) {
            std::cmp::Ordering::Less => StepState::Done,
            std::cmp::Ordering::Equal => StepState::Current,
            std::cmp::Ordering::Greater => StepState::ToCome,
        };
        list = list.step(*name, state);
    }
    let _ = list.show(ui);
}

/// The first page: the name (and the ResRef the builder's code gives it) and
/// the tileset list, with `room` points to fill; a double click on a tileset
/// goes on to Size (Aurora's `lbTileSetsDblClick`).
fn name_page(ui: &mut Ui, w: &mut AreaWizard, ws: Option<&crate::Workspace>, room: f32) {
    crate::widgets::field_label(ui, "Name");
    let field = ui.add(mg_uikit::text_field(&mut w.name));
    crate::widgets::autofocus(ui, &field);
    // The ResRef the name gives, where the builder's code makes it other
    // than the name (as Finish makes it).
    if let Some(module) = ws.map(|ws| &ws.module) {
        let naming = module.project_file.naming_for(mg_core::ResType::ARE);
        if !naming.is_plain() {
            let taken = |r: &mg_core::ResRef| {
                module.contains(&mg_resman::ResKey::new(*r, mg_core::ResType::ARE))
            };
            let made = mg_module::new::resref_for_named(w.name.trim(), &naming, taken);
            ui.weak(format!("ResRef: {made}"))
                .on_hover_text(crate::blueprint_wizard::made_from_the_name(&naming));
        }
    }
    ui.add_space(6.0);
    crate::widgets::field_label(ui, "Tileset");
    let mut next = false;
    egui::Frame::group(ui.style()).show(ui, |ui| {
        // (What the page has, less the name's rows and the check box.)
        let high = (room - 150.0).clamp(120.0, 420.0);
        let mut list = egui::ScrollArea::vertical().max_height(high);
        if std::mem::take(&mut w.fresh) {
            list = list.vertical_scroll_offset(0.0);
        }
        list.show(ui, |ui| {
            ui.set_width(280.0);
            mg_uikit::list_rows(ui);
            for (i, t) in w.tilesets.iter().enumerate() {
                if !w.shows(t) {
                    continue;
                }
                // (Chosen, filled across the row, not behind its words
                // alone: the Wizard board's, U30's #11. The name at the
                // row's left.)
                let row = egui::Button::selectable(
                    w.selected == Some(i),
                    (t.name.as_str(), egui::Atom::grow()),
                )
                .min_size(egui::vec2(ui.available_width(), 0.0));
                let mut label = ui.add(row);
                if w.hidden.contains(&t.resref) {
                    label = label.on_hover_text(format!(
                        "{}: hidden by the project (moonglow.toml's [tilesets])",
                        t.resref
                    ));
                }
                if label.clicked() {
                    w.selected = Some(i);
                }
                next |= label.double_clicked();
            }
        });
    });
    // The project's hidden tilesets, to reach all the same.
    if !w.hidden.is_empty() {
        let label = format!("Show hidden tilesets ({})", w.hidden.len());
        if ui
            .check_box(&mut w.show_hidden, label)
            .on_hover_text(
                "Tilesets the project's moonglow.toml hides. Areas on them still open as usual.",
            )
            .changed()
            && !w.show_hidden
            && let Some(i) = w.selected
            && !w.shows(&w.tilesets[i])
        {
            w.selected = w.tilesets.iter().position(|t| w.shows(t));
        }
    }
    if next && w.complete() {
        w.page = 1;
    }
}

/// The last page: what will be made, and what is done after.
fn finish_page(ui: &mut Ui, w: &mut AreaWizard) {
    if let Some(t) = w.selected.and_then(|i| w.tilesets.get(i)) {
        ui.label(format!(
            "The area \"{}\" will be made with {} by {} tiles of the {} tileset.",
            w.name.trim(),
            w.width,
            w.height,
            t.name
        ));
    }
    ui.add_space(6.0);
    ui.check_box(&mut w.launch_properties, "Open Area Properties");
    ui.check_box(&mut w.open_viewer, "Open area in the Area Viewer");
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    let ctx = ui.ctx().clone();
    let mut close = false;
    match &mut app.wizard {
        None => {}
        Some(Wizard::NewModule { name }) => {
            let footed = crate::widgets::modal(&ctx, "New Module").show_foot(&ctx, |ui| {
                crate::widgets::field_label(ui, "Module name");
                let field = ui.add(mg_uikit::text_field(name));
                crate::widgets::autofocus(ui, &field);
                let ok = !name.trim().is_empty();
                ((), crate::widgets::Foot::new().main("Create", ok).cancel())
            });
            if let Some(f) = footed {
                if f.pressed.main() {
                    app.actions.push(Action::NewModule(name.trim().to_string()));
                    close = true;
                }
                if f.pressed.cancelled() {
                    close = true;
                }
            }
        }
        Some(Wizard::NewArea(w)) => {
            let last = w.page + 1 == AREA_PAGES.len();
            let footed = crate::widgets::modal(&ctx, "Area Wizard")
                .resizable(true)
                .default_size([600.0, 440.0])
                .min_size([420.0, 320.0])
                .flush()
                .show_foot(&ctx, |ui| {
                    // The step list at the left (the Dialogs board's), the
                    // page under its bar beside it, as the other wizards.
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        if crate::widgets::step_list_fits(ui) {
                            area_steps(ui, w);
                        }
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            let (icon, name) = AREA_PAGES[w.page];
                            mg_uikit::dialog::page_bar(ui, Some(icon), name);
                            let pad = mg_uikit::dialog::PAGE_PADDING;
                            egui::Frame::NONE
                                .inner_margin(egui::Margin::symmetric(pad.x as i8, pad.y as i8))
                                .show(ui, |ui| {
                                    ui.set_min_size(ui.available_size());
                                    let room = ui.available_height();
                                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(
                                        ui,
                                        |ui| match w.page {
                                            0 => name_page(ui, w, app.ws.as_ref(), room),
                                            1 => area_size(ui, w),
                                            _ => finish_page(ui, w),
                                        },
                                    );
                                });
                        });
                    });
                    let foot = crate::creature_wizard::wizard_foot(
                        w.page > 0,
                        !last && w.complete(),
                        last,
                    );
                    (w.spec().filter(|s| !s.name.is_empty()), foot)
                });
            if let Some(f) = footed {
                if f.pressed.clicked("< Back") {
                    w.page = w.page.saturating_sub(1);
                } else if f.pressed.main() && !last {
                    w.page += 1;
                } else if f.pressed.main()
                    && let Some(spec) = f.inner.flatten()
                {
                    app.actions.push(Action::NewArea(spec));
                    app.after_new_area = (w.open_viewer, w.launch_properties);
                    close = true;
                }
                if f.pressed.cancelled() {
                    close = true;
                }
            }
        }
    }
    if close {
        app.wizard = None;
    }

    if app.closing {
        let mut open = true;
        let mut answer = None;
        let always_id = egui::Id::new("close-always");
        let (on, tip) = crate::widgets::always_now(
            &ctx,
            always_id,
            "Tools › Options › General › Questions › Ask before the window is closed",
        );
        let footed = crate::widgets::dialog(&ctx, "Close Moonglow?", &mut open)
            .collapsible(false)
            .resizable(false)
            .show_foot(&ctx, |ui| {
                ui.label("Nothing is unsaved. The tabs that are open will be closed.");
                let foot = crate::widgets::Foot::new()
                    .check("Don't ask again", on, Some(tip))
                    .main("Close Moonglow", true)
                    .no_enter()
                    .cancel();
                ((), foot)
            });
        let mut always = false;
        if let Some(f) = footed {
            always = crate::widgets::always_kept(&ctx, always_id, f.pressed.checked);
            if f.pressed.main() {
                answer = Some(true);
            }
            if f.pressed.cancelled() {
                answer = Some(false);
            }
        }
        if let Some(close) = answer {
            app.closing = false;
            if close {
                // (Not asked again: Options' switch, from now on.)
                if always {
                    app.settings.close_without_asking = true;
                }
                app.actions.push(Action::Quit);
            }
        } else if !open {
            app.closing = false;
        }
    }

    if let Some(pending) = app.confirm_discard.clone() {
        let what = app
            .module_path()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "the new module".into());
        let failed = app.module_failed.as_ref().map(|why| format!("It could not be saved: {why}"));
        let footed = crate::widgets::modal(&ctx, "Unsaved Changes").show_foot(&ctx, |ui| {
            ui.label(format!("Save changes to {what}?"));
            // (Its Save failed last time: why, under the question.)
            if let Some(failed) = &failed {
                crate::widgets::fit_min_width(ui, 440.0);
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(failed).color(ui.visuals().error_fg_color),
                    )
                    .wrap(),
                );
            }
            ((), crate::widgets::Foot::new().main("Save", true).button("Don't Save", true).cancel())
        });
        if let Some(f) = footed {
            if f.pressed.main() {
                app.actions.push(Action::SaveThen(Box::new(pending)));
                app.confirm_discard = None;
            } else if f.pressed.clicked("Don't Save") {
                app.actions.push(Action::Proceed(Box::new(pending)));
                app.confirm_discard = None;
            } else if f.pressed.cancelled() {
                app.confirm_discard = None;
            }
        }
        if app.confirm_discard.is_none() {
            app.module_failed = None;
        }
    }
    crate::module_folder::ui(app, &ctx);
}
