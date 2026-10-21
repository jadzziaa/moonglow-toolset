---
okf_version: "0.2"
---

Documentation of Moonglow Toolset, the reimplementation of the Aurora Toolset for Neverwinter Nights: Enhanced Edition: the architecture and the record of the phases, what was deferred, the findings about the game and Aurora, the user manual, the research behind the code, the Aurora parity inventory and checklists, and the plugin author's material. The overview is in the repository's `README.md`; rules for agents are in `CLAUDE.md`. Screenshots used by the README are in `images/`. History: [log.md](log.md).

<!-- okf index: generated; edit freely, or rerun `okf index` -->

# Backlog

* [Deferred: what was left out, for a decision later](deferred.md) - What finished work left undone or unchecked, filed by area of the program (an entry's last words say which report, issue or review it came from), each awaiting a verdict - fix, add or drop; what is settled by decision or design is gathered at the end.

# Finding

* [Findings](findings.md) - What Moonglow's authors learned about NWN:EE that is undocumented or documented wrongly - item costs, creatures, areas and tiles, talk tables and 2DAs, haks, GFF fields, what Aurora writes - each saying how it was checked, where Moonglow implements it and what nwn.wiki says.

# Proposal

* [A 2DA editor and merger: a proposal](2da-editor-proposal.md) - Editing a 2DA's cells and rows inside Moonglow and taking rows from another hak's copy - what exists to build on, the decisions that are the owner's (where a table is edited, what a merger is, rows in the middle, cells by what they hold), four phases with their sizes and tests, what is not known yet, and what was built of it, with pictures.
* [Lights placed on the fly: a proposal](lights-proposal.md) - Placing a light in an area without making a custom placeable by hand first - what the game offers for lights, what was built and measured (the stock light placeables from the area's right-click menu, their reach in the client), and the open proposal - lights of any color and reach by generated content, scripted light effects, and two questions not yet measured.
* [Plugins: a proposal](plugin-proposal.md) - The decision record of Moonglow's plugins, proposed and accepted 2026-10-03 - what plugins are for, the principles, safety, the runtimes compared and why sandboxed Luau, the decisions and what was not proposed - with what shipped noted beside it. The API as it is now is in the manual and the API change list.
* [The game's own shaders in the area view: a proposal for 2.0](shaders-proposal.md) - Drawing areas, blueprints and models with NWN:EE's own GLSL shaders (the stock ones from the install, a hak's overrides of them, and the custom shaders MTRs name) as the centerpiece of Moonglow 2.0 - what the tabled shaders branch held, what the renderer is today, the corpus now on hand, a spike in which all 102 programs reached valid wgpu shader modules, the route (glslang, SPIRV-Tools, naga with two fixups), the uniform contract, costs, milestones with their tests, and the owner's decisions (accepted 2026-10-21 - the game's shaders throughout, native glslang, on by default, a failed program falls back and is reported); M0 built - the mg-shader crate (glslang-rs, the sampler split in Rust), its corpus test (102 of 102) and Verify's shader-compile check; M1 built - the stock lit pair and its variants drawn by mg-render beside its WGSL shader (opt-in), matching it within 1 in 255 on plain lit meshes, the WGSL port's first oracle, and the time uniforms' units measured in the client; M2 built - tier A, a material's own program pair drawn in the area view, the model viewer and the previews behind a Shaders switch (off by default), see-through meshes in Moonglow's two parts, the area's wind, tangents made where a model has none, The Domains of Dread's grass, leaves, water and roof shaders moving, and the client's floor matched where its inc_lighting applies; M3 built - the Shaders switch on by default, every view within 144 fps where Moonglow's own shader is (a large stock area zoomed out 5.97 ms on, 3.97 off; a synthetic TDoD area seen whole, over the budget either way, 54 ms on, 29 off), through uniforms split in IR into a draw's, its lights', the frame's and the bones', draws as instances of storage arrays, compiles on a worker thread and a disk cache.
* [Proposed corrections to nwn.wiki](wiki-proposal.md) - Proposed corrections to fifteen nwn.wiki pages that say something the game or Aurora does differently - for each, what the page says, what it should say and how that was checked - in order of how much trouble the current text causes.

# Reference

* [Moonglow Toolset: Architecture and Record of the Phases](PLAN.md) - What Moonglow Toolset reproduces of Aurora, the landscape, its principles, crate architecture and key designs, a record of the phases it was built in with the tests that hold each, the testing strategy and licensing. The plan it began as, kept as the description of what was built.

# Report

* [Audit: the documentation at 1.21.3](docs-audit.md) - An audit of all 46 documents in docs/ against the code at 1.21.3 - per document, whether it is correct, what was wrong and fixed, what is wrong and left, and whether it is still worth keeping (keep, update, trim, deprecate) - with the decisions it leaves to the owner.
* [Review: test necessity, test speed and codebase cleanliness](test-review.md) - A dated record (2026-10-11, version 1.21.1) of where the time between a finished change and a release went, the decisions taken about it, the measurements before and after, and the state of the code by size and shape. How the tests run today is in the plan's §7 and CLAUDE.md.

# Subdirectories

* [manual](manual/index.md) - The user manual, a chapter per file, also built into the program (Help › User Manual, which shows each chapter without its frontmatter) and shipped beside it. (18 Manual Page, 1 Manual)
* [parity](parity/index.md) - Parity with Aurora: the generated inventory of every Aurora form and control, and the control-by-control checklists of Moonglow against it. (1 Reference, 1 Checklist)
* [plugins](plugins/index.md) - For plugin authors: the starting page, the API's changes, the example plugins (`examples/`) and the editor type file (`types/moonglow.luau`). (1 Changelog, 1 Guide)
* [research](research/index.md) - Research behind the code: briefs on NWN:EE's formats, rendering, tilesets and models, what Aurora computes and how that was measured, prior art, and builders' pain points. (18 Research Note, 1 Reference)
