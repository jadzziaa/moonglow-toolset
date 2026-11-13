---
type: Decision
title: "Walkmesh cutters are NWN2's, not EE's"
tags: []
status: stable
generated: { by: trail, at: 2026-10-29T00:00:00Z }
touches: [docs/deferred.md, docs/log.md]
fingerprint: v2-48b4bdbd9c2a2258
created: 2026-10-29
author: August
description: "NWN:EE has no walkmesh cutter; that is an NWN2 toolset feature, and EE's CNWSTrigger only has generic, transition and trap types, so requests to draw or place cutters have no engine support."
asks:
  - "Can I place a walkmesh cutter in the Neverwinter Nights Enhanced Edition toolset like in NWN2?"
  - "Builders keep asking for walkmesh cutters; does the EE engine read anything for them?"
  - "What trigger types does CNWSTrigger support in EE, and did EE add a new trigger kind?"
  - "How do I stop pathing through an area spot when I can't find a cutter option in the EE toolset?"
---
## Decided
- EE has no walkmesh cutter: no 'cutter' in nwtoolset.exe, nwserver-linux or nwmain-linux (strings -a and -el), nwscript.nss or the talk table; CNWSTrigger has one kind, Type 0 generic, 1 transition, 2 trap. The deferred entry asking to draw and place 'the trigger kind EE added' was closed as a Verification.
## Watch out
- Builders ask for walkmesh cutters (community_pain_points.md 2.6) meaning NWN2's toolset feature; the engine reads nothing for it.
