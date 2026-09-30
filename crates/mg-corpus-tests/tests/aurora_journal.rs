//! The Journal Editor's new categories and entries compared with Aurora's:
//! in a module without a journal, Aurora added two categories, two entries
//! to the first and one to the second (capture `journal/two-categories.jrl`).

use mg_gff::{Gff, Value};
use mg_module::journal::{entries, new_category, new_entry, new_journal};
use mg_schema::jrl;
use mg_testkit::aurora_capture;

#[test]
fn journal_editor_matches_aurora() {
    let aurora =
        Gff::read(&std::fs::read(aurora_capture!("journal/two-categories.jrl")).unwrap()).unwrap();
    let mut g = new_journal();
    let mut cats = Vec::new();
    cats.push(new_category(&cats));
    cats.push(new_category(&cats));
    for (c, n) in [(0, 2), (1, 1)] {
        let mut list = entries(&cats[c]).to_vec();
        for _ in 0..n {
            list.push(new_entry(&list));
        }
        cats[c].set(jrl::categories::ENTRY_LIST.label, Value::List(list));
    }
    g.root.set(jrl::CATEGORIES.label, Value::List(cats));
    assert_eq!(g, aurora);
}
