---
type: Gotcha
title: "codemode edit()/write() return promises: unawaited edits to one file race and lose changes"
description: "Why edits made through the codemode MCP's edit() or write() went missing, and how to apply many edits to one file safely."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T14:52:58Z }
asks:
  - Why did some of my codemode edits disappear?
  - How do I apply several edits to one file with run_code?
  - Why did a break test leave the break in the file?
created: 2026-11-23
author: augustaugur
---
Met twice in 5.2.0 (W7's break runs, W5's implementation): codemode's edit() and write() are async. Called without await in a loop, several edits of one file read the same old text and each writes its own result, so all but the last are lost (W5 lost edits in shell.rs, settings.rs, script_panels.rs, script_nav.rs, script_tools.rs and script_reference.rs and had to re-apply them, checking each with grep). Await every edit/write, or do one string transformation and one awaited write. For break runs, revert by the reverse edit of the current text (not by writing a copy saved earlier: other writers may edit the same file meanwhile), in a finally.
