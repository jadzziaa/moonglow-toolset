---
title: "The Right Dock follows the editor in front, the Preview is a tab, and Quick Open replaces the Command Palette"
author: August
date: 2026-11-12
branch: ui-redesign
cost: { minutes: 121, tokens: 640802 }
id: l-892d60a4
scope: repo
status: active
src: agent
touches:
  - crates/mg-ui/src/browser.rs
  - crates/mg-ui/src/commands.rs
  - crates/mg-ui/src/keys.rs
  - crates/mg-ui/src/lib.rs
  - crates/mg-ui/src/options.rs
  - crates/mg-ui/src/quick_open.rs
  - crates/mg-ui/src/widgets.rs
  - crates/mg-ui/tests/app/quick_open.rs
  - docs/log.md
  - docs/manual/02-main-window.md
---

## Decided
- Palette and Preview are two tabs of the Right Dock's one tab bar, as the approved board draws them. The plan's sentence said "stacked"; a stack was built first and left a creature's 3D view about 20 px tall at the default height, so it was rebuilt as tabs before commit. Check a plan's sentence against the board before briefing it.
- The Right Dock shows the panels of the active docked editor: the front tab of the focused pane of the main surface, never a floating window. An area's tab or the "No area open" placeholder brings Palette and Preview; any other tab brings none, and the dock is then not drawn at all (its fold state and width kept). Tools > Palettes and Tools > Preview still show it from any tab until an area is in front again, so no menu row leads nowhere.
- The Preview draws only while its tab is in front. Its body is drawn in the dock (so Tab reaches it), and late, after the editor area, only when an open tab may show the same model, so that tab keeps its view.
- The Status Bar's last log line is the last non-blank first line of the log's last 64 entries, cut to 300 characters, its tip built only when hovered. As the window narrows the line is cut to its least room, then the game folder goes, then the line, the job's name, the module's name last.
- A new installation starts with the log closed; existing settings keep it open. Reset Layout gives the shell's layout (log closed): the coordinator's decision, reversible on the owner's word.
- Quick Open ranks module resources, then commands, then load-order resources; within a group, resources with a view of their own before those without, then exact, prefix, word start, substring. One exhaustive match over every command says which run while its field has the keys and which wait.
- Quick Open's load-order index is built from every layer except the module's, on a thread, and gives up at once when the game data must be released.

## Tried and ruled out
- Reading the active editor from a pane index kept by the window: it could outlive its module. egui_dock's own focused leaf of the main surface is used.
- Letting a text field keep Undo only once it has been typed in: in Quick Open, Ctrl+Z before any typing undid the module's last change.
- Building the load-order index from the winning copy of each resource: it went stale when another module opened, because which game resources a module overrides differs.
- An unread count or dot on the Log switch, and a stop control in the Status Bar's job item: no board draws them.

## Watch out
- Hide All Panels and "are all panels hidden" must count an absent Right Dock as hidden and leave its fold flag alone.
- Tests that took "the window's first text field" by position skip Quick Open's field; a helper by accessible name is the honest fix when one is touched again.
- The hand-run timing tests need a quiet machine: under a load of 10 or more they fail on varying views; at a load under 3 only the known largest-area views are over budget.
- An agent's scratch break can leave a stale build: restore, touch the sources, and read the diff before the gates.

In the code: `crates/mg-ui/src/shell.rs` (`right_dock`, `Panel::available`, `status_bar`, `last_line`), `lib.rs` (`active_editor`, `palettes_asked`, `panels_hidden`), `area_tools.rs` (`preview_panel`), `model_view.rs` (`a_tab_may_show`, `let_go_of_unseen`), `quick_open.rs` (`runs_from_field`, `game_entries`, `has_own_view`), `settings.rs` (`new_installation`), `keys.rs` (`Keymap::give_way`).
