//! Game images in the UI (portraits, icons): a texture found as the game
//! finds it (DDS over TGA, as the engine prefers, the module being edited
//! below its haks), decoded once and kept as an egui texture.

use std::collections::HashMap;
use std::sync::Arc;

use mg_core::{ResRef, ResType};
use mg_edit::Workspace;
use mg_gff::Struct;
use mg_resman::ResKey;
use mg_rules::GameData;
use mg_rules::items::{ARMOR_ICON, icon_names as item_icon_names};

use crate::Moonglow;
use egui::Ui;

/// A decoded image: its texture and size in pixels.
#[derive(Clone)]
pub struct Picture {
    pub texture: egui::TextureHandle,
    pub size: egui::Vec2,
    /// The part of it that is drawn (not transparent), in texture
    /// coordinates (0 to 1).
    pub drawn: egui::Rect,
}

impl std::fmt::Debug for Picture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Picture({:?})", self.size)
    }
}

/// Decoded images, by lowercase name (and colouring, for PLTs).
pub(crate) type Pictures = HashMap<String, Option<Picture>>;

/// Decoded palettes (`pal_*`), by name.
pub(crate) type Palettes = HashMap<String, Option<Arc<mg_image::Rgba>>>;

/// Loads images from the module and the game into the cache.
pub(crate) struct Loader<'a> {
    pub pictures: &'a mut Pictures,
    pub palettes: &'a mut Palettes,
    pub ws: Option<&'a Workspace>,
    pub game: &'a GameData,
}

/// The PLT layers' colours an item gives (metal, cloth and leather; skin,
/// hair and tattoos the first colour).
pub(crate) fn item_colors(item: &Struct) -> [u8; 10] {
    let mut colors = [0; 10];
    for channel in mg_rules::items::ArmorChannel::ALL {
        colors[channel.plt_layer()] =
            item.integer(channel.field()).unwrap_or(0).clamp(0, 255) as u8;
    }
    colors
}

/// The PLT layers' colours an armor's part `part` (an index of
/// [`mg_rules::items::ARMOR_PARTS`]) is drawn in: the part's own where it
/// has them (EE), else the armor's. The game's inventory icon draws each
/// part's layer so (seen in the client: `reading_inventory_icons`).
pub(crate) fn part_colors(item: &Struct, part: usize) -> [u8; 10] {
    let mut colors = item_colors(item);
    for channel in mg_rules::items::ArmorChannel::ALL {
        if let Some(c) = mg_rules::items::armor_part_color(item, part, channel) {
            colors[channel.plt_layer()] = c;
        }
    }
    colors
}

/// The armor part (an index of [`mg_rules::items::ARMOR_PARTS`]) each
/// layer of an armor's icon is ([`item_icon_names`]'s, bottom first).
fn armor_icon_parts(item: &Struct) -> Vec<usize> {
    ARMOR_ICON
        .iter()
        .filter(|(l, _)| part(item, l) > 0)
        .filter_map(|(l, _)| mg_rules::items::ARMOR_PARTS.iter().position(|(f, _)| f == l))
        .collect()
}

/// A model or part number: the EE twin (`xModelPart1`) where there is one.
fn part(item: &Struct, label: &str) -> i64 {
    mg_rules::items::part_number(item, label).unwrap_or(0)
}

/// Whether an item's icon is an armor's, of parts (baseitems.2da
/// `ModelType` 3).
fn is_armor(game: &GameData, item: &Struct) -> bool {
    let Ok(t) = game.table("baseitems") else { return false };
    usize::try_from(item.integer("BaseItem").unwrap_or(-1))
        .is_ok_and(|row| t.get_int(row, "ModelType") == Some(3))
}

/// Where a resource comes from in the load order, as a builder names it,
/// by its layer's label ([`mg_resman::Layer::label`]): a hak by name, the
/// game's own data, the module, the override and development folders, a
/// Workshop item, a userpatch hak; any other label as it is.
pub(crate) fn origin_name(layer: &str) -> String {
    if let Some(hak) = layer.strip_prefix("hak:") {
        format!("Hak: {hak}")
    } else if let Some(key) = layer.strip_prefix("key:") {
        format!("Game data ({key})")
    } else if layer.starts_with("module") {
        "Module".into()
    } else if layer == "override" {
        "Override folder".into()
    } else if layer == "development" {
        "Development folder".into()
    } else if let Some(item) = layer.strip_prefix("workshop:") {
        format!("Workshop item {item}")
    } else if let Some(hak) = layer.strip_prefix("userpatch:") {
        format!("Userpatch hak: {hak}")
    } else {
        layer.into()
    }
}

