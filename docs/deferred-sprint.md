---
type: Plan
title: 'The deferred sprint: a quarter of the list addressed'
description: The sprint of late October 2026 that triages the whole deferred list and addresses at least a quarter of its open entries (110 of 440, at least 50 by code) - what counts, the rule that the list may not grow, the triage pass and its verdicts, the waves in their order, and the count after each.
tags: [plan, deferred, backlog, sprint]
status: draft
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-28T11:31:02Z }
stale_after: 2026-11-30T00:00:00Z
sources:
  - id: owner
    resource: conversation with the owner, 2026-10-28
    title: 'The owner''s scope: a quarter or more of the entries addressed, at least 50 by code; the triage pass approved'
  - id: deferred
    resource: deferred.md
    title: 'Deferred: what was left out, for a decision later'
---

# The deferred sprint: a quarter of the list addressed

The [deferred list](deferred.md) had grown past what the work can address:
440 open entries (597 counting an entry's items one by one) on
2026-10-28, against a pace at which each finished piece of work adds
entries of its own. This sprint triages the whole list and addresses at
least a quarter of it.[^owner]

## The goal

- **The unit** is a top-level entry of an open section (every section
  but the last, "Settled"), counted by `tools/deferred/count.py`. An
  entry with items of its own closes when all of them do; rewriting it
  down to what is left gains nothing.
- **Addressed** means gone from the open sections, in one of four ways,
  counted apart:
  - **fixed**: done in code, with its tests;
  - **verified**: checked (in the engine, the client, Aurora or the code)
    and found right or already done;
  - **dropped**: not worth doing, by the owner's verdict, deleted with a
    line in the [log](log.md);
  - **settled**: kept as it is by decision, moved to the Settled section.
- **The target:** at least 110 entries addressed (a quarter of 440), at
  least 50 of them fixed.

## The rule: the list does not grow

Work is deferred only when doing it would have real unintended
consequences or would over-implement what was asked (the owner's rule
of 2026-10-28, first of `CLAUDE.md`'s Rules): edge cases, tests, the
manual and checks that can be run here are the work, not entries. For
the sprint, every merge into `develop` closes at least as many
entries as it adds. An agent's brief says so; its report lists the
entries it adds and the ones it closes; the merge review accepts the new
ones or sends the work back. The count is taken before and after each
merge.

## The triage pass

Seven read-only agents, each a part of the list by section (50 to 94
entries, about 6,000 words), propose for every entry:

| Field | What |
|---|---|
| verdict | `already-fixed`, `duplicate`, `verify`, `fix-S`, `fix-M`, `fix-L`, `waits`, `drop?`, `settle?` |
| evidence | for `already-fixed` and `duplicate`, a `file:line`, a test or the other entry; for `verify`, how it would be checked |
| cluster | the code it touches (crate and file), so that entries in one place go to one worker |
| needs / unblocks | other entries it waits on or makes cheaper |
| builder | whether a builder, issue or review asked for it |

No agent drops or settles anything: `drop?` and `settle?` are proposals
for the owner. A sample of `already-fixed` claims is checked before the
owner sees them. The owner marks verdicts on a triage page; the verdicts
land in `deferred.md` itself.

## The waves

1. **Wave 0, no code:** duplicates merged, `already-fixed` entries
   closed with their evidence, the owner's drops and settles, and the
   seven questions waiting on builders, which by the owner's rule of
   2026-10-12 are dropped on 2026-11-12 without an answer. Entries that
   wait on someone else (El Grillo's tiles under Direct3D 12,
   Troubletcat's dropdown under Windows, Heed's interior) are waits, not
   work.
2. **Wave 1, verification:** the entries that need only a check, batched
   by oracle: one session driving Aurora, one the game client, one the
   engine. Each closes its entry or turns it into a sized fix.
3. **Waves 2 and after, fixes:** four worktrees at a time, each worker
   5 to 8 entries of one cluster. In a cluster, the entry of "Written
   more than once" that it holds goes first. Builders' entries before
   the owner's notes; the L entries are weighed for what they unblock.

## The count

| When | Open entries | Items | Fixed | Verified | Dropped | Settled |
|---|---|---|---|---|---|---|
| 2026-10-28, start | 440 | 597 | 0 | 0 | 0 | 0 |
| 2026-10-28, wave 0 (the owner's verdicts) | 330 | 485 | 0 | 6 closed, 3 merged | 53 | 48 |
| 2026-10-28, wave 2 (four packages merged) | 299 | 451 | 31 | 9 | 53 | 48 |
| 2026-10-28, wave 3 (four packages merged) | 266 | 405 | 63 | 10 | 53 | 48 |

[^owner]: The owner's scope, 2026-10-28: a quarter or more of the entries addressed, at least 50 by code; the triage pass approved.
