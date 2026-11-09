---
type: Report
title: 'Code review of November 2026: formats and core'
description: 'The findings of the code review of 2026-11-09 (develop at a46ba28a, version 3.18.1) in formats and core (mg-core, mg-gff, mg-erf, mg-key, mg-2da, mg-tlk, mg-ssf, mg-set, mg-image, mg-resman, mg-audio, mg-script, mg-serve, mg-testkit, mg-update): 25 in all (4 high, 7 medium, 14 low), each with its place, a scenario, a fix and the test that would catch it, found by reading and not run unless an entry says so; then what the reader did not read and what it checked and found sound.'
tags: [review, code-review, formats, mg-core, mg-gff, mg-set, mg-script, mg-update, mg-resman]
status: draft
generated: { by: claude-code/claude-fable-5-2, at: 2026-11-09T22:00:43Z }
stale_after: 2027-01-01T00:00:00Z
sources:
  - id: code
    resource: 'The source named in the title, at commit a46ba28a (develop, 2026-11-09), read by a reviewing agent under the brief the overview gives'
    title: The code as read
  - id: last-review
    resource: commit 2253fa78 (2026-10-27)
    title: The last whole-workspace review, which an entry's Since is told against
---

# Code review of November 2026: formats and core

One part of [the review of 9 November 2026](overview.md): formats and core (mg-core, mg-gff, mg-erf, mg-key, mg-2da, mg-tlk, mg-ssf, mg-set, mg-image, mg-resman, mg-audio, mg-script, mg-serve, mg-testkit, mg-update). An entry's id is the part's letter and the reader's number; the overview's tables use the same ids. Everything here is from reading the code, not from running it, unless an entry says otherwise: *confirmed by reading* means the reader traced the path through its callers, *suspected* that something is left to settle, and says what. *Since* tells code that was there at the last review (before `2253fa78`) from code added after it.
## High

### F1. An update replaces whatever file `$APPIMAGE` names, also when Moonglow is not that AppImage

- **Kind:** bug
- **Severity:** high (wrong file written: another program is overwritten)
- **Where:** crates/mg-update/src/kind.rs:92 (taken if it is a file), :132 (`Kind::AppImage` from it alone); the same trust in apps/moonglow/src/main.rs:438 (relaunch)
- **Since:** after (058953bb)
- **Scenario:** Moonglow built from source or from a distribution's package (normally `Kind::Other`: page link only) is started from a terminal or IDE that is itself an AppImage (WezTerm, Cursor, any Electron AppImage): children inherit its `APPIMAGE=/home/me/Apps/Cursor.AppImage`. Moonglow takes itself for that AppImage, offers Install Now, downloads `Moonglow-<v>-x86_64.AppImage` beside it and renames it over `Cursor.AppImage`; with Install updates automatically, at close and without asking. The other program is gone, Moonglow itself is not updated. "Restart with …" likewise starts the other program.
- **Fix:** take `$APPIMAGE` only when `$APPDIR` is set and `current_exe()` lies under it (the runtime sets both); put that in `Facts` so `detect` can be tested; use the same test in `relaunch`.
- **Test:** unit in `kind.rs`: facts with `appimage` set and `exe` outside the mount (`/usr/bin/moonglow`, `target/release/moonglow`) detect `Kind::Other`.
- **Confidence:** confirmed by reading (`Facts::here`, `Kind::detect`, `stage::fetch`, `install::plan`, `mg-ui/src/updates.rs` `finish`/`get`, `apps/moonglow/src/main.rs`); nothing compares the running program with the image (no `$APPDIR`, no look at `current_exe`)
- **Read again by the reviewer:** holds: `Facts::here` takes `$APPIMAGE` when it names a file, and `Kind::detect` makes `Kind::AppImage` of it alone (kind.rs:92, :132).

### F2. A tileset's highest-numbered section sizes the tile list: 60 bytes of `.set` ask for billions of tiles

