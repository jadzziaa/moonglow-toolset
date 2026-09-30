//! A conversation built with Moonglow's Conversation Editor operations
//! compared with Aurora's (capture `dialog/capdlg.dlg`): Aurora added the
//! greeting "Hello there." with the reply "Who are you?" and the answer
//! "A traveller.", a second greeting "Go away." with condition `sc_gate`
//! (nMin=3), action `at_do` (sTag=guard), the Taunt animation and a comment,
//! and pasted "Who are you?" as a link under "Go away.". The node lists'
//! order differs (Aurora renumbers on save); the conversations are the same.

use mg_gff::{Gff, Value};
use mg_module::dialog::{Kind, Parent, add_link, add_node, new_dialog, outline, params_value};
use mg_testkit::aurora_capture;

#[test]
fn conversation_matches_aurora() {
    let aurora = Gff::read(&std::fs::read(aurora_capture!("dialog/capdlg.dlg")).unwrap()).unwrap();
    let mut g = new_dialog();
    let hello = add_node(&mut g, Parent::Root, "Hello there.");
    let who = add_node(&mut g, Parent::Node(Kind::Entry, hello), "Who are you?");
    add_node(&mut g, Parent::Node(Kind::Reply, who), "A traveller.");
    let go = add_node(&mut g, Parent::Root, "Go away.");
    {
        let start = &mut g.root.list_mut("StartingList").unwrap()[1];
        start.set("Active", Value::resref(mg_core::ResRef::from_str("sc_gate").unwrap()));
        start.set("ConditionParams", params_value(&[("nMin".into(), "3".into())]));
        let n = &mut g.root.list_mut("EntryList").unwrap()[go as usize];
        n.set("Script", Value::resref(mg_core::ResRef::from_str("at_do").unwrap()));
        n.set("ActionParams", params_value(&[("sTag".into(), "guard".into())]));
        n.set("Animation", Value::Dword(28));
        n.set("Comment", Value::String(b"a comment".to_vec()));
    }
    add_link(&mut g, Parent::Node(Kind::Entry, go), who);

    assert_eq!(outline(&g), outline(&aurora));
    for field in [
        "DelayEntry",
        "DelayReply",
        "NumWords",
        "EndConversation",
        "EndConverAbort",
        "PreventZoomIn",
    ] {
        assert_eq!(g.root.get(field), aurora.root.get(field), "{field}");
    }
    // Each line has Aurora's fields in Aurora's order.
    let fields = |g: &Gff, list: &str| -> Vec<Vec<String>> {
        g.root
            .list(list)
            .unwrap()
            .iter()
            .map(|s| s.fields.iter().map(|f| f.label.to_string_lossy()).collect())
            .collect()
    };
    assert_eq!(fields(&g, "EntryList")[0], fields(&aurora, "EntryList")[1], "entry fields");
    assert_eq!(fields(&g, "ReplyList"), fields(&aurora, "ReplyList"), "reply fields");
}
