# Aurora Toolset pain points, and where Moonglow goes next

Surveyed 2026-10-01, after Moonglow 0.2.0. The question: now that Moonglow
does what Aurora does, what do builders most want changed? Three read-only
surveys:

- **Beamdog's records**:
  - [Beamdog/nwn-issues](https://github.com/Beamdog/nwn-issues): all 109
    issues labeled `toolset` (70 closed, 39 open) and 84 more that mention
    the toolset.
  - The EE release notes from 8154 (2018) to 37-17 (2025).
  - nwn.wiki's toolset pages, mainly *Common Errors and Their Causes* and
    *Toolset F9 Testing Issues*.
- **Community discussion**:
  - Beamdog forums: the 2017–18 wishlist threads, the eight-page EE toolset
    feedback thread (discussion 75035) and Builders - Toolset through 2026.
  - The Neverwinter Vault forum, Steam discussions, and the NWN Lexicon's
    script-editing tutorial.
- **Tools built around the toolset**, about 70 of them: what each fills
  in, how widely it's used (stars, downloads, installs, the persistent worlds
  that depend on it) and when it was last active. Build pipelines of
  persistent worlds were read from their repositories: The Frozen North,
  Ages, Amia, Dungeon Eternal X, Dark Sun and SWLOR.

Caveats:
- Reddit and Discord could not be read, and Beamdog's tracker from before
  2020 was not searched. Casual builders and newcomers are therefore
  under-counted.
- Every toolset issue on Beamdog's tracker has zero reactions, so demand is
  measured by independent reporters and by the tools people built, not by
  votes.
- Counts below are distinct people or projects, not posts.

## 1. Summary

1. **The loudest, longest-running complaints are ones Moonglow already
   answers.** Builders asked for these from 2017 to 2026:
   - a native toolset on Linux and macOS (20+ reports; Beamdog: "not in the
     works");
   - stability without crashes that corrupt modules (15+ crash reports, 12+
     reports of corruption);
   - windows that aren't modal and can sit side by side (12+);
   - a 64-bit toolset (5+);
   - saves that don't drop EE data (nwn-issues #528 and #370);
   - WASD and right-drag cameras (8+).

   These are design rules or shipped features in Moonglow. What's missing is
   telling Aurora users so: several of Aurora's own features (F2 completion,
   Ctrl+R replace) go unnoticed by people asking for them.
2. **The biggest gap nobody's toolset fills is version control.** A `.mod` is
   one binary archive, so the community moved team work to text trees:
   - neverwinter.nim's `nwn_gff` JSON, managed by nasher, is used by at least
     27 public repositories, including most open-source persistent worlds;
   - The Frozen North lints that JSON in CI;
   - Ages runs a bot that resolves `module.ifo` merge conflicts;
   - Amia documents a manual pack, copy, edit, copy and unpack loop, and
     several READMEs warn that unpacking at the wrong moment loses work.

   A toolset that opens and saves the source tree in place, without spurious
   diffs, removes that whole loop. SWLOR's new in-house toolset made "zero
   spurious git diff" a requirement for the same reason.
3. **Next come reference integrity, diagnostics and the test loop.**
   - **Where-used and rename:** "Which object or conversation node uses this
     script?" and "is this hak resource used anywhere?" recur.
   - **Diagnostics:** 15 or so documented crash causes are content mistakes
     that Aurora reports as access violations (bad SET files, 2DA rows
     pointing to nothing, MTR names over 16 characters).
   - **Testing:** builders distrust F9, can't choose the test character, and
     must restart Aurora to see a changed hak or 2DA.
4. **Placement and scripting are the everyday frictions.**
   - **Placement (12+):** locking, groups and prefabs, drop to ground,
     90° rotations, precise numbers.
   - **Scripting (12+):** go to definition, references, diagnostics as you
     type. Three language servers and five VS Code extensions exist because
     the built-in editor lacks them.
5. **Large teams leave Aurora one feature at a time**: scripts first (VS
   Code), then dialogs, quests and stores (into code), then blueprints. Areas
   go last; SWLOR and Sinfar both say area work is the remaining reason to
   open Aurora. Moonglow's area editor, which paints as Aurora paints, is the
   part competitors find hardest (see `prior_art.md`).
