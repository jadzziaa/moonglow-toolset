---
title: "Client readings: picture lag, NUI tags, clicks, CD key"
author: August
date: 2026-10-30
branch: worktree-agent-af9107fde14ba9c60
cost: { minutes: 1670, tokens: 11168408 }
touches:
  - crates/mg-corpus-tests/tests/client_render/nui.rs
  - crates/mg-testkit/src/joined.rs
  - tools/nwclient/run-server-and-client.sh
---

Client readings, the second batch (2026-10-29): what costs time with the sandboxed client.

- The client's window picture (`xdrive.py winshot`) runs seconds behind the client even with the frame limiter (FRAMES in client_nui.rs), and the lag grows over a long run. For a sequence of NUI windows, tag each window with a color of its own (a 6×6 draw-list square on its root, `nui.rs`'s `tag`) and attribute every picture by its tag, polling continuously; don't trust the log line saying which window is open.
- A draw list clipped to its control (`draw_list_scissor` true) fills the control's cell, margins included, and on a button with an explicit width or height draws nothing above, or the rest of the column below. To find where a control starts, draw a small unclipped square at its (0, 0) instead (`nui.rs`'s `marked`).
- Clicks: a bare `xdrive.py click` is not taken by the client; move the pointer near, then onto the spot, then press and release (`nui.rs`'s `click_in_window`).
- Each client started on the off-screen display cascades 28 px further down; after a dozen runs its window leaves the screen and `winshot` fails. Restart the display (`headless.sh stop` / `start`).
- "No space left on device" from Pillow means the disk filled for a moment (other worktrees building): the client's pictures fail with it.
- Multiplayer needs a CD key, which the sandboxed client (no Steam) has none of: Multiplayer asks "Enter CD-KEY" and `+connect` ends in "Your CD Key is unauthorized". `mg_testkit::joined` takes one from `MG_CLIENT_CDKEY` (the owner's to give), and sets the server's `masterserver.key-authentication` to `if-reachable` (its master server is out of reach in the sandbox). Never make a key up or copy the owner's without asking.
- The readings' `moonglow_beside` draws the emitters' particles (Animations on is `View::of`'s default), run ahead by `AreaScene::particles`.
