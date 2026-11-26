//! How much room a control asks for and gets: the client's layout, measured.
use super::*;

/// What the canvas has worked out of the window's layout, by the control
/// (where it is in memory: the window is not changed while it is drawn)
/// and its depth: the controls' natural sizes and the containers' plans.
/// Each container asks for its children's sizes, and each child's own
/// children ask again, so without them a window was walked whole once for
/// each level of it (a group's column of 1,000 rows, four times a frame).
/// Kept for a pass, and from one frame to the next while the window is
/// known not to have changed (`State::measured`): the column was still
/// measured whole once a frame for its group's scroll extent, and each of
/// its rows asked whether it fills its width and height.
pub(super) type Naturals = std::cell::RefCell<Measured>;

#[derive(Default)]
pub(in crate::nui_view) struct Measured {
    pub(super) naturals: std::collections::HashMap<(usize, usize), Vec2>,
    plans: std::collections::HashMap<(usize, usize), std::rc::Rc<Plan>>,
    /// Brought from a frame before this one, and (in a debug build) which
    /// of what was brought were worked out again in this pass to check
    /// them: a window changed while kept is a test's failure, not a
    /// layout left as it was.
    carried: bool,
    checked: std::collections::HashSet<(bool, usize, usize)>,
}

impl Measured {
    /// What was kept, for a pass of another frame.
    pub(in crate::nui_view) fn carry(mut self) -> Measured {
        self.carried = true;
        self.checked.clear();
        self
    }

    /// Whether what is kept at `key` (a size, or a plan) is to be worked
    /// out again and compared, once a pass.
    fn due_check(&mut self, plan: bool, key: (usize, usize)) -> bool {
        cfg!(debug_assertions) && self.carried && self.checked.insert((plan, key.0, key.1))
    }
}

/// What a row, column or group lays out its children by that only the
/// window, its settings and the scale decide (not where it is drawn):
/// each child's room, and which of them fill its width and its height,
/// with their sums.
#[derive(Debug, PartialEq)]
pub(super) struct Plan {
    pub sizes: Vec<Vec2>,
    /// Each child's (fills its width, fills its height).
    pub fills: Vec<(bool, bool)>,
    pub total_gap: f32,
    pub fixed: f32,
    pub flex: usize,
    pub fixed_height: f32,
    pub height_flex: usize,
    /// A column's widths after each child (`widths_after`).
    pub widths_after: Vec<f32>,
}

/// `node`'s plan (a container's, `children` its children), worked out by
/// `make`, kept in `kept` and taken from it (not a list's row's).
pub(super) fn plan(
    node: &Value,
    row: Option<usize>,
    depth: usize,
    kept: &Naturals,
    make: impl Fn() -> Plan,
) -> std::rc::Rc<Plan> {
    if row.is_some() {
        return std::rc::Rc::new(make());
    }
    let key = (std::ptr::from_ref(node) as usize, depth.min(33));
    let found = kept.borrow().plans.get(&key).cloned();
    if let Some(plan) = found {
        if kept.borrow_mut().due_check(true, key) {
            assert_eq!(*plan, make(), "a kept plan of a window that changed");
        }
        return plan;
    }
    let plan = std::rc::Rc::new(make());
    kept.borrow_mut().plans.insert(key, plan.clone());
    plan
}

/// A control's natural size (its own, or what its children ask for),
/// kept in `kept` and taken from it.
pub(super) fn natural(
    node: &Value,
    s: &Settings,
    row: Option<usize>,
    depth: usize,
    kept: Option<&Naturals>,
) -> Vec2 {
    // (Not a list's row: a cell given its row's value is a copy made for
    // the moment, whose place the next copy may take.)
    let kept = kept.filter(|_| row.is_none());
    let key = (std::ptr::from_ref(node) as usize, depth.min(33));
    if let Some(size) = kept.and_then(|k| k.borrow().naturals.get(&key).copied()) {
        if kept.is_some_and(|k| k.borrow_mut().due_check(false, key)) {
            let again = measure(node, s, row, depth, kept);
            assert_eq!(size, again, "a kept size of a window that changed");
        }
        return size;
    }
    let size = measure(node, s, row, depth, kept);
    if let Some(k) = kept {
        k.borrow_mut().naturals.insert(key, size);
    }
    size
}

