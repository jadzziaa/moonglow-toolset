---
title: "Hak and talk table names are their file's bytes"
author: August
date: 2026-10-30
branch: worktree-agent-aac006294c0965c17
cost: { minutes: 2321, tokens: 16061071 }
touches:
  - crates/mg-resman/src/file_name.rs
  - "crates/mg-module/src/lib.rs#haks_of"
  - crates/mg-ui/src/module_props.rs
  - crates/mg-module/src/attach.rs
---

decided: the Linux server opens <Mod_Hak bytes>.hak and <Mod_CustomTlk bytes>.tlk (in the user's tlk/) byte for byte, converting nothing (engine_hak_names): 0xF1 opens only the 0xF1-named file, C3 B1 only the UTF-8-named one. Names are held as Windows-1252 text (lossless both ways) through mg_resman::file_name_text / file_name_bytes / named_file / name_of_file; never String::from_utf8_lossy, OsStr::to_string_lossy or the module's codepage (text::decode: encoding.2da comes from the haks themselves).
decided: the engine is case-sensitive on Linux even in ASCII (MG_PLAIN misses mg_plain.hak and mg_plain.tlk). A name differing in case alone goes through mg_resman::listed (Listed::OtherCase, Found where CASE_IGNORED: Windows, macOS) and GameInstall::hak_not_found, the one message every tool says.
watch out: GameInstall::names_in / hak_names / tlk_names keep the file's case; file_names lowercases, which wrote names the Linux engine doesn't open.
watch out: a talk table in a hak or the module is a resource, found by its resref in any case: the Custom TLK dropdown maps those to the module's own spelling, not the folders' files.
ruled out: showing UTF-8-valid names as UTF-8 and others as cp1252: two readings again, and not invertible.
not tested: Windows (the engine presumably opens by ANSI codepage text; named_file passes the 1252 text) and macOS.
