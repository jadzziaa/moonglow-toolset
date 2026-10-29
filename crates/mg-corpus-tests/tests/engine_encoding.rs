//! A module's `encoding.2da` (EE 1.87), settled in the engine: the
//! character the game takes each byte of its text for, read where it turns
//! text to UTF-8 and back (its SQLite functions).

use std::time::Duration;

use mg_core::{Codepage, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

use crate::common;
use common::probe_module;

/// The text of bytes 0x20 to 0xFF is a creature's variable; the game says
/// it in UTF-8 (as hex), and where in it the characters SQLite makes are.
const PROBE: &str = r#"
void Log(string s) { WriteTimestampedLogEntry(s); }
// The game's text as the hex of its UTF-8 (a JSON string is UTF-8).
string Utf8(string s)
{
    sqlquery q = SqlPrepareQueryObject(GetModule(), "SELECT hex(@j)");
    SqlBindJson(q, "@j", JsonString(s));
    if (!SqlStep(q)) return "<none>";
    return SqlGetString(q, 0);
}
// A Unicode character as the game's text.
string Char(int n)
{
    sqlquery q = SqlPrepareQueryObject(GetModule(), "SELECT json_quote(char(" + IntToString(n) + "))");
    if (!SqlStep(q)) return "<none>";
    return JsonGetString(SqlGetJson(q, 0));
}

void Probe();
void main()
{
    DelayCommand(2.0, Probe());
}
void Probe()
{
    object c = CreateObject(OBJECT_TYPE_CREATURE, "mg_encoding", GetStartingLocation());
    string all = GetLocalString(c, "all");
    Log("MG_LEN " + IntToString(GetStringLength(all)));
    Log("MG_HEX " + Utf8(all));
    string at = "";
    at += IntToString(FindSubString(all, Char(287))) + " ";
    at += IntToString(FindSubString(all, Char(240))) + " ";
    at += IntToString(FindSubString(all, Char(305))) + " ";
    at += IntToString(FindSubString(all, Char(350))) + " ";
    at += IntToString(FindSubString(all, Char(8776))) + " ";
    at += IntToString(FindSubString(all, Char(233))) + " ";
    at += IntToString(FindSubString(all, Char(26085)));
    Log("MG_AT " + at);
    Log("MG_DONE");
}
"#;

/// Where the probe's `encoding.2da` lies, and the game's language.
#[derive(Default, Clone, Copy)]
struct Setup<'a> {
    /// In a hak of the module's.
    hak: Option<&'a str>,
    /// In the module file itself.
    module: Option<&'a str>,
    /// In the user directory's `override`.
    overridden: Option<&'a str>,
    /// The game's language (`settings.tml`'s `game.language.override`):
    /// English without.
    language: Option<&'a str>,
    /// The codepage `settings.tml` names (`game.language.codepage`).
    codepage: Option<&'a str>,
}

/// Runs the probe as `setup` has it.
fn probe(root: &std::path::Path, name: &str, setup: Setup<'_>) -> (String, Vec<String>) {
    let Setup { hak: in_hak, module: in_module, overridden, language, codepage } = setup;
    let gd = GameData::open(&GameInstall::new(root, None, "en")).unwrap();
    let data = gd.resman.get(&ResKey::parse("nw_chicken", ResType::UTC).unwrap()).unwrap();
    let mut utc = Gff::read(&data).unwrap();
    utc.root.set("TemplateResRef", Value::ResRef(b"mg_encoding".to_vec()));
    let mut var = Struct::new(0);
    var.set("Name", Value::String(b"all".to_vec()));
    var.set("Type", Value::Dword(3));
    var.set("Value", Value::String((0x20..=0xFF).collect()));
    utc.root.set("VarTable", Value::List(vec![var]));
    let mut extra =
        vec![(ResKey::parse("mg_encoding", ResType::UTC).unwrap(), utc.to_bytes().unwrap())];
    let dir = scratch_dir(name);
    let mut haks = Vec::new();
    let key = ResKey::parse("encoding", ResType::TWODA).unwrap();
    if let Some(t) = in_module {
        extra.push((key, t.as_bytes().to_vec()));
    }
    if let Some(t) = in_hak {
        let mut hak = mg_erf::ErfWriter::new(*b"HAK ");
        hak.add(key.resref, key.restype, t.as_bytes().to_vec()).unwrap();
        std::fs::create_dir_all(dir.join("hak")).unwrap();
        std::fs::write(dir.join("hak/mg_encoding.hak"), hak.to_bytes().unwrap()).unwrap();
        haks.push("mg_encoding");
    }
    if let Some(t) = overridden {
        std::fs::create_dir_all(dir.join("override")).unwrap();
        std::fs::write(dir.join("override/encoding.2da"), t).unwrap();
    }
    if language.is_some() || codepage.is_some() {
        let mut settings = String::from("[game.language]\n");
        for (key, value) in [("override", language), ("codepage", codepage)] {
            if let Some(value) = value {
                settings.push_str(&format!("{key} = \"{value}\"\n"));
            }
        }
        std::fs::write(dir.join("settings.tml"), settings).unwrap();
    }
    probe_module(root, &dir, name, PROBE, &haks, &extra);
    let run = run_server(root, &dir, name, "MG_DONE", Duration::from_secs(60)).unwrap();
    assert!(
        run.finished,
        "server did not finish; log tail:\n{}",
        &run.log[run.log.len().saturating_sub(3000)..]
    );
    let hex = run.values("MG_HEX").remove(0);
    let bytes: Vec<u8> = (0..hex.len() / 2)
        .map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap())
        .collect();
    let json = String::from_utf8(bytes).unwrap();
    (serde_json::from_str(&json).unwrap_or_else(|e| panic!("{json}: {e}")), run.values("MG_AT"))
}

