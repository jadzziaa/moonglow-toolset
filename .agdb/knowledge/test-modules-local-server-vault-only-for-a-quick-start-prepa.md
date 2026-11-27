---
type: Fact
title: "Test Module's local server: vault only for a quick start, prepare in mg-module"
description: "Why LocalRun passes -servervault 1 only for Join::QuickStart, where the vault folder's name comes from, and why the server folder's prepare moved to mg-module, not mg-serve."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T15:55:51Z }
touches: [crates/mg-ui/src/test_module/local.rs, crates/mg-module/src/server_dir.rs, crates/mg-serve/src/cdkey.rs]
asks:
  - "why does Test Module's local server not use a server vault for the character list"
  - where does the quick start put the character
  - why is prepare in mg-module and not mg-serve
  - can mg-serve depend on mg-module or mg-nwsync
fingerprint: v2-7833c0db614c0d19
created: 2026-11-27
author: augustaugur
---
Decided 2026-11-27 (NWNX core, 5.5.0).
- Join::CharacterList runs the server without -servervault: the player's game picks from its own local characters (localvault), as Aurora's Test Module does. Join::QuickStart(Character::Bic) needs -servervault 1: the .bic is copied (mg_serve::place_in_vault) to <server user dir>/servervault/<public key>/, the key's public part read from the player's cdkey.ini ([NWN1] YourKey=) by mg_serve::cdkey (a port of nwn-message 0.1.1's CDKey; differential test against its Python where installed). No cdkey.ini or a bad key: the run fails before the server starts. The private part is never returned, printed or logged; KeyError never quotes the key.
- Neither the joined-client harness nor a real server has yet checked the quick-start path end to end (2026-11-27): only fake servers and games in test_module/local/tests.rs.
- The server folder's prepare (module copy, haks, tlk, NWSync haks; was apps/mg/src/serve.rs) moved to mg_module::server_dir, not mg-serve: mg-testkit depends on mg-serve, and mg-serve stays light (mg-core only) so the GPU-free test tooling doesn't pull in mg-module/mg-nwsync. mg-module gained anyhow for it. mg-ui's LocalPlan takes prepare as a boxed closure (local::prepare_module) so its tests use fakes.
