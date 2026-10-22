//! The handlers of a window and its event script.
//!
//! A handler ([`Route`]) is an event of the window, of a control or of a
//! bind, and the code it starts with ([`Action`]). The event script
//! (`NAME_e`) has a section for each, between a `// mg:begin KEY` line and
//! the next `// mg:end`: [`merge_events`] writes the script around what its
//! sections hold, so code written in them is kept from Build to Build, and
//! [`edited_outside`] tells when something was written where it would not
//! be.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::{NuiError, Settings, script};

/// The events the client sends (`NuiGetEventType`), as the script reads
/// them.
pub const EVENT_TYPES: &[&str] = &[
    "click",
    "watch",
    "open",
    "close",
    "mousedown",
    "mouseup",
    "mousescroll",
    "focus",
    "blur",
    "range",
];

/// A handler: an event, whose it is, and the code it starts with. (An
/// entry of `actions` in the settings; the name is from before the Creator
/// called them handlers.)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Map<String, Value>", into = "Map<String, Value>")]
pub struct Route {
    /// One of [`EVENT_TYPES`].
    pub event: String,
    /// The control's ID; for `watch`, the bind's name; nothing the script
    /// reads for `open` and `close`.
    pub element: String,
    /// The code its section starts with.
    pub action: Action,
    /// Fields this version does not know, kept as read.
    pub extra: BTreeMap<String, Value>,
}

impl Route {
    /// A handler with no fields but its own.
    pub fn new(event: impl Into<String>, element: impl Into<String>, action: Action) -> Route {
        Route { event: event.into(), element: element.into(), action, extra: BTreeMap::new() }
    }
}

/// What a handler's section starts with (`action` in the settings, with
/// the fields of each beside it). After that it is code to change.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// A note where your own code goes.
    Code,
    /// Closes the window.
    Close,
    /// Switches a bind between true and false (in a list, its row's).
    Toggle { bind: String },
    /// Sets a bind to a value.
    Set { bind: String, value: Value },
    /// Shows a swap layout variant (`view`) in a group (`_window_`: in
    /// place of the whole window).
    View { group: String, view: String },
    /// An action this version does not know, as its `action` field reads:
    /// kept as it is (its other fields are the handler's `extra`), and
    /// reported by the checks.
    Other(Value),
}

impl TryFrom<Map<String, Value>> for Route {
    type Error = String;

    fn try_from(mut m: Map<String, Value>) -> Result<Route, String> {
        fn text(m: &mut Map<String, Value>, field: &str) -> Result<String, String> {
            match m.remove(field) {
                Some(Value::String(s)) => Ok(s),
                Some(_) => Err(format!("a handler's {field} is not text")),
                None => Err(format!("a handler has no {field}")),
            }
        }
        let event = text(&mut m, "event")?;
        let element = text(&mut m, "element")?;
        let action = match m.remove("action").ok_or("a handler has no action")? {
            Value::String(tag) => match tag.as_str() {
                "code" => Action::Code,
                "close" => Action::Close,
                "toggle" => Action::Toggle { bind: text(&mut m, "bind")? },
                "set" => Action::Set {
                    bind: text(&mut m, "bind")?,
                    value: m.remove("value").ok_or("a handler that sets a bind has no value")?,
                },
                "view" => {
                    Action::View { group: text(&mut m, "group")?, view: text(&mut m, "view")? }
                }
                _ => Action::Other(Value::String(tag)),
            },
            other => Action::Other(other),
        };
        Ok(Route { event, element, action, extra: m.into_iter().collect() })
    }
}

impl From<Route> for Map<String, Value> {
    fn from(route: Route) -> Map<String, Value> {
        let mut m = Map::new();
        m.insert("event".into(), route.event.into());
        m.insert("element".into(), route.element.into());
        let tag = |m: &mut Map<String, Value>, tag: &str| m.insert("action".into(), tag.into());
        match route.action {
            Action::Code => _ = tag(&mut m, "code"),
            Action::Close => _ = tag(&mut m, "close"),
            Action::Toggle { bind } => {
                tag(&mut m, "toggle");
                m.insert("bind".into(), bind.into());
            }
            Action::Set { bind, value } => {
                tag(&mut m, "set");
                m.insert("bind".into(), bind.into());
                m.insert("value".into(), value);
            }
            Action::View { group, view } => {
                tag(&mut m, "view");
                m.insert("group".into(), group.into());
                m.insert("view".into(), view.into());
            }
            Action::Other(raw) => _ = m.insert("action".into(), raw),
        }
        // (A field of the handler's own is never written twice.)
        for (k, v) in route.extra {
            m.entry(k).or_insert(v);
        }
        m
    }
}

/// A string as NWScript source: a plain literal when it is printable ASCII
/// (as scripts are written by hand), else JSON Unicode escapes, which don't
/// depend on the source's codepage.
pub fn string_expr(s: &str) -> String {
    if plain_literal(s) {
        return format!("\"{s}\"");
    }
    format!("JsonGetString(JsonParse({}))", script::literal(&script::ascii_json(&json!(s))))
}

fn plain_literal(s: &str) -> bool {
    s.bytes().all(|b| (b' '..=b'~').contains(&b) && b != b'"' && b != b'\\')
}

