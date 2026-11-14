//! The kit sheet: one of each standard egui control in its states under
//! the applied theme, the form helpers, the shell's parts (key caps,
//! badges, tabs, the Status Bar, the Top Bar's row and menu, a folded
//! dock's strip), the icon widgets in every state and the whole icon set,
//! to compare with the mockups' `_kit.png`, `Foundations.png` and
//! `LightFoundations.png`.

use egui::{Rect, RichText, Ui, vec2};
use egui_kittest::Harness;
use mg_uikit::Controls as _;
use mg_uikit::icons::{ButtonState, paint_icon_button};
use mg_uikit::shell::{self, DockTab, DockTabs, FoldStrip, PanelHeader, Side};
use mg_uikit::status::{self, Hint, Hints, StatusItem, StatusJob, StatusLog, StatusSwitch};
use mg_uikit::{Badge, BadgeKind, Icon, IconButton, KeyCap, Theme, TokensExt as _, metrics};

/// The sheet's size.
const SIZE: egui::Vec2 = egui::vec2(1600.0, 1400.0);
/// A cell's width.
const CELL: f32 = 376.0;

/// The values the controls show and change.
struct State {
    name: String,
    tag: String,
    note: String,
    search: String,
    scale: f32,
    angle: f32,
    light: f32,
    plot: bool,
    fast: bool,
    on: bool,
    day: bool,
    race: &'static str,
}

impl Default for State {
    fn default() -> Self {
        State {
            name: "Aribeth".into(),
            tag: "M1Q1FAribeth".into(),
            note: "m1q1faribeth".into(),
            search: String::new(),
            scale: 1.25,
            angle: 270.0,
            light: 35.0,
            plot: false,
            fast: true,
            on: true,
            day: true,
            race: "Elf",
        }
    }
}

/// A cell of the sheet: a caption in small weak text, then its controls.
fn cell(ui: &mut Ui, caption: &str, add: impl FnOnce(&mut Ui)) {
    ui.allocate_ui_with_layout(vec2(CELL, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
        ui.set_width(CELL);
        ui.label(RichText::new(caption).small().weak());
        add(ui);
    });
}

/// A cell two wide.
fn cell_wide(ui: &mut Ui, caption: &str, add: impl FnOnce(&mut Ui)) {
    let width = 2.0 * CELL + 16.0;
    ui.allocate_ui_with_layout(vec2(width, 0.0), egui::Layout::top_down(egui::Align::Min), |ui| {
        ui.set_width(width);
        ui.label(RichText::new(caption).small().weak());
        add(ui);
    });
}

/// `add` drawn as if hovered (egui's hovered visuals at rest).
fn as_hovered<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.scope(|ui| {
        let w = &mut ui.visuals_mut().widgets;
        w.inactive = w.hovered;
        add(ui)
    })
    .inner
}

/// A row or segment that paints no frame at rest (a selectable label, a
/// menu's button), made to, for [`as_hovered`] to show its hover.
fn hovered_row(text: &str) -> egui::Button<'_> {
    egui::Button::selectable(false, text).frame_when_inactive(true)
}

/// `add` drawn as if pressed or focused.
fn as_active<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.scope(|ui| {
        let w = &mut ui.visuals_mut().widgets;
        w.inactive = w.active;
        add(ui)
    })
    .inner
}

/// `add` drawn as if open (a combo box).
fn as_open<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.scope(|ui| {
        let w = &mut ui.visuals_mut().widgets;
        w.inactive = w.open;
        add(ui)
    })
    .inner
}

fn buttons(ui: &mut Ui, s: &mut State) {
    ui.horizontal_wrapped(|ui| {
        let _ = ui.button("Area Properties");
        as_hovered(ui, |ui| ui.button("Hovered"));
        as_active(ui, |ui| ui.button("Pressed"));
        let _ = ui.add(egui::Button::new("OK").selected(true));
        let _ = ui.add(egui::Button::new("Flat").frame_when_inactive(false));
        ui.add_enabled(false, egui::Button::new("Inventory…"));
    });
    ui.horizontal_wrapped(|ui| {
        let _ = mg_uikit::icon_text_button(ui, Icon::Compile, "Compile");
        let _ = mg_uikit::icon_text_toggle(ui, &mut s.on, Icon::Fog, "Fog");
        let mut off = false;
        let _ = mg_uikit::icon_text_toggle(ui, &mut off, Icon::Sun, "Day");
        ui.add_enabled_ui(false, |ui| mg_uikit::icon_text_button(ui, Icon::Save, "Save"));
    });
    ui.horizontal_wrapped(|ui| {
        let _ = ui.add(egui::Button::selectable(s.day, "Day"));
        let _ = ui.add(egui::Button::selectable(!s.day, "Night"));
        ui.separator();
        let _ = ui.selectable_label(true, "Scatter");
        let _ = ui.selectable_label(false, "Row");
        as_hovered(ui, |ui| ui.add(hovered_row("Fill")));
        let _ = ui.small_button("Small");
    });
}

