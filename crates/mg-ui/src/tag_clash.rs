//! Duplicate tags a command makes: "LAMP is now on two objects", with
//! Retag offered (each new object's tag with a number, unique in the
//! module, as one undoable step). Each command applied is weighed
//! ([`made_by`]: a tag typed in Properties, an object placed from the
//! palette or pasted, Create Set's waypoints, Find and Replace, Advanced
//! Search's Set a Field), and an area's Copy… with what it holds. A
//! script finds one object by a tag, and a copy pasted with its
//! original's is the one it finds (`engine_static_placeables`). The
//! project's `moonglow.toml` says which to warn of (`[tags]
//! paste_warning`) and which must be unique; where it doesn't say,
//! Options › General › Show duplicate tags warning does (off by default,
//! as Aurora has none).

use mg_core::ResType;
pub use mg_edit::tags::Made;
use mg_edit::tags::Weighed;
use mg_edit::{Command, Edit, Workspace};
use mg_gff::Value;
use mg_module::project_file::PasteWarning;
use mg_module::tags::{Clash, MadeTag, TagCache, TagIndex, clashes, retag};
use mg_resman::ResKey;

use crate::Moonglow;

/// The Duplicate Tags window: what was made twice, and the new objects
/// that have those tags.
#[derive(Debug, Clone, PartialEq)]
pub struct TagClash {
    /// "“Paste”", "The copy of inn".
    pub what: String,
    pub clashes: Vec<Clash>,
    /// The objects made with one of those tags.
    pub objects: Vec<(Made, String)>,
    /// Tags the window doesn't list one by one (an area copied with all
    /// its objects': only the area's own and those that must be unique
    /// are), how many.
    pub others: usize,
    /// The objects made with one of those (Retag All retags them too).
    pub more: Vec<(Made, String)>,
    /// Every object made: a pasted or copied door's `LinkedTo` that
    /// names another's tag follows it when that is retagged.
    pub made: Vec<Made>,
    /// The scripts' strings that spell a listed tag (`kz_inn.nss line
    /// 12`): they find the original's, or either.
    pub scripts: Vec<String>,
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

/// What a made object's tag is to [`clashes`] (`mg_edit::tags::weigh`),
/// its blueprint found as the module, its haks or the game have it.
fn weigh(ws: &mut Workspace, game: Option<&mg_rules::GameData>, made: &Made) -> Option<Weighed> {
    let blueprint =
        |ws: &Workspace, key: ResKey| mg_module::gff_root(Some(&ws.module), game?, &key);
    mg_edit::tags::weigh(ws, made, &blueprint)
}

/// The objects a command gives a tag, to check once it is applied
/// ([`check`]; `mg_edit::tags::made_by`): each placed object a `SetField`
/// gives another `Tag` (typed in Properties, Create Set's waypoints, Find
/// and Replace, Advanced Search's Set a Field, Edit Together, a
/// plugin's), each area's own, and each object it places (from the
/// palette, a paste, a prefab). Read before the command is applied.
pub(crate) fn made_by(ws: &mut Workspace, edits: &[Edit]) -> Vec<Made> {
    mg_edit::tags::made_by(ws, edits)
}
/// After a command or a copy (`what`, its name): says what it put on
/// more than one object (as
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
        .iter()
        .cloned()
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
    let mut counted = Vec::new();
    if whole_area && warning == PasteWarning::Any {
        let own = |t: &str| {
            tagged.iter().any(|(m, tag)| m.key.restype == ResType::ARE && tag == t.as_bytes())
        };
        (found, counted) = found.into_iter().partition(|c| c.unique.is_some() || own(&c.tag));
    }
    let others = counted.len();
    if found.is_empty() && others == 0 {
        return;
    }
    let of = |clashes: &[Clash]| -> Vec<(Made, String)> {
        (tagged.iter())
            .filter(|(_, t)| clashes.iter().any(|c| c.tag.as_bytes() == t.as_slice()))
            .map(|(m, t)| (m.clone(), crate::text::decode(t)))
            .collect()
    };
    let (objects, more) = (of(&found), of(&counted));
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
    let scripts = spelled(app, &[&found[..], &counted[..]].concat());
    let what = what.to_string();
    let clash =
        TagClash { what, clashes: found, objects, others, more, made, scripts, from_option };
    app.tag_clash = Some(clash);
}

/// The strings in the module's scripts that spell one of the tags
/// (`kz_inn.nss line 12`), as Find References finds them: a script that
/// looks one up finds the original's or the copy, and isn't changed by a
/// Retag. None while the module's references are read the first time.
fn spelled(app: &mut Moonglow, clashes: &[Clash]) -> Vec<String> {
    let codepage = crate::text::script_codepage();
    let Some(ws) = app.ws.as_ref() else { return Vec::new() };
    let Some(index) = app.references.index(&ws.module, app.background_jobs) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for c in clashes {
        for m in index.mentions(&c.tag, false, codepage) {
            out.push(format!("{} line {}", m.script, m.line));
        }
    }
    out.dedup();
    out
}

