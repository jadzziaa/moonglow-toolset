---
type: Reference
title: The machine's locks - quiet, the off-screen display and the GPU
description: The protocol every program shares to keep the machine quiet for a timing run, to take turns at the off-screen display (the game client, Aurora, the display's restarts) and at the GPU devices that tests open - the lock files, their modes, the order, MOONGLOW_LOCKS_HELD, the holder line, the waits and messages, who takes what and what takes nothing - for an outside program that wants to honor it.
tags: [locks, flock, quiet, display, gpu, tests, oracle, protocol]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-27T07:35:46Z }
sources:
  - id: code
    resource: "crates/mg-testkit/src/{display,lib,quiet_tests}.rs, crates/mg-testkit-gpu/src/{gpu,slots,quiet}.rs, tools/locks.sh, tools/aurora/{headless,run-aurora}.sh, tools/nwclient/run-{client,server-and-client}.sh"
    title: The implementation
  - id: owner
    resource: human:august (2026-11-25)
    title: The owner's ruling that the two kinds of lock be built
  - id: owner-quiet
    resource: human:august (2026-11-27)
    title: The owner's approval of the quiet hold, a third kind
---

# The machine's locks: quiet, the off-screen display and the GPU

Three kinds of lock, all `flock(2)` advisory locks on files (Rust's
`std::fs::File::lock` is the same family; `flock(1)` interlocks with it).
Another program that needs the machine to itself, uses the off-screen
display, or runs Moonglow's test suites, takes turns with Moonglow by taking
the same files the same way.

## The quiet lock