fn measure(
    node: &Value,
    s: &Settings,
    row: Option<usize>,
    depth: usize,
    kept: Option<&Naturals>,
) -> Vec2 {
    if depth > 32 {
        return vec2(120.0, 30.0);
    }
    let ty = node["type"].as_str().unwrap_or("");
    // Confirmed by the Creator-generated Controls window in stock NWN EE.
    // An unspecified control width is not a request to fill its column.
    let mut size = vec2(150.0, 30.0);
    if ty == "group" {
        // NuiGroup deliberately does not advise its parent about its child's
        // size (stock nw_inc_nui). It occupies the available span instead.
        size = Vec2::ZERO;
    } else if let Some(children) = node["children"].as_array() {
        let sizes: Vec<_> = children.iter().map(|c| natural(c, s, row, depth + 1, kept)).collect();
        let gap = children
            .iter()
            .take(children.len().saturating_sub(1))
            .map(|child| child_gap(ty, child))
            .sum::<f32>();
        size = if sizes.is_empty() {
            vec2(100.0, 36.0)
        } else if ty == "row" {
            vec2(
                sizes.iter().map(|s| s.x).sum::<f32>() + gap,
                sizes.iter().map(|s| s.y).fold(0.0, f32::max),
            )
        } else {
            vec2(
                sizes.iter().map(|s| s.x).fold(0.0, f32::max),
                sizes.iter().map(|s| s.y).sum::<f32>() + gap,
            )
        };
    } else if ty == "list" {
        // A list with no width is as wide as its column (NWN EE 8193.37: 304
        // beside two 150 tabs, 150 under a lone button); it doesn't widen it.
        size = vec2(150.0, 100.0);
    } else if ty == "chart" {
        // It stretches like a label (np_chartin): a row beside it keeps its room.
        size = vec2(150.0, 100.0);
    } else if ty == "color_picker" {
        size = vec2(300.0, 100.0);
    } else if matches!(ty, "options" | "tabbar") {
        let count = val(&node["elements"], s, row).as_array().map_or(0, Vec::len);
        let gap = 4.0 * count.saturating_sub(1) as f32;
        // NWN Orientations: vertical Toggles and Options retain horizontal
        // natural width (two entries:304); wide Options hit tested natively.
        let width = count as f32 * 150.0 + gap;
        size = if val(&node["direction"], s, row).as_i64() == Some(1) {
            vec2(width, count as f32 * 30.0 + gap)
        } else {
            vec2(width, 30.0)
        };
    } else if matches!(ty, "label" | "image" | "spacer" | "textedit") {
        // These controls take their span from the containing layout; their
        // content must not inject the button's default 150-unit width. (A
        // text input too, in NWN EE 8193.37: the Domains of Dread's
        // multiline description filled its 468 column.)
        size.x = 0.0;
    } else if ty == "text" {
        // As a label: the whole column (NWN EE 8193.37: 460 wide beside
        // rows of buttons), and only the column (wrapped at a lone 120
        // button's width).
        size = vec2(0.0, 64.0);
    } else if matches!(ty, "button" | "button_select" | "button_image") {
        // NUI_STYLE_PRIMARY_HEIGHT: a button with no height is 50 high in the
        // client (less where its row has less room).
        size.y = 50.0;
    }
    let padding = layout_padding(node, s, row);
    let margin = extra_margin(node, s, row);
    size += Vec2::splat(2.0 * padding);
    let width = num(&node["width"], s, row, -1.0);
    let height = num(&node["height"], s, row, -1.0);
    if width >= 0.0 {
        size.x = width;
    }
    if height >= 0.0 {
        size.y = height;
    }
    let aspect = num(&node["aspect"], s, row, 0.0);
    if aspect > 0.0 && width >= 0.0 && height < 0.0 {
        size.y = size.x / aspect;
    }
    if aspect > 0.0 && height >= 0.0 && width < 0.0 {
        size.x = size.y * aspect;
    }
    (size + Vec2::splat(2.0 * margin)).clamp(Vec2::ZERO, Vec2::splat(LARGEST))
}

/// The most points a side a control is laid out with: a window's own limit
/// (`window_size`). What a window asks for past it (a width, or an aspect
/// near nothing) is held to it.
pub(super) const LARGEST: f32 = 4096.0;

/// How much farther out a control's own margin puts it than the client's
/// default margin of 2 (which the layout's padding and gaps already hold): a
/// margin m moves it m - 2, 0 and 1 included (NWN EE 8193.37: margins 0..20
/// measured against the same control without one).
pub(super) fn extra_margin(node: &Value, s: &Settings, row: Option<usize>) -> f32 {
    if node.get("margin").is_none_or(Value::is_null) {
        return 0.0;
    }
    num(&node["margin"], s, row, 2.0).clamp(0.0, 256.0) - 2.0
}

pub(super) fn layout_padding(node: &Value, s: &Settings, row: Option<usize>) -> f32 {
    // Stock row/column layout inset, observed in the Creator Widths demo.
    let default = if matches!(node["type"].as_str(), Some("row" | "col")) { 2.0 } else { 0.0 };
    num(&node["padding"], s, row, default).clamp(0.0, 256.0)
}

pub(super) fn child_gap(parent_type: &str, child: &Value) -> f32 {
    // Native audit demo: an explicit vertical spacer advances the column by
    // its own height; it does not add a second widget-row gap after itself.
    if parent_type == "col"
        && matches!(child["type"].as_str(), Some("spacer" | "options" | "tabbar"))
    {
        0.0
    } else {
        4.0
    }
}

