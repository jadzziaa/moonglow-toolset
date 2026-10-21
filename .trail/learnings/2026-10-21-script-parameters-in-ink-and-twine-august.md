---
title: Script parameters in Ink and Twine exports
author: August
date: 2026-10-21
branch: worktree-agent-abbc011f37a81dad0
cost: { minutes: 177, tokens: 1268441 }
touches:
  - crates/mg-module/src/dialog_io.rs
---

Conversation parameters (ConditionParams on a link, ActionParams on a node) go out to Ink and Twine after the script's name as name=value words, quoted where a space is, with %, the quote, the backslash, line breaks and the characters Ink or Twine reads as syntax (# / { } [ ] | < >) percent-escaped (PARAM_ESCAPED in dialog_io.rs).

Percent, not Ink's backslash, on purpose: what inklecate does with a backslash inside a tag could not be checked (no inklecate on the machine), and percent escapes are inert text in any Ink tag, Twine link or brace. The readers' own scans (ink_uncomment's //, rfind("->"), ink_find('#'), reply_label's braces, parse_twee's [[ ]] and header [ and {) then never meet a raw syntax character in a value.

Ink conditions: {c} stays the condition; its parameters are a `# if:c k=v` tag on the choice, and for a conditional divert a `# if:` tag on its own line just before it (a tag after `{c: -> E}` on the same line is probably valid Ink but unverified). The reader binds a pending if: to the very next line, which must be that divert, and refuses a tag naming another condition.

Twine passage headers are whitespace-split, so an NPC line's action there is a query, `do:a_give?item=gold%20coins&k=v`; apply_tags takes the query form only when the part before '?' has no whitespace (an Ink value may hold '?'). Output without parameters is byte-identical to before.

Strings (parameters, speaker, quest, comment) are Windows-1252 both ways (CODEPAGE), as new_node writes a line's text. dialog::params_value (used by the Conversation Editor) writes UTF-8: a separate bug, left in docs/deferred.md.
