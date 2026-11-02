//! Engine checks of 2DA parsing questions, run in the game's headless server.

use std::process::Command;
use std::time::Duration;

use mg_core::{ResRef, ResType};
use mg_erf::{Erf, ErfWriter};
use mg_gff::{Gff, Struct, Value};
use mg_testkit::engine::{run_server, server_binary};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

const SCRIPT: &str = r#"
void main()
{
    WriteTimestampedLogEntry("MG_RESULT ADJECTIVE=[" + Get2DAString("random_hostile", "ADJECTIVE", 10) + "]");
    WriteTimestampedLogEntry("MG_RESULT GREETING=[" + Get2DAString("random_hostile", "GREETING", 10) + "]");
    WriteTimestampedLogEntry("MG_DONE");
}
"#;

/// `""` in a 2DA row is an empty cell in place (Moonglow's reading), not
/// skipped (nwn_twoda's): the engine returns "trivial" for ADJECTIVE and ""
/// for GREETING on row 10 of Beamdog's random_hostile.2da.
#[test]
fn engine_reads_quoted_empty_2da_cells_in_place() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let compiler = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_2da");
    std::fs::create_dir_all(dir.join("modules")).unwrap();

    // Compile the probe script.
    std::fs::write(dir.join("mg_probe.nss"), SCRIPT).unwrap();
    let out = Command::new(&compiler)
        .args([
            "-o",
            dir.join("mg_probe.ncs").to_str().unwrap(),
            dir.join("mg_probe.nss").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "compile failed: {}", String::from_utf8_lossy(&out.stderr));

    // A shipped module, with the probe as OnModuleLoad and the hak attached.
    let data = std::fs::read(root.join("data/mod/Neverwinter Chess.mod")).unwrap();
    let erf = Erf::read(&data).unwrap();
    let mut w = ErfWriter::new(*b"MOD ");
    for e in &erf.entries {
        let bytes = erf.data(e).unwrap().into_owned();
        let bytes = if e.restype == ResType::IFO {
            let mut ifo = Gff::read(&bytes).unwrap();
            ifo.root.set("Mod_OnModLoad", Value::ResRef(b"mg_probe".to_vec()));
            let mut hak = Struct::new(8);
            hak.set("Mod_Hak", Value::String(b"id_resources".to_vec()));
            ifo.root.set("Mod_HakList", Value::List(vec![hak]));
            ifo.to_bytes().unwrap()
        } else {
            bytes
        };
        w.add(e.resref, e.restype, bytes).unwrap();
    }
    let probe = std::fs::read(dir.join("mg_probe.ncs")).unwrap();
    w.add(ResRef::from_str("mg_probe").unwrap(), ResType::NCS, probe).unwrap();
    std::fs::write(dir.join("modules/mg_probe.mod"), w.to_bytes().unwrap()).unwrap();

    let run = run_server(&root, &dir, "mg_probe", "MG_DONE", Duration::from_secs(60)).unwrap();
    assert!(run.finished, "server did not finish; log:\n{}", run.log);
    let values = run.values("MG_RESULT");
    assert_eq!(values, ["ADJECTIVE=[trivial]", "GREETING=[]"], "log:\n{}", run.log);
}

