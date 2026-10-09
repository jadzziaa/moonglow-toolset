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
    Close,
    Toggle { bind: String },
    Set { bind: String, value: Value },
    View { group: String, view: String },
}

/// Use JSON Unicode escapes rather than depend on an NWScript source codepage.
pub(crate) fn string_expr(s: &str) -> String {
    format!("JsonGetString(JsonParse({}))", script::literal(&script::ascii_json(&json!(s))))
}

/// Render the handler used by generation, without compiling or changing resources.
/// Manual module scripts are handled separately by `generate` and may differ.
pub fn event_source(settings: &Settings) -> Result<String, String> {
    if settings.actions.is_empty() {
        return Ok(script::EVENTS.into());
    }
    let mut out="// Generated NUI events. Regeneration protects manual changes.\nvoid main()\n{\n    object oPlayer = NuiGetEventPlayer();\n    int nToken = NuiGetEventWindow();\n    string sType = NuiGetEventType();\n    string sElement = NuiGetEventElement();\n    int nRow = NuiGetEventArrayIndex();\n".to_owned();
    for route in &settings.actions {
        let guard = if matches!(route.event.as_str(), "open" | "close") {
            format!("sType == {}", string_expr(&route.event))
        } else {
            format!(
                "sType == {} && sElement == {}",
                string_expr(&route.event),
                string_expr(&route.element)
            )
        };
        writeln!(out, "    if ({guard})\n    {{").unwrap();
        match &route.action {
            Action::Close => out.push_str("        NuiDestroy(oPlayer, nToken);\n"),
            Action::Toggle { bind } => {
                let name = string_expr(bind);
                if settings.bindings.get(bind).is_some_and(|b| b.value.is_array()) {
                    writeln!(out,"        json jValues = NuiGetBind(oPlayer, nToken, {name});\n        if (nRow >= 0 && nRow < JsonGetLength(jValues))\n            NuiSetBind(oPlayer, nToken, {name}, JsonArraySet(jValues, nRow, JsonBool(!JsonGetInt(JsonArrayGet(jValues, nRow)))));").unwrap();
                } else {
                    writeln!(out,"        NuiSetBind(oPlayer, nToken, {name}, JsonBool(!JsonGetInt(NuiGetBind(oPlayer, nToken, {name}))));").unwrap();
                }
            }
            Action::Set { bind, value } => {
                script::json_string(&mut out, "sAction", value);
                writeln!(
                    out,
                    "        NuiSetBind(oPlayer, nToken, {}, JsonParse(sAction));",
                    string_expr(bind)
                )
                .unwrap();
            }
            Action::View { group, view } => {
                let layout =
                    settings.views.get(view).ok_or_else(|| format!("Missing view: {view}"))?;
                script::json_string(&mut out, "sAction", layout);
                writeln!(
                    out,
                    "        NuiSetGroupLayout(oPlayer, nToken, {}, JsonParse(sAction));",
                    string_expr(group)
                )
                .unwrap();
            }
        }
        out.push_str("        return;\n    }\n");
    }
    out.push_str("}\n");
    Ok(out)
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
