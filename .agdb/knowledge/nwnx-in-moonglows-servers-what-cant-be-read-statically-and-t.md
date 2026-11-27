---
type: Fact
title: "NWNX in Moonglow's servers: what can't be read statically, and the env rules"
description: "Why NWNX's game build is checked only at run time, how NWNX's loader finds plugins and reads SKIP settings, how includes map to plugins, why every server mg starts gets a clean environment, and how --preload/--env combine with [nwnx]."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T19:56:37Z }
touches: [crates/mg-module/src/nwnx.rs, crates/mg-serve/src/lib.rs, apps/mg/src/serve.rs, crates/mg-ui/src/test_module/local.rs]
asks:
  - "how does Moonglow check that NWNX's build matches the server's"
  - "why does the server Moonglow starts not see my shell's environment variables"
  - "how do --preload and --env combine with the project's [nwnx]"
  - how does NWNX find its plugin files and which SKIP variable applies
fingerprint: v2-9a78efaf6e7e0955
created: 2026-11-27
author: augustaugur
---
Decided 2026-11-27 (NWNX core for 5.5.0); review fixes the same day.
- NWNX's target build (8193.37-17) is compiled in as integer constants (CMakeLists TARGET_NWN_BUILD -> printf args), not a string: strings(NWNX_Core.so) shows only format strings; Binaries/<sha>.txt is a build timestamp. The server's build isn't in the install as text either. So the build check is run-time only: mg_module::nwnx::Builds compares "Starting NWNX A.B-C [sha]" and "Build: A.B-C [sha]"; different game build = error, other revision = warning; NWNX's own "Expected build" / "Will terminate" lines are errors.
- A failed LD_PRELOAD is NOT fatal for nwserver: ld.so prints "ERROR: ld.so: object '<path>' from LD_PRELOAD cannot be preloaded (...): ignored." on stderr. console_note makes it an error only when <path> is an NWNX_Core.so; another library refused is a warning (the server runs without it).
- NWNX's loader (~/.local/opt/nwnx/src/unified/Core/NWNXCore.cpp InitialSetupPlugins, NWNXLib/Config.hpp): every regular file/link in LOAD_PATH whose name minus its LAST extension starts with "NWNX_" (case-sensitive) is a plugin; "NWNX_Core" and "NWNX_Experimental" compared exactly. The switch is NWNX_<stem upper-cased>_SKIP, else NWNX_CORE_SKIP_ALL; Experimental needs NWNX_CORE_LOAD_EXPERIMENTAL_PLUGIN. The key->file map is not invertible (case lost), so a "SKIP=n for a missing plugin" error names no file: "no NWNX plugin library for TDOD in <Binaries>". Truthy t/true/y/yes/1, falsy f/false/n/no/0, else unset.
- Include -> plugin by the include's own `const string NWNX_X = "NWNX_X";` (nwnx_admin is Administration), module's copy first, then <home>/NWScript.
- Every server mg serve / mg test / mg watch --serve / Test Module's LocalRun starts gets mg_serve::Options::clean_env (only KEPT_VARIABLES + what is given), NWNX or not: a shell's LD_PRELOAD=.../NWNX_Core.so would load NWNX unchecked, TDoD's WEBHOOK plugin posts when TDOD_CRIER_WEBHOOK is set, and a "drop NWNX_*/TDOD_*" list can't name every project's variables. TDoD passes --env and --preload explicitly, so it lost nothing. Not mg-serve's default: tools/nwclient/run-server-and-client.sh is started through Server::start (mg_testkit::joined) and needs the inherited display/lock variables.
- --env lines are merged before the checks: mg_module::nwnx::resolve_with(project, o, more) (resolve = resolve_with(.., &[])); they win, a plugin they switch on is checked; their relative NWNX_CORE_LOAD_PATH is from the current folder. "NWNX is on" for the console checks (serve.rs ServerEnv::nwnx, Heard) = the final LD_PRELOAD holds an NWNX_Core.so (project's, --preload's or --env's), not "any --preload". --preload of the project's core is a no-op, another library goes beside it (core:lib), another NWNX_Core.so is refused.
- [nwnx] is resolved from the module the builder named (Config.project, else Config.module), never the scratch copy. Off Linux, use = true says LINUX_ONLY alone (no "no home" warning). Home only from --nwnx, user.toml, the team file.
- resolve (script_use tokenizes every NSS) runs on the UI thread for Test Module: measured in release 2026-11-27, 14.7 ms on TDoD's tdod.mod (440 scripts), 4 ms on Kingmaker (1392): under the ~50 ms bar, so it stayed there.
