//! Duplicate tags made by a paste or a copy: "LAMP is now on two
//! objects", with Retag offered (each new object's tag with a number,
//! unique in the module, as one undoable step). A script finds one object
//! by a tag, and a copy pasted with its original's is the one it finds
//! (`engine_static_placeables`). The project's `moonglow.toml` says
//! which to warn of (`[tags] paste_warning`) and which must be unique;
//! where it doesn't say, Options › General › Show duplicate tags warning
//! does (off by default, as Aurora has none).

use mg_core::ResType;
use mg_edit::{Command, Edit, GffPath, Workspace};
use mg_gff::Value;
use mg_module::project_file::PasteWarning;
use mg_module::tags::{Clash, MadeTag, TagCache, TagIndex, clashes, retag};
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
    /// Shown as the builder's option says (the project's moonglow.toml
    /// doesn't set `paste_warning`): the window offers Don't warn again.
    pub from_option: bool,
}

/// The module's objects by tag, as they are now (unsaved edits too). An
/// area not yet open is read from the module's copy and not kept as a
/// document (the workspace's documents are written back at a save, and a
/// module folder's file would be rewritten for one only read here): its
/// tags are kept in `cache`, read again only once its bytes change.
pub(crate) fn index(cache: &mut TagCache, ws: &Workspace) -> TagIndex {
    cache.index(&ws.module, |k| ws.loaded(k).map(|g| &g.root))
}

/// What a made object's tag is to [`clashes`]: the tag, whether a builder
/// set it (not its blueprint's own, as the module, its haks or the game
/// have the blueprint; an area's always), and whether it is a static
/// placeable.
struct Weighed {
    tag: Vec<u8>,
    custom: bool,
    is_static: bool,
}

fn weigh(ws: &mut Workspace, game: Option<&mg_rules::GameData>, made: &Made) -> Option<Weighed> {
    let doc = ws.doc(&made.key).ok()?;
    let s = made.path.get(&doc.root)?.clone();
    let tag = s.string("Tag").map(<[u8]>::to_vec)?;
    let kind = match made.path.0.last() {
        Some(mg_edit::Step::Item(list, _)) => mg_rules::ObjectKind::from_list(list),
        _ => None,
    };
    // (A store names its blueprint in ResRef, the others in TemplateResRef.)
    let template = kind.and_then(|kind| Some((kind, s.resref(kind.resref_field())?)));
    let blueprint = template.and_then(|(kind, r)| {
        let key = ResKey::new(r, kind.restype());
        mg_module::gff_root(Some(&ws.module), game?, &key)
    });
    let is_static =
        kind == Some(mg_rules::ObjectKind::Placeable) && s.integer("Static").unwrap_or(0) != 0;
    let custom = mg_module::tags::is_custom_tag(&s, blueprint.as_ref());
    Some(Weighed { tag, custom, is_static })
}

/// After a paste or a copy: says what it put on more than one object (as
/// the project says to, else as the builder's option: the tags a builder
/// set and those that must be unique with it on, only those that must be
/// unique with it off; never a static placeable's), in the log and the
/// Duplicate Tags window. `whole_area`: an area copied with all it holds,
/// whose every tag is its original's: with `paste_warning = "any"`, only
/// the area's own and those that must be unique are listed, the others
/// counted.
pub(crate) fn check(app: &mut Moonglow, what: &str, made: Vec<Made>, whole_area: bool) {
    let game = app.game.clone();
    let Some(ws) = app.ws.as_mut() else { return };
    let rules = ws.module.project_file.tags.clone();
    let warning = rules.paste_warning_or(app.settings.warn_duplicate_tags);
    let from_option = rules.paste_warning.is_none() && app.settings.warn_duplicate_tags;
    // (Nothing to warn of: the module's tags aren't read, which in a
    // persistent world of hundreds of areas takes a moment a paste.)
    let none_unique = warning == PasteWarning::Unique && !rules.any_unique();
    if warning == PasteWarning::Off || none_unique || made.is_empty() {
        return;
    }
    let weighed: Vec<(Made, Weighed)> = made
        .into_iter()
        .filter_map(|m| Some((m.clone(), weigh(ws, game.as_deref(), &m)?)))
        .collect();
    let index = index(&mut app.tag_cache, ws);
    let tags: Vec<MadeTag> = weighed
        .iter()
        .map(|(m, w)| MadeTag {
            tag: &w.tag,
            area: m.key.resref,
            custom: w.custom,
            is_static: w.is_static,
        })
        .collect();
    let mut found = clashes(&index, &tags, &rules, warning);
    let tagged: Vec<(Made, Vec<u8>)> = weighed
        .iter()
        .filter(|(_, w)| !w.is_static)
        .map(|(m, w)| (m.clone(), w.tag.clone()))
        .collect();
    let mut others = 0;
    if whole_area && warning == PasteWarning::Any {
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
    let what = what.to_string();
    app.tag_clash = Some(TagClash { what, clashes: found, objects, others, from_option });
}

/// The edits that give each new object with a clashing tag one of its
/// own (the tag with a number, unique in the module): one step to undo.
pub(crate) fn retag_edits(app: &mut Moonglow, clash: &TagClash) -> Vec<Edit> {
    let Some(ws) = app.ws.as_ref() else { return Vec::new() };
    let index = index(&mut app.tag_cache, ws);
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
            if clash.from_option {
                let mut dont = !app.settings.warn_duplicate_tags;
                if ui
                    .checkbox(&mut dont, "Don't warn again")
                    .on_hover_text(
                        "Turns off Options › General › Show duplicate tags warning after a \
                         paste or a copy, where it can be turned on again. Tags a project's \
                         moonglow.toml wants unique are still warned of",
                    )
                    .changed()
                {
                    app.settings.warn_duplicate_tags = !dont;
                    if dont {
                        app.log.info(
                            "Duplicate tags won't be warned of: Options › General › Show \
                             duplicate tags warning turns it back on",
                        );
                    }
                }
            } else {
                ui.weak("The project's moonglow.toml asks for this warning ([tags]).");
            }
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