/// A table as the 2DA editor writes it (cells set, a row added, a row
/// emptied, a cell of two words, a column added) is the table the game
/// reads, cell for cell.
#[test]
fn engine_reads_a_table_the_editor_wrote() {
    use mg_module::table_edit::Editing;
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_2da_edited");
    let start =
        b"2DA V2.0\r\n\r\n   Label   Cost\r\n0  Bread   1\r\n1  Cheese  2\r\n2  Wine    3\r\n";
    let mut e = Editing::new(mg_2da::TwoDa::parse(start, mg_core::Codepage::default()).unwrap());
    e.set(0, 1, Some("15".into())).unwrap();
    e.clear_row(1);
    let row = e.add_rows(1);
    e.set(row, 0, Some("Two Words".into())).unwrap();
    e.add_column("Note").unwrap();
    e.set(2, 2, Some("red".into())).unwrap();
    e.paste(row, 1, "7\tlast").unwrap();
    let t = e.table().clone();
    let bytes = t.to_bytes(mg_core::Codepage::default()).unwrap();
    // (And saved compact, one space between cells: the same cells.)
    let compact = t.to_bytes_as(mg_core::Codepage::default(), mg_2da::Layout::Compact).unwrap();
    let mut probes = String::new();
    for r in 0..t.len() + 1 {
        for c in t.columns() {
            probes += &format!(
                "    WriteTimestampedLogEntry(\"MG_CELL {r}.{c}=[\" + Get2DAString(\"mg_edited\", \"{c}\", {r}) + \"]\");\n"
            );
            probes += &format!(
                "    WriteTimestampedLogEntry(\"MG_COMPACT {r}.{c}=[\" + Get2DAString(\"mg_compact\", \"{c}\", {r}) + \"]\");\n"
            );
        }
    }
    let script =
        format!("void main()\n{{\n{probes}    WriteTimestampedLogEntry(\"MG_DONE\");\n}}\n");
    let key = mg_resman::ResKey::parse("mg_edited", ResType::TWODA).unwrap();
    let compact_key = mg_resman::ResKey::parse("mg_compact", ResType::TWODA).unwrap();
    let tables = [(key, bytes), (compact_key, compact)];
    crate::common::probe_module(&root, &dir, "mg_edited_p", &script, &[], &tables);
    let run = run_server(&root, &dir, "mg_edited_p", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished, "{}", &run.log[run.log.len().saturating_sub(2000)..]);
    let cells = run.values("MG_CELL");
    assert_eq!(cells.len(), (t.len() + 1) * 3);
    for r in 0..t.len() + 1 {
        for c in t.columns() {
            let ours = t.get(r, c).unwrap_or_default();
            let want = format!("{r}.{c}=[{ours}]");
            assert!(cells.contains(&want), "{want} not among {cells:?}");
        }
    }
    let mut compact = run.values("MG_COMPACT");
    let mut aligned = cells.clone();
    compact.sort();
    aligned.sort();
    assert_eq!(compact, aligned, "the compact table reads as the aligned one");
    assert!(cells.contains(&"3.Label=[Two Words]".to_string()));
    assert!(cells.contains(&"1.Label=[]".to_string()), "an emptied row is there, and empty");
}

/// A row of empty cells at a table's end, `****` or a label alone, is a
/// row to the game (`Get2DARowCount`), and the blank lines after it are
/// not: Moonglow reads it so, and writes it again.
#[test]
fn engine_counts_an_empty_last_row() {
    let root = corpus!();
    if server_binary(&root).is_none() {
        eprintln!("skipped: no nwserver for this platform");
        return;
    }
    let _ = oracle_tool!("nwn_script_comp");
    let dir = scratch_dir("engine_2da_last_row");
    let stars = b"2DA V2.0\r\n\r\n   Label   Cost\r\n0  Bread   1\r\n1  ****    ****\r\n\r\n";
    let label = b"2DA V2.0\r\n\r\n   Label   Cost\r\n0  Bread   1\r\n1\r\n\r\n\r\n";
    let script = "void main()\n{\n    WriteTimestampedLogEntry(\"MG_ROWS \" + IntToString(Get2DARowCount(\"mg_stars\")) + \" \" + IntToString(Get2DARowCount(\"mg_label\")));\n    WriteTimestampedLogEntry(\"MG_DONE\");\n}\n";
    let key = |n: &str| mg_resman::ResKey::parse(n, ResType::TWODA).unwrap();
    let tables = [(key("mg_stars"), stars.to_vec()), (key("mg_label"), label.to_vec())];
    crate::common::probe_module(&root, &dir, "mg_rows_p", script, &[], &tables);
    let run = run_server(&root, &dir, "mg_rows_p", "MG_DONE", Duration::from_secs(120)).unwrap();
    assert!(run.finished, "{}", &run.log[run.log.len().saturating_sub(2000)..]);
    assert_eq!(run.values("MG_ROWS"), ["2 2"], "the game's count");
    let cp = mg_core::Codepage::default();
    for data in [&stars[..], &label[..]] {
        let t = mg_2da::TwoDa::parse(data, cp).unwrap();
        assert_eq!(t.len(), 2, "Moonglow's count");
        assert_eq!(mg_2da::TwoDa::parse(&t.to_bytes(cp).unwrap(), cp).unwrap().len(), 2);
    }
}
