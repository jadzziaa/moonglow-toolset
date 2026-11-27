---
type: Decision
title: "Test Module's two server paths after the merge: Interface's join_command, command wins over NWNX, one stop rule"
description: "How Options › Test Module's command of the builder's own (#52) and the NWNX core's LocalRun coexist: which join_command was kept, which path a test takes, what stops what, and why the two readiness waits stay apart."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T22:32:41Z }
touches: ["crates/mg-ui/src/test_module.rs:join_command", "crates/mg-ui/src/test_command.rs:stop_last", "crates/mg-ui/src/lib.rs:launch_test", crates/mg-ui/src/test_module/local.rs]
asks:
  - which join_command does the local NWNX server use
  - what happens when Test Module is set to a command of my own and the project uses NWNX
  - does Stop Test Server stop the server my own test command started
  - can a test command and the local server run at once
fingerprint: v2-ad090cb090dcf6ff
created: 2026-11-27
author: augustaugur
---
Decided 2026-11-27 merging develop (661969b7) into nwnx-core.
- One test_module::join_command: Interface's from #52 (client_command, so Steam handling is shared with command(); +password only when non-empty). nwnx-core's had the same signature and was a stand-in; LocalRun's caller (lib.rs test_on_local_server) passes the same five arguments, and its password is always set, so nothing was adapted. The two join tests were folded into the_game_joins_a_server_with_connect.
- Which path: test_saved_module / test_from_here go to test_by_command when settings.test_starts is Command, before launch_test, so the builder's command wins over [nwnx] use: Moonglow starts no local server then. In Game mode launch_test tries test_on_local_server (NWNX, Linux) and else the game's +TestNewModule.
- Readiness stays two ways on purpose: LocalRun waits for Server::wait_ready (the module-loaded log line); the command's server, whose log Moonglow can't read, is asked with mg_serve::answers (BNES). Don't merge them.
- Stopping: test_command::stop_last runs at test_command::start AND at the top of launch_test, so the next test of any kind stops a command left running (its server would hold the game's port 5121). The reverse (a test while LocalRun runs) is refused by game_still_running. Build › Stop Test Server (Id::StopTestServer) stops only LocalRun; widening it to the command is Interface's call (on the seam list). Quit: stop_local_run_and_wait; the command's Running stops on Drop.
- Games: both paths' games are in test_module::GAMES (let_game_run; LocalRun pushes its own child's pid since it must spawn from its own thread for PDEATHSIG). UI tests that start stand-in games or commands hold TEST_GAMES (tests/app/common.rs), including test_module_runs_a_command_of_the_builders_own.
- link_development moved to mg_module::server_dir carries develop's Off/players_unless, the unlink with no user folder or Off, and the empty-folder rule; LocalRun's prepare_module calls it with Players, scratch = true (no --no-development in the UI).
