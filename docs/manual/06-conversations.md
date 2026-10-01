# Conversations

**Tools › New Conversation…** makes a conversation (`.dlg`); double-click
one in the module tree to open it in the **Conversation Editor**, laid out
as Aurora's.

## The tree

The conversation is a tree under **Root**: the lines the NPC says (red,
headed `[OWNER]` or the speaker's tag) and the player's replies (blue).
A line shown in gray is a **link**: it stands for a line elsewhere in the
tree, so branches can join and loop without copying text. Select a line
to edit it below the tree: its **Speaker** (NPC lines: the conversation's
owner, or a creature in the module by tag) and its **Text** (the **…**
button edits every language).

| Command | Does |
| --- | --- |
| Add | a reply to the selected line (an NPC line under Root or a player line, a player line under an NPC line) |
| Copy, Cut | the selected line and everything under it |
| Paste | the copied lines as new lines under the selected one |
| Paste As Link | a link to the copied line under the selected one |
| Delete | the selected line and everything under it (a link: just the link) |
| Expand All, Collapse All | open or close the whole tree |

**Dragging** a line onto another moves it there; Ctrl + drag (Cmd on
macOS) links it instead. Lines move only where they fit (an NPC line under
a player line or Root, and the reverse). Options › Conversation Editor
sets which way Paste As Link and Ctrl + drag link, and whether a new line
asks for its text in a popup first.

**Search** finds text in this conversation or every conversation in the
module (match case, whole words, replace); **Bookmarks** remember lines to
go back to. **Test** clicks through the conversation from its first
greeting, as a player would (without running its conditions).

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
**Token…**; the game replaces them when the line is spoken.

## The Script Wizard

From Text Appears When… or Actions Taken, **Script Wizard…** writes a
script without typing one: tick what to test (items the player carries, skill
checks, local variables, class and level, race, gender, alignment,
abilities, feats, skills, a random chance) or what to do (give or take
items, gold and experience, set local variables, change a faction, open a
store…), fill a page for each, and name the script. **Finish**
writes the source, compiles it and sets it on the line, as one undoable
step. The script is the module's like any other: open it in the script
editor to change it later.

## Backups

While a conversation with unsaved changes is open, Moonglow backs it up
as `<name>.bak` every 5 minutes, as Aurora does (Options › Conversation
Editor), in a `moonglow-backups/<module>` folder in the system's temporary
folder. Moonglow's recovery copies (see [Modules](03-modules.md)) keep the
whole module safe as well.