/// The line a section of the event script starts with, before its key.
pub const BEGIN: &str = "// mg:begin ";
/// The line a section ends with.
pub const END: &str = "// mg:end";
/// A handler no longer configured keeps its code, commented out, after main.
const REMOVED: &str = "// mg:removed ";
const STUB: &str = "        // Your code here.\n";

/// The key of a handler's section in the event script: its event, and
/// for any but the window's own, whose it is. A key is ASCII whatever the
/// names are ([`key_text`]), so it reads the same in any codepage the
/// script is saved in.
pub fn handler_key(handler: &Route) -> String {
    if matches!(handler.event.as_str(), "open" | "close") {
        handler.event.clone()
    } else {
        format!("{} {}", key_text(&handler.event), key_text(&handler.element))
    }
}

/// A name as a section's key carries it: printable ASCII as it is, a
/// backslash doubled, anything else as `\uXXXX` (UTF-16, as JSON spells
/// it).
fn key_text(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for c in name.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            ' '..='~' => out.push(c),
            _ => {
                for unit in c.encode_utf16(&mut [0; 2]) {
                    _ = write!(out, "\\u{unit:04x}");
                }
            }
        }
    }
    out
}

/// The name a key carries ([`key_text`]); what is no escape is taken as
/// written.
fn key_name(text: &str) -> String {
    let mut units: Vec<u16> = Vec::with_capacity(text.len());
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        let escaped = rest.strip_prefix("\\u").and_then(|hex| {
            let unit = u16::from_str_radix(hex.get(..4)?, 16).ok()?;
            Some((unit, 6))
        });
        match escaped {
            Some((unit, used)) => {
                units.push(unit);
                rest = &rest[used..];
            }
            None if rest.starts_with("\\\\") => {
                units.push(u16::from(b'\\'));
                rest = &rest[2..];
            }
            None => {
                units.extend(c.encode_utf16(&mut [0; 2]).iter());
                rest = &rest[c.len_utf8()..];
            }
        }
    }
    String::from_utf16_lossy(&units)
}

/// The code a handler of the window `name` starts with: a note for your
/// own, or what its action does. An error for a variant the settings do not
/// have.
pub fn handler_code(name: &str, handler: &Route, settings: &Settings) -> Result<String, NuiError> {
    let mut out = String::new();
    // A block of its own, so a second insert can declare its variable again.
    let block = |out: &mut String, variable: &str, value: &Value, call: String| {
        let mut json = String::new();
        script::json_string(&mut json, variable, value);
        out.push_str("        {\n");
        for line in json.lines() {
            writeln!(out, "        {line}").unwrap();
        }
        writeln!(out, "            {call}\n        }}").unwrap();
    };
    match &handler.action {
        // (An action this version does not know writes what it can: the note.)
        Action::Code | Action::Other(_) => out.push_str(STUB),
        Action::Close => out.push_str("        NuiDestroy(oPlayer, nToken);\n"),
        Action::Toggle { bind } => {
            let name = string_expr(bind);
            if settings.bindings.get(bind).is_some_and(|b| b.value.is_array()) {
                writeln!(out, "        json jValues = NuiGetBind(oPlayer, nToken, {name});\n        if (nRow >= 0 && nRow < JsonGetLength(jValues))\n            NuiSetBind(oPlayer, nToken, {name}, JsonArraySet(jValues, nRow, JsonBool(!JsonGetInt(JsonArrayGet(jValues, nRow)))));").unwrap();
            } else {
                writeln!(out, "        NuiSetBind(oPlayer, nToken, {name}, JsonBool(!JsonGetInt(NuiGetBind(oPlayer, nToken, {name}))));").unwrap();
            }
        }
        Action::Set { bind, value } => match literal(value) {
            // A value written as NWScript reads it, to change by hand.
            Some(json) => {
                writeln!(out, "        NuiSetBind(oPlayer, nToken, {}, {json});", string_expr(bind))
                    .unwrap()
            }
            None => block(
                &mut out,
                "sValue",
                value,
                format!("NuiSetBind(oPlayer, nToken, {}, JsonParse(sValue));", string_expr(bind)),
            ),
        },
        // The layout comes from the opener's function, rebuilt on every Build.
        Action::View { group, view } => {
            if !settings.views.contains_key(view) {
                return Err(NuiError::MissingVariant(view.clone()));
            }
            writeln!(
                out,
                "        NuiSetGroupLayout(oPlayer, nToken, {}, {}());",
                string_expr(group),
                variant_function(name, view)
            )
            .unwrap();
        }
    }
    Ok(out)
}

/// A JSON value as an NWScript expression one reads and edits: a string,
/// a whole number, a decimal or a flag; None for the rest (and for text or
/// numbers a literal can't spell).
fn literal(value: &Value) -> Option<String> {
    Some(match value {
        Value::String(s) if s.chars().all(|c| c.is_ascii() && !c.is_ascii_control()) => {
            format!("JsonString(\"{}\")", s.replace('\\', "\\\\").replace('"', "\\\""))
        }
        Value::String(s) => format!("JsonString({})", string_expr(s)),
        Value::Bool(b) => format!("JsonBool({})", if *b { "TRUE" } else { "FALSE" }),
        Value::Number(n) => match n.as_i64() {
            Some(i) if i32::try_from(i).is_ok() => format!("JsonInt({i})"),
            Some(_) => return None,
            None => {
                let f = n.as_f64()?;
                // Written out in full with a decimal point, as NWScript
                // reads a float: it knows no exponents, which `{:?}`
                // prints for a number under 0.0001.
                if !(f == 0.0 || (1e-6..1e15).contains(&f.abs())) {
                    return None;
                }
                let digits = format!("{f}");
                match digits.contains('.') {
                    true => format!("JsonFloat({digits})"),
                    false => format!("JsonFloat({digits}.0)"),
                }
            }
        },
        _ => return None,
    })
}

