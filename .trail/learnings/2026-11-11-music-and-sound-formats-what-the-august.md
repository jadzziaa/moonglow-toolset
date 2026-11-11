---
title: "Music and sound formats: what the game plays, and a WAVE holding MP3"
author: August
date: 2026-11-11
branch: develop
cost: { minutes: 56, tokens: 305197 }
touches:
  - CLAUDE.md
  - apps/mg/tests/groups/cli.rs
  - crates/mg-audio/Cargo.toml
  - crates/mg-audio/src/lib.rs
  - crates/mg-corpus-tests/Cargo.toml
  - crates/mg-corpus-tests/tests/audio.rs
  - crates/mg-ui/tests/app/area_view.rs
  - crates/mg-ui/tests/app/main.rs
  - docs/PLAN.md
  - docs/findings.md
---

## Decided

- A sound's format is decided in one place, `mg_audio::container` (`crates/mg-audio/src/lib.rs`), from its bytes; the name and resource type play no part. Every reader goes through it (`Moonglow::play_sound` in `crates/mg-ui/src/audio.rs`, the speakers in `apps/moonglow/src/speakers.rs`, the hak editor's View). Fix a format there, not in a view.
- The report "WAV isn't accepted (a .wav renamed to .bmu)" was a RIFF WAVE holding MP3 (`fmt ` tag 0x55), which Symphonia's WAVE reader refuses. `container` now reads such a file as MP3 from its `data` chunk, finds an MP3's first frame (`find_mp3`: ID3v2 tags skipped, a frame header confirmed by the next, up to a mebibyte on), and a WAVE's RIFF size is not believed (Symphonia is given 0xFFFFFFFF when it says less than the chunks need).
- What the game plays was read in the client, not taken from the wiki (`crates/mg-corpus-tests/tests/client_audio.rs`, by hand; the table is in `docs/findings.md`, "What the game plays as music and as sounds"). Music: only a `.bmu`, only MP3, in any wrapper (BMU header or none, ID3v2 of any size, junk in front, inside a WAVE); never a PCM or ADPCM WAVE. Ambient sound rows: a `.wav` holding a WAVE (PCM, IMA ADPCM), or a `.bmu` holding MP3; no MP3 under `.wav`, not even the game's own. `PlaySound`: `.wav` only, a WAVE or `BMU V1.0` + MP3 (the header is needed), mono.
- The client can be heard without an audio device: `MG_CLIENT_AUDIO_OUT=<file under SCRATCH>` makes `tools/nwclient/run-client.sh` give it OpenAL Soft's `wave` backend, which writes all it plays to a WAV file. Authored tones of distinct frequencies (Goertzel) tell "played" from other game audio; a known-good track after each doubtful form tells a refusal from a stopped player.

## Tried and ruled out

- Looking for a place that goes by the extension: there is none. No dialog filter, import, pack, picker or Verify check reads a sound's content or refuses `.wav` or `.bmu`.
- Plain WAVs: every PCM width, float, A-law, mu-law, MS and IMA ADPCM, extensible, with LIST, bext, id3, JUNK chunks, decoded before the fix. The report does not reproduce with a PCM WAVE.
- Enabling Symphonia's `id3v2` feature for large tags: not needed once the tag is skipped before probing.
- The first client runs' "silent" rows after the last track that played proved nothing (a bad file might have stopped the player); they were rerun with a working track after each group.

## Watch out

- Moonglow still plays forms the game is silent on (a PCM WAVE as music, a music track only as `.wav`, an MP3 under `.wav` as an ambient sound, a sound only as `.bmu` by a Play button) and says nothing: whether to warn was put to the owner on 2026-11-11. `sound_data` looks up `.wav` then `.bmu` for every channel; the game does not.
- Stereo files are silent by `PlaySound` (stereo PCM too), so use mono controls there. A stopped ambient sound fades for longer than 6 s and bleeds into the next measurement. Music stopped alone restarts after about a second: empty the row first.
- An MP3 for a test needs no encoder and no game data: MPEG-1 layer III frames of 417 bytes, header `FF FB 90 C4`, the rest zeros, decode as silence.
- Other sessions' test clients (with `-userdirectory`) share the off-screen display; only a client with no `-userdirectory` is the owner's game.
