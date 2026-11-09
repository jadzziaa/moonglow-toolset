---
title: "Code review 2026-11-09: the method, the record and what repeats"
author: August
date: 2026-11-09
branch: develop
cost: { minutes: 1, tokens: 4710 }
touches:
  - docs/index.md
  - docs/log.md
  - apps/mg/target/tmp/moonglow-serve/a1098591-0667e805-2/.mg-lock
  - apps/mg/target/tmp/moonglow-serve/a1098591-0667e805/.mg-lock
  - apps/mg/target/tmp/moonglow-serve/a1098591-f068852f/.mg-lock
  - apps/mg/target/tmp/moonglow-serve/a1109878-ec1a9131-2/.mg-lock
  - apps/mg/target/tmp/moonglow-serve/a1109878-ec1a9131/.mg-lock
  - apps/mg/target/tmp/moonglow-serve/a1109878-fec79a17/.mg-lock
  - apps/mg/target/tmp/moonglow-test/a1098591-0667e805/.mg-lock
  - apps/mg/target/tmp/moonglow-test/a1109878-ec1a9131/.mg-lock
---

## Decided
- The review's record is `docs/review-2026-11/`: `overview.md` (how it was done, the count, what was measured, the twelve patterns that repeat, the 37 high findings, an order to fix in) and a page a part. An id is the part's letter and the reader's number (E1, S4, P2...). 349 findings; nothing was fixed in the review itself: fixing waits on the owner's word.
- The method that worked: the source cut into twelve parts of 20,000 to 30,000 lines, one reader each under one brief (what to look for, the house rules, what the last review settled, read whole and say what was not read, check a bug through its callers); a second wave for the two parts whose readers ran out of room (most of mg-rules; the NUI Creator's drawing side); then every high finding read again at its lines and a random sample of the mediums. The brief, the readers' raw reports and the scripts that make the pages from them (`collect.py`, `gen_review.py`, `build_overview.py`) are in `target/review/` (not in git).
- The pages are generated: change a finding in `target/review/readers/<part>.md` and run the two scripts, not the page by hand.

## Tried and ruled out
- A bisect on one timing run a step: it named a commit that changes only `tabs.rs` (one run of 18.5 ms where four reruns gave 12.4 to 13.0). `game_shaders_frames_at_144`'s "draws encoded" moves by up to 2 ms between runs: take two or three runs a step and judge the lowest. A step under a millisecond is beyond a bisect by this measure: profile instead.
- Clippy's pedantic and nursery lints as a bug finder: 15,689 hits, and only `string_slice` (246, read one by one) led to bugs; `redundant_clone` and `needless_collect` are the cheap hygiene.
- A clone finder for duplication: 31 exact runs of 90 tokens or more in 305,000 lines. The duplication that matters here is one rule written two ways and drifted, which only a reader finds.

## Watch out
- `cd` into a scratch worktree in the session's own shell moves the session's working directory there: use `( cd … )` for anything in a worktree under `target/`.
- `game_shaders_frames_at_144` skips The Domains of Dread without `MOONGLOW_HAKS=~/Projects/tdod-module/build/hak`, and the deferred list's figures for the woods were taken before the test drew shadows: compare with `FRAME_SHADOWS=off`.
- What to look for in new code, from the 349: a window that holds an object by `(list, index)` while the editor behind it stays live (the windows stopped being modal on 2026-10-29; `blueprint::follow` moves only Properties tabs); a cache or window `Moonglow::close` or `game_data_changed` does not reset; a folder joined with a ResRef's text outside `ResKey::path_in`; a size, count or depth read from a file and used unbounded.
