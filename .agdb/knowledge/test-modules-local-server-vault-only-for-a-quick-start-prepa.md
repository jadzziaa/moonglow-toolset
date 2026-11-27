---
type: Fact
title: "Test Module's local server: vault only for a quick start, prepare in mg-module, its lifetime"
description: "Why LocalRun passes -servervault 1 only for Join::QuickStart, where the vault folder's name comes from, what the real joined check showed, why prepare lives in mg-module, and how the local server and game die with Moonglow."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T20:01:50Z }
touches: [crates/mg-ui/src/test_module/local.rs, crates/mg-module/src/server_dir.rs, crates/mg-serve/src/cdkey.rs, crates/mg-serve/src/lib.rs, crates/mg-corpus-tests/tests/client_render/joined.rs]
asks:
  - "why does Test Module's local server not use a server vault for the character list"
  - where does the quick start put the character
  - what happens to the local test server when Moonglow quits or crashes
  - how is the quick start checked with the real game client
fingerprint: v2-01eaa3972707802f
created: 2026-11-27
author: augustaugur
---
Decided 2026-11-27 (NWNX core, 5.5.0); review fixes and the joined check the same day.
- Join::CharacterList runs the server without -servervault: the player's game picks from its own local characters, as Aurora's Test Module does. Join::QuickStart(Character::Bic) needs -servervault 1: the .bic is copied (mg_serve::place_in_vault) to <server user dir>/servervault/<public key>/, the key's public part read from the player's cdkey.ini ([NWN1] YourKey=) by mg_serve::cdkey. No cdkey.ini or a bad key: the run fails before the server starts. The private part is never returned, printed or logged. The server's log names the public part ("Name (PUBLIC) Joined as Player 1"): keep it out of printed output too.
- Checked end to end 2026-11-27: crates/mg-corpus-tests/tests/client_render/joined.rs joined_nwnx_quick_start (ignored; DISPLAY=$D MG_CLIENT_CDKEY=... under the display lock). NWNX 8193.37-17 started in the sandboxed server, the client joined with the vault folder's key, and its list showed the vault's one character "(Server Vault)" (target/test-output/client_joined_nwnx/joined_nwnx_list.png). The joined harness gives the server alone an environment through MG_SERVER_ENV (a sourced file); LD_PRELOAD in Server::start's env would preload NWNX into bash, bwrap and the client too.
- Lifetime: the server and the game are spawned from LocalRun's own thread with mg_serve::die_with_parent (PR_SET_PDEATHSIG SIGKILL via pre_exec, Linux). PDEATHSIG fires when the spawning THREAD ends, not the process: never spawn them from a short-lived thread (mg-serve tests show both the thread and the kill -9 case, with a control without the signal). LocalRun::stop_and_wait joins it; Drop waits up to STOP_WAIT; Action::Quit stops and waits before the window closes. Build › Stop Test Server (Id::StopTestServer, shown only while a run is under way) is the safety stop; Interface owns Test Module's real controls.
- UI tests that start stand-in games hold TEST_GAMES (tests/app/common.rs): test_module::game_running / GAMES is process-wide.
- The server folder's prepare (module copy, haks, tlk, NWSync haks) is mg_module::server_dir, not mg-serve: mg-testkit depends on mg-serve, and mg-serve stays light (mg-core, and rustix on Linux). mg-ui's LocalPlan takes prepare as a boxed closure (local::prepare_module) so its tests use fakes.
