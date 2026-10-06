---
type: Manual Page
title: Conversations
description: Conversations - the conversation tree, a line's tabs, writing conversations elsewhere, the Script Wizard and backups.
tags: [manual, conversations]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-06T22:14:51Z }
---

# Conversations

**Tools › New Conversation…** makes a conversation (`.dlg`); double-click
one in the module tree to open it in the **Conversation Editor**, laid out
as Aurora's.

## The tree

The conversation is a tree under **Root**: the NPC's lines (red, headed
`[OWNER]` or the speaker's tag) and the player's replies (blue). A gray
line is a **link**: it stands for a line elsewhere in the tree, so
branches can join and loop without copying text. Select a line to edit it
below the tree: its **Speaker** (NPC lines: the conversation's owner, or
a creature in the module by tag) and its **Text** (the **…** button edits
every language). A line whose text is a talk-table string (as all the
original campaign's are) shows that string in the tree and under the
empty field; text typed in the field is said instead.

With the pointer over the editor, Ctrl+A adds a line, Delete deletes the
selected one, and Ctrl+C, Ctrl+X and Ctrl+V copy, cut and paste, as in
Aurora (Options › Keyboard changes Add's and Delete's keys).

| Command | Does |
| --- | --- |
| Add | a reply to the selected line (an NPC line under Root or a player line, a player line under an NPC line) |
| Copy, Cut | the selected line and everything under it |
| Paste | the copied lines as new lines under the selected one |
| Paste As Link | a link to the copied line under the selected one |
| ⏶ and ⏷ (Alt+Up, Alt+Down) | move the selected line up or down among its parent's lines: the order the game tries a speaker's lines in and lists a player's replies in |
| Delete | the selected line and everything under it (a link: just the link) |
| Expand All, Collapse All | open or close the whole tree |
| Export | the conversation as plain text, CSV, Twine or Ink (see below) |
| Import Lines… | read back an edited CSV export's text |
| Scripts | show each line's condition, action, journal update and sound |

With **Scripts** on (the default), each line names what it does besides
its text: `if c_has_key` (its Text Appears When), `do a_give_gold` (its
Actions Taken), `journal q_rats 20` and `sound vs_hello`.

**Dragging** a line onto another moves it there; Ctrl + drag (Cmd on
macOS) links it instead. Lines move only where they fit (an NPC line under
a player line or Root, and the reverse). Options › Conversation Editor
sets which way Paste As Link and Ctrl + drag link, and whether a new line
first asks for its text in a popup.

**Search** finds text in this conversation or every conversation in the
module (match case, whole words, replace); **Bookmarks** remember lines to
go back to. With the pointer over the editor, Ctrl+F (or Ctrl+R) opens
Search and F3 finds again, as in Aurora.

**Test** plays the conversation as the game does: the NPC says the first
of its lines whose condition passes, and the player is offered the
replies whose conditions pass, numbered.
- **Conditions:** Moonglow can't run the scripts, so each counts as TRUE
  until you click it to make it FALSE. Lines passed over show as not said
  or hidden.
- **What happens:** each line's actions and journal updates are listed.
  A transcript keeps what was said; **Back** goes back a turn.

(Aurora's Test shows every line, whatever its condition.)

## A line's tabs

- **Text Appears When…**: the script that decides whether the line is
  offered (it returns TRUE or FALSE), with its parameters. **Script
  Wizard…** writes one from a few choices.
- **Actions Taken**: the script that runs when the line is spoken, with
  its parameters; also from the Script Wizard.
- **Other Actions**: the animation the speaker plays, the sound it says
  (**Play** to hear it), and the journal entry it sets (a quest and an
  entry).
- **Comments**: notes for the builder.
- **Current File**: the conversation's own settings: the scripts run when
  it ends normally or is aborted, and whether the camera may zoom in.

**Tokens** (`<FirstName>`, `<Class>`, custom tokens…) go into the text from
**Token…**; the game replaces them when the line is spoken. A language
can have tokens of its own (the game's Polish has many English doesn't):
Token… lists those of the language you edit (Options › General), and in
the **…** window for a text in several languages each language's row has
its own **Token…**, which puts the token at the end of that text.

## Writing conversations elsewhere

**Export** writes the conversation in other formats:
- **Plain text**: a readable script, for proofreading or review.
- **CSV**: one row per line (`E3` an NPC line, `R5` a reply) with its
  speaker, text, condition, action and comment. Edit the text in a
  spreadsheet or have it translated; **Import Lines…** then reads back
  the speakers, text and comments of the lines it names, as one undoable
  step. Lines whose text comes from the game's talk table show empty.
- **Twine** (Twee 3, for Twine 2 or Tweego) and **Ink** (for Inky): the
  conversation as a branching story.

**File › Import Conversation…** reads a Twine (`.twee`) or Ink (`.ink`)
story as a new conversation named after the file, so dialogue can be
written in those tools and brought in. The mapping:
- **NPC lines** are passages (Twine) or knots (Ink). Tags give a line's
  speaker, action, journal update and sound: `speaker:TAG`,
  `do:a_script`, `journal:q_rats:20`, `sound:vs_hello`. In Ink a tag goes
  on its own line, as `# do:a_script`.
- **Player replies** are a passage's links or a knot's choices, leading
  to the next NPC line or to `END`.
  - Twine: `[[Who are you? {if c_curious} {do a_note}->E5]]`.
  - Ink: `+ {c_curious} [Who are you?] -> E5 # do:a_note`, the condition
    declared with `VAR c_curious = true`.
  - An empty reply is `(Continue)`.
- **Several NPC lines to choose from** (the first whose condition passes
  is said):
  - Twine: a passage tagged `npc-choice`, its links `[[if c_seen->E2]]`
    and `[[otherwise->E3]]`.
  - Ink: a knot of conditional diverts, `{c_seen: -> E2}` then `-> E3`.
- **A passage or knot reached twice** becomes a link.

Text Moonglow writes is escaped where the format would read it as syntax:
in Twine as HTML character references, which story formats show as the
characters; in Ink with backslashes, and a blank line as `//`. Only the
English text goes out, not condition and action parameters, animations,
comments or delays. Every conversation the game and its modules ship
plays the same after a round trip through Twine or Ink. Ink beyond this
subset (stitches, gathers, logic) is refused with its line number.

## The Script Wizard

From Text Appears When… or Actions Taken, **Script Wizard…** writes a
script without typing. Tick what to test (items the player carries, skill
checks, local variables, class and level, race, gender, alignment,
abilities, feats, skills, a random chance) or what to do (give or take
items, gold and experience, set local variables, change a faction, open a
store…), fill in a page for each, and name the script. **Finish** writes
the source, compiles it and sets it on the line, as one undoable step.
The script is the module's like any other: open it in the script editor
to change it later.

## Backups

While a conversation with unsaved changes is open, Moonglow backs it up
as `<name>.bak` every 5 minutes, as Aurora does (Options › Conversation
Editor), in a `moonglow-backups/<module>` folder in the system's temporary
folder. Moonglow's recovery copies (see [Modules](03-modules.md)) also
keep the whole module safe.
