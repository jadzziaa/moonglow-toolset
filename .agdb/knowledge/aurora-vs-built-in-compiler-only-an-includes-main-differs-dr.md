---
type: Fact
title: "Aurora vs built-in compiler: only an include's main differs; driving Aurora's Build unattended"
description: "Aurora 8193.37's Build Module makes the built-in compiler's NCS bytes except for scripts including a file with its own void main (Aurora takes the include's main as entry point); how build_in_aurora.sh drives Build unattended and its pitfalls."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T18:17:39Z }
touches: [tools/aurora/build_in_aurora.py, tools/aurora/capture_compile_scale.py, crates/mg-corpus-tests/tests/aurora_scripts.rs, crates/mg-corpus-tests/tests/engine_include_entry.rs]
asks:
  - Why does Moonglow say scripts changed after the module was compiled in Aurora?
  - "Does Aurora's compiler make the same NCS as Moonglow's built-in compiler?"
  - "How do I run Aurora's Build Module on a module without driving it by hand?"
  - Do CRLF or LF line endings change the compiled script?
fingerprint: v2-0bdebcb2984d09a0
created: 2026-11-25
author: augustaugur
---
Measured 2026-11-25 (builder's '41 changed' report). Aurora 8193.37-17 Build Module = built-in compiler (nim 0972fc1f) byte for byte on 992 base-game NCS, either line end (line ends never change NCS; Aurora saves NSS unchanged), constant folding/switch/struct/vector/loops, and module-vs-hak duplicates (both compile the module's copy although the game puts haks above the module). One batch-per-thread Compiler: order changes no byte (fresh vs forward vs reverse, 4,000 scripts). Only difference: a script including a file that has its own void main(): Aurora compiles a StartingConditional script (or a script that is only the #include) as the include's main alone; built-in (nim d20453e3) takes the script's own entry point and passes include-only scripts over. In the game Aurora's condition runs the include's main and always fails (engine_include_entry). Keep-or-match awaits the owner (docs/deferred.md). Driving: tools/locks.sh hold display -- tools/aurora/build_in_aurora.sh SHOTS NAME=OUT.mod, one module per session. Pitfalls found: a 'did not close properly' Confirmation after a killed run (answer No); the Open dialog is titled 'Open'; Build dialog remembers its boxes between runs and grows/moves when Advanced Controls opens, so find it again before clicking Build, and check the main log says 'Building' or the click missed (runs 5-7 sat idle for an hour). 1,036 scripts build in about a minute. Never pkill -f the driver: it killed the calling shell; kill by PID.
