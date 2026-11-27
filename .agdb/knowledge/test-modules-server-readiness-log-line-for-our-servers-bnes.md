---
type: Decision
title: "Test Module's server readiness: log line for our servers, BNES query only for a builder's own command"
description: "Moonglow's own servers (mg serve, the quick start) are ready at the module-loaded log line (Server::wait_ready); the UDP BNES probe mg_serve::answers is only for Options › Test Module's command, whose server Moonglow neither starts nor can read the log of."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T10:08:44Z }
touches: [crates/mg-ui/src/test_command.rs, "crates/mg-serve/src/lib.rs:answers", "crates/mg-ui/src/test_module.rs:join_command"]
asks:
  - How does Test Module know a server is ready before the game joins?
  - "Why is mg_serve::answers not used for mg serve or the quick start?"
  - "Where is the client's +connect command line built?"
  - "Does the game's BNES query get answered before the module loads?"
fingerprint: v2-163c6dda0732f218
created: 2026-11-27
author: augustaugur
---
Agreed with Builder Requests on 2026-11-27 (5.5.0, GitHub issue 52). A UDP query only shows the server is listening; Builder Requests (TDoD) saw a client that joins before the module has loaded quit, so servers Moonglow starts wait for the module-loaded log line (Server::wait_ready, moved from apps/mg/src/serve.rs into mg-serve). Options › Test Module's command of the builder's own (WSL, NWNX scripts) runs a server Moonglow can't read the log of, so its Then start the game asks the address with the game's BNES query (mg_serve::answers: BNES + the asking port as u16 LE + 0x00; the reply starts BNER) for up to two minutes. crates/mg-corpus-tests/tests/engine_server_query.rs records when a real nwserver first answers against when its module loads: read its result before changing the wait. test_module::join_command is the one place the client's +connect/+password line is built; the quick start's LocalRun takes it. The quick start's character is Character::Bic, alone in the scratch servervault: one confirm remains (owner's ruling), as no client argument skips the character list.
