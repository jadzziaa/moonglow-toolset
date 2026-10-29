//! The checks of a window: what the game would refuse or show wrong.
//!
//! [`validate`] holds a layout and its settings to the shapes the
//! `nw_inc_nui` constructors build (which arguments are fixed and which a
//! bind may give, the kind of value each takes: the tables of
//! [`crate::properties`]) and to what the client is
//! known to do with them (the comments name the game version each was seen
//! in). It describes that contract and does not run a window. Widgets,
//! fields and draw items it does not know are warned of and left alone:
//! only the client can say what they do.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::properties::{self, Kind, Property, Takes};
use crate::{Action, ELEMENTS, EVENT_TYPES, Names, Settings};

/// How bad a finding is: the module doctor's, so that a window's findings
/// read as `mg verify`'s do.
pub use mg_module::doctor::Severity;

/// One finding of [`validate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    /// Where: a JSON pointer into the layout (`/root/children/0/value`),
    /// or into the settings (`/bindings`, `/views/NAME/…`, `/actions`).
    pub path: String,
    pub message: String,
}

/// Checks a window's layout and settings; the findings in the order found.
/// An error is something the game refuses or [`crate::generate`] cannot
/// build, a warning something likely wrong. The document is read once,
/// whatever the number of handlers.
pub fn validate(window: &Value, settings: &Settings) -> Vec<Diagnostic> {
    let mut c = Check { settings, findings: Vec::new(), ids: BTreeSet::new(), uses: Vec::new() };
    if !window.is_object() {
        c.error("", "NuiWindow must be a JSON object");
        return c.findings;
    }
    if window.get("version").and_then(Value::as_u64) != Some(1) {
        c.error("/version", "Stock NuiWindow uses integer version 1");
    }
    match window.get("root") {
        // (A root that is no row, column or group, as `nw_inc_nui` makes
        // them, is shown all the same: a label, a button and a spacer in NWN
        // EE 8193.37, `root_*` in the client test.)
        Some(root) => c.element(root, "/root", false),
        None => c.error("/root", "Missing window layout"),
    }
    // NWN EE 8193.37: with Clip to control on the last draw list painted,
    // the whole window shows blank (its own layers aren't clipped either).
    if let Some(root) = window.get("root") {
        c.blank(root, "/root", "NWN shows this window blank: the last draw layers in it have Clip to control on. Turn it off (it clips nothing in the game)");
    }
    // A variant that replaces the whole window is its last drawing when shown.
    for (name, variant) in &settings.variants {
        if settings.variant_targets.get(name).is_some_and(|t| t == "_window_") {
            c.blank(variant, &format!("/views/{name}"), "NWN shows the window blank when this variant is shown: its last draw layers have Clip to control on. Turn it off (it clips nothing in the game)");
        }
    }
    // (Its version is said above.)
    for p in properties::WINDOW.iter().filter(|p| p.name != "version") {
        c.table(window, p, "", false);
    }
    // The names of the layout and of each variant, read once each.
    let mut names = Names::of(window);
    for name in &names.binds {
        if !settings.bindings.contains_key(name) {
            c.error(
                "/bindings",
                &format!("Bind {name:?} needs an explicit initial value in this generator"),
            );
        }
    }
    for name in settings.bindings.keys() {
        if name.is_empty() || name.contains('\0') {
            c.error("/bindings", "Bind names must be nonempty and contain no NUL");
        } else if !plain_name(name) {
            c.error(
                "/bindings",
                &format!("Bind name {name:?} starts or ends with a space, or holds a line break or tab: the event script can't name it"),
            );
        }
    }
    if settings.from_resref {
        c.warn("", "NuiCreateFromResRef needs this JUI on every player's client (for example in a distributed HAK)");
    }
    for (name, variant) in &settings.variants {
        c.ids.clear();
        c.element(variant, &format!("/views/{name}"), false);
        let Names { binds, ids, groups } = Names::of(variant);
        names.ids.extend(ids);
        names.groups.extend(groups);
        for bind in binds {
            if !settings.bindings.contains_key(&bind) {
                c.error("/views", &format!("Variant {name} needs initial bind {bind}"));
            }
        }
    }
    // What is wrong before any handler runs: a handler that sets a bind is
    // held to cause only what is not here already.
    let before: BTreeSet<(String, String)> = c
        .findings
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| (d.path.clone(), d.message.clone()))
        .collect();
    let mut handlers = BTreeSet::new();
    for handler in &settings.handlers {
        let window_event = matches!(handler.event.as_str(), "open" | "close");
        if !EVENT_TYPES.contains(&handler.event.as_str()) {
            c.error("/actions", "Unsupported event type");
        }
        // The script does not ask whose event a window's own is.
        let element = if window_event { "" } else { handler.element.as_str() };
        if !handlers.insert((handler.event.as_str(), element)) {
            c.error("/actions", "Two handlers for the same event of the same control");
        }
        if handler.event == "watch" {
            if !settings.bindings.contains_key(&handler.element) {
                c.error("/actions", "A Value changed handler needs an existing bind");
            }
        } else if !window_event && !names.ids.contains(&handler.element) {
            c.error("/actions", &format!("Unknown element ID: {}", handler.element));
        }
        match &handler.action {
            Action::Code | Action::Close => {}
            Action::Other(raw) => c.error(
                "/actions",
                &format!(
                    "Unknown handler action {raw}: kept as it is, with no code written for it"
                ),
            ),
            Action::Variant { group, variant: view } => {
                if group != "_window_" && !names.groups.contains(group) {
                    c.error("/actions", "A swap layout must name a Group ID");
                }
                if !settings.variants.contains_key(view) {
                    c.error("/actions", "Unknown swap layout variant");
                }
            }
            Action::Toggle { bind } | Action::Set { bind, .. } => {
                if !settings.bindings.contains_key(bind) {
                    c.error("/actions", "A handler needs an existing bind to set");
                }
                if handler.event == "watch" && bind == &handler.element {
                    c.error("/actions", "A Value changed handler cannot set the bind it watches");
                }
                if matches!(&handler.action, Action::Toggle { .. })
                    && settings.bindings.get(bind).is_some_and(|b| {
                        !b.value.is_boolean()
                            && !b.value.as_array().is_some_and(|a| a.iter().all(Value::is_boolean))
                    })
                {
                    c.error("/actions", "Toggling a bind needs a boolean or boolean array");
                }
            }
        }
    }
    // A new window's Close button comes with the handler that closes the
    // window; with that handler removed the button does nothing in the game.
    if names.ids.contains("mg_close")
        && !settings.handlers.iter().any(|r| r.event == "click" && r.element == "mg_close")
    {
        c.warn(
            "/actions",
            "The mg_close button has no Clicked handler: in game it does nothing. Add one that \
             closes the window.",
        );
    }
    if settings.window_id.as_ref().is_some_and(|id| id.is_empty() || id.contains('\0')) {
        c.error("/settings/window_id", "Window ID must be nonempty and contain no NUL");
    }
    // (Any other ID is the window's: NWN EE 8193.37 opens and finds one
    // with punctuation, spaces or letters outside ASCII, `id_punct` and
    // `id_unicode` in the client test.)
    // Handlers of watched binds that set each other's binds run without
    // end in the client: each circle is said once.
    let sets: BTreeMap<&str, &str> = settings
        .handlers
        .iter()
        .filter_map(|h| match &h.action {
            Action::Set { bind, .. } | Action::Toggle { bind } if h.event == "watch" => {
                Some((h.element.as_str(), bind.as_str()))
            }
            _ => None,
        })
        .collect();
    for circle in circles(&sets) {
        c.error(
            "/actions",
            &format!(
                "Value changed handlers form a recursive bind cycle: {}",
                circle.join(" \u{2192} ")
            ),
        );
    }
    // The value a handler sets is held to every place its bind is read (the
    // window's own properties too). Only what the new value causes is the
    // handler's: the rest is said once above.
    for handler in &settings.handlers {
        let Action::Set { bind, value } = &handler.action else { continue };
        let caused: BTreeSet<(&str, String)> = c
            .uses
            .iter()
            .filter(|u| u.bind == bind && !u.rule.accepts(value))
            .map(|u| (u.path.as_str(), u.rule.message()))
            .filter(|(path, message)| !before.contains(&((*path).to_owned(), message.clone())))
            .collect();
        for (path, message) in caused {
            c.findings.push(Diagnostic {
                severity: Severity::Error,
                path: "/actions".into(),
                message: format!("Set {bind}: {message} at {path}"),
            });
        }
    }
    // One cause, one finding: an error says what a warning on the same
    // property would.
    let errors: BTreeSet<String> = c
        .findings
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.path.clone())
        .collect();
    c.findings.retain(|d| d.severity == Severity::Error || !errors.contains(&d.path));
    c.findings
}

