---
title: Visuals page texture and animation pickers (issue 32)
author: August
date: 2026-10-26
branch: issue-32-visuals-pickers
cost: { minutes: 1035, tokens: 10068381 }
touches:
  - crates/mg-preview/src/lib.rs
  - crates/mg-ui/src/blueprint/mod.rs
  - crates/mg-ui/src/blueprint/visuals.rs
  - crates/mg-ui/src/images.rs
  - crates/mg-ui/tests/app.rs
  - crates/mg-ui/tests/editor_perf.rs
  - crates/mg-ui/tests/screens.rs
  - docs/deferred.md
  - docs/log.md
  - docs/manual/05-blueprints.md
---

## Decided
- The Visuals page's texture/animation lists come from `mg_preview::replaceable` (`textures`, `animations`) over the object's preview: meshes' `bitmap` (rendered meshes), PLT = a PLT of the name and no DDS/TGA; animations through `mg_render::anim::animations` (model + supermodel chain). The UI code is `blueprint/replacements.rs`; visuals.rs keeps shader params and MiscVisuals.
- A creature's list is its own body: the preview with `Equip_ItemList` cut to the chest slot (0x2), so armor still picks the body parts but weapons, helmet and cloak are not listed. Placed creatures carry equipped items inline, so filtering the item closure alone would not have been enough.
- The lists are cached in `ui.data` keyed by the form's id, stamped with `ws.revision()` and the game data's `Arc` pointer.
- Select Texture is an `egui::Modal` (like Select Portrait) with `picture_tile`/`fitted`/`size_slider` from the Appearance Gallery; texture names (DDS+TGA of resman and the module, ~29.5k) are listed once on opening (15 ms release).
- Thumbnails: `Loader::thumbnail(name, side)` decodes the first DDS mip level no larger than `side` (key `name#thumbN`), never the full texture. The gallery makes them under a 6 ms budget a frame and lets go of them when it closes, and of the oldest out of sight past 1500.
- Writing: an entry is edited in place (its other fields kept), a new one goes at the end as struct 0; Reset removes the entry; an empty list removes the field (as before).

## Tried and ruled out
- `ctx.request_repaint_after(10ms)` doesn't stop kittest's `h.run()` from looping: kittest's step_dt is 1/4 s, so egui treats any shorter delay as immediate and `run()` panics past max_steps 4. Tests that open the gallery loop `h.try_run()` until Ok (`settle` in app.rs) instead.
- `nw_bartender` and `nw_bandit001` are single-model creatures (c_ybartender, c_ynpc_h_m03), not bodies of parts: a test sets Appearance_Type 6 and the BodyPart fields.

## Watch out
- The game's `pmh0_handl00N` hand textures are simple textures, not PLTs, so a part-based creature's list is mostly, not only, PLTs.
- `show_rows` steps rows by the row height plus the scroll area's own `item_spacing.y` (egui's default 3), not the spacing set inside the rows: a scroll-to-row offset computed with the inner gap drifts by a pixel a row (the Appearance Gallery still does).
- The worktree guard refuses heredocs and `flock ... cargo test FILTER`: build with `--no-run`, run the binary from the crate folder under flock (a script in the scratchpad for several).
