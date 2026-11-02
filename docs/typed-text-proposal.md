---
type: Proposal
title: 'Text typed into the talk table: a design'
description: Putting a name, a description or a conversation line typed in Moonglow into the module's custom talk table as a new line, its StrRef in the field, where Aurora writes the text into the file - where the lines' range comes from (the project file, set in Module Properties), which fields, reuse of a line that has the text already (asked first), Undo taking the line and the field back together, saving the table with the module, languages and the feminine table, two builders on one project and a range running out, what Verify and mg say; the owner's questions, and what was built.
tags: [proposal, talk-table, tlk, blueprints, conversations, project-file]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-02T08:38:34Z }
sources:
  - id: deferred
    resource: deferred.md
    title: 'The deferred list: "New strings written to a talk-table range" (Blueprint editors and inventories) and "Text put in the talk table as it is written" (Custom content data), the 3rd and 4th large entries after 3.0.0'
  - id: builders
    resource: human:august, relaying builders' reports after 1.19.1 and a second builder's review (October 2026)
    title: 'The ask: text in the talk table as it is written, in a range set aside, a line with the same text reused after asking'
  - id: code
    resource: crates/mg-module/src/talk.rs, crates/mg-module/src/tlk_alloc.rs, crates/mg-module/src/project_file.rs, crates/mg-ui/src/talk_view.rs, crates/mg-ui/src/widgets.rs (String Edit), crates/mg-ui/src/lib.rs (apply, Undo, save), crates/mg-edit/src/lib.rs (read 2026-11-02, at 3.2.0)
    title: The code this is built on
---

# Text typed into the talk table: a design

Aurora writes the text typed into a name or a description into the file
(the localized string's own text in the language edited). A persistent
world translated, or one that keeps its text in one place, wants it in
the module's custom talk table instead: a line of the table, and the
field naming it by its StrRef.[^builders] The deferred list kept the
ask twice, from a blueprint and from a conversation.[^deferred] Moonglow had Move to Talk
Table (String Edit), one string at a time, at the table's end, and
`mg tlk-alloc`, strings kept by name in a file.[^code] This design makes
the text typed go there as it is typed, when the builder asks for it.

## Off by default

**Options › Language › Talk Table › Put text typed into the talk
table** turns it on, off by default: Aurora writes the text into the
file, and a module without a custom talk table, or a builder who wants
the text in the file, sees no change. Move to Talk Table stays for one
string at a time (and takes its line from the same range).

## Where the range comes from

The project's file, `moonglow.toml` (beside a `.mod`,
`NAME.moonglow.toml`), section `[talk]`:

```toml
[talk]
lines = "100000-149999"             # the custom table's lines typed text takes (0 is StrRef 16777216)
map = "tlk/strings.strrefs.json"    # mg tlk-alloc's map: the lines it gives keys are never taken
[talk.builders]                     # a builder's own part of the lines, by their code
ab = "100000-109999"
cd = "110000-119999"
```

- **Module Properties › Custom Content › Talk Table** shows the range
  and sets `lines` (written into the project's file, the rest of the
  file kept as it is), with the next free line and how many are left.
  A module never saved has no place for the file: the table's end is
  used.
- **Without a range**, a line is added at the table's end, as Move to
  Talk Table did.
- **A builder's own part** (`[talk.builders]`, by the code in
  `.moonglow/user.toml`) is theirs alone; a builder without one takes
  from `lines` outside every builder's part.
- **Lines passed over**: one with text or a sound (in either table),
  and one the map names (in use or retired). The next free line is the
  first line of the range past the table's end or empty.

## Which fields

The fields a builder types a localized string into: every Properties
window's name, description and other localized text (blueprints, placed
objects, areas, the module, the journal), a creature's random name, and
a conversation line's text (typed in the editor, or in Input Text when
a line is added). They are marked as typed where they are committed (the
keyboard leaving the field, or its window closed while typing), and
`Moonglow::apply` puts them in the table (`Action::ApplyTyped`). Not
converted: String Edit (it edits every language's text as written, and
has Move to Talk Table), the GFF editor (the file as it is), imports,
pastes, wizards, fixes and plugins.

## What a string typed becomes

A new line of the module's custom table, at the next free line of the
range, with the text typed (the feminine table, where there is one,
takes the same text, or the string's own feminine text in that
language). The field's StrRef becomes the line's (16,777,216 + line, as
the game reads it), and its own text in the table's language (masculine
and feminine) is taken out, so that the game reads the table's. Edit
Together's several objects take one line. A text typed again later is another line: a line once given keeps
its text, as `mg tlk-alloc`'s do, so a number saved in a character's
item never comes to name another text; the talk table editor changes a
line everywhere it is named.

## Reuse

Where the table has a line with the same text already (any line of the
custom table, the feminine text the same too), Moonglow asks: **Use
Line N**, **Add a Line** or **Keep in the File** (Aurora's way, for this
text). Options › Language › When the talk table has the text: Ask (the
default), Use that line, or Add a line.

## Undo