6. **2026 brought many replacement efforts:**
   - Radoub (C#, GPL-3, per-file editors);
   - SWLOR.Toolset (Windows, SWLOR-specific);
   - AuroraBorealius (Rust, egui and wgpu, like Moonglow);
   - Borealis (C++ and Qt);
   - rollnw's toolset;
   - NuToolset and nwn-rust-editor (both still empty).

   Their stated aims repeat ours: cross-platform, dark mode and
   accessibility, undo, git-friendly saves, validation, search across haks.
   None claims parity.

## 2. Pain points, and Moonglow 0.2.0 against them

Signal: ●●● many independent reports or a lot of tool-building; ●● several;
● a few. Status: ✅ addressed, ◐ partly, ✗ not yet.

### 2.1 Platform, stability and data safety

| Pain point | Signal | Evidence | Moonglow 0.2.0 |
| --- | --- | --- | --- |
| No toolset on Linux or macOS. Wine, Proton and CrossOver break with updates: wine-staging 10.10–10.15 hid the 3D view in 2025, and CrossOver 22 on Ventura broke it in 2022. | ●●● | Beamdog 66930, 67314, 82450; Vault 7861, 5807, 4110; nwn.wiki's Proton page; every 2026 replacement leads with Linux | ✅ native on all three (macOS not yet tried on a Mac) |
| Crashes when an area opens, tied to GPU drivers (NVIDIA Threaded Optimization, AMD, Intel) and injected overlays (Discord, Logitech) | ●●● | Steam threads 2019–20; Vault 1842, 7605, 8335; nwn-issues #696, #791; nwn.wiki *Common Errors* (about 20 access-violation causes) | ◐ wgpu on Vulkan, Metal or D3D12 rather than legacy OpenGL. The viewer is optional, since everything else works without a GPU. Untested against overlays. |
| Error loops (EEFFACE, "List index out of bounds") that force a Task Manager kill | ●●● | nwn-issues #369 (open since 2021), #163; Beamdog 71177 | ✅ parsers never panic; errors go to the log |
| Module corruption after crashes, after F9's save prompt, and from crash recovery ("3–4 out of 5"); backups that are all zeros | ●●● | Vault 5531 (140 hours lost), 3791, 7138; Beamdog 88400; nwn-issues #791 (a truncated save) | ✅ atomic saves with a `.bak`, recovery copies kept apart from the module, crash reports |
| Regressions after game patches. The toolset updates with the game and can't be pinned. | ●● | Vault 7655 (37-15 "critical toolset bugs"); nwn-issues #596, #593, #598, #766 | ✅ released separately from the game |
| Bad custom content crashes the toolset rather than naming the file or row. SET counts, missing tile models, lightcolor.2da over 32 rows, baseitems.2da over 255 rows and MTR names over 16 characters are each a different access violation. | ●●● | nwn.wiki *Common Errors*; Vault 6602, 6613; nwn-issues #154, #168 | ◐ no crashes, but no diagnosis either: a bad row shows up as a missing model or a log line (§3, item 3) |

### 2.2 Scale

| Pain point | Signal | Evidence | Moonglow 0.2.0 |
| --- | --- | --- | --- |
| A 32-bit toolset: out of memory on big areas, a crash at the 152nd area, no haks over 2 GB | ●● | Beamdog 89094, 90400, 67054; nwn.wiki *Resource Limits* | ✅ 64-bit; 300 areas tested. Over 2 GiB, the game itself can't read what a hak holds past the mark (`notes_scale.md`): Moonglow reads it as the game does, and the content doctor names it |
| Inventories take 20–30 s to open with 2,500–8,000 item blueprints. Area Properties freezes with 200+ areas. | ●● | nwn-issues #368, #374 (both open); #438 | ✅ budgets on a synthetic persistent world (`notes_scale.md`): a store of 1,000 items opens in 0.7 s, Area Properties with 300 areas in 8 ms |
| Build Module and saving are slow on large projects | ● | Beamdog 75035 p4; release notes 8186 and 8192 | ✅ Tyrants of the Moonsea compiles in 0.7 s and writes in 0.02 s |

### 2.3 Team work and version control

| Pain point | Signal | Evidence | Moonglow 0.2.0 |
| --- | --- | --- | --- |
| A `.mod` can't be diffed or merged; one person edits at a time | ●●● | nasher (50★, in 27+ repositories), neverwinter.nim (149★, 18.5k downloads), NWNT, nwn-devbase ("not created with collaboration in mind"); Vault 2187, 5125, 3330, 6613 | ◐ module folders open and save, but hold binary GFF; `mg gff` converts single files |
| The round trip between toolset and source tree loses work when run at the wrong moment | ●● | Amia, Ages and The Frozen North READMEs; Dark Sun onboarding ("45 minutes" one-on-one) | ✗ (§3, item 1) |
| Every branch that adds an area conflicts on `module.ifo`'s area list | ● | Ages' merge bot (`ifoupdate.py`) | ✗ |
| Code-page damage (windows-1250 vs 1252, color tokens) on the way through text | ● | nasher #119; Ages README | ◐ text stays bytes in its code page in Moonglow; JSON trees not yet supported |
| Saves drop EE fields Aurora doesn't know (texture, animation and shader replacements), turn StrRefs into embedded strings, and clobbered the module ID | ●● | nwn-issues #528, #370; release notes 8193.36; nwn.wiki's area-flags and creature JSON pages | ✅ lossless by design: untouched fields round-trip byte for byte, and the EE fields have fields: an editor's Visuals page, Shader Flags, a tile's replacement texture (`notes_ee_fields.md`) |

### 2.4 Finding things, references and validation

| Pain point | Signal | Evidence | Moonglow 0.2.0 |
| --- | --- | --- | --- |
| No "where is this used": which object or conversation node runs a script, whether a hak placeable is placed anywhere | ●● | Vault 1346, 609, 5637, 7161; Radoub #1318 and #1319 | ◐ Verify lists missing and unused resources, and a reference graph exists (`mg-module` `refs`), but there's no Find References command |
| Area resrefs can't be renamed ("area001" forever); renaming by hand breaks the module | ● | Vault 6613 (31 posts) | ✗ only blueprints rename |
| Build errors don't point at their cause (a missing script, but in which conversation node?) | ●● | Vault 609, 6602, 7138; Lexicon tutorial | ◐ Verify names each missing reference's source; build diagnostics as in Aurora |
| No validation for CI: teams write their own lint suites | ●● | The Frozen North's RSpec lints; nwn_sqlite; Moneo; nwn-mcp | ◐ `mg verify` exists; no rules beyond references, no machine-readable output |

### 2.5 Testing

| Pain point | Signal | Evidence | Moonglow 0.2.0 |
| --- | --- | --- | --- |
| F9 is distrusted: combat lag, corruption, a frozen toolset after the game quits. The wiki says to launch `+TestNewModule` by hand. | ●●● | nwn.wiki *Toolset F9 Testing Issues*; nwn-issues #314; Vault 6711, 7093 | ◐ Test Module saves first and runs the game separately. Not yet checked against the issues on that page. |
| The test character can't be chosen (it's the first alphabetically; people prefix names with `_`) | ●● | Beamdog 84125; Vault 6711, 7093; nwn-issues #595 | ✗ the first local character |
| Changed haks, 2DAs and TLKs only show after a restart | ●● | nwn-issues #346; Beamdog 70087 ("reload resources"); Vault 7765 | ✗ |

