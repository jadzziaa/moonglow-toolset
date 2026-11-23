//! Openers for the dialogs of the "shared" group (see `dialog_pattern.rs`):
//! the shared makers of `widgets.rs` (String Edit, Variables, Select
//! Resource, the name window, Files Already, the sets window), the plugin
//! windows, a plugin's held save, the job window, the talk table's question
//! and the project commands' windows.

use super::openers_core::{clear_field, click_reached};
use super::{Opener, is_shown};
use crate::common::*;
use mg_plugin::{Answer, FieldKind, FormField, Question};

/// The sample module open, no game.
fn module() -> Option<Harness<'static, Moonglow>> {
    let dir = mg_testkit::scratch_dir(&super::scratch("u27-shared-module"));
    let path = sample_module(&dir);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&path);
    Some(harness(app, 1280.0, 720.0))
}

/// A window with no module.
fn plain() -> Option<Harness<'static, Moonglow>> {
    Some(harness(app_with(Vec::new()), 1280.0, 720.0))
}

// -- widgets.rs ------------------------------------------------------------

/// A hak being made, which has `mg_one.2da`; the next Add Files… offers it
/// again with another.
fn hak_with_a_file() -> Option<Harness<'static, Moonglow>> {
    let dir = mg_testkit::scratch_dir(&super::scratch("u27-shared-hak"));
    let table = |text: &str| format!("2DA V2.0\n\n   Label\n0  {text}\n");
    let (old, new) = (dir.join("old"), dir.join("new"));
    for d in [&old, &new] {
        std::fs::create_dir_all(d).unwrap();
    }
    std::fs::write(old.join("mg_one.2da"), table("old")).unwrap();
    std::fs::write(new.join("mg_one.2da"), table("new one")).unwrap();
    std::fs::write(new.join("mg_two.2da"), table("two")).unwrap();
    let chosen = vec![new.join("mg_one.2da"), new.join("mg_two.2da")];
    // (Answered from the end: the old file first, then the two, as often
    // as the dialog is opened.)
    let mut open_many = vec![chosen; 40];
    open_many.push(vec![old.join("mg_one.2da")]);
    let dialogs = NoDialogs { open_many, ..Default::default() };
    let mut h = harness(Moonglow::new(None, Box::new(dialogs)), 1280.0, 720.0);
    mg_ui::hak_view::new_hak(h.state_mut());
    settle(&mut h);
    add_files(&mut h);
    Some(h)
}

/// Add Files…, in the header or (a narrow window folds it) in its More.
fn add_files(h: &mut Harness<'static, Moonglow>) {
    if h.query_by_label("Add Files…").is_none() {
        h.get_by_label(mg_uikit::header::MORE).click();
        settle(h);
    }
    h.get_by_label("Add Files…").click();
    settle(h);
}

fn info_target(label: &str) -> mg_ui::widgets::FieldTarget {
    mg_ui::widgets::FieldTarget::new(Module::info_key(), mg_edit::GffPath::root(), label)
}

fn variables() -> mg_ui::widgets::VarTableEdit {
    mg_ui::widgets::VarTableEdit::new(info_target("VarTable"), &[])
}

/// A sound blueprint of the module, open, whose Add Sounds… opens a picker.
fn sound_blueprint() -> Option<Harness<'static, Moonglow>> {
    let mut h = module()?;
    let key = ResKey::parse("u27_sound", ResType::UTS).unwrap();
    let data = Gff::new(*b"UTS ").to_bytes().unwrap();
    let make = mg_edit::Command::new(
        "A sound",
        vec![mg_edit::Edit::SetResource { key, data: Some(data) }],
    );
    h.state_mut().actions.push(mg_ui::Action::Apply(make));
    h.state_mut().actions.push(mg_ui::Action::OpenTab(Tab::Blueprint(key)));
    settle(&mut h);
    Some(h)
}

// -- plugins ---------------------------------------------------------------

