//! Copy…: a resource of the module, or one of the game's, copied into the
//! module under a ResRef that is asked for, and for a blueprint or an area
//! a Tag (the palette's Edit Copy, and Copy… in the module tree). An area
//! is copied with what is placed in it and listed in the module.

use mg_core::{ResRef, ResType};
use mg_edit::{Command, Edit};
use mg_gff::{Gff, Struct, Value};
use mg_module::palette::BlueprintKind;
use mg_resman::ResKey;

use crate::text::{decode, encode};
use crate::widgets::{autofocus, enter};
use crate::{Action, Moonglow, Tab};

/// What the Copy window asks.
#[derive(Debug, Clone, PartialEq)]
pub struct CopyDraft {
    pub from: ResKey,
    pub resref: String,
    /// The copy's tag, where the resource has one.
    pub tag: Option<String>,
    /// From the palette: the copy is shown there and opened.
    pub palette: bool,
}

/// The resources that go with one: an area's objects and comments.
pub(crate) fn family(t: ResType) -> Vec<ResType> {
    match t {
        ResType::ARE => vec![ResType::ARE, ResType::GIT, ResType::GIC],
        // (Its compiled script is the same under any name; its debug
        // information names the file, and is made again by a compile.)
        ResType::NSS => vec![ResType::NSS, ResType::NCS],
        t => vec![t],
    }
}

/// The field holding a resource's own resref, where it has one.
fn resref_field(t: ResType) -> Option<&'static str> {
    match t {
        ResType::ARE => Some("ResRef"),
        t => BlueprintKind::from_restype(t).map(BlueprintKind::resref_field),
    }
}

impl Moonglow {
    /// A resource's bytes: the module's as it is now, else the game's.
    fn copy_source(&mut self, key: ResKey) -> Option<Vec<u8>> {
        let ws = self.ws.as_mut()?;
        ws.flush().ok()?;
        let own = ws.module.get(&key).map(<[u8]>::to_vec);
        own.or_else(|| Some(self.game.as_deref()?.resman.get(&key).ok()?.into_owned()))
    }

    /// Whether the module has a resource `name` of the family of `t`.
    fn copy_taken(&self, name: &ResRef, t: ResType) -> bool {
        let ws = self.ws.as_ref();
        ws.is_some_and(|ws| family(t).iter().any(|t| ws.module.contains(&ResKey::new(*name, *t))))
    }

    /// Opens the Copy window for a resource: a free ResRef and its tag, to
    /// change before the copy is made.
    pub(crate) fn copy_dialog(&mut self, key: ResKey, palette: bool) {
        let Some(data) = self.copy_source(key) else {
            self.log.warn(format!("{key} is not there to copy"));
            return;
        };
        let in_game = |k: &ResKey| self.game.as_deref().is_some_and(|g| g.resman.contains(k));
        let taken = |name: &str| {
            ResKey::parse(name, key.restype)
                .is_none_or(|k| self.copy_taken(&k.resref, key.restype) || in_game(&k))
        };
        // (By the builder's code, in a project with one.)
        let naming = self.ws.as_ref().map(|ws| ws.module.project_file.naming()).unwrap_or_default();
        let from = key.resref.to_string();
        let resref = crate::palette_view::copy_resref_named(&from, &naming, &taken);
        let tag = resref_field(key.restype)
            .and_then(|_| Gff::read(&data).ok())
            .and_then(|g| g.root.string("Tag").map(decode));
        self.copy_as = Some(CopyDraft {
            from: key,
            resref: resref.map(|r| r.to_string()).unwrap_or_default(),
            tag,
            palette,
        });
    }

