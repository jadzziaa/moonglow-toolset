---
type: Manual Page
title: Scripts
description: Scripts - the script editor, finding your way in code, and the compiler.
tags: [manual, scripts, nwscript]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-27T19:18:43Z }
---

# Scripts

Scripts are NWScript source files (`.nss`) that Moonglow compiles into
the bytecode the game runs (`.ncs`). **Tools › New Script…** makes one;
double-click a script in the module tree to open it in the **script
editor**.

## The script editor

The editor highlights NWScript's syntax. Beside the text are Aurora's
lists:

- **Functions**, **Constants** and **Variables**: everything `nwscript.nss`
  declares (and the script's own variables), with **Help** on the
  selected one. Double-click to insert it at the cursor.
- **Templates**: code templates, the game's and those in your Code
  Templates Directory (Options › Script Editor). Double-click to insert.

**Help** shows the documentation as the source has it: the comments
directly above the declaration (no empty line between), `//` lines and
`/* … */` blocks alike, with their markers and their spacing, in the
editor's font and colors, then the declaration itself, so the aligned
lists in `nwscript.nss`'s comments line up as they do in the file.
BioWare's includes document a function with a `/* … */` block between
`//::` lines above its definition; where the prototype has no comment of
its own, Help shows the definition's. (`mg lsp`'s hover has the same
text without the markers: no `//`, `/*`, `*/` or the ` * ` down a
block's side.)

Below the text are the **Compiler** messages: click one to go to its line.
Each starts with the time it was written (`14:03:27`), so two compiles
in a row are easy to tell apart. When **Save** saves without compiling
(Automatically Compile Scripts on Save is off), the Compiler tab says
so instead of keeping the last compile's message; so does File › Save
(Ctrl+S with the keyboard elsewhere) for each script whose unsaved text
it stores into the module.

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
| Ctrl+Z, Ctrl+Y | take back and give again what was done in the editor (typing, a new line, an indent, a completion, a replacement), while it has the keyboard; Edit › Undo and Redo and the toolbar's are the editor's then too (see below) |
| F12, Ctrl+click | go to the definition of the name at the cursor |
| Shift+F12 | list where the name at the cursor is used |
| Ctrl+Shift+R | rename the name at the cursor everywhere |
| F5 | set or clear a bookmark on the line |
| Ctrl+Shift+1 … 9 | set numbered bookmark 1 to 9 |
| Ctrl+1 … 9 | go to numbered bookmark 1 to 9 |
| F7 | compile all the module's scripts |

**Undo in a script.** The editor keeps its own history of the text:
what you type stays in the editor until **Save** (or File › Save)
puts it into the module, and only that is a step of the module's
history. While
the editor has the keyboard (its cursor shows), Ctrl+Z and Ctrl+Y,
**Edit › Undo** and **Redo** and the toolbar's ⟲ and ⟳ all take a step
of the script's history, never the module's: the menu says plain
"Undo" and "Redo" rather than naming a module step, and the click on
the menu or the toolbar that takes the keyboard from the editor still
counts as being in it. After them the editor has the keyboard again,
so Ctrl+Z goes on in the script. With nothing to take back or give
again in the script they do nothing (they are not grayed). With the
keyboard anywhere else (after a click in the module tree, say), even
with the pointer over the script, Edit › Undo and the toolbar's are
the module's and name its step, as the Save key saves the module
there (Ctrl+Z in a field typed in is that field's). Making each
burst of typing a step of the module's history would have put it
under the module's Undo from any other tab too, where it would have
taken back typing in a script you were not looking at.

**Open…** on a script editor's toolbar opens other scripts: a list of
them found by name, of the module's alone, its haks' alone or all of
them with the game's (Aurora's Resources to Show). A click selects a
script; Ctrl+click adds one or takes it away, Shift+click selects the
ones between it and the one clicked before, and Down and Up (Page
Down, Page Up) move the selection, with Shift widening it, while the
name is typed too. **Open** (or Enter) opens every script selected,
each in its tab; with none selected, Enter opens the first listed
(typing in the name clears the selection). A double click opens that
script alone. The game's and a hak's open to
be read. In **Find Text**, Enter finds the next match, again and again;
without Match case, accents are set aside as case is (`arbol` finds
`Árbol` in a comment or a string).
**Find In Currently Open Scripts** lists every match in the scripts open
now under Search Results, and **Find In Files** searches (and replaces
in) every script in the module. A double click on a function's or a
constant's name in a script brings its Help forward (over the
compiler's messages, say).
**Used By** (on the toolbar) lists where the module runs or includes the
script: objects' and areas' events, conversation lines, the module's
events, `#include` lines, and strings in scripts that spell its name
(`ExecuteScript("name", …)`).

A script is read and saved in the codepage of the language chosen in
**Options › Language**: Windows-1250 with Polish, Windows-1252 with
English, French, German, Italian or Spanish (a module's own
`encoding.2da`, where its haks have one). The toolbar names it.
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

## Finding your way in code

Moonglow knows what each name in a script stands for:
- a local variable or parameter in scope;
- a function, variable, constant or struct of the script itself;
- one from its includes (in the order the compiler reads them);
- one from `nwscript.nss`, the engine's own.

With that:
- **Definition** (F12, or Ctrl+click a name): opens where the name is
  declared, at the line; for a function, its body if it has one. A game
  script or `nwscript.nss` opens read-only, with the declaration's
  comment in Help.
- **References** (Shift+F12): every use of the name in the module's
  scripts, listed in **Search Results**; click one to go there. A local's
  uses are only in its own block; a function's, only in scripts that
  include the file it's in.
- **Rename Symbol…** (Ctrl+Shift+R): renames a function, variable,
  constant, struct, local or parameter everywhere it's used, as one
  undoable step. A name already taken, or one of the engine's, is
  refused. Compiled scripts stay valid, since names aren't in them.

**Errors as you type**: half a second after you stop typing, Moonglow
compiles the script (an include file needs no `main`). The first error
shows under the toolbar and its line is underlined. That line under the
toolbar is kept whether or not there is an error, so the text doesn't
move as errors come and go; a long message is cut to fit (hover over it
for all of it). **Options › Script Editor › Check for errors as you
type** turns this off.

Struct members (`p.nX`) aren't followed. Other editors get the same
features through `mg lsp` (see [Command-line tools](11-command-line.md)).

The buttons:

- **Save** (Ctrl+S while the script's text has the keyboard, or the key
  given to Save in Options › Keyboard): put the text into the module,
  and compile it with Automatically Compile Scripts on Save. It writes
  no file: the module is written by **File › Save** (Ctrl+S with the
  keyboard anywhere else). For a module kept as a `.mod` file, the
  script is in that file once the module is saved; for a module kept as
  a folder or a nasher project, the script's own `.nss` file there is
  written by File › Save too, not by the script's Save. Until then the
  module has unsaved changes (the `*` in the title) and Edit › Undo,
  with the keyboard outside the editor, takes the script's Save back. Aurora's script editor has its own Save
  on Ctrl+S as well. A script tab with text not yet saved into the
  module says so; Moonglow's recovery copies keep that text too.
- **Compile**: save and compile this script; errors go to the Compiler
  messages and the log. With **Automatically Compile Scripts on Save**
  (Options › Script Editor), Save compiles too, and so does saving the
  module (File › Save) for the scripts whose text it saves. The scripts
  that include a script compiled or saved so are compiled again with it
  (more than 24 at once in the background): an include file changed
  leaves none of them with an older compiled script. **Compile All
  Scripts** passes over the scripts that, with what they include, are
  as they were when it last compiled them, and says how many it left
  alone. It remembers from one session to the next (in Moonglow's own
  data folder, not in the module), and compiles them all again when the
  haks, the content folders, the compiler or its settings change, or
  where a compiled script is no longer the one it made. `mg compile`
  remembers in the same place, so each passes over what the other
  compiled (with the built-in compiler).
- **Save As…**: save the script under another name.
- **Print…**: open the script, highlighted, in your browser to print it
  from there.
- **To Scratch**: save and compile the script, then copy it into the
  scratch folder: a folder you choose the first time (Tools › Options ›
  Folders changes it), such as the game's or a server's `development`
  folder, for a quick fix. Its compiled script (and debug file) goes
  with it when it compiled just now. A script that doesn't compile, an
  include file (no `main` or `StartingConditional`) or one with an
  error, is copied alone: the Compiler tab says it is an include file,
  or gives the compiler's error, then what was copied ("not compiled").
  A compiled script of that name (and its debug file) that an earlier
  copy left in the folder is removed, for the game would still run it,
  and the Compiler tab and the log name what was removed. A compiled
  script the module has from before is not copied with a script that no
  longer compiles. The module tree's Copy to Scratch Folder follows the
  same rule (see [Modules](03-modules.md), also for exporting files
  elsewhere).

**Indentation.** Tab indents as the script open is indented: Moonglow
looks at its indented lines when it opens it (or reads it again) and
takes tabs or spaces as most of them are, and for spaces the step by
which a line is most often further in than the one before it (two,
four…; the ` * ` down a `/* … */` block's side doesn't count). A script
with no indented line goes by Options › Script Editor › Indent with.
**Edit › Convert Indentation to Tabs** and **to Spaces** (also on the
editor's right-click menu) make every line of the script in front so:
as far in as it was, a tab for each step (what is left of a step, a
column lined up, stays spaces), or spaces for each tab (to the next
multiple of Options' Indent width). It is one step for the editor's
Undo (Ctrl+Z with the editor's keyboard), and Tab indents so from then
on. Enter still begins a new line with the spaces and tabs the line it
is started from has.
- **External Editor**: open the script in the editor set in Options ›
  Script Editor; what you save there comes back into Moonglow's editor.
  In a nasher project it is the project's own file that opens, in place
  (so your editor's own project features see it where it lives), and
  its saves are read again as files changed outside are. A script with
  changes Moonglow has not yet written to the project opens as a copy
  instead: save first.

## The compiler

Moonglow has Beamdog's own NWScript compiler built in, the one the game
and Aurora use, so a script compiles in Moonglow exactly as it does there.
To compile with a program of your own instead (a newer build of the
compiler, `nwnsc`), choose it in **Tools › Options › Script Editor ›
External Script Compiler** (see [Options](10-options.md)); what it says
of a script that doesn't compile shows in the Compiler messages.
`#include` files come from the module, its haks and the game, in the
game's order. It is given the scripts to compile and what they include,
not the whole module, and nothing to read from the keyboard; a run that
takes more than two minutes is stopped, and says so.

**Build › Compile All Scripts** (F7) compiles every script in the module;
the log lists the failures and how many changed. An include file (a
script with no `main` and no `StartingConditional`; one in a comment
doesn't count) isn't compiled on its own: it is compiled into the
scripts that include it, and may lean on what they bring. (Aurora's
build compiles include files too, and reports an error for one that
doesn't compile alone.)
Compile, in an include file's own editor, says that it is one. **Generate Debug
Information** (Options › Script Editor) also stores a `.ndb` with each
compiled script, for script debuggers.

Scripts are referred to by name (at most 16 characters) from events,
conversations and other scripts; the **Edit** buttons beside script
fields open them. The button is there as soon as a name is typed. For a name that is no script yet, **Edit** makes the
script in the module (the log says so) and opens it, as in Aurora: type
the name of a new script into an event and click Edit. A conversation's
"Text Appears When" script starts as a condition (`StartingConditional`).

## NUI Creator

The NUI Creator designs Enhanced Edition UI windows and writes their scripts
(**Tools › New NUI Window…**): see [NUI Creator](18-nui-creator.md).