/// The sample project with the example Builder tags plugin in its folder,
/// opened (its plugin is asked about).
fn plugin_project(name: &str, examples: &[&str]) -> std::path::PathBuf {
    let dir = mg_testkit::scratch_dir(name);
    let _ = std::fs::remove_file(dir.with_extension("answers.toml"));
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    m.save_as(&ModuleLocation::Folder(dir.clone())).unwrap();
    std::fs::write(
        dir.join("moonglow.toml"),
        "# The team's.\n[plugins.\"example.builder-tags\"]\nprefix = \"KRZ_\"\n",
    )
    .unwrap();
    for name in examples {
        let example = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/plugins/examples")
            .join(name);
        let into = dir.join("plugins").join(name);
        std::fs::create_dir_all(&into).unwrap();
        for file in ["plugin.cfg", "main.luau"] {
            std::fs::copy(example.join(file), into.join(file)).unwrap();
        }
    }
    dir
}

fn project_with_plugins() -> Option<Harness<'static, Moonglow>> {
    project_with(plugin_project(&super::scratch("u27-shared-plugin-project"), &["builder-tags"]))
}

/// The same with two plugins of its own: the question offers Allow All.
fn project_with_two_plugins() -> Option<Harness<'static, Moonglow>> {
    project_with(plugin_project(
        &super::scratch("u27-shared-plugin-project-two"),
        &["builder-tags", "hello"],
    ))
}

fn project_with(dir: std::path::PathBuf) -> Option<Harness<'static, Moonglow>> {
    let mut app = app_with(Vec::new());
    app.plugin_dir = Some(plugin_fixtures());
    app.plugin_answers = Some(dir.with_extension("answers.toml"));
    app.load_plugins();
    app.open_module(&dir);
    app.palette_dock = false;
    Some(harness(app, 1280.0, 720.0))
}

/// [`project_with_plugins`] with its plugin allowed, so that its settings
/// can be opened.
fn project_with_allowed_plugin() -> Option<Harness<'static, Moonglow>> {
    let mut h = project_with_plugins()?;
    settle(&mut h);
    h.state_mut().allow_project_plugin("example.builder-tags", true);
    settle(&mut h);
    Some(h)
}

fn plugins_installed() -> Option<Harness<'static, Moonglow>> {
    let mut app = app_with(Vec::new());
    app.plugin_dir = Some(plugin_fixtures());
    app.load_plugins();
    Some(harness(app, 1280.0, 720.0))
}

/// The sample module with the example House rules plugin on, which warns
/// about a save of an area whose tag is not upper case.
fn house_rules() -> Option<Harness<'static, Moonglow>> {
    let dir = mg_testkit::scratch_dir(&super::scratch("u27-shared-house-rules"));
    let mut m = Module::new();
    let mut info = Gff::new(*b"IFO ");
    let mut a = ifo::MOD_AREA_LIST.new_item();
    a.write(&ifo::mod_area_list::AREA_NAME, ResRef::from_str("tavern").unwrap());
    info.root.items_mut(&ifo::MOD_AREA_LIST).push(a);
    m.set_info(&info).unwrap();
    let mut are = Gff::new(*b"ARE ");
    are.root.set("Tag", mg_gff::Value::String(b"TAVERN".to_vec()));
    m.set(ResKey::parse("tavern", ResType::ARE).unwrap(), are.to_bytes().unwrap());
    let path = dir.join("rules.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&path);
    app.palette_dock = false;
    app.plugin_dir =
        Some(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/plugins/examples"));
    app.load_plugins();
    app.enable_plugin("example.house-rules", true);
    Some(harness(app, 1280.0, 720.0))
}

/// A save the House rules plugin holds for a warning: an area whose tag is
/// not upper case.
fn save_with_a_warning(h: &mut Harness<'static, Moonglow>) {
    let mut are = Gff::new(*b"ARE ");
    are.root.set("Tag", mg_gff::Value::String(b"Cellar".to_vec()));
    let key = ResKey::parse("cellar", ResType::ARE).unwrap();
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "A cellar",
        vec![mg_edit::Edit::SetResource { key, data: Some(are.to_bytes().unwrap()) }],
    )));
    settle(h);
    h.state_mut().actions.push(mg_ui::Action::Save);
    settle(h);
}

