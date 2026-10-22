---
title: "Plugin API 0.4 A2 and A5: a project's own plugins, plugin settings"
author: August
date: 2026-10-22
branch: plugin-api-a2-a5
touches:
  - crates/mg-plugin/src/project.rs
  - crates/mg-plugin/src/settings.rs
  - crates/mg-module/src/project_plugins.rs
  - crates/mg-ui/src/plugins.rs
  - apps/mg/src/plugins.rs
---

## Decided
- A project's plugins are allowed by content, not version: `Plugin::load_project` reads the folder whole (the same walk as `pack`: no hidden files, no top-level `tests/`, archive limits) into `Plugin.frozen` (`Frozen { digest: "sha256:…", files }`, SHA-256 in `mg_core::sha256`) and runs from that copy (`runtime::source_of` → `Source::Memory(Arc<…>)`), so what was allowed is what runs even if a pull changes the folder while open. `mg_plugin::project::{discover, trust, decide}`; `Trust::{Unasked, Allowed, Changed{was}, Denied}`; a no is sticky by id (no re-asking when it changes).
- Answers live in `.moonglow/user.toml` `[project_plugins]` as inline tables `{ allow, version, digest }`; team settings in `moonglow.toml` `[plugins."<id>"]`, user-project ones in user.toml `[plugins."<id>"]`, `[plugins] folder` (team only, inside the project). Parsing in `crates/mg-module/src/project_plugins.rs` (project_file.rs only gained the `plugins` field and two match arms, for the parallel [claims]/[commands] merge). Unquoted dotted ids nest: a table inside a plugin's table continues the id (setting values are never tables). Writing uses `toml_edit` (new workspace dep), keeping the rest of the file; an emptied table goes unless a comment sits over it.
- Settings: manifest `[setting]` (id `[a-z][a-z0-9_]*`, not `set`; type string/int/bool/choice/path; scope user/project/user-project; refused under api < 0.4). `Input.settings: Option<Settings>` (resolved by `settings::resolve`, which also returns the warnings: wrong kind, wrong file, undeclared); `ctx.settings` is a proxy table (`__index` with `set`, `__newindex` error, `__iter` in manifest order, protected metatable); writes come back in `Outcome.settings` and the host keeps them (UI: `keep_plugin_settings` after the job; user scope in `Settings.plugin_settings`; mg: project files, user scope noted as not kept). A check's `set` fails (`Shared.no_set`).
- UI: `Installed.project`; `enabled_plugins()` = installed-enabled not shadowed + allowed project ones; `load_project_plugins` in `use_module` before `plugins_module_opened` and in `close()`; allowing queues that plugin's module-opened event. Prompt `project_window` (Modal, Allow / Don't Allow / Decide Later), form `settings_window`. Grants of a project plugin are kept under `"id sha256:…"`.
- mg: `mg verify --allow-project-plugins` (else only allowed ones run, each skipped one noted); `mg plugin list --project`; `mg plugin run --setting ID=VALUE`. A plugin test's `settings`/`set` keys.

## Watch out
- kittest `get_by_label` panics on repeated labels (the form's scope column): use `get_all_by_label(..).count()`.
- In this worktree the shell guard refused a for-loop command that also contained `git status`; run git alone.
