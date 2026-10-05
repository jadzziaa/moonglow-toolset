//! Prefabs: placed objects saved together under a name (Save as Prefab…
//! in an area's menu) and placed again like a paste, in any area of any
//! module (Edit › Prefabs). They live in Moonglow's data folder as JSON
//! (GFF in neverwinter.nim's format): each object's GIT struct, its kind
//! and its height above the ground, and the point the others keep their
//! places around.

use std::path::PathBuf;

use glam::Vec3;
use mg_area::{AreaObject, ObjectKind};
use mg_core::Codepage;
use mg_gff::{Gff, Struct, TextStyle, Value, from_json, to_json, to_json_text};
use mg_rules::GameData;

use crate::area_view::ObjectClip;
use crate::{Moonglow, recovery};

const SUFFIX: &str = ".prefab.json";

/// Where prefabs are kept.
pub fn dir() -> Option<PathBuf> {
    recovery::data_dir().map(|d| d.join("prefabs"))
}

/// The prefabs saved in `dir`, by name.
pub fn list(dir: Option<&std::path::Path>) -> Vec<String> {
    let Some(dir) = dir else { return Vec::new() };
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().to_str()?.strip_suffix(SUFFIX).map(str::to_string))
        .collect();
    names.sort_by_key(|n| n.to_lowercase());
    names
}

/// Deletes a prefab's file.
pub fn delete(dir: Option<&std::path::Path>, name: &str) -> Result<(), String> {
    let dir = dir.ok_or("Moonglow has no data folder")?;
    if !valid_name(name) {
        return Err(format!("'{name}' is not a prefab's name"));
    }
    let path = dir.join(format!("{name}{SUFFIX}"));
    std::fs::remove_file(&path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Whether a name can be a prefab's (it names a file).
pub fn valid_name(name: &str) -> bool {
    !name.trim().is_empty()
        && name.len() <= 64
        && name.chars().all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.'))
        && !name.starts_with('.')
}

/// A clipboard of objects as prefab text.
pub fn to_text(clip: &ObjectClip) -> Result<String, String> {
    let mut g = Gff::new(*b"MGPF");
    g.root.set("AnchorX", Value::Float(clip.anchor.x));
    g.root.set("AnchorY", Value::Float(clip.anchor.y));
    g.root.set("AnchorZ", Value::Float(clip.anchor.z));
    let objects = clip
        .objects
        .iter()
        .map(|(o, s, lift)| {
            let mut item = Struct::new(0);
            item.set("Kind", Value::Byte(o.kind.index() as u8));
            item.set("Lift", Value::Float(*lift));
            item.set("Object", Value::Struct(s.clone()));
            item
        })
        .collect();
    g.root.set("Objects", Value::List(objects));
    let json = to_json(&g, Codepage::default()).map_err(|e| e.to_string())?;
    Ok(to_json_text(&json, TextStyle { sort_keys: true, ..Default::default() }))
}

/// Prefab text as a clipboard of objects.
pub fn from_text(game: &GameData, text: &str) -> Result<ObjectClip, String> {
    let json: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let g = from_json(&json, Codepage::default()).map_err(|e| e.to_string())?;
    let f = |l: &str| g.root.float(l).unwrap_or(0.0);
    let anchor = Vec3::new(f("AnchorX"), f("AnchorY"), f("AnchorZ"));
    let mut objects = Vec::new();
    for item in g.root.list("Objects").unwrap_or(&[]) {
        let kind = item
            .integer("Kind")
            .and_then(|k| ObjectKind::ALL.get(usize::try_from(k).ok()?).copied())
            .ok_or("an object of no known kind")?;
        let Some(Value::Struct(s)) = item.get("Object") else {
            return Err("an object without data".into());
        };
        let lift = item.float("Lift").unwrap_or(0.0);
        objects.push((AreaObject::read(game, kind, 0, s), s.clone(), lift));
    }
    if objects.is_empty() {
        return Err("no objects".into());
    }
    Ok(ObjectClip { objects, anchor })
}

impl Moonglow {
    /// Saves objects as a prefab.
    pub fn save_prefab(&mut self, name: &str, clip: &ObjectClip) -> Result<PathBuf, String> {
        let dir = self.prefab_dir.clone().ok_or("Moonglow has no data folder")?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join(format!("{}{SUFFIX}", name.trim()));
        std::fs::write(&path, to_text(clip)?).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(path)
    }

    /// Edit › Prefabs › a prefab: its objects follow the pointer in the area
    /// shown last, to be placed with a click.
    pub fn place_prefab(&mut self, name: &str) {
        let Some(game) = &self.game else { return };
        let path = self.prefab_dir.as_ref().map(|d| d.join(format!("{name}{SUFFIX}")));
        let loaded = path
            .as_ref()
            .ok_or("Moonglow has no data folder".to_string())
            .and_then(|p| std::fs::read_to_string(p).map_err(|e| e.to_string()))
            .and_then(|t| from_text(game, &t));
        match loaded {
            Ok(clip) => {
                self.object_clip = Some(clip);
                match self.palette.area.and_then(|a| self.area_views.get_mut(&a)) {
                    Some(view) => {
                        view.pasting = true;
                        self.log.info(format!("Prefab {name}: click in the area to place it"));
                    }
                    None => {
                        self.log.info(format!("Prefab {name}: open an area and paste (Ctrl+V)"))
                    }
                }
            }
            Err(e) => self.log.error(format!("Prefab {name}: {e}")),
        }
    }
}

/// The Save as Prefab window.
pub(crate) fn save_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some((mut name, clip)) = app.prefab_save.take() else { return };
    let mut open = true;
    let (mut save, mut cancel) = (false, false);
    egui::Window::new("Save as Prefab")
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .collapsible(false)
        .resizable(false)
        .open(crate::widgets::open_unless_escape(ctx, "Save as Prefab", &mut open))
        .show(ctx, |ui| {
            ui.label(format!(
                "{} object(s), placed again from Edit › Prefabs in any area.",
                clip.objects.len()
            ));
            ui.horizontal(|ui| {
                crate::widgets::field_label(ui, "Name");
                let r = ui.add(egui::TextEdit::singleline(&mut name).desired_width(220.0));
                crate::widgets::autofocus(ui, &r);
            });
            let ok = valid_name(&name);
            if !ok && !name.is_empty() {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    "Letters, digits, spaces, - and _ only",
                );
            }
            if list(app.prefab_dir.as_deref()).iter().any(|n| n.eq_ignore_ascii_case(name.trim())) {
                ui.weak("A prefab of that name will be replaced");
            }
            ui.horizontal(|ui| {
                save = ui.add_enabled(ok, egui::Button::new("Save")).clicked()
                    || (ok && crate::widgets::enter(ui));
                cancel = crate::widgets::cancel(ui);
            });
        });
    if save {
        match app.save_prefab(&name, &clip) {
            Ok(path) => app.log.info(format!("Prefab saved: {}", path.display())),
            Err(e) => app.log.error(format!("Save as Prefab: {e}")),
        }
    } else if open && !cancel {
        app.prefab_save = Some((name, clip));
    }
}
