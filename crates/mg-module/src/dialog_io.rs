//! Conversations in other formats, to write or review dialogue outside the
//! toolset: plain text (to read), CSV (to edit or translate the lines in a
//! spreadsheet, read back by line), Twine's Twee and Ink (to write
//! branching dialogue, read back as a new conversation).
//!
//! How a conversation maps onto Twine and Ink:
//! - **NPC lines** are passages (Twine) or knots (Ink), named `E3` when
//!   exported. The speaker, action script, journal update and sound are
//!   tags: `speaker:TAG`, `do:SCRIPT`, `journal:QUEST:ENTRY`,
//!   `sound:SOUND`.
//! - **Player replies** are the links (Twine) or choices (Ink) in an NPC
//!   line's passage, leading to the next NPC line, or `END`. A reply's
//!   condition and action go in braces in Twine (`[[Who are you? {if
//!   c_curious}->E5]]`); in Ink the condition is the choice's (`+
//!   {c_curious} [Who are you?] -> E5`, each condition declared a `VAR`)
//!   and the action a tag. An empty reply is `(Continue)`.
//! - **Several NPC lines to choose from** (the first whose condition passes
//!   is said), at the start or after a reply, are a passage tagged
//!   `npc-choice` (`[[if c_seen->E2]]`, `[[otherwise->E3]]`) or a knot of
//!   conditional diverts (`{c_seen: -> E2}`, `-> E3`).
//! - **Links** are references to the same passage or knot.
//!
//! Only English text is exported; condition and action parameters,
//! animations, comments and delays are not. A conversation read back
//! plays as the original did ([`play_outline`]).

use std::collections::HashMap;
use std::fmt::Write as _;

use mg_core::ResRef;
use mg_gff::{Gff, Struct, Value};

use crate::dialog::{
    Kind, Parent, add_link, add_node, is_link, link_index, links, new_dialog, node, node_mut,
    set_condition, text,
};

/// The formats a conversation is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    Text,
    Csv,
    Twine,
    Ink,
}

impl Format {
    pub const ALL: [Format; 4] = [Format::Text, Format::Csv, Format::Twine, Format::Ink];

    pub fn name(self) -> &'static str {
        match self {
            Format::Text => "Plain text",
            Format::Csv => "CSV",
            Format::Twine => "Twine (Twee)",
            Format::Ink => "Ink",
        }
    }

    pub fn extensions(self) -> &'static [&'static str] {
        match self {
            Format::Text => &["txt"],
            Format::Csv => &["csv"],
            Format::Twine => &["twee", "tw"],
            Format::Ink => &["ink"],
        }
    }

    /// The format a file's extension names.
    pub fn of(path: &std::path::Path) -> Option<Format> {
        let ext = path.extension()?.to_str()?.to_ascii_lowercase();
        Format::ALL.into_iter().find(|f| f.extensions().contains(&ext.as_str()))
    }

    /// The conversation written in this format.
    pub fn write(self, g: &Gff, title: &str) -> String {
        match self {
            Format::Text => to_text(g),
            Format::Csv => to_csv(g),
            Format::Twine => to_twee(g, title),
            Format::Ink => to_ink(g),
        }
    }

    /// What reading a file in this format gives.
    pub fn reads(self) -> Reads {
        match self {
            Format::Text => Reads::Nothing,
            Format::Csv => Reads::Text,
            Format::Twine | Format::Ink => Reads::Conversation,
        }
    }

    /// A conversation read from a story in this format; `None` for a
    /// format that does not hold a whole one ([`Format::reads`]).
    pub fn read(self, text: &str) -> Option<Result<Gff, String>> {
        match self {
            Format::Twine => Some(from_twee(text)),
            Format::Ink => Some(from_ink(text)),
            Format::Text | Format::Csv => None,
        }
    }

    /// A conversation's text read back from what was written from it in
    /// this format: how many lines changed; `None` for a format that is not
    /// read that way.
    pub fn update(self, g: &mut Gff, text: &str) -> Option<Result<usize, String>> {
        match self {
            Format::Csv => Some(update_from_csv(g, text)),
            Format::Text | Format::Twine | Format::Ink => None,
        }
    }
}

/// What a conversation format can be read as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reads {
    /// It is only written.
    Nothing,
    /// A whole conversation: a new one, or one replaced.
    Conversation,
    /// The text of the conversation it was written from, back into it.
    Text,
}

fn resref(s: &Struct, label: &str) -> Option<String> {
    s.resref(label).filter(|r| !r.is_empty()).map(|r| r.to_string())
}

fn string(s: &Struct, label: &str) -> String {
    String::from_utf8_lossy(s.string(label).unwrap_or_default()).into_owned()
}

/// A tag's value with spaces and `%` escaped (`kid%202`), as Twine's tags
/// can't hold spaces.
fn escape(v: &str) -> String {
    v.replace('%', "%25").replace(' ', "%20")
}

fn unescape(v: &str) -> String {
    v.replace("%20", " ").replace("%25", "%")
}

/// A line's tags: speaker, action, journal update, sound.
fn tags(kind: Kind, n: &Struct) -> Vec<String> {
    let mut out = Vec::new();
    let speaker = string(n, "Speaker");
    if kind == Kind::Entry && !speaker.is_empty() {
        out.push(format!("speaker:{}", escape(&speaker)));
    }
    if let Some(a) = resref(n, "Script") {
        out.push(format!("do:{}", escape(&a)));
    }
    let quest = string(n, "Quest");
    if !quest.is_empty() {
        out.push(format!("journal:{}:{}", escape(&quest), n.integer("QuestEntry").unwrap_or(0)));
    }
    if let Some(s) = resref(n, "Sound") {
        out.push(format!("sound:{}", escape(&s)));
    }
    out
}

