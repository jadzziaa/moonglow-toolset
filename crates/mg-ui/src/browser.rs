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
    /// 2DA views: StrRefs shown as numbers rather than their text.
    pub(crate) raw_strrefs: bool,
    /// 2DA views: only the rows a layer last changed (its index in the
    /// table's copies).
    pub rows_from: Option<usize>,
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
        crate::widgets::field_label(ui, "Name");
        ui.text_edit_singleline(&mut browser.filter);
        if ui.button("Refresh").clicked() {
            browser.stale = true;
        }
    });
    let count = browser.filtered().len();
    ui.horizontal(|ui| {
        ui.weak(format!("{count} of {} resources; double-click to open", browser.entries.len()));
        // (Narrowed by a type or a name: not the whole game by a slip.)
        let narrowed = count > 0 && count < browser.entries.len();
        if ui
            .add_enabled(narrowed, egui::Button::new(format!("Export {count} as Files…")))
            .on_hover_text(
                "The resources listed, as loose files in a folder (name.ext), as the load \
                 order has them. Choose a type or type part of a name first.",
            )
            .clicked()
        {
            let keys = browser.shown.iter().map(|&i| browser.entries[i].0).collect();
            actions.push(Action::SaveResources(keys));
        }
    });
    ui.separator();
    let row_height = ui.text_style_height(&egui::TextStyle::Body) + 4.0;
    let in_module = |k: &ResKey| ws.as_ref().is_some_and(|w| w.module.contains(k));
    let shown = &browser.shown;
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
                        Some(t @ Tab::Model(_)) => t,
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
                        if k.restype != ResType::MDL
                            && crate::model_view::previewable(k.restype)
                            && ui
                                .button("Preview")
                                .on_hover_text("See it in the 3D viewer")
                                .clicked()
                        {
                            actions.push(Action::OpenTab(Tab::Model(*k)));
                        }
                        if ui.button("Save As…").on_hover_text("As a file, as it is").clicked() {
                            actions.push(Action::SaveResource(*k));
                        }
                    });
                });
            }
        },
    );
}

/// Save As on a resource of the load order: its bytes to a file.
pub(crate) fn save_as(app: &mut Moonglow, key: ResKey) {
    let Some(game) = &app.game else { return };
    let Ok(data) = game.resman.get(&key) else {
        app.log.error(format!("{key} is not in the load order."));
        return;
    };
    let Some(path) = app.dialogs.save_file(FileKind::Any, Some(&PathBuf::from(key.to_string())))
    else {
        return;
    };
    match std::fs::write(&path, &data) {
        Ok(()) => app.log.info(format!("Saved {key} to {}", path.display())),
        Err(e) => app.log.error(format!("Could not write {}: {e}", path.display())),
    }
}

