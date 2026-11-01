//! The textures kept: those no frame has drawn for a while are let go once
//! there is more of them than [`Kept::stale_bytes`] (the longest unused
//! first), so that the areas, models and galleries' pictures seen in a
//! session don't each leave their textures on the GPU for good. What is
//! drawn is never let go, however much it is.

use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use super::Renderer;

/// How long a texture goes undrawn before it is stale: let go of when the
/// stale ones are too many.
pub(crate) const STALE_AFTER: Duration = Duration::from_secs(30);

/// The most a frame counts for on the textures' clock, however long it
/// was since the frame before.
const FRAME_GAP: Duration = Duration::from_secs(1);

/// How much of the stale textures is kept by default, bytes: a discrete
/// GPU's; an integrated one keeps less ([`STALE_TEXTURES_KEPT_INTEGRATED`]).
pub const STALE_TEXTURES_KEPT: u64 = 1 << 30;

/// The stale textures an integrated GPU keeps, bytes: its memory is the
/// system's, and small (a few hundred MB set aside, to 2 GB).
pub const STALE_TEXTURES_KEPT_INTEGRATED: u64 = 256 << 20;

/// When each material and texture was last drawn, and how much of what
/// isn't drawn now is kept.
#[derive(Debug)]
pub(crate) struct Kept {
    /// The time stamps are seconds of this clock: the time frames were
    /// drawn in, each frame counting for the time since the one before
    /// up to [`FRAME_GAP`] (a window left alone for an hour, drawing
    /// nothing, makes nothing stale: its next frame draws what it did).
    clock: Duration,
    /// When the last frame was drawn.
    last: Option<Instant>,
    /// This frame's stamp.
    pub(crate) now: u32,
    /// When each material (by its place among the renderer's groups) was
    /// last drawn.
    pub(crate) materials: Vec<u32>,
    /// When the textures were last looked over.
    checked: Option<u32>,
    /// How much of the stale textures is kept, bytes.
    pub(crate) stale_bytes: u64,
    /// How many textures have been let go so far.
    pub(crate) let_go: usize,
}

impl Default for Kept {
    fn default() -> Kept {
        Kept {
            clock: Duration::ZERO,
            last: None,
            now: 0,
            materials: Vec::new(),
            checked: None,
            stale_bytes: STALE_TEXTURES_KEPT,
            let_go: 0,
        }
    }
}

impl Kept {
    /// A frame begins at `at`: the clock moves on by the time since the
    /// last, up to [`FRAME_GAP`].
    fn frame_at(&mut self, at: Instant) {
        let gap = self.last.map_or(Duration::ZERO, |l| at.saturating_duration_since(l));
        self.last = Some(at);
        self.clock += gap.min(FRAME_GAP);
        self.now = self.clock.as_secs() as u32;
    }

    /// Notes that the frame draws with these materials.
    pub(crate) fn drawn(&mut self, materials: impl IntoIterator<Item = u32>) {
        let now = self.now;
        for m in materials {
            let m = m as usize;
            if m >= self.materials.len() {
                self.materials.resize(m + 1, 0);
            }
            self.materials[m] = now;
        }
    }
}

/// What the renderer's textures take on the GPU
/// ([`Renderer::texture_memory`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TextureMemory {
    /// The textures held, and their bytes in all.
    pub textures: usize,
    pub bytes: u64,
    /// Of those, the bytes of the ones held uncompressed (RGBA8: TGA,
    /// PLT, a DDS the GPU can't take as it is).
    pub uncompressed: u64,
    /// Of those, the bytes of the ones not drawn for [`STALE_AFTER`].
    pub stale: u64,
    /// How many have been let go so far, stale.
    pub let_go: usize,
}

impl Renderer {
    /// How much of the textures not drawn lately is kept on the GPU, bytes
    /// ([`STALE_TEXTURES_KEPT`] by default): past it, the longest unused
    /// are let go (read again when drawn again).
    pub fn set_stale_textures_kept(&mut self, bytes: u64) {
        self.kept.stale_bytes = bytes;
        self.kept.checked = None;
    }

    /// Moves the textures' clock on by `by`, as if frames had been drawn
    /// for that long since the last (for tests: what goes stale after a
    /// while).
    #[doc(hidden)]
    pub fn age_textures(&mut self, by: Duration) {
        self.kept.clock += by;
    }