fn combos(ui: &mut Ui, s: &mut State) {
    ui.horizontal_wrapped(|ui| {
        egui::ComboBox::from_id_salt("category")
            .icon(mg_uikit::combo_icon)
            .selected_text("Category")
            .width(110.0)
            .show_ui(ui, |ui| {
                let _ = ui.selectable_label(true, "Category");
            });
        as_hovered(ui, |ui| {
            egui::ComboBox::from_id_salt("hovered")
                .icon(mg_uikit::combo_icon)
                .selected_text("Hovered")
                .width(90.0)
                .show_ui(ui, |_| {})
        });
        as_open(ui, |ui| {
            egui::ComboBox::from_id_salt("open")
                .icon(mg_uikit::combo_icon)
                .selected_text("Female")
                .width(90.0)
                .show_ui(ui, |_| {})
        });
        ui.add_enabled_ui(false, |ui| {
            egui::ComboBox::from_id_salt("off")
                .icon(mg_uikit::combo_icon)
                .selected_text("Off")
                .width(60.0)
                .show_ui(ui, |_| {})
        });
    });
    // An open combo box's list (one popup is open at a time: drawn in place).
    ui.horizontal_wrapped(|ui| {
        ui.label("Race");
        egui::ComboBox::from_id_salt("race")
            .icon(mg_uikit::combo_icon)
            .selected_text(s.race)
            .width(120.0)
            .show_ui(ui, |ui| {
                for r in ["Dwarf", "Elf", "Human"] {
                    ui.selectable_value(&mut s.race, r, r);
                }
            });
    });
    egui::Frame::popup(ui.style()).show(ui, |ui| {
        ui.set_width(150.0);
        for (i, r) in ["Dwarf", "Elf", "Half-Elf", "Human"].into_iter().enumerate() {
            if i == 2 {
                as_hovered(ui, |ui| ui.add(hovered_row(r)));
            } else {
                let _ = ui.selectable_label(r == s.race, r);
            }
        }
    });
}

fn fields(ui: &mut Ui, s: &mut State) {
    let width = CELL - 20.0;
    let focused = ui.add(mg_uikit::text_field(&mut s.tag).desired_width(width));
    if ui.ctx().memory(|m| m.focused().is_none()) {
        focused.request_focus();
    }
    let _ = ui.add(mg_uikit::text_field(&mut s.name).desired_width(width));
    as_hovered(ui, |ui| ui.add(mg_uikit::text_field(&mut s.note).desired_width(width)));
    let _ = ui.add(mg_uikit::text_field(&mut s.search).hint_text("Search name, tag, resref"));
    let mut read_only = "m1q1faribeth".to_string();
    let _ = ui.add(mg_uikit::text_field(&mut read_only).interactive(false));
    ui.add_enabled_ui(false, |ui| {
        let mut off = "None".to_string();
        ui.add(mg_uikit::text_field(&mut off))
    });
    ui.horizontal_wrapped(|ui| {
        let _ = ui.add(egui::DragValue::new(&mut s.scale).speed(0.05).suffix(" m"));
        let _ = ui.add(egui::DragValue::new(&mut s.angle).suffix("°"));
        as_hovered(ui, |ui| ui.add(egui::DragValue::new(&mut s.angle)));
        ui.colored_label(ui.visuals().error_fg_color, "Not a number");
    });
}

