---
type: Gotcha
title: Client mip levels are plain means; static see-through order is not by distance; read overlaps by pixel counts
description: "The client makes a TGA's mip levels as plain (unweighted) means, so Moonglow's alpha-weighted levels were changed for the render path; static placeables' see-through meshes are not ordered by distance in the client, and window-based overlap readings of client pictures misread them."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-26T23:55:08Z }
touches: [crates/mg-render/src/texture.rs, crates/mg-image/src/encode.rs, crates/mg-corpus-tests/tests/client_render/batch9.rs, crates/mg-corpus-tests/tests/client_render/batch8d.rs]
asks:
  - "Does the game client weight a TGA's mip level colors by alpha?"
  - Why does a distant soft edge show the color under transparent texels?
  - "How does the client order two static placeables' see-through meshes?"
  - Why did a client reading of which card was drawn first come out wrong?
fingerprint: v2-99f785c5e48d8ea6
created: 2026-11-26
author: augustaugur
---
- batch9_soft_edge_levels (2026-11-26): a TGA field of opaque white / transparent green texels is light green from afar in the client: its levels are plain means. Moonglow's half() and the read-time compression (compress_plain_levels) now average plainly; the encoder's compress/compress_with (mg texture convert) stay weighted by alpha on purpose (files written for the game keep stored levels).
- Dynamic placeables' alpha<1 meshes are ordered by the instance origin (not the mesh's place) in the client, as Moonglow's instance_depth does. Static placeables are NOT ordered by distance: one texture first in every static pair of a scene (red in batch9, blue in batch8d). A fixed texture/bucket order and a per-texture group sort are not told apart (both batches' layouts tie per texture). Moonglow's static order differs in half the pairs; left open in deferred 'See-through meshes'. batch9_static_order (lone cards, list varied, names swapped) also ruled out: name order, first-listed, last-listed, card count, mean place, nearest/farthest card. Not separated: a set's place from the eye, and the tile its cards stand on (statics merge into tiles; keep a test's sets off tile boundaries and at the same x next time). batch9_static_tiles then ruled out tiles too: one pair of textures drew blue first in all 16 pairs (inside tiles near/far, across x 20 either way, across y 30). One order per texture pair over the whole area, not predicted by name or list; maybe a hash or load-address order, not read.
- Gotcha: the fitted camera puts the client's picture a few pixels off. Small sampling windows at projected overlaps misread batch8d (it reported 'own' static pairs farthest first; wrong). Count pixels of both colors over each pair's whole box (batch9::mixed_pixels, threshold 100: client 0 or 280+, Moonglow edge noise up to 30), and locate far cards by their pixels before reading them.
- cargo build -p mg-corpus-tests --tests does not build client_render; use cargo test --test client_render --no-run.
