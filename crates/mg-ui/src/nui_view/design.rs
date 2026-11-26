//! The Design page's parts the docked panels draw (`panels`): the
//! properties of what is selected, with their binds ([`inspector`]), and
//! the binds ([`bindings_in`], the Bindings page and panel). Also where a
//! new control goes ([`insert`], [`insert_at`]), a drop on a Layers row
//! ([`palette_target`]) and the controls' names ([`node_name`]).
use super::*;

#[derive(Clone)]
pub(super) struct Insert(pub(super) &'static str);

#[derive(Clone, Copy)]
pub(super) struct ComponentSpec {
    pub ty: &'static str,
    pub title: &'static str,
    pub sentence: &'static str,
    pub search: &'static str,
    pub summary: &'static str,
}

type ComponentGroup = (&'static str, &'static [ComponentSpec]);

const fn spec(
    ty: &'static str,
    title: &'static str,
    sentence: &'static str,
    search: &'static str,
    summary: &'static str,
) -> ComponentSpec {
    ComponentSpec { ty, title, sentence, search, summary }
}

pub(super) const COMPONENTS: &[ComponentGroup] = &[
    (
        "Layout",
        &[
            spec("col", "Column", "Column", "col column", "Stack vertically"),
            spec("row", "Row", "Row", "row row", "Arrange side by side"),
            spec("group", "Group", "Group", "group group", "A bordered container"),
            spec(
                "swap",
                "Swap Layout",
                "Swap layout",
                "swap swap layout",
                "A Group slot with alternate layouts",
            ),
            spec("spacer", "Spacer", "Spacer", "spacer spacer", "Leave space between controls"),
        ],
    ),
    (
        "Text & Actions",
        &[
            spec("label", "Label", "Label", "label label", "A short line of text"),
            spec("text", "Text Block", "Text block", "text text block", "Longer, scrollable text"),
            spec("button", "Button", "Button", "button button", "A clickable action"),
            spec(
                "button_select",
                "Toggle Button",
                "Toggle button",
                "button_select toggle button",
                "A selectable action",
            ),
        ],
    ),
    (
        "Inputs",
        &[
            spec("textedit", "Text Input", "Text input", "textedit text input", "Let players type"),
            spec("check", "Checkbox", "Checkbox", "check checkbox", "An on/off option"),
            spec("combo", "Dropdown", "Dropdown", "combo dropdown", "Choose from a list"),
            spec(
                "slider",
                "Integer Slider",
                "Integer slider",
                "slider integer slider",
                "Choose a whole number",
            ),
            spec(
                "sliderf",
                "Decimal Slider",
                "Decimal slider",
                "sliderf decimal slider",
                "Choose a decimal value",
            ),
            spec(
                "color_picker",
                "Color Picker",
                "Color picker",
                "color_picker color picker",
                "Choose a color",
            ),
        ],
    ),
    (
        "Display & Navigation",
        &[
            spec("image", "Image", "Image", "image image", "Display a game resource"),
            spec(
                "button_image",
                "Image Button",
                "Image button",
                "button_image image button",
                "An action with an image",
            ),
            spec(
                "progress",
                "Progress Bar",
                "Progress bar",
                "progress progress bar",
                "Show progress",
            ),
            spec("list", "List", "List", "list list", "Repeat a row template"),
            spec("options", "Options", "Options", "options options", "A set of choices"),
            spec("tabbar", "Tabs", "Tabs", "tabbar tabs", "Switch between choices"),
            spec("chart", "Chart", "Chart", "chart chart", "Plot a data series"),
        ],
    ),
];

fn title(ty: &str) -> &str {
    COMPONENTS.iter().flat_map(|(_, items)| *items).find(|c| c.ty == ty).map_or(ty, |c| c.sentence)
}

/// Where a document binds a property to `name`: the bound properties' paths.
/// Where each bind is used in `v`: by bind, the paths of the properties
/// bound to it, in the window's order. One walk for every bind, a path
/// written out only where a bind is found (`at` is the path so far): each
/// bind in use walked the whole window, a new text for every value in it,
/// every frame the Bindings page showed.
fn binds_uses(
    v: &Value,
    at: &mut String,
    out: &mut std::collections::BTreeMap<String, Vec<String>>,
) {
    use std::fmt::Write;
    let here = at.len();
    match v {
        Value::Object(o) => {
            if let Some(name) = o.get("bind").and_then(Value::as_str) {
                out.entry(name.to_owned()).or_default().push(at.clone());
                return;
            }
            for (k, v) in o {
                if !matches!(v, Value::Object(_) | Value::Array(_)) {
                    continue;
                }
                at.push('/');
                at.push_str(k);
                binds_uses(v, at, out);
                at.truncate(here);
            }
        }
        Value::Array(a) => {
            for (i, v) in a.iter().enumerate() {
                if !matches!(v, Value::Object(_) | Value::Array(_)) {
                    continue;
                }
                let _ = write!(at, "/{i}");
                binds_uses(v, at, out);
                at.truncate(here);
            }
        }
        _ => {}
    }
}

pub(super) fn node_name(node: &Value) -> String {
    let kind = title(node["type"].as_str().unwrap_or("Window"));
    let text = node["label"].as_str().or_else(|| node["value"].as_str()).filter(|s| !s.is_empty());
    match text {
        Some(t) => format!("{kind} · {}", t.chars().take(28).collect::<String>()),
        None => kind.into(),
    }
}

/// Clicking a palette item adds to the selected container, or to its nearest
/// container ancestor. Selection follows the new node (including list cells).
pub(super) fn insert(v: &mut Value, selected: &mut String, ty: &str) {
    let ty = if ty == "swap" { "group" } else { ty };
    let Some(parent_path) = insertion_parent(v, selected) else { return };
    let mut node = mg_nui::template(ty);
    assign_ids(&mut node, v);
    let Some(parent) = v.pointer_mut(&parent_path) else { return };
    fit_into(parent, &mut node);
    if parent["type"] == "list" {
        if let Some(cells) = parent["row_template"].as_array_mut() {
            *selected = format!("{parent_path}/row_template/{}/0", cells.len());
            cells.push(json!([node, 150.0, true]));
        }
    } else if let Some(children) = parent["children"].as_array_mut() {
        *selected = format!("{parent_path}/children/{}", children.len());
        children.push(node);
    }
}

/// Shrink a control put in a row of a fixed height (or a column of a fixed
/// width) to what fits there with its margins: the client refuses the whole
/// window if it doesn't (mg_nui::validate). Too little room: no fixed size.
pub(super) fn fit_into(parent: &Value, node: &mut Value) {
    let key = match parent["type"].as_str() {
        Some("row") => "height",
        Some("col") => "width",
        _ => return,
    };
    let (Some(room), Some(size)) = (parent[key].as_f64(), node[key].as_f64()) else { return };
    let margin = node["margin"].as_f64().unwrap_or(2.0);
    if size + 2.0 * margin > room
        && let Some(object) = node.as_object_mut()
    {
        let fits = room - 2.0 * margin;
        if fits >= 8.0 {
            object.insert(key.into(), json!(fits));
        } else {
            object.remove(key);
        }
    }
}

/// Insert a new palette element without ever replacing a Group's single child.
pub(super) fn insert_at(
    doc: &mut Value,
    target: &str,
    position: structure::Position,
    ty: &str,
) -> Option<String> {
    let (dest, index, cell) = structure::destination(doc, target, position)?;
    let mut node = mg_nui::template(if ty == "swap" { "group" } else { ty });
    assign_ids(&mut node, doc);
    if let Some(parent) = dest.strip_suffix("/children").and_then(|p| doc.pointer(p)) {
        fit_into(parent, &mut node);
    }
    let array = doc.pointer_mut(&dest)?.as_array_mut()?;
    if index > array.len() {
        return None;
    }
    array.insert(index, if cell { json!([node, 150.0, true]) } else { node });
    Some(format!("{dest}/{index}{}", if cell { "/0" } else { "" }))
}

pub(super) fn insertion_parent(v: &Value, selected: &str) -> Option<String> {
    let mut path = if selected.is_empty() { "/root" } else { selected }.to_owned();
    loop {
        let node = v.pointer(&path)?;
        if (matches!(node["type"].as_str(), Some("col" | "row")) && node["children"].is_array())
            || (node["type"] == "list" && node["row_template"].is_array())
        {
            return Some(path);
        }
        if node["type"] == "group" && node["children"][0]["children"].is_array() {
            return Some(format!("{path}/children/0"));
        }
        let (parent, _, _) = array_position(&path)?;
        path = parent.rsplit_once('/')?.0.to_owned();
    }
}

pub(super) fn palette_target(
    ui: &mut Ui,
    response: &egui::Response,
    path: &str,
    root: &Value,
    state: &mut State,
) {
    if state.resize.is_some() {
        return;
    }
    if let Some(payload) = response.dnd_hover_payload::<Insert>() {
        let doc = json!({"root": root});
        let mut mode = structure::drop_position(ui, response, false);
        // A leaf cannot own children: its middle behaves as an insertion after it.
        if mode == structure::Position::Inside
            && !doc.pointer(path).is_some_and(|n| {
                matches!(n["type"].as_str(), Some("col" | "row" | "group" | "list"))
            })
            && !path.is_empty()
        {
            mode = structure::Position::After;
        }
        if structure::destination(&doc, path, mode).is_some() {
            structure::paint_drop(ui, response, mode);
            if response.dnd_release_payload::<Insert>().is_some() {
                state.insert_pending = Some((payload.0, path.into(), mode));
            }
        }
    }
}

pub(super) fn tree_paths(v: &Value, collapsed: &std::collections::BTreeSet<String>) -> Vec<String> {
    fn visit(
        v: &Value,
        path: String,
        collapsed: &std::collections::BTreeSet<String>,
        out: &mut Vec<String>,
        depth: usize,
    ) {
        if depth > 64 {
            return;
        }
        out.push(path.clone());
        if collapsed.contains(&path) {
            return;
        }
        if path.is_empty() {
            visit(&v["root"], "/root".into(), collapsed, out, depth + 1);
        }
        if let Some(a) = v["children"].as_array() {
            for (i, c) in a.iter().enumerate() {
                visit(c, format!("{path}/children/{i}"), collapsed, out, depth + 1);
            }
        }
        if let Some(a) = v["row_template"].as_array() {
            for (i, c) in a.iter().enumerate() {
                visit(&c[0], format!("{path}/row_template/{i}/0"), collapsed, out, depth + 1);
            }
        }
    }
    let mut out = Vec::new();
    visit(v, String::new(), collapsed, &mut out, 0);
    out
}

/// The Properties panel's body ([`panels::properties`]): the chosen
/// control's binding (the board's Property Binding first), its properties
/// (or the window's), its swap layout's variants, its list's rows, images,
/// chart, Events, draw layers. Its fields'
/// ids are designer `key`'s and the control's, so that what is typed for
/// one is never shown or set in another's.
pub(super) fn inspector(
    ui: &mut Ui,
    v: &mut Value,
    s: &mut Settings,
    state: &mut State,
    assets: &mut skin::Assets,
    key: ResKey,
) {
    let Some(node) = v.pointer(&state.selected) else {
        ui.weak("Choose a control on the canvas or in the Outliner.");
        return;
    };
    // (Its bind chips' clicks are this designer's: `form::chip_asked`.)
    ui.ctx().data_mut(|d| d.insert_temp(form::chip_owner(), key));
    let whose = (key, &state.edit_view, &state.selected, state.typing);
    fields::of(ui.ctx(), Some(egui::Id::new(whose)));
    let swap = layouts::is_swap(s, node);
    // What the selection is first: a swap layout's variants, any control's
    // own properties; then how it changes (binds, events, drawing).
    if swap {
        ui.add_enabled_ui(state.resize.is_none(), |ui| layouts::inspector(ui, v, s, state));
    }
    // (Its binding first, the board's Property Binding over Content.)
    ui.add_enabled_ui(state.resize.is_none(), |ui| bindings::inspector(ui, v, s, state));
    if let Some(node) = v.pointer_mut(&state.selected) {
        ui.add_enabled_ui(state.resize.is_none(), |ui| properties(ui, node, s));
    }
    fields::list(ui, v, &state.selected, s);
    if !state.selected.is_empty()
        && let Some(node) = v.pointer_mut(&state.selected)
    {
        workflow::images(ui, node, s, state, assets);
        fields::chart(ui, node, s);
    }
    ui.add_enabled_ui(state.resize.is_none(), |ui| events::inspector(ui, v, s, state));
    if !state.selected.is_empty()
        && let Some(node) = v.pointer_mut(&state.selected)
    {
        // (The layer chosen in Layers goes where Up, Down and Remove
        // Drawing put it.)
        draw::draw_owner(ui.ctx(), key, &state.selected);
        if let Some(moved) = draw::editor(ui, node, s, assets) {
            moved.follow(&mut state.draw_item, &state.selected);
        }
    }
    if !swap {
        ui.add_enabled_ui(state.resize.is_none(), |ui| layouts::inspector(ui, v, s, state));
    }
    // (The pages' fields after it are no control's.)
    fields::of(ui.ctx(), None);
}

/// The Bindings page: [`bindings_in`] with its heading.
pub(super) fn bindings(
    ui: &mut Ui,
    s: &mut Settings,
    v: Option<&mut Value>,
    of: Option<&[Held; 2]>,
    state: &mut State,
) {
    bindings_in(ui, s, v, of, state, true);
}

/// Whether a control, or anything under it, has what Layers' filter looks
/// for: in a property's name or in a value, as its JSON text had it (a
/// control shows when one under it does), read where it is.
pub(super) fn holds(v: &Value, search: &mg_core::text::Needle) -> bool {
    match v {
        Value::Object(o) => o.iter().any(|(k, v)| search.finds(k) || holds(v, search)),
        Value::Array(a) => a.iter().any(|v| holds(v, search)),
        Value::String(text) => search.finds(text),
        Value::Null => search.finds("null"),
        Value::Bool(b) => search.finds(if *b { "true" } else { "false" }),
        Value::Number(n) => search.finds(&n.to_string()),
    }
}

fn take_binds_seen(
    state: &mut State,
    s: &Settings,
    v: Option<&Value>,
    of: Option<&[Held; 2]>,
) -> BindsSeen {
    if let Some(of) = of
        && let Some(kept) = state.binds_seen.take_if(|b| same(&b.of, of))
    {
        return kept;
    }
    let mut in_variants = std::collections::BTreeSet::new();
    for view in s.variants.values() {
        in_variants.extend(mg_nui::bind_names(view));
    }
    let mut used = in_variants.clone();
    let mut uses = std::collections::BTreeMap::new();
    if let Some(v) = v {
        used.extend(mg_nui::bind_names(v));
        binds_uses(v, &mut String::new(), &mut uses);
    }
    if of.is_some() {
        state.binds_walks += 1;
    }
    BindsSeen { of: of.cloned().unwrap_or_default(), used, uses, in_variants }
}

/// The window's binds (the Bindings page, `titled` with its heading, and
/// the Right Dock's Bindings panel): Create bind, each bind's initial
/// value, watch, name and uses, Client delivery and Window identity.
pub(super) fn bindings_in(
    ui: &mut Ui,
    s: &mut Settings,
    v: Option<&mut Value>,
    of: Option<&[Held; 2]>,
    state: &mut State,
    titled: bool,
) {
    // (The page's own scroll; the panel's is its own, round its margins.)
    let scroll = egui::ScrollArea::vertical().id_salt("nui-bindings").auto_shrink([false, false]);
    let body = |ui: &mut Ui| {
        if titled {
            widgets::section_heading(ui, "Bindings");
        }
        ui.label("A bind is a named value the event script can change.");
        ui.weak("Made here, it is chosen for a control's property in Design.");
        workflow::create_bind(ui, &mut s.bindings, true, false);
        let seen = take_binds_seen(state, s, v.as_deref(), of);
        for name in &seen.used {
            if !s.bindings.contains_key(name) {
                ui.horizontal(|ui| {
                    ui.label(format!("{name} needs an initial value"));
                    if ui.button("Add Value").clicked() {
                        s.bindings.insert(name.clone(), Binding::default());
                    }
                });
            }
        }
        if s.bindings.is_empty() && seen.used.is_empty() {
            ui.weak("No binds yet.");
        }
        let names: Vec<_> = s.bindings.keys().cloned().collect();
        // The window is read by every bind's part of the page and written
        // by none: a bind renamed is renamed in it once they are drawn.
        // (It was copied whole for the page, and again for each bind.)
        let mut renamed = None;
        {
            let none;
            let doc: &Value = match v.as_deref() {
                Some(doc) => doc,
                None => {
                    none = mg_nui::window();
                    &none
                }
            };
            for name in names {
                let used = Used {
                    anywhere: seen.used.contains(&name),
                    at: seen.uses.get(&name).map_or(&[], Vec::as_slice),
                    in_a_variant: seen.in_variants.contains(&name),
                };
                // (The bind chosen in the Bindings panel's table, brought
                // into sight and opened: M4's U19.)
                let reveal = state.bind_reveal && state.bind_chosen.as_deref() == Some(&name);
                let mut header = |ui: &mut Ui| {
                    egui::CollapsingHeader::new(&name)
                        .icon(mg_uikit::collapsing_icon)
                        .id_salt(("bind", &name))
                        .default_open(true)
                        .open(reveal.then_some(true))
                        .show(ui, |ui| bind(ui, &name, used, doc, s, state))
                };
                // (Those out of sight not laid out: a window of 60 binds.
                // The one to bring into sight is drawn, wherever it is.)
                let shown = match reveal {
                    true => Some(header(ui)),
                    false => widgets::in_sight(ui, ("bind-room", &name), header),
                };
                if let Some(to) = shown.as_ref().and_then(|s| s.body_returned.clone()).flatten() {
                    renamed = Some(to);
                }
                if let Some(shown) = shown.filter(|_| reveal) {
                    shown.header_response.scroll_to_me(Some(egui::Align::TOP));
                    state.bind_reveal = false;
                }
            }
        }
        if of.is_some() {
            state.binds_seen = Some(seen);
        }
        if let (Some(names), Some(doc)) = (renamed, v) {
            shortcuts::rename_binds(doc, &names);
        }
        // (Client Delivery and Window Identity are the Bindings panel's,
        // beside every page: one of each field, M4's U19.)
        if !titled {
            window_settings(ui, s);
        }
    };
    match titled {
        true => _ = scroll.show(ui, body),
        false => body(ui),
    }
}

/// The Bindings panel's table's header row, as a screen reader says it.
pub const BINDS_TABLE: &str = "Name, Initial Value, Watch";

/// The window's settings under the Bindings panel's table (the NUI board's
/// `Nui_right.png`): Custom window ID, and the two checks of Client
/// Delivery, what each does in its tip.
pub(super) fn window_settings(ui: &mut Ui, s: &mut Settings) {
    ui.add_space(form::ROW_GAP);
    let tip = "The ID NuiCreate gives the window and NuiFindWindow finds it by; empty, the \
               window's name. The resource keeps its name";
    form::row(ui, "Custom window ID", Some(tip), |ui, lead| {
        let width = ui.available_width();
        // (One set is edited where it is, as every field of the form: set
        // as it is left, its panel going, the designer closing or the
        // module saved; left empty, the window's name again. None set, a
        // name typed is given it as the field is left.)
        match &mut s.window_id {
            Some(id) => {
                if let Some(field) = fields::hinted(ui, id, width, "The window's name") {
                    let field = field.labelled_by(lead.id);
                    if !field.has_focus() && id.is_empty() {
                        s.window_id = None;
                    }
                }
            }
            // (None set: the field edits an empty one, which shows what is
            // typed as every field does, put back as none as the frame ends,
            // set as it is left or its panel goes: `fields::put_back_settings`.)
            None => {
                let id = s.window_id.insert(String::new());
                if let Some(field) = fields::hinted(ui, id, width, "The window's name") {
                    field.labelled_by(lead.id);
                }
                if s.window_id.as_deref() == Some("") {
                    s.window_id = None;
                }
            }
        }
    });
    ui.check_box(&mut s.from_resref, "Load JUI from the client by resource name").on_hover_text(
        "The players' clients must have the JUI then (from a hak or NWSync). Off, the opener \
         script sends the layout",
    );
    ui.check_box(&mut s.event_function, "Events go to the module's event script").on_hover_text(
        "Build writes the event script as an include whose function the module's OnNuiEvent \
         script calls",
    );
}

/// The Bindings panel's body (M4's U19, the NUI board's table): the binds as
/// a table (Name, Initial Value, Watch), a row chosen by a click, Watch a
/// check that adds or takes off the bind's handler of its changes; the
/// binds the window names that have no value yet; the window's settings;
/// and the bind chosen's form under them (its name, initial value by its
/// kind, When This Value Changes, where it is used, Delete), unless the
/// Bindings page shows every bind's (`page`). `create`: the panel's
/// header's plus was clicked (Create Bind's dialog).
fn preview_json_value(v: &Value, max_bytes: usize) -> String {
    fn write_truncated(v: &Value, out: &mut String, max: usize) -> bool {
        use std::fmt::Write;
        if out.len() >= max {
            return true;
        }
        match v {
            Value::Null => out.push_str("null"),
            Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Value::Number(n) => {
                let _ = write!(out, "{n}");
            }
            Value::String(s) => {
                if let Ok(q) = serde_json::to_string(s) {
                    if out.len() + q.len() <= max {
                        out.push_str(&q);
                    } else {
                        let room = max.saturating_sub(out.len());
                        let cut = q
                            .char_indices()
                            .take_while(|(i, _)| *i <= room)
                            .last()
                            .map_or(0, |(i, _)| i);
                        out.push_str(&q[..cut]);
                        return true;
                    }
                }
            }
            Value::Array(a) => {
                out.push('[');
                for (i, item) in a.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    if write_truncated(item, out, max) || out.len() >= max {
                        return true;
                    }
                }
                out.push(']');
            }
            Value::Object(o) => {
                out.push('{');
                for (i, (k, val)) in o.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    if let Ok(q) = serde_json::to_string(k) {
                        out.push_str(&q);
                        out.push(':');
                    }
                    if write_truncated(val, out, max) || out.len() >= max {
                        return true;
                    }
                }
                out.push('}');
            }
        }
        false
    }
    let mut out = String::with_capacity(max_bytes.min(128));
    if write_truncated(v, &mut out, max_bytes) {
        out.push('…');
    }
    out
}