/// The part of a loading screen's texture each mesh of `pnl_loadscreen`
/// shows, left to right: its texture coordinates' rectangle `[u0, v0, u1,
/// v1]`.
type ScreenParts = Vec<[f32; 4]>;

/// The game's own `pnl_loadscreen`'s: the left half of the picture in the
/// texture's lower half from its left edge, the right half in its upper
/// half up to its right edge, each 25/32 of its width.
const STOCK_SCREEN: [[f32; 4]; 2] = [[0.0, 0.0, 0.781_25, 0.5], [0.218_75, 0.5, 1.0, 1.0]];

/// How a `pnl_loadscreen` model (`model`, the one the game loads: a hak's
/// own, else the game's) lays the loading screen out: each mesh drawn with
/// `gui_replace01`, the texture the game puts the screen's picture in, as
/// the rectangle of its texture coordinates, by where the mesh stands
/// from the left. `None` where none is drawn so.
fn screen_parts(model: &[u8]) -> Option<ScreenParts> {
    let model = mg_mdl::Model::read(model).ok()?;
    let mut parts: Vec<(f32, [f32; 4])> = Vec::new();
    for node in &model.nodes {
        let Some(mesh) = node.mesh() else { continue };
        let replaced = mesh.textures[0].as_deref().is_some_and(|t| t == "gui_replace01");
        if !replaced || mesh.uvs[0].is_empty() {
            continue;
        }
        let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
        for uv in &mesh.uvs[0] {
            for i in 0..2 {
                lo[i] = lo[i].min(uv[i].clamp(0.0, 1.0));
                hi[i] = hi[i].max(uv[i].clamp(0.0, 1.0));
            }
        }
        let left = mesh.vertices.iter().map(|v| v[0]).fold(f32::MAX, f32::min);
        parts.push((node.position[0] + left, [lo[0], lo[1], hi[0], hi[1]]));
    }
    parts.sort_by(|a, b| a.0.total_cmp(&b.0));
    let parts: ScreenParts = parts.into_iter().map(|(_, r)| r).collect();
    (!parts.is_empty()).then_some(parts)
}

/// A loading screen's picture from its texture (rows from the bottom, as
/// stored), its `parts` side by side ([`screen_parts`]): each as many
/// pixels as it covers of the texture, as high as the lowest of them.
fn stitched(whole: &mg_image::Rgba, parts: &[[f32; 4]]) -> Option<mg_image::Rgba> {
    let (w, h) = (whole.width as usize, whole.height as usize);
    if w < 2 || h < 2 || whole.data.len() != w * h * 4 || parts.is_empty() {
        return None;
    }
    let px = |f: f32, n: usize| ((f * n as f32).round() as usize).min(n);
    let rects: Vec<[usize; 4]> =
        parts.iter().map(|p| [px(p[0], w), px(p[1], h), px(p[2], w), px(p[3], h)]).collect();
    let height = rects.iter().map(|r| r[3].saturating_sub(r[1])).min()?;
    let width: usize = rects.iter().map(|r| r[2].saturating_sub(r[0])).sum();
    if height == 0 || width == 0 {
        return None;
    }
    let mut data = Vec::with_capacity(width * height * 4);
    for y in 0..height {
        for r in &rects {
            let row = (r[1] + y) * w;
            data.extend_from_slice(&whole.data[(row + r[0]) * 4..(row + r[2]) * 4]);
        }
    }
    Some(mg_image::Rgba { width: width as u32, height: height as u32, data })
}

/// A texture as pixels no more than `side` across: its first mip level that
/// small (else its smallest), then every `n`th pixel of that.
fn small(texture: &mg_image::Texture, side: u32) -> mg_image::Rgba {
    let level = (0..texture.mips.len())
        .find(|&l| {
            let (w, h) = texture.level_dims(l);
            w.max(h) <= side
        })
        .unwrap_or(texture.mips.len().saturating_sub(1));
    let (w, h) = texture.level_dims(level);
    let data = texture.mips.get(level).map_or(&[][..], Vec::as_slice);
    let mut rgba = mg_image::Rgba {
        width: w,
        height: h,
        data: mg_image::bc::decode(texture.format, w, h, data),
    };
    if !texture.has_alpha {
        for px in rgba.data.as_chunks_mut::<4>().0 {
            px[3] = 255;
        }
    }
    let step = w.max(h).div_ceil(side.max(1)).max(1);
    if step == 1 {
        return rgba;
    }
    let (sw, sh) = (w.div_ceil(step), h.div_ceil(step));
    let mut out = mg_image::Rgba::new(sw, sh);
    for y in 0..sh {
        for x in 0..sw {
            let from = (((y * step) * w + x * step) * 4) as usize;
            let to = ((y * sw + x) * 4) as usize;
            out.data[to..to + 4].copy_from_slice(&rgba.data[from..from + 4]);
        }
    }
    out
}

