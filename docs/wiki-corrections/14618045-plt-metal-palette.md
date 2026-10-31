---
type: Wiki Edit
title: "PLT: Metal 2 is drawn in pal_armor01, never pal_armor02"
description: Ready-to-paste edit for the nwn.wiki PLT page, whose palette table guesses Metal 2 is drawn lighter in the game from pal_armor02.tga; the client draws Metal 2 from pal_armor01.tga and never reads pal_armor02.
tags: [nwn-wiki, corrections]
status: draft
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-31T03:30:00Z }
resource: https://nwn.wiki/spaces/NWN1/pages/14618045/PLT
sources:
  - id: client
    resource: crates/mg-corpus-tests/tests/client_render.rs
    title: "creatures_look, with pal_armor01 solid green and pal_armor02 solid red in the user folder's override (2026-10-31)"
  - id: page
    resource: https://nwn.wiki/spaces/NWN1/pages/14618045/PLT
    title: "nwn.wiki: PLT, version 33"
---

# PLT

https://nwn.wiki/spaces/NWN1/pages/14618045/PLT, edit written against version 33 (the local mirror's).

## Edit summary

Metal 2 is drawn from pal_armor01.tga, as Metal 1 is; pal_armor02.tga is not read.

## Change 1

**Find** ("Default Palette Files", the Metal 1 / Metal 2 row's Notes):

> pal_armor_02.tga seems to be a lightened version of 01. Presumably that PLT layer is a bit lighter / brighter in game even if the toolset shows the same options.

**Replace with:**

> pal_armor02.tga is a lightened version of 01, but the game doesn't use it: both metal layers are drawn from pal_armor01.tga, as the toolset's one selector for both suggests. (Checked in the EE client: with pal_armor01.tga solid green and pal_armor02.tga solid red in `override`, an armor's Metal 2 pixels are drawn green.)

## How it was checked

Moonglow's `creatures_look` (`crates/mg-corpus-tests/tests/client_render.rs`) starts a scratch module in the game client with a human and an elf in the game's `nw_aarcl001` armor, the user folder's `override` holding a solid green `pal_armor01.tga` and a solid red `pal_armor02.tga`. The client drew every metal pixel green, the Metal 2 dots on the legs and arms among them; a renderer reading `pal_armor02` for Metal 2 drew those red.
