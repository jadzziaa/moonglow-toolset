//! M4's U28: the seven dialogs where they are now. Find Instance and Find
//! and Replace Text in Search Results (their forms over their hits);
//! Build Module's and Verify's findings in Problems; Hak Pak Conflict
//! Analysis, Resources Used and the Claims Table as the Bottom Dock's
//! tabs. Each key and action on its effect, the searches as jobs, and the
//! rows in sight alone laid out, at 50,000.

use crate::common::*;
use mg_ui::Panel;
use mg_ui::problems::{Go, Problem, Source};

type H = Harness<'static, Moonglow>;

/// A module of two areas, `a1` and `a2`, each with two placeables
/// ("TREE_a1_0", from `mg_tree`) and a creature ("GUARD", from
/// `mg_guard`), and the sample's script `hello`.
fn objects_module(dir: &std::path::Path) -> std::path::PathBuf {
    let mut m = Module::new();
    let mut info = Gff::new(*b"IFO ");
    info.root.write(&ifo::MOD_TAG, ExoString::from("OBJECTS"));
    for area in ["a1", "a2"] {
        let mut a = ifo::MOD_AREA_LIST.new_item();
        a.write(&ifo::mod_area_list::AREA_NAME, ResRef::from_str(area).unwrap());
        info.root.items_mut(&ifo::MOD_AREA_LIST).push(a);
        m.set(ResKey::parse(area, ResType::ARE).unwrap(), Gff::new(*b"ARE ").to_bytes().unwrap());
        let object = |tag: String, template: &str| {
            let mut s = mg_gff::Struct::new(0);
            s.set("Tag", mg_gff::Value::String(tag.into_bytes()));
            s.set("TemplateResRef", mg_gff::Value::ResRef(template.as_bytes().to_vec()));
            s
        };
        let mut git = Gff::new(*b"GIT ");
        let trees = (0..2).map(|n| object(format!("TREE_{area}_{n}"), "mg_tree")).collect();
        git.root.set("Placeable List", mg_gff::Value::List(trees));
        git.root
            .set("Creature List", mg_gff::Value::List(vec![object("GUARD".into(), "mg_guard")]));
        m.set(ResKey::parse(area, ResType::GIT).unwrap(), git.to_bytes().unwrap());
    }
    m.set_info(&info).unwrap();
    m.set(ResKey::parse("hello", ResType::NSS).unwrap(), b"void main() { }\n".to_vec());
    let path = dir.join("objects.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    path
}

fn objects(name: &str) -> H {
    let dir = mg_testkit::scratch_dir(name);
    let mut app = app_with(Vec::new());
    // (No area opened with the module: Enter and a double click must.)
    app.settings.no_last_area = true;
    app.open_module(&objects_module(&dir));
    // (Editors as tabs: a floating one opened over the dock would take
    // its clicks.)
    app.settings.open_editors = mg_ui::OpenEditors::Tabs;
    let mut h = harness(app, 1280.0, 720.0);
    settle(&mut h);
    h
}

/// The rows of the table `name` drawn in the Bottom Dock, top to bottom,
/// by their accessible names.
fn rows_named(h: &H, starts: &str) -> Vec<String> {
    let dock = h.state().bottom_dock_rect().expect("the Bottom Dock open");
    let mut rows: Vec<(f32, String)> = h
        .query_all_by(|n| n.label().is_some_and(|l| l.starts_with(starts)))
        .filter(|n| n.accesskit_node().bounding_box().is_some() && dock.contains_rect(n.rect()))
        .map(|n| (n.rect().top(), n.accesskit_node().label().unwrap_or_default()))
        .collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    rows.into_iter().map(|r| r.1).collect()
}

fn key(h: &mut H, key: egui::Key) {
    h.key_press(key);
    h.run();
}

fn typed(h: &mut H, text: &str) {
    h.event(egui::Event::Text(text.into()));
    h.run();
}

/// Find Instance by Edit's menu, its blueprint field `blueprint` typed
/// and Enter pressed in it.
fn find(h: &mut H, blueprint: &str) {
    menu_command(h, "Edit", "Find Instance…");
    typed(h, blueprint);
    key(h, egui::Key::Enter);
    settle(h);
}

/// Edit › Find Instance… shows Search Results with its form, the keys in
/// From Blueprint (what is typed lands there); Enter in a field searches,
/// what it found listed under the form with its count; Escape in a field
/// closes the Bottom Dock, as it closed the window.
#[test]
fn find_instance_opens_on_its_form_and_enter_and_escape_act_in_its_fields() {
    let mut h = objects("u28-find-keys");
    h.state_mut().settings.hide_log = true;
    settle(&mut h);
    menu_command(&mut h, "Edit", "Find Instance…");
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::SearchResults), "opened on it");
    typed(&mut h, "mg_tree");
    let f = h.state().find_instance.as_ref().expect("its form");
    assert_eq!(f.template, "mg_tree", "the keys in From Blueprint");
    assert!(!f.searched, "nothing searched while typing");
    key(&mut h, egui::Key::Enter);
    dock_high(&mut h, 360);
    settle(&mut h);
    let f = h.state().find_instance.as_ref().unwrap();
    assert_eq!(f.results.len(), 4, "Enter searched: {:?}", f.results);
    in_dock(&h, &mg_ui::search_results::instances_words(4));
    assert_eq!(
        rows_named(&h, "Placeable TREE_"),
        [
            "Placeable TREE_a1_0",
            "Placeable TREE_a1_1",
            "Placeable TREE_a2_0",
            "Placeable TREE_a2_1"
        ]
    );
    // Escape in a field: the dock closed, the search kept.
    h.query_all_by(|n| n.placeholder() == Some("tag")).next().expect("With Tag").click();
    h.run();
    key(&mut h, egui::Key::Escape);
    assert!(h.state().settings.hide_log, "Escape closed the Bottom Dock");
    assert_eq!(h.state().find_instance.as_ref().map(|f| f.results.len()), Some(4));
    // Opened again, the keys in From Blueprint again.
    menu_command(&mut h, "Edit", "Find Instance…");
    assert!(!h.state().settings.hide_log, "opened again");
    typed(&mut h, "x");
    assert_eq!(h.state().find_instance.as_ref().unwrap().template, "mg_treex");
}

/// Find Instance's hits: a click chooses one and gives the table the
/// keys; the arrows, Page Down, Home and End move the one chosen; Enter
/// and a double click open its area at it; Ctrl + C copies its words;
/// a column's head sorts by it, again the other way; Clear empties the
/// form and the list.
#[test]
fn find_instance_s_hits_take_the_keys_open_their_area_sort_and_copy() {
    let mut h = objects("u28-find-hits");
    find(&mut h, "");
    dock_high(&mut h, 360);
    assert_eq!(h.state().find_instance.as_ref().unwrap().results.len(), 6);
    let chosen = |h: &H| {
        let i = h.state().search_results.instance_chosen()?;
        let f = h.state().find_instance.as_ref()?;
        Some(f.results[i].tag.clone())
    };
    in_dock(&h, "Creature GUARD").click();
    h.run();
    assert_eq!(chosen(&h).as_deref(), Some("GUARD"), "clicked");
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(chosen(&h).as_deref(), Some("GUARD"), "the second GUARD, a2's");
    assert_eq!(h.state().search_results.instance_chosen(), Some(3));
    key(&mut h, egui::Key::End);
    assert_eq!(chosen(&h).as_deref(), Some("TREE_a2_1"));
    key(&mut h, egui::Key::Home);
    assert_eq!(chosen(&h).as_deref(), Some("GUARD"));
    assert_eq!(h.state().search_results.instance_chosen(), Some(0));
    key(&mut h, egui::Key::PageDown);
    assert!(h.state().search_results.instance_chosen() != Some(0), "a page on");
    key(&mut h, egui::Key::Home);
    let (copied, _) = press_now(&mut h, egui::Modifiers::COMMAND, egui::Key::C);
    assert_eq!(copied, ["Creature GUARD\tin a1, from mg_guard"]);
    // Enter: its area opened at it.
    let a1 = ResRef::from_str("a1").unwrap();
    assert!(h.state().dock.find_tab(&Tab::Area(a1)).is_none(), "no area open yet");
    key(&mut h, egui::Key::Enter);
    settle(&mut h);
    assert!(h.state().dock.find_tab(&Tab::Area(a1)).is_some(), "Enter opened a1");
    // A double click on another: a2's tab.
    let row = in_dock(&h, "Placeable TREE_a2_0").rect().center();
    double_click_now(&mut h, row);
    settle(&mut h);
    let a2 = ResRef::from_str("a2").unwrap();
    assert!(h.state().dock.find_tab(&Tab::Area(a2)).is_some(), "the double click opened a2");
    // Sorted by Tag, then the other way.
    in_dock(&h, "Tag").click();
    h.run();
    let first =
        |h: &H| rows_named(h, "Creature ").into_iter().chain(rows_named(h, "Placeable ")).count();
    assert_eq!(first(&h), 6);
    let tags = |h: &H| {
        let mut all = rows_named(h, "Creature ");
        all.extend(rows_named(h, "Placeable "));
        let dock = h.state().bottom_dock_rect().unwrap();
        let mut at: Vec<(f32, String)> = Vec::new();
        for l in all {
            let top =
                h.get_all_by_label(&l).find(|n| dock.contains_rect(n.rect())).unwrap().rect().top();
            at.push((top, l));
        }
        at.sort_by(|a, b| a.0.total_cmp(&b.0));
        at.dedup();
        at.into_iter().map(|a| a.1).collect::<Vec<_>>()
    };
    let up = tags(&h);
    assert_eq!(up.first().map(String::as_str), Some("Creature GUARD"), "{up:?}");
    assert_eq!(up.last().map(String::as_str), Some("Placeable TREE_a2_1"), "{up:?}");
    in_dock(&h, "Tag, sorted up").click();
    h.run();
    let down = tags(&h);
    assert_eq!(down.first().map(String::as_str), Some("Placeable TREE_a2_1"), "{down:?}");
    in_dock(&h, "Tag, sorted down");
    // Clear: the form as it opens, nothing listed.
    in_dock(&h, "Clear").click();
    settle(&mut h);
    let f = h.state().find_instance.as_ref().unwrap();
    assert!(!f.searched && f.results.is_empty() && f.template.is_empty());
    assert!(rows_named(&h, "Placeable ").is_empty(), "nothing listed");
}

/// The table's keys are its own only while nothing is over it: a menu
/// open, Down leaves the hit chosen where it is (and the menu takes it).
#[test]
fn find_instance_s_keys_are_not_taken_under_a_menu() {
    let mut h = objects("u28-find-menu");
    find(&mut h, "");
    in_dock(&h, "Creature GUARD").click();
    h.run();
    assert_eq!(h.state().search_results.instance_chosen(), Some(0));
    h.get_by_label("Top Bar").get_by_label("View").click();
    h.run();
    key(&mut h, egui::Key::ArrowDown);
    key(&mut h, egui::Key::End);
    assert_eq!(h.state().search_results.instance_chosen(), Some(0), "the menu's keys");
}

/// Search Results' switch counts what it shows (none before a search);
/// closing the module lets Find Instance go, the switch's count with it.
#[test]
fn search_results_badge_counts_its_hits_and_a_closed_module_lets_them_go() {
    let mut h = objects("u28-find-badge");
    menu_command(&mut h, "Edit", "Find Instance…");
    assert_eq!(h.state().switch_badge(Panel::SearchResults), None, "nothing searched");
    key(&mut h, egui::Key::Enter);
    settle(&mut h);
    let badge = h.state().switch_badge(Panel::SearchResults).map(|b| b.0);
    assert_eq!(badge.as_deref(), Some("6"));
    h.state_mut().close();
    settle(&mut h);
    assert!(h.state().find_instance.is_none(), "Find Instance let go of");
    assert_eq!(h.state().switch_badge(Panel::SearchResults), None);
}

/// Closing the module lets Find and Replace Text's strings go (a click on
/// one would open another module's), its form kept as typed.
#[test]
fn a_closed_module_lets_find_and_replace_text_s_strings_go() {
    let mut h = camps("u28-text-close", 3);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::H);
    settle(&mut h);
    typed(&mut h, "orc");
    key(&mut h, egui::Key::Enter);
    settle(&mut h);
    assert_eq!(h.state().text_replace.as_ref().unwrap().hits.len(), 3, "found");
    let badge = h.state().switch_badge(Panel::SearchResults).map(|b| b.0);
    assert_eq!(badge.as_deref(), Some("3"));
    h.state_mut().close();
    settle(&mut h);
    let d = h.state().text_replace.as_ref().expect("its form kept");
    assert_eq!(d.find, "orc", "as typed");
    assert!(d.hits.is_empty() && d.searched.is_none(), "its strings let go of");
    assert_eq!(h.state().switch_badge(Panel::SearchResults), None);
}