/// An archive of the fixture Tag conventions plugin, and the question of
/// replacing the one installed.
fn replacing(h: &mut Harness<'static, Moonglow>) {
    let dir = mg_testkit::scratch_dir(&super::scratch("u27-shared-replace"));
    let work = dir.join("tag-conventions");
    std::fs::create_dir_all(&work).unwrap();
    for file in ["plugin.cfg", "main.luau", "rules.luau"] {
        std::fs::copy(plugin_fixtures().join("tag-conventions").join(file), work.join(file))
            .unwrap();
    }
    let plugin = mg_plugin::Plugin::load(&work).unwrap();
    let package = mg_plugin::Package::read(&mg_plugin::pack(&plugin).unwrap()).unwrap();
    h.state_mut().plugins.replace = Some(mg_ui::plugins::Replace {
        package,
        file: "tag-conventions-1.1.0.zip".into(),
        installed: "1.0.0".into(),
    });
}

// -- the job window --------------------------------------------------------

/// A job that runs until it is called off.
fn a_job_under_way(h: &mut Harness<'static, Moonglow>) {
    h.state_mut().start_job_for_tests("Held", |progress| {
        progress.step(1, 3);
        let started = std::time::Instant::now();
        while !progress.cancelled() && started.elapsed().as_secs() < 30 {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    });
}

// -- a plugin's questions --------------------------------------------------

/// The heading of a plugin's question asked by `ask_for_tests`.
const QUESTION: &str = "Plugin Question";

thread_local! {
    /// What the question asked last on this thread was answered with.
    static ANSWERED: std::cell::RefCell<Option<mg_ui::plugins::Answered>> =
        const { std::cell::RefCell::new(None) };
}

/// Asks `question` in the job's window, as a plugin's thread asks it.
pub(super) fn ask(h: &mut Harness<'static, Moonglow>, question: Question) {
    ask_late(h, question, std::time::Duration::ZERO, std::time::Duration::ZERO);
}

/// [`ask`], the job's thread asking `late` after it starts and ending
/// `ends_late` after it is answered; returns once the question waits in
/// the window. (The job's window is there, still,
/// before its thread asks: `rested` alone took it for the question's on
/// Windows CI, where the thread asked later, with no buttons or fields.)
fn ask_late(
    h: &mut Harness<'static, Moonglow>,
    question: Question,
    late: std::time::Duration,
    ends_late: std::time::Duration,
) {
    let answered = h.state_mut().ask_for_tests_late(question, late, ends_late);
    ANSWERED.with(|a| *a.borrow_mut() = Some(answered));
    let started = std::time::Instant::now();
    while !h.state().question_waiting() {
        assert!(started.elapsed().as_secs() < 20, "the plugin's question never came");
        h.step();
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// A question asked late by its thread (as Windows CI's was) is the
/// question's window once the opener returns and the window rests: its
/// field and its main action there.
#[test]
fn a_question_asked_late_is_waited_for() {
    super::running("-late-question");
    let mut h = module().unwrap();
    settle(&mut h);
    ask_late(&mut h, form(), std::time::Duration::from_millis(300), std::time::Duration::ZERO);
    super::rested(&mut h, QUESTION, true);
    assert_eq!(super::fields_in(&h, QUESTION).len(), 1, "the form's field");
    let all = mg_ui::widgets::footers_shown(&h.ctx);
    let seen = all.iter().find(|f| f.title == QUESTION).unwrap_or_else(|| panic!("{all:#?}"));
    assert!(seen.buttons.iter().any(|b| b.main), "a main action: {:?}", seen.buttons);
    h.get_all_by_label("Cancel").last().unwrap().click();
    settle(&mut h);
}

/// A question whose thread ends late after it is answered (as Windows
/// CI's once did: Plugin Question still shown after its Escape) is seen
/// closed by Escape, and answered and closed by Enter: the dialog
/// pattern's checks wait for a job's window to go, not for it to be still.
#[test]
fn a_question_ending_late_is_waited_for() {
    super::running("-late-end");
    let ends_late = std::time::Duration::from_millis(300);
    let op = Opener::new("jobs.rs:window#1 Confirm", QUESTION, module, |_| {})
        .job()
        .enter_does("answered yes, and closed", |h| {
            answered() == Some(Some(Answer::Yes)) && !is_shown(h, QUESTION)
        });
    let opened = || {
        let mut h = module().unwrap();
        settle(&mut h);
        ask_late(&mut h, confirm(), std::time::Duration::ZERO, ends_late);
        super::rested(&mut h, QUESTION, true);
        h
    };
    let mut h = opened();
    let seen = super::seen(&h, QUESTION);
    super::escape(&mut h, &op, &seen);
    assert_eq!(answered(), Some(Some(Answer::No)), "Escape answered no");
    let mut h = opened();
    super::enter(&mut h, &op, None, "nothing holding the keys");
}

/// The answer given to the question asked last (`None`: none yet).
pub(crate) fn answered() -> Option<Option<Answer>> {
    ANSWERED.with(|a| a.borrow().as_ref().and_then(|x| x.lock().unwrap().clone()))
}

/// A window with no module whose file and folder choosers answer with a
/// file and a folder.
fn choosing() -> Option<Harness<'static, Moonglow>> {
    let dir = mg_testkit::scratch_dir(&super::scratch("u27-shared-choosing"));
    let file = dir.join("chosen.2da");
    std::fs::write(&file, "2DA V2.0\n\n Label\n0 One\n").unwrap();
    let dialogs = mg_ui::NoDialogs { open: vec![file], folders: vec![dir], ..Default::default() };
    Some(harness(Moonglow::new(None, Box::new(dialogs)), 1280.0, 720.0))
}

fn text_field(id: &str, label: &str, default: &str) -> FormField {
    FormField {
        id: id.into(),
        label: label.into(),
        kind: FieldKind::Text { default: default.into() },
    }
}

pub(crate) fn message() -> Question {
    Question::Message("Done: 10 creatures prefixed.".into())
}

pub(super) fn confirm() -> Question {
    Question::Confirm("Prefix up to 10 creatures?".into())
}

fn form() -> Question {
    let fields = vec![text_field("prefix", "Prefix", "NPC_")];
    Question::Form { title: "Prefix Tags".into(), fields }
}

fn file() -> Question {
    Question::File { title: "The table to read".into(), extensions: vec!["2da".into()] }
}

fn page() -> Question {
    Question::Page(mg_plugin::Page {
        wizard: "Make a Shop".into(),
        title: "Its Name".into(),
        text: "What the shop is called.".into(),
        fields: vec![text_field("name", "Name", "Gold Shop")],
        step: 2,
        steps: 3,
        back: true,
        last: false,
        error: None,
    })
}

fn folder() -> Question {
    Question::Folder { title: "The folder of tables".into() }
}

// -- the talk table's question ---------------------------------------------

/// The sample module named "Sample", its custom talk table `mg_typed` with
/// the line "The Keep", typed text put in it; `None` without the game.
fn typed() -> Option<Harness<'static, Moonglow>> {
    use mg_module::talk::{Line, Table};
    let root = mg_testkit::nwn_root()?;
    let dir = mg_testkit::scratch_dir(&super::scratch("u27-shared-typed"));
    let user = user_folder(&dir, &[]);
    let path = sample_module(&dir);
    let mut m = Module::open(&path).unwrap();
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_CUSTOM_TLK, ExoString::from("mg_typed"));
    let name = LocString::from_text(Language::ENGLISH, Gender::Male, "Sample");
    info.root.write(&ifo::MOD_NAME, name);
    m.set_info(&info).unwrap();
    m.save().unwrap();
    let mut t = Table::create("mg_typed", &user.join("tlk"), Language::ENGLISH, true);
    let line =
        Line { text: "The Keep".into(), feminine: None, sound: String::new(), sound_length: 0.0 };
    t.add_line(&line).unwrap();
    t.save().unwrap();
    let mut app = game_app(&root, Some(user));
    app.settings.no_last_area = true;
    app.settings.talk_typed = true;
    app.open_module(&path);
    app.palette_dock = false;
    let mut h = harness(app, 1280.0, 720.0);
    settle(&mut h);
    Some(h)
}

