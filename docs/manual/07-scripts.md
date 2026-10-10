---
type: Manual Page
title: Scripts
description: Scripts - the script editor, finding your way in code, the compiler and the NUI Creator.
tags: [manual, scripts, nwscript]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-10T04:28:14Z }
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

**Open…** on a script editor's toolbar opens another script: a list of
them found by name, of the module's alone, its haks' alone or all of
them with the game's (Aurora's Resources to Show). A click, or Enter
for the first listed, opens one; the game's and a hak's open to be
read. In **Find Text**, Enter finds the next match, again and again.
**Find In Currently Open Scripts** lists every match in the scripts open
now under Search Results, and **Find In Files** searches (and replaces
in) every script in the module. A double click on a function's or a
constant's name in a script brings its Help forward (over the
compiler's messages, say).
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
  module (File › Save) for the scripts whose text it saves. The scripts
  that include a script compiled or saved so are compiled again with it
  (more than 24 at once in the background): an include file changed
  leaves none of them with an older compiled script. **Compile All
  Scripts** passes over the scripts that, with what they include, are
  as they were when it last compiled them, and says how many it left
  alone. It remembers from one session to the next (in Moonglow's own
  data folder, not in the module), and compiles them all again when the
  haks, the content folders, the compiler or its settings change, or
  where a compiled script is no longer the one it made.
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

## NUI Creator

**Tools › NUI Creator…** creates an Enhanced Edition UI window. Choose a
unique resource name of 1–14 lowercase ASCII letters, digits or underscores.
Existing windows open from **NUI windows** in the module tree. One JUI resource
represents one window. Opening a document replaces the start page in the same
creator workspace. **New NUI…** in the document header creates another document
with a validated resource name. **Load NUI…** searches and opens NUI resources
from the current module. Additional documents share that workspace as tabs.

