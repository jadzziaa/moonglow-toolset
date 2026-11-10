---
type: Research Note
title: The DXT1 and DXT5 encoder measured against Neverblender's
description: How mg-image's BC1 (DXT1) and BC3 (DXT5) encoder was checked - four pictures made in a test (photo-like noise, flat colors, a soft-edged cut-out, gradients with alpha in waves), PSNR of each after encoding and decoding, against Neverblender's numpy encoder on the same pixels, with both a rounding-down and an exact decoder; and Pillow and The Domains of Dread's decoder reading what it writes.
tags: [textures, dds, dxt, encoder, measurement]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-10T16:01:14Z }
sources:
  - id: encode-quality
    resource: crates/mg-image/tests/encode_quality.rs
    title: "The test that makes the pictures and holds the floors"
  - id: nvb-write-dds
    resource: ~/Projects/neverblender/tools/fbx/nwn_textures.py
    title: "Neverblender's write_dds, the reference encoder (numpy)"
  - id: tdod-decode-dxt
    resource: ~/Projects/tdod-modulework/tools/inventory/style/common.py
    title: "The Domains of Dread's _decode_dxt, the second decoder"
  - id: nwn-wiki-dds
    resource: https://nwn.wiki/spaces/NWN1/pages/3473496/DDS
    title: "nwn.wiki: DDS (both headers)"
verified:
  - { by: claude-code/claude-opus-5-5, at: 2026-11-10T16:01:14Z }
---

# The DXT1 and DXT5 encoder measured against Neverblender's

`mg_image::encode::compress` encodes a picture as BC1 (DXT1) or BC3
(DXT5) with its mip chain; `dds::write_bioware` and `dds::write_standard`
put it in either container. The asset scripts of The Domains of Dread and
Neverblender encoded with Neverblender's `write_dds` [nvb-write-dds], so
the encoder had to do at least as well as that.

## How it was measured

The test `encoded_pictures_keep_their_quality` [encode-quality] makes four
256×256 pictures and writes them as PNGs, with Moonglow's BioWare DDS of
each, into `target/tmp/encode-quality/`:

- **photo**: smooth value noise of six octaves in three correlated
  channels, with fine detail on top (DXT1).
- **flat**: tiles of flat colors that are not 565 values, with discs of
  another color over them (DXT1).
- **cutout**: leaves (discs) whose edges go from clear to solid over a
  texel and a half, magenta under the clear texels (DXT5).
- **gradient**: color ramps across the picture and alpha in waves a few
  texels from clear to solid (DXT5).

A scratch script then encoded the same PNG pixels with `write_dds` (`bpy`
stubbed out; it only loads images) and decoded both encoders' files with
two decoders: The Domains of Dread's `_decode_dxt` [tdod-decode-dxt], which
interpolates exactly, and one that rounds down as `mg_image::bc::decode`
does. Color PSNR is over the texels with alpha 128 and up (those that
show), alpha PSNR over all.

## Results

The largest level, in dB (exact decoder / rounding-down decoder):

| Picture | Format | Moonglow color | Neverblender color | Moonglow alpha | Neverblender alpha |
|---|---|---|---|---|---|
| photo | DXT1 | 41.04 / 41.12 | 40.42 / 40.30 | | |
| flat | DXT1 | 49.06 / 50.07 | 42.17 / 42.19 | | |
| cutout | DXT5 | 43.49 / 43.57 | 42.97 / 42.76 | 40.73 / 40.79 | 38.95 / 38.93 |
| gradient | DXT5 | 45.26 / 45.45 | 45.15 / 44.91 | 47.16 / 47.68 | 46.37 / 46.10 |

Moonglow's is better on every picture with either decoder. The gain is
largest on flat colors (about 7 dB: a block of one color takes the pair of
endpoints whose blend comes nearest it, where Neverblender rounds the color
to 565) and on soft alpha edges (about 1.8 dB: both of DXT5's alpha modes
are tried, the six-step one keeping 0 and 255 exact). A first version that
scored alpha only against the rounding-down decoder lost to Neverblender
under the exact one (45.98 dB against 46.37 on gradient); alpha is now
scored against both.

The test's floors sit just under Moonglow's own figures, so a change that
makes the encoder worse fails it.

## What reads the files

- Both of Moonglow's files of each picture decode with `_decode_dxt`
  (BioWare's header: width, height, 3 or 4 channels, the largest level's
  size, the alpha mean), and the mean is the picture's mean alpha, as it
  is in the game's own files (c_azer.dds: 0.6197 in its header, 0.6197
  computed; every 3-channel file has 1.0).
- Pillow reads the standard DDS (`.std.dds`) of each to within 1 of
  `_decode_dxt`'s rounding; `textures_match_pillow` (mg-corpus-tests) now
  gives Pillow the game's BioWare textures as `write_standard` writes
  them.
- Both containers hold the same rows in the same order, the bottom of the
  picture first, as the game uploads them [nwn-wiki-dds].
