---
title: "Hak names are their file's bytes"
author: August
date: 2026-10-30
branch: worktree-agent-aac006294c0965c17
cost: { minutes: 2321, tokens: 16061071 }
touches:
  - crates/mg-resman/src/hak_name.rs
  - "crates/mg-module/src/lib.rs#haks_of"
  - crates/mg-ui/src/module_props.rs
  - crates/mg-module/src/attach.rs
---

decided: the Linux server opens <Mod_Hak bytes>.hak byte for byte, converting nothing (engine_hak_names): 0xF1 opens only the 0xF1-named file, C3 B1 only the UTF-8-named one. Names are held as Windows-1252 text (lossless both ways) through mg_resman::hak_name / hak_name_bytes / hak_file_name / hak_name_of_file; never String::from_utf8_lossy, OsStr::to_string_lossy or the module's codepage (text::decode: encoding.2da comes from the haks themselves).
watch out: GameInstall::hak_names (Add hak's list) keeps the file's case; file_names lowercases, which wrote names the Linux engine doesn't open.
watch out: the engine is case-sensitive on Linux even in ASCII (MG_PLAIN misses mg_plain.hak); Module Properties' 'not found' still matches ignoring case: left as is.
ruled out: showing UTF-8-valid names as UTF-8 and others as cp1252: two readings again, and not invertible.
not tested: Windows (the engine presumably opens by ANSI codepage text; hak_file_name passes the 1252 text).
