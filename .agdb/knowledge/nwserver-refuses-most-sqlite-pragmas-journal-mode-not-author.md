---
type: Fact
title: "nwserver refuses most SQLite pragmas (journal_mode): not authorized, code 23"
description: "Why a module's PRAGMA journal_mode = WAL fails under mg test, mg serve and every nwserver: the engine's SQLite authorizer allows only a listed set of pragmas."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-28T06:04:03Z }
touches: [apps/mg/src/serve.rs]
asks:
  - sqlite error not authorized PRAGMA journal_mode WAL CampaignDB
  - does mg test make the campaign database read-only
  - which sqlite pragmas does NWN EE allow
  - "code=23 not authorized in a module's SqlPrepareQueryCampaign"
fingerprint: v2-ff0892293cc7cbb8
created: 2026-11-28
author: augustaugur
---
The engine's SQLite authorizer (nwsqlite.cpp LogError: SQL ERROR: [23] not authorized) refuses every pragma but those the 1.87.8193.35 patch notes list as enabled: collation_list, compile_options, defer_foreign_keys, foreign_key_check, foreign_key_list, foreign_keys, freelist_count, function_list, index_info, index_list, index_xinfo, page_count, page_size, table_info, table_list, table_xinfo. journal_mode is not among them, so SqlPrepareQueryCampaign(db, "PRAGMA journal_mode = WAL") fails in every server, whatever runs it (TDoD's tdod_bridge_inc.nss, 2026-11-28: seen under mg test, and on 2026-11-25 under bin/client's mg serve, before 5.5.0). Moonglow sets no authorizer and nothing read-only: not ours. WAL is kept in the database file, so the program on the other side can set it once.