pub(super) fn bindings_table(
    ui: &mut Ui,
    (s, v, of): (&mut Settings, &mut Value, &[Held; 2]),
    state: &mut State,
    page: bool,
    create: bool,
) {
    let t = mg_uikit::tokens(ui.ctx());
    let seen = take_binds_seen(state, s, Some(v), Some(of));
    if let Some(name) = workflow::create_bind(ui, &mut s.bindings, false, create) {
        state.bind_chosen = Some(name);
    }
    let names: Vec<String> = s.bindings.keys().cloned().collect();
    if state.bind_chosen.as_ref().is_some_and(|n| !s.bindings.contains_key(n)) {
        state.bind_chosen = None;
    }
    let row = 22.0;
    let width = ui.available_width();
    let watch_w = 52.0;
    let name_w = ((width - watch_w) * 0.36).max(60.0);
    let value_w = (width - watch_w - name_w).max(40.0);
    let mono = egui::TextStyle::Monospace.resolve(ui.style());
    let small = egui::TextStyle::Small.resolve(ui.style());
    let cell =
        |ui: &mut Ui, rect: egui::Rect, words: &str, font: &egui::FontId, color: egui::Color32| {
            let mut job =
                egui::text::LayoutJob::simple_singleline(words.to_string(), font.clone(), color);
            job.wrap = egui::text::TextWrapping::truncate_at_width((rect.width() - 12.0).max(0.0));
            let galley = ui.painter().layout_job(job);
            let at = egui::pos2(rect.left() + 6.0, rect.center().y - galley.size().y / 2.0);
            ui.painter().galley(at, galley, color);
        };
    // (Its header row: the columns' names, on the bar's ground.)
    let (head, said) = ui.allocate_exact_size(egui::vec2(width, row), egui::Sense::hover());
    said.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, BINDS_TABLE));
    ui.painter().rect_filled(head, 0.0, t.bar);
    let cols = |r: egui::Rect| {
        let n = egui::Rect::from_min_size(r.min, egui::vec2(name_w, r.height()));
        let v = egui::Rect::from_min_size(
            egui::pos2(n.right(), r.top()),
            egui::vec2(value_w, r.height()),
        );
        let w = egui::Rect::from_min_max(egui::pos2(v.right(), r.top()), r.max);
        (n, v, w)
    };
    let (hn, hv, hw) = cols(head);
    for (rect, words) in [(hn, "Name"), (hv, "Initial Value"), (hw, "Watch")] {
        cell(ui, rect, words, &small, t.text_3);
    }
    for (rect, line) in [(hv, hv.left()), (hw, hw.left())] {
        ui.painter().vline(line, rect.y_range(), (1.0, t.line));
    }
    let property_bind: Option<String> = v
        .pointer(&state.selected)
        .and_then(|n| n[&state.binding_property]["bind"].as_str())
        .map(str::to_owned);
    let mut toggled = None;
    let a11y = ui.ctx().accesskit_node_builder(said.id, |_| ()).is_some();
    let step = row + ui.spacing().item_spacing.y;
    let clip = ui.clip_rect();
    let start_y = ui.cursor().top();
    let (first_vis, last_vis) = if a11y || names.is_empty() {
        (0, names.len())
    } else {
        let first = ((clip.top() - start_y) / step).floor().max(0.0) as usize;
        let last =
            (((clip.bottom() - start_y) / step).ceil().max(0.0) as usize + 1).min(names.len());
        (first.min(last), last)
    };
    if first_vis > 0 {
        ui.allocate_space(egui::vec2(
            width,
            first_vis as f32 * row
                + first_vis.saturating_sub(1) as f32 * ui.spacing().item_spacing.y,
        ));
    }
    for name in &names[first_vis..last_vis] {
        let Some(binding) = s.bindings.get(name) else { continue };
        let (rect, r) = ui.allocate_exact_size(egui::vec2(width, row), egui::Sense::click());
        // (The bind the property chosen in Property Binding takes its value
        // from filled, as the board's `can_send`; the bind chosen in the
        // table ringed, its form under the table.)
        let bound = property_bind.as_deref() == Some(name.as_str());
        let chosen = state.bind_chosen.as_deref() == Some(name.as_str());
        if bound {
            ui.painter().rect_filled(rect, 0.0, t.accent_fill);
        } else if r.hovered() {
            ui.painter().rect_filled(rect, 0.0, t.raised_hover);
        }
        if chosen {
            let ring = rect.shrink(0.5);
            ui.painter().rect_stroke(ring, 0.0, (1.0, t.accent), egui::StrokeKind::Inside);
        }
        let text = if bound { t.on_fill } else { t.text };
        let (n, value, w) = cols(rect);
        cell(ui, n, name, &mono, text);
        let shown = preview_json_value(&binding.value, 160);
        cell(ui, value, &shown, &mono, text);
        let watched = s.handlers.iter().any(|h| h.event == "watch" && h.element == *name);
        let mut on = watched;
        let check = egui::Rect::from_center_size(w.center(), egui::vec2(18.0, 18.0));
        let c = ui
            .put(check, egui::Checkbox::without_text(&mut on))
            .on_hover_text("A handler runs when the event script's value of it changes");
        c.widget_info(|| {
            let said = format!("Watch {name}");
            egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, on, &said)
        });
        if on != watched {
            toggled = Some((name.clone(), on));
        }
        r.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, chosen, name)
        });
        if bound {
            let said = "Bound to the property chosen";
            ui.ctx().accesskit_node_builder(r.id, |node| node.set_description(said));
        }
        let clicked = r
            .on_hover_ui(|ui| {
                ui.label(format!("{name} = {}", preview_json_value(&binding.value, 256)));
            })
            .clicked();
        if clicked {
            state.bind_chosen = Some(name.clone());
            state.bind_reveal = true;
        }
    }
    let after = names.len().saturating_sub(last_vis);
    if after > 0 {
        ui.allocate_space(egui::vec2(
            width,
            after as f32 * row + after.saturating_sub(1) as f32 * ui.spacing().item_spacing.y,
        ));
    }
    if let Some((name, on)) = toggled {
        if on {
            s.handlers.push(mg_nui::Handler {
                event: "watch".into(),
                element: name,
                action: mg_nui::Action::Code,
                extra: Default::default(),
            });
        } else {
            s.handlers.retain(|h| !(h.event == "watch" && h.element == name));
        }
    }
    // The binds the window names with no value yet.
    let used = &seen.used;
    for name in used {
        if !s.bindings.contains_key(name) {
            ui.horizontal(|ui| {
                ui.label(format!("{name} needs an initial value"));
                if ui.button("Add Value").clicked() {
                    s.bindings.insert(name.clone(), Binding::default());
                }
            });
        }
    }
    if names.is_empty() && used.is_empty() {
        ui.weak("No binds yet: the plus makes one.");
    }
    window_settings(ui, s);
    // The bind chosen: its form (on the Bindings page, the page's).
    let Some(name) = state.bind_chosen.clone().filter(|_| !page) else {
        state.binds_seen = Some(seen);
        return;
    };
    ui.add_space(8.0);
    let used = Used {
        anywhere: seen.used.contains(&name),
        at: seen.uses.get(&name).map_or(&[], Vec::as_slice),
        in_a_variant: seen.in_variants.contains(&name),
    };
    let mut renamed = None;
    ui.push_id(("bind", &name), |ui| {
        let doc: &Value = v;
        form::section(ui, ("nui-bind-chosen", &name), &name, true, None, None, |ui| {
            renamed = bind(ui, &name, used, doc, s, state);
        });
    });
    if let Some(names) = renamed {
        if let Some(to) = names.values().next() {
            state.bind_chosen = Some(to.clone());
        }
        shortcuts::rename_binds(v, &names);
    }
    state.binds_seen = Some(seen);
}

