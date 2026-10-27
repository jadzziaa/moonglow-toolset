//! Talk tables as JSON, exactly as neverwinter.nim's `nwn_tlk` writes and
//! reads them (and so as nasher keeps a talk table in a repository):
//! `{"language":0,"entries":[{"id":0,"text":"...","sound":"...","soundLen":1.5}]}`,
//! compact unless `--pretty`, with no line end after it.
//!
//! What `nwn_tlk` does, as measured (`tests/tlk_json.rs` in
//! `mg-corpus-tests` holds every shipped table to it):
//! - An entry is written when its text or its sound is not empty, whatever
//!   its flags say; `text` always, `sound` when it has one (its bytes up to
//!   the last that isn't NUL), `soundLen` when the length, rounded to four
//!   places, is above 0. The length is the table's 32-bit float, printed
//!   as Nim prints the 64-bit one it widens to.
//! - The text is read in `--nwn-encoding`'s codepage (windows-1252 unless
//!   it says otherwise), whatever the table's language. A byte the codepage
//!   has no character for goes out as the byte itself; a character it can't
//!   write comes in as its UTF-8 bytes.
//! - Read back, an entry's flags are those of what it has: text 1, a
//!   sound 2 and 4 (with a length or not); a length alone sets none. The
//!   table has as many entries as the highest `id` asks for (one when it
//!   has none); an `id` given twice takes the later entry. (Moonglow
//!   refuses a table whose highest `id` would leave more than a million
//!   lines empty, which `nwn_tlk` makes.) `language`,
//!   `entries`, each entry's `id` and `text` must be there, a `soundLen`
//!   must be a number with a point, not below 0.

use mg_core::{Codepage, Language};

use crate::{FLAG_SOUND, FLAG_SOUND_LENGTH, FLAG_TEXT, Tlk, TlkEntry};

/// How a table is written as JSON: `nwn_tlk`'s `--nwn-encoding` and
/// `--pretty`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonStyle {
    pub codepage: Codepage,
    pub pretty: bool,
}

impl Default for JsonStyle {
    fn default() -> Self {
        JsonStyle { codepage: Codepage::WINDOWS_1252, pretty: false }
    }
}

/// The table as `nwn_tlk` writes it. Not always UTF-8: a byte of the text
/// that the codepage has no character for is written as it is, as
/// `nwn_tlk` writes it.
pub fn to_json(tlk: &Tlk, style: JsonStyle) -> Vec<u8> {
    let mut out = Vec::new();
    let (nl, ind1, ind2, ind3, colon) =
        if style.pretty { ("\n", "  ", "    ", "      ", ": ") } else { ("", "", "", "", ":") };
    let entries: Vec<(usize, &TlkEntry)> = (tlk.entries.iter().enumerate())
        .filter(|(_, e)| !e.text.is_empty() || !sound_bytes(e).is_empty())
        .collect();
    out.extend_from_slice(format!("{{{nl}{ind1}\"language\"{colon}{}", tlk.language.0).as_bytes());
    out.extend_from_slice(format!(",{nl}{ind1}\"entries\"{colon}[").as_bytes());
    for (n, (id, e)) in entries.iter().enumerate() {
        if n > 0 {
            out.push(b',');
        }
        out.extend_from_slice(format!("{nl}{ind2}{{{nl}{ind3}\"id\"{colon}{id}").as_bytes());
        out.extend_from_slice(format!(",{nl}{ind3}\"text\"{colon}").as_bytes());
        push_string(&mut out, &decode(&e.text, style.codepage));
        let sound = sound_bytes(e);
        if !sound.is_empty() {
            out.extend_from_slice(format!(",{nl}{ind3}\"sound\"{colon}").as_bytes());
            push_string(&mut out, sound);
        }
        let length = rounded(e.sound_length);
        if length > 0.0 {
            out.extend_from_slice(format!(",{nl}{ind3}\"soundLen\"{colon}").as_bytes());
            out.extend_from_slice(nim_float(f64::from(length)).as_bytes());
        }
        out.extend_from_slice(format!("{nl}{ind2}}}").as_bytes());
    }
    if !entries.is_empty() {
        out.extend_from_slice(format!("{nl}{ind1}").as_bytes());
    }
    out.extend_from_slice(format!("]{nl}}}").as_bytes());
    out
}

