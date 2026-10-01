//! Game images in the UI (portraits, icons): a texture, the module's or the
//! game's (DDS over TGA, as the engine prefers), decoded once and kept as
//! an egui texture.

use std::sync::Arc;

use mg_core::{ResRef, ResType};
use mg_resman::ResKey;

use crate::Moonglow;

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

/// A texture's bytes and type: the module's, else the game's.
fn texture_data(app: &Moonglow, name: ResRef) -> Option<(ResType, Arc<[u8]>)> {
    let module = app.ws.as_ref().and_then(|ws| {
        [ResType::DDS, ResType::TGA]
            .into_iter()
            .find_map(|t| ws.module.get(&ResKey::new(name, t)).map(|d| (t, Arc::from(d))))
    });
    module.or_else(|| {
        let (t, data) = app.game.as_ref()?.resman.texture(name)?;
        Some((t, Arc::from(&*data)))
    })
}

impl Moonglow {
    /// The image named `name` (a texture resref), loaded once; `None` if
    /// there is none or it does not decode.
    pub fn picture(&mut self, ctx: &egui::Context, name: &str) -> Option<Picture> {
        let key = name.to_ascii_lowercase();
        if let Some(p) = self.pictures.get(&key) {
            return p.clone();
        }
        let picture = ResRef::from_str(&key).ok().and_then(|r| {
            let (t, data) = texture_data(self, r)?;
            let rgba = mg_image::read(t, &data).ok()?.to_rgba().top_down();
            let size = [rgba.width as usize, rgba.height as usize];
            let image = egui::ColorImage::from_rgba_unmultiplied(size, &rgba.data);
            let texture = ctx.load_texture(&key, image, egui::TextureOptions::LINEAR);
            Some(Picture { texture, size: egui::vec2(size[0] as f32, size[1] as f32) })
        });
        self.pictures.insert(key, picture.clone());
        picture
    }
}
