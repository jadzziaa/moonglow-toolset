---
title: "Plugin API 0.9: panel fields, ctx.values, choice swatches"
author: August
date: 2026-11-09
branch: worktree-agent-a9070131690a28dff
cost: { minutes: 120, tokens: 38347 }
touches:
  - README.md
  - apps/mg/src/plugins.rs
  - crates/mg-plugin/src/kinds.rs
  - crates/mg-plugin/src/lib.rs
  - crates/mg-plugin/src/manifest.rs
  - crates/mg-plugin/src/runtime.rs
  - crates/mg-plugin/src/testing.rs
  - crates/mg-plugin/tests/examples.rs
  - crates/mg-ui/src/commands.rs
  - crates/mg-ui/src/jobs.rs
---

## Decided
- Plugin API 0.9: a panel's field is `{ type = "field", kind = …, id, label, value, … }` (`type` is the element's, so a field's kind is `kind`; `value`, not `default`, which is refused). One parser for both: `runtime::form_field(table, kind, start_key)`; `form_fields` calls it with `type`/`default`, `kinds::elements` with `kind`/`value`.
- `FieldKind::Choice.choices` is `Vec<mg_plugin::Choice { text, color }>` (`From<&str>`); the value is still the text.
- `ctx.values` is always a table (empty unless a panel ran the command): `Input.values`, `Action::RunPluginCommand { values }`, `Moonglow::run_plugin_command_with`. `api_names` lists it by hand (an empty table yields no names).
- The window keeps a panel field's value in `PanelState.fields` (`FieldValue { given, value, set, on }`); `PanelState::took` takes a new `Shown`: a field takes the handler's value only when it differs from the one given last, never while the user is at it (`on` is Some). `on` is the selection when the user began (picker opened, focus), which the preview and the command use, so a click that closes the picker on the area can't retarget it.
- `plugins::field_widget` draws one field for a form and a panel and says `editing`/`set`; a color's "set" is its popup closing (open state kept in ui data per frame) or a paste while closed.
- Choice swatches: the text is a `LayoutJob` with `leading_space` (accesskit label stays the plain text), and the swatch is painted at `rect.left() + button_padding.x` of the row and of the combo box (egui 0.36 lays the selected text there).
- A plugin job's window: `Job.plugin = Some((heading, name))` set right after `start_job` (only if no other job was running, or it would label that one); `jobs::window` heads it with the title and a transparent `Modal::backdrop_color`. `Job.title` stays `"name: title"` for the log.

## Watch out
- Bash in this worktree refuses heredoc python with escaped quotes and `export X=$PWD/...`; write scratchpad scripts and pass `TMPDIR=/abs/path` inline.
- The not-dimmed test compares pixels of `h.render()` before and with the form (mutation-checked: it fails with the backdrop back).
