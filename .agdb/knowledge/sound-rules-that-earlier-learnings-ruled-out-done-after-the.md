---
type: Decision
title: "Sound rules that earlier learnings ruled out, done after the client read them"
tags: []
status: stable
generated: { by: trail, at: 2026-11-11T00:00:00Z }
touches: [Cargo.lock, Cargo.toml, apps/mg/src/main.rs, apps/mg/tests/sounds.rs, apps/moonglow/Cargo.toml, crates/mg-audio/Cargo.toml, crates/mg-audio/src/lib.rs, crates/mg-corpus-tests/Cargo.toml, crates/mg-corpus-tests/tests/audio.rs, crates/mg-corpus-tests/tests/client_audio.rs]
fingerprint: v2-63320049bc4e3126
created: 2026-11-11
author: August
description: "Three earlier ruled-out sound rules are now deliberate after the client read them: Verify's `sound-silent` reads sound bytes, headerless MP3 and positional stereo WAVE are flagged silent, and long `Resource` names are flagged again."
asks:
  - "Why does `trail check` report conflicts with the earlier sound learnings, and should I change the code to fix them?"
  - "Why does Verify's `sound-silent` flag a music row with a long name like `mus_wc_ruralguitar` when a shorter name plays?"
  - "Which sound files does the game client play silently: headerless MP3 under `.wav`, or stereo WAVE on a positional placed sound?"
  - "Can a Verify check read a sound's content, and how does `mg-module` get the first bytes without decoding?"
---
## Decided

Three things earlier learnings of 2026-11-11 ruled out are now done on purpose, each after the game client read it (`client_audio`); those learnings recorded what was known when they were written.

- "No Verify check reads a sound's content" (the learning on music and sound formats) was a fact of that morning. Since the owner's word the same day ("The Verify check may be built"), Verify's `sound-silent` reads a sound's first bytes (`mg_audio::form_by_start` through `ResMan::get_start`), and `mg-module` depends on `mg-audio` without its `decode` feature.
- "Warning about more than the five" (the learning on sounds the game is silent on) ruled out a headerless MP3 under `.wav` outside the ambient role and stereo, as not settled. Both were then read: an MP3 under `.wav` without the `BMU V1.0` header is silent by `PlaySound` and from a placed sound (`Silent::HeaderlessMp3`, in the log warning and in Verify), and a positional placed sound is silent on any stereo WAVE, PCM and IMA ADPCM alike (`silent_positional`, Verify only: the log does not know whether a placed sound is positional). Stereo by `PlaySound` and stereo MP3 are still not judged.
- "Flagging a `Resource` longer than 16 characters" (the first learning on Verify's `sound-silent`) was dropped as a rule of our own, then restored once the client read a music row naming `mus_wc_ruralguitar` silent while `mus_wc_ruralg`, the same bytes, played.

## Tried and ruled out

- Quieting the check where it fired on the game's own content (an ambient row, a long name in Wyvern Crown of Cormyr, two positional blueprints with stereo files): each was sent to the client instead, each was silent there, and each is now an expected finding pinned in a corpus test.

## Watch out

- `trail check` reports these three as conflicts with the learnings above. They are intended; say so in the commit message rather than changing the code.