- **Kind:** bug
- **Severity:** high (out of memory or a hang on opening a module or hak with a crafted tileset)
- **Where:** crates/mg-set/src/lib.rs:217-241 (`n` = last section number below `Count`, then `(0..n)` with a `format!`, a `missing` and a `gone` entry each), :337-396 (a `Tile` per item)
- **Since:** before 2253fa78
- **Scenario:** `[GENERAL]\nName=x\n[TILES]\nCount=2147483647\n[TILE2147483646]\nModel=a\n`: `last` is 2147483646, so 2.1 billion iterations, each pushing a `usize`, a `String` and then a `Tile`. A smaller number (`TILE50000000`) is still tens of gigabytes. The comment says a count that is no count "costs nothing"; that holds only while no section carries a large number.
- **Fix:** bound `n` by what the file can hold (as GROUPS does with `ini.sections.len()`, with room for holes: e.g. sections × 8 + 4096), and report holes as ranges, not one string each.
- **Test:** unit beside `counts_that_are_no_counts_cost_nothing`: the text above parses in milliseconds with a warning.
- **Confidence:** confirmed by reading (`Sections::items`, its callers for TILES, TERRAIN, CROSSER and the rules; the existing test covers a count with no section, not a count with one far section)
- **Read again by the reviewer:** holds: `n` is the highest section number below `Count`, and `(0..n)` formats a name for each (lib.rs:217-241).

### F3. `#define ENGINE_STRUCTURE_<n>` in a module's or hak's `nwscript.nss` sizes a vector by `n`

- **Kind:** bug
- **Severity:** high (panic or allocation abort from a file a builder downloads)
- **Where:** crates/mg-script/src/spec.rs:125-131
- **Since:** before 2253fa78
- **Scenario:** `#define ENGINE_STRUCTURE_18446744073709551615 x`: `n + 1` wraps to 0 in a release build, `resize(0)`, then `engine_structures[n]` is out of bounds: panic. `ENGINE_STRUCTURE_99999999999`: a resize to 10^11 `String`s, the allocator aborts. Opening any script (or Find Uses of a 2DA row) reads the spec.
- **Fix:** accept `n` below a small bound (the game has 10; 256 is ample), skip the line otherwise.
- **Test:** unit in `spec.rs`: both lines parse to a spec without them.
- **Confidence:** confirmed by reading (`Spec::parse`; callers read `nwscript` through the load order: mg-ui/src/script_view.rs:640, mg-module/src/row_uses.rs:74, mg-preview/src/usage.rs:462, so a module, hak or override copy is parsed)
- **Read again by the reviewer:** holds: `engine_structures.resize(n + 1, ..)` with `n` parsed from the name (spec.rs:128).

### F4. Rename of a struct member panics when the script changed while the Rename window was open

- **Kind:** bug
- **Severity:** high (crash; the window is not modal)
- **Where:** crates/mg-script/src/analysis.rs:751 (`expect("indexed")`), :752-754 (`expect("the member's struct")`)
- **Since:** after (e16a33b2)
- **Scenario:** cursor on `m.sLabel`, Rename Symbol; with the window up, click into the editor and add a line above the struct (or undo something); press Rename. The stored `decl.at.span` matches no member any more: `expect` panics. If the declaring include was removed meanwhile, the first `expect` does. (Other kinds of declaration just find nothing.)
- **Fix:** return `Err("… changed since it was chosen")` for both; better, have `rename` resolve the declaration again from the place.
- **Test:** unit in `analysis.rs`: take a member's declaration, `set` the file with a line inserted above, `rename` returns an error.
- **Confidence:** confirmed by reading (`Index::rename`, `member_references`, mg-ui/src/script_nav.rs:153-160 and :270-296: the declaration is taken when the window opens, `sync_nav` re-indexes before `rename`, and `widgets::dialog` is an ordinary egui window). `mg lsp` is not affected (it resolves and renames in one request).
- **Read again by the reviewer:** holds for the two `expect`s (analysis.rs:751-754); that the window stays open while the script changes is the reader's.

## Medium

### F5. NWNT cannot keep text holding a backslash and `n` (or `r`), and only dotted labels are refused

