//! What API 0.6 added a plugin can be: an exporter, an importer, a wizard
//! and a panel; each declared, run, and held to what it may do.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use mg_edit::{Command, Workspace};
use mg_gff::{Gff, Value};
use mg_module::Module;
use mg_plugin::{
    Answer, Element, FieldKind, Host, Input, Level, Manifest, Plugin, PluginError, Question,
    Selected, Tone, inspect, run_export, run_import, run_panel, run_wizard,
};
use mg_resman::ResKey;

fn key(name: &str) -> ResKey {
    ResKey::from_filename(name).unwrap()
}

/// A module with two creatures.
fn module() -> Module {
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    for (name, tag) in [("guard", "gate_guard"), ("captain", "CAPTAIN")] {
        let mut g = Gff::new(*b"UTC ");
        g.root.set("Tag", Value::String(tag.as_bytes().to_vec()));
        m.set_gff(key(&format!("{name}.utc")), &g).unwrap();
    }
    m
}

#[derive(Default)]
struct TestHost {
    log: RefCell<Vec<String>>,
    answers: RefCell<VecDeque<Option<Answer>>>,
    asked: RefCell<Vec<Question>>,
    interactive: Cell<bool>,
}

impl Host for TestHost {
    fn log(&self, _: Level, text: &str) {
        self.log.borrow_mut().push(text.to_string());
    }

    fn interactive(&self) -> bool {
        self.interactive.get()
    }

    fn ask(&self, question: &Question) -> Option<Answer> {
        self.asked.borrow_mut().push(question.clone());
        let answer = self.answers.borrow_mut().pop_front().flatten();
        // (A page's fields not given are as the page shows them, as a
        // window answers.)
        match (question, answer) {
            (Question::Page(p), Some(Answer::Page { mut values, back })) => {
                for f in &p.fields {
                    values.entry(f.id.clone()).or_insert_with(|| f.kind.default_value());
                }
                Some(Answer::Page { values, back })
            }
            (_, answer) => answer,
        }
    }
}

const HEAD: &str = "[plugin]\nid = \"test.kinds\"\nname = \"Kinds\"\nversion = \"1\"\napi = \"0.6\"\nlicense = \"MIT\"\n";

/// A plugin written into a scratch folder of its own: its manifest (after
/// the `[plugin]` section) and its code.
fn written(name: &str, sections: &str, code: &str) -> Plugin {
    let dir = mg_testkit::scratch_dir(name);
    std::fs::write(dir.join("plugin.cfg"), format!("{HEAD}\n{sections}")).unwrap();
    std::fs::write(dir.join("main.luau"), format!("local mg = require(\"@moonglow\")\n{code}"))
        .unwrap();
    Plugin::load(&dir).unwrap()
}

fn input() -> Input {
    Input::new(module(), None)
}

fn values(pairs: &[(&str, serde_json::Value)]) -> BTreeMap<String, serde_json::Value> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
}

fn page(values_: &[(&str, serde_json::Value)], back: bool) -> Option<Answer> {
    Some(Answer::Page { values: values(values_), back })
}

fn applied(edits: Vec<mg_edit::Edit>) -> Workspace {
    let mut ws = Workspace::new(module());
    ws.apply(Command::new("t", edits)).unwrap();
    ws.flush().unwrap();
    ws
}

