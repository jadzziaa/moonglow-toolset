---
title: "Conversation bookmarks across a delete, and the rest of the tree's keys"
author: August
date: 2026-10-26
branch: conversation-tree-keys
cost: { minutes: 863, tokens: 9341710 }
touches:
  - crates/mg-module/src/dialog.rs
  - crates/mg-ui/src/dialog_view.rs
  - crates/mg-ui/tests/app.rs
  - docs/deferred.md
  - docs/log.md
  - docs/manual/06-conversations.md
  - bucket/moonglow.json
  - Cargo.lock
  - Cargo.toml
  - packaging/linux/io.github.moonglow_toolset.Moonglow.metainfo.xml
---

## Decided
- Only `mg_module::dialog::prune` (from `remove`: the editor's Delete and Cut) renumbers lines; add and paste append, move and Alt+Up/Down reorder links only. `remove` now returns a `Renumbering` (old -> new per kind, `inverse()` total over the lines left, `after` counts kept so inverse-of-inverse round-trips).
- Undo and redo are opaque `SetResource` byte swaps, so the editor can't be told what they did. `DialogView` keeps `seen: (workspace revision, fingerprint)` (fingerprint = hash of `g.to_bytes()`, computed only when `ws.revision()` moved) and a capped list of `Change { before, after, renumbered, dropped bookmarks/folds }` edges. Delete records its edge (after = fingerprint of the new doc). `follow` at the top of `editor` BFSes from the last fingerprint to the current one over the edges (forward or undone), so several steps seen at once (hidden tab, two Ctrl+Z) still resolve; an unknown change is recorded as an identity edge so later undo chains go through it. Undo reinserts the bookmarks it dropped at their old places. `to_bytes(read(to_bytes(g))) == to_bytes(g)` holds (unit-tested), which the scheme depends on.
- The selection follows as lines too: `selected_line` (the line the selected row shows) is stored at the end of each frame; after a renumbering the row is found again under the mapped parent. Only when some step really renumbered (identity steps leave the selection alone).
- Tree keys: `tree_step` gained Backspace (to `up`), Key::Plus / Key::Minus (fold, no move). `*` comes only as `Event::Text("*")` (egui has no asterisk Key; NumpadMultiply isn't mapped), letters are read from `Event::Text` (so accented letters and the layout's own characters work), consuming the Text event that the keypad's + and − also produce. Ctrl+Up/Down: `view.scroll` rows, applied with `ui.scroll_with_delta(vec2(0, -rows * pitch))` inside the tree's ScrollArea.
- Folding over the selection: `in_sight` walks the selected row's `up` chain in `shown_rows(g, {})` to the nearest drawn row; called after the tree is drawn whenever `collapsed` grew this frame (covers the − button, keypad −, Collapse All).

## Tried and ruled out
- Gating letters on the pointer over the editor, as the module tree does: not needed, since the area's B/Q/E/G need the area hovered, and the pointer in an area's view takes the keys back anyway. The camera's W/A/S/D/Z/C and digits are `key_down`-based (`held_but`) and work for the last-pointed area wherever the pointer is, so consuming events can't stop them: recorded in deferred.
- Matching a letter against the row's label: an NPC row begins with `[OWNER] - `, so letters would find only player lines; the line's own text is matched.

## Watch out
- After deleting a reply's only child, its row's label gains " [END DIALOGUE]": `get_all_by_label("Who are you?")` then panics (no nodes) rather than counting 0; use `query_by_label`.
- The Bookmarks pane labels rows `"{kind:?} {index}: {line text}"`, without the speaker.
- In kittest, `key_press(Key::H)` sends no Text event; letters for the conversation tree are typed with `h.event(Event::Text(...))`.
