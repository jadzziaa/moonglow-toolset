//! Where things are, as the command line says it: `mg refs` (a resource's
//! and a tag's), `mg search`, `mg replace` and `mg rename` on a module the
//! test builds, without the game, each printed in full (text and JSON)
//! and held to what they printed when GFF paths were text, byte for byte.

use mg_core::{Gender, Language, LocString, ResRef, ResType, StrRef};
use mg_gff::{Gff, Struct, Value};
use mg_module::{Module, ModuleLocation};
use mg_resman::ResKey;

fn rr(s: &str) -> Value {
    Value::resref(ResRef::from_str(s).unwrap())
}

fn text(s: &str) -> Value {
    Value::String(s.as_bytes().to_vec())
}

fn english(s: &str) -> Value {
    Value::LocString(LocString::from_text(Language::ENGLISH, Gender::Male, s))
}

fn with(id: u32, fields: Vec<(&str, Value)>) -> Struct {
    let mut s = Struct::new(id);
    for (label, v) in fields {
        s.set(label, v);
    }
    s
}

fn gff(file_type: &[u8; 4], fields: Vec<(&str, Value)>) -> Gff {
    let mut g = Gff::new(*file_type);
    g.root = with(u32::MAX, fields);
    g
}

/// A keep with a dozen guards (so that a list's items run past 9), a
/// door to a waypoint, an encounter, items lying about and carried, a
/// conversation with a link, the module's own references, and scripts
/// that include and spell `hello`.
fn module(dir: &std::path::Path) -> std::path::PathBuf {
    let mut m = Module::new();
    let mut set = |name: &str, t: ResType, g: Gff| {
        m.set_gff(ResKey::parse(name, t).unwrap(), &g).unwrap();
    };
    let var = || {
        with(0, vec![("Name", text("sDeidad")), ("Type", Value::Dword(3)), ("Value", text("Ezra"))])
    };
    let mut guards = vec![with(
        4,
        vec![
            ("Tag", text("GUARD")),
            ("TemplateResRef", rr("guard")),
            ("ScriptSpawn", rr("hello")),
            ("Conversation", rr("talk")),
            ("FirstName", english("Ezra's guard")),
            (
                "ItemList",
                Value::List(vec![with(
                    0,
                    vec![("InventoryRes", rr("sword")), ("Tag", text("SWORD"))],
                )]),
            ),
            ("VarTable", Value::List(vec![var()])),
        ],
    )];
    for _ in 1..12 {
        guards.push(with(
            4,
            vec![
                ("Tag", text("GUARD")),
                ("TemplateResRef", rr("guard")),
                ("ScriptHeartbeat", rr("hello")),
            ],
        ));
    }
    set(
        "keep",
        ResType::GIT,
        gff(
            b"GIT ",
            vec![
                ("Creature List", Value::List(guards)),
                (
                    "Door List",
                    Value::List(vec![with(
                        8,
                        vec![
                            ("Tag", text("DOOR1")),
                            ("TemplateResRef", rr("door1")),
                            ("LinkedTo", text("WP_IN")),
                            ("OnOpen", rr("hello")),
                        ],
                    )]),
                ),
                (
                    "WaypointList",
                    Value::List(vec![with(
                        5,
                        vec![("Tag", text("WP_IN")), ("TemplateResRef", rr("wp"))],
                    )]),
                ),
                (
                    "Encounter List",
                    Value::List(vec![with(
                        7,
                        vec![
                            ("Tag", text("ENC")),
                            ("TemplateResRef", rr("enc")),
                            (
                                "CreatureList",
                                Value::List(vec![with(0, vec![("ResRef", rr("guard"))])]),
                            ),
                        ],
                    )]),
                ),
                (
                    "List",
                    Value::List(vec![with(
                        0,
                        vec![("Tag", text("SWORD")), ("TemplateResRef", rr("sword"))],
                    )]),
                ),
                ("AreaProperties", Value::Struct(with(100, vec![("MusicDay", Value::Int(1))]))),
            ],
        ),
    );
    set(
        "keep",
        ResType::ARE,
        gff(
            b"ARE ",
            vec![
                ("Tag", text("KEEP")),
                ("ResRef", rr("keep")),
                ("Name", english("The Keep of Ezra")),
                ("OnEnter", rr("hello")),
            ],
        ),
    );
    set(
        "module",
        ResType::IFO,
        gff(
            b"IFO ",
            vec![
                ("Mod_Name", english("Places")),
                ("Mod_OnModLoad", rr("hello")),
                ("Mod_Entry_Area", rr("keep")),
                ("Mod_Area_list", Value::List(vec![with(6, vec![("Area_Name", rr("keep"))])])),
            ],
        ),
    );
    let link = |active: &str| {
        Value::List(vec![with(0, vec![("Index", Value::Dword(0)), ("Active", rr(active))])])
    };
    set(
        "talk",
        ResType::DLG,
        gff(
            b"DLG ",
            vec![
                ("EndConversation", rr("hello")),
                (
                    "EntryList",
                    Value::List(vec![with(
                        0,
                        vec![
                            ("Text", english("Halt! Who goes there, in Ezra's name?")),
                            ("Script", rr("hello")),
                            ("RepliesList", link("hello")),
                        ],
                    )]),
                ),
                (
                    "ReplyList",
                    Value::List(vec![with(
                        0,
                        vec![
                            ("Text", english("A friend.")),
                            ("Script", rr("hello")),
                            ("EntriesList", Value::List(Vec::new())),
                        ],
                    )]),
                ),
                ("StartingList", link("hello")),
            ],
        ),
    );
    set(
        "guard",
        ResType::UTC,
        gff(
            b"UTC ",
            vec![
                ("Tag", text("GUARD")),
                ("TemplateResRef", rr("guard")),
                ("ScriptSpawn", rr("hello")),
                ("Conversation", rr("talk")),
                ("FirstName", english("Guard")),
                ("ItemList", Value::List(vec![with(0, vec![("InventoryRes", rr("sword"))])])),
                ("Equip_ItemList", Value::List(vec![with(16, vec![("EquippedRes", rr("sword"))])])),
            ],
        ),
    );
    set(
        "sword",
        ResType::UTI,
        gff(
            b"UTI ",
            vec![
                ("Tag", text("SWORD")),
                ("TemplateResRef", rr("sword")),
                ("LocalizedName", Value::LocString(LocString::from_strref(StrRef(16_777_220)))),
                ("DescIdentified", english("Blessed by Ezra.")),
                ("VarTable", Value::List(vec![var()])),
            ],
        ),
    );
    set(
        "enc",
        ResType::UTE,
        gff(
            b"UTE ",
            vec![
                ("Tag", text("ENC")),
                ("TemplateResRef", rr("enc")),
                ("OnEntered", rr("hello")),
                ("CreatureList", Value::List(vec![with(0, vec![("ResRef", rr("guard"))])])),
            ],
        ),
    );
    m.set(ResKey::parse("hello", ResType::NSS).unwrap(), b"void main() {}\n".to_vec());
    let caller =
        b"#include \"hello\"\nvoid main()\n{\n    ExecuteScript(\"hello\", OBJECT_SELF);\n    \
                   SpeakStringByStrRef(16777220);\n}\n";
    m.set(ResKey::parse("caller", ResType::NSS).unwrap(), caller.to_vec());
    let path = dir.join("places.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    path
}

/// `mg ARGS` on no game, its standard output and error as printed.
fn run(nowhere: &std::path::Path, args: &[&str]) -> String {
    let out = crate::mg_on(nowhere, None).args(args).output().unwrap();
    format!(
        "$ mg {}\n{}{}",
        args.join(" "),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn where_things_are_said_as_they_were() {
    let dir = mg_testkit::scratch_dir("mg-places");
    let path = module(&dir);
    let p = path.to_str().unwrap();
    let nowhere = dir.join("no-game");
    let nowhere = nowhere.as_path();
    let mut said = String::new();
    let asked: &[&[&str]] = &[
        &["refs", p, "hello.nss"],
        &["refs", p, "guard.utc"],
        &["refs", p, "sword.uti"],
        &["refs", p, "talk.dlg"],
        &["refs", p, "keep.are"],
        &["refs", p, "--tag", "GUARD"],
        &["refs", p, "--tag", "WP_IN"],
        &["refs", p, "--tag", "SWORD"],
        &["search", p, "--script", "hello"],
        &["search", p, "--var", "sDeidad"],
        &["search", p, "--field", "Tag", "--text", "GUARD"],
        &["search", p, "--int", "0"],
        &["replace", p, "Ezra", "Zed", "--only", "all", "--dry-run"],
    ];
    for args in asked {
        said.push_str(&run(nowhere, args));
        let mut json = vec!["--json"];
        json.extend_from_slice(args);
        said.push_str(&run(nowhere, &json));
    }
    // A rename changes the module: a copy each time.
    for json in [false, true] {
        let copy = dir.join(format!("renamed-{json}.mod"));
        std::fs::copy(&path, &copy).unwrap();
        let c = copy.to_str().unwrap();
        let mut args = vec!["rename", c, "guard.utc", "sentry"];
        if json {
            args.insert(0, "--json");
        }
        said.push_str(&run(nowhere, &args).replace(c, "COPY"));
        said.push_str(&run(nowhere, &["--json", "refs", c, "sentry.utc"]).replace(c, "COPY"));
    }
    let said = said.replace(p, "MODULE").replace(nowhere.to_str().unwrap(), "NOWHERE");
    let expected = include_str!("places.txt");
    if said != expected {
        let at = mg_testkit::output_dir("mg-places").join("places.txt");
        std::fs::write(&at, &said).unwrap();
        panic!("what mg says differs from tests/places.txt: it said {}", at.display());
    }
}
