//! Talk tables as CSV, exactly as neverwinter.nim's `nwn_tlk -k csv`
//! writes and reads them (and so as a nasher project with `tlkFormat =
//! "csv"` keeps one): a line an entry, `id,"sound",soundLen,"text"`, no
//! header, `\n` after each.
//!
//! What `nwn_tlk` does, as measured (`tests/tlk.rs` in `mg-corpus-tests`
//! holds the game's tables to it):
//! - The entries written are those [`crate::json`] writes (a text or a
//!   sound), and their text, sound and length as it writes them: the text
//!   in `--nwn-encoding`'s codepage, always quoted, a `"` in it doubled
//!   and line ends kept inside the quotes; the sound quoted where there is
//!   one, else nothing; the length where it is above 0, else nothing.
//! - Read back, a table has no language of its own (CSV keeps none): it is
//!   English's, 0. The entries' flags are those of what they have, as
//!   [`crate::json::from_json`] gives them.

use mg_core::{Codepage, Language};

use crate::json::{self, JsonStyle};
use crate::{FLAG_SOUND, FLAG_SOUND_LENGTH, FLAG_TEXT, Tlk, TlkEntry};

/// The table as `nwn_tlk -k csv` writes it, its text read in `codepage`.
/// Not always UTF-8: a byte of the text the codepage has no character
/// for is written as it is.
pub fn to_csv(tlk: &Tlk, codepage: Codepage) -> Vec<u8> {
    let mut out = Vec::new();
    for (id, e) in tlk.entries.iter().enumerate() {
        let sound = json::sound_bytes(e);
        if e.text.is_empty() && sound.is_empty() {
            continue;
        }
        out.extend_from_slice(format!("{id},").as_bytes());
        if !sound.is_empty() {
            quoted(&mut out, sound);
        }
        out.push(b',');
        let length = json::rounded(e.sound_length);
        if length > 0.0 {
            out.extend_from_slice(json::nim_float(f64::from(length)).as_bytes());
        }
        out.push(b',');
        quoted(&mut out, &json::decode(&e.text, codepage));
        out.push(b'\n');
    }
    out
}

fn quoted(out: &mut Vec<u8>, bytes: &[u8]) {
    out.push(b'"');
    for &b in bytes {
        if b == b'"' {
            out.push(b'"');
        }
        out.push(b);
    }
    out.push(b'"');
}

/// A table from CSV as `nwn_tlk` reads it, its text written in
/// `codepage`. A file that isn't UTF-8 throughout (a byte `nwn_tlk` wrote
/// as it was) is read with that byte kept.
pub fn from_csv(text: &[u8], codepage: Codepage) -> Result<Tlk, String> {
    let text = json::with_raw_bytes(text);
    let mut found: Vec<(usize, TlkEntry)> = Vec::new();
    for (n, row) in rows(&text)?.into_iter().enumerate() {
        let at = format!("line {}", n + 1);
        if row.len() != 4 {
            return Err(format!("{at}: {} cells, where an entry has 4", row.len()));
        }
        let id = (row[0].trim().parse::<u64>().ok())
            .filter(|&id| id < json::MAX_ENTRIES)
            .ok_or_else(|| format!("{at}: {:?} is not a line's number", row[0]))?;
        let mut entry = TlkEntry { text: json::encode(&row[3], codepage), ..Default::default() };
        let mut sound = json::raw(&row[1]);
        sound.truncate(16);
        entry.sound = sound;
        if !row[2].trim().is_empty() {
            let length = (row[2].trim().parse::<f64>().ok())
                .filter(|l| *l >= 0.0)
                .ok_or_else(|| format!("{at}: {:?} is not a sound's length", row[2]))?;
            entry.sound_length = length as f32;
        }
        if !entry.text.is_empty() {
            entry.flags |= FLAG_TEXT;
        }
        if !entry.sound.is_empty() {
            entry.flags |= FLAG_SOUND | FLAG_SOUND_LENGTH;
        }
        found.push((id as usize, entry));
    }
    json::assemble(Language::ENGLISH, found)
}