/// The circles of `sets` (a watched bind, and the bind its handler sets),
/// each once, from its first name in order.
fn circles<'a>(sets: &BTreeMap<&'a str, &'a str>) -> BTreeSet<Vec<&'a str>> {
    let mut found = BTreeSet::new();
    for start in sets.keys() {
        let mut seen = BTreeSet::new();
        let mut at = *start;
        while let Some(next) = sets.get(at) {
            if !seen.insert(at) {
                // `at` is on a circle: its members, turned to start at the first.
                let mut circle = vec![at];
                let mut member = *next;
                while member != at {
                    circle.push(member);
                    member = sets[member];
                }
                let first = (0..circle.len()).min_by_key(|i| circle[*i]).unwrap_or(0);
                circle.rotate_left(first);
                circle.push(circle[0]);
                found.insert(circle);
                break;
            }
            at = next;
        }
    }
    found
}

/// Clip to control on, literally or by its bind's initial value.
fn scissor_on(node: &Value, settings: &Settings) -> bool {
    let v = &node["draw_list_scissor"];
    let v = v["bind"].as_str().and_then(|b| settings.bindings.get(b)).map_or(v, |b| &b.value);
    node["draw_list"].is_array() && *v == true
}

/// The element whose draw list is painted last, in layout order. A draw
/// list with nothing in it paints nothing, and is not the last for the
/// client (NWN EE 8193.37: `clip_empty` in the client test).
fn last_draw_list<'a>(v: &'a Value, path: &str) -> Option<(String, &'a Value)> {
    let painted = v["draw_list"].as_array().is_some_and(|items| !items.is_empty());
    let mut last = painted.then(|| (path.to_owned(), v));
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

/// What a property's value has to be.
#[derive(Clone, Copy)]
enum Rule {
    /// A value of a kind.
    Value {
        kind: Kind,
        /// Null will do (and false, for the window's title).
        nullable: bool,
        title: bool,
        /// The value comes from a bind: a number will do for text.
        bound: bool,
        /// A bind in a list's row: an array, a value a row.
        rows: bool,
    },
    /// Clip to control, on the draw list painted last: anything but true.
    Unclipped(&'static str),
}

impl Rule {
    fn accepts(self, value: &Value) -> bool {
        match self {
            Rule::Value { kind, nullable, title, bound, rows } => {
                let valid = |v: &Value| {
                    kind.matches(v)
                        || (matches!(kind, Kind::Text) && bound && v.is_number())
                        || (nullable && (v.is_null() || (title && v == false)))
                };
                if rows {
                    value.as_array().is_some_and(|a| a.iter().all(valid))
                } else {
                    valid(value)
                }
            }
            Rule::Unclipped(_) => *value != true,
        }
    }

    fn message(self) -> String {
        match self {
            Rule::Value { kind, rows, .. } => format!(
                "Expected {}{} according to nw_inc_nui",
                kind.label(),
                if rows { " array for list rows" } else { "" }
            ),
            Rule::Unclipped(message) => message.into(),
        }
    }
}

/// A place where a bind's value is read, and what it has to be there: what
/// a handler that sets the bind is checked against.
struct Use<'a> {
    bind: &'a str,
    path: String,
    rule: Rule,
}

/// One pass over a window: the findings, and what it learns on the way.
struct Check<'a> {
    settings: &'a Settings,
    findings: Vec<Diagnostic>,
    /// The IDs seen in the layout being read (the window, or one variant).
    ids: BTreeSet<String>,
    uses: Vec<Use<'a>>,
}

impl<'a> Check<'a> {
    /// An error if the draw list painted last in `layout` has Clip to
    /// control on, by itself or by its bind's first value.
    fn blank(&mut self, layout: &'a Value, path: &str, message: &'static str) {
        let Some((path, node)) = last_draw_list(layout, path) else { return };
        let path = format!("{path}/draw_list_scissor");
        let rule = Rule::Unclipped(message);
        let flag = &node["draw_list_scissor"];
        let bound = flag["bind"].as_str().filter(|b| self.settings.bindings.contains_key(*b));
        if scissor_on(node, self.settings) {
            self.error(&path, message);
        }
        if let Some(bind) = bound {
            self.uses.push(Use { bind, path, rule });
        }
    }

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
    /// Literal values outside what the stock constants allow, of the
    /// properties of `table` that `ty` has.
    fn ranges(&mut self, v: &Value, path: &str, ty: &str, table: &[Property]) {
        let keys = table.iter().filter(|p| p.of(ty)).filter_map(|p| Some((p.name, p.constants?)));
        for (key, (low, high, constants)) in keys {
            if let Some(n) = v.get(key).and_then(Value::as_i64)
                && !(low..=high).contains(&n)
            {
                self.warn(
                    &format!("{path}/{key}"),
                    &format!(
                        "{n} is none of the {constants} values ({low} to {high}); the game \
                         shows the window all the same"
                    ),
                );
            }
        }
    }