fn checks(ui: &mut Ui, s: &mut State) {
    ui.horizontal_wrapped(|ui| {
        let _ = ui.check_box(&mut s.plot, "Plot");
        let mut on = true;
        let _ = ui.check_box(&mut on, "Static");
        as_hovered(ui, |ui| ui.check_box(&mut on, "Hovered"));
        ui.add_enabled_ui(false, |ui| {
            let mut off = false;
            let _ = ui.check_box(&mut off, "Useable");
            let mut on = true;
            ui.check_box(&mut on, "Locked")
        });
    });
    ui.horizontal_wrapped(|ui| {
        let _ = ui.radio_choice(&mut s.fast, true, "Fast");
        let _ = ui.radio_choice(&mut s.fast, false, "High");
        ui.add_enabled_ui(false, |ui| ui.radio_button(false, "Off"));
    });
    ui.add(egui::Slider::new(&mut s.light, 0.0..=100.0).suffix(" %"));
    as_hovered(ui, |ui| ui.add(egui::Slider::new(&mut s.light, 0.0..=100.0).text("Hovered")));
    ui.add(egui::ProgressBar::new(0.4).desired_width(200.0).desired_height(6.0));
    ui.separator();
    ui.horizontal_wrapped(|ui| {
        ui.hyperlink_to("A link", "https://lucide.dev");
        ui.weak("weak: 113 objects");
        ui.label(RichText::new("small text").small());
    });
    ui.horizontal_wrapped(|ui| {
        ui.colored_label(ui.visuals().warn_fg_color, "A warning");
        ui.colored_label(ui.visuals().error_fg_color, "An error");
        ui.code("m1q3_formosa_9");
    });
}

fn tree(ui: &mut Ui) {
    mg_uikit::list_rows(ui);
    egui::CollapsingHeader::new("Areas").icon(mg_uikit::collapsing_icon).default_open(true).show(
        ui,
        |ui| {
            egui::CollapsingHeader::new("map_m1q1a")
                .icon(mg_uikit::collapsing_icon)
                .default_open(true)
                .show(ui, |ui| {
                    egui::CollapsingHeader::new("Creatures")
                        .icon(mg_uikit::collapsing_icon)
                        .default_open(true)
                        .show(ui, |ui| {
                            let _ = ui.selectable_label(true, "Halion  TALK_M1Q1NewGuard");
                            let _ = ui.selectable_label(false, "Aribeth");
                            as_hovered(ui, |ui| ui.add(hovered_row("Hovered row")));
                        });
                    egui::CollapsingHeader::new("Waypoints")
                        .icon(mg_uikit::collapsing_icon)
                        .show(ui, |_| {});
                });
        },
    );
}

fn table(ui: &mut Ui) {
    egui::Grid::new("table").striped(true).num_columns(4).spacing(vec2(12.0, 4.0)).show(ui, |ui| {
        let weak = |ui: &mut Ui, t: &str| ui.label(RichText::new(t).color(ui.tokens().text_2));
        for h in ["", "Label", "StrRef", "ModelName"] {
            weak(ui, h);
        }
        ui.end_row();
        for (i, (label, strref, model)) in [
            ("PlagueCorpse", "5753", "PLC_P01"),
            ("PlaguePyre", "5763", "PLC_P02"),
            ("Barrel", "5691", "PLC_A01"),
        ]
        .into_iter()
        .enumerate()
        {
            ui.weak(format!("{}", 119 + i));
            let _ = ui.selectable_label(i == 1, label);
            ui.label(strref);
            ui.monospace(model);
            ui.end_row();
        }
    });
}

fn form(ui: &mut Ui, s: &mut State) {
    mg_uikit::section_heading(ui, "Basic");
    egui::Grid::new("form")
        .num_columns(2)
        .spacing(vec2(metrics::FORM_COLUMN_GAP, metrics::FORM_ROW_GAP))
        .min_col_width(metrics::LABEL_WIDTH)
        .show(ui, |ui| {
            mg_uikit::field_label(ui, "First Name");
            let _ = ui.add(mg_uikit::text_field(&mut s.name).desired_width(200.0));
            ui.end_row();
            mg_uikit::field_label(ui, "Race");
            egui::ComboBox::from_id_salt("form-race")
                .icon(mg_uikit::combo_icon)
                .selected_text(s.race)
                .width(200.0)
                .show_ui(ui, |_| {});
            ui.end_row();
            mg_uikit::field_label(ui, "Challenge Rating");
            ui.label(RichText::new("5").color(ui.tokens().text_2));
            ui.end_row();
        });
    ui.weak("Not useable: only scripts reach it");
    mg_uikit::table_heading(ui, "A table heading");
}

/// A menu's popover as the kit's menus have it (drawn in place): no
/// padding at its sides, 4 above and below.
fn menu_frame(ui: &Ui) -> egui::Frame {
    egui::Frame::menu(ui.style()).inner_margin(mg_uikit::menu::MENU_MARGIN)
}