/// "Sample" typed over with the text the table has (asked again: the
/// field, kept at "The Keep" by the answer before, set back first).
fn retype_as_a_line_of_the_table(h: &mut Harness<'static, Moonglow>) {
    let input =
        |n: &egui_kittest::Node<'_>| n.accesskit_node().role() == egui::accesskit::Role::TextInput;
    let type_over = |h: &mut Harness<'static, Moonglow>, from: &str, to: &str| {
        // (Scrolled into sight first: the least window shows a part of
        // the page.)
        h.get_all_by_value(from).find(input).expect("the field").scroll_to_me();
        settle(h);
        h.get_all_by_value(from).find(input).expect("the field").click();
        h.run();
        h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
        h.run();
        h.event(egui::Event::Text(to.into()));
        h.run();
        h.key_press(egui::Key::Tab);
        settle(h);
    };
    if h.query_all_by_value("The Keep").any(|n| input(&n)) {
        type_over(h, "The Keep", "Sample");
    }
    type_over(h, "Sample", "The Keep");
}

// -- the project commands --------------------------------------------------

const COMMAND: &str = "[commands]\nupdate = \"echo said\"\n";

fn project_with_a_command() -> Option<Harness<'static, Moonglow>> {
    project_with_a_command_in("u27-shared-command")
}

