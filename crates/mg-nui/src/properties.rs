//! What each property of a window, a control, a chart's series and a draw
//! layer takes, as `nw_inc_nui`'s constructors build them: the kind of
//! value, whether a bind may give it or the constructor makes it static,
//! whether null will do, and the game's constants an integer is one of.
//! [`crate::validate`] holds a layout to these tables, and
//! [`crate::jui_schema`] is made of them: a property added here is checked
//! and in the schema both. What the tables can't say (a group's one
//! child, a list's cells, a row's fit across, a text field's `max`) the
//! checks hold in code.

use serde_json::{Value, json};

/// The kind of value a property takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Bool,
    /// A string, or a StrRef (`{"strref": N}`); from a bind, a number
    /// too, which the game writes as text.
    Text,
    /// An integer the game reads as an `int` (32 bits).
    Int,
    Number,
    /// `{x, y, w, h}`, numbers.
    Rect,
    /// `{r, g, b, a}`, integers.
    Color,
    Array,
    /// A list's row count: an integer, or (from a bind) an array whose
    /// length is the count.
    RowCount,
    /// An array of strings (Options' and Tabs' entries).
    Labels,
    /// An array of `[label, integer]` (a combo's entries).
    Entries,
    /// `{x, y}`, numbers.
    Vec2,
    /// A flat array of x and y numbers, in pairs.
    FloatPairs,
    /// An array of numbers.
    Numbers,
}

impl Kind {
    /// Whether `v` is a value of this kind.
    pub fn matches(self, v: &Value) -> bool {
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

    /// As the checks name it.
    pub fn label(self) -> &'static str {
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

    /// Its JSON Schema (a reference to the schema's `$defs`).
    pub fn schema(self) -> Value {
        json!({"$ref": format!("#/$defs/{}", self.def())})
    }

    /// Its name among the schema's `$defs`.
    fn def(self) -> &'static str {
        match self {
            Self::Bool => "boolean",
            Self::Text => "text",
            Self::Int => "integer",
            Self::Number => "number",
            Self::Rect => "rect",
            Self::Color => "color",
            Self::Array => "array",
            Self::RowCount => "row_count",
            Self::Labels => "labels",
            Self::Entries => "entries",
            Self::Vec2 => "vec2",
            Self::FloatPairs => "points",
            Self::Numbers => "numbers",
        }
    }

    pub const ALL: [Kind; 13] = [
        Self::Bool,
        Self::Text,
        Self::Int,
        Self::Number,
        Self::Rect,
        Self::Color,
        Self::Array,
        Self::RowCount,
        Self::Labels,
        Self::Entries,
        Self::Vec2,
        Self::FloatPairs,
        Self::Numbers,
    ];

    /// The schema of a value of this kind, for the schema's `$defs`.
    pub(crate) fn definition(self) -> (&'static str, Value) {
        let number = json!({"type": "number"});
        let int = json!({"type": "integer", "minimum": i32::MIN, "maximum": i32::MAX});
        let object = |keys: &[&str], each: &Value| {
            let properties: serde_json::Map<String, Value> =
                keys.iter().map(|k| ((*k).to_owned(), each.clone())).collect();
            json!({"type": "object", "required": keys, "properties": properties})
        };
        let schema = match self {
            Self::Bool => json!({"type": "boolean"}),
            Self::Text => json!({"oneOf": [{"type": "string"}, {"type": "object",
                "required": ["strref"], "properties": {"strref": {"type": "integer"}},
                "description": "A StrRef of the talk tables"}]}),
            Self::Int => int.clone(),
            Self::Number => number.clone(),
            Self::Rect => object(&["x", "y", "w", "h"], &number),
            Self::Color => object(&["r", "g", "b", "a"], &int),
            Self::Array => json!({"type": "array"}),
            Self::RowCount => json!({"oneOf": [int.clone(), {"type": "array",
                "description": "From a bind: its length is the row count"}]}),
            Self::Labels => json!({"type": "array", "items": {"type": "string"}}),
            Self::Entries => json!({"type": "array", "items": {"type": "array",
                "prefixItems": [{"type": "string"}, {"type": "integer"}], "minItems": 2,
                "maxItems": 2}}),
            Self::Vec2 => object(&["x", "y"], &number),
            Self::FloatPairs => json!({"type": "array", "items": number,
                "description": "x, y, x, y...: an even count"}),
            Self::Numbers => json!({"type": "array", "items": number}),
        };
        let mut schema = schema;
        schema["description"] = match schema.get("description") {
            Some(Value::String(s)) => format!("{}: {s}", self.label()).into(),
            _ => self.label().into(),
        };
        (self.def(), schema)
    }
}

