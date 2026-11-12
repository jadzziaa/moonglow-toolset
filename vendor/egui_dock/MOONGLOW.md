# egui_dock, patched for Moonglow

egui_dock 0.21.1 as published on crates.io, with two additions, in
`src/widgets/dock_area/mod.rs`, `src/widgets/dock_area/show/leaf.rs` and
`src/widgets/tab_viewer.rs`, each marked "Moonglow's patch" there.

The first is `DockArea::main_surface_end_room(room)`. With it, the main surface's tab
bars have no leaf collapse arrow and no close-all-tabs button (the windows'
bars keep both, as `show_leaf_collapse_buttons` and
`show_leaf_close_all_buttons` say), and `room` points at the right end of
each main bar are left free of tabs, for the application's own buttons.
Without the call nothing changes.

Why: egui_dock decides those two buttons for the whole `DockArea`, every
tab bar or none. Moonglow's editor area is one `DockArea` whose main
surface holds the editor groups and whose windows are the floating
editors; the design draws the groups' tab bars with no arrow and no close
all (the tab menu's Close All does that) and a Split Editor button at their
right end, and the floating windows' bars with the arrow that folds the
window to its bar. The arrow cannot be drawn by the application instead:
folding a window goes through egui_dock's crate-private state. (The
reporters of GitHub issue 10 met the same limit: `docs/deferred.md`.)

The second is `TabViewer::on_tab_dragged(ctx, tab, layer, rect)`, called for
the tab being dragged once its title is drawn on its own layer and before
that layer is moved after the pointer (`Context::transform_layer_shapes`
moves only what is drawn on it by then): Moonglow paints the tab's kind
icon, modified dot and close button there, so that the tab dragged looks
as it did in its bar. Without an implementation nothing changes.

The root `Cargo.toml` patches it in (`[patch.crates-io]`) and leaves it out
of the workspace. Drop it, and the patch, once egui_dock offers a per-surface
choice of those buttons, or Moonglow's editor area no longer needs one.
Moonglow Viewer takes `mg-uikit` by git tag, where this patch does not
apply: nothing in `mg-uikit` calls the addition. Everything else here is
egui_dock's own, under its MIT license (`LICENSE`).
