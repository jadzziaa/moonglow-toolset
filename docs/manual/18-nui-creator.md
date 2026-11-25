---
type: Manual Page
title: NUI Creator
description: The NUI Creator - designing an Enhanced Edition UI window (JUI) on a canvas with the game's skin and fonts, images, keys, binds, swap layout variants, event handlers (or one event script for several windows), the settings file, building its scripts, making windows without the window (render, layout, import), and what the game refuses or does on its own.
tags: [manual, nui, scripts]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-25T08:44:46Z }
sources:
  - id: mg-nui
    resource: crates/mg-nui
    title: The JUI document, its checks and the generated scripts
  - id: nui-view
    resource: crates/mg-ui/src/nui_view.rs
    title: The NUI Creator's editor and preview (with crates/mg-ui/src/nui_view/)
  - id: client
    resource: The game client (NWN:EE 8193.37), windows built here and looked at there
    title: Checks in the game
---

# NUI Creator

**Tools › New NUI Window** (or **New NUI Window…** in the right-click
menu of **NUI Windows** in the Module tree) creates an Enhanced Edition UI
window: it asks for a name of 1–14 lowercase ASCII letters, digits or
underscores, which leaves room for the suffixes of the window's two
scripts. Existing windows open from **NUI Windows** in the Module tree.
One JUI resource is one window, and each window you open is a tab of the
editor area, as the other editors are.

A field you type in changes the window when you leave it or press Enter, a
dragged number when you let go: each is one step for Undo, and the preview
shows what you are typing meanwhile. A field you are typing in whose panel
goes (another tab of the Right Dock chosen, the dock folded, another page)
sets what you typed as it goes. **OK** at the header's right end closes the
designer; **Cancel** closes it and sets the window and its settings back to
what they were when you opened it, as one step that Undo takes back. The
code in the event script's sections is yours and stays, whatever you wrote
in it meanwhile: the script is rebuilt around the handlers Cancel brings
back (the code of a handler that is gone again is kept, commented out, as
Build & Compile keeps it).

Deleting a window in the Module tree deletes its settings and its two
scripts with it. **Rename…** there renames them with it: your handlers'
code moves to the new event script, the opener is written for the new
name, and the scripts of yours that include it are changed to the new
include and function names. A window that was built is built again under
its new name in the same step, its scripts and yours that open it
compiled, as the other renames compile what they change; where it can't
be (no game install, an opener changed by hand, code that doesn't
compile), the log says why, and **Build & Compile** does it afterwards.
**Rename…** on the window's `NAME_o.nss` or `NAME_e.nss` renames the
window. `mg rename MODULE NAME.jui NEW` does the same from the command
line.

## Design

The designer's header holds its pages as a switch (**Design**, **Bindings**,
**Events & Variants**, **Images**), **Build & Compile** (the hammer's
accented button; off while the checks find an error, which its tip counts),
**Export…**, and **More** with **Import…** and **How to Open It…**; at its
right **Interact** and **Reset**, **View**, and **OK** and **Cancel**. What
has no room loses its words, then goes into More. Nothing stands between it
and the canvas.

**The source group.** **View › Layout › Split source** opens the window's
source as an editor group of its own at the design's right (its tab named
as the window, `crier.jui`); in a designer in a window of its own, in that
window, which widens by its room (moved left where the screen's edge is in
the way) and goes back where it was as the group closes (moved or sized
meanwhile, it keeps your place, narrowed again). The source is bound to its
design: Reopen Closed Tab brings it back beside the design, opening the
design first if it is closed, and wherever it was dragged it closes with
the design. A second NUI window opens among the designs, never in a source
group. Its header holds the switch **JUI Source** (the window's text as
the module holds it) and **Binding JSON** (its settings': binds, handlers,
variants), **Event Script…** (the window's `NAME_e.nss`, written and
opened at its handlers) and **Opener Script** (`NAME_o.nss`, off until
Build & Compile makes it). The text is numbered a line at a time in the
script editor's font and colors.

