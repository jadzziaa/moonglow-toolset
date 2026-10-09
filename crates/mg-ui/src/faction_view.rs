//! The Faction Editor (`repute.fac`): the factions, how each regards the
//! others, adding (from a parent), renaming and removing factions. Each
//! change is one undoable command that rewrites the file as Aurora does.

use egui::{Color32, Ui};
use mg_core::{ResRef, ResType};
use mg_edit::Command;
use mg_module::factions::{Factions, STANDARD, faction_users, renumber_faction_users};
use mg_resman::ResKey;

use crate::Moonglow;

/// The editor's view state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FactionView {
    pub advanced: bool,
    pub selected: u32,
    /// Also show how the others regard the selected faction.
    pub full_detail: bool,
    pub adding: Option<AddFaction>,
    pub renaming: Option<(u32, String)>,
}

/// The Add Faction window.
#[derive(Debug, Clone, PartialEq)]
pub struct AddFaction {
    pub name: String,
    pub global: bool,
    pub parent: u32,
}

fn key() -> ResKey {
    ResKey::new(ResRef::from_str("repute").expect("valid"), ResType::FAC)
}

/// Aurora's colours: hostile up to 10, friendly from 90, neutral between.
fn color(rep: u32) -> Color32 {
    match rep {
        0..=10 => Color32::from_rgb(170, 40, 40),
        90.. => Color32::from_rgb(50, 70, 190),
        _ => Color32::from_gray(120),
    }
}

/// How Aurora's status bar words a reputation.
pub fn attitude(rep: u32) -> &'static str {
    match rep {
        0..=10 => "hostile",
        90.. => "friendly",
        _ => "neutral",
    }
}

impl Moonglow {
    /// The factions as currently edited (the workspace's copy of
    /// `repute.fac`, or the standard factions).
    fn factions(&mut self) -> Option<Factions> {
        let ws = self.ws.as_mut()?;
        Some(match ws.doc(&key()) {
            Ok(g) => Factions::read_in(g, crate::text::game_codepage()),
            Err(_) => Factions::of_module(&ws.module),
        })
    }

    /// Replaces `repute.fac` (and, after a removal, the objects that were
    /// renumbered) through one command.
    fn store_factions(
        &mut self,
        what: &str,
        f: &Factions,
        renumber: Option<Box<dyn Fn(u32) -> Option<u32>>>,
    ) {
        let Some(ws) = &mut self.ws else { return };
        if let Err(e) = ws.flush() {
            self.log.error(e.to_string());
            return;
        }
        let mut staged = ws.module.clone();
        if staged.set_gff(key(), &f.to_gff()).is_err() {
            return;
        }
        if let Some(map) = renumber {
            renumber_faction_users(&mut staged, map);
        }
        let edits = ws.edits_to(&staged);
        if !edits.is_empty()
            && let Err(e) = self.apply(Command::new(what, edits))
        {
            self.log.error(e.to_string());
        }
    }
}