#[test]
fn a_manifest_declares_the_kinds_of_0_6() {
    let m = Manifest::parse(&format!(
        "{HEAD}\n[exporter]\nid = \"csv\"\ntitle = \"As CSV…\"\nextension = \".CSV\"\nfor = \"creature blueprint\"\n\
         \n[exporter]\nid = \"all\"\ntitle = \"All…\"\nto = \"folder\"\n\
         \n[importer]\nid = \"csv\"\ntitle = \"From CSV…\"\nextensions = \"csv, txt\"\n\
         \n[importer]\nid = \"tree\"\ntitle = \"From a Folder…\"\nfrom = \"folder\"\n\
         \n[wizard]\nid = \"shop\"\ntitle = \"Shop…\"\nhint = \"A shop\"\n\
         \n[panel]\nid = \"view\"\ntitle = \"View\"\nrefresh = \"selection\"\n"
    ))
    .unwrap();
    assert_eq!(m.ids("exporter"), ["csv", "all"]);
    assert_eq!((m.exporters[0].extension.as_str(), m.exporters[0].folder), ("csv", false));
    assert_eq!(m.exporters[0].targets.len(), 1);
    assert!(m.exporters[1].folder);
    assert_eq!(m.importers[0].extensions, ["csv", "txt"]);
    assert!(m.importers[1].folder);
    assert_eq!(m.wizards[0].hint, "A shop");
    assert!(m.panels[0].on_selection && !m.panels[0].on_edits);

    // Under an earlier API, refused, saying so.
    let early = HEAD.replace("0.6", "0.5");
    let e = Manifest::parse(&format!("{early}\n[panel]\nid = \"v\"\ntitle = \"V\"\n")).unwrap_err();
    assert!(e.contains("[panel] is plugin API 0.6"), "{e}");
    // And each section held to what it takes.
    for (section, why) in [
        ("[exporter]\nid = \"x\"\ntitle = \"X\"\nto = \"disk\"\n", "\"file\" or \"folder\""),
        (
            "[exporter]\nid = \"x\"\ntitle = \"X\"\nto = \"folder\"\nextension = \"csv\"\n",
            "no extension",
        ),
        ("[exporter]\nid = \"x\"\ntitle = \"X\"\nfor = \"nss\"\n", "objects, areas or tiles"),
        ("[exporter]\nid = \"x\"\ntitle = \"X\"\nextension = \"c.v\"\n", "no file's extension"),
        (
            "[importer]\nid = \"x\"\ntitle = \"X\"\nfrom = \"folder\"\nextensions = \"csv\"\n",
            "no extensions",
        ),
        ("[panel]\nid = \"x\"\ntitle = \"X\"\nrefresh = \"always\"\n", "\"edits\", \"selection\""),
        ("[wizard]\nid = \"x\"\n", "needs title"),
        (
            "[wizard]\nid = \"x\"\ntitle = \"X\"\n[wizard]\nid = \"x\"\ntitle = \"Y\"\n",
            "a second wizard",
        ),
        ("[panel]\nid = \"x\"\ntitle = \"X\"\npages = \"2\"\n", "has no key \"pages\""),
    ] {
        let e = Manifest::parse(&format!("{HEAD}\n{section}")).unwrap_err();
        assert!(e.contains(why), "{section}: {e}");
    }
}

/// What the code registers is held to what the manifest declares, for the
/// new kinds as for the others.
#[test]
fn the_code_registers_what_is_declared() {
    let p = written(
        "kinds-inspect",
        "[exporter]\nid = \"e\"\ntitle = \"E\"\n\n[wizard]\nid = \"w\"\ntitle = \"W\"\n\n[panel]\nid = \"p\"\ntitle = \"P\"\n",
        "mg.exporter(\"e\", function(ctx, out) end)\nmg.panel(\"q\", function(ctx) end)\n",
    );
    let faults = inspect(&p, Rc::new(TestHost::default())).unwrap();
    assert_eq!(
        faults,
        [
            "the wizard \"w\" is declared, but the code registers none",
            "the panel \"p\" is declared, but the code registers none",
            "the code registers the panel \"q\", which the manifest lacks",
        ]
    );
    // A wizard is a table with its pages and its finish.
    let bad = written(
        "kinds-inspect-bad",
        "[wizard]\nid = \"w\"\ntitle = \"W\"\n",
        "mg.wizard(\"w\", function() end)\n",
    );
    let e = inspect(&bad, Rc::new(TestHost::default())).unwrap_err();
    assert!(e.to_string().contains("a table"), "{e}");
}

