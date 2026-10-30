---
title: "Walkmesh cutters are NWN2's, not EE's"
author: August
date: 2026-10-29
branch: worktree-agent-a02be1f76ba28c11f
cost: { minutes: 10, tokens: 14132777 }
touches:
  - docs/deferred.md
  - docs/log.md
---

## Decided
- EE has no walkmesh cutter: no 'cutter' in nwtoolset.exe, nwserver-linux or nwmain-linux (strings -a and -el), nwscript.nss or the talk table; CNWSTrigger has one kind, Type 0 generic, 1 transition, 2 trap. The deferred entry asking to draw and place 'the trigger kind EE added' was closed as a Verification.
## Watch out
- Builders ask for walkmesh cutters (community_pain_points.md 2.6) meaning NWN2's toolset feature; the engine reads nothing for it.
