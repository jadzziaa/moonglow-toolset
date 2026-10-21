---
title: One row list widget for every 2DA row dropdown
author: August
date: 2026-10-21
branch: worktree-agent-a388d438e9b59f79e
cost: { minutes: 119, tokens: 1043323 }
touches:
  - crates/mg-rules/src/lib.rs
  - crates/mg-ui/src/blueprint/creature.rs
  - crates/mg-ui/src/blueprint/creature_lists.rs
  - crates/mg-ui/src/blueprint/encounter.rs
  - crates/mg-ui/src/blueprint/item.rs
  - crates/mg-ui/src/blueprint/mod.rs
  - crates/mg-ui/src/blueprint/situated.rs
  - crates/mg-ui/src/blueprint/store.rs
  - crates/mg-ui/src/blueprint_wizard.rs
  - crates/mg-ui/src/gff_view.rs
---

## Decided
- Every "choose a row of a table" dropdown goes through `widgets::row_choice` (a `RowChoice` builder: `width`, `ordered`, `wheel`, `none` for a row of none, `unset` for what shows with nothing chosen, `shown`, `numbered`, `hover_text`, `about` for a row's picture; `show` / `show_optional`). `Form::choice_shown` was the source of truth: by name past `mg_rules::ORDERED_CHOICES` (12), a Filter past `FILTERED_CHOICES` (30), opened at the chosen row, `arrow_pick`. The "name, else (n)" text is `mg_rules::choice_text`.
- Lists whose order means something take `.ordered()`: body and armor parts (`pick_part`, `pick_sized`: "Middle 21" sorts before "Middle 3" as text), item property costs (`pick_ordered`: +10 before +2), the palette category (its own palette or alphabetical order, Options › General), the 2DA editor's rows.
- The "≠" mark stays in `Form::mark_mixed`, called before the widget; it needs the form's other blueprints.
- No virtualized rows (`ScrollArea::show_rows`) in the widget: kittest finds only rows that are laid out, so tests clicking a row by label break, and the popup's width would change as rows scroll in. The 2DA editor instead makes its rows' names only while the list is in use (`RowChoice::in_use`).

## Watch out
- `ComboBox::from_id_salt(id)` makes its button id as `ui.make_persistent_id(IdSalt::new(id))`, not `ui.make_persistent_id(id)`: an id computed the second way never matches `ComboBox::is_open` or focus. `RowChoice::in_use` reads a flag the widget stores under its own key instead.
- A popup is measured in the pass a click opens it in (egui's sizing pass), and that size places it above or below the box from then on. A list whose rows are made lazily must have them in that pass, or it opens squeezed below the box at a window's foot: `in_use` counts the pointer over the box's last rect too (a `hovered()` flag stored the frame before flickered off in the harness). Seen in `table_row_list_opened` (screens.rs).
- A potion's Cast Spell lists only the spells iprp_spells allows on a potion (no Fireball): `property_subtypes` filters by base item.
- A clipped row in a long list may not take a kittest click: `scroll_to_me` first (the door's generic appearance, now by name).
