//! A NUI window drawn off the screen, as the NUI Creator's canvas draws it:
//! where each control goes ([`layout`]) and the picture ([`render`]), for
//! `mg nui layout` and `mg nui render`. Both are one frame of the canvas
//! itself ([`preview::still`]): the same layout, the game's skin, pictures
//! and fonts found as the editor finds them, nothing selected and no
//! guides. Laying out needs no GPU; the picture is egui's drawing of that
//! frame on the device given. [`Options`] can draw it otherwise than as it
//! opens: controls hovered, pressed or disabled, binds set as a script
//! would set them, a swap layout variant shown.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

use egui::{Pos2, Rect, vec2};
use mg_core::ResType;
use mg_module::Module;
use mg_nui::Settings;
use mg_resman::{ResKey, ResMan};
use serde::Serialize;
use serde_json::Value;

use super::{State, layouts, preview, skin};

/// A window of a module, and where its skin, pictures, fonts and strings
/// are found.
#[derive(Clone, Copy)]
pub struct Window<'a> {
    pub module: &'a Module,
    /// The game and the module's haks: the skin, the pictures and fonts
    /// (`None`: only what the module holds).
    pub resman: Option<&'a ResMan>,
    /// The talk tables, for text given as a StrRef.
    pub game: Option<&'a mg_rules::GameData>,
    /// The window's name: its NAME.jui, with NAME.txt beside it.
    pub name: &'a str,
}

impl fmt::Debug for Window<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Window").field("name", &self.name).finish_non_exhaustive()
    }
}

/// How the window is drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    /// The UI scale, as the game's option: 1 is 100%. Kept within 0.25 to 4.
    pub scale: f32,
    /// The window's width and height in points, in place of its geometry's
    /// (as a player resizing it would): `None` keeps the geometry's.
    pub size: Option<[f32; 2]>,
    /// Controls drawn in a state of their own, by ID (`None`: every
    /// control, but those given one by ID), as the Creator's State menu
    /// shows the selected one. A control its binds disable is drawn
    /// disabled whatever its state here, as in the game.
    pub states: BTreeMap<Option<String>, Look>,
    /// Binds' values in place of their first ones, by name, as the game's
    /// `NuiSetBind` sets them: a string, number, boolean, an array (a
    /// list's row template, a value a row), a color or a rectangle.
    pub binds: BTreeMap<String, Value>,
    /// A swap layout variant shown in its group (or as the whole window)
    /// in place of what the group holds at first, as `NuiSetGroupLayout`
    /// shows it: its controls are then laid out where the variant puts
    /// them, their paths the window's as shown.
    pub variant: Option<String>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            scale: 1.0,
            size: None,
            states: BTreeMap::new(),
            binds: BTreeMap::new(),
            variant: None,
        }
    }
}

/// How a control looks: the skin's states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Look {
    Normal,
    /// Under the pointer.
    Hover,
    /// Pressed (the skin's `active`).
    Pressed,
    /// Not enabled: its children too.
    Disabled,
}

impl Look {
    /// As `State::preview_state` counts them.
    pub(super) fn mode(self) -> usize {
        match self {
            Look::Normal => 1,
            Look::Hover => 2,
            Look::Pressed => 3,
            Look::Disabled => 4,
        }
    }

    /// Its name, as `mg nui` takes it.
    pub fn name(self) -> &'static str {
        match self {
            Look::Normal => "normal",
            Look::Hover => "hover",
            Look::Pressed => "pressed",
            Look::Disabled => "disabled",
        }
    }
}

impl FromStr for Look {
    type Err = String;

    fn from_str(s: &str) -> Result<Look, String> {
        match s.trim().to_ascii_lowercase().as_str() {
            "normal" => Ok(Look::Normal),
            "hover" => Ok(Look::Hover),
            "pressed" | "active" => Ok(Look::Pressed),
            "disabled" => Ok(Look::Disabled),
            _ => Err(format!("{s:?}: a state is normal, hover, pressed or disabled")),
        }
    }
}

