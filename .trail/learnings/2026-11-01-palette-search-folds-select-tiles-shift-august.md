---
title: "Palette search folds, Select Tiles Shift and void, import dialog, close-range handles"
author: August
date: 2026-11-01
branch: worktree-agent-a1efdaff9201cc25c
cost: { minutes: 90 }
touches:
  - crates/mg-ui/src/palette_view.rs
  - crates/mg-ui/src/terrain_mode.rs
  - crates/mg-ui/src/tile_select.rs
  - crates/mg-ui/src/transfer.rs
  - crates/mg-ui/src/area_view.rs
---

## Decided
- A palette's search must not leave egui's open state behind: `CollapsingHeader::open(Some(true))` (and `CollapsingState::set_open`) writes the state into egui memory, so every category a search opened stayed open once it was cleared. The fix gives the headers their own id while a search is typed (`searching` added to the `id_salt` in `palette_view::Tree::node`, and to the tiles palette's folders and Gallery terrain headings in `terrain_mode::show_item`): the search's states and the plain ones never touch, so clearing it finds each category as it was, in every palette, with no bookkeeping.
- Select Tiles: Shift + click adds a tile (Ctrl + click still toggles), Shift + drag adds a box (`AreaView::tile_box_adds`, Shift held as the box began or as it ends). A click on no tile (past the area's edge, the sky) clears the selection unless Ctrl or Shift is held. A box is the ground rectangle between its corners clamped to the area (`tile_select::boxed_tiles`), so one begun in the void takes the tiles it covers.
- The area view's handle radius is one function, `AreaView::handle_radius` (eye distance × 0.0765, about a tenth of the view's height; a narrow view scales by its aspect). The old `.max(0.5)` floor is what made them overflow from close by; tests ask the view (`handle_radius_at`, `handle_points`) instead of repeating the formula.
- Import Resources: resizable, the overwrite list's height is the room left less what is under it, measured the frame before (egui temp data), as Open Script does with an estimated footer.

## Tried and ruled out
- Recording each header's state before a search to restore it after: a `CollapsingHeader`'s id is `ui.id().with(IdSalt::new(id_salt))`, the salt hashed twice, so `ui.make_persistent_id(salt)` is not its id: reading the state before the header draws it means repeating egui 0.36's hashing (`make_persistent_id(egui::IdSalt::new(salt))`), which a version bump could break.

## Watch out
- `widgets::modal` is pivoted at its middle (`centered`): resized by a corner, the window grows both ways and the corner leaves the pointer. A resizable dialog wants `.pivot(LEFT_TOP).default_pos(center - size / 2)`.
- egui keeps a window inside the screen: a test that drags a corner past the harness's edge sees the window pushed back, not resized as asked.
- A text field's `placeholder()` is gone from AccessKit once something is typed: find the palette's search by its rect taken while it is empty.
- Kittest finds two "⏷" (Expand All) once a module is open: the module tree's and the palette's.