/// A search is a job: while one runs, Search is not offered and the
/// window says what runs, with Cancel; Cancel ends it and the log says
/// so; then Search finds again.
#[test]
fn a_search_waits_for_a_job_and_its_cancel_ends_it() {
    let mut h = objects("u28-find-job");
    menu_command(&mut h, "Edit", "Find Instance…");
    h.state_mut().start_job_for_tests("Find Instance", |p| {
        while !p.cancelled() {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    });
    // (Its spinner asks for frames while it runs.)
    h.run_steps(3);
    assert!(h.state().busy());
    let search = in_dock(&h, "Search");
    assert!(search.accesskit_node().is_disabled(), "Search waits for the job");
    let cancel = h.get_all_by_label("Cancel").last().expect("the job's Cancel").rect().center();
    click_pos(&h, cancel);
    settle(&mut h);
    assert!(!h.state().busy(), "Cancel ended it");
    let said: Vec<String> = h.state().log.entries.iter().map(|e| e.1.clone()).collect();
    assert!(said.iter().any(|l| l == "Find Instance: canceled"), "{said:?}");
    in_dock(&h, "Search").click();
    settle(&mut h);
    assert_eq!(h.state().find_instance.as_ref().unwrap().results.len(), 6, "searched");
}

/// The camp_00 to camp_29 waypoints of [`dock_pictures`]' text sample,
/// each "Orc camp by the old road, …".
fn camps(name: &str, n: usize) -> H {
    use mg_edit::{Command, Edit};
    let dir = mg_testkit::scratch_dir(name);
    let mut app = app_with(Vec::new());
    app.open_module(&sample_module(&dir));
    let edits = (0..n)
        .map(|i| {
            let key = ResKey::parse(&format!("camp_{i:05}"), ResType::UTW).unwrap();
            let mut g = Gff::new(*b"UTW ");
            let words = format!("Orc camp by the old road, the {i}th fire");
            let name = LocString::from_text(Language::ENGLISH, Gender::Male, words.as_str());
            g.root.set("LocalizedName", mg_gff::Value::LocString(name));
            Edit::SetResource { key, data: Some(g.to_bytes().unwrap()) }
        })
        .collect();
    app.ws.as_mut().unwrap().apply(Command::new("setup", edits)).unwrap();
    // (Editors as tabs: a floating one opened over the dock would take
    // its clicks.)
    app.settings.open_editors = mg_ui::OpenEditors::Tabs;
    let mut h = harness(app, 1280.0, 720.0);
    settle(&mut h);
    h
}

/// The string drawn chosen (its row's selected state, as egui gives it:
/// toggled), if one is.
fn chosen_string(h: &H) -> Option<String> {
    let dock = h.state().bottom_dock_rect()?;
    h.query_all_by(|n| {
        n.label().is_some_and(|l| l.starts_with("camp_"))
            && n.toggled() == Some(egui::accesskit::Toggled::True)
    })
    .find(|n| dock.contains_rect(n.rect()))
    .and_then(|n| n.accesskit_node().label())
}

/// Find and Replace Text (Ctrl + H): the keys in Find what, Enter finds;
/// its strings take the keys by a click: Down moves the one chosen, Space
/// unticks it (Replace in N counts one fewer), Enter opens its resource,
/// and a double click another's; Escape in a field closes the dock.
#[test]
fn find_and_replace_text_s_strings_take_the_keys() {
    let mut h = camps("u28-text-keys", 6);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::H);
    settle(&mut h);
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::SearchResults));
    typed(&mut h, "orc");
    key(&mut h, egui::Key::Enter);
    dock_high(&mut h, 420);
    settle(&mut h);
    let d = h.state().text_replace.as_ref().unwrap();
    assert_eq!(d.find, "orc", "the keys in Find what");
    assert_eq!(d.hits.len(), 6, "Enter found");
    in_dock(&h, "6 strings, 6 times");
    in_dock(&h, "Replace in 6");
    let first = rows_named(&h, "camp_00000.utw › LocalizedName: ");
    assert_eq!(first.len(), 1, "{first:?}");
    in_dock(&h, &first[0]).click();
    h.run();
    assert_eq!(chosen_string(&h).as_deref(), Some(first[0].as_str()), "clicked");
    key(&mut h, egui::Key::ArrowDown);
    let second = rows_named(&h, "camp_00001.utw › LocalizedName: ");
    assert_eq!(chosen_string(&h).as_deref(), Some(second[0].as_str()), "Down");
    key(&mut h, egui::Key::Space);
    let d = h.state().text_replace.as_ref().unwrap();
    assert!(!d.hits[1].1 && d.hits[0].1, "Space unticked the one chosen");
    in_dock(&h, "Replace in 5");
    in_dock(&h, "6 strings, 5 times");
    key(&mut h, egui::Key::Space);
    assert!(h.state().text_replace.as_ref().unwrap().hits[1].1, "and ticked it again");
    // A click on a string's own words (its second line, under its place)
    // chooses it too.
    let third = rows_named(&h, "camp_00002.utw › LocalizedName: ");
    let row = in_dock(&h, &third[0]).rect();
    let words = egui::pos2(row.left() + 60.0, row.bottom() - row.height() / 4.0);
    click_pos(&h, words);
    h.run();
    assert_eq!(chosen_string(&h).as_deref(), Some(third[0].as_str()), "a click on its words");
    key(&mut h, egui::Key::ArrowUp);
    let utw =
        |n: usize| Tab::Blueprint(ResKey::parse(&format!("camp_{n:05}"), ResType::UTW).unwrap());
    key(&mut h, egui::Key::Enter);
    settle(&mut h);
    assert!(h.state().dock.find_tab(&utw(1)).is_some(), "Enter opened camp_00001");
    let row = rows_named(&h, "camp_00003.utw › LocalizedName: ");
    let at = in_dock(&h, &row[0]).rect().center();
    double_click_now(&mut h, at);
    settle(&mut h);
    assert!(h.state().dock.find_tab(&utw(3)).is_some(), "the double click opened camp_00003");
    // Escape in Find what: the dock closed.
    h.state_mut().settings.hide_log = false;
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::H);
    settle(&mut h);
    key(&mut h, egui::Key::Escape);
    assert!(h.state().settings.hide_log, "Escape closed the Bottom Dock");
}

/// Problems filled as Build Module fills it, three findings of
/// `hello.nss` (an error first), the module as it is now.
fn built(h: &mut H) {
    let hello = ResKey::parse("hello", ResType::NSS).unwrap();
    let p = |severity, message: &str| {
        Problem::new(severity, "hello.nss".into(), message.into(), Some(Go::Resource(hello)))
    };
    use mg_module::doctor::Severity;
    let mut unused = p(Severity::Warning, "Unused (source and compiled)");
    unused.unused = Some(mg_module::verify::Unused {
        key: hello,
        script: Some(mg_module::verify::ScriptFiles::Both),
    });
    let found = vec![p(Severity::Warning, "first warning"), unused, p(Severity::Error, "bad")];
    let now = h.state().ws.as_ref().map(|ws| ws.revision());
    h.state_mut().problems.replace(Source::Build, found, now);
    if h.state().bottom_dock_shows() != Some(Panel::Problems) {
        h.state_mut().settings.hide_log = false;
        h.get_by_label("Status Bar").get_by_label("Problems").click();
    }
    dock_high(h, 320);
    settle(h);
}

/// Build Module's findings in Problems: under Build Module's heading
/// (their counts in the tab bar's place), errors first; a click chooses one, Down and Up move it,
/// Enter goes to it (its script opened); the Status Bar counts them; the
/// module changed, the heading says so and they are no longer counted.
#[test]
fn build_s_findings_in_problems_take_the_keys_and_go_stale() {
    let dir = mg_testkit::scratch_dir("u28-build-keys");
    let mut app = app_with(Vec::new());
    app.open_module(&sample_module(&dir));
    let mut h = harness(app, 1280.0, 720.0);
    settle(&mut h);
    built(&mut h);
    in_dock(&h, "Build Module");
    let rows = rows_named(&h, "Error: ")
        .into_iter()
        .chain(rows_named(&h, "Warning: "))
        .collect::<Vec<_>>();
    assert_eq!(
        rows,
        [
            "Error: hello.nss, bad",
            "Warning: hello.nss, first warning",
            "Warning: hello.nss, Unused (source and compiled)"
        ]
    );
    let badge = h.state().switch_badge(Panel::Problems).map(|b| b.0);
    assert_eq!(badge.as_deref(), Some("3"));
    in_dock(&h, "Error: hello.nss, bad").click();
    h.run();
    let chosen = |h: &H| h.state().problems.selected().map(|p| p.message.clone());
    assert_eq!(chosen(&h).as_deref(), Some("bad"));
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(chosen(&h).as_deref(), Some("first warning"));
    key(&mut h, egui::Key::End);
    assert_eq!(chosen(&h).as_deref(), Some("Unused (source and compiled)"));
    key(&mut h, egui::Key::ArrowUp);
    assert_eq!(chosen(&h).as_deref(), Some("first warning"));
    key(&mut h, egui::Key::Enter);
    settle(&mut h);
    let hello = ResKey::parse("hello", ResType::NSS).unwrap();
    assert!(h.state().dock.find_tab(&Tab::Script(hello)).is_some(), "Enter went to hello.nss");
    // The module changed: stale, not counted.
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "change",
        vec![mg_edit::Edit::SetResource {
            key: ResKey::parse("other", ResType::NSS).unwrap(),
            data: Some(b"void main() {}".to_vec()),
        }],
    )));
    settle(&mut h);
    in_dock(&h, "Changed since the check: not counted");
    assert_eq!(h.state().switch_badge(Panel::Problems), None, "stale: not counted");
}

/// An Unused finding's row menu offers Mark as Used where the module has a
/// place for the project file, and its tip says why not where it has none;
/// Build's heading's Counted as Used opens what the project file counts.
#[test]
fn an_unused_finding_offers_mark_as_used_and_the_heading_counted_as_used() {
    let dir = mg_testkit::scratch_dir("u28-build-marks");
    let project = dir.join("project");
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    m.set(ResKey::parse("hello", ResType::NSS).unwrap(), b"void main() { }\n".to_vec());
    m.save_as(&ModuleLocation::Folder(project.clone())).unwrap();
    let mut app = app_with(Vec::new());
    app.open_module(&project);
    let mut h = harness(app, 1280.0, 720.0);
    settle(&mut h);
    // (Built with Unused ticked: Counted as Used shown with its findings.)
    h.state_mut().problems.build_unused = true;
    built(&mut h);
    in_dock(&h, "Warning: hello.nss, Unused (source and compiled)").click_secondary();
    h.run();
    // (Its own command first, then Copy and Copy All: 5.2's W4.)
    let top = |name: &str| h.get_all_by_label(name).last().unwrap().rect().top();
    let order = [mg_ui::problems::MARK_AS_USED, mg_ui::row_copy::COPY, mg_ui::row_copy::COPY_ALL];
    assert!(order.windows(2).all(|w| top(w[0]) < top(w[1])), "the menu's order: {order:?}");
    h.get_by_label(mg_ui::problems::MARK_AS_USED).click();
    settle(&mut h);
    let toml = std::fs::read_to_string(project.join("moonglow.toml")).unwrap_or_default();
    assert!(toml.contains("hello"), "counted as used in the project file: {toml}");
    let rows = rows_named(&h, "Warning: ");
    assert_eq!(rows, ["Warning: hello.nss, first warning"], "the Unused one taken away");
    in_dock(&h, mg_ui::problems::COUNTED_AS_USED).click();
    settle(&mut h);
    assert!(
        h.query_by_label_contains("Script: hello").is_some(),
        "Counted as Used lists what the project file counts"
    );
}

/// A click on a finding's own words (its message, after its place) is the
/// row's: the left button chooses it, the right opens its menu (Mark as
/// Used, an Unused finding's).
#[test]
fn a_click_on_a_finding_s_words_is_its_row_s() {
    let dir = mg_testkit::scratch_dir("u28-build-words");
    let mut app = app_with(Vec::new());
    app.open_module(&sample_module(&dir));
    let mut h = harness(app, 1280.0, 720.0);
    settle(&mut h);
    built(&mut h);
    let row = in_dock(&h, "Warning: hello.nss, Unused (source and compiled)").rect();
    // (Past the mark, the word's 54 and the place: on the message.)
    let words = egui::pos2(row.left() + 200.0, row.center().y);
    click_pos(&h, words);
    h.run();
    let chosen = h.state().problems.selected().map(|p| p.message.clone());
    assert_eq!(chosen.as_deref(), Some("Unused (source and compiled)"), "chosen by its words");
    let button = egui::PointerButton::Secondary;
    let none = egui::Modifiers::NONE;
    h.hover_at(words);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton { pos: words, button, pressed, modifiers: none });
    }
    h.run();
    assert!(h.query_by_label(mg_ui::problems::MARK_AS_USED).is_some(), "its menu by its words");
}