    /// A literal picture name the game can look up: at most 16 characters.
    fn picture_name(&mut self, v: &Value, key: &str, path: &str) {
        if let Some(name) = v.get(key).and_then(Value::as_str)
            && name.chars().count() > 16
        {
            self.warn(
                &format!("{path}/{key}"),
                &format!(
                    "{name:?} is longer than 16 characters: the game finds no picture by that name"
                ),
            );
        }
    }

    fn warn(&mut self, path: &str, message: &str) {
        self.findings.push(Diagnostic {
            severity: Severity::Warning,
            path: path.into(),
            message: message.into(),
        });
    }
    /// Holds the property `k` of `obj` to `kind`: its value, or the first
    /// value of the bind it is read from (`array_bind`: in a list's row,
    /// where a bind holds a value a row).
    fn property(
        &mut self,
        obj: &'a Value,
        k: &str,
        path: &str,
        kind: Kind,
        nullable: bool,
        array_bind: bool,
    ) {
        let Some(v) = obj.get(k) else { return };
        let p = format!("{path}/{k}");
        let bind = v.get("bind");
        let rule = Rule::Value {
            kind,
            nullable,
            title: k == "title",
            bound: bind.is_some_and(Value::is_string),
            rows: array_bind && bind.is_some(),
        };
        let value = if let Some(name) = bind {
            for key in ["number_flags", "number_precision", "text_flags"] {
                self.static_property(v, key, &p, Kind::Int);
            }
            let Some(name) = name.as_str() else {
                self.error(&p, "Bind must name a string");
                return;
            };
            let Some(b) = self.settings.bindings.get(name) else { return };
            self.uses.push(Use { bind: name, path: p.clone(), rule });
            &b.value
        } else {
            v
        };
        if !rule.accepts(value) {
            self.error(&p, &rule.message());
        }
    }
    /// Holds `obj`'s property of the table `p` to what it takes: a static
    /// value, a modifier's number, or a value or bind (`per_row`: in a
    /// list's row, where a bind gives a value a row).
    fn table(&mut self, obj: &'a Value, p: &Property, path: &str, per_row: bool) {
        match p.takes {
            Takes::Static => self.static_property(obj, p.name, path, p.kind),
            Takes::Modifier => {
                if let Some(n) = obj.get(p.name)
                    && !n.is_number()
                {
                    self.error(
                        &format!("{path}/{}", p.name),
                        "Geometry modifiers are static numbers; nw_inc_nui does not support binds here",
                    );
                }
            }
            Takes::Bound => {
                // (A row count written in is a number; from a bind, an
                // array's length will do.)
                let kind = if p.kind == Kind::RowCount && obj[p.name].get("bind").is_none() {
                    Kind::Int
                } else {
                    p.kind
                };
                self.property(obj, p.name, path, kind, p.nullable, per_row && p.per_row);
            }
        }
    }