### 2.6 Areas and placement

| Pain point | Signal | Evidence | Moonglow 0.2.0 |
| --- | --- | --- | --- |
| Placement: no locking, groups or prefabs, no 90° or rotation hotkeys, unpredictable Z (forced grounding, then none in EE), coordinates shown only as integers | ●●● | Beamdog 67054, 69624, 75035 p3–p8; Vault 749, 3540; World Shaper; the axs patch | ◐ Adjust Location takes exact values, and objects can be raised. No locking, groups or drop to ground. |
| Visual transforms and Static fight: scale lost, not kept on paste, not stored in blueprints | ●● | Vault 2939; nwn-issues #567, #465 | ◐ transforms kept on paste; blueprints follow the engine |
| Rotating or resizing areas crashes Aurora | ●● | nwn-issues #766, #324 | ✅ Resize and Rotate Area match Aurora's results |
| Bugs in EE's multi-area tabs: lost property changes, the start location breaking | ●● | nwn-issues #688, #365, #709, #639; Beamdog 79051 | ✅ tabs are native; the start location is drawn on the ground |
| No walkmesh, AABB, PWK or skybox display; no walkmesh cutters | ● | Beamdog 69624, 75035 p2 | ◐ walkmesh overlay and skyboxes; no PWK or AABB toggles |
| Tile painting feels random; no control over variants | ● | Beamdog 69624 p2, 67054 | ◐ Aurora's rules, with Shift + click stepping variants and a "next variant" command |
| The mouse wheel zooms the area while scrolling a palette | ● | nwn-issues #458, #626 | ✅ egui routes the wheel to what's under the pointer |
| Camera: WASD, right drag, arrow keys too fast | ●● | Beamdog 89838, 69624; nwn.wiki | ✅ in 0.2.0 |