/// Verify Module's findings go to Problems, brought to the front, the
/// Log keeping its lines; a heading's Remove from Problems (Delete with it
/// focused) takes them away.
#[test]
fn verify_s_findings_go_to_problems_and_its_heading_removes_them() {
    let dir = mg_testkit::scratch_dir("u28-verify");
    let mut app = app_with(Vec::new());
    app.open_module(&sample_module(&dir));
    let mut h = harness(app, 1280.0, 720.0);
    settle(&mut h);
    let hello = ResKey::parse("hello", ResType::NSS).unwrap();
    let p = Problem::new(
        mg_module::doctor::Severity::Error,
        "hello.nss".into(),
        "Script: its source does not compile".into(),
        Some(Go::Resource(hello)),
    );
    let now = h.state().ws.as_ref().map(|ws| ws.revision());
    h.state_mut().problems.replace(Source::Verify, vec![p], now);
    h.state_mut().settings.hide_log = false;
    h.get_by_label("Status Bar").get_by_label("Problems").click();
    settle(&mut h);
    in_dock(&h, "Error: hello.nss, Script: its source does not compile");
    in_dock(&h, "Verify Module").click_secondary();
    h.run();
    h.get_by_label(mg_ui::problems::REMOVE).click();
    settle(&mut h);
    assert!(h.state().problems.of(&Source::Verify).is_none(), "taken away");
    assert!(rows_named(&h, "Error: ").is_empty());
}

fn hak_report(n: usize) -> mg_module::haks::HakReport {
    let mut r = mg_module::haks::HakReport::default();
    let haks = ["cep2_top_v28", "cep2_add_doors", "tdod_tiles", "tdod_creatures"];
    for h in haks {
        r.haks.push((h.into(), Some(format!("/home/builder/hak/{h}.hak").into())));
    }
    r.haks.push(("missing_hak".into(), None));
    for i in 0..n {
        let key = ResKey::parse(&format!("tdc01_x{i:05}"), ResType::MDL).unwrap();
        let by = vec![haks[i % 4].to_string(), haks[(i + 1) % 4].to_string()];
        r.resources.insert(key, by.clone());
        if i % 5 == 0 {
            r.overrides.insert(key, vec![by[0].clone()]);
        }
    }
    r
}

/// Hak Pak Conflict Analysis is a Bottom Dock tab: shown in front with
/// the keys; its views listed by their counts, each its rows; a head
/// sorts; the arrows choose, Ctrl + C copies a row, Copy Report the whole;
/// Close Report lets it go, its tab with it.
#[test]
fn hak_conflicts_is_a_tab_with_views_sorting_keys_and_close() {
    let dir = mg_testkit::scratch_dir("u28-hak-tab");
    let mut app = app_with(Vec::new());
    app.open_module(&sample_module(&dir));
    let mut h = harness(app, 1280.0, 720.0);
    settle(&mut h);
    h.state_mut().show_hak_report(hak_report(10));
    dock_high(&mut h, 360);
    settle(&mut h);
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::HakConflicts));
    in_dock(&h, "Conflicts (10)");
    assert_eq!(rows_named(&h, "tdc01_x").len(), 10);
    in_dock(&h, "Overrides (2)").click();
    settle(&mut h);
    assert_eq!(rows_named(&h, "tdc01_x"), ["tdc01_x00000.mdl", "tdc01_x00005.mdl"]);
    in_dock(&h, "Haks (5)").click();
    settle(&mut h);
    assert_eq!(rows_named(&h, "missing_hak").len(), 1, "the haks listed");
    in_dock(&h, "Conflicts (10)").click();
    settle(&mut h);
    in_dock(&h, "Resource, sorted up").click();
    h.run();
    let rows = rows_named(&h, "tdc01_x");
    assert_eq!(rows.first().map(String::as_str), Some("tdc01_x00009.mdl"), "sorted down: {rows:?}");
    // The keys: given by the command, Down chooses the first row.
    key(&mut h, egui::Key::Home);
    let (copied, _) = press_now(&mut h, egui::Modifiers::COMMAND, egui::Key::C);
    assert_eq!(copied, ["tdc01_x00009.mdl\tcep2_add_doors, tdod_tiles"]);
    in_dock(&h, "Copy Report").click();
    h.step();
    let out = h.output().platform_output.commands.clone();
    let report = out.iter().find_map(|c| match c {
        egui::OutputCommand::CopyText(t) => Some(t.clone()),
        _ => None,
    });
    assert!(report.is_some_and(|t| t.starts_with("Hak paks")), "the whole report copied");
    h.run();
    h.get_by_label("Close Report").click();
    settle(&mut h);
    assert!(h.state().hak_report.is_none(), "let go of");
    assert!(h.query_by_label("Hak Pak Conflict Analysis").is_none(), "its tab gone");
}

/// The Claims Table is a Bottom Dock tab: its rows sort by Whose; the
/// arrows choose one and Ctrl + C copies it; Close Report closes it.
#[test]
fn the_claims_table_tab_sorts_takes_the_keys_and_closes() {
    let table = format!(
        "{}1   kz_gone          ****     merged   2026-10-01  KZ_GONE\n",
        claims_table("cd", "claimed")
    );
    let (_, app) = claimed_project("u28-claims-tab", "[claims]\nfile = \"claims.2da\"\n", &table);
    let mut h = harness(app, 1280.0, 720.0);
    settle(&mut h);
    menu_command(&mut h, "Edit", "Claims Table…");
    dock_high(&mut h, 320);
    settle(&mut h);
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::ClaimsTable));
    let areas = |h: &H| {
        let dock = h.state().bottom_dock_rect().unwrap();
        let mut rows: Vec<(f32, String)> = ["start", "kz_gone"]
            .iter()
            .filter_map(|a| h.get_all_by_label(a).find(|n| dock.contains_rect(n.rect())))
            .map(|n| (n.rect().top(), n.accesskit_node().label().unwrap_or_default()))
            .collect();
        rows.sort_by(|a, b| a.0.total_cmp(&b.0));
        rows.into_iter().map(|r| r.1).collect::<Vec<_>>()
    };
    assert_eq!(areas(&h), ["kz_gone", "start"], "by area");
    in_dock(&h, "Area, sorted up").click();
    h.run();
    assert_eq!(areas(&h), ["start", "kz_gone"], "by area, the other way");
    key(&mut h, egui::Key::Home);
    let (copied, _) = press_now(&mut h, egui::Modifiers::COMMAND, egui::Key::C);
    assert_eq!(copied.len(), 1, "{copied:?}");
    assert!(copied[0].starts_with("start\t"), "{copied:?}");
    h.get_by_label("Close Report").click();
    settle(&mut h);
    assert!(h.state().claims_table_counts().is_none(), "closed");
}

/// The rows whose names start `starts` laid out in the window (AccessKit
/// nodes with a box, wherever they are: a row laid out past the dock's
/// bottom is counted too): what is drawn, not what the table says it laid
/// out.
fn drawn_rows(h: &H, starts: &str) -> usize {
    h.query_all_by(|n| n.label().is_some_and(|l| l.starts_with(starts)))
        .filter(|n| n.accesskit_node().bounding_box().is_some())
        .count()
}

/// Resources Used is a Bottom Dock tab (Build › Area Statistics): its rows
/// take the keys, Ctrl + C copying one; its area's view gone (the module
/// closed), it goes.
#[test]
fn resources_used_is_a_tab_that_goes_with_its_area() {
    let Some((mut h, area)) = area_ui_harness("u28-used-tab") else { return };
    menu_command(&mut h, "Build", "Area Statistics");
    settle(&mut h);
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::ResourcesUsed));
    assert_eq!(h.state().area_stats, Some(area));
    // (The keys its command gave the table, the pointer left over the
    // area's view: Down chooses the first row.)
    key(&mut h, egui::Key::ArrowDown);
    let (copied, _) = press_now(&mut h, egui::Modifiers::COMMAND, egui::Key::C);
    assert_eq!(copied, ["Tiles\t16 (4 by 4)"]);
    key(&mut h, egui::Key::ArrowDown);
    let (copied, _) = press_now(&mut h, egui::Modifiers::COMMAND, egui::Key::C);
    assert_eq!(copied, ["Objects\t2"], "Down chose the next row");
    // (Its rows are a handful, never 50,000: those drawn in the dock are
    // those it laid out.)
    let names = ["Tiles", "Objects", "Tile models", "Object models", "Meshes", "Triangles"];
    let drawn: usize = names.iter().map(|n| drawn_rows(&h, n)).sum();
    assert!(drawn >= 2, "{drawn} of its rows drawn");
    // (A closed tab's view is kept, its camera with it: the module closed,
    // the area's view goes, and the tab with it.)
    h.state_mut().close();
    settle(&mut h);
    assert_eq!(h.state().area_stats, None, "gone with its area");
    assert_ne!(h.state().bottom_dock_shows(), Some(Panel::ResourcesUsed));
}

/// Each tab at the Bottom Dock's least height shows its first row whole,
/// inside the dock, under its form or header.
#[test]
fn every_tab_s_first_row_is_whole_at_the_dock_s_least() {
    let mut h = objects("u28-least");
    h.state_mut().settings.bottom_dock_height = Some(0);
    find(&mut h, "");
    let dock = h.state().bottom_dock_rect().unwrap();
    let row = in_dock(&h, "Creature GUARD").rect();
    assert!(dock.contains_rect(row), "Find Instance's first row {row:?} in {dock:?}");
    built(&mut h);
    h.state_mut().settings.bottom_dock_height = Some(0);
    settle(&mut h);
    let dock = h.state().bottom_dock_rect().unwrap();
    let row = in_dock(&h, "Error: hello.nss, bad").rect();
    assert!(dock.contains_rect(row), "Problems' first finding {row:?} in {dock:?}");
    h.state_mut().show_hak_report(hak_report(10));
    settle(&mut h);
    let dock = h.state().bottom_dock_rect().unwrap();
    let row = in_dock(&h, "tdc01_x00000.mdl").rect();
    assert!(dock.contains_rect(row), "the hak report's first row {row:?} in {dock:?}");
}

/// No per-frame cost at 50,000: Find Instance's hits, Find and Replace
/// Text's strings, Problems' findings and the hak report's rows each made
/// once and laid out only in sight, the frames at rest making nothing.
#[test]
fn fifty_thousand_rows_are_laid_out_in_sight_and_made_once() {
    let mut h = objects("u28-fifty");
    let mut f = mg_ui::area_tools::FindInstance::default();
    f.results = (0..50_000)
        .map(|index| mg_ui::area_tools::Found {
            kind: mg_area::ObjectKind::Placeable,
            index,
            area: ResRef::from_str("a1").unwrap(),
            tag: format!("TREE_{index}"),
            template: "mg_tree".into(),
        })
        .collect();
    f.searched = true;
    h.state_mut().find_instance = Some(f);
    menu_command(&mut h, "Edit", "Find Instance…");
    settle(&mut h);
    let sr = |h: &H| {
        (
            h.state().search_results.instance_order_builds(),
            h.state().search_results.instance_rows_seen(),
        )
    };
    let (builds, (from, end)) = sr(&h);
    assert!(end - from < 60 && end > from, "{from}..{end} laid out");
    let drawn = drawn_rows(&h, "Placeable TREE_");
    assert!((1..60).contains(&drawn), "{drawn} hits laid out");
    h.run_steps(10);
    assert_eq!(sr(&h).0, builds, "the order not made again at rest");
    in_dock(&h, "Tag").click();
    h.run_steps(5);
    assert_eq!(sr(&h).0, builds + 1, "made once for the sort");
    // Problems.
    let hello = ResKey::parse("hello", ResType::NSS).unwrap();
    let many: Vec<Problem> = (0..50_000)
        .map(|i| {
            let s = mg_module::doctor::Severity::Warning;
            Problem::new(s, "hello.nss".into(), format!("warning {i}"), Some(Go::Resource(hello)))
        })
        .collect();
    let now = h.state().ws.as_ref().map(|ws| ws.revision());
    h.state_mut().problems.replace(Source::Build, many, now);
    h.state_mut().settings.hide_log = false;
    h.get_by_label("Status Bar").get_by_label("Problems").click();
    settle(&mut h);
    let made = h.state().problems.row_builds();
    h.run_steps(10);
    assert_eq!(h.state().problems.row_builds(), made, "Problems' rows not made again at rest");
    let shown = drawn_rows(&h, "Warning: hello.nss, warning");
    assert!((1..60).contains(&shown), "{shown} findings laid out");
    // The hak report.
    h.state_mut().show_hak_report(hak_report(50_000));
    settle(&mut h);
    let hak = |h: &H| {
        let r = h.state().hak_report.as_ref().unwrap();
        (r.order_builds(), r.rows_seen())
    };
    let (builds, (from, end)) = hak(&h);
    assert!(end - from < 60 && end > from, "{from}..{end} laid out");
    let drawn = drawn_rows(&h, "tdc01_x");
    assert!((1..60).contains(&drawn), "{drawn} hak rows laid out");
    h.run_steps(10);
    assert_eq!(hak(&h).0, builds, "the hak rows' order not made again at rest");
}

