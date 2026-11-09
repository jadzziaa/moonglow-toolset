---
type: Report
title: 'Code review of November 2026: the rules data and what the first pass left'
description: 'The findings of the code review of 2026-11-09 (develop at a46ba28a, version 3.18.1) in the rules data and what the first pass left (mg-rules, parts of mg-nui, mg-area, mg-schema and mg-module): 11 in all (0 high, 4 medium, 7 low), each with its place, a scenario, a fix and the test that would catch it, found by reading and not run unless an entry says so; then what the reader did not read and what it checked and found sound.'
tags: [review, code-review, mg-rules, mg-nui, script-wizard]
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

# Code review of November 2026: the rules data and what the first pass left

One part of [the review of 9 November 2026](overview.md): the rules data and what the first pass left (mg-rules, parts of mg-nui, mg-area, mg-schema and mg-module). An entry's id is the part's letter and the reader's number; the overview's tables use the same ids. Everything here is from reading the code, not from running it, unless an entry says otherwise: *confirmed by reading* means the reader traced the path through its callers, *suspected* that something is left to settle, and says what. *Since* tells code that was there at the last review (before `2253fa78`) from code added after it.
## Medium

### G1. `image_names` recurses without end on a bind whose first value names a bind: `mg nui preview` and the Creator's Export crash

- **Kind:** bug
- **Severity:** medium (a stack overflow, which aborts the process past the panic handler, on a downloaded window's settings)
- **Where:** crates/mg-nui/src/lib.rs:499-512 (`add`); callers apps/mg/src/nui/preview.rs:222, crates/mg-ui/src/nui_view.rs:570
- **Since:** before 2253fa78 (b2a9b17e)
- **Scenario:** `evil.jui` holds `{"type":"image","value":{"bind":"a"}}` (or any object with an `image` key), `evil.txt` holds `"bindings":{"a":{"value":[{"bind":"a"}]}}`. `add` replaces `{"bind":"a"}` by the bind's value, finds an array, calls itself on each item, which is `{"bind":"a"}` again: no depth limit (serde_json's 128 bounds the document, not this walk). `mg nui preview evil.jui` dies with a stack overflow; so does Export in the Creator on such a window after Import… (two binds naming each other do the same).
- **Fix:** resolve a bind once, at the top: inside a bind's value a `{"bind":…}` object is data, as in the game (`add(v, s, out, resolved)`), or carry the set of binds being resolved.
- **Test:** `lib.rs` unit test beside `a_windows_pictures_are_those_it_names…`: a bind whose value is `[{"bind": itself}]`, and two binds naming each other, return (an empty set).
- **Confidence:** confirmed by reading (`add` inside `image_names`; `Binding::value` is any JSON and `Settings::parse` checks only `format`; the two callers; the unit test covers a string value only)
- **Read again by the reviewer:** holds (drawn in the sample): `add` takes a bind's value and calls itself on each item of an array (lib.rs:499-512).

### G2. A torso and a body's parts past 255 are read from the BYTE field: wrong armor class, cost and proficiency, and a false "No model" on the creature

