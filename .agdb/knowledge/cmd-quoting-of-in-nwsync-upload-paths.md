---
type: Gotcha
title: cmd quoting of % in NWSync upload paths
description: "How a path with % is quoted for cmd /S /C in nwsync::shell_quoted_for, and why ^% inside quotes was wrong."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-24T01:44:51Z }
touches: [crates/mg-module/src/nwsync.rs]
asks:
  - how is a percent sign escaped in a cmd command line?
  - why is ^% inside double quotes wrong?
  - how does Moonglow quote the NWSync upload folder on Windows?
fingerprint: v2-1d0fa06de9faedfc
created: 2026-11-24
author: augustaugur
---
Upload commands run on Windows as `cmd /S /C "text"` (apps/mg main.rs, mg-ui project_commands.rs), command-line context, not a batch file. Inside double quotes a caret is literal, so the branch's `"...^%..."` put a caret into the path. shell_quoted_for now splits on `%` and joins quoted parts with `^%` outside the quotes: `"C:\100"^%" done\\"`. cmd removes the caret; `%"…"^%` names no variable, so nothing expands. Backslashes ending each quoted part are doubled before its `"` for the program's argv parsing. Not run on Windows here: waiting on a Windows machine.
