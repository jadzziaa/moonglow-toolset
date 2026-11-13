---
type: Decision
title: "NWSync upload in parts, a shared command runner, before pictures from an archive"
tags: []
status: stable
generated: { by: trail, at: 2026-10-29T00:00:00Z }
touches: [crates/mg-module/src/nwsync.rs, crates/mg-ui/src/nwsync_view.rs, crates/mg-ui/src/project_commands.rs, apps/mg/src/main.rs, crates/mg-ui/tests/screens.rs]
fingerprint: v2-2e4f720b1ee5746a
created: 2026-10-29
author: August
description: "NWSync upload runs the user's own command once per part (data, manifests, latest) in order, each only after the previous succeeds; also covers shared run_steps, allow-listing, and test and build pitfalls."
asks:
  - "Why does my NWSync upload send latest before manifests when I run rsync over the whole folder?"
  - "How do I configure a custom NWSync upload command in moonglow.toml, and which setting wins?"
  - "My screens test clicks a button that does nothing right after starting another job. Why, and how do I fix it?"
  - "Why does my 'before' binary built from an older commit behave like the new code?"
---
## Decided
- The NWSync upload is the builder's own command (Options › Folders, or `[nwsync] upload` in moonglow.toml / .moonglow/user.toml, the user's winning, the project's over Options'), run in the repository's folder once per part: `data`, `manifests`, `latest` (after a prune: `manifests`, `data`, no `latest`), each only after the one before exited 0. One command over the whole folder breaks the order: rsync sends by name, so `latest` goes before `manifests/`. `{part}` and `{folder}` (quoted per shell by `mg_module::nwsync::shell_quoted`) plus `MOONGLOW_NWSYNC_FOLDER`/`_PART`. No staging folder, no record of what was sent: the tools (rsync, aws s3 sync, rclone) skip what the server has.
- A project-file upload is a checkout's program: it goes through the project commands' allow (`allowed_commands`, name `[nwsync] upload`, which no `[commands]` name can be).
- Project commands and the upload share `project_commands::run_steps` (a `Run` of `Step`s; `let_go: None` keeps the project's files and haks as they are). Typed input is written from the poll loop, not a thread per child, so several programs in one run share one input line.

## Watch out
- A screens test that clicks a button right after starting another job (Remove Unused Data over 3000 files) clicks a disabled button: wait until `!busy()` first.
- Before pictures from an older commit: `git archive` the commit into the scratchpad and build it with its own `CARGO_TARGET_DIR`. Sharing this worktree's target dir reuses this worktree's artifacts (same relative paths hash alike, and the archive's files are older than them), so the "before" binary is the new code.
- `mg --json` tests require a quiet stderr: an upload's progress goes to the JSON's notes there, to stderr in text mode; the command's own stdout is sent to mg's stderr.
