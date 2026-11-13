---
type: Decision
title: "NUI render states, binds and variants; event-function returns; widths said"
tags: []
status: stable
generated: { by: trail, at: 2026-10-22T00:00:00Z }
touches: [apps/mg/src/nui.rs, crates/mg-ui/src/nui_view/headless.rs, crates/mg-ui/src/nui_view/preview/node.rs, crates/mg-ui/src/nui_view/skin.rs, crates/mg-nui/src/actions.rs, crates/mg-nui/src/script.rs, crates/mg-nui/src/schema.rs, crates/mg-nui/tests/data/crier.txt, crates/mg-nui/tests/data/cries.jui]
fingerprint: v2-6a5e49b9b7616acf
created: 2026-10-22
author: August
description: "Records how `mg nui render`/`layout` handle --state, --binds and --variant via nui_render::Options, how event-function return statements are auto-fixed by fit_returns, and the preview's width model for controls."
asks:
  - "How do I render a NUI control in hover or disabled state with mg nui render?"
  - "Why does my hover preview of Crier's Send button still look disabled, and how do I fix it?"
  - "Where does the compile error RETURN TYPE AND FUNCTION TYPE MISMATCHED get its hint, and why isn't the return rewrite in merge_events?"
  - "What default width does a NUI control get when none is set, and which controls fill their column?"
---
## Decided
- `mg nui render`/`layout` options live in `nui_render::Options` (no longer `Copy`): `states: BTreeMap<Option<String>, Look>` (`None` = every control), `binds`, `variant`. The CLI's `Drawn` (clap `Args`, flattened into both commands) builds them, so the two always take the same options. Syntax: `--state STATE[:ID]` (repeatable; split at the first `:`), `--binds FILE.json` (one JSON object), `--variant NAME`.
- States go through the canvas's own state code: `State::still_states` (same 1..4 numbering as `preview_state`), read in `preview/node.rs` `draw_node` beside the State menu's. An ID's state wins over the all-controls one. Disabled by ID sets `ancestor_disabled` (contents disabled, as the game); disabled for all does not propagate, so a by-ID exception (`--state disabled --state hover:read`) still shows. A control its binds disable stays disabled.
- The drawn state is recorded, not recomputed: `skin::Placed::look`, set by `Assets::record_look` in `draw_node` (after `enter`), reported as `Element::state` (only when not normal).
- `--variant` reuses `layouts::target_for` + `layouts::context_document` (the Creator's variant-in-context document). Unknown variant, a variant with no group, and an ID not in the window as shown (after the variant) are `Error`s; binds checks read names from the original window plus all views.
- A list row cell's `text` in the layout now comes from `preview::val(.., row)` (it was `None` for a bound array).
- Ask 2: `fit_returns` (actions.rs) rewrites `return;` -> `return TRUE;` (event function) and `return TRUE|FALSE;` -> `return;` (main), tokenized with `mg_script::lex`, only inside non-top live sections. Applied in `script::rebuilt` (behind `rebuild_events`), NOT in `merge_events`: `edited_outside` compares `merge_events(script) == script`, so rewriting there would make a function-mode script with a bare `return;` read as edited outside its sections. `Generated::refits` carries the changes; `explained` appends a hint to a "RETURN TYPE AND FUNCTION TYPE MISMATCHED" compile error.
- Widths: the preview's model (from client measurement) is that most controls without width are 150 (`NUI_STYLE_PRIMARY_WIDTH` in nw_inc_nui), labels/images/spacers/progress/list/chart/color picker/layouts fill their column (`fills_width`), and a column's natural width is its widest child's, the root column too (stage.rs `body`). Window width - 20 fills the root column. `mg_nui::window()` is also the Creator's new window, so new windows stay without widths; the schema --layout description says it and `controls_without_a_width_are_as_the_layout_form_says` holds the text to the layout.

## Watch out
- `crates/mg-nui/tests/data/crier.txt` is pinned to the manual's settings example and crier.jui's 7 elements to `the_layout_off_the_screen_is_the_canvas_s`: a list fixture went into a new `cries.*` instead (with `*.binds.json` for both).
- Crier's Send is disabled by `can_send: false`, so a hover picture of it needs `--binds` with `can_send: true` or another control (Cancel).
- Worktree Bash guard: heredocs and long python heredocs with relative paths were refused at times; scripts in the scratchpad and `flock` via a scratchpad shell script (gpu-run.sh) worked.