impl Loader<'_> {
    /// A resource's bytes as the game reads them: the module being edited
    /// (its unsaved changes too) in its place in the load order, below its
    /// haks and over `override/` (`ResMan::get_over`).
    fn data(&self, key: ResKey) -> Option<Arc<[u8]>> {
        let rm = &self.game.resman;
        let Some(ws) = self.ws else { return rm.get(&key).ok().map(|d| Arc::from(&*d)) };
        let module = |k: &ResKey| ws.module.get(k).map(std::borrow::Cow::Borrowed);
        rm.get_over(&key, module).map(|(d, _)| Arc::from(&*d))
    }

    /// A texture's type and bytes as the game finds it (DDS over TGA in a
    /// class of layers, the module being edited among the ERFs below its
    /// haks: `ResMan::texture_over`). A portrait both the module and a hak
    /// have is the hak's, as in the game.
    fn texture(&self, name: ResRef) -> Option<(ResType, Arc<[u8]>)> {
        let rm = &self.game.resman;
        let found = match self.ws {
            Some(ws) => {
                let module = |k: &ResKey| ws.module.get(k).map(std::borrow::Cow::Borrowed);
                rm.texture_over(name, module).map(|(t, d, _)| (t, d))
            }
            None => rm.texture(name),
        };
        found.map(|(t, d)| (t, Arc::from(&*d)))
    }

    fn keep(
        &mut self,
        ctx: &egui::Context,
        key: String,
        rgba: Option<mg_image::Rgba>,
    ) -> Option<Picture> {
        let picture = rgba.map(|rgba| {
            let rgba = rgba.top_down();
            let size = [rgba.width as usize, rgba.height as usize];
            let image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba.data);
            let texture = ctx.load_texture(&key, image, egui::TextureOptions::LINEAR);
            let drawn = drawn(&rgba);
            Picture { texture, size: egui::vec2(size[0] as f32, size[1] as f32), drawn }
        });
        self.pictures.insert(key, picture.clone());
        picture
    }

    /// A palette image (`pal_cloth01`, ...) as pixels, loaded once.
    pub(crate) fn palette(&mut self, name: &str) -> Option<Arc<mg_image::Rgba>> {
        // (By its name in lower case, as `picture`'s: one image, however
        // it is asked for.)
        let name = name.to_ascii_lowercase();
        if let Some(p) = self.palettes.get(&name) {
            return p.clone();
        }
        let rgba = ResRef::from_str(&name).ok().and_then(|r| {
            let (t, data) = self.texture(r)?;
            Some(Arc::new(mg_image::read(t, &data).ok()?.to_rgba()))
        });
        self.palettes.insert(name, rgba.clone());
        rgba
    }

    /// The image named `name` (a texture resref), loaded once.
    pub(crate) fn picture(&mut self, ctx: &egui::Context, name: &str) -> Option<Picture> {
        let key = name.to_ascii_lowercase();
        if let Some(p) = self.pictures.get(&key) {
            return p.clone();
        }
        let rgba = ResRef::from_str(&key).ok().and_then(|r| {
            let (t, data) = self.texture(r)?;
            Some(mg_image::read(t, &data).ok()?.to_rgba())
        });
        self.keep(ctx, key, rgba)
    }

    /// A small picture of the texture named `name`, no more than `side`
    /// pixels across, loaded once: a DDS's mip level of about that size
    /// (decoding the whole texture took too long for a gallery of
    /// thousands), a TGA scaled down.
    pub(crate) fn thumbnail(
        &mut self,
        ctx: &egui::Context,
        name: &str,
        side: u32,
    ) -> Option<Picture> {
        let key = format!("{}#thumb{side}", name.to_ascii_lowercase());
        if let Some(p) = self.pictures.get(&key) {
            return p.clone();
        }
        let rgba = ResRef::from_str(name).ok().and_then(|r| {
            let (t, data) = self.texture(r)?;
            Some(small(&mg_image::read(t, &data).ok()?, side))
        });
        self.keep(ctx, key, rgba)
    }

    /// A loading screen's picture as the game shows it, loaded once: the
    /// parts of its texture the `pnl_loadscreen` model the game loads (a
    /// hak's own, else the game's) maps, side by side ([`screen_parts`];
    /// the game's own model's where none is read: [`STOCK_SCREEN`]). A
    /// texture not found has the small preview Aurora shows in its place
    /// (`<name>s`), as it is, where there is one.
    pub(crate) fn load_screen(&mut self, ctx: &egui::Context, name: &str) -> Option<Picture> {
        let key = format!("{}#loadscreen", name.to_ascii_lowercase());
        if let Some(p) = self.pictures.get(&key) {
            return p.clone();
        }
        let model = ResRef::from_str("pnl_loadscreen").ok().map(|r| ResKey::new(r, ResType::MDL));
        let parts = model.and_then(|k| self.data(k)).and_then(|m| screen_parts(&m));
        let parts = parts.unwrap_or_else(|| STOCK_SCREEN.to_vec());
        let read = |r: ResRef| -> Option<mg_image::Rgba> {
            let (t, data) = self.texture(r)?;
            Some(mg_image::read(t, &data).ok()?.to_rgba())
        };
        let rgba = ResRef::from_str(name)
            .ok()
            .and_then(read)
            .map(|whole| stitched(&whole, &parts).unwrap_or(whole));
        let small = || ResRef::from_str(&format!("{name}s")).ok().and_then(read);
        let rgba = rgba.or_else(small);
        self.keep(ctx, key, rgba)
    }

    /// A PLT image coloured with a colour per layer, loaded once per
    /// colouring; `None` if there is no such PLT.
    pub(crate) fn plt_picture(
        &mut self,
        ctx: &egui::Context,
        name: &str,
        colors: [u8; 10],
    ) -> Option<Picture> {
        let key = format!("{}#{colors:?}", name.to_ascii_lowercase());
        if let Some(p) = self.pictures.get(&key) {
            return p.clone();
        }
        let rgba = ResRef::from_str(name).ok().and_then(|r| {
            let plt = mg_image::plt::Plt::read(&self.data(ResKey::new(r, ResType::PLT))?).ok()?;
            // (The ten palettes, read once for every colouring.)
            Some(plt.colorize_by(|p| self.palette(p), colors))
        });
        self.keep(ctx, key, rgba)
    }

    /// A portrait's image: `po_<base><size>` (h, l, m, s, t), the part a
    /// portrait uses (the upper 100/128 of its canvas), at `width` points;
    /// named after the picture (`po_<base>`), or the base in parentheses
    /// when there is no picture.
    pub(crate) fn portrait(
        &mut self,
        ui: &mut Ui,
        base: &str,
        size: char,
        width: f32,
        sense: egui::Sense,
    ) -> egui::Response {
        let pic = self.picture(ui.ctx(), &format!("po_{base}{size}"));
        let used = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 100.0 / 128.0));
        let at = egui::vec2(width, width * 100.0 / 64.0);
        match pic {
            // The used part (64 × 100 of 64 × 128) fills the size exactly;
            // keeping the whole texture's proportions drew it narrower.
            Some(p) => ui.add(
                egui::Image::new(&p.texture)
                    .uv(used)
                    .fit_to_exact_size(at)
                    .maintain_aspect_ratio(false)
                    .alt_text(format!("po_{base}"))
                    .sense(sense),
            ),
            None => ui.add_sized(at, egui::Label::new(format!("({base})")).sense(sense)),
        }
    }

    /// An item's inventory icon, its layers bottom first (PLT layers in
    /// the item's colours), else the base item's `DefaultIcon`; as a
    /// woman's inventory shows it where `female` (a layer without a
    /// picture of hers is the man's: the game has none of a cloak).
    pub(crate) fn item_icon_on(
        &mut self,
        ctx: &egui::Context,
        item: &Struct,
        female: bool,
    ) -> Vec<Picture> {
        let names = item_icon_names(self.game, item, female);
        let his = if female { item_icon_names(self.game, item, false) } else { Vec::new() };
        let colors = item_colors(item);
        // (An armor's layers each in its part's colors: its own where it
        // has them.)
        let parts = if is_armor(self.game, item) { armor_icon_parts(item) } else { Vec::new() };
        let colors_of = |i: usize| parts.get(i).map_or(colors, |&p| part_colors(item, p));
        let one = |l: &mut Self, n: &str, colors| {
            l.plt_picture(ctx, n, colors).or_else(|| l.picture(ctx, n))
        };
        let mut layers: Vec<Picture> = names
            .iter()
            .enumerate()
            .filter_map(|(i, n)| {
                let c = colors_of(i);
                one(self, n, c)
                    .or_else(|| his.get(i).filter(|m| *m != n).and_then(|m| one(self, m, c)))
            })
            .collect();
        if layers.is_empty() {
            let default = self.game.table("baseitems").ok().and_then(|t| {
                let row = usize::try_from(item.integer("BaseItem").unwrap_or(-1)).ok()?;
                t.get(row, "DefaultIcon").map(str::to_string)
            });
            if let Some(d) = default {
                layers.extend(self.picture(ctx, &d));
            }
        }
        layers
    }

    /// One part of a composite item's icon alone (`letter`: b, m or t; its
    /// number in the item's `label`), colored as the item is.
    pub(crate) fn item_icon_part(
        &mut self,
        ctx: &egui::Context,
        item: &Struct,
        (label, letter): (&str, &str),
    ) -> Option<Picture> {
        let t = self.game.table("baseitems").ok()?;
        let row = usize::try_from(item.integer("BaseItem").unwrap_or(-1)).ok()?;
        let class = t.get(row, "ItemClass").unwrap_or_default().to_ascii_lowercase();
        let name = format!("i{class}_{letter}_{:03}", part(item, label));
        self.plt_picture(ctx, &name, item_colors(item)).or_else(|| self.picture(ctx, &name))
    }
}

