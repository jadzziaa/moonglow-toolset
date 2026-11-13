# egui_dock, patched for Moonglow

egui_dock 0.21.1 as published on crates.io, with three additions, in
`src/widgets/dock_area/mod.rs`, `src/widgets/dock_area/show/leaf.rs` and
`src/widgets/tab_viewer.rs` (and `WindowBarButton` exported from
`src/widgets/mod.rs`), each marked "Moonglow's patch" there.

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

The third is `TabViewer::window_bar_button(ui, button, response)` (with the
enum `WindowBarButton`, `Collapse` or `CloseAll`, and whether the pane is
its window's only one), called for a window's collapse arrow and close
all once egui_dock has taken their clicks and before it paints them:
where it answers true, egui_dock paints nothing of the button (fill,
symbol, border) and gives it none of its tips, and the application
paints it and names it on the `Response` given. What the buttons do
(the collapse through crate-private state, the close all through
`TabViewer::on_close`) stays egui_dock's. Without an implementation
nothing changes.

Why: the Floating board draws a window's bar with the suite's icon
buttons (a chevron at the left, Close at the right beside Maximize, each
22 points square), and egui_dock's own are a triangle and a cross with no
accessible name; the collapse cannot be drawn by the application instead
(as above), and a screen reader found two unnamed buttons on every
window.

The root `Cargo.toml` patches it in (`[patch.crates-io]`) and leaves it out
of the workspace. Drop it, and the patch, once egui_dock offers a per-surface
choice of those buttons, or Moonglow's editor area no longer needs one.
Moonglow Viewer takes `mg-uikit` by git tag, where this patch does not
apply: nothing in `mg-uikit` calls the additions. Everything else here is
egui_dock's own, under its MIT license (`LICENSE`).
