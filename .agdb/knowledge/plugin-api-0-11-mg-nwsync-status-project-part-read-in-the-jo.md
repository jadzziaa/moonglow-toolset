---
type: Decision
title: "Plugin API 0.11 mg.nwsync.status(): project part read in the job, host part a State snapshot"
description: "How mg.nwsync.status() gets its data (job's module copy vs host snapshot), why it reads no disk, and where each host fills it."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-27T10:12:49Z }
touches: ["crates/mg-plugin/src/runtime.rs:nwsync_api", "crates/mg-module/src/nwsync_content.rs:State", "crates/mg-ui/src/nwsync_content_view.rs:plugin_nwsync", "crates/mg-ui/src/jobs.rs:Context", "apps/mg/src/plugins.rs:game_for", "crates/mg-module/src/project_file.rs:nwsync_off"]
asks:
  - why does mg.nwsync.status return no survey
  - where does a plugin job get the NWSync state
  - "how is source = \"\" shown to plugins"
  - why is mg.nwsync.status not mg.nwsync.status() row in the reference
fingerprint: v2-b64ffb314244c3ec
created: 2026-11-27
author: augustaugur
---
mg.nwsync.status() (API 0.11) has two halves. The project half (nil without a source, off, source, signed_in) is read inside the runtime from the job own module copy (ws.module.project_file), like mg.vars, so a host that forgets to pass state still answers correctly. The host half is Input::nwsync: Option<mg_module::nwsync_content::State>, a plain-data copy (no Survey list, no Origins: Context is built for every job) used only when State.source == the project url. Window: Moonglow::plugin_nwsync() fills jobs::Context.nwsync in start_job_on (after refresh_module_layer, only for needs_module jobs), plus the panel (plugin_kinds) and at-once events (plugin_events) inputs. mg plugin run: game_for now returns (game, State) from ForModule.nwsync (said() dropped it); no survey, never syncing. Nothing reads disk or network: survey is the NWSync Content window last look, else nil. Off: ProjectFile::nwsync_off keeps the team source that own source = "" turned off (own "" with no team source = nil, not off). Layer gained at (Found::at) and haks_missing. from words come from ManifestFrom::word, shared with mg nwsync-status --json. Docs tests: the reference rows whose first code span has a dot are API names, so 17 has one mg.nwsync row and a bare-field table (source, survey...), mirroring mg.vars; a mg.nwsync.status() row would need api_names and type-file mapping. signed_in = auth.sent_to(url), never the token; a token in user.toml needs host = when the own file names no source, else it is refused with a warning.