impl Moonglow {
    /// The image loader, while there is game data.
    pub(crate) fn loader(&mut self) -> Option<Loader<'_>> {
        let game = self.game.as_deref()?;
        Some(Loader {
            pictures: &mut self.pictures,
            palettes: &mut self.palettes,
            ws: self.ws.as_ref(),
            game,
        })
    }

    /// [`Moonglow::loader`], and the thumbnails made to look up beside it.
    pub(crate) fn loader_and_thumbnails(
        &mut self,
    ) -> Option<(Loader<'_>, &crate::model_view::Thumbnails)> {
        let game = self.game.as_deref()?;
        let loader = Loader {
            pictures: &mut self.pictures,
            palettes: &mut self.palettes,
            ws: self.ws.as_ref(),
            game,
        };
        Some((loader, &self.thumbnails))
    }

    /// The image named `name` (a texture resref), loaded once; `None` if
    /// there is none or it does not decode.
    pub fn picture(&mut self, ctx: &egui::Context, name: &str) -> Option<Picture> {
        self.loader()?.picture(ctx, name)
    }

    /// An item's inventory icon, its layers bottom first: a woman's where
    /// armor is shown on one (Options › General).
    pub fn item_icon(&mut self, ctx: &egui::Context, item: &Struct) -> Vec<Picture> {
        let female = self.settings.armor_on_woman;
        self.item_icon_of(ctx, item, female)
    }

    /// An item's inventory icon as a woman's inventory shows it, where
    /// `female`, else as a man's: a creature's own (the game shows a
    /// woman's armor with her `ipf_` icons, seen in the client).
    pub fn item_icon_of(
        &mut self,
        ctx: &egui::Context,
        item: &Struct,
        female: bool,
    ) -> Vec<Picture> {
        self.loader().map(|mut l| l.item_icon_on(ctx, item, female)).unwrap_or_default()
    }
}

