---
type: Decision
title: "NWSync folder source: copied into the cache through fetch::Folder, not read in place; source_url's rules"
description: "Why a file:// or folder [nwsync] source copies data into Moonglow's cache, how relative paths resolve, and why a bare name is refused."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T09:09:49Z }
touches: ["crates/mg-nwsync/src/fetch.rs:Folder", "crates/mg-nwsync/src/source.rs:source_url", "crates/mg-module/src/project_file.rs:nwsync_source", "apps/mg/src/nwsync_read.rs:settings"]
asks:
  - is a file source read straight from the folder or copied into the cache
  - how is a relative nwsync source path resolved
  - why is source = nwsync.example.org a warning and not a folder
  - why not hard link NWSync data from a repository folder
fingerprint: v2-ac8a2a41da783f92
created: 2026-11-27
author: augustaugur
---
TDoD request for 5.5.0 (2026-11-27). A repository folder source (file:///x, a whole path, or ./ ../ from the project's folder; a .mod's from its folder, Place::root; --source's from cwd, and a bare name there only if that folder exists) is kept in Settings.url as a file:// URL (mg_nwsync::source_url, folder_url/local_folder; . and .. removed lexically so two projects naming one folder share latest/<server_key>). fetch::Folder implements Fetch over the folder and fetch::for_settings picks Folder or Client for every caller (mg nwsync-status --online, nwsync-pull, the window's Sync). Decided: data and manifests are COPIED into the cache through the normal resolve/download path (SHA-1 checked), not read where they are: latest/ (written only once the manifest is here), used/ marks, prune and Found::at stay unchanged, and the module opens offline whatever happens to the folder (drive gone, nwn_nwsync_prune, republish). Hard links were rejected: a link shares the repository file's mtime, so the cache prune's min_age guard (which reads mtimes) would not protect a just-linked file, and touching it would change what the repository prune's --min-age reads. Cost: double disk. A bare name (nwsync.example.org, nwsync-repo) stays a warning (forgotten https://), as project_file::the_nwsync_source_is_read pins. ManifestFrom::Folder (said 'read from the repository folder', JSON 'folder') marks a manifest just read from a folder.
