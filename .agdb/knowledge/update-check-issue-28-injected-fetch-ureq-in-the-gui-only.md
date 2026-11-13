---
type: Decision
title: "Update check (issue 28): injected fetch, ureq in the GUI only"
tags: []
status: stable
generated: { by: trail, at: 2026-10-23T00:00:00Z }
touches: [Cargo.lock, Cargo.toml, apps/moonglow/Cargo.toml, apps/moonglow/src/main.rs, crates/mg-ui/src/commands.rs, crates/mg-ui/src/lib.rs, crates/mg-ui/src/manual.rs, crates/mg-ui/src/options.rs, crates/mg-ui/src/settings.rs, crates/mg-ui/tests/app.rs]
created: 2026-10-23
author: August
description: "The update check's HTTP request uses ureq in apps/moonglow only, with an injected Fetch in mg-ui's updates.rs (None by default), so the CLI and tests never hit the network; also covers test and Flatpak pitfalls."
asks:
  - "Why is the update check's ureq HTTP call in apps/moonglow instead of mg-ui?"
  - "How do I keep tests and the mg CLI from making network requests in the update check?"
  - "My egui_kittest click on a button in a re-centered window misses after a state change; what fixes it?"
  - "Why does Check for Updates fail in the Flatpak build, and what permission is missing?"
---
decided: The HTTP request lives in apps/moonglow (src/update_check.rs, ureq 3 with rustls+ring+webpki-roots, no gzip) because apps/mg depends on mg-ui: anything added to mg-ui links into the CLI. mg-ui's updates.rs takes an injected Fetch (Option<Arc<dyn Fn() -> Result<String,String>>>), None in Moonglow::new, so mg and every test never reach the network; with background_jobs off (tests) the fetch runs inline, as jobs do. update_checked is stamped when a request starts, so a failing network still asks at most once a day. Versions are compared by a small own parser (semver was only a build dependency in the tree).
ruled out: Committing a cargo example for the live check: CLAUDE.md counts the examples the doc-test run links; the check was run from a throwaway apps/moonglow/examples file that #[path]-included src/update_check.rs, then deleted.
watch out: egui_kittest clicks land where the node was last frame. A window anchored at the center that grows (a line added by changing state) is re-centered a frame later, so a click right after the change misses its button: run_steps(2) before clicking. The Flatpak has no --share=network, so its Check for Updates fails unless the user allows the network.
