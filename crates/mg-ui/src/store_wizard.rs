//! Setup Store (a creature's or placeable's context menu in the area
//! viewer; Aurora's Store Setup Wizard): the shopkeeper's conversation, the
//! script that opens the store, the store to place where the shopkeeper
//! stands, and a friendlier faction for a hostile shopkeeper, as one
//! undoable command (`mg_module::store_setup`). And Add Popup Text (a
//! placeable's): a one-line conversation the placeable takes as its own.

use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit, GffPath};
use mg_gff::{Gff, Struct, Value};
use mg_module::factions::Factions;
use mg_module::instances::{Placing, git_list, instance};
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
}

fn factions(app: &mut Moonglow) -> Option<Factions> {
    let key = ResKey::parse("repute", ResType::FAC)?;
    let ws = app.ws.as_mut()?;
    ws.doc(&key).ok().map(|g| Factions::read(&Gff { root: g.root.clone(), ..Gff::new(*b"FAC ") }))
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
    let hostile = store_setup::hostile(factions.as_ref(), faction);
    let module = &ws.module;
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
        store: None,
        custom: false,
        filter: String::new(),
        hostile,
        use_faction: true,
        faction: store_setup::merchant(factions.as_ref()),
        listed: None,
    });
}

/// The store blueprints to choose from: the game's (Standard) or the
/// module's (Custom), as (resref, name).
fn stores(app: &Moonglow, custom: bool) -> Vec<(ResRef, String)> {
    let Some(game) = app.game.as_ref() else { return Vec::new() };
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
    if w.page == Page::Store && w.listed.as_ref().is_none_or(|(c, _)| *c != w.custom) {
        w.listed = Some((w.custom, stores(app, w.custom)));
    }
    let choices = w.listed.as_ref().map_or(&[][..], |(_, l)| l.as_slice()).to_vec();
    let faction_names: Vec<String> = factions(app)
        .map(|f| f.factions.iter().map(|x| x.name.clone()).collect())
        .unwrap_or_else(|| mg_module::factions::STANDARD.iter().map(|s| s.to_string()).collect());
    egui::Window::new("Store Setup Wizard").collapsible(false).resizable(false).show(ctx, |ui| {
        match w.page {
            Page::Conversation => {
                ui.heading("Conversation");
                ui.label("Enter the conversation text for the shopkeeper.");
                ui.label("What does the shopkeeper say when the conversation begins?");
                let field = ui.add(egui::TextEdit::multiline(&mut w.greeting).desired_rows(2));
                crate::widgets::autofocus(ui, &field);
                ui.label("What does the player say when the player wishes to see the store?");
                ui.text_edit_singleline(&mut w.yes);
                ui.label("What does the player say when the player is not interested?");
                ui.text_edit_singleline(&mut w.no);
                ui.checkbox(&mut w.appraise, "Use appraise checks");
                ui.label("(optional) Enter the filename for the shopkeeper's conversation");
                ui.add(egui::TextEdit::singleline(&mut w.dialog).char_limit(16));
                ui.label("(optional) Enter the filename for the script that will open the store");
                ui.add(egui::TextEdit::singleline(&mut w.script).char_limit(16));
            }
            Page::Store => {
                ui.heading("Choose Store");
                ui.label("Choose the store to use.");
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut w.custom, false, "Standard");
                    ui.selectable_value(&mut w.custom, true, "Custom");
                    ui.add(egui::TextEdit::singleline(&mut w.filter).hint_text("Find"));
                });
                let needle = w.filter.to_lowercase();
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    for (r, name) in &choices {
                        let label = format!("{name} ({r})");
                        if !label.to_lowercase().contains(&needle) {
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
                    egui::ComboBox::from_id_salt("store-faction").selected_text(current).show_ui(
                        ui,
                        |ui| {
                            for (i, name) in faction_names.iter().enumerate() {
                                ui.selectable_value(&mut w.faction, i as u32, name);
                            }
                        },
                    );
                }
            }
        }
        ui.separator();
        let names_ok = ResRef::from_str(w.dialog.trim()).is_ok_and(|r| r != ResRef::EMPTY)
            && ResRef::from_str(w.script.trim()).is_ok_and(|r| r != ResRef::EMPTY);
        ui.horizontal(|ui| {
            if ui.add_enabled(w.page != Page::Conversation, egui::Button::new("< Back")).clicked() {
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
            }
            if ui.add_enabled(w.page == Page::Finish, egui::Button::new("Finish")).clicked()
                || (w.page == Page::Finish && crate::widgets::enter(ui))
            {
                finish = true;
            }
            if ui.button("Cancel").clicked() {
                close = true;
            }
        });
    });
    if finish {
        match build(app, &w) {
            Ok(edits) => app.actions.push(Action::Apply(Command::new("Setup Store", edits))),
            Err(e) => app.log.error(format!("Setup Store: {e}")),
        }
        close = true;
    }
    if !close {
        app.store_wizard = Some(w);
    }
}

