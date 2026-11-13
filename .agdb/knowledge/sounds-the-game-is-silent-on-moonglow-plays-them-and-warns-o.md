---
type: Decision
title: "Sounds the game is silent on: Moonglow plays them and warns once"
tags: []
status: stable
generated: { by: trail, at: 2026-11-11T00:00:00Z }
touches: [CLAUDE.md, apps/mg/tests/groups/cli.rs, crates/mg-audio/Cargo.toml, crates/mg-audio/src/lib.rs, crates/mg-corpus-tests/Cargo.toml, crates/mg-corpus-tests/tests/audio.rs, crates/mg-ui/src/audio.rs, crates/mg-ui/src/hak_view.rs, crates/mg-ui/src/lib.rs, crates/mg-ui/tests/app/area_view.rs]
fingerprint: v2-28bfe88283bd95d9
created: 2026-11-11
author: August
description: "Moonglow still plays sounds in roles where the game is silent and logs a warning once per file and role, decided by the pure function silent_in_game in mg-ui audio.rs, covering five cases; no Verify finding."
asks:
  - "Why does Moonglow play a sound that the game itself doesn't play, and where does it tell me?"
  - "Which format and role combinations are silent in the game, like a WAVE saved as .bmu?"
  - "How is the once-per-file audio warning tracked, and when does it reset after closing a module?"
  - "My music test fails because the log has a line starting with 'Sound'; what should music warnings be named?"
---
## Decided

- The owner ruled on 2026-11-11 (the question the learning "Music and sound formats: what the game plays, and a WAVE holding MP3" left open): Moonglow keeps playing a sound in a role where the game is silent, and says so in the log, once per file and role. No Verify finding was asked for or built.
- It is one pure function, `silent_in_game(role, found type, container)` in `crates/mg-ui/src/audio.rs`, with the whole matrix unit-tested there against the client's readings. Five cases: music that is a WAVE under `.bmu`; music that exists only as `.wav`; an ambient sound that is MP3 under `.wav`; an ambient sound that is a WAVE under `.bmu`; any other sound that exists only as `.bmu`. The hak editor's View has no role and marks only a WAVE under `.bmu`, which the game plays in none.
- `Moonglow::sound_file(role, name)` replaced `sound_data`: it looks in the game's order for the role (music `.bmu` then `.wav`; the rest `.wav` then `.bmu`) and returns the type found, which is what makes "exists only as" true. With both files present, music now plays the `.bmu`, as the game does.
- "Once" is `Moonglow::sound_said`, keyed by role, name, type found and the file's length (a replaced file is judged anew), cleared when a module is closed.

## Tried and ruled out

- Warning about more than the five: stereo by `PlaySound`, a headerless MP3 under `.wav` outside the ambient role, a WAVE cut short in its data. Either not settled in the client or not in the ruling.
- Asking `mg_audio::container` every frame in the hak editor's sound view for any file: an MP3's first frame is searched for (up to a mebibyte), so the view asks only of a file that starts with `RIFF`.

## Watch out

- A Verify finding for the same cases is the natural next step and would reuse `silent_in_game`, but Verify (`mg-module`) reads no sound's bytes today and has no `mg-audio` dependency: it is the owner's to ask for.
- The music warnings must not start with `Sound `: tests of music that plays assert the log has no `Sound {name}` line (that prefix is for a sound not found or not decoded).
