//! Editing a tileset file in place: the file is kept as its lines, and an
//! edit changes only the lines it is about (a value where it stands, a key
//! added at the end of its section, a section added after the last of its
//! kind), so comments, order, spacing, case and line ends stay as they
//! were, and an untouched file is written back byte for byte.
//!
//! [`SetFile`] edits keys and sections; the functions below it add the
//! tileset's things (terrains, crossers, tiles, groups) keeping the counts
//! in step.

use crate::ini::Ini;

/// A tileset file as lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetFile {
    lines: Vec<String>,
    /// The line end it uses (`\r\n` or `\n`).
    newline: &'static str,
    /// Whether the last line ends with one.
    trailing: bool,
}

fn header(line: &str) -> Option<&str> {
    let t = line.trim();
    t.strip_prefix('[').and_then(|l| l.strip_suffix(']')).map(str::trim)
}

fn key_of(line: &str) -> Option<&str> {
    let t = line.trim_start();
    if t.starts_with(';') || t.starts_with('#') {
        return None;
    }
    line.split_once('=').map(|(k, _)| k.trim())
}

/// A section's family: its name without its number (`TILE12` and
/// `TILE12DOOR0` are `TILE` and `TILEDOOR`), for placing new sections.
fn family(name: &str) -> String {
    name.chars().filter(|c| !c.is_ascii_digit()).collect::<String>().to_ascii_uppercase()
}

impl SetFile {
    pub fn parse(text: &str) -> SetFile {
        let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
        let trailing = text.ends_with('\n');
        let lines = text.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l).to_string());
        let mut lines: Vec<String> = lines.collect();
        if trailing {
            lines.pop();
        }
        SetFile { lines, newline, trailing }
    }

    /// An empty file using `\r\n`, as the game's are.
    pub fn new() -> SetFile {
        SetFile { lines: Vec::new(), newline: "\r\n", trailing: true }
    }

    /// Whether it has no lines.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn text(&self) -> String {
        let mut s = self.lines.join(self.newline);
        if self.trailing {
            s.push_str(self.newline);
        }
        s
    }

    /// The file as the reader sees it.
    pub fn ini(&self) -> Ini {
        Ini::parse(&self.text())
    }

    /// The line range of a section's body (after its header), if it is
    /// there.
    fn body(&self, section: &str) -> Option<(usize, usize)> {
        let start = self
            .lines
            .iter()
            .position(|l| header(l).is_some_and(|h| h.eq_ignore_ascii_case(section)))?;
        let end = self.lines[start + 1..]
            .iter()
            .position(|l| header(l).is_some())
            .map_or(self.lines.len(), |i| start + 1 + i);
        Some((start + 1, end))
    }

    pub fn has_section(&self, section: &str) -> bool {
        self.body(section).is_some()
    }

    /// A key's value in a section.
    pub fn get(&self, section: &str, key: &str) -> Option<&str> {
        let (a, b) = self.body(section)?;
        self.lines[a..b].iter().find_map(|l| {
            (key_of(l)?.eq_ignore_ascii_case(key))
                .then(|| l.split_once('=').map_or("", |(_, v)| v.trim()))
        })
    }

    /// A section's keys and values, in order.
    pub fn entries(&self, section: &str) -> Vec<(String, String)> {
        let Some((a, b)) = self.body(section) else { return Vec::new() };
        self.lines[a..b]
            .iter()
            .filter_map(|l| {
                let k = key_of(l)?;
                Some((k.to_string(), l.split_once('=').map_or("", |(_, v)| v.trim()).to_string()))
            })
            .collect()
    }

    /// Where a new line goes at the end of a section: after its last
    /// non-blank line.
    fn end_of(&self, a: usize, b: usize) -> usize {
        (a..b).rev().find(|&i| !self.lines[i].trim().is_empty()).map_or(a, |i| i + 1)
    }

    /// Sets a key's value: where it stands, else at the end of the section
    /// (the section is added if missing).
    pub fn set(&mut self, section: &str, key: &str, value: &str) {
        if !self.has_section(section) {
            self.add_section(section, &[]);
        }
        let (a, b) = self.body(section).expect("added");
        for i in a..b {
            if key_of(&self.lines[i]).is_some_and(|k| k.eq_ignore_ascii_case(key)) {
                let written =
                    self.lines[i].split_once('=').map(|(k, _)| k).unwrap_or(key).to_string();
                self.lines[i] = format!("{written}={value}");
                return;
            }
        }
        let at = self.end_of(a, b);
        self.lines.insert(at, format!("{key}={value}"));
    }

    /// Takes a key out of a section.
    pub fn remove_key(&mut self, section: &str, key: &str) {
        if let Some((a, b)) = self.body(section)
            && let Some(i) = (a..b)
                .find(|&i| key_of(&self.lines[i]).is_some_and(|k| k.eq_ignore_ascii_case(key)))
        {
            self.lines.remove(i);
        }
    }

    /// Adds a section after the last of its family (`[TILE12]` after the
    /// last `[TILEn]`), else at the end, with a blank line before it.
    pub fn add_section(&mut self, name: &str, entries: &[(String, String)]) {
        let fam = family(name);
        let last = self
            .lines
            .iter()
            .enumerate()
            .filter(|(_, l)| header(l).is_some_and(|h| family(h) == fam))
            .map(|(i, l)| (i, header(l).unwrap_or_default().to_string()))
            .next_back();
        let at = match last {
            Some((_, h)) => {
                let (a, b) = self.body(&h).expect("found");
                self.end_of(a, b)
            }
            None => self.end_of(0, self.lines.len()),
        };
        let mut new = if at == 0 { Vec::new() } else { vec![String::new()] };
        new.push(format!("[{name}]"));
        new.extend(entries.iter().map(|(k, v)| format!("{k}={v}")));
        self.lines.splice(at..at, new);
    }

    /// Takes a section out, with the blank lines before it.
    pub fn remove_section(&mut self, name: &str) {
        let Some((a, b)) = self.body(name) else { return };
        let mut start = a - 1;
        while start > 0 && self.lines[start - 1].trim().is_empty() {
            start -= 1;
        }
        // Keep one blank line between the sections around it.
        let keep_blank = start < a - 1 && b < self.lines.len();
        self.lines.drain(start..b);
        if keep_blank {
            self.lines.insert(start, String::new());
        }
    }

    fn count(&self, section: &str) -> usize {
        self.get(section, "Count").and_then(|v| v.trim().parse().ok()).unwrap_or(0)
    }
}

