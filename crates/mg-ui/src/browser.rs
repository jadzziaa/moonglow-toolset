//! The resource browser: every resource in the load order (game, haks,
//! module) with the layer it comes from, and a read-only view of any of
//! them that can be copied into the module or saved to a file.

use std::path::PathBuf;

use egui::Ui;
use mg_core::ResType;
use mg_edit::{Command, Edit};
use mg_gff::{Gff, Struct, Value};
use mg_resman::ResKey;

use crate::text::decode;
use crate::{Action, FileKind, Moonglow, Tab};

/// The browser's index and filters.
#[derive(Debug, Default)]
pub struct Browser {
    /// Each resource and the label of its layer; rebuilt when stale.
    entries: Vec<(ResKey, String)>,
    types: Vec<ResType>,
    pub restype: Option<ResType>,
    pub filter: String,
    /// Indices into `entries` matching the filters, for `query`.
    shown: Vec<usize>,
    query: Option<(Option<ResType>, String)>,
    /// The load order changed since the index was built.
    pub(crate) stale: bool,
}

impl Browser {
    pub fn new() -> Browser {
        Browser { stale: true, ..Default::default() }
    }

    fn refresh(&mut self, app_resman: Option<&mg_resman::ResMan>) {
        if !self.stale {
            return;
        }
        self.stale = false;
        self.query = None;
        self.entries = match app_resman {
            Some(rm) => {
                rm.entries().into_iter().map(|(k, i)| (k, rm.layers()[i].label.clone())).collect()
            }
            None => Vec::new(),
        };
        let mut types: Vec<ResType> = self.entries.iter().map(|(k, _)| k.restype).collect();
        types.sort_by_key(|t| t.extension().unwrap_or_default());
        types.dedup();
        self.types = types;
    }

    fn filtered(&mut self) -> &[usize] {
        let query = (self.restype, self.filter.to_ascii_lowercase());
        if self.query.as_ref() != Some(&query) {
            self.shown = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, (k, _))| {
                    query.0.is_none_or(|t| k.restype == t)
                        && (query.1.is_empty() || k.resref.to_string().contains(&query.1))
                })
                .map(|(i, _)| i)
                .collect();
            self.query = Some(query);
        }
        &self.shown
    }
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui) {
    let Moonglow { browser, game, ws, actions, .. } = app;
    browser.refresh(game.as_ref().map(|g| &g.resman));
    if game.is_none() {
        ui.label("The browser needs the game data (Tools > Options).");
        return;
    }
    ui.horizontal(|ui| {
        let current = browser.restype.and_then(|t| t.extension()).unwrap_or("all types");
        egui::ComboBox::from_id_salt("browser-type").selected_text(current).show_ui(ui, |ui| {
            ui.selectable_value(&mut browser.restype, None, "all types");
            for t in &browser.types {
                ui.selectable_value(&mut browser.restype, Some(*t), t.extension().unwrap_or("?"));
            }
        });
        ui.label("Name");
        ui.text_edit_singleline(&mut browser.filter);
        if ui.button("Refresh").clicked() {
            browser.stale = true;
        }
    });
    let count = browser.filtered().len();
    ui.weak(format!("{count} of {} resources; double-click to open", browser.entries.len()));
    ui.separator();
    let row_height = ui.text_style_height(&egui::TextStyle::Body) + 4.0;
    let in_module = |k: &ResKey| ws.as_ref().is_some_and(|w| w.module.contains(k));
    let shown = browser.shown.clone();
    egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(
        ui,
        row_height,
        count,
        |ui, range| {
            for &i in &shown[range] {
                let (k, layer) = &browser.entries[i];
                ui.horizontal(|ui| {
                    let r =
                        ui.selectable_label(false, egui::RichText::new(k.to_string()).monospace());
                    ui.weak(layer);
                    let tab = || match Tab::for_resource(*k) {
                        Some(t) if in_module(k) => t,
                        _ => Tab::Resource(*k),
                    };
                    if r.double_clicked() {
                        actions.push(Action::OpenTab(tab()));
                    }
                    r.context_menu(|ui| {
                        if ui.button("Open").clicked() {
                            actions.push(Action::OpenTab(tab()));
                        }
                    });
                });
            }
        },
    );
}

