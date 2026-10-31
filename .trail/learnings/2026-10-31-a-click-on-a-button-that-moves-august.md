---
title: "A UI test's click on a button that moves a frame later misses it"
author: August
date: 2026-10-31
branch: develop
touches:
  - crates/mg-ui/tests/app/projects.rs
---

## Decided
- `the_nwsync_repository_is_uploaded_part_by_part` failed in three release checks in a row (3.1.1 to 3.1.3), never alone. Its second Upload did nothing at all, with no log line. The cause: the test changed the upload command in the settings, which changes the text above the button; Publish to NWSync's window takes its new size a frame later, so the button moved (y 387 → 372) from under a click made at the rect the test had just read. kittest's `click()` clicks at the node's rect as of the last frame.
- The fix is in the test, via a helper `upload(h)`: `settle`, then `hover()` and a frame (the pointer brought to the button, as a builder's is), then `click()`, then `settle`. Nothing in the app was wrong.

## Tried and ruled out
- `settle` after the click alone (3.1.1), and before it (3.1.2): the window's work wasn't the cause.
- An outside-change job refusing the upload ("waiting for … to finish"): the stand-in did write into the module's watched folder, which only hid the fault. Moving the module to a folder of its own made the failure happen every time, which is how it was found.

## Watch out
- A test that changes what a window shows above a button (a setting, a field) and then clicks that button: hover it and run a frame first, or read its rect after a frame.
- Debugging a click that "does nothing": print the button's rect before and after one `h.run()`; a change means the click went to where it was.
- A watcher loop must not `pgrep -f` a pattern its own command line contains: it matched itself and never ended.
