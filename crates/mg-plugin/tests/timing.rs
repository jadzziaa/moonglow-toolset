//! Timing views of the review of November 2026's fixes C3 and C4 (the
//! deferred list's "performance fixes without a timing view"): a panel
//! that follows a large selection, and a check whose findings all have a
//! fix, each at the size its finding names and again at a small size
//! beside it. Run by hand, in release; they print `TIMING name = ms`
//! lines and the machine's load:
//! `cargo test --release -p mg-plugin --test plugins timing -- --ignored --nocapture --test-threads 1`.
//! A module of our own is made here: no game data.

use std::rc::Rc;
use std::time::{Duration, Instant};

use mg_core::ResRef;
use mg_gff::{Gff, Struct, Value};
use mg_module::Module;
use mg_plugin::{Input, Plugin, Selected, Selection, run_check, run_panel};
use mg_resman::ResKey;
use mg_rules::ObjectKind;

use crate::common::TestHost;

fn load() -> String {
    std::fs::read_to_string("/proc/loadavg").unwrap_or_default().trim().to_string()
}

fn report(name: &str, took: &[Duration]) {
    let list: Vec<String> =
        took.iter().map(|d| format!("{:.2}", d.as_secs_f64() * 1000.0)).collect();
    println!("TIMING {name} = {} ms (load {})", list.join(" "), load());
}

fn key(name: &str) -> ResKey {
    ResKey::from_filename(name).unwrap()
}

fn written(name: &str, sections: &str, code: &str) -> Plugin {
    let dir = mg_testkit::scratch_dir(name);
    let head = "[plugin]\nid = \"test.timing\"\nname = \"Timing\"\nversion = \"1\"\napi = \"0.6\"\nlicense = \"MIT\"\n";
    std::fs::write(dir.join("plugin.cfg"), format!("{head}\n{sections}")).unwrap();
    std::fs::write(dir.join("main.luau"), format!("local mg = require(\"@moonglow\")\n{code}"))
        .unwrap();
    Plugin::load(&dir).unwrap()
}

/// C3: a panel with `refresh = "selection"` over `selected` of the
/// `placeables` of one area, and of as many of its tiles: its time and
/// what it said (before the fix a panel was stopped at 250 ms, the
/// building of its context counted).
fn panel_over_a_selection(label: &str, placeables: usize, selected: usize) {
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    let mut git = Gff::new(*b"GIT ");
    let mut list = Vec::new();
    for i in 0..placeables {
        let mut p = Struct::new(9);
        p.set("Tag", Value::String(format!("PLC_{i}").into_bytes()));
        p.set("TemplateResRef", Value::resref(ResRef::from_str("plc_chest1").unwrap()));
        p.set("XPosition", Value::Float(i as f32));
        p.set("YPosition", Value::Float(1.0));
        let mut vars = Vec::new();
        for v in 0..4 {
            let mut var = Struct::new(0);
            var.set("Name", Value::String(format!("var{v}").into_bytes()));
            var.set("Type", Value::Dword(1));
            var.set("Value", Value::Int(v));
            vars.push(var);
        }
        p.set("VarTable", Value::List(vars));
        list.push(p);
    }
    git.root.set("Placeable List", Value::List(list));
    m.set_gff(key("big.git"), &git).unwrap();
    let mut are = Gff::new(*b"ARE ");
    let tiles: Vec<Struct> = (0..1024).map(|_| Struct::new(1)).collect();
    are.root.set("Tile_List", Value::List(tiles));
    are.root.set("Tag", Value::String(b"BIG".to_vec()));
    m.set_gff(key("big.are"), &are).unwrap();
    let area = ResRef::from_str("big").unwrap();
    let plugin = written(
        "plugin-timing-panel",
        "[panel]\nid = \"view\"\ntitle = \"View\"\nrefresh = \"selection\"\n",
        r#"mg.panel("view", function(ctx)
    local n = 0
    for _, s in ipairs(ctx.selection) do
        if s.tag or s.index then n = n + 1 end
    end
    return { { type = "value", label = "Selected", text = n } }
end)
"#,
    );
    for (what, selection) in [
        (
            "objects",
            (0..selected)
                .map(|index| Selected::Object { area, kind: ObjectKind::Placeable, index })
                .collect::<Vec<_>>(),
        ),
        (
            "tiles",
            (0..(if selected > 100 { 1024 } else { selected }))
                .map(|index| Selected::Tile { area, index })
                .collect(),
        ),
    ] {
        let n = selection.len();
        let mut took = Vec::new();
        let mut said = String::new();
        for _ in 0..3 {
            let input = Input {
                selection: Selection { selected: selection.clone(), ..Default::default() },
                ..Input::new(m.clone(), None)
            };
            let host = Rc::new(TestHost::default());
            let t = Instant::now();
            let shown = run_panel(&plugin, "view", input, host);
            took.push(t.elapsed());
            said = match shown {
                Ok(s) => format!("shown, {} elements", s.elements.len()),
                Err(e) => format!("{e}"),
            };
        }
        report(&format!("C3 {label}: a panel over {n} selected {what} ({said})"), &took);
    }
}

#[test]
#[ignore]
fn c3_a_panel_follows_a_large_selection() {
    println!("load at start {}", load());
    panel_over_a_selection("20 of 100 placeables", 100, 20);
    panel_over_a_selection("a GIT of 2,000 placeables", 2_000, 500);
}

/// C4: a check that gives each of its findings a fix, on a module of
/// `resources` creatures, `fixes` of them.
fn check_with_fixes(label: &str, resources: usize, fixes: usize) {
    let plugin = written(
        "plugin-timing-fixes",
        "[check]\nid = \"tags\"\ntitle = \"Tags\"\n",
        &format!(
            r#"mg.check("tags", function(ctx)
    local found = {{}}
    for i = 1, {fixes} do
        local name = `c{{i}}.utc`
        local fix = ctx.fix("Upper-case this tag")
        fix:set(name, "Tag", `TAG{{i}}`)
        table.insert(found, {{ resource = name, message = "lower case", fix = fix }})
    end
    return found
end)
"#
        ),
    );
    let mut m = Module::new();
    m.set_info(&Gff::new(*b"IFO ")).unwrap();
    let mut g = Gff::new(*b"UTC ");
    g.root.set("Tag", Value::String(b"tag".to_vec()));
    let bytes = g.to_bytes().unwrap();
    for i in 1..=resources {
        m.set(key(&format!("c{i}.utc")), bytes.clone());
    }
    let mut took = Vec::new();
    let mut said = String::new();
    for _ in 0..3 {
        let host = Rc::new(TestHost::default());
        let t = Instant::now();
        let found = run_check(&plugin, "tags", Input::new(m.clone(), None), host);
        took.push(t.elapsed());
        said = match found {
            Ok(f) => format!("{} findings", f.len()),
            Err(e) => format!("{e}"),
        };
    }
    report(
        &format!("C4 {label}: a check with {fixes} fixes over {resources} resources ({said})"),
        &took,
    );
}

#[test]
#[ignore]
fn c4_a_check_with_many_fixes() {
    println!("load at start {}", load());
    check_with_fixes("small", 500, 100);
    check_with_fixes("large", 10_000, 2_000);
}