/// Whether a portrait has a picture to show, in the module or the game
/// (portraits.2da lists some the game has no pictures for: `matron`,
/// `tm_tnobrl3_`).
pub(crate) fn has_portrait(game: &GameData, ws: Option<&mg_edit::Workspace>, base: &str) -> bool {
    ['m', 's', 'l', 'h', 't'].into_iter().any(|size| {
        let Ok(name) = ResRef::from_str(&format!("po_{}{size}", base.to_ascii_lowercase())) else {
            return false;
        };
        let module = ws.is_some_and(|ws| {
            [ResType::DDS, ResType::TGA]
                .into_iter()
                .any(|t| ws.module.get(&ResKey::new(name, t)).is_some())
        });
        module || game.resman.texture(name).is_some()
    })
}

/// A portrait thumbnail's width in a grid of them (Select Portrait, the
/// Creature Wizard), points; it is 100/64 of that tall.
pub(crate) const PORTRAIT_THUMB: f32 = 54.0;

/// A grid of portrait thumbnails `width` points wide (a scroll bar
/// included): how many to a row, and each row's height.
pub(crate) fn portrait_grid(ui: &Ui, width: f32) -> (usize, f32) {
    let gap = ui.spacing().item_spacing;
    let usable = width - ui.spacing().scroll.bar_width - gap.x;
    let per_row = ((usable + gap.x) / (PORTRAIT_THUMB + gap.x)).floor().max(1.0) as usize;
    (per_row, PORTRAIT_THUMB * 100.0 / 64.0 + gap.y)
}

