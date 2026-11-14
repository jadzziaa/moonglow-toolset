---
type: Decision
title: "mg test names a delayed assertion by its test: GetScriptName(0) in a delayed command"
description: "How mg test (format 2) attributes an assertion made in a DelayCommand or AssignCommand to the test that queued it, and why that works."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-14T14:31:01Z }
touches: [crates/mg-module/src/mg_test.nss, crates/mg-module/src/testing.rs]
asks:
  - How does mg test know which test a DelayCommand assertion belongs to?
  - What does GetScriptName return inside a DelayCommand or AssignCommand?
  - "Why is a delayed assertion's line tagged [test_x] in the server log?"
  - "Why did delayed assertions show the last test's Describe group?"
fingerprint: v2-9041865c65406951
created: 2026-11-14
author: august
---
## Decided
- The server runs a DelayCommand'd action as the script that queued it: GetScriptName(0) is the test (recursion level 0), also for AssignCommand inside DelayCommand and DelayCommand inside AssignCommand; checked in nwserver on 2026-11-14 with a probe test. Synchronous assertions run at level 1 under the driver: level 0 mg_test_run, level 1 the test.
- So mg_test.nss (format 2) tags a line whose level 0 is not the driver with `[test_name] ` (the first script that is not the driver); lines made as the test runs are unchanged. The driver names itself in the module local MG_TEST_DRIVER; without it (--driver none) nothing is tagged.
- Describe() keys its group per script (`MG_TEST_GROUP <script>`): a delayed assertion keeps its own test's group, and a test without Describe no longer inherits the previous test's.
- Report::line reads the tag only in format >= 2 and only when it names a test of the run; else the name is kept whole (under the running test or "(outside a test)"). Cases carry `delayed`.
## Watch out
- Asked by TDoD (docs/scope/pipelines/item-1-mg-brief.md there): its tools/tdod/verdict.py reads the lines.
