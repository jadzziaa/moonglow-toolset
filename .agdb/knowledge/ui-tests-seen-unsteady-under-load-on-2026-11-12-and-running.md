---
type: Decision
title: "UI tests seen unsteady under load on 2026-11-12, and running several sessions in one checkout"
tags: []
status: stable
generated: { by: trail, at: 2026-11-12T00:00:00Z }
touches: [crates/mg-ui/src/area_view.rs, crates/mg-ui/src/terrain_mode.rs, crates/mg-ui/tests/app/hak_closing.rs, crates/mg-ui/tests/app/main.rs, crates/mg-ui/tests/app/tiles.rs, docs/log.md, docs/manual/04-areas.md, docs/manual/12-differences.md, apps/mg/target/tmp/moonglow-serve/a1098591-0667e805-2/.mg-lock, apps/mg/target/tmp/moonglow-serve/a1098591-0667e805/.mg-lock]
fingerprint: v2-dae18d92c296cfe1
created: 2026-11-12
author: August
description: "A flaky hak_closing UI test was a test race, not a product bug; parallel sessions in one checkout need separate MOONGLOW_TEST_OUTPUT folders, and four mg-ui app tests failed under side-by-side runs, uninvestigated."
asks:
  - "Why did hak_closing::every_way_of_closing_a_hak_with_a_file_added fail once in a whole-binary run, and is it a product bug?"
  - "How do I run test suites from several sessions in the same checkout without them interfering with each other?"
  - "A test failed during my whole-binary run but I didn't touch that code; is it flaky or is another session mid-edit?"
  - "Which mg-ui app tests failed when two copies of cargo test ran side by side, and has anyone investigated them?"
---
## Decided
- `hak_closing::every_way_of_closing_a_hak_with_a_file_added` failed one cell in one of three whole-binary runs. The hak editor session found a race in the test (the close route pushed a frame after Save, when the save job could already be done), not in the product; its own learning has the detail.
- Sessions sharing the main checkout each ran their suites with their own `MOONGLOW_TEST_OUTPUT` folder and staged only their own files; each commit was checked with `git log -1 --stat` against the session's file list before a push, and pushes went through a `tmp/` branch's CI first.

## Tried and ruled out
- Reading a session's turn note as its result: the note is the first 400 characters. The full report (and anything it asks) is in the session's last message; the tree and `git log` are the check.

## Watch out
- Four existing tests on `develop` failed once or twice when `cargo test -p mg-ui --test app` ran as two copies side by side (about 28 runs), and nobody has looked at them: `area_view::a_paste_weighs_every_kind_alike_and_the_option_silences_it`, `blueprints::an_armor_s_parts_take_colors_of_their_own_and_keep_them`, `modules::unsaved_work_survives_a_crash`, `drawing::reload_resources_reads_an_area_again_on_a_thread`. Their failing output was not kept. The owner was asked whether to have them steadied and had not answered.
- A loaded whole-binary run by one session fails on another session's tests while those are mid-edit in the same checkout: check whose files a failing test belongs to before calling it unsteady.
