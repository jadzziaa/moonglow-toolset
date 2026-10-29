//! What the canvas draws with ([`Assets`]): the game's NUI skin, the
//! pictures and fonts a window names and the talk-table lines it shows,
//! read from the module, its haks and the game as the client would find
//! them, and read again when they change. No game art is built in.
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use egui::{Color32, FontFamily, FontId, Painter, Rect, pos2, vec2};
use mg_core::ResType;
use mg_module::Module;
use mg_resman::{ResKey, ResMan, priority};
use serde_json::Value;

use super::{Settings, resolved};

#[derive(Clone)]
pub(super) struct Picture {
    pub texture: egui::TextureHandle,
    pub size: egui::Vec2,
    stamp: u64,
}

#[derive(Clone)]
struct Font {
    family: FontFamily,
    size: f32,
    em_ratio: f32,
    spacing: f32,
}

fn pixel_height_to_em(bytes: &[u8]) -> Option<f32> {
    use skrifa::raw::TableProvider;
    let font = skrifa::FontRef::new(bytes).ok()?;
    let hhea = font.hhea().ok()?;
    let height = f32::from(hhea.ascender().to_i16()) - f32::from(hhea.descender().to_i16());
    let em = f32::from(font.head().ok()?.units_per_em());
    // Nuklear/STB sizes glyphs by hhea ascent - descent; egui uses units per em.
    // Using the same numeric size for both makes the preview font too large.
    (height > 0.0 && em > 0.0).then_some(em / height)
}

/// What a laid-out text was made from. `width` is where it wraps or is cut
/// (infinite when neither), `shape` which of the preview's layouts it is.
#[derive(Clone, Copy)]
pub(super) struct TextKey<'a> {
    pub text: &'a str,
    pub font: &'a FontId,
    pub spacing: f32,
    pub color: Color32,
    pub line_height: Option<f32>,
    pub width: f32,
    pub shape: u8,
}

impl TextKey<'_> {
    fn hash(&self) -> u64 {
        egui::util::hash((
            self.text,
            self.font,
            self.spacing.to_bits(),
            self.color,
            self.line_height.map(f32::to_bits),
            self.width.to_bits(),
            self.shape,
        ))
    }
}

struct KeptText {
    text: Box<str>,
    font: FontId,
    spacing: f32,
    color: Color32,
    line_height: Option<f32>,
    width: f32,
    shape: u8,
    galley: Arc<egui::Galley>,
    /// The pass it was last drawn in.
    used: u32,
}

impl KeptText {
    fn is(&self, key: &TextKey<'_>) -> bool {
        self.shape == key.shape
            && self.width.to_bits() == key.width.to_bits()
            && self.spacing.to_bits() == key.spacing.to_bits()
            && self.line_height.map(f32::to_bits) == key.line_height.map(f32::to_bits)
            && self.color == key.color
            && self.font == *key.font
            && *self.text == *key.text
    }
}

/// The preview's texts as they were laid out, kept from one pass to the next:
/// laying one out anew costs a section a glyph (`preview::native_job`).
///
/// A galley holds positions in egui's font atlas, and is garbage once egui
/// builds that atlas again: for new or added fonts, changed text options or
/// an atlas that filled up, always at the start of a pass, and each time it
/// empties its own cache of galleys (`epaint::Fonts::begin_pass`; a change
/// of pixels per point makes its galleys new ones too). So a mark is laid
/// out in every pass: egui keeps what was laid out in the pass before, and
/// while it hands the same mark back, its atlas is the one these galleys
/// were made for. When it is another (also after a pass that drew no
/// preview), everything kept goes.
#[derive(Default)]
struct TextCache {
    pass: Option<u64>,
    mark: Option<Arc<egui::Galley>>,
    generation: u32,
    kept: std::collections::HashMap<u64, KeptText>,
}

impl TextCache {
    /// Once a pass: drops everything if egui's atlas is not the same one,
    /// and otherwise what the pass before did not draw.
    fn begin_pass(&mut self, ctx: &egui::Context) {
        let pass = ctx.cumulative_pass_nr();
        if self.pass == Some(pass) {
            return;
        }
        self.pass = Some(pass);
        let mark = ctx.fonts_mut(|f| {
            f.layout_job(egui::text::LayoutJob::simple_singleline(
                "nui".into(),
                FontId::default(),
                Color32::PLACEHOLDER,
            ))
        });
        if self.mark.as_ref().is_some_and(|kept| Arc::ptr_eq(kept, &mark)) {
            let generation = self.generation;
            self.kept.retain(|_, t| t.used == generation);
        } else {
            self.kept.clear();
        }
        self.mark = Some(mark);
        self.generation = self.generation.wrapping_add(1);
    }

    fn get(
        &mut self,
        ctx: &egui::Context,
        key: &TextKey<'_>,
        hash: u64,
    ) -> Option<Arc<egui::Galley>> {
        self.begin_pass(ctx);
        let kept = self.kept.get_mut(&hash).filter(|t| t.is(key))?;
        kept.used = self.generation;
        Some(kept.galley.clone())
    }

    fn keep(&mut self, key: &TextKey<'_>, hash: u64, galley: Arc<egui::Galley>) {
        self.kept.insert(
            hash,
            KeptText {
                text: key.text.into(),
                font: key.font.clone(),
                spacing: key.spacing,
                color: key.color,
                line_height: key.line_height,
                width: key.width,
                shape: key.shape,
                galley,
                used: self.generation,
            },
        );
    }
}

/// A value of the skin, read once: as a number, or as a string that is a
/// color or names a picture (the key of `Assets::pictures`).
struct Leaf {
    number: Option<f32>,
    color: Option<Color32>,
    picture: Option<String>,
}

/// The values under `table`, by their dotted paths. A key with a dot in it
/// is one no path leads to.
fn leaves(prefix: &str, table: &toml::Table, out: &mut std::collections::HashMap<String, Leaf>) {
    for (key, value) in table.iter().filter(|(key, _)| !key.contains('.')) {
        let path = if prefix.is_empty() { key.clone() } else { format!("{prefix}.{key}") };
        if let toml::Value::Table(table) = value {
            leaves(&path, table, out);
            continue;
        }
        let text = value.as_str();
        out.insert(
            path,
            Leaf {
                number: toml_number(value),
                color: text.and_then(color),
                picture: text.map(str::to_lowercase),
            },
        );
    }
}