/// A table from JSON as `nwn_tlk` reads it, its text written in
/// `codepage`. A file that isn't UTF-8 throughout (a byte `nwn_tlk` wrote
/// as it was) is read with that byte kept.
pub fn from_json(text: &[u8], codepage: Codepage) -> Result<Tlk, String> {
    let json: serde_json::Value = match std::str::from_utf8(text) {
        Ok(s) => serde_json::from_str(s),
        Err(_) => serde_json::from_str(&with_raw_bytes(text)),
    }
    .map_err(|e| e.to_string())?;
    let language = json
        .get("language")
        .and_then(serde_json::Value::as_u64)
        .and_then(|l| u32::try_from(l).ok())
        .ok_or("\"language\" missing or not a number")?;
    let list = json
        .get("entries")
        .and_then(serde_json::Value::as_array)
        .ok_or("\"entries\" missing or not a list")?;
    let mut found: Vec<(usize, TlkEntry)> = Vec::with_capacity(list.len());
    for (n, e) in list.iter().enumerate() {
        let at = format!("entry {}", n + 1);
        let id = e
            .get("id")
            .and_then(serde_json::Value::as_u64)
            .filter(|&id| id < MAX_ENTRIES)
            .ok_or_else(|| format!("{at}: its \"id\" is missing or not a line's number"))?;
        let text = e
            .get("text")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("{at}: its \"text\" is missing or not text"))?;
        let mut entry = TlkEntry { text: encode(text, codepage), ..Default::default() };
        if let Some(sound) = e.get("sound").and_then(serde_json::Value::as_str) {
            let mut bytes = raw(sound);
            bytes.truncate(16);
            entry.sound = bytes;
        }
        if let Some(length) = e.get("soundLen") {
            let length = length
                .as_f64()
                .filter(|_| length.is_f64())
                .filter(|l| *l >= 0.0)
                .ok_or_else(|| format!("{at}: its \"soundLen\" is not a length (as 1.0)"))?;
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
    let count = found.iter().map(|(id, _)| id + 1).max().unwrap_or(1);
    if count.saturating_sub(found.len()) > MAX_EMPTY_LINES {
        return Err(format!(
            "its highest \"id\", {}, leaves more than {MAX_EMPTY_LINES} lines of the table \
             empty (a table of {count} lines for {} entries)",
            count - 1,
            found.len()
        ));
    }
    let mut tlk = Tlk { language: Language(language), entries: vec![TlkEntry::default(); count] };
    for (id, e) in found {
        tlk.entries[id] = e;
    }
    Ok(tlk)
}

/// An entry's sound: its bytes up to the last that isn't NUL.
fn sound_bytes(e: &TlkEntry) -> &[u8] {
    let end = e.sound.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    &e.sound[..end]
}

/// A sound's length as `nwn_tlk` writes it: rounded to four places.
fn rounded(length: f32) -> f32 {
    ((f64::from(length) * 10_000.0).round() / 10_000.0) as f32
}

/// Text bytes as `nwn_tlk` writes them: UTF-8, but for a byte the
/// codepage has no character for (one it reads as a C1 control), which
/// stays as it is.
fn decode(bytes: &[u8], codepage: Codepage) -> Vec<u8> {
    let Some(chars) = codepage.chars() else {
        return codepage.decode(bytes).as_bytes().to_vec();
    };
    let mut out = Vec::with_capacity(bytes.len());
    let mut buf = [0u8; 4];
    for &b in bytes {
        let c = chars[b as usize];
        if b >= 0x80 && ('\u{80}'..='\u{9f}').contains(&c) {
            out.push(b);
        } else {
            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
        }
    }
    out
}

/// The most lines a table read holds: as many as a custom table's
/// StrRefs can name.
const MAX_ENTRIES: u64 = 1 << 24;

/// The most empty lines a table read may have beyond those it lists: one
/// entry with `id` 16777215 would otherwise make a table of 16 million
/// lines (a gigabyte) from a few bytes. A table listed densely may still
/// have up to [`MAX_ENTRIES`] lines.
const MAX_EMPTY_LINES: usize = 1 << 20;

/// Where a byte that isn't part of UTF-8 stands in text read from JSON
/// (`with_raw_bytes`): a private-use character of its own, read back as
/// the byte.
const RAW: u32 = 0x10_FF00;

/// The text of a file that isn't UTF-8 throughout, each stray byte as the
/// character that stands for it.
fn with_raw_bytes(text: &[u8]) -> String {
    let mut out = String::with_capacity(text.len());
    for chunk in text.utf8_chunks() {
        out.push_str(chunk.valid());
        out.extend(chunk.invalid().iter().filter_map(|&b| char::from_u32(RAW + u32::from(b))));
    }
    out
}

/// A string's bytes, a stray byte's character as the byte.
fn raw(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len());
    let mut buf = [0u8; 4];
    for c in s.chars() {
        match u32::from(c).checked_sub(RAW) {
            Some(b) => out.push(b as u8),
            None => out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes()),
        }
    }
    out
}

