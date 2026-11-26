//! What the filters typed of late into a Find field found, each narrowing
//! the one before it: a key typed looks among what the filter before it
//! found, and a key taken back finds its filter's again, rather than each
//! searching every name (Choose Image's 130,000 pictures, Select Texture's
//! 135,000 textures).

use std::sync::Arc;

use mg_core::text::Needle;

/// The most filters kept: a long name typed key by key keeps the last few
/// dozen.
const KEPT: usize = 64;

/// The filters kept, each with what it found, by places in the list
/// searched, the widest first. Cheap to clone (egui's memory holds one).
#[derive(Clone, Default)]
pub(crate) struct Narrowing {
    kept: Vec<(Needle, Arc<Vec<u32>>)>,
}

impl Narrowing {
    /// The places among `len` names that `query` (not empty) finds,
    /// `found` telling of each: the filters it does not narrow are let go,
    /// then it is looked for among what the one left found, or in all.
    pub(crate) fn find(
        &mut self,
        query: Needle,
        len: usize,
        found: impl Fn(u32) -> bool,
    ) -> Arc<Vec<u32>> {
        while self.kept.last().is_some_and(|(wider, _)| !query.narrows(wider)) {
            self.kept.pop();
        }
        if let Some((_, at)) = self.kept.last().filter(|(same, _)| *same == query) {
            return at.clone();
        }
        let at: Arc<Vec<u32>> = Arc::new(match self.kept.last() {
            Some((_, within)) => within.iter().copied().filter(|&i| found(i)).collect(),
            None => (0..len as u32).filter(|&i| found(i)).collect(),
        });
        if self.kept.len() == KEPT {
            self.kept.remove(0);
        }
        self.kept.push((query, at.clone()));
        at
    }

    /// Lets every filter go (the list searched changed).
    pub(crate) fn clear(&mut self) {
        self.kept.clear();
    }

    /// How many filters are kept (for tests).
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.kept.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A filter typed key by key, taken back and typed anew finds what a
    /// search of every name does, and one taken back to is not searched
    /// again.
    #[test]
    fn filters_typed_key_by_key_find_what_a_search_of_all_does() {
        let names = ["gui_error", "gui_errorq", "gui_ok", "ctl_btn", "über_gui", "x"];
        let mut n = Narrowing::default();
        let searched = std::cell::Cell::new(0);
        for filter in ["g", "gu", "gui", "gui_e", "gui_eq", "gui_e", "gui", "btn", "gui ok"] {
            let query = Needle::new(filter);
            let every: Vec<u32> =
                (0..names.len() as u32).filter(|&i| query.finds(names[i as usize])).collect();
            let found = n.find(query.clone(), names.len(), |i| {
                searched.set(searched.get() + 1);
                query.finds(names[i as usize])
            });
            assert_eq!(*found, every, "{filter:?}");
        }
        n.find(Needle::new("gui_"), names.len(), |i| names[i as usize].contains("gui_"));
        n.find(Needle::new("gui_o"), names.len(), |i| names[i as usize].contains("gui_o"));
        let (kept, before) = (n.len(), searched.get());
        let back = n.find(Needle::new("gui_"), names.len(), |_| unreachable!());
        assert_eq!((back.len(), n.len()), (3, kept - 1));
        assert_eq!(searched.get(), before);
    }
}
