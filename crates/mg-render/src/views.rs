//! What the renderer keeps for a view from one of its frames to the next.
//!
//! Every 3D view of the window draws through one [`crate::Renderer`]: the
//! area views, the model viewer, a blueprint's preview, the thumbnails.
//! What a frame keeps for the next (the sun's shadow map and what it
//! holds, the lights each mesh chose, the line shapes sent, the stencil
//! volumes' target) is one view's: kept in one place, a second view drawn
//! between an area's frames undid it, and the area's next frame drew its
//! shadow map, chose every mesh's lights and sent its shapes again.
//!
//! A view is known by its targets' framebuffer copies
//! ([`crate::framebuffer::Copies`]' number; `None` for a draw without
//! copies, all of which are one view). A few views' states are kept
//! ([`VIEWS_KEPT`]); past them, a new view takes over the state of the one
//! drawn longest ago, as it is: each thing kept says what it is of (the
//! map what it holds, the lights' choices their lights, the target its
//! size), and is made again where that is not what the frame needs. The
//! state of a view whose targets are gone is dropped: when the view says
//! so ([`crate::Renderer::let_go_of`]), or at the next frame drawn
//! ([`Views::take_gone`]: targets made again at another size).

use std::sync::Weak;

/// How many views' states are kept: an area view or two side by side, the
/// model viewer or a preview, the thumbnails' targets.
pub(crate) const VIEWS_KEPT: usize = 4;

/// A view: its targets' copies' number, and what tells that they are
/// still there ([`crate::framebuffer::Copies::alive`]).
pub(crate) type View = (u64, Weak<()>);

/// A view's state `T`, for each of the views drawn lately.
#[derive(Debug)]
pub(crate) struct Views<T> {
    /// Each view's state, and when it was put back (by [`Views::clock`]).
    kept: Vec<(Option<View>, u64, T)>,
    clock: u64,
}

impl<T> Default for Views<T> {
    fn default() -> Self {
        Views { kept: Vec::new(), clock: 0 }
    }
}

fn number(view: &Option<View>) -> Option<u64> {
    view.as_ref().map(|(id, _)| *id)
}

impl<T: Default> Views<T> {
    /// Takes `view`'s state out, for its frame ([`Views::put`] gives it
    /// back): its own; else a new one, while fewer than [`VIEWS_KEPT`]
    /// are kept; else that of the view drawn longest ago, as it is.
    pub(crate) fn take(&mut self, view: Option<u64>) -> T {
        let own = self.kept.iter().position(|(of, ..)| number(of) == view);
        let at = own.or_else(|| {
            let oldest = self.kept.iter().enumerate().min_by_key(|(_, (_, when, _))| *when);
            oldest.filter(|_| self.kept.len() >= VIEWS_KEPT).map(|(i, _)| i)
        });
        at.map_or_else(T::default, |i| self.kept.swap_remove(i).2)
    }

    /// Gives `view`'s state back after its frame.
    pub(crate) fn put(&mut self, view: Option<View>, state: T) {
        self.clock += 1;
        self.kept.retain(|(of, ..)| number(of) != number(&view));
        self.kept.push((view, self.clock, state));
    }

    /// Drops `view`'s state (its targets are let go of).
    pub(crate) fn forget(&mut self, view: u64) {
        self.kept.retain(|(of, ..)| number(of) != Some(view));
    }

    /// Drops the states of the views whose targets are gone: their
    /// numbers.
    pub(crate) fn take_gone(&mut self) -> Vec<u64> {
        let gone =
            |of: &Option<View>| of.as_ref().is_some_and(|(_, alive)| alive.strong_count() == 0);
        let numbers =
            self.kept.iter().filter(|(of, ..)| gone(of)).filter_map(|(of, ..)| number(of));
        let numbers: Vec<u64> = numbers.collect();
        self.kept.retain(|(of, ..)| !gone(of));
        numbers
    }

    /// How many views' states are kept.
    pub(crate) fn len(&self) -> usize {
        self.kept.len()
    }

    /// `view`'s state, where it is kept.
    pub(crate) fn of(&self, view: Option<u64>) -> Option<&T> {
        self.kept.iter().find(|(of, ..)| number(of) == view).map(|(.., state)| state)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    /// A view finds its own state again whatever was drawn between; past
    /// the views kept, the one drawn longest ago gives its state up; a
    /// view forgotten, or whose targets are gone, leaves nothing.
    #[test]
    fn a_view_keeps_its_state_across_other_views_frames() {
        let mut views: Views<Vec<u32>> = Views::default();
        let targets: Vec<Arc<()>> = (0..5).map(|_| Arc::new(())).collect();
        let draw = |views: &mut Views<Vec<u32>>, view: Option<u64>, frame: u32| {
            let mut state = views.take(view);
            let before = state.clone();
            state.push(frame);
            views.put(view.map(|v| (v, Arc::downgrade(&targets[v as usize]))), state);
            before
        };
        assert!(draw(&mut views, Some(1), 10).is_empty());
        assert!(draw(&mut views, Some(2), 20).is_empty());
        assert_eq!(draw(&mut views, Some(1), 11), [10], "its own, after another's frame");
        assert!(draw(&mut views, None, 30).is_empty(), "draws without copies are a view");
        assert!(draw(&mut views, Some(3), 40).is_empty());
        assert_eq!(views.kept.len(), VIEWS_KEPT);
        // A fifth view: the one drawn longest ago (2) gives up its state.
        assert_eq!(draw(&mut views, Some(4), 50), [20]);
        assert_eq!(views.kept.len(), VIEWS_KEPT);
        assert!(views.of(Some(2)).is_none());
        assert_eq!(draw(&mut views, Some(1), 12), [10, 11]);
        assert_eq!(draw(&mut views, None, 31), [30]);
        views.forget(1);
        assert!(views.of(Some(1)).is_none());
        assert!(draw(&mut views, Some(1), 13).is_empty(), "forgotten");
        assert_eq!(views.of(Some(4)), Some(&vec![20, 50]));
        // Targets dropped: their view's state goes at the next frame.
        assert!(views.take_gone().is_empty());
        let mut targets = targets;
        targets[4] = Arc::new(());
        assert_eq!(views.take_gone(), [4]);
        assert!(views.of(Some(4)).is_none() && views.of(Some(1)).is_some());
        assert!(views.of(None).is_some(), "the draws without copies stay");
    }
}
