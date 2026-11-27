//! Fields named by the labels beside them, for a screen reader (and a
//! test's `get_by_label`): egui 0.37 names a widget only by what it is
//! told (`Response::labelled_by`, `accessible_name`), and a form's label
//! drawn before its field tells it nothing.

use egui::{Id, LayerId, Rect};

/// How far right of a label the first field it names may begin (points):
/// a form's label column and its gap.
const BESIDE: f32 = 200.0;
/// How far apart the fields of a row a label names may be (points): a
/// box and the filter before it, a slider and its number.
const BETWEEN: f32 = 60.0;
/// How far below a label a field under it may begin (points).
const UNDER: f32 = 12.0;
/// How far apart a field under a label and the label may begin sideways
/// (points).
const UNDER_SIDE: f32 = 8.0;

/// The context's plugin (added by [`crate::apply`]) that, as a pass ends
/// and while a screen reader (or a test) reads the window, names each
/// input with no name of its own (no label, none it is labelled by, no
/// hint) by the label before it on its row (beside it over some of its
/// height: the label and the row's fields, each close to the one before
/// it), or else by the one over it at its left edge (a narrow form's label
/// above its field; a table's column heading, over the fields of the rows
/// before), in the same layer: as egui's
/// `Ui::label_inputs_by` names each input of a row by its label, where a
/// form's code wraps each row.
///
/// A field keeps the label it had in the pass before while that label is
/// drawn and no other is found: a grid's rows are laid out by their heights
/// in the pass before, so a form's label stands a pass off its field as
/// the form changes (another object's). A field new in the pass is named
/// by a label as far as its height under its row too: a new grid's first
/// pass sized and discarded (egui's), its rows stand by those sizes in
/// the pass after it.
#[derive(Debug, Default)]
pub struct FieldLabels {
    /// Each field's label in the pass before.
    kept: egui::IdMap<Id>,
    /// The fields with no name of their own in the pass before.
    before: egui::IdSet,
}

/// A widget of the pass, as the plugin reads it.
struct Seen {
    layer: LayerId,
    /// The whole of it.
    rect: Rect,
    /// What of it its clip leaves, where anything is left.
    shown: Option<Rect>,
    /// Cut by its clip (in a scroll, partly or wholly out of sight).
    clipped: bool,
    id: Id,
    /// A label, with words.
    label: bool,
    /// An input with no name.
    unnamed: bool,
}

impl Seen {
    /// Whether `self` can be beside `at` as drawn: what of each is in sight
    /// overlaps (a row scrolled under a bar is not beside the bar's
    /// buttons), or where either is out of sight, each is clipped (a row
    /// scrolled out of sight is beside its label, out of sight too, not
    /// beside what is drawn where it would be).
    fn beside(&self, at: &Seen) -> bool {
        match (self.shown, at.shown) {
            (Some(w), Some(a)) => w.y_range().intersection(a.y_range()).span() > 0.0,
            _ => (self.shown.is_none() || self.clipped) && (at.shown.is_none() || at.clipped),
        }
    }
}

impl FieldLabels {
    /// Adds the plugin to `ctx` (once).
    pub fn install(ctx: &egui::Context) {
        if ctx.plugin_opt::<FieldLabels>().is_none() {
            ctx.add_plugin(FieldLabels::default());
        }
    }
}

/// The label of `seen` that names the input `at`, `new` in the pass (else
/// none under its row is taken).
fn label_for(seen: &[Seen], at: &Seen, new: bool) -> Option<Id> {
    let low = egui::Rangef::new(at.rect.top(), at.rect.bottom() + at.rect.height());
    label_in(seen, at, at.rect.y_range()).or_else(|| new.then(|| label_in(seen, at, low)).flatten())
}

