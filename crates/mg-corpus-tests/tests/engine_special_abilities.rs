//! A creature's special abilities (`SpecAbilityList`: `Spell`,
//! `SpellFlags`, `SpellCasterLevel`), settled in the engine: what it makes
//! of the flags BioWare's creature format names Ready (1), Spontaneous (2)
//! and Unlimited (4): an entry with any of them is a use the creature
//! has, one with none is a use spent, and none of them makes its uses
//! unlimited.

use std::time::Duration;

use mg_core::ResType;
use mg_gff::{Gff, Struct, Value};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

mod common;
use common::probe_module;

const PROBE: &str = r#"
void Log(string s) { WriteTimestampedLogEntry(s); }
string Has(object c)
{
    string s = "";
    int n;
    for (n = 0; n < 6; n++) s += IntToString(GetHasSpell(n, c)) + " ";
    return s;
}

void main()
{
    object c = CreateObject(OBJECT_TYPE_CREATURE, "mg_abilities", GetStartingLocation());
    Log("MG_HAS " + IntToString(GetIsObjectValid(c)) + " " + Has(c));
    // One use of each taken.
    int n;
    for (n = 0; n < 6; n++) DecrementRemainingSpellUses(c, n);
    Log("MG_USED " + Has(c));
    Log("MG_DONE");
}
"#;

#[test]
fn the_engine_counts_special_abilities_by_their_flags() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let gd = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let data = gd.resman.get(&ResKey::parse("nw_chicken", ResType::UTC).unwrap()).unwrap();
    let mut utc = Gff::read(&data).unwrap();
    utc.root.set("TemplateResRef", Value::ResRef(b"mg_abilities".to_vec()));
    // Spell 0 ready twice, 1 not ready, 2 spontaneous, 3 unlimited,
    // 4 ready and unlimited; 5 not there.
    let ability = |spell: u16, flags: u8| {
        let mut s = Struct::new(4);
        s.set("Spell", Value::Word(spell));
        s.set("SpellFlags", Value::Byte(flags));
        s.set("SpellCasterLevel", Value::Byte(5));
        s
    };
    utc.root.set(
        "SpecAbilityList",
        Value::List(vec![
            ability(0, 1),
            ability(0, 1),
            ability(1, 0),
            ability(2, 2),
            ability(3, 4),
            ability(4, 5),
        ]),
    );
    let extra = [(ResKey::parse("mg_abilities", ResType::UTC).unwrap(), utc.to_bytes().unwrap())];
    let dir = scratch_dir("engine_special_abilities");
    probe_module(&root, &dir, "mg_abil_probe", PROBE, &[], &extra);
    let run = run_server(&root, &dir, "mg_abil_probe", "MG_DONE", Duration::from_secs(60)).unwrap();
    assert!(
        run.finished,
        "server did not finish; log tail:\n{}",
        &run.log[run.log.len().saturating_sub(3000)..]
    );
    // An ability with any flag set can be used (`GetHasSpell` says 1,
    // however many entries it has); one with none cannot.
    assert_eq!(run.values("MG_HAS"), ["1 1 0 1 1 1 0"]);
    // Each entry is one use: of two, one is left. "Unlimited" is used up
    // like the rest.
    assert_eq!(run.values("MG_USED"), ["1 0 0 0 0 0"]);
}