/// The width of a column of the Advanced grid (its heading's).
const HEAD_WIDTH: f32 = 64.0;

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    let Some(mut f) = app.factions() else {
        ui.label("No module is open.");
        return;
    };
    let mut view = app.faction_view.clone();
    let n = f.factions.len() as u32;
    view.selected = view.selected.min(n.saturating_sub(1));
    let mut changed: Option<String> = None;
    let mut remove = None;

    ui.horizontal(|ui| {
        ui.selectable_value(&mut view.advanced, false, "Basic");
        ui.selectable_value(&mut view.advanced, true, "Advanced");
    });
    ui.separator();
    // The chart or grid has the room; the list beside it what it needs.
    let room = ui.available_size();
    let list = 280.0_f32.min(room.x * 0.4);
    ui.horizontal_top(|row| {
        let main = egui::vec2((room.x - list - 12.0).max(120.0), room.y);
        let layout = egui::Layout::top_down(egui::Align::Min);
        row.allocate_ui_with_layout(main, layout, |ui| {
        ui.set_min_width(main.x);
        egui::ScrollArea::both().id_salt("faction-main").show(ui, |ui| {
            if view.advanced {
                // Rows: who regards (PC's own feelings are not stored);
                // columns: whom.
                egui::Grid::new("faction-grid").spacing([4.0, 4.0]).show(ui, |ui| {
                    crate::widgets::field_label(ui, "");
                    // A column as wide as its numbers: a long name is cut
                    // short there (whole on hover, and in its row).
                    for t in &f.factions {
                        let head = egui::Label::new(egui::RichText::new(&t.name).strong()).truncate();
                        let height = ui.text_style_height(&egui::TextStyle::Body);
                        ui.add_sized([HEAD_WIDTH, height], head).on_hover_text(&t.name);
                    }
                    ui.end_row();
                    for p in 1..n {
                        ui.strong(&f.factions[p as usize].name);
                        for t in 0..n {
                            let before = f.reputation(p, t).unwrap_or(mg_module::factions::DEFAULT_REPUTATION);
                            let (set, rep, r) = crate::widgets::drag_number_shown(ui, before, |d| {
                                d.range(0..=100).custom_formatter(|v, _| format!("{v:.0}"))
                            });
                            ui.painter().rect_stroke(r.rect, 2.0, egui::Stroke::new(2.0, color(rep)), egui::StrokeKind::Outside);
                            r.on_hover_text(format!(
                                "{} is {} toward {}",
                                f.factions[p as usize].name,
                                attitude(rep),
                                f.factions[t as usize].name
                            ));
                            if let Some(rep) = set {
                                f.set_reputation(p, t, rep);
                                changed = Some("Change reputation".into());
                            }
                        }
                        ui.end_row();
                    }
                });
            } else {
                let s = view.selected;
                ui.strong(format!("How {} regards the others", f.factions[s as usize].name));
                if s == 0 {
                    ui.weak("The PC faction's own feelings are not stored; see how the others regard PCs with Full Detail.");
                }
                egui::Grid::new("faction-bars").num_columns(3).show(ui, |ui| {
                    for t in 0..n {
                        let rows: Vec<(u32, u32, &str)> = if view.full_detail {
                            vec![(s, t, "regards"), (t, s, "is regarded by")]
                        } else {
                            vec![(s, t, "")]
                        };
                        for (p, target, how) in rows {
                            if p == 0 {
                                continue;
                            }
                            let label = if how.is_empty() {
                                f.factions[target as usize].name.clone()
                            } else if p == s {
                                format!("Toward {}", f.factions[target as usize].name)
                            } else {
                                format!("From {}", f.factions[p as usize].name)
                            };
                            crate::widgets::field_label(ui, label);
                            let mut rep = f.reputation(p, target).unwrap_or(mg_module::factions::DEFAULT_REPUTATION);
                            let before = rep;
                            let r = ui.add(egui::Slider::new(&mut rep, 0..=100).clamping(egui::SliderClamping::Edits).show_value(true));
                            let fill = r.rect.with_max_x(r.rect.min.x + 4.0);
                            ui.painter().rect_filled(fill, 0.0, color(rep));
                            ui.weak(attitude(rep));
                            if (r.drag_stopped() || (r.changed() && !r.dragged())) && rep != before {
                                f.set_reputation(p, target, rep);
                                changed = Some("Change reputation".into());
                            }
                            ui.end_row();
                        }
                    }
                });
                ui.checkbox(&mut view.full_detail, "Full Detail");
            }
        });

        });
        row.separator();
        // The faction list and its commands.
        row.vertical(|ui| {
        crate::widgets::section_heading(ui, "Factions");
        for (i, fac) in f.factions.iter().enumerate() {
            let r = ui.selectable_label(view.selected == i as u32, &fac.name);
            if r.clicked() {
                view.selected = i as u32;
            }
        }
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button("Add Faction…").clicked() {
                view.adding = Some(AddFaction { name: String::new(), global: true, parent: 1 });
            }
            let custom = view.selected as usize >= STANDARD.len();
            if ui.add_enabled(custom, egui::Button::new("Remove Faction")).clicked() {
                remove = Some(view.selected);
            }
            if ui.add_enabled(custom, egui::Button::new("Change Name…")).clicked() {
                view.renaming = Some((view.selected, f.factions[view.selected as usize].name.clone()));
            }
        });
        let sel = view.selected as usize;
        let mut global = f.factions[sel].global;
        if ui
            .add_enabled(sel >= STANDARD.len(), egui::Checkbox::new(&mut global, "Global Effect"))
            .on_hover_text("A change of reputation applies to the whole faction")
            .changed()
        {
            f.factions[sel].global = global;
            changed = Some("Faction global effect".into());
        }
        });
    });

    // Add Faction.
    let ctx = ui.ctx().clone();
    if let Some(mut add) = view.adding.clone() {
        let mut close = false;
        egui::Window::new("Add Faction")
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .collapsible(false)
            .resizable(false)
            .show(&ctx, |ui| {
                ui.label("Name");
                let field = ui.text_edit_singleline(&mut add.name);
                crate::widgets::autofocus(ui, &field);
                ui.checkbox(&mut add.global, "Global Effect");
                ui.label("Parent (its reputations are copied)");
                for p in 1..STANDARD.len() as u32 {
                    ui.selectable_value(&mut add.parent, p, &f.factions[p as usize].name);
                }
                ui.horizontal(|ui| {
                    let ok = !add.name.trim().is_empty();
                    if ui.add_enabled(ok, egui::Button::new("OK")).clicked()
                        || (ok && crate::widgets::enter(ui))
                    {
                        let id = f.add(add.name.trim(), add.global, add.parent);
                        view.selected = id;
                        changed = Some(format!("Add faction {}", add.name.trim()));
                        close = true;
                    }
                    if crate::widgets::cancel(ui) {
                        close = true;
                    }
                });
            });
        view.adding = if close { None } else { Some(add) };
    }
    if let Some((id, mut name)) = view.renaming.clone() {
        let mut close = false;
        egui::Window::new("Change Faction Name")
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(ctx.content_rect().center())
            .collapsible(false)
            .resizable(false)
            .show(&ctx, |ui| {
                let field = ui.text_edit_singleline(&mut name);
                crate::widgets::autofocus(ui, &field);
                ui.horizontal(|ui| {
                    if ui.add_enabled(!name.trim().is_empty(), egui::Button::new("OK")).clicked()
                        || (!name.trim().is_empty() && crate::widgets::enter(ui))
                    {
                        f.factions[id as usize].name = name.trim().to_string();
                        changed = Some("Rename faction".into());
                        close = true;
                    }
                    if crate::widgets::cancel(ui) {
                        close = true;
                    }
                });
            });
        view.renaming = if close { None } else { Some((id, name)) };
    }

    if let Some(id) = remove {
        let users = app.ws.as_ref().map(|w| faction_users(&w.module, id)).unwrap_or_default();
        if users.is_empty() {
            let name = f.factions[id as usize].name.clone();
            if let Some(map) = f.remove(id) {
                app.store_factions(&format!("Remove faction {name}"), &f, Some(Box::new(map)));
                view.selected = view.selected.min(f.factions.len() as u32 - 1);
            }
        } else {
            let list: Vec<String> = users.iter().map(ToString::to_string).collect();
            app.log.error(format!(
                "{} is used by {}; give those objects another faction first",
                f.factions[id as usize].name,
                list.join(", ")
            ));
        }
    } else if let Some(what) = changed {
        app.store_factions(&what, &f, None);
    }
    app.faction_view = view;
}
