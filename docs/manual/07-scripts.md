---
type: Manual Page
title: Scripts
description: Scripts - the script editor, finding your way in code, and the compiler.
tags: [manual, scripts, nwscript]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-07T13:30:00Z }
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

Below the text are the **Compiler** messages: click one to go to its line.

| Keys | Do |
| --- | --- |
| Ctrl+F | Find |
| F3 | find the next |
| Ctrl+R | Replace |
| F2, Ctrl+Space | complete the word at the cursor (functions, constants, variables) |
| F12, Ctrl+click | go to the definition of the name at the cursor |
| Shift+F12 | list where the name at the cursor is used |
| Ctrl+Shift+R | rename the name at the cursor everywhere |
| F5 | set or clear a bookmark on the line |
| Ctrl+Shift+1 … 9 | set numbered bookmark 1 to 9 |
| Ctrl+1 … 9 | go to numbered bookmark 1 to 9 |
| F7 | compile all the module's scripts |

**Find In Files** searches (and replaces in) every script in the module.
**Used By** (on the toolbar) lists where the module runs or includes the
script: objects' and areas' events, conversation lines, the module's
events, `#include` lines, and strings in scripts that spell its name
(`ExecuteScript("name", …)`).

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
shows under the toolbar and its line is underlined.

Struct members (`p.nX`) aren't followed. Other editors get the same
features through `mg lsp` (see [Command-line tools](11-command-line.md)).

The buttons:

- **Save**: put the text into the module (File › Save writes the module
  itself). A script tab with text not yet saved into the module says so;
  Moonglow's recovery copies keep that text too.
- **Compile**: save and compile this script; errors go to the Compiler
  messages and the log. With **Automatically Compile Scripts on Save**
  (Options › Script Editor), Save compiles too, and so does saving the
  module (File › Save) for the scripts whose text it saves.
- **Save As…**: save the script under another name.
- **Print…**: open the script, highlighted, in your browser to print it
  from there.
- **To Scratch**: save and compile the script, then copy it and its
  compiled script into the scratch folder: a folder you choose the first
  time (Tools › Options › Folders changes it), such as the game's or a
  server's `development` folder, for a quick fix. A script that doesn't
  compile isn't copied. (See [Modules](03-modules.md) for exporting files
  elsewhere.)
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
game's order.

**Build › Compile All Scripts** (F7) compiles every script in the module;
the log lists the failures and how many changed. An include file (a
script with no `main` and no `StartingConditional`; one in a comment
doesn't count) isn't compiled on its own: it is compiled into the
scripts that include it, and may lean on what they bring. (Aurora's
build compiles include files too, and reports an error for one that
doesn't compile alone.)
Compile in its editor says so. **Generate Debug
Information** (Options › Script Editor) also stores a `.ndb` with each
compiled script, for script debuggers.

Scripts are referred to by name (at most 16 characters) from events,
conversations and other scripts; the **Edit** buttons beside script
fields open them. The button is there as soon as a name is typed. For a name that is no script yet, **Edit** makes the
script in the module (the log says so) and opens it, as in Aurora: type
the name of a new script into an event and click Edit. A conversation's
"Text Appears When" script starts as a condition (`StartingConditional`).
