//! The instances' material values, numbered: what a model's surfaces are
//! kept by is a number for each set of values ([`super::SurfaceKey`]), the
//! same for the same values however they are held.
//!
//! A set was found by comparing it with every set numbered so far, for
//! every instance with values of every frame: an area of two thousand
//! tinted placeables in fifty colorways came to fifty thousand
//! comparisons a frame. It is found by where its `Arc` points (an object's
//! values are made once and handed over each frame), then by a hash of
//! its contents.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use mg_image::mtr::{MaterialValue, Param};

/// How many places are kept past four a set before they are forgotten
/// (those of sets let go of since: found again by their contents).
const PLACES_KEPT: usize = 4096;

/// The sets of material values numbered, from 1 (0: none).
#[derive(Debug, Default)]
pub(super) struct ValueSets {
    sets: Vec<Arc<[MaterialValue]>>,
    /// A set's number by where its `Arc` points.
    by_place: crate::game::Fast<usize, u32>,
    /// The numbers of the sets of a hash.
    by_hash: HashMap<u64, Vec<u32>>,
    /// How many sets were compared, value for value (tests).
    #[cfg(test)]
    pub(super) compared: usize,
}

fn hash_of(values: &[MaterialValue]) -> u64 {
    let mut h = std::hash::DefaultHasher::new();
    for v in values {
        (&v.material, &v.name).hash(&mut h);
        match &v.value {
            Param::Int(i) => i.hash(&mut h),
            Param::Float(f) => f.iter().for_each(|x| x.to_bits().hash(&mut h)),
        }
    }
    h.finish()
}

impl ValueSets {
    /// The number of `values` (0: none): the same for the same values.
    pub(super) fn id(&mut self, values: &Arc<[MaterialValue]>) -> u32 {
        if values.is_empty() {
            return 0;
        }
        let place = Arc::as_ptr(values) as *const () as usize;
        // (Where it points, if what is numbered there is still it: the
        // place of a set let go of may be another's now.)
        if let Some(&id) = self.by_place.get(&place)
            && self.same(id, values)
        {
            return id;
        }
        let hash = hash_of(values);
        let known = self.by_hash.get(&hash).into_iter().flatten().copied();
        let id = match known.collect::<Vec<u32>>().into_iter().find(|&id| self.same(id, values)) {
            Some(id) => id,
            None => {
                self.sets.push(values.clone());
                let id = self.sets.len() as u32;
                self.by_hash.entry(hash).or_default().push(id);
                id
            }
        };
        if self.by_place.len() > 4 * self.sets.len() + PLACES_KEPT {
            self.by_place.clear();
        }
        self.by_place.insert(place, id);
        id
    }

    /// Whether the set numbered `id` is `values`: the same `Arc`, or the
    /// same values.
    fn same(&mut self, id: u32, values: &Arc<[MaterialValue]>) -> bool {
        let Some(set) = self.sets.get(id as usize - 1) else { return false };
        if Arc::ptr_eq(set, values) {
            return true;
        }
        #[cfg(test)]
        {
            self.compared += 1;
        }
        set == values
    }

    /// The set numbered `id`.
    pub(super) fn get(&self, id: u32) -> Option<&Arc<[MaterialValue]>> {
        self.sets.get(id.checked_sub(1)? as usize)
    }

    /// Forgets every set: numbered anew (with what is kept by the numbers).
    pub(super) fn clear(&mut self) {
        self.sets.clear();
        self.by_place.clear();
        self.by_hash.clear();
    }

    pub(super) fn len(&self) -> usize {
        self.sets.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(n: usize) -> Arc<[MaterialValue]> {
        let tint = |name: &str, x: f32| MaterialValue {
            material: None,
            name: name.into(),
            value: Param::Float(vec![x, 0.5, 0.25, 1.0]),
        };
        Arc::from(vec![tint("mgTint1", n as f32), tint("mgTint2", 1.0)])
    }

    /// A set has one number however it is held, found without comparing
    /// it with the others: by where it points, then by its contents' hash
    /// (of a thousand sets numbered, the last was compared with the 999
    /// before it for every instance of every frame).
    #[test]
    fn a_set_s_number_is_found_without_comparing_it_with_every_other() {
        let mut sets = ValueSets::default();
        assert_eq!(sets.id(&Arc::from(Vec::new())), 0, "no values: none");
        let all: Vec<Arc<[MaterialValue]>> = (0..1000).map(set).collect();
        let ids: Vec<u32> = all.iter().map(|s| sets.id(s)).collect();
        assert_eq!(ids, (1..=1000).collect::<Vec<u32>>());
        assert_eq!(sets.len(), 1000);
        // (None of the same hash among them: nothing compared.)
        assert!(sets.compared <= 4, "{} compared as they were numbered", sets.compared);
        // The same `Arc` again: by where it points, nothing compared.
        sets.compared = 0;
        assert_eq!(sets.id(&all[999]), 1000);
        assert_eq!(sets.id(&all[0]), 1);
        assert_eq!(sets.compared, 0);
        // The same values held elsewhere (another placeable of the same
        // colorway): compared once, with the set of its hash; then by
        // where it points, and once to see that it is still that set.
        let other = set(999);
        assert_eq!(sets.id(&other), 1000);
        assert_eq!(sets.compared, 1);
        assert_eq!(sets.id(&other), 1000);
        assert_eq!(sets.compared, 2);
        assert_eq!(sets.len(), 1000, "no set added");
        assert_eq!(sets.get(1000), Some(&all[999]));
        assert_eq!((sets.get(0), sets.get(1001)), (None, None));
        // Values that differ in one number, or in what they are set on.
        let mut near = set(7).to_vec();
        near[1].material = Some("m_cloth".into());
        assert_eq!(sets.id(&Arc::from(near)), 1001);
        // Forgotten: numbered anew.
        sets.clear();
        assert_eq!((sets.len(), sets.id(&all[500])), (0, 1));
    }
}
