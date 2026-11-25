---
type: Gotcha
title: "screens.rs pictures differ run to run: 716 of 1259 on one commit"
description: "Why a whole-suite picture diff of screens.rs between branches proves nothing by itself, and how to compare."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T15:58:09Z }
touches: [crates/mg-ui/tests/screens.rs]
asks:
  - Why do screens.rs pictures differ between two runs with no change?
  - Is a picture difference between develop and a branch a regression?
  - What explained the 779 of 930 pictures that differed in the M7 review?
  - How do I compare screens.rs pictures between branches?
fingerprint: v2-ba50ca88c63f3f6c
created: 2026-11-25
author: augustaugur
---
Two runs of the screens suite on one commit (41a64934, 2026-11-25) differed in 716 of 1,259 pictures. Causes, from crops of both runs:
- The Log's clock times: most of the 681 pictures under 1,000 pixels apart.
- The output folder's path, shown in the update window and the Log (MOONGLOW_TEST_OUTPUT, or a different worktree).
- Painting's random tile variants: crosser-cursor (14% of its pixels).
- 3D and gallery views: area-paint, gallery sliders, board gallery palette.
- script-include-error: the compiler's message named inc_bad.nss in one run and inc_bad.nss(2) in the other. Not timing but a compiler fault: the include trace printed a stale or uninitialized line. Fixed by the vendored compiler's include-trace-line patch (crates/mg-script/nwscript/README.md); the test now waits with settle instead of a 700 ms sleep.
This explains the M7 review's 779 of 930 (develop against the waves, run from different folders). To compare branches, hide or mask the Log (settings.hide_log), use the same output path, and compare per picture with a threshold: byte-equal files first, then the count of differing pixels (Pillow).