/// The rows of CSV: cells separated by commas, a quoted cell's `""` a
/// quote, and line ends inside quotes kept.
fn rows(text: &str) -> Result<Vec<Vec<String>>, String> {
    let mut rows = Vec::new();
    let (mut row, mut cell) = (Vec::new(), String::new());
    let (mut quoted, mut was_quoted) = (false, false);
    let mut chars = text.chars().peekable();
    let mut line = 1;
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                chars.next();
                cell.push('"');
            }
            '"' if quoted => quoted = false,
            '"' if cell.is_empty() && !was_quoted => (quoted, was_quoted) = (true, true),
            '\n' if quoted => {
                line += 1;
                cell.push('\n');
            }
            ',' if !quoted => {
                row.push(std::mem::take(&mut cell));
                was_quoted = false;
            }
            '\r' if !quoted && chars.peek() == Some(&'\n') => {}
            '\n' => {
                line += 1;
                row.push(std::mem::take(&mut cell));
                was_quoted = false;
                rows.push(std::mem::take(&mut row));
            }
            c => cell.push(c),
        }
    }
    if quoted {
        return Err(format!("line {line}: a quoted cell is not closed"));
    }
    if !cell.is_empty() || !row.is_empty() {
        row.push(cell);
        rows.push(row);
    }
    Ok(rows)
}

/// How a project keeps its talk tables: as JSON or as CSV
/// (`tlkFormat`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Json,
    Csv,
}

/// The table in `format`, as `nwn_tlk` writes it.
pub fn write(tlk: &Tlk, format: Format, style: JsonStyle) -> Vec<u8> {
    match format {
        Format::Json => json::to_json(tlk, style),
        Format::Csv => to_csv(tlk, style.codepage),
    }
}

/// A table read from `format`, as `nwn_tlk` reads it.
pub fn read(text: &[u8], format: Format, codepage: Codepage) -> Result<Tlk, String> {
    match format {
        Format::Json => json::from_json(text, codepage),
        Format::Csv => from_csv(text, codepage),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What `nwn_tlk -k csv` wrote for these entries (measured with
    /// neverwinter.nim 2.x): the first table is the JSON
    /// `{"language":2,"entries":[{"id":0,"text":"Hallo, \"Welt\"\nzwei"},
    /// {"id":2,"text":"","sound":"vs_x"},{"id":3,"text":"a,b","soundLen":1.25},
    /// {"id":5,"text":"Señor"}]}` made a table.
    #[test]
    fn written_and_read_as_nwn_tlk_has_it() {
        let json = r#"{"language":2,"entries":[{"id":0,"text":"Hallo, \"Welt\"\nzwei"},{"id":2,"text":"","sound":"vs_x"},{"id":3,"text":"a,b","soundLen":1.25},{"id":5,"text":"Señor"}]}"#;
        let tlk = json::from_json(json.as_bytes(), Codepage::WINDOWS_1252).unwrap();
        let csv = to_csv(&tlk, Codepage::WINDOWS_1252);
        let theirs =
            "0,,,\"Hallo, \"\"Welt\"\"\nzwei\"\n2,\"vs_x\",,\"\"\n3,,1.25,\"a,b\"\n5,,,\"Señor\"\n";
        assert_eq!(String::from_utf8(csv.clone()).unwrap(), theirs);
        let back = from_csv(&csv, Codepage::WINDOWS_1252).unwrap();
        assert_eq!(back.language, Language::ENGLISH, "CSV keeps no language");
        assert_eq!(back.entries.len(), 6);
        for i in [0, 2, 3, 5] {
            assert_eq!(back.entries[i], tlk.entries[i], "entry {i}");
        }
        // A CR LF file, an unquoted cell, a stray byte kept.
        let t = from_csv(b"0,,,\"x\"\r\n1,,,y\n2,,,\"\xB3\"\n", Codepage::WINDOWS_1250).unwrap();
        assert_eq!(t.entries[1].text, b"y");
        assert_eq!(t.entries[2].text, b"\xB3");
        assert!(from_csv(b"0,,,\"open\n", Codepage::WINDOWS_1252).is_err());
        assert!(from_csv(b"x,,,\"a\"\n", Codepage::WINDOWS_1252).is_err());
        assert!(from_csv(b"0,,\"a\"\n", Codepage::WINDOWS_1252).unwrap_err().contains("3 cells"));
        assert!(from_csv(b"0,,-1,\"a\"\n", Codepage::WINDOWS_1252).is_err());
        assert_eq!(from_csv(b"", Codepage::WINDOWS_1252).unwrap().entries.len(), 1);
    }
}
