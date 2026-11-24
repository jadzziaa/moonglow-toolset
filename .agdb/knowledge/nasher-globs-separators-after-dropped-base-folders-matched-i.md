---
type: Fact
title: "nasher globs: separators after ** dropped, base folders matched ignoring case"
description: "How nasher 1.1.3 actually matches include globs (measured), which Moonglow's nasher/glob.rs must copy."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-24T01:44:51Z }
touches: [crates/mg-module/src/nasher/glob.rs]
asks:
  - "does nasher's ** keep the slashes after it?"
  - why does src/**/sub/*.nss not match src/sub/a.nss?
  - "is a nasher include's folder matched case-insensitively on Linux?"
  - how do I check a glob against nasher itself?
fingerprint: v2-e6a6887f34f62e72
created: 2026-11-24
author: augustaugur
---
Measured against nasher 1.1.3 (~/.local/opt/neverwinter/bin/nasher list --verbose on a scratch project, 2026-11-23):

- After a `**`, the Nim glob package drops EVERY later `/`: `src/**/sub/*.nss` lists `src/suba.nss` and `src/x/subd.nss`, and NOT `src/sub/a.nss` or `src/x/sub/b.nss`. The adversarial-review branch (885106ab) "fixed" this to drop only the slash right after `**`; that was wrong and was reverted (to_regex's `'/' if globstar => {}`). Tests: globstar_drops_later_slashes_as_nasher_... in nasher/glob.rs.
- nasher on Linux matches the pattern's leading (non-wildcard) folders ignoring case too: `SRC/**/*.nss` and `src/AREAS` find `src/areas/f.nss`. Cover::covers compared the base case-insensitively; walk() did not (root.join(base) + read_dir), so the two disagreed. walk now resolves base components with resolve_ignoring_case (exact spelling first).

To re-check: write files under target/, a nasher.cfg with [package] and [target] include = "...", run `nasher list --verbose` in that folder.
