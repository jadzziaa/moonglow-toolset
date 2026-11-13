---
type: Decision
title: "Duplicate tags warning: builder's option, project rule wins, kinds alike"
tags: []
status: stable
generated: { by: trail, at: 2026-10-27T00:00:00Z }
touches: [crates/mg-ui/src/tag_clash.rs, crates/mg-module/src/tags.rs, crates/mg-module/src/project_file.rs, crates/mg-ui/src/options.rs, crates/mg-ui/tests/app.rs]
created: 2026-10-27
author: August
description: "The duplicate tags warning after paste or copy is a builder option (Settings.warn_duplicate_tags, off by default) that a moonglow.toml project rule overrides; the Custom rule warns on typed tags of any kind, and a store ResRef lookup bug was fixed."
asks:
  - "How do I turn the Duplicate Tags window on or off after pasting or copying an area?"
  - "Why did the duplicate tags warning not appear in my UI test, and why does tag_clash.is_none() pass?"
  - "Does moonglow.toml's paste_warning setting override the builder's Show duplicate tags warning option?"
  - "Why did pasting a store with its blueprint's own tag always trigger the duplicate tag warning before the fix?"
---
## Decided
- The Duplicate Tags window after a paste or an area's Copy… is Options › General › Show duplicate tags warning after a paste or a copy (Settings.warn_duplicate_tags), OFF by default: Aurora has none and builders share tags on purpose (spawn waypoints; Troubletcat's report). The window's Don't warn again sets it off at once and logs where to turn it back on.
- TagRules.paste_warning is Option<PasteWarning>: Some only where moonglow.toml sets it (the team's rule, which wins over the option). TagRules::paste_warning_or(builder_warns): unset -> Custom with the option on, Unique with it off (the project's unique/unique_in_area still said; none without them). mg_module::tags::clashes takes the resolved PasteWarning as an argument. TagClash.from_option says whether the window offers the tick.
- 'Only waypoints and triggers warn, not creatures' is the Custom rule, not a kind check: a tag typed in (not the blueprint's) warns, of any kind. Spawn waypoints have typed tags; creatures keep their blueprint's. Aurora keeps the blueprint's tag on placing (capture placement-defaults.mod).
- Real kind bug fixed: tag_clash::weigh looked the blueprint up by TemplateResRef for every kind; a store names it in ResRef (ObjectKind::resref_field), so a store pasted with its blueprint's tag always warned.

## Watch out
- UI tests that expect the window must set h.state_mut().settings.warn_duplicate_tags = true first (the harness default is off); tests asserting tag_clash.is_none() pass vacuously without it.
- Options pictures go to target/test-output/screens-options (scratch_dir), not screens/.