/// The sections of an event script: those in use, and those kept from
/// handlers since removed.
fn sections(script: &str) -> (BTreeMap<String, String>, BTreeMap<String, String>) {
    let (mut live, mut removed) = (BTreeMap::new(), BTreeMap::new());
    let mut open: Option<(bool, String, String)> = None;
    for line in script.split_inclusive('\n') {
        let trimmed = line.trim();
        // The key is the rest of the line as written: an ID may end in a space.
        let rest = |marker: &str| {
            line.trim_start()
                .strip_prefix(marker)
                .map(|k| k.trim_end_matches(['\n', '\r']).to_owned())
        };
        match open.as_mut() {
            Some(_) if trimmed == END => {
                let (gone, key, body) = open.take().unwrap();
                if gone { &mut removed } else { &mut live }.insert(key, body);
            }
            Some((gone, _, body)) => body.push_str(if *gone {
                line.strip_prefix("// ").or_else(|| line.strip_prefix("//")).unwrap_or(line)
            } else {
                line
            }),
            None => {
                if let Some(key) = rest(BEGIN) {
                    open = Some((false, key, String::new()));
                } else if let Some(key) = rest(REMOVED) {
                    open = Some((true, key, String::new()));
                }
            }
        }
    }
    // A section left open at the end keeps what it holds.
    if let Some((gone, key, body)) = open {
        if gone { &mut removed } else { &mut live }.insert(key, body);
    }
    (live, removed)
}

/// The event script for these handlers, before anything is written in it.
pub fn event_source(name: &str, settings: &Settings) -> Result<String, NuiError> {
    merge_events(name, settings, None)
}

/// The NWScript name for `text`: letters and digits as they are, anything
/// else spelled by its code, so two names never meet.
fn identifier(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() { c.to_string() } else { format!("_{:x}_", u32::from(c)) }
        })
        .collect()
}

/// The function in `<name>_o` that returns a swap layout variant, rebuilt
/// with the window on every Build.
pub fn variant_function(name: &str, view: &str) -> String {
    // A letter first, as NWScript names need, whatever the window is called.
    format!("Variant_{name}_{}", identifier(view))
}

/// The function in `<name>_o` that returns a swap layout's initial contents.
pub fn initial_function(name: &str, group: &str) -> String {
    format!("Initial_{name}_{}", identifier(group))
}

/// The function `NAME_e` is, for a window whose events go to the module's
/// own event script ([`Settings::event_function`]): `NAME_OnEvent`.
pub fn event_function(name: &str) -> String {
    format!("{name}_OnEvent")
}

/// The function in `<name>_o` that returns the window's ID in the game,
/// which [`event_function`] knows its window's events by.
pub fn window_id_function(name: &str) -> String {
    format!("WindowId_{name}")
}

/// The line of the event script that brings in the layout functions.
fn include_line(name: &str) -> String {
    format!("#include \"{name}_o\"\n")
}

/// The event script for these handlers, keeping what is written between the
/// markers of `existing`: each handler's code, the code of handlers removed
/// since (commented out, back when the handler is), and the top section.
pub fn merge_events(
    name: &str,
    settings: &Settings,
    existing: Option<&str>,
) -> Result<String, NuiError> {
    let (mut kept, removed) = existing.map(sections).unwrap_or_default();
    let top = kept.remove("top").unwrap_or_else(|| "// Includes and helper functions.\n".into());
    let include = include_line(name);
    // A script of its own (`main`), or a function the module's own event
    // script calls (`NAME_OnEvent`), which answers whether the event was
    // this window's.
    let function = settings.event_function;
    let (head, start, done) = if function {
        (
            format!(
                "// NUI events of the window {name}, for the module's own event script\n// (OnNuiEvent): include {name}_e there and call {name}_OnEvent(), which\n// handles this window's events and returns TRUE for them, FALSE for\n// another window's. Build & compile rebuilds this file: write your code\n// between the mg:begin and mg:end lines, where it is kept; return TRUE\n// from a handler, not return.\n"
            ),
            format!(
                "{}()\n{{\n    object oPlayer = NuiGetEventPlayer();\n    int nToken = NuiGetEventWindow();\n    if (NuiGetWindowId(oPlayer, nToken) != {}()) return FALSE;\n",
                event_function(name),
                window_id_function(name)
            ),
            "return TRUE;",
        )
    } else {
        (
            "// NUI events. Build & compile rebuilds this script: write your code\n// between the mg:begin and mg:end lines, where it is kept.\n".to_owned(),
            "void main()\n{\n    object oPlayer = NuiGetEventPlayer();\n    int nToken = NuiGetEventWindow();\n".to_owned(),
            "return;",
        )
    };
    let mut out = format!(
        "{head}{include}{BEGIN}top\n{top}{END}\n{}{start}    string sType = NuiGetEventType();\n    string sElement = NuiGetEventElement();\n    int nRow = NuiGetEventArrayIndex();\n",
        if function { "int " } else { "" }
    );
    let mut seen = BTreeSet::new();
    for handler in &settings.actions {
        let key = handler_key(handler);
        // A second handler for one event would never run (the first
        // returns), and its section could not be told from the first's.
        if !seen.insert(key.clone()) {
            continue;
        }
        let guard = if matches!(handler.event.as_str(), "open" | "close") {
            format!("sType == {}", string_expr(&handler.event))
        } else {
            format!(
                "sType == {} && sElement == {}",
                string_expr(&handler.event),
                string_expr(&handler.element)
            )
        };
        let mut body = match kept.remove(&key).or_else(|| removed.get(&key).cloned()) {
            Some(body) => body,
            None => handler_code(name, handler, settings)?,
        };
        if !body.ends_with('\n') {
            body.push('\n');
        }
        write!(
            out,
            "\n    if ({guard})\n    {{\n        {BEGIN}{key}\n{body}        {END}\n        {done}\n    }}\n"
        )
        .unwrap();
    }
    if function {
        // (An event of this window no handler takes is still its own.)
        out.push_str("    return TRUE;\n");
    }
    out.push_str("}\n");
    let gone = kept.into_iter().chain(removed.into_iter().filter(|(k, _)| !seen.contains(k)));
    for (key, body) in gone {
        if body.trim().is_empty() || body == STUB {
            continue;
        }
        writeln!(out, "{REMOVED}{key}").unwrap();
        for line in body.lines() {
            if line.is_empty() {
                out.push_str("//\n");
            } else {
                writeln!(out, "// {line}").unwrap();
            }
        }
        writeln!(out, "{END}").unwrap();
    }
    Ok(out)
}

