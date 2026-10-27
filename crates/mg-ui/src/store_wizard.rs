//! Setup Store (a creature's or placeable's context menu in the area
//! viewer; Aurora's Store Setup Wizard): the shopkeeper's conversation, the
//! script that opens the store, the store to place where the shopkeeper
//! stands, and a friendlier faction for a hostile shopkeeper, as one
//! undoable command (`mg_module::store_setup`). And Add Popup Text (a
//! placeable's): a one-line conversation the placeable takes as its own.
//!
//! A project's `moonglow.toml` (`[store]`) may name the script that opens
//! its stores (no `openstoreNNN` is written then), the store's tag (a
//! pattern, made unique in the module), the store blueprint offered
//! first, and a variable the shopkeeper keeps the store's tag in; without
//! it, the wizard does as Aurora's.

use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::{Gff, Struct, Value};
use mg_module::factions::Factions;
use mg_module::instances::{Placing, git_list, instance};
use mg_module::project_file::StoreRules;
use mg_module::store_setup::{self, GREETING, NO, YES};
use mg_resman::ResKey;

use crate::{Action, Moonglow};

/// The wizard's pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Conversation,
    Store,
    Finish,
}

/// The Store Setup Wizard's choices.
#[derive(Debug, Clone, PartialEq)]
pub struct StoreWizard {
    pub area: ResRef,
    /// The shopkeeper: its list in the area's GIT (creatures or
    /// placeables) and index there.
    pub list: &'static str,
    pub creature: usize,
    pub page: Page,
    pub greeting: String,
    pub yes: String,
    pub no: String,
    pub appraise: bool,
    /// The conversation's and the script's names.
    pub dialog: String,
    pub script: String,
    /// The store blueprint chosen, and whether from the module's (Custom).
    pub store: Option<ResRef>,
    pub custom: bool,
    pub filter: String,
    /// The shopkeeper's faction is hostile: use `faction` instead.
    pub hostile: bool,
    pub use_faction: bool,
    pub faction: u32,
    /// The stores listed, for Custom or not.
    listed: Option<(bool, Vec<(ResRef, String)>)>,
    /// The project's `[store]` settings, if it has them.
    pub rules: Option<StoreRules>,
    /// The store's tag, as Finish will give it (worked out on the way to
    /// the last page).
    pub tag: Option<String>,
}

fn factions(app: &mut Moonglow) -> Option<Factions> {
    let key = ResKey::parse("repute", ResType::FAC)?;
    let ws = app.ws.as_mut()?;
    let codepage = crate::text::game_codepage();
    (ws.doc(&key).ok())
        .map(|g| Factions::read_in(&Gff { root: g.root.clone(), ..Gff::new(*b"FAC ") }, codepage))
}

/// The faction field of a shopkeeper in `list`.
fn faction_field(list: &str) -> &'static str {
    if list == "Creature List" { "FactionID" } else { "Faction" }
}

/// Opens the wizard for object `creature` of `list` (creatures or
/// placeables) in `area`, with Aurora's defaults.
pub fn open(app: &mut Moonglow, area: ResRef, list: &'static str, creature: usize) {
    let git = ResKey::new(area, ResType::GIT);
    let factions = factions(app);
    let Some(ws) = app.ws.as_mut() else { return };
    let Ok(doc) = ws.doc(&git) else { return };
    let Some(c) = doc.root.list(list).and_then(|l| l.get(creature)) else { return };
    let faction = c.integer(faction_field(list)).unwrap_or(0).max(0) as u32;
    // (A creature of the PC faction is put in Hostile by the game:
    // `engine_factions.rs`.)
    let pc = list == "Creature List" && i64::from(faction) == mg_rules::objects::PC_FACTION;
    let hostile = pc || store_setup::hostile(factions.as_ref(), faction);
    let module = &ws.module;
    let rules = module.project_file.store.clone();
    // The project's blueprint, chosen at first: the module's own (Custom)
    // if it has one of that name.
    let store = rules.as_ref().and_then(|r| r.blueprint);
    let custom = store.is_some_and(|r| module.contains(&ResKey::new(r, ResType::UTM)));
    app.store_wizard = Some(StoreWizard {
        area,
        list,
        creature,
        page: Page::Conversation,
        greeting: GREETING.into(),
        yes: YES.into(),
        no: NO.into(),
        appraise: true,
        dialog: store_setup::next_name(module, "store", ResType::DLG).to_string(),
        script: store_setup::next_name(module, "openstore", ResType::NSS).to_string(),
        store,
        custom,
        filter: String::new(),
        hostile,
        use_faction: true,
        faction: store_setup::merchant(factions.as_ref()),
        listed: None,
        rules,
        tag: None,
    });
}