/// Find and Replace Text's 50,000 strings: laid out only in sight.
#[test]
fn fifty_thousand_strings_are_laid_out_in_sight() {
    let mut h = camps("u28-fifty-strings", 50_000);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::H);
    settle(&mut h);
    typed(&mut h, "orc");
    key(&mut h, egui::Key::Enter);
    settle(&mut h);
    assert_eq!(h.state().text_replace.as_ref().unwrap().hits.len(), 50_000);
    // (A click on a string gives the list the keys.)
    let first = rows_named(&h, "camp_00000.utw › LocalizedName: ");
    in_dock(&h, &first[0]).click();
    h.run();
    let (from, end) = h.state().text_replace.as_ref().unwrap().strings_seen();
    assert!(end > from && end - from < 40, "{from}..{end} laid out");
    key(&mut h, egui::Key::End);
    settle(&mut h);
    let (from, end) = h.state().text_replace.as_ref().unwrap().strings_seen();
    assert_eq!(end, 50_000, "End brought the last into sight");
    assert!(end - from < 40, "{from}..{end} laid out");
}

/// The Claims Table's rows at 5,000 areas: laid out only in sight, their
/// order made once.
#[test]
fn the_claims_table_lays_out_its_rows_in_sight() {
    let mut table = claims_table("cd", "claimed");
    for i in 1..5000 {
        table.push_str(&format!("{i}   area_{i:05}       ****     claimed  2026-10-01  ****\n"));
    }
    let (_, app) = claimed_project("u28-claims-many", "[claims]\nfile = \"claims.2da\"\n", &table);
    let mut h = harness(app, 1280.0, 720.0);
    settle(&mut h);
    menu_command(&mut h, "Edit", "Claims Table…");
    settle(&mut h);
    let (builds, (from, end)) = h.state().claims_table_counts().expect("open");
    assert!(end > from && end - from < 40, "{from}..{end} laid out");
    let drawn = drawn_rows(&h, "area_");
    assert!((1..40).contains(&drawn), "{drawn} areas laid out");
    h.run_steps(10);
    assert_eq!(h.state().claims_table_counts().unwrap().0, builds, "not made again at rest");
}

// M4's U28 review (the orchestrator's rulings): the keys a command gives
// a dock list hold over the area's view until the pointer is pressed
// somewhere else or another panel is given them.

/// The rows chosen in the Bottom Dock (toggled on), by their names.
fn chosen_in_dock(h: &H) -> Vec<String> {
    let Some(dock) = h.state().bottom_dock_rect() else { return Vec::new() };
    let mut v: Vec<String> = h
        .query_all_by(|n| n.toggled() == Some(egui::accesskit::Toggled::True))
        .filter(|n| n.accesskit_node().bounding_box().is_some() && dock.contains_rect(n.rect()))
        .filter_map(|n| n.accesskit_node().label())
        .collect();
    v.sort();
    v
}

/// The pointer comes into the area's view from the Top Bar, moves there
/// and rests (what took the keys back before the review's fix).
fn over_the_view(h: &mut H, area: ResRef) {
    let r = h.state().area_views[&area].rect;
    h.hover_at(egui::pos2(r.center().x, 4.0));
    h.run();
    for (x, y) in [(0.0, 0.0), (30.0, 20.0), (-40.0, 35.0), (-40.0, 35.0)] {
        h.hover_at(r.center() + egui::vec2(x, y));
        h.run();
    }
}

/// The dock list in front holds the keys with the pointer over `area`'s
/// view: Down chooses its first row (named from `first`), Down again the
/// next (`second`, where there is one); Ctrl + C copies the one chosen
/// (`copied`, where the list copies); the Status Bar's hints are the
/// list's, not the view's. A press in the view gives the view the keys
/// back: Down then leaves the row where it is.
fn holds_the_keys(
    h: &mut H,
    area: ResRef,
    first: &str,
    second: Option<&str>,
    copied: Option<&str>,
) {
    over_the_view(h, area);
    let before = chosen_in_dock(h);
    let rows = |h: &H| -> Vec<String> {
        chosen_in_dock(h).into_iter().filter(|c| !before.contains(c)).collect()
    };
    key(h, egui::Key::ArrowDown);
    let now = rows(h);
    assert!(now.len() == 1 && now[0].starts_with(first), "Down chose {first}: {now:?}");
    if let Some(second) = second {
        over_the_view(h, area);
        key(h, egui::Key::ArrowDown);
        let now = rows(h);
        assert!(now.len() == 1 && now[0].starts_with(second), "Down chose {second}: {now:?}");
    }
    if let Some(words) = copied {
        let (got, _) = press_now(h, egui::Modifiers::COMMAND, egui::Key::C);
        assert_eq!(got.len(), 1, "Ctrl + C copied the row: {got:?}");
        assert!(got[0].starts_with(words), "copied {words}: {got:?}");
    }
    let said = h.state().status_said();
    assert!(said.iter().any(|s| s == MOVE_IN_THE_LIST), "the list's hints: {said:?}");
    assert!(!said.iter().any(|s| AREA_HINTS.contains(&s.as_str())), "no view's hints: {said:?}");
    // A press in the view: the keys the view's.
    let at = rows(h);
    click_pos(h, h.state().area_views[&area].rect.center());
    h.run();
    key(h, egui::Key::ArrowDown);
    assert_eq!(rows(h), at, "the view's keys after a press in it");
}

/// The Status Bar's words for the dock lists' arrows.
const MOVE_IN_THE_LIST: &str = "Up or Down: Move in the list";
/// What the area's view's hints say with the keys its (none of them while
/// a dock list holds them).
const AREA_HINTS: &[&str] = &["Click: Select", "Drag: Move or box", "Ctrl, Drag: Move view"];

#[test]
fn resources_used_by_its_command_holds_the_keys_over_the_view() {
    let Some((mut h, area)) = area_harness("u28f-used-held") else { return };
    settle(&mut h);
    menu_command(&mut h, "Build", "Area Statistics");
    settle(&mut h);
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::ResourcesUsed));
    holds_the_keys(&mut h, area, "Tiles", Some("Objects"), Some("Objects\t2"));
}

#[test]
fn the_hak_report_by_its_command_holds_the_keys_over_the_view() {
    let Some((mut h, area)) = area_harness("u28f-hak-held") else { return };
    settle(&mut h);
    over_the_view(&mut h, area);
    // (What Build › Hak Pak Conflict Analysis' job ends with.)
    h.state_mut().show_hak_report(hak_report(10));
    settle(&mut h);
    let first = "tdc01_x00000.mdl";
    holds_the_keys(&mut h, area, first, Some("tdc01_x00001.mdl"), Some("tdc01_x00001.mdl\t"));
}

#[test]
fn the_claims_table_by_its_command_holds_the_keys_over_the_view() {
    let Some((mut h, area)) = read_only_area_harness("u28f-claims-held") else { return };
    settle(&mut h);
    menu_command(&mut h, "Edit", "Claims Table…");
    settle(&mut h);
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::ClaimsTable));
    let name = area.to_string();
    holds_the_keys(&mut h, area, &name, None, Some(&format!("{name}\t")));
}

#[test]
fn find_instance_by_its_command_holds_the_keys_over_the_view() {
    let Some((mut h, area)) = area_harness("u28f-find-held") else { return };
    settle(&mut h);
    menu_command(&mut h, "Edit", "Find Instance…");
    key(&mut h, egui::Key::Enter);
    settle(&mut h);
    assert_eq!(h.state().find_instance.as_ref().map(|f| f.results.len()), Some(2));
    holds_the_keys(&mut h, area, "Waypoint ", Some("Waypoint "), Some("Waypoint "));
}

/// Two waypoint blueprints, `camp_a` and `camp_b`, the game's
/// `nw_waypoint001` named "Waypoint camp", in the module; and the area's
/// OnEnter and OnExit naming scripts it does not have (Verify's findings).
fn waypoints_and_gone_scripts(m: &mut Module, game: &mg_rules::GameData, area: ResRef) {
    let bp = game.resman.get(&ResKey::parse("nw_waypoint001", ResType::UTW).unwrap()).unwrap();
    for name in ["camp_a", "camp_b"] {
        let mut gff = Gff::read(&bp).unwrap();
        let words = LocString::from_text(Language::ENGLISH, Gender::Male, "Waypoint camp");
        gff.root.set("LocalizedName", mg_gff::Value::LocString(words));
        m.set_gff(ResKey::parse(name, ResType::UTW).unwrap(), &gff).unwrap();
    }
    let key = ResKey::new(area, ResType::ARE);
    let mut are = m.gff(&key).unwrap().unwrap();
    for (field, script) in [("OnEnter", "gone_a"), ("OnExit", "gone_b")] {
        are.root.set(field, mg_gff::Value::ResRef(script.as_bytes().to_vec()));
    }
    m.set_gff(key, &are).unwrap();
}

#[test]
fn find_and_replace_text_by_its_command_holds_the_keys_over_the_view() {
    let prepare = &waypoints_and_gone_scripts;
    let Some((mut h, area)) = area_harness_with("u28f-text-held", "ttr01", None, 4, None, prepare)
    else {
        return;
    };
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::H);
    settle(&mut h);
    typed(&mut h, "Waypoint");
    key(&mut h, egui::Key::Enter);
    settle(&mut h);
    let hits = h.state().text_replace.as_ref().map_or(0, |d| d.hits.len());
    assert!(
        hits >= 2,
        "{hits} strings found for {:?}",
        h.state().text_replace.as_ref().map(|d| d.find.clone())
    );
    // (Space ticks the one chosen while the keys are held.)
    over_the_view(&mut h, area);
    key(&mut h, egui::Key::ArrowDown);
    let ticked =
        |h: &H| h.state().text_replace.as_ref().unwrap().hits.iter().filter(|t| t.1).count();
    let was = ticked(&h);
    over_the_view(&mut h, area);
    key(&mut h, egui::Key::Space);
    assert_eq!(ticked(&h), was - 1, "Space unticked the one chosen, the pointer over the view");
    key(&mut h, egui::Key::Space);
    key(&mut h, egui::Key::ArrowUp);
    holds_the_keys(&mut h, area, "camp_", Some("camp_"), None);
}

#[test]
fn verify_s_problems_by_its_command_hold_the_keys_over_the_view() {
    let prepare = &waypoints_and_gone_scripts;
    let Some((mut h, area)) =
        area_harness_with("u28f-verify-held", "ttr01", None, 4, None, prepare)
    else {
        return;
    };
    settle(&mut h);
    menu_command(&mut h, "Build", "Verify Module");
    settle(&mut h);
    assert_eq!(h.state().bottom_dock_shows(), Some(Panel::Problems));
    // (Words Problems' font has: no arrow, drawn as a box.)
    let rows = rows_named(&h, "Error: ");
    assert!(
        !rows.is_empty() && rows.iter().all(|r| !r.contains('→') && r.contains(" › ")),
        "{rows:?}"
    );
    // (Verify writes no file: its line in the log says where its findings
    // are, 5.2's W4.)
    let log = &h.state().log.entries;
    let said = log.iter().rev().find(|(_, m)| m.starts_with("Verify: ")).map(|(_, m)| m.clone());
    assert!(said.as_deref().is_some_and(|m| m.ends_with(mg_ui::problems::LISTED)), "{said:?}");
    holds_the_keys(&mut h, area, "Error: ", Some("Error: "), None);
}

/// Whether the row named `label` is drawn whole inside the Bottom Dock.
fn row_in_sight(h: &H, label: &str) -> bool {
    let dock = h.state().bottom_dock_rect().unwrap();
    h.query_all_by(|n| n.label().is_some_and(|l| l == label))
        .any(|n| n.accesskit_node().bounding_box().is_some() && dock.contains_rect(n.rect()))
}