/// The characters the probe asks the game's bytes of.
const ASKED: [char; 7] = ['ğ', 'ð', 'ı', 'Ş', '≈', 'é', '日'];

/// What Moonglow's codepage makes of the same: the text of the bytes, and
/// where in it each character asked for is (a "?" where it has no byte).
fn moonglow(codepage: Codepage) -> (String, Vec<String>) {
    let all: Vec<u8> = (0x20..=0xFF).collect();
    let at = ASKED.map(|c| {
        let byte = codepage.encode(&c.to_string()).map_or(b'?', |b| b[0]);
        (byte - 0x20).to_string()
    });
    (codepage.decode(&all).into_owned(), vec![at.join(" ")])
}

#[test]
fn the_game_reads_its_text_as_windows_1252_without_a_table() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    // (Bytes 0x81, 0x8D, 0x8F, 0x90 and 0x9D, which Windows-1252 leaves
    // out, are the control characters of their numbers, as here.)
    assert_eq!(probe(&root, "mg_enc_none", Setup::default()), moonglow(Codepage::WINDOWS_1252));
}

#[test]
fn the_game_reads_its_text_by_a_hak_s_encoding_table() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    // A letter of ASCII moved; hexadecimal with and without its prefix
    // ("305" is U+0305); a blank cell; a character past the Basic
    // Multilingual Plane; one character of two bytes; rows left out.
    let mut t = String::from("2DA V2.0\n\n     Codepoint\n");
    let own = Codepage::WINDOWS_1252.chars().unwrap();
    for b in 0..250usize {
        let cell = match b {
            126 => "0x2248".to_string(),
            240 => "0x11f".to_string(),
            241 => "305".to_string(),
            242 => "****".to_string(),
            243 => "0x1F600".to_string(),
            244 => "0X15E".to_string(),
            245 => "0x11f".to_string(),
            b => format!("0x{:x}", own[b] as u32),
        };
        t.push_str(&format!("{b} {cell}\n"));
    }
    let table = mg_2da::TwoDa::parse(t.as_bytes(), Codepage::WINDOWS_1252).unwrap();
    let codepage = mg_rules::encoding_table(&table, Codepage::WINDOWS_1252).unwrap();
    let (text, at) = moonglow(codepage);
    assert_eq!(text.chars().nth(0xF0 - 0x20), Some('ğ'));
    assert_eq!(
        probe(&root, "mg_enc_table", Setup { hak: Some(&t), ..Setup::default() }),
        (text, at)
    );
    // The same table in the module file alone is not read.
    assert_eq!(
        probe(&root, "mg_enc_in_mod", Setup { module: Some(&t), ..Setup::default() }),
        moonglow(Codepage::WINDOWS_1252)
    );
}

/// A table that moves byte 0xF0 to `ğ` and has no rows past it.
fn partial_table() -> String {
    let mut t = String::from("2DA V2.0\n\n     Codepoint\n");
    for b in 0..=0xF0usize {
        let cell = if b == 0xF0 { "0x11f".to_string() } else { format!("0x{b:x}") };
        t.push_str(&format!("{b} {cell}\n"));
    }
    t
}

#[test]
fn the_game_reads_an_encoding_table_from_override() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let t = partial_table();
    let table = mg_2da::TwoDa::parse(t.as_bytes(), Codepage::WINDOWS_1252).unwrap();
    let codepage = mg_rules::encoding_table(&table, Codepage::WINDOWS_1252).unwrap();
    let got = probe(&root, "mg_enc_override", Setup { overridden: Some(&t), ..Setup::default() });
    assert_eq!(got, moonglow(codepage));
}

#[test]
fn a_polish_game_keeps_its_own_letters_where_a_table_has_no_row() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let polish = Setup { language: Some("pl"), ..Setup::default() };
    // The language taken: without a table, its text is Windows-1250. (Of
    // the characters asked, the game writes `ð`, which Windows-1250
    // lacks, as 0xF0, its `đ`, where Moonglow writes a "?".)
    let none = probe(&root, "mg_enc_pl_none", polish);
    assert_eq!(none.0, moonglow(Codepage::WINDOWS_1250).0);
    assert_eq!(none.1, ["31 208 31 138 31 201 31"]);
    // A table's missing rows keep Windows-1250's letters, not
    // Windows-1252's: the table stands on the game's own codepage.
    let t = partial_table();
    let got = probe(&root, "mg_enc_pl_table", Setup { hak: Some(&t), ..polish });
    let table = mg_2da::TwoDa::parse(t.as_bytes(), Codepage::WINDOWS_1250).unwrap();
    let codepage = mg_rules::encoding_table(&table, Codepage::WINDOWS_1250).unwrap();
    assert_eq!(got, moonglow(codepage));
}

/// The codepage `settings.tml` names (`game.language.codepage`), which `mg
/// lsp` reads scripts in, is the game's text's in the server too: an
/// English game told Windows-1250 reads its text so.
#[test]
fn the_game_reads_its_text_in_the_codepage_its_settings_name() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let named = Setup { codepage: Some("cp1250"), ..Setup::default() };
    let got = probe(&root, "mg_enc_cp1250", named);
    assert_eq!(got.0, moonglow(Codepage::WINDOWS_1250).0);
}
