---
type: Decision
title: "Read-only viewers and one shared preview (M5 W6, rulings 49 and 19)"
description: "How read-only tabs show pictures, sounds and models through the hak Preview's viewers, and how two places showing one model share one render a pass."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-21T18:25:58Z }
touches: [crates/mg-ui/src/viewers.rs, crates/mg-ui/src/model_view.rs, crates/mg-ui/src/browser.rs]
asks:
  - why does the second preview of a model not render it again
  - where is the picture and sound viewer shared by the hak Preview and read-only tabs
  - who decides which sound plays on the Preview channel
  - saved versus edited blueprint in two previews
fingerprint: v2-4eda0ff21e2c3d0e
created: 2026-11-21
author: augustaugur
---
## Decided (owner's rulings of 2026-11-21, built in M5 W6)

- **Ruling 49:** `viewers.rs` is the one code path for pictures (TGA, DDS, PLT) and sounds (WAV, BMU). The hak Preview (`hak_view`) and read-only tabs (`browser.rs` `Content::Picture`/`Sound`/`Model`) both use it.
  - Pictures are decoded off the frame by the window's `Moonglow::decodes` (`viewers::Decodes`: at most 4 threads, the rest queued), and the texture is made on the frame. `jobs::under_way` reports "a picture being decoded" while `decodes.pending() > 0`. The count goes down when the result is stored (`Done`'s drop, which also runs on a panic or a job dropped unrun), not when the texture is made, so an undrawn picture never holds `settle`. Tests hold decodes with `viewers::hold(&[keys])` and make one panic with `viewers::fail`.
  - A read-only tab's picture is let go after `model_view::UNSEEN_AFTER` unseen (`browser::let_go_of_pictures`). Its time is refreshed under Show Bytes too: without that it was re-read and re-decoded every 2 s (reviewer C, W6-1).
  - One owner decides the sound: `Moonglow::viewer_sound` (`Player::Hak(id)` or `Player::Tab(tab)`). `audio::play_sound`/`stop_sound` on `Channel::Preview` clear it.
  - `viewers::follow` stops the sound when its owner was not drawn last pass, which covers both closed and behind.
  - Before W6, the hak's button went on saying Stop after another sound took the channel.
  - "Show Bytes" in the header's More popover still shows the hex of the first 512 bytes.
- **Ruling 19:** both places already shared one `ModelView` per `Source`, and both read the working copy.
  - A "saved vs edited" pair cannot exist. The brief's premise did not hold; a test shows that an unsaved head edit is seen in the shared view.
  - The second place to draw in a pass (`shown_in == Some(pass)`) calls `model_view::follow`. It paints the existing target texture and steers the camera, and never resizes or renders. `steer` asks for a frame when the camera moved (the leader drew already this pass). It says nothing without a model or a target.
  - The header row stays with the first place to draw: moving it with the pointer would change both rooms and remake and refit the leader's target.
  - `ModelView::renders()`/`followed()` let tests count this.
  - In `frame_perf`, the row "a creature's model tab, the Preview tab sharing its picture" took 0.82 ms.

## Watch out

- An animating model never settles: in `screens.rs`, use `run_steps(10)` for it.
- A read-only test needs `Mode::Tabs.settings()`, or the resource opens floating and is never "behind".
- An NCS tab has no More popover.
