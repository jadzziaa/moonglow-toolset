//! Update Instances, compared with Aurora's. The probes are made from the
//! placement captures (`aurora_instances.rs`): every placed object is given
//! values of its own (tag, name, a script, a variable, plot and so on) and
//! its blueprint other values for the same fields, and the area is copied
//! as `field2`. Aurora's palette › Update Instances on each blueprint, then
//! File › Save, is `update-<probe>.mod`. Moonglow's update of the same probe
//! must give the same objects.
//!
//! To make the probes for the oracle:
//! `cargo test -p mg-corpus-tests --test aurora_update_instances -- --ignored`
//! writes `update-probe-defaults.mod` and `update-probe-doors.mod` into the
//! oracle's `modules` folder.

use mg_core::{Gender, Language, LocString, ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::Module;
use mg_resman::ResKey;
use mg_testkit::{aurora_capture, corpus};

/// The probes: (the placement capture they're made from, their name).
const PROBES: [(&str, &str); 2] = [
    ("placement-defaults.mod", "update-probe-defaults"),
    ("placement-doors.mod", "update-probe-doors"),
];

/// The GIT lists and the blueprint type of each.
const LISTS: [(&str, ResType); 9] = [
    ("Creature List", ResType::UTC),
    ("Door List", ResType::UTD),
    ("Encounter List", ResType::UTE),
    ("List", ResType::UTI),
    ("Placeable List", ResType::UTP),
    ("SoundList", ResType::UTS),
    ("StoreList", ResType::UTM),
    ("TriggerList", ResType::UTT),
    ("WaypointList", ResType::UTW),
];

fn text(s: &str) -> Value {
    Value::String(s.as_bytes().to_vec())
}

fn loc(s: &str) -> Value {
    Value::LocString(LocString::from_text(Language::ENGLISH, Gender::Male, s))
}

/// The name field of a type.
fn name_field(t: ResType) -> &'static str {
    match t {
        ResType::UTC => "FirstName",
        ResType::UTI | ResType::UTW | ResType::UTE | ResType::UTT => "LocalizedName",
        _ => "LocName",
    }
}

/// A script field each type has.
fn script_field(t: ResType) -> Option<&'static str> {
    Some(match t {
        ResType::UTC => "ScriptSpawn",
        ResType::UTD => "OnOpen",
        ResType::UTP => "OnUsed",
        ResType::UTT => "ScriptOnEnter",
        ResType::UTE => "OnEntered",
        ResType::UTM => "OnOpenStore",
        _ => return None,
    })
}

/// Gives an object (`instance`) or its blueprint values of their own for
/// the same fields.
fn mark(s: &mut Struct, t: ResType, instance: bool) {
    let (who, n) = if instance { ("INST", 1) } else { ("BP", 2) };
    let tag = s.string("Tag").map(|b| String::from_utf8_lossy(b).into_owned()).unwrap_or_default();
    s.set("Tag", text(&format!("{who}_{tag}")));
    s.set(name_field(t), loc(&format!("{who} name")));
    if let Some(f) = script_field(t) {
        s.set(f, Value::resref(ResRef::from_str(&format!("mg_{}", who.to_lowercase())).unwrap()));
    }
    let mut var = Struct::new(0);
    var.set("Name", text(&format!("MG_{who}")));
    var.set("Type", Value::Dword(1));
    var.set("Value", Value::Int(n));
    s.set("VarTable", Value::List(vec![var]));
    match t {
        ResType::UTC | ResType::UTD | ResType::UTP | ResType::UTI => {
            s.set("Plot", Value::Byte(u8::from(instance)));
        }
        _ => {}
    }
    match t {
        ResType::UTD | ResType::UTP => {
            s.set("Locked", Value::Byte(u8::from(instance)));
            s.set("Description", loc(&format!("{who} description")));
        }
        ResType::UTC => s.set("Description", loc(&format!("{who} description"))),
        ResType::UTS => s.set("Volume", Value::Byte(if instance { 50 } else { 100 })),
        ResType::UTW => {
            s.set("HasMapNote", Value::Byte(1));
            s.set("MapNote", loc(&format!("{who} note")));
        }
        _ => {}
    }
    if instance && t == ResType::UTD {
        s.set("LinkedTo", text("MG_LINK"));
        s.set("LinkedToFlags", Value::Byte(2));
    }
}

/// A probe made from a placement capture.
fn probe(capture: &std::path::Path) -> Module {
    let mut m = Module::open(capture).unwrap();
    let area = m.areas().unwrap()[0];
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let mut blueprints = Vec::new();
    for (list, t) in LISTS {
        let Some(items) = git.root.list_mut(list) else { continue };
        for s in items {
            let field = if t == ResType::UTM { "ResRef" } else { "TemplateResRef" };
            if let Some(r) = s.resref(field).filter(|r| !r.is_empty()) {
                blueprints.push(ResKey::new(r, t));
            }
            mark(s, t, true);
        }
    }
    m.set_gff(git_key, &git).unwrap();
    blueprints.sort();
    blueprints.dedup();
    for k in blueprints {
        let Some(Ok(mut g)) = m.gff(&k) else { continue };
        mark(&mut g.root, k.restype, false);
        m.set_gff(k, &g).unwrap();
    }
    // The area again, as field2.
    let copy = ResRef::from_str("field2").unwrap();
    for t in [ResType::ARE, ResType::GIT, ResType::GIC] {
        if let Some(Ok(mut g)) = m.gff(&ResKey::new(area, t)) {
            if t == ResType::ARE {
                g.root.set("ResRef", Value::resref(copy));
                g.root.set("Tag", text("FIELD2"));
                g.root.set("Name", loc("Field 2"));
            }
            m.set_gff(ResKey::new(copy, t), &g).unwrap();
        }
    }
    let mut info = m.info().unwrap();
    let mut list = info.root.list("Mod_Area_list").unwrap().to_vec();
    let mut entry = list[0].clone();
    entry.set("Area_Name", Value::resref(copy));
    list.push(entry);
    info.root.set("Mod_Area_list", Value::List(list));
    m.set_info(&info).unwrap();
    m
}

