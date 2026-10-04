---
okf_version: "0.2"
---

Documentation of Moonglow Toolset, the reimplementation of the Aurora Toolset for Neverwinter Nights: Enhanced Edition: the plan and what was deferred, the findings about the game and Aurora, the user manual, the research behind the code, the Aurora parity inventory and checklists, and the plugin author's material. The overview is in the repository's `README.md`; rules for agents are in `CLAUDE.md`. Screenshots used by the README are in `images/`. History: [log.md](log.md).

<!-- okf index: generated; edit freely, or rerun `okf index` -->

# Backlog

* [Deferred: what was left out, for a decision later](deferred.md) - What the post-parity work left undone, by area (script intelligence, nasher projects, content doctor, test loop, scale, EE fields, bulk editing, conversations, palettes, custom content, haks, options), each awaiting a verdict - fix, add or drop.

# Finding

* [Findings](findings.md) - What Moonglow's authors learned about NWN:EE that is undocumented or documented wrongly - item costs, creatures, areas and tiles, talk tables and 2DAs, haks, GFF fields, what Aurora writes - each saying how it was checked, where Moonglow implements it and what nwn.wiki says.

# Plan

* [Moonglow Toolset: Plan](PLAN.md) - The plan of Moonglow Toolset with its current status - what of Aurora it reproduces, the landscape, principles, crate architecture, key designs, phases, testing strategy, licensing and risks.

# Proposal

* [Plugins: a proposal](plugin-proposal.md) - The proposal for Moonglow's plugins, accepted 2026-10-03 - what plugins are for and can add, how one runs (sandboxed Luau), the API, safety, runtimes compared, the groundwork, documentation and testing, and where the work stands.
* [Proposed corrections to nwn.wiki](wiki-proposal.md) - Proposed corrections to fifteen nwn.wiki pages that say something the game or Aurora does differently - for each, what the page says, what it should say and how that was checked - in order of how much trouble the current text causes.

# Subdirectories

* [manual](manual/index.md) - The user manual, a chapter per file, also built into the program (Help › User Manual, which shows each chapter without its frontmatter) and shipped beside it. (17 Manual Page, 1 Manual)
* [parity](parity/index.md) - Parity with Aurora: the generated inventory of every Aurora form and control, and the control-by-control checklists of Moonglow against it. (1 Reference, 1 Checklist)
* [plugins](plugins/index.md) - For plugin authors: the starting page, the API's changes, the example plugins (`examples/`) and the editor type file (`types/moonglow.luau`). (1 Changelog, 1 Guide)
* [research](research/index.md) - Research behind the code: briefs on NWN:EE's formats, rendering, tilesets and models, what Aurora computes and how that was measured, prior art, and builders' pain points. (15 Research Note, 1 Reference)