/// A menu's parts beyond its rows, drawn in place: a heading, rows, a
/// note, check boxes (one hovered) and radios as rows, a separator; and a
/// menu that holds a form.
fn menu_parts(ui: &mut Ui, s: &mut State) {
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        menu_frame(ui).show(ui, |ui| {
            ui.set_width(220.0);
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                mg_uikit::menu_rows(ui);
                mg_uikit::menu_heading(ui, "On this area's tiles");
                let _ = ui.button("Grass (3)");
                let _ = ui.button("Stone (4)");
                mg_uikit::menu_separator(ui);
                mg_uikit::menu_heading(
                    ui,
                    "Not on this area's tiles (nothing grows until a tile has it)",
                );
                let _ = ui.button("Dirt (1)");
                mg_uikit::menu_separator(ui);
                mg_uikit::menu_note(ui, "None saved yet");
                mg_uikit::menu_check(ui, &mut s.on, "Tiles' Lights Too");
                as_hovered(ui, |ui| mg_uikit::menu_check(ui, &mut s.on, "Hovered, on"));
                let mut off = false;
                mg_uikit::menu_check(ui, &mut off, "Preview on screen");
                mg_uikit::menu_radio(ui, true, "1280 × 720");
                as_hovered(ui, |ui| mg_uikit::menu_radio(ui, false, "1920 × 1080"));
                ui.add_enabled_ui(false, |ui| mg_uikit::menu_check(ui, &mut off, "Disabled"));
            });
        });
        menu_frame(ui).show(ui, |ui| {
            ui.set_width(220.0);
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                mg_uikit::menu_rows(ui);
                mg_uikit::menu_form(ui, |ui| {
                    mg_uikit::field_label(ui, "Name");
                    ui.add(mg_uikit::text_field(&mut s.note));
                    ui.horizontal(|ui| {
                        let _ = ui.button("Rename");
                        let _ = ui.button("Cancel");
                    });
                    ui.weak("A form in a menu keeps its gaps.");
                });
            });
        });
        // The same under a window's larger text (Options › Conversation
        // Editor at 20: the context's style is the window's while it is
        // drawn, as a popup opened from it starts from).
        let usual = ui.ctx().global_style();
        let mut large = (*usual).clone();
        let by = 20.0 / metrics::BODY_SIZE;
        for font in large.text_styles.values_mut() {
            font.size *= by;
        }
        large.spacing.interact_size.y *= by;
        ui.ctx().set_global_style(large.clone());
        ui.scope(|ui| {
            ui.set_style(large);
            menu_frame(ui).show(ui, |ui| {
                ui.set_width(300.0);
                ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                    mg_uikit::menu_rows(ui);
                    mg_uikit::menu_form(ui, |ui| {
                        mg_uikit::field_label(ui, "Name");
                        ui.add(mg_uikit::text_field(&mut s.tag));
                        ui.horizontal(|ui| {
                            let _ = ui.button("Rename");
                            let _ = ui.button("Copy");
                        });
                        ui.weak("Pass the player who should see it.");
                    });
                });
            });
        });
        ui.ctx().set_global_style(usual);
    });
}

fn popups(ui: &mut Ui) {
    // A menu, open (drawn in place: one popup is open at a time).
    ui.vertical(|ui| {
        menu_frame(ui).show(ui, |ui| {
            ui.set_width(200.0);
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                mg_uikit::menu_rows(ui);
                let undo = egui::Button::new("Undo").frame_when_inactive(false);
                let _ = mg_uikit::menu_row_with_keys(ui, undo, "Ctrl+Z");
                // (Its keys in `on-fill` on the lit row's fill.)
                as_hovered(ui, |ui| {
                    let redo = egui::Button::new("Redo").frame_when_inactive(true);
                    mg_uikit::menu_row_with_keys(ui, redo, "Ctrl+Shift+Z")
                });
                mg_uikit::menu_separator(ui);
                let _ = ui.add(egui::Button::new("Module Properties").frame_when_inactive(false));
                ui.add_enabled(false, egui::Button::new("Resize Area…").frame_when_inactive(false));
            });
        });
        // A tooltip (its own fill, which the theme's pass gives a tooltip's
        // layer: here, drawn in place, the token itself).
        let tip = egui::Frame::popup(ui.style()).fill(ui.tokens().tooltip_bg);
        tip.show(ui, |ui| {
            ui.set_max_width(260.0);
            ui.label("Show the game's shadows. Hold or right-click to choose which.");
        });
    });
}