The **Design** page puts the canvas between a searchable component palette
and the selected element's properties. Click a control on the canvas or in
**Layers** to select it. Click a component to add it to the selected layout
(or the selected control's parent); drag it onto a row, column or list to
choose its destination. A list adds template cells. **Element actions** and
the Layers menu offer move, duplicate and delete. The side panels resize;
in a narrow tab they become **Add element**, **Layers** and **Properties** menus.
**Layers** is a collapsible tree: use its arrows or **Expand all / Collapse all**.
Selecting a control on the canvas reveals its ancestors. Up/Down visit visible
rows; Left/Right collapse/expand branches. New lists start with an empty row
template and a 150-point height. Add the controls you want repeated in each row;
the empty-list hint belongs only to the editor. Existing list contents are kept.

Select **Window** in Layers (or click the preview title) to resize the whole
NUI window. Select a list or another element to resize that control instead.
Drag its right, bottom or corner handle
to change width, height or both. Release to apply one Undo step; Esc cancels.
Handles are hidden in Clean view and remain available during Interact.
Window resizing edits its geometry, preserving position and the `resizable`
setting that controls whether players may resize it in game. List-template cells get
their width from the template; resize their containing list or edit cell settings
in JUI source. Dynamic dimensions are not replaced by a resize gesture.

**Fit** and **100%** control canvas zoom. **Game preview** reads `nui_skin.tml`,
textures and TrueType fonts from your configured NWN installation and current
module content. No game artwork is bundled with Moonglow. **Scale** previews
100–200% UI scaling; **State** shows the selected control normally, hovered,
pressed or disabled. **Clean view** hides selection outlines and editor guides.
Images and image buttons show the named game resource,
including image regions, alignment and aspect modes. List rows use their
corresponding bind-array values.

Standard controls without an explicit width keep their 150-point width in a
column. Widening the window does not stretch those controls; set **Width** when
you need a wider label, button or input. Interact also supports the window's
collapse button. A static `collapsed: false` hides that button; Reset restores
the initial preview state. Font sizes use the installed font's vertical metrics.

Turn on **Interact** to try checkboxes, toggle buttons, sliders, text fields,
dropdowns, options, tabs and the color picker. Event handlers are NWScript and
run only in the game; Interact closes the window for a handler that starts with
closing it (the Close button's). Preview values and list-row values are temporary and leave the
JUI, initial bindings, module dirty state and Undo history unchanged. **Reset**
restores the initial values; leaving Interact discards them. Editing the source
also resets the simulation. List contents scroll with the wheel or scrollbar.
The palette, Layers and properties remain editable during Interact, including
dragging components into layouts. An authoring change resets the simulation
to the updated document. Select controls in Layers while their preview handles
input. Element shortcuts work over the editor panels; over the interactive
canvas, keys belong to the simulated controls.
Disabled and hidden controls cannot change values. This mode simulates local
input; NWN event timing and the native layout solver
still require a game test.

Drag palette components into Layers to add them: the middle of a container
appends a child, while its upper and lower edges insert before or after it.
A leaf inserts after itself when dropped on its middle. The highlighted line
or outline shows the destination. This also works inside the active Swap layout
variant; surrounding controls remain read-only until you return to the main window.
Drag existing controls in Layers to reorder them or move them into containers.
Drop near the top/bottom of a row to insert before/after; the middle targets its
container. Ctrl-click selects several elements for Delete. Use the layer search
to reveal matching branches. **Element actions** can wrap a selection in a row,
column or group. List-to-list moves retain cell width and variable-width settings.

Select a list cell to edit its width and stretch behavior. Select the list itself
to edit **Row data**: add, duplicate or remove a row across its bind arrays.
Structured arrays, colors and rectangles are editable in Properties and Bindings.
Dropdown entries expose their numeric values separately from their labels.
Bind formatting exposes the stock number flags, decimal precision and text flags.

Add **Swap layout** from the Design palette to create an area with replaceable
contents. Select it and use **Variants** in Properties: edit its initial contents,
add an empty variant or copy the initial contents. Variants appear underneath
that Swap layout in Layers and are edited in place, with the rest of the window
visible. The breadcrumb returns to that area's properties. Existing controls or
Groups can be converted with **Convert to Swap layout**.

A variant shows when a script sets it: add a handler (a button's **Clicked**,
say) and **Insert… › Show variant** with the variant, or with **Initial
contents** to switch back. That adds a `NuiSetGroupLayout` call to the
handler's code. The layout itself comes from a function Build writes into the
opener include (`Variant_<name>_<variant>()`, `Initial_<name>_<group>()`), so
a variant you change later reaches the game with the next Build. The event
script includes the opener for them, and Build compiles again every script of
yours that includes it, so the layout it opens is the current one. **Events &
variants** lists the variants with their targets; **Rename…** renames one, its
handlers and the calls to its function following. A variant a handler shows
can't be removed until that call is gone.
**Bindings › Window identity** sets a custom window ID, independent of the
resource name.

**Resources** shows where the skin, images and fonts came from and reports
missing or invalid resources. HAKs take precedence over the current module,
then lower resource layers; NUI images use DDS only after other supported
formats. **Tools › Reload Resources** refreshes changed assets. Edits to module
assets also invalidate the preview cache. Missing assets never alter the JUI.

The preview uses game artwork with local layout and font rendering; it does
not run the NWN engine or scripts. Stock frame bevels are preserved when resized.
Charts render authored series; **Chart series** edits line/column series and
their data. **Draw layers** adds polylines, curves, ellipses, arcs, text, images,
lines and rectangles. Coordinate handles move shapes/control points and resize
rectangles; fields expose binds, paint order and mouse conditions.
A drawing with **Per-row values (in a list)** takes one value per list row
from its bound arrays, drawn once in each row. Numeric text preview limits
precision to 16 digits, an editor limit, not the engine's. Verify exact spacing, drawing, scrolling and events in the game.

#### Images

**Advanced › Images** lists the pictures the window shows (its Images, Image
buttons and draw layers', its variants' too) and where the game finds each:
the module, a hak, the game, or nowhere (the game then shows its `gui_error`
picture). **Add images from disk…** puts pictures (PNG, TGA, DDS, JPG, BMP;
the file's name is the picture's, shortened to at most 16 letters, digits, `_`
or `-` when it is longer or has other symbols)
into the module, to use while you work. Players get custom pictures from a
hak: **Add images to a new hak…** writes them into one, or **Move module
images into a new hak** moves this window's pictures there from the module.
A module that has haks offers them too, in **Into**. A new hak is made in the
game's hak folder (one saved elsewhere is copied there) and added at the
top of the module's haks; Undo takes it off the list and brings the pictures
back into the module, while the hak file keeps them. A listed hak the game
finds elsewhere (its data or the Workshop) can't take pictures: choose a
new hak. Pictures added to the module that the window doesn't show yet are listed
too, ready to be the background. **Use as background**
puts a picture under the window's controls, stretched over it: a draw layer
of the root column, painted before it, whose rectangle and fit you can
change in Draw layers.

**From disk…** beside an Image's, an Image button's or a draw layer's picture
puts a picture file into the module (named as the file; a name the game can't
read, over 16 characters or with other symbols, is shortened, and the log says
to what) and uses it there. **Layers** lists each control's draw layers under
it: choose one to see its outline on the canvas, drag inside it to move it,
its corner to size it, or its points (a line's ends, a curve's, an arc's
centre); the change lands on release, one Undo step.
**Choose image** searches the current resource stack and displays up to 24
matching thumbnails. Cropping uses the native image region. Individual list-row
images are edited in Row data. **Screen** previews common
resolutions and enables title dragging to author window position. A geometry
bind remains a bind. Position and resize gestures commit on release; Esc cancels.
The layout selector offers the full editor, preview alone, or preview beside
editable JUI source. While editing a view, split source shows the companion
settings JSON. Incomplete drafts still participate in Undo and recovery.

With the pointer over **Design**, outside a text field or an open menu:

| Keys | Action |
| --- | --- |
| Delete | Delete the selected element; select its neighbor |
| Ctrl+D | Duplicate with new element IDs |
| Ctrl+C / Ctrl+X / Ctrl+V | Copy / cut / paste elements between NUI windows in this Moonglow session |
| Up / Down | Select the previous / next element in Layers |
| Left / Right | Collapse / expand the selected branch, or visit its parent / child |
| Alt+Up / Alt+Down | Move the element among its siblings |
| Ctrl+Z / Ctrl+Y (or Ctrl+Shift+Z) | Module Undo / Redo |

On macOS, use Cmd instead of Ctrl. **Options › Keyboard › NUI Creator**
changes the delete, duplicate, navigation and reorder keys; action menus
show the current bindings. Text fields retain their text editing keys.
Window roots and a group's required child cannot be deleted. Pasting retains
list cell settings and initial bind values, renaming conflicting binds instead
of replacing the destination's values. The element clipboard is internal to
Moonglow; copying places a short marker on the system clipboard so native
paste keys work even if it was empty. It does not import arbitrary clipboard
text as a layout.
Unknown fields and widget types are retained. Duplicate JSON keys are reported
instead of silently discarding a value.
Unused null `label`/`value` slots are hidden in the inspector. For a supported
property, **Set value…** initializes the appropriate type instead of offering
arbitrary JSON types. Custom data remains available in **Advanced › JUI source**.
**Advanced properties** appears only when the selected element has additional
editable fields; it does not show an empty section for type/version metadata.
**Tooltip when disabled** explains an unavailable control on hover
(for example, insufficient gold). It is optional text, not a switch that turns
the regular tooltip off; remove the tooltip property when no tooltip is wanted.

In **Design**, select a control and use **Property binding** at the top of its
Properties panel. Choose the property, then **+ New bind for property**.
The suggested name and initial value come from the control; **Create & connect**
creates and connects the value in one Undo step. **Cancel bind** leaves the module
unchanged. The source dropdown offers compatible existing binds, or **Constant
(no bind)** to detach while keeping the initial value (the first row for a list).
Other controls using the same bind are unaffected by detaching.

List-template properties create an array with one value per existing row.
Ordinary properties do not offer row arrays. A control whose value is bound
offers **Value changed** in its **Events**: a handler that runs when the value
changes. **Bindings** still lists all values for document-wide editing.

The property's **… › Make dynamic (bind)** menu remains a shortcut.
The menu is available for dynamic arguments in the stock NUI API. Layout
arguments such as list row height and group borders remain constants; replace
the layout to change them while the window is open.
**Bindings › + Create bind** creates a named, typed value with an explicit
initial value. Each bind's **+ Add handler** adds a **Value changed** handler;
the window watches the bind for it. The window's `close` event (the player
closing it) and code that closes the window are separate things.
**Bindings** also edits structured initial values and optional watches;
**Advanced › Binding JSON** exposes the underlying settings. List-template binds need arrays;
ordinary binds use the property's scalar, rectangle or color. The generator
requires explicit defaults and sets every value before enabling watches.
These authoring settings live in `<name>.txt`, marked `moonglow.nui/1`.
The JUI itself contains only game UI data. Both resources use UTF-8.

**Whole window** as a variant's target replaces the root content while keeping
the window's title and geometry; **Insert… › Show variant** adds the call
that shows one in its Swap layout or the whole window.

To make a fragment anywhere in the layout swappable, select it and choose
**Convert to Swap layout**. This wraps it in an unbordered Group with a unique
ID, keeping existing child IDs and explicit dimensions. Check the Group's
size: native Groups do not take their size from their children. Converting
and each new variant are undoable; neither creates an event.

**Layers** names a Swap layout **Swap layout · ID** and lists **Initial
contents** and its variants under it. Choose a variant there to edit its
contents in place, inside the surrounding window, whose other controls are
read-only meanwhile. Nested targets inside another variant are resolved
through their declared parent target. This is a local preview; verify the
result in NWN.

#### Events

What a window does is NWScript you write, in its event script
(`<name>_e.nss`). Each event you handle is a section of that script:

```c
    if (sType == "watch" && sElement == "volume")
    {
        // mg:begin watch volume
        // Your code here.
        // mg:end
        return;
    }
```

Select a control in **Design** and use its **Events** (the window's, with
nothing selected): pick an event and **+ Add handler**. Only the events that
control sends in the game are offered: **Clicked** for buttons, **Focus
gained** and **lost** for text inputs, **Visible range changed** for lists,
**Value changed** for a control whose value is bound, and the mouse for any
control; the window has **Window opened** and **Window closed**. A control
without an ID gets one. Its section is in the event script at once (one an
older Moonglow wrote, unchanged since, is rebuilt with sections, as Build
does). **Events & variants** lists every handler, with a
selector for whose event to add. Renaming a control's ID, a bind or a variant
renames its handlers' sections, and the name where your code quotes it.

**Edit code** opens the event script in the script editor at the handler's
code. **Insert…** adds ready-made code to it: **Close window**, **Set bind**
(to its current value, written as NWScript reads it: `JsonString("…")`,
`JsonInt(…)`, `JsonBool(…)`), **Toggle bind** and **Show variant**; a menu
with nothing to offer is off. After that it is
ordinary code to change. **Event script…** on the toolbar opens the whole
script. **Remove** takes the handler away; what you wrote in it stays in the
script, commented out after `main`, and comes back if you add the handler again.
Code at the top section (between `// mg:begin top` and its `// mg:end`) is the
place for `#include` lines and helper functions.

Choosing **Initial contents** in Layers returns from a variant to the
original. Switching the edited variant does not change the initial in-game
content or trigger an event. Use **Interact** on the main window to try it
locally, then Build and test in NWN.

**Build & compile** prepares `<name>_o.nss` (an include with `Open_<name>`),
`<name>_e.nss` (events) and the event handler's NCS bytecode using the installed
game's API and Moonglow's built-in Beamdog compiler. The opener is validated by
compiling a temporary caller; that caller is never stored in the module.
It changes the module only if validation and event compilation succeed;
one Undo reverses the generated set. The event script is rebuilt around its
handlers, keeping everything written between their markers. A script changed
outside the markers is not overwritten: Build says so, and you move that code
into a section. A script written by hand (with no markers) stays as it is.
The build status includes both scripts, actions and view layouts.
A manually changed opener is reported as a conflict, not replaced. Build
saves the window's own scripts first; save any other open script edits,
including changed includes, before building. A failed Build says so beside
its button (the reason on hover) until the next one.

The opener has **no `main` and no automatic player selection**. Include it in
your own script, choose the player for that script's event and call
`Open_<name>(oPlayer)`. For example, an explicitly chosen OnClientEnter hook is:

```c
#include "example_o"
void main()
{
    Open_example(GetEnteringObject());
}
```

Attach your own script to the event you want. Regenerating an untouched older
opener removes its obsolete NCS in the same undoable operation. Manually edited
openers are still protected. Recompile your calling scripts after regeneration
so they include the current layout; Build does not rewrite those callers.
The event handler receives the window's events directly; generating does not
change the module's OnNuiEvent binding. A new window has a **Clicked**
handler for its Close button whose code closes the window. The `mg_close` ID
has no built-in behavior: without that handler the button does nothing, and
the checks say so (**Add Close handler** adds it). Scripts an older Moonglow
generated are rebuilt into sections on the next Build.

The default opener sends the layout with `NuiCreate`. **Bindings › Client
delivery › Load JUI from the client by resource name** instead uses
`NuiCreateFromResRef`, which requires distributing the
JUI to every client, for example in a HAK. Images and fonts also need to be
available on the client. **Export…** compiles first, then opens the
normal export dialog with JUI, settings and both source/compiled scripts
selected together, plus referenced image/TXI resources present in the module,
including images in named views. External HAK/game artwork is not copied into
the module automatically; include required custom artwork in client distribution.

Edits, including incomplete JSON drafts, participate in module Save, Undo and
recovery. Invalid drafts cannot generate scripts. The checks describe the
stock `nw_inc_nui` API contract and generator requirements. Passing them and
compiling do **not** prove visual or event parity: use Aurora's script/build
workflow and test the window, edits, clicks, watches and closing in NWN.

### What the game refuses, and what it does on its own

These were measured in NWN EE 8193.37 with windows built here; the checks
and the preview follow them.

- **A child that doesn't fit across its row or column.** The game lays a
  window out with a constraint solver. In a row with a fixed height, each
  child's fixed height plus its margins (2 on each side unless you set one)
  must fit: a row 30 high with a button 30 high is refused with *Error
  constructing window from json: The constraint can not be satisfied*, a row
  34 high is fine, and so is a button with margin 0. The same holds for a
  column's fixed width. This is an error, so the window can't be built.
  Along a row or column, children may run past its size. The Creator keeps you
  from getting there: a control added or moved into such a row or column is
  shrunk to fit, a resize handle stops at the room left, and a size given to a
  row or column (the **Auto** button, **Add property**) leaves room for its
  controls and their margins.
- **Margins.** Every control has a margin of 2. Setting one replaces it, so a
  margin of 10 moves a control 8 further out, and 0 moves it 2 back.
- **Room left in a column.** The window's column fills the window. What its
  controls leave goes to those with no height of their own that take room:
  labels, text, images, lists, charts, color pickers, spacers, groups, and rows
  or columns holding only such controls (an empty row too). A button with no
  height is 50 high, or what its row has room for.
- **Text** breaks lines at spaces only; a word wider than the line is cut. Its
  scrollbars move what it holds plus 36 points: a horizontal bar always has 36
  to move, a short text has nothing to scroll vertically.
- **A disabled slider** still takes a click or a drag, and sets its bind to
  its minimum. A warning says so; hide such a slider instead, or check the
  value in your event script.
- **A color picker** has no alpha: the game writes 255 to its bind as soon as
  the window opens, and with every pick. Interact does the same.
- **A Close button** with the ID `mg_close` but no **Clicked** handler does
  nothing in the game. A warning says so.
- **Clip to control** (a control's draw layers, `draw_list_scissor`) clips
  nothing in the game: the layers draw past the control either way. On the
  last draw layers of a window it blanks the whole window. So the Creator
  doesn't offer it (its draw layers are written with it off); a JUI that has
  it on is flagged, an error on the last draw layers, and **Turn it off** in
  the problem list fixes it. The preview doesn't clip.
- **Events.** *Clicked* comes from buttons; a label or a list cell sends only
  *Mouse pressed* and *released* (with the button and the pointer's position
  in the control as payload). A text input sends *Focus gained* and *lost*; a
  list sends *Visible range changed* with the rows in view (`a` to `z`) on
  opening and when scrolled. *Window closed* comes only when the player closes
  the window with its X, never when a script does (`NuiDestroy`). A script
  setting a watched bind runs its *Value changed* handler at once, inside that
  script: a handler that sets its own bind back would never end.
- **Loading the JUI from the client** (`NuiCreateFromResRef`) works with the
  same events and actions, swap layout variants included.
- **Lists.** A list with no width is as wide as its column. It scrolls by whole
  rows: at its end, as many rows stay on top as fit with 36 points to spare.
  Its horizontal bar is drawn but has nothing to move; cells past its width
  are cut.
- **Charts.** Each series scales between its own lowest and highest value; equal
  values draw nothing. Columns follow the game's formula, quirks included: when
  every value is above zero they hang from the top of the chart, and a column
  can reach past the chart, up to what the window or a group cuts. Columns
  stand a point apart. The column under the pointer turns white and shows its
  value (2.00). With no width, a chart stretches like a label.
- **Progress bars** stretch like a label when they have no width. A value over 1
  draws past the bar's frame.
- **Options and tabs** place their entries 150 apart whatever the control's
  width: in a 200-wide Options the second entry starts 154 in.
- **Image buttons** whose picture is missing show the game's `gui_error`
  across the button. A button with a literal value of its own (Toggle button)
  doesn't change when clicked in the game; only a bound one does.
- **A combo's list** is at most 297 high; past that it scrolls.
- **Draw layers with array binds** (`arrayBinds`) take one value per list row,
  as other binds in a list do: outside a list the first values are drawn, once.
- **The window.** `title` set to false, with collapsing and closing off, hides
  the title bar: the window's **Title bar** switch does this, and brings back
  the title and buttons it had. With `accepts_input` false, clicks go through the window. An
  edge constraint keeps the window inside its margins without changing a bound
  geometry's value. A placeholder is drawn greyed. A missing image shows the
  game's `gui_error` picture at its own size in the middle of the image,
  whatever its aspect and alignment; the preview does the same. The title bar is 33
  high, and the body below it has its own frame.
- **Groups** scroll until the far edge of what they hold (with 2 points of
  padding on each side) meets the end of the bar's track. The track stops
  short of the view by the bar's two buttons, so a group always scrolls 36
  points past its content. The preview scrolls groups both ways.
- **Sliders** keep a bound value between their bound minimum and maximum, and
  write the corrected value back: 100 in 0 to 8 becomes 8, and raising the
  minimum to 5 turns 0 into 5. Interact does the same.
- **Bound values take effect at once**: a tooltip or disabled tooltip, and an
  image's aspect and alignment, change as soon as a script sets their binds.
- **Text that doesn't fit.** Left-aligned text (a label, a combo's choice)
  loses its last letters whole; the letter that reaches the edge stays.
  Centred and right-aligned text is cut at the edge instead. A combo's text
  stops 14 points before its arrow.
- **A combo's list** is as wide as its widest choice plus 67, whatever the
  combo's width, 23 points a choice plus 5 high, and always has a scrollbar.
- **Encouraged** controls pulse between their normal and their hover look; the
  preview shows the hover look.
- **Text inputs** count `max` in UTF-8 bytes. Below 4, a letter outside ASCII
  can be stored as garbage (with `max` 1, typing *Ż* stores U+0005). A warning
  says so.
- **UI scale** is capped at the screen's height divided by 720 (never below
  100%), unless `ui.unconstrain-scale` is set: at 150% on a window 800 high
  the game draws at 1.11, on one 993 high at 1.38. The preview's **Screen**
  mode applies the same cap. The game rounds each scaled size down to whole
  pixels (a row 34 high is 37 at 1.11, not 37.8), so at a scale that isn't
  whole the preview's rows drift from the game's by up to a pixel each.
- **A control with no width** in a column is as wide as the column's widest
  control, in the game and in the preview. One case differs: after a control
  with a width of its own, rows of buttons without widths can make the game
  pick a narrower width (an image after a row of three buttons, an image 200
  wide and a row of two is 308 wide in the game, 462 in the preview).