/// Draws an icon's layers over each other, `scale` times their size (the
/// first layer's); the response of the whole.
pub(crate) fn stacked(
    ui: &mut egui::Ui,
    layers: &[Picture],
    scale: f32,
    alt: &str,
) -> egui::Response {
    let size = layers.first().map_or(egui::vec2(32.0, 32.0), |p| p.size) * scale;
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    for p in layers {
        ui.painter().image(p.texture.id(), rect, uv, egui::Color32::WHITE);
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Image, true, alt));
    response
}

/// [`stacked`], of the icon what is drawn alone, and no taller than
/// `tallest`: a weapon's icon is a texture 256 points tall whatever the
/// weapon's length, and a hak's may be any size.
pub(crate) fn stacked_drawn(
    ui: &mut egui::Ui,
    layers: &[Picture],
    (scale, tallest): (f32, f32),
    alt: &str,
) -> egui::Response {
    let Some(first) = layers.first() else { return stacked(ui, layers, scale, alt) };
    let uv = icon_drawn(layers);
    let size = first.size * uv.size();
    let size = size * scale.min(tallest / size.y.max(1.0));
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    for p in layers {
        ui.painter().image(p.texture.id(), rect, uv, egui::Color32::WHITE);
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Image, true, alt));
    response
}

/// Draws what of a picture is not transparent, at its size where that is
/// no larger than `most` either way, else as large: a part of an icon as
/// large as it is in the icon, without the icon's empty room about it.
pub(crate) fn cropped(ui: &mut egui::Ui, p: &Picture, most: f32, alt: &str) -> egui::Response {
    let size = p.size * p.drawn.size();
    let size = size * (most / size.max_elem().max(1.0)).min(1.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    ui.painter().image(p.texture.id(), rect, p.drawn, egui::Color32::WHITE);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Image, true, alt));
    response
}

/// The part of an image that isn't transparent, in texture coordinates
/// (rows top first); all of it when none is.
fn drawn(rgba: &mg_image::Rgba) -> egui::Rect {
    let (w, h) = (rgba.width as usize, rgba.height as usize);
    let (mut min, mut max) = ([usize::MAX; 2], [0usize; 2]);
    for (i, px) in rgba.data.as_chunks::<4>().0.iter().enumerate() {
        if px[3] > 0 {
            let (x, y) = (i % w.max(1), i / w.max(1));
            min = [min[0].min(x), min[1].min(y)];
            max = [max[0].max(x + 1), max[1].max(y + 1)];
        }
    }
    if min[0] == usize::MAX || w == 0 || h == 0 {
        return egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    }
    let (w, h) = (w as f32, h as f32);
    egui::Rect::from_min_max(
        egui::pos2(min[0] as f32 / w, min[1] as f32 / h),
        egui::pos2(max[0] as f32 / w, max[1] as f32 / h),
    )
}

/// The largest an item icon is shown in lists: an armor's or a long
/// weapon's whole texture (64×128 and 32×128 pixels) at its own size.
pub(crate) const ICON_MAX: egui::Vec2 = egui::vec2(64.0, 128.0);

/// The part of an icon's layers that is drawn, in texture coordinates
/// (the layers share a size).
fn icon_drawn(layers: &[Picture]) -> egui::Rect {
    layers.iter().map(|p| p.drawn).reduce(|a, b| a.union(b)).unwrap_or(egui::Rect::NOTHING)
}

/// The size an icon is shown at within `max`: its drawn part at its own
/// size, or smaller to fit (an empty 32-point square without one).
pub(crate) fn icon_size(layers: &[Picture], max: egui::Vec2) -> egui::Vec2 {
    match layers.first() {
        Some(p) => {
            let size = icon_drawn(layers).size() * p.size;
            let scale = (max.x / size.x.max(1.0)).min(max.y / size.y.max(1.0)).min(1.0);
            size * scale
        }
        None => egui::vec2(32.0, 32.0),
    }
}