#[test]
#[ignore]
fn write_the_probes_for_the_oracle() {
    let oracle =
        std::env::var_os("MOONGLOW_ORACLE").map(std::path::PathBuf::from).unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var_os("HOME").unwrap())
                .join(".local/share/moonglow-oracle")
        });
    let dir = oracle.join("userdir/modules");
    for (capture, name) in PROBES {
        let Some(path) = mg_testkit::aurora_capture(capture) else {
            panic!("no capture {capture}");
        };
        let m = probe(&path);
        std::fs::write(dir.join(format!("{name}.mod")), m.to_archive_bytes().unwrap()).unwrap();
        println!("wrote {}", dir.join(format!("{name}.mod")).display());
    }
}

/// Field labels, types and values, one line each (floats to 4 places:
/// Aurora's carry noise).
fn lines(s: &Struct, indent: &str, out: &mut Vec<String>) {
    for f in &s.fields {
        let label = f.label.to_string_lossy();
        match &f.value {
            Value::List(items) => {
                out.push(format!("{indent}{label} list[{}]", items.len()));
                for (i, item) in items.iter().enumerate() {
                    out.push(format!("{indent}  [{i}] struct {}", item.id));
                    lines(item, &format!("{indent}    "), out);
                }
            }
            Value::Struct(c) => {
                out.push(format!("{indent}{label} struct {}", c.id));
                lines(c, &format!("{indent}  "), out);
            }
            Value::Float(v) => {
                let v = if v.abs() < 1e-6 { 0.0 } else { *v };
                out.push(format!("{indent}{label} Float({v:.4})"))
            }
            v => out.push(format!("{indent}{label} {v:?}")),
        }
    }
}

#[test]
fn update_instances_matches_aurora() {
    use mg_module::instances::{Placing, update};
    use mg_resman::GameInstall;
    use mg_rules::GameData;
    let root = corpus!();
    let game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut failures = Vec::new();
    let mut compared = 0;
    for (capture, name) in PROBES {
        let source = aurora_capture!(capture);
        let aurora =
            Module::open(&aurora_capture!(&format!("{}.mod", name.replace("probe", "aurora"))))
                .unwrap();
        let m = probe(&source);
        let gff = |k: ResKey| -> Option<Gff> {
            match m.gff(&k) {
                Some(g) => g.ok(),
                None => Gff::read(&game.resman.get(&k).ok()?).ok(),
            }
        };
        let item = |r: ResRef| gff(ResKey::new(r, ResType::UTI)).map(|g| g.root);
        let placing = Placing { game: &game, item: &item };
        // Only the module's own blueprints are updated.
        let blueprint = |t: ResType, r: ResRef| m.gff(&ResKey::new(r, t))?.ok().map(|g| g.root);
        for area in m.areas().unwrap() {
            let key = ResKey::new(area, ResType::GIT);
            let before = m.gff(&key).unwrap().unwrap();
            let (ours, n) = update(&placing, &before.root, &blueprint).expect("objects to update");
            let theirs = aurora.gff(&key).unwrap().unwrap();
            for (list, _) in LISTS {
                let ours = ours.list(list).unwrap_or(&[]);
                let theirs = theirs.root.list(list).unwrap_or(&[]);
                assert_eq!(ours.len(), theirs.len(), "{name} {area} {list}");
                for (i, (o, t)) in ours.iter().zip(theirs).enumerate() {
                    // Objects without a blueprint aren't updated; Aurora
                    // writes every object of an area it saves again.
                    let field = if list == "StoreList" { "ResRef" } else { "TemplateResRef" };
                    if o.resref(field).is_none_or(|r| r.is_empty()) {
                        continue;
                    }
                    let (mut a, mut b) = (Vec::new(), Vec::new());
                    lines(o, "", &mut a);
                    lines(t, "", &mut b);
                    // Aurora's quirks with blueprints lacking Comment (a
                    // creature's empty Comment, and its first skill's), not
                    // copied (`instances.rs`).
                    if !a.iter().any(|l| l.trim_start().starts_with("Comment ")) {
                        b.retain(|l| !l.trim_start().starts_with("Comment "));
                    }
                    // In the area it has open, Aurora writes an item's cost
                    // as 0; in the others, the cost (as Moonglow does).
                    if list == "List" && area.to_string() == "field" {
                        a.retain(|l| !l.starts_with("Cost "));
                        b.retain(|l| !l.starts_with("Cost "));
                    }
                    compared += 1;
                    if a != b {
                        let diff: Vec<String> = a
                            .iter()
                            .filter(|l| !b.contains(l))
                            .map(|l| format!("  ours   {l}"))
                            .chain(
                                b.iter()
                                    .filter(|l| !a.contains(l))
                                    .map(|l| format!("  aurora {l}")),
                            )
                            .collect();
                        failures.push(format!("{name} {area} {list}[{i}]:\n{}", diff.join("\n")));
                    }
                }
            }
            println!("{name} {area}: {n} updated");
        }
    }
    assert!(compared > 0);
    assert!(failures.is_empty(), "{} differ:\n{}", failures.len(), failures.join("\n"));
}
