---
type: Decision
title: "An include's own main: Moonglow keeps the script's entry point and warns (owner's ruling)"
description: "Why Moonglow compiles a script whose include has its own main differently from Aurora, and where the warning is made."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T12:37:09Z }
touches: ["crates/mg-script/src/return_types.rs:included_entry", "crates/mg-script/src/compiler.rs:passed_over", crates/mg-module/src/build.rs]
asks:
  - Why does a StartingConditional script compile differently in Aurora than in Moonglow?
  - "Where is the warning about an include's main made?"
  - Should Moonglow match Aurora when an include has a void main?
  - Why does a module built in Aurora show scripts changed after Compile All?
fingerprint: v2-4a908afcc1439032
created: 2026-11-26
author: augustaugur
---
Aurora 8193.37's Build compiles a script whose include has its own void main() as that main: an int StartingConditional() script, and a script that is only the #include (an include's StartingConditional too, for a script with none). In the game Aurora's condition always fails (engine_include_entry). Moonglow's compiler (neverwinter.nim since d20453e3) takes the script's own entry point and passes a script with none over as an include. The owner ruled 2026-11-26 'Compile: 1 (keep, warn)': do not patch the compiler to match Aurora (unlike the return-type patch). The warning: return_types::included_entry, called on a successful compile (compiler.rs) and, for a script passed over as an include (build.rs's Err branch with no entry point), through Compiler::passed_over, which reads the sources through the resolver since a failed compile returns no sources. A script whose own entry is main with an include's StartingConditional is not warned of: Aurora's handling of that case was never observed. aurora_scripts::codegen_compiles_as_aurora_builds_it holds the warned set to exactly the scripts Aurora builds otherwise. The external-compiler path gives no warnings at all, as for return types.