fn scroll(ui: &mut Ui) {
    egui::ScrollArea::vertical().id_salt("scroll").max_height(90.0).show(ui, |ui| {
        for i in 0..20 {
            ui.label(format!("m1q3_formosa_{i}"));
        }
    });
}

fn icon_widgets(ui: &mut Ui) {
    let t = ui.tokens();
    ui.horizontal_wrapped(|ui| {
        mg_uikit::icon(ui, Icon::Creature);
        let _ = mg_uikit::icon_button(ui, Icon::Undo, "Undo");
        for (state, on) in [
            (ButtonState::Hovered, false),
            (ButtonState::Pressed, false),
            (ButtonState::Rest, true),
            (ButtonState::Hovered, true),
        ] {
            let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(22.0), egui::Sense::hover());
            paint_icon_button(ui.painter(), &t, rect, Icon::Grid, state, on);
        }
        ui.add_enabled_ui(false, |ui| {
            let _ = mg_uikit::icon_button(ui, Icon::Cut, "Cut");
            let mut on = true;
            mg_uikit::icon_toggle(ui, &mut on, Icon::Eye, "Shown")
        });
        let mut on = false;
        let _ = mg_uikit::icon_toggle(ui, &mut on, Icon::Magnet, "Snap");
    });
    // Sizes: 12, 16, 20, 32.
    ui.horizontal_wrapped(|ui| {
        for side in [12.0, 16.0, 20.0, 32.0] {
            let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(side), egui::Sense::hover());
            Icon::Settings.paint(ui.painter(), rect, t.text);
        }
        for (c, icon) in [(t.accent, Icon::Info), (t.error, Icon::Error), (t.warn, Icon::Warning)] {
            let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(16.0), egui::Sense::hover());
            icon.paint(ui.painter(), rect, c);
        }
        let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(16.0), egui::Sense::hover());
        Icon::Check.paint(ui.painter(), rect, t.ok);
    });
}

/// Key caps, badges of every kind (and a tag), the modified dot (the
/// mock's "Key Caps and Badges" cell).
fn marks(ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        for key in ["Ctrl", "Right Drag", "F9"] {
            ui.add(KeyCap::new(key));
        }
    });
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        for (text, kind) in [
            ("12", BadgeKind::Plain),
            ("3", BadgeKind::Accent),
            ("2", BadgeKind::Error),
            ("5", BadgeKind::Warn),
            ("0", BadgeKind::Ok),
        ] {
            ui.add(Badge::new(text).kind(kind));
        }
        ui.add(Badge::new("NEW").kind(BadgeKind::Info).tag());
        mg_uikit::modified_dot(ui);
        ui.label(RichText::new("modified").color(ui.tokens().text_2));
    });
}

