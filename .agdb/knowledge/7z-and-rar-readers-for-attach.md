---
type: Decision
title: 7z and RAR readers for attach
tags: []
status: stable
generated: { by: trail, at: 2026-10-29T00:00:00Z }
touches: [crates/mg-module/src/attach/archive.rs, Cargo.toml]
fingerprint: v2-6d359089ae0cc0b4
created: 2026-10-29
author: August
description: "Chose the rars 0.12 crate for RAR and sevenz-rust2 0.24 for 7z in attach, with reasons, feature settings (single thread, no AES), and the need to sink-read solid archive members."
asks:
  - "Which Rust crate should we use to read RAR files, and why was rars picked over rar-codec or unrar?"
  - "Why did extracting a damaged 7z archive take 100 ms per case, and how do I fix it?"
  - "My solid 7z or RAR extraction gives wrong data or CannotSkipSolidMember when skipping entries; what should I do?"
  - "How can I detect a password-protected 7z when sevenz-rust2 has no aes256 support?"
---
Archive readers for Add Haks and Talk Table / mg attach (2026-10-29). RAR: rars 0.12 (Apache-2.0, default-features = false: no writer, crypto, rayon) was chosen over rar-codec 0.1 (Apache-2.0, zero deps): over 143 archives the genuine unrar extracts (rars' own tests/fixtures, 54 Workshop RAR5 files, ours), rars matched unrar's bytes on all but multi-volume sets and links; rar-codec failed 35, mostly RAR 4 (it doesn't skip NEWSUB blocks - comments, recovery records, rars' own RARS marker - and misreads unicode names and solid PPMd). The unrar/unrar-ng crates bundle UnRAR's source (license not GPL-compatible); compress-tools needs libarchive at run time; unarc-rs wraps rars. 7z: sevenz-rust2 0.24 with bzip2, deflate, ppmd (bzip2 resolves to pure-Rust libbz2-rs-sys); no aes256, so a locked 7z is told by UnsupportedCompressionMethod("AES...") or an AES coder in a block. set_thread_count(1): lzma-rust2's LZMA2 work pool polls errors every 100 ms, which made each damaged-archive case take 100 ms. Solid archives can't skip members: read unwanted 7z entries to a sink and give rars a sink writer, or the next member decodes wrong / rars refuses with CannotSkipSolidMember. No system RAR compressor here: fixtures were made with rars-cli (cargo install --root scratch) and checked with /usr/bin/unrar and 7z.