/// The Copy window's word on a blueprint's copy whose tag another
/// blueprint has (the module's, or the game's it is copied from): what is
/// placed from either has it. As a paste is weighed (the project's rule,
/// else the builder's option): a tag the project wants unique, any tag
/// where it warns of any, a tag typed in the window (not the original's)
/// with the option on. None while the module's references are read.
pub(crate) fn blueprint_copy_warning(
    app: &mut Moonglow,
    from: ResKey,
    original: Option<&str>,
    tag: &str,
) -> Option<String> {
    let tag = tag.trim();
    let ws = app.ws.as_ref()?;
    let rules = &ws.module.project_file.tags;
    let warning = rules.paste_warning_or(app.settings.warn_duplicate_tags);
    let unique = rules.unique_for(tag).map(|(p, _)| p.to_string());
    let typed = original != Some(tag);
    let warned = match warning {
        PasteWarning::Any => true,
        PasteWarning::Custom => typed || unique.is_some(),
        PasteWarning::Unique => unique.is_some(),
        PasteWarning::Off => false,
    };
    if tag.is_empty() || !warned {
        return None;
    }
    let mut others = (app.references.index(&ws.module, app.background_jobs)?)
        .blueprints_tagged(&crate::text::encode(tag));
    if !typed && !others.contains(&from) {
        others.push(from);
    }
    let names: Vec<String> = others.iter().map(ToString::to_string).collect();
    let (first, more) = match names.as_slice() {
        [] => return None,
        [one] => (one.clone(), String::new()),
        [one, rest @ ..] => (one.clone(), format!(" and {} more", rest.len())),
    };
    let why = unique.map_or(String::new(), |p| format!(" (the project wants {p} unique)"));
    Some(format!("{tag} is {first}'s tag{more} too: what is placed from either has it{why}"))
}

/// The edits that give each new object with a clashing tag one of its
/// own (the tag with a number, unique in the module), with `all` those of
/// the tags only counted too: one step to undo. What was made with them
/// that names one by its tag (a pasted or copied door's or trigger's
/// `LinkedTo`, where it leads to the one object made with that tag in its
/// area) names its new tag: the copy's transitions lead within the copy,
/// as the original's within the original.
pub(crate) fn retag_edits(app: &mut Moonglow, clash: &TagClash, all: bool) -> Vec<Edit> {
    let Some(ws) = app.ws.as_mut() else { return Vec::new() };
    let index = index(&mut app.tag_cache, ws);
    let mut given: Vec<String> = Vec::new();
    let mut edits = Vec::new();
    // (Where, the tag it had, the one it is given.)
    let mut retagged: Vec<(&Made, &str, String)> = Vec::new();
    let chosen = clash.objects.iter().chain(clash.more.iter().filter(|_| all));
    for (made, tag) in chosen {
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
        retagged.push((made, tag, new));
    }
    for m in &clash.made {
        let Some(link) = ws.doc(&m.key).ok().and_then(|d| m.path.get(&d.root)?.string("LinkedTo"))
        else {
            continue;
        };
        let link = crate::text::decode(link);
        let mut to = retagged.iter().filter(|(r, tag, _)| r.key == m.key && *tag == link);
        if let (Some((_, _, new)), None) = (to.next(), to.next()) {
            edits.push(Edit::SetField {
                key: m.key,
                path: m.path.clone(),
                label: "LinkedTo".into(),
                value: Some(Value::String(crate::text::encode(new))),
            });
        }
    }
    edits
}

pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(clash) = app.tag_clash.take() else { return };
    let mut open = true;
    let (mut retag_them, mut retag_all, mut keep) = (false, false, false);
    crate::widgets::dialog(ctx, "Duplicate Tags", &mut open)
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(format!("{} put tags on more than one object of the module:", clash.what));
            // (Eight lines at most, "and N more" one of them, as
            // `text::some_of` names things.)
            const ROOM: usize = 8;
            let n = clash.clashes.len();
            let shown = if n <= ROOM { n } else { ROOM - 1 };
            for c in clash.clashes.iter().take(shown) {
                ui.label(format!("• {}", c.sentence()));
            }
            if n > shown {
                ui.label(format!("• and {} more", n - shown));
            }
            if clash.others > 0 {
                let s = if clash.others == 1 { "" } else { "s" };
                ui.weak(format!(
                    "{} other tag{s} of the copy's objects are the original's too.",
                    clash.others
                ));
            }
            ui.weak("A script finds only one object by a tag (GetObjectByTag).");
            if !clash.scripts.is_empty() {
                let n = clash.scripts.len();
                let named = crate::text::some_of(&clash.scripts, 3);
                ui.weak(format!(
                    "Spelled in scripts ({n}: {named}): they find the original's or the new \
                     one's, and Retag leaves them as they are."
                ));
            }
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
                if !clash.more.is_empty() {
                    retag_all = ui
                        .button("Retag All")
                        .on_hover_text(
                            "Retags the copy's objects of the tags only counted too: each copy \
                             of the area's objects a tag of its own",
                        )
                        .clicked();
                }
                keep = ui.button("Keep Tags").clicked();
            });
        });
    if retag_them || retag_all {
        let edits = retag_edits(app, &clash, retag_all);
        let n = edits
            .iter()
            .filter(|e| matches!(e, Edit::SetField { label, .. } if label == "Tag"))
            .count();
        let links = edits.len() - n;
        if let Err(e) = app.apply(Command::new("Retag", edits)) {
            app.log.error(e.to_string());
        } else {
            let s = |n: usize| if n == 1 { "" } else { "s" };
            let led = match links {
                0 => String::new(),
                n => format!(", and {n} transition{} of what was made to them", s(n)),
            };
            app.log.info(format!("Retagged {n} object{}{led}", s(n)));
            if !clash.scripts.is_empty() {
                app.log.warn(format!(
                    "Retag: scripts spell the tags it changed, and still find the original's: {}",
                    crate::text::some_of(&clash.scripts, 6)
                ));
            }
        }
        return;
    }
    if open && !keep {
        app.tag_clash = Some(clash);
    }
}