/// At 50,000 rows the keys that move far (End, Home, Page Down three
/// times) bring the row they choose into sight: Find Instance's hits, the
/// hak report (`report_table`) and Problems' findings.
#[test]
fn far_keys_at_fifty_thousand_bring_the_chosen_row_into_sight() {
    let mut h = objects("u28f-far-keys");
    let mut f = mg_ui::area_tools::FindInstance::default();
    f.results = (0..50_000)
        .map(|index| mg_ui::area_tools::Found {
            kind: mg_area::ObjectKind::Placeable,
            index,
            area: ResRef::from_str("a1").unwrap(),
            tag: format!("TREE_{index}"),
            template: "mg_tree".into(),
        })
        .collect();
    f.searched = true;
    h.state_mut().find_instance = Some(f);
    menu_command(&mut h, "Edit", "Find Instance…");
    dock_high(&mut h, 320);
    settle(&mut h);
    in_dock(&h, "Placeable TREE_0").click();
    h.run();
    let hit =
        |h: &H| format!("Placeable TREE_{}", h.state().search_results.instance_chosen().unwrap());
    for (k, n) in [(egui::Key::End, 1), (egui::Key::Home, 1), (egui::Key::PageDown, 3)] {
        for _ in 0..n {
            key(&mut h, k);
        }
        settle(&mut h);
        let label = hit(&h);
        assert!(row_in_sight(&h, &label), "{k:?}: Find Instance's {label} in sight");
    }
    assert!(h.state().search_results.instance_chosen().unwrap() > 20, "three pages on");
    // The hak report.
    h.state_mut().show_hak_report(hak_report(50_000));
    settle(&mut h);
    in_dock(&h, "tdc01_x00000.mdl").click();
    h.run();
    let chosen_hak = |h: &H| {
        chosen_in_dock(h).into_iter().filter(|c| c.starts_with("tdc01_x")).collect::<Vec<_>>()
    };
    key(&mut h, egui::Key::End);
    settle(&mut h);
    assert!(row_in_sight(&h, "tdc01_x49999.mdl"), "End: the hak report's last in sight");
    assert_eq!(chosen_hak(&h), ["tdc01_x49999.mdl"]);
    key(&mut h, egui::Key::Home);
    settle(&mut h);
    assert_eq!(chosen_hak(&h), ["tdc01_x00000.mdl"], "Home: the first in sight");
    for _ in 0..3 {
        key(&mut h, egui::Key::PageDown);
    }
    settle(&mut h);
    let now = chosen_hak(&h);
    assert!(now.len() == 1 && now[0].as_str() > "tdc01_x00020", "three pages on: {now:?}");
    assert!(row_in_sight(&h, &now[0]), "Page Down: {now:?} in sight");
    // Problems.
    let hello = ResKey::parse("hello", ResType::NSS).unwrap();
    let many: Vec<Problem> = (0..50_000)
        .map(|i| {
            let s = mg_module::doctor::Severity::Warning;
            Problem::new(s, "hello.nss".into(), format!("warning {i}"), Some(Go::Resource(hello)))
        })
        .collect();
    let now = h.state().ws.as_ref().map(|ws| ws.revision());
    h.state_mut().problems.replace(Source::Build, many, now);
    h.get_by_label("Status Bar").get_by_label("Problems").click();
    settle(&mut h);
    in_dock(&h, "Warning: hello.nss, warning 0").click();
    h.run();
    let finding = |h: &H| {
        let m = h.state().problems.selected().map(|p| p.message.clone()).unwrap_or_default();
        format!("Warning: hello.nss, {m}")
    };
    for (k, n) in [(egui::Key::End, 1), (egui::Key::Home, 1), (egui::Key::PageDown, 3)] {
        for _ in 0..n {
            key(&mut h, k);
        }
        settle(&mut h);
        let label = finding(&h);
        assert!(row_in_sight(&h, &label), "{k:?}: Problems' {label} in sight");
    }
    assert_ne!(finding(&h), "Warning: hello.nss, warning 0", "three pages on");
}

/// The arrows, Home and End typed in a form's field are the field's: the
/// table's row chosen stays where it is (Find Instance's With Tag; Find
/// and Replace Text's Find what).
#[test]
fn arrows_in_a_form_field_leave_the_table_s_row() {
    let mut h = objects("u28f-arrows-field");
    find(&mut h, "");
    dock_high(&mut h, 360);
    settle(&mut h);
    in_dock(&h, "Creature GUARD").click();
    h.run();
    assert_eq!(h.state().search_results.instance_chosen(), Some(0));
    h.query_all_by(|n| n.placeholder() == Some("tag")).next().expect("With Tag").click();
    h.run();
    for k in [egui::Key::ArrowDown, egui::Key::ArrowDown, egui::Key::End, egui::Key::PageDown] {
        key(&mut h, k);
    }
    assert_eq!(h.state().search_results.instance_chosen(), Some(0), "the field's keys");
    // Find and Replace Text.
    let mut h = camps("u28f-arrows-text", 6);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::H);
    settle(&mut h);
    typed(&mut h, "orc");
    key(&mut h, egui::Key::Enter);
    dock_high(&mut h, 420);
    settle(&mut h);
    let first = rows_named(&h, "camp_00000.utw › LocalizedName: ");
    in_dock(&h, &first[0]).click();
    h.run();
    let find_what = h.query_all_by(|n| n.value().is_some_and(|v| v == "orc")).next();
    find_what.expect("Find what").click();
    h.run();
    for k in [egui::Key::ArrowDown, egui::Key::ArrowDown, egui::Key::End] {
        key(&mut h, k);
    }
    assert_eq!(chosen_string(&h).as_deref(), Some(first[0].as_str()), "the field's keys");
}

/// A dock list of the review's: how it is put in front with the keys
/// (a click on a row, or on what it says where a click on a row would act).
#[derive(Clone, Copy, Debug)]
enum List {
    Instances,
    Text,
    Lines,
    Problems,
    Hak,
    Used,
    Claims,
}

/// What is over the dock: one of the window's menus (View), a kit popover
/// (the area header's More), egui's popup Counted as Used (Problems'
/// alone), a dialog (About Moonglow Toolset, opened as its command opens
/// it).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Over {
    Menu,
    Popover,
    Counted,
    Dialog,
}

/// `list` in front of the Bottom Dock with the keys, in `h` over `area`.
fn list_in_front(h: &mut H, list: List) {
    let click_first = |h: &mut H, starts: &str| {
        let rows = rows_named(h, starts);
        let first = rows.first().unwrap_or_else(|| panic!("a row {starts}"));
        in_dock(h, first).click();
        h.run();
    };
    match list {
        List::Instances => {
            menu_command(h, "Edit", "Find Instance…");
            key(h, egui::Key::Enter);
            settle(h);
            click_first(h, "Waypoint ");
        }
        List::Text => {
            h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::H);
            settle(h);
            typed(h, "Waypoint");
            key(h, egui::Key::Enter);
            settle(h);
            click_first(h, "camp_");
        }
        List::Lines => {
            let script = ResKey::parse("hello", ResType::NSS).unwrap();
            let hits = (0..6)
                .map(|line| mg_ui::search_results::Hit::Line {
                    script,
                    line,
                    text: format!("int n{line} = 1;"),
                })
                .collect();
            let r = &mut h.state_mut().search_results;
            (r.what, r.hits) = ("\"int\": 6 lines found".into(), hits);
            r.showing = mg_ui::search_results::Showing::Lines;
            h.state_mut().settings.hide_log = false;
            h.get_by_label("Status Bar").get_by_label("Search Results").click();
            settle(h);
            // (A click on a line opens it: the keys by a click on what was
            // searched.)
            in_dock(h, "\"int\": 6 lines found").click();
            h.run();
        }
        List::Problems => {
            h.state_mut().problems.build_unused = true;
            built(h);
            click_first(h, "Error: ");
        }
        List::Hak => {
            h.state_mut().show_hak_report(hak_report(10));
            settle(h);
            click_first(h, "tdc01_x");
        }
        List::Used => {
            menu_command(h, "Build", "Area Statistics");
            settle(h);
            click_first(h, "Tiles");
        }
        List::Claims => {
            menu_command(h, "Edit", "Claims Table…");
            settle(h);
            let area = h.state().area_stats.or(h.state().area_views.keys().next().copied());
            in_dock(h, &area.unwrap().to_string()).click();
            h.run();
        }
    }
}

/// Two more rows of the Claims Table (areas the module does not have).
const CLAIMS_MORE: &str = "1   kz_one           cd       claimed  2026-10-20  ****\n\
                           2   kz_two           ab       claimed  2026-10-20  ****\n";

/// `over` opened over the dock.
fn open_over(h: &mut H, over: Over) {
    match over {
        Over::Menu => {
            h.get_by_label("Top Bar").get_by_label("View").click();
            h.run();
        }
        Over::Popover => {
            area_tool(h, "Fog");
            h.run();
            let open = mg_uikit::header::records(&h.ctx, AREA_HEADER)[0].open.clone();
            assert!(open.is_some(), "the area header's popover open");
        }
        Over::Counted => {
            in_dock(h, mg_ui::problems::COUNTED_AS_USED).click();
            h.run();
        }
        Over::Dialog => {
            // (As its command opens it: a click on the menu's row is a
            // press outside the list, and the keys would leave it before
            // the dialog is up.)
            h.state_mut().about = true;
            settle(h);
            assert!(h.query_by_label("About Moonglow Toolset").is_some(), "About open");
        }
    }
}

/// No dock list takes a key under a menu, a popover (Counted as Used's
/// included) or a dialog: Down and End leave its row where it is, and
/// with what was over it closed, Down moves it again (B4, B5, B8, B9 of
/// the review). Problems, Find Instance's hits, the strings, a script's
/// lines, the hak report, Resources Used and the Claims Table.
#[test]
fn no_dock_list_takes_a_key_under_a_menu_a_popover_or_a_dialog() {
    let lists = [
        List::Instances,
        List::Text,
        List::Lines,
        List::Problems,
        List::Hak,
        List::Used,
        List::Claims,
    ];
    let mut taken = Vec::new();
    for list in lists {
        let mut overs = vec![Over::Menu, Over::Popover, Over::Dialog];
        if matches!(list, List::Problems) {
            overs.push(Over::Counted);
        }
        for over in overs {
            let name = format!("u28f-under-{list:?}-{over:?}").to_lowercase();
            let made = match list {
                List::Claims => read_only_area_ui_harness_with(&name, CLAIMS_MORE),
                _ => {
                    area_ui_harness_with(&name, "ttr01", None, 4, None, &waypoints_and_gone_scripts)
                }
            };
            let Some((mut h, _)) = made else { return };
            settle(&mut h);
            list_in_front(&mut h, list);
            // (Find Instance's two waypoints have one name: its row by its
            // place.)
            let chosen_now = |h: &H| match list {
                List::Instances => {
                    vec![format!("{:?}", h.state().search_results.instance_chosen())]
                }
                _ => chosen_in_dock(h),
            };
            // (Two rows down from the first chosen, so that Up and Home
            // under what is over the list would move it.)
            let first = chosen_now(&h);
            key(&mut h, egui::Key::ArrowDown);
            key(&mut h, egui::Key::ArrowDown);
            let chosen = chosen_now(&h);
            assert_ne!(chosen, first, "{list:?}: Down moved the row before {over:?}");
            open_over(&mut h, over);
            for k in [egui::Key::ArrowUp, egui::Key::ArrowUp, egui::Key::Home] {
                h.key_press(k);
                h.run();
            }
            if chosen_now(&h) != chosen {
                taken.push(format!("{list:?} under {over:?}"));
            }
        }
    }
    assert!(taken.is_empty(), "keys taken: {taken:?}");
}

/// At the least window (420 by 320) with a form taller than half the
/// window (Find Instance's, wrapped), the Bottom Dock is no taller than
/// half the window (B6 of the review).
#[test]
fn the_dock_s_need_is_capped_at_half_the_window() {
    let mut h = objects("u28f-need-cap");
    h.set_size(egui::vec2(420.0, 320.0));
    h.state_mut().settings.bottom_dock_height = Some(0);
    find(&mut h, "");
    settle(&mut h);
    let form = h.state().search_results.form_rect().expect("the form drawn");
    let dock = h.state().bottom_dock_rect().unwrap();
    assert!(dock.height() <= 160.5, "the dock {dock:?} at most half of 320");
    assert!(form.height() < dock.height(), "the form scrolls in the dock: {form:?} in {dock:?}");
}

/// The controls of Find Instance's form, by their names or hints.
fn form_controls(h: &H) -> Vec<(String, egui::Rect)> {
    let names: Vec<String> = mg_area::ObjectKind::ALL
        .iter()
        .map(|k| k.label().to_string())
        .chain(["Search", "Clear", "(all areas)"].map(String::from))
        .collect();
    let dock = h.state().bottom_dock_rect().unwrap();
    h.query_all_by(|n| {
        n.label().is_some_and(|l| names.contains(&l))
            || n.placeholder().is_some_and(|p| p == "blueprint resref" || p == "tag")
    })
    .filter(|n| n.accesskit_node().bounding_box().is_some())
    .filter(|n| n.rect().left() >= dock.left() && n.rect().right() <= dock.right())
    .map(|n| {
        let a = n.accesskit_node();
        (a.label().or(a.placeholder().map(String::from)).unwrap_or_default(), n.rect())
    })
    .collect()
}

/// Each of the form's controls whole inside the form's scroll (in y) or
/// out of it, never cut; the dock's first hit whole under it.
fn form_whole_lines(h: &H, at: &str) {
    let form = h.state().search_results.form_rect().expect("the form drawn");
    let dock = h.state().bottom_dock_rect().unwrap();
    let cut: Vec<_> = form_controls(h)
        .into_iter()
        .filter(|(_, r)| {
            let inside = r.top() >= form.top() - 0.5 && r.bottom() <= form.bottom() + 0.5;
            let out = r.bottom() <= form.top() + 0.5 || r.top() >= form.bottom() - 0.5;
            !inside && !out
        })
        .collect();
    assert!(cut.is_empty(), "{at}: controls cut by the form's edge {form:?}: {cut:?}");
    let row = in_dock(h, "Creature GUARD").rect();
    assert!(
        row.top() >= form.bottom() - 0.5 && row.bottom() <= dock.bottom() + 0.5,
        "{at}: the first hit {row:?} whole under the form {form:?} in {dock:?}"
    );
}

/// The least window: Find Instance's form scrolls by whole lines (none
/// cut at rest, after a scroll that stops between two), a field or button
/// given the keys by Tab is brought wholly into sight, and the first hit
/// shows whole under it (B10 of the review).
#[test]
fn the_least_window_s_form_scrolls_by_whole_lines() {
    for scale in [100, 125, 150] {
        least_window_lines(scale);
    }
}