impl Default for SetFile {
    fn default() -> Self {
        SetFile::new()
    }
}

fn pairs(entries: &[(&str, String)]) -> Vec<(String, String)> {
    entries.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
}

/// Adds a terrain (`crosser`: a crosser) with a name and a talk-table
/// string; returns its index.
pub fn add_type(f: &mut SetFile, crosser: bool, name: &str, strref: Option<u32>) -> usize {
    let (list, each) =
        if crosser { ("CROSSER TYPES", "CROSSER") } else { ("TERRAIN TYPES", "TERRAIN") };
    let n = f.count(list);
    f.set(list, "Count", &(n + 1).to_string());
    let mut entries = vec![("Name", name.to_string())];
    if let Some(s) = strref {
        entries.push(("StrRef", s.to_string()));
    }
    f.add_section(&format!("{each}{n}"), &pairs(&entries));
    n
}

/// Adds a tile copied from tile `from` (with its door sections), the last
/// one; returns its index. Tiles are numbered by place: areas store the
/// numbers, so tiles are only added at the end.
pub fn duplicate_tile(f: &mut SetFile, from: usize) -> Option<usize> {
    let n = f.count("TILES");
    let entries = f.entries(&format!("TILE{from}"));
    if entries.is_empty() {
        return None;
    }
    f.set("TILES", "Count", &(n + 1).to_string());
    f.add_section(&format!("TILE{n}"), &entries);
    let doors =
        f.get(&format!("TILE{from}"), "Doors").and_then(|d| d.parse::<usize>().ok()).unwrap_or(0);
    for d in 0..doors {
        let door = f.entries(&format!("TILE{from}DOOR{d}"));
        if !door.is_empty() {
            f.add_section(&format!("TILE{n}DOOR{d}"), &door);
        }
    }
    Some(n)
}

