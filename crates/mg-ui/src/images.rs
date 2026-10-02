//! Game images in the UI (portraits, icons): a texture, the module's or the
//! game's (DDS over TGA, as the engine prefers), decoded once and kept as
//! an egui texture.

use std::collections::HashMap;
use std::sync::Arc;

use mg_core::{ResRef, ResType};
use mg_edit::Workspace;
use mg_gff::Struct;
use mg_resman::ResKey;
use mg_rules::GameData;

use crate::Moonglow;
use egui::Ui;

/// A decoded image: its texture and size in pixels.
#[derive(Clone)]
pub struct Picture {
    pub texture: egui::TextureHandle,
    pub size: egui::Vec2,
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

/// Armor icon parts, bottom first: (field, icon part).
const ARMOR_ICON: [(&str, &str); 6] = [
    ("ArmorPart_Pelvis", "pelvis"),
    ("ArmorPart_Torso", "chest"),
    ("ArmorPart_Belt", "belt"),
    ("ArmorPart_LShoul", "shol"),
    ("ArmorPart_RShoul", "shor"),
    ("ArmorPart_Robe", "robe"),
];

/// The PLT layers' colours an item gives (metal, cloth and leather; skin,
/// hair and tattoos the first colour).
pub(crate) fn item_colors(item: &Struct) -> [u8; 10] {
    let c = |l: &str| item.integer(l).unwrap_or(0).clamp(0, 255) as u8;
    [
        0,
        0,
        c("Metal1Color"),
        c("Metal2Color"),
        c("Cloth1Color"),
        c("Cloth2Color"),
        c("Leather1Color"),
        c("Leather2Color"),
        0,
        0,
    ]
}

/// A model or part number: the EE twin (`xModelPart1`) where there is one.
fn part(item: &Struct, label: &str) -> i64 {
    item.integer(&format!("x{label}")).or_else(|| item.integer(label)).unwrap_or(0)
}

/// The names of an item's inventory icon layers, bottom first (nwn.wiki,
/// baseitems.2da): `i<ItemClass>_<model>` for simple and layered items
/// (`_m_` between for gender-specific ones), a composite weapon's bottom,
/// middle and top (`_b_`, `_m_`, `_t_`), an armor's pelvis, chest, belt,
/// shoulders and robe (`ipm_<part><number>`).
pub(crate) fn item_icon_names(game: &GameData, item: &Struct) -> Vec<String> {
    let Ok(t) = game.table("baseitems") else { return Vec::new() };
    let Ok(row) = usize::try_from(item.integer("BaseItem").unwrap_or(-1)) else {
        return Vec::new();
    };
    let class = t.get(row, "ItemClass").unwrap_or_default().to_ascii_lowercase();
    match t.get_int(row, "ModelType").unwrap_or(0) {
        2 => [("ModelPart1", "b"), ("ModelPart2", "m"), ("ModelPart3", "t")]
            .iter()
            .map(|(l, p)| format!("i{class}_{p}_{:03}", part(item, l)))
            .collect(),
        3 => ARMOR_ICON
            .iter()
            .map(|(l, p)| (part(item, l), p))
            .filter(|(n, _)| *n > 0)
            .map(|(n, p)| format!("ipm_{p}{n:03}"))
            .collect(),
        _ => {
            let gender = if t.get_int(row, "GenderSpecific") == Some(1) { "m_" } else { "" };
            vec![format!("i{class}_{gender}{:03}", part(item, "ModelPart1"))]
        }
    }
}

impl Loader<'_> {
    /// A resource's bytes: the module's, else the game's.
    fn data(&self, key: ResKey) -> Option<Arc<[u8]>> {
        let module = self.ws.and_then(|ws| ws.module.get(&key)).map(Arc::from);
        module.or_else(|| self.game.resman.get(&key).ok().map(|d| Arc::from(&*d)))
    }

