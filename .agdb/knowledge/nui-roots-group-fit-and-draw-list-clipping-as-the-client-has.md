---
type: Decision
title: "NUI roots, group fit and draw-list clipping, as the client has them"
tags: []
status: stable
generated: { by: trail, at: 2026-11-06T00:00:00Z }
touches: [crates/mg-corpus-tests/tests/client_nui.rs, crates/mg-nui/src/validate.rs, crates/mg-ui/src/nui_view/headless.rs, crates/mg-ui/src/nui_view/preview/containers.rs, crates/mg-ui/src/nui_view/preview/node.rs, crates/mg-ui/src/nui_view/preview/stage.rs, crates/mg-ui/tests/nui.rs, docs/log.md, docs/manual/18-nui-creator.md, apps/mg/target/tmp/moonglow-serve/a1098591-0667e805-2/.mg-lock]
fingerprint: v2-f5986a00e51d9b59
created: 2026-11-06
author: August
description: "Records how the client lays out NUI group roots (no draw list, border 4 points out, child at column-root position, children not size-held, draw list reaching the window body), and that body clipping was kept."
asks:
  - "How does the NWN client position a group root's border and child inside a 400x300 window?"
  - "Why does my group root draw no draw list when a row or column root does?"
  - "Should I clip NUI draw lists at the window edge or the body, and what did measurements show?"
  - "My client_nui test run has early cases coming out Blank, or a script name fails; what causes that?"
---
## Decided
- Settled in the client (`client_nui`, 8193.37-17): a group root draws no draw list (a row or column root does); a group root's border sits 4 points out in the window's inset, its child where a column root's would be (12,45 in a 400x300 window); a row/col that is a group's child (or a variant shown in a group) is not held to its own size; a group's own draw list reaches past its box to the window's body.
- `mg nui layout`/`render` report the checks' "NWN refuses this window" errors (headless::document).

## Tried and ruled out
- Clipping draw lists at the window's edge instead of the body: the old body clip matched the client exactly (group_overflow 11,360 px in both); measure magenta counts in the client before changing a clip.

## Watch out
- client_nui case names make `{name}_o` scripts: at most 14 characters.
- Other sessions' clients can share display :2: a run whose early cases come out Blank was disturbed; look at the screenshot and rerun.
- A group inside a group root still differs by a few points (left as a measured difference in the test).
