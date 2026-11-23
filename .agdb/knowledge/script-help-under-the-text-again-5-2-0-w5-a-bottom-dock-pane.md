---
type: Decision
title: "Script Help under the text again (5.2.0 W5): a Bottom Dock panel, verbatim, brought forward from bottom_forward"
description: "Where a script's Help lives since 5.2.0 and how it is brought forward: Panel::Help in the Bottom Dock group (Compiler, Help, Bookmarks) and a floating window's bottom pane, the comment and declaration verbatim, help_forward run from bottom_forward."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T14:52:51Z }
touches: ["crates/mg-ui/src/shell.rs:help_forward", "crates/mg-ui/src/script_reference.rs:help", "crates/mg-ui/src/shell.rs:SCRIPT_BOTTOM"]
asks:
  - Why is Help in the Bottom Dock and not under Reference?
  - How does Go to Definition bring Help to the front?
  - Why does Help not show prose with a row per parameter any more?
  - Why is help_forward not part of follow_bottom?
fingerprint: v2-03501c862657b1a7
created: 2026-11-23
author: augustaugur
---
The owner's ruling on Jasperre's report (5.2.0) reverses M4 U12's Right Dock Help (see the U12 note for the drawn-before-the-editor design, which still holds): Help is under the text, full width, verbatim, as 4.2.1 had it.

- `Panel::dock`: Help is in the Bottom group; a script's `Brings` has `SCRIPT_RIGHT = [[Reference]]`, `SCRIPT_BOTTOM = [Compiler, Help, Bookmarks]` (Help after Compiler, 4.2.1's order); the old `shares` are gone. Old settings naming `bottom: Some("Help")` now open the dock on Help (valid again); `fronts:["Help"]` for the Right Dock is passed over.
- `script_reference::help` draws `verbatim(symbol)` (doc.trim_end + "\n" + signature) highlighted with the editor's palette, wrap at infinity, in a `ScrollArea::both`. The LayoutJob is cached in `ScriptTools::help_read` keyed by signature, doc and a palette hash, so nothing is re-parsed or re-highlighted per frame (the doc comes from the outline: `Symbol::doc`).
- Bringing forward: `ScriptTools::help_forward` is set by a double click on a name, Definition and (new) a click in Reference's list. `shell::help_forward` runs from `bottom_forward` at frame start: floating, `window_bottoms.insert(tab, Help)` for the asking window; docked, it waits until a Script is the active editor (Definition into nwscript.nss opens a Resource tab first), then `focus_bottom(Help)` and `hide_log = false` (opens a closed dock). Caret moves only change what Help shows.
- Do NOT add the pending ask to `follow_bottom`'s redraw rule: while it waits for a script to be in front it would request a frame every frame (endless redraw).
- Tests: script_header::reference_on_the_right_and_help_under_the_text_in_either_host, help_comes_forward_in_the_bottom_dock; scripts::help_shows_the_comment_and_the_declaration_as_written.
- Reference alone in the docked Right Dock would be drawn as a one-tab bar (no "nwscript and includes" header, no group named "Right Dock"): `right_dock` treats `[Panel::Reference]` as `titled`, as a form's lone Preview, so it keeps its PanelHeader. Tests that looked for the "Right Dock" group beside a script now look by `right_dock_rect()`.
- Help's Status Bar switch takes room: at 1600 beside a script the last hint (Ctrl, R: Replace) gives way with no error (7 of 8), and with the no-game FILE NOT FOUND error 7 (the folder gives way first); status_hints::a_script_tab_shows_its_keys asserts exactly that at 1600 (review round; see the note "Status Bar beside a live error").
