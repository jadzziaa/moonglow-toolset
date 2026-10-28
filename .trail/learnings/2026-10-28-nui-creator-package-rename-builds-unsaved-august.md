---
title: "NUI Creator package: rename builds, unsaved scripts, names, hak editor pictures"
author: August
date: 2026-10-28
branch: worktree-agent-a32f462b9bae812d0
cost: { minutes: 1449, tokens: 12309279 }
touches:
  - crates/mg-nui/src/rename.rs
  - crates/mg-ui/src/references.rs
  - crates/mg-ui/src/nui_view/events.rs
  - crates/mg-ui/src/nui_view/images.rs
  - crates/mg-nui/src/lib.rs
  - crates/mg-ui/src/nui_view.rs
---

## Decided
- A NUI window renamed is built again in the same command: `mg_nui::build_renamed` applies the rename's edits to a copy, runs `generate` on it and replaces the rename's command with `mg_edit::edits_between(module, staged)`, so one Undo takes both back. Its `Err` leaves the rename alone (the caller says why); `Renamed::build` false means nothing to build (`Ok(None)`). The Creator's Rename… and `mg rename NAME.jui` both use it; `nui_view::after_build` is Build's tail (refits said, compile memory kept, failed scripts said), shared with the rename.
- Code the Creator writes into an event script whose editor is dirty (`events::write_script`): the typed text is stored first as its own "Edit NAME_e.nss" command, then the buffer takes the new text and `mark_saved()` gives the edit's bytes (the file's line ends and codepage), so the editor and the module agree. Delete… does the same for scripts going with what it deletes (`unsaved_with`).
- Settings' fields renamed in code only: `#[serde(rename = "actions" | "views" | "view_targets")]`, and `Action::Variant { variant }` still reads and writes `"action":"view"` with `"view"`.
- A hak open in the hak editor takes Images' pictures in its document (`add_sources_as`): saved at once with `hak_view::save` (a `.bak` copied first) when it had no unsaved changes, else left dirty with a warning. `Hak::save` clears the hak's own Undo, so only the unsaved case keeps an "Add pictures" step.

## Watch out
- A press and release in one kittest frame on a "Select it" link did not lose a field's typed text even before `State::select_next`: the race the deferred list described could not be shown; the link now selects as the next frame starts anyway, and `text_typed_is_set_when_a_problem_s_link_selects_another_control` sends both events in one frame.
- Keys chosen in a test's `settings.key_bindings` must not be another command's default: F7 is Compile All and F9 Test Module, which take the key first; letters (H, L) are free in the NUI Creator.
- An Edit inserted after an `assert` that looked like a test's end landed in the middle of a function: check the next lines before inserting at what looks like a file's end.
- Hint text reaches the accessibility tree as `placeholder()` (egui sets it from `hint_text`), which a kittest test can read.
- The renamed Settings fields broke `.actions` across mg-ui, where `app.actions` is the action queue: the fields were renamed from the compiler's errors (E0560, E0609, shorthand fields), not by text.
