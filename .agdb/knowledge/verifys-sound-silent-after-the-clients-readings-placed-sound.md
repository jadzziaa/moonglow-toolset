---
type: Decision
title: "Verify's sound-silent after the client's readings: placed sounds, battle music, row 200, long names"
tags: []
status: stable
generated: { by: trail, at: 2026-11-11T00:00:00Z }
touches: [crates/mg-audio/src/role.rs, crates/mg-audio/src/lib.rs, crates/mg-module/src/sounds.rs, crates/mg-module/src/doctor.rs, crates/mg-ui/src/audio.rs, crates/mg-corpus-tests/tests/audio.rs, crates/mg-corpus-tests/tests/doctor.rs]
fingerprint: v2-c0663ac6564641fc
created: 2026-11-11
author: August
description: "After the client's readings, Verify's sound-silent rule treats battle music as music and judges placed sounds by form (.wav, BMU, bare MP3) and stereo; row 200 and music names over 16 characters are silent, with stereo ADPCM unresolved."
asks:
  - "Why does Verify flag a placed sound object as silent when its file is a stereo WAVE but positional?"
  - "Which audio formats does the game actually play under a .wav name, and which MP3 variants are silent?"
  - "My music row in ambientsound.2da is silent though the file is in data/mus; could the resource name length be the cause?"
  - "Why does shipped_modules_have_only_known_findings fail daily for LavaLakeLoop and sludgelake, and why aren't they allowlisted?"
---
## Decided
- After the first learning ("Verify's sound-silent: area music and ambient rows by the role rules"), the client read the two inferred roles (2026-11-11, `client_audio`, `MG_AUDIO_FORMS=placed,battle`, `MG_AUDIO_AREAWIDE=1`): battle music is exactly music (started by script); placed sound objects are as `PlaySound`: `.wav` only, a WAVE or MP3 behind `BMU V1.0`, MP3 without the header (bare, tagged, in a RIFF 0x55) silent, and a positional object silent on a stereo PCM WAVE that an area-wide one plays. Nothing in `mg_audio::role` is inferred now but a conversation line's sound (taken as `PlaySound`'s).
- The rules take `mg_audio::Form` (Wave, BmuMp3, Mp3 = without the header; `container()` unchanged), and stereo is `silent_positional(role, form, channels, positional)`, Verify's alone (the log can't tell positional). Channels come from the `fmt ` chunk (`mg_audio::wave_channels`, `WaveInfo::channels`, bounds-checked); `form_by_start` gives form and channels from the file's start (`container_of_start` now asks for the tag and channels, 4 bytes of `fmt `).
- `sound-silent` also judges each sound of a GIT `SoundList` entry and of a UTS (`Positional`, default 0), at `SoundList[i] (TAG), Sounds[j]` / `Sounds[j]`; Missing and only-`.bmu` stay the missing references'.
- Read in the client too: `ambientsound.2da` row 200 (`al_mus_throne2.wav`, BMU + stereo MP3 under `.wav`) is silent in the game, as the rule said; `the_game_s_own_music_and_ambient_rows_are_heard` takes every row every day and asserts exactly that finding. A music `Resource` over 16 characters is silent (Wyvern Crown's row 83 `mus_wc_ruralguitar`, though `data/mus` has the file; `mus_wc_ruralg`, the same bytes, plays): the finding is back (`sounds::long_name_message`), for ambient rows by the same name limit, and Wyvern Crown's six are KNOWN in `doctor.rs`.

## Watch out
- Still the owner's call: the game's positional `lavalakeloop.uts` and `sludgelake.uts` name stereo IMA ADPCM WAVEs (`al_na_lavalake1.wav`, `al_na_sludglake1.wav`; only stereo PCM was read), and XP2_Chapter2's q3_area_sanctum places a positional LavaLakeLoop: `shipped_modules_have_only_known_findings` fails every day, `the_game_s_own_sound_blueprints_name_sounds_it_plays` under MOONGLOW_FULL_CORPUS=1. Not allowlisted.
- Stereo MP3 (by `PlaySound` or a placed sound) is not judged: no control in the game's files.
