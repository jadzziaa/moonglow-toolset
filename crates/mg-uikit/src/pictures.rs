//! What a gallery keeps of its pictures, whatever draws them: the
//! pictures made, by what they are of; those asked for in this frame and
//! not made (or made and out of date), in the order asked; the frame each
//! was last asked for in; and those out of sight let go of when there are
//! many. A view asks for the pictures it shows, every frame it shows them
//! ([`Pictures::show`]); after the frame the application makes those
//! wanted, within a [`Budget`], and lets go of the unseen
//! ([`Pictures::let_go_unseen`]). No threads and no GPU: the application
//! reads, draws and frees, and what it keeps (`V`) is its own; or a
//! [`PicturePainter`] paints them on the caller's thread into egui
//! textures ([`PaintedPictures`]).
//!
//! ```
//! use mg_uikit::pictures::{Budget, Pictures};
//! let mut pictures: Pictures<&str, u32> = Pictures::default();
//! pictures.begin_frame();
//! pictures.show("chest", |_| false);
//! let (budget, started, mut made) = (Budget::default(), std::time::Instant::now(), 0);
//! for key in pictures.take_wanted() {
//!     if !budget.more(made, started) {
//!         break;
//!     }
//!     pictures.insert(key, 7); // (Drawn: a texture's id.)
//!     made += 1;
//! }
//! assert_eq!(pictures.get(&"chest"), Some(&7));
//! ```

use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};

/// How many pictures out of sight are kept, by default
/// ([`Pictures::let_go_unseen`]).
pub const PICTURES_KEPT: usize = 512;

/// The pictures made and wanted ([the module](self)).
#[derive(Debug)]
pub struct Pictures<K, V> {
    made: HashMap<K, (V, u64)>,
    wanted: Vec<K>,
    frame: u64,
    shown: usize,
}

impl<K, V> Default for Pictures<K, V> {
    fn default() -> Self {
        Pictures { made: HashMap::new(), wanted: Vec::new(), frame: 0, shown: 0 }
    }
}

impl<K: Copy + Eq + Hash, V> Pictures<K, V> {
    /// A new frame: nothing asked for in it yet.
    pub fn begin_frame(&mut self) {
        self.frame += 1;
        self.wanted.clear();
        self.shown = 0;
    }

    /// The frames, counted.
    pub fn frame(&self) -> u64 {
        self.frame
    }

    /// `key`'s picture shown this frame: kept while it is shown, and
    /// wanted if it is not made, or is and `stale` judges it out of date.
    pub fn show(&mut self, key: K, stale: impl FnOnce(&V) -> bool) {
        self.shown += 1;
        match self.made.get_mut(&key) {
            Some((value, used)) => {
                *used = self.frame;
                if stale(value) {
                    self.wanted.push(key);
                }
            }
            None => self.wanted.push(key),
        }
    }

    /// How many pictures were asked for this frame, made or not, each
    /// time asked.
    pub fn shown_now(&self) -> usize {
        self.shown
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.made.get(key).map(|(v, _)| v)
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.made.get_mut(key).map(|(v, _)| v)
    }