/// [`project_with_a_command`], in a scratch folder of its own (`name`).
pub(crate) fn project_with_a_command_in(name: &str) -> Option<Harness<'static, Moonglow>> {
    let dir = project_folder(&super::scratch(name), &[("moonglow.toml", COMMAND)]);
    let mut app = app_with(Vec::new());
    app.settings.no_last_area = true;
    app.open_module(&dir);
    Some(harness(app, 1280.0, 720.0))
}

/// The command run, answered, and its output shown (File › Project ›
/// Command Output).
pub(crate) fn command_output(h: &mut Harness<'static, Moonglow>) {
    h.state_mut().actions.push(mg_ui::Action::ProjectCommand("update".into()));
    settle(h);
    // (Asked the first time; allowed, it runs without asking.)
    if h.query_by_label("Allow and Run").is_some() {
        h.get_by_label("Allow and Run").click();
    }
    let started = std::time::Instant::now();
    while h.state().busy() {
        assert!(started.elapsed().as_secs() < 60, "the command never ended");
        std::thread::sleep(std::time::Duration::from_millis(10));
        h.step();
    }
    settle(h);
    h.get_by_label("File").click();
    h.run();
    h.get_by_label("Project").click();
    h.run();
    h.get_by_label("Command Output").click();
    settle(h);
}