/// An icon in a column `max.x` points wide: the drawn part of its layers
/// at [`icon_size`], centered.
pub(crate) fn icon_box(
    ui: &mut egui::Ui,
    layers: &[Picture],
    max: egui::Vec2,
    alt: &str,
) -> egui::Response {
    let size = icon_size(layers, max);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(max.x, size.y), egui::Sense::hover());
    let at = egui::Rect::from_center_size(rect.center(), size);
    let uv = icon_drawn(layers);
    for p in layers {
        ui.painter().image(p.texture.id(), at, uv, egui::Color32::WHITE);
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Image, true, alt));
    response
}

#[cfg(test)]
mod tests {
    use mg_gff::Value;

    use super::*;

    /// A texture whose every pixel says where it is (red its column, green
    /// its row).
    fn where_texture(w: u32, h: u32) -> mg_image::Rgba {
        let data = (0..h).flat_map(|y| (0..w).flat_map(move |x| [x as u8, y as u8, 0, 255]));
        mg_image::Rgba { width: w, height: h, data: data.collect() }
    }

    /// The game's own layout: the left half of the picture is the
    /// texture's first rows from its left edge, the right half its last
    /// rows up to its right edge, 25/32 of its width each.
    #[test]
    fn a_loading_screen_is_put_together_as_the_stock_model_lays_it_out() {
        let n = 64;
        let out = stitched(&where_texture(n, n), &STOCK_SCREEN).unwrap();
        assert_eq!((out.width, out.height), (100, 32));
        let at =
            |x: usize, y: usize| (out.data[(y * 100 + x) * 4], out.data[(y * 100 + x) * 4 + 1]);
        assert_eq!(at(0, 0), (0, 0));
        assert_eq!(at(49, 31), (49, 31), "the left half's last pixel");
        assert_eq!(at(50, 0), (14, 32), "the right half: the upper rows, from 7/32 across");
        assert_eq!(at(99, 31), (63, 63));
        // A texture of nothing, or parts of nothing: none.
        assert!(stitched(&where_texture(1, 1), &STOCK_SCREEN).is_none());
        assert!(stitched(&where_texture(8, 8), &[[0.5, 0.5, 0.5, 1.0]]).is_none());
        // The game's own model, read as a hak's would be, gives the same.
        let Some(root) = mg_testkit::nwn_root() else { return };
        let install = mg_resman::GameInstall::new(&root, None, "en");
        let rm = mg_resman::ResMan::for_game(&install).unwrap();
        let key = ResKey::parse("pnl_loadscreen", ResType::MDL).unwrap();
        let parts = screen_parts(&rm.get(&key).unwrap()).unwrap();
        assert_eq!(parts.len(), 2);
        for (got, want) in parts.iter().zip(STOCK_SCREEN) {
            assert!(got.iter().zip(want).all(|(g, w)| (g - w).abs() < 1e-4), "{got:?}");
        }
    }

    /// A hak's own `pnl_loadscreen` (written here: three panels of a wide
    /// texture, listed out of their order on screen, and a frame drawn
    /// with another texture) is laid out as it maps the texture.
    #[test]
    fn a_loading_screen_follows_a_hak_s_own_model() {
        let quad = |name: &str, x: f32, bitmap: &str, uv: [f32; 4]| {
            let [u0, v0, u1, v1] = uv;
            format!(
                "node trimesh {name}\n  parent screen\n  bitmap {bitmap}\n  verts 4\n    -1 1 0\n    \
                 -1 -1 0\n    1 1 0\n    1 -1 0\n  tverts 4\n    {u0} {v1} 0\n    {u0} {v0} 0\n    \
                 {u1} {v1} 0\n    {u1} {v0} 0\n  faces 2\n    0 1 2 1 0 1 2 1\n    \
                 3 2 1 1 3 2 1 1\n  position {x} 0 0\nendnode\n"
            )
        };
        let text = format!(
            "newmodel screen\nsetsupermodel screen NULL\nbeginmodelgeom screen\nnode dummy screen\n  \
             parent NULL\nendnode\n{}{}{}{}endmodelgeom screen\ndonemodel screen\n",
            quad("right", 2.0, "gui_replace01", [0.5, 0.5, 1.0, 1.0]),
            quad("frame", 0.0, "gui_boxes_ingame", [0.0, 0.0, 0.1, 0.1]),
            quad("left", -2.0, "gui_replace01", [0.0, 0.0, 0.5, 0.5]),
            quad("middle", 0.0, "gui_replace01", [0.5, 0.0, 1.0, 0.5]),
        );
        let parts = screen_parts(text.as_bytes()).unwrap();
        assert_eq!(parts, [[0.0, 0.0, 0.5, 0.5], [0.5, 0.0, 1.0, 0.5], [0.5, 0.5, 1.0, 1.0]]);
        let out = stitched(&where_texture(64, 32), &parts).unwrap();
        assert_eq!((out.width, out.height), (96, 16));
        let at = |x: usize| (out.data[x * 4], out.data[x * 4 + 1]);
        assert_eq!([at(0), at(32), at(64)], [(0, 0), (32, 0), (32, 16)]);
        // A model with no such mesh, or not a model: the stock layout.
        let bare = "newmodel x\nbeginmodelgeom x\nnode dummy x\n  parent NULL\nendnode\n\
                    endmodelgeom x\ndonemodel x\n";
        assert!(screen_parts(bare.as_bytes()).is_none());
        assert!(screen_parts(b"\0\0garbage").is_none());
    }

