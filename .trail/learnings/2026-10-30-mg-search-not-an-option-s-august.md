---
title: "mg search --not: an option's place on the command line"
author: August
date: 2026-10-30
branch: worktree-agent-a39b31ff7d63dc50a
cost: { minutes: 2406, tokens: 16786461 }
touches:
  - "apps/mg/src/search.rs#negated_from"
  - "apps/mg/src/main.rs#main"
---

mg search's --not negates the condition after it, so it needs where options were given: clap derive drops that, so main parses with CommandFactory::command().get_matches() + FromArgMatches and hands the search subcommand's ArgMatches to Args::negated_from (indices_of). Two traps: an ArgAction::Count flag keeps only one index (the last), so --not is an Append of bools (num_args 0, default_missing_value true) to keep one index per occurrence; and a flag not given still answers indices_of with its default's place, so check value_source == CommandLine first. The Query side (mg_module::search) keeps 'not' as separate searches, leaving out exactly what an AND of them keeps (Near::meets), and 'any' as a union sorted stably by resource key; the first condition is never negated, so no universe of objects is needed.