/// Adds a tile at the end with a model and the same terrain at each
/// corner; returns its index.
pub fn add_tile(f: &mut SetFile, model: &str, terrain: &str) -> usize {
    let n = f.count("TILES");
    f.set("TILES", "Count", &(n + 1).to_string());
    let mut entries = vec![
        ("Model".to_string(), model.to_string()),
        ("WalkMesh".to_string(), "msb01".to_string()),
    ];
    for corner in crate::CORNERS {
        entries.push((corner.to_string(), terrain.to_string()));
        entries.push((format!("{corner}Height"), "0".to_string()));
    }
    for edge in crate::EDGES {
        entries.push((edge.to_string(), String::new()));
    }
    for key in [
        "MainLight1",
        "MainLight2",
        "SourceLight1",
        "SourceLight2",
        "AnimLoop1",
        "AnimLoop2",
        "AnimLoop3",
        "Doors",
        "Sounds",
    ] {
        entries.push((key.to_string(), "0".to_string()));
    }
    entries.push(("PathNode".to_string(), "A".to_string()));
    entries.push(("Orientation".to_string(), "0".to_string()));
    entries.push(("ImageMap2D".to_string(), format!("mi_{model}")));
    f.add_section(&format!("TILE{n}"), &entries);
    n
}

/// A new tileset's file: its general settings, and no terrains, tiles or
/// groups yet.
pub fn skeleton(name: &str) -> SetFile {
    let mut f = SetFile::new();
    for (k, v) in [
        ("Name", name.to_uppercase()),
        ("Type", "SET".to_string()),
        ("Version", "V1.0".to_string()),
        ("Interior", "0".to_string()),
        ("HasHeightTransition", "0".to_string()),
        ("EnvMap", String::new()),
        ("Transition", "5".to_string()),
        ("DisplayName", "-1".to_string()),
        ("UnlocalizedName", name.to_string()),
        ("Border", String::new()),
        ("Default", String::new()),
        ("Floor", String::new()),
    ] {
        f.set("GENERAL", k, &v);
    }
    f.set("GRASS", "Grass", "0");
    for list in
        ["TERRAIN TYPES", "CROSSER TYPES", "PRIMARY RULES", "SECONDARY RULES", "TILES", "GROUPS"]
    {
        f.set(list, "Count", "0");
    }
    f
}

/// Takes away the last tile (and its door sections).
pub fn remove_last_tile(f: &mut SetFile) -> bool {
    let n = f.count("TILES");
    let Some(last) = n.checked_sub(1) else { return false };
    let doors =
        f.get(&format!("TILE{last}"), "Doors").and_then(|d| d.parse::<usize>().ok()).unwrap_or(0);
    for d in 0..doors {
        f.remove_section(&format!("TILE{last}DOOR{d}"));
    }
    f.remove_section(&format!("TILE{last}"));
    f.set("TILES", "Count", &last.to_string());
    true
}

/// Adds a group of `rows` × `columns` tiles (row by row from the bottom,
/// `None` for none); returns its index.
pub fn add_group(
    f: &mut SetFile,
    name: &str,
    rows: usize,
    columns: usize,
    tiles: &[Option<u32>],
) -> usize {
    let n = f.count("GROUPS");
    f.set("GROUPS", "Count", &(n + 1).to_string());
    let mut entries = vec![
        ("Name".to_string(), name.to_string()),
        ("Rows".to_string(), rows.to_string()),
        ("Columns".to_string(), columns.to_string()),
    ];
    for (i, t) in tiles.iter().enumerate() {
        entries.push((format!("Tile{i}"), t.map_or("-1".to_string(), |t| t.to_string())));
    }
    f.add_section(&format!("GROUP{n}"), &entries);
    n
}

