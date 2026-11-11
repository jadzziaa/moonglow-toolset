---
title: "Verify's sound-silent: the stock hits are read and pinned"
author: August
date: 2026-11-11
branch: develop
cost: { minutes: 46, tokens: 141598 }
touches:
  - crates/mg-corpus-tests/tests/audio.rs
  - crates/mg-corpus-tests/tests/doctor.rs
  - crates/mg-audio/src/role.rs
---

## Decided
- The stock hits of `sound-silent` are expected findings, read in the client on 2026-11-11, and pinned: `the_game_s_own_music_and_ambient_rows_are_heard` (all 253 rows every day) asserts exactly `ambientsound.2da` row 200; `the_game_s_own_sound_blueprints_name_only_two_sounds_it_does_not_play` (all 301 blueprints every day, ~20 ms; renamed from `..._name_sounds_it_plays`) asserts exactly `lavalakeloop.uts` and `sludgelake.uts` (stereo IMA ADPCM, positional: silent as shipped, heard made area-wide, `MG_AUDIO_FORMS=stereo`); `shipped_modules_have_only_known_findings` KNOWN has Wyvern Crown's 6 (row 83, `mus_wc_ruralguitar`) and XP2_Chapter2's 1 (q3_area_sanctum's placed LavaLakeLoop; only the blueprint's fields were read). This supersedes the second learning's "still the owner's call".
- The stereo rule holds for every WAVE codec: PCM and IMA ADPCM were read, distance ruled out (min 1 max 10, 50, 100).

## Watch out
- A change to the role rules that makes any of these stop firing, or makes a new stock one fire, fails those tests on purpose: it means the rules no longer match the client's readings.
