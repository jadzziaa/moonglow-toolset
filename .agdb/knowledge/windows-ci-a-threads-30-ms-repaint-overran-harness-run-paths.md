---
type: Gotcha
title: "Windows CI: a thread's 30 ms repaint overran Harness::run; paths cut must keep their own separators"
description: "Why two dialog tests overran max_steps only on Windows (the 2DA description look's 30 ms repaint) and why panels cut paths through shown_path instead of joining parts with '/'."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-22T14:12:02Z }
touches: ["crates/mg-ui/src/table_view.rs:take_look", crates/mg-ui/src/shown_path.rs, "crates/mg-ui/src/hak_view.rs:cut_start", crates/mg-ui/src/talk_view/panels.rs]
asks:
  - "Why did dialog_pattern tests exceed max_steps on Windows with table_view.rs:1044 as the repaint cause?"
  - Why does a cut path in the hak editor show forward slashes on Windows?
  - How should a panel shorten a long path by its folders?
  - The one-CPU recipe does not reproduce a Windows-only max_steps overrun; how else can I reproduce it?
fingerprint: v2-b8fd83502f4fab70
created: 2026-11-22
author: augustaugur
---
2026-11-22, Windows CI's first run in a day: dialog_pattern's editor and wizard tests overran Harness::run (causes table_view.rs:1044 twice: follow_looks and describe both call take_look). The 2DA editor's description look asked for a frame in 30 ms while its thread ran; under kittest that is a frame at once, and in tests the look restarts every simulated second (4 frames), so any run with a table open can land on one; Windows' thread start and file stats outlast ~3 frames. Not reproduced on Linux by the one-CPU recipe (3 of 3 passed); reproduced by holding the look (hold_description_looks) and calling try_run. Fix: Moonglow::frame_after (doc_tabs::a_table_s_look_under_way_asks_for_no_frames_at_once). Any thread poll with request_repaint_after under 250 ms is the same fault. Paths: hak_view::cut_start split at both separators but joined with '/', talk panels' shorter joined with '/' and at_separators broke only at '/'; CI's test paths mix '\\' and '/' (test_output joins ../../target). Rule: a cut path is a suffix of the path's own text after '…' (shown_path::cut_folders, last_parts, lines_at_separators, which take a windows flag so unit tests run anywhere); tests build expectations from Path::file_name and MAIN_SEPARATOR, never rsplit('/').