/// An exporter's `out:write` is what is written, where the user chose,
/// when it has returned: a file whole, or files of a folder by name.
#[test]
fn an_exporter_writes_where_the_user_chose() {
    let p = written(
        "kinds-export",
        "[exporter]\nid = \"tags\"\ntitle = \"Tags…\"\nextension = \"txt\"\n\
         \n[exporter]\nid = \"each\"\ntitle = \"Each…\"\nto = \"folder\"\n",
        r#"mg.exporter("tags", function(ctx, out)
    local tags = {}
    for _, name in ctx.module:resources("utc") do
        table.insert(tags, ctx.module:gff(name).Tag)
    end
    out:write("first")
    out:write(table.concat(tags, "\n"))
    return `{#tags} tags to {out.name}`
end)
mg.exporter("each", function(ctx, out)
    for _, name in ctx.module:resources("utc") do
        out:write(string.gsub(name, "%.utc$", ".txt"), ctx.module:gff(name).Tag)
    end
end)
"#,
    );
    let host = Rc::new(TestHost::default());
    let to = Path::new("/somewhere/tags.txt");
    let done = run_export(&p, "tags", to, input(), host.clone()).unwrap();
    assert!(done.edits.is_empty());
    assert_eq!(done.label, "Tags");
    assert_eq!(done.folders.len(), 1);
    assert_eq!(done.folders[0].dir, PathBuf::from("/somewhere"));
    assert_eq!(done.folders[0].files, [("tags.txt".to_string(), b"CAPTAIN\ngate_guard".to_vec())]);
    assert_eq!(host.log.borrow().as_slice(), ["2 tags to tags.txt"]);
    // To a folder: a file a creature, written for real.
    let dir = mg_testkit::scratch_dir("kinds-export-out");
    let done = run_export(&p, "each", &dir, input(), host.clone()).unwrap();
    assert_eq!(done.folders[0].write().unwrap(), (2, 0));
    assert_eq!(std::fs::read_to_string(dir.join("guard.txt")).unwrap(), "gate_guard");
}

/// An exporter only reads: edits, a hak, or a name that leads out of the
/// folder fail it.
#[test]
fn an_exporter_only_reads() {
    let p = written(
        "kinds-export-reads",
        "[exporter]\nid = \"edits\"\ntitle = \"E\"\n\n[exporter]\nid = \"hak\"\ntitle = \"H\"\n\
         \n[exporter]\nid = \"out\"\ntitle = \"O\"\nto = \"folder\"\n",
        r#"mg.exporter("edits", function(ctx, out)
    ctx.edit:set("guard.utc", "Tag", "X")
end)
mg.exporter("hak", function(ctx, out)
    ctx.hak:write("tiles", "a.txt", "a")
end)
mg.exporter("out", function(ctx, out)
    out:write("../up.txt", "a")
end)
"#,
    );
    let host = Rc::new(TestHost::default());
    host.answers.borrow_mut().push_back(Some(Answer::Yes));
    for (id, why) in
        [("edits", "only reads"), ("hak", "only reads"), ("out", "a file's name alone")]
    {
        let e = run_export(&p, id, Path::new("/x/out.txt"), input(), host.clone()).unwrap_err();
        assert!(e.to_string().contains(why), "{id}: {e}");
    }
}

/// An importer reads what the user chose, a file of its kinds or a
/// folder, and edits: one command, named as it says or "Import" and its
/// title.
#[test]
fn an_importer_reads_what_the_user_chose() {
    let p = written(
        "kinds-import",
        "[importer]\nid = \"tags\"\ntitle = \"Tags…\"\nextensions = \"txt\"\n\
         \n[importer]\nid = \"tree\"\ntitle = \"Tree…\"\nfrom = \"folder\"\n",
        r#"mg.importer("tags", function(ctx, file)
    for line in string.gmatch(file.bytes, "[^\n]+") do
        local name, tag = string.match(line, "(%w+)=(%w+)")
        ctx.edit:set(name .. ".utc", "Tag", tag)
    end
end)
mg.importer("tree", function(ctx, folder)
    for _, name in folder:files() do
        ctx.edit:set(string.gsub(name, "%.txt$", ".utc"), "Tag", folder:text(name))
    end
    return { label = `Read {folder.name}` }
end)
"#,
    );
    let dir = mg_testkit::scratch_dir("kinds-import-in");
    std::fs::write(dir.join("tags.txt"), "guard=NIGHT\n").unwrap();
    std::fs::write(dir.join("tags.csv"), "guard=NIGHT\n").unwrap();
    let host = Rc::new(TestHost::default());
    let done = run_import(&p, "tags", &dir.join("tags.txt"), input(), host.clone()).unwrap();
    assert_eq!(done.label, "Import Tags");
    let ws = applied(done.edits);
    let tag =
        |ws: &Workspace, n: &str| ws.module.gff(&key(n)).unwrap().unwrap().root.get("Tag").cloned();
    assert_eq!(tag(&ws, "guard.utc"), Some(Value::String(b"NIGHT".to_vec())));
    // Not one of its kinds: refused before its code runs.
    let e = run_import(&p, "tags", &dir.join("tags.csv"), input(), host.clone()).unwrap_err();
    assert!(e.to_string().contains("reads .txt"), "{e}");
    let e = run_import(&p, "tags", &dir.join("none.txt"), input(), host.clone()).unwrap_err();
    assert!(e.to_string().contains("is not a file"), "{e}");
    // A folder.
    let tree = mg_testkit::scratch_dir("kinds-import-tree");
    std::fs::write(tree.join("captain.txt"), "BOSS").unwrap();
    let done = run_import(&p, "tree", &tree, input(), host.clone()).unwrap();
    assert_eq!(done.label, "Read kinds-import-tree");
    let ws = applied(done.edits);
    assert_eq!(tag(&ws, "captain.utc"), Some(Value::String(b"BOSS".to_vec())));
    let e = run_import(&p, "tree", &tree.join("captain.txt"), input(), host).unwrap_err();
    assert!(e.to_string().contains("is not a folder"), "{e}");
}

