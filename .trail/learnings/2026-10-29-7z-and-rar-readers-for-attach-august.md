---
title: 7z and RAR readers for attach
author: August
date: 2026-10-29
branch: worktree-agent-a4ef2ddeed6f3bc6d
cost: { minutes: 2033, tokens: 14006671 }
touches:
  - crates/mg-module/src/attach/archive.rs
  - Cargo.toml
---

Archive readers for Add Haks and Talk Table / mg attach (2026-10-29). RAR: rars 0.12 (Apache-2.0, default-features = false: no writer, crypto, rayon) was chosen over rar-codec 0.1 (Apache-2.0, zero deps): over 143 archives the genuine unrar extracts (rars' own tests/fixtures, 54 Workshop RAR5 files, ours), rars matched unrar's bytes on all but multi-volume sets and links; rar-codec failed 35, mostly RAR 4 (it doesn't skip NEWSUB blocks - comments, recovery records, rars' own RARS marker - and misreads unicode names and solid PPMd). The unrar/unrar-ng crates bundle UnRAR's source (license not GPL-compatible); compress-tools needs libarchive at run time; unarc-rs wraps rars. 7z: sevenz-rust2 0.24 with bzip2, deflate, ppmd (bzip2 resolves to pure-Rust libbz2-rs-sys); no aes256, so a locked 7z is told by UnsupportedCompressionMethod("AES...") or an AES coder in a block. set_thread_count(1): lzma-rust2's LZMA2 work pool polls errors every 100 ms, which made each damaged-archive case take 100 ms. Solid archives can't skip members: read unwanted 7z entries to a sink and give rars a sink writer, or the next member decodes wrong / rars refuses with CannotSkipSolidMember. No system RAR compressor here: fixtures were made with rars-cli (cargo install --root scratch) and checked with /usr/bin/unrar and 7z.