/// Text as `nwn_tlk` writes it in `codepage`: a character at a time, one
/// the codepage can't write (C1 controls among them) as its UTF-8 bytes.
fn encode(s: &str, codepage: Codepage) -> Vec<u8> {
    if s.is_ascii() {
        return s.as_bytes().to_vec();
    }
    let mut out = Vec::with_capacity(s.len());
    let mut buf = [0u8; 4];
    for c in s.chars() {
        let one = c.encode_utf8(&mut buf);
        if let Some(b) = u32::from(c).checked_sub(RAW) {
            out.push(b as u8);
        } else if ('\u{80}'..='\u{9f}').contains(&c) {
            out.extend_from_slice(one.as_bytes());
        } else {
            match codepage.encode(one) {
                Some(bytes) => out.extend_from_slice(&bytes),
                None => out.extend_from_slice(one.as_bytes()),
            }
        }
    }
    out
}

/// A JSON string of bytes (UTF-8 but for stray bytes, which go as they
/// are), escaped as Nim's `escapeJson` escapes it.
fn push_string(out: &mut Vec<u8>, bytes: &[u8]) {
    out.push(b'"');
    for &b in bytes {
        match b {
            b'\n' => out.extend_from_slice(b"\\n"),
            0x08 => out.extend_from_slice(b"\\b"),
            0x0c => out.extend_from_slice(b"\\f"),
            b'\t' => out.extend_from_slice(b"\\t"),
            0x0b => out.extend_from_slice(b"\\u000b"),
            b'\r' => out.extend_from_slice(b"\\r"),
            b'"' => out.extend_from_slice(b"\\\""),
            b'\\' => out.extend_from_slice(b"\\\\"),
            0..=0x07 => out.extend_from_slice(format!("\\u000{b}").as_bytes()),
            0x0e..=0x1f => out.extend_from_slice(format!("\\u00{b:02X}").as_bytes()),
            _ => out.push(b),
        }
    }
    out.push(b'"');
}

