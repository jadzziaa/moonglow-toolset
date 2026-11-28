---
type: Proposal
title: 'Floating windows minimized to the Status Bar: a proposal'
description: A Minimize button on a floating window's bar that hides the window and leaves a chip for it in the Status Bar (its icon and title, after the hints and before the counts), a click on the chip restoring it where it was - where the chips sit and how they give way to the hints, an error line and a narrow window, what a chip shows, restore, close, opening a resource of a minimized window, the session and Reset Layout, Collapse kept, no Aurora parity, the mockup, the owner's three choices and the size of the work. Not built.
tags: [proposal, ui, floating-windows, status-bar]
status: draft
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-28T08:37:49Z }
stale_after: 2027-03-01T00:00:00Z
sources:
  - id: request
    resource: a builder's remark on 5.5.0, passed on by the owner (2026-11-28), with a screenshot of the Status Bar's empty middle outlined
    title: "Could there be a 'minimise' option on pop-out windows … that minimises them to a 'Tab' in the bar at the bottom of Moonglow's UI?"
  - id: status-bar
    resource: crates/mg-ui/src/shell.rs (status_bar), crates/mg-uikit/src/status.rs (StatusSwitch), docs/manual/02-main-window.md (The Status Bar, Floating windows)
    title: The Status Bar's parts and how they give way; the floating windows' bar
  - id: inventory
    resource: docs/parity/aurora-ui-inventory.md
    title: Aurora's interface inventory (no minimize of its windows listed)
  - id: mockup
    resource: crates/mg-ui/tests/screens.rs (proposal_minimized_windows)
    title: The rendered mockup, before and after
---

# Floating windows minimized to the Status Bar: a proposal

A builder asked for a **Minimize** on pop-out windows (a script, a
creature's Properties…) that turns the window into a tab in the empty
middle of the Status Bar, a click on the tab bringing it back.
**Collapse** stays: it folds a window to its bar where it stands, which
is right for a glance; Minimize takes it out of the way.

## What it does

- **The button.** On a floating window's bar, left of Maximize: **Minimize**
  (the kit's `minus` icon, tooltip "Minimize the window to the Status Bar").
  Docked tabs have none: they are already out of the way. A model's view
  window has it too.
- **The chip.** The window leaves the screen and the Status Bar gets a
  chip for it: the icon and title of the window's tab in front, as its
  tab shows them, with the unsaved dot where any of its tabs has changes
  and a count where the window holds several tabs (scripts gather in one
  window). Its tip names every tab in it. Drawn as the bar's switches
  are (`StatusSwitch`), off; a screen reader reads "Restore *title*".
- **Restore.** A click on a chip puts the window back at its place and
  size, in front, with the keys. So does opening any resource whose tab
  is in a minimized window (the tree, Quick Open, Go to Definition), and
  opening a script while the scripts' window is minimized (the script
  joins it, and it comes back). A chip's right-click menu: **Restore**,
  **Close Window** (which asks what closing the window asks).
- **The session.** A minimized window is kept with the module's layout and
  opens minimized again with the module. **View › Reset Layout** restores
  them all. Closing the module closes them as it closes the others.

## Where the chips sit

At the right of the bar's middle stretch: after the hints, before the
counts, the module's name and the game's folder (the mockup). The hints
keep their place by the switches, where the eye finds them while working
in an area; the chips stay put while the hints change.

As the room runs short they give way in this order, after the rules the
bar has now (the folder first, the hints from the last):

1. the hints, from the last, down to none;
2. the chips' words: icons alone, each title in its tip;
3. the chips beyond the room: folded into one last chip, **+3**, whose
   menu lists them.

A script's error line still takes the room it needs before anything
else, as today; the chips fold to **+N** before it is cut.

## Aurora

None: the inventory lists no minimize of Aurora's windows. This is
Moonglow's own, after a desktop's taskbar.

## The mockup

`proposal_minimized_windows` in `crates/mg-ui/tests/screens.rs`
(Chapter 1's `map_m1q1a` at 1600 by 900, its hints showing): **before**,
a script and a creature's Properties floating over the area; **after**,
both minimized, two chips at the end of the hints' room. The chips are
painted over the bar from the test; the feature is not built. The test
goes when it is.

## The owner's choices

1. **Where**: after the hints, before the counts (recommended: the hints
   stay where they are), or straight after the switches, where the
   builder's outline begins (the hints move right).
2. **The session**: kept minimized at the module's next opening
   (recommended), or every window restored.
3. **Folding**: icons alone before **+N** (recommended), or straight to
   **+N**.

## The work

About two days: the window bar's button in the vendored egui_dock (a
third `WindowBarButton`, drawn by `paint_window_bar_button`), a hidden
state per window kept with the module's layout, the chips and their
yielding in `status_bar`, restore on open, the manual (The main window:
Floating windows, The Status Bar) and tests (minimize and restore by
click, by opening a resource, by a new script; the session; the
yielding at three widths; the menu's Close asking).
