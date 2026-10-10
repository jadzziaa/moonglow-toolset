use crate::{ELEMENTS, Settings, bind_names};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub path: String,
    pub message: String,
}

/// Checks stock API shapes. Warnings deliberately retain extension widgets and
/// unknown fields: only the NWN client can adjudicate an extension's behavior.
pub fn validate(window: &Value, settings: &Settings) -> Vec<Diagnostic> {
    let mut c = Check { settings, findings: Vec::new(), ids: BTreeSet::new() };
    if !window.is_object() {
        c.error("", "NuiWindow must be a JSON object");
        return c.findings;
    }
    if window.get("version").and_then(Value::as_u64) != Some(1) {
        c.error("/version", "Stock NuiWindow uses integer version 1");
    }
    match window.get("root") {
        Some(root) => {
            if !matches!(root["type"].as_str(), Some("row" | "col" | "group")) {
                c.warn("/root", "NuiWindow documents a row, column or group root; client acceptance of other roots is unverified");
            }
            c.element(root, "/root", false);
        }
        None => c.error("/root", "Missing window layout"),
    }
    // NWN EE 8193.37: with Clip to control on the last draw list painted,
    // the whole window shows blank (its own layers aren't clipped either).
    if let Some((path, node)) = window.get("root").and_then(|r| last_draw_list(r, "/root"))
        && scissor_on(node, settings)
    {
        c.error(
            &format!("{path}/draw_list_scissor"),
            "NWN shows this window blank: the last draw layers in it have Clip to control on. Turn it off (it clips nothing in the game)",
        );
    }
    for k in ["resizable", "collapsed", "closable", "transparent", "border", "accepts_input"] {
        c.property(window, k, "", Kind::Bool, true, false);
    }
    c.property(window, "title", "", Kind::Text, true, false);
    c.property(window, "geometry", "", Kind::Rect, false, false);
    for k in ["size_constraint", "edge_constraint"] {
        c.property(window, k, "", Kind::Rect, true, false);
    }
    c.property(window, "font", "", Kind::Text, false, false);
    for name in bind_names(window) {
        if !settings.bindings.contains_key(&name) {
            c.error(
                "/bindings",
                &format!("Bind {name:?} needs an explicit initial value in this generator"),
            );
        }
    }
    for name in settings.bindings.keys() {
        if name.is_empty() || name.contains('\0') {
            c.error("/bindings", "Bind names must be nonempty and contain no NUL");
        }
    }
    if settings.from_resref {
        c.warn("", "NuiCreateFromResRef needs this JUI on every player's client (for example in a distributed HAK)");
    }
    let mut ids = crate::element_ids(window);
    let mut groups = crate::group_ids(window);
    for (name, view) in &settings.views {
        c.ids.clear();
        c.element(view, &format!("/views/{name}"), false);
        ids.extend(crate::element_ids(view));
        groups.extend(crate::group_ids(view));
        for bind in bind_names(view) {
            if !settings.bindings.contains_key(&bind) {
                c.error("/views", &format!("View {name} needs initial bind {bind}"));
            }
        }
    }
    let mut routes = BTreeSet::new();
    for route in &settings.actions {
        if !crate::EVENT_TYPES.contains(&route.event.as_str()) {
            c.error("/actions", "Unsupported event type");
        }
        // Lifecycle routes do not inspect the element in generated NWScript.
        let element = if matches!(route.event.as_str(), "open" | "close") {
            ""
        } else {
            route.element.as_str()
        };
        if !routes.insert((route.event.as_str(), element)) {
            c.error("/actions", "Duplicate event/element route");
        }
        if route.event == "watch" {
            if !settings.bindings.contains_key(&route.element) {
                c.error("/actions", "Watch route needs an existing bind");
            }
        } else if !matches!(route.event.as_str(), "open" | "close") && !ids.contains(&route.element)
        {
            c.error("/actions", &format!("Unknown element ID: {}", route.element));
        }
        match &route.action {
            crate::Action::Code | crate::Action::Close => {}
            crate::Action::View { group, view } => {
                if group != "_window_" && !groups.contains(group) {
                    c.error("/actions", "View target must name a Group ID");
                }
                if !settings.views.contains_key(view) {
                    c.error("/actions", "Unknown view");
                }
            }
            crate::Action::Toggle { bind } | crate::Action::Set { bind, .. } => {
                if !settings.bindings.contains_key(bind) {
                    c.error("/actions", "Action needs an existing bind");
                }
                if route.event == "watch" && bind == &route.element {
                    c.error("/actions", "Watch action cannot write its own watched bind");
                }
                if matches!(&route.action, crate::Action::Toggle { .. })
                    && settings.bindings.get(bind).is_some_and(|b| {
                        !b.value.is_boolean()
                            && !b.value.as_array().is_some_and(|a| a.iter().all(Value::is_boolean))
                    })
                {
                    c.error("/actions", "Toggle action needs a boolean or boolean array");
                }
            }
        }
    }
    // A window made before Close became an explicit event keeps its button:
    // regenerated, it closes nothing (seen in NWN EE with such a window).
    if ids.contains("mg_close")
        && !settings.actions.iter().any(|r| r.event == "click" && r.element == "mg_close")
    {
        c.warn(
            "/actions",
            "The mg_close button has no Clicked handler: in game it does nothing. Add one that closes the window.",
        );
    }
    if settings.window_id.as_ref().is_some_and(|id| id.is_empty() || id.contains('\0')) {
        c.error("/settings/window_id", "Window ID must be nonempty and contain no NUL");
    }
    if settings
        .window_id
        .as_ref()
        .is_some_and(|id| !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_'))
    {
        c.warn("/settings/window_id", "nwscript documents a short alphanumeric window ID; punctuation and Unicode identity need a native client check");
    }
    // Watch handlers can recursively trigger each other in the native client.
    let edges: std::collections::BTreeMap<_, _> = settings
        .actions
        .iter()
        .filter_map(|r| match &r.action {
            crate::Action::Set { bind, .. } | crate::Action::Toggle { bind }
                if r.event == "watch" =>
            {
                Some((r.element.as_str(), bind.as_str()))
            }
            _ => None,
        })
        .collect();
    for start in edges.keys() {
        let mut seen = BTreeSet::new();
        let mut at = *start;
        while let Some(next) = edges.get(at) {
            if !seen.insert(at) {
                c.error("/actions", "Watch actions form a recursive bind cycle");
                break;
            }
            at = next;
        }
    }
    // Include window geometry/title, not only child widgets. Clearing routes
    // prevents recursion through the action being checked. Only problems the
    // new value causes are the action's: the others are reported once above.
    let errors = |s: &Settings| -> BTreeSet<(String, String)> {
        validate(window, s)
            .into_iter()
            .filter(|d| d.severity == Severity::Error)
            .map(|d| (d.path, d.message))
            .collect()
    };
    let mut unchanged = None;
    for route in &settings.actions {
        if let crate::Action::Set { bind, value } = &route.action {
            let mut changed = Settings { actions: Vec::new(), ..settings.clone() };
            let unchanged = unchanged.get_or_insert_with(|| errors(&changed));
            if let Some(b) = changed.bindings.get_mut(bind) {
                b.value = value.clone();
            }
            for (path, message) in errors(&changed).difference(unchanged) {
                c.error("/actions", &format!("Set {bind}: {message} at {path}"));
            }
        }
    }
    c.findings
}

/// Clip to control on, literally or by its bind's initial value.
fn scissor_on(node: &Value, settings: &Settings) -> bool {
    let v = &node["draw_list_scissor"];
    let v = v["bind"].as_str().and_then(|b| settings.bindings.get(b)).map_or(v, |b| &b.value);
    node["draw_list"].is_array() && *v == true
}

/// The element whose draw list is painted last, in layout order.
fn last_draw_list<'a>(v: &'a Value, path: &str) -> Option<(String, &'a Value)> {
    let mut last = v["draw_list"].is_array().then(|| (path.to_owned(), v));
    if let Some(children) = v["children"].as_array() {
        for (i, c) in children.iter().enumerate() {
            if let Some(found) = last_draw_list(c, &format!("{path}/children/{i}")) {
                last = Some(found);
            }
        }
    }
    if let Some(cells) = v["row_template"].as_array() {
        for (i, c) in cells.iter().enumerate() {
            if let Some(found) = last_draw_list(&c[0], &format!("{path}/row_template/{i}/0")) {
                last = Some(found);
            }
        }
    }
    last
}