/// Why a window was not drawn.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// The module has no NAME.jui.
    Missing(ResKey),
    /// NAME is not a resource's name.
    Name(String),
    /// The layout or the settings could not be read.
    Read(ResKey, mg_nui::NuiError),
    /// The picture could not be made on the GPU.
    Gpu(String),
    /// No swap layout variant has this name: those there are.
    Variant(String, Vec<String>),
    /// The variant is shown by no handler and was made for no group.
    NoGroup(String),
    /// No control of the window as shown has this ID: those there are.
    Id(String, Vec<String>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Missing(key) => write!(f, "{key} is not in the module"),
            Error::Name(name) => write!(f, "{name:?} is not a resource's name"),
            Error::Read(key, e) => write!(f, "{key}: {e}"),
            Error::Gpu(e) => write!(f, "the GPU could not draw the window: {e}"),
            Error::Variant(name, known) if known.is_empty() => {
                write!(f, "no swap layout variant {name:?}: the window has none")
            }
            Error::Variant(name, known) => {
                write!(f, "no swap layout variant {name:?}: the window's are {}", quoted(known))
            }
            Error::NoGroup(name) => write!(
                f,
                "the variant {name:?} has no group to be shown in: no handler shows it, and the \
                 settings' view_targets don't say which it was made for"
            ),
            Error::Id(id, known) => write!(
                f,
                "no control has the ID {id:?}{}",
                if known.is_empty() {
                    ": the window's controls have none".to_owned()
                } else {
                    format!(": the IDs are {}", quoted(known))
                }
            ),
        }
    }
}

impl std::error::Error for Error {}

fn quoted(names: &[String]) -> String {
    names.iter().map(|n| format!("{n:?}")).collect::<Vec<_>>().join(", ")
}

/// Where the canvas put a window's controls, in the window's points (its
/// top left corner, title bar included, at 0, 0; unscaled).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Layout {
    pub window: String,
    /// The window's size in points, as drawn.
    pub width: f32,
    pub height: f32,
    /// The UI scale it was laid out at: a picture is this many pixels a point.
    pub scale: f32,
    /// What the controls are laid out in: the window inside its title bar
    /// and 8-point inset.
    pub body: Area,
    pub elements: Vec<Element>,
    /// Pictures and fonts the window names that were not found, and other
    /// problems with what it is drawn with.
    pub issues: Vec<String>,
    /// Kinds of control the canvas does not draw (an unknown `type`).
    pub not_drawn: Vec<String>,
}

/// A rectangle, in the window's points.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Area {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// A control as laid out.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Element {
    /// Its JSON pointer in the layout (`/root/children/1`).
    pub path: String,
    /// The list row it is drawn in (a list's row template, once a row).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row: Option<usize>,
    /// Its `type` (`label`, `button`, `row`...).
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The binds its properties read.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub binds: Vec<String>,
    /// The text it shows: its label or value; a text field's value, or
    /// its placeholder (its label) while the value is empty, as drawn.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Inside its margin.
    #[serde(flatten)]
    pub area: Area,
    /// `visible` is false (from the layout or its bind's value).
    pub hidden: bool,
    /// How it was drawn, when not normal: `hover`, `pressed` or `disabled`
    /// (by its binds, an ancestor's or as asked).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<&'static str>,
    /// No room: no width or no height.
    pub zero_sized: bool,
    /// Part of it is cut off by the window, a group or a list.
    pub clipped: bool,
    /// None of it shows.
    pub out_of_view: bool,
    /// The controls beside it (in the same row, column or group) that it
    /// overlaps, by path.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub overlaps: Vec<String>,
    /// Text of it that did not fit and was cut.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cut_text: Vec<String>,
    /// Text that runs past the bottom of its box, which doesn't scroll:
    /// the height (in points) the box needs to show it all.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub needs_height: Option<f32>,
}

/// The text a control shows (see [`Element::text`]).
fn shown_text(
    node: &serde_json::Value,
    s: &mg_nui::Settings,
    row: Option<usize>,
) -> Option<String> {
    let text = |k: &str| {
        preview::val(&node[k], s, row).is_string().then(|| preview::string(&node[k], s, row))
    };
    if node["type"] == "textedit" {
        return text("value").filter(|v| !v.is_empty()).or_else(|| text("label"));
    }
    text("label").or_else(|| text("value"))
}

/// A window drawn: RGBA pixels, the top row first, and its layout.
#[derive(Debug, Clone)]
pub struct Picture {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub layout: Layout,
}

