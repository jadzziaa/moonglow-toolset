---
type: Decision
title: "Installing updates from the app (issue 28): mg-update, signed sums, test seams"
tags: []
status: stable
generated: { by: trail, at: 2026-11-02T00:00:00Z }
touches: [Cargo.lock, Cargo.toml, apps/moonglow/src/main.rs, crates/mg-testkit/Cargo.toml, crates/mg-testkit/src/lib.rs, crates/mg-testkit/src/signing.rs, crates/mg-ui/src/updates.rs, crates/mg-ui/tests/app/updates.rs, crates/mg-ui/tests/screens.rs, crates/mg-update/Cargo.toml]
fingerprint: v2-f2d8aa68c061af00
created: 2026-11-02
author: August
description: "In-app updates are done by the mg-update crate with minisign-signed SHA256SUMS, staged downloads and undoable install steps; feature split keeps ureq and minisign out of shipped programs, and Windows/macOS installs remain untested."
asks:
  - "How does Moonglow verify and install an update downloaded from inside the app?"
  - "Why does THIRD-PARTY-LICENSES list minisign when it should only be a test dependency, and how was that avoided?"
  - "Where does the app get its public update key, and what happens if it is missing?"
  - "Restarting after an update relaunches the wrong binary inside the AppImage; what should be launched instead of current_exe?"
---
## Decided
Updates install from the app through the crate mg-update (apps/moonglow/src/update_check.rs is gone): releases, Kind (what installed this copy, from Facts), verify (SHA256SUMS signed with minisign, trusted comment "Moonglow Toolset <tag>"), stage (download to .moonglow-update-*.part beside what it replaces, then rename), install (steps as data: Step::Run/Rename/..., undone on failure). The ureq client is mg-update feature `http` (moonglow and mg-ui dev-deps only; mg links no ureq/rustls symbols even built together, checked with nm). The release server for tests is feature `serve` (127.0.0.1, Api::at); the tests signing key is mg_testkit::signing::TestKey so the minisign signing crate never enters the programs (third_party_licenses.py follows cargo metadata normal edges, and an optional dep enabled by a dev-dep shows up there). The public key comes from packaging/minisign.pub via apps/moonglow/build.rs (rerun-if-changed on the packaging folder, MOONGLOW_UPDATE_KEY, option_env!); absent, installing is unavailable. Restart for an update relaunches mg_update::Restart (the AppImage itself via $APPIMAGE, never current_exe inside the mount). The two switches went on Options > Updates, not General.
## Tried and ruled out
minisign as an optional dep of mg-update behind `serve`: listed in THIRD-PARTY-LICENSES because the license script resolves workspace-unified features.
## Watch out
App::ui in main.rs may draw a frame after Close: only overwrite Closing.relaunch/update when take() yields Some. Inno switch is /MOONGLOWRESTART=1 (not /restart, near Setup reserved names). Windows installer run, zip swap and macOS swap are unit-tested steps only: they wait on a Windows machine and a Mac.