/// The store blueprints to choose from: the game's (Standard) or the
/// module's (Custom), as (resref, name).
fn stores(app: &Moonglow, custom: bool) -> Vec<(ResRef, String)> {
    let Some(game) = app.game.as_deref() else { return Vec::new() };
    let names: Vec<ResRef> = if custom {
        app.ws
            .as_ref()
            .map_or(Vec::new(), |ws| ws.module.keys_of(ResType::UTM).map(|k| k.resref).collect())
    } else {
        game.resman.list(ResType::UTM)
    };
    let mut out: Vec<(ResRef, String)> = names
        .into_iter()
        .map(|r| {
            let k = ResKey::new(r, ResType::UTM);
            let data = app
                .ws
                .as_ref()
                .filter(|_| custom)
                .and_then(|ws| ws.module.get(&k).map(<[u8]>::to_vec))
                .or_else(|| game.resman.get(&k).ok().map(|d| d.into_owned()));
            let name = data
                .and_then(|d| Gff::read(&d).ok())
                .and_then(|g| g.root.locstring("LocName").and_then(|l| game.locstring(l)))
                .unwrap_or_default();
            (r, name)
        })
        .collect();
    out.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
    out
}

pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(mut w) = app.store_wizard.take() else { return };
    let (mut finish, mut close) = (false, false);
    let mut go_to_finish = false;
    if w.page == Page::Store && w.listed.as_ref().is_none_or(|(c, _)| *c != w.custom) {
        w.listed = Some((w.custom, stores(app, w.custom)));
    }
    let choices = w.listed.as_ref().map_or(&[][..], |(_, l)| l.as_slice()).to_vec();
    let faction_names: Vec<String> = factions(app)
        .map(|f| f.factions.iter().map(|x| x.name.clone()).collect())
        .unwrap_or_else(|| mg_module::factions::STANDARD.iter().map(|s| s.to_string()).collect());
    egui::Window::new("Store Setup Wizard")
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            match w.page {
                Page::Conversation => {
                    ui.heading("Conversation");
                    ui.label("Enter the conversation text for the shopkeeper.");
                    ui.label("What does the shopkeeper say when the conversation begins?");
                    let field = egui::TextEdit::multiline(&mut w.greeting).desired_rows(2);
                    let id = egui::Id::new("store-wizard-greeting");
                    let field = crate::widgets::text_area(ui, id, field).response.response;
                    crate::widgets::autofocus(ui, &field);
                    ui.label("What does the player say when the player wishes to see the store?");
                    ui.text_edit_singleline(&mut w.yes);
                    ui.label("What does the player say when the player is not interested?");
                    ui.text_edit_singleline(&mut w.no);
                    let opens = w.rules.as_ref().and_then(|r| r.open_script);
                    if opens.is_none() {
                        ui.checkbox(&mut w.appraise, "Use appraise checks");
                    }
                    ui.label("(optional) Enter the filename for the shopkeeper's conversation");
                    ui.add(egui::TextEdit::singleline(&mut w.dialog).char_limit(16));
                    match opens {
                        Some(script) => {
                            ui.label(format!(
                                "The store is opened by the project's script {script} \
                                 (moonglow.toml): no script is written."
                            ));
                        }
                        None => {
                            ui.label(
                                "(optional) Enter the filename for the script that will open \
                                 the store",
                            );
                            ui.add(egui::TextEdit::singleline(&mut w.script).char_limit(16));
                        }
                    }
                }
                Page::Store => {
                    ui.heading("Choose Store");
                    ui.label("Choose the store to use.");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut w.custom, false, "Standard");
                        ui.selectable_value(&mut w.custom, true, "Custom");
                        ui.add(egui::TextEdit::singleline(&mut w.filter).hint_text("Find"));
                    });
                    let needle = mg_core::text::Needle::new(&w.filter);
                    egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                        for (r, name) in &choices {
                            let label = format!("{name} ({r})");
                            if !needle.finds(&label) {
                                continue;
                            }
                            if ui.selectable_label(w.store == Some(*r), label).clicked() {
                                w.store = Some(*r);
                            }
                        }
                    });
                }
                Page::Finish => {
                    ui.heading("Finish");
                    ui.label("The Store Setup Wizard is now ready to create the store.");
                    ui.label("Click Finish to finish setting up the store.");
                    if let Some(tag) = &w.tag {
                        ui.label(format!("The store's tag: {tag}"));
                    }
                    ui.label(
                        "Tip: If you wish to move the store or the storekeeper later, it is \
                     strongly recommended that you move both of them together.",
                    );
                    if w.hostile {
                        ui.separator();
                        ui.label(
                            "Note: the shopkeeper's faction is hostile to player characters. It is \
                         strongly recommended that you change it to the Merchant faction.",
                        );
                        ui.checkbox(&mut w.use_faction, "Use selected faction instead:");
                        let current =
                            faction_names.get(w.faction as usize).cloned().unwrap_or_default();
                        // (Not the PC faction: the players' alone.)
                        let factions: Vec<u32> = (1..faction_names.len() as u32).collect();
                        crate::widgets::dropdown("store-faction").selected_text(current).show(
                            ui,
                            &mut w.faction,
                            &factions,
                            |i| faction_names[*i as usize].clone(),
                        );
                    }
                }
            }
            ui.separator();
            let opens = w.rules.as_ref().is_some_and(|r| r.open_script.is_some());
            let names_ok = ResRef::from_str(w.dialog.trim()).is_ok_and(|r| r != ResRef::EMPTY)
                && (opens || ResRef::from_str(w.script.trim()).is_ok_and(|r| r != ResRef::EMPTY));
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(w.page != Page::Conversation, egui::Button::new("< Back"))
                    .clicked()
                {
                    w.page = if w.page == Page::Finish { Page::Store } else { Page::Conversation };
                }
                let next_ok = match w.page {
                    Page::Conversation => names_ok,
                    Page::Store => w.store.is_some(),
                    Page::Finish => false,
                };
                if ui.add_enabled(next_ok, egui::Button::new("Next >")).clicked()
                    || (next_ok && crate::widgets::enter(ui))
                {
                    w.page = if w.page == Page::Conversation { Page::Store } else { Page::Finish };
                    if w.page == Page::Finish {
                        go_to_finish = true;
                    }
                }
                if ui.add_enabled(w.page == Page::Finish, egui::Button::new("Finish")).clicked()
                    || (w.page == Page::Finish && crate::widgets::enter(ui))
                {
                    finish = true;
                }
                if crate::widgets::cancel(ui) {
                    close = true;
                }
            });
        });
    // (Said on the last page where the project gives the tag; Aurora's
    // page says nothing of it.)
    if go_to_finish && w.rules.is_some() {
        w.tag = store_tag(app, &w).ok();
    }
    if finish {
        match build(app, &w) {
            Ok((edits, made)) => {
                app.actions.push(Action::ApplyMade(Command::new("Setup Store", edits), made))
            }
            Err(e) => app.log.error(format!("Setup Store: {e}")),
        }
        close = true;
    }
    if !close {
        app.store_wizard = Some(w);
    }
}

