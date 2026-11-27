---
type: Fact
title: "Static see-through order is the client's texture alpha-mean sort"
description: "Static placeables' alpha<1 meshes: client draws static buckets by texture, sorted by CAurTextureBasic+0x30 alpha mean (highest first), before dynamic Gobs; ties are std::sort (unmatchable). Confirmed by batch9_static_alpha_mean."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T00:38:10Z }
asks:
  - Why does the client draw one static see-through card before another regardless of distance?
  - "How does Moonglow order static placeables' see-through meshes?"
  - "Where does a texture's alpha mean come from in the client (DDS header, TXI alphamean, TGA)?"
  - Why did three client batches of opaque cards find no static order rule?
created: 2026-11-27
author: augustaugur
---
## Found (26 Nov 2026, from nwmain-linux with objdump, owner's leave; confirmed in the client)
- Scene::AddPartsToDrawBuckets (0x4751d0) walks meshbuckets in AurTextureGetOrdering's TextureIndices order, appending static parts to draw buckets, then sorts dynamic Gobs and appends theirs. So static before dynamic, static by texture.
- TextureIndices = static texture list sorted by std::sort, comparator 0x4842b0 (+0x30 greater first).
- +0x30: -1 at construction; TXI alphamean (ParseField); BioWare DDS header (LoadImage, after TXI, so header wins); PLT 0.5 (ConstructImage); still -1 at BuildAndStoreTexture: compressed -> 1.0 (0.5 with 4 channels), else ImageGetAlphaMean (f32 sum a/255 / count; 1.0 for <=3 channels).
- Ties (all opaque textures, mean 1) are std::sort's order: not reproducible. That is why batch9_static_order/tiles found "one texture first in every pair, no rule".
## Moonglow
- renderer.rs client_alpha_mean (TextureEntry/Surface.alpha_mean), see_through_order -> Draw.order between hint and depth in the blend key (hint now 13 bits). Applies to ordered draws with receives_shadows && !tile (static placeables). Ties keep origin depth. Tiles not changed (untried in client).
## Watch out
- batch8d's solid() / mg_image::write_tga write 24-bit TGAs: alpha is dropped (mean 1). Use batch9::tga32 for alpha.