    /// A picture both the module being edited and one of its haks have is
    /// the hak's, as the game reads it (`mg which`: the hak, the module
    /// overridden); one only the module has is its unsaved copy.
    #[test]
    fn a_picture_a_hak_and_the_module_both_have_is_the_hak_s() {
        let tga = |level: u8| {
            let mut t = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 32, 8];
            t.extend([level, level, level, 255]);
            t
        };
        let key = |n: &str| ResKey::parse(n, ResType::TGA).unwrap();
        let mut hak = mg_resman::MemContainer::new();
        hak.insert(key("po_bothm"), tga(10));
        let mut saved = mg_resman::MemContainer::new();
        saved.insert(key("po_bothm"), tga(20));
        let mut rm = mg_resman::ResMan::new();
        rm.add(
            mg_resman::priority::MODULE,
            mg_resman::MODULE_LABEL,
            mg_resman::LayerClass::Erf,
            saved,
        );
        rm.add(mg_resman::priority::HAK, "hak:pics", mg_resman::LayerClass::Erf, hak);
        let game = GameData::new(rm, mg_tlk::Tlk::new(mg_core::Language::ENGLISH));
        let mut m = mg_module::Module::new();
        m.set(key("po_bothm"), tga(30));
        m.set(key("po_modm"), tga(40));
        let ws = Workspace::new(m);
        let (mut pictures, mut palettes) = (Pictures::new(), Palettes::new());
        let loader =
            Loader { pictures: &mut pictures, palettes: &mut palettes, ws: Some(&ws), game: &game };
        let level = |name: &str| {
            let (t, d) = loader.texture(ResRef::from_str(name).unwrap()).unwrap();
            mg_image::read(t, &d).unwrap().to_rgba().data[0]
        };
        assert_eq!(level("po_bothm"), 10);
        assert_eq!(level("po_modm"), 40);
    }

    #[test]
    fn a_thumbnail_is_a_small_mip_level_or_scaled_down() {
        // Three levels, 8, 4 and 2 across, each its own grey.
        let level = |side: usize, v: u8| vec![v; side * side * 4];
        let t = mg_image::Texture {
            width: 8,
            height: 8,
            format: mg_image::Format::Rgba8,
            mips: vec![level(8, 10), level(4, 20), level(2, 30)],
            has_alpha: false,
            alpha_mean: None,
        };
        let s = small(&t, 4);
        assert_eq!((s.width, s.height, s.data[0], s.data[3]), (4, 4, 20, 255));
        assert_eq!(small(&t, 1).width, 1, "the smallest, scaled down");
        // One level: every other pixel.
        let mut one = t.clone();
        one.mips.truncate(1);
        let s = small(&one, 3);
        assert_eq!((s.width, s.height), (3, 3));
        assert_eq!(s.data.len(), 3 * 3 * 4);
    }

    #[test]
    fn a_part_past_255_is_read_from_its_ee_twin_whose_label_is_cut() {
        // `ArmorPart_Pelvis` has 16 characters: its twin is `xArmorPart_Pelvi`.
        let mut item = Struct::new(0);
        item.set("ArmorPart_Pelvis", Value::Byte(255));
        item.set("xArmorPart_Pelvi", Value::Word(300));
        item.set("ArmorPart_Torso", Value::Byte(255));
        item.set("xArmorPart_Torso", Value::Word(301));
        item.set("ArmorPart_Belt", Value::Byte(7));
        assert_eq!(part(&item, "ArmorPart_Pelvis"), 300);
        assert_eq!(part(&item, "ArmorPart_Torso"), 301);
        assert_eq!(part(&item, "ArmorPart_Belt"), 7);
        assert_eq!(part(&item, "ArmorPart_Robe"), 0);
    }
}
