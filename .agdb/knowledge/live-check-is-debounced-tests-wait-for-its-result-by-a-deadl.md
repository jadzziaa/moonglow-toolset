---
type: Gotcha
title: "Live check is debounced: tests wait for its result by a deadline, not settle"
description: "Why a UI test that edits a script must wait by a deadline for the check as you type, and what the Found as you type row and the Status Bar message do."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T14:52:52Z }
touches: ["crates/mg-ui/src/script_nav.rs:live_check", "crates/mg-ui/src/script_panels.rs:compiler"]
asks:
  - Why does settle return before the live error is updated?
  - "How does Compiler's Found as you type row redraw when the error changes?"
  - Why does the Status Bar show the whole live error?
fingerprint: v2-344a35fb2cdb1075
created: 2026-11-23
author: augustaugur
---
5.2.0 W5. The check as you type runs a moment after typing stops; `settle` (Moonglow::under_way) does not wait for it, so a test that types and expects the error to change loops with a deadline (sleep + step until `script_error_for_test` changes), then settles (script_bottom::compiler_lists_the_live_error_found_as_you_type).

- A `ScriptTools::made` bump when `Live.error` changes was tried and dropped: with the check made at the very frame the result comes in (the row gone, or a frame asked for), the test passes without it, so the result's own frame request redraws Compiler.
- Compiler's first row (`FOUND_AS_YOU_TYPE`, `live_row`) is the error whole, wrapped, a Button named "Found as you type: <error>", click to its line (the include's own line where the error is in one), `row_copy::row_menu`; Copy All (`messages_text`) puts it first. A floating window's bottom pane bar also says the error, so tests pick the lowest such Button in the pane.
- Status Bar: with a live error message the cap is infinite; what gives way to it and in what order is in the note "Status Bar beside a live error" (review ruling: the error shown whole, others yield only as far as it needs).
