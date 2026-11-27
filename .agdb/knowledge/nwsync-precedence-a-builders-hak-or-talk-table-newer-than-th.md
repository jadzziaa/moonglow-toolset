---
type: Decision
title: "NWSync precedence: a builder's hak or talk table newer than the manifest wins; source = \"\" opts out"
description: "The owner's rule (2026-11-27) for files on disk against a module's NWSync manifest, where it lives, how the manifest's time is taken, and how a builder turns NWSync off."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T06:35:44Z }
touches: ["crates/mg-module/src/nwsync_content.rs:OnDisk", "crates/mg-module/src/nwsync_content.rs:write_haks", "crates/mg-module/src/nwsync_content.rs:mount", "crates/mg-nwsync/src/cache.rs:touch", "apps/mg/src/serve.rs:place", "crates/mg-module/src/project_file.rs:nwsync_source"]
asks:
  - why is my local hak used over the NWSync manifest
  - how do I turn off NWSync haks for mg serve
  - why does the spread plan not overwrite a hak on disk
  - what time is an NWSync manifest compared with
fingerprint: v2-27c6d41280f73f12
created: 2026-11-27
author: augustaugur
---
Owner's word 2026-11-27, "newer local wins" (deferred.md Settled). A hak of Mod_HakList on disk (GameInstall::find_hak) or the custom talk table on disk (the project's JSON sources of it and its feminine table, else the tlk folders') whose mtime is newer than Found::at wins: mount moves its hak:<name> layer to priority::HAK_OVER_NWSYNC (41, ResMan::move_to, back to HAK/HAK_USER on a remount where it is no longer newer) and drops the manifest's TLK entries (name and namef) from the container; write_haks leaves it out of the plan, so mg serve/test/nwsync-haks and Test Module take it from disk. Each case is said (Layer::notes in Mounted::warnings, HakFiles::notes). Older stays under the manifest silently. Spread plan (no origins): slices go only to listed haks NOT on disk; on-disk haks are kept and said; all on disk -> nothing written and no data needed. Found::at = the game's created_at for a manifest from its storage, else the cache manifest file mtime; Cache::touch therefore no longer bumps manifests/<sha1> (prune reads used/ and written/). OnDisk::without_cache drops files that canonicalize into the NWSync cache: mg serve --real-user-dir leaves symlinks to the cache in the player's hak folder, which find_hak then finds and whose mtime (the written hak's) would otherwise count as a newer builder file (found by the_players_own_files_are_never_replaced). Opt-out: source = "" in .moonglow/user.toml (own=true) clears nwsync_source silently; --no-nwsync on serve/test/watch also clears it for the run's resman (module_resman_as) and stops prepare's ProjectFile::read fallback. serve place(): hard link from the cache, symlink-first elsewhere, symlink/copy only in the player's folder, made as .part and renamed; a regular file in the player's folder is never replaced (error).
