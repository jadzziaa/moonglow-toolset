//! Duplicate tags made by a paste or a copy: "LAMP is now on two
//! objects", with Retag offered (each new object's tag with a number,
//! unique in the module, as one undoable step). A script finds one object
//! by a tag, and a copy pasted with its original's is the one it finds
//! (`engine_static_placeables`). The project's `moonglow.toml` says
//! which to warn of (`[tags] paste_warning`) and which must be unique.

use mg_core::ResType;
use mg_edit::{Command, Edit, GffPath, Workspace};
use mg_gff::Value;
use mg_module::project_file::PasteWarning;
use mg_module::tags::{Clash, TagIndex, clashes, retag};
use mg_resman::ResKey;

use crate::Moonglow;

/// An object just made (pasted, or in a copied area): where its tag is.
#[derive(Debug, Clone, PartialEq)]
pub struct Made {
    /// Its GIT (a placed object) or ARE (an area's own tag).
    pub key: ResKey,
    pub path: GffPath,
}

/// The Duplicate Tags window: what was made twice, and the new objects
/// that have those tags.
#[derive(Debug, Clone, PartialEq)]
pub struct TagClash {
    /// "The paste", "The copy of inn".
    pub what: String,
    pub clashes: Vec<Clash>,
    /// The objects made with one of those tags.
    pub objects: Vec<(Made, String)>,
    /// Tags the window doesn't list one by one (an area copied with all
    /// its objects': only the area's own and those that must be unique
    /// are), how many.
    pub others: usize,
}

/// The module's objects by tag, as they are now (unsaved edits too). An
/// area not yet open is read from the module's copy and not kept: the
/// workspace's documents are written back at a save, and a module
/// folder's file would be rewritten for one only read here.
pub(crate) fn index(ws: &Workspace) -> TagIndex {
    let mut keys: Vec<ResKey> = ws
        .module
        .keys()
        .filter(|k| matches!(k.restype, ResType::ARE | ResType::GIT))
        .copied()
        .collect();
    keys.sort();
    let mut index = TagIndex::default();
    for key in keys {
        let read;
        let root = match ws.loaded(&key) {
            Some(doc) => &doc.root,
            None => match ws.module.gff(&key) {
                Some(Ok(g)) => {
                    read = g;
                    &read.root
                }
                _ => continue,
            },
        };
        match key.restype {
            ResType::ARE => index.add_area(key.resref, root),
            _ => index.add_git(key.resref, root),
        }
    }
    index
}

/// The tag of the object at `made`.
fn tag_of(ws: &mut Workspace, made: &Made) -> Option<Vec<u8>> {
    let doc = ws.doc(&made.key).ok()?;
    let s = made.path.get(&doc.root)?;
    s.string("Tag").map(<[u8]>::to_vec)
}

/// After a paste or a copy: says what it put on more than one object (as
/// the project says to), in the log and the Duplicate Tags window.
/// `whole_area`: an area copied with all it holds, whose every tag is its
/// original's: only the area's own and those that must be unique are
/// listed, the others counted.
pub(crate) fn check(app: &mut Moonglow, what: &str, made: Vec<Made>, whole_area: bool) {
    let Some(ws) = app.ws.as_mut() else { return };
    let rules = ws.module.project_file.tags.clone();
    if rules.paste_warning == PasteWarning::Off || made.is_empty() {
        return;
    }
    let tagged: Vec<(Made, Vec<u8>)> =
        made.into_iter().filter_map(|m| Some((m.clone(), tag_of(ws, &m)?))).collect();
    let index = index(ws);
    let tags: Vec<&[u8]> = tagged.iter().map(|(_, t)| t.as_slice()).collect();
    let mut found = clashes(&index, &tags, &rules);
    let mut others = 0;
    if whole_area {
        let own = |t: &str| {
            tagged.iter().any(|(m, tag)| m.key.restype == ResType::ARE && tag == t.as_bytes())
        };
        let before = found.len();
        found.retain(|c| c.unique.is_some() || own(&c.tag));
        others = before - found.len();
    }
    if found.is_empty() && others == 0 {
        return;
    }
    let objects: Vec<(Made, String)> = tagged
        .iter()
        .filter(|(_, t)| found.iter().any(|c| c.tag.as_bytes() == t.as_slice()))
        .map(|(m, t)| (m.clone(), crate::text::decode(t)))
        .collect();
    for c in &found {
        app.log.warn(format!("{what}: {}", c.sentence()));
    }
    if others > 0 {
        let s = if others == 1 { "" } else { "s" };
        app.log.info(format!("{what}: {others} other tag{s} are its original's too"));
    }
    // (Those only counted ask nothing.)
    if found.is_empty() {
        return;
    }
    app.tag_clash = Some(TagClash { what: what.to_string(), clashes: found, objects, others });
}

/// The edits that give each new object with a clashing tag one of its
/// own (the tag with a number, unique in the module): one step to undo.
pub(crate) fn retag_edits(app: &mut Moonglow, clash: &TagClash) -> Vec<Edit> {
    let Some(ws) = app.ws.as_mut() else { return Vec::new() };
    let index = index(ws);
    let mut given: Vec<String> = Vec::new();
    let mut edits = Vec::new();
    for (made, tag) in &clash.objects {
        let taken =
            |t: &str| index.count(&crate::text::encode(t)) > 0 || given.iter().any(|g| g == t);
        let new = retag(tag, &taken);
        given.push(new.clone());
        edits.push(Edit::SetField {
            key: made.key,
            path: made.path.clone(),
            label: "Tag".into(),
            value: Some(Value::String(crate::text::encode(&new))),
        });
    }
    edits
}

pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(clash) = app.tag_clash.take() else { return };
    let mut open = true;
    let (mut retag_them, mut keep) = (false, false);
    crate::widgets::dialog(ctx, "Duplicate Tags", &mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(format!("{} put tags on more than one object of the module:", clash.what));
            const SHOWN: usize = 8;
            for c in clash.clashes.iter().take(SHOWN) {
                ui.label(format!("• {}", c.sentence()));
            }
            if clash.clashes.len() > SHOWN {
                ui.label(format!("• and {} more", clash.clashes.len() - SHOWN));
            }
            if clash.others > 0 {
                let s = if clash.others == 1 { "" } else { "s" };
                ui.weak(format!(
                    "{} other tag{s} of the copy's objects are the original's too.",
                    clash.others
                ));
            }
            ui.weak("A script finds only one object by a tag (GetObjectByTag).");
            ui.separator();
            ui.horizontal(|ui| {
                let can = !clash.objects.is_empty();
                retag_them = ui
                    .add_enabled(can, egui::Button::new("Retag"))
                    .on_hover_text(
                        "Gives each new object its tag with a number (LAMP_2), unique in the \
                         module: one step to undo",
                    )
                    .clicked()
                    || (can && crate::widgets::enter(ui));
                keep = ui.button("Keep Tags").clicked();
            });
        });
    if retag_them {
        let edits = retag_edits(app, &clash);
        let n = edits.len();
        if let Err(e) = app.apply(Command::new("Retag", edits)) {
            app.log.error(e.to_string());
        } else {
            let s = if n == 1 { "" } else { "s" };
            app.log.info(format!("Retagged {n} object{s}"));
        }
        return;
    }
    if open && !keep {
        app.tag_clash = Some(clash);
    }
}