    /// Checks an element and what it holds; its place is `path`.
    fn element(&mut self, v: &'a Value, path: &str, array_bind: bool) {
        let Some(ty) = v.get("type").and_then(Value::as_str) else {
            self.error(path, "Element needs a string type");
            return;
        };
        self.ranges(v, path, ty, properties::ELEMENT);
        let picture = match ty {
            "image" => Some("value"),
            "button_image" => Some("label"),
            _ => None,
        };
        if let Some(key) = picture {
            self.picture_name(v, key, path);
        }
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
                if !plain_name(id) {
                    self.error(
                        path,
                        &format!("Element ID {id:?} starts or ends with a space, or holds a line break or tab: the event script can't name it"),
                    );
                }
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
        for p in properties::ELEMENT.iter().filter(|p| p.of(ty)) {
            self.table(v, p, path, array_bind);
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
                "What the player enters here is lost: bind its value to keep it and read it in the event script",
            );
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
        // (Unlike NuiCombo's, Options' and Tabs' labels are the array
        // itself at construction: a bind there fails in the client's parser.
        // The table has them static.)
        if ty == "chart"
            && let Some(slots) = v["value"].as_array()
        {
            for (i, slot) in slots.iter().enumerate() {
                let p = format!("{path}/value/{i}");
                if !slot.is_object() {
                    self.error(&p, "NuiChartSlot must be an object");
                    continue;
                }
                for property in properties::CHART_SLOT {
                    self.table(slot, property, &p, array_bind);
                }
                self.ranges(slot, &p, "", properties::CHART_SLOT);
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
                    if cell[1].as_f64().is_some_and(|w| w < 0.0) {
                        self.warn(
                            &format!("{p}/1"),
                            "A cell's width is 0 for automatic or a number of pixels, not below 0",
                        );
                    }
                    self.element(&cell[0], &format!("{p}/0"), true);
                }
            } else {
                self.error(path, "List needs a row_template array");
            }
        }
        if v.get("draw_list").is_some() {
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

    /// Checks one item of a draw list.
    fn draw(&mut self, v: &'a Value, path: &str) {
        let Some(ty) = v["type"].as_i64() else {
            self.error(path, "Draw item needs an integer type");
            return;
        };
        if !(0..=7).contains(&ty) {
            self.warn(path, "Unknown draw item preserved without validation");
            return;
        }
        let arrays = v["arrayBinds"].as_bool().unwrap_or(false);
        let ty = ty.to_string();
        for p in properties::DRAW.iter().filter(|p| p.of(&ty)) {
            self.table(v, p, path, arrays);
        }
        if v["order"].as_i64().is_some_and(|o| o != -1 && o != 1) {
            self.warn(
                &format!("{path}/order"),
                "Paint order is before (-1) or after (1) the control \
                 (NUI_DRAW_LIST_ITEM_ORDER_*); the game draws the item all the same",
            );
        }
        self.ranges(v, path, &ty, properties::DRAW);
        if ty == "5" {
            self.picture_name(v, "image", path);
        }
        // The stock include documents the arc's center as a rectangle; NWN
        // EE 8193.37 draws the arc the same with a point there (`arc_point`
        // and `arc_rect` in the client test), so either is kept as it is.
    }
}

/// Whether the event script's section lines can carry `name` (a control's
/// ID, a bind's or a variant's name): no space at either end, no line break
/// or other control character.
pub fn plain_name(name: &str) -> bool {
    name.trim() == name && !name.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{Binding, Handler, template, window};

    fn errors(w: &Value, s: &Settings) -> Vec<Diagnostic> {
        validate(w, s).into_iter().filter(|d| d.severity == Severity::Error).collect()
    }

    /// Whether a finding of `severity` says `what`.
    fn says(w: &Value, s: &Settings, severity: Severity, what: &str) -> bool {
        validate(w, s).iter().any(|d| d.severity == severity && d.message.contains(what))
    }

    fn error(w: &Value, s: &Settings, what: &str) -> bool {
        says(w, s, Severity::Error, what)
    }

    fn warning(w: &Value, s: &Settings, what: &str) -> bool {
        says(w, s, Severity::Warning, what)
    }

    /// A window with these in its column.
    fn with(children: Value) -> Value {
        let mut w = window();
        w["root"]["children"] = children;
        w
    }

    fn binds(values: &[(&str, Value)]) -> Settings {
        let mut s = Settings::default();
        for (name, value) in values {
            s.bindings
                .insert((*name).into(), Binding { value: value.clone(), ..Default::default() });
        }
        s
    }

    fn click(element: &str, action: Action) -> Handler {
        Handler::new("click", element, action)
    }

    fn watch(bind: &str, action: Action) -> Handler {
        Handler::new("watch", bind, action)
    }

    #[test]
    fn a_new_window_has_no_findings() {
        let mut s = Settings::default();
        s.handlers.push(click("mg_close", Action::Close));
        assert_eq!(validate(&window(), &s), []);
    }

    #[test]
    fn every_widget_as_its_template_makes_it_has_no_errors() {
        for ty in ELEMENTS {
            let found = errors(&with(json!([template(ty)])), &Settings::default());
            assert!(found.is_empty(), "{ty}: {found:?}");
        }
    }

    #[test]
    fn a_layout_that_is_no_object_is_one_error() {
        for w in [json!([]), json!(null), json!("window")] {
            let found = validate(&w, &Settings::default());
            assert_eq!(found.len(), 1);
            assert_eq!(found[0].message, "NuiWindow must be a JSON object");
        }
    }

    #[test]
    fn a_widget_the_crate_does_not_know_is_warned_of() {
        let mut w = window();
        w["root"] = json!({"type": "future", "payload": [2.5, {"a": 1}]});
        assert!(warning(&w, &Settings::default(), "Unknown widget"));
    }

    #[test]
    fn a_bind_in_a_list_row_holds_a_value_a_row() {
        let mut w = window();
        w["root"] = template("list");
        w["root"]["row_template"] =
            json!([[{"type":"label","value":{"bind":"names"}}, 150.0, true]]);
        assert_eq!(errors(&w, &binds(&[("names", json!(["A", "B"]))])), []);
        assert!(error(&w, &binds(&[("names", json!("not an array"))]), "array for list"));
    }

    #[test]
    fn a_size_read_from_a_bind_is_an_error() {
        let mut w = window();
        w["root"]["width"] = json!({"bind":"width"});
        assert!(error(&w, &binds(&[("width", json!(100.0))]), "static numbers"));
    }

    #[test]
    fn a_bind_without_a_first_value_is_an_error() {
        let mut w = window();
        w["title"] = json!({"bind":"caption"});
        assert!(error(&w, &Settings::default(), "explicit initial"));
        assert_eq!(errors(&w, &binds(&[("caption", json!("Title"))])), []);
    }

    #[test]
    fn a_variants_bind_without_a_first_value_is_an_error() {
        let mut s = Settings::default();
        s.variants.insert("page".into(), json!({"type":"label","value":{"bind":"caption"}}));
        let found = errors(&window(), &s);
        assert!(
            found
                .iter()
                .any(|d| d.path == "/views"
                    && d.message == "Variant page needs initial bind caption")
        );
    }

    /// NWN EE 8193.37 (np_scissor, np_scissor2, nui_draw_a/b/dyn): Clip to
    /// control clips nothing, and on the window's last draw list it blanks the
    /// whole window. Later lists without it bring the window back.
    #[test]
    fn clip_to_control_on_the_last_draw_list_blanks_the_window() {
        let spacer = |clip: Value| {
            json!({"type":"spacer","width":200.0,"height":60.0,"draw_list_scissor":clip,
                "draw_list":[{"type":7,"enabled":true,"rect":{"x":-20.0,"y":-20.0,"w":240.0,"h":100.0}}]})
        };
        let check = |children: Value, s: &Settings| {
            let w = with(children);
            (error(&w, s, "blank"), warning(&w, s, "clips nothing"))
        };
        let s = Settings::default();
        // Shown blank in the game.
        assert_eq!(
            check(json!([spacer(json!(true)), spacer(json!(false)), spacer(json!(true))]), &s),
            (true, true)
        );
        assert_eq!(
            check(
                json!([{"type":"button","label":"b","draw_list_scissor":true,
                    "draw_list":[{"type":7,"rect":{"x":0.0,"y":0.0,"w":9.0,"h":9.0}}]}]),
                &s
            ),
            // The error says what the warning on the same property would.
            (true, false)
        );
        // Shown, unclipped.
        assert_eq!(check(json!([spacer(json!(true)), spacer(json!(false))]), &s), (false, true));
        assert_eq!(check(json!([spacer(json!(false)), spacer(json!(false))]), &s), (false, false));
        // A bind's initial value counts.
        let bound = binds(&[("clip", json!(true))]);
        assert_eq!(check(json!([spacer(json!({"bind":"clip"}))]), &bound), (true, false));
    }

    #[test]
    fn a_whole_window_variant_whose_last_draw_list_clips_is_an_error() {
        let mut s = Settings::default();
        s.variants.insert(
            "cover".into(),
            json!({"type":"col","children":[],"draw_list_scissor":true,
            "draw_list":[{"type":6,"order":1,"render":0,"a":{"x":0.0,"y":0.0},"b":{"x":1.0,"y":1.0}}]}),
        );
        assert!(!error(&window(), &s, "when this variant is shown"), "shown in a group");
        s.variant_targets.insert("cover".into(), "_window_".into());
        assert!(error(&window(), &s, "when this variant is shown"));
    }

    #[test]
    fn checking_changes_neither_the_layout_nor_the_settings() {
        let w = with(json!([{"type":"check","id":"flag","label":"F","value":{"bind":"a"}}]));
        let mut s = binds(&[("a", json!(false))]);
        s.handlers.push(click("flag", Action::Set { bind: "a".into(), value: json!("no") }));
        let (before_w, before_s) = (w.clone(), s.clone());
        validate(&w, &s);
        assert_eq!((w, s), (before_w, before_s));
    }

    #[test]
    fn a_lists_row_count_may_be_a_bind_holding_the_rows() {
        let mut w = window();
        w["root"] = template("list");
        w["root"]["row_count"] = json!({"bind":"names"});
        w["root"]["row_template"] =
            json!([[{"type":"label","value":{"bind":"names"}}, 150.0, true]]);
        assert_eq!(errors(&w, &binds(&[("names", json!(["one", "two"]))])), []);
    }

    #[test]
    fn a_lists_row_count_written_as_an_array_is_an_error() {
        let mut w = window();
        w["root"] = template("list");
        w["root"]["row_count"] = json!([1, 2]);
        assert!(errors(&w, &Settings::default()).iter().any(|d| d.path == "/root/row_count"));
    }

    #[test]
    fn options_take_labels_and_not_a_combos_entries() {
        let mut w = window();
        w["root"] = template("options");
        w["root"]["elements"] = json!([["not a combo", 0]]);
        assert!(error(&w, &Settings::default(), "string labels"));
    }

    #[test]
    fn a_draw_lists_text_may_be_any_text() {
        let mut w = window();
        w["root"]["draw_list"] = json!([{"type":4,"enabled":true,"color":{"r":0,"g":0,"b":0,"a":255},"fill":null,"line_thickness":null,"rect":{"x":0.0,"y":0.0,"w":100.0,"h":30.0},"text":"żółć","font":"","arrayBinds":false}]);
        assert_eq!(errors(&w, &Settings::default()), []);
    }

    #[test]
    fn a_polylines_points_are_a_flat_array_of_number_pairs() {
        let mut w = window();
        w["root"]["draw_list"] = json!([{"type":0,"points":[0.0,0.0,20.0,30.0]}]);
        assert_eq!(errors(&w, &Settings::default()), []);
        w["root"]["draw_list"][0]["points"] = json!([{"x":0,"y":0}]);
        assert!(error(&w, &Settings::default(), "number pairs"));
    }

    #[test]
    fn watched_binds_whose_handlers_set_each_other_are_an_error() {
        let mut s = binds(&[("a", json!(false)), ("b", json!(false))]);
        s.handlers = vec![
            watch("a", Action::Toggle { bind: "b".into() }),
            watch("b", Action::Toggle { bind: "a".into() }),
        ];
        assert!(error(&window(), &s, "recursive bind cycle: a \u{2192} b \u{2192} a"));
    }

    #[test]
    fn a_circle_of_watched_binds_is_said_once_whatever_leads_into_it() {
        let names = ["a", "b", "c", "lead", "p", "q"];
        let mut s = binds(&names.map(|n| (n, json!(false))));
        let toggle = |from: &str, to: &str| watch(from, Action::Toggle { bind: to.into() });
        s.handlers = vec![
            toggle("a", "b"),
            toggle("b", "c"),
            toggle("c", "a"),
            // Into the circle, not of it.
            toggle("lead", "b"),
            // Another circle.
            toggle("p", "q"),
            toggle("q", "p"),
        ];
        let said: Vec<String> = errors(&window(), &s)
            .into_iter()
            .filter(|d| d.message.contains("recursive"))
            .map(|d| d.message)
            .collect();
        assert_eq!(said.len(), 2, "{said:?}");
        assert!(said[0].ends_with("a \u{2192} b \u{2192} c \u{2192} a"), "{said:?}");
        assert!(said[1].ends_with("p \u{2192} q \u{2192} p"), "{said:?}");
    }

    #[test]
    fn watched_binds_in_a_line_are_no_circle() {
        let mut s = binds(&[("a", json!(false)), ("b", json!(false)), ("c", json!(false))]);
        s.handlers = vec![
            watch("a", Action::Toggle { bind: "b".into() }),
            watch("b", Action::Toggle { bind: "c".into() }),
        ];
        assert_eq!(errors(&window(), &s), []);
    }

    #[test]
    fn a_handler_cannot_set_the_bind_it_watches() {
        let mut s = binds(&[("a", json!(false))]);
        s.handlers = vec![watch("a", Action::Toggle { bind: "a".into() })];
        assert!(error(&window(), &s, "cannot set the bind it watches"));
    }

    #[test]
    fn a_value_set_of_the_wrong_kind_is_the_handlers_error() {
        let w = with(json!([{"type":"check","id":"flag","label":"Flag","value":{"bind":"a"}}]));
        let mut s = binds(&[("a", json!(false))]);
        s.handlers = vec![click("flag", Action::Set { bind: "a".into(), value: json!("wrong") })];
        let found = errors(&w, &s);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].path, "/actions");
        assert_eq!(
            found[0].message,
            "Set a: Expected boolean according to nw_inc_nui at /root/children/0/value"
        );
        s.handlers = vec![click("flag", Action::Set { bind: "a".into(), value: json!(true) })];
        assert_eq!(errors(&w, &s), []);
    }

    #[test]
    fn a_value_set_is_held_to_the_windows_own_properties() {
        let mut w = with(json!([{"type":"button","id":"flag","label":"b"}]));
        w["geometry"] = json!({"bind":"bounds"});
        let mut s = binds(&[("bounds", json!({"x":-1.0,"y":-1.0,"w":420.0,"h":240.0}))]);
        s.handlers = vec![click("flag", Action::Set { bind: "bounds".into(), value: json!(true) })];
        assert!(error(&w, &s, "Set bounds: Expected rectangle {x,y,w,h}"));
        assert!(error(&w, &s, "at /geometry"));
    }

    #[test]
    fn a_value_set_is_held_to_every_place_its_bind_is_read() {
        let mut w = with(json!([
            {"type":"button","id":"go","label":{"bind":"text"}},
            {"type":"list","row_count":1,"row_height":25.0,"scrollbars":2,
             "row_template":[[{"type":"label","value":{"bind":"rows"}},0.0,true]]},
            {"type":"spacer","draw_list":[{"type":4,"text":{"bind":"text"},
                "rect":{"x":0.0,"y":0.0,"w":9.0,"h":9.0}}]}
        ]));
        w["title"] = json!({"bind":"text"});
        let mut s = binds(&[("text", json!("t")), ("rows", json!(["a", "b"]))]);
        s.variants.insert("page".into(), json!({"type":"label","value":{"bind":"text"}}));
        assert_eq!(errors(&w, &s), []);
        s.handlers = vec![
            click("go", Action::Set { bind: "text".into(), value: json!({"no":1}) }),
            click("go2", Action::Set { bind: "rows".into(), value: json!("one") }),
        ];
        let caused: Vec<String> = errors(&w, &s)
            .into_iter()
            .filter(|d| d.message.starts_with("Set "))
            .map(|d| d.message)
            .collect();
        let text = "Set text: Expected string or StrRef according to nw_inc_nui at ";
        assert_eq!(
            caused,
            [
                format!("{text}/root/children/0/label"),
                format!("{text}/root/children/2/draw_list/0/text"),
                format!("{text}/title"),
                format!("{text}/views/page/value"),
                "Set rows: Expected string or StrRef array for list rows according to nw_inc_nui at /root/children/1/row_template/0/0/value".to_owned(),
            ]
        );
        // A number will do for text that comes from a bind; null for a title.
        s.handlers = vec![click("go", Action::Set { bind: "text".into(), value: json!(7) })];
        assert_eq!(errors(&w, &s), []);
    }

    #[test]
    fn a_value_set_that_clips_the_last_draw_list_is_the_handlers_error() {
        let w = with(
            json!([{"type":"button","id":"go","label":"b","draw_list_scissor":{"bind":"clip"},
            "draw_list":[{"type":7,"rect":{"x":0.0,"y":0.0,"w":9.0,"h":9.0}}]}]),
        );
        let mut s = binds(&[("clip", json!(false))]);
        assert_eq!(errors(&w, &s), []);
        s.handlers = vec![click("go", Action::Set { bind: "clip".into(), value: json!(true) })];
        let found = errors(&w, &s);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].message.starts_with("Set clip: NWN shows this window blank"));
        assert!(found[0].message.ends_with(" at /root/children/0/draw_list_scissor"));
    }

    #[test]
    fn a_problem_a_value_set_does_not_cause_is_said_once() {
        let mut w = with(json!([{"type":"button","id":"flag","label":"b"}]));
        w["geometry"] = json!({"bind":"bounds"});
        w["title"] = json!(7);
        let mut s = binds(&[("bounds", json!({"x":-1.0,"y":-1.0,"w":420.0,"h":240.0}))]);
        s.handlers = (0..3)
            .map(|i| {
                let value = json!({"x":i,"y":0,"w":9,"h":9});
                Handler::new("click", "flag", Action::Set { bind: "bounds".into(), value })
            })
            .collect();
        let found = validate(&w, &s);
        assert_eq!(found.iter().filter(|d| d.path == "/title").count(), 1);
        assert!(!found.iter().any(|d| d.message.contains("Set bounds")), "{found:?}");
    }

    #[test]
    fn a_value_that_is_wrong_already_is_not_the_handlers_error() {
        let w = with(json!([{"type":"check","id":"flag","label":"Flag","value":{"bind":"a"}}]));
        let mut s = binds(&[("a", json!("wrong already"))]);
        s.handlers = vec![click("flag", Action::Set { bind: "a".into(), value: json!(3) })];
        let found = errors(&w, &s);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].path, "/root/children/0/value");
    }

    /// NWN EE 8193.37 (nui_ranges_s, nui_disable_s): a disabled slider dragged
    /// or clicked sets its bind to its minimum.
    #[test]
    fn a_disabled_slider_that_writes_a_bind_is_warned_of() {
        let warned = |slider: Value| {
            let s = binds(&[("v", json!(5)), ("on", json!(true))]);
            warning(&with(json!([slider])), &s, "disabled slider")
        };
        let slider = |enabled: Value| json!({"type":"slider","value":{"bind":"v"},"min":0,"max":10,"step":1,"enabled":enabled});
        assert!(warned(slider(json!(false))));
        assert!(warned(slider(json!({"bind":"on"}))));
        assert!(!warned(slider(json!(true))));
        assert!(!warned(json!({"type":"slider","value":{"bind":"v"},"min":0,"max":10,"step":1})));
        assert!(!warned(
            json!({"type":"slider","value":5,"min":0,"max":10,"step":1,"enabled":false})
        ));
    }

    /// NWN EE 8193.37, np_bsel: a Toggle button with a fixed value doesn't switch.
    #[test]
    fn a_toggle_button_with_a_fixed_value_is_warned_of() {
        let fixed = |value: Value| {
            let w = with(json!([{"type":"button_select","label":"t","value":value}]));
            warning(&w, &binds(&[("on", json!(false))]), "can't be switched")
        };
        assert!(fixed(json!(false)));
        assert!(!fixed(json!({"bind":"on"})));
    }

    #[test]
    fn an_id_the_event_script_cannot_name_is_an_error() {
        for id in ["ok ", " ok", "o\nk", "o\tk"] {
            let w = with(json!([{"type":"button","id":id,"label":"b"}]));
            assert!(error(&w, &Settings::default(), "can't name it"), "{id:?}");
        }
        for id in ["ok", "o k", "ok+", "Żółć"] {
            let w = with(json!([{"type":"button","id":id,"label":"b"}]));
            assert!(!error(&w, &Settings::default(), "can't name it"), "{id:?}");
        }
    }

    #[test]
    fn a_bind_name_the_event_script_cannot_name_is_an_error() {
        assert!(error(&window(), &binds(&[("flag ", json!(true))]), "can't name it"));
        assert!(error(&window(), &binds(&[("", json!(true))]), "nonempty"));
    }

    #[test]
    fn an_id_twice_in_one_layout_is_warned_of() {
        let w = with(json!([
            {"type":"button","id":"same","label":"a"},
            {"type":"button","id":"same","label":"b"}
        ]));
        assert!(warning(&w, &Settings::default(), "Repeated element id"));
    }

    #[test]
    fn a_value_none_of_the_stock_constants_has_is_warned_of() {
        let w = with(json!([{"type":"image","value":"a","image_aspect":9}]));
        assert!(warning(&w, &Settings::default(), "NUI_ASPECT_*"));
    }

    #[test]
    fn a_picture_name_longer_than_a_resref_is_warned_of() {
        let w = with(json!([{"type":"image","value":"a_name_of_twenty_chars"}]));
        assert!(warning(&w, &Settings::default(), "longer than 16 characters"));
    }

    #[test]
    fn a_list_cell_narrower_than_nothing_is_warned_of() {
        let w = with(json!([
            {"type":"list","row_count":1,"row_template":[[{"type":"label","value":"x"},-5.0,false]]}
        ]));
        assert!(warning(&w, &Settings::default(), "not below 0"));
    }

    #[test]
    fn a_draw_items_paint_order_is_before_or_after_the_control() {
        let w = with(json!([
            {"type":"spacer","draw_list":[{"type":6,"order":3,"render":0,"a":{"x":0.0,"y":0.0},"b":{"x":1.0,"y":1.0}}]}
        ]));
        assert!(warning(&w, &Settings::default(), "NUI_DRAW_LIST_ITEM_ORDER_*"));
    }

    /// max counts UTF-8 bytes; below 4 the client can keep part of a character
    /// (NWN EE 8193.37, max 1: "Ż" stores U+0005).
    #[test]
    fn a_text_input_too_short_for_a_utf8_character_is_warned_of() {
        let short = |max: u64| {
            let w = with(
                json!([{"type":"textedit","label":"","value":"","max":max,"multiline":false}]),
            );
            warning(&w, &Settings::default(), "UTF-8 bytes")
        };
        assert!(short(1));
        assert!(short(3));
        assert!(!short(4));
    }

    /// What the client refuses to construct, measured in NWN EE 8193.37 with
    /// windows generated by `mg nui generate` (target/nui-native probes).
    #[test]
    fn a_size_across_a_row_or_column_that_the_client_cannot_fit_is_an_error() {
        let button = |key: &str, size: f64, margin: Option<f64>| {
            let mut b = json!({"type":"button","label":"b","value":null});
            b[key] = json!(size);
            if let Some(m) = margin {
                b["margin"] = json!(m);
            }
            b
        };
        let fails = |root: Value| error(&with(json!([root])), &Settings::default(), "constraint");
        let row = |h: f64, child: Value| json!({"type":"row","height":h,"children":[child]});
        let col = |w: f64, child: Value| json!({"type":"col","width":w,"children":[child]});
        // Refused by the client.
        assert!(fails(row(60.0, button("height", 80.0, None))));
        assert!(fails(row(30.0, button("height", 30.0, None))));
        assert!(fails(row(33.0, button("height", 30.0, None))));
        assert!(fails(row(30.0, button("height", 29.0, None))));
        assert!(fails(col(100.0, button("width", 150.0, None))));
        assert!(fails(col(150.0, button("width", 150.0, None))));
        // Opened by the client.
        assert!(!fails(row(34.0, button("height", 30.0, None))));
        assert!(!fails(row(30.0, button("height", 30.0, Some(0.0)))));
        assert!(!fails(col(154.0, button("width", 150.0, None))));
        assert!(!fails(row(
            60.0,
            json!({"type":"group","height":40.0,"children":[{"type":"label"}]})
        )));
        // Along a row, children may overflow it; a group scrolls a taller child.
        assert!(!fails(json!({"type":"row","width":300.0,"children":[
            button("width", 200.0, None), button("width", 200.0, None)]})));
        assert!(!fails(
            json!({"type":"group","height":60.0,"children":[button("height", 100.0, None)]})
        ));
    }

    #[test]
    fn a_constructors_fixed_argument_read_from_a_bind_is_an_error() {
        let s = binds(&[("dynamic", json!(true))]);
        for (widget, field) in [
            ("group", "border"),
            ("text", "scrollbars"),
            ("list", "row_height"),
            ("options", "direction"),
            ("options", "elements"),
            ("tabbar", "elements"),
            ("textedit", "multiline"),
            ("textedit", "wordwrap"),
        ] {
            let mut w = with(json!([template(widget)]));
            w["root"]["children"][0][field] = json!({"bind":"dynamic"});
            assert!(
                errors(&w, &s)
                    .iter()
                    .any(|d| d.path.ends_with(field) && d.message.contains("static")),
                "{widget}.{field}"
            );
        }
    }

    #[test]
    fn a_windows_border_may_be_read_from_a_bind() {
        let mut w = window();
        w["border"] = json!({"bind":"dynamic"});
        assert_eq!(errors(&w, &binds(&[("dynamic", json!(true))])), []);
    }

    #[test]
    fn options_and_tabs_take_any_number_of_labels_written_out() {
        for ty in ["options", "tabbar"] {
            let mut w = window();
            w["root"] = template(ty);
            for labels in [json!([]), json!(["Only"]), json!(["First", "Second", "Third"])] {
                w["root"]["elements"] = labels;
                assert_eq!(errors(&w, &Settings::default()), [], "{ty}");
            }
        }
    }

    #[test]
    fn a_combos_entries_may_be_read_from_a_bind() {
        let mut w = window();
        w["root"] = template("combo");
        w["root"]["elements"] = json!({"bind":"labels"});
        assert_eq!(errors(&w, &binds(&[("labels", json!([["First", 0], ["Second", 1]]))])), []);
        assert!(!errors(&w, &binds(&[("labels", json!(["First", "Second"]))])).is_empty());
    }

    #[test]
    fn a_chart_slots_data_is_numbers() {
        let mut w = with(json!([template("chart")]));
        w["root"]["children"][0]["value"][0]["data"] = json!({"bind":"points"});
        assert_eq!(errors(&w, &binds(&[("points", json!([1.0, 2.0]))])), []);
        let found = errors(&w, &binds(&[("points", json!([1.0, "bad"]))]));
        assert!(found.iter().any(|d| d.path.ends_with("/data") && d.message.contains("numbers")));
    }

    #[test]
    fn a_pictures_region_is_a_whole_rectangle() {
        let mut w = with(json!([template("image")]));
        w["root"]["children"][0]["image_region"] = json!({"x":0.0,"y":0.0,"w":32.0,"h":32.0});
        assert_eq!(errors(&w, &Settings::default()), []);
        w["root"]["children"][0]["image_region"] = json!({"x":0.0,"y":0.0});
        let found = errors(&w, &Settings::default());
        assert!(found.iter().any(|d| d.path.ends_with("/image_region")));
    }

    #[test]
    fn what_a_bind_is_shown_with_is_a_fixed_whole_number() {
        let mut w = with(
            json!([{"type":"label","value":{"bind":"caption","number_precision":"not an integer"}}]),
        );
        let s = binds(&[("caption", json!(1.25))]);
        assert!(errors(&w, &s).iter().any(|d| d.path.ends_with("/number_precision")));
        w["root"]["children"][0]["value"]["number_precision"] = json!(2);
        assert_eq!(errors(&w, &s), []);
    }

    #[test]
    fn a_handler_of_the_windows_own_event_names_no_control() {
        let mut s = Settings::default();
        s.variants.insert("other".into(), template("col"));
        let show = Action::Variant { group: "_window_".into(), variant: "other".into() };
        s.handlers.push(Handler::new("open", "ignored_a", show));
        assert_eq!(errors(&window(), &s), []);
    }

    #[test]
    fn two_handlers_for_one_event_are_an_error() {
        let mut s = Settings::default();
        s.handlers.push(Handler::new("open", "ignored_a", Action::Code));
        s.handlers.push(Handler::new("open", "ignored_b", Action::Close));
        assert!(error(&window(), &s, "Two handlers for the same event"));
        s.handlers = vec![click("mg_close", Action::Code), click("mg_close", Action::Close)];
        assert!(error(&window(), &s, "Two handlers for the same event"));
    }

    #[test]
    fn a_handler_of_a_control_the_window_does_not_have_is_an_error() {
        let mut s = Settings::default();
        s.handlers.push(click("nowhere", Action::Code));
        assert!(error(&window(), &s, "Unknown element ID: nowhere"));
        // One in a variant counts.
        s.variants.insert("page".into(), json!({"type":"button","id":"nowhere","label":"b"}));
        assert_eq!(errors(&window(), &s), []);
    }

    #[test]
    fn an_event_the_client_does_not_send_is_an_error() {
        let mut s = Settings::default();
        s.handlers.push(click("mg_close", Action::Close));
        s.handlers[0].event = "closed".into();
        assert!(error(&window(), &s, "Unsupported event type"));
    }

    #[test]
    fn a_variant_is_shown_in_a_group() {
        let w = with(json!([
            {"type":"label","id":"header","value":"Fixed header"},
            {"type":"button","id":"next","label":"Next"},
            {"type":"group","id":"panel","children":[{"type":"label","value":"Original"}]}
        ]));
        let mut s = Settings::default();
        s.variants.insert("details".into(), template("col"));
        let show = |group: &str| Action::Variant { group: group.into(), variant: "details".into() };
        s.handlers = vec![click("next", show("panel"))];
        assert_eq!(errors(&w, &s), []);
        s.handlers = vec![click("next", show("header"))];
        assert!(error(&w, &s, "must name a Group ID"));
    }

    #[test]
    fn a_group_inside_a_variant_can_show_a_variant() {
        let w = with(json!([{"type":"button","id":"next","label":"Next"}]));
        let mut s = Settings::default();
        s.variants.insert(
            "details".into(),
            json!({"type":"group","id":"nested","children":[{"type":"label","value":"x"}]}),
        );
        s.variants.insert("inner".into(), template("col"));
        let show = Action::Variant { group: "nested".into(), variant: "inner".into() };
        s.handlers = vec![click("next", show)];
        assert_eq!(errors(&w, &s), []);
    }

    #[test]
    fn a_handler_that_shows_a_variant_there_is_not_is_an_error() {
        let mut s = Settings::default();
        let show = Action::Variant { group: "_window_".into(), variant: "gone".into() };
        s.handlers = vec![click("mg_close", show)];
        assert!(error(&window(), &s, "Unknown swap layout variant"));
    }

    #[test]
    fn toggling_a_bind_that_is_no_flag_is_an_error() {
        let mut s = binds(&[("n", json!(3)), ("rows", json!([true, false]))]);
        s.handlers = vec![click("mg_close", Action::Toggle { bind: "rows".into() })];
        assert_eq!(errors(&window(), &s), []);
        s.handlers = vec![click("mg_close", Action::Toggle { bind: "n".into() })];
        assert!(error(&window(), &s, "boolean or boolean array"));
        s.handlers = vec![click("mg_close", Action::Toggle { bind: "gone".into() })];
        assert!(error(&window(), &s, "needs an existing bind"));
    }

    #[test]
    fn an_action_the_crate_does_not_know_is_an_error() {
        let s = Settings {
            handlers: vec![click("mg_close", Action::Other(json!("teleport")))],
            ..Default::default()
        };
        assert!(error(&window(), &s, "Unknown handler action \"teleport\""));
    }

    #[test]
    fn a_window_id_is_not_empty() {
        let s = Settings { window_id: Some(String::new()), ..Default::default() };
        assert!(error(&window(), &s, "Window ID must be nonempty"));
    }

    /// NWN EE 8193.37 opens a window by such an ID and finds it by it
    /// (`id_punct`, `id_unicode` in the client test): nothing to say.
    #[test]
    fn a_window_id_with_punctuation_or_letters_outside_ascii_is_the_windows() {
        for id in ["a.b-c:d e/f!", "okno żółwia"] {
            let s = Settings { window_id: Some(id.into()), ..Default::default() };
            let found = validate(&window(), &s);
            assert!(!found.iter().any(|d| d.path == "/settings/window_id"), "{id}: {found:?}");
        }
    }

    /// A label, a button or a spacer as the root is shown (NWN EE 8193.37,
    /// `root_*` in the client test): nothing to say of it.
    #[test]
    fn a_root_that_is_no_row_column_or_group_is_shown() {
        for root in [
            json!({"type":"label","value":"Label"}),
            json!({"type":"button","label":"b","id":"b"}),
            json!({"type":"spacer"}),
        ] {
            let mut w = window();
            w["root"] = root.clone();
            let found = validate(&w, &Settings::default());
            assert!(!found.iter().any(|d| d.path == "/root"), "{root}: {found:?}");
        }
    }

    /// A constant out of range is said, as a likely slip, with what the
    /// game does with it: shows the window (NWN EE 8193.37, `halign_9`,
    /// `render_9` and `order_5` in the client test).
    #[test]
    fn a_constant_out_of_range_says_the_game_shows_the_window() {
        let w = with(json!([{"type":"label","value":"x","text_halign":9}]));
        assert!(warning(&w, &Settings::default(), "NUI_HALIGN_* values (0 to 2); the game shows"));
    }

    #[test]
    fn a_window_loaded_by_its_name_is_warned_to_be_on_the_client() {
        let s = Settings { from_resref: true, ..Default::default() };
        assert!(warning(&window(), &s, "NuiCreateFromResRef"));
    }

    #[test]
    fn a_name_is_plain_without_spaces_at_its_ends_or_control_characters() {
        for name in ["a", "a b", "a+", "Żółć"] {
            assert!(plain_name(name), "{name:?}");
        }
        for name in [" a", "a ", "a\n", "a\tb", "a\r"] {
            assert!(!plain_name(name), "{name:?}");
        }
    }

    #[test]
    fn a_close_button_without_its_handler_is_warned_of() {
        let w = crate::window();
        let said = |s: &Settings| {
            validate(&w, s).iter().any(|d| d.message.starts_with("The mg_close button has no"))
        };
        assert!(said(&Settings::default()));
        let mut handled = Settings::default();
        handled.handlers.push(crate::Handler::new("click", "mg_close", Action::Close));
        assert!(!said(&handled));
    }
}