/// Export as Files: the resources listed, each as `name.ext` in a folder
/// the user chooses.
pub(crate) fn export(app: &mut Moonglow, keys: &[ResKey]) {
    let Some(game) = &app.game else { return };
    let Some(dir) = app.dialogs.pick_folder("Export as files into", app.export_dir.as_deref())
    else {
        return;
    };
    let mut written = 0;
    for key in keys {
        let Ok(data) = game.resman.get(key) else { continue };
        if let Err(e) = std::fs::write(dir.join(key.to_string()), &data) {
            app.log.error(format!("Could not write to {}: {e}", dir.display()));
            return;
        }
        written += 1;
    }
    app.log.info(format!("Exported {written} resources to {}", dir.display()));
    app.export_dir = Some(dir);
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
    TwoDa(mg_module::table_layers::Layered),
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
        let codepage = game.codepage();
        let content = if key.restype.is_gff() {
            Gff::read(&data).map_or_else(|e| Content::Error(e.to_string()), Content::Gff)
        } else if key.restype == ResType::TWODA {
            match mg_module::table_layers::layered(rm, &key, codepage) {
                Some(l) => Content::TwoDa(l),
                None => match mg_2da::TwoDa::parse(&data, codepage) {
                    Err(e) => Content::Error(e.to_string()),
                    Ok(_) => Content::Error("the copy the game reads can't be read".into()),
                },
            }
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

/// A resource's bytes as something to read: a GFF's fields, text, or the
/// first bytes in hexadecimal.
#[derive(Debug)]
pub(crate) enum Plain {
    Gff(Box<Gff>),
    Text(String),
    Error(String),
}

/// What `data`, a resource of `key`'s type, shows as.
pub(crate) fn plain(key: ResKey, data: &[u8]) -> Plain {
    if key.restype.is_gff() {
        match Gff::read(data) {
            Ok(g) => Plain::Gff(Box::new(g)),
            Err(e) => Plain::Error(e.to_string()),
        }
    } else if is_text(key.restype, data) {
        // (Its bytes as Latin-1 where they aren't UTF-8.)
        let latin = || data.iter().map(|&b| b as char).collect();
        Plain::Text(String::from_utf8(data.to_vec()).unwrap_or_else(|_| latin()))
    } else {
        Plain::Text(hex(&data[..data.len().min(512)]))
    }
}

/// Draws a [`Plain`] view, scrolling.
pub(crate) fn plain_ui(ui: &mut Ui, key: ResKey, view: &Plain) {
    egui::ScrollArea::both().id_salt(("plain", key)).auto_shrink([false, false]).show(ui, |ui| {
        match view {
            Plain::Gff(g) => gff_tree(ui, &g.root, &key.to_string()),
            Plain::Text(text) => {
                ui.monospace(text);
            }
            Plain::Error(e) => {
                ui.colored_label(ui.visuals().error_fg_color, e);
            }
        }
    });
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
    if let Content::TwoDa(l) = &viewed.content {
        two_da(app, ui, l);
        return;
    }
    let palette = crate::script_view::Palette::for_ui(&app.settings.script_style, ui);
    egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| match &viewed.content {
        Content::Gff(g) => gff_tree(ui, &g.root, &key.to_string()),
        Content::TwoDa(_) => {}
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

/// A 2DA as a table; only the rows in view are laid out. With copies of
/// it in several layers (haks over the game's), each row says which layer
/// it comes from, and the cells a layer changed are marked, with what they
/// were below. Cells of StrRef columns show their text.
fn two_da(app: &mut Moonglow, ui: &mut Ui, l: &mg_module::table_layers::Layered) {
    use mg_module::table_layers::is_strref_column;
    let Moonglow { browser, game, .. } = app;
    let t = l.table();
    let layered = l.copies.len() > 1;
    ui.horizontal(|ui| {
        ui.checkbox(&mut browser.raw_strrefs, "StrRefs as numbers").on_hover_text(
            "Name, Description and other StrRef columns show their text unless this is on",
        );
        if layered {
            let label = |i: Option<usize>| match i {
                None => "All rows".to_string(),
                Some(i) => format!("Rows from {}", l.copies[i].0),
            };
            browser.rows_from = browser.rows_from.filter(|&i| i < l.copies.len());
            egui::ComboBox::from_id_salt("2da-rows-from")
                .selected_text(label(browser.rows_from))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut browser.rows_from, None, label(None));
                    for i in 0..l.copies.len() {
                        ui.selectable_value(&mut browser.rows_from, Some(i), label(Some(i)));
                    }
                });
        }
    });
    if layered {
        let parts: Vec<String> = l
            .summary()
            .iter()
            .rev()
            .map(|(label, added, changed)| match (added, changed) {
                (a, 0) => format!("{label}: {a} rows"),
                (0, c) => format!("{label}: changes {c}"),
                (a, c) => format!("{label}: adds {a}, changes {c}"),
            })
            .collect();
        ui.weak(format!("{} copies, lowest first. {}", l.copies.len(), parts.join("; ")));
    }
    let rows: Vec<usize> = match browser.rows_from.filter(|_| layered) {
        Some(o) => (0..t.len()).filter(|&r| l.row_origin(r) == o).collect(),
        None => (0..t.len()).collect(),
    };
    let strrefs: Vec<bool> =
        t.columns().iter().map(|c| !browser.raw_strrefs && is_strref_column(c)).collect();
    let game = game.as_ref();
    let bottom = l.copies.len() - 1;
    let changed = ui.visuals().warn_fg_color;
    let width = 110.0;
    let row_height = ui.text_style_height(&egui::TextStyle::Monospace) + 4.0;
    let fixed = |ui: &mut Ui, w: f32, text: egui::RichText| {
        ui.add_sized([w, row_height], egui::Label::new(text).truncate())
    };
    egui::ScrollArea::both().id_salt("2da").auto_shrink([false, false]).show(ui, |ui| {
        ui.horizontal(|ui| {
            fixed(ui, 50.0, egui::RichText::new(""));
            if layered {
                fixed(ui, width, egui::RichText::new("From").strong());
            }
            for c in t.columns() {
                fixed(ui, width, egui::RichText::new(c).strong());
            }
        });
        egui::ScrollArea::vertical().id_salt("2da-rows").auto_shrink([false, false]).show_rows(
            ui,
            row_height,
            rows.len(),
            |ui, range| {
                for &r in &rows[range] {
                    ui.horizontal(|ui| {
                        fixed(ui, 50.0, egui::RichText::new(r.to_string()).weak());
                        let origin = l.row_origin(r);
                        if layered {
                            let text = egui::RichText::new(&l.copies[origin].0);
                            fixed(ui, width, if origin < bottom { text } else { text.weak() });
                        }
                        for (c, &strref) in strrefs.iter().enumerate() {
                            let raw = t.cell(r, c);
                            let text = raw
                                .filter(|_| strref)
                                .and_then(|v| v.parse::<u32>().ok())
                                .and_then(|n| game?.string(mg_core::StrRef(n)));
                            let shown =
                                text.clone().unwrap_or_else(|| raw.unwrap_or("****").into());
                            let mut rich = egui::RichText::new(shown).monospace();
                            let cell_origin = l.cell_origin(r, c);
                            // Changed, not added: the row is in the copy below.
                            let was = (layered
                                && cell_origin < bottom
                                && r < l.copies[cell_origin + 1].1.len())
                            .then(|| l.before(r, c))
                            .flatten();
                            if was.is_some() {
                                rich = rich.color(changed);
                            }
                            let resp = fixed(ui, width, rich);
                            if text.is_some() || was.is_some() {
                                let mut tip = Vec::new();
                                if text.is_some() {
                                    tip.push(format!("StrRef {}", raw.unwrap_or_default()));
                                }
                                if let Some((below, value)) = was {
                                    let from = &l.copies[cell_origin].0;
                                    let value = value.unwrap_or("****");
                                    tip.push(format!("Set by {from}; {value} in {below}"));
                                }
                                resp.on_hover_text(tip.join("\n"));
                            }
                        }
                    });
                }
            },
        );
    });
}