/// An event script with a control's ID or a bind's name changed from
/// `from` to `to` where the code names it: a string that is an argument
/// of one of the game's NUI functions (`NuiSetBind(oPlayer, nToken,
/// "from", …)`), one compared with `==` or `!=` (`sElement == "from"`),
/// and the name quoted inside a string (JSON given to `JsonParse`: a
/// bind's `{"bind":"from"}`). Any other string that happens to spell the
/// name (a message, a tag, a variable's name) is the builder's own and
/// stays.
pub fn rename_in_code(script: &str, from: &str, to: &str) -> String {
    use mg_script::lex::{TokenKind, tokenize};
    let src = script.as_bytes();
    let tokens: Vec<_> = tokenize(src).into_iter().filter(|t| t.kind.is_significant()).collect();
    let text = |i: usize| tokens.get(i).and_then(|t| script.get(t.span.clone())).unwrap_or("");
    let (quoted, inner) = (format!("\"{from}\""), format!("\\\"{from}\\\""));
    let inner_to = format!("\\\"{to}\\\"");
    // The function each open parenthesis is the call of ("" for none).
    let mut calls: Vec<&str> = Vec::new();
    let (mut out, mut at) = (String::with_capacity(script.len()), 0);
    for (i, token) in tokens.iter().enumerate() {
        let spelled = text(i);
        match token.kind {
            TokenKind::Punct if spelled == "(" => {
                let callee = i.checked_sub(1).filter(|p| tokens[*p].kind == TokenKind::Ident);
                calls.push(callee.map_or("", text));
            }
            TokenKind::Punct if spelled == ")" => _ = calls.pop(),
            TokenKind::String { terminated: true } => {
                let compared = |j: Option<usize>| j.is_some_and(|j| matches!(text(j), "==" | "!="));
                let named = spelled == quoted
                    && (calls.last().is_some_and(|c| c.starts_with("Nui"))
                        || compared(i.checked_sub(1))
                        || compared(Some(i + 1)));
                let now = match named {
                    true => format!("\"{to}\""),
                    false => spelled.replace(&inner, &inner_to),
                };
                out.push_str(&script[at..token.span.start]);
                out.push_str(&now);
                at = token.span.end;
            }
            _ => {}
        }
    }
    out.push_str(&script[at..]);
    out
}

/// Whether anything outside the sections of the window `name`'s event
/// script differs from what Moonglow writes for the handlers the script
/// has: rebuilding it would lose that.
pub fn edited_outside(name: &str, script: &str) -> bool {
    let actions = script
        .lines()
        .filter_map(|l| l.trim_start().strip_prefix(BEGIN))
        .filter(|key| *key != "top")
        .map(|key| {
            let (event, element) = key.split_once(' ').unwrap_or((key, ""));
            Route::new(key_name(event), key_name(element), Action::Code)
        })
        .collect();
    // (A function for the module's event script, or a script of its own:
    // as it was written.)
    let event_function = script.contains(&format!("int {}()\n", event_function(name)));
    let settings = Settings { actions, event_function, ..Default::default() };
    merge_events(name, &settings, Some(script)).is_ok_and(|s| s != script)
}

/// The script with `code` added at the end of the section `key`, in place
/// of its note; `None` if the script has no such section, or it does not
/// end.
pub fn insert_code(script: &str, key: &str, code: &str) -> Option<String> {
    let body = handler_body_start(script, key)?;
    // The section ends at the first line that is the end marker alone.
    let mut line = body;
    for l in script[body..].split_inclusive('\n') {
        if l.trim() == END {
            break;
        }
        line += l.len();
    }
    if line >= script.len() {
        return None;
    }
    let current = &script[body..line];
    let mut out = script[..body].to_owned();
    if current != STUB {
        out.push_str(current);
    }
    out.push_str(code);
    out.push_str(&script[line..]);
    Some(out)
}