impl Picture {
    /// As a PNG.
    pub fn png(&self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut enc = png::Encoder::new(&mut out, self.width, self.height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        // (Writing to memory fails only on sizes that don't match.)
        let written = enc
            .write_header()
            .and_then(|mut w| w.write_image_data(&self.rgba).and_then(|()| w.finish()));
        debug_assert!(written.is_ok(), "{written:?}");
        out
    }
}

/// Where the canvas puts the window's controls.
pub fn layout(w: &Window<'_>, o: &Options) -> Result<Layout, Error> {
    Ok(frames(w, o, |_, _| {})?.0)
}

/// The window drawn on `gpu`, and its layout from the same frame.
pub fn render(gpu: &mg_render::Gpu, w: &Window<'_>, o: &Options) -> Result<Picture, Error> {
    let mut renderer = egui_wgpu::Renderer::new(
        &gpu.device,
        wgpu::TextureFormat::Rgba8Unorm,
        egui_wgpu::RendererOptions {
            msaa_samples: 1,
            depth_stencil_format: None,
            dithering: false,
            predictable_texture_filtering: false,
        },
    );
    let mut last = None;
    let (layout, ctx) = frames(w, o, |ctx, out| {
        for (id, deltas) in &out.textures_delta.set {
            for delta in deltas {
                renderer.update_texture(&gpu.device, &gpu.queue, *id, delta);
            }
        }
        // (Textures freed meanwhile go with the renderer, at the end.)
        last = Some(ctx.tessellate(std::mem::take(&mut out.shapes), out.pixels_per_point));
    })?;
    let jobs = last.ok_or_else(|| Error::Gpu("nothing was drawn".into()))?;
    let width = (layout.width * layout.scale).ceil().max(1.0) as u32;
    let height = (layout.height * layout.scale).ceil().max(1.0) as u32;
    let limit = gpu.device.limits().max_texture_dimension_2d;
    if width > limit || height > limit {
        return Err(Error::Gpu(format!("{width} × {height} is larger than the GPU draws")));
    }
    let screen = egui_wgpu::ScreenDescriptor {
        size_in_pixels: [width, height],
        pixels_per_point: ctx.pixels_per_point(),
    };
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("nui window"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    let buffers = renderer.update_buffers(&gpu.device, &gpu.queue, &mut encoder, &jobs, &screen);
    {
        let mut pass = encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("nui window"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            })
            .forget_lifetime();
        renderer.render(&mut pass, &jobs, &screen);
    }
    gpu.queue.submit(buffers.into_iter().chain(std::iter::once(encoder.finish())));
    // (Rows top first, as drawn.)
    let read = gpu.read_rgba(&texture);
    Ok(Picture { width, height, rgba: read.data, layout })
}

/// The window and its settings as the module has them, with what `o`
/// changes of them: the variant shown, the binds' values, the size given
/// to its geometry; and what is to be said of them.
fn document(w: &Window<'_>, o: &Options) -> Result<(Value, Settings, Vec<String>), Error> {
    let key = |ty| ResKey::parse(w.name, ty).ok_or_else(|| Error::Name(w.name.into()));
    let jui = key(ResType::JUI)?;
    let data = w.module.get(&jui).ok_or(Error::Missing(jui))?;
    let mut window = mg_nui::parse(data).map_err(|e| Error::Read(jui, e))?;
    let txt = key(ResType::TXT)?;
    let mut settings = match w.module.get(&txt) {
        Some(b) => Settings::parse(b).map_err(|e| Error::Read(txt, e))?,
        None => Settings::default(),
    };
    let mut said = Vec::new();
    // (The binds the window and its variants read: those a script sets.)
    let mut read = mg_nui::bind_names(&window);
    for v in settings.variants.values() {
        read.extend(mg_nui::bind_names(v));
    }
    if let Some(view) = &o.variant {
        if !settings.variants.contains_key(view) {
            return Err(Error::Variant(view.clone(), settings.variants.keys().cloned().collect()));
        }
        let target = layouts::target_for(&settings, view).ok_or(Error::NoGroup(view.clone()))?;
        window = layouts::context_document(&window, &settings, view, &target)
            .ok_or(Error::NoGroup(view.clone()))?;
    }
    if !o.binds.is_empty() {
        for (name, value) in &o.binds {
            if !read.contains(name) {
                said.push(format!("bind {name:?}: no property of the window reads it"));
            }
            let b = settings.bindings.entry(name.clone()).or_default();
            if !b.value.is_null() && kind(&b.value) != kind(value) {
                said.push(format!(
                    "bind {name:?}: given {} where its first value is {}",
                    kind(value),
                    kind(&b.value)
                ));
            }
            b.value = value.clone();
        }
    }
    let ids = mg_nui::element_ids(&window);
    for id in o.states.keys().flatten() {
        if !ids.contains(id) {
            return Err(Error::Id(id.clone(), ids.into_iter().collect()));
        }
    }
    if let Some([width, height]) = o.size
        && window.is_object()
    {
        // A geometry from a bind takes the size in the bind's first value.
        let geometry = match window["geometry"]["bind"].as_str().map(str::to_owned) {
            Some(bind) => &mut settings.bindings.entry(bind).or_default().value,
            None => &mut window["geometry"],
        };
        if !geometry.is_object() {
            *geometry = serde_json::json!({"x": -1.0, "y": -1.0});
        }
        geometry["w"] = width.into();
        geometry["h"] = height.into();
    }
    Ok((window, settings, said))
}

/// What a value is, as said of a bind given another.
fn kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

/// Frames of the canvas until it has all it draws with (fonts are added to
/// egui a frame after they are read): `each` is given every frame's output,
/// and the layout of the last is returned with the context.
fn frames(
    w: &Window<'_>,
    o: &Options,
    mut each: impl FnMut(&egui::Context, &mut egui::FullOutput),
) -> Result<(Layout, egui::Context), Error> {
    let (mut doc, settings, said) = document(w, o)?;
    let scale = if o.scale.is_finite() { o.scale.clamp(0.25, 4.0) } else { 1.0 };
    let mut state = State {
        preview_scale: scale,
        preview_clean: true,
        zoom: 1.0,
        still_states: o.states.iter().map(|(id, look)| (id.clone(), look.mode())).collect(),
        ..Default::default()
    };
    let mut assets = skin::Assets::default();
    let ctx = egui::Context::default();
    let size = (preview::window_size(&doc, &settings) * scale).ceil();
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size.max(vec2(1.0, 1.0)))),
        max_texture_side: Some(8192),
        ..Default::default()
    };
    let mut not_drawn = BTreeSet::new();
    // (Two frames have it all: the first reads the skin, the pictures and
    // fonts, the second draws with them. A few more are allowed for what
    // asks for another.)
    for frame in 0..6 {
        assets.start_record();
        let out = ctx.run_ui(input.clone(), |ui| {
            assets.prepare(ui.ctx(), w.resman, w.module, 0, &doc, &settings);
            assets.prepare_strings(w.game, &doc, &settings);
            not_drawn = preview::still(ui, &mut doc, &settings, &mut state, &assets);
        });
        let again = out
            .viewport_output
            .get(&egui::ViewportId::ROOT)
            .is_some_and(|v| v.repaint_delay.is_zero());
        let mut out = out;
        each(&ctx, &mut out);
        // (What a renderer was to take of the textures, it took.)
        out.textures_delta.clear();
        if frame >= 1 && !again {
            break;
        }
    }
    let record = assets.record.take().unwrap_or_default();
    let mut layout = laid_out(w.name, &record, scale, &settings);
    // (Said once each, however many frames found them.)
    layout.issues = assets.issues.clone();
    layout.issues.extend(said);
    layout.issues.sort();
    layout.issues.dedup();
    layout.not_drawn = not_drawn.into_iter().collect();
    Ok((layout, ctx))
}