/// A resource read and parsed once for its viewer.
#[derive(Debug)]
pub(crate) struct Viewed {
    data: Vec<u8>,
    origin: String,
    content: Content,
}

#[derive(Debug)]
enum Content {
    Gff(Gff),
    TwoDa(mg_2da::TwoDa),
    Script(String),
    Text(String),
    Binary(String),
    Error(String),
}

impl Viewed {
    fn load(game: &mg_rules::GameData, key: ResKey) -> Option<Viewed> {
        let rm = &game.resman;
        let data = rm.get(&key).ok()?.into_owned();
        let origin = rm.origin(&key).unwrap_or("?").to_string();
        let codepage = game.language.codepage();
        let content = if key.restype.is_gff() {
            Gff::read(&data).map_or_else(|e| Content::Error(e.to_string()), Content::Gff)
        } else if key.restype == ResType::TWODA {
            mg_2da::TwoDa::parse(&data, codepage)
                .map_or_else(|e| Content::Error(e.to_string()), Content::TwoDa)
        } else if key.restype == ResType::NSS {
            Content::Script(codepage.decode(&data).into_owned())
        } else if is_text(key.restype, &data) {
            Content::Text(codepage.decode(&data).into_owned())
        } else {
            Content::Binary(hex(&data[..data.len().min(512)]))
        };
        Some(Viewed { data, origin, content })
    }
}

/// Read-only view of a resource from the load order.
pub(crate) fn resource_ui(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    let Some(game) = &app.game else { return };
    let viewed = match app.viewed.get(&key) {
        Some(v) => v.clone(),
        None => match Viewed::load(game, key) {
            Some(v) => {
                let v = std::sync::Arc::new(v);
                app.viewed.insert(key, v.clone());
                v
            }
            None => {
                ui.label(format!("{key} is not in the load order."));
                return;
            }
        },
    };
    let mut copy = false;
    let mut save = false;
    ui.horizontal(|ui| {
        ui.strong(key.to_string());
        ui.weak(format!("from {}, {} bytes", viewed.origin, viewed.data.len()));
        let can_copy = app.ws.as_ref().is_some_and(|w| !w.module.contains(&key));
        copy = ui.add_enabled(can_copy, egui::Button::new("Copy to Module")).clicked();
        save = ui.button("Save As…").clicked();
    });
    ui.separator();
    if copy {
        app.actions.push(Action::Apply(Command::new(
            format!("Copy {key} to the module"),
            vec![Edit::SetResource { key, data: Some(viewed.data.clone()) }],
        )));
        if let Some(tab) = Tab::for_resource(key) {
            app.actions.push(Action::OpenTab(tab));
        }
    }
    if save
        && let Some(path) =
            app.dialogs.save_file(FileKind::Any, Some(&PathBuf::from(key.to_string())))
    {
        match std::fs::write(&path, &viewed.data) {
            Ok(()) => app.log.info(format!("Saved {key} to {}", path.display())),
            Err(e) => app.log.error(format!("Could not write {}: {e}", path.display())),
        }
    }
    let palette = crate::script_view::Palette::for_ui(&app.settings.script_style, ui);
    egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| match &viewed.content {
        Content::Gff(g) => gff_tree(ui, &g.root, &key.to_string()),
        Content::TwoDa(t) => two_da(ui, t),
        Content::Script(text) => {
            let job = crate::script_view::highlight(text, &palette);
            ui.label(job);
        }
        Content::Text(text) | Content::Binary(text) => {
            ui.monospace(text);
        }
        Content::Error(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e);
        }
    });
}

fn is_text(t: ResType, data: &[u8]) -> bool {
    matches!(
        t,
        ResType::NSS
            | ResType::TXT
            | ResType::INI
            | ResType::SET
            | ResType::TXI
            | ResType::MTR
            | ResType::SHD
            | ResType::LUA
    ) || (!data.is_empty()
        && data.iter().take(4096).all(|b| b.is_ascii_graphic() || b.is_ascii_whitespace()))
}