The table keeps its own history for what is done in its editor. A typed
line is the module's change: it is put in the table outside the table's
history, and noted with the module's step that set the field (by its
depth in the module's undo list). Undo of that step takes the line out
(the table's last line removed, another emptied) and Redo puts it back,
in one step with the field. Where the line was changed in the table
editor since, Undo leaves it as it is (a line no field names is
harmless; one taken away from a field is not), and Redo with its place
taken refuses, saying why. Cancel in the talk table editor drops its own
changes and keeps the typed lines.

## Saving

The table is saved with the module, before it (written beside each file
and moved over it, feminine table first; a table kept in the module is
a step of the module's save). Where the table can't be saved and has
typed lines the module names, the module is not saved either, and the
log says why: a module naming lines its table hasn't would show empty
text in the game. Closing without saving drops both.

## Languages and gender

The table has one language (`tlk.language`). Text typed in another
editing language (Options › Language) stays in the file: it is not the
table's. A string with text in another language as well stays in the
file too: the game shows any text of a string's own before its StrRef's
line, in the game's language or another (an English game named an item
by its French text beside the StrRef; `engine_tlk`
`a_typed_name_is_read_from_the_custom_talk_table`, [findings](findings.md)),
so its line would be shown to nobody, and taking the other text out
would lose it. So does a string with a feminine text of its own where
the table has no feminine table. A nasher project keeping the table in more than one language
keeps typed text in the file (a line in one language's table alone would
be empty in the others), and says so once. Text the table's codepage
can't write stays in the file, said in the log. The feminine table
(`NAMEf`), where the module has one, takes the line too, so the line
exists for both genders.

## Persistent-world scale

- **A range running out**: the text stays in the file and the log says
  the range is full; Verify warns when a tenth of the range is left.
- **Two builders on one project**: each takes from their own part
  (`[talk.builders]`), so two checkouts never give one line to two
  texts. The map's lines are never taken, and `mg tlk-alloc` passes over
  the project's typed lines (it reads the project's file beside the
  names file or above it, or `--project`), so the names kept in a file
  and the text typed share one table.

## Guards

Verify's **talk-range** check (the Module group): the range nearly or
wholly used, lines the map gives inside the range, builders' parts that
overlap one another or reach past `lines`, and a builder with no part
of their own where the others have one. The project's file says what it
can't read (a range that isn't one, past line 16,777,215). `mg verify`
says the same. A StrRef past the table's end was said already.

## The owner's questions

1. **Off by default?** Recommended and applied: off, as Aurora.
2. **Reuse asks?** Recommended and applied: ask, with Options to always
   use the line or always add one.
3. **The range in the project's file**, set from Module Properties, not
   in Moonglow's own settings: one place for every builder of the
   project and the command line. Applied.
4. **A text typed again is a new line**, not the line changed in place
   (whose number a player's item may carry). Applied.
5. **The switch is each builder's** (Options), not the project's: a team
   that wants it for everyone says so in its setup. A `[talk] typed`
   key could turn it on for a project.
6. **A string with another language's text stays in the file** (the
   game would show that text before the line, to everyone). The other
   way is to take every language's text out, losing translations typed
   into the file. Applied: kept, and the log says so.

## What was built

Built 2026-11-02 as designed, but for one change the engine asked for
(other languages' text, above):

- `mg_module::talk_range`: the lines (`Lines`, `TalkRules`, `Allowed`),
  the next free line and how many are left, the string a field becomes
  (`in_line`); `[talk]` read and `lines` written by
  `project_file` (`set_talk_lines`, `talk_map`, `talk_allowed`).
- `mg_module::talk::Table`: lines put in for the module's fields outside
  the table's history (`put_typed`, `take_back`, `put_back`; a line
  before the last emptied, not the lines after it taken), what makes the
  table unsaved, its Cancel keeping them (`drop_own_changes`), and the
  first line with a text (`find_text`).
- `mg_ui::talk_typed`: `Action::ApplyTyped` from every typed
  localized-string field, the question, the lines kept with the module's
  steps by their depth in its undo list (`Workspace::undo_depth`), the
  game data's custom table made the open one's so the fields show a line
  at once; Options › Language, Module Properties' Typed text, the
  module's save stopped by a table that can't be saved.
- Guards: Verify's `talk-range` (`doctor::talk_range`, also `mg
  verify`), `mg tlk-alloc` passing over the project's typed lines
  (`--project`).
- Tests: `talk_range`'s and `talk`'s unit tests,
  `talk_lines_are_read_and_written`,
  `the_lines_typed_text_takes_are_held_to_the_table`, the UI tests in
  `crates/mg-ui/tests/app/typed_text.rs` (a name typed, Undo and Redo,
  saved; a text the table has, asked; off by default; a conversation
  line; a full range and a table that can't be saved; several fields and
  another language), `new_keys_pass_over_the_project_s_typed_lines`,
  the engine's `a_typed_name_is_read_from_the_custom_talk_table`;
  pictures in `screens.rs`'s `typed_text_talk_table`.

See the [log](log.md) of 2026-11-02 and the manual's
[Blueprints](manual/05-blueprints.md),
[Conversations](manual/06-conversations.md),
[Modules](manual/03-modules.md), [Options](manual/10-options.md) and
[the command line](manual/11-command-line.md).

[^deferred]: The deferred list's two entries
[^builders]: Builders' reports after 1.19.1, a second builder's review
[^code]: The code read at 3.2.0
