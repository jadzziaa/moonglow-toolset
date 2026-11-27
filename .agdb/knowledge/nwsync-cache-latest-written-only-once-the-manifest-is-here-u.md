---
type: Decision
title: "NWSync cache: latest/ written only once the manifest is here; used/ marks; prune passes over unreadable latest/recent"
description: "Why resolve writes latest/<server> late, why Cache::touch leaves used/<sha1>, and which unreadable manifests stop a prune's data removal."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T05:46:11Z }
touches: ["crates/mg-nwsync/src/fetch.rs:resolve", "crates/mg-nwsync/src/prune.rs:prune", "crates/mg-nwsync/src/cache.rs:touch"]
asks:
  - why does a prune not remove any NWSync data files
  - "when is latest/<server> written in Moonglow's NWSync cache"
  - what is the used/ folder in the NWSync cache
  - "does a pinned manifest from the game's storage keep its downloaded data"
fingerprint: v2-fffee27e0e9b956d
created: 2026-11-27
author: augustaugur
---
Review of PR #53 (2026-11-27). fetch::resolve used to write latest/<server> as soon as /latest answered, before the manifest was fetched: a server naming a manifest it then 404s left a pointer to nothing, and prune (which keeps every SHA-1 in latest/) set data_left and removed no data at all, forever. Now latest/ is written only after find() or get_manifest() succeeds for it. In prune only an unreadable manifest of Keep::manifests (the caller named it) blocks data removal; an unreadable latest/ or recent one is passed over with a line in Pruned::notes. A manifest used from the game's storage alone has no file in manifests/ or written/, so Cache::touch (called by mg-module mount and write_haks) also leaves an empty used/<sha1> whose mtime counts as its last use; touch does not create a missing cache root. Such a manifest is readable by prune only when prune is given the GameStorage: with None it is passed over and its data goes. Found::includes_module (from the .json side file, else manifest_metadata) is exposed but not yet acted on by mg-module: the caller must leave the module's own resources out (haks::without_module) before haks::spread and NwSyncContainer::new.