/// The tag the store will have: the blueprint's (as Aurora gives it),
/// or, in a project with `[store]`, its pattern's (`tag`) or else the
/// blueprint's with a number where another object has it: a store made
/// from one blueprint never shares its tag then.
fn store_tag(app: &mut Moonglow, w: &StoreWizard) -> Result<String, String> {
    let store = w.store.ok_or("no store chosen")?;
    let game = app.game.as_deref().ok_or("no game data")?;
    let ws = app.ws.as_mut().ok_or("no module")?;
    let blueprint = mg_module::gff_root(Some(&ws.module), game, &ResKey::new(store, ResType::UTM))
        .ok_or("the store blueprint is missing")?;
    let own = blueprint.string("Tag").map(crate::text::decode).unwrap_or_default();
    let Some(rules) = &w.rules else { return Ok(own) };
    let area_tag = ws
        .doc(&ResKey::new(w.area, ResType::ARE))
        .ok()
        .and_then(|a| a.root.string("Tag").map(crate::text::decode))
        .unwrap_or_else(|| w.area.to_string());
    let index = crate::tag_clash::index(ws);
    let taken = |t: &str| index.count(&crate::text::encode(t)) > 0;
    Ok(match &rules.tag {
        Some(pattern) => mg_module::tags::store_tag(pattern, &area_tag, &taken),
        None if taken(&own) => mg_module::tags::retag(&own, &taken),
        None => own,
    })
}

