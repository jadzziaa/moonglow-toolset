//! Find and Replace over a campaign: Daggerford renamed throughout Darkness
//! over Daggerford's names, descriptions, conversations and journal (the
//! campaign with the most text of its own; BioWare's keep theirs in the
//! talk table). Every occurrence moves to the new name, and resources
//! without it are written back byte for byte.

use mg_module::Module;
use mg_module::text::{Options, TextKind, find, replace};
use mg_testkit::corpus;

#[test]
fn a_name_is_replaced_throughout_a_campaign() {
    let root = corpus!();
    let path = root.join("data/nwm/Neverwinter Nights - Darkness over Daggerford.nwm");
    if !path.is_file() {
        eprintln!("skipped: Darkness over Daggerford isn't installed");
        return;
    }
    let original = Module::open(&path).unwrap();
    let mut m = Module::open(&path).unwrap();
    let o = Options { match_case: true, whole_word: true, game: None };
    let kinds = TextKind::ALL;
    let hits = find(&m, "Daggerford", o, &kinds);
    let before: usize = hits.iter().map(|h| h.count).sum();
    assert!(hits.len() > 50, "{} strings", hits.len());
    let by_kind = |k| hits.iter().filter(|h| h.kind == k).count();
    println!(
        "{} strings, {before} times: {} names, {} conversation lines, {} journal",
        hits.len(),
        by_kind(TextKind::Name),
        by_kind(TextKind::Conversation),
        by_kind(TextKind::Journal)
    );
    assert!(by_kind(TextKind::Conversation) > 0 && by_kind(TextKind::Journal) > 0);
    let already = find(&m, "Dagford", o, &kinds).iter().map(|h| h.count).sum::<usize>();

    let (n, errors) = replace(&mut m, &hits, "Daggerford", "Dagford", o);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(n, before);
    assert!(find(&m, "Daggerford", o, &kinds).is_empty());
    let after: usize = find(&m, "Dagford", o, &kinds).iter().map(|h| h.count).sum();
    assert_eq!(after, before + already);
    // Only the resources she's in changed.
    let changed: std::collections::HashSet<_> = hits.iter().map(|h| h.key).collect();
    for k in original.keys() {
        let same = original.get(k) == m.get(k);
        assert_eq!(same, !changed.contains(k), "{k}");
    }
}