#[derive(Default)]
pub(crate) struct Assets {
    dirty: bool,
    signature: Option<u64>,
    /// The module's revision when it was last looked at: an edit that leaves
    /// its skin, pictures and fonts alone reloads nothing.
    revision: Option<u64>,
    /// Every resource the last load asked for, and those of them the module
    /// itself holds, as it held them.
    watched: Vec<ResKey>,
    held: Vec<(ResKey, Arc<[u8]>)>,
    /// The pictures of the game's layers (listed once for a load order) and
    /// the module's own, which make up `catalog`.
    layers: Option<u64>,
    game_pictures: BTreeSet<String>,
    module_pictures: BTreeSet<String>,
    pub(super) catalog: Vec<String>,
    requested: BTreeSet<String>,
    style: toml::Table,
    /// `style` by dotted path ("button.normal"), as the preview asks for it
    /// with every control it paints.
    leaves: std::collections::HashMap<String, Leaf>,
    pictures: BTreeMap<String, Picture>,
    fonts: BTreeMap<String, Font>,
    default_font: String,
    text: std::cell::RefCell<TextCache>,
    strings: BTreeMap<i64, String>,
    pub(super) origins: BTreeMap<String, String>,
    pub(super) issues: Vec<String>,
    pub(super) loaded: bool,
    /// Where the canvas put the window and each control, while a caller
    /// asks for it (`mg nui layout` and `render`: [`super::headless`]);
    /// `None`, and nothing gathered, in the editor.
    pub(super) record: std::cell::RefCell<Option<Record>>,
}

/// Where the canvas drew a window and its controls, in screen points, as
/// one frame of it laid them out.
#[derive(Debug, Clone)]
pub(super) struct Record {
    /// The window, title bar and frame included.
    pub window: Rect,
    /// What its controls are laid out in: inside its 8-point inset.
    pub body: Rect,
    /// Each control drawn or passed over, in the order the canvas reached
    /// them (a parent before its children).
    pub placed: Vec<Placed>,
    /// The controls being drawn, innermost last: whose text a cut belongs to.
    current: Vec<usize>,
}

/// A control as the canvas laid it out.
#[derive(Debug, Clone)]
pub(super) struct Placed {
    /// Its JSON pointer in the window (`/root/children/1`).
    pub path: String,
    /// The list row it was drawn in, for a list's row template.
    pub row: Option<usize>,
    pub node: Value,
    /// Its rectangle, inside its margin.
    pub rect: Rect,
    /// What is drawn of it at most: the window's, a group's or a list's clip.
    pub clip: Rect,
    /// The control it is in, if any: its index in `placed`.
    pub parent: Option<usize>,
    /// Text of it that did not fit and was cut.
    pub cut: Vec<String>,
    /// The height its text needs, in points, where the text runs past the
    /// bottom of its box and the box doesn't scroll.
    pub needs_height: Option<f32>,
    /// How it was drawn (`normal`, `hover`, `pressed`, `disabled`), once
    /// the canvas knows.
    pub look: Option<&'static str>,
}

impl Default for Record {
    fn default() -> Self {
        Record {
            window: Rect::NOTHING,
            body: Rect::NOTHING,
            placed: Vec::new(),
            current: Vec::new(),
        }
    }
}

impl Assets {
    /// Starts gathering where the canvas draws (a frame's worth: each frame
    /// starts again).
    pub(super) fn start_record(&self) {
        *self.record.borrow_mut() = Some(Record::default());
    }

    pub(super) fn record_window(&self, window: Rect, body: Rect) {
        if let Some(r) = self.record.borrow_mut().as_mut() {
            r.window = window;
            r.body = body;
        }
    }

    /// A control the canvas is about to draw, in `rect` within `clip`;
    /// what it draws until [`Assets::leave`] is its.
    pub(super) fn enter(
        &self,
        path: &str,
        row: Option<usize>,
        node: &Value,
        rect: Rect,
        clip: Rect,
    ) -> bool {
        let mut record = self.record.borrow_mut();
        let Some(r) = record.as_mut() else { return false };
        let parent = r.current.last().copied();
        r.placed.push(Placed {
            path: path.into(),
            row,
            node: node.clone(),
            rect,
            clip,
            parent,
            cut: Vec::new(),
            needs_height: None,
            look: None,
        });
        r.current.push(r.placed.len() - 1);
        true
    }

    pub(super) fn leave(&self, entered: bool) {
        if entered && let Some(r) = self.record.borrow_mut().as_mut() {
            r.current.pop();
        }
    }

    /// How the control being drawn looks.
    pub(super) fn record_look(&self, look: &'static str) {
        if let Some(r) = self.record.borrow_mut().as_mut()
            && let Some(&at) = r.current.last()
        {
            r.placed[at].look = Some(look);
        }
    }

    /// Text the control being drawn cut because it did not fit.
    pub(super) fn record_cut(&self, text: &str) {
        if let Some(r) = self.record.borrow_mut().as_mut()
            && let Some(&at) = r.current.last()
        {
            r.placed[at].cut.push(text.into());
        }
    }

    /// Text the control being drawn cut at the bottom of its box, which
    /// would need to be `height` points high to show it all.
    pub(super) fn record_cut_below(&self, text: &str, height: f32) {
        if let Some(r) = self.record.borrow_mut().as_mut()
            && let Some(&at) = r.current.last()
        {
            let placed = &mut r.placed[at];
            placed.cut.push(text.into());
            placed.needs_height = Some(placed.needs_height.unwrap_or(0.0).max(height.ceil()));
        }
    }

    pub(super) fn recording(&self) -> bool {
        self.record.borrow().is_some()
    }
}

fn hash(v: impl Hash) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    v.hash(&mut h);
    h.finish()
}

/// A live module takes its proper position below HAKs, even before it is saved.
fn resource(rm: Option<&ResMan>, module: &Module, key: ResKey) -> Option<(Vec<u8>, String)> {
    if let Some(rm) = rm {
        for l in rm.layers().iter().filter(|l| l.priority > priority::MODULE) {
            if l.container.contains(&key) {
                return l.container.read(&key).ok().map(|b| (b.into_owned(), l.label.clone()));
            }
        }
    }
    if let Some(b) = module.get(&key) {
        return Some((b.to_vec(), "module (current edits)".into()));
    }
    for l in rm?.layers().iter().filter(|l| l.priority < priority::MODULE) {
        if l.container.contains(&key) {
            return l.container.read(&key).ok().map(|b| (b.into_owned(), l.label.clone()));
        }
    }
    None
}

/// The types a NUI picture may have, in the order the client looks for them:
/// not the 3D renderer's DDS-first rule (DDS came as a fallback in EE 8193.36).
const PICTURE_TYPES: [ResType; 5] =
    [ResType::JPG, ResType::TGA, ResType::PNG, ResType::BMP, ResType::DDS];

/// Which of `keys` the module holds, each as it holds it now.
fn held(module: &Module, keys: &[ResKey]) -> Vec<(ResKey, Arc<[u8]>)> {
    keys.iter().filter_map(|k| Some((*k, module.shared(k)?.clone()))).collect()
}