/// [`the_least_window_s_form_scrolls_by_whole_lines`] at the interface
/// size `scale` (Options › General).
fn least_window_lines(scale: u16) {
    let mut h = objects(&format!("u28f-least-lines-{scale}"));
    h.state_mut().settings.ui_scale = (scale != 100).then_some(scale);
    h.set_size(egui::vec2(420.0, 320.0));
    h.state_mut().settings.bottom_dock_height = Some(0);
    find(&mut h, "");
    settle(&mut h);
    form_whole_lines(&h, &format!("{scale}%, at rest"));
    // A wheel's scroll that stops between two lines: on to the next.
    let form = h.state().search_results.form_rect().unwrap();
    h.hover_at(form.center());
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, -11.0),
        source: egui::MouseWheelSource::Unknown,
        modifiers: egui::Modifiers::NONE,
        phase: egui::TouchPhase::Move,
    });
    settle(&mut h);
    rest(&mut h);
    form_whole_lines(&h, &format!("{scale}%, after a scroll of 11 points"));
    // Tab through the form from where its command puts the keys (From
    // Blueprint), then Shift + Tab back over its lines: each control given
    // the keys whole in sight.
    menu_command(&mut h, "Edit", "Find Instance…");
    settle(&mut h);
    let mut seen = 0;
    let steps = [(egui::Modifiers::NONE, 3), (egui::Modifiers::SHIFT, 12)];
    for (n, modifiers) in steps.iter().flat_map(|(m, n)| std::iter::repeat_n(*m, *n)).enumerate() {
        h.key_press_modifiers(modifiers, egui::Key::Tab);
        settle(&mut h);
        rest(&mut h);
        let Some(r) = h.query_all_by(|n| n.is_focused()).next().map(|n| n.rect()) else { continue };
        let form = h.state().search_results.form_rect().unwrap();
        if form_controls(&h).iter().any(|(_, c)| *c == r) {
            seen += 1;
            assert!(
                r.top() >= form.top() - 0.5 && r.bottom() <= form.bottom() + 0.5,
                "{scale}%, Tab {n}: the control {r:?} whole in the form {form:?}"
            );
        }
        form_whole_lines(&h, &format!("{scale}%, Tab {n}"));
    }
    assert!(seen >= 6, "Tab went through the form's controls ({seen})");
}

/// While Verify Module runs, Problems says so (sentence case), not "No
/// problems found"; after a run that found none, it says that.
#[test]
fn problems_says_a_check_is_running_and_no_problems_only_after_it() {
    let mut h = objects("u28f-running");
    h.state_mut().settings.hide_log = false;
    h.get_by_label("Status Bar").get_by_label("Problems").click();
    settle(&mut h);
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    h.state_mut().start_job_for_tests("Verify Module", move |_| {
        let _ = rx.recv();
    });
    // (Stepped, not run: a job under way asks to be drawn again.)
    h.run_steps(3);
    let running = mg_ui::problems::running_words(&Source::Verify);
    assert_eq!(running, "Verify Module is running");
    in_dock(&h, &running);
    let nothing = |h: &H| {
        let dock = h.state().bottom_dock_rect().unwrap();
        h.query_all_by(|n| n.label().is_some_and(|l| l.contains(mg_ui::problems::NOTHING)))
            .any(|n| dock.contains_rect(n.rect()))
    };
    assert!(!nothing(&h), "not \"No problems found\" while it runs");
    tx.send(()).unwrap();
    settle(&mut h);
    let now = h.state().ws.as_ref().map(|ws| ws.revision());
    h.state_mut().problems.replace(Source::Verify, Vec::new(), now);
    settle(&mut h);
    assert!(nothing(&h), "a run that found none: \"No problems found\"");
    assert!(h.query_by_label(&running).is_none(), "no longer running");
}

/// Build's Counted as Used is in Problems' heading only where the build
/// shown was asked for its Unused results (the tip's dialog showed it with
/// Unused ticked); never in Verify's.
#[test]
fn counted_as_used_shows_only_for_a_build_asked_for_unused() {
    let dir = mg_testkit::scratch_dir("u28f-counted");
    let mut app = app_with(Vec::new());
    app.open_module(&sample_module(&dir));
    let mut h = harness(app, 1280.0, 720.0);
    settle(&mut h);
    let counted = |h: &H| h.query_by_label(mg_ui::problems::COUNTED_AS_USED).is_some();
    built(&mut h);
    assert!(!counted(&h), "a build without Unused: no Counted as Used");
    h.state_mut().problems.build_unused = true;
    settle(&mut h);
    assert!(counted(&h), "a build with Unused: Counted as Used");
    let now = h.state().ws.as_ref().map(|ws| ws.revision());
    h.state_mut().problems.replace(Source::Build, Vec::new(), now);
    let p = Problem::new(mg_module::doctor::Severity::Error, "x".into(), "bad".into(), None);
    h.state_mut().problems.replace(Source::Verify, vec![p], now);
    settle(&mut h);
    in_dock(&h, "Verify Module");
    let headings = rows_named(&h, "Build Module").len();
    assert!(!counted(&h) || headings > 0, "Verify's heading has no Counted as Used");
}

/// Problems' counts are said once, in the tab bar's place (the board's),
/// not again in a heading, which keeps its source's name; they agree with
/// the Status Bar's badge with one source, two, and a stale one.
#[test]
fn problems_counts_are_said_once_and_agree_with_the_badge() {
    let dir = mg_testkit::scratch_dir("u28f-counts-once");
    let mut app = app_with(Vec::new());
    app.open_module(&sample_module(&dir));
    let mut h = harness(app, 1280.0, 720.0);
    settle(&mut h);
    built(&mut h);
    let said = |h: &H, words: &str| {
        h.query_all_by(|n| {
            n.label().is_some_and(|l| l.contains(words))
                || n.value().is_some_and(|v| v.contains(words))
        })
        .filter_map(|n| n.accesskit_node().bounding_box())
        // (A label and its text run: one place.)
        .map(|r| (r.x0 as i32, r.y0 as i32))
        .collect::<std::collections::BTreeSet<_>>()
        .len()
    };
    let badge = |h: &H| h.state().switch_badge(Panel::Problems).map(|b| b.0);
    in_dock(&h, "Build Module");
    assert_eq!(said(&h, "1 error · 2 warnings"), 1, "the counts said once");
    assert_eq!(badge(&h).as_deref(), Some("3"));
    // A second source.
    let now = h.state().ws.as_ref().map(|ws| ws.revision());
    let p = Problem::new(mg_module::doctor::Severity::Error, "x".into(), "bad".into(), None);
    h.state_mut().problems.replace(Source::Verify, vec![p], now);
    settle(&mut h);
    in_dock(&h, "Verify Module");
    assert_eq!(said(&h, "2 errors · 2 warnings"), 1, "both sources' counts, once");
    assert_eq!(said(&h, "1 error · 2 warnings"), 0, "no heading's own count");
    assert_eq!(badge(&h).as_deref(), Some("4"));
    // Stale: none counted.
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "change",
        vec![mg_edit::Edit::SetResource {
            key: ResKey::parse("other", ResType::NSS).unwrap(),
            data: Some(b"void main() {}".to_vec()),
        }],
    )));
    settle(&mut h);
    assert_eq!(badge(&h), None);
    assert_eq!(said(&h, " error"), 0, "nothing counted, nothing said");
}

/// Find Instance's count: "No objects found", "1 object found", "120
/// objects found"; on the form's line, unwrapped, at 1280.
#[test]
fn find_instance_says_how_many_it_found_on_the_form_s_line() {
    use mg_ui::search_results::instances_words;
    assert_eq!(instances_words(0), "No objects found");
    assert_eq!(instances_words(1), "1 object found");
    assert_eq!(instances_words(120), "120 objects found");
    let mut h = objects("u28f-found-words");
    find(&mut h, "");
    let words = in_dock(&h, "6 objects found").rect();
    let search = in_dock(&h, "Search").rect();
    assert!((words.center().y - search.center().y).abs() < 4.0, "{words:?} on {search:?}'s line");
    assert!(words.height() < 24.0, "one line: {words:?}");
    assert!(h.query_by_label_contains("double click").is_none(), "no instructions in the count");
    find(&mut h, "no_such");
    in_dock(&h, "No objects found");
}

/// While a dock list holds the keys the Status Bar's hints are its own,
/// each true now: Find Instance's Go to and Copy with a row chosen, the
/// strings' Tick, Problems' Mark as used on an Unused finding, Copy where
/// one is chosen (5.2's W4); with no rows none of them.
#[test]
fn each_dock_list_s_hints_are_true_of_it_now() {
    let mut h = objects("u28f-hints");
    let said = |h: &H| h.state().status_said();
    let has = |h: &H, w: &str| said(h).iter().any(|s| s == w);
    find(&mut h, "no_such");
    assert!(!has(&h, MOVE_IN_THE_LIST), "no rows, no hints: {:?}", said(&h));
    in_dock(&h, "Clear").click();
    settle(&mut h);
    find(&mut h, "");
    in_dock(&h, "Creature GUARD").click();
    h.run();
    for w in [MOVE_IN_THE_LIST, "Enter: Go to", "Double Click: Go to", "Ctrl, C: Copy"] {
        assert!(has(&h, w), "Find Instance: {w} in {:?}", said(&h));
    }
    // Problems: Mark as used on the Unused finding alone.
    built(&mut h);
    in_dock(&h, "Error: hello.nss, bad").click();
    h.run();
    assert!(has(&h, "Enter: Go to") && !has(&h, "Right Click: Mark as used"), "{:?}", said(&h));
    assert!(has(&h, "Ctrl, C: Copy"), "Problems: Copy (5.2's W4) in {:?}", said(&h));
    in_dock(&h, "Warning: hello.nss, Unused (source and compiled)").click();
    h.run();
    assert!(has(&h, "Right Click: Mark as used"), "{:?}", said(&h));
    // The strings: Tick.
    let mut h = camps("u28f-hints-text", 3);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::H);
    settle(&mut h);
    typed(&mut h, "orc");
    key(&mut h, egui::Key::Enter);
    settle(&mut h);
    let first = rows_named(&h, "camp_00000.utw › LocalizedName: ");
    in_dock(&h, &first[0]).click();
    h.run();
    for w in [MOVE_IN_THE_LIST, "Space: Check", "Enter: Go to", "Ctrl, C: Copy"] {
        assert!(has(&h, w), "the strings: {w} in {:?}", said(&h));
    }
}

/// Find Instance's In Area list is made with the module's areas, as the
/// module changes: not again at rest.
#[test]
fn find_instance_s_area_list_is_made_as_the_module_changes() {
    let mut h = objects("u28f-area-list");
    find(&mut h, "");
    let builds = h.state().search_results.area_list_builds();
    for _ in 0..10 {
        h.run();
    }
    assert_eq!(h.state().search_results.area_list_builds(), builds, "not made again at rest");
    h.state_mut().actions.push(mg_ui::Action::Apply(mg_edit::Command::new(
        "Name",
        vec![mg_edit::Edit::SetField {
            key: ResKey::parse("a1", ResType::ARE).unwrap(),
            path: mg_edit::GffPath::root(),
            label: "Name".into(),
            value: Some(mg_gff::Value::LocString(LocString::from_text(
                Language::ENGLISH,
                Gender::Male,
                "Glade",
            ))),
        }],
    )));
    settle(&mut h);
    assert_eq!(h.state().search_results.area_list_builds(), builds + 1, "made again once");
}

/// The hak report's header: the view switch alone at the left, What the
/// Module Uses and Copy Report set apart from it (a gap of the header's,
/// or a row of their own), at 1280 and at 420.
#[test]
fn the_hak_report_s_buttons_stand_apart_from_its_view_switch() {
    for width in [1280.0, 420.0] {
        let dir = mg_testkit::scratch_dir("u28f-hak-header");
        let mut app = app_with(Vec::new());
        app.open_module(&sample_module(&dir));
        let mut h = harness(app, width, 720.0);
        settle(&mut h);
        h.state_mut().show_hak_report(hak_report(10));
        dock_high(&mut h, 360);
        settle(&mut h);
        let seg = ["Conflicts (10)", "Overrides (2)", "Haks (5)"].map(|s| in_dock(&h, s).rect());
        let last = seg.iter().copied().reduce(|a, b| a.union(b)).unwrap();
        for b in ["What the Module Uses", "Copy Report"] {
            let r = in_dock(&h, b).rect();
            let apart = r.left() >= last.right() + 15.5 || r.top() >= last.bottom() - 0.5;
            assert!(apart, "{width}: {b} {r:?} apart from the switch {last:?}");
        }
    }
}