/// The edits: the conversation, the script (compiled), the shopkeeper's
/// conversation and faction, and the store placed where it stands; and
/// what they make, for the plugins' handlers.
fn build(
    app: &mut Moonglow,
    w: &StoreWizard,
) -> Result<(Vec<Edit>, crate::plugin_events::Made), String> {
    let dialog = ResRef::from_str(w.dialog.trim()).map_err(|e| e.to_string())?;
    let opens = w.rules.as_ref().and_then(|r| r.open_script);
    let script = match opens {
        Some(script) => script,
        None => ResRef::from_str(w.script.trim()).map_err(|e| e.to_string())?,
    };
    let store = w.store.ok_or("no store chosen")?;
    let tag = store_tag(app, w)?;
    let git = ResKey::new(w.area, ResType::GIT);
    let (creature, count) = {
        let ws = app.ws.as_mut().ok_or("no module")?;
        let doc = ws.doc(&git).map_err(|e| e.to_string())?;
        let creature: Struct = doc
            .root
            .list(w.list)
            .and_then(|l| l.get(w.creature))
            .cloned()
            .ok_or("the shopkeeper is gone")?;
        (creature, doc.root.list("StoreList").map_or(0, <[_]>::len))
    };
    let game = app.game.as_deref().ok_or("no game data")?;
    let ws = app.ws.as_ref().ok_or("no module")?;
    let read = |k: ResKey| mg_module::gff_root(Some(&ws.module), game, &k);
    let blueprint =
        read(ResKey::new(store, ResType::UTM)).ok_or("the store blueprint is missing")?;
    let item = |r: ResRef| read(ResKey::new(r, ResType::UTI));
    let placing = Placing { game, item: &item };
    let at = store_setup::store_placement(&creature);
    let mut placed = instance(&placing, ResType::UTM, &blueprint, at, &[]).ok_or("not a store")?;
    if w.rules.is_some() {
        placed.set("Tag", Value::String(crate::text::encode(&tag)));
    }
    // (In the game's codepage, as `store_tag` reads it: a tag of `é`
    // read as UTF-8 named a store there is none of.)
    let tag = placed.string("Tag").map(crate::text::decode).unwrap_or_default();
    let found = |t: ResType| {
        let key = ResKey::new(script, t);
        ws.module.contains(&key) || game.resman.contains(&key)
    };
    if opens.is_some() && !found(ResType::NCS) && !found(ResType::NSS) {
        app.log.warn(format!(
            "Setup Store: the project's script {script} is neither the module's nor the game's"
        ));
    }

    let text = store_setup::script(&tag, w.appraise);
    let nss = ResKey::new(script, ResType::NSS);
    let mut edits = vec![Edit::SetResource {
        key: ResKey::new(dialog, ResType::DLG),
        data: Some(
            {
                // (Its lines in the module's own letters, if it has a
                // table of them, and with plain line ends, as Aurora
                // writes a line's text.)
                let line = |t: &str| {
                    crate::text::for_windows_1252(&crate::text::with_loc_line_ends(t, ""))
                };
                store_setup::conversation(&line(&w.greeting), &line(&w.yes), &line(&w.no), script)
            }
            .to_bytes()
            .map_err(|e| e.to_string())?,
        ),
    }];
    // (The project's own script opens it: none is written.)
    if opens.is_none() {
        edits.push(Edit::SetResource { key: nss, data: Some(crate::text::encode(&text)) });
        match crate::script_view::compile_source(app, nss, &text) {
            Ok(ncs) => edits.push(Edit::SetResource {
                key: ResKey::new(script, ResType::NCS),
                data: Some(ncs),
            }),
            Err(e) => app.log.warn(format!("Setup Store: {script}: {}", e.message)),
        }
    }
    let path = GffPath::root().item(w.list, w.creature);
    let set = |label: &str, value: Value| Edit::SetField {
        key: git,
        path: path.clone(),
        label: label.into(),
        value: Some(value),
    };
    edits.push(set("Conversation", Value::resref(dialog)));
    // The shopkeeper keeps the store's tag, where the project's script
    // reads it.
    if let Some(name) = w.rules.as_ref().and_then(|r| r.keeper_variable.as_deref()) {
        let mut vars = creature.list("VarTable").unwrap_or(&[]).to_vec();
        let named = crate::text::encode(name);
        let mut var = Struct::new(0);
        var.set("Name", Value::String(named.clone()));
        var.set("Type", Value::Dword(3));
        var.set("Value", Value::String(crate::text::encode(&tag)));
        match vars.iter().position(|v| v.string("Name") == Some(&named[..])) {
            Some(i) => vars[i] = var,
            None => vars.push(var),
        }
        edits.push(set("VarTable", Value::List(vars)));
    }
    if w.hostile && w.use_faction {
        edits.push(set(faction_field(w.list), Value::Dword(w.faction)));
    }
    let (list, _) = git_list(ResType::UTM).expect("stores have a list");
    edits.push(Edit::insert_item(git, GffPath::root(), list, count, placed));
    // What it makes: the conversation and the script (by its source), and
    // the store placed.
    let mut resources = vec![ResKey::new(dialog, ResType::DLG)];
    if opens.is_none() {
        resources.push(nss);
    }
    let store = mg_plugin::Selected::Object {
        area: w.area,
        kind: mg_rules::ObjectKind::Store,
        index: count,
    };
    let made = crate::plugin_events::Made {
        by: "setup-store",
        resources,
        objects: vec![store],
        pasted: false,
    };
    Ok((edits, made))
}