fn same_held(a: &[(ResKey, Arc<[u8]>)], b: &[(ResKey, Arc<[u8]>)]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.0 == b.0 && Arc::ptr_eq(&a.1, &b.1))
}

fn image_resource(
    rm: Option<&ResMan>,
    module: &Module,
    name: &str,
) -> Option<(ResType, Vec<u8>, String)> {
    PICTURE_TYPES
        .into_iter()
        .find_map(|ty| Some((ty, resource(rm, module, ResKey::parse(name, ty)?)?)))
        .map(|(ty, (bytes, origin))| (ty, bytes, origin))
}

/// The longest side a picture of a window may have.
const MAX_SIDE: u32 = 4096;

fn image(bytes: &[u8], ty: ResType) -> Result<egui::ColorImage, String> {
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("image exceeds 64 MiB".into());
    }
    match ty {
        ResType::DDS => {
            let t = mg_image::read(ty, bytes).map_err(|e| e.to_string())?;
            if u64::from(t.width) * u64::from(t.height) > 16_777_216 {
                return Err("image exceeds 16 megapixels".into());
            }
            let rgba = t.to_rgba().top_down();
            Ok(egui::ColorImage::from_rgba_unmultiplied(
                [rgba.width as usize, rgba.height as usize],
                &rgba.data,
            ))
        }
        ResType::TGA => read_tga(bytes),
        ResType::PNG => read_png(bytes),
        ResType::JPG => other(bytes, image::ImageFormat::Jpeg),
        ResType::BMP => other(bytes, image::ImageFormat::Bmp),
        _ => Err(format!("Unsupported NUI image: {ty}")),
    }
}

/// A TGA, through the workspace's reader. That one keeps rows and columns
/// as they are stored, which is how the game takes a texture for a model;
/// a window's picture is drawn by the origin its file declares.
fn read_tga(bytes: &[u8]) -> Result<egui::ColorImage, String> {
    // The size its header declares, before anything is read by it: the
    // reader takes textures up to 16,384 a side, and a run-length encoded
    // file of a few bytes may declare that.
    let side =
        |at: usize| bytes.get(at..at + 2).map(|b| u32::from(u16::from_le_bytes([b[0], b[1]])));
    if side(12).is_some_and(|w| w > MAX_SIDE) || side(14).is_some_and(|h| h > MAX_SIDE) {
        return Err(format!("image exceeds {MAX_SIDE} pixels a side"));
    }
    let rgba = mg_image::tga::read_picture(bytes).map_err(|e| e.to_string())?;
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [rgba.width as usize, rgba.height as usize],
        &rgba.data,
    ))
}

/// A PNG of any color type and depth as RGBA8 (`mg_image::png`, as a
/// plugin's `mg.image` reads one), no more than [`MAX_SIDE`] a side.
fn read_png(bytes: &[u8]) -> Result<egui::ColorImage, String> {
    let most = u64::from(MAX_SIDE) * u64::from(MAX_SIDE);
    let rgba = mg_image::png::read_picture(bytes, most).map_err(|e| e.to_string())?;
    if rgba.width > MAX_SIDE || rgba.height > MAX_SIDE {
        return Err(format!("image exceeds {MAX_SIDE} pixels a side"));
    }
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [rgba.width as usize, rgba.height as usize],
        &rgba.data,
    ))
}

/// A JPEG or BMP.
fn other(bytes: &[u8], format: image::ImageFormat) -> Result<egui::ColorImage, String> {
    let mut reader = image::ImageReader::with_format(std::io::Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let rgba = reader.decode().map_err(|e| e.to_string())?.into_rgba8();
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        rgba.as_raw(),
    ))
}

pub(super) fn names(v: &Value, s: &Settings, out: &mut BTreeSet<String>) {
    fn add(v: &Value, s: &Settings, out: &mut BTreeSet<String>) {
        let v = resolved(v, s);
        if let Some(a) = v.as_array() {
            for v in a {
                add(v, s, out);
            }
        }
        if let Some(name) = v.as_str().filter(|n| !n.is_empty()) {
            out.insert(name.to_lowercase());
        }
    }
    match v["type"].as_str() {
        Some("image") => add(&v["value"], s, out),
        Some("button_image") => add(&v["label"], s, out),
        _ => {}
    }
    if let Some(v) = v.get("image") {
        add(v, s, out);
    }
    match v {
        Value::Object(o) => {
            for v in o.values() {
                names(v, s, out);
            }
        }
        Value::Array(a) => {
            for v in a {
                names(v, s, out);
            }
        }
        _ => {}
    }
}

impl Assets {
    /// Resolve only referenced talk-table rows through the same game/module TLK
    /// stack as the rest of Moonglow. Refresh independently of texture caching:
    /// a custom TLK or a runtime binding can change without a new image resource.
    pub(super) fn prepare_strings(
        &mut self,
        game: Option<&mg_rules::GameData>,
        doc: &Value,
        s: &Settings,
    ) {
        fn collect(v: &Value, refs: &mut BTreeSet<i64>) {
            if let Some(id) = v.get("strref").and_then(Value::as_i64) {
                refs.insert(id);
            }
            match v {
                Value::Object(o) => o.values().for_each(|v| collect(v, refs)),
                Value::Array(a) => a.iter().for_each(|v| collect(v, refs)),
                _ => {}
            }
        }
        let mut refs = BTreeSet::new();
        collect(doc, &mut refs);
        for binding in s.bindings.values() {
            collect(&binding.value, &mut refs);
        }
        self.strings = refs
            .into_iter()
            .filter_map(|id| {
                let raw =
                    u32::try_from(id).ok().or_else(|| i32::try_from(id).ok().map(|n| n as u32))?;
                let reference = mg_core::StrRef(raw);
                let text =
                    if reference.is_none() { String::new() } else { game?.string(reference)? };
                Some((id, text))
            })
            .collect();
    }

    pub(super) fn string(&self, id: i64) -> String {
        self.strings.get(&id).cloned().unwrap_or_else(|| format!("[Missing StrRef {id}]"))
    }

    pub(super) fn request_images(&mut self, ctx: &egui::Context, names: BTreeSet<String>) {
        if self.requested != names {
            self.requested = names;
            ctx.request_repaint();
        }
    }
    pub(crate) fn invalidate(&mut self) {
        self.dirty = true;
    }

