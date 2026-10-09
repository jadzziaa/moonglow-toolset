//! NUI resources are read from the current content stack. No game artwork is bundled.
use std::collections::{BTreeMap, BTreeSet};
use std::hash::{Hash, Hasher};

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

#[derive(Default)]
pub(crate) struct Assets {
    dirty: bool,
    signature: Option<u64>,
    catalog_signature: Option<u64>,
    pub(super) catalog: Vec<String>,
    requested: BTreeSet<String>,
    style: toml::Table,
    pictures: BTreeMap<String, Picture>,
    fonts: BTreeMap<String, Font>,
    default_font: String,
    strings: BTreeMap<i64, String>,
    pub(super) origins: BTreeMap<String, String>,
    pub(super) issues: Vec<String>,
    pub(super) loaded: bool,
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

fn image_resource(
    rm: Option<&ResMan>,
    module: &Module,
    name: &str,
) -> Option<(ResType, Vec<u8>, String)> {
    // NUI's image format preference is not the 3D renderer's DDS-first rule.
    // DDS was added as a fallback in EE 8193.36.
    [ResType::JPG, ResType::TGA, ResType::PNG, ResType::BMP, ResType::DDS]
        .into_iter()
        .find_map(|ty| Some((ty, resource(rm, module, ResKey::parse(name, ty)?)?)))
        .map(|(ty, (bytes, origin))| (ty, bytes, origin))
}

fn image(bytes: &[u8], ty: ResType) -> Result<egui::ColorImage, String> {
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("image exceeds 64 MiB".into());
    }
    if ty == ResType::DDS {
        let t = mg_image::read(ty, bytes).map_err(|e| e.to_string())?;
        if u64::from(t.width) * u64::from(t.height) > 16_777_216 {
            return Err("image exceeds 16 megapixels".into());
        }
        let rgba = t.to_rgba().top_down();
        return Ok(egui::ColorImage::from_rgba_unmultiplied(
            [rgba.width as usize, rgba.height as usize],
            &rgba.data,
        ));
    }
    let format = match ty {
        ResType::TGA => image::ImageFormat::Tga,
        ResType::PNG => image::ImageFormat::Png,
        ResType::JPG => image::ImageFormat::Jpeg,
        ResType::BMP => image::ImageFormat::Bmp,
        _ => return Err(format!("Unsupported NUI image: {ty}")),
    };
    let mut reader = image::ImageReader::with_format(std::io::Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let rgba = reader.decode().map_err(|e| e.to_string())?.into_rgba8();
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        rgba.as_raw(),
    ))
}

fn names(v: &Value, s: &Settings, out: &mut BTreeSet<String>) {
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
        wanted.extend(self.requested.iter().cloned());
        let layers: Vec<_> = rm
            .into_iter()
            .flat_map(|r| r.layers())
            .map(|l| (&l.label, l.priority, l.fingerprint))
            .collect();
        let signature = hash((revision, &layers, &wanted));
        let catalog_signature = hash((revision, &layers));
        if self.dirty || self.catalog_signature != Some(catalog_signature) {
            self.catalog_signature = Some(catalog_signature);
            let types = [ResType::JPG, ResType::TGA, ResType::PNG, ResType::BMP, ResType::DDS];
            let mut catalog: BTreeSet<String> = module
                .keys()
                .filter(|k| types.contains(&k.restype))
                .map(|k| k.resref.to_string())
                .collect();
            if let Some(rm) = rm {
                for ty in types {
                    catalog.extend(rm.list(ty).into_iter().map(|r| r.to_string()));
                }
            }
            self.catalog = catalog.into_iter().collect();
        }
        if !self.dirty && self.signature == Some(signature) {
            return;
        }
        self.dirty = false;
        self.signature = Some(signature);
        self.issues.clear();
        self.origins.clear();
        self.fonts.clear();
        self.style.clear();
        self.loaded = false;
        let key = ResKey::parse("nui_skin", ResType::TML).unwrap();
        if let Some((b, origin)) = resource(rm, module, key) {
            match std::str::from_utf8(&b)
                .map_err(|e| e.to_string())
                .and_then(|t| t.parse::<toml::Table>().map_err(|e| e.to_string()))
            {
                Ok(style) => {
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
        for name in wanted {
            let Some((ty, bytes, origin)) = image_resource(rm, module, &name) else {
                self.pictures.remove(&name);
                self.issues.push(format!("Missing image: {name}"));
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
            let data =
                ResKey::parse(resref, ResType::TTF).and_then(|key| resource(rm, module, key));
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
        ctx.request_repaint();
    }

    fn value(&self, path: &str) -> Option<&toml::Value> {
        let mut parts = path.split('.');
        let mut v = self.style.get(parts.next()?)?;
        for p in parts {
            v = v.get(p)?;
        }
        Some(v)
    }
    pub(super) fn number(&self, path: &str, default: f32) -> f32 {
        self.value(path).and_then(toml_number).unwrap_or(default)
    }
    pub(super) fn color(&self, path: &str, default: Color32) -> Color32 {
        self.value(path).and_then(toml::Value::as_str).and_then(color).unwrap_or(default)
    }
    pub(super) fn picture(&self, name: &str) -> Option<&Picture> {
        self.pictures.get(&name.to_lowercase())
    }
    pub(super) fn style_picture(&self, path: &str) -> Option<&Picture> {
        self.picture(self.value(path)?.as_str()?)
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
        let Some(value) = self.value(path).and_then(toml::Value::as_str) else {
            return false;
        };
        if let Some(c) = color(value) {
            p.rect_filled(rect, 0, c * tint);
            return true;
        }
        let Some(image) = self.picture(value) else {
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
    fn nui_strref_preview_resolves_module_tlk_and_indexed_binds_without_mutating_source() {
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
    fn nui_resources_observe_hak_live_module_override_order_and_dds_fallback() {
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
    fn nui_native_draw_image_ignores_legacy_tint_and_alpha() {
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
    fn nui_native_button_image_stretches_while_plain_image_keeps_aspect() {
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
                    &json!({"type":ty}),
                    &s,
                    None,
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
    fn nui_native_crop_changes_uv_without_changing_image_aspect() {
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
                    &json!({"type":ty,"image_region":{"x":0.0,"y":1.0,"w":1.0,"h":1.0}}),
                    &s,
                    None,
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
    fn nui_tga_origin_is_respected_and_slice_edges_do_not_cross() {
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

    #[test]
    fn nui_asset_cache_reuses_textures_and_reloads_changed_bytes() {
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
        let mut changed = tga(false);
        changed[18] = 127;
        module.set(tex, changed);
        assets.prepare(&ctx, None, &module, 2, &v, &s);
        assert_ne!(first, assets.picture("test_image").unwrap().texture.id());
        module.set(skin, b"broken = [".to_vec());
        assets.prepare(&ctx, None, &module, 3, &v, &s);
        assert!(!assets.loaded);
        assert!(!assets.issues.is_empty());
        assert!(assets.picture("test_image").is_none());
    }
}