/// The edits: the conversation, the script (compiled), the shopkeeper's
/// conversation and faction, and the store placed where it stands.
fn build(app: &mut Moonglow, w: &StoreWizard) -> Result<Vec<Edit>, String> {
    let dialog = ResRef::from_str(w.dialog.trim()).map_err(|e| e.to_string())?;
    let script = ResRef::from_str(w.script.trim()).map_err(|e| e.to_string())?;
    let store = w.store.ok_or("no store chosen")?;
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
    let game = app.game.as_ref().ok_or("no game data")?;
    let ws = app.ws.as_ref().ok_or("no module")?;
    let read = |k: ResKey| {
        let data = ws
            .module
            .get(&k)
            .map(<[u8]>::to_vec)
            .or_else(|| game.resman.get(&k).ok().map(|d| d.into_owned()))?;
        Gff::read(&data).ok().map(|g| g.root)
    };
    let blueprint =
        read(ResKey::new(store, ResType::UTM)).ok_or("the store blueprint is missing")?;
    let item = |r: ResRef| read(ResKey::new(r, ResType::UTI));
    let placing = Placing { game, item: &item };
    let at = store_setup::store_placement(&creature);
    let placed = instance(&placing, ResType::UTM, &blueprint, at, &[]).ok_or("not a store")?;
    let tag =
        placed.string("Tag").map(|t| String::from_utf8_lossy(t).into_owned()).unwrap_or_default();

    let text = store_setup::script(&tag, w.appraise);
    let nss = ResKey::new(script, ResType::NSS);
    let mut edits = vec![
        Edit::SetResource {
            key: ResKey::new(dialog, ResType::DLG),
            data: Some(
                store_setup::conversation(&w.greeting, &w.yes, &w.no, script)
                    .to_bytes()
                    .map_err(|e| e.to_string())?,
            ),
        },
        Edit::SetResource { key: nss, data: Some(crate::text::encode(&text)) },
    ];
    match crate::script_view::compile_source(app, nss, &text) {
        Ok(ncs) => edits
            .push(Edit::SetResource { key: ResKey::new(script, ResType::NCS), data: Some(ncs) }),
        Err(e) => app.log.warn(format!("Setup Store: {script}: {}", e.message)),
    }
    let path = GffPath::root().item(w.list, w.creature);
    let set = |label: &str, value: Value| Edit::SetField {
        key: git,
        path: path.clone(),
        label: label.into(),
        value: Some(value),
    };
    edits.push(set("Conversation", Value::resref(dialog)));
    if w.hostile && w.use_faction {
        edits.push(set(faction_field(w.list), Value::Dword(w.faction)));
    }
    let (list, _) = git_list(ResType::UTM).expect("stores have a list");
    edits.push(Edit::InsertItem {
        key: git,
        path: GffPath::root(),
        list: list.into(),
        index: count,
        item: placed,
    });
    Ok(edits)
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
    egui::Window::new("Add Popup Text").collapsible(false).resizable(false).show(ctx, |ui| {
        ui.label("Popup Text");
        let field = ui.add(egui::TextEdit::multiline(&mut p.text).desired_rows(4));
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
            close = ui.button("Cancel").on_hover_text("Discard changes").clicked();
        });
    });
    if ok && let Some(name) = name {
        let git = ResKey::new(p.area, ResType::GIT);
        match store_setup::popup(&p.text).to_bytes() {
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
