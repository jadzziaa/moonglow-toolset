//! Declarative authoring actions lower to stock NWScript calls.
use super::*;
use std::fmt::Write;

/// Native NuiGetEventType values. Shared by validation and the route editor.
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Route {
    pub event: String,
    pub element: String,
    #[serde(flatten)]
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    /// A handler for your own code, written in the event script.
    Code,
    Close,
    Toggle {
        bind: String,
    },
    Set {
        bind: String,
        value: Value,
    },
    View {
        group: String,
        view: String,
    },
}

/// Use JSON Unicode escapes rather than depend on an NWScript source codepage.
pub fn string_expr(s: &str) -> String {
    format!("JsonGetString(JsonParse({}))", script::literal(&script::ascii_json(&json!(s))))
}

/// Where your code goes: between a `BEGIN key` line and the next `END`.
pub const BEGIN: &str = "// mg:begin ";
pub const END: &str = "// mg:end";
/// A handler no longer configured keeps its code, commented out, after main.
const REMOVED: &str = "// mg:removed ";
const STUB: &str = "        // Your code here.\n";

/// The name of a handler's section in the event script.
pub fn handler_key(route: &Route) -> String {
    if matches!(route.event.as_str(), "open" | "close") {
        route.event.clone()
    } else {
        format!("{} {}", route.event, route.element)
    }
}

/// Code for one handler: a note for your own, or what an action does.
pub fn handler_code(name: &str, route: &Route, settings: &Settings) -> Result<String, String> {
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
    match &route.action {
        Action::Code => out.push_str(STUB),
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
                return Err(format!("Missing view: {view}"));
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
                // Debug keeps the ".0" NWScript needs; tiny or huge ones print exponents.
                if !(f == 0.0 || (1e-6..1e15).contains(&f.abs())) {
                    return None;
                }
                format!("JsonFloat({f:?})")
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
pub fn event_source(name: &str, settings: &Settings) -> Result<String, String> {
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
) -> Result<String, String> {
    let (mut kept, removed) = existing.map(sections).unwrap_or_default();
    let top = kept.remove("top").unwrap_or_else(|| "// Includes and helper functions.\n".into());
    let include = include_line(name);
    let mut out = format!(
        "// NUI events. Build & compile rebuilds this script: write your code\n// between the mg:begin and mg:end lines, where it is kept.\n{include}{BEGIN}top\n{top}{END}\nvoid main()\n{{\n    object oPlayer = NuiGetEventPlayer();\n    int nToken = NuiGetEventWindow();\n    string sType = NuiGetEventType();\n    string sElement = NuiGetEventElement();\n    int nRow = NuiGetEventArrayIndex();\n"
    );
    let mut seen = BTreeSet::new();
    for route in &settings.actions {
        let mut key = handler_key(route);
        // Two routes of an old window for one event stay apart.
        while !seen.insert(key.clone()) {
            key.push('+');
        }
        let guard = if matches!(route.event.as_str(), "open" | "close") {
            format!("sType == {}", string_expr(&route.event))
        } else {
            format!(
                "sType == {} && sElement == {}",
                string_expr(&route.event),
                string_expr(&route.element)
            )
        };
        let mut body = match kept.remove(&key).or_else(|| removed.get(&key).cloned()) {
            Some(body) => body,
            None => handler_code(name, route, settings)?,
        };
        if !body.ends_with('\n') {
            body.push('\n');
        }
        write!(
            out,
            "\n    if ({guard})\n    {{\n        {BEGIN}{key}\n{body}        {END}\n        return;\n    }}\n"
        )
        .unwrap();
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

/// Whether anything outside the markers differs from what Moonglow writes
/// for the handlers the script has: rebuilding would lose it.
pub fn edited_outside(name: &str, script: &str) -> bool {
    let actions = script
        .lines()
        .filter_map(|l| l.trim_start().strip_prefix(BEGIN))
        .map(|key| key.trim_end_matches(['\n', '\r']).trim_end_matches('+'))
        .filter(|key| *key != "top")
        .map(|key| {
            let (event, element) = key.split_once(' ').unwrap_or((key, ""));
            Route { event: event.into(), element: element.into(), action: Action::Code }
        })
        .collect();
    let settings = Settings { actions, ..Default::default() };
    // Scripts from before the layout functions have no include line.
    let include = include_line(name);
    merge_events(name, &settings, Some(script))
        .is_ok_and(|s| s.replacen(&include, "", 1) != script.replacen(&include, "", 1))
}

/// The script with `code` added to a handler's section, in place of its note.
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

/// Where a handler's code starts (a character offset), to open it there.
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

pub fn element_ids(v: &Value) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    fn visit(v: &Value, ids: &mut BTreeSet<String>) {
        match v {
            Value::Object(o) => {
                if let Some(id) = o.get("id").and_then(Value::as_str) {
                    ids.insert(id.into());
                }
                for v in o.values() {
                    visit(v, ids)
                }
            }
            Value::Array(a) => {
                for v in a {
                    visit(v, ids)
                }
            }
            _ => {}
        }
    }
    visit(v, &mut ids);
    ids
}

pub fn group_ids(v: &Value) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    fn visit(v: &Value, ids: &mut BTreeSet<String>) {
        match v {
            Value::Object(o) => {
                if v["type"] == "group"
                    && let Some(id) = v["id"].as_str()
                {
                    ids.insert(id.into());
                }
                for v in o.values() {
                    visit(v, ids)
                }
            }
            Value::Array(a) => {
                for v in a {
                    visit(v, ids)
                }
            }
            _ => {}
        }
    }
    visit(v, &mut ids);
    ids
}
