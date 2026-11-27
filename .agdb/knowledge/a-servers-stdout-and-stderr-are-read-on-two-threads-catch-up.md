---
type: Gotcha
title: "A server's stdout and stderr are read on two threads: catch_up before trusting their order"
description: "Why mg_serve::Server::catch_up exists: lines from the server's two pipes can be read out of the order they were printed, and how tests reproduce that."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T23:34:42Z }
touches: ["crates/mg-serve/src/lib.rs:catch_up", crates/mg-ui/src/test_module/local.rs]
asks:
  - why did a_core_the_loader_refuses_fails_the_run fail under load
  - "the loader's LD_PRELOAD error arrives after module loaded"
  - how do I read everything the server printed before it ended or loaded
  - how to reproduce a reader thread falling behind in mg-serve tests
fingerprint: v2-85e8549894d751cf
created: 2026-11-27
author: augustaugur
---
mg_serve::Server reads stdout and stderr on a thread each, into one channel. The kernel keeps each pipe's order but nothing orders the two: ld.so's refusal of a preloaded NWNX core goes to stderr before the server prints 'Server: Module loaded' on stdout, yet a descheduled stderr thread can deliver it after. Test Module's LocalRun then reported only 'never said Starting NWNX' (2026-11-27, full-suite flake of a_core_the_loader_refuses_fails_the_run). Fix: Moonglow keeps a dup of each pipe's writing end (io::pipe, try_clone) and catch_up() writes a MARK line into both and reads until both marks return; the reader strips a mark that lands mid-line and joins the line. wait_ready calls it before Ok, stop() after the server ends, and an ended server's last lines use it instead of the old 100 ms sleeps; the writing ends are dropped once the child exited so the readers reach EOF. Under no load the race never showed in 100 runs, nor with 2000 stderr lines in front (wait_ready polls every 50 ms): the test-only ERRORS_LAG_MS (500 ms per stderr line) makes a_server_is_waited_for_until_its_module_loads fail without catch_up every time.