fn hex(data: &[u8]) -> String {
    data.chunks(16)
        .enumerate()
        .map(|(i, row)| {
            let bytes: Vec<String> = row.iter().map(|b| format!("{b:02x}")).collect();
            let text: String =
                row.iter().map(|&b| if b.is_ascii_graphic() { b as char } else { '.' }).collect();
            format!("{:08x}  {:<47}  {text}", i * 16, bytes.join(" "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn value_text(v: &Value) -> String {
    match v {
        Value::Byte(x) => x.to_string(),
        Value::Char(x) => x.to_string(),
        Value::Word(x) => x.to_string(),
        Value::Short(x) => x.to_string(),
        Value::Dword(x) => x.to_string(),
        Value::Int(x) => x.to_string(),
        Value::Dword64(x) => x.to_string(),
        Value::Int64(x) => x.to_string(),
        Value::Float(x) => x.to_string(),
        Value::Double(x) => x.to_string(),
        Value::String(b) | Value::ResRef(b) => format!("{:?}", decode(b)),
        Value::LocString(ls) => {
            let mut parts: Vec<String> = Vec::new();
            if !ls.strref.is_none() {
                parts.push(format!("StrRef {}", ls.strref.0));
            }
            parts.extend(ls.strings.iter().map(|(k, b)| format!("[{}] {:?}", k.0, decode(b))));
            parts.join(", ")
        }
        Value::Void(b) => format!("{} bytes", b.len()),
        Value::Struct(_) | Value::List(_) => String::new(),
    }
}

/// A GFF struct as a read-only tree.
fn gff_tree(ui: &mut Ui, s: &Struct, id: &str) {
    for (i, f) in s.fields.iter().enumerate() {
        let label = f.label.to_string_lossy();
        let child_id = format!("{id}/{i}");
        match &f.value {
            Value::Struct(child) => {
                egui::CollapsingHeader::new(format!("{label} (struct {})", child.id))
                    .id_salt(&child_id)
                    .show(ui, |ui| gff_tree(ui, child, &child_id));
            }
            Value::List(items) => {
                egui::CollapsingHeader::new(format!("{label} (list, {} items)", items.len()))
                    .id_salt(&child_id)
                    .show(ui, |ui| {
                        for (j, item) in items.iter().enumerate() {
                            let item_id = format!("{child_id}/{j}");
                            egui::CollapsingHeader::new(format!("[{j}] struct {}", item.id))
                                .id_salt(&item_id)
                                .show(ui, |ui| gff_tree(ui, item, &item_id));
                        }
                    });
            }
            v => {
                ui.horizontal(|ui| {
                    ui.label(&label);
                    ui.weak(v.field_type().json_name());
                    ui.monospace(value_text(v));
                });
            }
        }
    }
}

/// A 2DA as a table; only the rows in view are laid out.
fn two_da(ui: &mut Ui, t: &mg_2da::TwoDa) {
    let width = 110.0;
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 4.0;
    let cell = |ui: &mut Ui, text: &str| {
        ui.add_sized(
            [width, row_height],
            egui::Label::new(egui::RichText::new(text).monospace()).truncate(),
        );
    };
    ui.horizontal(|ui| {
        ui.add_sized([50.0, row_height], egui::Label::new(egui::RichText::new("").monospace()));
        for c in t.columns() {
            ui.add_sized(
                [width, row_height],
                egui::Label::new(egui::RichText::new(c).strong()).truncate(),
            );
        }
    });
    egui::ScrollArea::vertical().id_salt("2da-rows").auto_shrink([false, false]).show_rows(
        ui,
        row_height,
        t.len(),
        |ui, rows| {
            for r in rows {
                ui.horizontal(|ui| {
                    ui.add_sized(
                        [50.0, row_height],
                        egui::Label::new(egui::RichText::new(r.to_string()).weak()),
                    );
                    for c in 0..t.columns().len() {
                        cell(ui, t.cell(r, c).unwrap_or("****"));
                    }
                });
            }
        },
    );
}
