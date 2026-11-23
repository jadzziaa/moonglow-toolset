---
type: Decision
title: "Status Bar beside a live error: what gives way, in what order (5.2.0 review ruling)"
description: "How shell::status_bar lays out a script's live error: shown whole where room allows, others yielding in a fixed order, and what the module name keeps."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T17:32:33Z }
touches: ["crates/mg-ui/src/shell.rs:status_bar", "crates/mg-ui/src/shell.rs:ERROR_LEAVES"]
asks:
  - Why do the hints disappear when a script has an error?
  - What gives way on the Status Bar to the live error?
  - Why is the module name cut short beside a script error?
  - Why are only six or seven script hints shown at 1600?
fingerprint: v2-c22b09da7ac328c0
created: 2026-11-23
author: augustaugur
---
Owner's ruling at the 5.2.0 W5 review (the first build removed every hint and count for any error, even a short one; without a game install every script has FILE NOT FOUND for good, so its hints never showed).

- The error (a `Note`) has no cap and is shown whole wherever the bar has room. The others give way only as far as its width needs, in this order (the coordinator's second ruling: a wider window must never show fewer hints): the game folder first (the no-error rule `full <= room`: only where the middle at its fullest, hints and whole error, fits beside it), then the hints from the last (they stay in the middle and take what the error leaves, `measured.fitting`), then the switches' words (the icons rule `switches + bare > available`), then the module's name (and a note or job) cut down to `ERROR_LEAVES` (120 points given; its text, cut at a letter with an ellipsis, measures a few less, ~117), then the counts from the first (`counts.drain(..n - kept)`): with the Resources panel in use its count is inserted at index 0 (shell::shown), so dropping from the end would lose the cursor's "Line, column" first; it goes last, since builders read it against the error's line (script_bottom::the_cursor_s_count_goes_last_beside_the_resources_count). Only then is the error cut, the whole in its tip.
- Error branch: `if error_line && !show_folder` in status_bar, separate from the non-error path (which drops the middle before counts and keeps the old rule).
- Measured against HEAD (a `git archive` copy built apart under target/v52/W5/head): `status_hints::a_script_tab_shows_its_keys` at 1600, no game: HEAD drew all 8 hints with and without the error; now 7 with FILE NOT FOUND (the first build kept the folder "No game install" over the hints and drew 6, fewer than at 1280; ruled against) and 7 without (Replace gives way to Help's new switch and the log line at its 170 cap). At 1280: HEAD 8 and 8, now 7 and 7.
- Shortfall for the 8th hint at 1600 with no error (measured): Compile all ends at 1375.9, the count starts at 1433.8; Replace needs about 93 with its gap, so about 40 points short. Help's switch is 58 wide (+8 gap); the log line takes its 170 cap. Ruled: 7 of 8 accepted; LAST_LINE_CAP_SCRIPT stays 170 and Help's switch keeps its word (the command is in the menu).
- Tests: script_bottom::the_live_error_takes_the_status_bar_s_room, a_long_module_name_leaves_the_error_its_room (2400, 1100, 700, 500 with a 54-character module name).