### 2.7 Blueprints, palettes and bulk edits

| Pain point | Signal | Evidence | Moonglow 0.2.0 |
| --- | --- | --- | --- |
| No palette search; messy categories (CEP's long lists) | ●● | Steam "Toolset Search" (2021); Vault 7150, 7936; Radoub #2566 | ◐ Find filters by name and resref; no tag, fuzzy search or hover preview |
| Changed haks leave palettes blank or stale ("Refresh Palette" confusion) | ●● | nwn-issues #346; Vault 7765 | ◐ palettes are rebuilt from the module; no reload of changed haks |
| Update Instances works on one blueprint at a time; no mass edit of blueprints | ●● | Beamdog 78962 (Winter's LOTR), 82940; SWLOR's store-sync script; Radoub #1318 | ✅ Update Instances for a selection or a whole palette category, editing several blueprints together, Find and Replace across the module's text |
| No variable sets or templates | ● | Beamdog 69624 p2, 82940 | ✅ variable sets (Save Set, Add Set); blueprints are the templates |
| The item editor won't load DDS icons; PLT-layered icon bugs; MinRange ≥ 100 breaks appearance pickers | ●● | nwn-issues #455, #456, #658 | ◐ DDS icons load; MinRange ≥ 100 not checked |
| The creature editor can't set familiars, companions, domains or the wizard school, and strips some of them | ● | nwn.wiki Creature and Creature JSON pages; nwn-issues #566, #816 | ✅ domains, school, familiar and companion on the Classes page, with the rule for when the game reads them (`notes_ee_fields.md`) |

### 2.8 Scripts and conversations

| Pain point | Signal | Evidence | Moonglow 0.2.0 |
| --- | --- | --- | --- |
| A weak script editor. Builders move to VS Code and edit in `temp0`. | ●●● | Lexicon *NWN:EE Script Editing Tutorial* (2025); Vault 3321; Beamdog 69624, 67054, 84125; 3 language servers, 5 VS Code extensions (~3k installs), Notepad++, Sublime, Vim and Emacs plugins | ◐ completion, Find in Files, editor tabs, dark theme, an external editor whose saves come back. No go to definition, references or live diagnostics. |
| Poor compiler messages; slow full compiles | ●● | Vault 7056; nwn-issues #736, #561; ARE_Compile (Arelith) | ◐ Beamdog's own compiler, messages link to lines; 1,433 scripts in 0.7 s |
| The conversation editor: tiny font, no undo for nodes, no script names on nodes, paste-as-link bugs | ●● | Vault 6783, 1183, 749; Beamdog 67054; nwn.wiki Conversation Editor page | ✅ node undo (unlimited), zoomable UI, Paste As Link, lines name their scripts and journal updates, a Test that follows conditions |
| Dialogue can't be written outside the toolset and imported | ●● | Beamdog 67054; Vault 6783 (Articy, Twine, Ink); Flamewind; Radoub Parley | ✅ export to text, CSV, Twine and Ink; import from Twine and Ink, and CSV lines back |
| No spell check that works (EE ships no dictionary) | ● | nwn.wiki (spell check "wipes text"); Radoub; TlkEdit | ✗ left out on purpose (`12-differences.md`) |

### 2.9 Custom content data

| Pain point | Signal | Evidence | Moonglow 0.2.0 |
| --- | --- | --- | --- |
| No 2DA or TLK editing; 2DA rows from several haks merged by hand. Builders are split on whether this belongs in a module toolset. | ●● | Eos Toolset (1.1k downloads, its own wiki section), TlkEdit-EE, Killer TLK (3k), at least 7 2DA mergers; Beamdog 69368 (virusman: "pointless") | ◐ hak conflict report; 2DAs and TLKs readable in the resource browser |
| Hak tooling is primitive (nwhak truncates names; Aurora locks up when nwhak has the hak open) | ●● | nwn-issues #82; NWN Explorer (10.5k downloads); Aurora Hak Explorer (2026) | ◐ resource browser, `mg pack` / `unpack` / `ls`; no hak editor in the GUI |

### 2.10 Interface

| Pain point | Signal | Evidence | Moonglow 0.2.0 |
| --- | --- | --- | --- |
| Modal windows that open behind the main window and lock it | ●●● | Beamdog 66932, 69624, 84125; nwn-issues #230, #368 | ✅ tabs and floating windows |
| Unreadable at high DPI; icons vanish at scaling that isn't a multiple of 50%; no dark mode | ●● | Vault 6225, 6783; nwn.wiki *Aurora Toolset*; Beamdog 82940 | ✅ follows the system's scale and theme; Ctrl + plus and minus zoom |
| Keys can't be remapped | ● | Beamdog 69624, 75035 | ✗ |
| Small things: 4 recent modules, column widths forgotten, tab order | ● | Beamdog 75035, 82940 | ◐ 10 recent modules |

### 2.11 Reported pain that isn't the toolset's to fix

- Encounter design (spawn radius, schedules, per-creature chances; Beamdog
  68067) is limited by what the engine reads from an encounter.
- NWSync publishing exists as tools (`nwsync`, neverwinter.nim, a GitHub
  Actions template). Nobody asked for it inside the toolset.
- No NWN1 complaints about missing undo turned up; the "no undo" posts
  are about NWN2.

## 3. Map of improvements

Ordered by demand and by how much each one sets Moonglow apart. Effort: S is
days, M is a week or two, L is longer.

### Tier 1: reasons to switch (next releases)

1. **Git-native modules (L).** Open a nasher project (`nasher.cfg`, its
   source globs, JSON or NWNT per file) as a module, edit it in place, and
   save only the files that changed.
   - **Diffs:** written exactly as `nwn_gff` writes them, so a save makes no
     spurious diff.
   - **Building:** `mg build` (and Build Module) packs the `.mod`.
   - **Area list:** on open, areas missing from `module.ifo`'s list are
     offered, and listed areas that are missing are reported, so merges
     that only touch the list stop hurting.
   - **Code pages:** text keeps its code page.
   - **Tests:** every shipped module through a JSON tree and back with no
     diff against `nwn_gff`; packing compared with nasher's.

   This removes the round trip that every public persistent-world pipeline
   documents. It extends the "git-friendly project folders" idea from
   `PLAN.md`.
2. **Where-used and rename everywhere (M).** Find References for a script,
   resref, tag, conversation or blueprint, covering placed instances,
   conversation nodes and module events, built on `mg-module`'s reference
   graph.
   - **Rename:** renaming an area, script or blueprint updates every
     reference, in one undoable step.
   - **Unused hak content:** "unused in this module" for a hak's resources,
     with clippy's caveat that scripts can build names at run time.
3. **Content doctor (M).** Verify gains a custom-content pass that names the
   file, row and column:
   - 2DA rows pointing to missing models, icons or StrRefs;
   - SET files whose counts or sections are wrong, or whose tile models are
     missing;
   - MTR names over 16 characters;
   - rows beyond the limits the engine enforces;
   - 2DAs shadowed by an older hak.

   `mg verify` gains machine-readable output and a failing exit code, for CI
   in place of hand-written lint suites. Each check is established in the
   engine first, as the rules require.
4. **A test loop builders trust (M).**
   - **Character:** choose the test character, or the module's DefaultBIC.
   - **Test from here:** start at the camera's location, by running a
     scratch copy of the module with its entry point moved, never touching
     the real one.
   - **Reload Resources:** watch the haks, override, `development/` and
     talk tables, and re-index on demand or on change, without a restart.
   - **F9's known problems:** check the wiki's list against Moonglow's
     launch.
5. **Placement tools (M).**
   - **Positioning:** drop to ground for the selection; rotate by 90° and
     15° with keys.
   - **Alignment:** snap to grid and angle; align and distribute; mirror.
   - **Locking:** a lock that keeps objects from being picked. It's a
     Moonglow setting kept beside the module, not a field the game would
     carry.
   - **Prefabs:** a group of objects, with their offsets and transforms,
     saved and placed like a blueprint.
6. **Tell Aurora users what's already fixed (S).** A "Coming from Aurora"
   chapter in the manual and a section in the README. It answers each of the
   loud complaints in §2 that Moonglow has already addressed, and points out
   features people miss (F2 completion, Find in Files, palette Find).

### Tier 2: depth for heavy users

7. **Script intelligence (M–L).** Go to definition (into includes and
   `nwscript.nss`), Find References, Rename Symbol, and diagnostics as you
   type from the built-in compiler. Also `mg lsp`, so VS Code, Neovim and
   Emacs users get the same engine (the "language server" idea in
   `PLAN.md`). Incremental compiles of only what changed and its dependents,
   as Arelith's ARE_Compile does.
8. **Persistent-world scale (S–M, mostly tests).** Performance budgets on a
   synthetic persistent world: 8,000 item blueprints, 300 areas, 50 haks,
   and a hak over 2 GB. Cover inventories, Area Properties, palettes with
   CEP, opening and saving. Fix what they show.
9. **The EE data Aurora hides (M, in small pieces).**
   - Texture, animation and shader replacements on objects (#528).
   - Area flags beyond the three.
   - Tile texture replacement.
   - Familiars and companions, cleric domains and the wizard school on
     creatures.
   - Placeables with an inventory that aren't usable, if the engine allows
     them.
   - Item stacks and costs beyond Aurora's UI limits.

   Each is checked in the engine before it gets a field.
10. **Bulk editing (M).**
    - Update Instances for many blueprints, or a whole palette category, at
      once.
    - Edit a field across several blueprints.
    - Saved variable sets.
    - Find and Replace across the module's text: names, descriptions,
      conversation lines and journal entries.
11. **Conversation authoring (M–L).**
    - Mark nodes that have conditions, actions or scripts, and show their
      names.
    - Import and export dialogue: plain text and CSV first, then Twine or
      Ink.
    - Play a conversation through with its conditions shown.
    - Spell checking through the system's dictionaries, since EE ships
      none.
12. **Palettes (S–M).** Search by tag, fuzzy matching, a preview on hover,
    recently used and favorites, and moving custom blueprints between
    categories by dragging.

### Tier 3: later, or wait for demand

13. **Custom content data (M–L, contested).** Start with a read-only 2DA view
    that resolves StrRefs and shows which hak each row comes from, and a
    custom TLK editor. A full 2DA editor and merger is Eos's territory;
    revisit when asked.
14. **Haks in the GUI (M).** Build a hak from a folder, browse and extract a
    hak's contents, and attach haks and a TLK to a module in one step (the
    job of NIT and the PRC installer).
15. **Area visibility (S–M).** PWK and AABB toggles, and minimap export
    (`mg minimap`; today nwn_minimap does this).
16. **Remappable keys (S–M).**
17. **Automation (M).**
    - JSON output from every `mg` command.
    - Query commands such as `mg refs` and `mg find`.
    - Later, a plugin or scripting API. Aurora has none, so every Aurora
      extension patches its exe. Tools for AI assistants (nwn-mcp,
      nwn-manager) are a new pressure toward documented formats and CLIs.
18. **NWSync publishing from the CLI (S–M),** if asked for; neverwinter.nim
    already does it.
19. **Tileset authoring (L).** A niche, served by the Set File Editor; the
    SET reader and the painter already exist.

### Ordering notes

- **Items 1–3 build on each other.** The reference graph serves Find
  References, rename and the content doctor. JSON trees make all three
  matter more, because teams run them in CI.
- **Item 6 is the cheapest win.** It's also the right moment for it: the
  repository goes public once these settle, and that is when Aurora users
  will look.
- **Item 5** covers what builders do most in an area, and areas are what
  keeps teams on Aurora.
- **Items 7 and 1 serve the same people,** the persistent-world teams who
  already live in VS Code and git.

## 4. Gaps in this survey

- **Reddit and Discord weren't searched,** and the community's Discord
  servers are where most builders talk now. Once the repository is public,
  ask there and on the Vault which of Tier 1 matters most.
- **Some reports are only reported:**
  - inventory times with thousands of items;
  - the 152-area limit;
  - haks over 2 GB.

  Moonglow should measure these (item 8), not assume them. Done:
  `notes_scale.md`. The 2 GB limit turned out to be the game's too.
- **Several Aurora bugs need a check in Moonglow before it claims to avoid
  them:**
  - MinRange ≥ 100 in the item appearance pickers (#658);
  - Polish characters (#534);
  - epic multiclass saves (#816);
  - spells for custom caster classes (#566).