const WIZARD: &str = r#"mg.wizard("shop", {
    pages = {
        {
            id = "name",
            title = "Name",
            text = "What it is called.",
            fields = { { id = "name", label = "Name", default = "Shop" } },
            check = function(ctx, a)
                if a.name == "" then return "A shop needs a name." end
                return nil
            end,
        },
        function(ctx, a)
            if a.name == "skip" then return nil end
            return { id = "price", title = `{a.name}'s price`, fields = {
                { id = "markup", type = "number", default = 100 },
            } }
        end,
        { id = "last", title = "Done", fields = { { id = "open", type = "check", default = true } } },
    },
    finish = function(ctx, a)
        ctx.edit:set("guard.utc", "Tag", `{a.name}:{a.markup or "none"}:{a.open}`)
        return `Shop {a.name}`
    end,
})
"#;

fn wizard_plugin(name: &str, code: &str) -> Plugin {
    written(name, "[wizard]\nid = \"shop\"\ntitle = \"Shop Wizard…\"\n", code)
}

fn guard_tag(edits: Vec<mg_edit::Edit>) -> String {
    let ws = applied(edits);
    let g = ws.module.gff(&key("guard.utc")).unwrap().unwrap();
    match g.root.get("Tag") {
        Some(Value::String(t)) => String::from_utf8(t.clone()).unwrap(),
        other => panic!("{other:?}"),
    }
}

/// A wizard's pages are asked one at a time, each with where it is; Back
/// goes to the page before with what was given kept as its defaults; a
/// page made by a function sees the answers so far (or is skipped); Finish
/// makes the edits, one command.
#[test]
fn a_wizard_asks_its_pages_in_turn() {
    let p = wizard_plugin("kinds-wizard", WIZARD);
    let host = Rc::new(TestHost::default());
    host.answers.borrow_mut().extend([
        page(&[("name", "Forge".into())], false),
        // (Back from the second, to change the name.)
        page(&[("markup", 150.into())], true),
        page(&[("name", "Smithy".into())], false),
        page(&[("markup", 140.into())], false),
        page(&[("open", false.into())], false),
    ]);
    let done = run_wizard(&p, "shop", input(), host.clone()).unwrap();
    assert_eq!(done.label, "Shop Smithy");
    assert_eq!(guard_tag(done.edits), "Smithy:140:false");
    let asked = host.asked.borrow();
    let pages: Vec<&mg_plugin::Page> = asked
        .iter()
        .map(|q| match q {
            Question::Page(p) => p,
            other => panic!("{other:?}"),
        })
        .collect();
    let at: Vec<(usize, usize, bool, bool, &str)> =
        pages.iter().map(|p| (p.step, p.steps, p.back, p.last, p.title.as_str())).collect();
    assert_eq!(
        at,
        [
            (1, 3, false, false, "Name"),
            (2, 3, true, false, "Forge's price"),
            (1, 3, false, false, "Name"),
            (2, 3, true, false, "Smithy's price"),
            (3, 3, true, true, "Done"),
        ]
    );
    assert_eq!(pages[0].wizard, "Shop Wizard…");
    assert_eq!(pages[0].text, "What it is called.");
    // Back: the page again with what was given before.
    assert_eq!(pages[2].fields[0].kind, FieldKind::Text { default: "Forge".into() });
    assert_eq!(
        pages[3].fields[0].kind,
        FieldKind::Number { default: 150.0, min: None, max: None },
        "what was given on it, going back"
    );

    // A page its function skips is not asked.
    let host = Rc::new(TestHost::default());
    host.answers.borrow_mut().extend([page(&[("name", "skip".into())], false), page(&[], false)]);
    let done = run_wizard(&p, "shop", input(), host.clone()).unwrap();
    assert_eq!(guard_tag(done.edits), "skip:none:true");
    assert_eq!(host.asked.borrow().len(), 2);
}

