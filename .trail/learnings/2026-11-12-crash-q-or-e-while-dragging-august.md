---
title: "Crash: Q or E while dragging a selection (negative wait)"
author: August
date: 2026-11-12
branch: develop
cost: { minutes: 12, tokens: 35746 }
id: l-2a42645f
scope: repo
status: active
src: agent
touches:
  - "crates/mg-ui/src/area_view.rs#key_turns"
  - "crates/mg-ui/src/jobs.rs#wait"
  - crates/mg-ui/tests/app/area_view.rs
---

## Decided

- The 4.1.0 Windows crash put down to moving a placeable across raised tiles (the learning "Tile-edge drag crash report raised", 2026-11-12, could not reproduce it) is Q or E while dragging the selection, not the tile heights. The reporter turned the placeable with Q or E while dragging it. Both crash reports say `library\core\src\time.rs:962:23: cannot convert float seconds to Duration: value is negative`, the panic of `Duration::from_secs_f64` given a negative value. The same line and column came out of the local before-run (rustc 1.99.1), on today's tree and on the `v4.1.0` tag.
- The cause was in `key_turns` (`crates/mg-ui/src/area_view.rs`; lines 7174 to 7190 before the fix, 7027 to 7043 at `v4.1.0`). `rest = KEY_TURN_REST - (now - k.at)` (0.6 s), and `dragging` held the commit back while the selection was dragged (Move, Lift or Slide). The else branch then asked for `request_repaint_after(from_secs_f64(rest))`, and that wait went negative 0.6 s after the last turn key.
- The fix: while dragged, `key_turns` asks for no frame. The release arm of the drag (`Some(drag)` in the drag handling) already folds `view.key_turn` into the drag's moves and clears it in the release event's own frame. That makes one command (Move or Raise), one Undo step, also when the pointer never moves after the release.
- Escape during the drag drops the drag and the turn, also with the pointer led off the view. Before the fix, `escaped` (which needs `response.hovered()`) missed that case, and the turn became a Rotate of its own once the drag was gone. The fix adds `dragging && view.held.cancelled()`.
- E before the drag stays as it was: the press that begins the drag commits the turn (`other`), so you get two steps, Rotate then Move.
- One helper for waits worked out from the time: `jobs::wait(secs) -> Option<Duration>` (positive and finite, via `try_from_secs_f64`) and `jobs::repaint_in(ctx, secs)`. These sites use it: `key_turns`, `wheel_scales`, both waits in `viewport` (an overdue animation step still asks for a frame at once), and `open_tabs`.

## Tried and ruled out

- The existing test `q_and_e_turn_what_is_being_dragged` slept 700 ms of real time. kittest's clock is `time + step_dt` per frame and never sees a sleep, so the test passed over the crash for a release. Move the clock with `h.input_mut().time = Some(t)`. `RawInput::take` copies `time`, so the clock stays held there until it is set to `None` again (the `hold_clock` and `free_clock` helpers in `tests/app/area_view.rs`).
- Key-then-drag cannot panic: the press commits the turn.

## Watch out

- A Windows crash report from a release has an unsymbolized backtrace. The panic message and its `library\core\...:line:col` are the lead: grep the workspace for what can produce that message, and match the line and column with a local run on the same rustc before trusting a reading.
- A test that needs time to pass (a rest, a fade, a timer) must move the harness's clock, not count frames or sleep.
- `Duration - Duration` and `Instant - Duration` panic on underflow. `Instant - Instant` and `duration_since` saturate on current Rust. egui's `request_repaint_after_secs` is already guarded (`try_from_secs_f32`), but `request_repaint_after(Duration::from_secs_f*(x))` is not.
