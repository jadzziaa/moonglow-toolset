---
type: Decision
title: "Machine locks: one display lock, GPU slots, display before GPU"
description: "Why the display, Aurora and the client share one exclusive flock, why started sessions must not inherit its descriptor, the lock order, and MOONGLOW_LOCKS_HELD."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-25T14:06:02Z }
touches: [docs/locks.md, tools/locks.sh, tools/aurora/headless.sh, crates/mg-testkit/src/display.rs, crates/mg-testkit-gpu/src/slots.rs]
asks:
  - why is there one display lock and not separate Aurora and client locks
  - why is the display lock held forever after the first client test in a binary
  - "why does display::hold panic when the test holds a GPU place"
  - why does headless.sh close file descriptors before starting KWin
  - what is MOONGLOW_LOCKS_HELD for
fingerprint: v2-90dbfe44b4b824e5
created: 2026-11-25
author: augustaugur
---
One exclusive flock on ${MOONGLOW_ORACLE}/locks/display.lock covers the off-screen display, Aurora and the client: xdrive.py input goes to the focused window, winshot finds the client by title, and a display restart kills everything on it, so no two runs may share it (no shared mode; separate aurora/client locks would add nothing). Protocol: docs/locks.md.

Gotchas. (1) flock belongs to the open file: a KWin session started by headless.sh start would inherit the lock descriptor and hold the lock for the session's life, so the launching subshell closes every fd above 2 (mg_close_extra_fds; a test with a fake dbus-run-session fails without it). (2) A child opening the lock file its parent holds blocks on its own parent: holders export MOONGLOW_LOCKS_HELD=display and takers skip. Rust's File is CLOEXEC, so a Rust holder must pass it in the Command env. (3) Test binaries hold the display until exit, not per run: a client batch reads DISPLAY once and a restart between runs would break it. (4) Order is display then GPU slot, never upgrading; display::hold panics if its first call is made by a thread holding a GPU place, so client tests call common::gpu_after_display() in place of gpu::hold(). Later calls in a process that already has the display do not panic. (5) GPU slots (XDG_STATE_HOME/moonglow/locks/gpu/0..N-1) are unlocked explicitly before close for the same descriptor-copy reason as mg-update's staging lock. Waiting lines use io::stderr().write_all: libtest capture only intercepts print macros (observed).
