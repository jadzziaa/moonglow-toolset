//! Conversations in other formats, to write or review dialogue outside the
//! toolset: plain text (to read), CSV (to edit or translate the lines in a
//! spreadsheet, read back by line), Twine's Twee and Ink (to write
//! branching dialogue, read back as a new conversation).
//!
//! How a conversation maps onto Twine and Ink:
//! - **NPC lines** are passages (Twine) or knots (Ink), named `E3` when
//!   exported. The speaker, action script, journal update, sound,
//!   animation, delay and comment are tags: `speaker:TAG`, `do:SCRIPT`,
//!   `journal:QUEST:ENTRY`, `sound:SOUND`, `anim:talk-forceful` (the
//!   Other Actions tab's name, else its number), `delay:500` (where it
//!   isn't the game's own), `comment:TEXT` (escaped as a parameter's
//!   value is, and a space too in a Twine passage's tags).
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
//! - **Script parameters** follow the script's name, `name=value`
//!   separated by spaces, a name or value with a space (or empty) in
//!   quotes, and `%`, `"`, `\`, line breaks and what Ink or Twine reads as
//!   syntax (`# / { } [ ] | < >`) percent-escaped (`%22`): `do:a_give
//!   item="gold coins" count=5`, `{if c_has item=gold}`. In Ink a
//!   condition's parameters are a tag naming it, `# if:c_has item=gold`:
//!   on the choice, or on its own line before a conditional divert. In a
//!   Twine passage's tags, which can't hold a space, an action's are a
//!   query: `do:a_give?item=gold%20coins&count=5`. Names and values are
//!   text in the game's codepage, as the lines' are.
//!
//! A CSV carries every language's text, a column each; Twine and Ink,
//! which hold one text a line, carry English's. A conversation read back
//! plays as the original did ([`play_outline`]).

use std::collections::HashMap;
use std::fmt::Write as _;