/// The Claims Table at 420 wide, at the three zooms: each row's actions
/// (Release, Hand To…, Take Over…, Claim) whole inside the dock at rest,
/// the other columns narrowed first; where they still did not fit, one
/// Actions button a row whose menu holds them by the same names (checked
/// whole where it shows).
#[test]
fn the_claims_table_s_actions_are_never_cut_at_420() {
    use mg_ui::dock_reports::{ACTIONS, CLAIM, HAND_TO, RELEASE, TAKE_OVER};
    let table = format!(
        "{}1   kz_mine          ab       claimed  2026-10-01  ****\n\
         2   kz_free          ****     free     2026-10-01  ****\n",
        claims_table("cd", "claimed")
    );
    let (_, app) = claimed_project("u28f-claims-420", "[claims]\nfile = \"claims.2da\"\n", &table);
    let mut h = harness(app, 420.0, 720.0);
    settle(&mut h);
    menu_command(&mut h, "Edit", "Claims Table…");
    let mut folded_at = Vec::new();
    for scale in [100, 125, 150] {
        h.state_mut().settings.ui_scale = (scale != 100).then_some(scale);
        h.run_steps(3);
        dock_high(&mut h, 320);
        settle(&mut h);
        let dock = h.state().bottom_dock_rect().unwrap();
        let folded = h.query_all_by(|n| n.label() == Some(ACTIONS.into())).count();
        if folded == 0 {
            for name in [RELEASE, HAND_TO, TAKE_OVER, CLAIM] {
                let r = in_dock(&h, name).rect();
                assert!(dock.contains_rect(r), "{scale}%: {name} {r:?} whole in {dock:?}");
            }
        } else {
            folded_at.push(scale);
            assert_eq!(folded, 3, "{scale}%: one Actions button a row");
            let menus = h
                .query_all_by(|n| n.label() == Some(ACTIONS.into()))
                .map(|n| n.rect())
                .collect::<Vec<_>>();
            for r in &menus {
                assert!(dock.contains_rect(*r), "{scale}%: Actions {r:?} whole in {dock:?}");
            }
            // (Each row's menu: its own actions by their names.)
            let mut named = Vec::new();
            for r in menus {
                click_pos(&h, r.center());
                h.run();
                for name in [RELEASE, HAND_TO, TAKE_OVER, CLAIM] {
                    if h.query_by_label(name).is_some() {
                        named.push(name);
                    }
                }
                key(&mut h, egui::Key::Escape);
                if h.state().settings.hide_log {
                    h.state_mut().settings.hide_log = false;
                    settle(&mut h);
                }
            }
            named.sort();
            let mut all = vec![RELEASE, HAND_TO, TAKE_OVER, CLAIM];
            all.sort();
            assert_eq!(named, all, "{scale}%: the actions in the rows' menus");
        }
    }
    // (The least window is 420 points wide at every zoom: the narrowed
    // columns fit there, and the folded menu is for a room narrower than
    // the window allows, `claim_columns`' unit test.)
    assert!(folded_at.is_empty(), "the actions fit narrowed at 420: {folded_at:?} folded");
}

/// The focused node's rect, if one has the focus.
fn focused_rect(h: &H) -> Option<egui::Rect> {
    h.query_all_by(|n| n.is_focused()).next().map(|n| n.rect())
}

/// Find Instance by the keyboard alone: Tab from the form reaches the
/// table's heads, Down chooses a row there, Shift + Tab goes back into
/// the form (not to the window's last control), Tab comes back to the
/// heads, and Enter goes to the chosen row (M4's U28 review: a head's
/// focus had kept the keys from the rows, and Shift + Tab from them
/// went to the Top Bar).
#[test]
fn find_instance_s_form_and_rows_by_the_keyboard_alone() {
    let mut h = objects("u28f-tab-order");
    find(&mut h, "");
    dock_high(&mut h, 360);
    menu_command(&mut h, "Edit", "Find Instance…");
    settle(&mut h);
    let head = |h: &H| {
        h.query_all_by(|n| n.is_focused())
            .any(|n| n.accesskit_node().label().is_some_and(|l| l.starts_with("Type")))
    };
    let mut tabs = 0;
    while !head(&h) {
        tabs += 1;
        assert!(tabs <= 8, "Tab from the form reaches the table's heads");
        h.key_press(egui::Key::Tab);
        settle(&mut h);
    }
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(h.state().search_results.instance_chosen(), Some(0), "Down at a head");
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Tab);
    settle(&mut h);
    let form = h.state().search_results.form_rect().expect("the form");
    let back = focused_rect(&h);
    assert!(
        back.is_some_and(|r| form.contains_rect(r)),
        "Shift + Tab from the rows into the form {form:?}: {back:?}"
    );
    h.key_press(egui::Key::Tab);
    settle(&mut h);
    assert!(head(&h), "Tab from the form's last control back to the heads");
    key(&mut h, egui::Key::ArrowDown);
    let chosen = h.state().search_results.instance_chosen();
    // (Chosen counts the hits as found; the rows are shown sorted.)
    assert!(chosen.is_some_and(|c| c != 0), "Down moves in the rows again: {chosen:?}");
    key(&mut h, egui::Key::Enter);
    settle(&mut h);
    let opened = ["a1", "a2"]
        .iter()
        .any(|a| h.state().dock.find_tab(&Tab::Area(ResRef::from_str(a).unwrap())).is_some());
    assert!(opened, "Enter goes to the chosen row's area");
}

/// The six lists opened by their commands over an area, the pointer
/// over its view, Down pressed: the row chosen, the keys' outline and the
/// Status Bar's hints the list's (M4's U28 review), at 1280 by 720, Dark
/// and Light. By hand: `MG_U28_PICS=<folder> cargo test -p mg-ui --test
/// app lists_opened_by_their_commands_pictured -- --ignored`.
#[test]
#[ignore]
fn lists_opened_by_their_commands_pictured() {
    let out = std::env::var_os("MG_U28_PICS")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| mg_testkit::test_output().join("u28-pics"));
    let prepare = &waypoints_and_gone_scripts;
    for light in [false, true] {
        let theme = if light { "light" } else { "dark" };
        let dir = out.join(theme).join("held");
        std::fs::create_dir_all(&dir).unwrap();
        for name in ["Resources Used", "Hak Pak Conflict Analysis", "Claims Table"]
            .into_iter()
            .chain(["Find Instance", "Find and Replace Text", "Verify Module"])
        {
            let scratch = format!("u28f-held-pic-{}-{theme}", name.replace(' ', "-"));
            let made = match name {
                "Claims Table" => read_only_area_harness(&scratch),
                _ => area_harness_with(&scratch, "ttr01", None, 4, None, prepare),
            };
            let Some((mut h, area)) = made else { return };
            h.state_mut().settings.light_theme = light;
            settle(&mut h);
            match name {
                "Resources Used" => menu_command(&mut h, "Build", "Area Statistics"),
                "Hak Pak Conflict Analysis" => h.state_mut().show_hak_report(hak_report(10)),
                "Claims Table" => menu_command(&mut h, "Edit", "Claims Table…"),
                "Find Instance" => {
                    menu_command(&mut h, "Edit", "Find Instance…");
                    key(&mut h, egui::Key::Enter);
                }
                "Find and Replace Text" => {
                    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::H);
                    settle(&mut h);
                    typed(&mut h, "Waypoint");
                    key(&mut h, egui::Key::Enter);
                }
                _ => menu_command(&mut h, "Build", "Verify Module"),
            }
            settle(&mut h);
            over_the_view(&mut h, area);
            key(&mut h, egui::Key::ArrowDown);
            over_the_view(&mut h, area);
            settle(&mut h);
            h.render().expect("drawn").save(dir.join(format!("{name}.png"))).unwrap();
        }
    }
}

/// Find and Replace Text by the keyboard alone: its field's hints are
/// the form's, Enter there searches and gives the rows the keys (Down
/// chooses a string), and Shift + Tab from the rows comes back to that
/// field, Tab going on from it to the next (M4's U28 review: from the
/// rows Tab and Shift + Tab went to the Top Bar's menus).
#[test]
fn find_and_replace_text_s_field_and_rows_by_the_keyboard_alone() {
    let prepare = &waypoints_and_gone_scripts;
    let made = area_ui_harness_with("u28f-text-tab", "ttr01", None, 4, None, prepare);
    let Some((mut h, _)) = made else { return };
    settle(&mut h);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::H);
    settle(&mut h);
    typed(&mut h, "Waypoint");
    let said = h.state().status_said();
    assert!(said.iter().any(|s| s == "Enter: Search"), "the field's hints: {said:?}");
    let field = || {
        |n: &egui_kittest::kittest::AccessKitNode<'_>| {
            n.role() == egui::accesskit::Role::TextInput
                && n.value().is_some_and(|v| v == "Waypoint")
        }
    };
    let find_what = h.query_all_by(field()).next().expect("Find what").rect();
    key(&mut h, egui::Key::Enter);
    settle(&mut h);
    key(&mut h, egui::Key::ArrowDown);
    let chosen = chosen_in_dock(&h);
    assert!(chosen.iter().any(|c| c.starts_with("camp_")), "Down chose a string: {chosen:?}");
    h.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Tab);
    settle(&mut h);
    assert_eq!(focused_rect(&h), Some(find_what), "Shift + Tab back to Find what");
    // (Enter again, then Tab: on from the field, to the one after it.)
    key(&mut h, egui::Key::Enter);
    settle(&mut h);
    key(&mut h, egui::Key::ArrowDown);
    h.key_press(egui::Key::Tab);
    settle(&mut h);
    let next = focused_rect(&h);
    let dock = h.state().bottom_dock_rect().unwrap();
    assert!(
        next.is_some_and(|r| dock.contains_rect(r) && r != find_what),
        "Tab from the rows on from Find what, in the dock {dock:?}: {next:?}"
    );
}

/// With Find Instance's In Area list open, Enter (which closes the list,
/// choosing) opens no hit's area, and the arrows and Space move no hit:
/// a popup open as the frame began has the frame's keys, though it closed
/// earlier in that frame (M4's U29 review of U28's tables).
#[test]
fn a_combo_box_list_closed_by_enter_leaves_the_hits_alone() {
    let mut h = objects("u29-find-combo");
    find(&mut h, "");
    dock_high(&mut h, 360);
    in_dock(&h, "Creature GUARD").click();
    h.run();
    assert_eq!(h.state().search_results.instance_chosen(), Some(0));
    let open_list = |h: &mut H| {
        let combo = h
            .query_all_by(|n| n.value().as_deref() == Some("(all areas)"))
            .next()
            .expect("In Area's list")
            .rect()
            .center();
        h.event(egui::Event::PointerMoved(combo));
        h.event(egui::Event::PointerButton {
            pos: combo,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: egui::Modifiers::NONE,
        });
        h.event(egui::Event::PointerButton {
            pos: combo,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        });
        h.run();
        assert!(egui::Popup::is_any_open(&h.ctx), "the list is open");
    };
    let a1 = ResRef::from_str("a1").unwrap();
    for k in [egui::Key::ArrowDown, egui::Key::End, egui::Key::Space, egui::Key::Enter] {
        open_list(&mut h);
        key(&mut h, k);
        settle(&mut h);
        assert_eq!(h.state().search_results.instance_chosen(), Some(0), "{k:?} moved no hit");
        assert!(h.state().dock.find_tab(&Tab::Area(a1)).is_none(), "{k:?} opened no area");
        if egui::Popup::is_any_open(&h.ctx) {
            key(&mut h, egui::Key::Escape);
            settle(&mut h);
        }
        h.state_mut().find_instance.as_mut().unwrap().area = None;
        settle(&mut h);
        if rows_named(&h, "Creature ").is_empty() {
            key(&mut h, egui::Key::Enter);
        }
        in_dock(&h, "Creature GUARD").click();
        h.run();
    }
}

// ------------------------------------------------------------------ 5.2's W4

/// The texts copied to the clipboard in the last pass run.
fn copies(h: &H) -> Vec<String> {
    (h.output().platform_output.commands.iter())
        .filter_map(|c| match c {
            egui::OutputCommand::CopyText(t) => Some(t.clone()),
            _ => None,
        })
        .collect()
}

/// The open menu's `item` clicked: what it copied.
fn menu_copies(h: &mut H, item: &str) -> Vec<String> {
    h.get_all_by_label(item).last().unwrap_or_else(|| panic!("{item} in the menu")).click();
    h.step();
    let out = copies(h);
    h.run();
    out
}

/// Ctrl+C (the backend's Copy) pressed alone: what it copied.
fn ctrl_c(h: &mut H) -> Vec<String> {
    h.event(egui::Event::Copy);
    h.step();
    let out = copies(h);
    h.run();
    out
}

/// The backend's Copy with `modifiers` held, as egui-winit sends it for
/// Ctrl+C and for Ctrl+Alt+C and Ctrl+Shift+C alike: what it copied.
fn copy_held(h: &mut H, modifiers: egui::Modifiers) -> Vec<String> {
    h.event(egui::Event::ModifiersChanged(modifiers));
    h.event(egui::Event::Copy);
    h.step();
    let out = copies(h);
    h.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    h.run();
    out
}

