//! Openers for the dialogs of the "module" group (see `dialog_pattern.rs`):
//! the dialogs about the module as a whole (Build Module, Delete from
//! Module, Rename, Duplicate Tags, Recover Unsaved Work, Export and Import
//! Resources, Claims, About, Updates...). (Find and Replace Text, the Claims
//! Table and Hak Pak Conflict Analysis are the Bottom Dock's since M4's
//! U28.)

use super::Opener;
use super::openers_core::{clear_field, click_reached};
use crate::common::*;

type H = Harness<'static, Moonglow>;

/// The sample module open, with no game.
fn module() -> Option<H> {
    let dir = mg_testkit::scratch_dir(&super::scratch("u27-module"));
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&sample_module(&dir));
    Some(harness(app, 1280.0, 720.0))
}

/// The module with a game of its own, empty: an install in the scratch
/// folder and no resources (what Publish and the haks' report ask for).
fn module_with_game() -> Option<H> {
    let dir = mg_testkit::scratch_dir(&super::scratch("u27-module-game"));
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    let (game, user) = (dir.join("game"), dir.join("user"));
    for d in [&game, &user] {
        std::fs::create_dir_all(d).unwrap();
    }
    let tlk = mg_tlk::Tlk::new(mg_core::Language::ENGLISH);
    // (The haks' walk reads the talk table: an empty one.)
    let lang = game.join("lang/en/data");
    std::fs::create_dir_all(&lang).unwrap();
    std::fs::write(lang.join("dialog.tlk"), tlk.to_bytes().unwrap()).unwrap();
    app.install = Some(mg_resman::GameInstall::new(game, Some(user), "en"));
    let game = mg_rules::GameData::new(mg_resman::ResMan::new(), tlk);
    app.game = Some(std::sync::Arc::new(game));
    app.open_module(&sample_module(&dir));
    Some(harness(app, 1280.0, 720.0))
}

/// A module with an area open (the game's).
fn area() -> Option<H> {
    area_ui_harness(&super::scratch("u27-module-area")).map(|(h, _)| h)
}

fn script() -> ResKey {
    ResKey::parse("hello", ResType::NSS).unwrap()
}

/// The sample module as a project folder (no claims).
fn project() -> Option<H> {
    let dir = project_folder(&super::scratch("u27-module-project"), &[]);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&dir);
    Some(harness(app, 1280.0, 720.0))
}

/// A project whose claims table gives `start` to `holder` with `status`;
/// the builder is `ab`.
fn claimed(holder: &str, status: &str) -> Option<H> {
    let (_dir, mut app) = claimed_project(
        &super::scratch("u27-module-claims"),
        "[claims]\nfile = \"claims.2da\"\n",
        &claims_table(holder, status),
    );
    app.settings.no_last_area = true;
    Some(harness(app, 1280.0, 720.0))
}

fn claimed_other() -> Option<H> {
    claimed("cd", "claimed")
}

fn claimed_merged() -> Option<H> {
    claimed("ab", "merged")
}

/// A module in a project folder (its files are its source) and a script of
/// it with unsaved changes, for the files changed outside.
fn outside_project() -> Option<H> {
    let scratch = mg_testkit::scratch_dir(&super::scratch("u27-module-outside"));
    let mut m = mg_module::Module::open(&sample_module(&scratch)).unwrap();
    let dir = scratch.join("project");
    m.save_as(&mg_module::ModuleLocation::Project { root: dir.clone(), target: "default".into() })
        .unwrap();
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&dir);
    Some(harness(app, 1280.0, 720.0))
}

/// A recovery copy left by a session that did not end, found at the start.
fn recovery() -> Option<H> {
    let dir = mg_testkit::scratch_dir(&super::scratch("u27-module-recovery"));
    let recovery = dir.join("recovery");
    // (Two modules' copies, as the board shows.)
    for name in ["first", "second"] {
        let path = dir.join(format!("{name}.mod"));
        std::fs::rename(sample_module(&dir), &path).unwrap();
        let mut app = app_with(Vec::new());
        app.recovery_dir = Some(recovery.clone());
        app.open_module(&path);
        let edit = mg_edit::Edit::SetField {
            key: ResKey::parse("module", ResType::IFO).unwrap(),
            path: mg_edit::GffPath::root(),
            label: "Mod_Tag".into(),
            value: Some(mg_gff::Value::String(b"RECOVERED".to_vec())),
        };
        app.actions.push(mg_ui::Action::Apply(mg_edit::Command::new("tag", vec![edit])));
        let mut h = window_harness(app);
        h.run();
        h.state_mut().write_recovery().unwrap();
    }
    let mut app = app_with(Vec::new());
    app.recovery_dir = Some(recovery);
    find_recoveries_let_go(&mut app);
    Some(harness(app, 1280.0, 720.0))
}

