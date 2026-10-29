---
title: "NUI asks left: row states, variants, bind rules, property tables, Creator import, whole pixels"
author: August
date: 2026-10-29
branch: worktree-agent-a1e955be3e9ff0af1
cost: { minutes: 2023, tokens: 13972673 }
touches:
  - docs/deferred.md
  - docs/log.md
  - docs/manual/18-nui-creator.md
  - crates/mg-ui/src/nui_view/preview/containers.rs
  - crates/mg-ui/src/nui_view/preview/layout.rs
  - crates/mg-ui/src/nui_view/preview/node.rs
  - crates/mg-ui/src/nui_view/preview/stage.rs
  - crates/mg-ui/tests/nui.rs
  - apps/mg/src/nui.rs
  - apps/mg/src/nui/preview.rs
---

## Decided
- `mg nui render`/`layout` row states: `Options::row_states` keyed `(id, row)` beside `states` (which stays `Option<String>` keyed, every row's); `State::still_row_states` is looked up in `preview/node.rs` before the ID's. The CLI parses `STATE:ID@ROW` with `rsplit_once('@')` and a number after it (an ID may hold an `@`). A row the list doesn't have is found after the frames, from the recorded elements' rows.
- `Options::variants` (a Vec): the `_window_` target's first, then in order, each through `layouts::context_document` on the previous result; two for one target refused (`Error::SameGroup`).
- Binds given are checked by running `mg_nui::validate` on the window before and with them, and saying the findings that are new (the bind found by the finding's path's `bind`, `/views/NAME/...` paths too). Validate allows a number for a bound Text, so a number where a label's text is bound is not said: `layout`'s `text` now gives a bound number's text as drawn (12.5 at precision 0 is "13": ties away from zero).
- While recording (`mg nui layout`/`render`), a list lays out all its rows from 0 (`ROWS_DRAWN` 512), not only those in view; the window's title is recorded (`Assets::record_title`), and a cut recorded with no control current is the title's (`Record::title_cut`).
- Import's gathering lives in `mg_nui::import` (`gather`, `incoming`, `command`); the CLI and the Creator's Import… (`Action::ImportNui`, `app.nui_import` until `widgets::files_already` is answered) both use it.
- `mg_nui::properties` holds the tables (`WINDOW`, `ELEMENT`, `CHART_SLOT`, `DRAW`; `Takes::{Bound, Static, Modifier}`); `validate`'s `Check::table` applies one property; what isn't a property's value (group's one child, the constraint solver, list cells, textedit max's range, warnings) stays code. `jui_schema` is made of the tables. A test sets a wrong value and a bind on every table property of every type and asserts validate's error follows `takes`.
- Whole-pixel scaling: `Renderer::scaled` (floor at the UI scale, then zoom) for natural sizes, a list's row height and cell widths; `Stage`/`Renderer` carry `ui_scale` apart from `scale` (zoom × UI scale). Gaps and margins are not floored (unmeasured).

## Watch out
- In a kittest harness, `files_already`'s buttons (an egui::Window opened from an action) didn't take a click after `run_steps(2)`; `h.run()` before and after the click did. Its layer id is `Id::new(Some(title))`, not `Id::new(title)`.
- Worktree Bash guard: commands with `$VAR` expansions or `cd` + pipes were refused; python heredocs and scratchpad scripts with literal paths worked (`scratchpad/m11/mgrun.sh` runs the debug `mg` under a memory cap).
- `mg()` in `apps/mg/tests/nui.rs` panics on a clap error (no JSON): use `output()` for a refused flag combination.
