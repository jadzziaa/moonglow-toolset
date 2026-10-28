---
title: Return-type mismatches compile as in Aurora (x2_inc_cutscene)
author: August
date: 2026-10-28
branch: worktree-agent-a9831a83dc89282e6
cost: { minutes: 30, tokens: 311708 }
touches:
  - apps/mg/tests/compile.rs
  - crates/mg-corpus-tests/tests/groups/aurora.rs
  - crates/mg-module/src/refs.rs
  - crates/mg-module/src/rename.rs
  - crates/mg-script/nwscript/README.md
  - crates/mg-script/nwscript/native/scriptcompparsetree.cpp
  - crates/mg-script/src/lex.rs
  - crates/mg-script/src/lib.rs
  - crates/mg-script/src/outline.rs
  - crates/mg-ui/tests/app/scripts.rs
---

decided: the vendored compiler's upstream return-type check (FUNCTION IMPLEMENTATION AND DEFINITION DIFFER when only the return type differs) is patched out (// MOONGLOW PATCH (return-type-mismatch), scriptcompparsetree.cpp): Aurora 8193.37 compiles these, keeping the first-seen type at call sites, byte for byte equal to Moonglow's after the patch (captures script-compile/build.mod). Warnings come from Rust (mg-script return_types.rs), from the sources the compiler's load callback delivered, walked in include order; the C API has no warning channel.
decided: the dangerous directions (definition returns a value of another type than declared) stay warnings, as Aurora compiles them; the engine (engine_return_types) settled what they do: void-declared/int-defined overwrites the caller's last variable; string-declared/int-defined stops the script silently where used; non-void-declared/void-defined gives the type's empty value (x2's CutCreateObjectCopy is always OBJECT_INVALID).
watch out: the scripts corpus test compares with nwn_script_comp 2.3.1, which refuses all of them (338 scripts of the base game and shipped modules, all of the 'defined void' kind); it allows exactly those that warn. Shipped NCS are no byte oracle for them (22 of 338 equal: compiled by the 2002-2003 compiler).
watch out: Aurora's Build Module reports include files with errors (an include leaning on its includer too) as Compile Error and says nothing of good ones; Moonglow skips include errors by decision (2026-10-07). has_entry_point also scans tokens for 'main ( ) {', or a script whose main follows broken code was passed over as an include.
tried: reading Aurora's Build Module results: a 5-line Delphi listbox, paged with Home and Down keys (xdrive key) and screenshot crops.
