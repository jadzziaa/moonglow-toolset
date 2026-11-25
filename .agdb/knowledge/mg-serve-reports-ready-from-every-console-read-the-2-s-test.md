---
type: Fact
title: mg serve reports ready from every console read; the 2 s test race
description: "mg serve raised Event::Ready only inside the serving loop; the final console drains skipped it, so a module loading after the --timeout under full-suite load gave no ready event and a_module_is_served_until_a_line_of_its_log failed. Fixed with console_events/drain_console; never-said step uses --timeout 10."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T15:14:46Z }
touches: [apps/mg/src/serve.rs, apps/mg/tests/serve.rs]
asks:
  - why did a_module_is_served_until_a_line_of_its_log fail with no ready event
  - where does mg serve raise the ready event
  - why does the never-said step use a 10 s timeout
fingerprint: v2-0ade75b878b74bd4
created: 2026-11-25
author: augustaugur
---
Cause: apps/mg/src/serve.rs serve() raised Event::Ready only in the serving loop. The two final console drains (after server.exited() and after server.stop()) passed lines on as Console only. Under full-suite load the module loaded after the 2 s --timeout of the never-said step, so its Server: Module loaded line arrived only in the last drain and no ready event came; the test failed at its ready count.

Fix: console_events (line to Ready/failed/Console) and drain_console (the last reads) are used in all three places; unit test a_load_said_after_the_loop_is_still_ready. The never-said step in apps/mg/tests/serve.rs uses --timeout 10. A repro alone and 4 at once passed 47/47: it needs heavy load.
