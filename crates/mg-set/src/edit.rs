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

    /// A tile's door sections (`[TILE<tile>DOOR<d>]`) that are there: each
    /// door's number and its section's name, by number.
    fn door_sections(&self, tile: usize) -> Vec<(usize, String)> {
        let prefix = format!("TILE{tile}DOOR");
        let mut doors: Vec<(usize, String)> = self
            .lines
            .iter()
            .filter_map(|l| {
                let name = header(l)?;
                let d = name.to_ascii_uppercase().strip_prefix(&prefix)?.parse().ok()?;
                Some((d, name.to_string()))
            })
            .collect();
        doors.sort();
        doors.dedup_by_key(|(d, _)| *d);
        doors
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
        self.insert_section(at, name, entries);
    }

    /// Adds a section right after section `after`, else as
    /// [`SetFile::add_section`] does.
    pub fn add_section_after(&mut self, after: &str, name: &str, entries: &[(String, String)]) {
        match self.body(after) {
            Some((a, b)) => {
                let at = self.end_of(a, b);
                self.insert_section(at, name, entries);
            }
            None => self.add_section(name, entries),
        }
    }

    fn insert_section(&mut self, at: usize, name: &str, entries: &[(String, String)]) {
        let mut new = if at == 0 { Vec::new() } else { vec![String::new()] };
        new.push(format!("[{name}]"));
        new.extend(entries.iter().map(|(k, v)| format!("{k}={v}")));
        self.lines.splice(at..at, new);
    }

    /// Sets a section's keys where it is there (one past a count, which
    /// the game doesn't read), else adds it after section `after`.
    fn put_section(&mut self, name: &str, entries: &[(String, String)], after: &str) {
        if self.has_section(name) {
            for (k, v) in entries {
                self.set(name, k, v);
            }
        } else {
            self.add_section_after(after, name, entries);
        }
    }

    /// A tile's doors as the game reads them: its door sections numbered
    /// below its `Doors` count (which in custom tilesets may be no count
    /// at all), each door's number and section, by number.
    pub fn doors(&self, tile: usize) -> Vec<(usize, String)> {
        let declared = self
            .get(&format!("TILE{tile}"), "Doors")
            .and_then(|d| d.trim().parse::<i64>().ok())
            .unwrap_or(0)
            .max(0);
        let mut doors = self.door_sections(tile);
        doors.retain(|(d, _)| (*d as i64) < declared);
        doors
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

/// Renames terrain `index` (`crosser`: crosser `index`) to `to`, and every
/// place the file names it with it, each value where it stands: the tiles'
/// corners (a crosser: their edges), the general Border, Default and Floor
/// (a terrain's), and the painting rules' Placed, Adjacent and Changed.
/// Names are matched without case, as the game reads them. Returns how
/// many values besides its own name were changed; `None` when there is no
/// such terrain or crosser. (`to` is the caller's to check: not empty, no
/// spaces, no other type's name.) Areas name tiles by number, not
/// terrains, so they need nothing; a palette (`.itp`) names terrains and
/// crossers by name, and follows apart.
pub fn rename_type(f: &mut SetFile, crosser: bool, index: usize, to: &str) -> Option<usize> {
    let own = format!("{}{index}", if crosser { "CROSSER" } else { "TERRAIN" });
    let from = f.get(&own, "Name")?.to_string();
    f.set(&own, "Name", to);
    // Which sections name it, and in which keys: the tiles' (not their
    // doors'), the rules', the general ones.
    let numbered = |name: &str, prefix: &str| {
        name.to_ascii_uppercase()
            .strip_prefix(prefix)
            .is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
    };
    let tile_keys: &[&str] = if crosser { &crate::EDGES } else { &crate::CORNERS };
    // (Rules and the general names are of terrains.)
    let rule_keys: &[&str] = if crosser { &[] } else { &["Placed", "Adjacent", "Changed"] };
    let general_keys: &[&str] = if crosser { &[] } else { &["Border", "Default", "Floor"] };
    // One pass over the lines (a large tileset has thousands of tiles):
    // in each section, the first line of each key, as `get` reads it.
    let mut changed = 0;
    let mut keys: &[&str] = &[];
    let mut seen: Vec<&str> = Vec::new();
    for line in &mut f.lines {
        if let Some(section) = header(line) {
            keys = if numbered(section, "TILE") {
                tile_keys
            } else if numbered(section, "PRIMARY RULE") || numbered(section, "SECONDARY RULE") {
                rule_keys
            } else if section.eq_ignore_ascii_case("GENERAL") {
                general_keys
            } else {
                &[]
            };
            seen.clear();
            continue;
        }
        let Some(key) = key_of(line).and_then(|k| keys.iter().find(|x| x.eq_ignore_ascii_case(k)))
        else {
            continue;
        };
        if seen.contains(key) {
            continue;
        }
        seen.push(key);
        let (written, value) = line.split_once('=').expect("a key's line");
        if value.trim().eq_ignore_ascii_case(&from) {
            *line = format!("{written}={to}");
            changed += 1;
        }
    }
    Some(changed)
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
    // The door sections that are there, up to the count (which in custom
    // tilesets may be no count at all).
    let doors =
        f.get(&format!("TILE{from}"), "Doors").and_then(|d| d.parse::<usize>().ok()).unwrap_or(0);
    for (d, name) in f.door_sections(from) {
        let door = f.entries(&name);
        if d < doors && !door.is_empty() {
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
    for (_, name) in f.door_sections(last) {
        f.remove_section(&name);
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

/// The list section and the sections' name of the primary painting rules
/// (`secondary`: the secondary).
pub fn rule_sections(secondary: bool) -> (&'static str, &'static str) {
    match secondary {
        false => ("PRIMARY RULES", "PRIMARY RULE"),
        true => ("SECONDARY RULES", "SECONDARY RULE"),
    }
}

/// Adds a painting rule (`secondary`: a secondary one), the last: placing
/// a corner of `placed` next to one of `adjacent` turns that one into
/// `changed`, all at height 0. Returns its index.
pub fn add_rule(
    f: &mut SetFile,
    secondary: bool,
    placed: &str,
    adjacent: &str,
    changed: &str,
) -> usize {
    let (list, each) = rule_sections(secondary);
    let n = f.count(list);
    f.set(list, "Count", &(n + 1).to_string());
    let entries = pairs(&[
        ("Placed", placed.to_string()),
        ("PlacedHeight", "0".to_string()),
        ("Adjacent", adjacent.to_string()),
        ("AdjacentHeight", "0".to_string()),
        ("Changed", changed.to_string()),
        ("ChangedHeight", "0".to_string()),
    ]);
    // (After the rule before it, the first after its list, as the game's
    // files have them.)
    let before = format!("{each}{}", n.wrapping_sub(1));
    let after = if n > 0 && f.has_section(&before) { before.as_str() } else { list };
    f.put_section(&format!("{each}{n}"), &entries, after);
    n
}

/// Takes away the last painting rule (`secondary`: the last secondary).
pub fn remove_last_rule(f: &mut SetFile, secondary: bool) -> bool {
    let (list, each) = rule_sections(secondary);
    let n = f.count(list);
    let Some(last) = n.checked_sub(1) else { return false };
    f.remove_section(&format!("{each}{last}"));
    f.set(list, "Count", &last.to_string());
    true
}

/// Adds a door hook to tile `tile`, the last of its doors (after those the
/// game reads: [`SetFile::doors`]), of `doortypes.2da` row `door_type`,
/// at the tile's middle, unturned. Its `Doors` count follows. Returns the
/// door's number; `None` without such a tile.
pub fn add_door(f: &mut SetFile, tile: usize, door_type: i32) -> Option<usize> {
    let section = format!("TILE{tile}");
    if !f.has_section(&section) {
        return None;
    }
    let shown = f.doors(tile);
    let n = shown.last().map_or(0, |(d, _)| d + 1);
    f.set(&section, "Doors", &(n + 1).to_string());
    let entries = pairs(&[
        ("Type", door_type.to_string()),
        ("X", "0.0".to_string()),
        ("Y", "0.0".to_string()),
        ("Z", "0.0".to_string()),
        ("Orientation", "0.0".to_string()),
    ]);
    // (After the tile's last door, else the tile: not after another
    // tile's doors, the last of their family.)
    let name = format!("TILE{tile}DOOR{n}");
    let after = shown.last().map_or(section, |(_, s)| s.clone());
    f.put_section(&name, &entries, &after);
    Some(n)
}

/// Takes away tile `tile`'s last door hook (of those the game reads), its
/// `Doors` count following.
pub fn remove_last_door(f: &mut SetFile, tile: usize) -> bool {
    let mut doors = f.doors(tile);
    let Some((_, name)) = doors.pop() else { return false };
    f.remove_section(&name);
    // (Up to the door before it: a count past a section missing would
    // be a door the game is told of and hasn't.)
    let count = doors.last().map_or(0, |(d, _)| d + 1);
    f.set(&format!("TILE{tile}"), "Doors", &count.to_string());
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
    fn a_renamed_terrain_or_crosser_is_followed_where_the_file_names_it() {
        let text = "[GENERAL]\r\nName=ZZZ01\r\nBorder=grass\r\nDefault=Grass\r\nFloor=Water\r\n\
\r\n[TERRAIN TYPES]\r\nCount=2\r\n\r\n[TERRAIN0]\r\nName=Grass\r\n\r\n[TERRAIN1]\r\nName=Water\r\n\
\r\n[CROSSER TYPES]\r\nCount=1\r\n\r\n[CROSSER0]\r\nName=Road\r\n\
\r\n[PRIMARY RULES]\r\nCount=1\r\n\r\n[PRIMARY RULE0]\r\nPlaced=Water\r\nAdjacent=GRASS\r\nChanged=Grass\r\n\
\r\n[TILES]\r\nCount=1\r\n\r\n[TILE0]\r\nModel=zzz01_a01_01\r\nTopLeft=Grass\r\nTopRight=Water\r\n\
BottomLeft=grass\r\nBottomRight=Grass\r\nTop=Road\r\nRight=\r\nBottom=road\r\nLeft=\r\nDoors=1\r\n\
\r\n[TILE0DOOR0]\r\nType=Grass\r\n";
        let mut f = SetFile::parse(text);
        assert_eq!(rename_type(&mut f, false, 0, "Meadow"), Some(7));
        let t = crate::Tileset::parse(f.text().as_bytes(), mg_core::Codepage::default()).unwrap();
        assert!(t.warnings.is_empty(), "every name found: {:?}", t.warnings);
        assert_eq!(t.terrains[0].name, "Meadow");
        let corners: Vec<&str> = t.tiles[0].corners.iter().map(|c| c.0.as_str()).collect();
        assert_eq!(corners, ["Meadow", "Water", "Meadow", "Meadow"]);
        let g = &t.general;
        assert_eq!(
            (g.border.as_str(), g.default.as_str(), g.floor.as_str()),
            ("Meadow", "Meadow", "Water")
        );
        let r = &t.primary_rules[0];
        assert_eq!(
            (r.placed.as_str(), r.adjacent.as_str(), r.changed.as_str()),
            ("Water", "Meadow", "Meadow")
        );
        // Only those lines: the door's, the other terrain's, the rest stay.
        let now = f.text();
        let changed: Vec<(&str, &str)> =
            text.lines().zip(now.lines()).filter(|(a, b)| a != b).collect();
        assert_eq!(changed.len(), 8, "{changed:?}");
        assert!(f.text().contains("[TILE0DOOR0]\r\nType=Grass"));
        // A crosser: the tiles' edges.
        assert_eq!(rename_type(&mut f, true, 0, "Path"), Some(2));
        assert_eq!((f.get("TILE0", "Top"), f.get("TILE0", "Bottom")), (Some("Path"), Some("Path")));
        assert_eq!(rename_type(&mut f, true, 3, "Nope"), None, "no such crosser");
    }

    /// Painting rules added and taken away keep their counts: the first
    /// right after its list, the next after it; one past the count (which
    /// the game doesn't read) is used again rather than made twice.
    #[test]
    fn rules_keep_their_counts_and_places() {
        let mut f = SetFile::parse(SAMPLE);
        f.set("PRIMARY RULES", "Count", "0");
        // (A rule the file has past its count.)
        f.add_section("PRIMARY RULE1", &pairs(&[("Placed", "Old".into())]));
        assert_eq!(add_rule(&mut f, false, "Water", "Grass", "Sand"), 0);
        assert_eq!(add_rule(&mut f, false, "Grass", "Water", "Sand"), 1);
        assert_eq!(add_rule(&mut f, true, "Grass", "Grass", "Grass"), 0);
        let t = crate::Tileset::parse(f.text().as_bytes(), mg_core::Codepage::default()).unwrap();
        let rule = |r: &crate::Rule| (r.placed.clone(), r.adjacent.clone(), r.changed.clone());
        assert_eq!(
            t.primary_rules.iter().map(rule).collect::<Vec<_>>(),
            [
                ("Water".into(), "Grass".into(), "Sand".into()),
                ("Grass".into(), "Water".into(), "Sand".into())
            ]
        );
        assert_eq!(t.secondary_rules.len(), 1);
        let now = f.text();
        assert_eq!(now.matches("[PRIMARY RULE1]").count(), 1, "{now}");
        assert!(
            now.contains("[PRIMARY RULES]\r\nCount=2\r\n\r\n[PRIMARY RULE0]\r\nPlaced=Water\r\n"),
            "{now}"
        );
        // (A secondary list the file lacked: added, its rule after it.)
        assert!(now.contains("[SECONDARY RULES]\r\nCount=1\r\n\r\n[SECONDARY RULE0]\r\n"), "{now}");
        assert!(remove_last_rule(&mut f, true) && remove_last_rule(&mut f, false));
        assert!(remove_last_rule(&mut f, false) && !remove_last_rule(&mut f, false));
        assert_eq!(f.get("PRIMARY RULES", "Count"), Some("0"));
        assert!(!f.has_section("PRIMARY RULE0") && !f.has_section("PRIMARY RULE1"));
    }

    /// A tile's door hooks added and taken away keep its `Doors` count: a
    /// door goes after the tile's own (not another tile's), and a count
    /// that is no count (a custom tileset's) becomes the doors there are.
    #[test]
    fn door_hooks_keep_their_tile_s_count() {
        let text = "[GENERAL]\r\nName=X\r\n\r\n[TILES]\r\nCount=2\r\n\r\n[TILE0]\r\nModel=a\r\nDoors=1\r\n\r\n[TILE0DOOR0]\r\n\
Type=1\r\n\r\n[TILE1]\r\nModel=b\r\nDoors=1869573190\r\n\r\n[TILE1DOOR0]\r\nType=2\r\n\r\n\
[TILE1DOOR2]\r\nType=3\r\n";
        let mut f = SetFile::parse(text);
        assert_eq!(add_door(&mut f, 0, 4), Some(1));
        let now = f.text();
        assert!(
            now.contains("[TILE0DOOR0]\r\nType=1\r\n\r\n[TILE0DOOR1]\r\nType=4\r\nX=0.0"),
            "{now}"
        );
        assert_eq!(f.get("TILE0", "Doors"), Some("2"));
        assert_eq!(add_door(&mut f, 1, 5), Some(3));
        assert_eq!(f.get("TILE1", "Doors"), Some("4"));
        let t = crate::Tileset::parse(f.text().as_bytes(), mg_core::Codepage::default()).unwrap();
        let types = |i: usize| t.tiles[i].doors.iter().map(|d| d.door_type).collect::<Vec<_>>();
        assert_eq!((types(0), types(1)), (vec![1, 4], vec![2, 3, 5]));
        assert_eq!(add_door(&mut f, 7, 1), None, "no such tile");
        // Taken away: the count is up to the door before.
        assert!(remove_last_door(&mut f, 1));
        assert!(remove_last_door(&mut f, 1));
        assert_eq!(f.get("TILE1", "Doors"), Some("1"), "DOOR1 is no section: up to DOOR0");
        assert!(remove_last_door(&mut f, 1) && !remove_last_door(&mut f, 1));
        assert_eq!(f.get("TILE1", "Doors"), Some("0"));
        assert!(remove_last_door(&mut f, 0));
        assert_eq!(f.doors(0), [(0, "TILE0DOOR0".to_string())]);
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