A run that measures time (a frame-timing run, Moonglow Client's soak) needs
the machine to itself: a test suite beside it moves its numbers. It holds
the quiet lock **alone**. The ordinary heavy runs (test binaries, the
oracle scripts, an orchestrator's suites) **share** it: they never exclude
each other, and they all exclude a quiet run, waiting while one holds it.

| | |
| --- | --- |
| Files | `${XDG_STATE_HOME:-$HOME/.local/state}/moonglow/locks/quiet.lock` and `quiet.gate`, beside the GPU slots (not moved by `MOONGLOW_ORACLE`: the machine is the machine) |
| An ordinary run | `quiet.gate` shared, then `quiet.lock` shared, then the gate given back; the lock kept for the run's life |
| A quiet run | `quiet.gate` exclusive, then `quiet.lock` exclusive, both kept for the run's life |
| Made | on first use, never deleted |
| Holder line | the quiet run writes one line into `quiet.lock`, as the display's holder does: `PID 1234, started 09:12:05, <command>, in <cwd>` (ordinary runs write nothing: there are many) |

**The gate** puts a waiting quiet run first. A lock waiting to be exclusive
does not stop new shared takers, so test binaries starting one after another
could keep a quiet run waiting for ever. A quiet run takes the gate before
it waits for the ordinary runs already running: new ones stop at the gate,
and the quiet run starts once those end. An ordinary run is at the gate only
for the moment it takes the lock.

### Waiting and refusing

* Both kinds wait up to `MOONGLOW_LOCK_WAIT` seconds (default 1800; `0` refuses at once), saying so on **stderr** at once and every 5 minutes.
* An ordinary run: `waiting for a quiet run to end: held by <holder line>; up to 30 min`; past the wait, `the machine is still kept quiet after 30 min: nothing was run`.
* A quiet run: `waiting for the machine to be quiet (2 runs hold it: PIDs 1234, 5678); up to 30 min` (PIDs from `/proc/locks` on Linux, else `other runs hold it`), or the ordinary run's line while another quiet run holds it; past the wait, `the machine is still busy after 30 min: nothing was run`.
* A script refuses with exit status 2; a test panics with the refusal.

### Held by an ancestor

A holder of either kind exports `MOONGLOW_LOCKS_HELD=quiet` (in its comma
list) to what it starts, and anything that sees `quiet` there takes
nothing. This matters more than for the display: a child that shared the
lock again would wait at the gate behind a quiet run that is waiting for
the child's own parent. A Rust holder passes it in the `Command`'s
environment (`mg_testkit::quiet::held_by_us`; `display::held_by_us`
passes `quiet` and `display`).

### How a test run honors it

`cargo test` itself takes nothing. Each test binary shares the quiet lock
for the rest of its run at the first of these, before anything else is
taken:

* its first GPU hold (`mg_testkit::gpu::hold`, `open`, `alone`), before the GPU slot;
* its first display hold (`mg_testkit::display::hold`), before the display;
* its first `mg_testkit::nwn_root()`, which every corpus test calls (`corpus!`).

So a whole suite waits, binary by binary, while a quiet run holds the
machine, and a quiet run waits for the binaries already running to end.
Tests that touch none of these (the plain unit tests) take nothing: they
are light. A binary that waits mid-run does nothing in the meantime.

A timing run holds the machine around the whole command:
`tools/locks.sh hold quiet -- cargo test --release -p mg-ui --test frame_perf -- --ignored`
(its binary sees `quiet` in `MOONGLOW_LOCKS_HELD` and shares nothing). A
Rust program holds it for its own life with `mg_testkit::quiet::hold()`,
called before any GPU or display hold.

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

**The quiet lock, then the display, then a GPU slot.** Nobody upgrades a
lock: a process that shares the quiet lock never takes it alone
(`mg_testkit::quiet::hold()` panics, naming the rule, after anything that
shared it), since two runs doing so would each wait for the other's share.
Taking the display shares the quiet lock first (`mg_lock_display` in
`tools/locks.sh`, `mg_testkit::display::hold`), and a test binary's first
GPU hold does too, so the quiet lock always comes first.

Between the display and the GPU: a test that holds a
GPU place and waits for the display could wait for a run that holds the
display and waits for a slot, so `mg_testkit::display::hold()` panics, naming
the rule, when its first call comes from a thread that holds a GPU place.
The client tests call `common::gpu_after_display()` instead of
`gpu::hold()`.

## Who takes what

| Taker | Takes |
| --- | --- |
| A timing run, Moonglow Client's soak | the quiet lock alone, for its life (`tools/locks.sh hold quiet -- CMD...`, `mg_testkit::quiet::hold()`, or the example below) |
| An orchestrator's suites, any other heavy run by hand | the quiet lock shared (`tools/locks.sh share quiet -- CMD...`); test binaries share it themselves |
| Test binaries (`mg_testkit`'s first GPU or display hold, `nwn_root`) | the quiet lock shared, for the binary's run |
| `tools/aurora/headless.sh start` (only when it launches a session), `stop` | the quiet lock shared and the display, for the command |
| `tools/aurora/run-aurora.sh run` | the quiet lock shared and the display, for Aurora's life (Wine inherits the descriptor; it ends a few seconds after Aurora closes, or at `run-aurora.sh kill`) |
| `tools/nwclient/run-client.sh`, `run-server-and-client.sh` (not with `MG_CLIENT=none`) | the quiet lock shared and the display, for the client's life (a `kill -9` of the sandbox frees it) |
| Client tests (`mg-corpus-tests`: `common::client_lock`, `mg_testkit::joined::start`) | display, once per test binary, kept to the end of its run (a batch reads `DISPLAY` once); runs replaying an earlier run's pictures (`MG_CLIENT_PICTURES`) take nothing (`MG_CLIENT_CAPTURED` takes it: a picture not kept yet is taken with the client) |
| `mg_testkit::gpu` users (mg-render, mg-ui, mg-preview, the corpus tests, the viewer's tests) | one GPU slot per test |
| A person or agent at the shell | `tools/locks.sh hold display -- CMD...` (which shares the quiet lock too) for anything driven by hand: one client run with xdrive calls under it, a whole Aurora capture session |

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

**The quiet lock.** A program that needs the machine to itself (Moonglow
Client's soak) holds it alone, as this shell does around its run (`"$@"`):

<!-- example: quiet-alone -->
```sh
#!/usr/bin/env bash
# Holds Moonglow's quiet lock alone around "$@" (docs/locks.md).
locks="${XDG_STATE_HOME:-$HOME/.local/state}/moonglow/locks"
wait="${MOONGLOW_LOCK_WAIT:-1800}"
# (Under a holder of either kind, nothing to take.)
case ",${MOONGLOW_LOCKS_HELD:-}," in *,quiet,*) exec "$@" ;; esac
mkdir -p "$locks" || exit 2
exec 8>>"$locks/quiet.gate" 9>>"$locks/quiet.lock"
# (The gate first: new ordinary runs stop there while this waits for the
# runs already going. flock -w 0 refuses at once.)
flock -x -w "$wait" 8 || { echo "another quiet run holds the machine" >&2; exit 2; }
flock -x -w "$wait" 9 || { echo "the machine is still busy: nothing was run" >&2; exit 2; }
echo "PID $$, started $(date +%T), $*, in $PWD" > "$locks/quiet.lock"
export MOONGLOW_LOCKS_HELD="quiet${MOONGLOW_LOCKS_HELD:+,$MOONGLOW_LOCKS_HELD}"
"$@"   # the run; the locks end when this shell and what inherited 8 and 9 end
```

A program that is an ordinary heavy run (a suite of its own, a batch of
renders) shares it, and so waits while a quiet run holds the machine:

<!-- example: quiet-shared -->
```sh
#!/usr/bin/env bash
# Shares Moonglow's quiet lock around "$@" (docs/locks.md).
locks="${XDG_STATE_HOME:-$HOME/.local/state}/moonglow/locks"
wait="${MOONGLOW_LOCK_WAIT:-1800}"
# (Under a holder of either kind, nothing to take.)
case ",${MOONGLOW_LOCKS_HELD:-}," in *,quiet,*) exec "$@" ;; esac
mkdir -p "$locks" || exit 2
exec 8>>"$locks/quiet.gate" 9>>"$locks/quiet.lock"
flock -s -w "$wait" 8 || { echo "the machine is still kept quiet: nothing was run" >&2; exit 2; }
flock -s -w 5 9 || { echo "the machine is still kept quiet: nothing was run" >&2; exit 2; }
flock -u 8 && exec 8>&-   # through the gate: given back at once
export MOONGLOW_LOCKS_HELD="quiet${MOONGLOW_LOCKS_HELD:+,$MOONGLOW_LOCKS_HELD}"
"$@"
```

Both take nothing where `MOONGLOW_LOCKS_HELD` already names `quiet`. Take the
quiet lock before the display and before any GPU slot. A program that runs
Moonglow's test binaries needs nothing more: they share it themselves.

**The display.**

1. Open `display.lock` for append (not truncating), `flock -x -w SECONDS`, write your holder line over the file, keep the descriptor open for your run, export `MOONGLOW_LOCKS_HELD=display` to what you start.
2. Take it before restarting or checking the display, and read the display's number only inside the hold. Keep your existing checks.
3. Keep the descriptor out of anything that must outlive you.
4. If you run Moonglow's test suites, nothing else is needed: they take the quiet lock, the display and the slots themselves.
