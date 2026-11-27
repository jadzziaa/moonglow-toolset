---
type: Gotcha
title: "Kept galleys must follow egui's font rebuilds (theme switch garbles them)"
description: "Why text kept laid out between frames (Arc<Galley>) is drawn garbled after a theme switch, and how caches key on fonts_made."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T22:47:09Z }
touches: [crates/mg-ui/src/dialog_view.rs, crates/mg-ui/src/script_layout.rs]
asks:
  - "Why is the conversation tree's text garbled in the light theme?"
  - How should a cache of laid-out text know egui rebuilt its fonts?
  - What does fonts_made / watch_fonts do?
fingerprint: v2-9afc61cd9a0b2002
created: 2026-11-26
author: augustaugur
---
A galley (Arc<Galley>) kept across frames holds UVs into egui's font atlas. egui makes its fonts (and the atlas) again when the theme switches (mg_uikit::apply installs fonts), when the atlas fills, or when text options change; a kept galley is then drawn from the old atlas's places, garbled. Every cache of galleys must key on crate::script_layout::fonts_made(ctx) (watch_fonts runs once a frame in Moonglow::ui) and pixels_per_point, as script_layout and nui_view/source do; dialog_view's cut_words did not until 2026-11-26 (seen in the Light theme's screens.rs pictures: conversation rows garbled). nui_view/skin's TextCache detects it by a probe galley's identity instead. A screens.rs run with MG_LIGHT=1 shows it, since the pictures switch theme after the window is first drawn.