/// The Add Popup Text window's fields.
#[derive(Debug, Clone, PartialEq)]
pub struct PopupText {
    pub area: ResRef,
    /// The placeable's index in the area's placeables.
    pub placeable: usize,
    pub text: String,
    /// The conversation's name.
    pub name: String,
}

pub(crate) fn popup_window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(mut p) = app.popup_text.take() else { return };
    let (mut ok, mut close) = (false, false);
    let name = ResRef::from_str(p.name.trim()).ok().filter(|r| *r != ResRef::EMPTY);
    let exists = name.is_some_and(|r| {
        app.ws.as_ref().is_some_and(|ws| ws.module.contains(&ResKey::new(r, ResType::DLG)))
    });
    egui::Window::new("Add Popup Text")
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label("Popup Text");
            let field = egui::TextEdit::multiline(&mut p.text).desired_rows(4);
            let field =
                crate::widgets::text_area(ui, egui::Id::new("popup-text"), field).response.response;
            crate::widgets::autofocus(ui, &field);
            ui.label("Conversation File");
            ui.add(egui::TextEdit::singleline(&mut p.name).char_limit(16));
            if exists {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    "A conversation of that name is replaced.",
                );
            }
            ui.separator();
            ui.horizontal(|ui| {
                let ready = name.is_some() && !p.text.trim().is_empty();
                ok = ui
                    .add_enabled(ready, egui::Button::new("OK"))
                    .on_hover_text("Accept changes")
                    .clicked()
                    || (ready && crate::widgets::enter(ui));
                close = crate::widgets::cancel_discard(ui);
            });
        });
    if ok && let Some(name) = name {
        let git = ResKey::new(p.area, ResType::GIT);
        let text = crate::text::with_loc_line_ends(&p.text, "");
        match store_setup::popup(&crate::text::for_windows_1252(&text)).to_bytes() {
            Ok(data) => {
                let edits = vec![
                    Edit::SetResource { key: ResKey::new(name, ResType::DLG), data: Some(data) },
                    Edit::SetField {
                        key: git,
                        path: GffPath::root().item("Placeable List", p.placeable),
                        label: "Conversation".into(),
                        value: Some(Value::resref(name)),
                    },
                ];
                app.actions.push(Action::Apply(Command::new("Add Popup Text", edits)));
            }
            Err(e) => app.log.error(format!("Add Popup Text: {e}")),
        }
        close = true;
    }
    if !close {
        app.popup_text = Some(p);
    }
}
