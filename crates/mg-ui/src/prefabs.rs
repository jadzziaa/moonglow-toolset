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

use crate::Moonglow;
use crate::area_view::ObjectClip;
use crate::saved::Shelf;
pub use crate::saved::valid_name;

const SHELF: Shelf = Shelf { folder: "prefabs", suffix: ".prefab.json", what: "prefab" };

/// Where prefabs are kept.
pub fn dir() -> Option<PathBuf> {
    SHELF.dir()
}

/// The prefabs saved in `dir`, by name.
pub fn list(dir: Option<&std::path::Path>) -> Vec<String> {
    SHELF.list(dir)
}

/// A prefab's file.
fn file(dir: Option<&std::path::Path>, name: &str) -> Result<PathBuf, String> {
    SHELF.file(dir, name)
}

/// Deletes a prefab's file: its text, to put it back with [`restore`].
pub fn delete(dir: Option<&std::path::Path>, name: &str) -> Result<String, String> {
    SHELF.delete(dir, name)
}

/// Puts a deleted prefab back, unless one of its name was saved since.
pub fn restore(dir: Option<&std::path::Path>, name: &str, text: &str) -> Result<(), String> {
    SHELF.restore(dir, name, text)
}

/// Gives a prefab another name. A name another prefab has is refused.
pub fn rename(dir: Option<&std::path::Path>, from: &str, to: &str) -> Result<(), String> {
    SHELF.rename(dir, from, to)
}

/// What a prefab holds, in a line or two: how many objects of each kind,
/// and their tags.
pub fn summary(dir: Option<&std::path::Path>, name: &str) -> Result<String, String> {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};
    use std::time::SystemTime;

    type Key = PathBuf;
    type Entry = (u64, Option<SystemTime>, Result<String, String>);
    static CACHE: OnceLock<Mutex<HashMap<Key, Entry>>> = OnceLock::new();

    let path = file(dir, name)?;
    let meta = std::fs::metadata(&path).ok();
    let stamp = meta.as_ref().map(|m| (m.len(), m.modified().ok()));
    if let Some((len, mtime)) = stamp
        && let Ok(cache) = CACHE.get_or_init(Default::default).lock()
        && let Some((clen, cmtime, res)) = cache.get(&path)
        && *clen == len
        && *cmtime == mtime
    {
        return res.clone();
    }

    let computed = (|| -> Result<String, String> {
        let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        let json: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
        let g = from_json(&json, Codepage::default()).map_err(|e| e.to_string())?;
        let mut counts = [0usize; ObjectKind::ALL.len()];
        let mut tags: Vec<String> = Vec::new();
        for item in g.root.list("Objects").unwrap_or(&[]) {
            let kind = item.integer("Kind").and_then(|k| usize::try_from(k).ok());
            if let Some(n) = kind.and_then(|k| counts.get_mut(k)) {
                *n += 1;
            }
            if let Some(Value::Struct(s)) = item.get("Object")
                && let Some(tag) = s.string("Tag").map(crate::text::decode)
                && !tag.is_empty()
                && !tags.contains(&tag)
            {
                tags.push(tag);
            }
        }
        let kinds: Vec<String> = ObjectKind::ALL
            .iter()
            .zip(counts)
            .filter(|(_, n)| *n > 0)
            .map(|(k, n)| {
                let noun = if n == 1 { k.name().to_string() } else { k.label().to_lowercase() };
                format!("{n} {noun}")
            })
            .collect();
        if kinds.is_empty() {
            return Err("no objects".into());
        }
        let mut line = kinds.join(", ");
        if !tags.is_empty() {
            line.push_str(&format!("\nTags: {}", crate::text::some_of(&tags, SUMMARY_TAGS)));
        }
        Ok(line)
    })();

    if let Some((len, mtime)) = stamp
        && let Ok(mut cache) = CACHE.get_or_init(Default::default).lock()
    {
        if cache.len() >= 256 {
            cache.clear();
        }
        cache.insert(path, (len, mtime, computed.clone()));
    }
    computed
}

/// How many tags a prefab's summary names.
const SUMMARY_TAGS: usize = 8;

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
        SHELF.write(self.prefab_dir.as_deref(), name, &to_text(clip)?)
    }

    /// Edit › Prefabs › a prefab: its objects follow the pointer in the area
    /// shown last, to be placed with a click.
    pub fn place_prefab(&mut self, name: &str) {
        let Some(game) = &self.game else { return };
        let loaded = SHELF.read(self.prefab_dir.as_deref(), name).and_then(|t| from_text(game, &t));
        match loaded {
            Ok(clip) => {
                self.object_clip = Some(clip);
                match self.palette.area.and_then(|a| self.area_views.get_mut(&a)) {
                    Some(view) => {
                        view.pasting = true;
                        view.paste_turn = 0.0;
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
    // (The prefabs kept, read from their folder as the window opens.)
    let names = crate::area_tools::kept_while_shown(ctx, egui::Id::new("prefab-names"), || {
        std::sync::Arc::new(list(app.prefab_dir.as_deref()))
    });
    let footed = crate::widgets::dialog(ctx, "Save as Prefab", &mut open)
        .collapsible(false)
        .resizable(false)
        .show_foot(ctx, |ui| {
            ui.label(format!(
                "{} object(s), placed again from Edit › Prefabs in any area.",
                clip.objects.len()
            ));
            ui.horizontal(|ui| {
                crate::widgets::field_label(ui, "Name");
                let r = ui.add(mg_uikit::text_field(&mut name).desired_width(220.0));
                crate::widgets::autofocus(ui, &r);
            });
            let ok = valid_name(&name);
            if !ok && !name.is_empty() {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    "Letters, digits, spaces, - and _ only",
                );
            }
            if names.iter().any(|n| n.eq_ignore_ascii_case(name.trim())) {
                ui.weak("A prefab of that name will be replaced");
            }
            ((), crate::widgets::Foot::new().main("Save", ok).cancel())
        });
    let save = footed.as_ref().is_some_and(|f| f.pressed.main());
    let cancel = footed.as_ref().is_some_and(|f| f.pressed.cancelled());
    if save {
        match app.save_prefab(&name, &clip) {
            Ok(path) => app.log.info(format!("Prefab saved: {}", path.display())),
            Err(e) => app.log.error(format!("Save as Prefab: {e}")),
        }
    } else if open && !cancel {
        app.prefab_save = Some((name, clip));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_uses_singular_for_one_and_plural_for_many() {
        let dir = mg_testkit::scratch_dir("prefab-summary-plurals");
        let item = |kind: ObjectKind, tag: &str| {
            let mut obj = mg_gff::Struct::new(kind.struct_id());
            obj.set("Tag", Value::String(tag.as_bytes().to_vec()));
            let mut s = mg_gff::Struct::new(0);
            s.set("Kind", Value::Byte(kind.index() as u8));
            s.set("Object", Value::Struct(obj));
            s
        };
        let mut g = Gff::new(*b"MGPF");
        g.root.set(
            "Objects",
            Value::List(vec![
                item(ObjectKind::Waypoint, "WP_1"),
                item(ObjectKind::Creature, "CRE_1"),
                item(ObjectKind::Creature, "CRE_2"),
            ]),
        );
        let json = to_json(&g, Codepage::default()).unwrap();
        std::fs::write(dir.join("Camp.prefab.json"), to_json_text(&json, TextStyle::default()))
            .unwrap();
        let text = summary(Some(&dir), "Camp").unwrap();
        assert!(text.contains("2 creatures"), "{text}");
        assert!(text.contains("1 waypoint"), "{text}");
        assert!(!text.contains("1 waypoints"), "{text}");
    }
}
