---
type: Manual Page
title: Conversations
description: Conversations - the editor's header and tree, a line's Data and the Search in the Right Dock's Node section, the Conversation panel, the bookmarks in the Outliner, the Status Bar's hints, the Conversation Test, writing conversations elsewhere, the Script Wizard and backups.
tags: [manual, conversations]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-25T08:49:31Z }
---

# Conversations

**Tools › New Conversation** makes a conversation (`.dlg`); double-click
one in the module tree to open it in the **Conversation Editor**: its
header (what you do to the lines, how they look, then OK and Cancel) over
its tree; the line chosen, the conversation's Search and its own settings
in the Right Dock beside it (in a window of its own, in the window's
right pane), and its bookmarks in the Outliner under the Left Dock's
tabs (in a window of its own, at the window's left).

## The tree

The conversation is a tree under **Root**: the NPC's lines (red, headed
`[OWNER]` or the speaker's tag) and the player's replies (blue). A gray
line is a **link**: it stands for a line elsewhere in the tree, so
branches can join and loop without copying text. Thin **guides** run
down from each line's arrow past the lines under it, so you can see which
line a reply belongs to deep in a branch; the line selected is marked the
tree's whole width. The tree scrolls up and down, never sideways.

A line too long for the tree is cut at its right edge with an ellipsis
(…); `[END DIALOGUE]` always stays whole after it, and its scripts'
marks follow, each drawn whole or not at all. Where there is no room
for them all, the words are cut first, down to a third of the tree's
width (at least twelve times the text's size), and then the marks go
from the right (`if`, `do`, `journal`, `sound`: the last first). Point
at a line cut, or short of a mark, to read the whole line and the marks
with no room; the Node section's Text shows the line too. A line deep in
a branch stands in no further than leaves its words that room, its
guides with it.

Double-click a link, or right-click it for **Go to Linked**, to go to
the line itself (the
branches above it open out). Select a line to edit it in the Right
Dock's **Node** section, its **Data** tab (see [a line's
Data](#a-lines-data)): its **Speaker tag** (NPC lines: the conversation's
owner, or a creature in the module by tag) and its **Text** (the **…**
button edits every language). A line whose text is a talk-table string
(as all the original campaign's are) shows that string in the tree, in
the field itself in the talk table's color, and under the field
("From the talk table (string 1234): …"); text typed in the field is
said instead. Over its
fields the line's letters and words are counted, the talk table's text
where that is what is shown, and the file's words.

While its tree has the keys (see below), or with the pointer over the
editor, Ctrl+A adds a line, Delete deletes the selected one (on macOS the
delete key too, which is Backspace), and Ctrl+C, Ctrl+X and Ctrl+V copy,
cut and paste, as in Aurora (Options › Keyboard changes Add's and
Delete's keys). Delete deletes only the line: the whole conversation is
deleted from the module tree (its menu's Delete…, which asks first).

**The keyboard.** Opening a conversation (from the module tree, or
bringing its tab to the front) gives its tree the keys, as Aurora does,
and so does a click in the editor or in its panels in the Right Dock (a
thin outline shows it has them), as a click gives them to the module tree;
they stay its until you click another list, point at an area's view or
press Escape (Aurora's Escape does nothing there; Moonglow's hands the
keys back, as in the module tree). Then, as in Aurora's tree:

| Key | Does |
| --- | --- |
| Up, Down | select the line above or below, as the tree shows them (Root is the first) |
| Right | open the selected line's branch; if it is open, go to its first line |
| Left | fold the selected line's branch; if it is folded (or has none), go to the line it is under |
| Home, End | select Root, or the last line shown |
| Page Up, Page Down | go up or down by as many lines as the tree has room for |
| Backspace | go to the line the selected one is under (from a first line, Root); on macOS, where it is the delete key, it deletes the line |
| + and − on the keypad | open or fold the selected line's branch, the selection staying |
| * on the keypad | open everything under the selected line, branches folded further down too (from Root: the whole tree) |
| A letter or a digit | go to the next line shown whose row begins with it, upper or lower case, round to the top after the last; the same letter again goes on to the next |
| Letters typed in quick succession | go to the first line shown, from the selected one on, whose row begins with all of them (`gol` finds "Gold, please." past "Goodbye."); a pause of a second starts over |
| Ctrl+Up, Ctrl+Down | scroll the tree a line up or down, the selection staying |

Letters go by a row's text as the tree shows it, as Aurora's do: an NPC
line's row begins with its speaker, so `[` goes to the NPC lines and
`[owner] - g` to the owner's line beginning with G (with the speakers
hidden, Options › Conversation Editor, by the line's own text); Root is
"Root". A key you give one of the conversation's commands in Options ›
Keyboard (Delete Line on Backspace, say) is the command's, not the
tree's.

The line selected scrolls into sight. While you type in a field (the
line's Text, Find What), the keys are the field's: they move its text
cursor, and letters, Backspace and − type there; once Tab has taken you
on from the field to a button, they are the tree's again. A list of
choices Tab reaches (Play animation, Journal) keeps the arrow keys: they
choose in it, and the tree's selection stays. The − above the letters
folds too, as the keypad's does, and Shift+8 opens everything out as the
keypad's * does (Moonglow can't tell them apart).

Folding a branch with the arrow before its line (or Collapse All) while a
line in it is selected selects the line folded, as Windows' tree does, so
the selection stays in sight.

The header has, at its left, **Add** with its word, then Copy, Cut,
Paste, Paste As Link and Delete, Move Up and Move Down, Expand All and
Collapse All as icons (each named in its tip); at its right the five
highlights, **Scripts**, **Export**, **Import Lines…** and **Test**; then
**OK** and **Cancel** at its end (OK closes the conversation, its changes
kept; Cancel sets back what it changed since it was opened, as one Undo
step). Where the editor is too narrow for them all, Add, Scripts,
Export, Import Lines… and Test first lose their words (an icon each,
named in its tip), and then the commands go into **More** at the
header's end: the left group's from its end (Collapse All first), then
the right group's, Add last of all; OK and Cancel never do. The header
keeps the window's text size whatever Options › Conversation Editor
gives the lines.

| Command | Does |
| --- | --- |
| Add | a reply to the selected line (an NPC line under Root or a player line, a player line under an NPC line) |
| Copy, Cut | the selected line and everything under it |
| Paste | the copied lines as new lines under the selected one, appearing when the copied line did (its **Text Appears When…** script and parameters) |
| Paste As Link | a link to the copied line under the selected one (the line copied, wherever deletes and Undo since have moved it; off once that line is gone, as after Cut) |
| Move Up and Move Down (Alt+Up, Alt+Down) | move the selected line up or down among its parent's lines: the order the game tries a speaker's lines in and lists a player's replies in |
| Delete | the selected line and everything under it (a link: just the link) |
| Expand All, Collapse All | open or close the whole tree |
| Export | the conversation as plain text, CSV, Twine or Ink (see below) |
| Import Lines… | read back an edited CSV export's text |
| Comments, Actions, Quests, Animations, Sounds | highlight the lines with a comment, an action script, a journal update, an animation or a sound |
| Scripts | show each line's condition, action, journal update and sound |
| Test | play the conversation through (see below; also on the Node section's bar) |

With **Scripts** on (the default), each line names what it does besides
its text: `if c_has_key` (its Text Appears When…), `do a_give_gold` (its
Actions Taken), `journal q_rats 20` and `sound vs_hello`.

**Dragging** a line onto another moves it there: under a line of the other
kind (a player's line under an NPC's), or above a line of its own kind,
which is how lines change places and parents in one go; a frame or a line
above shows which it will be. Ctrl + drag (Cmd on macOS) links it instead.
Lines move only where they fit (an NPC line under a player line or Root,
and the reverse). **Right-click** a line for Add, Copy, Cut, Paste,
Paste As Link and Delete on that line, and Bookmark (its key beside it).
Options › Conversation Editor sets which way Paste As Link and Ctrl +
drag link, and whether a new line first asks for its text in a popup.

**The Right Dock.** Beside a conversation in front, the Right Dock holds
its **Node** section (its tabs **Data** and **Search**, with
**Bookmark** and **Test** at the end of their bar) and under it the
**Conversation** panel, the file's word count in its header; drag the
line between them to share the dock's height as you like. Under 1400
points of window the three are tabs of one bar. The dock stays while the
conversation's group has the focus, and follows whichever conversation
is in front; a conversation in a window of its own has the same panels
in its window's right pane, and leaves the main window's Right Dock as
it was. Each panel scrolls where it has more than its room to show, and
takes the editor's text size (Options › Conversation Editor) as the
lines do. A field you are typing in belongs to its line: choosing
another line keeps what you typed in the first.

**Search** (the Node section's tab) finds the text in **Find What** in
this conversation, or with **All Files in Module** in every
conversation in the module, by **Find**; **Match Case** and **Match
Whole Word Only** narrow it (without Match Case, accents are set aside
as case is, so `arbol` finds `Árbol`), and **Replace All** puts
**Replace With** in the place of each. With the pointer over the editor,
Ctrl+F (or Ctrl+R) brings Search to the front (unfolding the Right Dock
if it was folded) and F3 finds again, as in Aurora, from the
conversation or from Search's Find What field.

## Bookmarks

**Bookmark**, at the end of the Node section's bar or in a line's
right-click menu, puts a bookmark on the line chosen or takes it off.
So does F5 while the tree has the keys or the pointer is over the
editor (Toggle Bookmark in Options › Keyboard's Conversation Editor
group, the script editor's key), and the Command Palette's Toggle
Bookmark.

Beside a conversation in front, the **Outliner** under the Left Dock's
tabs names the conversation and lists its bookmarks in a **Bookmarks**
group, its count beside it: each row the line's kind and number and the
start of its text (`Entry 40: Welcome, <FullName>. I r…`), in the order
you set them. A click on a row goes to its line in the tree, opening out
the lines above it, and gives the list the keys: Up and Down go from row
to row, Page Up, Page Down, Home and End as in the other lists, Right and
Left open and close the group (Left from a row goes to the group), Enter
goes to the line (on the group's row, opens or closes it), letters go to
the bookmark whose line begins with them, and Escape hands the keys back.
While the list has the keys, the conversation's own keys (Delete, Ctrl+A
and the rest) do nothing with the pointer over the tree; a click in the
tree gives them back to it. F6 comes to the list as to the Left Dock's
other panels. With no bookmarks the group says 0 and has no arrow. A
bookmark set on a link's row is the line it links to.

Bookmarks, their rows and folded branches stay with their lines when a
delete numbers the lines after it anew (a bookmark on a line deleted
goes, and Undo brings it back), and when something else changes the
conversation (a plugin, or the file changed outside Moonglow): there
they are found again by their text, speaker and the lines above them,
and one whose line can't be found goes (the debug log says which).
The list follows the conversation in front, and is kept with the
conversation's other state when the module is opened again. Drag the line
over the Outliner to give it the room you like: beside a conversation it
is kept apart from the one beside an area.

A conversation in a window of its own has its Outliner at the window's
left, the main window's Outliner staying with the editor docked in front
there. A new window opens that much wider where the screen has the room.
Where the window is too narrow for it beside the conversation's own room,
the Outliner folds to a strip at the left of the tree, under the header,
so that the header keeps its commands: its icon opens the bookmarks over
the tree, and closes them again. Drag the pane's edge to share the window
as you like.

## The Status Bar

Beside a conversation in front, the Status Bar names the tree's ways:
**Drag** Move line, **Ctrl + Drag** Link (Cmd on macOS), **Double Click**
Go to linked. While the conversation takes the keys (its tree has them,
or the pointer is over the editor with no other list holding them, and
you are not typing in a field), it adds **Ctrl + A** Add and **Alt +
Up** Move up (Option on macOS); while the tree itself has the keys,
**\*** Open branch; then **Ctrl + F** Find, and last, with a line
selected, **F5** Bookmark. Add's, Find's
and Bookmark's keys are as Options › Keyboard binds them (a key you
change shows changed; a command with no key has no hint). At its right
it counts the conversation's words, as the file counts them. With a menu or a
dialog open the hints go, and the count stays. Where the bar has no room
for them all, the hints go from the last, as the bar makes room for
anything else. A conversation in a window of its own leaves the main
window's Status Bar to the editor docked in front.

## Testing a conversation

**Test** (at the header's right, or on the Node section's bar) opens the
Conversation Test window, which plays the conversation as the game does:
the NPC says the first of its lines whose condition passes, and the
player is offered the replies whose conditions pass, numbered.
- **Conditions:** Moonglow can't run the scripts, so each counts as TRUE
  until you click it to make it FALSE. Lines passed over show as not said
  or hidden.
- **What happens:** each line's actions and journal updates are listed. A
  transcript keeps what was said; **< Back** goes back a turn, and
  **Done** closes the window. Drag the window's edges or corners to make
  it larger: the transcript takes the room.

(Aurora's Test shows every line, whatever its condition.)

## A line's Data

The Node section's **Data** tab shows the line chosen in sections on one
scroll, a row each field, its label at its left (over it where the panel
is too narrow, the editor's larger text too):

- **Text**: the speaker's tag, the text with its **…** (every language)
  and **Token…**, and the talk table's string in it (in the talk table's
  color) and under it where the text is that. With Options › Language › Put text typed into the talk table on,
  text typed (here, or in Input Text as the line is added) goes into the
  module's custom talk table as a line of its own, the conversation's
  line naming it; Undo takes both (see [Text typed into the talk
  table](03-modules.md#text-typed-into-the-talk-table)).
- **Text Appears When…**: the script that decides whether the line is
  offered (it returns TRUE or FALSE), its parameters and a preview of its
  source, its lines numbered. **Script Wizard…**, at the section's
  heading, writes one from a few choices. A script's or a sound's field
  always shows a name of 16 characters whole: where the panel has not the
  room beside its label, the label stands over it.
- **Actions Taken**: the script that runs when the line is spoken, its
  parameters and preview; also from its own Script Wizard….
- **Parameter sets**, in both of those sections on the **Parameters**
  row (**+** adds a parameter, **Add Set**, and the **…** menu of Save
  Set…, Copy Set, Paste Set and Save to File…), save
  typing where the same scripts are used with the same parameters time
  and again. **Save Set…** keeps the script and its parameters under a
  name (typed in place; Enter saves it; a set of the name kept already
  is replaced, as the row says); **Save to File…** saves them as
  a set file (`.params.json`) wherever you choose, to share or to keep
  beside a module. **Add Set** adds a set to the line (or link) shown: a
  saved one, chosen by its name (with the pointer resting on it, its
  script and parameters), or a file's (**Add from File…**, one or
  several at once). Its parameters are added to those there: one of the
  same name takes the set's value, and the others stay. A set saved
  with a script puts it in the Script field; one saved without leaves
  the field as it is. One **Undo** takes back the whole set, script and
  parameters. To carry them without keeping a set, **Copy Set** puts the
  script and the parameters on the clipboard (as a set file's text), and
  **Paste Set** on another line or link, in this conversation or another,
  in this module or another, adds them as **Add Set** adds a saved set
  (one Undo again). What is copied can be pasted into a message to
  another builder too, and pasted back from one; text on the clipboard
  that isn't a set is said so in the log, and nothing is changed.
  **Manage Sets…** and **Open Sets Folder**, in Add Set's menu, are as the
  [Variables window's](05-blueprints.md)
  are: in the Parameter Sets window a saved set is renamed or deleted
  (**Undo Delete** puts the last one back). Saved sets are small JSON
  files, `NAME.params.json`, in the `parameter-sets` folder of
  Moonglow's data folder (see [where Moonglow keeps its
  files](13-troubleshooting.md#where-moonglow-keeps-its-files)), to copy
  in, share or write by hand: `{ "script": "ttc_c_skillcheck",
  "parameters": [ { "name": "CHECK", "value": "12" }, { "name": "DC",
  "value": "1" } ] }`. `script` may be left out; values are text, as
  the game's parameters are (a number written without quotes is read as
  its text). A file there that isn't a set is listed as "NAME (not a
  set)", with what is wrong with it as the pointer rests on it.
- **Other Actions**: the animation the speaker plays (**Play
  animation**), the sound it says (**Play sound**, with **Play** to hear
  it), and the journal entry it sets (**Journal**: a quest and an
  entry).
- **Comments**: notes for the builder (a link's own on a link).

The **Conversation** panel, under the Node section, holds the
conversation's own settings (Aurora's Current File): the scripts run when
it ends normally or is aborted, and whether the camera may zoom in.

**Tokens** (`<FirstName>`, `<Class>`, custom tokens…) go into the text
from **Token…**; the game replaces them when the line is spoken. A
language can have tokens of its own (the game's Polish has many English
doesn't): Token… lists those of the language you edit (Options › Language),
and in the **…** window for a text in several languages each language's
row has its own **Token…** (its list resizes with its window). A token
goes in where the caret was last in the text, in place of what was
selected (at the end, if the caret was never there).

## Writing conversations elsewhere

**Export** writes the conversation in other formats:
- **Plain text**: a readable script, for proofreading or review.
- **CSV**: one row per line (`E3` an NPC line, `R5` a reply) with its
  speaker, text, condition and action (each with its script parameters,
  written as in Twine and Ink below: `c_has item=gold`) and comment, and a
  column for each other language the lines have text in: `text_fr`,
  `text_de`, `text_it`, `text_es`, `text_pl` (`text_fr_f` for a feminine
  text, `text_en_f` English's). With another language chosen in Options ›
  Language, its column is there even while no line has text in it, ready to
  be translated into; a column you add by its name is read too. Edit the
  text in a spreadsheet or have it translated; **Import Lines…** then
  reads back the speakers, texts and comments of the lines it names, as
  one undoable step (a language's cell left empty takes that language's
  text off the line). Lines whose text comes from the game's talk table
  show empty.
- **Twine** (Twee 3, for Twine 2 or Tweego) and **Ink** (for Inky): the
  conversation as a branching story.

Several at once: choose the conversations in the module tree (Ctrl or
Shift and a click) and right-click one of them: **Export › Export as
Ink** (or **Twine**, **Text**, **CSV**) writes each into a folder
you choose as a file named after it (`gate.ink`), as this Export writes
it, a conversation open here as it is in the editor (see [the module
tree](02-main-window.md#the-module-tree)). `mg dialog-export MODULE
A.dlg B.dlg --to FOLDER --format ink` (or `--all`) does it from the
[command line](11-command-line.md).

**File › Import Conversation** reads a Twine (`.twee`) or Ink (`.ink`)
story as a new conversation named after the file, so dialogue can be
written in those tools and brought in. The mapping:
- **NPC lines** are passages (Twine) or knots (Ink). Tags give a line's
  speaker, action, journal update and sound: `speaker:TAG`,
  `do:a_script`, `journal:q_rats:20`, `sound:vs_hello`. In Ink a tag goes
  on its own line, as `# do:a_script`.
- **Other Actions and the comment** are tags too, of an NPC line or a
  reply, where a line has them: `anim:talk-forceful` (the animation as
  Other Actions names it, lower case with `-` for its spaces, or its
  number), `delay:500`, and `comment:Asked about the sword`. A
  comment's characters that Ink or Twine read as syntax are escaped as
  a parameter's are (below), and in a Twine passage's tags, which can't
  hold a space, its spaces too (`comment:Asked%20about%20the%20sword`).
  In a Twine reply they go in braces as the action does:
  `[[Bye. {anim bow} {comment Leaves for good}->END]]`. An animation
  that plays once (the line's `AnimLoop` 0, which neither Moonglow's
  Other Actions nor Aurora's shows) has `loop:0` beside it.
- **The conversation's own settings** (Current File): the scripts run
  when it ends and when it is aborted, and, where they are not Aurora's
  own, the delays before NPC lines and replies and the camera kept from
  zooming in.
  - Ink: tags before the first knot, `# end:gate_done` and
    `# abort:gate_done` (nothing after the colon: no script is run),
    `# delay-entry:500`, `# delay-reply:500`, `# zoom:no`.
  - Twine: an object of Moonglow's in the story's data,
    `"moonglow": {"end":"gate_done","abort":"gate_done"}` (and
    `"delay-entry"`, `"delay-reply"`, `"zoom"`). Twine itself may not
    keep it when it saves the story again.
  - A story that says none of them (one written in Twine or Inky) makes
    a conversation with Aurora's (`nw_walk_wp` for both scripts); where
    `mg dialog-import --force` puts it in the place of a conversation,
    that conversation's stay as they are.
- **Player replies** are a passage's links or a knot's choices, leading
  to the next NPC line or to `END`.
  - Twine: `[[Who are you? {if c_curious} {do a_note}->E5]]`.
  - Ink: `+ {c_curious} [Who are you?] # do:a_note -> E5`, the condition
    declared with `VAR c_curious = true` (tags before the divert, as
    Inky wants them; the other order is read too).
  - An empty reply is `(Continue)`.
- **Several NPC lines to choose from** (the first whose condition passes
  is said):
  - Twine: a passage tagged `npc-choice`, its links `[[if c_seen->E2]]`
    and `[[otherwise->E3]]`.
  - Ink: a knot of conditional diverts, `{c_seen: -> E2}` then `-> E3`.
- **A passage or knot reached twice** becomes a link.
- **Script parameters** (read with `GetScriptParam`) follow the script's
  name, `name=value` separated by spaces; a name or value with a space,
  or an empty one, goes in quotes: `do:a_give item="gold coins" count=5`,
  `{if c_has item=gold}`.
  - Ink: a condition's parameters are a tag naming the condition,
    `+ {c_has} [Here it is.] # if:c_has item=gold -> E4`; before a
    conditional divert, on its own line just above it:
    `# if:c_seen times=2` then `{c_seen: -> E2}`.
  - Twine: in a reply's braces and an `npc-choice` link as above
    (`[[if c_seen times=2->E2]]`). A passage's tags can't hold a space,
    so there an NPC line's action takes its parameters as a web address
    does: `do:a_give?item=gold%20coins&count=5`.
  - In a name or value, `%`, `"`, `\`, line breaks and the characters
    Ink or Twine reads as syntax (`# / { } [ ] | < >`) are written as
    `%` and their hexadecimal code (`"` is `%22`, `#` is `%23`), so the
    story stays valid Ink and Twine.

Text Moonglow writes is escaped where the format would read it as syntax:
in Twine as HTML character references, which story formats show as the
characters; in Ink with backslashes, and a blank line as `//`. Only the
English text goes out to Twine and Ink (a passage holds one text: the
CSV carries the other languages).
Every conversation the game and its modules ship
plays the same after a round trip through Twine or Ink. Ink beyond this
subset (stitches, gathers, logic) is refused with its line number.

## The Script Wizard

From Text Appears When… or Actions Taken, **Script Wizard…** writes a
script without typing. Check the conditions to test (items the player carries, skill
checks, local variables, class and level, race, gender, alignment,
abilities, feats, skills, a random chance) or what to do (give or take
items, gold and experience, set local variables, change a faction, open a
store…), fill in a page for each, and name the script. **Finish** writes
the source, compiles it and sets it on the line, as one undoable step.
The script is the module's like any other: open it in the script editor
to change it later.

The wizard shows a **step list** at its left ("Step N of M"): Choose, one
step for each condition or action page, and Name. Steps done carry a
check, the current one is highlighted and those to come are dimmer. A
page you did not check on the first page shows as off, so the list never
changes length (in a window made too narrow for it and the page, the
list is left out). Over each page a **page bar** (an icon and the page's
question) stands where a heading was. The footer holds **< Back**, **Next
>** (the main button), **Finish** and **Cancel**; on the last page
**Finish** is the main button and **Next >** is off, and **Finish** is
off while the name is not a ResRef. **Enter** presses the main button
whenever it is enabled, with a text field holding the keys too;
**Escape** is Cancel. The window is resizable (it opens 720 by 500 and
is never smaller than 560 by 420), and **Start the script editor** is
in its body on the last page.

The conversation stays live behind the wizard. Where lines were deleted
or put back in it meanwhile, Finish sets the script on the line the
wizard was opened for, wherever that is now; where that line is gone,
it writes nothing, the log says so, and the wizard stays open with what
you chose (undo the change and finish again, or cancel).

An amount of gold or experience is a whole number from 1 to
2,147,483,647, the most a script's integer holds; anything else typed
there gives or takes nothing. A float variable's value is a number: `nan`,
`inf` and numbers past what a float holds are not taken.

## Backups

While a conversation with unsaved changes is open, Moonglow backs it up as
`<name>.bak` every 5 minutes, as Aurora does (Options › Conversation
Editor), in a `conversation-backups/<module>` folder in Moonglow's data
folder (see
[Troubleshooting](13-troubleshooting.md#where-moonglow-keeps-its-files);
without one, none are written). Moonglow's recovery copies (see
[Modules](03-modules.md)) also keep the whole module safe.
