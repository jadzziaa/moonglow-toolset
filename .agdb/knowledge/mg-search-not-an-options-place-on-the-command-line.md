---
type: Decision
title: "mg search --not: an option's place on the command line"
tags: []
status: stable
generated: { by: trail, at: 2026-10-30T00:00:00Z }
touches: ["apps/mg/src/search.rs#negated_from", "apps/mg/src/main.rs#main"]
created: 2026-10-30
author: August
description: "mg search's --not needs argument positions, which clap derive discards, so main uses get_matches plus Args::negated_from with indices_of; --not must be an Append of bools, and value_source must be checked."
asks:
  - "How can I find where --not appeared relative to other options in mg search when clap derive loses the order?"
  - "Why does indices_of return only one index for my ArgAction::Count flag, so repeated --not occurrences get lost?"
  - "Why does indices_of report a position for a flag the user never passed, and how do I avoid it?"
  - "How does mg search handle 'not' and 'any' conditions in the query, and why is no universe of objects needed?"
---
mg search's --not negates the condition after it, so it needs where options were given: clap derive drops that, so main parses with CommandFactory::command().get_matches() + FromArgMatches and hands the search subcommand's ArgMatches to Args::negated_from (indices_of). Two traps: an ArgAction::Count flag keeps only one index (the last), so --not is an Append of bools (num_args 0, default_missing_value true) to keep one index per occurrence; and a flag not given still answers indices_of with its default's place, so check value_source == CommandLine first. The Query side (mg_module::search) keeps 'not' as separate searches, leaving out exactly what an AND of them keeps (Near::meets), and 'any' as a union sorted stably by resource key; the first condition is never negated, so no universe of objects is needed.
