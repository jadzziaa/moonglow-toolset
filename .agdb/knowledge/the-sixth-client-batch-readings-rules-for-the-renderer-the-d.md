---
type: Decision
title: "The sixth client batch: readings, rules for the renderer, the display incident"
tags: []
status: stable
generated: { by: trail, at: 2026-11-02T00:00:00Z }
touches: [crates/mg-corpus-tests/tests/client_render/batch6.rs, crates/mg-preview/src/lib.rs, crates/mg-image/src/tga.rs, tools/nwclient/run-client.sh, tools/nwclient/execlog.c, docs/deferred.md]
fingerprint: v2-584cb4caa3fcc5e0
created: 2026-11-02
author: August
description: "Records where sixth client batch results live, how to run a batch from a copied binary, texture probing tips, and a display incident where an empty DISPLAY after headless.sh restart opened clients on the owner's desktop."
asks:
  - "Why did headless.sh stop then start immediately leave DISPLAY empty and what should I check before running the client?"
  - "How do I run the client_render batch without cargo test rebuilding the binary in the middle of the run?"
  - "Clients popped up on my desktop instead of the headless session; what caused that and how is it prevented now?"
  - "How can I see which texel each pixel samples in the client using a gradient texture and texture2D(texUnit0)?"
---
## Decided
- The sixth client batch's readings are in `crates/mg-corpus-tests/tests/client_render/batch6.rs`; what each saw is in its `Seen` paragraph, `docs/findings.md` and the deferred entry. Fixes inside mg-render and mg-shader were written into the entries as sized rules (another package changed those crates that round).
- Run a batch from a copied test binary (`target/batchbin/client_render`, run from `crates/mg-corpus-tests`) so the sources can change while the client runs; cargo test rebuilds mid-batch otherwise.
- A program's custom pair painting `texture2D(texUnit0, ...)` raw (no lighting) reads a texture exactly as the client samples it; a gradient texture (red u, green v) shows which texel each pixel samples (ripples, ribbons).
- `tools/nwclient/execlog.c` (LD_PRELOAD, `MG_EXEC_LOG` under the scratch) logs every exec/spawn in the sandbox: the client starts nothing after an NWSync download.

## Watch out
- `headless.sh stop` then `start` at once can fail ("unable to lock lockfile /run/user/1000/moonglow-oracle.lock"): the old compositor still holds it. `D` comes back empty, and run-client.sh used to default to `:1`, the desktop's Xwayland: clients opened on the owner's desktop for ~12 minutes (2026-11-02 05:08-05:20 UTC). The scripts now refuse an empty DISPLAY; wait a second or two and retry the start, and check `D` is not :0/:1/empty.
- Don't put LD_PRELOAD in the environment that starts headless.sh: the headless session's dbus/gvfs processes inherit it.
- `fslit` patched in the override paints the tiles too (they use it), so a "black with alpha" probe tells blending only on the probe's meshes.
- The client's `worldtimerTimeOfDay` start is A (17,697,400) or B (24,897,600) by some cause not found; runs agree within a sitting.
- Tile_Height on a tileset's plain tiles draws no cliff in the client.

## Tried and ruled out
- TZ changes nothing in the timer; a run held 30 min shows no jump.
- Fog doesn't explain TDoD's fountain (10.6 with and without).