/// Where a property's value comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Takes {
    /// A value, or a bind (`{"bind": NAME}`) whose value a script sets.
    Bound,
    /// A value: the constructor makes it static, and a bind is refused.
    Static,
    /// A static number that shapes the layout (width, height, aspect,
    /// margin, padding): `nw_inc_nui`'s modifiers take no bind.
    Modifier,
}

/// A property and what it takes.
#[derive(Clone, Copy, Debug)]
pub struct Property {
    pub name: &'static str,
    /// The `type`s that have it: of controls by name, of draw layers by
    /// number; empty for every one.
    pub of: &'static [&'static str],
    pub kind: Kind,
    pub takes: Takes,
    /// Null will do (and false, for the window's title).
    pub nullable: bool,
    /// In a list's row, a bind gives a value a row (an array).
    pub per_row: bool,
    /// The game's constants an integer is one of: the least, the most
    /// and their name. Another is shown all the same, and warned of.
    pub constants: Option<(i64, i64, &'static str)>,
    pub about: &'static str,
}

impl Property {
    /// Whether a control (or draw layer) of type `ty` has it.
    pub fn of(&self, ty: &str) -> bool {
        self.of.is_empty() || self.of.contains(&ty)
    }
}

const fn bound(name: &'static str, kind: Kind, about: &'static str) -> Property {
    Property {
        name,
        of: &[],
        kind,
        takes: Takes::Bound,
        nullable: false,
        per_row: true,
        constants: None,
        about,
    }
}

const fn fixed(name: &'static str, kind: Kind, about: &'static str) -> Property {
    Property { takes: Takes::Static, ..bound(name, kind, about) }
}

const fn modifier(name: &'static str, about: &'static str) -> Property {
    Property { takes: Takes::Modifier, ..bound(name, Kind::Number, about) }
}

const fn of(p: Property, of: &'static [&'static str]) -> Property {
    Property { of, ..p }
}

const fn nullable(p: Property) -> Property {
    Property { nullable: true, ..p }
}

const fn one_of(p: Property, low: i64, high: i64, name: &'static str) -> Property {
    Property { constants: Some((low, high, name)), ..p }
}

const fn not_per_row(p: Property) -> Property {
    Property { per_row: false, ..p }
}

/// The window's own (NuiWindow), beside its `root`.
pub const WINDOW: &[Property] = &[
    fixed("version", Kind::Int, "1, the stock NuiWindow's"),
    nullable(bound(
        "title",
        Kind::Text,
        "The title bar's text; false (with collapsing and closing off) for no title bar",
    )),
    bound(
        "geometry",
        Kind::Rect,
        "Where it opens and its size, in points; x and y -1 to center it",
    ),
    nullable(bound("resizable", Kind::Bool, "The player can resize it")),
    nullable(bound("collapsed", Kind::Bool, "Collapsed to its title bar; null lets the player")),
    nullable(bound("closable", Kind::Bool, "It has a close button")),
    nullable(bound("transparent", Kind::Bool, "No background")),
    nullable(bound("border", Kind::Bool, "A border")),
    nullable(bound("accepts_input", Kind::Bool, "Takes clicks; false lets them through")),
    nullable(bound(
        "size_constraint",
        Kind::Rect,
        "The least (x, y) and most (w, h) size the player can resize it to",
    )),
    nullable(bound("edge_constraint", Kind::Rect, "Margins it keeps from the screen's edges")),
    bound("font", Kind::Text, "The skin's font its text is drawn in, by name"),
];