/// What the canvas recorded, in the window's points.
fn laid_out(name: &str, r: &skin::Record, scale: f32, s: &Settings) -> Layout {
    let origin = r.window.min;
    let area = |rect: Rect| {
        let round = |n: f32| (n * 100.0).round() / 100.0;
        Area {
            x: round((rect.min.x - origin.x) / scale),
            y: round((rect.min.y - origin.y) / scale),
            w: round(rect.width().max(0.0) / scale),
            h: round(rect.height().max(0.0) / scale),
        }
    };
    let mut elements: Vec<Element> = r
        .placed
        .iter()
        .map(|p| {
            let zero_sized = !(p.rect.width() > 0.0 && p.rect.height() > 0.0);
            let shown = p.rect.intersect(p.clip);
            let out_of_view = zero_sized || !(shown.width() > 0.0 && shown.height() > 0.0);
            // (Half a point of rounding is not a cut.)
            let clipped = !zero_sized && !p.clip.expand(0.5).contains_rect(p.rect);
            Element {
                path: p.path.clone(),
                row: p.row,
                kind: p.node["type"].as_str().unwrap_or("unknown").into(),
                id: p.node["id"].as_str().map(str::to_owned),
                binds: own_binds(&p.node),
                text: shown_text(&p.node, s, p.row),
                area: area(p.rect),
                hidden: !preview::flag(&p.node["visible"], s, p.row, true),
                state: p.look.filter(|l| *l != "normal"),
                zero_sized,
                clipped,
                out_of_view,
                overlaps: Vec::new(),
                cut_text: p.cut.clone(),
                needs_height: p.needs_height,
            }
        })
        .collect();
    // Overlaps between controls of one parent, in one row of a list.
    for (i, a) in r.placed.iter().enumerate() {
        for (j, b) in r.placed.iter().enumerate().skip(i + 1) {
            if a.parent != b.parent || a.row != b.row {
                continue;
            }
            let both = a.rect.intersect(b.rect);
            if both.width() > 0.5 && both.height() > 0.5 {
                let (pi, pj) = (a.path.clone(), b.path.clone());
                elements[i].overlaps.push(pj);
                elements[j].overlaps.push(pi);
            }
        }
    }
    let window = area(r.window);
    Layout {
        window: name.into(),
        width: window.w,
        height: window.h,
        scale,
        body: area(r.body),
        elements,
        issues: Vec::new(),
        not_drawn: Vec::new(),
    }
}