/// [`label_for`], beside the row `row`.
fn label_in(seen: &[Seen], at: &Seen, row: egui::Rangef) -> Option<Id> {
    let here = || (seen.iter()).filter(|w| w.layer == at.layer && w.id != at.id && w.beside(at));
    // (On its row: beside it over two points of its height at least, a
    // label centered on a field and what is under it, a strref's line.)
    let on_row = |w: Rect| w.y_range().intersection(row).span() >= 2.0;
    // From the field leftward, across the row's other fields, to a label.
    let mut left = at.rect.left();
    loop {
        let before = here()
            .filter(|w| on_row(w.rect) && w.rect.right() <= left + 1.0)
            .max_by(|a, b| a.rect.right().total_cmp(&b.rect.right()));
        let Some(w) = before else { break };
        let gap = left - w.rect.right();
        if w.label {
            if gap <= BESIDE {
                return Some(w.id);
            }
            break;
        }
        if gap > BETWEEN {
            break;
        }
        left = w.rect.left();
    }
    // Else from the field upward, across the fields over it at its left
    // edge (a table's column), to a label: a narrow form's label over its
    // field, a column's heading.
    let mut top = at.rect.top();
    loop {
        // (In sight or not: a field scrolled out of sight under its label
        // still in sight is under it.)
        let over = (seen.iter())
            .filter(|w| w.layer == at.layer && w.id != at.id)
            .filter(|w| w.rect.bottom() <= top + 1.0)
            .filter(|w| (w.rect.left() - at.rect.left()).abs() <= UNDER_SIDE)
            .max_by(|a, b| a.rect.bottom().total_cmp(&b.rect.bottom()));
        let w = over?;
        let gap = top - w.rect.bottom();
        if w.label {
            return (gap <= UNDER).then_some(w.id);
        }
        if gap > UNDER {
            return None;
        }
        top = w.rect.top();
    }
}

impl egui::Plugin for FieldLabels {
    fn debug_name(&self) -> &'static str {
        "mg-uikit field labels"
    }

    fn on_end_pass(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        // (Nothing to name where nothing reads the tree: the usual case.)
        if !ctx.has_accesskit_node(egui::accesskit_root_id()) {
            return;
        }
        // (Not those egui leaves out of the tree: invisible, as in a sizing
        // pass.)
        let widgets: Vec<(LayerId, Rect, Rect, Id)> = ctx.viewport(|v| {
            (v.this_pass.widgets.layers())
                .flat_map(|(_, ws)| ws.iter().filter(|w| w.visible))
                .map(|w| (w.layer_id, w.rect, w.interact_rect, w.id))
                .collect()
        });
        let mut seen = Vec::new();
        for (layer, rect, interact, id) in widgets {
            if !rect.is_positive() || !ctx.has_accesskit_node(id) {
                continue;
            }
            let read = ctx.accesskit_node_builder(id, |n| {
                // (Each label once: a field named by its form's label and
                // by its caller's `labelled_by` too is said "Into Into".)
                let by = n.labelled_by();
                if (1..by.len()).any(|i| by[..i].contains(&by[i])) {
                    let mut once = Vec::new();
                    for l in by {
                        if !once.contains(l) {
                            once.push(*l);
                        }
                    }
                    n.set_labelled_by(once);
                }
                let role = n.role();
                let input = egui::accessibility::is_input(role) && !n.is_read_only();
                let named = n.label().is_some_and(|l| !l.trim().is_empty())
                    || !n.labelled_by().is_empty()
                    || n.placeholder().is_some_and(|p| !p.trim().is_empty());
                let label = role == egui::accesskit::Role::Label
                    && n.value().is_some_and(|v| !v.trim().is_empty());
                (label, input && !named, input)
            });
            if let Some((label, unnamed, input)) = read
                && (label || input)
            {
                let shown = interact.is_positive().then_some(interact);
                let clipped = interact != rect;
                seen.push(Seen { layer, rect, shown, clipped, id, label, unnamed });
            }
        }
        let labels: egui::IdSet = seen.iter().filter(|s| s.label).map(|s| s.id).collect();
        let named: Vec<(Id, Id)> = (seen.iter())
            .filter(|s| s.unnamed)
            .filter_map(|at| {
                let kept = || self.kept.get(&at.id).copied().filter(|l| labels.contains(l));
                let new = !self.before.contains(&at.id);
                Some((label_for(&seen, at, new).or_else(kept)?, at.id))
            })
            .collect();
        self.kept = named.iter().map(|(label, id)| (*id, *label)).collect();
        self.before = seen.iter().filter(|s| s.unnamed).map(|s| s.id).collect();
        for (label, id) in named {
            ctx.accesskit_node_builder(id, |n| {
                n.clear_label();
                n.push_labelled_by(label.accesskit_id());
            });
        }
    }
}