// Measured in GUI-authored Orientations on NWN EE89.8193.37-17:
// vertical height30/90/150 reserves layout space without stretching items;
// overflow remains painted/clickable. Horizontal height30/60 caps at35.
// These are stock control row metrics, separate from the outer allocation.
pub(super) fn choice_rects(
    body: Rect,
    count: usize,
    vertical: bool,
    tabs: bool,
    scale: f32,
) -> Vec<Rect> {
    let gap = 4.0 * scale;
    let height = if vertical {
        35.0 * scale
    } else {
        (body.height() - 5.0 * scale).clamp(0.0, 35.0 * scale)
    };
    // Entries are 150 wide side by side whatever the control's width: in a
    // 200-wide Options the second starts 154 in (NWN EE 8193.37, np_opts).
    let step = if vertical { height + gap } else { 150.0 * scale + gap };
    (0..count)
        .map(|i| {
            let offset = i as f32 * step;
            Rect::from_min_size(
                body.min + if vertical { vec2(0.0, offset) } else { vec2(offset, 0.0) },
                // Vertical tabs are as wide as a horizontal one (NWN EE: 150
                // each), whatever room the control has; vertical options are
                // hit across their whole width.
                vec2(
                    if vertical && tabs {
                        body.width().min(150.0 * scale)
                    } else if vertical {
                        body.width()
                    } else {
                        (step - gap).max(0.0)
                    },
                    height,
                ),
            )
        })
        .collect()
}

pub(in crate::nui_view) fn window_size(window: &Value, s: &Settings) -> Vec2 {
    let geometry = resolved(&window["geometry"], s);
    let limits = resolved(&window["size_constraint"], s);
    let axis = |value: &Value, min: &str, max: &str, default| {
        let mut value = num(value, s, None, default).clamp(80.0, 4096.0);
        let lower = num(&limits[min], s, None, 0.0);
        let upper = num(&limits[max], s, None, 0.0);
        // NuiWindow: zero means unrestricted on that individual axis.
        if lower > 0.0 {
            value = value.max(lower);
        }
        if upper > 0.0 {
            value = value.min(upper);
        }
        value
    };
    vec2(axis(&geometry["w"], "x", "w", 420.0), axis(&geometry["h"], "y", "h", 240.0))
}

pub(super) fn fills_width(node: &Value, s: &Settings, row: Option<usize>) -> bool {
    num(&node["width"], s, row, -1.0) < 0.0
        && !(num(&node["aspect"], s, row, 0.0) > 0.0 && num(&node["height"], s, row, -1.0) >= 0.0)
        && matches!(
            node["type"].as_str(),
            Some(
                "label"
                    | "text"
                    | "textedit"
                    | "image"
                    | "spacer"
                    | "col"
                    | "row"
                    | "group"
                    | "list"
                    | "color_picker"
                    | "chart"
                    | "progress"
            )
        )
}

/// What takes the room a column has left, when it has no height of its own.
/// The client's root column fills its window: in NWN EE 8193.37 an empty Row
/// with no height pushes everything after it to the window's bottom, and a
/// Label, Text or picker with none is centred in a tall slot.
pub(super) fn fills_height(node: &Value, s: &Settings, row: Option<usize>) -> bool {
    num(&node["height"], s, row, -1.0) < 0.0
        && !(num(&node["aspect"], s, row, 0.0) > 0.0 && num(&node["width"], s, row, -1.0) >= 0.0)
        && match node["type"].as_str() {
            Some(
                "group" | "spacer" | "label" | "text" | "image" | "list" | "chart" | "color_picker",
            ) => true,
            // A row or column is as high as its controls want, unless none of
            // them wants a height (an empty one too).
            Some("row" | "col") => node["children"]
                .as_array()
                .is_none_or(|c| c.iter().all(|c| fills_height(c, s, row))),
            _ => false,
        }
}

/// Native List demo: variable cells share the remaining span equally, while
/// fixed cells retain their pixel width. Cell fWidth is not a proportional weight.
/// The widths of a list's cells in `width`: a cell's own, `scaled` (points
/// to the canvas's pixels, as the game scales them), and the variable ones
/// sharing what is left.
pub(super) fn list_widths(
    cells: &[Value],
    width: f32,
    scale: f32,
    scaled: impl Fn(f32) -> f32,
) -> Vec<f32> {
    let own = |c: &Value| scaled(c[1].as_f64().unwrap_or(150.0).max(0.0) as f32);
    let fixed: f32 = cells.iter().filter(|c| !c[2].as_bool().unwrap_or(true)).map(own).sum();
    let count = cells.iter().filter(|c| c[2].as_bool().unwrap_or(true)).count();
    let gaps = 4.0 * scale * cells.len().saturating_sub(1) as f32;
    let share = ((width - gaps - fixed) / count.max(1) as f32).max(0.0);
    cells.iter().map(|c| if c[2].as_bool().unwrap_or(true) { share } else { own(c) }).collect()
}
