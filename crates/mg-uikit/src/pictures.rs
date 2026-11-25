//! What a gallery keeps of its pictures, whatever draws them: the
//! pictures made, by what they are of; those asked for in this frame and
//! not made (or made and out of date), in the order asked; the frame each
//! was last asked for in; and those out of sight let go of when there are
//! many. A view asks for the pictures it shows, every frame it shows them
//! ([`Pictures::show`]); after the frame the application makes those
//! wanted, within a [`Budget`], and lets go of the unseen
//! ([`Pictures::let_go_unseen`]). No threads and no GPU: the application
//! reads, draws and frees, and what it keeps (`V`) is its own.
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
    fn a_budget_makes_one_then_what_fits() {
        let started = Instant::now();
        let none = Budget { time: Duration::ZERO, most: 8 };
        assert!(none.more(0, started) && !none.more(1, started));
        let eight = Budget { time: Duration::from_secs(60), most: 8 };
        assert!(eight.more(7, started) && !eight.more(8, started));
    }
}