/// Where the code of the section `key` starts, in characters from the
/// script's start: where the script editor opens it.
pub fn handler_offset(script: &str, key: &str) -> Option<usize> {
    let line = handler_body_start(script, key)?;
    Some(script[..line].chars().count())
}

/// Where the line after a handler's begin marker starts: the marker read as
/// a whole line, as sections() reads it.
fn handler_body_start(script: &str, key: &str) -> Option<usize> {
    let mut at = 0;
    for l in script.split_inclusive('\n') {
        at += l.len();
        if l.trim_start().strip_prefix(BEGIN).map(|k| k.trim_end_matches(['\n', '\r'])) == Some(key)
        {
            return Some(at);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(event: &str, element: &str) -> Route {
        Route::new(event, element, Action::Code)
    }

    fn handlers(actions: Vec<Route>) -> Settings {
        Settings { actions, ..Default::default() }
    }

    #[test]
    fn a_name_is_renamed_where_the_code_names_it() {
        let code = concat!(
            "NuiSetBind(oPlayer, nToken, \"ok\", JsonBool(TRUE));\n",
            "if (sElement == \"ok\" || \"ok\" != sType) {}\n",
            "json j = JsonParse(\"{\\\"bind\\\":\\\"ok\\\"}\");\n",
            "SendMessageToPC(oPlayer, \"ok\");\n",
            "SetLocalInt(oPlayer, \"ok\", NuiGetEventWindow());\n",
            "// \"ok\" in a comment\n",
        );
        let renamed = rename_in_code(code, "ok", "confirm");
        assert!(renamed.contains("NuiSetBind(oPlayer, nToken, \"confirm\", JsonBool(TRUE));"));
        assert!(renamed.contains("sElement == \"confirm\" || \"confirm\" != sType"));
        assert!(renamed.contains("{\\\"bind\\\":\\\"confirm\\\"}"), "{renamed}");
        // The builder's own strings, and a comment, stay.
        assert!(renamed.contains("SendMessageToPC(oPlayer, \"ok\");"));
        assert!(renamed.contains("SetLocalInt(oPlayer, \"ok\", NuiGetEventWindow());"));
        assert!(renamed.contains("// \"ok\" in a comment"));
        // (Hostile text is read through.)
        for text in ["\"ok", ")))(\"ok\"", "\u{17b}\"ok\" == ("] {
            rename_in_code(text, "ok", "confirm");
        }
    }

    #[test]
    fn a_plain_name_is_written_as_a_string_literal() {
        assert_eq!(string_expr("click"), r#""click""#);
        assert_eq!(string_expr("pick_2"), r#""pick_2""#);
        assert_eq!(string_expr("with space"), r#""with space""#);
    }

    #[test]
    fn a_name_a_literal_cannot_hold_is_spelled_through_json_escapes() {
        assert_eq!(string_expr("Żó"), r#"JsonGetString(JsonParse("\"\\u017b\\u00f3\""))"#);
        for odd in ["a\"b", "a\\b", "a\nb", "🐉"] {
            let written = string_expr(odd);
            assert!(written.starts_with("JsonGetString(JsonParse("), "{odd}");
            assert!(written.is_ascii(), "{odd}");
        }
    }

    #[test]
    fn a_handlers_guard_compares_its_event_and_its_control() {
        let script = event_source("w", &handlers(vec![code("click", "pick")])).unwrap();
        assert!(script.contains(r#"if (sType == "click" && sElement == "pick")"#), "{script}");
        assert!(!script.contains("JsonParse"));
    }

    #[test]
    fn a_handler_of_the_windows_own_event_asks_for_no_control() {
        let script = event_source("w", &handlers(vec![code("open", "ignored")])).unwrap();
        assert!(script.contains("    if (sType == \"open\")\n"), "{script}");
        assert!(script.contains("// mg:begin open\n"));
        assert_eq!(handler_key(&code("close", "ignored")), "close");
        assert_eq!(handler_key(&code("click", "ok")), "click ok");
    }

    #[test]
    fn the_event_script_includes_the_opener() {
        let script = event_source("shop", &Settings::default()).unwrap();
        assert!(script.contains("#include \"shop_o\"\n// mg:begin top\n"), "{script}");
    }

    #[test]
    fn a_new_section_holds_a_note_where_the_code_goes() {
        let s = handlers(vec![code("click", "ok"), code("watch", "volume")]);
        let fresh = event_source("test", &s).unwrap();
        assert_eq!(fresh.matches("// Your code here.").count(), 2);
    }

    #[test]
    fn a_sections_offset_is_the_line_after_its_begin_marker() {
        let s = handlers(vec![code("click", "ok"), code("watch", "volume")]);
        let fresh = event_source("test", &s).unwrap();
        let ok = handler_offset(&fresh, "click ok").unwrap();
        assert!(fresh[..ok].ends_with("// mg:begin click ok\n"));
        assert_eq!(handler_offset(&fresh, "click gone"), None);
        // In characters: text before it may be of any script.
        let before = format!("// Żółć\n{fresh}");
        assert_eq!(handler_offset(&before, "click ok"), Some(ok + 8));
    }

    #[test]
    fn inserted_code_takes_the_place_of_the_note() {
        let fresh = event_source("test", &handlers(vec![code("click", "ok")])).unwrap();
        let line = "        SendMessageToPC(oPlayer, \"ok\");\n";
        let written = insert_code(&fresh, "click ok", line).unwrap();
        assert!(!written.contains("// Your code here."));
        assert!(written.contains(&format!("// mg:begin click ok\n{line}        // mg:end\n")));
        // More code goes after what is there.
        let more = insert_code(&written, "click ok", "        int n = 1;\n").unwrap();
        assert!(more.contains(&format!("{line}        int n = 1;\n        // mg:end\n")));
        assert_eq!(insert_code(&fresh, "click gone", line), None);
    }

    #[test]
    fn code_written_in_a_section_is_kept_when_the_script_is_rebuilt() {
        let mut s = handlers(vec![code("click", "ok")]);
        let fresh = event_source("test", &s).unwrap();
        let written = insert_code(&fresh, "click ok", "        int nKept = 1;\n").unwrap();
        assert_eq!(merge_events("test", &s, Some(&written)).unwrap(), written);
        // A handler's action is only what its section starts with.
        s.actions[0].action = Action::Close;
        assert_eq!(merge_events("test", &s, Some(&written)).unwrap(), written);
        s.actions.push(code("open", ""));
        let more = merge_events("test", &s, Some(&written)).unwrap();
        assert!(more.contains("int nKept = 1;") && more.contains("// mg:begin open\n"));
    }

    #[test]
    fn code_written_in_the_top_section_is_kept() {
        let s = handlers(vec![code("click", "ok")]);
        let fresh = event_source("test", &s).unwrap();
        let written = insert_code(&fresh, "top", "#include \"mine\"\n").unwrap();
        assert_eq!(merge_events("test", &s, Some(&written)).unwrap(), written);
        assert!(!edited_outside("test", &written));
    }

    #[test]
    fn a_removed_handlers_code_is_kept_commented_out_after_main() {
        let mut s = handlers(vec![code("click", "ok"), code("watch", "volume")]);
        let fresh = event_source("test", &s).unwrap();
        let written =
            insert_code(&fresh, "click ok", "        SendMessageToPC(oPlayer, \"ok\");\n").unwrap();
        s.actions.remove(0);
        let without = merge_events("test", &s, Some(&written)).unwrap();
        assert!(without.contains("// mg:removed click ok\n//         SendMessageToPC"));
        assert!(!without.lines().any(|l| l.trim_start().starts_with("SendMessageToPC")));
        assert!(!edited_outside("test", &without));
    }

    #[test]
    fn a_removed_handlers_code_comes_back_with_the_handler() {
        let mut s = handlers(vec![code("click", "ok"), code("watch", "volume")]);
        let fresh = event_source("test", &s).unwrap();
        let written = insert_code(
            &fresh,
            "click ok",
            "        SendMessageToPC(oPlayer, \"ok\");\n\n        int n;\n",
        )
        .unwrap();
        s.actions.remove(0);
        let without = merge_events("test", &s, Some(&written)).unwrap();
        s.actions.insert(0, code("click", "ok"));
        assert_eq!(merge_events("test", &s, Some(&without)).unwrap(), written);
    }

    #[test]
    fn a_removed_handler_that_held_only_the_note_leaves_nothing() {
        let mut s = handlers(vec![code("click", "ok")]);
        let fresh = event_source("test", &s).unwrap();
        s.actions.clear();
        assert!(!merge_events("test", &s, Some(&fresh)).unwrap().contains("mg:removed"));
    }

    #[test]
    fn a_handlers_action_is_the_code_its_section_starts_with() {
        let mut s = Settings::default();
        s.views.insert("page".into(), json!({"type": "col", "children": []}));
        let starts = |action: Action| handler_code("w", &Route::new("click", "b", action), &s);
        assert!(starts(Action::Close).unwrap().contains("NuiDestroy(oPlayer, nToken);"));
        assert_eq!(starts(Action::Code).unwrap(), STUB);
        assert_eq!(starts(Action::Other(json!("later"))).unwrap(), STUB);
        let show = Action::View { group: "pages".into(), view: "page".into() };
        assert_eq!(
            starts(show).unwrap(),
            "        NuiSetGroupLayout(oPlayer, nToken, \"pages\", Variant_w_page());\n"
        );
        let gone = Action::View { group: "pages".into(), view: "gone".into() };
        assert_eq!(starts(gone), Err(NuiError::MissingVariant("gone".into())));
    }

    #[test]
    fn a_value_set_is_written_as_nwscript_reads_it() {
        let s = Settings::default();
        let set = |value: Value| {
            let action = Action::Set { bind: "b".into(), value };
            handler_code("w", &Route::new("click", "x", action), &s).unwrap()
        };
        let call = |json: &str| format!("        NuiSetBind(oPlayer, nToken, \"b\", {json});\n");
        assert_eq!(set(json!("a \"q\" \\")), call(r#"JsonString("a \"q\" \\")"#));
        assert_eq!(set(json!(true)), call("JsonBool(TRUE)"));
        assert_eq!(set(json!(-7)), call("JsonInt(-7)"));
        assert_eq!(set(json!(2.0)), call("JsonFloat(2.0)"));
        // (A small one in full: NWScript reads no exponent, which Rust's
        // `{:?}` writes under 0.0001.)
        assert_eq!(set(json!(0.00005)), call("JsonFloat(0.00005)"));
        assert_eq!(set(json!(1e14)), call("JsonFloat(100000000000000.0)"));
        // What a literal cannot spell goes through JsonParse.
        for value in [json!({"x": 1}), json!([1, 2]), json!(null), json!(5_000_000_000_i64)] {
            assert!(set(value.clone()).contains("JsonParse(sValue)"), "{value}");
        }
        assert!(set(json!("Żółć")).is_ascii());
    }

    #[test]
    fn a_sections_key_is_read_as_written() {
        // An ID ending in a space keeps its code (the checks refuse such IDs).
        let s = handlers(vec![code("click", "ok ")]);
        let fresh = event_source("test", &s).unwrap();
        let written = insert_code(&fresh, "click ok ", "        int nKept = 1;\n").unwrap();
        assert_eq!(merge_events("test", &s, Some(&written)).unwrap(), written);
        assert!(!edited_outside("test", &written));
    }

    #[test]
    fn a_sections_key_is_ascii_whatever_the_names_are() {
        let odd = ["Żółw \"x\"", "a\\b", "🐉", "line\nbreak", "\\u017b", "ok ", "tab\there"];
        for id in odd {
            let handler = code("click", id);
            let key = handler_key(&handler);
            assert!(key.bytes().all(|b| (b' '..=b'~').contains(&b)), "{key:?}");
            let (event, element) = key.split_once(' ').unwrap();
            assert_eq!((key_name(event), key_name(element)), ("click".into(), id.to_owned()));
        }
        assert_eq!(handler_key(&code("click", "Żó")), "click \\u017b\\u00f3");
        assert_eq!(handler_key(&code("click", "plain_id 2")), "click plain_id 2");
        // What is no escape is a name as written.
        assert_eq!(key_name("a\\u12"), "a\\u12");
        assert_eq!(key_name("\\ud83d alone"), "\u{fffd} alone");
    }

    #[test]
    fn code_in_the_section_of_a_control_with_any_name_is_kept() {
        let s = handlers(vec![code("click", "Żółw \"x\""), code("click", "a\\b")]);
        let fresh = event_source("w", &s).unwrap();
        assert!(fresh.is_ascii(), "{fresh}");
        let key = handler_key(&s.actions[0]);
        let written = insert_code(&fresh, &key, "        int nKept = 1;\n").unwrap();
        assert!(!edited_outside("w", &written));
        assert_eq!(merge_events("w", &s, Some(&written)).unwrap(), written);
    }

    #[test]
    fn a_second_handler_for_one_event_gets_no_section() {
        let s = handlers(vec![
            Route::new("click", "ok", Action::Close),
            code("click", "ok"),
            code("open", "a"),
            code("open", "b"),
        ]);
        let script = event_source("test", &s).unwrap();
        assert_eq!(script.matches("// mg:begin click ok\n").count(), 1);
        assert_eq!(script.matches("// mg:begin open\n").count(), 1);
        assert!(script.contains("NuiDestroy"), "the first one's code");
        assert_eq!(merge_events("test", &s, Some(&script)).unwrap(), script);
    }

    #[test]
    fn a_change_outside_the_sections_is_seen() {
        let s = handlers(vec![code("click", "pick")]);
        let script = event_source("w", &s).unwrap();
        assert!(!edited_outside("w", &script));
        assert!(edited_outside("w", &script.replace("void main()", "void main() // mine")));
        assert!(edited_outside("w", &format!("{script}\n// After main\n")));
        assert!(edited_outside("w", &script.replace("#include \"w_o\"\n", "")));
        // Another window's script is not this one's.
        assert!(edited_outside("other", &script));
    }

    #[test]
    fn a_handler_keeps_fields_the_crate_does_not_know() {
        let raw = json!({"event": "click", "element": "ok", "action": "toggle", "bind": "flag",
            "note": [1, {"deep": true}], "zeta": null});
        let handler: Route = serde_json::from_value(raw.clone()).unwrap();
        assert_eq!(handler.action, Action::Toggle { bind: "flag".into() });
        assert_eq!(handler.extra.keys().collect::<Vec<_>>(), ["note", "zeta"]);
        assert_eq!(serde_json::to_value(&handler).unwrap(), raw);
    }

    #[test]
    fn a_handler_is_written_as_it_was_before_it_kept_unknown_fields() {
        let handler =
            Route::new("click", "set", Action::Set { bind: "caption".into(), value: json!("x") });
        assert_eq!(
            serde_json::to_string(&handler).unwrap(),
            r#"{"event":"click","element":"set","action":"set","bind":"caption","value":"x"}"#
        );
        let show = Action::View { group: "g".into(), view: "v".into() };
        assert_eq!(
            serde_json::to_string(&Route::new("open", "", show)).unwrap(),
            r#"{"event":"open","element":"","action":"view","group":"g","view":"v"}"#
        );
    }

    #[test]
    fn an_action_the_crate_does_not_know_is_kept_as_read() {
        for raw in [
            json!({"event": "click", "element": "ok", "action": "teleport", "to": "x", "bind": 3}),
            json!({"event": "click", "element": "ok", "action": 7}),
            json!({"event": "click", "element": "ok", "action": null}),
        ] {
            let handler: Route = serde_json::from_value(raw.clone()).unwrap();
            assert_eq!(handler.action, Action::Other(raw["action"].clone()));
            assert_eq!(serde_json::to_value(&handler).unwrap(), raw);
        }
    }

    #[test]
    fn a_handler_without_its_own_fields_is_an_error() {
        for raw in [
            json!({"element": "ok", "action": "close"}),
            json!({"event": "click", "action": "close"}),
            json!({"event": "click", "element": "ok"}),
            json!({"event": 1, "element": "ok", "action": "close"}),
            json!({"event": "click", "element": "ok", "action": "set", "bind": "b"}),
            json!({"event": "click", "element": "ok", "action": "view", "group": "g"}),
            json!("click"),
        ] {
            assert!(serde_json::from_value::<Route>(raw.clone()).is_err(), "{raw}");
        }
    }

    /// Scripts no Build wrote: none may panic, whatever they return.
    fn hostile() -> Vec<String> {
        let s = handlers(vec![code("click", "ok"), code("watch", "volume")]);
        let good =
            insert_code(&event_source("w", &s).unwrap(), "click ok", "        int n;\n").unwrap();
        let mut scripts = vec![
            String::new(),
            "\n".into(),
            "// mg:begin".into(),
            "// mg:begin ".into(),
            "// mg:begin click ok".into(),
            "// mg:begin click ok\n".into(),
            "// mg:begin click ok\r\n// mg:end".into(),
            "// mg:end\n// mg:end\n".into(),
            "// mg:end\n// mg:begin click ok\n".into(),
            // Nested, duplicated, never closed.
            "// mg:begin click ok\n// mg:begin watch volume\nint a;\n// mg:end\n// mg:end\n".into(),
            "// mg:begin click ok\nint a;\n// mg:end\n// mg:begin click ok\nint b;\n// mg:end\n"
                .into(),
            "// mg:begin top\n// mg:begin top\n".into(),
            "// mg:removed click ok\n// mg:removed click ok\n//\n//x\n".into(),
            "// mg:removed click ok\n// mg:begin click ok\n// mg:end".into(),
            "// mg:begin click ok\n// mg:removed click ok\n// mg:end\n".into(),
            // A marker with nothing after it, and one that ends the text.
            "void main() {}\n// mg:begin ".into(),
            "void main() {}\n        // mg:end".into(),
            // Other line ends, other characters.
            good.replace('\n', "\r\n"),
            good.replace('\n', "\r"),
            good.replace("int n;", "string s = \"Żółć 🐉\"; // \u{2028}\u{0}"),
            "// mg:begin click Żółć\nint ż;\n// mg:end\n".into(),
            "\u{feff}// mg:begin click ok\n\u{a0}// mg:end\n".into(),
            "// mg:begin click ok\u{85}// mg:end\n".into(),
            good.repeat(3),
        ];
        // The good one, cut at every byte (it is ASCII).
        assert!(good.is_ascii());
        scripts.extend((0..good.len()).map(|end| good[..end].to_owned()));
        scripts
    }

    #[test]
    fn a_script_no_build_wrote_is_merged_without_a_panic() {
        let s = handlers(vec![code("click", "ok"), code("open", ""), code("click", "Żółć")]);
        for script in hostile() {
            let merged = merge_events("w", &s, Some(&script)).unwrap();
            // What comes of it is Moonglow's to rebuild, unless a marker
            // was written inside a section: that one Build refuses.
            if !edited_outside("w", &merged) {
                assert_eq!(merge_events("w", &s, Some(&merged)).unwrap(), merged, "{script:?}");
            }
        }
    }

    #[test]
    fn a_script_no_build_wrote_is_looked_over_without_a_panic() {
        for script in hostile() {
            edited_outside("w", &script);
            edited_outside("", &script);
        }
    }

    #[test]
    fn code_is_inserted_into_a_script_no_build_wrote_without_a_panic() {
        for script in hostile() {
            for key in ["click ok", "top", "", "click Żółć", "watch volume", "open"] {
                if let Some(written) = insert_code(&script, key, "    int nNew;\n") {
                    assert!(written.contains("int nNew;"), "{script:?}");
                }
                if let Some(at) = handler_offset(&script, key) {
                    assert!(at <= script.chars().count(), "{script:?}");
                }
            }
        }
    }

    #[test]
    fn a_section_that_does_not_end_takes_no_code() {
        assert_eq!(insert_code("// mg:begin click ok\nint a;\n", "click ok", "x"), None);
        assert_eq!(insert_code("// mg:begin click ok", "click ok", "x"), None);
        // An end marker without a line end after it still ends it.
        assert_eq!(
            insert_code("// mg:begin click ok\n// mg:end", "click ok", "x\n").as_deref(),
            Some("// mg:begin click ok\nx\n// mg:end")
        );
    }

    #[test]
    fn a_function_name_spells_out_what_nwscript_names_cannot_hold() {
        assert_eq!(variant_function("shop", "details"), "Variant_shop_details");
        assert_eq!(variant_function("shop", "details page"), "Variant_shop_details_20_page");
        assert_eq!(initial_function("shop", "_window_"), "Initial_shop__5f_window_5f_");
        assert_ne!(variant_function("w", "a b"), variant_function("w", "a_b"));
    }
}