- **Kind:** bug (a drift between copies: `part_number` is the rule)
- **Severity:** medium
- **Where:** crates/mg-rules/src/items.rs:162 (`torso: int("ArmorPart_Torso")`), :216; crates/mg-rules/src/models.rs:196-204 (`number = utc.integer(label)`)
- **Since:** items.rs before 2253fa78 (1d3a3b1a, 0b271c93); `body_models` after (eae16fbf); the editors' `min(255)` after (d265ee3b)
- **Scenario:** a hak with chest and head models numbered past 255 (EE allows 999). (a) In an armor's Appearance choose torso 300: the file gets `ArmorPart_Torso = 255`, `xArmorPart_Torso = 300`. `armor_class` reads parts_chest.2da row 255: the Statistics' armor class, weight and Total Cost, the `Cost` written after every change of the item (mg-ui/src/blueprint/item.rs:63-70), the gear term of the challenge rating and `required_feats` are row 255's. With row 255 a plate and row 300 a robe, `wear_problem` says the wearer lacks Heavy Armor Proficiency and the creature's preview takes the armor off. (b) Give a creature head 300: the preview draws `pmh0_head300`; `appearance_problems` (the warning under Appearance, Verify's `object-row`) looks for `pmh0_head255` and reports "No model", or says nothing when 300 is the one missing.
- **Fix:** read the three through `part_number` (`ItemValue::from_gff`'s torso, `required_feats`' torso, `body_models`' parts and head).
- **Test:** unit tests: `items.rs` `costs_follow_the_engine` with `ArmorPart_Torso = 255`, `xArmorPart_Torso = 2` gives row 2's armor class and cost; `models.rs` `a_body_s_skeleton_and_parts` with `xAppearance_Head = 300` beside `Appearance_Head = 255` names `…_head300`.
- **Confidence:** confirmed by reading (`ItemValue::from_gff`, `required_feats`, `models::body_models` against `items::part_number`; the preview reads every one of these through `part_number`: mg-preview/src/creature.rs:53,285,328 and item.rs:110; Moonglow's own editors write the BYTE as `min(v, 255)` and the number in the twin: mg-ui/src/blueprint/item.rs:327-336 `set_part`, :508-514 `with_model`; `client_render/batch4.rs:960` says the game reads the twin first)

### G3. The Statistics page's saves leave out save feats the game hard-codes

- **Kind:** bug
- **Severity:** medium
- **Where:** crates/mg-rules/src/creatures.rs:229-242
- **Since:** before 2253fa78 (84d2cee5)
- **Scenario:** a blackguard 10 of Charisma 18 with Dark Blessing, or an epic fighter with Epic Fortitude: the page shows saves 4 lower than `GetFortitudeSavingThrow` gives in the game.
- **Fix:** add the feats the engine run confirms to `feat` and to `saves_modifier`.
- **Test:** `engine_saves.rs` `saving_throws_match_the_engine` with one creature per feat.
- **Confidence:** suspected (read: `creature_stats` adds feats 14, 22, 24 (+2), 248 and 382 (+1) and 217 alone; `docs/findings.md` "Saving throws past level 20" says the page follows the engine and was measured on thirteen Creature Wizard creatures. Not read in any measurement: Epic Fortitude 583, Epic Reflexes 585, Epic Will 695 (+4 each), Dark Blessing 473 (Charisma to all, as Divine Grace), Strong Soul 388 (+1 Fortitude and Will), Bullheaded 380 (+1 Will), Snake Blood 386 (+1 Reflex): constants from nwscript.nss. A run of `engine_saves.rs` with a creature holding each settles it)

### G4. An item's Properties page builds and sorts every subtype list of every property type each frame

- **Kind:** performance
- **Severity:** medium (with a large hak's tables; low with the game's)
- **Where:** crates/mg-rules/src/items.rs:632-646 (`available_properties`), :650-663 (`named_rows`), :683-726; callers crates/mg-ui/src/blueprint/item.rs:944, :971, :1146, :1164, :1181
- **Since:** before 2253fa78 for the lists; the sort after (96c4bab1)
- **Scenario:** a module on the PRC's haks, whose iprp_feats.2da follows its feat.2da (over twenty thousand rows, from memory: the haks are not on this machine): each frame of the Properties page names and sorts all of them (a talk-table lookup, three strings and a sort key a row): tens of milliseconds, where the budget is 6.9. With the game's own tables it is about 1,000 rows a frame, inside the budget as measured.
- **Fix:** keep the lists as `choice_list` keeps its own: an `Arc<Vec<Choice>>` per table and use column in `GameData` (dropped by `invalidate` and `set_custom_tlk`), sorted once; the page asks for a type's list only when its header is open or the search has text.
- **Test:** `frame_perf`: the Properties page with a synthetic iprp_feats.2da of 25,000 rows in the test module's hak stays inside the frame.
- **Confidence:** confirmed by reading for the path (`properties` in mg-ui runs every frame the page shows and calls `game.available_properties(base)`, then for each type `mg_rules::by_name(game.property_subtypes(t, base))` before it knows whether the type's header is open; `named_rows` makes a `String` per row from the talk table; `property_editor` builds three more lists for the property selected). The cost is estimated, not measured: `frame_perf`'s "item blueprint, Properties page" (e9ad7fc2) runs on the game's tables (iprp_spells 540 rows, iprp_feats 62), and the per-frame sort came after it (96c4bab1)

## Low

### G5. The draw layers' table has no arc center (`c`) and says it is `a`; `row_count`'s schema takes an array the checks refuse

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-nui/src/properties.rs:401 (`"A line's or curve's start; an arc's center"`), :379-412 (no `c`), :149-150 (`RowCount`'s definition); crates/mg-nui/src/validate.rs:500-504, :757-759
- **Since:** after (4dbfed7b)
- **Scenario:** (a) a program writes an arc from `mg nui schema --layout`: the schema names no `c` and describes `a` as the arc's center, so the center goes into `a`; the preview reads `c` (the key nw_inc_nui gives the game), finds none and draws the arc at the control's corner, and `mg nui validate` passes it. A `c` of the wrong kind (`"c": true`) or a bind in `c` is not checked either, and a `set` handler on that bind is not held to a point. (b) `"row_count": [1, 2, 3]` written in the JUI passes the schema (`#/$defs/row_count` is "integer or array") and fails the checks ("Expected integer"): the array is the bind's value, in the settings, never in the layout.
- **Fix:** add `c` for type 3 with a kind that takes a point or a rectangle (the comment at validate.rs:757 says the game draws both) and correct `a`'s text; make `row_count`'s definition an integer and say the bind's array in its description.
- **Test:** `schema.rs` `the_layout_schema_is_the_checks_table`: every key each `NuiDrawList*` constructor writes (a list in the test, from nw_inc_nui) is in `$defs/draw`; `validate.rs`: an arc with `"c": true` is an error.
- **Confidence:** confirmed by reading (`properties::DRAW`; nw_inc_nui's `NuiDrawListArc` writes `"c"`, `"radius"`, `"amin"`, `"amax"`; the preview reads `point("c")` at mg-ui/src/nui_view/draw.rs:210 and the Creator writes `item["c"]` at :27; `Check::draw` holds only the table's keys; `Check::table` makes an unbound `row_count` `Kind::Int`)
- **Read again by the reviewer:** holds for the table (drawn in the sample): `a` is described as "an arc's center" and there is no `c` (properties.rs:401).

### G6. The Script Wizard writes amounts past 2,147,483,647 and floats that are not numbers into the script

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-module/src/script_wizard.rs:154 (float), :368-374 and :577-625 (amounts); crates/mg-ui/src/script_wizard.rs:241
- **Since:** before 2253fa78 (bb79ea90)
- **Scenario:** Take XP `3000000000`: the script holds `int nXP = GetXP(GetPCSpeaker()) - 3000000000;`, compiled as minus −1294967296: the player's XP is set 1.29 billion higher. Give gold of that size is a call with a negative amount. A float local set to `nan`, `inf` or `1e39` is written as `NaN` or `inf`, an identifier the compiler does not know: the wizard's own script does not compile.
- **Fix:** amounts are `1..=i32::MAX` (refused in the page as a bad number is); `VarType::value` refuses a float that is not finite.
- **Test:** `script_wizard.rs` `local_values_follow_their_type`: `check(VarType::Float, "nan")`, `"inf"` and `"1e39"` are `None`; an app test: Take XP of `3000000000` leaves Next disabled (or the amount out).
- **Confidence:** confirmed by reading (`to_action`'s `amount` parses a `u32`; `ActionScript`'s fields are `u32`; the built-in compiler adds an integer literal up digit by digit in an `int32_t` (`crates/mg-script/nwscript/native/scriptcompparsetree.cpp:494-512`), so a number past `INT_MAX` wraps to a negative one without an error; `VarType::value` takes whatever `f32::from_str` takes and prints it with `{:.6}`)

### G7. Loops and allocations bounded only by a 2DA's numbers

- **Kind:** bug
- **Severity:** low (a hang or an allocation abort on a hostile or broken hak)
- **Where:** crates/mg-rules/src/levelup.rs:410-437 (`slots = (base + bonus) as usize`, then `while list.len() < slots { push }`: a `cls_spgn_*.2da` cell of 2000000000 makes two billion spells and as many structs); :604-617 and :663-668 (`InvSlotWidth`/`InvSlotHeight` cut to `u16`, then `occupy` inserts width × height cells into a `BTreeSet` and `x + i` adds in `u16`: 40000 by 40000 is 1.6 billion inserts); crates/mg-rules/src/item_wizard.rs:173-176 and :252-253 (`(0..=first).rev().chain(first + 1..=self.top)` calls `at`, a scan of the rows, once per tier number up to an `itmwiz*.2da` `Value`, and `step * self.top * 2` overflows `i32` past 89 million)
- **Since:** before 2253fa78 (0f56c75c, ea6d3ffb)
- **Scenario:** the Levelup or Creature Wizard adds Wizard to a creature in a module whose hak has `SpellLevel1 = 2000000000` at that level: the window hangs and memory runs out. A creature carrying an item whose base item has `InvSlotWidth 40000`: the same in `new_class_gear`. The Item Wizard's Magical on a table whose only rows are of tier 0 and 100000000.
- **Fix:** hold the slots to what a creature's list can be (a byte, as the game's own tables), the sizes to the backpack's (10 wide; skip an item that cannot fit), and walk the tiers the rows have (sorted, distinct) instead of the range of numbers.
- **Test:** unit tests: `level_up` with a spell gain cell of `i32::MAX` returns with a bounded list; `new_class_gear` with a 60000 by 60000 base item returns; `ItemMagic::generate` with tiers 0 and 100000000 returns.
- **Confidence:** confirmed by reading (each value comes from `get_int` with no bound; whether `mg verify` warns of such cells was not checked)

### G8. A class new to a creature gets no spells of a level whose slots are bonus slots alone (a paladin's and a ranger's first): the level-up and the spell warnings count bonus slots two ways

- **Kind:** bug
- **Severity:** low
- **Where:** crates/mg-rules/src/levelup.rs:410-421 (and its module comment, lines 27-34, "the `SpellGainTable`, and bonus slots for the casting ability"); crates/mg-rules/src/spell_warnings.rs:118-128
- **Since:** before 2253fa78 (0f56c75c)
- **Scenario:** the Levelup Wizard adds Paladin 4 to a creature of Wisdom 14: no `MemorizedList1`, though the class casts first-level spells from its bonus slot; the Spells page then offers one slot, empty.
- **Fix:** tell a cell of 0 (a level the class casts, by bonus slots) from `****` (none): `get_int(…)` is `Some(0)`; one function for "the slots of a level" that both the level-up and `spell_slots` call, with the two differences (level 0, known spells) as its arguments.
- **Test:** `levelup` unit test with a gain table of `SpellLevel1 0` and Wisdom 14: one `MemorizedList1` spell; with `****`, none.
- **Confidence:** suspected (read: `new_class_spells` gives a bonus slot only where `base > 0` and `n > 0`; `spell_slots`, Aurora's check, adds the bonus to a base of 0 and at level 0; the game's `cls_spgn_pal.2da`, printed with `nwn_resman_cat`, has `SpellLevel1 = 0` and `NumSpellLevels = 2` at levels 4 and 5, where the game gives a paladin of Wisdom 12 one slot. What Aurora's Levelup Wizard writes there is not in the captures named: a capture of Paladin 4 with Wisdom 14 settles it)

### G9. Rules written in mg-rules and again elsewhere, some drifted

- **Kind:** duplication
- **Severity:** low
- **Where:** (a) a body's skeleton (gender letter, race letter, phenotype and its `DefaultPhenoType`): crates/mg-rules/src/models.rs:100-129 (`skeletons`) and crates/mg-preview/src/creature.rs:185-195. (b) a torso's armor class: crates/mg-rules/src/items.rs:189-192 rounds `ACBONUS`, crates/mg-preview/src/item.rs:110-120 cuts it (`ac as i32`): a hak's 1.5 is leather for the cost and the proficiency and cloth for the model lying on the ground. (c) "is armor": `GameData::is_armor` (items.rs:195-198), crates/mg-ui/src/images.rs:90-94, and crates/mg-rules/src/appearance.rs:299-303, which compares the cell to the text `"3"`. (d) `EquipableSlots`: crates/mg-rules/src/levelup.rs:646-652 parses the hexadecimal itself (strips every leading `0x`, reads `16` as 0x16) where crates/mg-ui/src/blueprint/creature_lists.rs:1696 and model_view.rs:2054 use `get_int`. (e) a placeable's light: crates/mg-rules/src/placeables.rs:34-35 (`parse::<f32>() > 0`) and crates/mg-preview/src/object.rs:53 (`cell_int > 0`): `3x` is a light for one and not the other. (f) the first free numbered name: crates/mg-module/src/script_wizard.rs:714-724 (`(1..)` with `expect("a free name")`, no end) and `Naming::numbered` (project_file.rs:544-549, to 9999, an `Option`). (g) a row's name: items.rs:650-663 (`named_rows`: a row without a `Name` number is left out), :761-764 (`name_of`: its label), rows.rs:438-452 (`named_row`: `DisplayName` too). (h) the base Fortitude save: levelup.rs:537 (each class's table at its full level) and creatures.rs:212-227 (the first 20 levels, then a point every two: the engine's); ability totals: creatures.rs:202 counts the Great feats, levelup.rs:86-88 and spell_warnings.rs:117 do not. (i) `USE_LIMITATION_CLASS`: item_wizard.rs:42 and items.rs:19.
- **Since:** before 2253fa78 (models.rs is after)
- **Scenario:** see (b), (d), (e) and (h) for where two copies answer differently.
- **Fix:** the preview calls `models::skeletons` and `GameData::armor_class`; `is_armor`, `get_int` and `has_light` are the one reading each; the Script Wizard's name comes from `Naming::numbered`; `named_row` names the property lists' rows, with the "skip a row without a `Name`" as an argument.
- **Test:** unit tests where the copies differ: a parts_chest.2da `ACBONUS` of 1.5 gives one armor class to the cost and the preview; `EquipableSlots` of `16` and `0x10`.
- **Confidence:** confirmed by reading; the drifts are said where there is one

### G10. More unchecked arithmetic on numbers from files (places finding 22 of the earlier report does not list)

- **Kind:** hygiene
- **Severity:** low
- **Where:** crates/mg-rules/src/creatures.rs:107-126 (`int(a) as i32`), :202 (`abilities + racial + great`), :204 (the levels' sum in `u32`), :248, :257; crates/mg-rules/src/challenge.rs:66, :90 (`lvl * (die as u32 + 1)`), :97 (a division by the PC's walk rate, 0 where creaturespeed.2da row 0 has no `WALKRATE`: the sum becomes NaN or infinite, and Build › Creature CR writes every creature 1/8 or 9.2e18; with a `CRAdjust` above 0, `rounded + i64::from(c.cr_adjust)` at :164 overflows too); crates/mg-rules/src/levelup.rs:81, :234 (`*spare as i32 + per_level`); crates/mg-rules/src/item_wizard.rs:175
- **Since:** before 2253fa78
- **Scenario:** a UTC whose `Str` is an INT of 2147483647, or two classes of level 4294967295: the Statistics page panics in a test build and shows wrapped numbers in a release.
- **Fix:** read abilities, levels and bonuses into their fields' ranges (`clamp`), sum with `saturating_add`, and give the hit point term 0 where the PC's rate is not above 0.
- **Test:** `creature_stats` and `challenge` unit tests with the fields at `i64::MAX`, and a creaturespeed.2da whose row 0 is blank: finite numbers, no panic.
- **Confidence:** confirmed by reading (release builds wrap, as the workspace sets no `overflow-checks`; a debug or test build panics)

### G11. Comments that say what the code does not, and a check left open without an entry

- **Kind:** hygiene
- **Severity:** low
- **Where:** crates/mg-rules/src/names.rs:8-10 ("between 4 and 13 letters") against :93-107: a name of 13 letters still takes its end letter, so 14 come out (the toy table of the test cannot reach it). crates/mg-rules/src/items.rs:200-209: "`ReqFeat0`–`5`" and `(0..6)`; baseitems.2da has `ReqFeat0` to `4` (mg-module/src/doctor.rs:2299-2303). crates/mg-rules/src/levelup.rs:32-34: "(not yet checked against Aurora)" for a class that knows its spells, with no entry in `docs/deferred.md`. crates/mg-rules/src/levelup.rs:548-559: `base_attack` is public as "the base attack bonus of a creature's classes" and sums each class's table at its full level, which past 20 levels is not the game's (a fighter 30: 30, the game 25); its one caller is a feat's `MINATTACKBONUS`. crates/mg-rules/src/challenge.rs:107 sums the special abilities' `Innate` as they are and :121 with `.max(0)`: a negative cell counts in one term and not the other.
- **Since:** before 2253fa78
- **Scenario:** none a builder sees today, the 14-letter name aside.
- **Fix:** correct the comments (or stop the name at 13), read `0..5`, file the known-spells check, name `base_attack` for what it is or give it the epic rule.
- **Test:** `names.rs`: a table that can only end after 13 letters yields no name longer than the comment says.
- **Confidence:** confirmed by reading

## Not read

Nothing of the part's code. Test modules (`#[cfg(test)]`) were read only where a finding needed them (items.rs, item_wizard.rs, names.rs, schema.rs, lib.rs of mg-nui); appearance.rs 466–753 and rows.rs 497–723 (tests) were not.

## Looked at and found sound

- `item_cost` against nwn.wiki's account of the engine's formula and the module comment: property cost by type else subtype, the cost table by itempropdef's `CostTableResRef`, spells at a half each plus a half and a quarter of the two tracked, the truncations and the stack; every float-to-integer cast saturates, and every table is read through `get`/`get_int`/`get_float`, which give `None` for a missing table, column or row and for `****`.
- `Ltr::read` and `generate`: the header, the letter count (26 or 28) and the whole length are checked before any slice; every index into the tables and `LETTERS` is below the letter count; a hundred tries bound the work.
- The Script Wizard's text: every string a builder types (a tag, a ResRef, a variable's name and value) goes through `quote` (backslash, then quote); a constant's name is nwscript.nss's own or the row's number, never a 2DA's cell; integers are parsed and printed again; the pages' fields are single lines, and the compiler refuses a line break in a string.
- The NUI tables against nw_inc_nui's constructors (read from the game's include): the window's, every control's, a chart series' and the draw layers' keys, kinds and static or bound, the arc's center aside; the schema and the checks read the one table; `fit_returns` works on the lexer's tokens inside the handlers' sections only, so strings, comments and the top section's functions stay as written.
- `challenge`'s nine terms against the table in its module comment and `docs/findings.md`, term by term; `Alignment`'s rows and groups, the Use Limitation and Cast Spell property rows, the Great ability feats' rows (764 to 823) and the armor proficiency feats against nwscript.nss's constants.
