---
type: Manual Page
title: Scripts
description: Scripts - the script editor, finding your way in code, and the compiler.
tags: [manual, scripts, nwscript]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-26T12:35:56Z }
---

# Scripts

Scripts are NWScript source files (`.nss`) that Moonglow compiles into
the bytecode the game runs (`.ncs`). **Tools › New Script** makes one;
double-click a script in the module tree to open it in the **script
editor**.

## The script editor

The editor highlights NWScript's syntax. Its lines stand 18 points
apart at the 13-point text (in proportion at another Options › Script
Editor size) and are numbered at the left, the numbers right-aligned, a
bookmark's mark (or a numbered bookmark's digit) before the number; the
cursor's line is marked across the editor, its number brighter (not
while text is selected). Over the text is the
script's **header**, one row: **Save** and **Compile** with their words,
then **Save As…**, **Print…**, **Open…** and **External Editor**,
**Find…** and **Replace…**, **Bookmark**, and **Definition**,
**References**, **Rename Symbol…** and **Used By** as icons (each named
in its tooltip with its key); at its right end the codepage the script
is read and saved in (below), **More** (⋯), which holds **To
Scratch**, and **OK** and **Cancel**: OK puts the text into the module
and closes the tab, keeping its bookmarks, Undo history, codepage and
External Editor link for the session as a clean close does. Cancel closes it as last saved; with text not yet
saved into the module it asks first, as closing the tab does (its ×,
Ctrl+W, the tab's menu, its window's Close): **Save** puts the text
into the module, as Save does, and closes the tab, **Don't Save** closes
it as last saved, and **Cancel** keeps it open. A script closed with
nothing unsaved (by its ×, its menu or its header's Cancel), or closed by that question's Save, and opened again in
the same session has its bookmarks, Undo history, codepage and External
Editor link as it left them; one closed by Don't Save opens anew. A script deleted from the module while closed loses what was kept. Where the
editor is narrow, Save and Compile lose their words first, then the icons go into More from the end of the row, then
the codepage, Save and Compile last of all, so every command stays one
click away; OK and Cancel stay at the end whatever the width. An unsaved
script shows the dot on its tab (the header has no "modified" of its
own), and Save is grayed while there is nothing to save. Each button is
described under [The header's buttons](#the-headers-buttons) below.
While you work in a script, the Status Bar's hints show its keys (Find,
Find next, Definition, References, Complete, Bookmark, Compile all,
Replace, as Options › Keyboard binds them).

Beside the text, a script brings Aurora's lists to the **Right Dock**:
**Reference**, with its header. Where the script floats in a window of
its own, it stands at the window's right instead, and narrows to half
the window in a small one. The Right Dock is there whenever a script's
tab is the editor in front, in its group too: click an area's tab and
the area's panels are back.

**Reference** lists, chosen above its filter (where the panel is too
narrow for the four words, from a list):

- **Functions**, **Variables** and **Constants**: everything
  `nwscript.nss` declares, and the script's own and its includes' (in
  the theme's text, the game's quieter). A click shows one in Help,
  under the text, bringing Help to the front (and opening the Bottom
  Dock on it where it was closed); double-click to insert it at the
  cursor.
- **Templates**: code templates, the game's and those in your Code
  Templates Directory (Options › Script Editor). Double-click to insert.

F6 to Reference puts the keyboard in its filter; F6 to Help takes it
from the text (what you type then goes nowhere) until you click back in
it.

**Help** stands under the text, as wide as it, as in the Aurora
Toolset: a tab of the Bottom Dock, or of the bottom pane of a script's
own window. It shows the function or constant chosen in Reference or
under the cursor as the source has it: its documentation comment as
written (its `//` or `/* … */`, its spaces and the columns of its
lists, so `nwscript.nss`'s tables line up as in the file), then its
declaration, in the editor's font and colors on the editor's ground. Its
lines are not wrapped: a long one scrolls sideways. The documentation is
the comments directly above the declaration (no empty line between),
`//` lines and `/* … */` blocks alike, and the comment that trails
its `;` on the same line (`int nLast; // the last one` is about
`nLast`, not about the next declaration; with both, the one above
comes first). A comment with an empty line under it, a heading for a
group of constants, belongs to none of them. BioWare's includes
document a function with a `/* … */` block after a `//::` line above
its definition; where the prototype has no comment of its own, Help
shows the definition's. A struct's members and local variables have
theirs too, the comments above and beside their lines. (`mg lsp`'s
hover has the comment's text without the markers.) A double click on a
function's or a constant's name in a script, or Definition, brings Help
forward, opening the Bottom Dock where it was closed; moving the cursor
only changes what it shows.

A script brings three panels of its own to the **Bottom Dock** under
the editor area, **Compiler**, **Help** and **Bookmarks**, before
**Problems**, **Search Results** and the **Log**, which are there
whatever is in front; each has its switch in the Status Bar (Log,
Problems, Compiler, Help, Bookmarks, Search Results), which opens the
dock on it or closes it. They follow the script in front: with two
scripts side by side, the one whose group you work in.
A script in a window of its own has them on its window's **bottom
pane** instead (Compiler, Help, Bookmarks, Search Results), under its
text beside Reference; that pane takes at most a third of the window's
height, however far its edge is dragged, so that the text keeps the
rest in a small window. The width you drag that window's Reference
pane to and the height you drag its bottom pane to are kept: the next
script window opens with them, after a restart too (every kind of
window keeps its own: [The main window](02-main-window.md#floating-windows)).

- **Compiler**: first, where checking as you type (below) found an
  error, a **Found as you type** row with the compiler's words whole,
  wrapped over as many lines as they take; click it to go to the line
  (the include's own line where the error is in an include). Then what
  the script's latest compile said; click a message
  that names a line to go there (in an include too). Each starts with
  the time it was written (`14:03:27`), so two compiles in a row are
  easy to tell apart. A row's right click (or Shift+F10 with the
  keyboard on it, reached by Tab) has **Copy** (the message without its
  time) and **Copy All** (every row, the error found as you type first);
  Ctrl+C copies the row the keyboard is on. When **Save** saves without compiling
  (Automatically Compile Scripts on Save is off), Compiler says so
  instead of keeping the last compile's message; so does File › Save
  (Ctrl+S with the keyboard elsewhere) for each script whose unsaved
  text it stores into the module. A compile brings Compiler to the
  front where the dock is open on another panel than the Log, and a
  compile that failed opens the dock on Compiler whatever it showed.
- **Bookmarks**: the numbered bookmarks (`[1]`) and the bookmarked
  lines, each with its line's number and text; click one to go to its
  line, or right-click it (or Shift+F10 with the keyboard on it) to
  **Copy** it or **Copy All**; Ctrl+C copies the row the keyboard is on. Setting or clearing a bookmark brings it to the front where the
  dock is open on another panel than the Log.
- **Search Results**: what the last search found (Find In Files, Find
  In Currently Open Scripts, References), what was searched over the
  lines, its switch's count the lines found; click a line (or Enter on
  the one the arrow keys are on) to go there, in its own script, or
  right-click it to **Copy** it whole or **Copy All**. A
  search that finds something opens the dock on it; one that finds
  nothing opens nothing (a script in a window of its own shows the
  hits on its window's bottom pane). The panel holds one search at a
  time: **Edit › Find Instance** and **Edit › Find and Replace
  Text** show theirs there too (see [The main
  window](02-main-window.md#search-results)), and a script's next search takes
  their place.

F6 goes round to the panel in front of the Bottom Dock too, which then
has the keyboard: what you type goes nowhere until you click back in
the text.

The editor works with plain line ends. A script saved with Windows line
ends (CRLF, as BioWare's scripts and Moonglow's new ones are) has no
hidden character at the end of each line for the cursor to stop on, End
goes to the line's last character, and copied text has no `\r`. Text
you paste is turned into plain line ends too. When the script is saved,
it gets its own line ends back: a script you didn't change is written
byte for byte as it was, and in a changed one the new lines use the
file's line ends (CRLF for a CRLF script). In a file that mixes both,
every line you didn't change keeps its own, wherever it is (Moonglow
compares the lines as a diff does); a line changed in place keeps the
end it had, and a line added takes the kind most of the file's lines
have.

| Keys | Do |
| --- | --- |
| Ctrl+S | **Save** the script into the module (not the module to its file), while the editor has the keyboard; anywhere else Ctrl+S saves the module |
| Ctrl+F | Find |
| F3 | find the next (from the script, or from the Find Text window's field, as in Aurora) |
| Ctrl+R | Replace |
| F2, Ctrl+Space | complete the word at the cursor (functions, constants, variables) |
| Enter | a new line, begun as far in as the line it is started from |
| Tab | one step of indentation at the cursor, as the script is indented: a tab, or its step of spaces (to the next multiple of it); a script with no indented line to go by, as Options › Script Editor › Indent with says (four spaces); on several selected lines, indent each one a step (empty lines stay empty) |
| Shift+Tab | outdent the selected lines, or the cursor's line, a step: a tab, or up to the indent width in spaces |
| Page Down, Page Up | a page down or up, the text with the cursor; with Shift, selecting |
| Ctrl+Z, Ctrl+Y | take back and give again what was done in the editor (typing, a new line, an indent, a completion, a replacement), while it has the keyboard; Edit › Undo and Redo and the Top Bar's are the editor's then too (see below) |
| F12, Ctrl+click | go to the definition of the name at the cursor |
| Shift+F12 | list where the name at the cursor is used |
| Ctrl+Shift+R | rename the name at the cursor everywhere |
| F5 | set or clear a bookmark on the line |
| Ctrl+Shift+1 … 9 | set numbered bookmark 1 to 9 |
| Ctrl+1 … 9 | go to numbered bookmark 1 to 9 |
| F7 | compile all the module's scripts |

**The completion list** (F2) is as wide as the longest name it
lists, so no name is broken over two lines, and never narrower than a
short list needs; it stops at the window's edge, where a longer name
ends in "…" and its tip gives it whole with its signature. Its scroll
bar shows only when the names are more than it holds. Up and Down
choose a name, Enter or Tab puts it in, Escape closes the list.
Beside the list, the name chosen in it shows its declaration and the
first line of its documentation, at the list's right, at its left where
the window's edge is near, or under (or over) it where neither side has
room, always inside the window.

**Tips over names.** Rest the pointer on the name of a function or
constant the script knows (its own, its includes' or the game's) for a
tip with its declaration and the first line of its documentation; move
the cursor onto one with the arrow keys (or Home, End, Page Up, Page
Down) and the same tip stands under it until the next key, click or
scroll of the text; it hides while the cursor is out of sight. Over
a line with an error, the error's own tip shows instead, and neither
tip shows while the editor's right-click menu is open. **Options
› Script Editor › Show declarations in tips** turns both off.

**The editor's right-click menu** has **Undo** and **Redo** (the
script's own, grayed with nothing to take back or give again), **Cut**,
**Copy**, **Paste** and **Select All**, then **Convert Indentation to
Tabs** and **to Spaces**, and any command a plugin adds for scripts.
Cut and Copy are grayed with no text selected. A script of the game's
or a hak's, open only to be read, has Cut, Copy, Paste and Select All,
Cut and Paste grayed, then **Find…** (see [The main window](02-main-window.md)). A right click
inside the selection keeps it, so the menu works on it; outside the
selection the click puts the cursor there. Shift+F10 opens the menu at
the cursor, and Up, Down and Enter choose from it.

**Undo in a script.** The editor keeps its own history of the text:
what you type stays in the editor until **Save** (or File › Save)
puts it into the module, and only that is a step of the module's
history. While
the editor has the keyboard (its cursor shows), Ctrl+Z and Ctrl+Y,
**Edit › Undo** and **Redo** and the Top Bar's Undo and Redo all take a step
of the script's history, never the module's: the menu says plain
"Undo" and "Redo" rather than naming a module step, and the click on
the menu or the Top Bar that takes the keyboard from the editor still
counts as being in it. After them the editor has the keyboard again,
so Ctrl+Z goes on in the script; a menu opened from the editor and
closed with nothing chosen (Escape, Alt, F10, or a click on its name)
gives it the keyboard back too, where a click elsewhere leaves it
nowhere. Keys given to Undo and Redo in Tools › Options › Keyboard are
the script's in the editor as Ctrl+Z and Ctrl+Y are (which stay the
editor's whatever Undo's key is, as in any text field). With nothing to
take back or give again in the script they do nothing (they are not
grayed). With the
keyboard anywhere else (after a click in the module tree, say), even
with the pointer over the script, Edit › Undo and the Top Bar's are
the module's and name its step, as the Save key saves the module
there (Ctrl+Z in a field typed in is that field's; after work in a
hak's or a 2DA's editor they are that editor's: see Undo and redo in
[The main window](02-main-window.md#tabs-and-windows)). **Replace** and
**Replace All** are each a step of the script's history, and while the
Replace (or Find) window is where you are at work (the keyboard in one
of its fields, or its button just clicked) Edit › Undo and Redo are
its script's, so that Undo right after Replace All takes the
replacements back. From a menu, a step that would only move the cursor
is passed over to the one that changes the text.

**Open…** on a script editor's header opens the **Open Script** window:
the scripts found by the name you type, of the module's alone, its
haks' alone or all of them with the game's (Aurora's Resources to
Show). A click selects a script; Ctrl+click adds one or takes it away,
Shift+click selects the ones between it and the one clicked before, and
Down and Up (Page Down, Page Up) move the selection, with Shift
widening it, while the name is typed too. Home and End go to the first
and the last script listed (Shift widens to it), and Ctrl+A selects
every one listed; while a name is typed in its field, those three are
the field's own (to the start or end of the name, all of it). **Open**
(or Enter) opens every script selected, each in its tab; with none
selected, Enter opens the first listed (typing in the name clears the
selection). A double click opens that script alone. A right click on a
script selects it (unless it is selected already) and has **Open**
(**Open N Selected**), **Select All Listed** and **Copy Name**. Only
scripts listed stay selected: choosing other resources to show takes
away those it hides. The game's and a hak's open to be read.

**Find…** opens the **Find Text** window: **Find What**, then **Match
Case**, **Match Whole Word Only** and **Search Backward**. Enter (**Find
Next**) finds the next match, again and again; without Match Case,
accents are set aside as case is (`arbol` finds `Árbol` in a comment
or a string). Checked, **Find In Currently Open Scripts** lists every
match in the scripts open now (those with a tab; one closed earlier in the session is not searched) under Search Results, and **Find In Files
(all scripts in the module)** every match in the module's scripts; the
main button is then **Find All**. Search Results keeps the first 10,000
lines found (the log says how many there were). **Replace…** opens the
same window as **Replace Text**, with **Replace With**, **Replace** and
**Replace All**, which work in the script it was opened from, never in a script whose tab is closed.

**Used By** (on the header) opens the **References** tab on the script:
where the module runs or includes it, objects' and areas' events,
conversation lines, the module's events, `#include` lines, and strings
in scripts that spell its name (`ExecuteScript("name", …)`), each with
its line in the scripts' codepage (below).

A script is read and saved in the codepage of the language chosen in
**Options › Language**: Windows-1250 with Polish, Windows-1252 with
English, French, German, Italian or Spanish (a module's own
`encoding.2da`, where its haks have one). The header names it, at its
right end.
NWScript has no codepage of its own: the compiler copies a string's
bytes into the compiled script, and the game shows them by its
language's codepage. So a Polish module's scripts, kept in Windows-1250
as Aurora writes them, show their Polish letters, compile them as the
game reads them, and keep every line you don't change byte for byte. A
character the codepage can't hold (one from another alphabet) is saved
as `?`, and the log says which. Choosing another language reads again
the scripts open without unsaved changes; one with unsaved changes is
saved in the codepage it was read in. An external script editor
(Options › Script Editor) is given the script in that codepage: set it
to open `.nss` files so (in VS Code, `"files.encoding": "windows1250"`
for them).

**Indentation.** Tab indents as the script open is indented: Moonglow
looks at its indented lines when it opens it (or reads it again) and
takes tabs or spaces as most of them are, and for spaces the step by
which a line is most often further in than the one before it (two,
four…; the ` * ` down a `/* … */` block's side doesn't count). A script
with no indented line goes by Options › Script Editor › Indent with.
**Edit › Convert Indentation to Tabs** and **to Spaces** (also on the
editor's right-click menu, above) make every line of the script in front so:
as far in as it was, a tab for each step (what is left of a step, a
column lined up, stays spaces), or spaces for each tab (to the next
multiple of Options' Indent width). With text selected in the editor
(a right click inside it keeps it; from the Edit menu, with the
editor's keyboard as the menu opens), only the lines it touches are
converted. It is one step for the editor's Undo (Ctrl+Z with the
editor's keyboard), and Tab indents so from then on (after a
selection's, as the script is then indented); taken back with Undo,
Tab indents as before it again. The guess is made when the script is
read, not as you type, and not kept from one session to the next.
Enter still begins a new line with the spaces and tabs the line it is
started from has.

### The header's buttons

Open…, Find…, Replace… and Used By are described above; **Bookmark**
sets or clears a bookmark on the cursor's line, as F5 does; Definition,
References and Rename Symbol… are under [Finding your way in
code](#finding-your-way-in-code). The rest:

- **Save** (Ctrl+S while the script's text has the keyboard, or the key
  given to Save in Options › Keyboard): put the text into the module,
  and compile it with Automatically Compile Scripts on Save. It writes
  no file: the module is written by **File › Save** (Ctrl+S with the
  keyboard anywhere else). For a module kept as a `.mod` file, the
  script is in that file once the module is saved; for a module kept as
  a folder or a nasher project, the script's own `.nss` file there is
  written by File › Save too, not by the script's Save. Until then the
  module has unsaved changes (the `*` in the title) and Edit › Undo,
  with the keyboard outside the editor, takes the script's Save back.
  Aurora's script editor has its own Save on Ctrl+S as well. A script
  tab with text not yet saved into the module shows the dot, and closing
  it asks Save, Don't Save or Cancel (above); Moonglow's recovery copies
  keep that text too.
- **Compile**: save and compile this script; errors go to **Compiler**
  and the log. The scripts it includes are compiled as the
  module has them: an include open in another editor with changes not
  yet saved is compiled as saved, and Compiler says so
  (save the include first to compile with its changes). With
  **Automatically Compile Scripts on Save** (Options › Script Editor),
  Save compiles too, and so does saving the
  module (File › Save) for the scripts whose text it saves. The scripts
  that include a script compiled or saved so are compiled again with it
  (more than 24 at once in the background): an include file changed
  leaves none of them with an older compiled script. **Compile All
  Scripts** passes over the scripts that, with what they include, are
  as they were when it last compiled them, and says how many it left
  alone. It remembers from one session to the next (in Moonglow's own
  data folder, not in the module), and compiles them all again when the
  haks, the content folders, the compiler or its settings change, or
  where a compiled script is no longer the one it made. A script
  compiled on its own (its Compile, a save that compiles it, the
  scripts compiled with one that they include) counts as compiled too:
  the next Compile All passes over it. `mg compile`, `mg build` and
  `mg test` remember in the same place, so each passes over what the
  others compiled (with the built-in compiler; a nasher project is one
  module to them whether named by its folder or its `nasher.cfg`).
- **Save As…**: save the script under another name.
- **Print…**: open the script, highlighted, in your browser to print it
  from there.
- **External Editor**: open the script in the editor set in Options ›
  Script Editor (grayed until one is set there); what you save there
  comes back into Moonglow's editor.
  In a nasher project it is the project's own file that opens, in place
  (so your editor's own project features see it where it lives), and
  its saves are read again as files changed outside are. A script with
  changes Moonglow has not yet written to the project opens as a copy
  instead: save first.
- **To Scratch** (in More): save and compile the script, then copy it into the
  scratch folder: a folder you choose the first time (Tools › Options ›
  Folders changes it), such as the game's or a server's `development`
  folder, for a quick fix. Its compiled script goes with it when it
  compiled just now, and its debug file when that compile made one
  (Options › Script Editor › Generate Debug Information When Compiling
  Scripts). A script that doesn't compile, an
  include file (no `main` or `StartingConditional`) or one with an
  error, is copied alone: Compiler says it is an include file,
  or gives the compiler's error, then what was copied ("not compiled").
  A compiled script or debug file of that name that the copy did not
  write is removed from the folder, whatever its spelling (`HELLO.NCS`
  as well as `hello.ncs`): one an earlier copy left, which the game
  would still run, or the debug file of an older compile; Compiler
  and the log name what was removed. A compiled
  script the module has from before is not copied with a script that no
  longer compiles. The module tree's Export › Copy to Scratch Folder follows the
  same rule (see [Modules](03-modules.md), also for exporting files
  elsewhere).

## Finding your way in code

Moonglow knows what each name in a script stands for:
- a local variable or parameter in scope;
- a struct's member after `.`, by the type of what is before it: a
  variable or parameter (`p.nX`), another member (`a.b.nX`) or a
  function's call (`GetMark().vPos`);
- a function, variable, constant or struct of the script itself;
- one from its includes (in the order the compiler reads them);
- one from `nwscript.nss`, the engine's own.

With that:
- **Definition** (F12, or Ctrl+click a name): opens where the name is
  declared, at the line; for a function, its body if it has one. A game
  script or `nwscript.nss` opens read-only instead, scrolled to the
  declaration's line with that line marked, and Help shows the
  declaration with its comment once the script is in front again. As
  Floating Windows it opens as a tab of the window (or group) of the
  script you asked from; as Tabs, as a tab of the main window. Where it is open
  already, that tab comes to the front and goes to the line: a second
  Definition into the same file reuses it. **Find** (Ctrl+F) works in
  it as in any read-only text (see [The main window](02-main-window.md),
  the resource browser).
- **References** (Shift+F12): every use of the name in the module's
  scripts, listed in **Search Results**; click one to go there. A local's
  uses are only in its own block; a function's, only in scripts that
  include the file it's in.
- **Rename Symbol…** (Ctrl+Shift+R): renames a function, variable,
  constant, struct, struct member, local or parameter everywhere it's
  used, as one undoable step. A name already taken, or one of the engine's, is
  refused. Compiled scripts stay valid, since names aren't in them.

**Errors as you type**: half a second after you stop typing, Moonglow
compiles the script (an include file needs no `main`), with its
includes as Compile reads them: as saved. The first error's line gets
a red wavy underline, under what is on it; rest the pointer on it for
the error. An error at the end of the file (where its line is blank)
underlines the nearest line with text above it; an error in an include
underlines the `#include` line that names it, and its tip says which
include and which line. While the Status Bar follows the script (it is
the tab in front of the main window, or it is in a window of its own
and the pointer is over that window or you last clicked in it), the
Status Bar says the error too, in red, where the log's last line would
be, whether the log is open or not: whole wherever the bar has room
for it (the others give way only as far as it needs: the game's folder,
then the hints from the last, then the switches' words, then the
module's name is cut short, and "Line, column" goes last), with all of
it in its tip where even so it does not fit; while the log is
closed, a warning or an error logged after it shows instead. Compiler
lists it too, whole (above). Click it to go to the underlined line.
A script in a window of its own also says it at the right end of its
window's bottom pane's tab bar, in red (its icon alone where the pane
is narrow); click it the same way. Nothing above the text comes and
goes with an error, so the text doesn't move. **Options › Script
Editor › Check for errors as you type** turns this off.

**Line and column.** While the Status Bar follows the script, its right
end says where the cursor is: "Line 17, column 12" (columns in
characters, a tab one of them).

Other editors get the same
features through `mg lsp` (see [Command-line tools](11-command-line.md)).

## The compiler

Moonglow has Beamdog's own NWScript compiler built in, the one the game
and Aurora use, so a script compiles in Moonglow exactly as it does there.

One thing newer builds of that compiler refuse, Moonglow compiles as
Aurora does: a function declared with one return type and defined with
another, such as the base game's `x2_inc_cutscene`, whose
`CutCreateObjectCopy` is declared `object` and defined `void`. Every
script that includes it compiles, whether it calls the function or not,
and the compile says a warning instead (in Compiler, at the
line, and the log; `mg compile` says each once, with how many scripts it
was met in): the function, the line it is declared at and the line it is
defined at, and what the game does with it. A definition that returns
nothing gives the declared type's empty value (`CutCreateObjectCopy`
always returns `OBJECT_INVALID`); one that returns a value where the
declaration says `void` overwrites a variable of the script that calls
it; one that returns another type stops the script where the value is
used. Make the two agree to be rid of the warning.

One way of writing compiles otherwise than in Aurora, on purpose: a
script that includes a file with a `void main()` of its own. Aurora's
Build compiles the include's `main` in place of the script. A
condition (`int StartingConditional()`) becomes the include's `main`,
which in the game always fails as a condition, and a script that is
only the `#include` becomes it too (and a script with no entry point
of its own, whose include has a `StartingConditional`, becomes that).
Moonglow compiles what was written: the script's own entry point, and
nothing for a script without one, which is an include. Each such script
is warned of, in Compile All's log and by `mg compile`: the include's
`main`, its line, and what Aurora makes of it. A module built in Aurora
and then in Moonglow counts these scripts as changed.

To compile with a program of your own instead (a newer build of the
compiler, `nwnsc`), choose it in **Tools › Options › Script Editor ›
External Script Compiler** (see [Options](10-options.md)); what it says
of a script that doesn't compile shows in Compiler.
`nwn_script_comp` 2.3.1 (neverwinter.nim) is such a build: as the
external compiler it fails every script that includes `x2_inc_cutscene`
("FUNCTION IMPLEMENTATION AND DEFINITION DIFFER").
`#include` files come from the module, its haks and the game, in the
game's order. It is given the scripts to compile and what they include,
not the whole module, and nothing to read from the keyboard; a run that
takes more than two minutes is stopped, and says so.

**Build › Compile All Scripts** (F7, or **Compile** at the Top Bar's
right end) compiles every script in the module;
the log lists the failures and says how many were compiled, how many
failed, and how many changed (compiled scripts whose bytes differ from
what was there; debug files, with Generate Debug Information, are
counted apart, when there are any). **Details**, beside that line,
opens a line for each script compiled, those changed first, then the
rest by name: the script, why it was compiled, and what came of it
(compiled again with the bytes differing or the same, a new compiled
script, or the compiler's error). The reasons are: Moonglow had not
compiled it before (nothing is remembered of it here, as after the
first Compile All of a module, or when the haks or settings changed);
it or something it includes changed; its compiled script was not
Moonglow's (another tool made it, or it was changed); it had no
compiled script; it had no debug file (with Generate Debug Information);
it was asked for (compiling the scripts named). `mg compile --json`
says the same for each script (`compiled_scripts`: `script`, `why`,
`ok`, `changed`, `debug_changed`). An include file (a
script with no `main` and no `StartingConditional`; one in a comment
doesn't count) isn't compiled on its own: it is compiled into the
scripts that include it, and may lean on what they bring. (Aurora's
build compiles include files too, and reports an error for one that
doesn't compile alone.) An `#include` commented out (after `//`, or in a
`/* … */` block) includes nothing, as for the compiler: the scripts
compiled with an include they include, and Verify's missing includes,
leave it out.
Compile, in an include file's own editor, says that it is one.
**Generate Debug Information When Compiling Scripts** (Options › Script
Editor) also stores a `.ndb` with each compiled script, for script
debuggers.

Scripts are referred to by name (at most 16 characters) from events,
conversations and other scripts; the **Edit** buttons beside script
fields open them. The button is there as soon as a name is typed. For a
name that is no script yet, **Edit** makes the script in the module
(the log says so) and opens it, as in Aurora: type the name of a new
script into an event and click Edit. A conversation's "Text Appears
When…" script starts as a condition (`StartingConditional`).

## NUI Creator

The NUI Creator designs Enhanced Edition UI windows and writes their scripts
(**Tools › New NUI Window**): see [NUI Creator](18-nui-creator.md).
