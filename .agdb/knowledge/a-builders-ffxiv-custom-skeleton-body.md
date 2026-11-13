---
type: Decision
title: "A builder's FFXIV custom-skeleton body"
tags: []
status: stable
generated: { by: trail, at: 2026-11-04T00:00:00Z }
touches: [crates/mg-corpus-tests/tests/client_render.rs, crates/mg-mdl/src/lib.rs, crates/mg-preview/src/compose.rs, crates/mg-preview/src/files.rs, crates/mg-preview/tests/previews.rs, crates/mg-render/src/game.rs, crates/mg-render/src/renderer.rs, crates/mg-shader/src/lib.rs, crates/mg-testkit/src/lib.rs, docs/findings.md]
fingerprint: v2-900f69b1bc7fd560
created: 2026-11-04
author: August
description: "Decisions and pitfalls for building an FFXIV custom-skeleton body in Moonglow: parts are identified by Part::item not node name, 11 texture slots, GLSL uniform defaults, supermodel chain limit, and client loading-screen traps."
asks:
  - "How does the renderer tell a body part from a held item like a helmet, cloak or weapon on a custom skeleton?"
  - "Why does the client hang forever on the loading screen after I change appearance.2da rows 1 or 6 in a hak?"
  - "My creature built from nw_commale shows no body parts in either engine; what is missing and how do I fix it?"
  - "How many texture slots does the renderer support, and what happens to textures beyond texture10?"
---
## Decided
- A body part vs a held item is Part::item (creature.rs sets it for helmets, cloaks, weapons), not the node name: capart.2da may hang parts from any bone (j_sebo_a). compose.rs place_unguarded.
- Renderer SLOTS = 11 (texture0..texture10, the MTR's range per nwn.wiki); texUnit11+ is white.
- GLSL uniform initializers (uniform float x = 0.5;) are read from the assembled text (mg_shader::uniform_defaults, Program::defaults) and used as Key::ParamOr when the MTR has no parameter of that name. Stock material shaders have none (only fsfb* post-process ones).
- animations_scaled walks the whole supermodel chain, a seen-name set stopping loops, SUPERMODEL_CHAIN_LIMIT 256; files.rs Gatherer likewise.
## Tried and ruled out
- The idle animation lookup through a deep chain was not the cause: pause1 at link 2 bound fine by part number.
## Watch out
- Builder files: mg_testkit::builder_files(name) (MOONGLOW_BUILDER_FILES or ~/Projects/moonglow-toolset/target/builder-files); hard-link them into a scratch override, and remove the link before writing a changed copy (never write through it).
- In a client reading, the TestNewModule player is an elf woman (appearance 1, pfv0): a hak whose appearance.2da changes rows 1 or 6 to a race with no files for her leaves the client on its loading screen forever.
- nw_commale has no BodyPart_* fields: a part-based creature made from it shows no parts in both engines; copy CreatureLook::to_utc's fields in.
- A big override (1.4 GB) needs ~20 s before the client leaves its loading screen: MG_READY later.
