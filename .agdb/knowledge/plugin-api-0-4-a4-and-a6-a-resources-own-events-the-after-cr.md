---
type: Decision
title: "Plugin API 0.4 A4 and A6: a resource's own events, the after-create hook"
tags: []
status: stable
generated: { by: trail, at: 2026-10-22T00:00:00Z }
touches: [crates/mg-plugin/src/runtime.rs, crates/mg-plugin/src/manifest.rs, crates/mg-ui/src/plugin_events.rs, crates/mg-ui/src/lib.rs, apps/mg/src/plugins.rs, apps/mg/src/edits.rs, docs/manual/17-plugin-api.md]
fingerprint: v2-5d925a320cac4c27
created: 2026-10-22
author: August
description: "Plugin API 0.4 lets handlers of creation events edit within the user's single Undo command via with_plugins on a scratch Workspace, while save events are read-only and can warn or refuse with a fix."
asks:
  - "Why can't my plugin edit a resource in a resource-saving or resource-saved handler, and what happens if it tries?"
  - "How do I make a plugin block or warn on Save and offer a fix to the user?"
  - "My plugin's edits after object-placed or resource-created should undo together with the user's action; how does the API achieve that?"
  - "Why does mg --json print a failed command's error on stdout instead of stderr, and why do mg tests fail on stderr output?"
---
## Decided
- Edits in events follow "on the user's word": handlers of `object-placed`, `object-pasted`, `resource-created` may edit, and their edits go INTO the user's own command (one Undo); around a save (`resource-saving`, `resource-saved`) a plugin only reads (an edit, or `ctx.settings:set` before a save, fails the handler), and `resource-saving` warns/refuses via `ctx.event:warn/refuse(text, fix?)` with an A3 `ctx.fix`. `module-saved` (0.3) still edits after a save: left as it was.
- Not `ws.amend` after the creation: `mg_ui::plugin_events::with_plugins(app, cmd, &made)` applies the creation to a scratch `Workspace::new(ws.module.clone())`, runs the handlers synchronously on it (each plugin sees the earlier ones' edits), and returns `Command { label, edits: creation ++ plugins }` for `Moonglow::apply` (so claims::gate, derived values, sync_haks and a ProtectedAsk's re-pushed `Action::Apply` all carry the plugins' edits). Creation sites push `Action::ApplyMade(cmd, Made)` (paste_at, append_object(by), create_waypoint, add_to_palette, creature/blueprint wizards, Setup Store's `build` returns its Made); `new_area` and `copy_resource` call `with_plugins` before their own `self.apply`. CheckTags (C5) stays pushed after, so it sees the plugins' retag.
- Save hold: `plugin_events::hold_save` after `claims::hold_save` in `Action::Save`/`SaveThen`. Old values: `Module::as_read` for folder/project modules, `plugins.saved` (module at last save/open) for a `.mod` (as_read is None there). Script editors' unsaved text is overlaid (`ScriptBuffer::bytes()`), not stored, so `store_script_text`/compile-on-save in `save()` are untouched. Window: egui::Modal "plugin-save"; refusals sorted first; a fix applies `fix.command()` then re-pushes `then` (asked again); Save Anyway sets `plugins.save.cleared` (consumed by the next hold).
- A failing/timeout handler before a save is a warning ("could not check the save"), never a refusal. Time budget: `Input.budget` (None for commands), `EVENT_BUDGET` 5 s for at-once kinds; `Shared::out_of_time` checked in the interrupt and in the pcall wrapper (`__cancelled` returns `stopped()`), `failed(plugin, &sh, e)` names it.
- Per-resource kinds call the handler once per resource inside ONE Lua VM; nothing runs when no resource/object matches `types`/`for`. `ctx.event.old/new/changed` are lazy (`__index` on the event table). `ctx.selection` stays empty in events; objects are `ctx.event.objects`.
- CLI: `edits::run(..., guard: Option<&SaveGuard>)` — `mg set`/`mg apply --plugins` (+ project plugins as allowed, notes of unallowed ones dropped); `mg plugin run` passes None. `--select` no longer conflicts with `--event`.
- Only the 0.4 events are refused under an earlier `api`: the 0.3 ones were never held to it (a fixture with `api = "0.1"` and `module-opened` exists).

## Watch out
- With `mg --json`, a failed command's error is `{"error": …}` on stdout, not stderr.
- Area variables live in the GIT's VarTable (the doctor's `area-variables`), not the ARE's.
- `mg` tests' `mg()` helper asserts an empty stderr: notes must stay in the JSON.
- The worktree shell guard refuses heredocs and `sed` with a `$VAR` path; scratchpad Python scripts and `gpu.sh` (exec flock) work.