pub(crate) fn openers() -> Vec<Opener> {
    vec![
        Opener::new(
            "widgets.rs:files_already#1",
            "Files Already in the Hak",
            hak_with_a_file,
            add_files,
        ),
        Opener::new("widgets.rs:sets_window#1", "Variable Sets", module, |h| {
            h.state_mut().var_set_dir =
                Some(mg_testkit::scratch_dir(&super::scratch("u27-shared-sets")));
            let mut edit = variables();
            edit.sets = Some(Default::default());
            h.state_mut().var_edit = Some(edit);
        }),
        Opener::new("widgets.rs:show#1", "New Script", module, |h| {
            h.state_mut().new_script = Some("u27_script".into())
        })
        .field(0)
        .off(|h| clear_field(h, "New Script", 0)),
        Opener::new("widgets.rs:ui#1", "String Edit: Module name", module, |h| {
            let name = LocString::from_text(Language::ENGLISH, Gender::Male, "Sample");
            let edit =
                mg_ui::widgets::LocStringEdit::new(info_target("Mod_Name"), "Module name", &name);
            h.state_mut().loc_edit = Some(edit);
        })
        .field(0)
        .off(|h| h.state_mut().loc_edit.as_mut().unwrap().strref = "not a number".into()),
        Opener::new("widgets.rs:ui#2", "Variables", plain, |h| {
            h.state_mut().var_edit = Some(variables())
        })
        .off(|h| {
            h.state_mut()
                .var_edit
                .as_mut()
                .unwrap()
                .rows
                .push(mg_ui::widgets::VarRow::new("", 1, "0"))
        }),
        Opener::new("widgets.rs:ui#3", "Add a sound", sound_blueprint, |h| {
            click_reached(h, "Add Sounds…")
        }),
        Opener::new(
            "plugins.rs:project_window#1",
            "This Project Comes with Plugins",
            project_with_two_plugins,
            |h| {
                // (Asked when the project is opened: again, for a second opening.
                // Two plugins, so that Allow All, which Enter is withheld from,
                // is there.)
                let asking = vec!["example.builder-tags".to_string(), "example.hello".to_string()];
                h.state_mut().plugins.project_asking = asking;
            },
        ),
        Opener::new(
            "plugins.rs:remove_window#1",
            "Remove Tag conventions 1.0.0?",
            plugins_installed,
            |h| {
                let dir = mg_testkit::scratch_dir(&super::scratch("u27-shared-remove"));
                h.state_mut().plugins.remove = Some(mg_ui::plugins::Remove {
                    dir,
                    name: "Tag conventions 1.0.0".into(),
                    id: None,
                });
            },
        ),
        Opener::new(
            "plugins.rs:replace_window#1",
            "Replace Tag conventions?",
            plugins_installed,
            replacing,
        ),
        Opener::new(
            "plugins.rs:settings_window#1",
            "Settings: Builder tags",
            project_with_allowed_plugin,
            |h| h.state_mut().open_plugin_settings("example.builder-tags"),
        ),
        Opener::new("plugin_events.rs:window#1", "Before Saving", house_rules, save_with_a_warning),
        // (A running job asks for a frame every 50 ms, its clock and
        // spinner: `.job()` waits for its window to be still, not to rest.)
        Opener::new("jobs.rs:window#1", "Held", module, a_job_under_way).job(),
        // A plugin's questions, in the job's window (`ask_for_tests`).
        Opener::new("jobs.rs:window#1 Message", QUESTION, module, |h| ask(h, message()))
            .job()
            .enter_does("answered OK (yes), and closed", |h| {
                answered() == Some(Some(Answer::Yes)) && !is_shown(h, QUESTION)
            }),
        Opener::new("jobs.rs:window#1 Confirm", QUESTION, module, |h| ask(h, confirm()))
            .job()
            .enter_does("answered yes, and closed", |h| {
                answered() == Some(Some(Answer::Yes)) && !is_shown(h, QUESTION)
            }),
        Opener::new("jobs.rs:window#1 Form", QUESTION, module, |h| ask(h, form()))
            .job()
            .field(0)
            .enter_does("answered with the form's values, and closed", |h| {
                matches!(answered(), Some(Some(Answer::Values(v))) if v["prefix"] == "NPC_")
                    && !is_shown(h, QUESTION)
            }),
        Opener::new("jobs.rs:window#1 File", QUESTION, choosing, |h| ask(h, file()))
            .job()
            .enter_does("answered with the file chosen, and closed", |h| {
                matches!(answered(), Some(Some(Answer::Path(_)))) && !is_shown(h, QUESTION)
            }),
        Opener::new("jobs.rs:window#1 Page", QUESTION, module, |h| ask(h, page()))
            .job()
            .field(0)
            .enter_does("answered the page on (not back), and closed", |h| {
                matches!(answered(), Some(Some(Answer::Page { back: false, .. })))
                    && !is_shown(h, QUESTION)
            }),
        Opener::new("jobs.rs:window#1 Folder", QUESTION, choosing, |h| ask(h, folder()))
            .job()
            .enter_does("answered with the folder chosen, and closed", |h| {
                matches!(answered(), Some(Some(Answer::Path(_)))) && !is_shown(h, QUESTION)
            }),
        Opener::new(
            "talk_typed.rs:window#1",
            "Text in the Talk Table",
            typed,
            retype_as_a_line_of_the_table,
        ),
        Opener::new(
            "project_commands.rs:output_window#1",
            "Command Output",
            project_with_a_command,
            command_output,
        )
        .enter_does("copies the output, a line each, and stays open", |h| {
            super::copied().iter().any(|t| t.contains("said")) && is_shown(h, "Command Output")
        }),
        Opener::new(
            "project_commands.rs:window#1",
            "Run a Project Command?",
            project_with_a_command,
            |h| h.state_mut().actions.push(mg_ui::Action::ProjectCommand("update".into())),
        ),
    ]
}