/// A float as Nim's `$` writes it: the shortest digits that read back as
/// it, with the point among them while it is at most 17 digits in (else
/// as `d.ddde+NN`), and `.0` after a whole number.
fn nim_float(v: f64) -> String {
    if v == 0.0 {
        return "0.0".into();
    }
    // (Rust's `{:e}` gives as few digits, `1.5e0`, `9.99e-5`; where two
    // are as near, as 2.4235000610351562 and ...563 to 2.42350006103515625,
    // it takes the higher and Nim the even one, as rounding to that many
    // digits does.)
    let shortest = format!("{:e}", v.abs());
    let places = shortest.split_once('e').map_or(0, |(m, _)| m.len().saturating_sub(2));
    let sci = format!("{:.places$e}", v.abs());
    let (mantissa, exp) = sci.split_once('e').expect("{:e} has an e");
    let exp: i32 = exp.parse().expect("an exponent");
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let n = digits.len() as i32;
    let point = exp + 1;
    let mut out = String::from(if v < 0.0 { "-" } else { "" });
    if (-5..=17).contains(&point) {
        if point <= 0 {
            out.push_str("0.");
            out.extend(std::iter::repeat_n('0', (-point) as usize));
            out.push_str(&digits);
        } else if point < n {
            out.push_str(&digits[..point as usize]);
            out.push('.');
            out.push_str(&digits[point as usize..]);
        } else {
            out.push_str(&digits);
            out.extend(std::iter::repeat_n('0', (point - n) as usize));
            out.push_str(".0");
        }
    } else {
        out.push_str(&digits[..1]);
        if n > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        let sign = if exp < 0 { '-' } else { '+' };
        out.push_str(&format!("e{sign}{:02}", exp.abs()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(flags: u32, text: &[u8], sound: &[u8], length: f32) -> TlkEntry {
        TlkEntry {
            flags,
            text: text.into(),
            sound: sound.into(),
            sound_length: length,
            ..Default::default()
        }
    }

    /// What `nwn_tlk -k json` wrote for these entries (measured).
    #[test]
    fn written_as_nwn_tlk_writes_it() {
        let mut t = Tlk::new(Language::ENGLISH);
        t.entries = vec![
            entry(0, b"text without flag", b"", 0.0),
            entry(1, b"", b"", 0.0),
            entry(0, b"", b"snd_noflag", 0.0),
            entry(1, b"len", b"", 1.1),
            entry(1, b"ctl \x01\x1f\x7f end / \\ \xe9 \x80 \x81", b"", 0.0),
            entry(7, b"floats", b"a", 1e-5),
            entry(7, b"floats", b"a", 123456789.0),
            entry(7, b"floats", b"a", 3.0e20),
            entry(7, b"floats", b"a", 0.1),
            entry(0, b"", b"", 5.0),
            entry(0, b"", b"", 0.0),
        ];
        let json = to_json(&t, JsonStyle::default());
        let expected: &[u8] =
            b"{\"language\":0,\"entries\":[{\"id\":0,\"text\":\"text without flag\"},\
            {\"id\":2,\"text\":\"\",\"sound\":\"snd_noflag\"},\
            {\"id\":3,\"text\":\"len\",\"soundLen\":1.100000023841858},\
            {\"id\":4,\"text\":\"ctl \\u0001\\u001F\x7f end / \\\\ \xc3\xa9 \xe2\x82\xac \x81\"},\
            {\"id\":5,\"text\":\"floats\",\"sound\":\"a\"},\
            {\"id\":6,\"text\":\"floats\",\"sound\":\"a\",\"soundLen\":123456792.0},\
            {\"id\":7,\"text\":\"floats\",\"sound\":\"a\",\"soundLen\":3.000000060122632e+20},\
            {\"id\":8,\"text\":\"floats\",\"sound\":\"a\",\"soundLen\":0.10000000149011612}]}";
        assert_eq!(String::from_utf8_lossy(&json), String::from_utf8_lossy(expected));
        assert_eq!(json, expected);
        // Read back: the bytes it had, the flags of what it has.
        let back = from_json(&json, Codepage::WINDOWS_1252).unwrap();
        assert_eq!(back.entries.len(), 9);
        assert_eq!(back.entries[4].text, t.entries[4].text);
        assert_eq!(back.entries[0].flags, FLAG_TEXT);
        assert_eq!(back.entries[2].flags, FLAG_SOUND | FLAG_SOUND_LENGTH);
        assert_eq!((back.entries[3].flags, back.entries[3].sound_length), (FLAG_TEXT, 1.1));
        assert_eq!(to_json(&back, JsonStyle::default()), json, "the same again");
    }

    #[test]
    fn escapes_and_lengths_as_nim_writes_them() {
        let mut t = Tlk::new(Language::POLISH);
        t.entries = (0u8..0x20).map(|b| entry(1, &[b, b'|'], b"", 0.0)).collect();
        let json = String::from_utf8(to_json(&t, JsonStyle::default())).unwrap();
        for (b, is) in [
            (0, "\\u0000|"),
            (7, "\\u0007|"),
            (8, "\\b|"),
            (9, "\\t|"),
            (10, "\\n|"),
            (11, "\\u000b|"),
            (12, "\\f|"),
            (13, "\\r|"),
            (14, "\\u000E|"),
            (26, "\\u001A|"),
        ] {
            assert!(json.contains(&format!("\"id\":{b},\"text\":\"{is}\"")), "{b}: {json}");
        }
        // Lengths rounded to four places (in 64 bits), then a 32-bit float.
        let cases: [(f32, Option<&str>); 12] = [
            (9e-5, Some("0.00009999999747378752")),
            (1.1e-5, None),
            (5e-5, None),
            (-1.0, None),
            (f32::NAN, None),
            (0.00015, Some("0.00019999999494757503")),
            (2.00015, Some("2.0000998973846436")),
            (1.00005, Some("1.0")),
            (7.0, Some("7.0")),
            (1e17, Some("99999998430674940.0")),
            (1.2345e17, Some("1.2344999588082483e+17")),
            (3e38, Some("3.0000000054977558e+38")),
        ];
        for (length, written) in cases {
            let mut t = Tlk::new(Language::ENGLISH);
            t.entries = vec![entry(7, b"f", b"a", length)];
            let json = String::from_utf8(to_json(&t, JsonStyle::default())).unwrap();
            let got = json.split_once("\"soundLen\":").map(|(_, r)| r.trim_end_matches("}]}"));
            assert_eq!(got, written, "{length}");
        }
    }

    /// One entry with the highest `id` would make a table of 16 million
    /// lines: refused past a million empty lines, with the `id` named.
    #[test]
    fn a_far_id_alone_is_refused() {
        let json = |id: usize| format!(r#"{{"language":0,"entries":[{{"id":{id},"text":"x"}}]}}"#);
        let far = from_json(json(16_777_215).as_bytes(), Codepage::WINDOWS_1252).unwrap_err();
        assert!(far.contains("16777215"), "{far}");
        let t = from_json(json(MAX_EMPTY_LINES).as_bytes(), Codepage::WINDOWS_1252).unwrap();
        assert_eq!(t.entries.len(), MAX_EMPTY_LINES + 1);
        assert!(from_json(json(MAX_EMPTY_LINES + 1).as_bytes(), Codepage::WINDOWS_1252).is_err());
    }

    #[test]
    fn pretty_as_nwn_tlk_writes_it() {
        let mut t = Tlk::new(Language::ENGLISH);
        t.entries = vec![entry(1, b"A", b"", 0.0), entry(7, b"B", b"vo", 1.5)];
        let style = JsonStyle { pretty: true, ..Default::default() };
        let json = String::from_utf8(to_json(&t, style)).unwrap();
        assert_eq!(
            json,
            "{\n  \"language\": 0,\n  \"entries\": [\n    {\n      \"id\": 0,\n      \"text\": \
             \"A\"\n    },\n    {\n      \"id\": 1,\n      \"text\": \"B\",\n      \"sound\": \
             \"vo\",\n      \"soundLen\": 1.5\n    }\n  ]\n}"
        );
        let empty = Tlk::new(Language::ENGLISH);
        let json = String::from_utf8(to_json(&empty, style)).unwrap();
        assert_eq!(json, "{\n  \"language\": 0,\n  \"entries\": []\n}");
        assert_eq!(to_json(&empty, JsonStyle::default()), b"{\"language\":0,\"entries\":[]}");
    }

    /// What `nwn_tlk` made of these (measured with `-k debug`).
    #[test]
    fn read_as_nwn_tlk_reads_it() {
        let text = r#"{"language":4,"entries":[{"id":3,"text":""},{"id":1,"text":"a\u0001b\r\n","soundLen":0.5},
            {"id":6,"text":"","sound":"s"},{"id":1,"text":"dup"},{"id":0,"text":"x","sound":"abcdefghijklmnopqrs"},
            {"id":2,"text":"€Ā\u0081","extra":1}]}"#;
        let t = from_json(text.as_bytes(), Codepage::WINDOWS_1252).unwrap();
        assert_eq!((t.language, t.entries.len()), (Language::SPANISH, 7));
        assert_eq!((t.entries[1].text.as_slice(), t.entries[1].sound_length), (&b"dup"[..], 0.0));
        assert_eq!(t.entries[3].flags, 0);
        assert_eq!(t.entries[5], TlkEntry::default());
        assert_eq!(t.entries[6].sound, b"s");
        assert_eq!(t.entries[6].flags, FLAG_SOUND | FLAG_SOUND_LENGTH);
        assert_eq!(t.entries[0].sound, b"abcdefghijklmnop");
        // € written, Ā and the C1 control as their UTF-8.
        assert_eq!(t.entries[2].text, b"\x80\xc4\x80\xc2\x81");
        // Polish text in windows-1252 (nwn_tlk's default): as UTF-8.
        let t = from_json(
            "{\"language\":5,\"entries\":[{\"id\":0,\"text\":\"ł\"}]}".as_bytes(),
            Codepage::WINDOWS_1252,
        )
        .unwrap();
        assert_eq!(t.entries[0].text, "ł".as_bytes());
        let t = from_json(
            "{\"language\":5,\"entries\":[{\"id\":0,\"text\":\"ł\"}]}".as_bytes(),
            Codepage::WINDOWS_1250,
        )
        .unwrap();
        assert_eq!(t.entries[0].text, b"\xb3");
        // A stray byte (one nwn_tlk wrote as it was) is kept.
        let t = from_json(
            b"{\"language\":0,\"entries\":[{\"id\":0,\"text\":\"\x81z\xe9\"}]}",
            Codepage::WINDOWS_1252,
        )
        .unwrap();
        assert_eq!(t.entries[0].text, b"\x81z\xe9");
        // No entries: one, empty.
        let t = from_json(b"{\"language\":0,\"entries\":[]}", Codepage::WINDOWS_1252).unwrap();
        assert_eq!(t.entries, [TlkEntry::default()]);
        // What nwn_tlk refuses.
        for bad in [
            r#"{"entries":[]}"#,
            r#"{"language":0}"#,
            r#"{"language":0,"entries":[{"text":"x"}]}"#,
            r#"{"language":0,"entries":[{"id":0,"sound":"s"}]}"#,
            r#"{"language":0,"entries":[{"id":0,"text":"x","soundLen":1}]}"#,
            r#"{"language":0,"entries":[{"id":0,"text":"x","soundLen":-2.5}]}"#,
            r#"{"language":0,"entries":[{"id":99999999,"text":"x"}]}"#,
            "{",
        ] {
            assert!(from_json(bad.as_bytes(), Codepage::WINDOWS_1252).is_err(), "{bad}");
        }
    }

    /// The file nwn_tlk made of this JSON, field for field (`-k debug`):
    /// the entry with a sound and no text points at the text's end.
    #[test]
    fn compiled_as_nwn_tlk_compiles_it() {
        let text = r#"{"language": 0, "entries": [{"id": 0, "text": "Fenwick's Ferry"},
            {"id": 2, "text": "A line\nwith \"quotes\" and é / slash\ttab", "sound": "vo_line", "soundLen": 1.5},
            {"id": 5, "text": "", "sound":"x"},{"id": 7, "text": "len only", "soundLen": 2.0}]}"#;
        let t = from_json(text.as_bytes(), Codepage::WINDOWS_1252).unwrap();
        let bytes = t.to_bytes_as_nwn_tlk().unwrap();
        let field = |entry: usize, at: usize| {
            let o = 20 + 40 * entry + at;
            u32::from_le_bytes(bytes[o..o + 4].try_into().unwrap())
        };
        // (Flags, then the offset and size of the text.)
        let expected = [
            (1, 0, 15),
            (0, 0, 0),
            (7, 15, 38),
            (0, 0, 0),
            (0, 0, 0),
            (6, 53, 0),
            (0, 0, 0),
            (1, 53, 8),
        ];
        for (n, (flags, offset, size)) in expected.into_iter().enumerate() {
            assert_eq!(
                (field(n, 0), field(n, 28), field(n, 32)),
                (flags, offset, size),
                "entry {n}"
            );
        }
        assert_eq!(f32::from_bits(field(7, 36)), 2.0, "a length without a sound is kept");
        // (Moonglow's own writing points such an entry at 0.)
        let ours = t.to_bytes().unwrap();
        assert_eq!(&ours[20 + 40 * 5 + 28..20 + 40 * 5 + 32], &[0, 0, 0, 0]);
    }

    #[test]
    fn floats_as_nim_writes_them() {
        for (v, s) in [
            (1.5, "1.5"),
            (2.0, "2.0"),
            (10000000.0, "10000000.0"),
            (0.0001, "0.0001"),
            (0.30000001192092896, "0.30000001192092896"),
            (1.0000000150474662e30, "1.0000000150474662e+30"),
            (123.45600128173828, "123.45600128173828"),
            // (Halfway between two of as many digits: the even one, as
            // nwn_tlk writes a length of dialog.tlk's.)
            (f64::from(2.4235f32), "2.4235000610351562"),
        ] {
            assert_eq!(nim_float(v), s);
        }
    }
}