/// The sample module open, and an archive there for Export to write over.
fn module_with_archive() -> Option<H> {
    let dir = mg_testkit::scratch_dir(&super::scratch("u27-module-export"));
    let erf = dir.join("kept.erf");
    let mut old = mg_erf::ErfWriter::new(*b"ERF ");
    old.add(script().resref, script().restype, b"void main() {}\n".to_vec()).unwrap();
    std::fs::write(&erf, old.to_bytes().unwrap()).unwrap();
    let mut app =
        Moonglow::new(None, Box::new(NoDialogs { save: vec![erf; 8], ..Default::default() }));
    app.settings.no_last_area = true;
    app.open_module(&sample_module(&dir));
    Some(harness(app, 1280.0, 720.0))
}

/// The sample module open, and an archive for Import to read (whose script
/// the module has already).
fn module_with_import() -> Option<H> {
    let dir = mg_testkit::scratch_dir(&super::scratch("u27-module-import"));
    let erf = dir.join("many.erf");
    let mut w = mg_erf::ErfWriter::new(*b"ERF ");
    w.add(script().resref, script().restype, b"// other\nvoid main() {}\n".to_vec()).unwrap();
    let fresh = ResKey::parse("mg_fresh", ResType::NSS).unwrap();
    w.add(fresh.resref, fresh.restype, b"void main() {}\n".to_vec()).unwrap();
    std::fs::write(&erf, w.to_bytes().unwrap()).unwrap();
    let mut app =
        Moonglow::new(None, Box::new(NoDialogs { open: vec![erf; 8], ..Default::default() }));
    app.settings.no_last_area = true;
    app.open_module(&sample_module(&dir));
    Some(harness(app, 1280.0, 720.0))
}

fn menu_item(h: &mut H, menu_name: &str, item: &str) {
    menu(h, menu_name).click();
    h.run();
    click_reached(h, item);
    h.run();
}

fn retag_start(h: &mut H) {
    let key = ResKey::parse("start", ResType::ARE).unwrap();
    let edit = mg_edit::Edit::SetField {
        key,
        path: mg_edit::GffPath::root(),
        label: "Tag".into(),
        value: Some(mg_gff::Value::String(b"START_2".to_vec())),
    };
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new("Tag", vec![edit])));
    h.run();
    h.run();
}

fn save_the_claimed(h: &mut H) {
    h.state_mut().actions.push(common_rename_start());
    h.run();
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
}

fn common_rename_start() -> mg_ui::Action {
    rename_start("Market")
}

/// Another program changes a script that has unsaved changes here. (A
/// script of its own each time: the files left for Later are not asked
/// about again until others are named.)
fn change_outside(h: &mut H) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NTH: AtomicUsize = AtomicUsize::new(0);
    let name = format!("mg_out_{}", NTH.fetch_add(1, Ordering::Relaxed));
    let key = ResKey::parse(&name, ResType::NSS).unwrap();
    let set = |h: &mut H, text: &str| {
        let edit = mg_edit::Edit::SetResource { key, data: Some(text.as_bytes().to_vec()) };
        h.state_mut()
            .actions
            .push(mg_ui::Action::Apply(mg_edit::Command::new("Script", vec![edit])));
        h.run_steps(2);
    };
    set(h, "void main() {}\n");
    h.state_mut().actions.push(mg_ui::Action::Save);
    h.run_steps(3);
    set(h, "// mine\nvoid main() {}\n");
    let location = h.state().ws.as_ref().unwrap().module.location.clone();
    let Some(mg_module::ModuleLocation::Project { root, .. }) = location else {
        panic!("a project");
    };
    std::fs::write(root.join(format!("src/{name}.nss")), "void main() { int outside; }\n").unwrap();
    h.state_mut().reload_project_files();
    h.run_steps(2);
}

fn duplicate_tags(h: &mut H) {
    h.state_mut().settings.warn_duplicate_tags = true;
    h.state_mut().ws.as_mut().unwrap().module.project_file.tags.unique =
        mg_module::project_file::Patterns::new(&["NW_*"]).unwrap();
    let area = h.state().ws.as_ref().unwrap().module.areas().unwrap()[0];
    h.state_mut().actions.push(mg_ui::Action::CopyDialog(ResKey::new(area, ResType::ARE)));
    h.run_steps(3);
    // (Opened again in the same window: another name each time.)
    let nth = h.state().ws.as_ref().unwrap().module.areas().unwrap().len();
    h.state_mut().copy_as.as_mut().unwrap().resref = format!("field_{nth}");
    h.run_steps(2);
    h.get_by_label("Create Copy").click();
    h.run_steps(3);
}

