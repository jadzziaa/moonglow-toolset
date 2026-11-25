---
type: Decision
title: "Kit stacks and pictures: what mg-uikit holds and what stays in mg-ui"
description: "mg_uikit::stack (StackLayout/StackRules/StackZone, StackDrop/DockDrop, stack_heights/seams, KeptLayout) and mg_uikit::pictures (Pictures/Budget) came from mg-ui's area Right Dock and Thumbnails in 2026-11; what was kept app-side and the traps."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T17:17:56Z }
asks:
  - "Where does the area Right Dock's drop and normalize logic live now?"
  - "Why doesn't StackRules.all filter panels, and what must a dock that gives panels away set?"
  - "Why did Thumbnails' PictureReader not move into mg-uikit?"
  - How do I compare screens.rs pictures between two commits without the noise?
created: 2026-11-25
author: augustaugur
---
Decided (item 2, 2026-11-25): mg-uikit holds the dock stack's arrangement and arithmetic, generic over the app's panel type; mg-ui's AreaRightLayout delegates (stack(), AREA_RIGHT_RULES, From<StackLayout>) and its RON format is unchanged (no serde flatten: RON writes flattened structs as maps). DockLayout keeps its own fields and calls keep_front/keep_shares; KeptLayout is the same four fields for the viewer.
Watch out: StackLayout::normalized no longer filters to rules.all, only adds what's missing, so a panel moved in from another dock survives; a dock that gives panels away must use all: &[] or normalize puts the panel back. A drop on an empty stack makes its one section; apply's Column follows most_columns. The toolset registers its Right Dock alone (moves between docks are off unless the owner says).
Pictures: only bookkeeping moved (made/wanted/frame/let_go_unseen/Budget). mg-uikit must stay egui-only, and the viewer's own gallery::Pictures paints on the UI thread, so PictureReader, loading/ready/era and drawing stay in model_view.rs.
Comparing pictures: run both to the same MOONGLOW_TEST_OUTPUT path (move the old aside), compare with Pillow, and ignore diffs whose bbox is the Log's time column (~54 px wide), spinners, stdout/stderr order, 3D views and random tile variants. The pre-item baseline's one frame_perf failure was load; quiet runs pass.