use mg_core::{Codepage, Gender, Language, LocString, LocStringKey, ResRef};
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

    /// [`write`](Self::write) as a game whose codepage is `game` reads the
    /// text (a module's `encoding.2da` table, else Windows-1252): a CSV
    /// with a column for each of `languages` too ([`to_csv_in`]); the
    /// others in English.
    pub fn write_in(self, g: &Gff, title: &str, game: Codepage, languages: &[Language]) -> String {
        match self {
            Format::Csv => to_csv_in(g, game, languages),
            f => {
                let en = game.for_language(Language::ENGLISH);
                CODEPAGE.spelled_in(&f.write(g, title), en).into_owned()
            }
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

    /// [`update`](Self::update) for a game whose codepage is `game`.
    pub fn update_in(
        self,
        g: &mut Gff,
        text: &str,
        game: Codepage,
    ) -> Option<Result<usize, String>> {
        match self {
            Format::Csv => Some(update_from_csv_in(g, text, game)),
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

/// The codepage a conversation's strings (speaker, quest, comment,
/// parameters) are read and written in, as its lines' text is
/// ([`crate::dialog::new_node`]).
const CODEPAGE: Codepage = Codepage::WINDOWS_1252;

fn string(s: &Struct, label: &str) -> String {
    CODEPAGE.decode(s.string(label).unwrap_or_default()).into_owned()
}

/// Text as a string field's value (a character the codepage lacks is a
/// "?").
fn string_value(s: &str) -> Value {
    Value::String(CODEPAGE.encode_lossy(s))
}

// ---------------------------------------------------------------- parameters

/// A script's parameters (`ActionParams`, `ConditionParams`): name and
/// value.
type Params = Vec<(String, String)>;

/// The parameters in a list.
fn params(s: &Struct, label: &str) -> Params {
    crate::dialog::params_in(s, label, CODEPAGE)
}

/// A parameter list, as Aurora writes it.
fn params_list(params: &Params) -> Value {
    crate::dialog::params_value_in(params, CODEPAGE)
}

/// What a parameter's name or value has percent-escaped (`%22`), in Ink
/// and in Twine: the characters either reads as syntax where a parameter
/// is written (a tag, a link, braces, a comment), `%` itself, the quote,
/// the backslash and line breaks.
const PARAM_ESCAPED: &str = "%\"\\#/{}[]|<>\n\r\t";

fn percent(s: &str, also: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if PARAM_ESCAPED.contains(c) || also.contains(c) {
            let _ = write!(out, "%{:02X}", c as u32);
        } else {
            out.push(c);
        }
    }
    out
}

/// Percent escapes (`%22`, of ASCII characters) undone; a `%` not
/// starting one is kept.
fn unpercent(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find('%') {
        out.push_str(&rest[..at]);
        let hex = rest.get(at + 1..at + 3).filter(|h| h.bytes().all(|b| b.is_ascii_hexdigit()));
        match hex.and_then(|h| u8::from_str_radix(h, 16).ok()).filter(u8::is_ascii) {
            Some(b) => {
                out.push(b as char);
                rest = &rest[at + 3..];
            }
            None => {
                out.push('%');
                rest = &rest[at + 1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// A parameter's name or value as written: escaped, and in quotes when
/// it is empty or holds a space.
fn param_word(s: &str, also: &str) -> String {
    let w = percent(s, also);
    if w.is_empty() || w.contains(char::is_whitespace) { format!("\"{w}\"") } else { w }
}

/// A script and its parameters as Ink's tags and Twine's braces hold
/// them: `a_give item="gold coins" count=5` (just the name without
/// parameters).
fn call_text(script: &str, params: &Params) -> String {
    let mut out = script.to_string();
    for (k, v) in params {
        let _ = write!(out, " {}={}", param_word(k, "="), param_word(v, ""));
    }
    out
}

/// A script and its parameters, as [`call_text`] writes them.
fn read_call(s: &str) -> Result<(String, Params), String> {
    let s = s.trim();
    let (script, rest) = s.split_once(char::is_whitespace).unwrap_or((s, ""));
    let mut params = Vec::new();
    let mut chars = rest.chars().peekable();
    loop {
        while chars.next_if(|c| c.is_whitespace()).is_some() {}
        if chars.peek().is_none() {
            break;
        }
        let (mut key, mut value, mut quoted) = (String::new(), None::<String>, false);
        loop {
            let q = quoted;
            let Some(c) = chars.next_if(move |c| q || !c.is_whitespace()) else { break };
            match c {
                '"' => quoted = !quoted,
                '=' if !quoted && value.is_none() => value = Some(String::new()),
                c => value.as_mut().unwrap_or(&mut key).push(c),
            }
        }
        if quoted {
            return Err(format!("{s}: a quote doesn't end"));
        }
        let value = value.ok_or(format!("{s}: the parameter {key:?} has no =value"))?;
        params.push((unpercent(&key), unpercent(&value)));
    }
    Ok((script.to_string(), params))
}

/// A script's parameters in a Twine passage's tag, which can't hold a
/// space: `item=gold%20coins&count=5`.
fn query_text(params: &Params) -> String {
    let word = |s: &str, also: &str| percent(s, also);
    params
        .iter()
        .map(|(k, v)| format!("{}={}", word(k, " &=?"), word(v, " &?")))
        .collect::<Vec<_>>()
        .join("&")
}

fn read_query(s: &str) -> Result<Params, String> {
    s.split('&')
        .filter(|p| !p.is_empty())
        .map(|p| {
            let (k, v) = p.split_once('=').ok_or(format!("{p}: a parameter has no =value"))?;
            Ok((unpercent(k), unpercent(v)))
        })
        .collect()
}

/// A link's condition and its parameters (`c_has item=gold`), if it has
/// one.
fn condition(l: &Struct) -> Option<String> {
    resref(l, "Active").map(|c| call_text(&c, &params(l, "ConditionParams")))
}

/// Sets the condition of the link at `pos` under a parent, as
/// [`condition`] writes it: the script and its parameters.
fn set_condition_call(g: &mut Gff, parent: Parent, pos: usize, call: &str) -> Result<(), String> {
    let (script, params) = read_call(call)?;
    set_condition(g, parent, pos, &script);
    if !params.is_empty()
        && let Some(l) = crate::dialog::links_mut(g, parent).and_then(|l| l.get_mut(pos))
    {
        l.set("ConditionParams", params_list(&params));
    }
    Ok(())
}

/// A tag's value with spaces and `%` escaped (`kid%202`), as Twine's tags
/// can't hold spaces.
fn escape(v: &str) -> String {
    v.replace('%', "%25").replace(' ', "%20")
}

fn unescape(v: &str) -> String {
    v.replace("%20", " ").replace("%25", "%")
}

/// An animation's name in a tag (`talk-forceful`): the Other Actions
/// tab's, lower case, spaces as `-`; one it doesn't name by its number.
fn animation_name(value: u32) -> String {
    match crate::dialog::ANIMATIONS.iter().find(|(_, v)| *v == value) {
        Some((label, _)) => label.to_ascii_lowercase().replace(' ', "-"),
        None => value.to_string(),
    }
}

/// The animation an `anim:` tag names (a name as [`animation_name`]
/// writes it, in any case, or a number).
fn read_animation(v: &str) -> Result<u32, String> {
    let v = v.trim();
    if let Ok(n) = v.parse() {
        return Ok(n);
    }
    let name = v.to_ascii_lowercase().replace(' ', "-");
    (crate::dialog::ANIMATIONS.iter())
        .find(|(label, _)| label.to_ascii_lowercase().replace(' ', "-") == name)
        .map(|(_, value)| *value)
        .ok_or(format!("anim:{v} isn't an animation of a conversation's lines (talk-normal, bow…)"))
}

/// A line's comment in a tag: percent-escaped as a parameter is, and a
/// space too in a Twine passage's tags (which can't hold one), or at
/// either end (which a tag's reader trims).
fn comment_text(comment: &str, passage_tags: bool) -> String {
    // (Of the whitespace, only the space is left unescaped.)
    let inner = percent(comment, if passage_tags { " " } else { "" });
    let core = inner.trim_matches(' ');
    if core.is_empty() {
        return "%20".repeat(inner.len());
    }
    let lead = inner.len() - inner.trim_start_matches(' ').len();
    let trail = inner.len() - inner.trim_end_matches(' ').len();
    format!("{}{core}{}", "%20".repeat(lead), "%20".repeat(trail))
}

/// A line's tags: speaker, action (with its parameters: `do:a_give
/// item="gold coins"`, or in a Twine passage's tags, which can't hold a
/// space, `do:a_give?item=gold%20coins`), journal update, sound,
/// animation (`anim:bow`), delay (`delay:500`, where it has one) and
/// comment (`comment:Asked about the sword`).
fn tags(kind: Kind, n: &Struct, passage_tags: bool) -> Vec<String> {
    let mut out = Vec::new();
    let speaker = string(n, "Speaker");
    if kind == Kind::Entry && !speaker.is_empty() {
        out.push(format!("speaker:{}", escape(&speaker)));
    }
    if let Some(a) = resref(n, "Script") {
        let p = params(n, "ActionParams");
        out.push(match passage_tags {
            _ if p.is_empty() => format!("do:{}", escape(&a)),
            true => format!("do:{a}?{}", query_text(&p)),
            false => format!("do:{}", call_text(&a, &p)),
        });
    }
    let quest = string(n, "Quest");
    if !quest.is_empty() {
        out.push(format!("journal:{}:{}", escape(&quest), n.integer("QuestEntry").unwrap_or(0)));
    }
    if let Some(s) = resref(n, "Sound") {
        out.push(format!("sound:{}", escape(&s)));
    }
    if let Some(a) = n.integer("Animation").filter(|a| *a != 0) {
        out.push(format!("anim:{}", animation_name(a as u32)));
    }
    // (None: the game's own, `0xFFFFFFFF`, which new lines have.)
    if let Some(d) = n.integer("Delay").filter(|d| *d != i64::from(u32::MAX)) {
        out.push(format!("delay:{d}"));
    }
    let comment = string(n, "Comment");
    if !comment.is_empty() {
        out.push(format!("comment:{}", comment_text(&comment, passage_tags)));
    }
    out
}

/// Applies tags (`speaker:`, `do:`, `journal:`, `sound:`, `anim:`,
/// `delay:`, `comment:`) to a line; others are ignored.
fn apply_tags(n: &mut Struct, tags: &[String]) -> Result<(), String> {
    let r = |v: &str| ResRef::from_str(v).map(Value::resref).map_err(|e| format!("{v}: {e}"));
    for t in tags {
        let (k, raw) = t.split_once(':').unwrap_or((t, ""));
        let v = unescape(raw.trim());
        match k.trim() {
            "speaker" => n.set("Speaker", string_value(&v)),
            "do" => {
                // `a_give?item=gold%20coins` (a Twine passage's tag) or
                // `a_give item="gold coins"`.
                let query =
                    raw.trim().split_once('?').filter(|(s, _)| !s.contains(char::is_whitespace));
                let (script, params) = match query {
                    Some((s, q)) => (unescape(s), read_query(q)?),
                    None => read_call(raw).map(|(s, p)| (unescape(&s), p))?,
                };
                n.set("Script", r(&script)?);
                if !params.is_empty() {
                    n.set("ActionParams", params_list(&params));
                }
            }
            "sound" => n.set("Sound", r(&v)?),
            "journal" => {
                let (q, e) = v.rsplit_once(':').unwrap_or((&v, "0"));
                n.set("Quest", string_value(q));
                n.set("QuestEntry", Value::Dword(e.trim().parse().unwrap_or(0)));
            }
            "anim" => n.set("Animation", Value::Dword(read_animation(&v)?)),
            "delay" => {
                let d = v.trim().parse().map_err(|_| format!("delay:{v} isn't a number"))?;
                n.set("Delay", Value::Dword(d));
            }
            "comment" => n.set("Comment", string_value(&unpercent(raw.trim()))),
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
                condition(l).unwrap_or_default(),
                tags(kind, n, false).join(" "),
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
            if let Some(c) = condition(l) {
                notes.push(format!("if {c}"));
            }
            notes.extend(tags(kind, n, false).into_iter().filter(|t| !t.starts_with("speaker:")));
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

/// The column of a variant of a line's text other than English's (`text`):
/// `text_fr`, `text_de`..., a feminine one `text_fr_f` (`text_en_f`
/// English's); a language with no two-letter code by its number
/// (`text_128`).
pub fn csv_text_column(key: LocStringKey) -> String {
    let l = key.language();
    let code = l.short_code().map_or_else(|| l.0.to_string(), str::to_string);
    match key.gender() {
        Gender::Male => format!("text_{code}"),
        Gender::Female => format!("text_{code}_f"),
    }
}

/// The variant a [`csv_text_column`] header names (`TEXT_FR` too), not
/// English's male `text`.
fn csv_text_key(header: &str) -> Option<LocStringKey> {
    let header = header.trim().to_ascii_lowercase();
    let rest = header.strip_prefix("text_")?;
    let (code, gender) = match rest.strip_suffix("_f") {
        Some(c) => (c, Gender::Female),
        None => (rest, Gender::Male),
    };
    let language = Language::from_short_code(code).or_else(|| code.parse().ok().map(Language))?;
    let key = LocStringKey::new(language, gender);
    (key != LocStringKey::new(Language::ENGLISH, Gender::Male)).then_some(key)
}

/// Every line as a CSV row: `id` (`E3` an NPC line, `R5` a reply),
/// speaker, text, condition and action (each with its parameters, as in
/// Twine and Ink: `c_has item=gold`), the line it follows, comment; then
/// a column for each other language (and feminine text) a line has text
/// in ([`csv_text_column`]). Only `speaker`, the texts and `comment` are
/// read back ([`update_from_csv`]).
pub fn to_csv(g: &Gff) -> String {
    to_csv_in(g, Codepage::WINDOWS_1252, &[])
}

/// [`to_csv`] as a game whose codepage is `game` reads the text (a
/// module's `encoding.2da` table, else Windows-1252), with a column for
/// each of `languages` too, though no line has text in it yet (the
/// languages it is to be translated into).
pub fn to_csv_in(g: &Gff, game: Codepage, languages: &[Language]) -> String {
    let english = LocStringKey::new(Language::ENGLISH, Gender::Male);
    let mut keys: Vec<LocStringKey> =
        languages.iter().map(|l| LocStringKey::new(*l, Gender::Male)).collect();
    for kind in [Kind::Entry, Kind::Reply] {
        for n in crate::dialog::nodes(g, kind) {
            let variants = n.locstring("Text").into_iter().flat_map(|t| &t.strings);
            keys.extend(variants.map(|(k, _)| *k));
        }
    }
    keys.retain(|k| *k != english);
    keys.sort();
    keys.dedup();
    let mut header: Vec<String> = CSV_HEADER.iter().map(|h| h.to_string()).collect();
    header.extend(keys.iter().map(|k| csv_text_column(*k)));
    let mut out = header.join(",") + "\r\n";
    let mut seen = std::collections::HashSet::new();
    struct Ctx<'a> {
        g: &'a Gff,
        game: Codepage,
        keys: &'a [LocStringKey],
    }
    fn walk(
        c: &Ctx<'_>,
        parent: Parent,
        seen: &mut std::collections::HashSet<(Kind, u32)>,
        out: &mut String,
    ) {
        let kind = parent.child_kind();
        let en = c.game.for_language(Language::ENGLISH);
        let plain =
            |s: &Struct, label: &str| en.decode(s.string(label).unwrap_or_default()).into_owned();
        let call = |s: &Struct, script: &str, params: &str| {
            let params = crate::dialog::params_in(s, params, en);
            resref(s, script).map(|r| call_text(&r, &params)).unwrap_or_default()
        };
        for l in links(c.g, parent) {
            let i = link_index(l);
            if is_link(l) || !seen.insert((kind, i)) {
                continue;
            }
            let Some(n) = node(c.g, kind, i) else { continue };
            let from = match parent {
                Parent::Root => "start".to_string(),
                Parent::Node(k, p) => passage(k, p),
            };
            let variant = |k: LocStringKey| {
                n.locstring("Text")
                    .and_then(|t| t.text_in(k.language(), k.gender(), c.game))
                    .map(|t| t.into_owned())
                    .unwrap_or_default()
            };
            let mut row = vec![
                passage(kind, i),
                plain(n, "Speaker"),
                variant(LocStringKey::new(Language::ENGLISH, Gender::Male)),
                // The scripts with their parameters, as in Twine and Ink.
                call(l, "Active", "ConditionParams"),
                call(n, "Script", "ActionParams"),
                from,
                plain(n, "Comment"),
            ];
            row.extend(c.keys.iter().map(|k| variant(*k)));
            out.push_str(&row.iter().map(|f| csv_field(f)).collect::<Vec<_>>().join(","));
            out.push_str("\r\n");
            walk(c, Parent::Node(kind, i), seen, out);
        }
    }
    walk(&Ctx { g, game, keys: &keys }, Parent::Root, &mut seen, &mut out);
    out
}

/// Reads back the speaker, text and comment of the lines a CSV names by
/// `id` (columns found by their header; others ignored), and the text of
/// each other language a `text_fr`... column has ([`csv_text_column`]):
/// a cell left empty there takes that language's text away. How many
/// lines changed; an unknown id or a missing `id` column is an error.
pub fn update_from_csv(g: &mut Gff, csv: &str) -> Result<usize, String> {
    update_from_csv_in(g, csv, Codepage::WINDOWS_1252)
}

/// [`update_from_csv`] for a game whose codepage is `game`.
pub fn update_from_csv_in(g: &mut Gff, csv: &str, game: Codepage) -> Result<usize, String> {
    let rows = read_csv(csv)?;
    let (header, rows) = rows.split_first().ok_or("the file is empty")?;
    let col = |name: &str| header.iter().position(|h| h.trim().eq_ignore_ascii_case(name));
    let id = col("id").ok_or("no id column")?;
    let (speaker, text_col, comment) = (col("speaker"), col("text"), col("comment"));
    let others: Vec<(usize, LocStringKey)> =
        header.iter().enumerate().filter_map(|(i, h)| csv_text_key(h).map(|k| (i, k))).collect();
    let en = game.for_language(Language::ENGLISH);
    let plain =
        |s: &Struct, label: &str| en.decode(s.string(label).unwrap_or_default()).into_owned();
    let shown = |ls: &LocString, l: Language, gender: Gender| {
        ls.text_in(l, gender, game).map(|t| t.into_owned())
    };
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
        let mut ls = n.locstring("Text").cloned().unwrap_or_default();
        if let Some(t) = text_col.and_then(|c| row.get(c))
            && shown(&ls, Language::ENGLISH, Gender::Male).unwrap_or_default() != *t
        {
            ls.set_text_in(Language::ENGLISH, Gender::Male, t, game).ok_or(format!(
                "line {line}: {name}'s text can't be written in English's codepage"
            ))?;
        }
        for (c, key) in &others {
            let Some(t) = row.get(*c) else { continue };
            let (l, gender) = (key.language(), key.gender());
            match shown(&ls, l, gender) {
                Some(old) if old == *t => {}
                None if t.is_empty() => {}
                Some(_) if t.is_empty() => {
                    ls.remove(l, gender);
                }
                _ => {
                    let lang = l.name().map_or_else(|| l.0.to_string(), str::to_string);
                    ls.set_text_in(l, gender, t, game).ok_or(format!(
                        "line {line}: {name}'s {lang} text can't be written in its codepage"
                    ))?;
                }
            }
        }
        if n.locstring("Text") != Some(&ls) && !(ls.is_empty() && n.locstring("Text").is_none()) {
            n.set("Text", Value::LocString(ls));
        }
        if kind == Kind::Entry
            && let Some(s) = speaker.and_then(|c| row.get(c)).filter(|s| **s != plain(n, "Speaker"))
        {
            n.set("Speaker", Value::String(en.encode_lossy(s)));
        }
        if let Some(c) = comment.and_then(|c| row.get(c)).filter(|c| **c != plain(n, "Comment")) {
            n.set("Comment", Value::String(en.encode_lossy(c)));
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
            let cond = condition(l).map_or("otherwise".to_string(), |c| format!("if {c}"));
            let _ = writeln!(out, "[[{cond}->{}]]", passage(Kind::Entry, link_index(l)));
        }
        out.push('\n');
    };
    if start == "Start" {
        choice(&mut out, "Start", Parent::Root);
    }
    for (i, n) in crate::dialog::nodes(g, Kind::Entry).iter().enumerate() {
        let i = i as u32;
        let tags = tags(Kind::Entry, n, true);
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
            if let Some(c) = condition(l) {
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
    tags(Kind::Reply, n, false).into_iter().map(|t| t.replacen(':', " ", 1)).collect()
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

fn parse_twee(source: &str) -> Story {
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
    (passages, start, order)
}

/// A reply's label as read: its text and the braces after it (`{if c}`,
/// `{do a}`, `{journal q:10}`, `{sound s}`, `{anim bow}`, `{delay 500}`,
/// `{comment text}`).
fn reply_label(label: &str) -> (String, Option<String>, Vec<String>) {
    let mut text = label.to_string();
    let mut cond = None;
    let mut tags = Vec::new();
    while let Some(open) = text.rfind('{') {
        let Some(close) = text[open..].find('}') else { break };
        let inner = text[open + 1..open + close].trim().to_string();
        match inner.split_once(' ') {
            Some(("if", c)) => cond = Some(c.trim().to_string()),
            Some((k @ ("do" | "journal" | "sound" | "anim" | "delay" | "comment"), v)) => {
                tags.push(format!("{k}:{}", v.trim()))
            }
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
                    set_condition_call(&mut self.g, at, last, c)?;
                }
                if let Some(to) = to {
                    self.place(Parent::Node(Kind::Reply, r), to, None, depth + 1)?;
                }
            }
        }
        if let Some(c) = cond {
            set_condition_call(&mut self.g, parent, pos, c)?;
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
    let (passages, start, order) = parse_twee(source);
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
                    // The condition's parameters: a tag on the line
                    // before.
                    let p = params(l, "ConditionParams");
                    if !p.is_empty() {
                        let _ = writeln!(out, "# if:{}", call_text(&c, &p));
                    }
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
        for t in tags(Kind::Entry, n, false) {
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
            // The condition's parameters are a tag, `if:` and the
            // condition with them.
            let cond_params = condition(l).filter(|_| !params(l, "ConditionParams").is_empty());
            let tags: String = (cond_params.map(|c| format!("if:{c}")).into_iter())
                .chain(tags(Kind::Reply, reply, false))
                .map(|t| format!(" # {t}"))
                .collect();
            // (Tags before the divert: Inky refuses a tag after it.)
            let _ = writeln!(out, "+ {cond}[{t}]{tags} -> {target}");
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

/// Where an `# if:` tag goes, on its own line.
const IF_TAG_PLACE: &str =
    "an # if: tag on its own line goes just before the {condition: -> KNOT} it is for";

/// A condition as an `# if:` tag gives it (`c_x key=value`), which must
/// name the condition it is for (`{c_x}`).
fn with_params(at: usize, cond: Option<&str>, call: String) -> Result<String, String> {
    let script = call.split_whitespace().next().unwrap_or_default();
    match cond {
        Some(c) if c == script => Ok(call),
        Some(c) => Err(format!("line {at}: # if:{script} isn't the condition there, {{{c}}}")),
        None => Err(format!("line {at}: # if:{script} has no {{{script}}} to give parameters to")),
    }
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
    // A conditional divert's `# if:` tag, on the line before it.
    let mut pending: Option<(usize, String)> = None;
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
        let pending_if = pending.take();
        if let Some((from, _)) = &pending_if
            && !line.starts_with('{')
        {
            return Err(format!("line {from}: {IF_TAG_PLACE}"));
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
            match tag.trim().strip_prefix("if:") {
                Some(call) => pending = Some((at, call.trim().to_string())),
                None => k.tags.push(tag.trim().to_string()),
            }
        } else if let Some(choice) = line.strip_prefix('+').or_else(|| line.strip_prefix('*')) {
            // + {cond} [text] # tag -> target (and, as Moonglow wrote it
            // before 1.22.1, + {cond} [text] -> target # tag)
            let named = |tags: &str| tags.split('#').map(|t| t.trim().to_string()).collect();
            let joined;
            let (choice, tags): (&str, Vec<String>) = match ink_find(choice, '#') {
                Some(at) => match choice[at..].rfind("->") {
                    Some(arrow) => {
                        joined = format!("{} {}", &choice[..at], &choice[at + arrow..]);
                        (joined.as_str(), named(&choice[at + 1..at + arrow]))
                    }
                    None => (&choice[..at], named(&choice[at + 1..])),
                },
                None => (choice, Vec::new()),
            };
            let mut rest = choice.trim();
            let mut cond = None;
            if let Some(r) = rest.strip_prefix('{') {
                let end = r.find('}').ok_or(format!("line {at}: a condition doesn't end"))?;
                cond = Some(r[..end].trim().to_string());
                rest = r[end + 1..].trim();
            }
            // `# if:c_x key=value`: the condition's parameters.
            let mut tags = tags;
            if let Some(i) = tags.iter().position(|t| t.starts_with("if:")) {
                let call = tags.remove(i)["if:".len()..].trim().to_string();
                if tags.iter().any(|t| t.starts_with("if:")) {
                    return Err(format!("line {at}: a choice has one condition, one # if:"));
                }
                cond = Some(with_params(at, cond.as_deref(), call)?);
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
            let cond = match pending_if {
                Some((from, call)) => with_params(from, Some(cond), call)?,
                None => cond.to_string(),
            };
            k.choice.get_or_insert_with(Vec::new).push((Some(cond), target.to_string()));
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
    if let Some((from, _)) = pending {
        return Err(format!("line {from}: {IF_TAG_PLACE}"));
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

    /// A choice's tags are written before its divert (Inky refuses them
    /// after it), and read in either place.
    #[test]
    fn an_ink_choice_s_tags_come_before_its_divert() {
        let ink = to_ink(&sample());
        for line in ink.lines().filter(|l| l.starts_with('+')) {
            let (arrow, tag) = (line.rfind("->").unwrap(), line.find('#'));
            assert!(tag.is_none_or(|t| t < arrow), "{line}");
        }
        assert!(ink.lines().any(|l| l.starts_with('+') && l.contains(" # do:a_bye -> ")), "{ink}");
        let new = "-> a\n=== a ===\nHi\n+ {c} [Go] # do:a_x # journal:q:1 -> b\n\n=== b ===\nB\n";
        let old = "-> a\n=== a ===\nHi\n+ {c} [Go] -> b # do:a_x # journal:q:1\n\n=== b ===\nB\n";
        let (new, old) = (from_ink(new).unwrap(), from_ink(old).unwrap());
        assert_eq!(to_ink(&new), to_ink(&old));
        assert!(
            to_ink(&new).contains("+ {c} [Go] # do:a_x # journal:q:1 -> E1"),
            "{}",
            to_ink(&new)
        );
    }

    fn pairs(p: &[(&str, &str)]) -> Params {
        p.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    /// Condition and action parameters go out to Ink and Twine and come
    /// back: a starting link's (a knot of conditional diverts, a passage
    /// tagged `npc-choice`), a reply's condition and action, an NPC line's
    /// action; names and values with spaces, quotes, what Ink and Twine
    /// read as syntax, `%`, a letter outside ASCII and nothing.
    #[test]
    fn script_parameters_survive_ink_and_twine() {
        let mut g = sample();
        let odd = pairs(&[
            ("item", "gold coins"),
            ("say", "\"Hi\" #1 // a->b {x} [y] |z| \\ 100%20"),
            ("who", "Zoë"),
            ("empty", ""),
            ("a b", "c=d?e&f"),
        ]);
        let set = |g: &mut Gff, parent: Parent, pos: usize, p: &Params| {
            crate::dialog::links_mut(g, parent).unwrap()[pos]
                .set("ConditionParams", params_list(p));
        };
        // E0 (c_seen) at the start, R0 (c_curious) under E1.
        set(&mut g, Parent::Root, 0, &pairs(&[("times", "2")]));
        set(&mut g, Parent::Node(Kind::Entry, 1), 0, &odd);
        node_mut(&mut g, Kind::Entry, 1).unwrap().set("ActionParams", params_list(&odd));
        let cold = pairs(&[("mood", "cold")]);
        node_mut(&mut g, Kind::Reply, 1).unwrap().set("ActionParams", params_list(&cold));
        // The game's codepage, as the lines' text.
        let zoe = &node(&g, Kind::Entry, 1).unwrap().list("ActionParams").unwrap()[2];
        assert_eq!(zoe.string("Value"), Some(&b"Zo\xeb"[..]));

        let want = play_outline(&g, 10_000).unwrap();
        assert!(want.iter().any(|l| l.contains("|if c_seen times=2|")), "{want:?}");
        let ink = to_ink(&g);
        assert!(ink.contains("=== start ===\n# if:c_seen times=2\n{c_seen: -> E0}\n"), "{ink}");
        assert!(ink.contains("# do:a_bye mood=cold -> END"), "{ink}");
        let tag = "a_hello item=\"gold coins\" \
            say=\"%22Hi%22 %231 %2F%2F a-%3Eb %7Bx%7D %5By%5D %7Cz%7C %5C 100%2520\" \
            who=Zoë empty=\"\" \"a b\"=c=d?e&f";
        assert!(ink.contains(&format!("\n# do:{tag}\n")), "{ink}");
        assert!(ink.contains("+ {c_curious} [Who are you?] # if:c_curious item="), "{ink}");
        let twee = to_twee(&g, "keeper");
        assert!(twee.contains("[[if c_seen times=2->E0]]"), "{twee}");
        assert!(
            twee.contains(":: E1 [speaker:KEEPER do:a_hello?item=gold%20coins&say=%22Hi%22%20"),
            "{twee}"
        );
        assert!(twee.contains("&a%20b=c=d%3Fe%26f]"), "{twee}");
        assert!(twee.contains("{do a_bye mood=cold}->END]]"), "{twee}");

        for (format, back) in [("Ink", from_ink(&ink)), ("Twine", from_twee(&twee))] {
            let back = back.unwrap_or_else(|e| panic!("{format}: {e}"));
            assert_eq!(play_outline(&back, 10_000).unwrap(), want, "{format}");
            let start = links(&back, Parent::Root);
            assert_eq!(params(&start[0], "ConditionParams"), pairs(&[("times", "2")]));
            let hello = link_index(&start[1]);
            assert_eq!(params(node(&back, Kind::Entry, hello).unwrap(), "ActionParams"), odd);
            let replies = links(&back, Parent::Node(Kind::Entry, hello));
            assert_eq!(params(&replies[0], "ConditionParams"), odd, "{format}");
            let bye = node(&back, Kind::Reply, link_index(&replies[1])).unwrap();
            assert_eq!(params(bye, "ActionParams"), cold, "{format}");
            let zoe = &node(&back, Kind::Entry, hello).unwrap().list("ActionParams").unwrap()[2];
            assert_eq!(zoe.string("Value"), Some(&b"Zo\xeb"[..]), "{format}");
        }
    }

    /// A line's animation, delay and comment go out to Ink and Twine as
    /// tags and come back: an NPC line's (a passage's tags, which can't
    /// hold a space; a knot's own lines) and a reply's (Twine's braces, a
    /// choice's tags); an animation the Other Actions tab doesn't name by
    /// its number; a comment with what Ink and Twine read as syntax and
    /// spaces at its ends. A line with the defaults has none of them.
    #[test]
    fn animations_comments_and_delays_survive_ink_and_twine() {
        let mut g = sample();
        let comment = " Asked {twice} about the sword #2 // keep -> E3 ";
        let hello = node_mut(&mut g, Kind::Entry, 1).unwrap();
        hello.set("Animation", Value::Dword(40));
        hello.set("Delay", Value::Dword(500));
        hello.set("Comment", string_value(comment));
        let bye = node_mut(&mut g, Kind::Reply, 1).unwrap();
        bye.set("Animation", Value::Dword(123));
        bye.set("Delay", Value::Dword(0));
        bye.set("Comment", string_value("Zoë's goodbye"));
        let want = play_outline(&g, 10_000).unwrap();
        assert!(
            want.iter().any(|l| l.contains("anim:talk-forceful delay:500 comment:")),
            "{want:?}"
        );

        let ink = to_ink(&g);
        assert!(ink.contains("\n# anim:talk-forceful\n# delay:500\n# comment:%20Asked %7Btwice%7D about the sword %232 %2F%2F keep -%3E E3%20\n"), "{ink}");
        assert!(ink.contains("# anim:123 # delay:0 # comment:Zoë's goodbye -> END"), "{ink}");
        let twee = to_twee(&g, "keeper");
        assert!(
            twee.contains(" anim:talk-forceful delay:500 comment:%20Asked%20%7Btwice%7D%20about"),
            "{twee}"
        );
        assert!(twee.contains("{anim 123} {delay 0} {comment Zoë's goodbye}->END]]"), "{twee}");
        // (The defaults: no tag, on the other lines.)
        assert!(ink.matches("anim:").count() == 2 && ink.matches("delay:").count() == 2, "{ink}");

        for (format, back) in [("Ink", from_ink(&ink)), ("Twine", from_twee(&twee))] {
            let back = back.unwrap_or_else(|e| panic!("{format}: {e}"));
            assert_eq!(play_outline(&back, 10_000).unwrap(), want, "{format}");
            let start = links(&back, Parent::Root);
            let hello = node(&back, Kind::Entry, link_index(&start[1])).unwrap();
            assert_eq!(hello.integer("Animation"), Some(40), "{format}");
            assert_eq!(hello.integer("Delay"), Some(500), "{format}");
            assert_eq!(string(hello, "Comment"), comment, "{format}");
            let replies = links(&back, Parent::Node(Kind::Entry, link_index(&start[1])));
            let bye = node(&back, Kind::Reply, link_index(&replies[1])).unwrap();
            assert_eq!((bye.integer("Animation"), bye.integer("Delay")), (Some(123), Some(0)));
            assert_eq!(bye.string("Comment"), Some(&b"Zo\xeb's goodbye"[..]), "{format}");
            // (Those without: as a new line has them.)
            let who = node(&back, Kind::Reply, link_index(&replies[0])).unwrap();
            assert_eq!(
                (who.integer("Animation"), who.integer("Delay")),
                (Some(0), Some(u32::MAX.into()))
            );
        }
        // An animation by its label, in any case; one there isn't.
        let mut n = crate::dialog::new_node(Kind::Entry, "");
        apply_tags(&mut n, &["anim:Talk Laughing".to_string()]).unwrap();
        assert_eq!(n.integer("Animation"), Some(41));
        let e = apply_tags(&mut n, &["anim:dance".to_string()]).unwrap_err();
        assert!(e.contains("anim:dance"), "{e}");
        assert!(apply_tags(&mut n, &["delay:soon".to_string()]).is_err());
    }

    /// The CSV's condition and action name their scripts' parameters too.
    #[test]
    fn csv_scripts_carry_their_parameters() {
        let mut g = sample();
        crate::dialog::links_mut(&mut g, Parent::Node(Kind::Entry, 1)).unwrap()[0]
            .set("ConditionParams", params_list(&pairs(&[("item", "gold coins")])));
        node_mut(&mut g, Kind::Reply, 1)
            .unwrap()
            .set("ActionParams", params_list(&pairs(&[("mood", "cold")])));
        let rows = read_csv(&to_csv(&g)).unwrap();
        let row = |id: &str| rows.iter().find(|r| r[0] == id).unwrap().clone();
        assert_eq!(row("R0")[3], "c_curious item=\"gold coins\"");
        assert_eq!(row("R1")[4], "a_bye mood=cold");
        assert_eq!(row("E1")[4], "a_hello", "no parameters: the script alone");
    }

    /// An `# if:` tag names the condition it gives parameters to, and one
    /// on its own line goes just before its conditional divert.
    #[test]
    fn ink_condition_parameters_go_with_their_condition() {
        let ok = "-> a\n=== a ===\nHi\n+ {c_x} [Go] # if:c_x n=1 # do:a_y k=\"v w\" -> END\n";
        let g = from_ink(ok).unwrap();
        let l = &links(&g, Parent::Node(Kind::Entry, 0))[0];
        assert_eq!(params(l, "ConditionParams"), pairs(&[("n", "1")]));
        let r = node(&g, Kind::Reply, link_index(l)).unwrap();
        assert_eq!(params(r, "ActionParams"), pairs(&[("k", "v w")]));
        for (bad, line) in [
            ("-> a\n=== a ===\nHi\n+ {c_x} [Go] # if:c_z n=1 -> END\n", "line 4:"),
            ("-> a\n=== a ===\nHi\n+ [Go] # if:c_z n=1 -> END\n", "line 4:"),
            ("-> a\n=== a ===\nHi\n+ {c_x} [Go] # if:c_x n=1 # if:c_x n=2 -> END\n", "line 4:"),
            ("-> s\n=== s ===\n# if:c_x n=1\n-> a\n=== a ===\nHi\n", "line 3:"),
            ("-> s\n=== s ===\n# if:c_x n=1\n", "line 3:"),
            ("-> a\n=== a ===\nHi\n+ {c_x} [Go] # if:c_x n=\"1 -> END\n", "doesn't end"),
        ] {
            let e = from_ink(bad).unwrap_err();
            assert!(e.contains(line), "{bad}: {e}");
        }
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

    /// A CSV carries a column for each language a line has text in, and
    /// those asked for, for a translator; they are read back.
    #[test]
    fn csv_carries_every_language() {
        let mut g = sample();
        let hello = node_mut(&mut g, Kind::Entry, 1).unwrap();
        let mut ls = hello.locstring("Text").cloned().unwrap();
        ls.set(Language::GERMAN, Gender::Male, b"Hallo, Fremder.".to_vec());
        ls.set(Language::GERMAN, Gender::Female, b"Hallo, Fremde.".to_vec());
        hello.set("Text", Value::LocString(ls));
        let csv = to_csv(&g);
        let header = csv.lines().next().unwrap();
        assert_eq!(header, "id,speaker,text,condition,action,parent,comment,text_de,text_de_f");
        assert!(csv.contains(",\"Hallo, Fremder.\",\"Hallo, Fremde.\"\r\n"), "{csv}");
        // Asked for, French has a column though no line has text in it;
        // Polish is read in its own codepage.
        let csv = to_csv_in(&g, Codepage::WINDOWS_1252, &[Language::FRENCH, Language::POLISH]);
        assert!(
            csv.starts_with("id,speaker,text,condition,action,parent,comment,text_fr,text_de,")
        );
        assert_eq!(update_from_csv(&mut g, &csv), Ok(0), "untouched");
        let rows = read_csv(&csv).unwrap();
        let col = |h: &str| rows[0].iter().position(|c| c == h).unwrap();
        let (fr, pl, de_f) = (col("text_fr"), col("text_pl"), col("text_de_f"));
        let mut edited = rows.clone();
        let hello_row = edited.iter_mut().find(|r| r[0] == "E1").unwrap();
        hello_row[fr] = "Bonjour, étranger.".into();
        hello_row[pl] = "Witaj, przybyszu. Łódź".into();
        hello_row[de_f] = String::new();
        let back: String = (edited.iter())
            .map(|r| r.iter().map(|f| csv_field(f)).collect::<Vec<_>>().join(",") + "\r\n")
            .collect();
        assert_eq!(update_from_csv(&mut g, &back), Ok(1));
        let t = node(&g, Kind::Entry, 1).unwrap().locstring("Text").unwrap();
        assert_eq!(t.text(Language::FRENCH, Gender::Male).unwrap(), "Bonjour, étranger.");
        assert_eq!(
            t.get(Language::POLISH, Gender::Male).unwrap(),
            b"Witaj, przybyszu. \xa3\xf3d\x9f"
        );
        assert_eq!(t.text(Language::GERMAN, Gender::Male).unwrap(), "Hallo, Fremder.");
        assert_eq!(t.get(Language::GERMAN, Gender::Female), None, "an emptied cell");
        assert_eq!(text(node(&g, Kind::Entry, 1).unwrap()), "Hello, stranger.");
        // Headers are named in any case; one that names no language is
        // another column, ignored.
        assert_eq!(
            csv_text_key("TEXT_FR"),
            Some(LocStringKey::new(Language::FRENCH, Gender::Male))
        );
        assert_eq!(csv_text_key("text_en_f").map(|k| k.gender()), Some(Gender::Female));
        assert_eq!(csv_text_key("text_en"), None);
        assert_eq!(csv_text_key("text_xx"), None);
        assert_eq!(csv_text_column(LocStringKey::new(Language::KOREAN, Gender::Male)), "text_128");
        assert_eq!(csv_text_key("text_128").map(|k| k.language()), Some(Language::KOREAN));
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
