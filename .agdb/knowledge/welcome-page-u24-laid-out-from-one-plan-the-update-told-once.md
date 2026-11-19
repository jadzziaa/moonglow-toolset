---
type: Gotcha
title: "Welcome page (U24 and its review): one plan, the update told once by the last pass, rows out of sight taken for the keys"
description: "How the U24 Welcome page is laid out and drawn, when the update strip shows over it (scrolled away, under a window, the first frame), why rows out of sight are still registered, and the test pitfalls (labels on screen with no module, out-of-sight clicks, the dock empty while tabs draw)."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-19T11:04:17Z }
verified:
  - { by: agdb/0.2.0, at: 2026-11-19T11:46:06Z }
  - { by: agdb/0.2.0, at: 2026-11-19T12:25:50Z }
touches: [crates/mg-ui/src/welcome.rs, "crates/mg-ui/src/updates.rs:line", "crates/mg-ui/src/updates.rs:notice_bar", crates/mg-ui/tests/app/welcome.rs]
asks:
  - Why does the update strip show over the Welcome page?
  - "Why does a test find two nodes named Plugins, Help or User Manual with no module open?"
  - Why does a kittest click on a Welcome page button do nothing?
  - How does Tab reach recent rows that are out of sight on the Welcome page?
fingerprint: v2-0c7afab5504c5605
created: 2026-11-19
author: augustaugur
---
## What was built
- `welcome::plan` lays the page out analytically (a list of parts with rects: three columns at shares 1.45:1:1, two, or one under 560), then `welcome()` draws it: centered where it fits, else the cursor is advanced and the tab's own `ScrollArea::both` scrolls it. The Welcome tab is not in `fits`.
- Recent rows and Learn's links out of sight are not laid out or painted, but each is still registered with a cheap `ui.interact` (ids by list index), so Tab goes through all of them in order; a row that gains focus calls `scroll_to_me`. Skipping them entirely (the first U24) made Tab jump from the last visible row to New Module….
- `updates::line` is the one reading of the notice's state, for the page's Status and for the strip (`notice_bar` adds Don't Remind Me and Close). It is None only while the Check for Updates window tells of the same release (the strip's rule). Returning None whenever the window was open made the page jump when Check for Updates… was clicked on it.
- The strip hides only while `welcome::in_sight`: Welcome in front of its leaf AND the line was wholly inside the clip, under no `Order::Middle` window but a dialog (`under_a_window`, from the areas' last rects; `widgets::is_dialog` excluded, since About over the line brought the strip in and moved the page), at the LAST PASS (`Kept::told` stamped with `ctx.cumulative_pass_nr()`). A stale stamp (Welcome not drawn in front last pass, the first frame) counts as in sight. Where the page finds the strip assumed wrongly, it calls `ctx.request_discard`, so no frame shows both or neither.
- The cached work is counted (`Moonglow::welcome_counts`, with the page's discarded passes): area walks keyed by `Workspace::revision`, chapter titles once, `why_not` keyed by release and setup, a dragged folder once per drag (`Kept::drag`).
- No row, link or button acts by the keys while `shell::keys_blocked` (About, a dialog, a menu): Enter on a focused row under About used to open the module. A pointer click beside a dialog still acts (`pointer.any_click()`).

## Watch out
- `app.dock` is empty while the tabs draw: inside `welcome()` do not call anything that asks the dock whether Welcome is in front (the first try at the pass rule did, saw "not in front" in every pass, and discarded every pass).
- kittest's `click()` is a pointer click at the node's center: a button scrolled out of the tab's clip (or under the log panel) is not hit. Use `click_in_sight`. Turn the wheel over the page (the "Moonglow Toolset" heading), not over a node under the log panel.
- With no module open the page puts Plugins, Options, Troubleshooting, Getting started, User Manual, About Moonglow Toolset, Check for Updates… and the heading Help on screen: an unscoped `get_by_label` finds two. The Help menu: `get_by_label("Top Bar").get_by_label("Help")`; chapter and Help links are `Role::Link`, headings `Role::Heading`; the Help menu's User Manual is labeled with its key ("User Manual F1"), the page's is not. egui has no menu-item role.
- Headings set to `Role::Heading` lose the Label role's text as their name: set the label too (`heading_node`).
- A break script that reverts by string replace fails silently where the text occurs twice: count first, restore from a copy.
- kittest keeps `raw.hovered_files` from step to step: pushing a hovered file every step makes the list grow (1, 2, 3...), and the drop overlay reads no folder when more than one path is hovered. Push it once.
- A wrong first-pass assumption about the line is invisible (the discard redraws before anything is shown) and the harness's first `step()` already runs several passes, so pass numbers prove nothing: the page counts its own discards (`welcome_counts()[4]`, `Counted::Discard`) and the first-frame test asserts none.
- The half-in-sight rule cannot be tested at the page's bottom edge: the strip's going gives the page 27 points, more than the line's 15, so a line told half in sight is wholly in sight in the redrawn pass. Test it where the line leaves at the top, under the tabs (sixty recent modules make the page tall enough).
- Do not call `ctx.data` (or anything that locks the context, such as `widgets::is_dialog`) inside a `ctx.memory(|m| ...)` closure: collect the layers first, then ask (`under_a_window`).