/// Document tabs (active and modified, hovered, plain) painted with the
/// kit's parts on a 28 px bar, and a dock's tab bar with badges and tools
/// (the mock's "Tabs" cell), and a panel's header.
fn tabs(ui: &mut Ui) {
    let t = ui.tokens();
    let (bar, _) = ui.allocate_exact_size(vec2(CELL, metrics::DOCUMENT_TABS), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(bar, 0.0, t.bar);
    p.hline(bar.x_range(), bar.bottom() - 0.5, egui::Stroke::new(1.0, t.line));
    let mut x = bar.left();
    let docs = [
        (Icon::Area, "map_m1q1a", true, true, ButtonState::Rest),
        (Icon::Creature, "m1q1faribeth.utc", false, false, ButtonState::Rest),
        (Icon::Module, "Module", false, false, ButtonState::Rest),
    ];
    for (i, (icon, title, active, modified, close)) in docs.into_iter().enumerate() {
        let font = egui::FontId::proportional(metrics::BODY_SIZE);
        let color = if active || i == 1 { t.text } else { t.text_2 };
        let galley = p.layout_no_wrap(title.to_string(), font, color);
        let width = shell::doc_tab_width(galley.size().x, modified) + 1.0;
        let tab = Rect::from_min_size(egui::pos2(x, bar.top()), vec2(width, bar.height()));
        let body = tab.with_max_x(tab.right() - 1.0);
        let parts = shell::doc_tab_parts(body, galley.size().x, modified);
        if active {
            p.rect_filled(body, 0.0, t.panel);
            let line = Rect::from_min_size(body.min, vec2(body.width(), metrics::ACTIVE_TAB_LINE));
            p.rect_filled(line, 0.0, t.accent);
        } else if i == 1 {
            p.rect_filled(body.with_max_y(body.bottom() - 1.0), 0.0, t.tab_hover);
        }
        p.vline(tab.right() - 0.5, tab.top()..=tab.bottom() - 1.0, egui::Stroke::new(1.0, t.line));
        shell::paint_doc_tab_icon(p, &t, parts.icon, icon, active);
        let y = (parts.title.center().y - galley.size().y / 2.0).round();
        p.galley(egui::pos2(parts.title.left(), y), galley, color);
        if let Some(dot) = parts.dot {
            mg_uikit::paint_modified_dot(p, &t, dot.center());
        }
        if active || i == 1 {
            shell::paint_doc_tab_close(p, &t, parts.close, close);
        }
        x = tab.right();
    }
    ui.add_space(4.0);
    let mut on = 0;
    DockTabs::new("Bottom Dock")
        .tab("Log")
        .with(DockTab::new("Problems").badge(Badge::count(3).kind(BadgeKind::Error)))
        .with(DockTab::new("Search Results").badge(Badge::count(12)))
        .drawn_hovered(2)
        .show(ui, &mut on, |ui| {
            ui.add(IconButton::new(Icon::Delete, "Clear the log").icon_only(true));
            ui.add(IconButton::new(Icon::Close, "Close").icon_only(true));
        });
    ui.add_space(4.0);
    PanelHeader::new("Outliner").detail("map_m1q1a").show(ui, |ui| {
        ui.add(IconButton::new(Icon::CollapseAll, "Collapse All").icon_only(true));
    });
}

/// The Status Bar's parts (the mock's "Status Bar" cell, and the rows of
/// Main.png and Narrow.png): switches on, off with a badge and hovered;
/// hints dropping whole where the room ends; a job; the right-hand items;
/// then the narrow window's row, the switches icons alone and the game
/// folder gone.
fn status_bar(ui: &mut Ui) {
    let t = ui.tokens();
    let hints = [
        Hint::keys(["Drag"], "Move"),
        Hint::keys(["Shift"], "Handles"),
        Hint::keys(["Alt", "Drag"], "Raise"),
        Hint::either(["Q", "E"], "Turn by a step"),
        Hint::keys(["G"], "Drop to ground"),
    ];
    for narrow in [false, true] {
        let width = 2.0 * CELL + 16.0;
        let (row, _) =
            ui.allocate_exact_size(vec2(width, metrics::STATUS_BAR), egui::Sense::hover());
        ui.painter().rect_filled(row, 0.0, t.bar);
        let builder = egui::UiBuilder::new().max_rect(row.shrink2(vec2(status::PADDING, 0.0)));
        ui.scope_builder(builder, |ui| {
            status::bar(
                ui,
                |ui| {
                    ui.add(StatusSwitch::new(Icon::Log, "Log", true).icon_only(narrow));
                    let problems = Badge::count(3).kind(BadgeKind::Error);
                    let switch =
                        StatusSwitch::new(Icon::Problems, "Problems", false).badge(problems);
                    ui.add(switch.icon_only(narrow));
                    if narrow {
                        let hovered = StatusSwitch::new(Icon::Find, "Search Results", false);
                        ui.add(hovered.icon_only(true).drawn_as(ButtonState::Hovered));
                    }
                    status::separator(ui);
                },
                |ui, room| {
                    if narrow {
                        Hints::new(&hints).show(ui, room)
                    } else {
                        // (The log's last line, the log closed, cut short
                        // to its room: the hints have the rest.)
                        let line = StatusLog::new("m1q3_formosa_9.nss: compiled").max_width(110.0);
                        let used = line.width(ui) + status::GAP;
                        ui.add(line);
                        Hints::new(&hints).show(ui, room - used)
                    }
                },
                |ui| {
                    if narrow {
                        // (A job whose progress is not known: its mark.)
                        ui.add(StatusJob::unknown("Build Module"));
                        status::separator(ui);
                        ui.add(StatusItem::new("192 objects"));
                    } else {
                        ui.add(StatusJob::new("Verify Module", Some(0.4)));
                    }
                    status::separator(ui);
                    ui.add(StatusItem::new("Chapter1 (modified)").modified(true));
                    if !narrow {
                        status::separator(ui);
                        ui.add(StatusItem::new(".../Neverwinter Nights").weak());
                    }
                },
            );
        });
        ui.add_space(4.0);
    }
}

/// The Top Bar's row (menus' titles, one open; quick actions; Quick Open
/// centered; the build actions), an open menu's 24 px rows (one hovered,
/// a submenu's, a separator, a disabled one) and a folded dock's strip.
fn top_bar(ui: &mut Ui) {
    let t = ui.tokens();
    let (row, _) =
        ui.allocate_exact_size(vec2(2.0 * CELL + 16.0, metrics::TOP_BAR), egui::Sense::hover());
    ui.painter().rect_filled(row, 0.0, t.bar);
    let builder = egui::UiBuilder::new().max_rect(row.shrink2(vec2(shell::TOP_BAR_PADDING, 0.0)));
    ui.scope_builder(builder, |ui| {
        shell::top_bar(
            ui,
            shell::QUICK_OPEN_MAX,
            |ui| {
                ui.scope(|ui| {
                    let mut style = (**ui.style()).clone();
                    mg_uikit::menu_bar_style(&mut style);
                    ui.set_style(style);
                    let _ = ui.button("File");
                    as_hovered(ui, |ui| {
                        ui.add(egui::Button::new("Edit").frame_when_inactive(true))
                    });
                    as_open(ui, |ui| ui.add(egui::Button::new("View").frame_when_inactive(true)));
                    let _ = ui.button("Help");
                });
                shell::vsep(ui);
                for (icon, name) in [(Icon::New, "New"), (Icon::Open, "Open"), (Icon::Save, "Save")]
                {
                    ui.add(IconButton::new(icon, name).icon_only(true));
                }
            },
            |ui, width| {
                let mut find = String::new();
                ui.add(
                    mg_uikit::text_field(&mut find)
                        .hint_text("Go to resource or command")
                        .desired_width(width),
                );
            },
            |ui| {
                ui.add(IconButton::new(Icon::Compile, "Compile"));
                ui.add(IconButton::new(Icon::Verify, "Verify"));
                ui.add(IconButton::new(Icon::Test, "Test Module"));
            },
        );
    });
    ui.add_space(4.0);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        // An open menu (drawn in place).
        menu_frame(ui).show(ui, |ui| {
            ui.set_width(240.0);
            ui.with_layout(egui::Layout::top_down_justified(egui::Align::Min), |ui| {
                mg_uikit::menu_rows(ui);
                let tree = egui::Button::new("Module Tree");
                let _ = mg_uikit::menu_row_with_keys(ui, tree, "Ctrl+Alt+1");
                // (A lit row's keys in `on-fill`: egui's are the weak
                // text's, which does not read on `accent-fill`.)
                as_hovered(ui, |ui| {
                    let log = egui::Button::new("Log").frame_when_inactive(true);
                    mg_uikit::menu_row_with_keys(ui, log, "Ctrl+Alt+3")
                });
                let _ = ui.add(egui::Button::new("Copy Log"));
                mg_uikit::menu_separator(ui);
                let sub = IconButton::new(Icon::ChevronRight, "Position")
                    .label(mg_uikit::Label::WordsOnly);
                let _ = mg_uikit::submenu_button(ui, sub);
                ui.add_enabled(false, egui::Button::new("Go to Start Location"));
            });
        });
        ui.allocate_ui(vec2(shell::STRIP, 160.0), |ui| {
            ui.set_height(160.0);
            FoldStrip::new("Left Dock", Side::Left)
                .panel(Icon::Module, "Module", "Module")
                .panel(Icon::Folder, "Resources", "Resources")
                .panel(Icon::Outliner, "Outliner", "Outliner")
                .show(ui);
        });
        ui.vertical(|ui| {
            ui.label(RichText::new("fold button").small().weak());
            shell::fold_button(ui, Side::Left, "Left Dock");
        });
    });
}