/// The binds a control's own properties read: not those of the controls
/// in it.
fn own_binds(node: &Value) -> Vec<String> {
    let mut found = BTreeSet::new();
    if let Some(o) = node.as_object() {
        for (k, v) in o {
            if k != "children" && k != "row_template" {
                found.extend(mg_nui::bind_names(v));
            }
        }
    }
    found.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::pos2;
    use serde_json::json;

    fn placed(path: &str, parent: Option<usize>, rect: Rect) -> skin::Placed {
        skin::Placed {
            path: path.into(),
            row: None,
            node: json!({"type": "button", "id": path, "label": {"bind": "caption"}}),
            rect,
            clip: Rect::from_min_size(Pos2::ZERO, vec2(400.0, 300.0)),
            parent,
            cut: Vec::new(),
            needs_height: None,
            look: None,
        }
    }

    /// Two controls of one parent drawn over each other overlap, both ways;
    /// a parent and its child, or controls of other parents, don't. Places
    /// are the window's points at any scale.
    #[test]
    fn controls_of_one_parent_in_one_place_overlap() {
        let mut r = skin::Record::default();
        r.window = Rect::from_min_size(pos2(100.0, 50.0), vec2(800.0, 600.0));
        r.body = Rect::from_min_max(pos2(116.0, 132.0), pos2(884.0, 634.0));
        let at = |x: f32, y: f32| Rect::from_min_size(pos2(x, y), vec2(100.0, 60.0));
        r.placed = vec![
            placed("/root", None, Rect::from_min_size(pos2(116.0, 132.0), vec2(700.0, 400.0))),
            placed("/root/children/0", Some(0), at(120.0, 140.0)),
            placed("/root/children/1", Some(0), at(180.0, 140.0)),
            placed("/root/children/1/children/0", Some(2), at(180.0, 140.0)),
            placed("/root/children/2", Some(0), at(350.0, 140.0)),
        ];
        let s = Settings::default();
        let l = laid_out("w", &r, 2.0, &s);
        assert_eq!((l.width, l.height), (400.0, 300.0));
        assert_eq!(l.body, Area { x: 8.0, y: 41.0, w: 384.0, h: 251.0 });
        let overlaps = |i: usize| l.elements[i].overlaps.clone();
        assert_eq!(overlaps(1), ["/root/children/1"]);
        assert_eq!(overlaps(2), ["/root/children/0"]);
        assert!(overlaps(0).is_empty() && overlaps(3).is_empty() && overlaps(4).is_empty());
        assert_eq!(l.elements[1].area, Area { x: 10.0, y: 45.0, w: 50.0, h: 30.0 });
        assert_eq!(l.elements[1].binds, ["caption"]);
        // Past the clip, 400 points across: clipped.
        assert!(l.elements[4].clipped && !l.elements[4].out_of_view);
    }
}