/// Takes away the last group.
pub fn remove_last_group(f: &mut SetFile) -> bool {
    let n = f.count("GROUPS");
    let Some(last) = n.checked_sub(1) else { return false };
    f.remove_section(&format!("GROUP{last}"));
    f.set("GROUPS", "Count", &last.to_string());
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "; made by hand\r\n[GENERAL]\r\nName=ZZZ01\r\nTransition=5\r\n\r\n[TERRAIN TYPES]\r\nCount=1\r\n\r\n[TERRAIN0]\r\nName=Grass\r\n\r\n[TILES]\r\nCount=1\r\n\r\n[TILE0]\r\nModel=zzz01_a01_01\r\nDoors=1\r\n\r\n[TILE0DOOR0]\r\nType=1\r\n\r\n[GROUPS]\r\nCount=0\r\n";

    #[test]
    fn a_skeleton_reads_as_a_tileset() {
        let f = skeleton("zzz01");
        let t = crate::Tileset::parse(f.text().as_bytes(), mg_core::Codepage::default()).unwrap();
        assert_eq!((t.general.name.as_str(), t.tiles.len()), ("ZZZ01", 0));
        assert!(f.text().starts_with("[GENERAL]\r\nName=ZZZ01\r\n"), "{}", f.text());
    }

    #[test]
    fn untouched_is_the_same_and_edits_touch_their_lines() {
        let mut f = SetFile::parse(SAMPLE);
        assert_eq!(f.text(), SAMPLE);
        assert_eq!(SetFile::parse("[A]\nb=1").text(), "[A]\nb=1");
        f.set("general", "transition", "7.5");
        assert!(f.text().contains("Transition=7.5\r\n"), "the key keeps its spelling");
        f.set("GENERAL", "Interior", "1");
        assert!(f.text().contains("Transition=7.5\r\nInterior=1\r\n\r\n[TERRAIN TYPES]"));
        assert_eq!(f.get("GENERAL", "interior"), Some("1"));
        f.remove_key("GENERAL", "Interior");
        f.set("GENERAL", "Transition", "5");
        assert_eq!(f.text(), SAMPLE);
    }

    #[test]
    fn terrains_tiles_and_groups_keep_their_counts() {
        let mut f = SetFile::parse(SAMPLE);
        assert_eq!(add_type(&mut f, false, "Water", Some(2000)), 1);
        assert_eq!(add_type(&mut f, true, "Road", None), 0);
        assert_eq!(duplicate_tile(&mut f, 0), Some(1));
        assert_eq!(add_group(&mut f, "Pair", 1, 2, &[Some(0), None]), 0);
        let text = f.text();
        assert!(
            text.contains(
                "[TERRAIN0]\r\nName=Grass\r\n\r\n[TERRAIN1]\r\nName=Water\r\nStrRef=2000\r\n"
            ),
            "{text}"
        );
        assert!(text.contains("[TILE1]\r\nModel=zzz01_a01_01\r\nDoors=1\r\n"), "{text}");
        assert!(text.contains("[TILE1DOOR0]\r\nType=1"), "{text}");
        assert!(
            text.contains("[GROUP0]\r\nName=Pair\r\nRows=1\r\nColumns=2\r\nTile0=0\r\nTile1=-1"),
            "{text}"
        );
        let ini = f.ini();
        assert_eq!(ini.section("TILES").unwrap().int("Count"), Some(2));
        assert_eq!(ini.section("CROSSER TYPES").unwrap().int("Count"), Some(1));
        assert_eq!(add_tile(&mut f, "zzz01_a02_01", "Grass"), 2);
        assert_eq!(f.get("TILE2", "TopLeft"), Some("Grass"));
        assert!(remove_last_tile(&mut f));
        assert!(remove_last_tile(&mut f) && remove_last_group(&mut f));
        assert!(
            !f.has_section("TILE1") && !f.has_section("TILE1DOOR0") && !f.has_section("GROUP0")
        );
        assert_eq!(f.get("TILES", "Count"), Some("1"));
    }
}
