//! A generic editor for any GFF resource: its field tree with every value
//! editable. The fallback before (and beside) the dedicated editors.

use std::collections::HashMap;

use egui::Ui;
use mg_core::{Gender, Language};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::{Struct, Value};
use mg_resman::ResKey;

use crate::text::{decode, encode, from_editor, to_editor, with_english};
use crate::{Action, Moonglow};

struct Ctx<'a> {
    key: ResKey,
    actions: &'a mut Vec<Action>,
    buffers: &'a mut HashMap<egui::Id, String>,
}

impl Ctx<'_> {
    fn set(&mut self, path: &GffPath, label: &str, value: Value) {
        self.actions.push(Action::Apply(Command::new(
            format!("Set {label}"),
            vec![Edit::SetField {
                key: self.key,
                path: path.clone(),
                label: label.to_string(),
                value: Some(value),
            }],
        )));
    }

    /// A text box that commits on focus loss; text with line breaks gets a
    /// multi-line box and keeps its line-end style.
    fn text(&mut self, ui: &mut Ui, id: egui::Id, current: &str) -> Option<String> {
        let (shown, crlf) = to_editor(current);
        let buf = self.buffers.entry(id).or_insert_with(|| shown.clone());
        let edit = if shown.contains('\n') {
            egui::TextEdit::multiline(buf).desired_rows(3)
        } else {
            egui::TextEdit::singleline(buf)
        };
        let r = ui.add(edit.id(id).desired_width(280.0));
        if !r.has_focus() && !r.lost_focus() && *buf != shown {
            *buf = shown.clone();
        }
        (r.lost_focus() && *buf != shown).then(|| from_editor(buf, crlf))
    }
}

pub(crate) fn ui(app: &mut Moonglow, ui: &mut Ui, key: ResKey) {
    let Moonglow { ws, actions, buffers, .. } = app;
    let Some(ws) = ws else { return };
    let doc = match ws.doc(&key) {
        Ok(d) => d,
        Err(e) => {
            ui.colored_label(ui.visuals().error_fg_color, e.to_string());
            return;
        }
    };
    ui.horizontal(|ui| {
        ui.strong(key.to_string());
        ui.weak(format!("{} file", doc.file_type_str()));
    });
    ui.separator();
    let mut ctx = Ctx { key, actions, buffers };
    egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
        struct_ui(&mut ctx, ui, &doc.root, &GffPath::root());
    });
}

fn struct_ui(ctx: &mut Ctx<'_>, ui: &mut Ui, s: &Struct, path: &GffPath) {
    egui::Grid::new(("gff", ctx.key, path.to_string())).num_columns(3).striped(true).show(
        ui,
        |ui| {
            for f in &s.fields {
                let label = f.label.to_string_lossy();
                let id = egui::Id::new(("gff", ctx.key, path.to_string(), &label));
                match &f.value {
                    Value::Struct(child) => {
                        ui.label(&label);
                        ui.weak(format!("struct {}", child.id));
                        let child_path = path.field(&label);
                        egui::CollapsingHeader::new(format!("{} fields", child.fields.len()))
                            .id_salt(id)
                            .show(ui, |ui| struct_ui(ctx, ui, child, &child_path));
                    }
                    Value::List(items) => {
                        ui.label(&label);
                        ui.weak("list");
                        egui::CollapsingHeader::new(format!("{} items", items.len()))
                            .id_salt(id)
                            .show(ui, |ui| {
                                for (i, item) in items.iter().enumerate() {
                                    let item_path = path.item(&label, i);
                                    egui::CollapsingHeader::new(format!(
                                        "[{i}] struct {}",
                                        item.id
                                    ))
                                    .id_salt((id, i))
                                    .show(ui, |ui| struct_ui(ctx, ui, item, &item_path));
                                }
                            });
                    }
                    v => {
                        ui.label(&label);
                        ui.weak(v.field_type().json_name());
                        value_ui(ctx, ui, id, path, &label, v);
                    }
                }
                ui.end_row();
            }
        },
    );
}

fn value_ui(ctx: &mut Ctx<'_>, ui: &mut Ui, id: egui::Id, path: &GffPath, label: &str, v: &Value) {
    macro_rules! number {
        ($x:expr, $variant:ident) => {{
            let mut n = *$x;
            let r = ui.add(egui::DragValue::new(&mut n));
            if (r.drag_stopped() || (r.changed() && !r.dragged())) && n != *$x {
                ctx.set(path, label, Value::$variant(n));
            }
        }};
    }
    match v {
        Value::Byte(x) => number!(x, Byte),
        Value::Char(x) => number!(x, Char),
        Value::Word(x) => number!(x, Word),
        Value::Short(x) => number!(x, Short),
        Value::Dword(x) => number!(x, Dword),
        Value::Int(x) => number!(x, Int),
        Value::Dword64(x) => number!(x, Dword64),
        Value::Int64(x) => number!(x, Int64),
        Value::Float(x) => {
            let mut n = *x;
            let r = ui.add(egui::DragValue::new(&mut n).speed(0.01));
            if (r.drag_stopped() || (r.changed() && !r.dragged())) && n.to_bits() != x.to_bits() {
                ctx.set(path, label, Value::Float(n));
            }
        }
        Value::Double(x) => {
            let mut n = *x;
            let r = ui.add(egui::DragValue::new(&mut n).speed(0.01));
            if (r.drag_stopped() || (r.changed() && !r.dragged())) && n.to_bits() != x.to_bits() {
                ctx.set(path, label, Value::Double(n));
            }
        }
        Value::String(b) => {
            if let Some(t) = ctx.text(ui, id, &decode(b)) {
                ctx.set(path, label, Value::String(encode(&t)));
            }
        }
        Value::ResRef(b) => {
            // Resrefs longer than 16 characters are not accepted; the field
            // shows the stored value again.
            if let Some(t) = ctx.text(ui, id, &decode(b))
                && t.len() <= 16
            {
                ctx.set(path, label, Value::ResRef(encode(&t)));
            }
        }
        Value::LocString(ls) => {
            ui.horizontal(|ui| {
                if !ls.strref.is_none() {
                    ui.weak(format!("StrRef {}", ls.strref.0));
                }
                let current = ls
                    .text(Language::ENGLISH, Gender::Male)
                    .map(|t| t.into_owned())
                    .unwrap_or_default();
                if let Some(t) = ctx.text(ui, id, &current) {
                    ctx.set(path, label, Value::LocString(with_english(ls.clone(), &t)));
                }
            });
        }
        Value::Void(b) => {
            ui.weak(format!("{} bytes", b.len()));
        }
        Value::Struct(_) | Value::List(_) => {}
    }
}