/// The job window (a plugin's question and a project command's output are
/// in it too): the footer, with Cancel as what leaves it, whole at the foot
/// of the window; it asks for frames while its job runs, so it never rests
/// for [`settle`] (the table's test can't hold it), and comes to rest when
/// the job ends.
#[test]
fn the_job_window_holds_the_pattern_and_comes_to_rest() {
    super::running("-job");
    let mut h = module().unwrap();
    settle(&mut h);
    a_job_under_way(&mut h);
    super::rested(&mut h, "Held", true);
    let all = mg_ui::widgets::footers_shown(&h.ctx);
    let seen = all.iter().find(|f| f.title == "Held").unwrap_or_else(|| panic!("{all:#?}"));
    let foot = seen.rect.expect("a footer");
    assert_eq!(foot.height(), mg_uikit::dialog::FOOTER_HEIGHT);
    let words: Vec<_> = seen.buttons.iter().map(|b| (b.words.as_str(), b.main, b.leaves)).collect();
    assert_eq!(words, [("Cancel", false, true)]);
    let title = h.get_by_label("Held").rect();
    assert!(title.bottom() < foot.top() && h.ctx.content_rect().contains_rect(foot));
    assert!(foot.contains_rect(seen.buttons[0].rect), "whole");
    // Cancel calls the job off; the window goes and the application rests.
    h.get_all_by_label("Cancel").last().unwrap().click();
    settle(&mut h);
    assert!(h.query_by_label("Held").is_none(), "gone with its job");
    assert!(!h.state().busy());
    // Escape is its Cancel too (an `egui::Modal`'s Escape, through the
    // footer); Enter, with no main action, calls nothing.
    a_job_under_way(&mut h);
    super::rested(&mut h, "Held", true);
    mg_ui::widgets::dialog_presses(&h.ctx);
    h.key_press(egui::Key::Enter);
    super::rested(&mut h, "Held", true);
    assert!(h.query_by_label("Held").is_some(), "Enter leaves it running");
    h.key_press(egui::Key::Escape);
    settle(&mut h);
    let presses = mg_ui::widgets::dialog_presses(&h.ctx);
    assert!(presses.contains(&("Held".into(), "Cancel".into(), "Escape")), "{presses:?}");
    assert!(h.query_by_label("Held").is_none() && !h.state().busy(), "called off by Escape");
}