    pub(super) fn prepare(
        &mut self,
        ctx: &egui::Context,
        rm: Option<&ResMan>,
        module: &Module,
        revision: u64,
        v: &Value,
        s: &Settings,
    ) {
        let mut wanted = BTreeSet::new();
        names(v, s, &mut wanted);
        // What the client draws for an image it can't find (nuklear_image.cpp).
        wanted.insert("gui_error".into());
        wanted.extend(self.requested.iter().cloned());
        let layers: Vec<_> = rm
            .into_iter()
            .flat_map(|r| r.layers())
            .map(|l| (&l.label, l.priority, l.fingerprint))
            .collect();
        let signature = hash((&layers, &wanted));
        let layers = hash(&layers);
        // The module changes with every edit of the window, and with every
        // key typed in a script beside it: only what it holds of the skin,
        // the pictures and the fonts is looked at then, and nothing is read
        // again unless one of those changed.
        let edited = self.dirty || self.revision != Some(revision);
        self.revision = Some(revision);
        let mut listed = false;
        if self.dirty || self.layers != Some(layers) {
            self.layers = Some(layers);
            self.game_pictures = rm
                .into_iter()
                .flat_map(|rm| PICTURE_TYPES.into_iter().flat_map(|ty| rm.list(ty)))
                .map(|r| r.to_string())
                .collect();
            listed = true;
        }
        if edited {
            let pictures: BTreeSet<String> = module
                .keys()
                .filter(|k| PICTURE_TYPES.contains(&k.restype))
                .map(|k| k.resref.to_string())
                .collect();
            if pictures != self.module_pictures {
                self.module_pictures = pictures;
                listed = true;
            }
        }
        if listed {
            self.catalog = self.game_pictures.union(&self.module_pictures).cloned().collect();
        }
        if !self.dirty
            && self.signature == Some(signature)
            && (!edited || same_held(&self.held, &held(module, &self.watched)))
        {
            return;
        }
        self.dirty = false;
        self.signature = Some(signature);
        self.issues.clear();
        self.origins.clear();
        self.fonts.clear();
        // (egui builds its atlas again for the fonts added below, which
        // `TextCache` sees for itself; these are only no longer wanted.)
        self.text.get_mut().kept.clear();
        self.style.clear();
        self.leaves.clear();
        self.loaded = false;
        let key = ResKey::parse("nui_skin", ResType::TML).unwrap();
        if let Some((b, origin)) = resource(rm, module, key) {
            match std::str::from_utf8(&b)
                .map_err(|e| e.to_string())
                .and_then(|t| t.parse::<toml::Table>().map_err(|e| e.to_string()))
            {
                Ok(style) => {
                    leaves("", &style, &mut self.leaves);
                    self.style = style;
                    self.loaded = true;
                    self.origins.insert(key.to_string(), origin);
                }
                Err(e) => self.issues.push(format!("nui_skin.tml: {e}")),
            }
        } else {
            self.issues.push(
                "nui_skin.tml is unavailable. Configure the game folder and module resources."
                    .into(),
            );
        }
        fn skin_images(v: &toml::Value, out: &mut BTreeSet<String>) {
            match v {
                toml::Value::String(n) if !n.starts_with('#') => {
                    out.insert(n.to_lowercase());
                }
                toml::Value::Table(t) => {
                    for v in t.values() {
                        skin_images(v, out);
                    }
                }
                _ => {}
            }
        }
        for (k, v) in &self.style {
            if k != "fonts" {
                skin_images(v, &mut wanted);
            }
        }
        self.pictures.retain(|k, _| wanted.contains(k));
        self.watched = std::iter::once(key)
            .chain(
                wanted
                    .iter()
                    .flat_map(|n| PICTURE_TYPES.into_iter().filter_map(|ty| ResKey::parse(n, ty))),
            )
            .collect();
        for name in wanted {
            let Some((ty, bytes, origin)) = image_resource(rm, module, &name) else {
                self.pictures.remove(&name);
                if name != "gui_error" {
                    self.issues.push(format!("Missing image: {name}"));
                }
                continue;
            };
            self.origins.insert(format!("{name}.{ty}"), origin);
            let stamp = hash((ty.0, &bytes));
            if self.pictures.get(&name).is_some_and(|p| p.stamp == stamp) {
                continue;
            }
            match image(&bytes, ty) {
                Ok(image) => {
                    let size = vec2(image.size[0] as f32, image.size[1] as f32);
                    let texture = ctx.load_texture(
                        format!("nui/{name}/{stamp}"),
                        image,
                        egui::TextureOptions::LINEAR,
                    );
                    self.pictures.insert(name, Picture { texture, size, stamp });
                }
                Err(e) => {
                    self.pictures.remove(&name);
                    self.issues.push(format!("{name}: {e}"));
                }
            }
        }
        let fonts =
            self.style.get("fonts").and_then(toml::Value::as_array).cloned().unwrap_or_default();
        self.default_font = fonts
            .first()
            .and_then(|f| f.get("name"))
            .and_then(toml::Value::as_str)
            .unwrap_or("(default)")
            .into();
        for f in fonts.iter().take(32) {
            let name = f.get("name").and_then(toml::Value::as_str).unwrap_or("(default)");
            let resref = f.get("resref").and_then(toml::Value::as_str).unwrap_or("");
            let font = ResKey::parse(resref, ResType::TTF);
            self.watched.extend(font);
            let data = font.and_then(|key| resource(rm, module, key));
            let Some((bytes, origin)) = data else {
                self.issues.push(format!("Missing font: {resref}"));
                continue;
            };
            if bytes.len() > 32 * 1024 * 1024 || skrifa::FontRef::new(&bytes).is_err() {
                self.issues.push(format!("Invalid font: {resref}"));
                continue;
            }
            let font_key = format!("nui-font/{resref}/{:x}", hash(&bytes));
            let family = FontFamily::Name(font_key.clone().into());
            let em_ratio = pixel_height_to_em(&bytes).unwrap_or(1.0);
            ctx.add_font(egui::epaint::text::FontInsert {
                name: font_key,
                data: egui::FontData::from_owned(bytes),
                families: vec![egui::epaint::text::InsertFontFamily {
                    family: family.clone(),
                    priority: egui::epaint::text::FontPriority::Highest,
                }],
            });
            self.origins.insert(format!("{resref}.ttf"), origin);
            let num = |k: &str, default: f32| f.get(k).and_then(toml_number).unwrap_or(default);
            self.fonts.insert(
                name.into(),
                Font {
                    family,
                    size: num("font_size", 17.0).clamp(4.0, 128.0),
                    em_ratio,
                    spacing: num("spacing_h", 0.0).clamp(-2.0, 10.0),
                },
            );
        }
        self.held = held(module, &self.watched);
        ctx.request_repaint();
    }

