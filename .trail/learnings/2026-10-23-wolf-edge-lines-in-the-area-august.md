---
title: "Wolf edge lines in the area view: the game's translucence"
author: August
date: 2026-10-23
branch: wolf-edges-area
touches:
  - crates/mg-corpus-tests/tests/game_shaders.rs
  - crates/mg-corpus-tests/tests/client_render.rs
  - crates/mg-ui/tests/screens.rs
  - docs/deferred.md
---

## Decided
- The wolf's (dd_n2e_wolfdire) light lines along fur-card cut edges in the AREA view, game shaders on, are the game's own: inc_lighting's translucence at SHADER_QUALITY_MODE 2 (Diffuse += C*att*(-NdotL)*(1-alpha) when NdotL<0) lights the kept edge texels (0.2<a<1) of a BACKLIT card. Not alpha (0 pixels alpha<255 in a direct area render), not egui, not MSAA (samples 1 too), not the pose, not selection (an overlay only).
- Client verdict (creatures_look with the wolf, now writing SETTINGS = High Quality, MG_LIGHT=ambient,diffuse hex, MG_CLIENT_CAMERA close): the client draws the same lines; a copy of fslit with `#define LIGHT_TRANSLUCENCE_VALUE 0.0` before the include in the override removes them in both. The earlier 'client is clean' run had no settings.tml: Performance (quality 1), no translucence.
- TDoD's inc_lighting (tdod_shaders.hak, diffuse wrap: NdotL=(NdotL+1)/4 >= 0) never fires translucence; there the fur card (fslit, vertex normal, softening) shows lighter than the normal-mapped body (fslit_nm) under a bluish sun ambient. The client does the same (MG_LIGHT=664D4D,FFFFFF with TDoD's inc_lighting in MG_OVERRIDE). No renderer change.
- Held by a_backlit_cut_out_s_edge_is_lit_through_by_the_game_s_programs (mg-corpus-tests models group): stock programs edge 98 vs solid 35; WGSL equal (it sets a cut-out's alpha 1 before lighting).
## Tried and ruled out
- Env map from appearance.2da ENVMAP: the wolf's row (31002) has ****.
- Mip bleed: the DDS's transparent texels are dark (0x2104), edge texels' RGB ~ the opaque ones'.
## Watch out
- Reading what a game program computes: a marked copy in the user override works in Moonglow too (screens.rs area_creatures_game_shaders takes MG_SCREEN_OVERRIDE); paint `FragmentColor.a < 0.9` red to find edge texels.
- The light in a new ttr01 area by day is a bluish ambient (0x643232) and a white sun from (4000,4500,7000): a creature seen from its far side is backlit.
- The installed mgv is pinned to older toolset crates: its pictures still show c22accb's alpha bug.
