---
title: "Deferred round: plugins beyond API 0.4 (package D)"
author: August
date: 2026-11-02
branch: worktree-agent-af3de424d9cf0e960
cost: { minutes: 2777, tokens: 19272198 }
touches:
  - docs/deferred.md
  - docs/log.md
  - docs/plugin-proposal.md
  - apps/mg/src/plugins.rs
  - apps/mg/tests/plugins.rs
  - crates/mg-plugin/src/lib.rs
  - crates/mg-plugin/src/testing.rs
  - crates/mg-plugin/tests/plugins.rs
  - crates/mg-ui/src/nui_view.rs
  - crates/mg-ui/src/plugins.rs
---

## Decided
- Plugin API 0.5 (`mg_plugin::API`): area:batch, setting kinds float/list/color/pattern/pick, `for` = `placed X` / `X blueprint` / `tile` / a resource type (`Target::{Placed,Blueprint,Tile,Resource}`, `Target::takes`; a placed object's `Selected::target()` is `Placed`, a blueprint's `Blueprint`), `ctx.open.together`, `ctx.fix(label, { command })` (mg_edit::fix::Asks). Each 0.5 addition is refused under an earlier `api` in `Manifest::parse`.
- Workspace steps have process-wide numbers (`last_step`, `next_redo_step`, `has_step`, `step_done`, `join_last`): a plugin's hak (`HakWritten`, links in the hak folder's hidden `.moonglow-undo-<pid>`) and Verify's fixes (`Guard::made` returns `Made`, `Guard::unmade`) follow their step through Undo/Redo (`plugin_haks_follow`, `fixes_follow` in Action::Undo).
- Trust: a yes holds for the same digest in any project (`Answers::trust_in`); a no is the project's alone. User-scope settings live in `plugin-settings.json` in the data folder (`settings::user_path`), shared by mg and the window (`Moonglow::plugin_values`, None in tests).
- `mg verify --fix UNKNOWN` fails: known = doctor `Check::ALL` ids + declared plugin checks (`plugins::findings(.., &mut declared)`).
- mg's saving commands ask the project's plugins through `save_asked` (main.rs); `--strict` on set/apply.

## Watch out
- A log line's button out of sight in kittest takes no click: `scroll_to_me()` first.
- `git add -A` brings back `.trail/cache/` files the owner untracked: `git rm --cached -r .trail/cache` before committing.
- Raw strings with `"#` inside (a color) need `r##"…"##`.
- A front tab for `plugin_open` needs the dock's focused leaf: set it with `set_focused_node_and_surface` in a test.