/// Where the window names each bind, as the Bindings table last walked it
/// ([`bindings_table`]), for the bytes it walked.
pub(super) struct BindsSeen {
    of: [Held; 2],
    used: std::collections::BTreeSet<String>,
    uses: std::collections::BTreeMap<String, Vec<String>>,
    in_variants: std::collections::BTreeSet<String>,
}

/// Where a bind is used: whether anywhere, the paths of the window's
/// properties bound to it, and whether a swap layout's variant uses it.
#[derive(Clone, Copy)]
struct Used<'a> {
    anywhere: bool,
    at: &'a [String],
    in_a_variant: bool,
}

/// One bind on the Bindings page: its initial value, its handlers, its name
/// and where it is used (`doc` is the window, read here). Renamed, the
/// names to rename in the window (its variants, its value and its handlers
/// are renamed here).
fn bind(
    ui: &mut Ui,
    name: &str,
    used: Used<'_>,
    doc: &Value,
    s: &mut Settings,
    state: &mut State,
) -> Option<std::collections::BTreeMap<String, String>> {
    let in_use = used.anywhere;
    let mut renamed = None;
    let binding = s.bindings.get_mut(name)?;
    widgets::field_label(ui, "Initial value");
    fields::value(ui, &mut binding.value, 0);
    if !in_use {
        ui.weak("Not used by any property yet: choose it on a property in Design.");
    }
    bindings::watch(ui, name, doc, s, state);
    ui.horizontal(|ui| {
        let draft_id = ui.make_persistent_id(("rename-bind", name));
        let mut to: String =
            ui.ctx().data_mut(|d| d.get_temp(draft_id)).unwrap_or_else(|| name.to_owned());
        let label = widgets::field_label(ui, "Name");
        ui.add(mg_uikit::text_field(&mut to).desired_width(160.0))
            .labelled_by(label.id)
            .on_hover_text("A new name for this bind");
        let taken = to != name && s.bindings.contains_key(&to);
        // (A new bind's name, by the rule the windows that make one have;
        // typed here with no spaces at either end, as the tooltip says.)
        let plain = to == name || workflow::new_bind_name(&to, &s.bindings) == Ok(to.as_str());
        let why_not = if taken {
            "Another bind has that name"
        } else {
            "Type a new name: no spaces at either end, no + at the end"
        };
        if ui
            .add_enabled(to != name && plain && !taken, egui::Button::new("Rename"))
            .on_disabled_hover_text(why_not)
            .clicked()
        {
            // Every property, variant and handler using it follows.
            let names = std::collections::BTreeMap::from([(name.to_owned(), to.clone())]);
            renamed = Some(names.clone());
            for view in s.variants.values_mut() {
                shortcuts::rename_binds(view, &names);
            }
            if let Some(b) = s.bindings.remove(name) {
                s.bindings.insert(to.clone(), b);
            }
            for handler in s.handlers.iter_mut().filter(|r| r.event == "watch" && r.element == name)
            {
                handler.element = to.clone();
            }
        }
        ui.ctx().data_mut(|d| d.insert_temp(draft_id, to));
        if ui
            .add_enabled(!in_use, egui::Button::new("Delete"))
            .on_disabled_hover_text(
                "Properties use it (listed below): give them a constant value first",
            )
            .on_hover_text(
                "Its handlers go too. Their code stays in the event script, commented out.",
            )
            .clicked()
        {
            s.bindings.remove(name);
            s.handlers.retain(|r| !(r.event == "watch" && r.element == name));
        }
    });
    if in_use {
        ui.horizontal_wrapped(|ui| {
            ui.weak("Used by");
            for path in used.at {
                let (target, place) = super::describe_path(doc, path);
                if ui.link(place).on_hover_text("Select it").clicked()
                    && let Some(target) = target
                {
                    state.select_next = Some(target);
                    ui.ctx().request_repaint();
                }
            }
            if used.in_a_variant {
                ui.weak("a swap layout variant");
            }
        });
    }
    renamed
}

