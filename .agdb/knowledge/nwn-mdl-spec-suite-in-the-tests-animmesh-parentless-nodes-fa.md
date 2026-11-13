---
type: Decision
title: "nwn-mdl-spec suite in the tests; animmesh, parentless nodes, face records"
tags: []
status: stable
generated: { by: trail, at: 2026-10-29T00:00:00Z }
touches: [apps/mg/tests/models.rs, crates/mg-mdl/src/ascii_write.rs, crates/mg-mdl/src/ascii.rs, crates/mg-mdl/src/binary_write.rs, crates/mg-testkit/src/lib.rs]
fingerprint: v2-0d43418f8922c410
created: 2026-10-29
author: August
description: "Explains how the nwn-mdl-spec conformance suite runs in apps/mg/tests/models.rs and the compiler/decompiler decisions it fixed: face records, neighbours, bounds from source text, animmesh verts/tverts repetition, parentless node exclusion."
asks:
  - "How do I run the nwn-mdl-spec conformance suite, and where does it find the spec checkout?"
  - "Why does the game divide by zero on an animmesh node, and how does the decompiler avoid it?"
  - "My model nodes disappear after compiling and the numbering looks off; what happens to a node with no parent line?"
  - "How are face normals, adjacency neighbours and bounds computed when compiling MDL, and why is -1 used past the kept count?"
---
The nwn-mdl-spec conformance suite runs in apps/mg/tests/models.rs (the_model_spec_suite_passes) through its own Python runner (runner/mdlconf.py run|stock --cmd '<mg> --no-user-dir model'), so mg has 'mg model compile|decompile IN OUT'. The checkout is read from NWN_MDL_SPEC or ~/.local/share/moonglow-oracle/nwn-mdl-spec (mg_testkit::mdl_spec), not vendored (fixtures hold only data we author). The runner passes PASS with SHOULD notes; the test also fails on the SHOULD paths already fixed (faces.normal/distance/adjacent, radius, average, nodes.tree). Decided: the game's compiler takes a mesh's face records (plane of the source face of the same number, degenerate normal +X) and neighbours (by the text's vertex numbers, either direction, lowest face among those kept, else -1) and bounds (every source vertex, no dedup) from the text before merging: ascii::SourceMap::meshes carries them to binary_write::write_from. The -1 rule for neighbours past the kept count was fitted to zspec_merge's degen (one data point); it holds every probe. Watch out: an animmesh animation node without its own verts/tverts makes the game divide by zero (spec E54): the decompiler repeats verts, tverts and faces there, and keeps UV-split vertices apart so the frames are per compiled vertex. A node with no parent line (or PARENT) is left out with its subtree; numbering counts it (an index past the nodes in number_nodes' order consumes a number). Ruled out: changing mg-render for MDL-SKIN-030 (c_wings2's 102 boneless vertices) without seeing the client first: a w=0 collapse renders differently by GPU.
