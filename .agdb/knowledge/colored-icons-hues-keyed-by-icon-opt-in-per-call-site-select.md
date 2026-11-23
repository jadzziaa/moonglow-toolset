---
type: Decision
title: "Colored icons: hues keyed by Icon, opt-in per call site, selected rows keep on-fill (5.2.0 W7)"
description: "How Options › General › Colored icons is built (mg_uikit::hues): one table of kind hues keyed by the kit's Icon, a context flag, opt-in builders on tree rows and icon buttons, tint for rows that paint their own icons (Resources, Quick Open, hak editor), and why a selected row keeps on-fill."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T22:47:18Z }
touches: [crates/mg-uikit/src/hues.rs, crates/mg-uikit/src/tree.rs, crates/mg-uikit/src/icons.rs, crates/mg-ui/src/palette_view.rs, crates/mg-ui/src/browser.rs, crates/mg-ui/src/quick_open.rs, crates/mg-ui/src/hak_view.rs]
asks:
  - "Why are the module tree's icons all gray, and how do I color them?"
  - "Where are the object kinds' icon colors defined?"
  - "Why does a selected row's icon stay white with colored icons on?"
  - "How do I make a new view's kind icons follow the Colored icons option?"
fingerprint: v2-720e7820ff64d447
created: 2026-11-23
author: augustaugur
---
mg_uikit::hues holds KIND_HUES (Icon, dark, light) for twelve kinds: creature, encounter, area, sound, store, trigger, waypoint, door, script, placeable, item, conversation. Keyed by mg_uikit::Icon because mg-uikit knows no mg_area::ObjectKind and each kind has one icon everywhere (mg_ui::icons::object, browser::kind_icon). Journal, NUI, faction, model, texture, files and folders return None and stay text-2.

The switch is a temp bool in the egui context (hues::colored_icons(ctx, on), called each frame in Moonglow::ui beside smooth_scrolling; writes only when it changes). Views opt in explicitly: TreeRow::kind_icon / TreeGroup::kind_icon (mg-ui tree.rs: the Module panel's groups via group_icon and an area's kinds, which the Outliner shares; outliner.rs hak resource types), IconButton::kind_hue (the palette's type row), hues::kind_icon (the narrow palette's dropdown icon). Rows that paint their icon with Icon::paint themselves take hues::tint(ctx, icon, their old color) in the unselected branch: the Resources panel (browser.rs, an overridden copy keeps text-2), Quick Open (quick_open.rs, only enabled unlit rows; commands' Command/Plugin icons have no hue) and the hak editor's table (hak_view.rs). Tests for those find the icon by the strokes' colors inside the row (they are not recorded by record_icons). It is deliberately not global on Icon or IconButton: header buttons use Icon::Script etc. and must stay gray. Off, the paint path yields the same colors, so frames are identical (a kit test compares the shapes' Debug text).

Contrast: each hue is >= 3:1 on panel, hover-row, inset, bar, raised and raised-hover in its theme (dark luminance >= ~0.25, light <= ~0.145 because of raised-hover). A selected row (accent-fill) keeps on-fill: in the light theme no hue can be 3:1 on both the panel (#e4e7eb) and accent-fill (#2765a1); only a near-black could. Dimmed/quiet rows keep text-3 so they still read dimmed. The gallery cell's placeholder kind icon (text-off on thumb-bg) was left gray: it is the 'picture not made yet' state.

Trap met: running break-tests from a codemode script with un-awaited edit() calls raced and left breaks in the files; await each edit and revert in a finally.