/// Ctrl+Alt+C (the Creature Wizard's) and Ctrl+Shift+C copy nothing.
fn other_keys_copy_nothing(h: &mut H) {
    let alt = egui::Modifiers::COMMAND | egui::Modifiers::ALT;
    assert!(copy_held(h, alt).is_empty(), "Ctrl+Alt+C copies nothing");
    let shift = egui::Modifiers::COMMAND | egui::Modifiers::SHIFT;
    assert!(copy_held(h, shift).is_empty(), "Ctrl+Shift+C copies nothing");
    // (What those keys opened of their own closed.)
    if egui::Popup::is_any_open(&h.ctx) {
        key(h, egui::Key::Escape);
    }
}

/// Whether the tip shown with the pointer at `at` says `words` whole (as
/// a node of its own, apart from the row's, whose name says them too).
fn tip_says(h: &mut H, at: egui::Pos2, words: &str) -> bool {
    let tip = tip_at(h, egui::Rect::from_center_size(at, egui::vec2(2.0, 2.0)));
    tip.is_some_and(|t| t.lines().any(|l| l == words))
}

/// A builder's report of Verify (5.2): a finding could not be selected,
/// copied or right-clicked. A finding's menu has Copy (its line whole:
/// kind, place, message, source, never the row's cut words) and Copy All
/// (every finding listed, one a line); Ctrl+C copies the one chosen,
/// Shift+F10 opens its menu; its message cut short is whole in its tip;
/// a heading's Problems Actions menu has Copy These Findings and Save
/// These Findings As… (a text file of the same lines), each of that
/// heading's findings alone as its Remove from Problems is. A right click
/// chooses the finding (Ctrl+C after it copies that one); Ctrl+Alt+C and
/// Ctrl+Shift+C copy nothing.
#[test]
fn problems_findings_copy_and_their_heading_saves_them() {
    use mg_module::doctor::Severity;
    use mg_ui::problems::{COPY_THESE, MORE, REMOVE, SAVE_AS, copy_line};
    use mg_ui::row_copy::{COPY, COPY_ALL};
    let dir = mg_testkit::scratch_dir("w4-problems-copy");
    let saved = dir.join("found.txt");
    let mut app = app_with(Vec::new());
    app.open_module(&sample_module(&dir));
    app.dialogs = Box::new(NoDialogs { save: vec![saved.clone()], ..Default::default() });
    let mut h = harness(app, 1100.0, 720.0);
    settle(&mut h);
    let long = "Model › chest_01.utp › Appearance: names a model (plc_x99) that is in none of \
                the module's haks nor the game, so the game draws nothing where it stands, and \
                its walkmesh is missing too, which a builder reads to the end";
    let p = Problem::new(Severity::Error, "chest_01.utp".into(), long.into(), None);
    let q = Problem::new(Severity::Warning, "area_02.are".into(), "a second finding".into(), None);
    let b = Problem::new(Severity::Error, "hello.nss".into(), "bad".into(), None);
    let now = h.state().ws.as_ref().map(|ws| ws.revision());
    h.state_mut().problems.replace(Source::Verify, vec![p.clone(), q], now);
    h.state_mut().problems.replace(Source::Build, vec![b], now);
    h.state_mut().settings.hide_log = false;
    h.get_by_label("Status Bar").get_by_label("Problems").click();
    settle(&mut h);
    let line = copy_line(h.state().problems.of(&Source::Verify).unwrap(), &p);
    assert_eq!(line, format!("Error: {long} (Verify Module)"), "the place said once");
    let all = [
        line.clone(),
        "Warning: area_02.are, a second finding (Verify Module)".to_string(),
        "Error: hello.nss, bad (Build Module)".to_string(),
    ]
    .join("\n");
    let row = mg_ui::problems::row_name(&p);
    // Its menu by a right click: Copy and Copy All.
    in_dock(&h, &row).click_secondary();
    h.run();
    assert!(h.query_by_label(COPY_ALL).is_some(), "Copy All in its menu");
    assert_eq!(menu_copies(&mut h, COPY), [line.as_str()], "Copy: the whole message");
    in_dock(&h, &row).click_secondary();
    h.run();
    assert_eq!(menu_copies(&mut h, COPY_ALL), [all.as_str()], "Copy All: every finding");
    // A right click chooses: Ctrl+C after its menu copies the one it was on.
    let second = "Warning: area_02.are, a second finding (Verify Module)";
    let other = (h.state().problems.of(&Source::Verify).unwrap().problems.get(1).cloned())
        .expect("the second finding");
    in_dock(&h, &mg_ui::problems::row_name(&other)).click_secondary();
    h.run();
    key(&mut h, egui::Key::Escape);
    assert_eq!(ctrl_c(&mut h), [second], "Ctrl+C: the finding right-clicked");
    // Ctrl+C on the one chosen; Shift+F10 its menu, Copy taking the keys.
    in_dock(&h, &row).click();
    h.run();
    assert_eq!(ctrl_c(&mut h), [line.as_str()], "Ctrl+C copies the one chosen, once");
    assert_eq!(copy_held(&mut h, egui::Modifiers::COMMAND), [line.as_str()], "Ctrl held");
    other_keys_copy_nothing(&mut h);
    press_now(&mut h, egui::Modifiers::SHIFT, egui::Key::F10);
    h.run();
    let copy = h.get_all_by_label(COPY).last().expect("Shift+F10 opens its menu");
    assert!(copy.accesskit_node().is_focused(), "its first item has the keyboard");
    key(&mut h, egui::Key::Escape);
    assert!(h.query_by_label(COPY_ALL).is_none(), "Escape closes it");
    // The message cut short, whole in its tip.
    let r = in_dock(&h, &row).rect();
    let words = egui::pos2(r.right() - 120.0, r.center().y);
    assert!(tip_says(&mut h, words, long), "the whole message in its tip");
    // The heading's menu: Copy These Findings, Save These Findings As…
    // and Remove from Problems, all three of Verify's findings alone.
    let verify = format!("{line}\n{second}");
    // (Verify's More button: the one level with Verify's heading.)
    let more = |h: &H| {
        let heading = in_dock(h, "Verify Module").rect().center().y;
        (h.get_all_by_label(MORE))
            .min_by_key(|m| (m.rect().center().y - heading).abs() as i32)
            .expect("Verify's Problems Actions")
            .click();
    };
    more(&h);
    h.run();
    assert!(h.query_by_label(REMOVE).is_some(), "Remove from Problems kept");
    assert!(h.query_by_label(COPY_ALL).is_none(), "no Copy All of every source's");
    assert_eq!(menu_copies(&mut h, COPY_THESE), [verify.as_str()], "Verify's findings alone");
    more(&h);
    h.run();
    h.get_by_label(SAVE_AS).click();
    settle(&mut h);
    assert_eq!(std::fs::read_to_string(&saved).unwrap(), verify, "Save As…: the same lines");
    let said = h.state().log.entries.iter().any(|(_, m)| m.starts_with("Saved Problems to"));
    assert!(said, "the log says where");
}

/// Search Results' lines (a script's search): a row's menu copies its line
/// whole or every line; Ctrl+C the one the arrows are on; a line cut at
/// the panel's edge is whole in its tip.
#[test]
fn search_results_lines_copy_whole_and_say_it_in_their_tips() {
    use mg_ui::row_copy::{COPY, COPY_ALL};
    use mg_ui::search_results::Hit;
    let mut h = objects("w4-lines-copy");
    let hello = ResKey::parse("hello", ResType::NSS).unwrap();
    let text = format!("string s = \"{}\";", "a long line of words ".repeat(12));
    let hits = vec![
        Hit::Line { script: hello, line: 2, text: text.clone() },
        Hit::Line { script: hello, line: 7, text: "int n;".into() },
    ];
    let first = hits[0].row();
    let r = &mut h.state_mut().search_results;
    (r.what, r.hits) = ("\"s\": 2 lines found".into(), hits);
    h.state_mut().actions.push(mg_ui::Action::ShowPanel(Panel::SearchResults));
    h.state_mut().settings.hide_log = false;
    settle(&mut h);
    in_dock(&h, &first).click_secondary();
    h.run();
    assert_eq!(menu_copies(&mut h, COPY), [first.as_str()], "Copy: the line whole");
    in_dock(&h, &first).click_secondary();
    h.run();
    let all = format!("{first}\nhello(8): int n;");
    assert_eq!(menu_copies(&mut h, COPY_ALL), [all], "Copy All: every line");
    // (The right click gave the list the keys and chose its row, which
    // Ctrl+C copies, and the arrows move on from.)
    assert_eq!(ctrl_c(&mut h), [first.as_str()], "Ctrl+C: the one right-clicked");
    let second = h.state().search_results.hits[1].row();
    in_dock(&h, &second).click_secondary();
    h.run();
    key(&mut h, egui::Key::Escape);
    assert_eq!(ctrl_c(&mut h), [second.as_str()], "Ctrl+C: the one right-clicked last");
    key(&mut h, egui::Key::ArrowUp);
    assert_eq!(ctrl_c(&mut h), [first.as_str()], "Ctrl+C: the one the arrows are on");
    other_keys_copy_nothing(&mut h);
    let r = in_dock(&h, &first).rect();
    assert!(tip_says(&mut h, r.center(), &first), "the whole line in its tip");
}

/// Find Instance's table (a report table): a row's menu copies the row as
/// Ctrl+C does, and Copy All every row found, in the order the table is
/// sorted; what the search left out is not copied.
#[test]
fn find_instance_s_rows_copy_in_the_table_s_order() {
    use mg_ui::row_copy::{COPY, COPY_ALL};
    let mut h = objects("w4-find-copy");
    find(&mut h, "mg_guard");
    let n = h.state().find_instance.as_ref().unwrap().results.len();
    assert!(n >= 1, "found");
    let row = "Creature GUARD";
    in_dock(&h, row).click_secondary();
    h.run();
    assert_eq!(menu_copies(&mut h, COPY), ["Creature GUARD\tin a1, from mg_guard"]);
    in_dock(&h, "Area").click();
    h.run();
    in_dock(&h, "Area, sorted up").click();
    h.run();
    in_dock(&h, row).click_secondary();
    h.run();
    let all = menu_copies(&mut h, COPY_ALL);
    let lines: Vec<&str> = all[0].lines().collect();
    assert_eq!(lines.len(), n, "only what the search found: {lines:?}");
    assert!(lines.iter().all(|l| l.contains("from mg_guard")), "{lines:?}");
    let down = ["Creature GUARD\tin a2, from mg_guard", "Creature GUARD\tin a1, from mg_guard"];
    assert_eq!(lines, down, "in the table's order, Area sorted down");
    // (The right click chose the row it was on, which Ctrl+C copies.)
    assert_eq!(ctrl_c(&mut h), [down[0]], "Ctrl+C: the row right-clicked");
    other_keys_copy_nothing(&mut h);
    let a1 = h.get_all_by_label(row).filter(|n| n.accesskit_node().bounding_box().is_some());
    let a1 = a1.max_by_key(|n| n.rect().top() as i32).expect("its second row").rect();
    h.hover_at(a1.center());
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: a1.center(),
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    h.run();
    key(&mut h, egui::Key::Escape);
    assert_eq!(ctrl_c(&mut h), [down[1]], "Ctrl+C: the row right-clicked last");
    in_dock(&h, row).click();
    h.run();
    press_now(&mut h, egui::Modifiers::SHIFT, egui::Key::F10);
    h.run();
    assert!(h.query_by_label(COPY_ALL).is_some(), "Shift+F10 opens the chosen row's menu");
}

/// Find and Replace Text's strings: a row's menu copies its place and the
/// whole string, Copy All every string found (ticked or not), Ctrl+C the
/// one chosen; the string is whole in its tip.
#[test]
fn find_and_replace_text_s_strings_copy_whole() {
    use mg_ui::row_copy::{COPY, COPY_ALL};
    let mut h = camps("w4-text-copy", 3);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::H);
    settle(&mut h);
    typed(&mut h, "orc");
    key(&mut h, egui::Key::Enter);
    dock_high(&mut h, 420);
    settle(&mut h);
    let first = rows_named(&h, "camp_00000.utw › LocalizedName: ");
    let line = |i: usize| {
        format!("camp_{i:05}.utw › LocalizedName: Orc camp by the old road, the {i}th fire")
    };
    in_dock(&h, &first[0]).click_secondary();
    h.run();
    assert_eq!(menu_copies(&mut h, COPY), [line(0)]);
    in_dock(&h, &first[0]).click_secondary();
    h.run();
    let all = (0..3).map(line).collect::<Vec<_>>().join("\n");
    assert_eq!(menu_copies(&mut h, COPY_ALL), [all]);
    in_dock(&h, &first[0]).click();
    h.run();
    key(&mut h, egui::Key::ArrowDown);
    assert_eq!(ctrl_c(&mut h), [line(1)], "Ctrl+C: the one chosen");
    let words = "Orc camp by the old road, the 2th fire";
    let third = rows_named(&h, "camp_00002.utw › LocalizedName: ");
    let r = in_dock(&h, &third[0]).rect();
    let at = egui::pos2(r.left() + 60.0, r.bottom() - r.height() / 4.0);
    assert!(tip_says(&mut h, at, words), "the string in its tip");
}