    /// `key`'s picture made, as used in this frame; the one it takes the
    /// place of, to let go of.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.made.insert(key, (value, self.frame)).map(|(v, _)| v)
    }

    /// Lets go of one picture (its subject edited).
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.made.remove(key).map(|(v, _)| v)
    }

    /// The pictures made.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.made.iter().map(|(k, (v, _))| (k, v))
    }

    /// How many pictures are made.
    pub fn len(&self) -> usize {
        self.made.len()
    }

    pub fn is_empty(&self) -> bool {
        self.made.is_empty()
    }

    /// Those wanted, in the order asked (one asked twice is there twice).
    pub fn wanted(&self) -> &[K] {
        &self.wanted
    }

    /// Those wanted, to make after the frame; those left for the next go
    /// back with [`set_wanted`](Self::set_wanted).
    pub fn take_wanted(&mut self) -> Vec<K> {
        std::mem::take(&mut self.wanted)
    }

    pub fn set_wanted(&mut self, wanted: Vec<K>) {
        self.wanted = wanted;
    }

    /// Past `keep` made, those not asked for in this frame or the last,
    /// the longest unseen first, let go of: what they kept, to free.
    pub fn let_go_unseen(&mut self, keep: usize) -> Vec<V> {
        if self.made.len() <= keep {
            return Vec::new();
        }
        let frame = self.frame;
        let mut unseen: Vec<(u64, K)> = (self.made.iter())
            .filter(|(_, (_, used))| used + 1 < frame)
            .map(|(k, (_, used))| (*used, *k))
            .collect();
        unseen.sort_unstable_by_key(|(used, _)| *used);
        let over = self.made.len() - keep;
        unseen.iter().take(over).filter_map(|(_, k)| self.remove(k)).collect()
    }

    /// Lets go of every picture (the data read anew: the same names may
    /// look different now): what they kept, to free.
    pub fn forget(&mut self) -> Vec<V> {
        self.wanted.clear();
        self.made.drain().map(|(_, (v, _))| v).collect()
    }
}

/// How many pictures are made after a frame: at least one, then more
/// while `time` has not passed since the first began and fewer than
/// `most` are made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    pub time: Duration,
    pub most: usize,
}

impl Default for Budget {
    /// 2 ms, as many as fit: the toolset's galleries.
    fn default() -> Self {
        Budget { time: Duration::from_millis(2), most: usize::MAX }
    }
}

impl Budget {
    /// Whether another picture is made, `made` made since `started`.
    pub fn more(&self, made: usize, started: Instant) -> bool {
        made == 0 || made < self.most && started.elapsed() < self.time
    }
}

/// What a painter made of a picture asked for ([`PicturePainter::paint`]).
#[derive(Clone, Debug)]
pub enum Painted {
    /// Pixels, made an egui texture the pictures own.
    Ready(egui::ColorImage),
    /// A texture the painter drew and keeps (a native one registered with
    /// the renderer): handed back when let go of, to free
    /// ([`PaintedPictures::pump`], `forget`, `forget_one`).
    Drawn(egui::TextureId),
    /// Not yet (its shaders still being made): asked again after
    /// [`Retry::again`], in a frame it is still shown in.
    Later,
    /// None to be had, and why: never asked again.
    Failed(String),
}

/// A picture as a gallery's cell shows it ([`PaintedPictures::get`]).
#[derive(Clone, Copy)]
pub enum Picture<'a> {
    Ready(egui::TextureId),
    /// Not made yet (or still being made): the kind's icon.
    Waiting,
    /// "No picture", the reason its tip.
    Failed(&'a str),
}

/// What paints pictures on the caller's thread, once a frame
/// ([`PaintedPictures::pump`]): a painter that owns the window's GPU
/// device, which cannot go to other threads. `as_drawn`: waited for
/// past [`Retry::give_up`], take it as it is drawn. A closure is one.
pub trait PicturePainter<K> {
    fn paint(&mut self, key: K, as_drawn: bool) -> Painted;
}

impl<K, F: FnMut(K, bool) -> Painted + ?Sized> PicturePainter<K> for F {
    fn paint(&mut self, key: K, as_drawn: bool) -> Painted {
        self(key, as_drawn)
    }
}

/// When a picture painted [`Painted::Later`] is asked for again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Retry {
    /// After this long (50 ms).
    pub again: Duration,
    /// Waited for this long (20 s), it is asked for as drawn, and failed
    /// if it is still not.
    pub give_up: Duration,
}

impl Default for Retry {
    fn default() -> Self {
        Retry { again: Duration::from_millis(50), give_up: Duration::from_secs(20) }
    }
}

/// A picture made by a [`PicturePainter`]: its texture, or why there is
/// none.
pub struct PaintedPicture(Result<Texture, String>);

/// A picture's texture: the pictures' own, or the painter's.
enum Texture {
    Own(egui::TextureHandle),
    Painters(egui::TextureId),
}

impl Texture {
    fn id(&self) -> egui::TextureId {
        match self {
            Texture::Own(handle) => handle.id(),
            Texture::Painters(id) => *id,
        }
    }
}

