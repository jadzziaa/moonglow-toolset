---
title: "Deferred sprint m05: dialogs, OK and Cancel, Form for module and journal"
author: August
date: 2026-10-29
branch: worktree-agent-a39e1ebac6a52c552
cost: { minutes: 1799, tokens: 12988443 }
touches:
  - crates/mg-ui/src/widgets.rs
  - crates/mg-ui/src/side_doc.rs
  - crates/mg-ui/src/blueprint/mod.rs
  - crates/mg-ui/src/faction_view.rs
  - crates/mg-ui/src/module_props.rs
  - crates/mg-ui/src/journal_view.rs
---

## Decided
- Dialogs are `widgets::modal(ctx, title)` (no close button, answered by its buttons) or `widgets::dialog(ctx, title, &mut open)` (close button and Escape), both on `widgets::centered`: pivot CENTER_CENTER at `ctx.content_rect().center()`. egui keeps an area's pivot position, so a centered window grows about its middle (as the old `anchor` did) and is movable. `widgets::window` (anchored) is gone.
- `egui::Modal` is kept only for what blocks the whole app (jobs, plugin questions, plugin save, Command Palette). A Modal opened over a Properties window did not register with `escape_on`, so `escape_past_dialogs` saw no dialog and the Properties window took the same Escape as its Cancel (seen: Escape in Select Portrait closed the chest's window). Pickers are `modal`s with `cancel`/`cancel_discard`/`escape_here`.
- Files saved when asked (hak, 2DA, tileset, talk table, script) share `side_doc::ok_cancel(app, ui, &tab, read_only, before)`: OK = save (`save_side`) + `forget_side` + CloseTab; Cancel = `Moonglow::may_cancel` (queues the Unsaved Changes question via `side_closing`). `asked_about` covers Tab::TalkTable and Tab::Script without making them `SideDoc`s, so their × still keeps unsaved work for the module's Save.
- `Moonglow::held_text` is a `blueprint::HeldText`; `window_drawn_but` claims it for its window (first window drawn after `hold_text` set it this frame), and the frame's end notes its command for that window's Cancel before `properties_changed.retain`.
- `field_changed` notes the field itself inside struct steps (GIT `AreaProperties/MusicDay`) and the list at the first list step; `follow.rs` remaps a noted path by its last Item step.
- Faction removal's Cancel: `faction_view::CancelSteps` records the factions file after each window step and the removed id; Cancel finds the step the file is at now and numbers users back with `put_back` (inverse maps, last removal first) from the current module, falling back to whole snapshots when the file matches no step.

## Watch out
- The script editor fits its own rect (`fit` from `max_rect` and clip): a bottom panel added before the fit lands off-screen; draw OK/Cancel inside the fit's `scope_builder`.
- A blueprint copy's resref in `blueprint_harness` must be 16 characters or less (`chest_held_cancel` panicked in common.rs).
- With an editor's own Cancel on screen, tests clicking a question's Cancel need `get_all_by_label("Cancel").last()`.
- Module Properties' sample module has the name and tag both "SAMPLE"-like: find a text field by its label's row.