    /// A texture's type and bytes: the module's (DDS, then TGA), else the
    /// game's as the engine prefers.
    fn texture(&self, name: ResRef) -> Option<(ResType, Arc<[u8]>)> {
        let module = self.ws.and_then(|ws| {
            [ResType::DDS, ResType::TGA]
                .into_iter()
                .find_map(|t| ws.module.get(&ResKey::new(name, t)).map(|d| (t, Arc::from(d))))
        });
        module.or_else(|| {
            let (t, data) = self.game.resman.texture(name)?;
            Some((t, Arc::from(&*data)))
        })
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
            Picture { texture, size: egui::vec2(size[0] as f32, size[1] as f32) }
        });
        self.pictures.insert(key, picture.clone());
        picture
    }

    /// A palette image (`pal_cloth01`, ...) as pixels, loaded once.
    pub(crate) fn palette(&mut self, name: &str) -> Option<Arc<mg_image::Rgba>> {
        if let Some(p) = self.palettes.get(name) {
            return p.clone();
        }
        let rgba = ResRef::from_str(name).ok().and_then(|r| {
            let (t, data) = self.texture(r)?;
            Some(Arc::new(mg_image::read(t, &data).ok()?.to_rgba()))
        });
        self.palettes.insert(name.to_string(), rgba.clone());
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
            let palettes: Vec<Option<mg_image::Rgba>> = mg_image::plt::PALETTES
                .iter()
                .map(|p| {
                    let (t, data) = self.texture(ResRef::from_str(p).ok()?)?;
                    mg_image::read(t, &data).ok().map(|t| t.to_rgba())
                })
                .collect();
            let refs: [Option<&mg_image::Rgba>; 10] = std::array::from_fn(|i| palettes[i].as_ref());
            Some(plt.colorize(&refs, colors))
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
    /// the item's colours), else the base item's `DefaultIcon`.
    pub(crate) fn item_icon(&mut self, ctx: &egui::Context, item: &Struct) -> Vec<Picture> {
        let names = item_icon_names(self.game, item);
        let colors = item_colors(item);
        let mut layers: Vec<Picture> = names
            .iter()
            .filter_map(|n| self.plt_picture(ctx, n, colors).or_else(|| self.picture(ctx, n)))
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
}

impl Moonglow {
    /// The image loader, while there is game data.
    pub(crate) fn loader(&mut self) -> Option<Loader<'_>> {
        let game = self.game.as_ref()?;
        Some(Loader {
            pictures: &mut self.pictures,
            palettes: &mut self.palettes,
            ws: self.ws.as_ref(),
            game,
        })
    }

    /// The image named `name` (a texture resref), loaded once; `None` if
    /// there is none or it does not decode.
    pub fn picture(&mut self, ctx: &egui::Context, name: &str) -> Option<Picture> {
        self.loader()?.picture(ctx, name)
    }

    /// An item's inventory icon, its layers bottom first.
    pub fn item_icon(&mut self, ctx: &egui::Context, item: &Struct) -> Vec<Picture> {
        self.loader().map(|mut l| l.item_icon(ctx, item)).unwrap_or_default()
    }
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

/// A palette colour's tones (its row, `index` counted from the top, at four
/// grey levels), as the colour chooser shows them.
pub(crate) fn tones(palette: &mg_image::Rgba, index: u8) -> [egui::Color32; 4] {
    let rows = palette.height.max(1);
    let y = rows - 1 - u32::from(index).min(rows - 1);
    [64u32, 128, 192, 250].map(|x| {
        let [r, g, b, _] = palette.pixel(x.min(palette.width.saturating_sub(1)), y);
        egui::Color32::from_rgb(r, g, b)
    })
}

/// Paints a swatch: the tones side by side.
pub(crate) fn swatch(ui: &Ui, rect: egui::Rect, tones: &[egui::Color32; 4]) {
    let w = rect.width() / tones.len() as f32;
    for (i, c) in tones.iter().enumerate() {
        let r = egui::Rect::from_min_size(
            rect.min + egui::vec2(w * i as f32, 0.0),
            egui::vec2(w, rect.height()),
        );
        ui.painter().rect_filled(r, 0.0, *c);
    }
}

/// An icon in a `side`-point square: its layers at their own size, or
/// smaller to fit, centered; an empty square without one.
pub(crate) fn icon_box(
    ui: &mut egui::Ui,
    layers: &[Picture],
    side: f32,
    alt: &str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(side, side), egui::Sense::hover());
    if let Some(p) = layers.first() {
        let scale = (side / p.size.x.max(1.0)).min(side / p.size.y.max(1.0)).min(1.0);
        let at = egui::Rect::from_center_size(rect.center(), p.size * scale);
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        for p in layers {
            ui.painter().image(p.texture.id(), at, uv, egui::Color32::WHITE);
        }
    }
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Image, true, alt));
    response
}