impl PaintedPicture {
    /// The painter's texture, to free, if it is one.
    fn painters(self) -> Option<egui::TextureId> {
        match self.0 {
            Ok(Texture::Painters(id)) => Some(id),
            _ => None,
        }
    }
}

impl std::fmt::Debug for PaintedPicture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Ok(texture) => write!(f, "Ready({:?})", texture.id()),
            Err(reason) => write!(f, "Failed({reason:?})"),
        }
    }
}

impl std::fmt::Debug for Picture<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Picture::Ready(id) => write!(f, "Ready({id:?})"),
            Picture::Waiting => write!(f, "Waiting"),
            Picture::Failed(reason) => write!(f, "Failed({reason:?})"),
        }
    }
}

/// [`Pictures`] whose pictures a [`PicturePainter`] paints on the
/// caller's thread into egui textures: the asking and letting go as
/// [`Pictures`]'s, and those painted [`Painted::Later`] waited for while
/// they are shown.
#[derive(Debug)]
pub struct PaintedPictures<K> {
    pub pictures: Pictures<K, PaintedPicture>,
    pub retry: Retry,
    /// Those painted Later: when first and last asked.
    tries: HashMap<K, (Instant, Instant)>,
}

impl<K> Default for PaintedPictures<K> {
    fn default() -> Self {
        PaintedPictures {
            pictures: Pictures::default(),
            retry: Retry::default(),
            tries: HashMap::new(),
        }
    }
}

impl<K: Copy + Eq + Hash + std::fmt::Debug> PaintedPictures<K> {
    /// A new frame ([`Pictures::begin_frame`]).
    pub fn begin_frame(&mut self) {
        self.pictures.begin_frame();
    }

