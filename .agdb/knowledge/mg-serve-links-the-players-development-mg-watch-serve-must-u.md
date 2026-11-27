---
type: Gotcha
title: "mg serve links the player's development/; mg watch --serve must unlink it first"
description: "Why mg serve's development folder is a link, how mg watch --serve avoids writing through it into the player's folder, and which entries count as Moonglow's."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T10:32:15Z }
touches: ["apps/mg/src/serve.rs:link_development", apps/mg/src/main.rs, crates/mg-module/src/links.rs]
asks:
  - why does mg watch --serve remove the development link
  - "how does mg serve give the server the player's development folder"
  - "could mg watch write into the player's development folder"
  - where are the link_dir and junction helpers
fingerprint: v2-c49771bf5ba0b21d
created: 2026-11-27
author: augustaugur
---
Owner's ruling 2026-11-27: mg serve (and mg test, which starts through serve::serve) gives the server's folder a link to the player's development/ (gi.user_dir, so --user-dir is followed), made again at each start in serve::serve after prepare (serve::link_development). Hazard: mg serve and mg watch --serve share one scratch folder (run_dir("serve", module)), and watch's default --dev-dir is SERVER/development, created with create_dir_all, which succeeds on a link and would write compiled scripts and .mg-watch into the player's own folder. So watch calls serve::unlink_development first and sets Config.development = Development::Watched (no link, no note). Rule for what is Moonglow's: any link at DIR/development is ours and is remade (unlink: remove_file else remove_dir, never remove_dir_all); a real folder in a --dir is the builder's, kept, with a note; a real folder in Moonglow's own scratch folder (watch's leftover) is removed. The link helpers (symlink, else junction for folders on Windows; is_link, unlink) live in mg_module::links, shared with Test Module (mg-ui has no mg-serve dependency). Engine check: engine_linked_files::the_engine_runs_a_script_from_a_linked_development_folder.
