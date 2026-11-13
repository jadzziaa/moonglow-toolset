---
type: Decision
title: Windows with OK and Cancel at their foot fit their pane (issue 38)
tags: []
status: stable
generated: { by: trail, at: 2026-10-30T00:00:00Z }
touches: [crates/mg-ui/src/tabs.rs, crates/mg-ui/tests/app/windows.rs, crates/mg-ui/tests/screens.rs]
fingerprint: v2-b706f5973a83c737
created: 2026-10-30
author: August
description: "At 125% display scale egui_dock's tab ScrollArea overflows by a pixel sliver, making scroll bars flicker and OK/Cancel feet jump; fixed by making foot-pinning windows return [false, false] from scroll_bars and scroll internally."
asks:
  - "Why do the scroll bars flicker and the OK/Cancel buttons jump about 10 points at 125% display scale?"
  - "How do I stop a window with a bottom panel from fighting egui_dock's tab scroll area?"
  - "Which windows were switched to fit their pane in TabViewer::scroll_bars, and which still use the pane's bars?"
  - "Why does my UI test report constant repainting from scroll_area.rs, and how do I catch this bug in tests?"
---
## Decided
- egui_dock wraps every tab in `ScrollArea::new(TabViewer::scroll_bars(tab))` and stretches the content to that area's rect rounded to pixels (plus its frame): at a display scale of 125% the content comes out a sliver past the area at some window positions and sizes, the scroll bars come and go every frame, and a window's foot (OK and Cancel, a `Panel::bottom`) jumps ~10 points with the horizontal bar (GitHub issue 38). 100%, 150%, 175% and 200% don't do it.
- The fix is in `scroll_bars`: a window that pins a foot and scrolls its own page fits its pane (`[false, false]`), as Script, NUI, Manual and Options did already: Module, Area(s), Blueprint(s), Instance(s), Gff Properties, Journal, Dialog, TalkTable. Their page and list scroll areas became `both()` so a narrow window still scrolls sideways; the talk table's Find row became `horizontal_wrapped`.
- Factions, Hak, Table (2DA) and Tileset didn't move in the sweep and keep the pane's bars.

## Tried and ruled out
- Flooring the tab's `set_min_size(available)` to whole pixels in `TabViewer::ui`: no change. The overflow is inside egui_dock's scroll area (egui's `content_max_rect.round_to_pixels` plus egui_dock's `expand_to_include_rect`), which Moonglow can't reach.

## Watch out
- A UI test at one scale won't see it: `a_properties_window_s_buttons_stay_put` runs scales 1.0/1.25/1.5 at sizes that moved (from a 504-size sweep). The symptom in a harness is "the UI keeps repainting" with `scroll_area.rs` among the repaint causes (`show_scroll_this_frame != state.show_scroll`).
- In a fitted window, a non-wrapping row past the window's edge widens the max rect for the wrapped rows after it (they then wrap too late): make the first wide row wrap too. `screens.rs`'s `narrow()` shows a window at 420 points wide.
- A new window with a foot added later needs the same: add it to `scroll_bars` and to the sweep's list.
