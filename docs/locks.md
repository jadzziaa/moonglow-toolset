---
type: Reference
title: The machine's locks on the off-screen display and the GPU
description: The protocol every program shares to take turns at the off-screen display (the game client, Aurora, the display's restarts) and at the GPU devices that tests open - the lock files, their modes, the order, MOONGLOW_LOCKS_HELD, the holder line, the waits and messages, who takes what and what takes nothing - for an outside program that wants to honor it.
tags: [locks, flock, display, gpu, tests, oracle, protocol]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-25T19:00:31Z }
sources:
  - id: code
    resource: "crates/mg-testkit/src/display.rs, crates/mg-testkit-gpu/src/{gpu,slots}.rs, tools/locks.sh, tools/aurora/{headless,run-aurora}.sh, tools/nwclient/run-{client,server-and-client}.sh"
    title: The implementation
  - id: owner
    resource: human:august (2026-11-25)
    title: The owner's ruling that the two kinds of lock be built
---

# The machine's locks on the off-screen display and the GPU

Two kinds of lock, both `flock(2)` advisory locks on files (Rust's
`std::fs::File::lock` is the same family; `flock(1)` interlocks with it).
Another program that uses the off-screen display, or runs Moonglow's test
suites, takes turns with Moonglow by taking the same files the same way.

## The display lock

One exclusive lock covers the off-screen display, Aurora and the game
client. Input sent with `xdrive.py` goes to whatever window has focus and a
client's window is found by its title, so every run on the display needs it
alone; restarting the display kills whatever else is on it. There is no
shared mode.

| | |
| --- | --- |
| File | `${MOONGLOW_ORACLE:-$HOME/.local/share/moonglow-oracle}/locks/display.lock` |
| Mode | exclusive, held for the run's life |
| Made | on first use, never deleted (deleting a lock file splits it into two locks) |
| Holder line | the holder writes one line into the file: `PID 1234, started 09:12:05, <command>, in <cwd>` (on Windows a lock is mandatory, so the line can't be read while the file is held: a waiter says "held by someone"; the display is Linux's alone) |

Take it **before** restarting the display (`headless.sh stop`, `start`) and
before `headless.sh check`, and keep the descriptor open for as long as
anything is on the display. A session that is started must not inherit the
descriptor (the headless KWin would hold the lock for its life): close
every descriptor above 2 in the process that launches it
(`mg_close_extra_fds` in `tools/locks.sh`).

### Waiting and refusing

* Wait up to `MOONGLOW_LOCK_WAIT` seconds (default 1800; `0` refuses at once).
* While waiting, on **stderr**: `waiting for the off-screen display: held by <holder line>; up to 30 min`, repeated every 5 minutes.
* Past the wait: `the off-screen display is still held after 30 min: nothing was run`, exit status 2. (A wait under a minute set by hand says seconds; `0` says `0 min`.)

### Held by an ancestor

A lock belongs to the open file, so a child that opens the file its parent
holds would wait for its own parent. A holder exports
`MOONGLOW_LOCKS_HELD=display` (a comma list); anything that sees `display`
in it takes nothing.

## GPU slots

Tests that open a GPU device (`mg_testkit::gpu::hold`, `open`, `alone`) take
one slot of the machine, so that test binaries run at once keep at most N
devices open between them (a run of dozens at once once coincided with a
kernel panic).

| | |
| --- | --- |
| Files | `${XDG_STATE_HOME:-$HOME/.local/state}/moonglow/locks/gpu/0` to `N-1` (not moved by `MOONGLOW_ORACLE`) |
| N | `MG_TEST_GPUS`, default 4: a process uses slots `0` to `N-1` |
| Mode | exclusive, one slot per test, held until the test ends (`release()` earlier) |
| Taking | `try_lock` each slot in turn; while all are held, poll every 50 ms (a lock cannot wait for "any of N") |
| Waiting line | after 10 s, then once a minute, on stderr: `waiting for a GPU slot (4 held: PIDs 1234, 5678)` (PIDs from `/proc/locks` on Linux, else omitted) |