/// A page's check that objects asks the page again, saying why, where a
/// person answers; where nobody does (`mg`, a test) it ends the wizard
/// with what it said. Closed on a page: canceled, nothing made.
#[test]
fn a_wizard_s_check_and_its_cancel() {
    let p = wizard_plugin("kinds-wizard-check", WIZARD);
    let host = Rc::new(TestHost::default());
    host.interactive.set(true);
    host.answers.borrow_mut().extend([
        page(&[("name", "".into())], false),
        page(&[("name", "Ok".into())], false),
        page(&[], false),
        page(&[], false),
    ]);
    let done = run_wizard(&p, "shop", input(), host.clone()).unwrap();
    assert_eq!(guard_tag(done.edits), "Ok:100:true");
    let asked = host.asked.borrow();
    let Question::Page(again) = &asked[1] else { panic!() };
    assert_eq!((again.step, again.error.as_deref()), (1, Some("A shop needs a name.")));
    let Question::Page(after) = &asked[2] else { panic!() };
    assert_eq!(after.error, None);

    let host = Rc::new(TestHost::default());
    host.answers.borrow_mut().push_back(page(&[("name", "".into())], false));
    let e = run_wizard(&p, "shop", input(), host).unwrap_err();
    assert!(e.to_string().contains("Name: A shop needs a name."), "{e}");

    let host = Rc::new(TestHost::default());
    host.answers.borrow_mut().extend([page(&[], false), None]);
    let e = run_wizard(&p, "shop", input(), host).unwrap_err();
    assert!(matches!(e, PluginError::Canceled(_)), "{e:?}");
}

/// A wizard's pages only read, and a field's id is its own across them.
#[test]
fn a_wizard_s_pages_only_read() {
    let edits = wizard_plugin(
        "kinds-wizard-edits",
        r#"mg.wizard("shop", { pages = {
    function(ctx, a) ctx.edit:set("guard.utc", "Tag", "X") return { fields = {} } end,
}, finish = function() end })
"#,
    );
    let e = run_wizard(&edits, "shop", input(), Rc::new(TestHost::default())).unwrap_err();
    assert!(e.to_string().contains("a wizard's pages only read"), "{e}");
    let twice = wizard_plugin(
        "kinds-wizard-twice",
        r#"mg.wizard("shop", { pages = {
    { id = "a", fields = { { id = "n" } } },
    { id = "b", fields = { { id = "n" } } },
}, finish = function() end })
"#,
    );
    let host = Rc::new(TestHost::default());
    host.answers.borrow_mut().push_back(page(&[], false));
    let e = run_wizard(&twice, "shop", input(), host).unwrap_err();
    assert!(e.to_string().contains("the field \"n\" is on the pages \"a\" and \"b\""), "{e}");
}

