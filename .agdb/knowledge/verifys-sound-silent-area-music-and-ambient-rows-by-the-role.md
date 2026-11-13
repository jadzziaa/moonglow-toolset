---
type: Decision
title: "Verify's sound-silent: area music and ambient rows by the role rules"
tags: []
status: stable
generated: { by: trail, at: 2026-11-11T00:00:00Z }
touches: [crates/mg-audio/src/role.rs, crates/mg-audio/src/lib.rs, crates/mg-module/src/sounds.rs, crates/mg-module/src/doctor.rs, crates/mg-module/src/verify.rs, crates/mg-module/src/refs.rs, crates/mg-resman/src/container.rs, crates/mg-ui/src/audio.rs, apps/mg/tests/sounds.rs, crates/mg-corpus-tests/tests/audio.rs]
fingerprint: v2-206624d1b20844ec
created: 2026-11-11
author: August
description: "Verify's sound-silent warning (doctor::area_sounds) flags area music and ambient rows whose sound files the game client wouldn't play, using shared role rules in mg-audio's role.rs and start-of-file container detection."
asks:
  - "Why does Verify warn sound-silent for an area's MusicDay or AmbientSndNight field when the sound file exists in the module?"
  - "Where are the rules for which audio container formats the game plays as music, ambient or PlaySound defined?"
  - "How does Verify detect a .bmu file placed where the game only looks for wav or mp3, without reading whole files?"
  - "How do I build mg without Symphonia, and why does the mg-corpus-tests row 200 ambientsound test fail under MOONGLOW_FULL_CORPUS=1?"
---
## Decided
- The role rules live once, in `crates/mg-audio/src/role.rs` (`Role`, `Silent`, `silent_in_game`, `Role::looked_for`, `Role::lookup`), re-exported by `mg-ui/src/audio.rs` (`Channel::role`) for the log and used by `mg-module` for Verify. Roles: Music, BattleMusic (inferred: as music), Ambient, Other (PlaySound, measured), Placed (inferred: as PlaySound), Viewed. A new client reading changes the arms there; `refs.rs` takes `RefKind::Sound`'s types from `Role::Placed.looked_for()`.
- Verify's `sound-silent` (Objects group, Warning, as missing sounds are: the exit code is unchanged) is `doctor::area_sounds`, a `Once::AreaSounds` pass over each GIT's `AreaProperties` (`sounds::AREA_SOUNDS`: MusicDay/MusicNight/MusicBattle of ambientmusic.2da, AmbientSndDay/AmbientSndNight of ambientsound.2da, tables via `Doctor::table_shared`, i.e. the load order). `mg_module::sounds::unheard_by` finds the file in `Role::lookup` order, skips reading where both containers give the same verdict, else asks what it holds. One finding per area and field; row 0, blank, stingers and >16-character names are passed over; a row past the end is said.
- A track found nowhere is this check's finding, not a `verify::Missing` (a Reference has other consumers: unused, outside_references, rename, Find References). A placed/blueprint/dialog sound kept only as `.bmu` stays a Missing, with `Missing::found_as` (JSON `found_as`) and `what()` saying "missing (only X.bmu is there, which the game does not look for)".
- Bytes: `mg_audio::container_of_start(start, whole) -> Result<Option<Container>, usize>` and `container_by_start(read)` (4 KiB first; a bare MP3 needs its ID3 tags + 1 MiB + 2885 bytes; a RIFF up to its first `fmt ` tag) give exactly what `container(whole)` gives; `ResMan::get_start` reads only that (ERF pread, KEY mmap slice, MemContainer slice, and now `DirContainer::read_start`). `sounds::Judged` keeps each file's answer for the run, shared by the workers.
- mg-audio's Symphonia half is the feature `decode` (workspace dep has `default-features = false`; mg-ui, apps/moonglow and mg-corpus-tests ask for `decode`). `cargo tree -p mg --no-default-features -i symphonia` is empty; default `mg` still has it through mg-ui.

## Tried and ruled out
- `container(prefix)` as is: wrong for a `fmt ` behind a long LIST/JUNK chunk, an ID3v2 tag longer than the prefix, and a bare MP3 found far on, and a `None` from a prefix is not "no sound".
- Flagging a `Resource` longer than 16 characters: Wyvern Crown of Cormyr's ambientmusic row 83 names `mus_wc_ruralguitar`, and the game's `data/mus` has `mus_wc_ruralguitar.bmu` (and `mus_wc_ruralg.bmu`); what the game does was not read, so nothing is said.
- Measuring on TDoD's nasher project tells little: its 9 areas all use row 0 (the check costs 15 to 55 ms of worker time, no sound read). A stress module (target/sound-stress: Chess, 40 areas, 122 files in a 1.9 GB hak) costs 0.18 to 0.29 s of worker time, wall unchanged.

## Watch out
- The game's own `ambientsound.2da` row 200, `al_mus_throne2.wav` (BMU + stereo MP3 under `.wav`), draws a finding by the client readings: `the_game_s_own_music_and_ambient_rows_are_heard` (mg-corpus-tests formats, audio.rs) fails on it under MOONGLOW_FULL_CORPUS=1 (the everyday sweep of every 2nd row passes over it) until it is read in the client. No shipped module's area uses row 200.
- `mg_audio::container` stays at its path: `client_audio.rs` uses it.
- The doctor's GOLDEN hash test is untouched only because `a_world` has no sound tables; a world with them would change its findings.
