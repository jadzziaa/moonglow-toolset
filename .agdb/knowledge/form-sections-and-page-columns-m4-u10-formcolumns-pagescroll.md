---
type: Decision
title: "Form sections and page columns (M4 U10): FormColumns, PageScroll::columns, strref in the field"
description: "How a form page's sections flow into columns (mg_uikit::form::FormColumns) and how PageScroll stands pages side by side at wide widths (columns, natural heights, last page to the foot), with the feedback loops found doing it."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-16T04:18:10Z }
touches: ["crates/mg-uikit/src/form.rs:FormColumns", "crates/mg-uikit/src/pages.rs:show_columns", crates/mg-ui/src/form_pages.rs, "crates/mg-ui/src/blueprint/mod.rs:locstring", "crates/mg-ui/src/widgets.rs:spin_with"]
asks:
  - Why do the pages in columns jump between columns every frame?
  - "How do I put a page's sections into columns?"
  - "When do a blueprint's pages stand side by side?"
  - How is the StrRef drawn inside a name field?
fingerprint: v2-c0d3d63936a8be40
created: 2026-11-16
---
## Decided
- Sections: `mg_uikit::form::FormColumns` (kit, egui alone): `form_columns(w)` = as many columns of >= 440 (metrics::FORM_COLUMN) with a 32 gap as fit; sections kept whole, contiguous runs down column 1 then 2 (Tab order the same), split where the tallest column is least by heights kept from the last frame. Each section its own `UiBuilder::id`. A page that wants a different order per count asks `form_columns` first (creature Statistics: [Abilities, Saves] | [AC, HP] in two, Abilities, AC, HP, Saves in one).
- `blueprint::section(ui, title, id, add)` = heading + `form_grid(..).labels()` (label column >= LABEL_WIDTH 104 so sections line up).
- Pages side by side: `form_pages::page_columns(body)` = fit of PAGE_COLUMN 480 (the board's column, margins included; coordinator's ruling). Rows laid out for a window that ran past a 480 column were made to fit, each by its own room check (`blueprint::room_right` in a grid cell = to the page's right edge): door Generic appearance's Gallery under the list (GENERIC_ROW 450), Visuals' texture rows on two lines (TEXTURE_ROW 560), Trap's three checks one under another (TRAP_CHECKS_ROW 540), creature Sound set's filters over its list (SOUND_SET_ROW 420), encounter's palette over its Creature List; `every_page_of_every_kind_fits_its_column` sweeps 3420, 2090 (four columns of ~480) and 1180 with 1-pt lines, only when >= 4 fit, else 1 (one page across, sections in columns); from 4, not 3, so a docked editor up to ~2000 wide keeps a placeable's/door's model beside Basic (situated::beside_model needs side + 340, a 440 column hasn't it: a_placeable_s_and_a_door_s_page_show_the_model). 2960 room at 3440 today = 6; the board's 2400 (beside a Right Dock / U11's Preview) = 5. `form_pages::scroll` caps columns at the page count (< 4 -> one page across). Module/Area Properties get it too through `form_pages::scroll`.
- `PageScroll::columns(k)`: greedy shortest column by "natural" heights (Kept.natural, separate from the one-column heights); the column's last page drawn down to the foot (max(view, tallest column)), others given PAGE_FILL 320 for what fills; each page clipped to its column (beside_model reads clip_rect). Page at the top = the held page while its bar is in view, else the first whose bar is in view; no tail room after the last page. `Seen::pages_in_view` (non-contiguous) feeds `PageList::pages_in_view`.
- StrRef inside the name field: `Form::locstring` reserves room via temp data `blueprint::FIELD_END_ROOM` read in `widgets::commit_field` (TextEdit right margin), then a non-selectable Label in a child Ui over the field's right end (named "StrRef N", tip kept). Field width = row room minus two 22 buttons, 160..360; narrow forms keep 240 clamped to the room.
- Number arrows: `widgets::SpinArrow`, stacked, 14 wide, half the field height each.
## Gotchas
- Natural heights must not depend on the layout: a page measured in the room it gets when last (stretched) vs not last measures differently (Inventory 320 vs 463, Visuals 320 vs 876), and the greedy assignment oscillated every frame ("Harness::run exceeded max_steps", scroll_area repaint). Fix: a column's last page never updates its natural height (keeps the estimate or what it measured when not last).
- Any page drawn where the new layout puts it elsewhere -> `request_discard`.
- The place read after a pass must use the held page's place under the new heights (`into + (offset - start)`, then `tops[anchor] + into_now`), not egui's offset read against the new layout: a page above grown slid the held page (found by strengthening `in_columns_a_page_above_growing_moves_nothing_in_view`).
- Overflowing content in a column overlapped the next column until each page child got `set_clip_rect(column)`.
## Review (2026-11-16)
- Flow is sticky (`Kept::flowed`): re-flowed only when the column count or page count changes or a page is measured for the first time (coordinator's ruling); a growing page moves only those under it.
- Fill pages are told by a first measure in PROBE_ROOM 4000 (>= half of it = fills); every unmeasured page is drawn once (in a discarded pass). Fill natural = PAGE_FILL + what overhangs its room; fills share the column's spare *in view* (view_h - column bottom), not the tallest column's foot (that stretched Feats past the view and hid Spells forever).
- Fill detection by "measured == room" failed both ways: content taller than its room looked like a fill (pages drawn over each other), and a list with buttons under it measured room+k and grew without bound (`Harness::run exceeded max_steps`).
- Tab draws the pages beside the keyboard's page (in columns those in view are not consecutive).
- `FormColumns::keys` keeps ids/heights per section key (Statistics reorders by count); split changes and misplaced pages use `request_discard`, not repaint (no one-frame flash).
- `blueprint::list_beside`: list + side (35%, 150..320), side truncates; do not Truncate the list part (grid columns lock to first-frame width).