/// A panel's handler returns what it shows, as data: its elements, read
/// and held to the plugin's commands and to the limits.
#[test]
fn a_panel_returns_what_it_shows() {
    let p = written(
        "kinds-panel",
        "[panel]\nid = \"view\"\ntitle = \"View\"\n\n[command]\nid = \"go\"\ntitle = \"Go\"\n",
        r#"mg.panel("view", function(ctx)
    return {
        { type = "heading", text = "Creatures" },
        "plain text",
        { type = "text", text = "careful", color = "warning" },
        { type = "text", text = "quiet", weak = true },
        { type = "value", label = "Count", text = #ctx.module:resources("utc") },
        { type = "separator" },
        { type = "button", text = "Go", command = "go", on = { resource = "guard.utc" } },
        { type = "list", items = {
            { text = "Guard", hint = "the guard", open = "guard.utc" },
            { text = "Placed", open = { resource = "tavern.git", path = "/Creature List[1]" } },
            "just text",
        } },
        { type = "text", text = string.rep("x", 1200) },
    }
end)
mg.command("go", function() end)
"#,
    );
    let shown = run_panel(&p, "view", input(), Rc::new(TestHost::default())).unwrap();
    let e = &shown.elements;
    assert_eq!(e[0], Element::Heading("Creatures".into()));
    assert_eq!(e[1], Element::Text { text: "plain text".into(), tone: Tone::Plain });
    assert_eq!(e[2], Element::Text { text: "careful".into(), tone: Tone::Warning });
    assert_eq!(e[3], Element::Text { text: "quiet".into(), tone: Tone::Weak });
    assert_eq!(e[4], Element::Value { label: "Count".into(), text: "2".into() });
    assert_eq!(e[5], Element::Separator);
    let Element::Button { command, on, .. } = &e[6] else { panic!("{:?}", e[6]) };
    assert_eq!(
        (command.as_str(), on.as_slice()),
        ("go", &[Selected::parse("guard.utc").unwrap()][..])
    );
    let Element::List(items) = &e[7] else { panic!("{:?}", e[7]) };
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].open, Some(Selected::parse("guard.utc").unwrap()));
    assert_eq!(items[1].open, Some(Selected::parse("tavern.git/Creature List[1]").unwrap()));
    assert_eq!(items[2].open, None);
    let Element::Text { text, .. } = &e[8] else { panic!() };
    assert_eq!(text.chars().count(), mg_plugin::MAX_TEXT + 1, "cut, with its …");
    assert!(shown.shortened && shown.cut == 0);
    assert!(shown.texts().contains(&"Count: 2".to_string()));
}

/// A panel shows at most so much, only reads, runs only its own plugin's
/// commands, and has a time budget.
#[test]
fn a_panel_is_held_to_its_limits() {
    let p = written(
        "kinds-panel-limits",
        "[panel]\nid = \"many\"\ntitle = \"M\"\n\n[panel]\nid = \"edits\"\ntitle = \"E\"\n\
         \n[panel]\nid = \"other\"\ntitle = \"O\"\n\n[panel]\nid = \"slow\"\ntitle = \"S\"\n\
         \n[panel]\nid = \"set\"\ntitle = \"T\"\n\n[setting]\nid = \"n\"\ntype = \"int\"\n",
        r#"mg.panel("many", function(ctx)
    local items = {}
    for i = 1, 2500 do table.insert(items, tostring(i)) end
    return { "first", { type = "list", items = items }, "after" }
end)
mg.panel("edits", function(ctx)
    ctx.edit:set("guard.utc", "Tag", "X")
    return {}
end)
mg.panel("other", function(ctx)
    return { { type = "button", text = "Off", command = "elsewhere" } }
end)
mg.panel("slow", function(ctx)
    while true do end
end)
mg.panel("set", function(ctx)
    ctx.settings:set("n", 2)
end)
"#,
    );
    let host = || Rc::new(TestHost::default());
    let many = run_panel(&p, "many", input(), host()).unwrap();
    let Element::List(items) = &many.elements[1] else { panic!() };
    assert_eq!(items.len(), mg_plugin::MAX_ELEMENTS - 2);
    assert_eq!(many.cut, 2500 - (mg_plugin::MAX_ELEMENTS - 2) + 1, "the rest, and the last");
    for (id, why) in [
        ("edits", "a panel only reads"),
        ("other", "declares no command \"elsewhere\""),
        ("slow", "ran out of time"),
        ("set", "a panel only reads: ctx.settings:set"),
    ] {
        let e = run_panel(&p, id, input(), host()).unwrap_err();
        assert!(e.to_string().contains(why), "{id}: {e}");
    }
}