    pub(super) fn number(&self, path: &str, default: f32) -> f32 {
        self.leaves.get(path).and_then(|v| v.number).unwrap_or(default)
    }
    pub(super) fn color(&self, path: &str, default: Color32) -> Color32 {
        self.leaves.get(path).and_then(|v| v.color).unwrap_or(default)
    }
    pub(super) fn picture(&self, name: &str) -> Option<&Picture> {
        // Pictures are kept by lowercase name, which most names already are.
        if name.bytes().all(|b| b.is_ascii() && !b.is_ascii_uppercase()) {
            self.pictures.get(name)
        } else {
            self.pictures.get(&name.to_lowercase())
        }
    }
    pub(super) fn style_picture(&self, path: &str) -> Option<&Picture> {
        self.pictures.get(self.leaves.get(path)?.picture.as_ref()?)
    }
    pub(super) fn font(&self, ctx: &egui::Context, name: &str, scale: f32) -> (FontId, f32) {
        let f = self.fonts.get(if name.is_empty() { self.default_font.as_str() } else { name });
        if let Some(f) = f
            && ctx.fonts(|fonts| fonts.definitions().families.contains_key(&f.family))
        {
            return (FontId::new(f.size * f.em_ratio * scale, f.family.clone()), f.spacing * scale);
        }
        (FontId::proportional(17.0 * scale), 0.0)
    }
    /// The galley for `key`: the one kept if it was drawn in the pass
    /// before or in this one, and otherwise what `lay_out` makes of it.
    pub(super) fn galley(
        &self,
        ctx: &egui::Context,
        key: &TextKey<'_>,
        lay_out: impl FnOnce() -> Arc<egui::Galley>,
    ) -> Arc<egui::Galley> {
        let hash = key.hash();
        if let Some(galley) = self.text.borrow_mut().get(ctx, key, hash) {
            return galley;
        }
        let galley = lay_out();
        self.text.borrow_mut().keep(key, hash, galley.clone());
        galley
    }

    pub(super) fn font_height(&self, name: &str) -> f32 {
        self.fonts
            .get(if name.is_empty() { self.default_font.as_str() } else { name })
            .map_or(17.0, |f| f.size)
    }
    pub(super) fn paint(
        &self,
        p: &Painter,
        path: &str,
        rect: Rect,
        scale: f32,
        sliced: bool,
        tint: Color32,
    ) -> bool {
        let Some(value) = self.leaves.get(path) else {
            return false;
        };
        if let Some(c) = value.color {
            p.rect_filled(rect, 0, c * tint);
            return true;
        }
        let Some(image) = value.picture.as_ref().and_then(|name| self.pictures.get(name)) else {
            return false;
        };
        if sliced {
            nine_slice(p, image, rect, scale, tint, path.ends_with("border_image"));
        } else {
            p.image(
                image.texture.id(),
                rect,
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                tint,
            );
        }
        true
    }

    /// A scrollbar's cursor: its picture repeated along the bar at the
    /// picture's own size (the stock 16-pixel ribs, as the client draws them),
    /// stretched across it.
    pub(super) fn paint_tiled(
        &self,
        p: &Painter,
        path: &str,
        rect: Rect,
        scale: f32,
        horizontal: bool,
        tint: Color32,
    ) -> bool {
        let Some(image) = self.style_picture(path) else {
            return self.paint(p, path, rect, scale, true, tint);
        };
        let axis = usize::from(!horizontal);
        let across = 1 - axis;
        let tile = (image.size[axis] * scale).max(1.0);
        // Without the picture's rounded ends across the bar, as in the client.
        let cap = 3.0_f32.min(image.size[across] / 4.0) / image.size[across];
        let mut at = rect.min[axis];
        while at < rect.max[axis] {
            let end = (at + tile).min(rect.max[axis]);
            let mut r = rect;
            r.min[axis] = at;
            r.max[axis] = end;
            let mut uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
            uv.min[across] = cap;
            uv.max[across] = 1.0 - cap;
            uv.max[axis] = (end - at) / tile;
            p.image(image.texture.id(), r, uv, tint);
            at = end;
        }
        true
    }
}

fn toml_number(v: &toml::Value) -> Option<f32> {
    v.as_float()
        .or_else(|| v.as_integer().map(|n| n as f64))
        .filter(|n| n.is_finite())
        .map(|n| n as f32)
}

fn color(s: &str) -> Option<Color32> {
    let s = s.strip_prefix('#')?;
    if s.len() != 8 {
        return None;
    }
    let n = u32::from_str_radix(s, 16).ok()?;
    Some(Color32::from_rgba_unmultiplied((n >> 24) as u8, (n >> 16) as u8, (n >> 8) as u8, n as u8))
}

fn cuts(start: f32, length: f32, source: f32, scale: f32) -> ([f32; 4], [f32; 4]) {
    // Four source pixels preserve the stock skin's bevel. This is a preview
    // policy, not a claim about the closed engine's nine-slice parameters.
    let edge = 4.0_f32.min(source / 3.0);
    let target = (edge * scale).min(length * 0.5);
    let end = start + length;
    let left = (start + target).min(end);
    let right = (end - target).max(left);
    ([start, left, right, end], [0.0, edge / source, 1.0 - edge / source, 1.0])
}

