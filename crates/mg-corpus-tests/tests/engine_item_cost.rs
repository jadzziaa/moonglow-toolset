//! Item costs compared with the engine: every base-game item blueprint,
//! created in `nwserver` and re-identified (which makes the engine compute
//! its value again from its properties), against `mg_rules::item_cost`.

use std::collections::HashMap;
use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

mod common;
use common::probe_module;

/// Items per delayed chunk, each its own script run (the instruction limit).
const CHUNK: usize = 150;

/// The engine's value of each item: (as created, after re-identifying,
/// stack size, charges), by resref.
fn engine_values(
    root: &std::path::Path,
    resrefs: &[String],
    extra: &[(ResKey, Vec<u8>)],
) -> HashMap<String, [i64; 4]> {
    let dir = scratch_dir("engine_item_cost");
    let mut script = String::from(
        "void Probe(string r)\n{\n\
         \x20   object o = CreateObject(OBJECT_TYPE_ITEM, r, GetStartingLocation());\n\
         \x20   if (!GetIsObjectValid(o)) { WriteTimestampedLogEntry(\"MG_COST \" + r + \" invalid\"); return; }\n\
         \x20   int v0 = GetGoldPieceValue(o);\n\
         \x20   SetIdentified(o, FALSE);\n\
         \x20   SetIdentified(o, TRUE);\n\
         \x20   int v1 = GetGoldPieceValue(o);\n\
         \x20   WriteTimestampedLogEntry(\"MG_COST \" + r + \" \" + IntToString(v0) + \" \" + IntToString(v1)\n\
         \x20       + \" \" + IntToString(GetItemStackSize(o)) + \" \" + IntToString(GetItemCharges(o)));\n\
         \x20   DestroyObject(o);\n}\n",
    );
    let chunks: Vec<_> = resrefs.chunks(CHUNK).collect();
    for (i, chunk) in chunks.iter().enumerate() {
        script += &format!("void Chunk{i}()\n{{\n");
        for r in *chunk {
            script += &format!("    Probe(\"{r}\");\n");
        }
        script += "}\n";
    }
    script += "void Done() { WriteTimestampedLogEntry(\"MG_DONE\"); }\n";
    script += "void main()\n{\n";
    for i in 0..chunks.len() {
        script += &format!("    DelayCommand({}.0, Chunk{i}());\n", i + 1);
    }
    script += &format!("    DelayCommand({}.0, Done());\n}}\n", chunks.len() + 2);
    probe_module(root, &dir, "mg_cost", &script, &[], extra);
    let timeout = Duration::from_secs(60 + chunks.len() as u64 * 2);
    let run = run_server(root, &dir, "mg_cost", "MG_DONE", timeout).unwrap();
    assert!(
        run.finished,
        "server did not finish; log tail:\n{}",
        &run.log[run.log.len().saturating_sub(3000)..]
    );
    run.values("MG_COST")
        .into_iter()
        .filter_map(|v| {
            let mut it = v.split_whitespace();
            let r = it.next()?.to_string();
            let n: Vec<i64> = it.filter_map(|x| x.parse().ok()).collect();
            Some((r, n.try_into().ok()?))
        })
        .collect()
}

/// Items made up to settle what the base game does not show: spells used
/// by charges, once, or with no charge per use, on an item with 10 charges.
fn synthetic() -> Vec<(String, Gff)> {
    let mut out = Vec::new();
    for (name, charge_cost) in [("mg_cost_single", 1u16), ("mg_cost_three", 4), ("mg_cost_zero", 7)]
    {
        let mut g = Gff::new(*b"UTI ");
        g.root.set("TemplateResRef", Value::resref(ResRef::from_str(name).unwrap()));
        g.root.set("BaseItem", Value::Int(24));
        g.root.set("Charges", Value::Byte(10));
        g.root.set("StackSize", Value::Word(1));
        g.root.set("Identified", Value::Byte(1));
        let mut p = Struct::new(0);
        p.set("PropertyName", Value::Word(15));
        p.set("Subtype", Value::Word(1));
        p.set("CostTable", Value::Byte(3));
        p.set("CostValue", Value::Word(charge_cost));
        p.set("Param1", Value::Byte(255));
        p.set("Param1Value", Value::Byte(0));
        p.set("ChanceAppear", Value::Byte(100));
        g.root.set("PropertiesList", Value::List(vec![p]));
        out.push((name.to_string(), g));
    }
    out
}

#[test]
fn item_costs_match_the_engine() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let gd = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut items: Vec<(String, Gff)> = gd
        .resman
        .list(ResType::UTI)
        .iter()
        .filter_map(|r| {
            let data = gd.resman.get(&ResKey::new(*r, ResType::UTI)).ok()?;
            Some((r.to_string(), Gff::read(&data).ok()?))
        })
        .collect();
    items.sort_by(|a, b| a.0.cmp(&b.0));
    let extra: Vec<_> = synthetic()
        .into_iter()
        .map(|(name, g)| (ResKey::parse(&name, ResType::UTI).unwrap(), g.to_bytes().unwrap()))
        .collect();
    items.extend(synthetic());
    let names: Vec<String> = items.iter().map(|(r, _)| r.clone()).collect();
    let engine = engine_values(&root, &names, &extra);

    let base_items = gd.table("baseitems").unwrap();
    // Creature weapons and hides (equipped only in creature slots).
    let creature_item = |base: u32| {
        let slots = base_items.get_int(base as usize, "EquipableSlots").unwrap_or(0);
        slots != 0 && slots & !0x3_C000 == 0
    };
    let mut failures = Vec::new();
    let mut compared = 0;
    for (r, g) in &items {
        let Some(&[_, identified, _, _]) = engine.get(r) else {
            failures.push(format!("{r}: not logged by the engine"));
            continue;
        };
        let value = mg_rules::ItemValue::from_gff(&g.root);
        let ours = i64::from(gd.item_cost(&value));
        let plot = g.root.integer("Plot").unwrap_or(0) != 0;
        // The engine's gold value: 0 for plot and creature items, else at
        // least 1 (gold pieces themselves are worth nothing).
        let shown = if plot || creature_item(value.base_item) {
            0
        } else if ours == 0 && value.base_item != 76 {
            1
        } else {
            ours
        };
        compared += 1;
        if shown != identified {
            failures.push(format!("{r}: ours {ours}, engine {identified}"));
        }
    }
    eprintln!("compared {compared} items");
    assert!(failures.is_empty(), "{} mismatches:\n{}", failures.len(), failures.join("\n"));
}
