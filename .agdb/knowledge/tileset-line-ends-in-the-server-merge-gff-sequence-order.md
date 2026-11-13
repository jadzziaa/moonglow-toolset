---
type: Decision
title: Tileset line ends in the server; merge-gff sequence order
tags: []
status: stable
generated: { by: trail, at: 2026-10-30T00:00:00Z }
touches: [crates/mg-corpus-tests/tests/engine_tileset_holes.rs, crates/mg-module/src/doctor.rs, crates/mg-gff/src/merge.rs]
fingerprint: v2-468e5d5240b83809
created: 2026-10-30
author: August
description: "LF line endings in a tileset .set file are harmless at the head or middle, but near the end can hang nwserver at Loading Module by an unclear rule; merge-gff by_identity ordering and inventory keys are also described."
asks:
  - "Why is my nwserver stuck at Loading Module after I added a tileset .set file with LF line endings to a hak?"
  - "Do LF instead of CRLF line endings in a tileset .set file break loading in nwserver?"
  - "What is the exact rule for how many trailing LF bytes in a tileset file make the server fail to load the module?"
  - "How does merge-gff by_identity order the base's kept items and each side's additions, and what key do inventory items use?"
---
Measured in nwserver with ttr01.set copies in a hak (engine_tileset_holes.rs):
- LF lines at a tileset's head, in one section, or through every [TILEn] section (8,474 lines) load and read exactly as CRLF.
- At the END they can stop the module loading (server stuck at Loading Module): the last 28 lines in LF (264 bytes) fail, the last 27 (251 bytes) load; one 389-byte LF line fails even with 300 blank CRLF lines after it, yet 200 short LF lines followed by 300 blank CRLF lines load. A 245-byte single LF line at the end loaded. CRLF in the first line only fails like all-LF. The rule is NOT 'last 256 bytes have a CR' (a probe built on that failed both ways); don't re-derive it from these numbers, take more readings.
- A failing run costs the whole timeout (3x the CRLF run, >= 20 s): batch cases in one test.
merge-gff: by_identity now lays out the base's kept items in the order the side that reordered gave, and each side's additions after the base item they follow there (ours first). Inventory items key on resref+Repos_PosX/Repos_Posy, so two different items in one grid cell are both kept (overlap isn't checked).
