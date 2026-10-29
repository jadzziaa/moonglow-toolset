---
title: "NWSync prune, userpatch in the engine, Beamdog and the registry, side-editor Undo"
author: August
date: 2026-10-29
branch: worktree-agent-aad321f06a1eb3147
cost: { minutes: 1767, tokens: 12986302 }
touches:
  - crates/mg-module/src/nwsync.rs
  - crates/mg-module/src/verify.rs
  - crates/mg-resman/src/install.rs
  - crates/mg-ui/src/side_doc.rs
---

What the install-and-publish fix package measured, so nobody measures it again:

- nwn_nwsync_prune (neverwinter.nim): removes any file under data/sha1 (any name, recursively, .part too) that no manifest in manifests/ names, only when older than --min-age seconds (strictly; default 1209600, two weeks: files just written survive --min-age 0), then the folders left empty (data/sha1 itself stays). Removes `<sha1>.json` whose manifest is gone regardless of age. Only reports (does not mend): `latest` naming a missing manifest, a manifest without .json, data files a manifest names that are missing. Files outside data/ (root notes) are left. mg_module::nwsync::prune matches it file for file (tests/nwsync.rs pruned_as_nwn_nwsync_prune_prunes).
- nwserver reads the user directory's userpatch.ini: ResManGetAliasFor names its haks `PATCH:<name>`, under the module and over override/, higher PatchFileNNN over lower (engine_resman.rs). A module's custom TLK is loaded from a userpatch hak or from override/ too (engine_tlk.rs), so a player without it can't load the module: Verify warns (verify::Local).
- Beamdog Client: neverwinter.nim's findNwnRoot reads `<config>/Beamdog Client/settings.json` "folders" and looks for 00829 (development) then 00785 (stable); Moonglow tries stable first. It reads no registry; Moonglow runs `reg query` for Steam's SteamPath/InstallPath, which works under Wine (a scratch WINEPREFIX with WINEDLLOVERRIDES="mscoree,mshtml=" and no DISPLAY, else prefix creation hangs on the Mono/Gecko prompt).
- Side editors' Undo from the Edit menu or toolbar: the pointer leaving the editor for the bars made Undo the module's (undo_under_pointer is per frame). undo_before_bars keeps the editor; a menu entry is chosen in a frame where the popup closes, so "a menu was open as the frame began" must count as on the bars.