    /// Makes the copy (one command); its key.
    pub(crate) fn copy_resource(&mut self, draft: &CopyDraft) -> Option<ResKey> {
        let to = ResRef::from_str(draft.resref.trim()).ok()?;
        let new = ResKey::new(to, draft.from.restype);
        let mut edits = Vec::new();
        for t in family(draft.from.restype) {
            let Some(mut data) = self.copy_source(ResKey::new(draft.from.resref, t)) else {
                continue;
            };
            if t == draft.from.restype
                && let Some(field) = resref_field(t)
                && let Ok(mut gff) = Gff::read(&data)
            {
                gff.root.set(field, Value::resref(to));
                if let Some(tag) = &draft.tag {
                    gff.root.set("Tag", Value::String(encode(tag.trim())));
                }
                data = gff.to_bytes().ok()?;
            }
            edits.push(Edit::SetResource { key: ResKey::new(to, t), data: Some(data) });
        }
        if edits.is_empty() {
            return None;
        }
        // An area is one of the module's once the module lists it.
        let ifo = crate::module_props::info_key();
        if draft.from.restype == ResType::ARE
            && let Some(ws) = self.ws.as_mut()
            && let Ok(info) = ws.doc(&ifo)
        {
            let mut item = Struct::new(6);
            item.set("Area_Name", Value::resref(to));
            edits.push(Edit::insert_item(
                ifo,
                mg_edit::GffPath::root(),
                "Mod_Area_list",
                info.root.list("Mod_Area_list").map_or(0, <[Struct]>::len),
                item,
            ));
        }
        // (With what the plugins do on what is made, in the same command.)
        let made = crate::plugin_events::Made::resources("copy", vec![new]);
        let cmd = Command::new(format!("Copy {} as {new}", draft.from), edits);
        let cmd = crate::plugin_events::with_plugins(self, cmd, &made);
        match self.apply(cmd) {
            Ok(()) => {
                self.log.info(format!("Copied {} as {new}", draft.from));
                // An area's copy has its objects' tags, and perhaps its own.
                if new.restype == ResType::ARE {
                    let git = ResKey::new(to, ResType::GIT);
                    let mut made =
                        vec![crate::tag_clash::Made { key: new, path: mg_edit::GffPath::root() }];
                    if let Some(ws) = self.ws.as_mut()
                        && let Ok(doc) = ws.doc(&git)
                    {
                        for kind in mg_area::ObjectKind::ALL {
                            let n = doc.root.list(kind.list()).map_or(0, <[Struct]>::len);
                            made.extend((0..n).map(|i| crate::tag_clash::Made {
                                key: git,
                                path: mg_edit::GffPath::root().item(kind.list(), i),
                            }));
                        }
                    }
                    let what = format!("The copy of {}", draft.from.resref);
                    crate::tag_clash::check(self, &what, made, true);
                }
                Some(new)
            }
            Err(e) => {
                self.log.error(e.to_string());
                None
            }
        }
    }
}

/// The Copy window.
pub(crate) fn window(app: &mut Moonglow, ctx: &egui::Context) {
    let Some(mut draft) = app.copy_as.take() else { return };
    let title = format!("Copy {}", draft.from);
    let mut open = true;
    let (mut done, mut cancel) = (false, false);
    crate::widgets::dialog(ctx, &title, &mut open).collapsible(false).resizable(false).show(
        ctx,
        |ui| {
            egui::Grid::new("copy-as").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                crate::widgets::field_label(ui, "ResRef");
                let r = ui.add(egui::TextEdit::singleline(&mut draft.resref).desired_width(200.0));
                autofocus(ui, &r);
                ui.end_row();
                if let Some(tag) = &mut draft.tag {
                    crate::widgets::field_label(ui, "Tag");
                    ui.add(egui::TextEdit::singleline(tag).desired_width(200.0));
                    ui.end_row();
                }
            });
            let name = draft.resref.trim();
            let parsed = ResRef::from_str(name);
            // (message, whether it stops the copy)
            let problem = match &parsed {
                _ if name.is_empty() => Some(("Type the copy's ResRef".to_string(), true)),
                Err(e) => Some((e.to_string(), true)),
                Ok(r) if app.copy_taken(r, draft.from.restype) => {
                    Some((format!("The module has a {name} already"), true))
                }
                Ok(r) => {
                    let key = ResKey::new(*r, draft.from.restype);
                    let game = app.game.as_deref().is_some_and(|g| g.resman.contains(&key));
                    game.then(|| (format!("The game has a {key}: this one replaces it"), false))
                }
            };
            let stops = problem.as_ref().is_some_and(|p| p.1);
            if let Some((p, _)) = &problem {
                ui.colored_label(ui.visuals().warn_fg_color, p);
            }
            if draft.from.restype == ResType::ARE {
                ui.weak("The area is copied with everything placed in it.");
            }
            ui.horizontal(|ui| {
                let ok = ui.add_enabled(!stops, egui::Button::new("Create Copy"));
                if ok.clicked() || (!stops && enter(ui)) {
                    done = true;
                }
                if crate::widgets::cancel(ui) {
                    cancel = true;
                }
            });
        },
    );
    if done {
        if let Some(new) = app.copy_resource(&draft) {
            if draft.palette {
                app.palette.custom = true;
                app.palette.selected = Some(new);
            }
            // A blueprint's copy is opened, to change.
            if BlueprintKind::from_restype(new.restype).is_some()
                && let Some(t) = Tab::for_resource(new)
            {
                app.actions.push(Action::OpenTab(t));
            }
        }
    } else if open && !cancel {
        app.copy_as = Some(draft);
    }
}