/// Applies tags (`speaker:`, `do:`, `journal:`, `sound:`) to a line;
/// others are ignored.
fn apply_tags(n: &mut Struct, tags: &[String]) -> Result<(), String> {
    let r = |v: &str| ResRef::from_str(v).map(Value::resref).map_err(|e| format!("{v}: {e}"));
    for t in tags {
        let (k, v) = t.split_once(':').unwrap_or((t, ""));
        let v = unescape(v.trim());
        match k.trim() {
            "speaker" => n.set("Speaker", Value::String(v.as_bytes().to_vec())),
            "do" => n.set("Script", r(&v)?),
            "sound" => n.set("Sound", r(&v)?),
            "journal" => {
                let (q, e) = v.rsplit_once(':').unwrap_or((&v, "0"));
                n.set("Quest", Value::String(q.as_bytes().to_vec()));
                n.set("QuestEntry", Value::Dword(e.trim().parse().unwrap_or(0)));
            }
            _ => {}
        }
    }
    Ok(())
}

fn passage(kind: Kind, i: u32) -> String {
    match kind {
        Kind::Entry => format!("E{i}"),
        Kind::Reply => format!("R{i}"),
    }
}

// ---------------------------------------------------------------- escapes

/// Text for Ink: a backslash before each character Ink reads as syntax
/// (and before `-`, `*`, `+` or `=` starting a line), a blank line as an
/// empty comment (`//`), which Ink skips and Moonglow reads back.
fn ink_text(s: &str) -> String {
    s.lines()
        .map(|line| {
            if line.trim().is_empty() {
                return "//".to_string();
            }
            let mut out = String::new();
            for (i, c) in line.trim_start().chars().enumerate() {
                let syntax =
                    matches!(c, '\\' | '{' | '}' | '[' | ']' | '|' | '#' | '/' | '<' | '>' | '~')
                        || (i == 0 && matches!(c, '-' | '*' | '+' | '='));
                if syntax {
                    out.push('\\');
                }
                out.push(c);
            }
            out
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// A line of Ink without its comment (`//` not escaped), escapes kept.
fn ink_uncomment(raw: &str) -> &str {
    let b = raw.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'/' if b.get(i + 1) == Some(&b'/') => return &raw[..i],
            _ => i += 1,
        }
    }
    raw
}

/// Where a character is in Ink, not escaped.
fn ink_find(s: &str, ch: char) -> Option<usize> {
    let mut escaped = false;
    for (i, c) in s.char_indices() {
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == ch {
            return Some(i);
        }
    }
    None
}

/// Ink's escapes undone.
fn ink_unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            out.extend(chars.next());
        } else {
            out.push(c);
        }
    }
    out
}

/// Text for Twine: what its links and macros read as syntax, as HTML
/// character references (which story formats show as the characters), and
/// `:` starting a line (a passage header) likewise.
fn html_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut line_start = true;
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '[' => out.push_str("&#91;"),
            ']' => out.push_str("&#93;"),
            '{' => out.push_str("&#123;"),
            '}' => out.push_str("&#125;"),
            '|' => out.push_str("&#124;"),
            ':' if line_start => out.push_str("&#58;"),
            c => out.push(c),
        }
        line_start = c == '\n';
    }
    out
}