    /// `key`'s picture shown this frame, as its cell shows it.
    pub fn get(&mut self, key: K) -> Picture<'_> {
        self.pictures.show(key, |_| false);
        match self.pictures.get(&key) {
            Some(PaintedPicture(Ok(texture))) => Picture::Ready(texture.id()),
            Some(PaintedPicture(Err(reason))) => Picture::Failed(reason),
            None => Picture::Waiting,
        }
    }

    /// After a frame: paints those it asked for and lacks, within
    /// `budget`, one painted Later again only once [`Retry::again`] has
    /// passed (and as drawn past [`Retry::give_up`]); those no longer
    /// shown stop being waited for. Asks for another frame where more
    /// wait. Past `keep` made, the longest unseen are let go of: the
    /// painter's own textures among them ([`Painted::Drawn`]) are
    /// returned, to free.
    pub fn pump(
        &mut self,
        ctx: &egui::Context,
        painter: &mut (impl PicturePainter<K> + ?Sized),
        budget: Budget,
        keep: usize,
    ) -> Vec<egui::TextureId> {
        let wanted = self.pictures.take_wanted();
        self.tries.retain(|k, _| wanted.contains(k));
        let (started, mut painted, mut left) = (Instant::now(), 0, Vec::new());
        // (Those waiting their turn ask for a frame after the wait; those
        // over the budget, and those made, for the next at once.)
        let (mut waiting, mut at_once) = (false, false);
        for key in wanted {
            if self.pictures.get(&key).is_some() {
                continue;
            }
            let now = Instant::now();
            let try_at = self.tries.get(&key).copied();
            if try_at.is_some_and(|(_, last)| now.duration_since(last) < self.retry.again) {
                waiting = true;
                left.push(key);
                continue;
            }
            if !budget.more(painted, started) {
                at_once = true;
                left.push(key);
                continue;
            }
            let as_drawn =
                try_at.is_some_and(|(first, _)| now.duration_since(first) >= self.retry.give_up);
            painted += 1;
            let made = match painter.paint(key, as_drawn) {
                Painted::Ready(image) => Ok(Texture::Own(ctx.load_texture(
                    format!("picture-{key:?}"),
                    image,
                    Default::default(),
                ))),
                Painted::Drawn(id) => Ok(Texture::Painters(id)),
                Painted::Failed(reason) => Err(reason),
                Painted::Later if as_drawn => {
                    Err(format!("Not drawn within {} s", self.retry.give_up.as_secs()))
                }
                Painted::Later => {
                    let first = try_at.map_or(now, |(first, _)| first);
                    self.tries.insert(key, (first, now));
                    waiting = true;
                    left.push(key);
                    continue;
                }
            };
            self.tries.remove(&key);
            self.pictures.insert(key, PaintedPicture(made));
            at_once = true;
        }
        if waiting {
            ctx.request_repaint_after(self.retry.again);
        }
        if at_once {
            ctx.request_repaint();
        }
        self.pictures.set_wanted(left);
        self.pictures.let_go_unseen(keep).into_iter().filter_map(PaintedPicture::painters).collect()
    }

    /// Lets go of every picture, and of what was waited for; the painter's
    /// own textures returned, to free.
    pub fn forget(&mut self) -> Vec<egui::TextureId> {
        self.tries.clear();
        self.pictures.forget().into_iter().filter_map(PaintedPicture::painters).collect()
    }

    /// Lets go of one picture (its subject edited); the painter's own
    /// texture returned, to free.
    pub fn forget_one(&mut self, key: K) -> Option<egui::TextureId> {
        self.tries.remove(&key);
        self.pictures.remove(&key).and_then(PaintedPicture::painters)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shown_pictures_are_wanted_until_made_and_again_when_stale() {
        let mut p: Pictures<u32, &str> = Pictures::default();
        p.begin_frame();
        p.show(1, |_| false);
        p.show(2, |_| false);
        p.show(1, |_| false);
        assert_eq!((p.wanted(), p.shown_now()), (&[1, 2, 1][..], 3));
        let wanted = p.take_wanted();
        assert_eq!(p.insert(wanted[0], "one"), None);
        p.set_wanted(vec![2]);
        p.begin_frame();
        assert!(p.wanted().is_empty(), "a new frame asks anew");
        p.show(1, |v| *v == "one");
        p.show(2, |_| false);
        assert_eq!(p.wanted(), &[1, 2]);
        assert_eq!(p.insert(1, "uno"), Some("one"));
        assert_eq!(p.get(&1), Some(&"uno"));
    }

    #[test]
    fn the_longest_unseen_go_past_those_kept() {
        let mut p: Pictures<u32, u32> = Pictures::default();
        for key in 0..6 {
            p.begin_frame();
            p.show(key, |_| false);
            p.insert(key, key * 10);
        }
        // (Frame 6: 5 seen in it, 4 in the last; 0 longest unseen.)
        assert!(p.let_go_unseen(8).is_empty());
        let mut gone = p.let_go_unseen(4);
        gone.sort();
        assert_eq!(gone, [0, 10]);
        // (Past what may go: never one seen in this frame or the last.)
        let mut gone = p.let_go_unseen(0);
        gone.sort();
        assert_eq!(gone, [20, 30]);
        assert_eq!(p.len(), 2);
        let mut all = p.forget();
        all.sort();
        assert_eq!(all, [40, 50]);
        assert!(p.is_empty() && p.wanted().is_empty());
    }

    #[test]
    fn painted_pictures_wait_for_later_and_keep_failures() {
        let ctx = egui::Context::default();
        let retry = Retry { again: Duration::ZERO, give_up: Duration::from_secs(3600) };
        let mut p: PaintedPictures<u32> = PaintedPictures { retry, ..Default::default() };
        let asked: std::cell::RefCell<Vec<(u32, bool)>> = Default::default();
        let mut painter = |key: u32, as_drawn: bool| {
            asked.borrow_mut().push((key, as_drawn));
            match key {
                1 => Painted::Ready(egui::ColorImage::filled([2, 2], egui::Color32::RED)),
                2 => Painted::Failed("no model".into()),
                _ => Painted::Later,
            }
        };
        let frame = |p: &mut PaintedPictures<u32>,
                     painter: &mut dyn FnMut(u32, bool) -> Painted,
                     keys: &[u32]| {
            p.begin_frame();
            for k in keys {
                let _ = p.get(*k);
            }
            p.pump(&ctx, painter, Budget::default(), PICTURES_KEPT);
        };
        frame(&mut p, &mut painter, &[1, 2, 3]);
        assert!(matches!(p.get(1), Picture::Ready(_)));
        assert!(matches!(p.get(2), Picture::Failed("no model")));
        assert!(matches!(p.get(3), Picture::Waiting));
        frame(&mut p, &mut painter, &[1, 2, 3]);
        assert_eq!(
            *asked.borrow(),
            [(1, false), (2, false), (3, false), (3, false)],
            "a failure not asked again"
        );
        // (Out of sight: no longer waited for, its first try forgotten.)
        frame(&mut p, &mut painter, &[1]);
        assert!(p.tries.is_empty());
        // (Past the limit: asked as drawn, and failed if still Later.)
        p.retry.give_up = Duration::ZERO;
        frame(&mut p, &mut painter, &[3]);
        frame(&mut p, &mut painter, &[3]);
        assert_eq!(asked.borrow().last(), Some(&(3, true)));
        assert!(matches!(p.get(3), Picture::Failed(r) if r.starts_with("Not drawn")));
    }

    /// A texture the painter drew is shown as its own, and handed back when
    /// let go of (past those kept, one forgotten, all forgotten); the
    /// pictures' own textures never are.
    #[test]
    fn a_painter_s_own_texture_is_handed_back_when_let_go_of() {
        let ctx = egui::Context::default();
        let mut p: PaintedPictures<u64> = PaintedPictures::default();
        let mut painter = |key: u64, _| match key {
            0 => Painted::Ready(egui::ColorImage::filled([2, 2], egui::Color32::RED)),
            key => Painted::Drawn(egui::TextureId::User(key)),
        };
        let mut gone = Vec::new();
        for key in 0..6 {
            p.begin_frame();
            let _ = p.get(key);
            gone.extend(p.pump(&ctx, &mut painter, Budget::default(), 3));
        }
        assert!(matches!(p.get(5), Picture::Ready(egui::TextureId::User(5))));
        gone.sort_unstable_by_key(|id| format!("{id:?}"));
        assert_eq!(gone, [egui::TextureId::User(1), egui::TextureId::User(2)], "0 is the kit's");
        assert_eq!(p.forget_one(5), Some(egui::TextureId::User(5)));
        assert_eq!(p.forget_one(5), None);
        let mut rest = p.forget();
        rest.sort_unstable_by_key(|id| format!("{id:?}"));
        assert_eq!(rest, [egui::TextureId::User(3), egui::TextureId::User(4)]);
    }

    #[test]
    fn a_later_is_asked_again_only_after_its_wait() {
        let ctx = egui::Context::default();
        let retry = Retry { again: Duration::from_secs(3600), give_up: Duration::from_secs(7200) };
        let mut p: PaintedPictures<u32> = PaintedPictures { retry, ..Default::default() };
        let mut count = 0;
        let mut painter = |_: u32, _: bool| {
            count += 1;
            Painted::Later
        };
        // (The frame each pump asks for: after the wait, never at once
        // while it only waits.)
        let mut delays = Vec::new();
        for _ in 0..4 {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                let ctx = ui.ctx();
                p.begin_frame();
                let _ = p.get(7);
                p.pump(ctx, &mut painter, Budget::default(), PICTURES_KEPT);
            });
            delays.push(out.viewport_output[&egui::ViewportId::ROOT].repaint_delay);
            out.textures_delta.clear();
        }
        assert_eq!(count, 1, "four frames within the wait: asked once");
        assert_eq!(p.pictures.wanted(), &[7]);
        // (egui asks for its own first frames again.)
        assert!(delays[2..].iter().all(|d| *d >= Duration::from_secs(3000)), "{delays:?}");
    }

    #[test]
    fn a_budget_makes_one_then_what_fits() {
        let started = Instant::now();
        let none = Budget { time: Duration::ZERO, most: 8 };
        assert!(none.more(0, started) && !none.more(1, started));
        let eight = Budget { time: Duration::from_secs(60), most: 8 };
        assert!(eight.more(7, started) && !eight.more(8, started));
    }
}