/// The whole set, names beside.
fn icon_grid(ui: &mut Ui) {
    let t = ui.tokens();
    let columns = 12;
    // (The window's width less the harness's margins: the cells above
    // may have pushed the sheet's own wider.)
    let width = ((ui.ctx().content_rect().width() - 16.0) / columns as f32).floor();
    for row in Icon::ALL.chunks(columns) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for icon in row {
                let (rect, _) = ui.allocate_exact_size(vec2(width, 22.0), egui::Sense::hover());
                let square =
                    Rect::from_min_size(rect.min + vec2(0.0, 3.0), egui::Vec2::splat(16.0));
                icon.paint(ui.painter(), square, t.text);
                ui.painter().text(
                    rect.min + vec2(24.0, 11.0),
                    egui::Align2::LEFT_CENTER,
                    icon.name(),
                    egui::FontId::proportional(11.0),
                    t.text_2,
                );
            }
        });
    }
}

/// The sheet under `theme`.
fn sheet(ui: &mut Ui, theme: Theme, s: &mut State) {
    mg_uikit::apply(ui.ctx(), theme);
    let t = ui.tokens();
    // (The harness leaves a margin around its panel unpainted.)
    let all = ui.ctx().content_rect();
    let back = egui::LayerId::background();
    ui.ctx().layer_painter(back).rect_filled(all, 0.0, ui.visuals().panel_fill);
    ui.spacing_mut().item_spacing.y = 6.0;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        cell(
            ui,
            "buttons: normal, hovered, pressed, selected, flat, disabled; icon and words; segments",
            |ui| buttons(ui, s),
        );
        cell(ui, "combo boxes: plain, hovered, open, disabled; an open list", |ui| combos(ui, s));
        cell(
            ui,
            "text fields: focused, plain, hovered, hint, read-only, disabled; numbers",
            |ui| fields(ui, s),
        );
        cell(ui, "checkboxes, radios, sliders, progress, separator, text colors", |ui| {
            checks(ui, s)
        });
    });
    ui.add_space(8.0);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        cell(ui, "tree: collapsing headers, a selected and a hovered row", tree);
        cell(ui, "table: a striped grid, a selected row", table);
        cell(ui, "form: section heading, field labels, weak note", |ui| form(ui, s));
        cell(ui, "a menu and a tooltip (drawn in place); scroll bars", |ui| {
            popups(ui);
            scroll(ui);
        });
    });
    ui.add_space(8.0);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 16.0;
        cell(ui, "key caps; badges: plain, accent, error, warn, ok, tag; modified dot", marks);
        cell(
            ui,
            "document tabs (kit parts); dock tabs: on, badge, hovered, tools; panel header",
            tabs,
        );
        cell_wide(
            ui,
            "status bar: switches on, badge; hints dropping whole; job; items; narrow: icons, hovered",
            status_bar,
        );
    });
    ui.add_space(8.0);
    cell_wide(
        ui,
        "top bar: menus (hovered, open), quick actions, Quick Open, build actions; a menu's 24 px rows; folded dock strip",
        top_bar,
    );
    ui.add_space(8.0);
    cell_wide(
        ui,
        "a menu's parts: headings (one wrapped), separators, a note, check and radio rows (hovered, disabled); a menu holding a form, and one under a 20 px text",
        |ui| menu_parts(ui, s),
    );
    ui.add_space(8.0);
    cell(
        ui,
        "icon widgets: icon, button (rest, hovered, pressed, on, on hovered, disabled), toggle; sizes 12 16 20 32; state colors",
        icon_widgets,
    );
    ui.add_space(8.0);
    ui.label(RichText::new(format!("icon set ({} icons, 16 px)", Icon::ALL.len())).small().weak());
    icon_grid(ui);
    // A window over it all.
    egui::Window::new("Window").default_pos(egui::pos2(1100.0, 700.0)).default_width(280.0).show(
        ui.ctx(),
        |ui| {
            ui.label("A floating window: panel fill, a line-soft border, no shadow.");
            ui.horizontal(|ui| {
                let _ = ui.button("Cancel");
                let _ = ui.add(egui::Button::new("OK").fill(t.accent_fill));
            });
        },
    );
}

