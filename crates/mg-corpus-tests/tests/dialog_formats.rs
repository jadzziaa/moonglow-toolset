//! Every conversation the game and its modules ship, exported to Twine and
//! Ink and read back, plays as the original (`dialog_io::play_outline`; for
//! those with too many paths to play out, each line leads to the same
//! lines in the same order under the same conditions); its CSV read back
//! changes nothing.

use std::collections::BTreeSet;

use mg_core::ResType;
use mg_gff::Gff;
use mg_module::Module;
use mg_module::dialog::{Kind, Parent, link_index, links, node, nodes, text};
use mg_module::dialog_io::{
    from_ink, from_twee, play_outline, to_csv, to_ink, to_twee, update_from_csv,
};
use mg_resman::{GameInstall, ResMan};
use mg_testkit::corpus;

/// Conversations whose played-out paths run past this are skipped (links
/// back and forth multiply the paths).
const LIMIT: usize = 20_000;

#[test]
fn conversations_survive_twine_ink_and_csv() {
    let root = corpus!();
    let mut dialogs: Vec<(String, Vec<u8>)> = Vec::new();
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    for r in rm.list(ResType::DLG) {
        let k = mg_resman::ResKey::new(r, ResType::DLG);
        dialogs.push((format!("game/{r}"), rm.get(&k).unwrap().into_owned()));
    }
    for path in mg_testkit::bundled_modules(&root) {
        let m = Module::open(&path).unwrap();
        let name = path.file_stem().unwrap().to_string_lossy().into_owned();
        for k in m.keys_of(ResType::DLG) {
            dialogs.push((format!("{name}/{}", k.resref), m.get(k).unwrap().to_vec()));
        }
    }
    let (mut checked, mut structural) = (0, 0);
    let mut failures = Vec::new();
    for (name, data) in &dialogs {
        let Ok(g) = Gff::read(data) else { continue };
        let mut fail = |format: &str, why: String| {
            if failures.len() < 30 {
                failures.push(format!("{name} ({format}): {why}"));
            }
        };
        let Some(want) = play_outline(&g, LIMIT) else {
            structural += 1;
            let want = edges(&g);
            for (format, back) in
                [("Twine", from_twee(&to_twee(&g, name))), ("Ink", from_ink(&to_ink(&g)))]
            {
                match back {
                    Ok(b) if edges(&b) == want => {}
                    Ok(b) => {
                        let e = edges(&b);
                        let missing: Vec<_> = want.difference(&e).take(2).collect();
                        let extra: Vec<_> = e.difference(&want).take(2).collect();
                        fail(format, format!("missing {missing:?}, extra {extra:?}"));
                    }
                    Err(e) => fail(format, e),
                }
            }
            continue;
        };
        checked += 1;
        match from_twee(&to_twee(&g, name)) {
            Ok(back) if play_outline(&back, LIMIT).as_ref() == Some(&want) => {}
            Ok(back) => fail("Twine", first_difference(&want, &play_outline(&back, LIMIT))),
            Err(e) => fail("Twine", e),
        }
        match from_ink(&to_ink(&g)) {
            Ok(back) if play_outline(&back, LIMIT).as_ref() == Some(&want) => {}
            Ok(back) => fail("Ink", first_difference(&want, &play_outline(&back, LIMIT))),
            Err(e) => fail("Ink", e),
        }
        let mut same = g.clone();
        match update_from_csv(&mut same, &to_csv(&g)) {
            Ok(0) => {}
            Ok(n) => fail("CSV", format!("{n} lines changed")),
            Err(e) => fail("CSV", e),
        }
    }
    println!("{checked} conversations played out, {structural} compared line by line");
    assert!(checked > 1000, "{checked}");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn first_difference(want: &[String], got: &Option<Vec<String>>) -> String {
    let Some(got) = got else { return "too large read back".into() };
    let at = want.iter().zip(got).position(|(a, b)| a != b).unwrap_or(want.len().min(got.len()));
    format!(
        "line {at}: want {:?}, got {:?} ({} vs {} lines)",
        want.get(at),
        got.get(at),
        want.len(),
        got.len()
    )
}

/// What each line leads to: `line -> position: child` for every link, lines
/// named by kind, text (trimmed) and tags; a reply told apart by text only.
fn edges(g: &Gff) -> BTreeSet<String> {
    let line = |k: Kind, i: u32| -> String {
        let n = node(g, k, i).unwrap();
        let all = text(n);
        let t: Vec<&str> = all.lines().map(str::trim).collect();
        let tag = |l: &str| n.resref(l).map(|r| r.to_string()).unwrap_or_default();
        let speaker = String::from_utf8_lossy(n.string("Speaker").unwrap_or_default()).into_owned();
        let quest = String::from_utf8_lossy(n.string("Quest").unwrap_or_default()).into_owned();
        format!(
            "{k:?}|{}|{speaker}|{}|{}|{quest}:{}",
            t.join("\n").trim(),
            tag("Script"),
            tag("Sound"),
            if quest.is_empty() { 0 } else { n.integer("QuestEntry").unwrap_or(0) }
        )
    };
    let mut out = BTreeSet::new();
    let mut add = |from: String, parent: Parent| {
        let kind = parent.child_kind();
        for (pos, l) in links(g, parent).iter().enumerate() {
            let cond = l.resref("Active").map(|r| r.to_string()).unwrap_or_default();
            out.insert(format!("{from} -> {pos}: if {cond} {}", line(kind, link_index(l))));
        }
    };
    add("start".into(), Parent::Root);
    for k in [Kind::Entry, Kind::Reply] {
        for i in 0..nodes(g, k).len() as u32 {
            add(line(k, i), Parent::Node(k, i));
        }
    }
    out
}
