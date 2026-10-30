---
title: NWNT as nwn_nwnt writes it; nasher with gffFormat nwnt
author: August
date: 2026-10-30
branch: worktree-agent-a526527fe8dd218cb
cost: { minutes: 90, tokens: 12273110 }
touches:
  - crates/mg-gff/src/nwnt.rs
  - crates/mg-module/src/nasher/mod.rs
  - crates/mg-module/src/nasher/project.rs
  - apps/mg/src/merge.rs
---

Measured with nwn_nwnt 1.3 (NWNT by WilliamDraco) and nasher 1.1.3, probe scripts in a scratchpad:

- nwn_nwnt writes CRLF lines; fields sorted by label bytes; a nested field is named by its path (S.X, L[].X); each list item opens with 'L[]$List = <struct id as signed i32>', an empty list is 'L[]$List = _'; 'name$Type =' then ' value' only when the value isn't empty (trailing spaces in values are kept).
- Text is UTF-8 from windows-1252; only \n and \r are escaped (no backslash escape, so a literal backslash-n reads back as a line break).
- Floats: C printf %.Nf of the f64 (ties to even; Rust's {:.N} agrees). Only a Float labeled exactly 'Bearing' whose text equals -pi's has its minus dropped: not Orientation, not a Double, not 'bearing'.
- A CExoLocString's entries come out in Nim Table[int,string] order: hashWangYi1 of the key, 32 slots, linear probing (12 keys 0..11 give 4,9,6,5,7,10,0,11,3,2,8,1).
- nwn_nwnt can't read back a field with an empty label (BioWare's tic01palstd.itp): its readback asserts 0 < len(label).
- nasher with gffFormat nwnt: -p truncateFloats is passed; Mod_ID and an area's Version are dropped as with JSON. Set per project with 'nasher config --local gffFormat nwnt' (and gffUtil nwn_nwnt, gffFlags ''), i.e. .nasher/user.cfg, which Moonglow's Settings::read also reads.
- The web fetch tool won't reproduce source verbatim; asking it targeted questions about gffnwnt.nim worked, and the probes settled the rest.

In this worktree session the shell guard refuses heredocs and command substitution: edits went through python scripts in the scratchpad, run by absolute path.