fn attach(h: &mut H) {
    use mg_module::attach::{Placement, There};
    let place = |name: &str, is_tlk: bool, there| Placement {
        from: format!("/somewhere/{name}").into(),
        entry: None,
        to: format!("/user/{name}").into(),
        there,
        name: name.split('.').next().unwrap().to_string(),
        is_tlk,
    };
    let placements = vec![
        place("one.hak", false, There::Nothing),
        place("two.hak", false, There::Different),
        place("words.tlk", true, There::Same),
    ];
    h.state_mut().attach = Some(mg_ui::module_props::AttachDraft { placements, replace: false });
}

pub(crate) fn openers() -> Vec<Opener> {
    vec![
        Opener::new("build_view.rs:window#1", "Build Module", module, |h| {
            h.state_mut().build = Some(Default::default())
        })
        // (M4's U28: what it finds is in Problems, nothing found the log's.)
        .enter_does("builds the module and says so, in Problems or the log", |h| {
            h.state().problems.of(&mg_ui::problems::Source::Build).is_some()
                || h.state().log.entries.iter().any(|(_, m)| m == "No errors found")
        }),
        Opener::new("builder_code.rs:window#1", "Builder Code", project, |h| {
            menu_item(h, "Edit", "Builder Code…")
        })
        .field(0),
        Opener::new("bulk.rs:update_window#1", "Update Instances", area, |h| {
            let waypoint = ResKey::parse("nw_waypoint001", ResType::UTW).unwrap();
            h.state_mut().update_instances_of(vec![waypoint]);
        }),
        Opener::new("claims.rs:protected_window#1", "Protected Area", claimed_merged, retag_start),
        Opener::new("claims.rs:save_window#1", "Areas Not Yours", claimed_other, save_the_claimed),
        Opener::new("copy_as.rs:window#1", "Copy hello.nss", module, |h| {
            h.state_mut().actions.push(mg_ui::Action::CopyDialog(script()))
        })
        .field(0)
        .off(|h| clear_field(h, "Copy hello.nss", 0)),
        Opener::new("references.rs:delete_window#1", "Delete from Module", module, |h| {
            h.state_mut().actions.push(mg_ui::Action::DeleteDialog(script()))
        }),
        Opener::new("references.rs:rename_window#1", "Rename hello.nss", module, |h| {
            h.state_mut().actions.push(mg_ui::Action::RenameDialog(script()))
        })
        .field(0),
        Opener::new("tag_clash.rs:window#1", "Duplicate Tags", area, duplicate_tags),
        // (Later leaves the copies kept: they are found again, as at the
        // next start.)
        Opener::new("recovery.rs:window#1", "Recover Unsaved Work", recovery, |h| {
            if h.state().recoveries.is_empty() {
                h.state_mut().find_recoveries();
            }
        }),
        Opener::new(
            "outside.rs:window#1",
            "Changed Outside Moonglow",
            outside_project,
            change_outside,
        ),
        Opener::new("module_folder.rs:ui#1", "Open Directory as a Module", module, |h| {
            h.state_mut().module_folder = Some("/somewhere/sample.mod".into())
        }),
        Opener::new("module_props.rs:attach_window#1", "Add Haks and Talk Table", module, attach),
        Opener::new("transfer.rs:ask_ui#1", "Export", module_with_archive, |h| {
            h.state_mut().actions.push(mg_ui::Action::ExportDialog(vec![script()]));
            settle(h);
            h.get_by_label("Export 1…").click();
        }),
        Opener::new("transfer.rs:ui#1", "Export Resources", module, |h| {
            h.state_mut().actions.push(mg_ui::Action::ExportDialog(vec![script()]))
        })
        .field(0),
        Opener::new("transfer.rs:ui#2", "Import Resources", module_with_import, |h| {
            h.state_mut().actions.push(mg_ui::Action::ImportDialog)
        }),
        Opener::new("nwsync_view.rs:window#1", "Publish to NWSync", module_with_game, |h| {
            h.state_mut().publish = Some(Default::default())
        })
        // (No repository folder chosen: publishing stops at asking for one.)
        .enter_does("publishes, here asking for the folder, in it", |h| {
            let error = h.state().publish.as_ref().and_then(|p| p.error.as_deref());
            super::is_shown(h, "Publish to NWSync")
                && error == Some("Choose the repository folder.")
        }),
        Opener::new("updates.rs:window#1", "Check for Updates", module, |h| {
            let updates = &mut h.state_mut().updates;
            updates.window = true;
            updates.report = Some(mg_ui::updates::Outcome::Newest);
        })
        .enter_does("checks again and says so, in it", |h| {
            super::shows_other_words(h, "Check for Updates")
        }),
        Opener::new("manual.rs:about_window#1", "About Moonglow Toolset", module, |h| {
            h.state_mut().about = true
        })
        .bare(),
    ]
}
