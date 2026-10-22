//! A resource's own events and the after-create hook (plugin API 0.4): a
//! resource saving or saved (old and new, warnings, refusals and their
//! fixes; never an edit), objects placed or pasted, a resource made (whose
//! edits the host makes part of the user's own step), and the time budget
//! of a handler that runs at once.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use mg_core::ResRef;
use mg_gff::{Gff, Struct, Value};
use mg_module::Module;
use mg_plugin::{
    Answer, Event, EventKind, Host, Input, Level, Plugin, PluginError, Question, Selected, Target,
    run_event,
};
use mg_resman::ResKey;
use mg_rules::ObjectKind;

#[derive(Default)]
struct TestHost {
    log: RefCell<Vec<String>>,
}

impl Host for TestHost {
    fn log(&self, _: Level, text: &str) {
        self.log.borrow_mut().push(text.to_string());
    }

    fn ask(&self, _: &Question) -> Option<Answer> {
        None
    }
}

fn key(name: &str) -> ResKey {
    ResKey::from_filename(name).unwrap()
}

fn text(t: &str) -> Value {
    Value::String(t.as_bytes().to_vec())
}

/// A module with an area (tagged `tag`), two placeables and a waypoint in
/// it, and a creature.
fn module(tag: &str) -> Module {
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    let mut are = Gff::new(*b"ARE ");
    are.root.set("Tag", text(tag));
    are.root.set("ResRef", Value::resref(ResRef::from_str("tavern").unwrap()));
    m.set_gff(key("tavern.are"), &are).unwrap();
    let mut git = Gff::new(*b"GIT ");
    let lamp = |t: &str| {
        let mut s = Struct::new(9);
        s.set("Tag", text(t));
        s.set("TemplateResRef", Value::resref(ResRef::from_str("lamp").unwrap()));
        s
    };
    git.root.set("Placeable List", Value::List(vec![lamp("LAMP"), lamp("LAMP_2")]));
    let mut wp = Struct::new(5);
    wp.set("Tag", text("WP_START"));
    git.root.set("WaypointList", Value::List(vec![wp]));
    m.set_gff(key("tavern.git"), &git).unwrap();
    let mut utc = Gff::new(*b"UTC ");
    utc.root.set("Tag", text("GUARD"));
    m.set_gff(key("guard.utc"), &utc).unwrap();
    m
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("mg-plugin-ev-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn manifest(api: &str, sections: &str) -> Result<mg_plugin::Manifest, String> {
    mg_plugin::Manifest::parse(&format!(
        "[plugin]\nid = \"test.ev\"\nname = \"Rules\"\nversion = \"1\"\napi = \"{api}\"\n\
         license = \"MIT\"\n{sections}"
    ))
}

fn written(name: &str, sections: &str, code: &str) -> Plugin {
    let dir = scratch(name);
    std::fs::write(
        dir.join("plugin.cfg"),
        format!(
            "[plugin]\nid = \"test.{name}\"\nname = \"Rules\"\nversion = \"1\"\napi = \"0.4\"\n\
             license = \"MIT\"\n{sections}"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("main.luau"), code).unwrap();
    Plugin::load(&dir).unwrap()
}

/// What a save writes, with each resource as it was: the module of `old`
/// against that of `new`.
fn saving(kind: EventKind, old: &Module, new: &Module) -> Event {
    let gone: Vec<ResKey> = old.keys().copied().collect();
    Event::saving(kind, new, |k| old.get(k), gone)
}

#[test]
fn the_manifest_declares_the_new_events_for_api_0_4() {
    let m = manifest(
        "0.4",
        "[event]\nid = \"a\"\non = \"resource-saving\"\ntypes = \"are, GIT\"\n\
         [event]\nid = \"b\"\non = \"object-pasted\"\nfor = \"placeable, waypoint\"\n\
         [event]\nid = \"c\"\non = \"resource-created\"\nfor = \"area\"\n\
         [event]\nid = \"d\"\non = \"resource-saved\"\n[event]\nid = \"e\"\non = \"object-placed\"\n",
    )
    .unwrap();
    let on: Vec<EventKind> = m.events.iter().map(|e| e.on).collect();
    assert_eq!(
        on,
        [
            EventKind::ResourceSaving,
            EventKind::ObjectPasted,
            EventKind::ResourceCreated,
            EventKind::ResourceSaved,
            EventKind::ObjectPlaced
        ]
    );
    assert_eq!(m.events[0].types, [mg_core::ResType::ARE, mg_core::ResType::GIT]);
    assert!(m.events[0].is_for(&key("x.git")) && !m.events[0].is_for(&key("x.utc")));
    assert_eq!(
        m.events[1].targets,
        [Target::Object(ObjectKind::Placeable), Target::Object(ObjectKind::Waypoint)]
    );
    assert!(m.events[2].is_for(&key("new.are")) && !m.events[2].is_for(&key("new.utc")));
    assert!(m.events[3].is_for(&key("x.nss")));

    // Each where it belongs, and under the API that has it.
    let refused = [
        ("0.3", "[event]\nid = \"a\"\non = \"resource-saving\"\n", "plugin API 0.4"),
        ("0.4", "[event]\nid = \"a\"\non = \"object-pasted\"\ntypes = \"git\"\n", "types"),
        ("0.4", "[event]\nid = \"a\"\non = \"resource-saving\"\nfor = \"area\"\n", "for"),
        ("0.4", "[event]\nid = \"a\"\non = \"resource-saved\"\ntypes = \"zzz\"\n", "no type"),
        ("0.4", "[event]\nid = \"a\"\non = \"object-placed\"\nfor = \"tile\"\n", "an event is for"),
        ("0.4", "[event]\nid = \"a\"\non = \"midnight\"\n", "resource-created"),
    ];
    for (api, sections, says) in refused {
        let e = manifest(api, sections).unwrap_err();
        assert!(e.contains(says), "{sections}: {e}");
    }
    // What 0.3 had is as it was.
    assert!(manifest("0.3", "[event]\nid = \"a\"\non = \"module-saved\"\n").is_ok());
}

const SAVING: &str = r#"
local mg = require("@moonglow")
mg.on("names", function(ctx)
    local e = ctx.event
    ctx.log:info(`{e.name} {e.resource} {e.kind} of {#e.resources}`)
    if e.kind == "area" then
        if e.old and e.old.Tag ~= e.new.Tag then
            e:refuse(`the area's tag is kept: it was {e.old.Tag}, now {e.new.Tag}`)
        end
        if e.new.Tag ~= string.upper(e.new.Tag) then
            local fix = ctx.fix("Upper-case it")
            fix:set(e.resource, "Tag", string.upper(e.new.Tag))
            e:warn(`tag {e.new.Tag} is not upper case`, fix)
        end
        ctx.log:info(table.concat(e.changed or {}, "; "))
    end
end)
mg.on("edits", function(ctx)
    ctx.edit:set(ctx.event.resource, "Tag", "X")
end)
mg.on("sets", function(ctx)
    ctx.settings:set("n", 2)
end)
"#;

/// A resource saving's handler is called for each resource the save
/// writes (of its types), reads it as it was and as it will be, and warns
/// or refuses, with a fix; it never edits.
#[test]
fn a_resource_saving_s_handler_reads_warns_and_refuses() {
    let plugin = written(
        "saving",
        "[event]\nid = \"names\"\non = \"resource-saving\"\ntypes = \"are, utc\"\n\
         [event]\nid = \"edits\"\non = \"resource-saving\"\n\
         [event]\nid = \"sets\"\non = \"resource-saving\"\n\
         [setting]\nid = \"n\"\ntype = \"int\"\n",
        SAVING,
    );
    let (old, new) = (module("TAVERN"), module("tavern_inn"));
    let mut new = new;
    let mut utc = new.gff(&key("guard.utc")).unwrap().unwrap();
    utc.root.set("Tag", text("GUARD2"));
    new.set_gff(key("guard.utc"), &utc).unwrap();
    let event = saving(EventKind::ResourceSaving, &old, &new);
    assert_eq!(event.resources, [key("guard.utc"), key("tavern.are")]);
    let host = Rc::new(TestHost::default());
    let input = Input::new(new.clone(), None);
    let outcome = run_event(&plugin, "names", &event, input, host.clone()).unwrap();
    assert!(outcome.edits.is_empty());
    assert_eq!(
        host.log.borrow().as_slice(),
        [
            "resource-saving guard.utc creature of 2",
            "resource-saving tavern.are area of 2",
            r#"/Tag: cexostring "TAVERN" != cexostring "tavern_inn""#,
        ]
    );
    let said: Vec<(String, bool, Option<&str>)> = outcome
        .said
        .iter()
        .map(|s| (s.resource.to_string(), s.refused, s.fix.as_ref().map(|f| f.label.as_str())))
        .collect();
    assert_eq!(
        said,
        [
            ("tavern.are".to_string(), true, None),
            ("tavern.are".to_string(), false, Some("Upper-case it")),
        ]
    );
    assert_eq!(outcome.said[0].text, "the area's tag is kept: it was TAVERN, now tavern_inn");
    assert_eq!(outcome.said[1].fix.as_ref().unwrap().edits.len(), 1);

    // An edit, or a setting set, is refused: around a save a plugin only
    // reads.
    let input = Input::new(new.clone(), None);
    let e = run_event(&plugin, "edits", &event, input, host.clone()).unwrap_err().to_string();
    assert!(e.contains("only reads"), "{e}");
    let e = run_event(&plugin, "sets", &event, Input::new(new.clone(), None), host.clone())
        .unwrap_err()
        .to_string();
    assert!(e.contains("only reads"), "{e}");

    // A resource the save writes for the first time has no old value; the
    // handler runs for none that is not of its types.
    let mut fresh = new.clone();
    fresh.set_gff(key("cellar.are"), &new.gff(&key("tavern.are")).unwrap().unwrap()).unwrap();
    let event = saving(EventKind::ResourceSaving, &new, &fresh);
    assert_eq!(event.resources, [key("cellar.are")]);
    let host = Rc::new(TestHost::default());
    let outcome =
        run_event(&plugin, "names", &event, Input::new(fresh, None), host.clone()).unwrap();
    assert_eq!(outcome.said.len(), 1, "{:?}", outcome.said);
    let only_scripts = Event::new(EventKind::ResourceSaving, vec![key("old.nss")]);
    let host = Rc::new(TestHost::default());
    run_event(&plugin, "names", &only_scripts, Input::new(new, None), host.clone()).unwrap();
    assert!(host.log.borrow().is_empty());
}

/// A resource saved: after the save, each resource as it was and is; it
/// only reads too.
#[test]
fn a_resource_saved_s_handler_reads_old_and_new() {
    let plugin = written(
        "saved",
        "[event]\nid = \"after\"\non = \"resource-saved\"\n[event]\nid = \"edits\"\non = \"resource-saved\"\n",
        r#"
        local mg = require("@moonglow")
        mg.on("after", function(ctx)
            local e = ctx.event
            ctx.log:info(`{e.resource}: {e.old and e.old.Tag} -> {e.new and e.new.Tag}`)
            if e.warn then error("a saved resource is past warning") end
        end)
        mg.on("edits", function(ctx) ctx.edit:set("guard.utc", "Tag", "X") end)
        "#,
    );
    let (old, new) = (module("TAVERN"), module("INN"));
    let event = saving(EventKind::ResourceSaved, &old, &new);
    let host = Rc::new(TestHost::default());
    run_event(&plugin, "after", &event, Input::new(new.clone(), None), host.clone()).unwrap();
    assert_eq!(host.log.borrow().as_slice(), ["tavern.are: TAVERN -> INN"]);
    let e = run_event(&plugin, "edits", &event, Input::new(new, None), host).unwrap_err();
    assert!(e.to_string().contains("only reads"), "{e}");
}

/// Objects pasted: the handler gets them (those of the kinds it is for)
/// and their area, and its edits come back to be made with the paste.
#[test]
fn an_object_pasted_s_handler_edits_the_new_objects() {
    let plugin = written(
        "pasted",
        "[event]\nid = \"retag\"\non = \"object-pasted\"\nfor = \"placeable\"\n",
        r#"
        local mg = require("@moonglow")
        mg.on("retag", function(ctx)
            local e = ctx.event
            for _, o in e.objects do
                ctx.edit:set(o.resource, o.path .. "/Tag", `{o.tag}_{e.by}`)
            end
            ctx.log:info(`{#e.objects} in {e.area}`)
            return "Retag the paste"
        end)
        "#,
    );
    let area = ResRef::from_str("tavern").unwrap();
    let mut event = Event::new(EventKind::ObjectPasted, Vec::new());
    event.by = "paste".into();
    event.objects = vec![
        Selected::Object { area, kind: ObjectKind::Placeable, index: 1 },
        Selected::Object { area, kind: ObjectKind::Waypoint, index: 0 },
    ];
    let host = Rc::new(TestHost::default());
    let m = module("TAVERN");
    let outcome =
        run_event(&plugin, "retag", &event, Input::new(m.clone(), None), host.clone()).unwrap();
    assert_eq!(host.log.borrow().as_slice(), ["1 in tavern"]);
    assert_eq!(outcome.label, "Retag the paste");
    let mut ws = mg_edit::Workspace::new(m.clone());
    ws.apply(mg_edit::Command::new("x", outcome.edits)).unwrap();
    let git = ws.doc(&key("tavern.git")).unwrap();
    let tag = git.root.list("Placeable List").unwrap()[1].string("Tag").unwrap();
    assert_eq!(tag, b"LAMP_2_paste");

    // Nothing of its kinds: its code does not run.
    event.objects.remove(0);
    let host = Rc::new(TestHost::default());
    let outcome = run_event(&plugin, "retag", &event, Input::new(m, None), host.clone()).unwrap();
    assert!(outcome.edits.is_empty() && host.log.borrow().is_empty());
}

/// A resource made: the handler is called for each (of the kinds it is
/// for), with its kind, what made it and all that was made; it may edit
/// it at once.
#[test]
fn a_resource_created_s_handler_edits_what_was_made() {
    let plugin = written(
        "created",
        "[event]\nid = \"land\"\non = \"resource-created\"\nfor = \"area\"\n",
        r#"
        local mg = require("@moonglow")
        mg.on("land", function(ctx)
            local e = ctx.event
            ctx.log:info(`{e.resource} {e.kind} by {e.by}: {table.concat(e.made, " ")}`)
            ctx.edit:insert((string.gsub(e.resource, "%.are$", ".git")), "/VarTable",
                { __struct_id = 0, Name = mg.string("tdod_land"), Type = mg.dword(3),
                  Value = mg.string("krezk") })
        end)
        "#,
    );
    let mut event =
        Event::new(EventKind::ResourceCreated, vec![key("tavern.are"), key("tavern.git")]);
    event.by = "wizard".into();
    let host = Rc::new(TestHost::default());
    let m = module("TAVERN");
    let outcome =
        run_event(&plugin, "land", &event, Input::new(m.clone(), None), host.clone()).unwrap();
    assert_eq!(host.log.borrow().as_slice(), ["tavern.are area by wizard: tavern.are tavern.git"]);
    let mut ws = mg_edit::Workspace::new(m);
    ws.apply(mg_edit::Command::new("x", outcome.edits)).unwrap();
    let git = ws.doc(&key("tavern.git")).unwrap();
    assert_eq!(git.root.list("VarTable").unwrap()[0].string("Value").unwrap(), b"krezk");
}

/// A handler that runs at once has a time budget: past it, it is stopped
/// (no `pcall` catches that), and the failure says so.
#[test]
fn a_handler_past_its_time_budget_is_stopped() {
    let plugin = written(
        "slow",
        "[event]\nid = \"spin\"\non = \"resource-saving\"\n",
        r#"
        local mg = require("@moonglow")
        mg.on("spin", function(ctx)
            pcall(function() while true do end end)
            ctx.event:warn("never")
        end)
        "#,
    );
    let event = Event::new(EventKind::ResourceSaving, vec![key("guard.utc")]);
    let input = Input { budget: Some(Duration::from_millis(100)), ..Input::new(module("T"), None) };
    let started = std::time::Instant::now();
    let host = Rc::new(TestHost::default());
    let e = run_event(&plugin, "spin", &event, input, host).unwrap_err();
    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(
        matches!(&e, PluginError::Script { message, .. } if message.contains("ran out of time"))
    );
    assert!(e.to_string().contains("0.1 seconds"), "{e}");
}
