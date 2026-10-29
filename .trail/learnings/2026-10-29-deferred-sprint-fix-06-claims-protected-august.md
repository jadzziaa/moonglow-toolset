---
title: "Deferred sprint fix 06: claims, protected areas and a project's tag rules"
author: August
date: 2026-10-29
branch: worktree-agent-af05f4a17cd64c98e
cost: { minutes: 1742, tokens: 13499984 }
touches:
  - apps/mg/src/claims.rs
  - apps/mg/src/main.rs
  - apps/mg/tests/claims.rs
  - crates/mg-module/src/claims.rs
  - crates/mg-module/src/project_file.rs
  - crates/mg-ui/src/builder_code.rs
  - crates/mg-ui/src/claims.rs
  - crates/mg-ui/src/commands.rs
  - crates/mg-ui/tests/app/projects.rs
  - crates/mg-ui/tests/screens.rs
---

## Decided
- The paste's tag index is cached per resource, not by `Workspace::revision()`: the paste itself moves the revision, so a revision-keyed cache misses on every paste. `mg_module::tags::TagCache` keeps each closed area's ARE/GIT tags with the `Arc<[u8]>` the module held (`Module::shared`, compared with `Arc::ptr_eq`; holding the Arc means the address can't be reused); documents an editor holds (`ws.loaded`) are walked each time and their cache entries dropped. It lives on `Moonglow.tag_cache`, a field beside `ws` (disjoint borrows in `tag_clash::check`, `retag_edits`, `store_wizard::store_tag`).
- `unique_in_area` tags in other areas: quiet for "custom" and "unique", said (as a plain clash, `unique: None`) only for "any". Setup Store's `{n}` counts through `tags::taken_in` (area for a Scope::Area pattern, module otherwise); it takes the tag's bytes in the game codepage (`text::encode`), as the index stores them.
- Protected-name pass: `claims::Pass { area, what }`, consumed by `gate`/`asks_first` only for the same `Protected` kind. `end_pass` runs at the top of `claims::windows` (after `rename_window`/`delete_window` that frame): it drops the pass when its Rename/Delete window isn't open AND `app.actions` is empty. The empty-actions guard matters: Delete's button pushes `DeleteResource` and closes `confirm_delete` in the same frame; without it the pass went before the delete ran.
- Areas Not Yours keeps the claims it was made by (`SaveAsk.by`: table, `[claims]` rules, code) and recomputes when they differ; with none left it pushes `then` (the save) and logs, without setting `cleared`.
- Claims Table window: `ProjectFile::claims_listing` is shared with `mg claims list`; `claims::change(app, area, How)` does Claim/Release/TakeOver (Change::Claim + force)/Hand (`Change::Hand { from, to }`, refused unless the holder is `from` or forced).

## Watch out
- In a `claimed_project` harness Module Properties is open: its OK is the last "OK" (Builder Code's OK isn't reachable by `get_all_by_label("OK").last()`); press Enter instead. Its Cancel comes before a dialog drawn later, so `.last()` finds the dialog's.
- egui's `scroll_to_me` animates: a screens picture taken with `run_steps(3)` shows the scrollbar moved and the rows not; `h.run()` first.
- The worktree guard refuses heredocs and python fed with text containing the word "git" (e.g. a variable `mill_git`); write the script with Write and run it.