fn harness(theme: Theme, gpu: bool) -> Harness<'static> {
    let mut state = State::default();
    let builder = Harness::builder().with_size(SIZE);
    let builder = if gpu { builder.wgpu() } else { builder };
    builder.build_ui(move |ui| sheet(ui, theme, &mut state))
}

/// The sheet lays out and paints in both themes (no GPU).
#[test]
fn the_kit_sheet_draws() {
    for theme in Theme::ALL {
        let mut h = harness(theme, false);
        h.run_steps(4);
        assert_eq!(h.ctx.theme(), theme.egui());
        assert!(!h.output().shapes.is_empty());
    }
}

/// The sheet's pictures, dark and light, in `target/test-output/uikit/`.
/// Run by hand: `cargo test -p mg-uikit --test kit -- --ignored`.
#[test]
#[ignore]
fn kit_sheet_pictures() {
    mg_testkit::gpu::hold();
    let dir = mg_testkit::output_dir("uikit");
    for theme in Theme::ALL {
        let mut h = harness(theme, true);
        h.run_steps(4);
        let image = h.render().expect("render");
        let name = format!("uikit-{}.png", theme.name().to_lowercase());
        image.save(dir.join(&name)).unwrap();
        eprintln!("wrote {}", dir.join(name).display());
    }
}