The slot is unlocked explicitly before the file is closed: a child spawned
on another thread can hold a copy of the descriptor until its `exec`. Where
the folder cannot be made (no home, a read-only disk), the hold says so once
and goes on with the process's own places: a test never fails for want of a
slot. `alone()` is alone in its process (it is about process-wide render
state): it takes all the process's places and one slot.

## The order

**The display, then a GPU slot.** Nobody upgrades a lock. A test that holds a
GPU place and waits for the display could wait for a run that holds the
display and waits for a slot, so `mg_testkit::display::hold()` panics, naming
the rule, when its first call comes from a thread that holds a GPU place.
The client tests call `common::gpu_after_display()` instead of
`gpu::hold()`.

## Who takes what

| Taker | Takes |
| --- | --- |
| `tools/aurora/headless.sh start` (only when it launches a session), `stop` | display, for the command |
| `tools/aurora/run-aurora.sh run` | display, for Aurora's life (Wine inherits the descriptor; it ends a few seconds after Aurora closes, or at `run-aurora.sh kill`) |
| `tools/nwclient/run-client.sh`, `run-server-and-client.sh` (not with `MG_CLIENT=none`) | display, for the client's life (a `kill -9` of the sandbox frees it) |
| Client tests (`mg-corpus-tests`: `common::client_lock`, `mg_testkit::joined::start`) | display, once per test binary, kept to the end of its run (a batch reads `DISPLAY` once); runs replaying an earlier run's pictures (`MG_CLIENT_PICTURES`) take nothing (`MG_CLIENT_CAPTURED` takes it: a picture not kept yet is taken with the client) |
| `mg_testkit::gpu` users (mg-render, mg-ui, mg-preview, the corpus tests, the viewer's tests) | one GPU slot per test |
| A person or agent at the shell | `tools/locks.sh hold display -- CMD...` for anything driven by hand: one client run with xdrive calls under it, a whole Aurora capture session |

**Take the lock first, then the display.** Start or check the display
(`tools/aurora/headless.sh start` or `check`) inside your hold, and never
read `DISPLAY` or the display's number before it: the holder before you
may restart the display, so a number read outside the hold can name a
session that is gone. A batch by hand is one hold around a script that
starts the display and runs everything:
`tools/locks.sh hold display -- bash -c 'export DISPLAY=$(tools/aurora/headless.sh start) && ...'`.

## The display's history

`${MOONGLOW_ORACLE}/locks/display.history` keeps one line per event at the
display, the last 1,000 (`mg_display_history` in `tools/locks.sh`), so a
run that failed can be set beside what happened to the display at the time:

- `headless.sh start` when it launches a session (its display and session PID, or that it did not start), and `headless.sh stop`;
- `run-client.sh` at the client's start (the display, the scratch folder, the arguments) and at its end, with the exit status (a SIGTERM or SIGINT to the script stops the client and still writes the end, which is how the client tests stop it, `common::stop_client`; a `kill -9` of the script leaves no end line: the sandbox dies with it);
- `run-server-and-client.sh` at its start.

Each line gives the UTC time, the PID, the event, the lock's state (held
by this run, held by an ancestor with its holder line, or not held), the
PID and command line that called the script, and the folder:

```
2026-11-25T19:02:11Z PID 41230: run-client.sh end: status 0, /…/target/test-output/client_reading_x; lock held by an ancestor (PID 41190, started 14:01:30, client_render, in /…); called by PID 41190 /…/client_render-… --ignored; in /…
```

Other programs that restart the display through `headless.sh` (TDoD's
scripts do) show in it too.

Taking nothing: the engine runner (`run_server`, `save_game`, `timed_turn`:
`nwserver` is CPU-only), `mg`, `mgv`, and the GUIs (Moonglow, Moonglow
Viewer): the owner's window never waits.

## For an outside program

1. Open `display.lock` for append (not truncating), `flock -x -w SECONDS`, write your holder line over the file, keep the descriptor open for your run, export `MOONGLOW_LOCKS_HELD=display` to what you start.
2. Take it before restarting or checking the display, and read the display's number only inside the hold. Keep your existing checks.
3. Keep the descriptor out of anything that must outlive you.
4. If you run Moonglow's test suites, nothing else is needed: they take the display and the slots themselves.