/// Where Properties is drawn: no game install or GPU is needed.
#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::{NodeT, Queryable};

    struct Fixture {
        doc: Value,
        settings: Settings,
        state: State,
        assets: skin::Assets,
        widths: Option<(f32, f32)>,
    }

    fn fixture() -> Fixture {
        let mut doc = mg_nui::window();
        doc["root"]["children"] = json!([{
            "type":"row","children":[{
                "type":"combo","id":"language","value":0,"height":30.0,
                "elements":[["First",0],["An intentionally long option label that must remain editable inside Properties",2147483647]]
            }]
        }]);
        Fixture {
            doc,
            settings: Settings::default(),
            state: State { selected: "/root/children/0/children/0".into(), ..Default::default() },
            assets: skin::Assets::default(),
            widths: None,
        }
    }

    /// The Right Dock's Properties beside the canvas, as the shell lays
    /// them out (M4's U18): the panel `widths`' right width (the dock's
    /// least, 280, where none is given; the left is the Outliner's, not
    /// drawn here).
    fn show(ui: &mut Ui, f: &mut Fixture) {
        let key = ResKey::parse("fixture", ResType::JUI).unwrap();
        fields::begin(ui.ctx(), key);
        let right = f.widths.map_or(280.0, |(_, right)| right);
        egui::Panel::right(ui.id().with("nui-inspector")).exact_size(right).resizable(false).show(
            ui,
            |ui| {
                egui::ScrollArea::vertical().id_salt("nui-properties").auto_shrink(false).show(
                    ui,
                    |ui| {
                        inspector(ui, &mut f.doc, &mut f.settings, &mut f.state, &mut f.assets, key)
                    },
                );
            },
        );
        egui::CentralPanel::default().show(ui, |ui| {
            preview::canvas(ui, &mut f.doc, &f.settings, &mut f.state, &mut f.assets);
        });
    }

    /// Properties stays in its panel: what it shows (found by its labels) is
    /// between the canvas and the window's edge. Content wider than the panel
    /// makes the panel grow, under the canvas.
    fn assert_inspector_fits(h: &Harness<'_, Fixture>) {
        let canvas = h.state().state.preview_rect.unwrap();
        let edge = h.ctx.content_rect().right();
        let mut seen = 0;
        for label in [
            "Content",
            "Size & Layout",
            "Add Property",
            "Option 1 label",
            "Option 2 label",
            "Add Option",
            "Property Binding",
            "Bind Formatting",
            "Events",
        ] {
            for rect in h.query_all_by_label(label).map(|n| n.rect()) {
                seen += 1;
                assert!(
                    rect.left() >= canvas.right() && rect.right() <= edge + 1.0,
                    "{label} is outside Properties: {rect:?}, canvas {canvas:?}, edge {edge}"
                );
            }
        }
        assert!(seen >= 3, "Properties shows its sections");
    }

    #[test]
    fn properties_stay_in_their_panel_as_the_window_and_panels_are_sized() {
        let mut h =
            Harness::builder().with_size(egui::vec2(1130.0, 760.0)).build_ui_state(show, fixture());
        h.run();
        assert_inspector_fits(&h);
        let before = h.state().doc.clone();
        // (The Right Dock's widths: its least, beside the canvas in a
        // narrow window, and wider.)
        for (width, left, right) in
            [(1130.0, 0.0, 280.0), (800.0, 0.0, 340.0), (1380.0, 0.0, 440.0)]
        {
            h.state_mut().widths = Some((left, right));
            h.set_size(egui::vec2(width, 760.0));
            h.run();
            assert_inspector_fits(&h);
        }
        assert_eq!(h.state().doc, before, "Resizing panels must not change authored NUI");
    }

    #[test]
    fn a_long_bound_text_and_deep_layers_fit_narrow_panels() {
        let mut f = fixture();
        let mut node = json!({"type":"label","value":{"bind":"caption"},"height":30.0});
        f.state.selected = "/root".into();
        for _ in 0..12 {
            node = json!({"type":"col","children":[node]});
            f.state.selected.push_str("/children/0");
        }
        f.doc["root"] = node;
        f.settings.bindings.insert(
            "caption".into(),
            Binding {
                value: json!(
                    "A very long bound string that should be edited without growing the panel sideways"
                ),
                ..Default::default()
            },
        );
        f.widths = Some((0.0, 280.0));
        let mut h = Harness::builder().with_size(egui::vec2(800.0, 760.0)).build_ui_state(show, f);
        h.run();
        assert_inspector_fits(&h);
        h.get_by_label("Bind Formatting").click();
        h.run();
        assert_inspector_fits(&h);
    }

    #[test]
    fn a_dropdown_s_new_option_has_a_value_the_game_can_hold() {
        for (highest, expected) in [(i32::MAX, 1), (7, 8)] {
            let mut f = fixture();
            f.doc["root"]["children"][0]["children"][0]["elements"][1][1] = json!(highest);
            let mut h =
                Harness::builder().with_size(egui::vec2(1130.0, 760.0)).build_ui_state(show, f);
            h.run();
            h.get_by_label("Add Option").click();
            h.run();
            let elements = &h.state().doc["root"]["children"][0]["children"][0]["elements"];
            assert_eq!(elements[0][1], 0);
            assert_eq!(elements[1][1], highest, "Keep existing user IDs");
            assert_eq!(elements[2], json!(["Option", expected]));
            let errors: Vec<_> = mg_nui::validate(&h.state().doc, &h.state().settings)
                .into_iter()
                .filter(|d| d.severity == mg_nui::Severity::Error)
                .collect();
            assert!(errors.is_empty(), "Adding an option must produce valid NUI: {errors:?}");
            assert_inspector_fits(&h);
        }
    }

    /// A window of every kind of place a bind can be at, for the walks.
    fn bound_window() -> Value {
        json!({"version":1,"title":{"bind":"title"},"root":{"type":"col","children":[
            {"type":"label","id":"caption","value":{"bind":"a"},"enabled":{"bind":"b"}},
            {"type":"list","row_count":3,"row_template":[
                [{"type":"label","value":{"bind":"a"},"height":30.5},100.0,true]]},
            {"type":"group","children":[{"type":"button","label":"R\u{e9}sum\u{e9}",
                "draw_list":[{"type":4,"text":{"bind":"b"},"enabled":true,"color":null}]}]}]}})
    }

    /// Every bind's uses, gathered in one walk of the window, are those a
    /// walk for each bind found, in the same order.
    #[test]
    fn every_bind_s_uses_are_gathered_in_one_walk() {
        fn plain(v: &Value, name: &str, at: String, out: &mut Vec<String>) {
            match v {
                Value::Object(o) if o.get("bind").and_then(Value::as_str) == Some(name) => {
                    out.push(at)
                }
                Value::Object(o) => {
                    for (k, v) in o {
                        plain(v, name, format!("{at}/{k}"), out);
                    }
                }
                Value::Array(a) => {
                    for (i, v) in a.iter().enumerate() {
                        plain(v, name, format!("{at}/{i}"), out);
                    }
                }
                _ => {}
            }
        }
        let doc = bound_window();
        let mut uses = std::collections::BTreeMap::new();
        binds_uses(&doc, &mut String::new(), &mut uses);
        assert_eq!(uses.keys().collect::<Vec<_>>(), ["a", "b", "title"]);
        for name in ["a", "b", "title", "none"] {
            let mut want = Vec::new();
            plain(&doc, name, String::new(), &mut want);
            assert_eq!(uses.get(name).cloned().unwrap_or_default(), want, "{name}");
        }
        assert_eq!(uses["a"].len(), 2);
        assert!(uses["b"].contains(&"/root/children/2/children/0/draw_list/0/text".to_string()));
    }

    /// Layers' filter finds in a control, read where it is, what it found
    /// in the control's JSON text; with nothing typed nothing is looked
    /// for at all.
    #[test]
    fn layers_filter_finds_what_a_control_s_json_text_held() {
        fn each(v: &Value, f: &mut impl FnMut(&Value)) {
            f(v);
            match v {
                Value::Object(o) => o.values().for_each(|v| each(v, f)),
                Value::Array(a) => a.iter().for_each(|v| each(v, f)),
                _ => {}
            }
        }
        let doc = bound_window();
        let needles = [
            "caption",
            "LABEL",
            "resume",
            "30.5",
            "100",
            "true",
            "null",
            "row_count",
            "bind",
            "draw",
            "3",
            "nowhere",
            "titl",
        ];
        for needle in needles {
            let search = mg_core::text::Needle::new(needle);
            let mut seen = 0;
            each(&doc, &mut |v| {
                seen += 1;
                assert_eq!(holds(v, &search), search.finds(&v.to_string()), "{needle} in {v}");
            });
            assert!(seen > 30);
        }
    }

    /// A list tried at its real size: its row arrays are drawn to a count
    /// in the forms (Properties' Initial value, Row data), the rest said
    /// and left to Binding JSON, and nothing of them is changed by being
    /// shown. Every row's header was laid out, every frame; now only those
    /// in sight are, the rest given their room.
    #[test]
    fn a_list_of_thousands_of_rows_is_drawn_to_a_count() {
        let rows: Vec<Value> = (0..5_000).map(|i| json!(format!("Row text {i}"))).collect();
        let mut f = fixture();
        f.doc["root"] = json!({"type":"list","row_count":5000,"row_height":30.0,"row_template":[
            [{"type":"label","value":{"bind":"names"}},150.0,true]]});
        f.settings
            .bindings
            .insert("names".into(), Binding { value: json!(rows), ..Default::default() });
        f.state.selected = "/root/row_template/0/0".into();
        f.state.binding_property = "value".into();
        let before = f.settings.bindings["names"].value.clone();
        let mut h = Harness::builder().with_size(egui::vec2(1130.0, 760.0)).build_ui_state(show, f);
        h.run();
        let headers = |h: &Harness<'_, Fixture>, what: &str| {
            let all = h.query_all_by_label_contains(what);
            // ("Row 12", not "Row Data".)
            let numbered =
                |l: String| l.strip_prefix(what).is_some_and(|n| n.parse::<usize>().is_ok());
            all.filter(|n| n.accesskit_node().label().is_some_and(numbered)).count()
        };
        // (Properties shows the bind once, in Property Binding: the
        // property's own row is its bind's chip since M4's U19.)
        let forms = 1;
        let items = headers(&h, "Item ");
        assert!((forms..forms * fields::MOST_ITEMS).contains(&items), "{items} items in sight");
        let said = h.query_all_by_label("And 4800 more items: edit them in Binding JSON.");
        assert_eq!(said.count(), forms);
        h.state_mut().state.selected = "/root".into();
        h.run();
        // (The first frame has no sizes yet: it lays out every row once.)
        h.run_steps(2);
        let rows = headers(&h, "Row ");
        assert!((1..fields::MOST_ITEMS).contains(&rows), "{rows} rows in sight");
        h.get_by_label("And 4800 more rows: edit them in Binding JSON.");
        assert_eq!(h.state().settings.bindings["names"].value, before);
    }
}
