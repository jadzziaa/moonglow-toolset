---
type: Gotcha
title: "A job sets held typing before its snapshot, or its write replaced the typing"
description: "Why journal_factions::typed_text_follows_its_entry_through_a_whole_write flaked on CI: typing held in a field was set mid-job when the job's window took the keyboard, then the job's whole write (from the earlier snapshot) replaced it; real lost typing, fixed in start_job_on."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T15:24:41Z }
touches: ["crates/mg-ui/src/jobs.rs:start_job_on", "crates/mg-ui/src/jobs.rs:set_held_text", "crates/mg-ui/tests/app/journal_factions.rs:typed_text_follows_its_entry_through_a_whole_write"]
asks:
  - Why did typed_text_follows_its_entry_through_a_whole_write fail on CI?
  - "Can a plugin's write lose text typed in a field?"
  - When is held_text set before a job runs?
  - "Why does the journal test's plugin loop 30 million times?"
fingerprint: v2-71e11eaa9d7a81d5
created: 2026-11-25
author: augustaugur
---
## Cause
Text typed in a field is held (`Moonglow::held_text`) until the field is left. A job (`start_job_on`) snapshots the module without it. When the job spans frames, its modal window takes the keyboard: the held command is set as a step of its own mid-job, and the job's write (a plugin's `write_gff` of module.jrl, whole) replaces it from the snapshot. Real lost typing, not only a test race: in the product a job begun while a field holds typing (a plugin event's; a command by key or menu leaves the field first). CI (run 45640613492) saw it once: the job outlived a frame there.

## Fix
`start_job_on` calls `set_held_text` (the held command applied now, as leaving the field; shared with Pop In Properties) before the snapshot, for jobs that read the module.

## Test
The test's plugin loops 30M times so its job spans frames; without the fix it fails every run (probe: undo 1, typing set mid-job undo 2, write undo 3 without it). The paste case's expectation changed: the typing is set before the write, so it stays on its entry; only the selection goes. `journal_view::retarget`'s held-text path is no longer reached by plugin writes.

## Ruled out
Running the original test 160 times under load locally: never failed (the job finished within a frame).
