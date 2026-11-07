---
title: "Area view edits that repeat: show them, commit when they rest"
author: August
date: 2026-11-07
branch: develop
cost: { minutes: 25, tokens: 689092 }
touches:
  - crates/mg-ui/src/area_view.rs
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/src/prefabs.rs
  - crates/mg-ui/tests/app/area_view.rs
  - crates/mg-ui/tests/app/common.rs
  - crates/mg-ui/tests/app/looks.rs
  - crates/mg-ui/tests/frame_perf.rs
  - docs/log.md
  - docs/manual/04-areas.md
  - apps/mg/target/tmp/moonglow-serve/a1098591-0667e805-2/.mg-lock
---

## Decided
- Every command on an area makes `refresh` read the area again (AreaModel::read, Ground::update, AreaScene::update): a few ms in an area of 100-200 objects, growing with the count. Anything that edits repeatedly (a held key, the wheel) is shown first and made ONE command when it rests, as the ring's drag (commits on release) and Ctrl+wheel scaling (`WheelScale`, 0.4 s) do. Q and E now do too (`KeyTurn`, `KEY_TURN_REST` 0.6 s, longer than a held key's repeat delay).
- A pending turn is drawn through `AreaView::dragged()` (returns `key_turned()` when no drag), so the scene and the overlays follow it; `follows` must include it or the kept picture hides it.
- It is committed: when the keys rest; when the selection changes; before any other click or key in the view (`key_turns`, at the start of the input handler); before any `Moonglow::apply` (`commit_key_turns`, applied at once, not as a queued action); before save and before actions that close the module. Committing also poses the view's model in place (`pose`), so G or a drag in the same frame starts from the turned objects. Escape (it counts as "in hand") and Undo drop it.
- Pastes: `pasted_places` gives each copy's place and turn (`paste_turn`, Q/E while pasting); `mg_area::edit::moved` already turns an outline's points and spawn points.
## Tried and ruled out
- Making `refresh` incremental for moves: bigger and riskier; deferring the command fixes the repeat.
## Watch out
- kittest steps frames whether or not a repaint was asked for, so a missing `request_repaint` (the paste's ghost after Ctrl+V) can't be caught by a UI test.
- Tests that press Q/E must wait for the commit: `rest_turn_keys(h, area)` in tests/app/common.rs.
- `frame_perf`'s `Pointer::Keys` presses a key every N frames; "turned by E held" sits beside the drag timing.

## Later the same day: the read itself made incremental
- `Workspace::touched_since(rev)` says what each change touched (key, struct path, label, whole); `changed_at(key)` when a resource last changed. derive/adopt are "unknown": callers fall back.
- `area_view::refresh` asks it before cloning anything: only object-list items touched → `AreaModel::touched_objects` → `place_objects` (placement/tag/lock labels, `PLACED_ONLY`: look kept, no scene work) or `read_objects` + `AreaScene::update_objects`; nothing of this area → nothing. Fallback where the workspace can't tell: compare the GIT with `read_from` (`changed_objects`, ~1 ms per thousand objects). Anything else → full read (`whole_reads` counts them, for tests).
- Creatures are the expensive objects (0.2-2.3 ms each to compose their look); everything else reads in microseconds. `frame_perf refresh_parts` (by hand) times it all; REFRESH_KINDS=1 per kind.
- What remains of a command's frame is the area's redraw (as a drag frame), not reading.
