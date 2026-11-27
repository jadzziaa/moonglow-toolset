---
type: Fact
title: "NWNX in Moonglow's servers: what can't be read statically, and the env rules"
description: "Why NWNX's game build is checked only at run time, how NWNX reads its SKIP settings, how includes map to plugins, why a server with NWNX gets a clean environment, and how --preload/--env combine with [nwnx]."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T14:27:55Z }
touches: [crates/mg-module/src/nwnx.rs, crates/mg-serve/src/lib.rs, apps/mg/src/serve.rs, crates/mg-ui/src/test_module/local.rs]
asks:
  - "how does Moonglow check that NWNX's build matches the server's"
  - "why does the server Moonglow starts not see my shell's environment variables"
  - "how do --preload and --env combine with the project's [nwnx]"
  - why is a failed LD_PRELOAD treated as an error
fingerprint: v2-6e3287052b01d1e2
created: 2026-11-27
author: augustaugur
---
Decided 2026-11-27 (NWNX core for 5.5.0).
- NWNX's target build (8193.37-17) is compiled in as integer constants (CMakeLists TARGET_NWN_BUILD → printf args), not a string: strings(NWNX_Core.so) shows only format strings; Binaries/<sha>.txt is a build timestamp; the folder name build8193.37.17-HEAD is TDoD's convention. The server's build isn't in the install as text either (bin/linux-x86/build.txt is a git sha). So the build check is run-time only: mg_module::nwnx::Builds compares the console's "Starting NWNX A.B-C [sha]" and "Build: A.B-C [sha]"; different game build = error, other revision = warning; NWNX's own "NWNX: Expected build" / "Will terminate" lines are errors too.
- A failed LD_PRELOAD is NOT fatal for nwserver: ld.so prints "ERROR: ld.so: object ... from LD_PRELOAD cannot be preloaded (...): ignored." on stderr and the module loads normally without NWNX. Hence mg-serve now pipes stderr (with a console) into the same line channel, and mg serve / LocalRun stop the server and fail (End::Nwnx) on that line, or when the module loaded with NWNX asked for but no "Starting NWNX" seen.
- NWNX reads NWNX_<PLUGIN>_SKIP (upper-cased stem), else NWNX_CORE_SKIP_ALL; truthy t/true/y/yes/1, falsy f/false/n/no/0, case-insensitive, anything else = unset; Core always loads; Experimental needs NWNX_CORE_LOAD_EXPERIMENTAL_PLUGIN (Core/NWNXCore.cpp InitialSetupPlugins, NWNXLib/Utils/String.cpp).
- Include → plugin by the include's own `const string NWNX_X = "NWNX_X";` (nwnx_admin is Administration), module's copy first, then <home>/NWScript.
- Servers with NWNX (or --preload) get mg_serve::Options::clean_env: only KEPT_VARIABLES (PATH, HOME, USER, LANG, LC_*, TZ, TMPDIR, Windows' own) of the caller's. Not the default: tools/nwclient/run-server-and-client.sh is started through Server::start and needs the inherited display/lock variables. Reason: TDoD's WEBHOOK plugin posts when TDOD_CRIER_WEBHOOK is in the env.
- --env lines go after the project's env (win); --preload of the project's own core is a no-op, another library is preloaded beside it (LD_PRELOAD=core:lib), another NWNX_Core.so is refused.
- [nwnx] is resolved from the module the builder named (Config.project, else Config.module), never the scratch copy mg test / watch serve. Home comes only from --nwnx, user.toml, the team file (no NWNX_HOME env).