    /// What the textures take on the GPU, and how much of it is stale.
    pub fn texture_memory(&self) -> TextureMemory {
        let last = self.last_drawn();
        let mut m = TextureMemory { let_go: self.kept.let_go, ..Default::default() };
        for (name, entry) in &self.textures {
            let Some(e) = entry else { continue };
            m.textures += 1;
            m.bytes += e.bytes;
            if e.gpu.texture.format() == wgpu::TextureFormat::Rgba8Unorm {
                m.uncompressed += e.bytes;
            }
            if self.stale(last.get(name.as_str()).copied(), e) {
                m.stale += e.bytes;
            }
        }
        m
    }

    /// Stamps this frame's time: what is drawn in it is drawn now.
    pub(crate) fn begin_kept(&mut self) {
        self.kept.frame_at(Instant::now());
    }

    /// When each texture a material binds was last drawn (by its
    /// materials: a texture's own stamp is in its entry).
    fn last_drawn(&self) -> HashMap<&str, u32> {
        let mut last: HashMap<&str, u32> = HashMap::new();
        for (i, key) in self.material_keys.iter().enumerate() {
            let at = self.kept.materials.get(i).copied().unwrap_or(0);
            for name in key.0.iter().flatten().chain([&key.1]) {
                let e = last.entry(name.as_str()).or_default();
                *e = (*e).max(at);
            }
        }
        last
    }

    fn stale(&self, by_material: Option<u32>, e: &super::TextureEntry) -> bool {
        let at = by_material.unwrap_or(0).max(e.used.load(Ordering::Relaxed));
        self.kept.now.saturating_sub(at) >= STALE_AFTER.as_secs() as u32
    }

    /// Once a second at most: where the stale textures take more than is
    /// kept of them, lets go of the longest unused until half that is
    /// left, and of what was made from them (the materials' bind groups
    /// and each model's surfaces, made again as they are drawn). Says how
    /// many were let go.
    pub(crate) fn let_go_of_stale_textures(&mut self) -> usize {
        if self.kept.checked == Some(self.kept.now) {
            return 0;
        }
        self.kept.checked = Some(self.kept.now);
        // (Not even all of them past what is kept: none can be stale past it.)
        let all: u64 = self.textures.values().flatten().map(|e| e.bytes).sum();
        if all <= self.kept.stale_bytes {
            return 0;
        }
        let last = self.last_drawn();
        let mut stale: Vec<(u32, &str, u64)> = (self.textures.iter())
            .filter_map(|(name, entry)| {
                let e = entry.as_ref()?;
                let by_material = last.get(name.as_str()).copied();
                self.stale(by_material, e).then(|| {
                    let at = by_material.unwrap_or(0).max(e.used.load(Ordering::Relaxed));
                    (at, name.as_str(), e.bytes)
                })
            })
            .collect();
        let mut bytes: u64 = stale.iter().map(|s| s.2).sum();
        if bytes <= self.kept.stale_bytes {
            return 0;
        }
        stale.sort_unstable();
        let mut gone = Vec::new();
        for (_, name, b) in stale {
            if bytes <= self.kept.stale_bytes / 2 {
                break;
            }
            bytes -= b;
            gone.push(name.to_string());
        }
        drop(last);
        for name in &gone {
            self.textures.remove(name);
        }
        // What binds them goes with them (else the GPU holds them still).
        self.materials.clear();
        self.groups.clear();
        self.material_keys.clear();
        self.kept.materials.clear();
        self.surfaces.clear();
        self.game.forget_materials();
        self.kept.let_go += gone.len();
        gone.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The clock moves on with the frames: a window left alone an hour
    /// counts a second of it, and a frame's materials are stamped with
    /// its time.
    #[test]
    fn the_clock_is_the_frames_time() {
        let mut k = Kept::default();
        let t0 = Instant::now();
        k.frame_at(t0);
        k.frame_at(t0 + Duration::from_millis(500));
        k.frame_at(t0 + Duration::from_millis(2500));
        assert_eq!(k.clock, Duration::from_millis(1500), "a gap counts a second at most");
        k.frame_at(t0 + Duration::from_secs(3600));
        assert_eq!(k.now, 2, "an hour alone: a second");
        k.drawn([3, 1]);
        assert_eq!(k.materials, [0, 2, 0, 2]);
    }
}
