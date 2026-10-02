//! Haks over 2 GiB. Aurora (32-bit) and nwsync don't read past 2 GiB, so
//! the wiki tells builders to keep haks under it. The game doesn't either:
//! a resource that starts past the mark is missing to it (a 2DA reads
//! empty, a script doesn't run), while one that starts before it is read
//! whole, and a copy in a hak below doesn't take its place. Moonglow's
//! resource manager does the same, and the content doctor names what the
//! game misses. The haks are sparse files: their bulk is a hole.

use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_resman::{Container, ErfContainer, ResKey};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

mod common;
use common::{compile, probe_module};

fn table(cell: &str) -> Vec<u8> {
    format!("2DA V2.0\n\n   Value\n0  {cell}\n").into_bytes()
}

fn script(tag: &str) -> String {
    format!("void main() {{ WriteTimestampedLogEntry(\"{tag} ran\"); }}\n")
}

/// A hak with `mg_near` (2DA and script) at the start and `mg_far`'s 2DA
/// starting at byte `far`, then its script.
fn write_hak(path: &std::path::Path, far: u64, near_ncs: &[u8], far_ncs: &[u8]) {
    let (t_near, t_far) = (table("near"), table("far"));
    // The header, 5 keys and 5 resource entries come first.
    let data = 160 + 5 * (24 + 8) + (t_near.len() + near_ncs.len()) as u64;
    mg_testkit::erf::write_padded(
        path,
        b"HAK ",
        &[("mg_near", 2017, &t_near), ("mg_near", 2010, near_ncs)],
        ("mg_padding", 10, far - data),
        &[("mg_far", 2017, &t_far), ("mg_far", 2010, far_ncs)],
    )
    .unwrap();
}

#[test]
fn the_game_reads_nothing_that_starts_past_2_gib_in_a_hak() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_big_hak");
    let near = compile(&dir, "mg_near", &script("MG_NEAR"));
    let far = compile(&dir, "mg_far", &script("MG_FAR"));
    let key = |name: &str, t: ResType| ResKey::new(ResRef::from_str(name).unwrap(), t);
    let probe = r#"void main() {
  WriteTimestampedLogEntry("MG_NEAR_CELL [" + Get2DAString("mg_near", "Value", 0) + "]");
  WriteTimestampedLogEntry("MG_FAR_CELL [" + Get2DAString("mg_far", "Value", 0) + "]");
  ExecuteScript("mg_near", OBJECT_SELF);
  ExecuteScript("mg_far", OBJECT_SELF);
  WriteTimestampedLogEntry("MG_DONE");
}
"#;
    const MARK: u64 = 1 << 31;
    // Where mg_far's 2DA starts (its script follows it): well before the
    // mark, straddling it (the script then starts past it), at it, past it.
    let cases = [MARK - 4096, MARK - 8, MARK, MARK + (64 << 20)];
    for (i, at) in cases.into_iter().enumerate() {
        let user = dir.join(format!("user{i}"));
        std::fs::create_dir_all(user.join("hak")).unwrap();
        let hak = user.join("hak/mg_big.hak");
        write_hak(&hak, at, &near, &far);

        // Moonglow reads what the game reads, and lists what it can't.
        let c = ErfContainer::open(&hak).unwrap();
        let script_at = at + table("far").len() as u64;
        let (t_far, s_far) = (key("mg_far", ResType::TWODA), key("mg_far", ResType::NCS));
        assert_eq!(c.read(&t_far).ok().as_deref(), (at < MARK).then_some(&table("far")[..]));
        assert_eq!(c.read(&s_far).ok().as_deref(), (script_at < MARK).then_some(&far[..]));
        let mut unreadable = Vec::new();
        unreadable.extend((at >= MARK).then_some(t_far));
        unreadable.extend((script_at >= MARK).then_some(s_far));
        unreadable.sort();
        assert_eq!(c.unreadable(), unreadable);

        if server_binary(&root).is_none() {
            eprintln!("skipped the engine: no nwserver for this platform");
            return;
        }
        probe_module(&root, &user, "bighak", probe, &["mg_big"], &[]);
        let run = run_server(&root, &user, "bighak", "MG_DONE", Duration::from_secs(300)).unwrap();
        let _ = std::fs::remove_file(&hak);
        assert!(run.finished, "the server didn't finish:\n{}", run.log);
        // The control: what sits at the start is read.
        assert_eq!(run.values("MG_NEAR_CELL"), ["[near]"]);
        assert_eq!(run.values("MG_NEAR ran").len(), 1);
        // A resource is read when it starts before the mark, wherever it
        // ends.
        let cell = if at < MARK { "[far]" } else { "[]" };
        assert_eq!(run.values("MG_FAR_CELL"), [cell], "mg_far at {at:#x}");
        let ran = usize::from(script_at < MARK);
        assert_eq!(run.values("MG_FAR ran").len(), ran, "mg_far at {at:#x}");
    }
}

/// A table past the mark in one hak, and a copy in a hak below it.
#[test]
fn past_2_gib_a_lower_haks_copy_doesnt_count() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let dir = scratch_dir("engine_big_hak_shadow");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    let (high, low) = (table("high"), table("low"));
    mg_testkit::erf::write_padded(
        &user.join("hak/mg_big.hak"),
        b"HAK ",
        &[],
        ("mg_padding", 10, 1 << 31),
        &[("mg_shadow", 2017, &high)],
    )
    .unwrap();
    mg_testkit::erf::write_padded(
        &user.join("hak/mg_low.hak"),
        b"HAK ",
        &[("mg_shadow", 2017, &low)],
        ("mg_empty", 10, 0),
        &[],
    )
    .unwrap();
    let probe = r#"void main() {
  WriteTimestampedLogEntry("MG_CELL [" + Get2DAString("mg_shadow", "Value", 0) + "]");
  WriteTimestampedLogEntry("MG_DONE");
}
"#;
    probe_module(&root, &user, "bighak", probe, &["mg_big", "mg_low"], &[]);
    let run = run_server(&root, &user, "bighak", "MG_DONE", Duration::from_secs(300)).unwrap();
    assert!(run.finished, "the server didn't finish:\n{}", run.log);
    // The game finds the table in the higher hak, can't read it, and
    // doesn't look further.
    assert_eq!(run.values("MG_CELL"), ["[]"]);
}