fn nine_slice(
    p: &Painter,
    image: &Picture,
    rect: Rect,
    scale: f32,
    tint: Color32,
    border_only: bool,
) {
    if !rect.is_positive() {
        return;
    }
    let (x, u) = cuts(rect.left(), rect.width(), image.size.x, scale);
    let (y, v) = cuts(rect.top(), rect.height(), image.size.y, scale);
    for row in 0..3 {
        for col in 0..3 {
            if border_only && row == 1 && col == 1 {
                continue;
            }
            let r = Rect::from_min_max(pos2(x[col], y[row]), pos2(x[col + 1], y[row + 1]));
            if r.is_positive() {
                p.image(
                    image.texture.id(),
                    r,
                    Rect::from_min_max(pos2(u[col], v[row]), pos2(u[col + 1], v[row + 1])),
                    tint,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_resman::{LayerClass, MemContainer};

    #[test]
    fn strref_preview_resolves_module_tlk_and_indexed_binds_without_mutating_source() {
        use crate::nui_view::preview::localized_string;
        use mg_core::{Language, StrRef};
        use mg_nui::Binding;
        use mg_tlk::{Tlk, TlkEntry};
        use serde_json::json;
        let mut base = Tlk::new(Language::ENGLISH);
        base.entries.push(TlkEntry::text("Base text"));
        let mut game = mg_rules::GameData::new(ResMan::new(), base);
        let custom = |text: &str| {
            let mut tlk = Tlk::new(Language::ENGLISH);
            tlk.entries.push(TlkEntry::text(text));
            tlk
        };
        game.set_custom_tlk(Some(custom("Module text")));
        let doc = json!({"title":{"strref":0},"root":{"type":"label","value":{"bind":"rows","text_flags":2}}});
        let mut settings = Settings::default();
        settings.bindings.insert("rows".into(), Binding {
            value: json!([{"strref":0},{"strref":StrRef::CUSTOM_FLAG},{"strref":-1},{"strref":987654}]),
            ..Default::default()
        });
        let original = (doc.clone(), settings.clone());
        let mut assets = Assets::default();
        assets.prepare_strings(Some(&game), &doc, &settings);
        assert_eq!(localized_string(&doc["title"], &settings, None, &assets), "Base text");
        let binding = &doc["root"]["value"];
        assert_eq!(localized_string(binding, &settings, Some(0), &assets), "BASE TEXT");
        assert_eq!(localized_string(binding, &settings, Some(1), &assets), "MODULE TEXT");
        assert_eq!(localized_string(binding, &settings, Some(2), &assets), "");
        assert!(localized_string(binding, &settings, Some(3), &assets).contains("987654"));
        game.set_custom_tlk(Some(custom("Reloaded text")));
        assets.prepare_strings(Some(&game), &doc, &settings);
        assert_eq!(localized_string(binding, &settings, Some(1), &assets), "RELOADED TEXT");
        assets.prepare_strings(None, &doc, &settings);
        assert_eq!(localized_string(&doc["title"], &settings, None, &assets), "[Missing StrRef 0]");
        assert_eq!((doc, settings), original);
    }

    #[test]
    fn the_skin_is_read_by_dotted_path() {
        let ctx = egui::Context::default();
        let mut module = Module::new();
        module.set(
            ResKey::parse("nui_skin", ResType::TML).unwrap(),
            b"[window]\nscrollbar_size_x = 20\nbackground = \"#102030ff\"\n\
              [window.header]\nnormal = \"Test_Image\"\nratio = 1.5\n\
              [odd]\n\"a.b\" = 7\n[odd.a]\nb = 8\n"
                .to_vec(),
        );
        module.set(ResKey::parse("test_image", ResType::TGA).unwrap(), tga(true));
        let mut assets = Assets::default();
        assets.prepare(&ctx, None, &module, 0, &mg_nui::window(), &Settings::default());
        assert!(assets.loaded, "{:?}", assets.issues);
        assert_eq!(assets.number("window.scrollbar_size_x", 18.0), 20.0);
        assert_eq!(assets.number("window.header.ratio", 0.0), 1.5);
        assert_eq!(assets.number("window.scrollbar_size_y", 18.0), 18.0);
        // A table, a string and a path through a value are no numbers.
        assert_eq!(assets.number("window.header", 3.0), 3.0);
        assert_eq!(assets.number("window.background", 3.0), 3.0);
        assert_eq!(assets.number("window.background.x", 3.0), 3.0);
        assert_eq!(
            assets.color("window.background", Color32::RED),
            Color32::from_rgba_unmultiplied(0x10, 0x20, 0x30, 0xff)
        );
        assert_eq!(assets.color("window.header.normal", Color32::RED), Color32::RED);
        // A picture is found whatever the case the skin names it in.
        assert!(assets.style_picture("window.header.normal").is_some());
        assert!(assets.style_picture("window.background").is_none());
        assert!(assets.picture("TEST_image").is_some());
        // A path's dots only ever separate tables.
        assert_eq!(assets.number("odd.a.b", 0.0), 8.0);
    }

    #[test]
    fn resources_observe_hak_live_module_override_order_and_dds_fallback() {
        let key = ResKey::parse("nui_skin", ResType::TML).unwrap();
        let mut rm = ResMan::new();
        let mut module = Module::new();
        let mut hak = MemContainer::new();
        hak.insert(key, b"hak".to_vec());
        let mut ovr = MemContainer::new();
        ovr.insert(key, b"override".to_vec());
        rm.add(priority::HAK, "hak", LayerClass::Erf, hak);
        rm.add(priority::OVERRIDE, "override", LayerClass::Directory, ovr);
        module.set(key, b"live module".to_vec());
        assert_eq!(resource(Some(&rm), &module, key).unwrap().0, b"hak");
        rm.remove("hak");
        assert_eq!(resource(Some(&rm), &module, key).unwrap().0, b"live module");
        assert_eq!(resource(Some(&rm), &Module::new(), key).unwrap().0, b"override");
        let mut base = MemContainer::new();
        base.insert(ResKey::parse("icon", ResType::PNG).unwrap(), b"png".to_vec());
        rm.add(priority::KEY, "base", LayerClass::Key, base);
        module.set(ResKey::parse("icon", ResType::DDS).unwrap(), b"dds".to_vec());
        assert_eq!(image_resource(Some(&rm), &module, "icon").unwrap().0, ResType::PNG);
    }

    fn tga(top_down: bool) -> Vec<u8> {
        let mut b = vec![0u8; 18];
        b[2] = 2;
        b[12] = 1;
        b[14] = 2;
        b[16] = 24;
        b[17] = if top_down { 32 } else { 0 };
        b.extend_from_slice(if top_down {
            &[0, 0, 255, 255, 0, 0]
        } else {
            &[255, 0, 0, 0, 0, 255]
        });
        b
    }

    #[test]
    fn native_draw_image_ignores_an_old_tint_and_alpha() {
        use serde_json::json;
        let ctx = egui::Context::default();
        let mut module = Module::new();
        module.set(ResKey::parse("test_image", ResType::TGA).unwrap(), tga(true));
        let s = Settings::default();
        let mut assets = Assets::default();
        assets.prepare(&ctx, None, &module, 0, &json!({"type":"image","value":"test_image"}), &s);
        let texture = assets.picture("test_image").unwrap().texture.id();
        for alpha in [0, 128, 255] {
            let host = json!({"draw_list":[{"type":5,"enabled":true,"image":"test_image",
                "rect":{"x":10.0,"y":10.0,"w":140.0,"h":70.0},
                "color":{"r":220,"g":180,"b":80,"a":alpha}}]});
            let mut output = ctx.run_ui(Default::default(), |ui| {
                crate::nui_view::draw::paint(
                    &ui.ctx().layer_painter(egui::LayerId::background()),
                    &host,
                    Rect::from_min_size(pos2(20.0, 20.0), vec2(200.0, 110.0)),
                    1.0,
                    &s,
                    &assets,
                    None,
                    false,
                    [false; 4],
                );
            });
            let mesh = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Mesh(mesh) if mesh.texture_id == texture => Some(mesh),
                    _ => None,
                })
                .expect("Draw Image must actually paint its texture");
            assert!(
                mesh.vertices.iter().all(|v| v.color == Color32::WHITE),
                "NuiDrawListImage has no tint, including legacy alpha={alpha}"
            );
            output.textures_delta.clear();
        }
    }

    #[test]
    fn native_button_image_stretches_while_plain_image_keeps_aspect() {
        use serde_json::json;
        let ctx = egui::Context::default();
        let mut module = Module::new();
        module.set(ResKey::parse("test_image", ResType::TGA).unwrap(), tga(true));
        let s = Settings::default();
        let mut assets = Assets::default();
        assets.prepare(
            &ctx,
            None,
            &module,
            0,
            &json!({"type":"button_image","label":"test_image"}),
            &s,
        );
        let target = Rect::from_min_size(pos2(20.0, 20.0), vec2(150.0, 64.0));
        let texture = assets.picture("test_image").unwrap().texture.id();
        for (ty, width) in [("button_image", 150.0), ("image", 32.0)] {
            let mut output = ctx.run_ui(Default::default(), |ui| {
                let painter = ui.ctx().layer_painter(egui::LayerId::background());
                assert!(crate::nui_view::preview::draw_image(
                    &painter,
                    &assets,
                    "test_image",
                    target,
                    crate::nui_view::preview::Props { node: &json!({"type":ty}), s: &s, row: None },
                    1.0,
                    Color32::WHITE,
                ));
            });
            let mesh = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Mesh(mesh) if mesh.texture_id == texture => Some(mesh),
                    _ => None,
                })
                .unwrap();
            let bounds =
                Rect::from_points(&mesh.vertices.iter().map(|v| v.pos).collect::<Vec<_>>());
            assert_eq!(bounds.width(), width, "{ty}");
            assert_eq!(bounds.height(), 64.0);
            // This test inspects paint meshes; no GPU backend consumes deltas.
            output.textures_delta.clear();
        }
    }

    #[test]
    fn native_crop_changes_uv_without_changing_image_aspect() {
        use serde_json::json;
        let ctx = egui::Context::default();
        let mut module = Module::new();
        module.set(ResKey::parse("test_image", ResType::TGA).unwrap(), tga(true));
        let s = Settings::default();
        let mut assets = Assets::default();
        assets.prepare(&ctx, None, &module, 0, &json!({"type":"image","value":"test_image"}), &s);
        let texture = assets.picture("test_image").unwrap().texture.id();
        // The native Crop demo keeps the full texture's aspect even when only
        // half of that texture is sampled. A crop is a UV region, not a new image.
        for (ty, width) in [("image", 32.0), ("button_image", 150.0)] {
            let mut output = ctx.run_ui(Default::default(), |ui| {
                assert!(crate::nui_view::preview::draw_image(
                    &ui.ctx().layer_painter(egui::LayerId::background()),
                    &assets,
                    "test_image",
                    Rect::from_min_size(pos2(20.0, 20.0), vec2(150.0, 64.0)),
                    crate::nui_view::preview::Props {
                        node: &json!({"type":ty,"image_region":{"x":0.0,"y":1.0,"w":1.0,"h":1.0}}),
                        s: &s,
                        row: None,
                    },
                    1.0,
                    Color32::WHITE,
                ));
            });
            output.textures_delta.clear();
            let mesh = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Mesh(mesh) if mesh.texture_id == texture => Some(mesh),
                    _ => None,
                })
                .unwrap();
            let bounds =
                Rect::from_points(&mesh.vertices.iter().map(|v| v.pos).collect::<Vec<_>>());
            let uv = Rect::from_points(&mesh.vertices.iter().map(|v| v.uv).collect::<Vec<_>>());
            assert_eq!(bounds.size(), vec2(width, 64.0), "{ty}");
            assert_eq!(uv, Rect::from_min_max(pos2(0.0, 0.5), pos2(1.0, 1.0)));
        }
    }

    #[test]
    fn tga_origin_is_respected_and_slice_edges_do_not_cross() {
        let a = image(&tga(false), ResType::TGA).unwrap();
        let b = image(&tga(true), ResType::TGA).unwrap();
        assert_eq!(a.pixels, b.pixels);
        assert_eq!(a.pixels, [Color32::RED, Color32::BLUE]);
        for target in [0.1, 2.0, 8.0, 40.0, 300.0] {
            let (x, uv) = cuts(7.0, target, 42.0, 2.0);
            assert!(x.windows(2).all(|w| w[1] >= w[0]));
            assert!(uv.windows(2).all(|w| w[1] >= w[0]));
            assert_eq!(x[3], 7.0 + target);
        }
    }

    /// A TGA header: no ID, no color map.
    fn tga_header(kind: u8, width: u16, height: u16, bits: u8, descriptor: u8) -> Vec<u8> {
        let mut b = vec![0, 0, kind, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        b.extend(width.to_le_bytes());
        b.extend(height.to_le_bytes());
        b.extend([bits, descriptor]);
        b
    }

    /// The workspace's TGA reader keeps rows and columns as stored; a
    /// window's picture is the one its file's origin declares, as the
    /// `image` crate read it (compared pixel for pixel when the reader was
    /// changed: 8-bit gray, 24- and 32-bit and color-mapped, plain and
    /// run-length encoded, from all four corners).
    #[test]
    fn tga_is_read_from_the_corner_it_declares() {
        let [red, green, blue, white] = [[0, 0, 255], [0, 255, 0], [255, 0, 0], [255, 255, 255]];
        let want = [Color32::RED, Color32::GREEN, Color32::BLUE, Color32::WHITE];
        // The same picture (red, green over blue, white) stored from each corner.
        for (descriptor, stored) in [
            (0x20, [red, green, blue, white]),
            (0x00, [blue, white, red, green]),
            (0x30, [green, red, white, blue]),
            (0x10, [white, blue, green, red]),
        ] {
            let mut plain = tga_header(2, 2, 2, 24, descriptor);
            plain.extend(stored.as_flattened());
            assert_eq!(image(&plain, ResType::TGA).unwrap().pixels, want, "{descriptor:#x}");
            // Run-length encoded, with alpha: a raw packet of three, a run of one.
            let mut packed = tga_header(10, 2, 2, 32, descriptor | 8);
            packed.push(2);
            for pixel in &stored[..3] {
                packed.extend(pixel);
                packed.push(255);
            }
            packed.push(0x80);
            packed.extend(stored[3]);
            packed.push(255);
            assert_eq!(image(&packed, ResType::TGA).unwrap().pixels, want, "{descriptor:#x}");
        }
        let mut gray = tga_header(3, 2, 1, 8, 0x20);
        gray.extend([0, 200]);
        let gray = image(&gray, ResType::TGA).unwrap();
        assert_eq!(gray.pixels, [Color32::BLACK, Color32::from_gray(200)]);
        let mut translucent = tga_header(2, 1, 1, 32, 8);
        translucent.extend([10, 20, 30, 128]);
        assert_eq!(
            image(&translucent, ResType::TGA).unwrap().pixels,
            [Color32::from_rgba_unmultiplied(30, 20, 10, 128)]
        );
        // Bad input is an issue to show, never a panic.
        assert!(image(&tga_header(2, 4097, 1, 24, 0), ResType::TGA).is_err());
        // A few bytes that declare a gigabyte of pixels are refused by their
        // header, not unpacked.
        let mut bomb = tga_header(10, 16_384, 16_384, 32, 8);
        bomb.extend([0xff, 1, 2, 3, 4].repeat(4));
        let refused = image(&bomb, ResType::TGA).unwrap_err();
        assert!(refused.contains("4096 pixels a side"), "{refused}");
        assert!(image(&plain_tga_cut_short(), ResType::TGA).is_err());
        assert!(image(b"", ResType::TGA).is_err());
    }

    fn plain_tga_cut_short() -> Vec<u8> {
        let mut b = tga_header(2, 2, 2, 24, 0);
        b.extend([1, 2, 3]);
        b
    }

    fn png_of(
        color: png::ColorType,
        depth: png::BitDepth,
        palette: Option<(&[u8], &[u8])>,
        data: &[u8],
    ) -> Vec<u8> {
        let mut out = Vec::new();
        let mut encoder = png::Encoder::new(&mut out, 2, 1);
        encoder.set_color(color);
        encoder.set_depth(depth);
        if let Some((palette, alpha)) = palette {
            encoder.set_palette(palette.to_vec());
            encoder.set_trns(alpha.to_vec());
        }
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(data).unwrap();
        drop(writer);
        out
    }

    /// PNGs of every color type become RGBA8 as the `image` crate made them
    /// (compared pixel for pixel when the reader was changed): 16-bit
    /// samples rounded, palettes and their transparency unpacked.
    #[test]
    fn png_of_any_kind_is_read_as_rgba() {
        use png::{BitDepth, ColorType};
        // (As the picture holds them: a color multiplied by its alpha.)
        let pixels = |bytes: &[u8]| {
            let image = image(bytes, ResType::PNG).unwrap();
            assert_eq!(image.size, [2, 1]);
            image.pixels
        };
        let colors = |c: [[u8; 4]; 2]| {
            c.map(|[r, g, b, a]| Color32::from_rgba_unmultiplied(r, g, b, a)).to_vec()
        };
        let rgb = png_of(ColorType::Rgb, BitDepth::Eight, None, &[1, 2, 3, 4, 5, 6]);
        assert_eq!(pixels(&rgb), colors([[1, 2, 3, 255], [4, 5, 6, 255]]));
        let rgba = png_of(ColorType::Rgba, BitDepth::Eight, None, &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(pixels(&rgba), colors([[1, 2, 3, 4], [5, 6, 7, 8]]));
        let gray = png_of(ColorType::Grayscale, BitDepth::One, None, &[0b0100_0000]);
        assert_eq!(pixels(&gray), colors([[0, 0, 0, 255], [255, 255, 255, 255]]));
        let gray_alpha = png_of(ColorType::GrayscaleAlpha, BitDepth::Eight, None, &[9, 8, 7, 6]);
        assert_eq!(pixels(&gray_alpha), colors([[9, 9, 9, 8], [7, 7, 7, 6]]));
        // 0x0080 is nearer 0 than 1, 0x0081 nearer 1; 0xff7f is 254.5 and a bit.
        let deep = png_of(ColorType::Grayscale, BitDepth::Sixteen, None, &[0, 0x80, 0, 0x81]);
        assert_eq!(pixels(&deep), colors([[0, 0, 0, 255], [1, 1, 1, 255]]));
        let deep = png_of(
            ColorType::Rgba,
            BitDepth::Sixteen,
            None,
            &[0xff, 0x7f, 0x80, 0, 0x12, 0x34, 0xff, 0xff, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        assert_eq!(pixels(&deep), colors([[255, 128, 18, 255], [0, 0, 0, 0]]));
        let indexed = png_of(
            ColorType::Indexed,
            BitDepth::Eight,
            Some((&[10, 20, 30, 40, 50, 60], &[77])),
            &[1, 0],
        );
        assert_eq!(pixels(&indexed), colors([[40, 50, 60, 255], [10, 20, 30, 77]]));
        assert!(image(&rgb[..rgb.len() / 2], ResType::PNG).is_err());
        assert!(image(b"not a picture", ResType::PNG).is_err());
    }

    #[test]
    fn asset_cache_reuses_textures_and_reloads_changed_bytes() {
        let ctx = egui::Context::default();
        let mut module = Module::new();
        let skin = ResKey::parse("nui_skin", ResType::TML).unwrap();
        let tex = ResKey::parse("test_image", ResType::TGA).unwrap();
        module.set(skin, b"[button]\nnormal = \"test_image\"\n".to_vec());
        module.set(tex, tga(false));
        let mut assets = Assets::default();
        let v = mg_nui::window();
        let s = Settings::default();
        assets.prepare(&ctx, None, &module, 0, &v, &s);
        assert!(assets.issues.is_empty());
        let first = assets.picture("test_image").unwrap().texture.id();
        assets.invalidate();
        assets.prepare(&ctx, None, &module, 1, &v, &s);
        assert_eq!(first, assets.picture("test_image").unwrap().texture.id());
        // An edit of something else in the module reads nothing again.
        module.set(ResKey::parse("other", ResType::NSS).unwrap(), b"void main() {}".to_vec());
        assets.issues.push("kept".into());
        assets.prepare(&ctx, None, &module, 2, &v, &s);
        assert_eq!(assets.issues, ["kept"]);
        assert!(assets.catalog.contains(&"test_image".to_owned()));
        let mut changed = tga(false);
        changed[18] = 127;
        module.set(tex, changed);
        assets.prepare(&ctx, None, &module, 3, &v, &s);
        assert_ne!(first, assets.picture("test_image").unwrap().texture.id());
        // A picture new to the module is listed, and one the window names
        // that was missing is found.
        module.set(ResKey::parse("gui_error", ResType::TGA).unwrap(), tga(false));
        assert!(assets.picture("gui_error").is_none());
        assets.prepare(&ctx, None, &module, 4, &v, &s);
        assert!(assets.picture("gui_error").is_some());
        assert!(assets.catalog.contains(&"gui_error".to_owned()));
        module.set(skin, b"broken = [".to_vec());
        assets.prepare(&ctx, None, &module, 5, &v, &s);
        assert!(!assets.loaded);
        assert!(!assets.issues.is_empty());
        assert!(assets.picture("test_image").is_none());
    }
}
