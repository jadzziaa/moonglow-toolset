---
type: Decision
title: "Edit Areas Together's Cancel All and the Properties panel's one scroll (M5 W2, rulings 11, 12, 66)"
description: "How Cancel All decides what it takes back, why the one scroll is the object Properties panel in the wide layout only, and the test pitfalls met."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-21T16:24:29Z }
touches: [crates/mg-ui/src/tool_sections.rs, "crates/mg-ui/src/area_props.rs:AreaChooser", crates/mg-ui/src/properties_panel.rs, "crates/mg-uikit/src/pages.rs:PageScroll", "crates/mg-uikit/src/popover.rs:PopCheck"]
asks:
  - how does Cancel All in Edit Areas Together know which edits are its own
  - why does the Properties panel show all pages on one scroll only in the wide layout (2560 points and up)
  - can Cancel All undo an edit made in another tab
  - why do the Overlays popover rows have no icons
fingerprint: v2-860b7ad031165f50
created: 2026-11-21
author: augustaugur
---
## Decided (owner's rulings of 2026-11-21, built in M5 W2)
- Ruling 11: Overlays popover rows have no icons (Main board); the toggles unfolded into the header row keep theirs via kit `PopCheck::unfolded_icon` (add-only).
- Ruling 12: the brief said "Area Properties in the Right Dock", but Area Properties is a tab/dialog already on one scroll (U9). The WideArea board draws the *placed object's* Properties on one scroll with `.sec-h` headings, so `properties_panel::one_scroll` = `Host::Dock && yields.wide` (window layout, not dock width: no flip at a dragged seam). Drawn by `blueprint::form_ui` with `AllPages { panel: true }` and kit `PageScroll::headings(true)`. Rail = index; the page at the top is stored in `panel_pages` per restype. Edit Areas Together keeps one page at a time (no board; its Enter/list keys need page 0).
- Ruling 66: Cancel All = the unbroken run of the section's own steps on top of the undo history since the ticks changed: `join_last(n)` then one Undo (Redo brings all back). Own steps: commands pushed during `together_page`'s draw are cloned into `Tools::marks` (and the held text's command into `held_mark`); `Moonglow::apply` asks `tool_sections::marked` and records `(step, undo_depth)` in `AreaChooser::steps`. `AreaChooser::run` walks back from the entry equal to (last_step, undo_depth): undone entries skipped, a depth gap (an edit made elsewhere) ends it. After a join, later entries' depths and `talk_typed`'s redo depths shift down n-1, and the run's typed lines merge into one undo entry at the joined depth (`TypedState::joined(n, top)`; no Edit Areas Together page has a localized field today, and a typed edit's rewritten command would not match its mark anyway, so it would end the run, not be taken). Marks are cleared at every frame's start in `tool_sections::follow` (not only while the section is drawn); `held_mark` survives only while `held_text` holds that command. A focused field counts as typing only once its buffer differs from its text when it took the keys (`AreaChooser::typed_from`). `Tools::history_only` keeps Cancel All's Undo off a side editor (`local_undo`) when the pointer rests on a hak/2DA editor. Escape stays Close (dialog pattern: Escape presses what leaves).
## Watch out
- `PageScroll` brings a page whenever the rail index differs from its anchor, so a "rail brings nothing" break passed until the test clicked the page already at the top.
- Page group bounds from AccessKit are clipped at the scroll's top: measure a page's bottom, not its top, after scrolling into it.
- A script field (`resref_field`) does not use `held_text`; "typing" is detected as a focused id with an `app.buffers` entry inside the page whose text changed since focus.
- Fix round (review A): `joined`, the two disabled reasons and the frame-start mark clearing had no guard; tests now: `cancel_all_keeps_a_typed_line_undone_above_it_for_redo`, `cancel_all_says_which_of_its_reasons_keeps_it_disabled`, `cancel_all_waits_for_a_field_with_the_keys_to_be_typed_in`, lib `the_section_s_marks_go_with_the_frame_shown_or_not`, `steps_made_one_keep_their_typed_lines_and_redo_lands_lower`.
- kittest `click()` moves the pointer; use `click_accesskit()` to press a button with the pointer resting elsewhere.
- Another task's run in the shared `target/test-output` emptied scratch modules mid-test: set `MOONGLOW_TEST_OUTPUT`.
