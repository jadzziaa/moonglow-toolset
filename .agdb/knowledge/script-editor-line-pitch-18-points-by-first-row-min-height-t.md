---
type: Decision
title: "Script editor line pitch: 18 points by first_row_min_height, text centered, gutter painted (M5 W3)"
description: "How the script editor's lines get the Script board's 18-point pitch and gutter (ruling 21), why line_height was not used, and where the pitch must be counted the same."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-21T15:27:17Z }
touches: [crates/mg-ui/src/script_layout.rs, "crates/mg-ui/src/script_view.rs:Palette", crates/mg-ui/src/browser.rs, crates/mg-ui/src/nui_view/source.rs]
asks:
  - "how is the script editor's line height set"
  - why are the script lines 18 points apart
  - "why doesn't TextFormat line_height center the text"
  - "where are the script editor's line numbers drawn"
fingerprint: v2-9c1faed0fcd1734a
created: 2026-11-21
author: augustaugur
---
The owner ruled (21, 2026-11-21) "match the board": Script.png's `.code` has 13 px Hack at
`line-height: 18px`, the gutter `.ln` padding 12 left / 10 right with numbers right-aligned in
an auto-width column, the text `.tx` padding 8 left / 12 right, `.ln.cur` + `.tx.cur` band.

- Pitch: `Palette::pitch()` = round(size * 18/13) (24 at 17). `Palette::row_height(ppp)` is the
  same rounded to whole pixels as epaint rounds a row: count every `line * row` with it (Page
  keys, LongText's `show_rows`), or better take the galley's own row rects.
- Do NOT use `TextFormat::line_height`: epaint places a glyph at
  `ascent + valign * (row_height - glyph.line_height)`, and with line_height set on the section
  that difference is 0, so the text sits at the row's top with all leading below. Each line is
  its own single-row job (`script_layout::LaidOut`), so `job.first_row_min_height = pitch` with
  `valign: Center` (`Palette::format`, `Palette::pitched`) centers it. That only works for
  single-row jobs: whole-text users go through `highlight_lines` / `highlight_galley`.
  (The Help panel's declaration keeps its own top-aligned `line_height = 18`.)
- Gutter: `LaidOut::numbers` keeps one galley per line number (`{n}`, no padding); the editor
  allocates `gutter_width()` (12 + widest + 10) and `paint_gutter` paints only rows in sight,
  right-aligned, beside `out.galley`'s rows (binary search on row y). Bookmark icons and
  numbered digits go in the 12-point pad (`paint_marks`). The old 6-column Label of all
  numbers (one huge galley, mark as first character) is gone.
- `script_layout::text_frame()` is the text's frame (8/12/0/0); the kit's `code_frame` (4 by 2)
  stays for other users. The NUI source view uses the same gutter and frame; its band and
  scroll take the galley's rows (before, field top + a * font row, off by the frame's 2).
- Current-line band: drawn under the text only when the cursor has no selection;
  `Moonglow::script_gutter_for_test` exposes the gutter's x range and the band's y range.

Fix round (review B, the coordinator's rulings):
- Sizes 6 to 9 (Options allows 6 to 48): the board's ratio left Hack's glyphs touching the
  row's top. `pitch()` is now `max(round(size*18/13), ceil(size*MONO_HEIGHT + 2*LEAST_ROOM))`
  with `MONO_HEIGHT` = 1.164 (Hack's egui row height per point, held by
  `the_monospace_font_is_as_high_as_the_pitch_takes_it`) and `LEAST_ROOM` = 1: 6..9 become
  9, 11, 12, 13; 10 and up keep the ratio. Options' least size stays 6 (the board shows only 13).
- The error wave: `script_view::wave_y` = baseline + 3*size/13, clamped so the wave and its
  1-point stroke stay inside the row (at 13 that is 2 to 2.8 under the baseline, not 3).
- The current-line hairlines: `script_view::hairlines` puts them on whole pixels (1 px at 1.1
  and 1.25, 2 px from 1.5).
- Screen reader: `script_layout::name_gutter` ("Line numbers 1 to N" via `widget_info`) and
  `describe_line` (the field's AccessKit description "Line 12 of N"); the NUI source takes the
  cursor from `TextEditState` when the field has no focus (its `cursor_range` is then None).
- Painted numbers are tested from `h.output().shapes` (`painted_texts`/`painted_inside` in
  tests/app/common.rs): no widget holds them. The NUI band record is written every frame,
  `None` included, or it outlives the band (Binding JSON).
- In AccessKit a LongText line's text is the node's value, not its label; "int nLine5 " also
  starts another run in the window, so the Page Down check measures line 30.

Second fix round (review D):
- Ruled: the error wave lies at the descenders' foot and may touch them (as the board's
  `text-underline-offset: 3px` and every editor); it only must stay inside its row. At 13
  the descenders' ink ends at 17 of 18, so a wave clear of them (3 high, 1-point stroke) would
  need a 24-point pitch or text 3 points off center. Don't "fix" this again.
- Bookmark marks: drawn in the first 10 of GUTTER_LEFT's 12 points (`MARK_GAP` = 2); the
  icon's ink fills about 70% of its square, so the test asserts its ink 3+ points clear of the
  numbers' column (with no gap it is 2). The board draws no mark: no measure of its own.
