---
title: "Door and item hover pictures, Shown on kept in Options"
author: August
date: 2026-10-26
branch: appearance-pictures-and-shown-on
cost: { minutes: 1083, tokens: 10120909 }
touches:
  - crates/mg-ui/src/blueprint/door.rs
  - crates/mg-ui/src/blueprint/item.rs
  - crates/mg-ui/src/blueprint/mod.rs
  - crates/mg-ui/src/blueprint/situated.rs
  - crates/mg-ui/src/images.rs
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/src/model_view.rs
  - crates/mg-ui/src/options.rs
  - crates/mg-ui/src/settings.rs
  - crates/mg-ui/tests/app.rs
---

## Decided
- Hover pictures in a 2DA row list go through `RowChoice::about` (the `Shown` fn pointers in `blueprint/mod.rs`: `model_picture(table, column, row)` serves placeables, doortypes `Model` and genericdoors `ModelName`). A door's Generic Appearance writes two fields, so it builds `situated::pick_list(...).about(..)` itself rather than going through `Form::choice`.
- An item's lists (simple model, composite shapes/colors, armor parts) picture the item as it would be with the row: `Pictured::Item(u64)`, the u64 a hash of the item struct's Debug text plus the gender, the struct kept in `Thumbnails::items` until neither made nor wanted. Its icon (`item_icon_on`) is drawn beside: rings and amulets have no model of their own and preview as the bag.
- "Shown on" is `Settings::armor_on_woman` (Options › General). `Moonglow::item_icon` reads it, so every inventory, store page and the palette's Preview follow without touching their code. Female armor icons exist for every part (`ipf_*`, 182 like `ipm_*`); cloaks have only `icloak_m_*`, so `item_icon_on`'s fallback to his icon is what shows.
- Thumbnails are kept by `(Pictured, bool)`: the bool is "drawn on a woman" and is set only for UTI resources while the setting is on, so a man's and a woman's palette pictures coexist and switching back shows the kept ones at once. `begin_frame(revision, female)` carries the setting; `ModelView` stores the `female` it read with and `refresh` reads again when it differs (replaces the old `read_again`).

## Tried and ruled out
- Passing a built `RowChoice<'_>` into a helper and adding `.about()` there: `RowChoice` is invariant in its lifetime (`&'a mut dyn FnMut`), so a closure local to the helper can't be attached. Build the list inside the helper from (name, choices, current).
- Dropping UTI thumbnails when the setting changes instead of keying them: works, but redraws all armor on every switch and shows stale men meanwhile.

## Watch out
- `on_hover_ui` shows nothing on a disabled widget: the Generic Appearance list is disabled unless Appearance Type is 0, so a test hovers it on a generic door (`nw_door_strong`).
- A blueprint copy's ResRef in tests must fit 16 characters (`pics_nw_it_mring001` panicked in `ResKey::parse`).
- In screens.rs, rows of a long list scrolled out of sight are still in the accesskit tree with real-looking rects; find rows in sight from the list's Filter field (`placeholder() == Some("Filter")`, aligned with the box), and remember a long list may open above its box. Short lists (< 30) have no Filter.
- Non-GPU harnesses still "make" thumbnails (`drawn` counts, `(no picture)` shows): UI tests can count those without a GPU.