#[derive(Clone, Copy)]
enum Kind {
    Bool,
    Text,
    Int,
    Number,
    Rect,
    Color,
    Array,
    RowCount,
    Labels,
    Entries,
    Vec2,
    FloatPairs,
    Numbers,
}
impl Kind {
    fn matches(self, v: &Value) -> bool {
        match self {
            Self::Bool => v.is_boolean(),
            Self::Text => v.is_string() || v.get("strref").is_some_and(|s| s.is_i64()),
            Self::Int => v.as_i64().is_some_and(|n| i32::try_from(n).is_ok()),
            Self::Number => v.is_number(),
            Self::Array => v.is_array(),
            Self::FloatPairs => {
                v.as_array().is_some_and(|a| a.len() % 2 == 0 && a.iter().all(Value::is_number))
            }
            Self::Numbers => v.as_array().is_some_and(|a| a.iter().all(Value::is_number)),
            Self::RowCount => v.is_i64() || v.is_array(),
            Self::Labels => v.as_array().is_some_and(|a| a.iter().all(Value::is_string)),
            Self::Entries => v.as_array().is_some_and(|a| {
                a.iter().all(|v| {
                    v.as_array().is_some_and(|a| a.len() == 2 && a[0].is_string() && a[1].is_i64())
                })
            }),
            Self::Vec2 => ["x", "y"].iter().all(|k| v.get(k).is_some_and(Value::is_number)),
            Self::Rect => {
                ["x", "y", "w", "h"].iter().all(|k| v.get(k).is_some_and(Value::is_number))
            }
            Self::Color => ["r", "g", "b", "a"].iter().all(|k| v.get(k).is_some_and(Value::is_i64)),
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Bool => "boolean",
            Self::Text => "string or StrRef",
            Self::Int => "integer",
            Self::Number => "number",
            Self::Rect => "rectangle {x,y,w,h}",
            Self::Color => "color {r,g,b,a} with integer channels",
            Self::Array => "array",
            Self::FloatPairs => "flat array of x/y number pairs",
            Self::Numbers => "array of numbers",
            Self::RowCount => "integer or bind array (its length is the row count)",
            Self::Labels => "array of string labels",
            Self::Entries => "array of [label, integer] combo entries",
            Self::Vec2 => "vector {x,y}",
        }
    }
}