/// HTML character references undone (named `amp`, `lt`, `gt`, `quot`,
/// `apos`, and numeric ones).
fn html_unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let decoded = rest.find(';').filter(|&end| end <= 10).and_then(|end| {
            let name = &rest[1..end];
            let c = match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                n => n
                    .strip_prefix("#x")
                    .or_else(|| n.strip_prefix("#X"))
                    .and_then(|h| u32::from_str_radix(h, 16).ok())
                    .or_else(|| n.strip_prefix('#').and_then(|d| d.parse().ok()))
                    .and_then(char::from_u32),
            }?;
            Some((c, end + 1))
        });
        match decoded {
            Some((c, len)) => {
                out.push(c);
                rest = &rest[len..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

// ---------------------------------------------------------------- outline

/// The conversation as it plays: every path from the start, links
/// followed (an NPC line already on the path ends it), as `kind|text|if
/// c|do a|journal|speaker` lines. Two conversations with the same play
/// outline say the same things under the same conditions, however their
/// lines are owned and linked, and whether a reply reached twice is one
/// line or two. `None` past `limit` lines.
pub fn play_outline(g: &Gff, limit: usize) -> Option<Vec<String>> {
    fn walk(
        g: &Gff,
        parent: Parent,
        depth: usize,
        path: &mut Vec<(Kind, u32)>,
        out: &mut Vec<String>,
        limit: usize,
    ) -> bool {
        let kind = parent.child_kind();
        for l in links(g, parent) {
            let i = link_index(l);
            let Some(n) = node(g, kind, i) else { continue };
            // Replies are told apart by text, not identity: Twine and Ink
            // have none.
            let looped = kind == Kind::Entry && path.contains(&(kind, i));
            // Whitespace around a line's text and its lines doesn't show.
            let t: Vec<String> = text(n).lines().map(|l| l.trim().to_string()).collect();
            out.push(format!(
                "{}{kind:?}|{}|if {}|{}{}",
                "  ".repeat(depth),
                t.join("\n").trim(),
                resref(l, "Active").unwrap_or_default(),
                tags(kind, n).join(" "),
                if looped { "|again" } else { "" }
            ));
            if out.len() > limit {
                return false;
            }
            if !looped {
                path.push((kind, i));
                let ok = walk(g, Parent::Node(kind, i), depth + 1, path, out, limit);
                path.pop();
                if !ok {
                    return false;
                }
            }
        }
        true
    }
    let mut out = Vec::new();
    walk(g, Parent::Root, 0, &mut Vec::new(), &mut out, limit).then_some(out)
}

// ---------------------------------------------------------------- text

/// The conversation as readable text: NPC lines and the numbered replies
/// under them, conditions and actions in brackets, links named.
pub fn to_text(g: &Gff) -> String {
    fn walk(g: &Gff, parent: Parent, depth: usize, out: &mut String) {
        let kind = parent.child_kind();
        for (pos, l) in links(g, parent).iter().enumerate() {
            let i = link_index(l);
            let Some(n) = node(g, kind, i) else { continue };
            let mut notes: Vec<String> = Vec::new();
            if let Some(c) = resref(l, "Active") {
                notes.push(format!("if {c}"));
            }
            notes.extend(tags(kind, n).into_iter().filter(|t| !t.starts_with("speaker:")));
            let who = match kind {
                Kind::Entry => {
                    let s = string(n, "Speaker");
                    if s.is_empty() { "NPC".to_string() } else { s }
                }
                Kind::Reply => format!("{}.", pos + 1),
            };
            let t = text(n);
            let t = if t.is_empty() && kind == Kind::Reply { "(Continue)".into() } else { t };
            let pad = "    ".repeat(depth);
            let mut line = format!("{pad}{who} {}", t.replace('\n', &format!("\n{pad}  ")));
            if !notes.is_empty() {
                let _ = write!(line, "   [{}]", notes.join("; "));
            }
            if is_link(l) {
                let _ = write!(line, "   (goes to {} above)", passage(kind, i));
            }
            out.push_str(&line);
            out.push('\n');
            if !is_link(l) {
                walk(g, Parent::Node(kind, i), depth + 1, out);
            }
        }
    }
    let mut out = String::new();
    walk(g, Parent::Root, 0, &mut out);
    out
}

// ---------------------------------------------------------------- CSV

pub(crate) fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Reads CSV (RFC 4180: quoted fields may hold commas, quotes and line
/// breaks).
pub fn read_csv(text: &str) -> Result<Vec<Vec<String>>, String> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    let mut line = 1;
    while let Some(c) = chars.next() {
        if quoted {
            match c {
                '"' if chars.peek() == Some(&'"') => {
                    chars.next();
                    field.push('"');
                }
                '"' => quoted = false,
                c => {
                    if c == '\n' {
                        line += 1;
                    }
                    field.push(c);
                }
            }
            continue;
        }
        match c {
            '"' if field.is_empty() => quoted = true,
            ',' => row.push(std::mem::take(&mut field)),
            '\r' if chars.peek() == Some(&'\n') => {}
            '\n' => {
                line += 1;
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
            }
            c => field.push(c),
        }
    }
    if quoted {
        return Err(format!("line {line}: a quoted field doesn't end"));
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    Ok(rows)
}

const CSV_HEADER: [&str; 7] = ["id", "speaker", "text", "condition", "action", "parent", "comment"];

/// Every line as a CSV row: `id` (`E3` an NPC line, `R5` a reply),
/// speaker, text, condition, action, the line it follows, comment.
/// Only `speaker`, `text` and `comment` are read back
/// ([`update_from_csv`]).
pub fn to_csv(g: &Gff) -> String {
    let mut out = CSV_HEADER.join(",") + "\r\n";
    let mut seen = std::collections::HashSet::new();
    fn walk(
        g: &Gff,
        parent: Parent,
        seen: &mut std::collections::HashSet<(Kind, u32)>,
        out: &mut String,
    ) {
        let kind = parent.child_kind();
        for l in links(g, parent) {
            let i = link_index(l);
            if is_link(l) || !seen.insert((kind, i)) {
                continue;
            }
            let Some(n) = node(g, kind, i) else { continue };
            let from = match parent {
                Parent::Root => "start".to_string(),
                Parent::Node(k, p) => passage(k, p),
            };
            let row = [
                passage(kind, i),
                string(n, "Speaker"),
                text(n),
                resref(l, "Active").unwrap_or_default(),
                resref(n, "Script").unwrap_or_default(),
                from,
                string(n, "Comment"),
            ];
            out.push_str(&row.iter().map(|f| csv_field(f)).collect::<Vec<_>>().join(","));
            out.push_str("\r\n");
            walk(g, Parent::Node(kind, i), seen, out);
        }
    }
    walk(g, Parent::Root, &mut seen, &mut out);
    out
}

/// Reads back the speaker, text and comment of the lines a CSV names by
/// `id` (columns found by their header; others ignored). How many lines
/// changed; an unknown id or a missing `id` column is an error.
pub fn update_from_csv(g: &mut Gff, csv: &str) -> Result<usize, String> {
    let rows = read_csv(csv)?;
    let (header, rows) = rows.split_first().ok_or("the file is empty")?;
    let col = |name: &str| header.iter().position(|h| h.trim().eq_ignore_ascii_case(name));
    let id = col("id").ok_or("no id column")?;
    let (speaker, text_col, comment) = (col("speaker"), col("text"), col("comment"));
    let mut changed = 0;
    for (n, row) in rows.iter().enumerate() {
        if row.iter().all(|f| f.is_empty()) {
            continue;
        }
        let line = n + 2;
        let name = row.get(id).map(|s| s.trim()).unwrap_or_default();
        let (kind, rest) = match name.split_at_checked(1) {
            Some(("E", r)) => (Kind::Entry, r),
            Some(("R", r)) => (Kind::Reply, r),
            _ => return Err(format!("line {line}: {name:?} isn't a line id (E3, R5)")),
        };
        let index: u32 =
            rest.parse().map_err(|_| format!("line {line}: {name:?} isn't a line id"))?;
        let n = node_mut(g, kind, index).ok_or(format!("line {line}: there's no line {name}"))?;
        let before = n.clone();
        if let Some(t) = text_col.and_then(|c| row.get(c)).filter(|t| **t != text(n)) {
            let mut ls = n.locstring("Text").cloned().unwrap_or_default();
            ls.set_text(mg_core::Language::ENGLISH, mg_core::Gender::Male, t).ok_or(format!(
                "line {line}: {name}'s text can't be written in English's codepage"
            ))?;
            n.set("Text", Value::LocString(ls));
        }
        if kind == Kind::Entry
            && let Some(s) =
                speaker.and_then(|c| row.get(c)).filter(|s| **s != string(n, "Speaker"))
        {
            n.set("Speaker", Value::String(s.as_bytes().to_vec()));
        }
        if let Some(c) = comment.and_then(|c| row.get(c)).filter(|c| **c != string(n, "Comment")) {
            n.set("Comment", Value::String(c.as_bytes().to_vec()));
        }
        changed += usize::from(*n != before);
    }
    if changed > 0 {
        crate::dialog::recount(g);
    }
    Ok(changed)
}

// ---------------------------------------------------------------- Twine

/// Where a reply leads: nowhere (the end), one NPC line, or several to
/// choose from (with conditions).
enum Next {
    End,
    One(u32),
    Choice,
}

fn next_of(g: &Gff, parent: Parent) -> Next {
    let l = links(g, parent);
    match l {
        [] => Next::End,
        [one] if resref(one, "Active").is_none() => Next::One(link_index(one)),
        _ => Next::Choice,
    }
}

/// A Twine story ID (a UUID) made from the title, so exports are stable.
fn ifid(title: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    title.hash(&mut h);
    let a = h.finish();
    (a, "moonglow").hash(&mut h);
    let b = h.finish();
    format!(
        "{:08X}-{:04X}-4{:03X}-8{:03X}-{:012X}",
        a >> 32,
        (a >> 16) & 0xFFFF,
        a & 0xFFF,
        b >> 52,
        b & 0xFFFF_FFFF_FFFF
    )
}

/// The conversation as a Twine story (Twee 3; see the module's notes).
pub fn to_twee(g: &Gff, title: &str) -> String {
    let start = match next_of(g, Parent::Root) {
        Next::One(i) => passage(Kind::Entry, i),
        _ => "Start".to_string(),
    };
    let mut out = format!(
        ":: StoryTitle\n{title}\n\n:: StoryData\n{{\n  \"ifid\": \"{}\",\n  \"format\": \"Harlowe\",\n  \"format-version\": \"3.3.9\",\n  \"start\": \"{start}\"\n}}\n\n",
        ifid(title)
    );
    let choice = |out: &mut String, name: &str, parent: Parent| {
        let _ = writeln!(out, ":: {name} [npc-choice]");
        for l in links(g, parent) {
            let cond = resref(l, "Active").map_or("otherwise".to_string(), |c| format!("if {c}"));
            let _ = writeln!(out, "[[{cond}->{}]]", passage(Kind::Entry, link_index(l)));
        }
        out.push('\n');
    };
    if start == "Start" {
        choice(&mut out, "Start", Parent::Root);
    }
    for (i, n) in crate::dialog::nodes(g, Kind::Entry).iter().enumerate() {
        let i = i as u32;
        let tags = tags(Kind::Entry, n);
        let tags = if tags.is_empty() { String::new() } else { format!(" [{}]", tags.join(" ")) };
        let _ = writeln!(out, ":: {}{tags}", passage(Kind::Entry, i));
        let t = text(n);
        if !t.is_empty() {
            out.push_str(&html_text(&t));
            out.push_str("\n\n");
        }
        for l in links(g, Parent::Node(Kind::Entry, i)) {
            let r = link_index(l);
            let Some(reply) = node(g, Kind::Reply, r) else { continue };
            let t = text(reply);
            let mut label = if t.is_empty() {
                "(Continue)".to_string()
            } else {
                html_text(&t).replace('\n', "&#10;")
            };
            if let Some(c) = resref(l, "Active") {
                let _ = write!(label, " {{if {c}}}");
            }
            for tag in tags_of_reply(reply) {
                let _ = write!(label, " {{{tag}}}");
            }
            let target = match next_of(g, Parent::Node(Kind::Reply, r)) {
                Next::End => "END".to_string(),
                Next::One(e) => passage(Kind::Entry, e),
                Next::Choice => passage(Kind::Reply, r),
            };
            let _ = writeln!(out, "[[{label}->{target}]]");
        }
        out.push('\n');
    }
    // The replies leading to a choice of NPC lines.
    for (r, _) in crate::dialog::nodes(g, Kind::Reply).iter().enumerate() {
        let parent = Parent::Node(Kind::Reply, r as u32);
        if matches!(next_of(g, parent), Next::Choice) {
            choice(&mut out, &passage(Kind::Reply, r as u32), parent);
        }
    }
    out
}

/// A reply's tags in Twine's braces: `do a_x`, `journal q:10`, `sound s`.
fn tags_of_reply(n: &Struct) -> Vec<String> {
    tags(Kind::Reply, n).into_iter().map(|t| t.replacen(':', " ", 1)).collect()
}

/// A Twine story as read: its passages, its start (`StoryData`'s), and
/// the passages' names in order.
type Story = (HashMap<String, Passage>, Option<String>, Vec<String>);

/// A reply as read: text, condition, tags, where it leads (`None`: the
/// end).
type Reply = (String, Option<String>, Vec<String>, Option<String>);

/// A Twine passage, as read.
#[derive(Debug, Default)]
struct Passage {
    tags: Vec<String>,
    text: String,
    /// Its links: text and target.
    links: Vec<(String, String)>,
}

fn parse_twee(source: &str) -> Result<Story, String> {
    let mut passages: HashMap<String, Passage> = HashMap::new();
    let mut order = Vec::new();
    let mut current: Option<(String, Vec<String>, String)> = None;
    let mut start = None;
    let finish = |c: Option<(String, Vec<String>, String)>,
                  passages: &mut HashMap<String, Passage>,
                  start: &mut Option<String>| {
        let Some((name, tags, body)) = c else { return };
        match name.as_str() {
            "StoryTitle" => {}
            "StoryData" => {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
                    *start = v.get("start").and_then(|s| s.as_str()).map(str::to_string);
                }
            }
            _ => {
                let mut links = Vec::new();
                let mut text = String::new();
                let mut rest = body.as_str();
                while let Some(at) = rest.find("[[") {
                    text.push_str(&rest[..at]);
                    let Some(end) = rest[at..].find("]]") else { break };
                    let inner = &rest[at + 2..at + end];
                    let (label, target) = if let Some((l, t)) = inner.rsplit_once("->") {
                        (l, t)
                    } else if let Some((t, l)) = inner.split_once("<-") {
                        (l, t)
                    } else if let Some((l, t)) = inner.split_once('|') {
                        (l, t)
                    } else {
                        (inner, inner)
                    };
                    links.push((label.trim().to_string(), target.trim().to_string()));
                    rest = &rest[at + end + 2..];
                }
                text.push_str(rest);
                let text = text.lines().map(str::trim_end).collect::<Vec<_>>().join("\n");
                let text = html_unescape(text.trim());
                passages.insert(name, Passage { tags, text, links });
            }
        }
    };
    for line in source.lines() {
        if let Some(header) = line.strip_prefix("::") {
            finish(current.take(), &mut passages, &mut start);
            // Name, then [tags], then {metadata}.
            let header = header.trim();
            let (head, _) = header.split_once('{').map_or((header, ""), |(a, b)| (a.trim(), b));
            let (name, tags) = match head.split_once('[') {
                Some((n, t)) => (
                    n.trim(),
                    t.trim_end_matches(']').split_whitespace().map(str::to_string).collect(),
                ),
                None => (head, Vec::new()),
            };
            let name = name.replace("\\[", "[").replace("\\]", "]");
            if !matches!(name.as_str(), "StoryTitle" | "StoryData") {
                order.push(name.clone());
            }
            current = Some((name, tags, String::new()));
        } else if let Some((_, _, body)) = current.as_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    finish(current.take(), &mut passages, &mut start);
    Ok((passages, start, order))
}

/// A reply's label as read: its text and the braces after it (`{if c}`,
/// `{do a}`, `{journal q:10}`, `{sound s}`).
fn reply_label(label: &str) -> (String, Option<String>, Vec<String>) {
    let mut text = label.to_string();
    let mut cond = None;
    let mut tags = Vec::new();
    while let Some(open) = text.rfind('{') {
        let Some(close) = text[open..].find('}') else { break };
        let inner = text[open + 1..open + close].trim().to_string();
        match inner.split_once(' ') {
            Some(("if", c)) => cond = Some(c.trim().to_string()),
            Some((k @ ("do" | "journal" | "sound"), v)) => tags.push(format!("{k}:{}", v.trim())),
            _ => break,
        }
        text.replace_range(open..open + close + 1, "");
    }
    let text = html_unescape(text.trim());
    let text = if text == "(Continue)" { String::new() } else { text };
    (text, cond, tags)
}

/// Builds a conversation from NPC lines named by passage or knot: each
/// line is made where it's first reached and linked to after.
struct Builder<'a> {
    g: Gff,
    made: HashMap<String, u32>,
    lines: &'a HashMap<String, Line>,
}

/// An NPC line (or a choice of them), as read from Twine or Ink.
#[derive(Debug, Default)]
struct Line {
    text: String,
    tags: Vec<String>,
    /// For a choice: the NPC lines, each with its condition.
    choice: Option<Vec<(Option<String>, String)>>,
    replies: Vec<Reply>,
}

impl Builder<'_> {
    fn place(
        &mut self,
        parent: Parent,
        name: &str,
        cond: Option<&str>,
        depth: usize,
    ) -> Result<(), String> {
        if depth > 2000 {
            return Err("the story nests too deep".into());
        }
        let line = self.lines.get(name).ok_or(format!("{name:?} isn't a passage or knot"))?;
        if let Some(choice) = &line.choice {
            for (c, target) in choice {
                self.place(parent, target, c.as_deref(), depth + 1)?;
            }
            return Ok(());
        }
        let pos = links(&self.g, parent).len();
        if let Some(&i) = self.made.get(name) {
            let linked = match parent {
                Parent::Root => crate::dialog::add_start(&mut self.g, i),
                _ => add_link(&mut self.g, parent, i),
            };
            if !linked {
                return Ok(());
            }
        } else {
            let i = add_node(&mut self.g, parent, &line.text);
            apply_tags(node_mut(&mut self.g, Kind::Entry, i).expect("made"), &line.tags)?;
            self.made.insert(name.to_string(), i);
            for (text, rc, tags, to) in &line.replies {
                let at = Parent::Node(Kind::Entry, i);
                let r = add_node(&mut self.g, at, text);
                apply_tags(node_mut(&mut self.g, Kind::Reply, r).expect("made"), tags)?;
                if let Some(c) = rc {
                    let last = links(&self.g, at).len() - 1;
                    set_condition(&mut self.g, at, last, c);
                }
                if let Some(to) = to {
                    self.place(Parent::Node(Kind::Reply, r), to, None, depth + 1)?;
                }
            }
        }
        if let Some(c) = cond {
            set_condition(&mut self.g, parent, pos, c);
        }
        Ok(())
    }

    fn build(lines: &HashMap<String, Line>, start: &str) -> Result<Gff, String> {
        let mut b = Builder { g: new_dialog(), made: HashMap::new(), lines };
        b.place(Parent::Root, start, None, 0)?;
        crate::dialog::recount(&mut b.g);
        Ok(b.g)
    }
}

/// Reads a Twine story (Twee 3, as [`to_twee`] writes it, or written in
/// Twine) as a new conversation. Harlowe's or SugarCube's macros are kept
/// as text.
pub fn from_twee(source: &str) -> Result<Gff, String> {
    let (passages, start, order) = parse_twee(source)?;
    let start = start.or_else(|| order.first().cloned()).ok_or("no passages")?;
    let mut lines = HashMap::new();
    for (name, p) in &passages {
        let line = if p.tags.iter().any(|t| t == "npc-choice") {
            let choice = p
                .links
                .iter()
                .map(|(label, target)| {
                    let cond = label.trim().strip_prefix("if ").map(|c| c.trim().to_string());
                    (cond, target.clone())
                })
                .collect();
            Line { choice: Some(choice), ..Default::default() }
        } else {
            let replies = p
                .links
                .iter()
                .map(|(label, target)| {
                    let (text, cond, tags) = reply_label(label);
                    let to = (target != "END").then(|| target.clone());
                    (text, cond, tags, to)
                })
                .collect();
            Line { text: p.text.clone(), tags: p.tags.clone(), choice: None, replies }
        };
        lines.insert(name.clone(), line);
    }
    Builder::build(&lines, &start)
}

// ---------------------------------------------------------------- Ink

/// The conversation as an Ink story (see the module's notes): each
/// condition declared a `VAR`, true, so it plays in Inky as written.
pub fn to_ink(g: &Gff) -> String {
    let mut conditions: Vec<String> = Vec::new();
    let mut add = |c: Option<String>| {
        if let Some(c) = c
            && !conditions.contains(&c)
        {
            conditions.push(c);
        }
    };
    let entries = crate::dialog::nodes(g, Kind::Entry);
    let replies = crate::dialog::nodes(g, Kind::Reply);
    for l in links(g, Parent::Root) {
        add(resref(l, "Active"));
    }
    for (i, _) in entries.iter().enumerate() {
        for l in links(g, Parent::Node(Kind::Entry, i as u32)) {
            add(resref(l, "Active"));
        }
    }
    for (i, _) in replies.iter().enumerate() {
        for l in links(g, Parent::Node(Kind::Reply, i as u32)) {
            add(resref(l, "Active"));
        }
    }
    let mut out = String::new();
    for c in &conditions {
        let _ = writeln!(out, "VAR {c} = true");
    }
    if !conditions.is_empty() {
        out.push('\n');
    }
    let divert = |out: &mut String, parent: Parent| {
        for l in links(g, parent) {
            let to = passage(Kind::Entry, link_index(l));
            match resref(l, "Active") {
                Some(c) => {
                    let _ = writeln!(out, "{{{c}: -> {to}}}");
                }
                None => {
                    let _ = writeln!(out, "-> {to}");
                }
            }
        }
    };
    match next_of(g, Parent::Root) {
        Next::One(i) => {
            let _ = writeln!(out, "-> {}\n", passage(Kind::Entry, i));
        }
        Next::End => out.push_str("-> END\n\n"),
        Next::Choice => {
            out.push_str("-> start\n\n=== start ===\n");
            divert(&mut out, Parent::Root);
            out.push('\n');
        }
    }
    for (i, n) in entries.iter().enumerate() {
        let i = i as u32;
        let _ = writeln!(out, "=== {} ===", passage(Kind::Entry, i));
        for t in tags(Kind::Entry, n) {
            let _ = writeln!(out, "# {t}");
        }
        let t = text(n);
        if !t.trim().is_empty() {
            out.push_str(&ink_text(&t));
            out.push('\n');
        }
        for l in links(g, Parent::Node(Kind::Entry, i)) {
            let r = link_index(l);
            let Some(reply) = node(g, Kind::Reply, r) else { continue };
            let t = text(reply);
            // A choice is one line: its line breaks are written `\n`.
            let t = if t.is_empty() {
                "(Continue)".to_string()
            } else {
                t.lines().map(ink_text).collect::<Vec<_>>().join("\\n")
            };
            let cond = resref(l, "Active").map(|c| format!("{{{c}}} ")).unwrap_or_default();
            let target = match next_of(g, Parent::Node(Kind::Reply, r)) {
                Next::End => "END".to_string(),
                Next::One(e) => passage(Kind::Entry, e),
                Next::Choice => passage(Kind::Reply, r),
            };
            let tags: String = tags(Kind::Reply, reply).iter().map(|t| format!(" # {t}")).collect();
            let _ = writeln!(out, "+ {cond}[{t}] -> {target}{tags}");
        }
        out.push('\n');
    }
    for (r, _) in replies.iter().enumerate() {
        let parent = Parent::Node(Kind::Reply, r as u32);
        if matches!(next_of(g, parent), Next::Choice) {
            let _ = writeln!(out, "=== {} ===", passage(Kind::Reply, r as u32));
            divert(&mut out, parent);
            out.push('\n');
        }
    }
    out
}

/// Reads an Ink story as a new conversation: the subset [`to_ink`]
/// writes (knots, `# tags`, text, choices with a condition and a divert,
/// conditional diverts). Anything else (stitches, gathers, logic, glue) is
/// refused with its line.
pub fn from_ink(source: &str) -> Result<Gff, String> {
    let mut lines: HashMap<String, Line> = HashMap::new();
    let mut start: Option<String> = None;
    let mut current: Option<String> = None;
    let mut first_knot: Option<String> = None;
    for (n, raw) in source.lines().enumerate() {
        let at = n + 1;
        // An empty comment inside a knot's text is a blank line in it.
        if raw.trim() == "//" {
            if let Some(k) = current.as_ref().and_then(|k| lines.get_mut(k))
                && !k.text.is_empty()
            {
                k.text.push('\n');
            }
            continue;
        }
        let line = ink_uncomment(raw).trim();
        if line.is_empty() || line.starts_with("VAR ") || line.starts_with("CONST ") {
            continue;
        }
        if let Some(name) = line.strip_prefix("===") {
            let name = name.trim().trim_end_matches('=').trim();
            if name.contains(char::is_whitespace) {
                return Err(format!("line {at}: knots with parameters aren't read"));
            }
            lines.insert(name.to_string(), Line::default());
            first_knot.get_or_insert_with(|| name.to_string());
            current = Some(name.to_string());
            continue;
        }
        let Some(knot) = current.clone() else {
            // Before the first knot: the start's divert.
            match line.strip_prefix("->") {
                Some(t) => start = Some(t.trim().to_string()),
                None => return Err(format!("line {at}: text before the first knot isn't read")),
            }
            continue;
        };
        let k = lines.get_mut(&knot).expect("made");
        if let Some(tag) = line.strip_prefix('#') {
            k.tags.push(tag.trim().to_string());
        } else if let Some(choice) = line.strip_prefix('+').or_else(|| line.strip_prefix('*')) {
            // + {cond} [text] -> target # tag
            let (choice, tags) = match ink_find(choice, '#') {
                Some(at) => (
                    &choice[..at],
                    choice[at + 1..].split('#').map(|t| t.trim().to_string()).collect(),
                ),
                None => (choice, Vec::new()),
            };
            let mut rest = choice.trim();
            let mut cond = None;
            if let Some(r) = rest.strip_prefix('{') {
                let end = r.find('}').ok_or(format!("line {at}: a condition doesn't end"))?;
                cond = Some(r[..end].trim().to_string());
                rest = r[end + 1..].trim();
            }
            let (text, after) = match rest.strip_prefix('[') {
                Some(r) => {
                    let end = ink_find(r, ']')
                        .ok_or(format!("line {at}: a choice's [text] doesn't end"))?;
                    (r[..end].replace("\\n", "\n"), r[end + 1..].trim())
                }
                None => match rest.split_once("->") {
                    Some((t, _)) => (t.trim().to_string(), &rest[t.len()..]),
                    None => (rest.to_string(), ""),
                },
            };
            let target = after
                .trim()
                .strip_prefix("->")
                .map(|t| t.trim().to_string())
                .ok_or(format!("line {at}: a choice must divert (-> KNOT or -> END)"))?;
            let text = text.trim();
            let text = if text == "(Continue)" {
                String::new()
            } else {
                text.split('\n').map(ink_unescape).collect::<Vec<_>>().join("\n")
            };
            let to = (target != "END" && target != "DONE").then_some(target);
            k.replies.push((text, cond, tags, to));
        } else if let Some(r) = line.strip_prefix('{') {
            // {cond: -> target}
            let inner = r.strip_suffix('}').ok_or(format!("line {at}: logic isn't read"))?;
            let (cond, target) = inner
                .split_once(':')
                .and_then(|(c, t)| Some((c.trim(), t.trim().strip_prefix("->")?.trim())))
                .ok_or(format!("line {at}: only {{condition: -> KNOT}} is read"))?;
            k.choice
                .get_or_insert_with(Vec::new)
                .push((Some(cond.to_string()), target.to_string()));
        } else if let Some(t) = line.strip_prefix("->") {
            k.choice.get_or_insert_with(Vec::new).push((None, t.trim().to_string()));
        } else if line.starts_with('-')
            || line.starts_with('=')
            || line.starts_with('~')
            || line.contains("<>")
        {
            return Err(format!("line {at}: gathers, stitches, logic and glue aren't read"));
        } else {
            if !k.text.is_empty() {
                k.text.push('\n');
            }
            k.text.push_str(&ink_unescape(line));
        }
    }
    let start = start.or(first_knot).ok_or("no knots")?;
    if start == "END" {
        return Ok(new_dialog());
    }
    Builder::build(&lines, &start)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A greeting chosen by condition, replies with a condition and an
    /// action, a link back, a continue.
    fn sample() -> Gff {
        let mut g = new_dialog();
        let again = add_node(&mut g, Parent::Root, "Back again?");
        set_condition(&mut g, Parent::Root, 0, "c_seen");
        let hello = add_node(&mut g, Parent::Root, "Hello, stranger.");
        let n = node_mut(&mut g, Kind::Entry, hello).unwrap();
        n.set("Speaker", Value::String(b"KEEPER".to_vec()));
        n.set("Script", Value::resref(ResRef::from_str("a_hello").unwrap()));
        let at = Parent::Node(Kind::Entry, hello);
        let who = add_node(&mut g, at, "Who are you?");
        set_condition(&mut g, at, 0, "c_curious");
        let bye = add_node(&mut g, at, "Bye, \"friend\", see you.");
        node_mut(&mut g, Kind::Reply, bye)
            .unwrap()
            .set("Script", Value::resref(ResRef::from_str("a_bye").unwrap()));
        let keeper = add_node(&mut g, Parent::Node(Kind::Reply, who), "I am the keeper.\nOf keys.");
        let cont = add_node(&mut g, Parent::Node(Kind::Entry, keeper), "");
        add_link(&mut g, Parent::Node(Kind::Reply, cont), hello);
        add_node(&mut g, Parent::Node(Kind::Entry, again), "Yes.");
        crate::dialog::recount(&mut g);
        g
    }

    /// What a format says it reads is what it reads.
    #[test]
    fn formats_read_what_they_say() {
        for f in Format::ALL {
            let mut g = Gff::new(*b"DLG ");
            let (read, update) = (f.read("").is_some(), f.update(&mut g, "").is_some());
            let expected = match f.reads() {
                Reads::Nothing => (false, false),
                Reads::Conversation => (true, false),
                Reads::Text => (false, true),
            };
            assert_eq!((read, update), expected, "{}", f.name());
        }
    }

    #[test]
    fn twine_and_ink_play_as_the_original() {
        let g = sample();
        let want = play_outline(&g, 10_000).unwrap();
        let twee = to_twee(&g, "keeper");
        assert!(
            twee.contains(":: Start [npc-choice]\n[[if c_seen->E0]]\n[[otherwise->E1]]"),
            "{twee}"
        );
        assert!(twee.contains("[[Who are you? {if c_curious}->E2]]"), "{twee}");
        let back = from_twee(&twee).unwrap();
        assert_eq!(play_outline(&back, 10_000).unwrap(), want, "{twee}");
        let ink = to_ink(&g);
        assert!(ink.contains("VAR c_seen = true"), "{ink}");
        assert!(ink.contains("+ {c_curious} [Who are you?] -> E2"), "{ink}");
        let back = from_ink(&ink).unwrap();
        assert_eq!(play_outline(&back, 10_000).unwrap(), want, "{ink}");
    }

    #[test]
    fn csv_edits_read_back_by_line() {
        let mut g = sample();
        let csv = to_csv(&g);
        assert!(csv.starts_with("id,speaker,text,condition,action,parent,comment\r\n"));
        assert!(csv.contains("R1,,\"Bye, \"\"friend\"\", see you.\",,a_bye,E1,"), "{csv}");
        // Untouched: nothing changes.
        assert_eq!(update_from_csv(&mut g, &csv), Ok(0));
        let edited = csv.replace("I am the keeper.\nOf keys.", "I keep the keys.");
        assert_eq!(update_from_csv(&mut g, &edited), Ok(1));
        assert_eq!(text(node(&g, Kind::Entry, 2).unwrap()), "I keep the keys.");
        assert!(
            update_from_csv(&mut g, "id,text\r\nE99,x\r\n").unwrap_err().contains("no line E99")
        );
        assert_eq!(read_csv("a,\"b\nc\",d\r\n").unwrap(), [vec!["a", "b\nc", "d"]]);
    }

    #[test]
    fn plain_text_reads_as_a_script() {
        let t = to_text(&sample());
        assert!(t.contains("NPC Back again?   [if c_seen]"), "{t}");
        assert!(t.contains("    1. Who are you?   [if c_curious]"), "{t}");
        assert!(t.contains("(goes to E1 above)"), "{t}");
    }

    #[test]
    fn unsupported_ink_is_refused_with_its_line() {
        let e = from_ink("-> a\n=== a ===\nHi\n- gather\n").unwrap_err();
        assert!(e.starts_with("line 4:"), "{e}");
    }
}
