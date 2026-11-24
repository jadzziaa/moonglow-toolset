---
type: Gotcha
title: "mg commands: before_save runs without edits; watch tells a standing error once"
description: "Why run_with_before_save runs before_save with zero edits, and how mg watch dedupes read errors."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-24T00:55:31Z }
touches: ["apps/mg/src/edits.rs:run_with_before_save", "apps/mg/src/watch.rs:news_of"]
asks:
  - Why does mg bake-walkmesh write its hak when there are no edits?
  - Why does mg watch not reprint the same error every 400 ms?
  - "Does Module::save write anything besides resources?"
fingerprint: v2-da285c438b5e4ce2
created: 2026-11-24
author: augustaugur
---
bake-walkmesh writes its hak and rows file in before_save, which must run whenever it is not a dry run, even with 0 edits (an object already on its row, hak lost its MDL). Only palettes and the module save wait for count > 0. Watcher::news_of: read errors (scan problems, reread failure) are told and re-polled (retry) only while the set changes; a standing error is told once. Module::save writes only resources (folder/archive/project files), so compile, replace and import skipping the save when module_changed is false loses nothing.
