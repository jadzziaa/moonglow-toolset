---
type: Decision
title: "UI per-frame suspects measured (Nov 2026): what was fixed and what was left"
description: "Release timings of the deferred per-frame suspects with temporary timers: chart fingerprint fixed (3.7 to 1.3 ms); placed sounds 58 us, Right Dock 6 us, NUI check 0.3 us, Interact compares 47 us at 304 controls, variant context 277 us at 304 controls re-deferred (a cache hit would cost compare plus clone)."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T18:21:48Z }
asks:
  - Why does the area read its GIT sounds every frame and is it worth caching?
  - Should the NUI variant context document be cached?
  - How fast is the NUI chart of a million samples and why the custom fingerprint?
  - How were per-frame costs in mg-ui measured without whole-frame noise?
created: 2026-11-25
author: augustaugur
---
Method: put Instant timers with call counters around the suspect (a Drop guard printing per call or per N frames), run the existing perf view in release (frame_perf sound_ranges, nui_perf), then git checkout the files; whole-frame A/B drowns in 3D noise. A 70 us line (1% of 6.9 ms) was set before looking. Fixed: chart.rs sample_fingerprint used SipHash over every value (3.7 ms for 1M); now a multiply-rotate fold (1.3 ms), still sound because each step is a bijection (one changed value always changes the hash); the window is edited in place (&mut Value), so array pointer+len alone can't key the cache. Left: area_audio::update GIT read 58 us with 317 sounds (runs whenever an area view is shown, not only while sounds play); shell's Right Dock layout plus the tab lists ~6 us; nui_view check 0.3 us idle (copies only on new bytes); Interact's matches() 13 us at 96 controls, 47 us at 304; layouts::canvas_context 86 us for a variant edited at 96 controls but 277 us at 304 (measure at the size the entry names!); a safe cache key needs deep compares of doc, settings and main_doc (~95 us at 304) since state.copies misses typed-but-unset values, and the canvas takes the context owned (Cow::Owned), so a hit also clones it (168 us): no saving unless the canvas borrows the cache; re-deferred. Right Dock with a custom area_right_layout (the normalized clone path): 9 us. The chart's 1.3 ms walk (line fingerprint, columns min/max) stays deferred: no change signal for an in-place edited window.
