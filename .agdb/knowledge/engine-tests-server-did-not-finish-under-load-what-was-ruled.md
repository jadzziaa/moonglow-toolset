---
type: Gotcha
title: "Engine tests' 'server did not finish' under load: what was ruled out, and where to look next"
description: "Why engine tests sometimes say 'server did not finish' under load: what was ruled out, and the runner's new why_unfinished output to read on the next failure"
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T02:25:14Z }
touches: [crates/mg-testkit/src/engine.rs, crates/mg-serve/src/lib.rs]
asks:
  - why did an engine test fail with server did not finish
  - is the free port in mg_serve a race
  - what does run_server print when a server stops early
  - Server shutting down right after Loading Module
fingerprint: v2-170ef31c534e5093
created: 2026-11-26
author: augustaugur
---
Seen twice on 2026-11-25 under load: engine_armor_torso in a full suite run, and engine_encoding in the Verify branch's run. The server log showed only 'Loading Module' and then 'Server shutting down' in the same second, with no done marker.

Not reproduced: 30 runs of the test, 3 at a time, at a load of 18; and 9 runs of the whole engine group, 3 at a time (603 tests, up to 24 servers), all passed.

Ruled out:
- A port taken between run_server's free-port pick and the server's bind: nwserver started on a port another socket holds still runs the module and finishes.
- A stray kill: no test signals other processes or process groups.
- Shared scratch folders: every engine test's scratch name is unique.

The failed run's engine log was gone, because the rerun reused its folder. So run_server now returns and prints why_unfinished (ServerRun::why): how long it ran, whether and how the server ended (exit status or signal), and the last 40 lines of nwengineLog.txt and nwserverError1.txt. The next failure names its cause; read that first.