const PICTURES: &[&str] = &["image", "button_image"];
const LABELED: &[&str] = &["button", "button_image", "button_select", "check", "textedit"];
const TEXT_VALUE: &[&str] = &["label", "text", "textedit", "image"];
const BOOL_VALUE: &[&str] = &["button_select", "check"];
const INT_VALUE: &[&str] = &["slider", "combo", "options", "tabbar"];
const NUMBER_VALUE: &[&str] = &["sliderf", "progress"];

/// A control's (any `type`'s unless it says): its `type`, `id`,
/// `children`, a list's `row_template` and the `draw_list` are its shape,
/// which the checks hold in code.
pub const ELEMENT: &[Property] = &[
    modifier("width", "Its width in points; without one, as the layout gives it"),
    modifier("height", "Its height in points"),
    modifier("aspect", "Its width over its height"),
    modifier("margin", "Room around it, in points (2 without one)"),
    modifier("padding", "Room inside it, in points"),
    bound("enabled", Kind::Bool, "Takes input; false draws it, and what is in it, disabled"),
    bound("visible", Kind::Bool, "Shown"),
    bound(
        "encouraged",
        Kind::Bool,
        "Drawn to draw the eye: it pulses between its normal and hover looks",
    ),
    bound("tooltip", Kind::Text, "Shown under the pointer"),
    bound("disabled_tooltip", Kind::Text, "Shown under the pointer while it is disabled"),
    bound("font", Kind::Text, "The skin's font its text is drawn in, by name"),
    bound("foreground_color", Kind::Color, "Its text's color"),
    of(bound("image_region", Kind::Rect, "The part of the picture drawn, in its pixels"), PICTURES),
    of(bound("value", Kind::Text, "Its text; an image's picture, by name"), TEXT_VALUE),
    of(bound("value", Kind::Bool, "Checked, or on"), BOOL_VALUE),
    of(bound("value", Kind::Int, "Its value; the entry chosen"), INT_VALUE),
    of(bound("value", Kind::Number, "Its value; a progress bar's from 0 to 1"), NUMBER_VALUE),
    of(bound("value", Kind::Color, "The color chosen"), &["color_picker"]),
    of(
        fixed(
            "value",
            Kind::Array,
            "Its series (NuiChartSlot), each a series' type, legend, color and data",
        ),
        &["chart"],
    ),
    of(
        bound(
            "label",
            Kind::Text,
            "Its text; a text field's placeholder; an image button's picture",
        ),
        LABELED,
    ),
    one_of(bound("text_halign", Kind::Int, "Its text across"), 0, 2, "NUI_HALIGN_*"),
    one_of(bound("text_valign", Kind::Int, "Its text up and down"), 0, 2, "NUI_VALIGN_*"),
    one_of(bound("image_aspect", Kind::Int, "How its picture fits"), 0, 5, "NUI_ASPECT_*"),
    one_of(bound("image_halign", Kind::Int, "Its picture across"), 0, 2, "NUI_HALIGN_*"),
    one_of(bound("image_valign", Kind::Int, "Its picture up and down"), 0, 2, "NUI_VALIGN_*"),
    one_of(
        fixed("direction", Kind::Int, "Options' and Tabs' entries across (0) or down (1)"),
        0,
        1,
        "NUI_DIRECTION_*",
    ),
    one_of(fixed("scrollbars", Kind::Int, "Its scrollbars"), 0, 4, "NUI_SCROLLBARS_*"),
    fixed("border", Kind::Bool, "A border"),
    fixed("multiline", Kind::Bool, "A text field of several lines"),
    fixed("wordwrap", Kind::Bool, "Its text wraps"),
    of(bound("min", Kind::Int, "The least value"), &["slider"]),
    of(bound("max", Kind::Int, "The most value"), &["slider"]),
    of(bound("step", Kind::Int, "A step of the value"), &["slider"]),
    of(bound("min", Kind::Number, "The least value"), &["sliderf"]),
    of(bound("max", Kind::Number, "The most value"), &["sliderf"]),
    of(bound("step", Kind::Number, "A step of the value"), &["sliderf"]),
    of(fixed("max", Kind::Int, "The most UTF-8 bytes it holds: 1 to 65535"), &["textedit"]),
    of(fixed("elements", Kind::Labels, "Its entries' labels"), &["options", "tabbar"]),
    of(
        bound("elements", Kind::Entries, "Its entries: a label and the value chosen with it"),
        &["combo"],
    ),
    of(
        not_per_row(bound(
            "row_count",
            Kind::RowCount,
            "How many rows: a number, or from a bind an array (its length)",
        )),
        &["list"],
    ),
    of(fixed("row_height", Kind::Number, "Each row's height, in points"), &["list"]),
    bound(
        "draw_list_scissor",
        Kind::Bool,
        "Clip to control: clips nothing in NWN EE 8193.37, and on the window's last draw list blanks the window",
    ),
];