What you type in the source is set when you leave it (another switch
position, a click elsewhere, the group or the design closed with the
keyboard still in it) and before the module is saved, as one step for
Undo. Where the text you typed over was changed from elsewhere meanwhile,
what you typed is let go. A draft that doesn't read is set too: it is
kept, and the pages ask for it to be put right; meanwhile the canvas shows
what you type wherever it reads. The design and the source follow each
other: a control chosen on the canvas or in the Outliner brings the text's
cursor to its first line, the cursor's line marked; a click in the text
chooses the control it is in, and moving the cursor with the keys goes on
choosing while it was the text that chose (a control chosen on the canvas
or in the Outliner stays chosen as you read with the keys, and several
chosen are never let go by the cursor). Choosing a swap layout variant to
edit moves the switch to Binding JSON, where the variant is written. The
switch is kept for each window while Moonglow runs, and with the module's
open tabs when it is opened again. **Editor** or **Preview only** in
**View › Layout**, or the group's own ×, closes the group, and closing the
design closes it too. While the source group is the editor in front, the
keys are its own (Delete and Tab are the text's); a click on the canvas
gives them back to the design.

**Checks** is a panel of the Bottom Dock, its switch in the Status Bar
with the count of what the checks found, errors and warnings together (in
the error color where one is an error, else in the warning color; no count
when they found nothing). It has a row for each finding, errors first, a
window or settings that don't read first of all, each with its kind, the
control it is about (a click chooses it on the canvas, in the Outliner and
in the source), its message (whole in its tip) and its fix (**Bind It**,
**Turn It Off**, **Add Close Handler**); its bar says the window's count
(`crier: 0 errors · 1 warning`). A row's right click (or Shift+F10 with
the keyboard on its control's name, reached by Tab) has
**Copy** (its kind, control and whole message as one line) and **Copy
All**; Ctrl+C copies the row the keyboard is on. New errors bring it to the front and open
the Bottom Dock (warnings open nothing); a designer in a window of its own
shows it in the window's bottom pane. The Outliner marks a control a check
names with a warning (or error) mark at its row's right, its message as
the mark's tip; a click on the mark goes to Checks.

**The Status Bar** says, while the designer is the editor in front, what
the last Build & Compile left, in the warning color (**Not built**: the
module's scripts differ from what Build & Compile makes of the window as
it stands; **Build failed**, the reason on hover), and the window's count
of binds and handlers (`3 binds, 3 handlers`); a designer in a window of
its own says the first in its header. Its hints are the canvas's, as
Options › Keyboard binds them: **Click** Select, **Drag** Resize by
handle, **Esc** Cancel resize, and with a control chosen **Ctrl+D**
Duplicate, **Alt+Up** Move up and **Delete**; in Interact, **Click** Try a
control. It shows none while a field is typed in or a popover is open, on
the other pages, or while the source group is in front.

The designer's panels are the docks', on every page and in every
**Layout** (the editor keeps its width as you switch): **Components** over
**Properties** over **Bindings** in the Right Dock (one bar of three tabs
in a window narrower than 1400 points, and in a narrow editor too:
Properties is always a panel, never a menu, and a dropdown in it opens and
chooses as anywhere), and the window as a tree in the **Outliner** under
the Left Dock's tabs. A designer in a window of its own brings them into
its window: the three at its right, the Outliner at its left where the
window has the room (else a strip at the canvas's left whose icon unfolds
them over it). Two designers in two groups each bring their own as their
group is chosen; a designer's source group brings its designer's.

**Properties** is a form: each label beside its field (over it only where
the panel is too narrow for both), under bold section headings that fold
and unfold with a click (**Property Binding**, **Content**, **Size &
Layout**, **Appearance**, **Behavior**, then the control's own: **List
Cell**, **Row Data**, **Events**, **Draw Layers**, **Variants**…; the
window's **Title bar** check box stands over its Content). **Width** and
**Height** share a row, with one `…` for both, and a size the control
doesn't have says **Automatic** beside **Set Size**, which gives it one
that fits. A property's description is its label's tooltip. Buttons that
add something (**Add Property**, **Add Handler**, **Create Bind**, **Draw
Primitive**, **Add Variant**, **Add Option**…) carry a plus. A field you
are typing in is set when you leave it, when its panel goes, or when the
designer closes (its **Cancel** drops it), and before the module is saved;
if the window changes under it from elsewhere, what you typed is let go,
never set on another control.

**Adding and arranging controls.** Click a control on the canvas or in the
**Outliner** to select it; Ctrl+click in the Outliner adds controls to the
selection, to delete several at once. Click a component in **Components**
(**Find a control…** narrows them) to add it to the selected layout, or to
the selected control's parent. To choose where it goes, drag it into the
Outliner or onto the canvas instead: dropped on the middle of a row,
column or list it becomes its last child, and dropped on a control's upper
or lower edge it goes before or after that control (a control that holds
nothing takes it after itself from its middle too). A line or outline
shows where it will land. A component added to a list adds template cells.
Drag the controls already in the Outliner in the same way to put them in
another order or into another container; a control moved from list to list
keeps its cell width and variable-width settings. **Element Actions** (the
`…` at the Properties panel's header) and **Outliner Actions** (the `…` at
the Outliner's) move, duplicate, wrap (in a row, column or group), delete,
copy, cut and paste. The Properties panel's header names the control
selected; the Bindings panel's counts the binds.

The **Outliner** is a tree whose branches fold, only the rows in sight
laid out: use its arrows or **Expand All** and **Collapse All** at its
header, and **Find a layer or ID…** to list the controls that hold what
you type, their branches unfolded. Selecting a control on the canvas
unfolds its ancestors and brings its row into sight. A click in the
Outliner gives it the keys (F6 too, through the panes): Up and Down visit
the visible rows, Home and End the first and the last, Page Up and Page
Down go by a page, Left and Right fold and unfold branches (Left on a
folded or plain one goes to its parent), Enter folds or unfolds, letters
go to a control by its words, Delete, Duplicate, Move Up and Move Down are
Options › Keyboard's NUI keys, and Escape hands the keys back to the
canvas.

A new list starts with an empty row template and a height of 150 points.
Add the controls you want repeated in each row; the hint an empty list
shows is the editor's alone. A list's existing contents are kept.

**Resizing.** To resize the whole window, select **Window** in the
Outliner (or click the preview's title); to resize a list or another
control, select it. Then drag its right, bottom or corner handle to change
its width, its height or both: releasing it is one Undo step, and Esc
cancels. The handles hide in Clean view and stay in Interact. Resizing the
window changes its geometry and keeps its position and its `resizable`
setting, which decides whether players may resize it in the game. A list
template's cells take their width from the template: resize the list that
holds them, or change the cell's settings in JUI Source. A size that a
bind gives stays a bind when you drag a handle.

A control without a width of its own is as wide as the game makes it,
which the preview follows: see [a control with no
width](#what-the-game-refuses-and-what-it-does-on-its-own) below. Set
**Width** where you want a wider label, button or text field.

**The canvas.** **Fit** and **100%** at the canvas's bottom right set its
zoom (as **View › Zoom** does). At its top left it says **Game preview**
with the UI scale and the state shown (the window's size in its tip), or
**Interact** while you try the window; under it stand the resources'
issues and, with **View › Resources**, where each picture and font came
from. At its bottom left it says what is not drawn here and Interact's
last event. These notes stand over the canvas and take no click beside
them.

The preview reads `nui_skin.tml`, the textures and the TrueType fonts
from your NWN installation and the module's content; no game artwork comes
with Moonglow. It draws the game's artwork with its own layout and font
rendering, and runs neither the game's engine nor its scripts. Images and
image buttons show their game resource, with its image region, alignment
and aspect mode; a list's rows show the values of their bind arrays; the
stock frames keep their bevels when resized; font sizes follow the
installed font's vertical metrics.

The header's **View** holds how the canvas shows the window, kept for the
designer while Moonglow runs:

- **Layout**: **Editor** (the full editor), **Preview only** (the preview
  alone) or **Split source** (the source group, above).
- **UI scale** (100–200%) and **Zoom** (Fit, 100%).
- **State**: **Follow pointer** (the controls look as the pointer over
  them makes them), or the selected control shown **Normal**, **Hover**,
  **Pressed** or **Disabled**; off in Interact.
- **Resources** (below) and **Clean view**, which hides the selection
  outlines and the editor's guides.
- **Preview on screen** shows the window on a screen of the size
  **Screen** chooses (1280 × 720, 1920 × 1080 or 2560 × 1440; choosing one
  turns Preview on screen on). Drag the window's title there to place it:
  the move lands as you release it, Esc cancels it, and a geometry given
  by a bind stays a bind.

Its dropdowns open inside it, and Escape closes it alone.

**Resources** shows where the skin, the images and the fonts came from,
and reports those missing or that don't read. A hak comes before the
module, and the module before the lower layers; a NUI image is taken from
a DDS file only where no other format it reads has it. **Tools › Reload
Resources** reads changed files again, and a change to the module's own
resources reaches the preview too. A missing resource never changes the
JUI.

**Interact.** Turn on **Interact** (the header's play button) to try the
window: check boxes, toggle buttons, sliders, text fields, dropdowns,
options, tabs, the color picker and the window's collapse button (a
static `collapsed: false` hides that button). A text field of several
lines takes no Page Down or Page Up, as in the game. Lists scroll with the
wheel or their scrollbar. Disabled and hidden controls don't change their
values. Event handlers are NWScript and run only in the game, but
Interact closes the window for a handler that starts by closing it (the
Close button's). The values you set, a list's rows' too, are the
preview's alone: the JUI, the binds' initial values, the module's unsaved
state and the Undo history stay as they were. **Reset** sets them back to
their initial values, and so does any change to the window, in its source
or in its panels; leaving Interact drops them.

Components, the Outliner and Properties stay editable in Interact, and you
can drag components into the layout there too. Select controls in the
Outliner while the preview takes their input. The element keys work in the
Outliner, and over the canvas outside Interact; over the canvas in
Interact, the keys are the controls'. Interact simulates local input only:
the game's event timing and its own layout solver want a test in the game.

**Values and properties.** Select a list's cell to edit its width and
whether it stretches. Select the list itself to edit its **Row Data**:
add, duplicate or remove a row across its bind arrays. Properties and
Bindings edit structured arrays, colors and rectangles. Their forms show
an array's first 200 items and the first 200 rows of Row Data, and say how
many more there are: edit a list of thousands of rows in **Binding JSON**,
in the source group. A dropdown shows its entries' numbers apart from
their labels. **Bind Formatting** offers the stock number flags, the
decimal places and the text flags.

The window keeps the fields and the control types Moonglow doesn't know,
and a key written twice in its JSON is reported rather than one of its
values dropped. Properties hides a control's `label` or `value` left null
and unused. **Set Value…** on a property Moonglow knows starts it with a
value of the right type rather than offering any JSON type; other data is
yours to write in **JUI Source**. **Advanced Properties** appears only
when the control has more fields to edit, never as a section holding its
type and version alone. **Tooltip when disabled** explains on hover why a
control can't be used (too little gold, say). It is text you may leave
out, not a switch that turns the plain tooltip off: take the tooltip
property away when you want no tooltip.

**Charts and drawings.** A chart draws the series you give it, and
**Chart series** edits its line and column series and their data. **Draw
Layers** adds polylines, curves, ellipses, arcs, text, images, lines and
rectangles: their handles on the canvas move a shape or its control points
and size a rectangle, and their fields set binds, paint order and the
mouse conditions they draw on. A drawing with **Per-row values (in a
list)** takes one value a list row from its bound arrays, and is drawn
once in each row. A number bound to text is written as the game writes
it: the bind's value as a 32-bit float, to as many places as its
precision asks, ties rounded to even (1/3 at 17 places is
0.33333334326744080; 2.5 at none is 2). Check exact spacing, drawing,
scrolling and events in the game.

For a swap layout, which a script fills with one of several contents, see
[Swap layout variants](#swap-layout-variants).

## Images

**Images** (the header's switch) lists the pictures the window shows (its
Images, Image buttons and draw layers', its variants' too) and where the
game finds each: the module, a hak, the game, or nowhere (the game then
shows its `gui_error` picture). **Add Images from Disk…** puts pictures
(PNG, TGA, DDS, JPG, BMP; the file's name is the picture's, shortened to
at most 16 letters, digits, `_` or `-` when it is longer or has other
symbols) into the module, to use while you work. Players get custom
pictures from a hak: **Add Images to a New Hak…** writes them into one, or
**Move Module Images into a New Hak** moves this window's pictures there
from the module. A module that has haks offers them too, in **Into**. A
new hak is made in the game's hak folder (one saved elsewhere is copied
there) and added at the top of the module's haks; Undo takes it off the
list and brings the pictures back into the module, while the hak file
keeps them. A listed hak the game finds elsewhere (its data or the
Workshop) can't take pictures: choose a new hak. A hak open in the hak
editor takes them there: it is saved at once (a copy of what it was kept
as `.hak.bak`), unless it has other unsaved changes, when the log asks you
to save it there (and a Move leaves the module its pictures until then).
Pictures added to the module that the window doesn't show yet are listed
too, ready to be the background. **Use as Background** puts a picture
under the window's controls, stretched over it: a draw layer of the root
column, painted before it, whose rectangle and fit you can change in Draw
Layers.

**From Disk…** beside an Image's, an Image button's or a draw layer's
picture puts a picture file into the module (named as the file; a name the
game can't read, over 16 characters or with other symbols, is shortened,
and the log says to what) and uses it there. A file of a name the module
has takes that picture's place, which the log says (Undo brings the other
back); a file over 64 MB is refused. The **Outliner** lists each control's
draw layers under it: choose one to see its outline on the canvas, drag
inside it to move it, its corner to size it, or its points (a line's ends,
a curve's, an arc's center); the change lands on release, one Undo step.

**Choose image** searches the resources in the load order and shows up to
24 matching thumbnails. A picture is cropped with the game's own image
region. The pictures of a list's rows are edited in Row Data.

## Keys

With the pointer over **Design**, outside a text field or an open menu:

| Keys | Action |
| --- | --- |
| Delete | Delete the selected element; select its neighbor |
| Ctrl+D | Duplicate with new element IDs |
| Ctrl+C / Ctrl+X / Ctrl+V | Copy / cut / paste elements between NUI windows in this Moonglow session |
| Up / Down | Select the previous / next element in the Outliner |
| Left / Right | Collapse / expand the selected branch, or visit its parent / child |
| Alt+Up / Alt+Down | Move the element among its siblings |
| Ctrl+Z / Ctrl+Y (or Ctrl+Shift+Z) | Module Undo / Redo |

On macOS, use Cmd instead of Ctrl. **Options › Keyboard › NUI Creator**
changes the delete, duplicate, navigation (Up, Down, Left and Right) and
reorder keys, and the action menus show the keys as bound. A text field
keeps its own editing keys.

You can't delete a window's root or the child a group needs. Pasting keeps
a list's cell settings and the binds' initial values, and renames a bind
whose name the destination has rather than replace its value. The element
clipboard is Moonglow's own: copying puts a short marker on the system
clipboard so that the paste keys work even if it was empty, and pasting
never reads other text on the clipboard as a layout.

## Bindings

In **Design**, select a control and open **Property Binding** at the top
of its Properties. Choose the property, then the plus beside **Source**
(**New Bind for a Property**): the name and initial value it suggests come
from the control, **Create** makes the bind and connects it in one Undo
step, and **Cancel** leaves the module as it was. **Source** offers the
existing binds of the right kind, and **Constant (no bind)**, which takes
the property off its bind and keeps its initial value (a list's first
row's); other controls on the same bind keep it. A bound property's row
shows its bind as a chip beside its label (the link icon and the bind's
name): a click on the chip shows that property in Property Binding, where
**Source** changes the bind or takes it off, with its initial value and
its **Bind Formatting**.

**Make Dynamic (Bind)** in a property's **…** menu binds it in one click:
it makes a bind named after the property (`label_1`, say) that holds its
value. It is offered where the stock NUI API takes a dynamic argument.
Layout arguments, such as a list's row height and a group's border, are
constants: replace the layout to change them while the window is open. A
bind made for a property of a list's row template is an array, one value
for each row the list has; any other property's bind holds one value (a
scalar, a rectangle or a color), never an array of rows.

The **Bindings** panel is a table of the window's binds (**Name**,
**Initial Value**, **Watch**; the row of the bind the property chosen in
Property Binding takes its value from is filled, the bind chosen in the
table ringed). **New Bind**, the plus on its header, creates a named,
typed value with an explicit initial value. **Watch** adds or takes off a
bind's **Value changed** handler (the window watches the bind for it). A
row chosen shows that bind's form under the table, where it is renamed,
its initial value edited by its kind, its handlers written (**When This
Value Changes**), where it is used listed and the bind deleted. A bind the
layout names that has no value yet is listed under the table (`name needs
an initial value`) with **Add Value**, which the build needs.

Under the table stand **Custom window ID**, the window's ID in the game
apart from its resource name (empty: the window's name; set as you leave
it, or as its panel goes), and the two choices of client delivery, what
each does in its tip: **Load JUI from the client by resource name** (see
[Build and export](#build-and-export)) and **Events go to the module's
event script** (see [One event script for several
windows](#one-event-script-for-several-windows)). The **Bindings** page
shows every bind's form at once (with **Create Bind**), the panel beside
it then the table alone.

**Binding JSON**, in the source group, shows the settings themselves. A
bind's name (given in **Create Bind**, in **New Bind for a Property** or
by its **Rename**) can't hold a line break, a tab or another control
character, which the event script's section lines can't carry. Every bind
needs an initial value, and the opener sets each value before it watches
any. The settings are the window's `NAME.txt` ([the settings
file](#the-settings-file)), marked `moonglow.nui/1`; the JUI holds the
game's UI data alone, and both are UTF-8.

## Swap layout variants

A swap layout is an area of the window whose contents a script replaces
with one of several variants. Add **Swap Layout** from **Components** to
make one. To make a part of the layout you have swappable,
select it and choose **Convert to Swap Layout**, at the foot of its
Properties: it is wrapped in a Group with no border and an ID of its own,
its children keeping their IDs and its explicit size. Check the Group's
size: a Group in the game doesn't take its size from its children.

Select the swap layout and use **Variants** in its Properties: **Edit
Initial Contents**, **Edit Variant · NAME** for each variant, **Add
Variant** (an empty one) or **Copy Initial Contents** (a variant that
starts as a copy of them). Converting and each new variant are one Undo
step each, and neither adds an event. A variant's target can also be the
whole window (**Whole window**): it replaces the root's contents and keeps
the window's title and geometry.

The **Outliner** names a swap layout **Swap layout · ID** and lists
**Initial Contents** and its variants (**Variant · NAME**) under it.
Choose a variant there to edit its contents in place, inside the rest of
the window, whose other controls are read-only meanwhile; components
dragged into it land in it. A bar over the canvas names the variant: its
**Swap layout · ID** button goes to that area's properties (as **Swap
Layout Properties** in Properties does), and **Back to Main Window**, or
**Initial Contents** in the Outliner, ends the editing. A target nested
inside another variant is found through its declared parent target.
Choosing a variant to edit changes nothing the window shows first in the
game, and sends no event.

A variant shows when a script sets it: add a handler (a button's
**Clicked**, say) and choose **Show variant** in its **Insert…** with the
variant, or with **Initial Contents · ID** to switch back. That adds a
`NuiSetGroupLayout` call to the handler's code. The layout itself comes
from a function **Build & Compile** writes into the opener include
(`Variant_<name>_<variant>()`, `Initial_<name>_<group>()`), so a variant
you change reaches the game with the next build. The event script
includes the opener for them, and the build compiles again the scripts
that include it ([Build and export](#build-and-export)), so the layout it
opens is the current one. **Events & Variants** lists the variants with
their targets under **Swap Layout Variants**: **Edit Variant** opens one,
**Rename…** renames it (its handlers and the calls to its function
follow), and **Remove** takes it away. A variant a handler shows can't be
removed until that call is gone.

What you see while you edit a variant is a local preview: try the main
window in **Interact**, then build it and try it in the game with [Test
Module](09-build-and-test.md).

## Events

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
nothing selected): pick an event and **Add Handler**. Only the events that
control sends in the game are offered: **Clicked** for buttons, **Focus
gained** and **lost** for text inputs, **Visible range changed** for lists,
**Value changed** for a control whose value is bound, and the mouse for any
control; the window has **Window opened** and **Window closed**. A control
without an ID gets one. Its section is in the event script at once.
**Events & Variants** lists every handler, with a selector for whose event
to add. The window's `close` event (the player closing it) is not the same
as code that closes the window.

Renaming a control's ID, a bind or a variant renames its handlers'
sections, and the name where your code names it: as an argument of one of
the game's NUI functions (`NuiSetBind(oPlayer, nToken, "name", …)`), in a
comparison (`sElement == "name"`), and quoted inside a JSON string.
Another string that happens to spell the name (a message, a tag, a local
variable's name) is left as it is, and so is the name passed through a
function of your own.

**Edit Code** opens the event script in the script editor at the handler's
code. **Insert…** adds ready-made code to it: **Close Window**, **Set bind**
(to its current value, written as NWScript reads it: `JsonString("…")`,
`JsonInt(…)`, `JsonBool(…)`), **Toggle bind** and **Show variant**; a menu
with nothing to offer is off. After that it is ordinary code to change.
**Event Script…** in the source group's header opens the whole script.
**Remove** takes the handler away; what you wrote in it stays in the
script, commented out after `main`, and comes back if you add the handler
again. Code at the top section (between `// mg:begin top` and its
`// mg:end`) is the place for `#include` lines and helper functions. When
the script's editor holds unsaved text as the Creator writes into it
(Insert…, a handler added or removed), that text is stored first, as a
step of its own: **Undo** takes back what the Creator wrote and leaves
what you typed.

### One event script for several windows

A module that sends every window's events to one script of its own (its
OnNuiEvent script, a dispatcher handing each event on by window) can
keep doing so. **Events go to the module's event script**, under the
Bindings panel's table, makes the window's `NAME_e.nss` an include rather
than a script: its handlers' sections are the same, inside a function
`NAME_OnEvent()` that handles the window's events and returns `TRUE` for
them (`FALSE` for another window's, known by the window's ID). The opener
creates the window with no event script of its own, so its events reach
the module's. Include it there and call it:

```c
#include "crier_e"
#include "shop_e"
void main()
{
    if (crier_OnEvent()) return;
    if (shop_OnEvent()) return;
    // The module's other windows.
}
```

In such a handler, end early with `return TRUE;`, not `return;`. Build &
Compile compiles the include through a caller of its own, and compiles
again the module's scripts that include it; switching the option keeps
the code in the sections. Code written while the window had a script of
its own often ends a handler with `return;`, which the function can't
compile (the compiler's *RETURN TYPE AND FUNCTION TYPE MISMATCHED*):
Build & Compile makes each bare `return;` in a handler's section
`return TRUE;`, and switched back, `return TRUE;` and `return FALSE;`
`return;` again, and says which section and line it changed (in the log;
`mg nui generate` prints them, and with `--json` lists them in
`returns`). Only the handlers' sections are changed: the top section's
helper functions, strings, comments and a return of anything else
(`return Foo();`) stay as you wrote them, and a return the compiler still
refuses is said with what the function returns. Renaming the window
renames the function and the `#include` lines.

## The settings file

What the Creator keeps of a window beside its layout is the TXT resource
of its name, `NAME.txt`: UTF-8 JSON, no key twice in one object. A
program or an agent can write it as the Creator does; `mg nui schema`
prints its form as a JSON Schema, made from the same definitions
Moonglow reads it with. Keys it does not know are kept, at every level.
A key left out takes its default.

| Key | Default | What it holds |
| --- | --- | --- |
| `format` | (required) | `"moonglow.nui/1"`: a TXT with another is not a window's settings |
| `bindings` | none | The binds by name, each `{"value": …, "watch": false}`. `value` is what the opener sets first: a string, number, boolean, color (`{"r","g","b","a"}`), rectangle (`{"x","y","w","h"}`), or for a list's row template an array, a value a row. Every bind the layout reads needs one. `watch`: a change is a `watch` event (a `watch` handler watches it as well) |
| `actions` | none | The handlers, in the event script's order: `{"event", "element", "action"}`. `event` is one the game sends (`click`, `watch`, `open`, `close`, `mousedown`, `mouseup`, `mousescroll`, `focus`, `blur`, `range`); `element` the control's ID, the bind's name for `watch`, empty for the window's `open` and `close`. `action` is what its section starts with: `code` (a note for your code), `close`, `toggle` (with `bind`), `set` (with `bind` and `value`) or `view` (with `group`, `_window_` for the whole window, and `view`) |
| `views` | none | The swap layout variants by name, each a layout (an element, as the JUI's `root` is) |
| `view_targets` | none | The group each variant was made for, by the variant's name |
| `swap_hosts` | none | The IDs of the groups made as swap layouts |
| `window_id` | the window's name | Its ID in the game (`NuiFindWindow`) |
| `from_resref` | `false` | The opener loads the JUI on the client by name (`NuiCreateFromResRef`): the JUI has to be in a hak |
| `event_function` | `false` | The window's events go to the module's own event script: Build & Compile writes `NAME_e.nss` as an include with `NAME_OnEvent()` ([above](#one-event-script-for-several-windows)) |
| `opener_hash`, `event_hash` | none | Written by Build & Compile, to tell a script changed by hand from its own: leave them out of a file you write |

The town crier's window, a text box whose watch turns Send on, Send and
Cancel:

```json
{
  "format": "moonglow.nui/1",
  "bindings": {
    "can_send": {"value": false},
    "fee_text": {"value": "The fee is 5 silver pieces."},
    "message": {"value": "", "watch": true}
  },
  "actions": [
    {"event": "watch", "element": "message", "action": "code"},
    {"event": "click", "element": "send", "action": "code"},
    {"event": "click", "element": "cancel", "action": "close"}
  ],
  "window_id": "tdod_crier"
}
```

`mg nui schema --layout` prints a new window and each control as the
Creator adds it, with the fields the game's `nw_inc_nui` functions write;
the game's `nw_inc_nui.nss` says what they mean. `mg nui schema --jui`
prints the layout's form as a JSON Schema: the window's properties and
each control's by its type, the kind of value each takes, and whether a
bind may give it or `nw_inc_nui` makes it static. It is made of the
tables the checks hold a window to, so the two agree; what the checks
hold beyond a property's value (a group's one child, a child that doesn't
fit across its row, a text field's `max`) `mg nui validate` says.

## Build and export

**Build & Compile** writes `<name>_o.nss` (an include with `Open_<name>`)
and `<name>_e.nss` (the events), and compiles the event script to NCS with
the installed game's API and the compiler Compile All uses: the built-in
one, or the external one chosen in Options › Script Editor, with debug
files (`.ndb`) when Generate Debug Information is on. It checks the opener
by compiling a caller it makes for the purpose and never stores in the
module. It changes the module only when that check and the event script's
compiling succeed, and one Undo takes back all it wrote. Like any edit,
what it writes reaches the disk when you save the module: in a nasher
project, File › Save writes the scripts into the project's sources, where
its rules put them. A failed build is said in the Status Bar until the
next one ([Design](#design)).

The event script is rebuilt around its handlers, keeping everything
written between their markers. Build & Compile never overwrites a script
changed by hand: an event script changed outside the markers is left as it
is and the build says so, for you to move that code into a section; a
script written by hand (with no markers) stays as it is; and an opener
changed by hand is reported as a conflict, not replaced. A compiled
`NAME_o.ncs` beside an opener you haven't changed is deleted in the same
Undo step: the opener is an include, compiled into the scripts that use
it. Build & Compile saves the window's own scripts first; save your other
open scripts, the includes you changed among them, before you build. What
it says covers both scripts, the handlers and the swap layout variants.

The opener has **no `main` and no automatic player selection**. Include it
in your own script, attach that script to the event you want, choose the
player for that event and call `Open_<name>(oPlayer)`. For example, an
OnClientEnter script of your own:

```c
#include "example_o"
void main()
{
    Open_example(GetEnteringObject());
}
```

Build & Compile compiles again the scripts of yours that include the
opener (through an include of yours too), so they hold the current layout;
it doesn't rewrite them. One of them that doesn't compile is said in the
log, as Compile All says it, and keeps the compiled script it had: the
window's own scripts are built all the same. The window's events go
straight to its event script: building doesn't change the module's
OnNuiEvent script. A new window has a **Clicked** handler for its Close
button whose code closes the window. The `mg_close` ID has no built-in
behavior: without that handler the button does nothing, and the checks
say so (**Add Close Handler** adds it).

The opener sends the layout with `NuiCreate`. With **Load JUI from the
client by resource name** (under the Bindings panel's table) it uses
`NuiCreateFromResRef` instead, and every client then needs the JUI, in a
hak for example; the window's images and fonts must be on the client too.
A window loaded so sends the same events to the same handlers, swap layout
variants included (measured in NWN EE 8193.37).

**Export…** compiles first, then opens the export dialog with the JUI, the
settings and both scripts, source and compiled, chosen together, with the
image and TXI resources the window names that the module holds, those of
its swap layout variants included. Pictures the window takes from a hak or
the game are not exported; the log names those a client could miss (from
a hak, this machine's override or development folder, or found nowhere),
which players get from the hak or which you add to the module. The game's
own go unsaid: every client has them.

**Import…**, in the header's More, brings windows made elsewhere into the
module, as `mg nui import` does: choose `.jui` files (each brings its
`NAME.txt` and `NAME_e.nss` from beside it) or ERF archives (`.erf`,
`.hak`, `.mod`, `.nwm`) and zip files of windows. Each window comes with
its settings, when they are the Creator's, and its event script's source;
not its opener or compiled scripts, which Build & Compile writes for this
module. A window the module has otherwise is asked about first (**Files
Already in the Module**): **Replace** (with their count) brings every
window, **Skip Those** only the ones the module doesn't have. The windows
come in as one Undo step, the first opens in the Creator, and the log says
what their checks found.

Your edits, unfinished JSON drafts included, are part of the module's
Save, Undo and recovery; a draft that doesn't read can't be built. The
checks (in **Checks**, above) hold the window to the stock `nw_inc_nui`
API and to what the build needs. Passing them and compiling do **not**
prove that the window looks and behaves so in the game: build the module
and use [Test Module](09-build-and-test.md) to try the window, its edits,
clicks, watches and closing there.

## Without the window

A window can be made without opening the Creator, by hand or by an
agent working from a brief ([The command line](11-command-line.md) has
every option):

1. `mg nui new MODULE NAME` adds a starting window, or `mg nui import
   MODULE FILE.jui` brings one written elsewhere (with its `.txt`) into a
   module, an archive too. `mg nui schema` prints the settings' form.
2. `mg nui validate MODULE NAME` runs the Creator's checks.
3. `mg nui render MODULE NAME -o NAME.png` draws it as the canvas does,
   off the screen, at a UI scale and a window size of your choosing, and
   `mg nui layout MODULE NAME --json` gives each control's rectangle and
   what doesn't fit (clipped, out of view, no room, text cut, a title
   longer than its bar, and what the game refuses to build at all, as the
   checks say it, such as a child that doesn't fit across its row or
   column): the same layout the picture shows, a list's rows out of its
   view included. Both take a `.jui` file alone too, and
   draw the window otherwise than as it opens with the same options:
   `--state hover:send` (or `pressed`, `disabled`, `normal`; without
   `:ID`, every control; `hover:read@2`, a list's control in its third
   row alone), `--binds FILE.json` (the binds as a script sets them, so
   a list has the rows a character would see; a value the bind's
   properties don't take is said, as the checks say it) and `--variant
   NAME` (a swap layout variant in its group; again for another group's).
   For the crier's list of cries in this repository's tests:
   `mg nui render cries.jui --binds cries.binds.json --state hover:read
   --state pressed:read@2`.
4. `mg nui generate MODULE NAME` writes and compiles its scripts.

In a nasher project whose sources take in scripts and JSON only, a
window's files would be written where nasher neither packs them nor
Moonglow reads them back: `new`, `validate`, New NUI Window and Verify
(the `nui-sources` check) say so, with the `include` line that takes
them in, which **Build › Add Left-Out Files to nasher.cfg** adds once
they are saved.

## What the game refuses, and what it does on its own

These were measured in NWN EE 8193.37 with windows built here; the checks
and the preview follow them.

- **A child that doesn't fit across its row or column.** The game lays a
  window out with a constraint solver. In a row with a fixed height, each
  child's fixed height plus its margins (2 on each side unless you set one)
  must fit: a row 30 high with a button 30 high is refused with *Error
  constructing window from json: The constraint can not be satisfied*, a row
  34 high is fine, and so is a button with margin 0. The same holds for a
  column's fixed width: a column 316 wide holding a label 316 wide (320 with
  its margins) is refused, in a row as in a column. A row or column that is a
  group's child (or a variant shown in a group) is not held to its own size:
  the group lays it out, and a column of 100 holding a label of 150 opens
  there (checked in 8193.37-17). This is an error, so the window can't be
  built. Along a row or column, children may run past its size. The
  Creator keeps you from getting there: a control added or moved into such
  a row or column is shrunk to fit, a resize handle stops at the room
  left, and a size given to a row or column (**Set Size**, **Add
  Property**) leaves room for its controls and their margins.
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
  it on is flagged, an error on the last draw layers, and **Turn It Off** in
  **Checks** fixes it. The preview doesn't clip. A group's draw layers reach
  past the group out to the window's body, as the game draws them.
- **A group as the window's root** draws no draw layers of its own (a row's
  or a column's at the root does): put them on its child. Its border is 4
  points out from where its controls begin, in the window's inset; the
  preview draws both so (checked in 8193.37-17).
- **Events.** *Clicked* comes from buttons; a label or a list cell sends only
  *Mouse pressed* and *released* (with the button and the pointer's position
  in the control as payload). A text input sends *Focus gained* and *lost*; a
  list sends *Visible range changed* with the rows in view (`a` to `z`) on
  opening and when scrolled. *Window closed* comes only when the player closes
  the window with its X, never when a script does (`NuiDestroy`). A script
  setting a watched bind runs its *Value changed* handler at once, inside that
  script: a handler that sets its own bind back would never end.
- **Lists.** A list with no width is as wide as its column. It scrolls by whole
  rows: at its end, as many rows stay on top as fit with 36 points to spare.
  Its horizontal bar is drawn but has nothing to move; cells past its width
  are cut.
- **Charts.** Each series scales between its own lowest and highest value; equal
  values draw nothing. Columns follow the game's formula, quirks included: when
  every value is above zero they hang from the top of the chart, and a column
  can reach past the chart, up to what the window or a group cuts. Columns
  stand a point apart. The column under the pointer turns white and shows its
  value (2.00). With no width, a chart stretches like a label. The preview
  draws a line of more samples than twice its pixels by the lowest and
  highest sample of each pixel, and a polyline of more than 8,192 points by
  as many of them, evenly taken: the same picture, of fewer shapes.
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
  the title bar: the window's **Title bar** check box does this, and brings
  back the title and buttons it had. With `accepts_input` false, clicks go
  through the window. An edge constraint keeps the window inside its
  margins without changing a bound geometry's value. A placeholder is drawn
  grayed. A missing image shows the game's `gui_error` picture at its own
  size in the middle of the image, whatever its aspect and alignment; the
  preview does the same. The title bar is 33 high, and the body below it
  has its own frame.
- **Groups** scroll until the far edge of what they hold (with 2 points of
  padding on each side) meets the end of the bar's track. The track stops
  short of the view by the bar's two buttons, so a group always scrolls 36
  points past its content. The preview scrolls groups both ways.
- **A scrollbar's end buttons** scroll while they are held, not by a step a
  click: a hundredth of the bar's length each frame the game draws, the same
  for a list, a group and a text (a list shows whole rows of it). So a click
  scrolls a few points, and how fast a button held scrolls goes with the
  player's frame rate; the preview scrolls as the game does at 60 frames a
  second.
- **Sliders** keep a bound value between their bound minimum and maximum, and
  write the corrected value back: 100 in 0 to 8 becomes 8, and raising the
  minimum to 5 turns 0 into 5. Interact does the same.
- **Bound values take effect at once**: a tooltip or disabled tooltip, and an
  image's aspect and alignment, change as soon as a script sets their binds.
- **Text that doesn't fit.** Left-aligned text (a label, a combo's choice)
  loses its last letters whole; the letter that reaches the edge stays.
  Centered and right-aligned text is cut at the edge instead. A combo's text
  stops 14 points before its arrow. Wrapped text (a text box with no
  vertical scrollbar, a label) that runs past the bottom of its box is cut
  there; `mg nui layout` says so, with the height that shows it all
  (`needs_height`).
- **A combo's list** is as wide as its widest choice plus 67, whatever the
  combo's width, 23 points a choice plus 5 high, and always has a scrollbar.
- **Encouraged** controls pulse between their normal and their hover look; the
  preview shows the hover look.
- **What the game takes all the same.** A window whose root is a label, a
  button or a spacer rather than a row, column or group is shown; a window
  ID with punctuation, spaces or letters outside ASCII opens, and the
  window is found by it; an arc's center can be a point or the rectangle
  `nw_inc_nui` documents. The checks say nothing of these. An alignment,
  a draw layer's **Show when** or its paint order outside the game's
  constants is shown too: a warning says so, as it is likely a slip.
- **Text inputs** count `max` in UTF-8 bytes. Below 4, a letter outside ASCII
  can be stored as garbage (with `max` 1, typing *Ż* stores U+0005). A warning
  says so.
- **UI scale** is capped at the screen's height divided by 720 (never below
  100%), unless `ui.unconstrain-scale` is set: at 150% on a window 800 high
  the game draws at 1.11, on one 993 high at 1.38. **View › Preview on
  screen** applies the same cap. The game rounds each scaled size down to whole
  pixels (a row 34 high is 37 at 1.11, not 37.7), and so does the preview:
  each control's size, its margins and the gaps between controls (ten
  buttons 30 high with margins of 5 are 43 pixels apart at 1.11), a
  list's rows and its cells' widths.
- **A control with no width.** A button, image button, toggle button,
  check box, combo box or slider without a width is 150 points wide (a
  button in a row too; `NUI_STYLE_PRIMARY_WIDTH` in the game's
  `nw_inc_nui.nss`, and the width the client gives it); options and tabs take 150 an entry, a
  color picker 300. A label, text, text field, image, spacer, progress
  bar, list, chart, row, column or group without one takes its column's
  width. A column is as wide as its widest control (a label, text, text
  field, image or spacer counts nothing toward it; a list, chart or
  progress bar counts 150), and so is the window's own column: it is not
  as wide as the window. A new window's column is 154 points wide whatever
  the window's size, and a list or label in it 150; widening the window
  doesn't stretch them. A control as wide as the window less 20 points
  fills the window's column (the 8-point inset and the column's 2 points
  of padding on each side), and what has no width of its own fills it
  with it. A new window's controls have no widths, as `nw_inc_nui`'s
  functions make them, which `mg nui schema --layout` shows. A row's last
  button without a width stretches to a control with a width of its own
  after the row in its column, until the row is as wide as that control
  less 2 (one button under an image 200 wide is 196, two under one 400
  wide are 150 and 242), never to one before the row, nor where its
  buttons are wider already.