- **Kind:** bug
- **Severity:** medium (a nasher NWNT project's strings change silently on save and reopen)
- **Where:** crates/mg-gff/src/nwnt.rs:64-66, :132-148, :389-406; also labels with `$`, `=`, `[` or `]` (:221, :237, :285, :345) and a ResRef holding a line break (:104)
- **Since:** after (704ee39f)
- **Scenario:** a variable or a description whose text is the four characters `a\nb` (a builder's own escape for a script to expand, a Windows path such as `C:\new`) is written `…$CExoString = a\nb`; read back it is `a`, LF, `b`. No message; the built module differs from what was edited. This is the format's own limit (as the dot is), but the dot is refused and this is not.
- **Fix:** one check for all of it (`nwnt_unkept`): text that reads back differently, labels with the format's punctuation; refuse as for the dot, or say it in the log.
- **Test:** unit: `from_nwnt(to_nwnt(g)) == g` for a string `a\\nb`, and the check names it.
- **Confidence:** confirmed by reading (`escape` writes LF as `\n` and leaves a typed backslash as it is; `unescape` turns every `\n` into LF; `nwnt_unkept_label` checks `.` only; mg-module/src/nasher/mod.rs:155 is the only guard)

### F6. A second Moonglow started while an update is downloading or staged deletes it

- **Kind:** bug
- **Severity:** medium (the update fails or is reported "isn't installed" though it was)
- **Where:** crates/mg-update/src/stage.rs:294-311; "Nothing staged outlives the run that staged it" assumes one run at a time
- **Since:** after (058953bb)
- **Scenario:** with Install updates automatically, window A has staged `.moonglow-update-3.19.0.AppImage` for its close; the builder opens a second window for another module. B's start removes A's staged file (or A's `.part` mid-download: A then fails with "can't make … runnable" or "can't rename"), B downloads again. Whichever closes second finds its staged file gone: `still` fails and the next start warns "Moonglow 3.19.0 isn't installed: can't read …" after it was installed. On Windows the zip's unpacked folder and the cached installer go the same way.
- **Fix:** a lock file in the cache folder held while an instance downloads or holds a staged update; `clean` skips while another process holds it (or removes only what is older than every running instance).
- **Test:** unit in `stage.rs`: stage, hold the lock, `clean` leaves the staged file; released, `clean` removes it.
- **Confidence:** confirmed by reading (`stage::clean` removes every `.moonglow-update-*` beside the program and all of the cache but the report; `updates::startup` calls it at every start, apps/moonglow/src/main.rs:608; no single-instance guard or lock anywhere)
- **Read again by the reviewer:** holds (drawn in the sample): `clean` removes all of the cache but the report and every file beside the program that begins with the prefix, with no lock or owner (stage.rs:294-311).

### F7. A resource name with a byte above 0x7F does not survive a folder: written as one name, read back as another

- **Kind:** bug
- **Severity:** medium (a resource renamed, or dropped when the name grows past 16 bytes)
- **Where:** crates/mg-resman/src/lib.rs:55-73, crates/mg-resman/src/container.rs:420-425, crates/mg-core/src/resref.rs:78, :105
- **Since:** before 2253fa78
- **Scenario:** a hak or module holds `pièce01.utc` with `è` as the byte 0xE8 (other tools make such names; Aurora doesn't type them). Unpack, Export or Save As into a folder writes the file `pièce01.utc` (0xC3 0xA8). Opening the folder gives the resref bytes `pi`, 0xC3, 0xA8, `ce01`: every reference to the original name misses. A name of nine or more such letters is over 16 bytes read back and the file is ignored as "not a resource".
- **Fix:** one mapping both ways, as `file_name.rs` has for haks (Windows-1252 text ↔ bytes; on Unix the raw bytes), used by `path_in` and `from_filename`; or refuse such names in `is_file_name` with a message.
- **Test:** unit in mg-resman: for a key with byte 0xE9, `from_filename(path_in(dir).file_name())` is the key.
- **Confidence:** confirmed by reading (`ResKey::path_in` → `Display` → `ResRef::to_string_lossy`: each byte as its Latin-1 character, so the file name is UTF-8; `ResKey::from_filename` → `ResRef::from_str`: the name's UTF-8 bytes; mg-module's `folder_key` and `DirContainer` both read through `from_filename`; `is_file_name` lets such names through)

### F8. A compressed resource still expands to whatever its header claims, up to 4 GiB from about 130 KB

- **Kind:** bug
- **Severity:** medium (memory exhaustion from a crafted hak or NWSync file; several at once on loader threads)
- **Where:** crates/mg-erf/src/compressedbuf.rs:66-77
- **Since:** before 2253fa78 (9f4bc8dc)
- **Scenario:** an `E1.0` hak entry whose buffer header says 4294967295 and whose zstd frame is run-length blocks of zeros (4 bytes per 128 KB): `read_to_end` grows the vector to 4 GiB before the size check.
- **Fix:** bound the read as copies are bounded elsewhere (`CopyBudget`'s factor and slack over the frame's length, or the ERF entry's own `size` field cross-checked first), failing once past it.
- **Test:** unit: a hand-built RLE frame claiming 1 GiB is an error after reading no more than the bound.
- **Confidence:** confirmed by reading; this is what the 2026-10-27 fix left: it bounded the *capacity* by the frame (×64), while the *read* is bounded by the header's size alone (`take(size + 1)`), and the comment says a crafted size "runs out of nothing". Not in `docs/deferred.md`.

### F9. A long value in a `.set` makes `Section::float` try every prefix: a hang

- **Kind:** bug
- **Severity:** medium (hostile tileset: seconds for a 100 KB line, minutes for 1 MB)
- **Where:** crates/mg-set/src/ini.rs:30-33 (used for `Transition`, `SelectorHeight`, the grass values and each door's X, Y, Z, Orientation)
- **Since:** before 2253fa78
- **Scenario:** `Transition=` followed by 500,000 digits and 500,000 letters: 500,000 parses of 500,000 digits each.
- **Fix:** find the longest numeric prefix in one pass and parse once, as `mg_2da::parse_float` does; one function for both crates (see 18).
- **Test:** unit: a 200 KB value returns at once, `1.5f` still reads 1.5.
- **Confidence:** confirmed by reading (`(1..=v.len()).rev()…find_map(|n| v[..n].parse().ok())`; `f32::from_str` reads the digits before it fails on what follows)

### F10. A module's hak and talk-table names are joined to folders unchecked

- **Kind:** bug
- **Severity:** medium by the house rule (path traversal), low in effect as far as I could follow it
- **Where:** crates/mg-resman/src/file_name.rs:44-57, crates/mg-resman/src/install.rs:240-243
- **Since:** after for `named_file` (a9b882c3); the join in `find_hak` is older
- **Scenario:** a module naming the hak `../../x` or `/abs/x` makes Moonglow open `x.hak` outside the hak folders (read only; NWSync publishing would take its resources). In `mg serve`/`mg test`, `place` does `remove_file(to)` and links at `<scratch>/hak/../../x.hak`, outside the scratch `hak` folder, when the same relative name exists under a hak folder.
- **Fix:** `named_file` refuses (returns `None` for) names with a separator, a drive colon, `..` or control characters, as `ResRef::is_file_name` does; callers treat it as not found, with a message.
- **Test:** unit in `file_name.rs`; an `apps/mg` test that a module naming `../x` places nothing outside `hak/`.
- **Confidence:** confirmed by reading (`named_file` keeps `/`, `\`, `..` and a leading `/`; `Module::haks` and `custom_tlk` pass the IFO's bytes through; callers: `find_hak`, mg-module/src/talk.rs:119 and :348, mg-module/src/nwsync.rs:112, mg-plugin/src/testing.rs:617, mg-ui/src/nui_view/images.rs:210, apps/mg/src/serve.rs:303 and :312)

### F11. A 2DA cell with a character the codepage lacks fails to save as "contains a double quote"

- **Kind:** bug
- **Severity:** medium (the builder is told the wrong thing and can't find the cell)
- **Where:** crates/mg-2da/src/lib.rs:468
- **Since:** before 2253fa78
- **Scenario:** a cell or column name holding `→`, `ł` or a curly quote in a Windows-1252 module: Save fails with "a cell value contains a double quote, which 2DA cannot express".
- **Fix:** an error of its own naming the row, column and character (found by encoding cell by cell once the whole fails).
- **Test:** unit: `to_bytes` of a table with `ł` under Windows-1252 gives that error.
- **Confidence:** confirmed by reading (`write` maps `codepage.encode` failing to `TwoDaError::Unquotable`)
- **Read again by the reviewer:** holds (drawn in the sample): `codepage.encode(&out)..ok_or(TwoDaError::Unquotable)` (lib.rs:468).

## Low

### F12. `TwoDa::set` takes a value with a line break, which is written as it is

- **Kind:** bug
- **Severity:** low (rows split and shifted on the next read), suspected reachable
- **Where:** crates/mg-2da/src/lib.rs:256-270, :403-410
- **Since:** before 2253fa78
- **Scenario:** a cell set to `a\nb`: the row is written on two lines; read back, every row after it has moved one down.
- **Fix:** refuse `\r` and `\n` where `"` is refused.
- **Test:** unit: `set(0, 0, Some("a\nb"))` is an error.
- **Confidence:** confirmed in mg-2da by reading (`set` and `esc` refuse only `"`); which callers can pass a line break (the plugin API, `mg`'s 2DA commands, a merge) I did not follow; egui's single-line fields replace them on paste

### F13. `mg merge-gff`: the "same in every field" pass compares each unmatched item with the whole other list

- **Kind:** performance
- **Severity:** low to medium (seconds per area in a git merge)
- **Where:** crates/mg-gff/src/merge.rs:563-570 (and `m.contains(&Some(j))` per item at :380, the `partition` over all added items per kept item at :422-433)
- **Since:** before 2253fa78
- **Scenario:** an area's GIT with 5,000 placed objects where one side changed most of them (a mass move, a field set on all with `mg areas`, a re-save that rounds floats otherwise): each of the 5,000 finds no equal and is compared with all 5,000 (struct equality walks tens of equal fields before the first differing float): about 25 million struct comparisons, several seconds; four times that for 10,000.
- **Fix:** hash each side item once and look equal ones up (falling back to equality on a hit); a `used` vector in `added`; place added items from a map by the item they follow.
- **Test:** unit with a time bound: 5,000 items, every one changed on one side, merges in well under a second.
- **Confidence:** confirmed by reading (complexity); cost estimated

### F14. Indexing a script filters every token once per function, twice

- **Kind:** performance
- **Severity:** low to medium
- **Where:** crates/mg-script/src/analysis.rs:186-190 (parameters) and :256-260 (`body_locals`), both a filter over all significant tokens for each function with a body
- **Since:** before 2253fa78
- **Scenario:** a script of 14,000 lines (script_tools.rs:374 names one) with 400 functions is some 110,000 tokens: 2 × 400 × 110,000 = 90 million filter steps, on the order of 0.1 to 0.2 s for every `Index::set`: each `didChange` in `mg lsp`, each `sync_nav` in the editor.
- **Fix:** the tokens are in order: `partition_point` for the body's range (or one pass assigning tokens to bodies).
- **Test:** a timed unit: 400 functions of 30 lines index in a few milliseconds.
- **Confidence:** confirmed by reading (complexity); cost estimated

### F15. `Index::completions` resolves each name with a search of the whole unit: quadratic over `nwscript.nss`

- **Kind:** performance
- **Severity:** low (`mg lsp` only)
- **Where:** crates/mg-script/src/analysis.rs:847-862 calling `top_level` (:415-484), which rebuilds the unit (`unit`: a set, a list and each file's include names, allocated again) and scans every file's functions, then globals, linearly
- **Since:** before 2253fa78
- **Scenario:** every `textDocument/completion`: 7,466 calls, each a unit rebuild and on average some 4,000 string comparisons: about 30 million comparisons and half a million allocations, 0.1 to 0.3 s before the list appears, for any script.
- **Fix:** build the list in one pass over the unit's files with a set of names seen (the declaration made where the name is first met; a function's definition preferred over its prototype as now).
- **Test:** a timed unit over a spec of 7,000 constants.
- **Confidence:** confirmed by reading (complexity; `nwscript.nss` here has 1,187 functions and 6,279 constants, and is last in every unit); cost estimated

### F16. The tileset editor reads counts strictly where the reader reads them leniently

- **Kind:** duplication (drifted)
- **Severity:** low (needs a count with something after the number)
- **Where:** crates/mg-set/src/edit.rs:245-247, :219-224, :349-350 against crates/mg-set/src/ini.rs:20-27
- **Since:** before 2253fa78
- **Scenario:** `[TILES] Count=250;` (or `250x`): the game and `Tileset::parse` see 250 tiles; `add_tile` sees 0, writes `Count=1` and a second `[TILE0]`: the tileset now has one tile and every area of it breaks. `duplicate_tile` on `Doors=2x` copies the key but none of the door sections.
- **Fix:** `SetFile` reads numbers through the same function as `Section::int` (source of truth: `ini.rs`).
- **Test:** unit in `edit.rs`: `add_tile` on a file with `Count=2;` gives `Count=3` and `[TILE2]`.
- **Confidence:** confirmed by reading (`Section::int` takes a sign, digits and ignores what follows; `SetFile::count` is `v.trim().parse::<usize>()`, `doors` `parse::<i64>()`, `duplicate_tile` `parse::<usize>()`)

### F17. A talk-table or soundset entry's sound bytes after the first NUL are dropped on read

- **Kind:** bug (lossless editing)
- **Severity:** low
- **Where:** crates/mg-tlk/src/lib.rs:87, crates/mg-ssf/src/lib.rs:46, crates/mg-tlk/src/json.rs:158-161
- **Since:** before 2253fa78
- **Scenario:** an entry whose 16-byte sound field is `ab\0cd…` (a tool that doesn't clear its buffer): read as `ab`, written back without `cd`; `nwn_tlk` would write `ab\u0000cd`.
- **Fix:** read the 16 bytes and trim trailing NULs only.
- **Test:** unit: such an entry round-trips byte for byte and its JSON has the NUL.
- **Confidence:** suspected: confirmed by reading that `fixed_str` cuts at the first NUL while `TlkEntry::sound` is documented "kept raw because shipped tables contain junk here" and `json::sound_bytes` trims only trailing NULs (logic that can never matter after this read); whether any real table has bytes after a NUL I could not tell without running the corpus round-trip (the shipped tables pass their tests, so not those)

### F18. The same small functions written two and three times, two of them drifted

- **Kind:** duplication
- **Severity:** low
- **Where:** (a) "a float as Nim writes it": crates/mg-gff/src/json.rs:389-437 and crates/mg-tlk/src/json.rs:276-317; they differ where the fixed form ends (`-6..=17` against `-5..=17`), in the exponent (`1e-8` against `e-08`) and in rounding ties; (b) Nim's JSON string escape: mg-gff/src/json.rs:492-509 and mg-tlk/src/json.rs:253-271; (c) "an integer/float as the game reads one, the longest prefix": mg-2da/src/lib.rs:561-617, mg-set/src/ini.rs:20-33 (see 9 and 16); (d) SHA-256: `mg_core::sha256` ("written here rather than taken from a crate") and the `sha2` crate, used by mg-update alone (crates/mg-update/src/verify.rs:8), which already depends on mg-core; (e) zstd: `ruzstd` to read and the reference library (`zstd`) to write, both in mg-erf/src/compressedbuf.rs
- **Since:** (a) before; (d), (e) after
- **Scenario:** (a) is harmless today only because a talk table's lengths are rounded to four places (never below 1e-4); a caller reusing the TLK one for other floats would write `9.99e-06` where `nwn_gff` writes `0.00000999`.
- **Fix:** one `nim_float` and one `escape_json_nim` in mg-core (the GFF one's limits match Nim's current formatter; settle against `nwn_tlk` for a value below 1e-5); one prefix-number reader; mg-update on `mg_core::sha256`; read zstd with the reference library too (its streaming decoder under the bound of finding 8) and drop `ruzstd`.
- **Test:** the differential tests already there, plus a value at each edge (1e-6, 1e-7, 1e17, 1e18) against both tools.
- **Confidence:** confirmed by reading

### F19. External programs run without a time limit, by bare name

- **Kind:** hygiene (house rule: `output_within`)
- **Severity:** low
- **Where:** crates/mg-update/src/install.rs:64-75 (`hdiutil`, `ditto` with `command.status()`: inherited input, no limit, found through `PATH`; run on the download thread, so a stuck `hdiutil attach`, e.g. a disk image that asks to agree to a license, leaves the notice at "Downloading…" for good); crates/mg-resman/src/install.rs:482-491 (`reg query` with `output()`, no limit)
- **Since:** after for mg-update; before for `reg`
- **Scenario:** as above; and a `hdiutil` earlier on `PATH` is run in the update's stead.
- **Fix:** move `output_within` down to mg-core (both crates sit under mg-module) and use it with `/usr/bin/hdiutil`, `/usr/bin/ditto` and a limit; `Stdio::null()` for input.
- **Test:** unit: a `Step::Run` of a program that sleeps fails after the limit.
- **Confidence:** confirmed by reading

### F20. Finding the game on Windows touches every drive letter before it tests the first candidate

- **Kind:** performance
- **Severity:** low
- **Where:** crates/mg-resman/src/install.rs:379-395 (`C:` to `Z:`, then `libraryfolders.vdf` read under two folders of every Steam candidate on every letter), called by `GameInstall::detect` at mg-ui/src/settings.rs:982 (start) and lib.rs:3742
- **Since:** before 2253fa78
- **Scenario:** a mapped but offline network drive: each open waits for the network timeout; the list is built in full (dozens of opens) before `find` looks at the first entry, on the window's thread at start, even when the game is in Steam's default folder.
- **Fix:** make the candidates lazily (default folders and their library lists first, return at the first install; the other letters only after), or off the window's thread.
- **Test:** unit: `find_root` with the default folder present never asks for another drive (the candidate list as an iterator over an injected "exists").
- **Confidence:** suspected (needs a Windows machine with an unreachable mapped network drive to time)

### F21. `userpatch.ini`'s hak names don't go through the hak-name functions

- **Kind:** duplication (drifted)
- **Severity:** low
- **Where:** crates/mg-resman/src/install.rs:69-87
- **Since:** before 2253fa78 (4fcbb72a)
- **Scenario:** on Linux, `PatchFile000=señor` (0xF1): the game opens the file with the byte 0xF1, Moonglow looks for the UTF-8 name and leaves the hak out of the load order.
- **Fix:** `file_name_text` for the bytes and `named_file(name, "hak")` for the file.
- **Test:** unit beside the userpatch tests with a hak named by the byte 0xF1 (Unix).
- **Confidence:** confirmed by reading (`file_name.rs` says every reader of these names goes through it; `userpatch_haks` decodes the file byte by byte as Latin-1 and joins `format!("{name}.hak")`, a UTF-8 name)

### F22. The sandboxed client's CD key copy stays in the scratch folder

- **Kind:** hygiene
- **Severity:** low (a secret left under `target/test-output/`, which is where pictures to share come from)
- **Where:** crates/mg-testkit/src/joined.rs:98, :174-177
- **Since:** after (b01a165c)
- **Fix:** remove the copy in a `Drop` of `Joined` (and in `stop`); or have the sandbox script bind the owner's file read-only at that path instead of copying it.
- **Test:** the joined tests assert the file is gone after `stop`.
- **Confidence:** confirmed by reading (`start` copies `MG_CLIENT_CDKEY`'s file to `<scratch>/user/cdkey.ini`; neither `Joined::stop` nor a `Drop` removes it; `scratch_dir` only empties it at the same test's next run)

### F23. An unpacked zip update and a copied-out macOS app are not checked again before they are swapped in, though `install` says so

- **Kind:** hygiene
- **Severity:** low (the same user can write the program itself)
- **Where:** crates/mg-update/src/install.rs:226-235 (doc: "its files checked again first"), crates/mg-update/src/stage.rs:38-42
- **Since:** after (058953bb)
- **Fix:** say so in the doc, or keep a hash per unpacked file (the zip's own list) and check it in `plan`.
- **Test:** unit: a staged zip folder with a file changed is refused, if the check is added.
- **Confidence:** confirmed by reading (`install` re-hashes `Ready::Image` and `Ready::Installer` only; `Ready::Files` and `Ready::App` carry no hash and wait beside the program until Moonglow closes)

### F24. The NWNT reader has no nesting limit, and its localized-string order is quadratic

- **Kind:** bug (parsers never panic)
- **Severity:** low (megabytes of crafted text are needed)
- **Where:** crates/mg-gff/src/nwnt.rs:265-374, :166-198
- **Since:** after (704ee39f)
- **Scenario:** lines `$Struct = 0`, `.$Struct = 0`, `..$Struct = 0`, … (an empty label is allowed): depth N for about N²/2 bytes; some thousands deep overflows a worker thread's stack. A localized string with 100,000 entries makes `to_nwnt` take 10^10 steps.
- **Fix:** a depth parameter with the binary reader's `MAX_DEPTH`; a map of key to slot in `nim_table_order`.
- **Test:** unit: 5,000 nested structs are an error, not a crash.
- **Confidence:** suspected for the stack (frame size not measured); confirmed by reading that `read_struct` recurses per nesting with no depth count while the binary reader stops at 128, and that `nim_table_order` scans every slot for every entry

### F25. Numbers read from GFF JSON are cut with `as` without a range check

- **Kind:** hygiene
- **Severity:** low
- **Where:** crates/mg-gff/src/json.rs:228, :233 (`StrRef(as_int(..)? as u32)`), :252 (`as i64 as u32` for a struct field's id, while `struct_read` at :152-155 checks the same id's range)
- **Since:** before 2253fa78
- **Scenario:** `"id": 4294967296` in a hand-edited or generated JSON becomes talk-table line 0, `"id": -2` becomes 4294967294; no error.
- **Fix:** the range check `struct_read` has (−1 for none, else 0 to `u32::MAX`), an error otherwise.
- **Test:** unit: both values are errors naming the path.
- **Confidence:** confirmed by reading

## Not read

- The `#[cfg(test)]` modules of every file (not under review; read only where they say what the code is meant to do).
- `crates/mg-testkit/src/hlsl.rs` was read, but whether its binding map repeats wgpu's Direct3D 12 backend could not be judged from this part (the code it mirrors is wgpu-hal's).
- Nothing else: every non-test line of the fifteen crates' `src/` was read. Callers outside the part were read only as far as a finding needed (named in each).

## Looked at and found sound

- **mg-update's chain of trust**: package and staged names come from a parsed `Version` (digits, letters, `-`, `.`, no empty part) and constants, never from the release's text; asset links must begin with the releases' download prefix; `SHA256SUMS` is verified with minisign (prehashed only) against the built-in key, its trusted comment tied to the tag (so an older signed list can't pass for a newer release, and the tag gives the version that must exceed this one); the hash is taken as the download is written and compared before any rename; the staged image and installer are re-hashed before use; zip entries go through `enclosed_name`; every failure path removes its `.part`.
- **The binary readers** (GFF, ERF, KEY/BIF, TLK, SSF, TGA, DDS, PLT, NCS, compressed buffers' headers): every offset and count goes through `Reader`/`slice`, tables are checked whole before a vector is sized by their count, GFF structs and fields are used once and nesting stops at 128, `CopyBudget` is charged for GFF strings, voids and localized strings and for TLK text. (The exceptions are findings 2, 3, 8 and 24.)
- **Private servers**: `Server::start` always passes `-publicserver 0` unless `public` is set and makes up the three passwords not given; `mg_testkit::engine` and `joined` start servers only through it with defaults; the joined sandbox needs `DISPLAY`, the script checks it is the off-screen one and `exec`s bwrap with `--die-with-parent`, so dropping the `Server` ends server and client. Two things to know, not findings: the passwords are on nwserver's command line (visible to other users of the machine), and nwserver listens on all interfaces (the free port is probed on 127.0.0.1 only).
- **The compiler wrapper** (`mg-script/src/compiler.rs`): both callbacks catch panics; the thread-local is set and restored on the thread the compile runs on; the resolver pointer lives only inside `with_active`; the error text is read from the instance after the compile thread has ended. `lex` token spans always end on character boundaries, and every loop of `outline`'s parser advances.
- **`mg_gff::merge`'s order and added-items logic** (new since the last review): followed through its cases (one side reorders, both reorder, items added after a moved item, the same key added twice); no wrong result found.
