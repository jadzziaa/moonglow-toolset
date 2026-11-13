---
type: Decision
title: "Joined client readings: the DM Creator, NWSync, light counts"
tags: []
status: stable
generated: { by: trail, at: 2026-10-30T00:00:00Z }
touches: [crates/mg-corpus-tests/tests/client_render/joined.rs, crates/mg-corpus-tests/tests/client_render/batch3.rs, tools/nwclient/run-server-and-client.sh, crates/mg-testkit/src/joined.rs]
fingerprint: v2-a738048d15af3012
created: 2026-10-30
author: August
description: "Practical gotchas for driving a joined client in the mg_testkit sandbox: required name setting, DM character vault, Creator double-click, NWSync self-exit, a script quoting trap, TLK byte encoding, and a shader to count lights per draw."
asks:
  - "Why does my sandbox client stop at 'Please Enter a Player Name' when joining a server, and how do I fix it?"
  - "The DM client's character list is empty in the test sandbox. What do I need to put in dmvault?"
  - "Client exits with 'We disconnected you from the server while NWSync is running' and doesn't restart. How do I get it back?"
  - "How can I see how many lights affect each draw, and what is the default max-lights limit?"
---
Driving a client joined to a server in the sandbox (mg_testkit::joined, MG_CLIENT_CDKEY): joining stops at 'Please Enter a Player Name' unless settings.tml has [client] [client.identity] name; the DM client's character list is empty (pregenerated or not) unless a .bic is in the scratch user dir's dmvault (copy, never link, the install's data/dmv/dungeonmaster.bic). In the Creator a tree node opens on a quick double click (xdrive.py click --double), not on its '+'; a click needs the pointer moved there first. MG_JOINED_HOLD keeps the client while you drive it from the shell. After an NWSync download the client ends itself (exit 0, 'We disconnected you from the server while NWSync is running'); its own restart never comes back in the sandbox; MG_CLIENT_RUNS starts it again, and it finds the server, shows the character list and ends again: a player in the game after a download is not reached. INNER in run-server-and-client.sh is single-quoted: an apostrophe in a comment there breaks the script silently (the test then waits on a server that never starts). A talk table's text is the language's codepage bytes: mg_tlk::TlkEntry::text takes raw bytes, so encode Polish first. To read which draws lights reach and how many, patch inc_standard to paint MgValue(1) (numLights) on every lit fragment, every other four columns so the scene stays visible (batch3.rs light_count_shader): a draw takes up to graphics.lighting.max-lights (32 by default).
