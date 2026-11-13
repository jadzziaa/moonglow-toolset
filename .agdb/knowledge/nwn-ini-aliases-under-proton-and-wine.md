---
type: Decision
title: nwn.ini aliases under Proton and Wine
tags: []
status: stable
generated: { by: trail, at: 2026-11-06T00:00:00Z }
touches: [apps/mg/src/main.rs, crates/mg-resman/src/install.rs, crates/mg-ui/src/lib.rs, crates/mg-ui/src/options.rs, crates/mg-ui/src/settings.rs, crates/mg-ui/tests/app/options_and_keys.rs, docs/log.md, docs/manual/10-options.md, docs/manual/11-command-line.md, apps/mg/target/tmp/moonglow-serve/a1098591-0667e805-2/.mg-lock]
fingerprint: v2-744895d03b77cb7e
created: 2026-11-06
author: August
description: "nwn.ini alias paths written by Proton/Wine are Windows paths that `native_path` translates to native paths via the prefix's dosdevices or drive_c on Linux/macOS, with an opt-out switch and reload pitfalls."
asks:
  - "How does mg translate Windows paths from nwn.ini aliases when running NWN under Proton or Wine on Linux or macOS?"
  - "Why were hak folders wrongly joined onto the user folder in versions 3.10.0 through 3.12.0 on Linux?"
  - "How do I turn off nwn.ini alias handling, in the window or from the command line?"
  - "Why do Workshop, userpatch or alias toggles only take effect after restarting Moonglow, and what must options_reload compare?"
---
## Decided
- nwn.ini alias values are read as the game wrote them. Under Proton or Wine those are Windows paths (`C:\users\steamuser\Documents\Neverwinter Nights\hak`). On Linux and macOS, `mg_resman::install::native_path` translates them through the prefix the user folder sits in: walk up the folder's ancestors to `dosdevices/<letter>:` (or, for C:, a `drive_c` ancestor). A drive that can't be found falls back to the default `user/<sub>`. On Windows the value is used as is.
- `GameInstall.aliases` (on by default) is the switch: `Settings.no_nwn_aliases` in the window, `mg --no-aliases` on the command line. dafena asked for it so a builder can opt out.
## Watch out
- `Moonglow::options_reload` must compare `Settings::install()` as a whole, not only the root, user folder and language. Otherwise switches that change the install (Workshop, userpatch, aliases) apply only at the next start.
- On Unix, a Windows path given to `Path::join` counts as relative and gets glued onto the user folder: that was the 3.10.0 to 3.12.0 bug.