struct Check<'a> {
    settings: &'a Settings,
    findings: Vec<Diagnostic>,
    ids: BTreeSet<String>,
}
impl Check<'_> {
    fn static_property(&mut self, obj: &Value, k: &str, path: &str, kind: Kind) {
        let Some(value) = obj.get(k) else { return };
        let path = format!("{path}/{k}");
        if value.get("bind").is_some() {
            self.error(&path, "This constructor argument is static in nw_inc_nui; replace the layout to change it");
        } else if !kind.matches(value) {
            self.error(&path, &format!("Expected static {} according to nw_inc_nui", kind.label()));
        }
    }
    fn error(&mut self, path: &str, message: &str) {
        self.findings.push(Diagnostic {
            severity: Severity::Error,
            path: path.into(),
            message: message.into(),
        });
    }
    fn warn(&mut self, path: &str, message: &str) {
        self.findings.push(Diagnostic {
            severity: Severity::Warning,
            path: path.into(),
            message: message.into(),
        });
    }
    fn property(
        &mut self,
        obj: &Value,
        k: &str,
        path: &str,
        kind: Kind,
        nullable: bool,
        array_bind: bool,
    ) {
        let Some(v) = obj.get(k) else { return };
        let p = format!("{path}/{k}");
        let value = if let Some(name) = v.get("bind") {
            for key in ["number_flags", "number_precision", "text_flags"] {
                self.static_property(v, key, &p, Kind::Int);
            }
            let Some(name) = name.as_str() else {
                self.error(&p, "Bind must name a string");
                return;
            };
            let Some(b) = self.settings.bindings.get(name) else { return };
            &b.value
        } else {
            v
        };
        let valid = |v: &Value| {
            kind.matches(v)
                || (matches!(kind, Kind::Text) && obj[k]["bind"].is_string() && v.is_number())
                || (nullable && (v.is_null() || (k == "title" && v == false)))
        };
        let ok = if array_bind && v.get("bind").is_some() {
            value.as_array().is_some_and(|a| a.iter().all(valid))
        } else {
            valid(value)
        };
        if !ok {
            self.error(
                &p,
                &format!(
                    "Expected {}{} according to nw_inc_nui",
                    kind.label(),
                    if array_bind && v.get("bind").is_some() { " array for list rows" } else { "" }
                ),
            );
        }
    }
    fn element(&mut self, v: &Value, path: &str, array_bind: bool) {
        let Some(ty) = v.get("type").and_then(Value::as_str) else {
            self.error(path, "Element needs a string type");
            return;
        };
        if !ELEMENTS.contains(&ty) {
            self.warn(
                path,
                &format!(
                    "Unknown widget {ty:?}: preserved, with no stock API or preview validation"
                ),
            );
        }
        if let Some(id) = v.get("id") {
            if let Some(id) = id.as_str() {
                if !self.ids.insert(id.into()) {
                    self.warn(
                        path,
                        &format!(
                            "Repeated element id {id:?}; events cannot distinguish these elements"
                        ),
                    );
                }
            } else {
                self.error(path, "Element id must be a string");
            }
        }
        for k in ["width", "height", "aspect", "margin", "padding"] {
            if let Some(n) = v.get(k)
                && !n.is_number()
            {
                self.error(
                    &format!("{path}/{k}"),
                    "Geometry modifiers are static numbers; nw_inc_nui does not support binds here",
                );
            }
        }
        for k in ["enabled", "visible", "encouraged"] {
            self.property(v, k, path, Kind::Bool, false, array_bind);
        }
        for k in ["tooltip", "disabled_tooltip", "font"] {
            self.property(v, k, path, Kind::Text, false, array_bind);
        }
        self.property(v, "foreground_color", path, Kind::Color, false, array_bind);
        if matches!(ty, "image" | "button_image") {
            self.property(v, "image_region", path, Kind::Rect, false, array_bind);
        }
        let val = match ty {
            "label" | "text" | "textedit" | "image" => Some(Kind::Text),
            "button_select" | "check" => Some(Kind::Bool),
            "slider" | "combo" | "options" | "tabbar" => Some(Kind::Int),
            "sliderf" | "progress" => Some(Kind::Number),
            "color_picker" => Some(Kind::Color),
            _ => None,
        };
        if let Some(kind) = val {
            self.property(v, "value", path, kind, false, array_bind);
        }
        if matches!(ty, "button" | "button_image" | "button_select" | "check" | "textedit") {
            self.property(v, "label", path, Kind::Text, false, array_bind);
        }
        if matches!(
            ty,
            "check"
                | "button_select"
                | "slider"
                | "sliderf"
                | "combo"
                | "options"
                | "tabbar"
                | "textedit"
                | "color_picker"
        ) && v["value"].get("bind").is_none()
        {
            self.warn(
                path,
                "Static input value: bind value to retain edits and read them in a server event",
            );
        }
        for k in ["text_halign", "text_valign", "image_aspect", "image_halign", "image_valign"] {
            self.property(v, k, path, Kind::Int, false, array_bind);
        }
        for k in ["direction", "scrollbars"] {
            self.static_property(v, k, path, Kind::Int);
        }
        for k in ["border", "multiline", "wordwrap"] {
            self.static_property(v, k, path, Kind::Bool);
        }
        if matches!(ty, "slider" | "sliderf") {
            for k in ["min", "max", "step"] {
                self.property(
                    v,
                    k,
                    path,
                    if ty == "slider" { Kind::Int } else { Kind::Number },
                    false,
                    array_bind,
                );
            }
        }
        // A Toggle button with a fixed value stays as it is when clicked in
        // NWN EE 8193.37 (np_bsel): only a bound one switches.
        if ty == "button_select"
            && v.get("value").is_some_and(|x| !x.is_null() && x.get("bind").is_none())
        {
            self.warn(
                &format!("{path}/value"),
                "A Toggle button with a fixed value can't be switched in NWN: bind its value",
            );
        }
        // NWN EE 8193.37: a disabled slider still takes a click or a drag,
        // and sets its bind to its minimum (seen with integer and decimal
        // sliders, dragged either way); enabled ones behave.
        if matches!(ty, "slider" | "sliderf")
            && v["value"]["bind"].is_string()
            && v.get("enabled").is_some_and(|e| *e != Value::Bool(true))
        {
            self.warn(
                &format!("{path}/enabled"),
                "A disabled slider still sets its bind to its minimum when clicked or dragged in NWN: hide it, or check the value in the server script",
            );
        }
        if ty == "textedit" {
            self.property(v, "max", path, Kind::Int, false, false);
            if v.get("max").is_some_and(|m| m.as_u64().is_none_or(|n| !(1..=65535).contains(&n))) {
                self.error(
                    &format!("{path}/max"),
                    "NuiTextEdit max length must be a static integer from 1 to 65535",
                );
            } else if v["max"].as_u64().is_some_and(|n| n < 4) {
                // max counts UTF-8 bytes: in NWN EE 8193.37 with max 1, typing
                // "Ż" stores U+0005 and "é" U+0003.
                self.warn(
                    &format!("{path}/max"),
                    "max counts UTF-8 bytes, and below 4 NWN can store part of a non-ASCII character as garbage: allow at least 4",
                );
            }
        }
        // Unlike NuiCombo, Options/Toggles require the labels array itself at
        // construction. A bind object here fails in the native client parser.
        if matches!(ty, "options" | "tabbar") {
            self.static_property(v, "elements", path, Kind::Labels);
        }
        if ty == "combo" {
            self.property(v, "elements", path, Kind::Entries, false, array_bind);
        }
        if ty == "chart" {
            self.static_property(v, "value", path, Kind::Array);
            if let Some(slots) = v["value"].as_array() {
                for (i, slot) in slots.iter().enumerate() {
                    let p = format!("{path}/value/{i}");
                    if !slot.is_object() {
                        self.error(&p, "NuiChartSlot must be an object");
                        continue;
                    }
                    self.static_property(slot, "type", &p, Kind::Int);
                    self.property(slot, "legend", &p, Kind::Text, false, array_bind);
                    self.property(slot, "color", &p, Kind::Color, false, array_bind);
                    self.property(slot, "data", &p, Kind::Numbers, false, array_bind);
                }
            }
        }
        if matches!(ty, "row" | "col" | "group") {
            if let Some(a) = v.get("children").and_then(Value::as_array) {
                if ty == "group" && a.len() != 1 {
                    self.error(path, "NuiGroup has exactly one child");
                }
                // The client lays NUI out with a constraint solver: across a
                // row (its height) or a column (its width), a child's size and
                // margins must fit in the parent's. Otherwise the window fails
                // with "The constraint can not be satisfied" (NWN EE 8193.37:
                // row 30/33 with a child of 30 fails, 34 opens; margin 0 opens
                // at 30). Along the row or column, children may overflow.
                let across = match ty {
                    "row" => Some("height"),
                    "col" => Some("width"),
                    _ => None,
                };
                if let Some(key) = across
                    && let Some(room) = v.get(key).and_then(Value::as_f64)
                {
                    for (i, child) in a.iter().enumerate() {
                        let Some(size) = child.get(key).and_then(Value::as_f64) else { continue };
                        let margin = child.get("margin").and_then(Value::as_f64).unwrap_or(2.0);
                        if size + 2.0 * margin > room + 1e-3 {
                            self.error(
                                &format!("{path}/children/{i}/{key}"),
                                &format!(
                                    "NWN refuses this window: {key} {size} plus margins {} exceeds the {}'s {key} {room} (\"The constraint can not be satisfied\")",
                                    2.0 * margin,
                                    if ty == "row" { "row" } else { "column" },
                                ),
                            );
                        }
                    }
                }
                for (i, child) in a.iter().enumerate() {
                    self.element(child, &format!("{path}/children/{i}"), array_bind);
                }
            } else {
                self.error(path, "Layout children must be an array");
            }
        }
        if ty == "list" {
            self.property(
                v,
                "row_count",
                path,
                if v["row_count"].get("bind").is_some() { Kind::RowCount } else { Kind::Int },
                false,
                false,
            );
            self.static_property(v, "row_height", path, Kind::Number);
            if v["scrollbars"] == 4 {
                self.error(path, "NuiList cannot use AUTO scrollbars");
            }
            if let Some(cells) = v.get("row_template").and_then(Value::as_array) {
                if cells.len() > 16 {
                    self.error(path, "NuiList supports at most 16 template cells");
                }
                for (i, cell) in cells.iter().enumerate() {
                    let p = format!("{path}/row_template/{i}");
                    if cell.as_array().is_none_or(|a| a.len() != 3)
                        || !cell[1].is_number()
                        || !cell[2].is_boolean()
                    {
                        self.error(
                            &p,
                            "List cell must be [element, width, variable] (NuiListTemplateCell)",
                        );
                        continue;
                    }
                    self.element(&cell[0], &format!("{p}/0"), true);
                }
            } else {
                self.error(path, "List needs a row_template array");
            }
        }
        if v.get("draw_list").is_some() {
            self.property(v, "draw_list_scissor", path, Kind::Bool, false, array_bind);
            if let Some(items) = v["draw_list"].as_array() {
                if scissor_on(v, self.settings) {
                    self.warn(
                        &format!("{path}/draw_list_scissor"),
                        "Clip to control clips nothing in NWN EE 8193.37 (the layers draw past the control either way), and on the window's last draw list it blanks the whole window",
                    );
                }
                for (i, item) in items.iter().enumerate() {
                    self.draw(item, &format!("{path}/draw_list/{i}"));
                }
            } else {
                self.error(
                    path,
                    "NuiDrawList is a static array; bind individual drawing properties",
                );
            }
        }
    }

    fn draw(&mut self, v: &Value, path: &str) {
        let Some(ty) = v["type"].as_i64() else {
            self.error(path, "Draw item needs an integer type");
            return;
        };
        if !(0..=7).contains(&ty) {
            self.warn(path, "Unknown draw item preserved without validation");
            return;
        }
        let arrays = v["arrayBinds"].as_bool().unwrap_or(false);
        self.static_property(v, "arrayBinds", path, Kind::Bool);
        for k in ["order", "render"] {
            self.static_property(v, k, path, Kind::Int);
        }
        for k in ["enabled", "fill"] {
            self.property(v, k, path, Kind::Bool, true, arrays);
        }
        self.property(v, "color", path, Kind::Color, true, arrays);
        for k in ["line_thickness", "radius", "amin", "amax"] {
            self.property(v, k, path, Kind::Number, true, arrays);
        }
        for k in ["rect", "image_region"] {
            self.property(v, k, path, Kind::Rect, false, arrays);
        }
        for k in ["a", "b", "ctrl0", "ctrl1"] {
            self.property(v, k, path, Kind::Vec2, false, arrays);
        }
        // The stock include documents the arc's center as Rect. Preserve it
        // without enforcing a guessed shape (client behavior needs a probe).
        for k in ["text", "font", "image"] {
            self.property(v, k, path, Kind::Text, false, arrays);
        }
        for k in ["image_aspect", "image_halign", "image_valign"] {
            self.property(v, k, path, Kind::Int, false, arrays);
        }
        if ty == 0 {
            self.property(v, "points", path, Kind::FloatPairs, false, arrays);
        }
    }
}