/// A chart's series (NuiChartSlot), each of `value`.
pub const CHART_SLOT: &[Property] = &[
    one_of(fixed("type", Kind::Int, "A line (0) or columns (1)"), 0, 1, "NUI_CHART_TYPE_*"),
    bound("legend", Kind::Text, "Its name in the legend"),
    bound("color", Kind::Color, "Its color"),
    bound("data", Kind::Numbers, "Its values"),
];

/// The draw layers' types, by number (NUI_DRAW_LIST_ITEM_TYPE_*).
pub const DRAW_TYPES: [&str; 8] =
    ["polyline", "curve", "circle", "arc", "text", "image", "line", "rect"];

/// A draw layer's (of `draw_list`), any type's unless it says (by number:
/// 0 a polyline, 5 an image). With `arrayBinds`, a bind in a list's row
/// gives a value a row.
pub const DRAW: &[Property] = &[
    fixed("arrayBinds", Kind::Bool, "In a list, its binds give a value a row"),
    fixed("order", Kind::Int, "Painted before (-1) or after (1) the control"),
    one_of(
        fixed(
            "render",
            Kind::Int,
            "When it shows: always, or as the pointer is over or presses the control",
        ),
        0,
        5,
        "NUI_DRAW_LIST_ITEM_RENDER_*",
    ),
    nullable(bound("enabled", Kind::Bool, "Drawn")),
    nullable(bound("fill", Kind::Bool, "Filled, not outlined")),
    nullable(bound("color", Kind::Color, "Its color")),
    nullable(bound("line_thickness", Kind::Number, "Its lines' width")),
    nullable(bound("radius", Kind::Number, "A circle's or arc's radius")),
    nullable(bound("amin", Kind::Number, "An arc's start, in radians")),
    nullable(bound("amax", Kind::Number, "An arc's end, in radians")),
    bound("rect", Kind::Rect, "Where it is drawn, in the control's points"),
    bound("image_region", Kind::Rect, "The part of the picture drawn"),
    bound("a", Kind::Vec2, "A line's or curve's start; an arc's center"),
    bound("b", Kind::Vec2, "A line's or curve's end"),
    bound("ctrl0", Kind::Vec2, "A curve's first control point"),
    bound("ctrl1", Kind::Vec2, "A curve's second control point"),
    bound("text", Kind::Text, "The text drawn"),
    bound("font", Kind::Text, "The skin's font of the text"),
    bound("image", Kind::Text, "The picture drawn, by name"),
    one_of(bound("image_aspect", Kind::Int, "How the picture fits"), 0, 5, "NUI_ASPECT_*"),
    one_of(bound("image_halign", Kind::Int, "The picture across"), 0, 2, "NUI_HALIGN_*"),
    one_of(bound("image_valign", Kind::Int, "The picture up and down"), 0, 2, "NUI_VALIGN_*"),
    of(bound("points", Kind::FloatPairs, "A polyline's points, x and y in turn"), &["0"]),
];
