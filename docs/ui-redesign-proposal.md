---
type: Proposal
title: 'One look and one shell for the suite: the UI redesign'
description: The approved redesign of Moonglow Toolset and Moonglow Viewer - the seven rules, every color token in dark and light, type and metrics, the shell and how it yields at small widths, the components, the icons, what the redesign does to each editor, the area view's handles (Fine Lines), the owner's rulings, the features it adds, what is decided but not drawn, the probable bugs found in today's app, and the milestones to build it (M1 the shared crate mg-uikit to M6 the viewer).
tags: [proposal, ui, redesign, theme, tokens, shell, icons, gizmos, galleries, light-theme, viewer, mg-uikit]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-12T21:50:22Z }
status: stable
stale_after: 2027-03-01T00:00:00Z
sources:
  - id: canvas
    resource: https://claude.ai/artifact/A82Wjnuz9umTkAhsRDMWpj
    title: The design canvas, 48 boards on ten pages (Overview, Toolset, More Editors, States and Dialogs, Galleries, Gizmos, Light Theme, Viewer, Ultrawide, Foundations); private to the owner
  - id: spec
    resource: target/ui-mock/SPEC.md (not in git)
    title: The mockup spec - rules, constraints, tokens, shell, icons, the rulings of 2026-11-10 and 2026-11-11
  - id: components
    resource: target/ui-mock/COMPONENTS.md (not in git)
    title: The component catalog and the gallery cell (Quiet Tiles)
  - id: gizmos
    resource: target/ui-mock/GIZMOS.md (not in git), work/Gizmos/gz.py, src/GizmoStates.html
    title: The area view's handles - today's facts, the chosen look, the measures and colors as built
  - id: gallery
    resource: target/ui-mock/GALLERY.md, PICKERS.md (not in git)
    title: Facts from the code on gallery views and pickers (2026-11-10)
  - id: css
    resource: target/ui-mock/canvas/project/moonglow.css (not in git)
    title: The stylesheet - the dark tokens and the metrics
  - id: light
    resource: target/ui-mock/src/LightArea.html (LIGHT-BEGIN to LIGHT-END), work/LightArea/light.css, work/LightArea/ratios.txt (not in git)
    title: The light theme's value for every token, and the contrast ratios
  - id: moves
    resource: target/ui-mock/src/Moves.html, Moves2.html, Moves3.html (not in git)
    title: Where Things Moved - the control-by-control mapping (here, ui-redesign-moves.md)
  - id: overview
    resource: target/ui-mock/src/Overview.html (not in git)
    title: The overview board - rulings, Found on the Way, what is not mocked
  - id: rulings
    resource: human:august (2026-11-10 and 2026-11-11)
    title: The owner's rulings on the redesign, in chat
---

# One look and one shell for the suite: the UI redesign

**Status.** The design is approved by the owner. Implementation began on 2026-11-11 on the branch `ui-redesign`. This document is the design in words, for an implementer who cannot open the mock files: they are working files in a build folder (`target/ui-mock/`) and are not in git. The pictures are the boards of a private design canvas, which only the owner can open: <https://claude.ai/artifact/A82Wjnuz9umTkAhsRDMWpj> (ten pages, 48 boards). Every value and name below comes from those files; where they disagree, the later ruling wins and the text says so. The control-by-control checklist is [Where every control goes](ui-redesign-moves.md).

## 1. What was asked

The owner asked for a more polished, consistent look across the suite that uses the screen intelligently and stays intuitive, keeping the suite's utilitarian spirit: no decoration for its own sake. The named inspirations are Blender and Godot. The owner's screen is 3440 by 1440 and the budget is 144 frames per second (about 6.9 ms a frame).

Two programs of the suite have a window and both are covered:

- **Moonglow Toolset** (`crates/mg-ui`, egui), the repository this document is in.
- **Moonglow Viewer** (`crates/mgv-ui`, in its own repository), which copies the toolset's few shared helpers by hand today.

### Where the applications are today (the facts the design started from)

- Neither app sets a theme: both run on egui 0.36's stock dark visuals (panel gray 27, 13 px Ubuntu Light, Hack mono, corner radius 2). Icons are emoji and Unicode glyphs.
- Toolset shell: menu bar, a toolbar row of 13 word buttons, a left module tree (240 px), a dock that holds only areas and the palette, a log panel (120 px) and a status line. Every other editor opens as a floating window of fixed size (780 to 1140 px wide) over the dock.
- The area view puts about 35 toggles and dropdowns in a strip that wraps to three rows, plus a hint line, above the 3D view.
- Blueprint editors are a wrapped row of 5 to 14 page tabs, one page of form at a time, OK and Cancel at the bottom, and the 3D preview in a separate small window.
- The viewer has an `egui_dock` layout (Resources and Nodes left; 3D View, Texture and ASCII in the middle; Inspector and Lighting right; Animation, Effects and Log below) and no toolbar or theme.

## 2. The seven rules

1. **Docked or floating, by the builder's choice** (ruled 2026-11-10; this rule first said "everything docked"). An editor opens as a tab in the editor area or as a floating window, by a switch in Options. Either way it wears the same theme, header row and panels. A floating editor keeps its panels inside its own window; the docks follow the active docked editor.
2. **One shell for every app**, with the same region names and positions: Top Bar, Left Dock, Editor Area, Right Dock, Bottom Dock, Status Bar.
3. **The active editor fills the docks.** The Left Dock is where you find things (Module, Resources, and an Outliner of what the active editor holds). The Right Dock is where you change things (area: Palette and Properties; script: Reference; conversation: Node; blueprint: Preview; viewer: Inspector and Lighting). The Bottom Dock is where results arrive (Log, Problems, Compiler, Search Results, the viewer's Timeline).
4. **One header row per editor**, Blender's way: what you do on the left, how the view looks on the right. What does not fit goes into a named popover on that row. Nothing wraps to a second row. Where the row is wide enough (ultrawide), popovers unfold into labeled toggles in place.
5. **Hints go to the Status Bar**, as key caps with a word each, not into the editor.
6. **Width is used.** Forms flow into columns of about 440 to 520 px; a blueprint's pages sit side by side when there is room; docks stack two panels high on a normal screen and stand two columns wide on an ultrawide one; the editor area splits into editor groups.
7. **Chrome recedes.** Three surface levels, 1 px lines, one accent color, one drawn icon set with a tooltip on every icon. No gradients, no blur, no rounded cards, no shadows except under popovers.

Two smaller rules the boards follow:

- **Pages.** A form of many pages gets a **page list at its left** (a creature blueprint has 14 pages, Options 10). An editor with a few short pages whose body needs the width gets a **switch in its header** (the tileset editor, the NUI designer).
- **A tool's options take the header's left group** while the tool is active (Paint, Select Tiles); what does not fit goes into a popover named for the tool; the view controls stay at the right.

A panel's switch in the Status Bar shows or hides that panel wherever it is docked.

## 3. Constraints

- **Buildable in egui at 144 fps.** Flat fills, 1 px strokes, radius 3, the fonts the apps ship (Ubuntu Light, Ubuntu Bold, Hack mono). Nothing that needs blur, gradients, animation or web-only typography.
- **Nothing disappears.** Every control of today's screens has a named home in the redesign (header, popover, dock, tool shelf, Status Bar, menu). The mapping is [ui-redesign-moves.md](ui-redesign-moves.md). A screen that looks clean because controls were left out is wrong.
- **Real names.** Page names, button names, menu names and field labels come from the code, not from memory or invention. What the redesign adds that does not exist today is limited to the items marked NEW (section 14).
- **Interface text** follows [the style guide](style-guide.md): title case for titles, tabs, menus, buttons and tool names; sentence case for field labels, setting checkboxes, tooltips and status text; American English; an ellipsis only after a command that asks more. The boards write field labels in sentence case even where today's code writes title case (`Hit points`, not `Hit Points`).
- **Field labels are bright, not bold; section headings are Ubuntu Bold, larger, with a rule under them** (the owner's standing preference; `widgets::field_label` and `widgets::section_heading` today).
- **Content is the game's** in every picture: the Chapter1 module, area `map_m1q1a`, creature Aribeth `m1q1faribeth`, script `m1q3_formosa_9`, conversation `m1q1faribeth`, viewer model `c_drgred`.
- **Contrast.** Text at least 4.5:1 on `--panel` and `--bar` (`--text-off` only for disabled controls). The ratios for both themes are in section 4.4.

## 4. Tokens, type and metrics

All colors are named tokens on one root; the theme is the set of values. Dark is the default theme; the light theme is the second set, ruled in on 2026-11-10 (its values come from the Light Theme page's tokens board and are checked against the ratios in section 4.4). The names below are the stylesheet's; the Rust names are for M1 to choose, one-to-one.

### 4.1 Colors

| Token | Dark | Light | Use |
|---|---|---|---|
| `--base` | `#15171a` | `#c4c9d0` | behind the docks, the 3D view's surround |
| `--panel` | `#1e2125` | `#e4e7eb` | dock and editor bodies |
| `--bar` | `#262a2f` | `#d8dce1` | top bar, tab bars, editor headers, status bar |
| `--raised` | `#30353c` | `#c9ced5` | buttons, combo boxes, toggles that are off |
| `--raised-hover` | `#3b414a` | `#bcc2ca` | the same, hovered; also the scroll thumb |
| `--inset` | `#131518` | `#f4f5f7` | text fields, list wells, the code editor |
| `--line` | `#0c0d0f` | `#adb3bb` | 1 px separators between regions |
| `--line-soft` | `#383d45` | `#b9bfc7` | control borders, rules inside a panel |
| `--text` | `#e6e8ec` | `#14171b` | primary text, field labels |
| `--text-2` | `#aab0ba` | `#363c44` | secondary text, inactive tabs, read-only values |
| `--text-3` | `#8a919e` | `#4c535b` | weak text: counts, hints, notes |
| `--text-off` | `#5f6672` | `#838a95` | disabled |
| `--accent` | `#62a8ea` | `#175896` | accent as text, icon or line |
| `--accent-fill` | `#1d5488` | `#2765a1` | toggled on, selected row |
| `--accent-fill-hover` | `#24629d` | `#1f598f` | the same, hovered |
| `--on-fill` | `#ffffff` | `#ffffff` | text on `--accent-fill` |
| `--select` | `#ffb02e` | `#f09000` | selection in a 3D view, the modified dot |
| `--error` | `#f0666a` | `#b4232b` | errors (dark was `#e5484d`, under 4.5:1 as text) |
| `--warn` | `#e8a33d` | `#8a5000` | warnings |
| `--ok` | `#4cc35a` | `#1b6b2d` | success |
| `--info` | `#5b9bd5` | `#1f5a98` | information |
| `--hover-row` | `#2a2e34` | `#d5d9df` | a hovered tree or list row |
| `--current-line` | `#1b1e22` | `#e9edf3` | the code view's current line |
| `--overlay` | `rgba(21, 23, 26, 0.86)` | `rgba(232, 235, 239, 0.90)` | overlays on a 3D view (compass ring, minimap) |
| `--shadow` | `0 6px 18px rgba(0,0,0,0.55), 0 0 0 1px rgba(0,0,0,0.4)` | `0 6px 18px rgba(28,34,44,0.22), 0 0 0 1px rgba(28,34,44,0.10)` | under popovers and floating windows only |
| `--thumb-bg` | `#292e36` | same | the tile behind a thumbnail (the renders' own background, 41, 46, 54) |
| `--thumb-flag` | `#e8a33d` | same | a favorite's bookmark on a tile |
| `--thumb-badge` | `#f0666a` | same | the "No model" badge on a tile |
| `--thumb-badge-text` | `#15171a` | same | its text |
| `--syn-text` | `var(--text)` | `var(--text)` | script editor, plain text |
| `--syn-comment` | `#6a9955` | `#008000` | script editor |
| `--syn-directive` | `#c586c0` | `#800080` | script editor |
| `--syn-keyword` | `#569cd6` | `#0000c8` | script editor |
| `--syn-number` | `#ce9178` | `#a31515` | script editor |
| `--syn-string` | `#ce9178` | `#a31515` | script editor |
| `--syn-constant` | `#4fc1ff` | `#0070c1` | script editor |
| `--syn-error` | `#ff0000` | `#ff0000` | script editor |
| `--mdl-plain` | `#d4d4d4` | `#202020` | viewer's ASCII model |
| `--mdl-structure` | `#c586c0` | `#af00db` | viewer's ASCII model |
| `--mdl-keyword` | `#9cdcfe` | `#001080` | viewer's ASCII model |
| `--mdl-keyed` | `#4ec9b0` | `#267f99` | viewer's ASCII model |
| `--mdl-node-type` | `#dcdcaa` | `#795e26` | viewer's ASCII model |
| `--mdl-number` | `#b5cea8` | `#098658` | viewer's ASCII model |
| `--mdl-comment` | `#6a9955` | `#008000` | viewer's ASCII model |

Notes on the table:

- Text on `--accent-fill` is `--on-fill` (a selected row's count is `--on-fill` at 85%).
- The `--syn-*` values are the script editor's present palette from Options (Script Editor page), the `--mdl-*` values the viewer's present ASCII colors (`editor.rs`): the dark values are today's, kept; the light values are VS Code-style equivalents chosen for contrast on `--inset`. The apps keep the user's own Options colors where they set them.
- The `--thumb-*` tokens have one value in both themes, because the thumbnails' renders keep their dark background. They are the tokens added after the first token sheet (`SPEC.md` section 5 lists 21 colors; the stylesheet has the rest, and the list above is the stylesheet's).
- The 3D view's picture, the gizmo colors (section 11) and the game's renders are the same in both themes; only chrome follows the theme.

### 4.2 Fixed colors that need a token before the light theme can exist

The stylesheet still has dark hex values written into components. Each needs a token (or a derived value) in M1; the light value is what the Light Theme boards use.

| Where it is used | Dark value today | Light value on the boards |
|---|---|---|
| Toggle on and disabled; checkbox checked and disabled (fill and border) | `#22364a` | `#c4d4e5`; its tick or icon `#7189a3` |
| Document tab and dock tab, hovered | `#2b2f35` | `#dee1e6` |
| Table cell borders (rows, and columns in a lined table) | `#25292e` | `#d3d7dc` |
| Table cell, selected (with a 1 px `--accent` inset line) | `#1a2a3a` | `#d3e2f2` |
| Text on the colored status badges (error, warn, ok, info) | `#15171a` | `#ffffff` |
| Text on the selection tag in a 3D view | `#15171a` | `#15171a` (7.44:1 on the light `--select`) |
| Scroll thumb | `--raised-hover` | `#b3b9c2` |
| Scroll thumb, hovered | `#555c67` | `#8d949e` |
| Code view, current line's hairlines (top and bottom) | `#262a30` | `#dce1e8` |
| Code view, selection | `#264f78` | `#add6ff` |
| Color swatch, the line around the inner color | `rgba(0,0,0,0.35)` | `rgba(0,0,0,0.25)` |

Five components borrow a token whose role does not carry over to light. Each wants its own token:

| Component | Dark (borrowed) | Light on the boards |
|---|---|---|
| Key cap | background `--raised`, border `--line-soft`, text `--text` | background `--inset`, border `--line-soft`, text `--text-2` |
| Tooltip | background `--base`, border `--line-soft` | background `--inset` |
| Switch (track and knob) | track `--raised-hover` off, `--accent-fill` on; knob `--text-2`, `--on-fill` when on | track `#a9b0b9` off, `--accent-fill` on; knob `#ffffff` |
| Slider thumb | `--text-2` | `#ffffff` with a 1 px `#8d949e` ring |
| The modified dot (document tab, Status Bar) | `--select`, no ring | `--select` with a 1 px `#a35c00` ring (the light `--select` is only 1.75:1 on the bar) |

The compass and the minimap (overlays on a 3D view) carried hex attributes in their markup; the light board gives them classes (`cp-*` and `mm-*`) that read tokens: in light the minimap grid is `--bar`, its view rectangle `rgba(23,88,150,0.12)` filled with an `--accent` stroke, its camera mark `--accent`, its selection mark `#a35c00`; the compass ring is `--overlay` with a `--line-soft` stroke, ticks `--text-3`, north `--error`, south `--text-3`, labels `--text`.

### 4.3 Type

| Use | Face | Size / line | Weight |
|---|---|---|---|
| Everything (body, labels, buttons, tabs, menus) | Ubuntu | 13 px / 16 px | 300 (Light) |
| Section headings (`widgets::section_heading`), with a 1 px `--line-soft` rule under them | Ubuntu | 15 px / 20 px | 700 (Bold), nothing else is bold |
| Badges, key caps, the Status Bar's weak text, the small detail text | Ubuntu | 11 px / 14 px | 300 |
| Gallery cell name (section 8.4) | Ubuntu | 12 px / 15 px | 300 |
| Code, ASCII models, resource names, readout chips | Hack, then DejaVu Sans Mono, Ubuntu Mono, monospace | 13 px (chips 11 px) | regular |

No italics except where the app has them today (a linked conversation line). Ubuntu Light stays (ruling 7 of 2026-11-10); it is to be judged in the running application (M2, section 15).

### 4.4 Metrics (px)

| Metric | Value |
|---|---|
| Top bar height | 30 |
| Document tab bar | 28 |
| Editor header | 30 |
| Dock tab bar | 26 |
| Tree and list rows | 22 (`--row-h`) |
| Controls | 22 high, padding 0 8 (`--ctl-h`) |
| Icon | 16 drawn, in a 22 square button |
| Tool shelf | 36 wide, 28 buttons |
| Status bar | 24 |
| Corner radius | 3 (popover 4; gallery tiles 4, the small variant 3) |
| Gaps | on a 4 px step: 4, 8, 12 |
| Panel padding | 8 |
| Form grid | column gap 12, row gap 6; label column `--lbl-w` 104 |
| Tree indent | 14 (`--indent`) |
| Default dock sizes at 1600 by 900 | left 260, right 320, bottom 180 |
| Lines | 1 px; stroke of icons 1.5 on a 24 by 24 grid |

Contrast the boards were checked against (light | dark, minimum): text on panel 14.49 | 13.17; text on bar 13.05 | 11.77; text-2 on panel 8.97 | 7.41; text-3 on panel 6.28 | 5.10 and on bar 5.66 | 4.55; accent, warn, ok and info above 4.5 on panel and bar in both themes; error on panel 5.26 light and 5.24 dark, on bar 4.74 light and 4.68 dark (with the raised dark `#f0666a`; the check file's dark column still shows the old `#e5484d` at 4.13 and 3.69, which is why the token was raised); `--on-fill` on `--accent-fill` 6.07 | 7.85; white on the colored badges at least 4.5 in light. `--text-off` is about 2.2 to 2.8 on purpose (disabled). The script editor's and the viewer's own palettes are measured on `--inset` and are the apps' own. M1 turns these ratios into assertions.

## 5. The shell

```
+--------------------------------------------------------------------------+
| Top Bar: menus | quick actions |      Quick Open       | build actions   |
+----------+---------------------------------------------+-----------------+
| Left     | document tabs                               | Right Dock      |
| Dock     +---------------------------------------------+ (active editor's|
| Module / | editor header (one row)                     |  panels, stacked|
| Resources+--+------------------------------------------+  or tabbed)     |
|----------|  |                                          |                 |
| Outliner |ts| editor body                              |                 |
|          |  |                                          |                 |
|          +--+------------------------------------------+                 |
|          | Bottom Dock (only when a panel is switched on)                |
+----------+---------------------------------------------+-----------------+
| Status Bar: panel switches | hints as key caps | counts, job, module     |
+--------------------------------------------------------------------------+
```

(`ts` is the tool shelf.) The six regions:

| Region | What it holds |
|---|---|
| **Top Bar** (one row; today's menu bar and toolbar merged) | The menus as text; icon buttons for New, Open, Save, Undo, Redo; in the middle the **Quick Open** field ("Go to resource or command", Ctrl+P); on the right the build actions as icon and word (Compile, Verify, Test Module (F9) with Choose Character under its arrow). Today's other toolbar buttons move: Properties and New Area to the Module panel, Start to the area editor (the camera button under the compass; also the View menu), Resources to the Left Dock's tab, Palettes and Preview to the Right Dock. |
| **Left Dock** | Tabs **Module** (filter field, the tree with a kind icon and a count per group, Expand All, Collapse All) and **Resources** (today's Resources window). Under it, split, the **Outliner** (what the active editor holds; for an area the placed objects by kind with a show or hide eye per kind; in the viewer it is today's Nodes panel). |
| **Editor Area** | Document tabs with a kind icon, the name, a modified dot (`--select`) and a close button on hover; the active tab is `--panel` with a 2 px `--accent` line on top. It can be split into **editor groups** side by side. An editor is a header row (one row, 30 px), an optional **tool shelf** (a vertical strip of the editor's modes or tools at the left of the body, 36 px) and the body. |
| **Right Dock** | The active editor's panels, stacked (each with its own small tab bar) or tabbed. |
| **Bottom Dock** | Closed by default; opened by the panel switches in the Status Bar; resizable. The Log is here (closed, its last line shows in the Status Bar). |
| **Status Bar** | Left: the Bottom Dock's panel switches (a word, and a count badge where it has one: Log, Problems, Search Results). Middle: the active tool's hints as key caps (`Drag` Move, `Shift` + `Right Drag` Turn...). Right: the editor's counts (for an area "11 by 11 tiles", "192 objects"), a running job, the module name with "(modified)", the game folder. |

### 5.1 Open editors: docked or floating

The Options switch **Open editors** has two values, **As Tabs** and **As Floating Windows**.

- **Default**: As Tabs on a new installation; existing settings keep As Floating Windows, so no builder finds the window changed after an update (ruling 1 of 2026-11-10, second set).
- Areas always open as tabs. A kind of editor you have moved opens where you last put one, until View › Reset Layout.
- A floating editor is a window over the main window (1040 by 640 for a script) with a border, radius and shadow, and a bar row: collapse arrow, tabs with ×, Maximize, Close. It keeps its panels inside its own window (the script's Reference as a pane at its right, Compiler, Bookmarks and Search Results as a bottom pane). The docks and the Status Bar follow the active docked editor. Double-click and right-click on the empty bar and the tab menu (Eject, Close, Rename…, Show in Module Tree, Close Others, Close Tabs to the Right) are unchanged; a resize grip sits at the bottom right.
- The three settings that matter for floating windows stay under the switch: Open Properties as tabs of one window, Closing a window cancels its changes, Move windows by dragging anywhere. The "Toolbar buttons" setting is retired.
- Found on the way: today a kind of editor moved to a window is never stored (a window is the default, so the app only forgets the place). With As Tabs as the default the app must remember "this kind goes to a window" (section 13).

### 5.2 How the shell yields at small widths

W is the window width, E the editor width. Measured on the board "Area Editor at 1280×800".

| Below | What changes |
|---|---|
| W 1300 | The build actions (Compile, Verify, Test Module) lose their words; Quick Open is capped at 300 px (an icon button when it would fall under 240). |
| W 1500 | The Left Dock folds to a 24 px strip of its panels' icons (the user can unfold it; the chevron replaces the 16 px pane edge). |
| W 1400 | The Right Dock goes from 320 to 280 px and its panels become tabs of one panel (the palette's type strip becomes a combo under 300 px of panel width). The Status Bar's switches become icons, then the game folder drops, then hints drop whole from the end (at 1280 all six hints still fit). |
| E 990 | The area header goes compact: the ten kind toggles become a Show popover with a count ("5 of 10"), Day and Night one moon toggle, and Lighting, Overlays, Audio and Area Properties icon buttons. At 1280 the editor is 974 wide. |
| E 550 | Snap and Turn lose their value text. |
| E 490 | Together and Onto Objects move into More. |

### 5.3 Wide screens

At 3440 by 1440 the docks stand two columns wide (the Right Dock: Palette and Properties; the viewer: Inspector and Log in column 1, Lighting and Effects in column 2), the Left Dock is 320 wide with taller lists, the editor area splits into groups (a script in a second group; the viewer's 3D View, Texture and ASCII in three), the Bottom Dock can be open, and the header's popovers unfold into the row: in the area editor Day, Night, Lighting, Shadows, Fog, Grid, Minimap, Walkmesh and Sound Ranges carry their words, Fade Geometry is a combo, and Animations, Grass, Tile Border, Shaders and the three Audio switches are icon toggles. A creature blueprint shows all 14 pages at once in five columns (Basic, Statistics, Appearance, Classes and Skills head the columns, the other pages stack under them; the page list keeps the real order and marks the pages in view).

## 6. Components

The stylesheet defines each once; M1 builds them once in `mg-uikit`. The catalog, one line each (states in brackets):

- **Top bar**, **menu title** (rest, hover, open) and **menu item** (icon slot, keys, submenu, hover, disabled) in a popover.
- **Icon button** (22 px, transparent until hovered; hover, filled, disabled; the tooltip is its accessible name) and **word button** (hover, primary, flat, disabled, open; an optional leading icon, a chevron for a popover or menu).
- **Toggle** (off raised, on accent fill, hover, disabled; an icon or a word) and **joined group** (buttons or toggles with 1 px seams, Blender's way).
- **Segmented control** (one child on; icons or words).
- **Combo box** (lead icon, value, chevron; wide, grow, open, disabled, mixed).
- **Text field** (focus, read-only, error, disabled, mono), **search field**, **field with a glued button** (the locstring "…"), **number field with unit** (right-aligned), **multi-line text**.
- **Checkbox** and **radio** (disabled, mixed), **switch** (a popover's master on/off), **slider** and **slider with number**.
- **Color swatch button**.
- **Document tab** (active, hover, modified dot, close), **dock tab** (on, hover, title).
- **Panel header** and **panel**; **page rail** (a panel's pages as icons at its left edge).
- **Tree row** (indent levels, folder arrow, kind icon, name, weak second word, count, eye; selected, hovered, dim) and **list row**.
- **Table** (header row, cells, row headers, lined; selected row, selected cell).
- **Section heading**, **field label**, **form grid** (label/value, full-width rows, columns of 440 to 560 px), **weak note**, text levels.
- **Key cap**, **badge** (accent, error, warn, ok, info, tag; colored ones take dark text in dark, white in light), **modified dot**.
- **Popover** (with a title and optionally a switch, sub headings, checkbox rows, separators; below or below-right), **tooltip** (with key caps), **splitter** (a hovered or dragged seam; idle seams are the 1 px region gaps), **scroll bar** (drawn).
- **Tool shelf** (36 px, 28 px buttons; on, hover, disabled; separators).
- **Viewport with overlays**: compass, minimap, selection outline and name tag, view buttons, a label, in the four corners.
- **Status bar** (switches, separators, last log line, hints, right-hand items, a running job with a bar).
- **Code view** (line numbers, current line, error line, selection, caret; the script editor's and the viewer's present palettes).
- **Empty state**.
- **Gallery cell** (Quiet Tiles, section 8.4).

### 6.1 Rules that apply across components

- **Page list versus header switch**: see section 2 (pages).
- **Tool options in the header's left group**: while Paint or Select Tiles is the active tool, its options (Scatter, Row, Fill, Erase segmented control, Radius and Density numbers, a Brush popover for the rest) take the header's left group; Together and Onto Objects return when Select is the active tool; the view controls stay at the right.
- **Dialogs** keep the code's order (main action first, Cancel last), right-aligned in a footer row with a rule above it, the main action in the accent fill; a setting or a count sits at the footer's left; the window title is left-aligned in a bar row; no dimming behind a dialog (today's do not dim); wizards get a step list at the left with a check on done steps and a page-header bar over each page.
- **OK and Cancel**: every tab that has them today keeps them, at the right end of its header. Properties in the Properties panel apply at once and are undone with Undo; a blueprint's tab keeps OK and Cancel for now (rulings 2 of 2026-11-10 and 10 of the second set).
- **Undo and Redo**: the Top Bar's act on the active editor and no editor repeats them. An editor keeps its own Save where it has one today (a script, a table, a hak, a talk table, a tileset); the Top Bar's Save saves the module.
- **Quick Open and the Command Palette are one feature**: a field in the Top Bar, Ctrl+P, finding resources and commands; Ctrl+Shift+P stays (Help menu). Today the palette finds commands only.
- **A header row never overflows.** The owner found the Hak editor's header drawn past the editor's right edge on its board (2026-11-11): a fault of the mock, and the rule for the code. A header is laid out against the width it has: when its controls do not fit, the left group's word buttons lose their words (an icon with a tooltip), then move into the More popover, secondary commands before Save; OK and Cancel never yield and are never clipped or drawn over a neighbor. The shared header widget does this itself, and each editor has a test that narrows it to its smallest width and finds every control either on the row or in More.
- **Tooltips**: one short sentence, no semicolons, on every icon (style guide).

## 7. Icons

- One set of 24 by 24 stroke icons (stroke 1.5, round caps and joins, no fill), drawn at 16 px, a tooltip on every icon. The set has 192 names, in these groups (the boards' sheet is on the Foundations page), and two more added in M2, both Lucide's shapes (its `minimize` and `folder-output`, under the ISC license the set's README names): `restore` (four corners turned in, a maximized window's Restore) and `extract-all` (a folder with an arrow out, the hak editor's Extract All…).

| Group | Names |
|---|---|
| Object kinds and resources | creature, door, encounter, item, placeable, sound, store, trigger, waypoint, start, area, conversation, script, journal, faction, table, talk-table, nui, model, texture, tile, tileset, light, prefab, module, hak, plugin, dialog |
| Model nodes | node-dummy, node-mesh, node-skin, node-light, node-emitter |
| File and edit | new, open, save, undo, redo, cut, copy, paste, delete, find, search, replace, rename, edit, print, export, import, link, bookmark, open-in-tab, to-scratch, save-as, external-editor |
| Code | definition, references, used-by, decompile, compile-view, ascii |
| View | eye, eye-off, grid, layers, overlays, sun, moon, fog, shadow, lightbulb, magnet, rotate, camera, compass, frame, zoom, wireframe, axes, walkmesh, speaker, music, ambient, play, pause, stop, loop, restart, minimap, sound-ranges, grass, tile-border, shaders, fade, animations |
| Area tools | select, select-tiles, paint, terrain, together, onto-objects, area-properties, brush, eraser, fill, scatter, raise, lower |
| Build | compile, hammer, verify, check-circle, test, play-circle, error, warning, info, check, update, sync |
| Structure | chevron-right, chevron-down, chevron-up, chevron-left, close, plus, minus, more-horizontal, more-vertical, filter, folder, folder-open, file, settings, panel-left, panel-right, panel-bottom, split, maximize, pin, lock, unlock, expand-all, collapse-all, list, gallery, properties, palette, outliner, inspector, timeline, log, problems, tag, user, comment, sword, shield, sparkles, book, flag, home, help, command, window, grip, arrow-up, arrow-down, bolt, dice, column, row, sort, wizard, key, globe, swatch, colors |
| Blueprint pages | inventory, trap, advanced, variables, visuals, description, statistics, appearance (plus info, lock and script above); creature pages class, skill, feat, spell, ability |

Aliases (same drawing): `search` = `find`, `edit` = `rename`, `hammer` = `compile`, `check-circle` = `verify`, `play-circle` = `test`, `lightbulb` = `light`, `folder-open` = `open`.

- **Kinds that must tell apart at 16 px**: creature, door, encounter, item, placeable, sound, store (merchant), trigger, waypoint, start location, area, conversation, script, journal, faction, 2DA table, talk table, NUI window, model, texture, tile. Check each at 16 px beside its neighbors.
- **Ruling 3 of 2026-11-10**: an open icon font for the common icons, the object kinds drawn ourselves. The spec allows "an icon font or a small set of drawn shapes"; M1 chooses (a drawn set from the one stroke vocabulary keeps the look uniform and needs no font file).
- **The sources**: the icons were SVG path data (192 names) in the mock folder; since M1 they are `crates/mg-uikit/icons/icons.json` (194 names after M2), the one source, made into strokes at build time.
- A missing icon is added to the set (24 by 24, stroke only, no fill or style attributes), checked at 16 px, and named in the change.

## 8. Components in use

### 8.1 Header row, popovers, tool shelf

The header never wraps. Left group: what you do; a spacer; right group: how the view looks. A **More** button at the end holds what does not fit. Popovers hang from their button (below, or below-right so that they do not cover the view's top-right overlays) and nothing up to the editor group may clip them. A popover has a title (with a master switch where there is one), sub headings, checkbox rows and separators.

### 8.2 Status Bar hints

Key caps with a word each, ordered by importance; hints drop whole from the end when there is no room. Examples from the boards: `Drag` Move; `Shift` + `Right Drag` Turn; Enter OK, Tab Next field, Shift+Tab Previous field; Click, Double Click, Ctrl+Z, Ctrl+Shift+Z (journal); Click, Ctrl + Click, Shift + Click, double-click (hak, resources); Up, Down, Page, Home, End, Enter, Esc (resources); `Double-click` Apply effect and `Ctrl + Wheel` Zoom texture (viewer); Ctrl+N, Ctrl+O, Ctrl+P (welcome); Paint: Drag Paint, Shift + Wheel Brush size, Q / E Turn, Right Click / Esc Stop; Tiles: Click, Drag Box, Shift + Click, Ctrl + Click, Right Click. The gizmo hints are in section 11.

### 8.3 Tree and well

A long tree group can scroll within itself (the Areas list in the Module panel; ruling 10 of 2026-11-10). Picture grids in dialogs sit in a well with a scroll bar. Panels scroll with a drawn bar.

### 8.4 The gallery cell ("Quiet Tiles")

Chosen by the owner on 2026-11-10 from the comparison board "Gallery Cell · Polish Options" (column A), after saying that the plain framing and labels read as unpolished. It is one shared component for every picture grid of the suite: the palettes' Gallery mode, the Tiles palette, the Appearance Gallery, Tile Properties' variants, the pickers (portraits, textures), and the viewer's galleries.

**Anatomy.**

- The picture is a **tile**: radius 4, a 1 px `--line` rim drawn inside it over the picture (so the picture never moves), on `--thumb-bg`. Nothing is drawn around a whole cell; no plate behind it in any state.
- Under it, 6 px lower: the **name**, 12/15 px `--text-2`, centered. An optional **detail** (a challenge rating, a row number) on its own 11/14 px `--text-3` line, at the foot of the row so that the details of a row line up. The name is the thing's own name only: `Wraith` and `CR 5`, never `Wraith (CR 5)`. Placeables and appearances have no detail line.
- **Grid**: cells grow to fill the row, as many as fit at the slider's side; column gap 10, row gap 14.
- **States** change only the rim and the name: hover - rim `--raised-hover`, name `--text`; selected - 2 px `--accent` rim, name `--text`.
- A picture still being made shows the **kind's icon** dimmed (24 px, `--text-off`, centered), with no "…". "No picture" adds a small caption under the icon. "No model" is a small badge at the tile's top left (`--thumb-badge`, text `--thumb-badge-text`). A **favorite** is a small bookmark mark, 12 px filled `--thumb-flag`, at the tile's top right (it replaces the "★" before the name).
- A model or resource name (tile models, `nw_door_…`) is set in Hack 11 px.
- Pictures with no name (portraits) use the same rim and ring with radius 3.
- Kind icons: placeable, creature, door, item, waypoint, tile, texture, model, user (portraits).

**Size.** One slider for every gallery, range **64 to 220**, default **128** (today's), shared between galleries; the Variant grid of Tile Properties keeps its own slider (default 84, range 64 to 224) and the small variant (tile radius 3, name 11/14 px, 3 px under the tile, 6 px apart).

**The name rule** (ruled 2026-11-10: "You may have large names break into ellipses"): a name never breaks inside a word. A name of one word is one line, cut with an ellipsis. A name of several words wraps between words on up to two lines and ends in an ellipsis after the second. A model name is one word.

**Justified rows** (ruled 2026-11-10 on the portrait picker: no gap between the last picture of a row and the scroll bar): cells of a fixed size (portraits, 54 by 84) use as many columns as fit at the smallest gap (8), and the room left over is shared out between them; a last row that is not full keeps the same columns. Cells that grow (the galleries) already fill their row. The Appearance Gallery keeps seven columns whatever the width.

**Thumbnails** are real renders made by the app's own thumbnail path (224 px, the game's shaders, the viewer's light, background about RGB 41, 46, 54); the boards' 80 renders were made with `mgv`.

### 8.5 Pickers

Select Portrait, Select Texture, the armor part gallery and the Creature Wizard's grids use the shared cell. Select Portrait and Select Texture stay dialogs (the dialog pattern; the count at the footer's left; a close button; the grid in a well with a scroll bar). Select Portrait keeps its three radio choices (Characters and Creatures, Placeable Objects and Doors, Plot Characters), the Race and Gender or Category dropdowns, a 128 pt preview and "N portraits"; Select Texture keeps "To draw NAME with", Find, the size slider, "N of M textures", Interface textures too, and a 200 pt preview. The armor part gallery stays on the item's Appearance page. The Creature Wizard's Appearance and Portrait step carries the grids with the step list.

## 9. Per editor

Each subsection names the canvas board (page · board) and says what the redesign does. The row-by-row mapping is [ui-redesign-moves.md](ui-redesign-moves.md).

### 9.1 Main window and area editor

Boards: Toolset · Area Editor (beside Today · Area Editor); Ultrawide · Build Layout; States and Dialogs · Area Editor at 1280×800, Paint Active, Tiles Selected, Area Wizard Open; Light Theme · Area Editor.

- **Header** (one row): the ten show toggles (Creatures, Doors, Encounters, Items, Placeables, Sounds, Stores, Triggers, Waypoints, Start) as icons only (ruling 5), with All and None on the arrow at the end of the group, repeated as eyes in the Outliner; Snap and Turn combos (magnet and rotate icons); Together and Onto Objects icon toggles; Day | Night; popovers **Lighting** (Lighting, Shadows and their kinds and quality, Fog, Fade Geometry, Shaders, Grass, Tile Border, Animations), **Overlays** (Grid, Minimap, Ground walkmesh, Object walkmeshes, Sound Ranges, with a master switch) and **Audio** (Sounds, Ambient, Music); **Area Properties** at the right end; **More** (To Scratch, Save as Prefab…). The right-click menu of the strip becomes the header's right-click menu.
- **Tool shelf** at the left of the 3D view: Select Tiles (F10), Paint (B), and the new Select Objects and Terrain buttons.
- **Overlays on the view**: the compass; under it Reorient Camera and Go to Start Location; the minimap.
- **Docks**: Left Dock Module / Resources and the Outliner; Right Dock Palette over Properties (a placed object's properties, its pages as an icon rail, with Open in Tab); the Log in the Bottom Dock; counts ("11 by 11 tiles, 192 objects") and hints in the Status Bar.
- **Palette**: type picker, Standard and Custom, search, List and Gallery, New…, More, Group By, in the same order. In Gallery mode it wants a wider Right Dock (440 px shows three a row); at that width the header takes its compact form. The Brush section appears while painting ("2 chosen", × to take a blueprint out of the brush).
- **Paint** (board Paint Active): see section 6.1; Count All, Spacing, By Size, Flow, Turn, Scale, Height, Lock in the Brush popover, Gap in Row and Fill modes, Snap and Turn copied into the popover under "Placing, as with the Select tool"; the show toggles are the Outliner's eyes. The ring still draws while Paint is on.
- **Tiles** (board Tiles Selected): Refine Tile (T) icon, joined ‹ Appearance › control, Rotate (R), Copy, Cut, Paste, Delete (the Eraser (X) stays in the palette as a brush), a Tile Properties toggle (also the lower Right Dock panel: lights, loops, Height, Replacement texture, Defaults, Textures; no OK or Cancel: the step is one undo step and Cancel becomes Ctrl+Z); Tile Border and Grid among the view controls; the Tiles palette's search, List and Gallery and Exact Tile; the tile menu (right click) keeps Put Down and plugin tile commands.
- **Handles** over the view: section 11.

### 9.2 Blueprint editors (creature first)

Boards: Toolset · Creature Blueprint (beside Today); Ultrawide · Creature, Pages as Columns. The other blueprint kinds, Module Properties and Area Properties follow the creature blueprint and are not mocked.

- The floating window becomes a document tab whose header carries the name, the resref, OK and Cancel (meaning unchanged).
- The wrapped row of 14 page tabs becomes a **page list** at the left in the same order; the pages follow one another on one scroll with their forms in columns.
- Sections on pages that have none today: Identity (First name, Last name with its languages button and Random name, Tag), Race and Appearance (Race, Appearance with Gallery…, Phenotype, Gender), Palette (Challenge rating, Category), Portrait (the picture, its combo, Portraits…), Conversation (the picker, Edit, No interrupt), Description (full width). The strref is drawn inside a name field.
- The preview window becomes the **Preview panel** in the Right Dock (info line, Animation, Pause, Frame, Lights, Shadow, Shaders, Export Files…) in one header row over the 3D view; the Appearance page's own preview is the same panel.

### 9.3 Script editor

Board: Toolset · Script Editor (beside Today); Light Theme · Script Editor.

- Header: Save and Compile with icon and word; Save As…, Print…, Open…, External Editor, Bookmark, To Scratch (More), Definition, References, Rename Symbol…, Used By as icons; Find… and Replace… in their own group; the codepage label at the right end; "modified" becomes the dot on the document tab; the live error line becomes a Status Bar message and a wavy underline on the line.
- Right Dock: **Reference** (Functions, Variables, Constants, Templates and the filter), **Help** under it (laid out as prose with a row per parameter).
- Bottom Dock tabs: Compiler, Bookmarks, Search Results, with switches in the Status Bar. New: a line and column readout.

### 9.4 Conversation editor

Board: Toolset · Conversation Editor (beside Today).

- Header: Add (with its word), Copy, Cut, Paste, Paste As Link, Delete; move up and down; Expand All and Collapse All; highlight toggles for Comments, Actions, Quests, Animations, Sounds; the Scripts word toggle; Export and its formats, Import Lines…; Test at the right end (also in the Node panel header).
- Right Dock **Node** panel: speaker tag, Text with its languages button and Token…, Text Appears When…, Actions Taken (script, Script Wizard…, parameters, preview), Other Actions (animation, sound, journal), Comments, the Bookmark button; the Data and Search tabs of today's bottom pane become Node panel tabs. A **Conversation** panel under Node holds the file's own settings (Normal end, Aborted, Stop camera zoom in, word count).
- Bookmarks are in the Outliner under the Module panel; indent guides are new. Row context menu, drag to move and Ctrl+drag to link are unchanged.

### 9.5 Options

Board: Toolset · Options, the Open Editors Switch.

- Ten pages (Folders, Area, General, Script Editor, Conversation Editor, Sounds, Verify, Language, Keyboard, Updates) in a page list with icons. OK and Cancel at the header's right end. Status Bar hints: Enter OK, Tab Next field, Shift+Tab Previous field (no Esc: docked it does nothing).
- The General page in three columns in the code's order: (1) Editors, Saving and Exporting, Opening Modules, Testing; (2) Windows and Tabs, Module Tree and Palettes, Appearance; (3) Questions, Warnings, Troubleshooting. The **Open editors** switch heads the Editors section with a note (section 5.1); a **Light or Dark** theme setting is added in M2.

### 9.6 2DA table editor

Board: More Editors · 2DA Table Editor. Header: Save, Add Row with its count, Empty Row, Delete Row, Add Column…, Find, Go to, "N cells are wrong", Values as names and Strings as text (icon toggles, right), More (Save As File…, Take Rows…, Compact, Show Hidden Columns), OK and Cancel. Right Dock: **Cell** panel (value, list or flags choice, kind words, the "Described by …" note) and **References** panel (Find References to Row N). Left Dock Outliner of the columns (kind icon, kind words, show or hide eye). Status Bar: rows, columns, where it is kept, "6 cells" selected, key hints. The column right-click menu stays on the header.

### 9.7 Talk table editor

Board: More Editors · Talk Table Editor. Header: Find, Go to, Add Line, Find References, Remove Last Line, Save, Only lines with text (toggle), Export menu (CSV, JSON), More (Import CSV…, Import JSON…, Open File…, New File…, Module's Table), Close (read-only) or OK and Cancel. A header row on the list. Right Dock: **String** panel (the line editor) and **Talk Table** panel (the info line, and the page for a module with no talk table). `dialog.tlk` opens read-only under its own StrRefs. The draggable splitter is gone.

### 9.8 NUI designer

Board: More Editors · NUI Designer. Design group with a page switch (Design, Bindings, Events & Variants, Images); source group (a second editor group) with its own switch (JUI source, Binding JSON) and header buttons (Opener Script, Event Script…); Build & compile as the primary icon button; Export…; More (Import…, How to open it…); a View popover (Editor, Preview only, Split source, UI scale, State, Resources, Clean view, Screen); zoom (Fit, 100%, "Game preview") as overlays in the canvas corners; Interact and Reset in the header; "Not built" and "Build failed" in the Status Bar in the warning color; the Checks switch (with a count) in the Status Bar and the Checks panel in the Bottom Dock. Right Dock: Components palette over the Properties panel (binding, Content, Size & layout, Behavior, Events, Element actions) over the Bindings panel (Create bind, Window identity, Client delivery). Layers with search, Expand All and Collapse All are the Outliner. Text in title case by the style guide.

### 9.9 Tileset editor

Board: More Editors · Tileset Editor. Header: Save (the tileset file), the five pages as a switch, Make Palette, Check, Render Minimap Pictures, OK and Cancel. A bar above the tile list (Find, Duplicate Tile, Add Tile, Remove Last Tile); the list is a table (#, Model, Corners, Edges); the form at the right (Model, Walkmesh, Minimap picture, Path node, Orientation, flags, corners, edges), the doors grid with Add Door and Remove Last Door under it. Right Dock: **Preview** (with a button to open the model in a tab, a diagram and the door sentence) and **Textures**. Check's findings go to the Problems panel in the Bottom Dock. Outliner: General, Terrains, Crossers, rules, Tiles, Groups, with counts. Path line and counts in the Status Bar.

### 9.10 Hak editor

Board: More Editors · Hak Editor. A notice strip under the header for "This hak is part of the game…". Header: Add Files…, Add Folder…, Remove, Extract…, Extract All…, Save, Save As…, Find, Compile Models, OK, Cancel. Sortable columns Name, Size, Source (the Type column and a sort arrow are new). Details panel (File, Description, Built from with Update from Folder, Contents "2706 resources, 183.8 MB"), Preview panel (inline preview, Play or Stop, model facts: nodes, faces, vertices, textures, animations, supermodel), Outliner of the contents by type with counts.

### 9.11 Resources

Board: More Editors · Resources and a Read-Only Resource. The Resources tab of the Left Dock: Type dropdown, Name filter, Overridden copies at the top; Refresh and Export N as Files… as icons in the panel header; rows with a kind icon, name, layer, indented overridden copies; the count in the panel and the double-click hint in the Status Bar. A read-only resource opens as a tab titled "name (read-only)" with Copy to Module and Save As… in the header and a note; its GFF view has three columns, read-only, with the names beside the numbers as `mg gff --names` gives them.

### 9.12 Journal editor

Board: More Editors · Journal Editor. Header: Add, Copy, Cut, Paste, Delete, Expand All, Collapse All, OK, Cancel. Tree rows (an End badge marks an entry that finishes). Right Dock: **Entry** panel (ID, Finish Category, Text, language; a subtitle on its tab) and **Category** panel (Name, Tag, Priority, XP, Comments; shows the parent while an entry is selected).

### 9.13 Faction editor

Board: More Editors · Faction Editor. A Basic | Advanced segmented control in the header; the Advanced matrix fills the editor with frozen row headers; Add Faction…, Remove Faction, Change Name… as header word buttons; OK and Cancel. Left Dock Outliner is the factions list. Right Dock: **Faction** (Global Effect, a read-only Parent field) and **Reactions** (Full Detail, and for a selected cell a slider, the sentence and a Hostile, Neutral, Friendly scale). Cell tooltips carry the attitude words. The Add Faction and Change Faction Name windows stay modal.

### 9.14 Welcome

Board: States and Dialogs · Welcome. A page heading with the version; a **Start** section (New Module…, Open Module…, Open Folder… with icons and keys); **Recent Modules** (the full path as a weak second line); a **Status** section ("Game data found" or the warning, Options…, the update bar with What's New…, Install Now, Later); a "Without a module" group (Open Hak…, New Hak, Open Tileset…, New Tileset…, 2DA File…, Talk Table, Resource Browser, Appearance Gallery…); links to the manual's chapters; a Help group (Command Palette…, Check for Updates…, About); a note about dropping a module on the window. Build actions, Save, Undo and Redo are drawn disabled. The "No area open" page becomes a centered empty state with Area Wizard… and Go to Start.

### 9.15 Wizards and dialogs

Board: States and Dialogs · Dialogs and Wizards, and Area Wizard Open. The dialog pattern is in section 6.1. Drawn: Rename, Unsaved Changes, Duplicate Tags, Area Wizard, Resize Area, Recover Unsaved Work, Find and Replace Text ("Replace in N" in the footer), Delete from Module ("Don't ask…" in the footer, at the left), Build Module (Counted as Used stays under the results), Adjust Location and Build Module's missing resources flowed into columns, the Creature Wizard (nine steps). Wording fixes: "6 strings, 5 times" in place of "string(s)" and "time(s)", sentence case for "animation speed" and "X rotation". Which dialogs become panels is in section 12.

### 9.16 Galleries

Boards: Galleries page (Area Editor · Palette Gallery; Tiles Gallery and Variants; Toolset · Appearance Gallery; Toolset at 3440×1440 · Galleries; Picture Pickers; Gallery Cell · Polish Options).

- **Palette Gallery** keeps the palette's header and categories (headings with counts; the pictures wrap inside each heading); the size slider shows with Gallery on; the hover enlargement (ResRef, Tag, Challenge rating) becomes a tooltip beside the dock.
- **Tiles palette Gallery**: the tile's model name under each picture; counts on terrain and crosser headings.
- **Tile Properties** (Right Dock, lower panel): All tiles (ignore fit) and Variant (N fit here) with their own Size slider; the Turn choices and search appear with All tiles on (not drawn).
- **Appearance Gallery** opens as a **tab** (it floats with As Floating Windows): the kind switch (Placeables | Creatures | Doors), Find, the slider and the count in its header, the hint line as a strip under it; a Preview panel and an Appearance panel (the chosen row's facts) beside it. Click, double click, drag and right-click Give to the Selection are unchanged, and the common ones are Status Bar hints.
- Pickers: section 8.5.

### 9.17 Moonglow Viewer

Boards: Viewer · Model with ASCII Editor (beside Today · Viewer); Creature, Colors, Texture, Effects; Viewer at 3440×1440; Resources as a Gallery (New); Gallery Tab at 3440×1440 (New).

- **Shell**: the same Top Bar (menus File, Model, View, Help and every item unchanged; Open…, Save, Decompile, Compile, Compile and View also as icon buttons). Left Dock: Resources on top (search, type filter, count, list; "838 creatures" on its own row for creatures), **Outliner** below (today's Nodes panel; a creature's real node tree). Right Dock: Inspector and Lighting stacked. Bottom Dock: **Timeline** (today's Animation panel: one transport row over a full-width scrub bar, with a seconds ruler under it showing the animation's length), Effects and Log as tabs.
- **3D View header**: Frame, the View combo, Grid and Axes icon toggles, an **Overlays** popover (Walkmesh, Wireframe, Normals, Nodes, Shadow casters; unfolded into word toggles at 3440), Shaders, Shadow, Model lights and a background color swatch.
- **Renames** (ruled): Nodes to **Outliner**, Animation to **Timeline**, and the View menu's Ground Grid, Axes, Model Lights, Game Shaders, Shadow to **Grid, Axes, Model lights, Game shaders, Shadow** to match the toggles. Each rename lands with its change to the viewer's manual.
- **Texture tab**: shows the open model's texture beside the model, in a second editor group (channel switch, Mip and Zoom, a caption line with name and info; Ctrl + Wheel zooms). **ASCII editor**: its state text, Save, Compile, Compile and View in the editor header of the second group; the " *" becomes the tab's dot; the Diagnostics panel becomes a strip under the text with a title row and the error and warning counts.
- **Creature**: the Inspector's Creature section (Gender, Phenotype, Skin, Hair, Tattoos, Head, Body parts, Wings, Tail) in a two-column grid; color pickers as a swatch and combo that opens a 16 by 11 grid; PLT colors (ten layers, Reset Colors) in two columns; a Model section. Effects: search, size, Clear, rows; double-click applies.
- **Gallery mode** (NEW, asked for by the owner as a mock): the Resources panel gets a List | Gallery switch, the shared slider and cell, and the gallery can open as a tab. The viewer's window has no picture grid today.
- Status Bar: document, origin and game folder at the right, the 3D view's mouse and key actions as key caps. The viewer has no manipulators (camera drags only); it does not get the gizmos.

## 10. The rest of the "Where Things Moved" detail

The control-by-control rows, the New lists and the Not Drawn lists of every screen are [ui-redesign-moves.md](ui-redesign-moves.md). It is the checklist M3 and M4 work from.

## 11. Gizmos: the area view's handles

Asked for by the owner on 2026-11-11, before implementation: "go over gizmos, as in the rotate disc and the arrows which appear under SHIFT, more if the placeable selected is dynamic". The code never says "gizmo"; it says rings, arrows, handles. Boards: Gizmos · Look Options (three looks over real renders of the view) and Gizmos · States and Measures (the chosen look through every state).

**Ruled 2026-11-11**: the owner chose treatment A, **Fine Lines** ("We may go with A") and accepted the five picks on open behavior points ("I have no objections to your recommendations to the points raised"). Who gets which handle, sizes, hit reach, keys and behavior stay today's; the look changes. The handles are 2D egui painting over the 3D picture (`area_view.rs`, `overlays`), with no hiding behind geometry (a recorded decision, `deferred.md`); they exist when Options' "Show the turning and tilt rings around selected objects" is on (the default) and the area is not read-only; locked objects and hidden kinds get none; at most 32 selected objects get a set each.

### 11.1 Who gets which handle

| Kind | Turning ring | X, Y arrows | Up arrow | Tilt rings | Scale |
|---|---|---|---|---|---|
| Placeable, static | yes | yes | yes | no | no |
| Placeable, dynamic | yes | yes | yes | yes | yes |
| Door, Item | yes | yes | yes | yes | yes |
| Creature | yes | yes | no | yes | yes |
| Waypoint, Store | yes | yes | yes | no | no |
| Sound | no | yes | yes | no | no |
| Trigger, Encounter | yes (about the outline's middle) | yes | no | no | no |

The turning ring shows with no key held; the arrows, tilt rings and scale square with Shift alone. A dynamic placeable gets tilt rings and the scale square, a static one does not (Make Dynamic and Make Static in the context menu switch between them; Adjust Location edits the same fields by number). Only an arrow's head is a target, not its shaft. A press on a handle never selects what is behind it. Escape cancels a drag; the middle button still swings the camera during one. With several objects selected each has its own set and one drag leads them all (turning: each about itself, or about their middle with Together on). Snapping is the header's Snap (grid) and Turn (angle).

### 11.2 Behavior kept from today (what each handle does)

| Handle | Radius and reach | Fields it writes, undo text | Readout while dragged |
|---|---|---|---|
| Turning ring | one radius r for every handle of an object: the eye's distance × 0.0765 (× the view's width/height where it is narrower than tall), about a tenth of the view's height, constant on screen; hit reach 11.5 px from a line | `Bearing` (doors, placeables), `XOrientation`/`YOrientation` (others), outline points; "Rotate" | `182°` at the facing mark |
| Tilt rings (red about the model's X, green about its Y), in the model's frame, centered on the visual transform's translate point | r; 11.5 px | `VisTransformList` `RotateY` (red) / `RotateZ` (green); "Tilt" | red `Y Rotation 12°`, green `Z Rotation 12°` |
| Axis arrows (east X red, north Y green, up Z blue) | shaft to 1.2 r, head 1.2 r to 1.7 r; only the head is a target | position X/Y ("Move", height kept, grid snap) or Z ("Raise", no snap); while one is dragged only it is drawn | `X 28.00 m` at the tip |
| Scale handle (stalk up and to the camera's right, 1.25 r) | 8 px from the square's center; follows the pointer while dragged | `ScaleX/Y/Z` alike; "Scale" | `Scale 1.25` at the pointer |

Also in the vocabulary: the **encounter spawn point** (a post and a 1 m ground arrow in the selection's color; a round tip turns it, readout `90°`, "Turn Spawn Point"; a square at the post's foot moves it, "Move Spawn Point"); the **start location** (a blue ring (70,130,255) 3 px, 1 m radius, with a red arrow (235,40,40); selected, a round handle at the arrow's tip turns it); the **selection box**. Without handles: a plain drag moves over the ground, Alt + drag raises, Alt + right drag turns, Q / E turn by the snap angle (Shift: 90 degrees), G drops to the ground, Ctrl + wheel scales.

### 11.3 The chosen look, A "Fine Lines", as built

Measures (screen px at 1x):

| Part | Measure |
|---|---|
| Line with its edge | the color's line over an under-stroke 2 px wider, black at 55% |
| Ring | 1.5 px over 3.5 px (lit 2.5 over 4.5), 96 segments |
| Far half of a tilt ring (behind the pivot, seen from the eye) | drawn at 40%, edge included |
| Axis shaft | 1 px over 3 px (not a target) |
| Arrow head | a solid triangle 12 by 8 px (lit 14 by 10), 1 px dark edge; head line 1.5 px over 3.5 px (lit 2.5 over 4.5), 1.2 r to 1.7 r; the up arrow has the same head |
| Facing mark of the turning ring | a small solid triangle on the ring pointing outward (8 by 9, lit 10 by 11), edged as a head; no spoke |
| Scale square | 9 px (lit 11), radius 1, 1 px dark edge, reach 8 px from its center; stalk 1 px at 75%, 1.25 r up and to the camera's right |
| Pivot dot | radius 2, 1 px edge, `#eef0f3`, at the object's position |
| Selection box | the model's 12 edges, 1 px `#ffb02e` at 90% over a 3 px dark edge at 45% (the handles lead, and the amber ring does not merge with it; at 1.5 px it did) |
| Others of a set while one handle is hovered or held | 50% strength |
| Lit | thicker and brighter; state is never told by color alone |

Colors (not theme tokens: the 3D picture is the same in both themes):

| Handle | Idle | Lit (hover, held) | Use |
|---|---|---|---|
| X arrow, red ring | `#f0566a` | `#ff9aa6` | east; the ring about the model's X |
| Y arrow, green ring | `#86d13a` | `#c4f08c` | north; the ring about the model's Y |
| Z arrow | `#4d9bff` | `#a6ccff` | up |
| Turning ring | `#ffb02e` | `#ffd48a` | the selection's own `--select` |
| Scale square | `#eef0f3` | `#ffffff` | neutral |
| Dark edge | `#000000` at 55% | | under every line, head and square |

### 11.4 Drag feedback (NEW)

- Turning: a filled wedge (the ring's color at 18%) from the facing at the press to the facing now, a hollow facing triangle where the turn began, ticks on the ring at the Turn snap's steps when one is set (a 5 px line out from the ring at each step, with the edge). Chip: `182°`.
- Tilting: the same wedge in the ring's plane. Chip: `Y Rotation 12°`.
- Sliding: a guide line along the axis through the object across the view (axis color at 35%, 1 px, no edge, clipped at the near plane), a hollow dot where the object began. Chip: `X 28.00 m`.
- Scaling: a hollow square where the handle began (1.25 px stroke). Chip: `Scale 1.25`.
- Start marks at 80 to 90%.

### 11.5 Readout chips (NEW, theme-aware)

The value in Hack 11 px on a chip 18 px high, radius 3: dark theme `#0c0d0f` at 86%; light theme `#e8ebef` at 92% with a 1 px rim; a 2 px strip at the chip's left in the handle's color (amber for turning, red or green for a ring or arrow, blue for up, near-white for scale). Its bottom-left corner sits 10 right and 10 up of the dot (rings), 10 right and 4 up of the tip (arrows), 12 right and 12 up of the pointer (scale). Texts are today's with the capital the style guide asks for (`Scale 1.25`, not `scale 1.25`).

### 11.6 Status Bar hints (NEW, theme-aware)

| State | Hints, in order |
|---|---|
| Object selected, no key held | ring glyph Turn (drag the ring); `Drag` Move; `Shift` Handles; `Alt` + `Drag` Raise; `Q` `E` Turn by a step; `G` Drop to ground. Counts at the right: "4 by 4 tiles", "1 selected", the module with "(modified)". |
| Shift held (the Shift cap shows held) | glyphs for Move along an axis, Tilt, Scale; `Right Drag` Turn view |
| Sliding (an arrow held) | `Esc` Cancel; `Middle Drag` Turn view; the snap as a magnet icon with "Snap 0.5 m" |
| Turning (the ring held) | `Esc` Cancel; `Middle Drag` Turn view; the snap as a rotate icon with "Turn 15°" |

### 11.7 How each is painted (egui painter calls)

| Part | Call |
|---|---|
| Line with its edge | `Shape::line` twice: black at 55%, 2 px wider, then the color (one `PathShape` per ring half, so alpha never doubles at a join) |
| Arrow head | `convex_polygon` of the three points, filled black at 55% with a 2 px black stroke, then filled in the color |
| Far half of a ring | split at the points level with the pivot as seen from the eye; that half's color and edge at 40% |
| Facing triangle | a convex polygon on the ring, pointing out, edged as a head; hollow (a closed 1 px path) where a turn began |
| Pivot dot | `circle_filled` radius 3 black at 55%, then radius 2 in `#eef0f3` |
| Scale square | `rect_filled` 11 px radius 2 black at 55%, then 9 px radius 1; hollow: `rect_stroke` 1.25 px |
| Wedge | a triangle fan from the pivot over the ring's points between the two angles, the color at 18% (convex under 180°) |
| Snap ticks | a 5 px line out from the ring at each step, with the edge |
| Guide line | `line_segment` 1 px, the axis color at 35%, no edge, clipped at the near plane |
| Chip | `rect_filled` radius 3 at 86%, a 2 px strip in the handle's color at its left, Hack 11 px |

A ring is a true projected circle (the camera rebuilt from the view's own matrices), not an affine map through quarter points (6 px off at the half-turn). For a tilt ring the half nearer the eye is the front.

The spawn point and the start location get the same edge, the lit color and the size increase; the start location keeps its blue ring and red arrow and its round handle turns it.

### 11.8 The five rulings

1. Readouts keep the game's field names: the red ring reads "Y Rotation", the green "Z Rotation" (as Adjust Location has them) beside arrows that read X, Y, Z by color; the manual explains them.
2. Several objects selected: a full set on each, as today.
3. Shift stays a held key; a pinned toggle can come later.
4. A name chip beside a handle when the pointer is on it, before the drag (NEW; for example `Move East or West`, `Scale`).
5. The grab cursor over every handle (NEW; today only the start location changes the cursor).

Also NEW with the look: the drag feedback, the readout chips, the Status Bar hints for the three states, and the thinner selection box.

## 12. Decided but not drawn, and not mocked

**Decided, but no board draws it:**

- The dialogs that become dock panels (none of the new panels is drawn). Bottom Dock: Find Instance (a results list searched again and again; a Search Results tab), the hits of the module-wide Find and Replace Text (the form as the panel's header), Build Module's results (as Problems, the options as a panel), and the read-only reports (Hak Pak Conflict Analysis, Resources Used, Claims Table, Areas Not Yours). Right Dock of the area editor: Edit Areas Together (a Properties-style panel), Light Maker, Bake Walkmesh to Transform. A tab: Plugins. Staying dialogs: wizards, confirmations, Rename, Adjust Location, the pickers, NWSync publish.
- Floating editors: the tab menu (Eject, Close, Rename…, Show in Module Tree, Close Others, Close Tabs to the Right) unchanged.
- 2DA editor: Talk Table, Choose String…, New Line…, Choose Model… and the row chooser in the Cell panel for those column kinds; the read-only resource view with Copy to Module and Save As….
- Talk table editor: the Language dropdown (nasher projects only), which would sit in the Talk Table panel.
- Tiles: variant pictures (today only for a single selected tile); Tile Properties' Turn choices and search with All tiles on.
- Welcome: a "last opened" time on Recent Modules (the settings keep only paths); the Left Dock (today only with a module open).
- Hak editor: the row's right-click menu and the 2 GiB warning. Resources: Models as text (only when a model is listed), the row context menu, the Edit menu for 2DAs. Journal: the "Select a category or an entry" message and the Root or no-selection state. Factions: the Add Faction and Change Faction Name windows and the Basic chart view. Dialogs: Find Instance, Plugins, NWSync, Bake and Light maker; steps for the Area Wizard (one page today); a dimmed backdrop (today's do not dim). Viewer: TXI text; a list of animations beside the transport (the Timeline has only the combo).

**Not mocked:**

- The other blueprint kinds, Module Properties and Area Properties (they follow the creature blueprint), the other nine Options pages, the manual tab, the Plugins window.
- The viewer and the remaining editors in the light theme (the light tokens cover them).
- Drawn to show a state and not read from anywhere: the five bookmarks in the conversation's Outliner; the update version and the order of the recent modules on the Welcome page; the two modules in the Recover Unsaved Work dialog; the grids on the gallery boards (real rows in their real order with rows between left out and a "…" cell wherever there was no picture); the armor part pictures (whole armors standing in for single parts); the two color sections shown together in the viewer's Inspector (today a creature shows one and a lone model the other); the Text input added to the NUI example so that the Checks panel has a finding; the Custom area lighting on the wide viewer board over a picture rendered with Studio lighting; the word "empty" in the creature's inventory slots; the Casting value in its Spells table; the skin, hair and tattoo color swatches; the spelling of the script editor's codepage label. The spawn point's and the start location's handle drawings (no render of these exists).
- The 3D pictures are cut from the screenshots in the repositories, so they are soft where a board enlarges them. The canvas page itself and its phone view could not be opened from the session that made it.

## 13. Found on the way (probable bugs in today's app)

These were seen while reading the code and the screens for the redesign. They are not fixed by this work unless a milestone says so; each belongs on the [deferred list](deferred.md) or in its milestone.

1. Today the turning ring comes back during a scale drag but not during an arrow or tilt drag.
2. The conversation editor reports "0 letters, 0 words" and an empty Text box for a line whose text comes from the talk table. Probably a bug.
3. The Welcome page's recent list puts a "/" after anything that is not a `.mod`, so `Chapter1.nwm` shows as "Chapter1.nwm/".
4. In Options, Esc cancels only when Options is a floating window. Docked as a tab, Esc does nothing (the redesign's hints omit Esc there).
5. With As Tabs as the default, the application has to remember "this kind of editor goes to a window". Today it only forgets the place, because a window is the default.
6. The dark error red was a little under the contrast floor as text; it is now `#f0666a` in the token sheet (fixed in the design).
7. The viewer's window has no picture grid, and a long name under a small picture breaks mid-word today (an ellipsis reads better; fixed by the name rule).
8. The red tilt ring's readout says "Y Rotation" while the red arrow's says "X" (kept by ruling 1 of the gizmo rulings, with the manual explaining it).

## 14. New features the redesign adds

Everything marked NEW in the sources, as one list. Nothing else is added without being said.

**Shell**

- Quick Open in the Top Bar: go to a resource or a command by typing (the Command Palette extended to resources; Ctrl+P, Ctrl+Shift+P kept).
- The Outliner in the toolset: what the active editor holds (an area's placed objects by kind with an eye per kind; a hak's contents by type; a 2DA's columns; a tileset's General, Terrains, Crossers, rules, Tiles, Groups; the factions; the conversation's Bookmarks).
- Editor groups: two or more editors side by side in the Editor Area, and a button to split it.
- A More button at the end of an editor header for what does not fit.
- The icon strip of a folded Left Dock and its "5 of 10" count.
- Light theme and a Light or Dark setting.
- Open editors switch (As Tabs | As Floating Windows) with a note on where editors open.

**Area editor**

- Select Objects and Terrain as tool shelf buttons beside Select Tiles and Paint; a compass; Day as a named state beside Night; a master switch on Overlays; the page rail in the Properties panel; an arrow on Test Module for Choose Character.
- The palette's Brush section (× to take a blueprint out of the brush; today only Ctrl+click); Snap and Turn copies in the Brush popover; a note in the popover on which options each mode has.
- The heading "Lights and Loops" and the "6 tiles" subtitle in Tile Properties; the tileset name and counts on the Tiles palette folders; the status count "3 by 2 tiles selected"; the key cap X beside Eraser.
- Gizmos: drag feedback (wedge, snap ticks, guide line, start marks), readout chips, Status Bar hints for selected, Shift held and mid-drag, a name chip on hover, the grab cursor over every handle, a thinner selection box.

**Blueprints and editors**

- Section headings on the pages that have none today (Identity, Race and Appearance, Palette and the like); the strref drawn inside a name field.
- Script editor: a line and column readout; Help laid out as prose with a row per parameter.
- Conversation editor: indent guides, Bookmarks in the Outliner, the file's own settings as a Conversation panel under Node.
- Options: the page icons in the page list; the General page bar, as on Creature.
- 2DA editor: the Outliner of columns with kind icons and a show or hide eye; the selection count ("6 cells") in the Status Bar.
- Talk table editor: a header row on the list; `dialog.tlk` opened read-only and numbered by its own StrRefs; the Talk Table panel as a home for the no-table page.
- NUI designer: the View popover and the zoom overlay; the source as a second editor group with its own page switch; a warning mark on the Outliner row, IDs and bind names as weak text; a bind chip on a bound property; Bindings as a table.
- Tileset editor: the Preview diagram and the door sentence under it.
- Hak editor: a Type column and a sort arrow; model facts in Preview.
- Resources: names beside numbers in the GFF view; both tabs open in the same shell.
- Journal: the Category panel shows the parent while an entry is selected; a subtitle on the Entry panel tab.
- Faction editor: a selectable cell with a Reaction panel (slider, sentence and the Hostile, Neutral, Friendly scale); a read-only Parent field (today only in Add Faction); frozen row headers and the hint "Select a reaction".
- Welcome: a "Without a module" group; links to the manual's chapters and a Help group; a "Game data found" line; the drop-a-module note; Status Bar hints for Ctrl+N, Ctrl+O and Ctrl+P.
- Dialogs: the step list, the page header bar, the footer row, the accent fill on the main button; wording fixes ("6 strings, 5 times").

**Galleries and viewer**

- A Preview panel and an Appearance panel beside the Appearance Gallery; counts on the Tiles gallery's terrain and crosser headings; a well with a scroll bar and a close button on the two picker dialogs.
- Viewer: a seconds ruler under the Timeline's scrub bar with the animation's length; a title row on Diagnostics; the status hints "Double-click: Apply effect" and "Ctrl + Wheel: Zoom texture"; the texture caption line; Gallery mode for the Resources panel.

## 15. The owner's rulings, in order

### 15.1 2026-11-10, the first ten

1. Docked or floating editors: a switch in Options, not one or the other.
2. A change in the Properties panel applies at once (Undo); a blueprint's tab keeps OK and Cancel for now.
3. Icons: an open icon font for the common ones, the object kinds drawn.
4. Keep the new features (Quick Open, Outliner, editor groups, the smaller ones).
5. Kind toggles as icons only in the area header: fine.
6. Viewer renames (Nodes to Outliner, Animation to Timeline, View menu names matching the toggles): fine; the manuals are updated with each rename.
7. Ubuntu Light stays; judge it in the running app.
8. Rows and controls 22 px: fine.
9. One shared crate for theme, icons, shell and common widgets: yes.
10. The Areas list scrolls inside its own group: fine.

### 15.2 2026-11-10, the second ten ("use your picks")

Ten points were open after the second round (the default of Open editors; four settings the redesign touches; the Top Bar's Save, Undo and Redo against an editor's own; OK and Cancel on the hak and NUI tabs; which dialogs become dock panels; the viewer's Texture tab beside a model; dialog.tlk read-only). The owner said to use my picks. These are they:

1. Open editors defaults to As Tabs on a new installation; existing settings keep As Floating Windows.
2. The "Toolbar buttons" setting is retired; the three floating-window settings stay, under the switch.
3. Quick Open and the Command Palette are one feature (Ctrl+P, and Ctrl+Shift+P kept), finding resources and commands.
4. Dialog buttons keep today's order, right-aligned, main action accented.
5. Become Bottom Dock panels: Find Instance, module-wide Find and Replace Text's hits, Build Module's results, the read-only reports. Become Right Dock panels of the area editor: Edit Areas Together, Light Maker, Bake Walkmesh. Plugins becomes a tab. The rest stay dialogs. Not drawn yet.
6. The viewer's Texture tab shows the open model's texture beside it.
7. `dialog.tlk` opens read-only under its own string numbers.
8. NUI designer text in title case, by the style guide.
9. The Top Bar's Undo and Redo act on the active editor and no editor repeats them; an editor keeps its own Save where it has one today; the Top Bar's Save saves the module. (Narrower than first proposed: taking Save away from scripts and tables would have changed how saving them works.)
10. Every tab that has OK and Cancel today keeps them, at the header's right end.

### 15.3 2026-11-10, the gallery views

The owner asked for the gallery views after the second round. Rulings:

- One cell everywhere and one size slider (64 to 220); the palette's Gallery wants a wider Right Dock; the Appearance Gallery opens as a tab; tile variants stay in Tile Properties; the pickers stay dialogs.
- The picture-name ruling: "You may have large names break into ellipses" (section 8.4, the name rule).
- The portrait picker: no gap between the last picture of a row and the scroll bar (justified rows).
- The gallery cell is Quiet Tiles (treatment A of the Polish Options board).
- The viewer's Resources panel gets a Gallery mode (asked for as a mock).

### 15.4 2026-11-11

- **Gizmos**: A, Fine Lines ("We may go with A"); the five points as picked ("I have no objections to your recommendations to the points raised"): section 11.8.
- **"You may begin implementation."** Work started the same day on the branch `ui-redesign`.
- **Commits and merging**: "You may commit on that branch as tasks are completed, and merge into develop once all green." Read as: a commit for each finished and reviewed task, a merge into `develop` where a milestone leaves the application whole with the tests, clippy and rustfmt clean. No push and no release were named: those are asked for.

### 15.5 Where the sources disagreed

- Rule 1 first said "everything docked"; the ruling of 2026-11-10 replaced it (docked or floating by a switch).
- `SPEC.md` section 12 lists ten points as open for the owner; section 13 settles them all. Section 13 wins.
- `SPEC.md` section 6 introduces Quick Open as new (Ctrl+P); section 12 says it is the existing Command Palette (Ctrl+Shift+P) extended; section 13 makes them one feature. One feature wins.
- `GIZMOS.md` section 3 describes the readout chip in 11 px Ubuntu Mono and a 2 px yellow selection box; its section 6 (as built, 2026-11-11) has the chip in Hack 11 px and the box at 1 px over a 3 px dark edge, the under-stroke the line's width plus 2 px, the stalk at 75%, lit heads 2 px larger and the lit square 11 px. The as-built figures are used above.
- `SPEC.md` section 5 says dark is the only theme mocked; section 12 gives the light value of every token (the Light Theme page). The light values are not in `moonglow.css`, which holds the dark theme only. The Light Theme page wins.
- The ratio check file (`ratios.txt`) prints the old dark error red; the token sheet and this document use `#f0666a`.

## 16. Implementation plan

Planned, not done. Each milestone is planned in detail when it starts (the files, the tests, the order of the changes); what follows is the scope and the exit. Every milestone lands with its **tests** (at the lowest tier that catches a regression: unit, UI with `egui_kittest`, `screens.rs` pictures), its **pictures compared with the mock boards** (rendered through `crates/mg-ui/tests/screens.rs` and looked at, before and after, as the project's rule asks), **frame time measured before and after** (`frame_perf.rs`, the 144 fps budget at 3440 by 1440), and **the manual in the same change** (`docs/manual/`, the built-in help; the viewer's manual with each viewer rename). Work is deferred, and filed in [deferred.md](deferred.md), only for a reason; the finish-what-was-asked rule of the repository's `CLAUDE.md` stands.

### M1. The shared crate `mg-uikit`

**Done 2026-11-11.** `crates/mg-uikit` depends on egui (and serde) alone. It holds `Tokens::DARK` and `Tokens::LIGHT` (67 colors, the fixed colors of section 4.2 among them), the metrics, `Theme` with `apply`, the fonts, the 192 icons turned into strokes at build time from `icons/icons.json` (painted by egui's painter, with icon, toggle and icon-and-words buttons whose accessible name is their words), and the form helpers `field_label`, `section_heading`, `table_heading` and the swatches, which `mg-ui` re-exports under their old paths. Nothing in the application calls `apply` yet, so nothing looks different. Its tests assert the contrast of every text pair in both themes and draw a kit sheet in both (`cargo test -p mg-uikit --test kit -- --ignored`).

What egui's style cannot express, which M2 answers with widgets of the crate's own where a screen shows it: one border setting shared by buttons and fields; a text field's height (20 px, not 22) and its focus border; the check box and radio (no accent fill when on); the slider's rail and thumb; chevrons for the combo box and the tree (egui draws triangles); the hover color of list and menu rows; the tooltip's own fill; a disabled control fading whole; every heading bold once the theme is applied. Key cap, badge and chip helpers come with the milestones that first use them.

As planned:

A new crate under `crates/`, below `mg-ui` in the layering (and the viewer's `mgv-ui` later depends on it by tag).

- The tokens, dark and light (section 4.1), each as a named constant or a field of a `Theme`; the fixed colors of section 4.2 made tokens (so the light theme is complete); the metrics (section 4.4); the type (section 4.3).
- `apply`: sets egui's visuals and style from a theme (surface levels, strokes, rounding 3, spacing on the 4 px step, row and control heights, selection and hover colors), once per theme change.
- Fonts: Ubuntu Light, Ubuntu Bold and Hack, the apps' own (no new typefaces).
- The drawn icon set: the 192 icons as vector shapes (24 by 24, stroke 1.5) painted by egui's painter at 16 px, with a tooltip helper; the icon sources are being brought into the crate's folder by the code work.
- The generic form helpers moved out of `mg-ui` (`field_label`, `section_heading`, the form grid, key cap, badge and the like), so both apps share them.
- Tests: token tables checked against the contrast figures of section 4.4 (the ratios become assertions), a snapshot of the foundations board (colors, type, controls in their states, the icon sheet), a test that every icon name draws something at 16 px.
- Exit: `mg-ui` builds against the crate with no look change yet; the foundations picture matches the Foundations board and the Light Theme · Tokens and Controls board.

### M2. The toolset adopts the look on today's layout

Can ship alone.

**First half done 2026-11-11: the theme on today's layout.** The application applies `mg_uikit::apply` every frame in the choice of Options › General › Light theme (the setting, its stored name and its place kept; it changes on OK, as Interface size beside it does), so every window and test harness wears the tokens, type and 22 px rows and controls in both themes. What egui's style could not say is now said: `mg-uikit` gained a check box and a radio of its own (`Checkbox`, `RadioButton`, `ui.check_box`, `ui.radio_choice`, `ui.radio_button`: on in `accent-fill` with a white mark, off a bordered well, the same ids, keys and accessible names as egui's, egui's own where no theme is applied), chevrons for combo boxes and trees (`combo_icon`, `collapsing_icon`), the slider (`slider`), the 22 px text field (`text_field`), menu and list row styles (`menu_style`, `menu_bar_style`, `list_rows`: rows hovered in `accent-fill` and `hover-row`, a row as wide at rest as hovered), and a pass at the end of each frame that turns the focused field's border `accent` and gives tooltips `tooltip-bg`. The headings' text style is the body face (window titles and `ui.heading` are not bold; dialog titles are in the body size); `section_heading` stays the one bold. The dock is styled from the tokens (`mg-ui`'s `dock_look`: tab bars `bar` 28 px high, the tab in front `panel` with a 2 px line along its top, `accent` in the focused pane, 1 px `line` between tabs). Chrome colors written into `mg-ui` take tokens; the colors kept (3D view overlays, the game's look, data, tests) are listed with their reasons in `mg-ui/src/chrome_colors.rs`, whose test holds the list. The toolbars that cut their last buttons wrap. `mg nui render` and `mg nui layout` draw byte for byte as before. The sliders are egui's drawn in the theme (`mg_uikit::slider`: an `inset` rail, the filled part `accent-fill`, a thumb ringed in `text-2`; egui fills the rail and the resting thumb alike, so the thumb at rest is the rail's color). The drawn icons came in a third round (below).

**Second round, 2026-11-11: rows, read-only forms, frames.** Rows are the mock's `--row-h`: every tree, list of rows and table (the module tree, the conversation, palette, journal and category trees, the Options' pages, every editor's and dialog's lists, the 2DA grid and its row numbers, the variables and other editable tables, the combo boxes' lists) lays its rows 22 px apart with no gap (a table of fields 23, below), their fills square and drawn on the row itself so that a selected row and its hovered neighbor meet with neither overlap nor seam. One mechanism does it: `mg_uikit::row_style` (no gap, no expansion and so no border, square corners, `hover-row`), given with `list_rows` (the rest of a `Ui`), `with_rows` (a scope, the gap under it kept) or `show_rows` (egui's `show_rows`, which reads the gap outside the rows). A table whose rows are fields (the variables, a creature's skills, the GFF view, a tileset's rules) leaves a 1 px line of the panel between its rows (`table_rows`, `metrics::TABLE_RULE`; the mock's `.table td` draws its border inside its 22 px, which would take a point off each field: here the line is added, 23 px a row): a text field in a row is its well, borderless, ringed in `accent` with the keys, and the wells ran together without it. Forms keep a 22 px control and the 4 px gap. A window opens as much taller as its lines grew: `tabs::FORMS_TALLER` (26 over 21) for one that is mostly a form, `tabs::ROWS_TALLER` (8%) for one that is mostly rows (its toolbars and the few form lines under its rows grew, its rows only a point each). A form shown only to be looked at is `mg_uikit::read_only`: no fill, a quiet border, `text-2`, nothing lit under the pointer, and the kit's check box, radio button and slider quiet too (`text-3`, never the accent). A field with a ground of its own takes a frame of its own (`mg_uikit::code_frame`: the script editor, no border with the keys or without). Menus keep their own rows: 22 px with 4 between (26 a row, the mock's `.menu-row` is 24 with none between), left for M3's menus. The manual's screenshots under `docs/images/` are not replaced now: they change again with the shell milestone (M3) and are replaced once then.

**Third round, 2026-11-11: the drawn icons.** No button, toggle, menu row or mark of the toolset carries an emoji or a symbol character any more: each draws one of the set's icons (`mg_uikit::Icon`), 16 px in 1.5 px strokes on the pixel grid, and is named by its words alone (`get_by_label("Save")`, a screen reader's "Save"), with its icon or without. The widget is `mg_uikit::IconButton` (the icon and the words, the icon alone as a 22 px `.btn-icon`, or the words alone; small, frameless, selected, a toggle's look, filled; disabled it fades whole), with `menu` (a menu's button, or in a menu a submenu row with a drawn chevron, egui's `⏵` glyph gone from every submenu), `submenu_button`, `menu_button`, `icon_label` (a warning's or a note's mark before its words), `name_group` (a row named as a group, so that its buttons are found within it: the main toolbar's "Toolbar", the area view's "Area toolbar", the palette's "Palette types") and `record_icons`/`icons_drawn` (which icon a frame drew where, for tests). A word button's icon sits 2 px into its padding with 6 px before the words (the mock's `.btn .ic`). `mg-ui`'s `icons.rs` holds the toolset's icons by meaning (`object(kind)` the kind icons, the same in the area view's filters and the palette's types; `editor::*` the editors' toolbars'), and `Tools` labels a toolbar as Options › General › Toolbar buttons says: the icon and the words, or with Icons only the icon alone, its words in its tip and its name; the editors' toolbars keep their words alone beside it. Converted: the main toolbar, the area view's filters, switches, tools and its three dropdowns' leading icons, the script, conversation, model and NUI toolbars, the palette's types, List and Gallery, Expand All and Collapse All, the tiles palette's brushes, Select Tiles and Exact Tile, the module tree's area arrows and claim marks (check, lock, pin), the journal's and the NUI layers' arrows, a favorite's bookmark (before its name in the list, at a picture's top right in the gallery), every search field's glass (drawn in the field) and clear button, the number fields' arrows, the remove, move and add buttons of forms, the equipment's worn-off warning, the window bar's Maximize and Restore (now drawn where a window's bar lies over the main toolbar too), the View menu's and plugin panels' ticks, the script editor's bookmarks in its line numbers and its live error's mark, the renderer and update notices. What stays text (signs in data and messages: `≠` for values that differ, `×` in sizes and counts, `↺90°` a turn's value) is listed with its reasons in `mg-ui/src/interface_glyphs.rs`, whose test reads every string and character literal of the application and holds the list exactly, and another that the app's fonts draw each: the proportional fonts have no `→` (it showed as a square), so the messages they draw write `->`: the 2DA editor's merge lines and their tip, Refine Tile's brush, the import window's missing references, the attach-hak window's placements (a tip) and the NUI Creator's bind cycles; the messages only the log shows, in its monospace font, which has the arrow, keep `→` (a verify finding's reference, an export's missing references, a terrain renamed in its tileset's palette), each listed with the font that draws it. The scan reads the sources of every crate whose strings the application can show (all of `crates/` but the two that are tests alone, and `apps/moonglow`), skipping test modules alone and reading byte strings too. The hak editor's header no longer runs past its window (the owner's finding on the mock and in the app): its path and its hint line are cut short with an ellipsis instead of widening the editor, and its buttons show their icons alone when their words do not fit the editor's width (`Tools::fitted`, section 6.1's first step), wrapping where even those do not; a test holds every button inside the window at 640 and 480 px. The fonts are unchanged: the emoji and symbol fonts are egui's `default_fonts`, which text the user writes or reads (a conversation line, a talk-table string, a script comment) and egui's own widgets (the color picker's copy button) still need. The plugin API names no icons and is left as it is: a plugin's titles are text, emoji included. Known differences: the toolbars' buttons keep 4 px between icon and words, not the mock's 6, and even so the area view's toolbar takes a row more at some widths (its forty buttons a point or two wider each than with their glyphs), until M3 moves its switches into popovers.

**Frame times, 2026-11-11.** Measured on a quiet machine against `develop` before the redesign (`frame_perf` and `editor_perf`, each alone, 318 views and modes in both): the median view costs 0.04 ms more on the CPU (1.03 times), of which 0.03 ms is tessellation, with 1.45 times the vertices (the drawn icons' strokes and the controls' borders). The largest increase is 0.57 ms (the largest area with everything selected and its walkmesh shown, 5.41 to 5.98 ms), and no view crosses the 6.94 ms budget in either direction: the four over it are the largest area's overview and turning under the wheel, as before. The script editor's times are the same. The hand-run timing tests had not been run after the controls were renamed, and one stopped at the tree's Clear Filter: their clicks now fail the test where a control is not found, where five of them had left a view untimed without a word.

- Theme and a **Light or Dark** setting in Options; chrome colors from tokens everywhere (panels, bars, buttons, inputs, tabs, trees, tables, scroll bars, popovers).
- Drawn icons in place of emoji and Unicode glyphs; the tests that look for the emoji by name migrate to the controls' words (their accessible names), and assert the icon drawn where the glyph was their subject.
- Field labels bright, section headings bold with a rule (already the rule), 22 px rows and controls.
- This answers ruling 7 (Ubuntu Light judged in the running app): after M2 the owner looks at it in the real application, and the weight is changed only if the owner says so.
- Exit: every screen of today's layout wears the theme in both modes; frame time within the budget; the manual's screenshots and the parity inventory updated where they show the old look.

### M3. The shell

- Top Bar (menus, New, Open, Save, Undo, Redo, Quick Open, Compile, Verify, Test Module) replacing the menu bar and toolbar row; the "Toolbar buttons" setting retired at M4 pending ruling R4 (the Top Bar ignores it from M3, because the area and editor toolbars read it until M4).
- Left Dock: Module, Resources and the Outliner; Right Dock; Bottom Dock; Status Bar with panel switches, hints as key caps and counts (M3 has the Log's switch; Problems and Search Results arrive with their panels in M4).
- Document tabs and editor groups; the **Open editors** switch (As Tabs by default for a new installation, existing settings keep As Floating Windows) and the stored "this kind goes to a window".
- Quick Open (the Command Palette extended to resources); the shell yielding at the measured widths (section 5.2); the wide layout (section 5.3).
- Exit: today's editors open in the new shell, docked or floating, without yet changing their insides; the Where Things Moved rows for the main window are checked off.
- Known differences left by the theme pass (M2), which the shell settles:
  - Window titles are centered by egui; the mock's are at the left.
  - The main toolbar wraps to a second row under about 1000 px wide (the Top Bar replaces it).
  - Remembered window sizes keep the heights they were kept at before the theme (a new window opens at its new size).
  - Wizard dialogs cover part of the Welcome page's subtitle.
  - Menu rows are 26 px apart (22 and a 4 px gap); the mock's are 24 with none between.
- An option for later, not work to do now: the drawn icons' meshes could be cached by icon, size and color instead of tessellated each frame. egui draws a stroke's round ends and small dots from discs pre-drawn in the font atlas, whose texture coordinates move when the atlas grows (a new glyph or size), so a cache would need the atlas's size in its key, or draw those dots as plain circles. T15 decides it by measure.

**Planned in detail, 2026-11-11.**

M3 as scoped is too big to merge as one change (about 16 days of work, with roughly 300 test lines to migrate). It holds as one milestone cut into three merge points: M3a (the new frame), M3b (editors in the shell) and M3c (widths, Outliner, hints). That makes 16 tasks, T0 to T15. Five facts frame the plan:

- **Where the shell lives.** The side docks are egui panels with the kit's own tab bars. The editor area stays the one egui_dock `DockArea` it is today. This is a hard constraint of egui_dock 0.21 (Part 2).
- **Settings.** `Settings::default()` keeps today's floating windows, so existing settings and the roughly 1,100 UI tests keep working. Only a new installation (no settings file) gets As Tabs and the Bottom Dock closed.
- **The parity file.** `docs/parity/aurora-ui-inventory.md` is generated from Aurora's executable and lists only Aurora's controls; the file M3's tasks update is `docs/parity/checklists.md`.
- **Tools.** Code is read with `trail` (`trail ask`, `trail grep`, `trail skeleton`, `trail callers`, `trail check`) and with Read.
- **Rulings.** Seven choices are decided (the Decided list), and the owner ruled the five rulings (Rulings for M3) on 2026-11-11, each as recommended: no task waits on an answer.

#### Part 1. How today's shell is built (what the tasks change)

**The frame, top to bottom.** All in `Moonglow::ui`, `crates/mg-ui/src/lib.rs:1580-1985`.

- `Panel::top("menu")` (lib.rs:1706) draws `commands::menu_bar` (commands.rs:1209) and then `commands::toolbar` (commands.rs:2059), which wraps to a second row. The toolbar is one group named "Toolbar" (`mg_uikit::name_group`); tests find its buttons with `common::tool` and `in_toolbar` (`tests/app/common.rs:323-358`).
- While the pointer is on the bars, `pointer_on_bars` and `undo_before_bars` (lib.rs:1711-1718) keep Undo pointed at the editor the pointer came from.
- The update and renderer notices are strips under the toolbar (lib.rs:1719-1723).
- `Panel::bottom("status")` calls `status_bar` (lib.rs:3040), which shows the module's full path, "(modified)" and the game folder.
- `Panel::bottom("log")`, 120 px, `hide_log` (lib.rs:1736).
- `Panel::left("tree")`, 240 px, `hide_tree` (lib.rs:1746).
- Each of these panes has a 16 px fold strip, `pane_edge` and `pane_fold` (lib.rs:2237-2285), which follows the one-button rule of GitHub issue 10 (`docs/deferred.md:921-944`).
- `CentralPanel` holds one `DockArea` over one `DockState<Tab>` (lib.rs:1789-1838). After it come `dock_look::marks`, `window_bars` (Maximize) and `palettes_edge`.
- Then about 60 dialog windows (lib.rs:1848-1942), the Command Palette modal (`commands::palette_window`, commands.rs:1960) and `run_actions`.

**Everything else lives in that one `DockState`.**

- The middle is the leaf holding `Tab::Area` or `Tab::NoArea`.
- The palettes are `Tab::Palette`, a leaf split off the root's right (`OpenTab(Tab::Palette)`, lib.rs:4245-4269).
- Splits off the middle are `Place::Split(Side, share)` (lib.rs:157-208, `split_middle` 2071, `split_of` 2090).
- Floating editors are egui_dock window surfaces (`open_window` 2725, `WindowTrack` 513, `remember_window_sizes` 2878, `toggle_maximize` 2986).
- Much code exists only for the palettes' pane: `keep_middle` (2646, issues 11, 20, 23), `palette_split` and `palette_share`, `pass_covered_arrow` (2291), `palettes_edge` (2323).
- `Tab` is `pub` (tabs.rs:15). `Tab::docks()` (tabs.rs:118) is true only for Area, Welcome and NoArea; every other tab opens through `place_for` (lib.rs:2123), which defaults to `Place::Window`.
- Kinds gather into one window by `Tab::gathers()` and the `*_own_windows` settings (lib.rs:4289).
- Tabs are drawn by `tabs::Viewer`: `title` (" *" for a dirty script or talk table), the tab menu `context_menu` (tabs.rs:670), and `on_close`, which calls `may_close`.
- Escape cancels only a floating Properties or Options window (`front` needs `Order::Middle`, tabs.rs:393-431).

**Settings and stored layouts.**

- `Settings` is `#[derive(Default)] #[serde(default)]` with no `deny_unknown_fields` (settings.rs:505). A missing field takes its default and an unknown field is ignored.
- A new installation and an old file can only be told apart in `read_eframe`'s `NotFound` branch (settings.rs:1131), used in `apps/moonglow/src/main.rs:618-622`.
- Layout fields: `window_sizes`, `window_places` (`middle`, `palettes`, `split left 300`; a window is never stored, settings.rs:605-609, lib.rs:2152-2193), `hide_tree`, `hide_log`, `hide_palettes` (672-674), `toolbar_icons_only` (743).
- Dock widths are egui `PanelState`s under the ids `"tree"` and `"log"`, persisted by eframe.
- Each module's open tabs are kept in `open-tabs/<name>-<hash>.json` (`open_tabs.rs`; `Pane.place` is the `Place` text). An unknown place parses to `None`, which means "where its kind opens" (`place_pane`, open_tabs.rs:1204-1273). So old and new records read each other gracefully.

**Commands.**

- `commands::Id` (commands.rs:19-131), `MENUS` (8 menus, :161) and `TOOLBAR` (16 entries, :289). Today's toolbar has **no Test Module**.
- Keys are kept by id; `keys::Cmd` (keys.rs) gives Ctrl+Shift+P to the Command Palette. Ctrl+P is free.
- `found()` (commands.rs:1894) feeds the palette: commands, plugin commands, plugin entries and project commands.
- The plugin API names menus by their text ("Plugins", "File › Export") and reads "the pane last clicked in" (`plugins.pane`, set in tree.rs:792 and tabs.rs:437). `crates/mg-plugin` reads nothing from `commands.rs`.

**Outside users of `mg_ui`.**

- `apps/moonglow` (graphics, recovery, plugins paths, `Moonglow::new`), `apps/mg` (`nui_render`, `project_commands::shell`), a corpus test (`test_module::command`). None uses `commands`, `Tab::Palette` or `Tab::Resources`.
- **Moonglow Viewer** pins `v3.14.0` of mg-core, mg-render, mg-preview and others, but neither `mg-ui` nor `mg-uikit`. No M3 change to a `pub` item of `mg-ui` reaches it. `mg-uikit` is not consumed yet (M6), so M3 sets the API the viewer will adopt. Additions only; no renames are needed.

**Tests that pin today's layout.** `--test app` has about 1,100 tests. These are hit:

- `windows.rs` (49 tests): panes, palettes pane, splits, windows, maximize, open-tabs records.
- `Tab::Palette`: about 100 uses (tiles 31, windows 26, palettes 21, frame_perf 15, screens 12, area_view 8, looks 5, others).
- `Tab::Resources`: about 33 uses (windows 17, resources 8, others).
- Fold-button names "Show/Fold Module Tree|Palettes|Log": 20 uses.
- Command Palette: 14 uses.
- `common::tool`: about 15 uses on main-toolbar buttons.
- `own_window`, `is_main()`, `click_window_close`, `close_windows`: about 40 uses.
- `screens.rs` has 169 `Moonglow::new`. Shell-relevant cases include `panes_folded`, `main_window_log_and_tree`, `view_menu`, `toolbar_icons_only`, `command_palette`, `windows_at_their_sizes`, `window_bar_maximize_and_an_older_hak`, `dialog_windows`, `theme_read_only_focus_and_tabs`, `readme_screenshots`.
- Perf tests open `Tab::Palette` through actions, not labels. `frame_perf.rs:45` measures at 3440 by 1400.

#### Part 2. Architecture decisions for M3

1. **The docks are egui panels; the editor area stays egui_dock.**
   - egui_dock 0.21 has one `Style` per `DockArea`, so the document tabs (28 px) and dock tabs (26 px) cannot both come from it.
   - Its splits are fractions, with no fixed or least widths.
   - Its collapse arrow is on every leaf or none.
   - So Left, Right and Bottom Docks are `egui::Panel`s holding a kit dock-tab bar and the panels. The Editor Area is today's `DockArea`, now holding only editors: document tabs, editor groups as main-surface splits, floating windows as egui_dock windows.
   - This keeps `WindowTrack`, Maximize, `may_close` and close-as-Cancel, and the open-tabs records. It retires the palettes-pane code (`Place::Palettes`, `palettes_edge`, `pass_covered_arrow`, `palette_split`, `palette_share`, the side-pane half of `keep_middle`).
   - Keep the panel ids `"tree"` and `"log"` for the Left and Bottom Docks, so remembered sizes carry over.
2. **A panel registry in mg-ui** (`shell.rs`):
   - `enum Panel { Module, Resources, Outliner, Palette, Preview, Log }` plus, per editor kind, the panels it brings.
   - The docks follow the active *docked* editor: the front tab of the focused main-surface leaf, else the last one.
   - In M3 the area brings Palette and Preview (right) and Outliner (left). Other editors bring nothing yet (M4 adds Reference, Node, Cell and the rest).
   - A dock or split with no panels is not drawn. **No merge point shows an empty panel.**
3. **Floating editors keep drawing their own side and bottom panes**, as `script_view` does today (`Panel::right("script-side")`, `Panel::bottom("script-info")`). Handing those panes to the docks when docked is each editor's M4 change.
4. **Settings.**
   - `Settings::default()` keeps today's meaning, the settings before M3: floating, log shown.
   - A new `Settings::new_installation()` (As Tabs, `hide_log = true`) is used only in main.rs's `NotFound` branch. An unreadable file gets the defaults, as an existing user.
   - Every existing test keeps passing without change; the new shell's tests set Tabs themselves.
5. **What lives where.**
   - `mg-uikit` (egui, plus the optional `dock` feature for egui_dock 0.21, decided as O5 below):
     - key cap, badge, modified dot;
     - dock-tab bar and panel header;
     - the fold strip (24 px icon strip with chevron);
     - Status Bar parts (switch with count badge, separator, the hint list that drops whole from the end, right items, job with bar);
     - Top Bar parts (menu title style, 24 px menu rows, centered field slot);
     - the document-tab painting helpers (kind icon, dot, close);
     - `shell::Yields`, a pure function from window and editor width to the yields of section 5.2;
     - the egui_dock `Style` from tokens (today's `dock_look::style`).
   - `mg-ui`:
     - the command table and menus' contents;
     - the panel registry and which editor brings which panels;
     - Quick Open's index;
     - settings and migrations, open-tabs records, `DockState` management;
     - the panels' contents (tree, browser, palette, preview, log).

**Checked against the code on 2026-11-11:**

- egui_dock 0.21.1 takes one `Style` for a `DockArea` (`widgets/dock_area/mod.rs`, the `style: Option<Style>` field), which is why the docks are egui panels and the editor area stays the one `DockArea`.
- Moonglow Viewer's `Cargo.toml` takes twelve toolset crates by git tag (v3.14.0) and neither `mg-ui` nor `mg-uikit`, so removing `Tab::Resources` and `Tab::Palette` does not reach it.
- A new installation is told from an existing one only in `Settings::read_eframe`'s `NotFound` branch (`crates/mg-ui/src/settings.rs:1134`).
- Ctrl+P is bound to nothing today (`keys.rs:382` gives the Command Palette Ctrl+Shift+P).

#### Part 3. The plan

Three merge points cut M3 into coherent, taggable states. Every task ends with:

- `cargo test --workspace --exclude mg-corpus-tests --lib --bins --tests`;
- the three clippy commands (including the hand-run tests);
- `cargo fmt`;
- `trail check`;
- where docs changed: `okf index docs`, `okf lint docs --links` and a `docs/log.md` entry;
- the task's pictures re-rendered with `screens.rs` and looked at before and after.

##### M3a: The frame (T0 to T8; merge point after T8)

**T0. The shell's pictures (do first).**

- **Goal.** Add the `screens.rs` cases every later task re-renders. The rule "reproduce a view as a picture before fixing it" needs the "before".
- **User sees.** Nothing.
- **Files.** `crates/mg-ui/tests/screens.rs`: a `shell` family on Chapter1 `map_m1q1a`, in both themes:
  - `shell_main` at 1600x900 (board Main.png);
  - `shell_narrow` at 1280x800 (Narrow.png);
  - `shell_wide` at 3440x1440 (WideArea.png);
  - `shell_floating`, a script floating over the area (Floating.png);
  - `shell_welcome` (Welcome.png).

  Do not touch `src/`.
- **Baseline for frame times (already measured).** The measuring half of T0 is done: the quiet runs of `frame_perf` and `editor_perf` made on 2026-11-11 on this same tree are kept at `/home/august/Projects/moonglow-toolset/target/ui-mock/perf-m2/perf-after/quiet-frame_perf.log` and `quiet-editor_perf.log`, with `compare.md` beside them, against `develop` before the redesign in `perf-before/`. (`target/` is not in git.) Later tasks compare their `frame_perf` runs with those logs. Also run `FRAME_SHOTS=1`.
- **Order.** Nothing before it. No app change, so the app stays coherent.
- **Tests.** The pictures only (ignored cases).

**T1. The kit's shell parts (`mg-uikit`).**

- **Goal.** The components the shell draws, each once.
- **User sees.** Only menu rows at 24 px with no gap (known difference "menu rows 26 vs 24", settled here). Today `menu_style` and `menu_rows` give 22 + 4 (controls.rs:478-506).
- **Adds.** `KeyCap`, `Badge` (accent, error, warn, ok, info, tag; text per proposal section 4.2), `modified_dot`, `DockTabs`, `PanelHeader`, `FoldStrip`, the `status` parts (switch with badge, separator, `Hints` that drop whole, right items, job bar), `TopBar` menu-title style, and `shell::Yields` (pure). Move `dock_look::style` behind feature `dock` (O5) and keep `dock_look` as a re-export; the crate's "egui alone" comment (`mg-uikit/Cargo.toml` and `lib.rs:11`) is changed with it. Measures come from `moonglow.css` (.kbd, .badge, .docktab, .statusbar, .sb-*, .menu, .menu-row, .topbar).
- **Must not touch.** mg-ui's layout.
- **Tests.**
  - Unit: `Yields` at 1279, 1280, 1299, 1300, 1399, 1400, 1499, 1500; menu row pitch 24; badge text contrast in both themes.
  - Kit sheet picture (`cargo test -p mg-uikit --test kit -- --ignored`), compared with Foundations and LightFoundations.
  - Run `menus_by_the_keyboard` and the context-menu screens.
- **Risks.** Every menu's height changes: `grouped_context_menus` and `context_menus` pictures.
- **Coherent.** Yes.

**Done, 2026-11-12.** The parts are in `mg-uikit`'s `marks` (key cap, badge, modified dot), `shell` (Top Bar row, dock tabs, panel header, fold strip, document-tab helpers, `Yields`) and `status` (switch, separator, hints, items, job), each on the kit sheet in both themes; the egui_dock style and tab marks are `mg_uikit::dock` (feature `dock`), re-exported by `mg_ui::dock_look`. Menu rows are 24 px with no gap. Beyond the plan, its review's fix round (pictures before and after in both themes, `screens.rs`'s `menus_*` cases): a menu's popover is padded 4 px above and below and not at its sides (`.menu-pop`), so its rows and their fills reach its edges, inside its border and its corners' radius; `menu_style` carries it to the popover through `MenuConfig` or `Popup::style`, which the kit's `IconButton::menu`, `menu_button`, the new `context_menu` (every right-click menu of the application) and submenus give it. The menu's other parts are the kit's (`mg_uikit::menu`): headings (`.pop-h`, padding 6 10 2, `text-3`), notes (a 24 px line), separators (`.pop-sep`, a 1 px `line-soft` line in 9 px), check boxes and radios as 24 px rows with their box where a row's icon goes, and a row's keys in `on-fill` while it is lit (`MenuKeys`, `menu_row_with_keys`; the Edit, View and other menus' rows use it too). Menus that hold a form (the NUI designer's narrow Properties, Add element and Layers, Rename…, How to open it…, the color picker) are laid out by `menu_form` (`.pop-body`: padding 4 10 6, a gap of 6), in the window's own style, as tall as a window's larger text makes its controls; `form_menu_button` and `IconButton::form_menu` keep such a menu open when a field in it is clicked. A panel header's detail is 12 px after its title, the stylesheet's 10 and the 2 the boards give the detail (its comment had said 14), and a hint's icon is said by the name its caller gives (`HintPart::NamedIcon`; an unnamed icon is not said). Against the stylesheet: key caps are Ubuntu Light (no regular face in the kit); egui_dock's tab menu (Rename…, Eject, Close) keeps egui's 4 px popover padding, its frame made by egui_dock; the Walkmesh and Shadows buttons' held menus stay egui popovers of check boxes. 1280 is not a threshold.

**T2. The frame skeleton: Top Bar, Left and Bottom Dock frames, Status Bar (the smallest task that proves the frame end to end).**

- **Goal.** Replace lib.rs:1706-1753 with `shell.rs`:
  - a 30 px **Top Bar**: the eight menus; New, Open, Save, Undo, Redo as icon buttons; Compile, Verify and **Test Module** with an arrow for Choose Character on the right (Test Module is new to the bar);
  - the **Left Dock** frame with one dock tab, Module (today's tree inside);
  - the **Bottom Dock** frame with one tab, Log, opened by a **Log** switch in a new **Status Bar**;
  - the editor area unchanged, palettes pane included for now.
- **User sees.** One top row instead of two; the tree and log in docks with tab bars; a Status Bar with the Log switch and its right items.
- **Today's toolbar buttons get homes:**
  - Properties and New Area: Module panel header icon buttons.
  - Start: View › Go to Start (exists; the compass button is M4).
  - Resources, Palettes: the Tools menu (until T4 and T5).
  - Preview: the Command Palette (until T6).
- **Files.**
  - `lib.rs:1706-1753` and `pane_edge`/`pane_fold` for tree and log (the strip becomes the Left Dock's `FoldStrip`; the Bottom Dock's one button is its Status Bar switch).
  - `commands::toolbar` and `TOOLBAR` become `TOP_BAR`; the test `the_toolbar_and_menus_name_commands_once` (commands.rs:2145) follows to `TOP_BAR`.
  - The accessible group becomes "Top Bar". Undo and Redo keep the `undo_before_bars` path (lib.rs:1711-1718) keyed on the Top Bar's rect, and fall back to the focused leaf's side editor (the active editor).
- **Settings.** `toolbar_icons_only` stops applying to the Top Bar, which is always icons. The Options control is renamed "Editors' toolbar buttons" until M4 (scope move S1; R4 gates only this wording). `hide_tree` means "Left Dock folded" and `hide_log` "Bottom Dock closed".
- **Tests.**
  - `common::tool` and `in_toolbar` look in "Top Bar".
  - Rewrite `the_toolbar_shows_icons_only_when_asked` as "the Top Bar draws icons with tips; the area toolbar still honors the setting".
  - Migrate `panels_fold_away_and_come_back`, `each_pane_folds_by_one_button_on_its_inner_edge`, `the_log_shrinks_to_one_line` and `screens::panes_folded`, `main_window_log_and_tree`, `toolbar_icons_only`: about 8 tests, 20 fold-name lines.
  - New: Top Bar buttons named, enabled and run; Test Module's arrow runs Test Module, Choose Character.
  - Pictures: `shell_main` against Main.png (Top Bar, Status Bar).
- **Manual.** 02-main-window intro (lines 11-29) and "The log" (761); 01-getting-started:83 ("New Area on the toolbar"); 07-scripts:86,99 and 03-modules:1739 ("the toolbar's Undo" becomes the Top Bar's); 10-options:256.
- **Risks.**
  - `frame_perf` run: the Top Bar's icons are tessellated each frame.
  - Plugin menus unchanged.
  - Not a merge point.

**Done, 2026-11-12.** The frame is `crates/mg-ui/src/shell.rs`, drawn by `Moonglow::ui` in this order: the Top Bar (`Panel::top("menu")`, 30 px and its line), the notices under it, the Status Bar (24 px), the Left Dock (`Panel::left("tree")`, the whole height between the bars, 260 px by default, a remembered width kept), the Bottom Dock (`Panel::bottom("log")`, under the editor area only, as WideArea.png has it, 180 px by default), then the editor area unchanged. The Top Bar is the kit's `top_bar` over `commands::TOP_BAR` (quick actions after the menus, build actions at the right end) and `TOP_BAR_TEST_MENU` (Choose Character, a menu under Test Module's arrow, as "under its arrow" says; the board draws a plain button); its middle is left empty for T8. The Left Dock is a `DockTabs` with the Module tab, Properties and New Area (named so, their tips "Module properties" and "Area Wizard") and the kit's fold button at its right (moved by T4 beside the dock's inner edge, half way down: see T4); folded, `Panel::left("tree-strip")` holds the kit's `FoldStrip`, whose chevron and Module button unfold it (since T4's fix round the strip has no chevron: the button that unfolds it stands beside the strip's edge, where the fold button was). The Bottom Dock is a `DockTabs` with the Log tab and the log's Copy Log, Save Log and Clear Log as icons on its tab bar (T7's place for them; their corner is gone). The Status Bar has the Log switch at its left and, at its right, "Reading files changed outside Moonglow…" while that runs (as the status line said it), the module's name with "(modified)" and the modified dot (the path in its tip; "No module open", "New module, not saved yet"), and the game folder's last part ("…/Neverwinter Nights", the path in its tip), left out where it does not fit. Undo and Redo on the Top Bar keep `undo_before_bars` and, with the pointer come from no side editor, fall back to the active editor (as built in the fix round below). Beyond the plan, found on the way: egui's menu bar takes the whole width it is given, so `commands::menu_bar` gives it the menus' own width (measured the frame before); the kit's right-hand groups (`at_right_end`) no longer widen what holds them before their width is known (a dock's panel grew by its tab bar's tools for good); a pass asked for again (`request_discard`) runs without the frame's input, and the Status Bar's right group changing width on a click's frame (the module becoming modified) lost the click, so the Status Bar is laid out by its right group's computed width (`status::bar_sized`); in a window too narrow for the whole Top Bar (under about 640 px) the build actions go whole, then the quick actions, every one of them being in the menus; the build actions as icons (under 1300) are framed as their words are (`IconButton::framed`), and Test Module and its arrow are joined (`IconButton::joined`, `joined_spacing`). Differences from the plan: the Left Dock's header buttons are named Properties and New Area, not Module Properties (the tree's row of that name is found by name in tests; T3 renames them with the Module panel); the Bottom Dock's and Left Dock's default sizes are the kit's (180 and 260, from 120 and 240). `frame_perf` on a quiet machine against M2's quiet run (median CPU ms, idle): module open with no area 0.35 (0.34), as it opens 0.50 (0.48), with its area 0.45 (0.38), File menu open 0.27 (0.26), the tree with every group open 0.81 (0.77); tessellation unchanged (0.03 to 0.11 ms) and about 1,400 vertices fewer in each view (the toolbar's row gone); none over its budget but the known area084. Its click on "Properties" now takes the blueprint's page, not the first of that name (the Module panel's, or before it the toolbar's): the item's Properties page view draws 11,860 vertices against the baseline's 14,754: the baseline's click had taken the old toolbar's Module Properties, so that view's numbers before T2 were another view's, and its numbers from the T2 run (`m3-t2-frame_perf.log`: 1.75 ms CPU idle, 11,860 vertices) are its baseline from now on. The five T2 timing logs are kept in `target/ui-mock/perf-m3/t2/` (not in git) for later tasks to compare with.

**T2's fix round, 2026-11-12** (after its review). (1) Undo: the fallback to the side editor in front of the focused pane made the module's Undo unreachable from the Top Bar after work in the tree (an empty hak's window focused left the button and Edit › Undo grayed while Ctrl+Z over the tree undid the module's step), or took a hak's step instead. The fallback is now the active editor, the one the user last worked in: `Moonglow::note_worked`, after the dock is drawn, notes a pointer press or a key away from the Top Bar and its menus (a press on a side editor's body or tab makes it the active editor; anywhere else, the module; a key is the 2DA grid's with the keys, unchanged while a text field is focused, else where the pointer is; read from the frame's raw events, since the keymap has taken Ctrl+Z by then), and `side_doc::worked_side_editor` takes it only while the editor is open and its tab in front of its pane. The rule as built, with the pointer on the Top Bar or a menu open (button, Edit › Undo and Ctrl+Z alike): come straight from a side editor, that editor's (`undo_before_bars`, unchanged); else the editor last worked in, if still in front; else the module's. Over a side editor, Ctrl+Z is the editor's; elsewhere, the module's (unchanged). An editor with no steps of its own still grays them (the side editors' rule of 2026-10-21). Tests: `shell.rs`'s `undo_on_the_top_bar_is_the_active_editor_s` (replacing `..._falls_back_to_the_active_side_editor`), `..._after_an_area_is_the_module_s`, `..._with_a_floating_editor` and `..._after_the_talk_table_is_its`, each asking by the button, Edit › Undo and Ctrl+Z and checking that the three say the same. (2) The Status Bar's right-hand items share the room left after the Log switch by one rule (`shell::share`): the game folder goes first; then the module's name and the note of a look under way ("Reading files changed outside Moonglow…") are cut short with an ellipsis, each to half the room where both are wider, or to what the other leaves; the note's words and the name's path in their tips, " (modified)" kept whole (the kit's `StatusItem::kept` and `max_width`, with `shown` and `is_cut`). A kit test, a unit test of the sharing, `a_long_module_name_is_cut_short_in_the_status_bar` at 420, 600 and 1000 with and without the note (`Moonglow::show_looking_note`, hidden, sets it for tests), and `screens.rs`'s `shell_long_name` (`shell_long_name_note_420.png` too). (3) The Top Bar's groups go in order as the window narrows: both are decided after the menus are laid out, the quick actions where they fit, the build actions only where both do (`the_top_bar_drops_its_build_then_its_quick_actions_as_the_window_narrows`, 300 to 1400 by 20). (4) The main window has a least inner size, `mg_ui::LEAST_WINDOW_SIZE`, 420 by 320 points: the menus end about 395 points from the left, as many points at every interface size, and a size kept smaller opens at it (`at_least` in `apps/moonglow`, on the builder eframe gives with the kept size, in points that egui-winit makes pixels at the zoom eframe restored). While Moonglow runs, `shell::keep_least_size` gives the least size again (`ViewportCommand::MinInnerSize`, in points, which egui-winit turns into pixels at the frame's scale) each time the scale changes once the interface size is in effect (Options › General › Interface size, Ctrl with + and -, a screen of another scale), and grows a window smaller than it in points (`InnerSize`). Tests: `at_the_least_window_size_the_menus_fit` at each size Options offers (`mg_ui::UI_SCALES`, 90% to 200%), `the_least_size_follows_the_interface_size` (the commands sent as the setting changes, and the window grown at 300%), and `a_window_kept_smaller_than_the_least_size_opens_at_it`; what a window manager does with the commands needs a display and was not run.

**T3. The Module panel.**

- **Goal.** The Left Dock's Module panel as drawn:
  - header icon buttons Module Properties, New Area, Expand All, Collapse All;
  - the filter;
  - a kind icon and right-aligned count per group;
  - the **Areas group scrolls inside its own group** (ruling 10);
  - the Left Dock folds to the 24 px icon strip with chevron.
- **Files.** `tree.rs:786-860` (`module_tree` header and groups), `shell.rs`.
- **Must not touch.** The tree's keys, selection, filter or follow-the-tab logic (learnings 2026-10-16, 10-28).
- **Tests.**
  - App: the Areas group scrolls on its own with 113 areas while groups below stay in place; header buttons run their commands; the strip unfolds.
  - Migrate the `tree.rs` tests that find "Expand All" globally (the palette has one too).
  - Picture `shell_main` Left Dock against Main.png and LightArea_crop_left.png.
- **Manual.** 02 "The module tree" (225).

**Done, 2026-11-12.** The Module panel is `tree::module_tree` under the Left Dock's tab bar (`shell::left_dock`). The tab bar carries the panel's four header buttons after the Module tab, in the board's order: **Module Properties** (the `module` cube, as its row: the board's `properties` sliders are drawn as Area Properties' icon, and an icon is one command's, `icons::an_icon_is_one_command_s`), **New Area** (`plus`), **Expand All**, **Collapse All**, then the fold button (beside the dock's edge since T4). Expand All and Collapse All are the tree's (`palette_view::fold_icons`, its keys unchanged) and reach the tree the same frame through `module_tree`'s new `fold` argument. The header button and the tree's row named Module Properties are told apart by group and role: the button is in the "Left Dock" group, a button; the row is in the "Module Tree Rows" group (inside "Module Panel"; "Module Tree" was the View menu's row, "Left Dock" since T4's fix round), a selectable label. The Filter is the board's: the `filter` icon, the placeholder "Filter" (`SearchField::icon`), and Clear Filter, in a `.panel-bar` margin of 4 and 8 px; the tree's rows reach the panel's sides. Each group's row is the kit's new `mg_uikit::TreeGroup` (`mg-uikit/src/tree.rs`, with `TreeRow` for a row that opens nothing): 22 px, the chevron in a 14 px column, the kind's icon, the name 42 px in, the count against the edge in sight (6 px in, kept there when the tree scrolls sideways), hover `hover-row`, marked `accent-fill`. It keeps egui's collapsing state under the same id and keeps the accessible name "Areas (113)", said expanded or not, so no test that finds a group changed. An area's kinds of placed objects (R2, kept) are `TreeGroup`s too, with the kind's icon, and "3 of 46" under the Filter; a kind with nothing placed is a quiet `TreeRow`; Module Properties and the "N other resources" line are `TreeRow`s. A resource row, an area's row and a placed object's are the kit's `TreeRow` too: the name 36 px from the panel's edge (22 into the group's rows, after the arrow's column, as the board's), the fill across the whole row from the panel's edge, an area's arrow a button of its own in the arrow's column. **The Areas group scrolls in a well of its own** (`areas_well`): a vertical and sideways `ScrollArea` as wide as the tree's view, pinned there when the tree scrolls sideways, taking its height alone in the tree's layout, with a height of the tree's whole view (a scroll bar across its foot or not) less one row for each of the tree's other rows (Module Properties, each group shown, the line under them), in whole rows, at least the board's five, and no more than its rows. The board gives no rule but draws the same arithmetic: its Module panel of 496 px is the tabs, the filter, two rows, five in the well and thirteen groups. With the groups closed, every group is in sight under Areas in an 800 px window (Chapter1, 113 areas: 14 rows in the well), and the well is not resizable. Rows out of sight in the well are not laid out (the tree's culling works on the well's clip). The tree's keys, selection, Filter and follow-the-tab logic are unchanged. Only `bring_into_view` and `reveal_into_view` learned of the well: a scroll area works out a row to scroll to by the part of itself in sight (none, with the tree scrolled past the well) and keeps the target from the tree around it. So inside the well they leave the row (`well_target`), the well scrolls itself to it after its rows are laid out (to the middle, or only as far as needed, as before), and the tree brings the well into its view. Home and End over the well go to its top and foot, and elsewhere over the tree to the tree's. Differences from the plan and the board: the board shows no module-wide count, so the "N resources" beside Expand All is gone (each group's count and the "N other resources" line remain); New Area's tip says "Make a new area with the Area Wizard" and Module Properties' "Open the module's properties"; the Left Dock's least width is 190 px (`LEFT_DOCK_LEAST`, from 150), so its tab bar's controls never overlap, also in the least window; the folded strip is T2's, unchanged, and shows the Module panel alone until T4 adds Resources. A dropdown's Filter says "Filter" as the tree's does; the tree's is named "Filter Module Tree" for a screen reader (`SearchField::name`), which the tests find it by (`common::tree_filter`, `list_filter`). Timing: the machine stayed loaded throughout (a 1-minute load of 15 to 25 from other work), so `frame_perf` was run against T2's tree built beside it (`git archive` of T2's commit), the two alternated three times. Median CPU ms idle, T2 then T3: module open with no area 0.38 to 0.41 against 0.37 to 0.41; the tree with every group open 0.90 to 0.92 against 0.84 to 0.96; all 25,000 resources listed 3.28 to 3.68 against 3.08 to 4.64 (the highest beside a tessellation twice as slow as the same view's other runs: the machine); a word in the Filter (300 areas' objects) 2.16 to 3.29 against 2.37 to 4.02; 300 areas in the tree 0.73 to 0.81 against 0.71 to 0.94. Tessellation is unchanged, about 1,800 vertices more where the groups below Areas are now in sight (their icons and counts). A first run had the Filter view about half as slow again: a group's row laid out its words twice, and those of rows out of sight too. It now lays out its words once, and none for a row out of sight. `world_perf`'s module tree with 8,000 items takes 5.1 and 5.9 ms against T2's 3.5 and 4.7 alternated. In T3 its items are in sight, where T2's 300 area rows hid them below the window. `campaign_perf` is the same either side. Nothing is over its budget but `frame_perf`'s known area084 (Sound Ranges).

**Fix round, 2026-11-12.** A review of the commit found five faults, which this round fixed, each with a test in `tests/app/module_panel.rs` and pictures before and after (`target/ui-mock/impl/m3-t3-fix/`, not in git). (1) The well was laid out as wide as the tree's view, and the tree keeps the widest it has drawn (`tree-widest`), so a Left Dock widened and narrowed again scrolled 322 px sideways into nothing, with a scroll bar that took a row from the well: the well takes its height alone in the tree's layout (`Ui::new_child`, as the kit's rows take only the width their words need) (`a_narrowed_dock_leaves_the_tree_no_wider_than_its_rows`, `shell_tree_narrowed`). (2) The well's place, kept for Home and End, outlived the well: once the Areas group closed, Home and End did nothing where it had been. It is known for the frame the well is drawn in and only within the tree's view (`home_and_end_go_to_the_well_s_ends_over_it_and_the_tree_s_elsewhere`). (3) A word typed in the Filter and cleared brought the well back at its top, and a word that finds nothing did the same to the tree: the well scrolls under an id of its own while the Filter has something typed, and the tree's place before the Filter is put back once it is cleared (`the_well_and_the_tree_keep_their_places_through_the_filter`). (4) Resource rows' fill hugged their names and the names stood 40 px in: they are now `TreeRow`s (above), for which the kit gained `TreeRow::sense` (a blueprint's row drags), `arrow_apart` with `tree::arrow_column` (an area's arrow is its own button, before the row, so that it keeps its clicks, its tip and the keyboard's focus before the row's; the row answers from its name on), `fill_under` (the row's fill under that button) and `show_parts` (where the marks after the name go: the claims mark and "(not in the area list)", not selectable, so that a click on them is the row's); a row's fill reaches the left edge of the scroll area it is in (the board fills an indented row's indent), and the arrow of a marked area row is drawn on the fill in `on-fill` (`a_tree_row_fills_the_row_drags_and_leaves_its_arrow_to_a_button` in the kit; `shell_tree_rows`). (5) The well dropped a row when another group opened: the tree's scroll bar (8 px) came from (1), and the well now measures the tree's whole height, so a sideways bar that is needed does not change it either (`the_well_keeps_its_rows_whatever_else_is_open`). The review's other asks: the menu test opens the menu of a row in the well and of the rows cut at its top and foot, waiting for them rather than counting frames, and says that nothing in the well drags (an area's row and a placed object's are clicked, not dragged; the drag tested is a blueprint's under the well, the well scrolled, so the auto-scroll at the well's edge has no row to try); the tab test chooses the tab with Ctrl+Page Up and a click on it; new tests hold the five-row floor and a three-area module's three rows, every group reached in the least window by the wheel and the keys, F2 and the Rename window on an area in the well (there is no field of the tree's own: Escape closes the window and leaves the well, Enter renames), Delete on one (the well as high and where it was), a tab's menu's Show in Module Tree for an area deep in the well and a script under it, F6 and Shift+F6 out of the tree and back to a row in the well and Tab from it to the next row, and Shift and Ctrl choosing across the well's border. Found and left as they were, the keys' logic not being this round's: Escape in the Rename window also takes the keys from the tree, and the keys stay at a resource deleted from the tree. `shell_tree_groups` presses the Left Dock's Expand All. `frame_perf`, run alone at a 1-minute load of 2.9 (4.4 at its end), against T2's log for vertices and order of magnitude (median CPU ms idle, T2 then now): module open with no area 0.35 and 0.37 (8,410 and 10,266 vertices: the groups under the well in sight, as T3 had them), every group open 0.81 and 0.82 (8,410 and 8,490), all 25,000 resources listed 3.14 and 3.05 (8,398 both), 300 areas in the tree 0.68 and 0.70 (8,398 and 10,266), a word in the Filter 2.31 (9,258); nothing over its budget but the known area084 (Sound Ranges). Then, at the owner's word, the two faults it had left (both older than the redesign, the same code at `992ed8d2`): the tree no longer reads the keys while a dialog is open over it (`tree::keys_free`, `widgets::dialog_open`), so Escape and Enter in the Rename window or Delete's question (where Enter is now Delete) leave the tree's keys at their row; and the keys at a resource deleted (several together, an area in the well, the making undone) go to the row that takes its place (`past_deleted_resource`, by the rows listed the frame before), and at a renamed one to its new row (`retarget`).

**T4. The Resources panel (R5, ruled).**

- **Goal.** `Tab::Resources` becomes `Panel::Resources`, the Left Dock's second tab: Type, Name filter and Overridden copies on top; Refresh and Export N as Files… in the panel header; the count in the panel; double-click opens the read-only tab in the editor area.
- **Files.** `browser.rs` (`ui` and its width), `tabs.rs` (remove the variant), `lib.rs` (`keep_middle`'s `Tab::Resources` branch at 2672-2678, `step_pane`'s Browser at 2526-2537), `commands.rs:787` (`Id::Resources` shows the panel), `open_tabs.rs` (`parse_tab("resources")` shows the panel; never written as a tab).
- **Settings and layouts.** `window_places` entries of kind `resources` are dropped (one migration function, `Settings::shell_migrate`, idempotent). An open-tabs record with "resources" shows the panel.
- **Tests.**
  - Migrate `resources.rs` (36 tests; 8 open the tab).
  - Retire with a reason in the commit `the_resource_browser_dropped_on_a_side_is_docked_there` and `the_resource_browser_alone_keeps_the_middle_beside_it` (the behavior is gone; see R5). Their deferred entry (issues 11, 20, 23, deferred.md:957-975) closes.
  - Migrate the windows.rs record tests that name it (`side_editors_and_tools_open_again_with_the_module`, `what_side_editors_…`).
  - New: an old record with "resources" opens the panel.
  - Picture against Resources.png at 260 px.
- **Manual.** 02 "The resource browser" (645).
- **Risks.** The browser at 260 px wide: its columns must cut with an ellipsis, not overflow (picture at the least width).

**Done, 2026-11-12.** The resource browser is the Left Dock's **Resources** panel, its second tab beside Module; `Tab::Resources` is gone. The registry of decision 2 is `shell::Panel` (`Module`, `Resources`; public, as `mg_ui::Panel`) with `Panel::dock` (a `Dock`, Left only so far), `Panel::available` (Module with a module open; Resources with one, or without one once asked for, for the run: the Left Dock with the Resources tab alone) and `shell::panels(app, dock)`, the panels a dock holds now (none: it is not drawn); T5 adds `Dock::Right`, `Palette` and its rule, and T6 `Preview` with the rule that follows the active editor. The tab chosen is `Moonglow::left_panel`, for the run (the plan names no setting); `Action::ShowPanel(panel)` shows the dock unfolded on a panel (Tools › Resource Browser, its key where one is given; asked again, the panel stays), and `Moonglow::left_dock_shows` says which is shown. Folded, the strip has a button for each panel (Module's box, Resources' folder, `Narrow.html`'s rail), each unfolding the dock on it, the chevron on the panel shown (since the fix round below, no chevron: the button beside the strip's edge). The panel is the board's: Type (the whole width), the Name field (the filter's funnel, "Name", **Clear Name**, named "Filter Resources" for a screen reader), Overridden copies and, where models are listed, Models as text; under them the count ("627 of 113538 resources", and the overridden copies listed), cut short with an ellipsis at the dock's width, whole in its tip (the "double-click to open" of before is that tip until the Status Bar's hints, T12); a line; the list. The header (the tab bar's tools) has **Export N as Files…** (named with how many it writes, enabled with the list narrowed, the board's "write the listed resources", as before; the browser has no selection of several rows, nor sorting, copying or dragging, today or now) and **Refresh**. A row is the board's `.tree-row`: the kind's icon, the name in 12 px monospace, the layer at the right in small weak words, an overridden copy set in and dimmed; where both do not fit, the layer is cut to 45% of the room and the name to the rest with an ellipsis (`browser::share`; the name first since the fix round below), never past the dock's edge, the whole of both in the row's tip ("portraits.2da from hak:low, overridden by hak:top"). The row is two targets side by side that answer as one: up to the layer a selectable label named by the name (the one Tab reaches), the layer a label of its own clicked as the row (not a stop for Tab); before, the layer was a label beside the name. Double-click and Enter open the resource read-only in the editor area, as before (in a window or tab as its kind opens). F6 steps to the Left Dock's panel shown, the tree or the browser's list (at the row the keys were at, else the first); choosing the other tab takes the keys with it; Show in Module Tree chooses the Module tab. A click in the panel leaves the plugins' "pane last clicked in" as a click in the browser's tab did. The "files already there" question of Export as Files is asked with the frame's windows, so it outlives the panel folded. Settings: `Settings::shell_migrate` (run by `Moonglow::with_settings`) drops the `window_places` and `window_sizes` of kind `resources`, nothing else, and run again changes nothing. An open-tabs record with `resources` shows the panel (with the record's other tabs, or alone), and no record is written with it again; an older Moonglow reads what is written now, which has nothing new. Removed with the tab: `keep_middle`'s pane of the browser alone (issue 23), `step_pane`'s browser in the middle and in the palettes' pane, `Tab::Resources` in `kind`, `window_size`, the title and the body. The deferred entry of issues 11, 20 and 23 keeps its palettes' part for T5; its browser part (21, 23) is settled. Tests: `resources_panel.rs` (Tools › Resource Browser and its key, folded or not; the tabs by click, keyboard and strip; Export and Refresh; the rows inside the dock at 190, 260 and 500 with a cut cell's tip; double-click, Enter and the layer's right-click; F6; the plugins' pane; an old settings file; the browser with no module; the rows in sight alone), `module_panel.rs`'s `show_in_module_tree_with_the_resources_panel_chooses_the_module_tab`, `windows.rs`'s `the_resource_browser_is_a_panel_not_a_tab`, `the_middle_keeps_the_editor_area_whatever_the_left_dock_shows` and `an_old_record_with_the_resource_browser_shows_the_panel`, and unit tests of `shell_migrate`, `share` and the record's `resources`. Pictures: `screens.rs`'s `shell_resources`, `shell_resources_least` (190 and 500) and `shell_resources_folded`, with `shell_main`, in both themes (`target/ui-mock/impl/m3-t4/`). Timing: the machine stayed loaded by other work (a 1-minute load of 9 to 15 through the twenty minutes waited and every run), so these are rough. `frame_perf` (median CPU ms idle, T2's log, T3's fix round's, now): the resource browser 0.35, 0.38, 0.32 (10,680, 12,340, 15,958 vertices: a window of 920 by 700 before, a panel 260 wide and the screen's height now, with an icon a row); the resource browser over 300,000 resources 1.80, 1.78, 0.52 (13,100, 12,924, 17,618 vertices); the tree with every group open 0.81, 0.82, 0.86. The views are not alike: before, the browser was a floating window drawn beside the module tree; now the panel is drawn in the tree's place, so the frame no longer draws the tree (or a window) beside it, and the lower times say nothing of the browser alone. That the list lays out the rows in sight alone, as before (`show_rows`), is tested (`the_list_lays_out_only_the_rows_in_sight`: of 113,000 resources, the rows the panel has room for). `world_perf`'s resource browser frame 4.13 (T2), 6.40 (T3), 3.09 ms; `campaign_perf`'s over the TDoD haks (`MOONGLOW_HAKS`) 0.55 ms a frame. Nothing over its budget but `frame_perf`'s known area084 (Sound Ranges).

**The Left Dock's tab bar, by the coordinator's rule of 2026-11-12.** Two tabs, a panel's four tools and T2's fold button did not fit the bar (259 px at the dock's default 260: the Module tab's bar needed 261.4; at the least width, 190, no panel's tools fitted, and they ran past the dock's edge). (1) The fold button left the tab bar, as the boards draw it (Main.png's bar: the two tabs and the four tools): it stands on the dock's inner edge, half way down, across the 1 px edge, 12 by 44 px, drawn over the edge after the editor area beside it (`mg_uikit::shell::edge_fold_button` and `edge_fold_rect`, new in the kit with a kit test; `shell::left_dock_fold`), where builders knew the panes' buttons from issue 10's rule. It keeps its name ("Fold the Left Dock", said open), tip and key (View › Module Tree's); the edge drags above and below it (the dock's resizing is off only under the button, as before); folded, the strip's chevron unfolds as before (no 16 px strip comes back). (2) Where the tabs' words and the shown panel's tools do not fit the bar side by side, the tabs show their icons alone (the strip's: Module's box, Resources' folder), each still a tab named by its words, said selected or not, its name in its tip; the words come back where they fit (`DockTab::icon`, `DockTabs::tools_width`, `DockTabsOutput::icons`, new in the kit with a kit test). It is decided each frame from the words measured and the bar's width alone, so at one width it is always one or the other: swept a point at a time from 190 to 600 on both tabs, the bar never toggles and nothing passes the dock's edge (`shell::the_left_dock_s_tab_bar_fits_at_its_least_width`, now stricter). Icon tabs are 33 px, so at 190 the two of them and the Module panel's four tools take 160 of the bar's 189; the Module tab shows words from about 240, the Resources tab from about 196 (since the fix round below, both from about 240: the widest tools decide). The least width stays 190.

**Fix round, 2026-11-12.** A review of the commit found ten faults, which this round fixed, each with a test and, where it shows, pictures before and after in both themes (`target/ui-mock/impl/m3-t4-fix/`, not in git; `screens.rs`'s new `shell_tabs_220` and `shell_fold_edge`). (1) The module tree and the browser's list were both egui's scroll area of the default id in the same panel, so each took the other's place when its tab came back (the tree 4,600 px down; the list back at its top, its keys' row out of sight): the list scrolls under an id of its own (`resources-list`); the tree keeps the default id, under which eframe has kept its place from run to run (egui's scroll state is persisted, and eframe saves egui's memory), so nobody's stored place is lost. The Areas well, `tree-widest` and the two Filter fields had ids of their own already (`the_tree_and_the_list_keep_their_own_scroll_places`, Chapter1, three rounds each way). (2) The tabs' form (words or icons) followed the shown panel's tools, so from 198 to 240 px a click on a tab changed the bar under the pointer: it follows the widest tools of the panels the dock holds (the Module panel's four), the dock's width alone (`the_dock_s_tabs_keep_their_form_and_place_whichever_panel_is_in_front`: 190 to 300 px, the tabs' rects alike on either panel; two clicks and a double-click at 220 end on the tab clicked). (3) Across the edge, the fold button covered the browser's scroll bar but 2 px of it, and a press there folded the dock: it stands beside the edge on the editor area's side (`mg_uikit::shell::edge_fold_rect_beside`, new, with a kit test), 12 by 44 px from the line's outer side, as the palettes' button stands on its pane's side of its edge; the edge drags above and below it (`a_press_on_a_list_s_scroll_bar_by_the_fold_button_scrolls_it`: the browser's list, the tree's Areas well with every group open, and the tree scrolled past the well; and the edge dragged above and below). Its right 4 px lie over the editor area's margin's last 4 px and the middle pane's first 4 (the margin is 8): nothing of the pane's is under it in the pictures. (4) Issue 10's "in the same place, folded" had been dropped: the button that unfolds the dock is now beside the strip's edge at the same height and size as the one that folds it, pointing the other way (`mg_uikit::shell::edge_unfold_button`, new, named "Show the Left Dock" and said closed), and the strip has no chevron of its own (`FoldStrip::chevron(false)`, new, with a kit test; its panels' buttons start at its top). `each_pane_folds_by_one_button_on_its_inner_edge` has its old assertion back (the same size, the same height, folded and unfolded) and a second click at the first one's height unfolds the dock; the x differs by the dock's width, which the edge moves by, so two clicks at one point cannot both be on it. (5) Export's count was only in its accessible name and its tip was two sentences: the tip says "Write the 627 listed resources into a folder as files" (one: "the 1 listed resource… as a file"), and disabled "Narrow the list to export it" or "Nothing is listed to export" (`export_s_tip_says_how_many_it_writes`). (6) At 190 px four rows read "nw_ch_srch…": the name comes first (`browser::share`): the layer takes what the whole name leaves, at most 45% as before, and is left out where less than a stub ("key:n…") would be; a name longer than its row is cut in its middle with its extension kept (`browser::middle_cut`, "act_q2dji…stone.nss"), laid out from the monospace advance measured once a frame for the rows in sight alone (`RowFonts`), the cut built and measured, else cut at its end (`at_the_least_width_the_rows_are_told_apart_by_their_names`, `a_name_longer_than_its_row_is_cut_in_its_middle`, unit tests of `share` and `middle_cut`). (7) With no module, the dock on Resources had no menu row or key: View's row, its key and F6 work whenever the dock holds a panel (`without_a_module_the_view_row_its_key_and_f6_reach_the_resources_panel`). (8) View › Module Tree folded the dock on either panel: the row is **Left Dock** ("Show the Left Dock, or fold it away"); the moves document names no row, so the dock's name. Its id (`view-tree`) and so a key a user gave it are kept, as keys are stored by id (`keys.rs`); no plugin or manual chapter names the row's text (`view_left_dock_folds_the_dock_on_whichever_panel`, the Command Palette by "left dock" too). (9) The fold button is drawn right after the dock, in an `egui::Area` of the background order made a sublayer of the dock's layer, so that it is over the editor area beside it, under the windows over it, and Tab reaches it after the dock's controls (and egui's resize edge) and before the editor area's (`tab_reaches_the_fold_button_after_the_dock_s_controls`; a window moved over it takes the click there, `a_window_over_the_fold_button_takes_its_clicks`); Left and Right on a focused dock tab go to the tab before or after it, round the ends (`DockTabs`, `the_arrow_keys_go_between_a_dock_s_tabs` in the kit; `the_arrow_keys_on_a_dock_tab_choose_the_other_tab`, the tree's keys left alone); `shell.rs`'s module doc, a stray blank line in `deferred.md`, and `fold_button_tip`'s doc (kept, for a dock that keeps the button among its tools). (10) Tests of the records read before: an open-tabs record with the browser in front of a split, and one with it in a window with a stored size, beside another tab and alone (`windows.rs`'s `an_old_record_with_the_browser_in_front_of_a_split_shows_the_panel`, `…_in_a_window_shows_the_panel`), and settings parsed as JSON given to the app (`json_settings_with_a_resources_place_open_without_it`: no settings file is read as JSON; `Moonglow::with_settings` migrates whatever was parsed). Tests changed to the new place of the button (its left side, not its middle, is the dock's edge): `shell.rs`'s `fold_edge`, the strip test's button beside the strip, `module_panel.rs`'s `dock_edge` and the header test, `windows.rs`'s middle test, `screens.rs`'s `shell_tree_narrowed`, `frame_perf`'s panel; to the row's new name: `shell.rs`'s View menu test and `windows.rs`'s pane-keys test. Timing: `frame_perf` run once, alone, after twenty minutes' wait in which the 1-minute load stayed above 3 (it was 6.7 when the run began and 6.3 at its end, from other work), so the numbers are rough. Median CPU ms idle, T3's fix round, T4, now: the resource browser 0.38, 0.32, 0.30 (12,340, 15,958, 16,030 vertices); the resource browser over 300,000 resources 1.78, 0.52, 0.49 (12,924, 17,618, 16,014: the narrower layer and its stub at the default width, fewer rows' layers whole); the tree with every group open 0.82, 0.86, 0.83; all 25,000 resources listed in the tree 3.05, 3.41, 3.22; a word in the Filter 2.31, 2.47, 2.42; the module with 300 areas 0.70, 0.75, 0.72 (72 vertices more in each tree view: the fold button now drawn in its own layer). The list still lays out the rows in sight alone (`the_list_lays_out_only_the_rows_in_sight` passes; the middle cut measures the monospace advance once a frame and lays out a cut row's name once more). Nothing over its budget but the known area084 (Sound Ranges). The log is kept in `target/ui-mock/perf-m3/t4/` (not in git).

**T5. The Right Dock and the Palette panel (R5, ruled).**

- **Goal.** `Tab::Palette` becomes `Panel::Palette` in a Right Dock (320 px default, **least width 280**). The palettes' pane machinery goes.
- **User sees.** The palette in a dock with its own tab bar; the Right Dock folds to a strip like the left (issue 10's one-button rule, kept; see U3).
- **Files.**
  - `lib.rs`: `Place::Palettes`, `palettes_edge`, `pass_covered_arrow`, `palette_split`, `palette_share`, the side-pane half of `keep_middle`, `OpenTab(Tab::Palette)` (4245-4269), `open_palette` (1756-1761), `show_panel`/`panel_shown` (2197-2231), `reset_layout` (2369), `closed` (2419).
  - `tabs.rs:437` (`plugins.pane = Palette` moves into the panel's draw).
  - `palette_view.rs` (its keys and `Keys::Palette` in `step_pane`).
- **Settings and layouts.**
  - `hide_palettes` means "Right Dock folded".
  - `window_places` entries with place `palettes` are dropped (the kind opens at its default), and so is the palette's own entry (`split left …`).
  - Open-tabs place `palettes` already parses to "where its kind opens"; the tab text "palette" means "show the panel".
- **Tests.**
  - Add `common::show_palette(h)` first and make the about 100 `Tab::Palette` lines call it (tiles 31, palettes 21, windows 26, area_view 8, looks 5, tree 3, screens 12, frame_perf 15, world_perf 2, campaign_perf 1).
  - Retire `the_palettes_dropped_on_the_middle_join_its_tabs`, `the_palettes_covered_arrow_is_passed_by_the_keyboard`, `tabs_open_beside_the_palette_not_in_its_pane`, `the_middle_says_no_area_is_open_without_the_palettes` (the pane they test is gone).
  - Turn `a_tab_kept_in_the_palettes_pane_opens_there_again` into the migration test.
  - Run `views_draw_nothing_over_anything_else` (windows.rs:914) at the Right Dock's least width: the palette list toolbar no longer spills under the fold button at about 135 px (known difference, settled here).
  - Unit: `shell_migrate`.
  - Pictures `shell_main` and `shell_wide` against Main.png.
- **Manual.** 02 "The palette" (624) and the intro sentence "Palettes on the toolbar brings it back".
- **Parity.** `docs/parity/checklists.md` "Palettes" (192).
- **Risks.**
  - `frame_perf` run: the palette draws in a panel now.
  - Plugin `ctx.selection` from the palette: run `plugins.rs`.
  - Closes the issue-10 entry's palettes part (deferred.md:921-944).
  - Largest task of the milestone; if it runs past a day, land `show_palette` and the registry stub first as their own commit.

**Done, 2026-11-12.** The palettes are the Right Dock's **Palette** panel; `Tab::Palette` is gone. The registry `shell::Panel` has `Palette` and `Dock::Right` (`Panel::RIGHT`, `shell::right_shown`, `Moonglow::right_dock_shows` and `right_dock_rect`); `Panel::available` holds the Palette with the game's data read and a module open, or without a module once Tools › Palettes asked for it (`palettes_alone`, for the run: the game's standard palettes, as the palettes' pane opened beside the start page before), and a test hook, `Moonglow::palette_dock`, keeps it from coming with the module (what `open_palette = false` did; asked for, it shows). T6 adds `Panel::Preview` to `Panel::RIGHT` and the rule that follows the active editor in `available`; `right_panel` keeps the tab chosen. The dock (`shell::right_dock`, `Panel::right("right-dock")`, an id no other panel or area has) is drawn after the Left Dock and before the Bottom Dock, the whole height between the bars (WideArea.png's log between the docks), 320 px at first (`metrics::RIGHT_DOCK`), at least 280 (`kit::RIGHT_DOCK_NARROW`), at most two fifths of the window as the Left Dock; no width of the palettes' pane was kept from run to run (it was a share of the editor area), so none seeds it. Its tab bar is the kit's `DockTabs` with the Palette tab (no tools: the boards draw none on it); the panel's body has the board's 8 px round it. Its fold button is the Left Dock's, mirrored (`shell::edge_fold`, shared by both docks): beside its inner edge, half way down, the same place and size folded beside the strip's edge (`Fold the Right Dock`, `Show the Right Dock`), the strip with a Palette button (`Icon::Palette`). `Settings::hide_palettes` is "the Right Dock folded". View's row is **Right Dock** ("Show the Right Dock, or fold it away"), its id `view-palettes` and key Ctrl+Alt+2 kept; Tools › Palettes shows the dock unfolded on the panel (`Action::ShowPanel(Panel::Palette)`, its tip "The blueprint and tile palettes, in the Right Dock"). The palette's body is unchanged (`palette_view::ui`) but for the type strip's buttons 1 px apart (`.seg`: the thirteen on one row at 320, two rows at 280; the combo under 300 px of panel width is T14's, the strip not broken without it) and its lists' scroll areas under ids of their own (`palette-list`, `palette-tiles-list`; prefabs' had one). A press in the panel makes it the plugins' pane last clicked in, as a press in the tab did. F6 goes Left Dock, editor area, Right Dock, Bottom Dock and round (a folded dock passed over). Removed: `Place::Palettes`, `palettes_edge`, `pass_covered_arrow`, `palette_split`, `palette_share`, `palette_edge`, `palette_arrow`, `tab_back`, `open_palette`, `pane_edge`, `pane_fold` and the old `widgets::pane_fold_button`, `pane_fold_rect`, `Toward` and `PANE_EDGE`, the side-pane half of `keep_middle` (the NoArea placeholder kept), `OpenTab`'s palette branches, the palettes' arms of `show_panel`, `panel_shown`, `place_of`, `middle_pane`, `open_window`, `window_bars`, `remember_window_places`, `reset_layout` (which now forgets the Right Dock's width too) and `closed`, the open-tabs wait for the palettes' pane, and `tree::follow_front_tab`'s and `tabs.rs`' palette exceptions. Settings: `Settings::shell_migrate` drops `window_places` of kind `palette` or place `palettes` and `window_sizes` of kind `palette`; an open-tabs record's `palettes` place opens a tab where its kind opens, and a `palette` tab (no Moonglow wrote one) shows the panel. Found on the way and fixed: a window's bar took its double click (maximize, restore) only over the editor area, its clicks cut to it, so a maximized window's bar did nothing over the docks (`window_bars`). Tests: `right_dock.rs` (Tools › Palettes and View › Right Dock and their keys folded and not, with no module; every palette type swept from 280 to 600 with nothing outside the dock, under its button or overlapping; F6; old settings as RON and JSON; the plugins' pane; the maximized window's bar over the docks; the palette's scroll bar at the button's height; the lists' own scroll places), `windows.rs` (the Right Dock in `each_pane_folds_by_one_button_on_its_inner_edge`, `panels_fold_away_and_come_back`, `views_draw_nothing_over_anything_else` at 280; `tab_reaches_the_right_dock_s_fold_button_after_its_controls`, `the_palette_is_a_panel_not_a_tab` and `a_tab_kept_in_the_palettes_pane_opens_where_its_kind_opens` in place of the four retired and the one turned), and unit tests of `shell_migrate` and the record. The area harness is 1160 wide (`AREA_HARNESS_WIDTH`), the area's view as wide as at 1100 beside the palettes' pane. Pictures: `screens.rs`'s `shell_right_dock` (280 on the tiles and on the placeables, the Gallery at 320 and 440, folded) with `shell_main`, `shell_narrow`, `shell_wide`, `shell_760` and `shell_floating`, in both themes (`target/ui-mock/impl/m3-t5/`). Timing (`frame_perf` at a 1-minute load of 2.8 to 4.4, T4's fix round then now, median CPU ms idle): module as it opens 0.68 and 0.54, the standard placeable palette 0.35 and 0.34, searched 0.58 and 0.54, the custom item palette (8,000) 0.67 and 0.65, all 8,000 shown 2.01 and 1.95, the Gallery 0.76 and 0.71, tessellation and vertices alike; nothing over its budget but the known area084. In a window with no room for its least width beside the Left Dock (under about 500 px) the Right Dock takes what is left but the room of the two fold buttons, never past the window's edge (`the_least_window_holds_both_docks`, 420 to 600). Left as it is, for the coordinator: in a window under about 760 px the two docks at their sizes leave the editor area narrow (73 px at 640, 170 at 760, where the palettes' pane shared it and left about 235), and the Bottom Dock's tab and buttons overlap under about 500; T14's yields (the Left Dock folding under 1500, the Right Dock at 280 under 1400) are its rule, and `the_area_toolbar_stays_inside_its_tab_at_every_width` folds the Left Dock under 760 meanwhile.

**Review, 2026-11-12.** An independent review of T5 found, and this round fixed: (1) a window made narrower shrank both side docks for good (egui kept the width drawn as the panel's, and eframe saved it): the width each dock is given is now kept apart from the width drawn (`Settings::left_dock_width`, `right_dock_width`, and the Bottom Dock's `bottom_dock_height`, which had the same fault), each drawn held to what the window allows and changed only by a drag of its edge or Reset Layout; a width egui kept from an earlier run is taken once, so none is lost. They are kept in the settings rather than egui's memory because the settings are saved and merged between two windows field by field and read back by the tests as the next run reads them. (2) In a window too narrow for both docks the palette was wider than the Right Dock (a category's name never cut, the type's name over Standard), so the dock's edge was pushed past the window's and the editor area and the Bottom Dock were drawn over it: a category's name is now cut short with an ellipsis, Standard and Custom go under the type's name where the two do not fit, the panel's body is laid out in the dock's room and cut at its edges, and the editor area and the Bottom Dock are drawn within the room the docks leave them. (3) Tools › Palettes with no game data read did nothing: it shows the dock, the panel saying so. (4) F6 and Shift+F6 were grayed with the Right Dock the only dock shown. (5) The sweep could pass with the fault there: it now takes the dock's nodes by the tree (its panel is a pane), checks each against the dock and its fold button, everything else drawn across the dock's edges, and overlaps across layers, 8 points apart from the least window's 190 (`the_palette_fits_the_right_dock_in_the_narrowest_window`) to 600, the Gallery on every type that has one, with a floor of rows on each; let the list's toolbar not wrap, and it fails. Tests turned to fail where they could not: the W/A/S/D test's Ctrl+W, `the_palette_is_a_panel_not_a_tab`, and waits by frame count. The deferred entry of issues 11 and 20 has its open item back (a tool split off the middle takes the editor area when the last area closes), for M3b's T9 and T10. Pictures: `shell_main`, `shell_760`, `shell_right_dock`, and the new `shell_right_dock_narrow` (420, 460 and 500) and `shell_right_dock_no_game`, in both themes (`target/ui-mock/impl/m3-t5-fix/`).

**T6. The Preview panel; the docks follow the active editor.**

- **Goal.** `preview_window` (`area_tools::preview_window`, area_tools.rs:787) becomes `Panel::Preview`, the second tab of the Right Dock's tab bar beside the Palette's, as the Main board draws them (this sentence first said "under the Palette, stacked, each with its own tab bar", against the board: corrected when T6 was built). The registry rule goes live: the Right Dock shows the active docked editor's panels and is not drawn when there are none.
- **User sees.** Preview on the tab beside the palette's. With a script tab in front (once As Tabs exists), the Right Dock goes away.
- **Files.** `area_tools.rs:787-833`, `commands.rs:901` (`Id::PreviewWindow` brings the panel to the front), `shell.rs`.
- **Tests.**
  - App: Preview shows the palette's blueprint, and its tab shows it and only it; the Right Dock follows the front tab of the focused main leaf, not a floating window's.
  - Migrate `tool(&h, "Preview")`.
  - Picture against Main.png's right column.
- **Manual.** 02 (the Preview button line); 05-blueprints if it names the Preview window.
- **Risks.** `frame_perf` run (the preview's 3D view now draws every frame while shown).

**Done, 2026-11-12.** The plan's sentence above said the Preview goes under the Palette, stacked, each with a tab bar of its own; the board the owner approved (`Main.png`, and Narrow, WideArea, Floating) draws Palette and Preview as two tabs of one bar, the stacked section under them being M4's Properties panel. A stacked build was made first and withdrawn (at its first 240 px a creature's 3D view was left about 20 px); tabs were built, and the sentence is corrected. The Preview window is the Right Dock's **Preview** panel, the second tab of its one tab bar (`Panel::Preview`, `Panel::RIGHT = [Palette, Preview]`, the kit's `DockTabs` with each panel's drawn icon, words or icons alone by the dock's width as the Left Dock's: with no tools on the bar the words fit down to the 190 of the least window; Left and Right on a focused tab go to the other); `Moonglow::preview_window` and the window (`area_tools::preview_window`, a dialog of egui's own) are gone. `Moonglow::right_panel` keeps the tab in front for the run, as `left_panel` does (not in the settings), and a module opens on the Palette; choosing a blueprint never changes the tab. The registry rule is live: the active docked editor (`Moonglow::active_editor`, public: the front tab of the focused main-surface leaf, else of the one focused last, `Moonglow::main_leaf`, else of the middle's; a floating window never counts) brings both panels where it is an area's tab or the `NoArea` placeholder and the game's data is read (`shell::editor_brings_palette`); any other tab in front (Module Properties, a script, a blueprint, the manual, a tool split beside the area) brings none, and the dock is not drawn at all (no strip, no fold button, F6 and Shift+F6 pass over it, View › Right Dock grayed), its fold and width kept for the next area. Asked for by name (Tools › Palettes, Tools › Preview, the strip's buttons) the dock is shown on that panel beside any tab, both tabs in it (`Moonglow::palettes_asked`, which takes T5's `palettes_alone`'s place: one mechanism for no module, no game data and a tab that brings none), until the active editor brings them itself (`shell::follow_editor`, level-based). The Preview draws only while its tab is in front and the dock unfolded: its body is laid out in the dock and drawn after the editor area, where the window was drawn (`Moonglow::preview_slot`, set by `right_dock` only for the Preview in front and taken by `area_tools::preview_panel` in the same frame, on the dock's layer, under its pane in the accessibility tree, cut at the dock's edges), so a Model tab or a blueprint's Appearance page of the same blueprint keeps its view as it did beside the window. Behind the Palette's tab it reads, draws and asks for nothing; brought to the front it shows what the palette has chosen then (its model view, unseen for two seconds or more, is let go of as every unseen one is, so its camera starts afresh). What it shows is unchanged (fields, an item's icon, the 3D view with the model viewer's controls, a store's stock), laid out for a narrow panel: the fields in a list of their own at most a quarter of the panel high where a model or a stock is under them, an item's icon over its fields under 300 points, the model's room never wider than the panel; the model viewer's Animation box goes to a row of its own where it does not fit (`model_view`: the box and its label are one widget, which a wrapping row did not wrap; every model view gains it). At 320 in a 1600 by 900 window the Preview's body is 303 by 788 points: a creature's fields take about 150, the model viewer's toolbar three rows (about 95, its info line, Animation, then Pause to Shadow, then Shaders and Export Files), and the 3D view about 303 by 520. `Id::PreviewWindow` keeps its id (`preview-window`) and so a key given it; it is **Tools › Preview**, a plain row after Tools › Palettes (`Action::ShowPanel(Panel::Preview)`), named and tipped as Palettes and Resource Browser are ("The blueprint chosen in the palette, in the Right Dock"), always enabled as they are. Nothing of the Preview is kept in the settings, so there was nothing to migrate (the window's open state and place were never kept: it was egui's own window, not a tab). Found on the way and fixed: an area opened again (`OpenTab` of a tab open in another main pane) came to the front of its pane without the focus, so the docks stayed the focused tool's (`run_actions` now gives a main-surface tab's pane the focus, as `show_tab` did). Tests: `preview_panel.rs` (what it shows, a store's stock, each tab showing its panel alone and the Preview making no model view behind the Palette, Tools › Preview and the strip, the tabs' form at 320, 280 and 190, the arrow keys on the tabs, the Palette in front as a module opens and after a restart, Escape over the Preview as over the Palette), `right_dock.rs` (the dock following the active editor through Module Properties, a script and the manual docked, folded and width kept; a floating window; two panes; the placeholder; Tools › Palettes and Tools › Preview beside a tab that brings none; nothing of the dock left the frame after it goes; both sweeps once with each tab in front, a blueprint chosen for every palette type, down to 190), `drawing.rs`'s `a_preview_shown_twice_in_a_frame_is_drawn_once` (the tab keeps the view), `frame_perf`'s two new views (an area with a creature, the Preview's tab in front, then the Palette's). Pictures: `screens.rs`'s `shell_preview` (a creature at 320, 280 and in the 420 window, an item and a placeable at 320, folded, no dock beside Module Properties, Tools › Preview beside it) with `shell_main`, `shell_760`, `shell_narrow`, `shell_wide`, both themes (`target/ui-mock/impl/m3-t6/`).

**Review, 2026-11-12.** An independent review of T6 found, and this round fixed: (1) Hide All Panels beside a tab that brings no Right Dock folded the absent dock, which came back folded at the next area: an absent dock counts as hidden, Hide All Panels and its undoing leave its fold alone, and View › Right Dock grayed is not ticked. (2) F6 skipped the Right Dock with the Preview's tab in front: it reaches the panel in front, and with the Preview's (a `Keys::Preview` of its own) the arrow keys turn its model as a drag does. (3) An old open-tabs record naming the palettes asked for them by name, so the dock showed beside the restored tab in front: it unfolds the dock on the Palette without asking. (4) A comment named View › Preview Panel. (5) `Moonglow::main_leaf` could outlive its module: the active editor now reads egui_dock's own record of the main surface's focused pane (kept while a window has the focus, moved as panes close, new with the dock), and a restore of kept tabs gives the middle's pane the focus before the tab in front takes it (the pane made last had it). (6) Tests that could not fail: Escape over the Preview (now: a blueprint in hand stays in hand over either panel and is let go of over the area, as the manual says), the restart half of the front-tab test (dropped), the dropped tool's test (now the two states), `palette_row` (asserts the Palette in front), a wrong comment, and `frame_perf`'s fixed wait. (7) Tab skipped the Preview's body, drawn after the editor area: it is drawn in the dock in its place unless an open tab may show the same model (`model_view::a_tab_may_show`: a Model tab or a blueprint's tab of it, a hak's tab, a tile's), when it is still drawn late so that the tab keeps its view; Tab then goes tab bar, body, the dock's edge, fold button (`tab_reaches_the_right_dock_s_fold_button_after_its_controls`, both tabs). (8) The Preview's view went whole after two seconds unseen: `model_view::let_go_of_unseen` keeps the view of the palette's chosen blueprint (camera, animation, play, lights), its targets let go of as every unseen view's, until another is chosen, the module closes or the game's data is read again. The blueprint is still read and parsed each frame the Preview is in front, as the window did (the timing's 0.75 against 0.52 ms idle, with the Palette's tab in front, is that and the 3D view's frame together).

**T7. Bottom Dock and Status Bar completed.**

- **Goal.**
  - The Log's Status Bar switch shows the **last log line** while closed.
  - Right items: the module's **name** with "(modified)" and its path in the tip (U4), a running job with a bar, the game folder.
  - The notices stay strips under the Top Bar.
  - The Log keeps Copy, Save…, Clear in its panel header (today in its corner).
  - A new installation's Bottom Dock is closed (`new_installation()`).
- **Not in M3.** Problems and Search Results switches arrive with their panels in M4 (dialogs to panels), so no switch without a panel. M3 has the Log's switch only.
- **Files.** `lib.rs:3040-3068` (`status_bar`), `log_ui` (3070), `apps/moonglow/src/main.rs:618-622`.
- **Tests.**
  - Unit: `new_installation()` differs from `default()` only in `open_editors` and `hide_log`; an old RON file without the new fields reads as `default()` with nothing unread; the retired field reads with no warning and no `app.ron.unreadable-*` copy.
  - App: the last line shows when closed; the switch opens and closes and keeps its size.
  - Migrate the log tests (`the_log_s_lines_carry_their_time`, `the_whole_log_is_copied_and_saved`, `the_log_takes_the_keys_only_from_its_own_clicks`).
  - Picture `shell_main` Status Bar against Main-status.png.
- **Manual.** 02 "The log", 13-troubleshooting mentions.

**Done, 2026-11-12.** What T2 had built stayed: the Log switch, the module's name with "(modified)" and its path in the tip (U4), the game folder, the Bottom Dock's Log tab with Copy Log, Save Log and Clear Log on its tab bar (their corner was gone since T2), and the notices, which are still strips drawn between the Top Bar and the docks (unchanged; `updates.rs`'s `the_update_notice_appears_and_is_dismissed` now checks the update notice lies under the Top Bar and over the docks and the editor area). Built: (1) **The log's last line** (the kit's `status::StatusLog`, the board's `.sb-log`): while the Bottom Dock is closed, after a separator beside the Log switch, the last line's message without its time (the board has none), in 11 px `text-3` for a plain line (the board's color for it; the log's own plain color is the body text's), `warn` for a warning and `error` for an error, its first line where it has several; cut with an ellipsis to the room the bar's middle has, never under the right-hand items, its time and first lines in its tip, a button named by its first line (since the review: bounded, and named "Last log line: …"); pressed, it opens the log; not shown while the dock is open, nor with nothing said, nor in less than 48 points (`shell::LAST_LINE_LEAST`), and with nothing to show no room is kept. Neither the board nor SPEC draws a count or a dot of unread warnings on the switch, so there is none. (2) **A running job** (the kit's `StatusJob`, now with a mark of a quarter of its bar going to and fro where the job's progress is not known (`StatusJob::unknown` since the review), `max_width`, `name` and `tooltip`): every job the window runs is `Moonglow::job` (`jobs.rs`: Build Module, Compile All Scripts and Compile Scripts, Compile Models, Verify Module, the Round-Trip Check, What the Module Uses, Attach haks, a hak saved, Extract Files, Export as Files, Import Files, an NWSync publish and upload, a project's and a plugin's commands), one at a time: a second asked for while one runs is refused, the log saying "…: waiting for … to finish", so two never show. It stands before the module's name: its title and its bar, named "Compile All Scripts, 40 of 312" (or "…, under way"), its part done the accessible value, its steps and what it says it is doing in its tip. Its Cancel stays in the job's window (the board draws no stop control in the bar), which dims the window while the job runs, the bar's item with it. Test Module is no job (it starts the game and returns), and an area being read or the look at the module's files keep their own notes. While a job's progress is not known its mark asks for a frame every 50 ms, as `poll_job` already did for the job, and nothing once it ends. (3) **The bar's order of yielding** as the window narrows (corrected by the review below): the last line is cut short down to its least room (48 points); then the game folder goes (SPEC section 12: the folder drops before what is in the middle), and the line widens again; then the line is cut again and goes; then the job's name is cut short (its bar kept); then the note of a look under way and the module's name share what is left as before (`share`), the name last. (4) **`Settings::new_installation()`**: the defaults with `hide_log` set (T10 adds the Open editors switch as its second difference); `Settings::read_eframe` gives it where the settings file does not exist, the desktop app's first start; a file that exists, reads or not, or names no key, gives the defaults (an existing user's: the log open), as does a start with no storage folder. 4.1.0's settings (as RON and as JSON) read as the defaults with nothing unread and no `app.ron.unreadable-*` copy; no field was retired since 4.1.0 (three were added: the docks' sizes), so the "retired field" is tested as a field this version does not have, which reads with no warning. (5) **The Bottom Dock's tab bar in a narrow dock** (taken from T14's list): the Log tab shows its icon alone where its words and the tools do not fit side by side, the tools give way whole from the end (Clear Log first, each still in View's menu), and a dock narrower than the tab's icon (24 points beside both side docks in the least window) draws its bar empty; the tab and the tools never overlap or leave the dock. Also, the carry-over from T6's review: `drawing.rs`'s `a_blueprint_s_appearance_page_keeps_its_view_beside_the_preview` (a creature's editor on its Appearance page showing the model the palette's chosen blueprint has, docked and floating: the tab keeps its view, the Preview says it is shown elsewhere). Public additions: `mg_uikit::status::StatusLog`, `JOB_MARK`, `JOB_MARK_PERIOD`, `JOB_MARK_FRAME`, `StatusJob::{name, tooltip, max_width, width, is_cut, mark_offset}`, `mg_uikit::shell::DockTabs::tabs_width`; `mg_ui::Settings::new_installation`, `Moonglow::bottom_dock_rect`, and `Moonglow::start_job_for_tests` (hidden, for tests and pictures). Tests: `status_bar.rs` (the last line shown and gone, its colors in both themes, cut with its tip, pressed, nothing kept without one; the Bottom Dock's height through the switch, a restart and Reset Layout; a job's item with and without progress, its end, its failure and a second job refused; a project's command (`sleep`) shown as a job and gone when canceled; an idle window's frames; a new installation's window; the bar swept from 420 to 1600 by 8 with a long name, a long last line, a job and a long game folder, nodes and drawn texts inside the bar and apart, the name always there and the order of yielding; the Bottom Dock's tab bar swept from 420 to 1000 beside an area with both side docks), the settings' unit tests, and the kit's `a_job_of_unknown_progress_has_a_moving_mark` and `the_log_s_last_line_is_cut_short_and_pressed`; the three log tests needed no change (they found the tools by name already). Pictures: `screens.rs`'s `shell_status` (the last line, a warning, a job with progress and one without), `shell_status_narrow` (420, 640 and 1000 with long texts) `shell_new_installation` and `shell_bottom_dock_narrow` (the dock open beside an area at 420, 560 and 640; T6's `shell_preview_420` is the picture before, its Log tab over the tools and cut), with `shell_main` and `shell_760`, both themes (`target/ui-mock/impl/m3-t7/`); the kit sheet shows a last line and a job of unknown progress. Seen in the pictures and left: the job's own window draws its progress bar's words ("40 of 312") in the bar's light text over the light track in the light theme, half unreadable (the job window's egui `ProgressBar`, M4's dialogs); in a 420 window the log's lines wrap a character a line in its 24-point dock (T14's squeeze).

**Review, 2026-11-12.** An independent review of T7 (committed as `dddc0a21`) found, and this round fixed: (1) The order the Status Bar yields in was described wrongly here, in the manual, the log, `status_bar`'s comment and the sweep's (and in T7's commit message, which stays as made): the code cuts the last line to its least 48 points, then drops the game folder (the line widens again), then cuts and drops the line, then cuts the job's name, the module's name last, as SPEC's section 12 has the folder go before what is in the middle; every description now says so, and the sweep asserts it (the folder never shown with the line under its least room; a width where the folder has gone and the line has widened). (2) A blank last entry (a project command's empty output line, `"  "`) drew a focusable button with nothing in it and took the line's room: the last line is now the last of the log's last 64 entries (`shell::LAST_LINE_LOOK_BACK`, a fixed walk: the log has no revision to key a cache by) whose first line is not blank; with none, nothing is drawn and no room kept. (3) The whole entry was copied and named each frame (a 4 MB entry gave a 4,000,000-character accessible name): only the first 4 KB of an entry is read for its first line, cut to 300 characters with an ellipsis (`LAST_LINE_CHARS`); the tip (its time, its first 8 lines each cut, "… more in the log") is made only in a frame where the line is hovered or has the focus (`Moonglow::last_line_tips`, hidden, counts it for the test). (4) The Bottom Dock's sweep now asserts each stage by what the tab draws (its words, or its icon alone, the tab named "Log" in both), in order of the dock's width: words with the three tools, the icon with the tools going one by one from the end, the icon alone, nothing under the icon's width; and that a tool not shown is in the View menu. (5) The last line is named "Last log line: …", so a message reading "Log" no longer shares the switch's name. (6) The tests' frame-count waits went (a deadline for the job window's Cancel; `settle`), and the real job's `sleep` is killed when the test ends whatever happens (a guard reads the PID the command wrote and kills it if it is still `sleep`). (7) **Reset Layout gives the shell's own layout** (the coordinator's decision, reversible on the owner's word): the side docks unfolded at their first widths and the Bottom Dock closed, as a new installation has it, its first height kept for when it is opened; it had opened the log (the layout from before the shell). (8) T14's list has the 24-point Bottom Dock of the least window; the update notice is checked against the Left Dock's and the Bottom Dock's tops too; `StatusJob::new(_, None)` keeps its meaning (an empty bar, no frames asked for) and the moving mark is `StatusJob::unknown`, as the kit's items are not changed under their callers; a first start with no game found shows "No Neverwinter Nights installation found…" as the last line (`a_first_start_without_the_game_says_so_in_the_last_line`), as the troubleshooting chapter says. The scratch folder `target/test-output/ui-area-zz-debug` (an `area.mod` of 06:17 that day) is not from T7's runs and was left.

**T8. Quick Open.**

- **Goal.** The Top Bar field "Go to resource or command" (Ctrl+P key cap; at most 440 px) with a popover of results:
  - the module's resources (by ResRef and, for areas, by name), the read-only load-order resources, the commands, plugin entries and project commands;
  - empty field: recent areas and recent modules' areas first.

  Ctrl+Shift+P and Help › Command Palette… open it restricted to commands (U7). The modal `palette_window` goes.
- **Files.** New `quick_open.rs` (an index cached by `ws.revision()` and the load order's generation; `Needle` filter; at most 50 rows with `show_rows`). `commands.rs` (`found` reused; a new `Id::QuickOpen` in the `Id::all` extras, as `FullScreen` is), `keys.rs` (`Cmd::QuickOpen`, Ctrl+P, id `quick-open`), `lib.rs:3032-3034` (`shortcuts` waits while the field has focus).
- **Tests.**
  - Unit: ranking (module before game, name before ResRef); an empty query shows recent areas; the index is rebuilt on revision change only.
  - App: Ctrl+P, type "formosa", Enter opens `m1q3_formosa_9`'s tab; Ctrl+Shift+P finds Build Module and runs it.
  - Migrate `the_command_palette_finds_a_command_and_runs_it` and the 14 palette lines; `the_keys_that_work_anywhere_are_all_commands` holds with the new id.
  - Picture `screens::command_palette` becomes `quick_open`, against Main.png.
- **Manual.** 02 "Keyboard shortcuts" (147) and "Menus"; **17-plugin-api.md:274** ("the Command Palette" becomes "Quick Open"); 14-coming-from-aurora if it names the palette.
- **Risks.**
  - A user who bound Ctrl+P keeps their key (Keymap's conflicts).
  - Index cost on a large load order: build lazily on first focus, off the frame if it exceeds 2 ms (time it).
  - `frame_perf` not needed unless the field draws results at rest.

**Done, 2026-11-12.** Quick Open is `crates/mg-ui/src/quick_open.rs`, the Top Bar's middle (`shell::top_bar_row` hands the kit's slot to `quick_open::middle`; `quick_open::after_bar`, called by `Moonglow::ui` right after the Top Bar, draws the popover and does what was chosen). **The field**: the glass, "Go to resource or command", the key cap of the key actually bound (`Cmd::QuickOpen`, id `quick-open`, Ctrl+P; none where it is unbound; said as the field's description), at most 440 points (300 under W 1300), named "Quick Open". A click, typing or the key opens it (bare focus by Tab does not, so Tab passes through to the build actions); Ctrl+Shift+P and Help › Command Palette… (the same ids) open it with `>`, which restricts it to the commands (SPEC names no other prefix). **What it finds**, one list best first: the module's resources (by ResRef, and an area's or a blueprint's name, as the Module panel's Filter; the objects placed in an area, which the Filter also looks through, and tags are not matched), then the commands (`commands::found(app)`, every row the palette had: the menus' commands, plugin commands and entries, project commands, grayed where they cannot run, a tip saying why: `Id::why_disabled`), then the load order's resources (the game's, haks', override's, the module's own layer left out; they open as the Resources panel opens them: `browser::tab_for`). The rule, in the module's comment and held by unit tests: equal, then prefix, then a word's start (after a character that is no letter or digit), then anywhere, then every word apart, then (commands only) with the menu's name; at one tier a name match before a ResRef match, then the shorter name, the name and the ResRef alphabetically, and of one ResRef the file with an editor first (a script's source before its compiled form). At most 50 rows (`MOST`); a line under them, always in sight, says how many more ("N more: type more to narrow them"), and "Indexing…" while the index is made. Each row: the kind's icon, the name, a dim detail (the ResRef and type of a named resource, a command's menu, a load-order resource's layer, a recent module's folder), a command's key cap. **Empty field**: the module's recent areas (`Settings::recent_areas`), most recent first, still in the module; recent modules' areas are not offered (they cannot be opened without opening their module); with no module open, the recent modules (opened as File › Recent Modules opens them, asking about unsaved work) and the commands that work without one. **Keys**: Down and Up move the choice with wrapping, Page Down and Page Up a page (stopping at the ends), Enter runs it, Tab stays in the field while it is open, Escape closes it and gives the keys back to the widget that had them (recorded in `keys_pressed` before a typed-in field gives them up; a pane's arrow keys are untouched, so the area view keeps its keys), and keeps a dialog under it open (`widgets::keep_escape`). A press outside closes it running nothing; a press on it is neither the area view's nor work in an editor under it (it lies on a Foreground layer); the areas opened lately are named at once where the index has not come to them, and frames are asked for while it indexes, none at rest. While the field has the keys it is a text field as any: a key that types is its (a command given a plain letter waits); a key with Ctrl or a function key runs its command (Ctrl+S saves the module, as in any field) and closes Quick Open. A job closes and grays it. **Ctrl+P and a builder's own key**: `Keymap::give_way` (called by `Keymap::new` and after plugins' and projects' keys are taken in) leaves Quick Open without Ctrl+P where another command that works at the same moment was given it by the user, a plugin's manifest or a project's file, unless the user chose Quick Open's keys too (then both have it, a conflict in Options as usual); the key it gives up is recorded as its own for the while, so the settings never keep it unbound. **Yields** (taken from T14, local to the Top Bar): the field narrows with the room the bar leaves it, is capped at 300 under W 1300, and becomes an icon button (the glass, "Quick Open", its key in its tip) where it would be under 240; then the build actions go, then the icon, then the quick actions. Under the icon (or with no room at all, as at 420), the key and the icon open the field over the bar's middle (300 wide at most, inside the window) with the same popover. **The index**: made the first time Quick Open opens; the module's part again when the workspace's revision, its count of resources or its file changes (`ModuleStamp`), names taken 2 ms a frame (`BUDGET`) where the window works in the background; the load order's part again when `Moonglow::load_order` moves and its layers but the module's (label, fingerprint) changed, made on a thread in the application (it holds the game data only while it reads, about 200 ms). The load order's resources are searched as one text (`Game::find`, with `memchr`'s `memmem`, already in the tree through aho-corasick): on the game with TDoD's 150 haks (382,394 resources, a 1-minute load of 8 on this machine) the index takes about 200 ms and its search at most 1.9 ms ("a", 138,306 found), 0.08 to 0.3 ms for longer queries; a whole keystroke there with Chapter 1 open (the popover's rows made, commands and all, nothing narrowed from before) 2.1 ms for "a" and 0.19 to 0.44 ms for the rest; Chapter 1's module part 0.7 ms, its names 43 ms in all; a made-up module of 50,000 named resources 1.1 to 1.6 ms a keystroke (searched entry by entry), so a first one-letter keystroke on both would be about 3 ms. The load order's part holds about 55 MB for those 382,394 resources once Quick Open has opened (an estimate from the entries' size, not measured). At rest Quick Open draws its field and nothing else. **Gone**: the modal `commands::palette_window`, `commands::Finder` and `Moonglow::command_palette`. **What builders notice**: Escape closes at once (the palette's first Escape cleared what was typed); a key with Ctrl runs its command while typing (the palette held every key back); `>` with nothing after it shows the first 50 commands and how many more (the palette listed every one); a command's detail is its top menu. Tests: `quick_open.rs`'s unit tests (the rule case by case, groups, folding of accents and Cyrillic, spaces, long queries, `>`, 50 and the rest, narrowing, the load order's search against the rule, the builds counted on a revision, a resource put in, a new load order, and a load order changed only in the module's layer), `keys.rs`'s `quick_open_gives_ctrl_p_to_a_command_given_it`, and `tests/app/quick_open.rs` (a resource by name, recent areas, 50 and more, the arrows and Page keys, Ctrl+Shift+P and Help's row, Escape back to a field and to the area view's camera, Escape and a press outside with a Properties window open, a grayed command, the single keys, no module open, the user's Ctrl+P and a rebound cap, a job, the index's builds, Chapter 1's `m1q3_formosa_9` and a read-only game resource with the game installed, the field's stages swept from 1600 to 420 by 8 against the room the bar leaves it, the popover inside windows from 420 to 3440 and 320 tall, the icon at 760, Tab through the field, background indexing, and a press on the popover over the area view); `the_command_palette_finds_a_command_and_runs_it` and the palette lines of `plugins.rs`, `projects.rs`, `resources_panel.rs` and `nui.rs` migrated; the tests that took the window's first text field take the first that is not Quick Open's. Pictures: `screens.rs`'s `quick_open`, `quick_open_no_module` and `quick_open_widths` in place of `command_palette`, with `shell_main`, both themes (`target/ui-mock/impl/m3-t8/`). Differences from the plan: typing "formosa" puts the creature blueprint named Formosa first (an equal name), so Enter opens it, not `m1q3_formosa_9`'s editor; the test types `formosa_9`.

**Review, 2026-11-12.** An independent review of T8 (committed as `6d307f99`) found, and this round fixed: (1) Ctrl+Z in Quick Open undid the module's last change (Undo was left to a text field only once typed in): while the field has the keys, the window's commands are on one side of one rule, `quick_open::runs_from_field`, an exhaustive match over `commands::Id` (a new command must be put on a side): saving, building, testing, panels, views, tools and help run (Ctrl+S, F7, F9, F1...); what changes or closes the module, a resource or a tab waits, its keys left to the field (Undo and Redo, New, Open and Close Module, Import, the wizards, New Script, Close Tab and the tabs' and panes' keys, Reload Resources, Clear Log), as do the plugins' and the project's commands; opened with `>` or nothing, the field's own undo starts there. (2) The load order's part had kept only the copy the game reads, so a game resource one module overrode was missing in the next: it is now made of every layer but the module's (the highest copy of each), a resource the module has too shown once, as the module's (a bit set per load-order row, worked out when the module or the part changes), and kept across a generation whose layers but the module's are the same. (3) A file's name with its extension is found (`hello.nss`, `nss` in it at a word's start), a file-name match ranking after a name's and a ResRef's of its tier, and a last word naming a type narrows (`.utc`, `hello nss`); a unit test holds Quick Open to the Module panel's Filter and the Resources panel's search case by case (it finds all either finds). (4) A build's part is taken whenever it arrives if its layers are those of now; (5) a build gives up at once when the game's data is to be let go of (`release_game_data`), the module closes or the window ends (a flag it looks at every 4,096 resources, joined on drop), begun again on the next open; (6) the load order's part is one text of file names searched with memchr and entries of at most 40 bytes (22.4 MB for 382,394 resources, from about 55 estimated before), let go of when its layers change or the game's data is read again, and not kept without game data; kept while a module stays open, so that Quick Open opens again at once; (7) within a group, resources with a view of their own (a tab of their kind, a 2DA, a text: `has_own_view`, from `Tab::for_resource` and the Resources panel's view) come before those shown only as their bytes, and a module resource with no editor opens read-only rather than not at all; (8) the popover's 8-point margin and the background indexing are asserted (the latter with the build held back by a test's hook, `QuickOpen::hold_builds`); (9) the module's stamp is its revision alone (`Workspace::adopt` advances it; the count of resources guarded no path the window takes); (10) each row's detail is its accessible description; (11) grayed rows' tips say what is missing, and Options › Keyboard gives way on the draft as Apply will, Quick Open's row saying "No key while … has Ctrl+P"; (12) while an input method composes, Enter, the arrows and Escape are its, and a menu opened by its key closes Quick Open (the menus open by Alt and their letters and F10 in the least window). Costs after it (1-minute load 5 to 7): the index in 105 ms (from 196), its search 2.05 ms for "a" (186,270 found, the extensions' letters now counted, from 1.85) and 0.10 to 0.33 ms for the rest, a whole keystroke 2.24 ms for "a" (from 2.24) and 0.16 to 0.43 ms otherwise, a module of 50,000 named resources 0.70 to 1.58 ms (from 1.2 to 1.5; the module's single words are searched in one text now, as the load order's are).

**Merge point M3a, after T8.** Coherent:

- The toolbar is gone and every one of its buttons has a home; Top Bar, Left Dock (Module, Resources), Right Dock (Palette, Preview), Bottom Dock (Log) and Status Bar exist.
- Editors still open as today (floating unless moved).
- The manual is true. Tick the Main Toolbar and Window rows of `ui-redesign-moves.md` that are done (all but Start's compass button, which is M4).
- `frame_perf` against the T0 baseline logs.

##### M3b: Editors in the shell (T9 to T11; merge point after T11)

**T9. Document tabs and editor groups.**

- **Goal.**
  - Document tabs with a kind icon (`icons::object` and editor icons), the name, a **modified dot** replacing " *" (script, talk table, side editors' dirty state), and a close button shown on hover and on the active tab.
  - The 2 px accent line kept (`dock_look::marks`), `line-soft` in an unfocused group.
  - A **Split** button at the tab bar's right makes a second editor group from the front tab.
  - Groups are main-surface splits stored as `split right 500`, so older versions read them.
- **Files.** `tabs.rs:536-609` (`title` with room left for the icon), `dock_look.rs` (paint icon and dot in `marks`), `lib.rs` (`middle_pane`, `split_of`, `split_middle`, `keep_middle` generalized from "the leaf with an area" to "the editor groups"; `NoArea` stays the empty editor area's state).
- **Tests.**
  - App: the dot appears on an edited script and goes on save; Split makes two groups and the docks follow the focused one; the tab keys (Ctrl+Tab, Ctrl+digit, Ctrl+W) work per group.
  - Migrate `tabs_by_the_keyboard`, `a_tool_window_reopens_in_its_split_of_the_middle`, `a_tool_window_opens_where_its_kind_was_last_docked`.
  - Picture `shell_wide` (a script in a second group) against WideArea.png.
- **Manual.** 02 "Tabs and windows" (450-622, the tab part).
- **Risks.**
  - `frame_perf` run (icons on every tab).
  - Accessibility: the tab's name stays its title without " *". Tests that read titles with " *" (only the OS window title, modules.rs:49, which stays) are unaffected.

**Done, 2026-11-12.** **Plan against board** (the boards won where they differ): (1) Split Editor is not grayed with one tab, it is absent, as the boards draw it (Welcome's one tab, WideArea's second group): it stands at the right end of the focused group's bar only (`Moonglow::focused_group`, the main surface's focused pane even while a window has the focus, as Floating draws it on the main group behind the window), where that group has two tabs or more and its bar is at least the button's room wide (28 points: the least window's editor area, both docks open, is narrower until T14); View › Split Editor is grayed with one tab, saying why. (2) Its name is "Split Editor" (the decision's), its tip "Move the tab in front to an editor group of its own" (the board's label, "Split the editor area", is its tip there). (3) The board's tabs have no leaf buttons on the main surface (no arrow, no close all) and keep them on a floating window's bar (its collapse arrow, Close): egui_dock decides both for every bar or none, so it is vendored with two additions (`vendor/egui_dock`, `MOONGLOW.md`: `DockArea::main_surface_end_room`, which also leaves `tabs::SPLIT_ROOM`, 28 points, free at each group bar's end, and `TabViewer::on_tab_dragged`), patched in by the root `Cargo.toml` and called from `mg-ui` alone (the viewer takes the kit by git tag). (4) The dot goes after the name and before the close button, the tab 13 points wider with it (the board's flex). (5) The close button's room is kept on every tab whether shown or not (`visibility: hidden`). (6) Welcome's tab is drawn with a close button on the board; it is still not closeable (closing it would leave the window empty with no way back), a difference left. (7) The boards say nothing of a crowded bar (`.doctabs` cuts what passes its end): built as decision 5 asks. **Built.** Each tab's look is laid out before the dock is drawn (`tabs::prepare`, `Moonglow::doc_tabs`): its kind's icon (`tabs::icon`, an exhaustive match over `Tab`: the resource type's icon as the Resources panel draws it, `browser::kind_icon`, a placed object's by its list, the editor's own for Module Properties (`module`), the manual (`book`), Options (`settings`), Advanced Search, the Placeable Gallery, Tile Properties, a plugin's panel (`plugin`: the plugin API names no icons)), its name (`tabs::name`: the title as before, with no star), and whether its own document has unsaved changes: a script's, the talk table's and the side editors' own state as the star said, and for an area (its ARE, GIT and GIC), Area Properties, a conversation, a blueprint, a raw GFF, a NUI window, Module Properties, the factions and the journal, whether its resources differ from the module as saved (`mg_edit::Workspace::differs_from_saved`, new: a document edited is compared by what it holds with the saved copy, read once and kept until saved again, so an edit undone is not a change; asked again only when the resource changed, `ResourceStamp`; on the game's largest GIT, Tyrants of the Moonsea's 1.3 MB `zhentkeepwar`, in release, 1.1 ms after a change to its last placeable and under a microsecond after one to its root, where writing it out and reading the saved copy each time took 3.2 ms: `doc_tabs::an_area_s_dot_cost`, by hand). A placed object's Properties show no dot (their document is the area's, whose tab shows it). The title is a galley with the icon's room before it and the dot's and close button's after it (`mg_uikit::dock::doc_tab_title`), so egui_dock makes the tab `doc_tab_width` wide; titles are cached by what they are laid out from (`LookCache`, with the display's scale and `script_layout::fonts_made`), and the words drawn in egui_dock's colors for the tab's state. After the dock is drawn, `dock_look::doc_tab_parts` paints each tab's icon (`text-3`, `text-2` in front), dot and close button (`mg_uikit::dock::doc_tab_marks` over `marks`, whose lines it keeps) and registers the close buttons, on the tab in front of each pane and the tab under the pointer (where no window lies over it), named "Close " and the tab's name, tipped "Close this tab", closing as the menu's Close does (`CloseTabs`, asking where it asks, the tab brought to the front while asked as egui_dock's close did); a middle click closes a tab as before. Each tab is a node of role Tab named by its name, selected where in front, its description "Modified" with the dot; the OS window title keeps its star. The tab menu's Close is Moonglow's own (first of the close rows: egui_dock's went with its close buttons); Eject follows. A crowded bar cuts every title alike to fit (`mg_uikit::dock::doc_tab_cap`), never under `DOC_TAB_LEAST_TITLE` (64 points, about eight letters), the whole name in the tip of a tab cut; past that egui_dock scrolls the bar by the wheel, its own scroll bar off (`doc_tabs_style`: 7.5 points of egui's colors under the bar), and a pane whose tab in front changed scrolls it into sight (`Moonglow::reveal_fronts`); no list of tabs (the boards draw none). The drop targets are in the theme's colors (`doc_tabs_style`), and the tab being dragged keeps its icon, dot and close (`on_tab_dragged`, on the layer that follows the pointer). Found on the way: as the window opened (the Welcome tab, before any click) the editor area's pane had `line-soft` over its tab, egui_dock having no focus yet; the focused group (`focused_group`) is given to the marks, so it has the accent, as the Welcome board draws it (`doc_tab_marks` takes the pane with the focus). **Groups.** `Moonglow::editor_groups` (depth first, left to right), `middle_pane` (the group of the areas or the placeholder, else the first group), `split_of` (a group's side of the middle and share, walking up to the part of the main surface whose other half holds the middle: three groups in a row are each `split right N`), `split_middle` (splits the middle), and `group_off` (a kind kept `split right 500` opens in the group there, not one more split each time). Split Editor (`Action::SplitEditor`, `commands::Id::SplitEditor`, View › Split Editor, Ctrl+\\ by default, id `split-editor`; `Moonglow::can_split`, `split_editor`): one group: the tab in front to a new group at its right, half the room; two or more: into the group after it (the one before, for the last); the focus goes with it, so the docks follow (`active_editor`). A group that loses its last tab goes (egui_dock's); `keep_middle` now says what it is for groups: the placeholder only where no tab is left. The tab keys act on the focused group (`tab_pane`: the group with the focus whatever its tabs, a window with the focus where it has two or more; Ctrl and a digit too, `number_tab`, which went by the middle before). F6 keeps the editor area one stop (the focused group's tab in front), and the manual's row stays true. The open-tabs record keeps each group with its place as 4.1.0 reads it (`split right 500`), and where there are two groups or more the middle's pane also carries the groups' tree (`split`, as a window's), which this version lays out first so three groups come back in order; read without it (4.1.0's way) two groups come back as they were. Kept panes' tabs are all found before any pane's tree is read (a tree names other panes' tabs). **Decision 8.** The open item of the issues 11 and 20 entry is settled and taken out of the entry (which stays for issue 21's window drop): a tool's group left alone takes the editor area, as any group does, and that is now right (the boards' editor area holds editors alone; the placeholder shows once no tab is left). The issue 10 entry's note on the middle's arrows is updated (gone; a window keeps its arrow). The entry count stays 233. **What a builder notices.** Tabs carry their kind's icon and a dot for unsaved changes (an area's too); the close button shows only on the tab in front and under the pointer; the arrow at the left of the main pane's tab bar and the close-all button at its right are gone (Close All is on the tab menu; a pane split above or below can no longer be folded to its tab bar, its seam dragged instead); Split Editor; Ctrl and a digit act in the focused group; a crowded bar cuts titles rather than scrolling at once; a kind kept beside the middle opens into the group there. **Tests.** `doc_tabs.rs` (new, 16 and two by hand: the dot for a script, an area (undone, saved), a 2DA and the talk table with the name unstarred and "Modified" said and the window's star; the icons of an area, a script, a creature and Module Properties by the icon recorder; the close button in front and under the pointer, its room kept, its tip, closing its own tab, a middle click and the menu's Close; Split Editor one group to two and back, absent with one tab, a group emptied, the placeholder; the Right Dock following the focused group (game data); the tab keys per group; two groups kept and read the oldest way; three groups in order; a tool's group left alone; thirty tabs in 800 points; a title cut with its tip; the bar swept from the 420 point window with both docks open to 1600 with 1, 3 and 12 tabs, every tab as it takes the pointer between the bar's left end and Split Editor's room, the buttons apart; a floating window's tabs; a kind kept beside the middle; a tab that asks brought to the front; by hand, a frame's cost with 32 tabs and the area dot's), `tabs.rs`'s `every_kind_of_tab_has_its_icon`, the kit's four dock tests (the title's room, a crowded bar's cap, the parts painted, the accent line with the focus given), `mg-edit`'s `a_resource_differs_from_saved_until_saved_or_undone`. Migrated: `click_tab_close` (hovers first, finds the 16 point close in its tab), `overlapping` (a close button in its own tab), and tests that found a tree row or a tab by a name both now have (`tree_row`, the new `doc_tab`): areas, plugins, tables, tree, module panel; Quick Open's "editor" now lists Split Editor first. `tabs_by_the_keyboard`, `a_tool_window_reopens_in_its_split_of_the_middle` and `a_tool_window_opens_where_its_kind_was_last_docked` pass as they were (their doc comments name the group tests). None retired. **Guards broken once each** (and put back): the dot (`own_unsaved` said false: the script's dot test failed, "said_modified(&h, \"hello.nss\")"); a script's icon (`File`: the unit test and the icon recorder's test failed); the close button shown on every tab ("not under the pointer"); the groups' tree not written (three groups came back s02, s01); the bar not scrolled to its front tab (the 30 tabs and the sweep failed, a tab under Split Editor); Split Editor a no-op (three tests: one group to two, the groups kept, the keys per group); `differs_from_saved` false once flushed (the `mg-edit` test, "flushed into the module, not saved"); the cap never above its least (the kit's crowded-bar test); no room kept for Split Editor (the sweep: "a tab at … in …", the 30 tabs); Ctrl and a digit on the middle again ("its second tab"); `group_off` passed over (the scripts in two groups); the cut title's tip dropped ("its whole name in its tip"); a window's tabs given no parts (the icon recorder's and the floating window's tests); the tab that asks not brought to the front ("in front while asked"); the accent with egui_dock's focus alone (the kit's `the_accent_line_follows_the_focus_given` only: no app test reads the line's color, the picture `shell_welcome` shows it). **Cost.** A frame of the window with 32 tabs against 2 (release, a load of 4 to 8): before T9 252 and 255 µs against 203 and 208, after 278 and 277 against 214 and 211 µs: about 20 µs more for 30 tabs, the icons tessellated each frame (T15's cache decision), the titles from the cache. `frame_perf` and `editor_perf` were compiled, not run (the 1-minute load was 4.5 to 13 all day). Left as it was, a difference from the boards for T14 or T15: the editor area lies inside egui's central panel's margin (about 8 points), so its tab bar stands 9 points under the Top Bar and 8 in from the docks, where the boards put it flush (the before pictures the same); the tabs' widths are the boards' to within 5 points (`shell_groups_left` against `Main.png`: 144 against 145 for the area's tab with its dot, 156 against 161 for Faribeth's, the fonts' measures). The before pictures in Light were written over by the after run (the light pictures went to the worktree's own `target/test-output` whatever `MOONGLOW_TEST_OUTPUT` said: `screens.rs`'s `picture_dir` now follows it). Pictures: `target/ui-mock/impl/m3-t9/` (before, dark, light): `shell_main`, `shell_wide` (the board's tabs, a script in a second group), `shell_floating`, `shell_narrow`, `shell_welcome`, and `screens.rs`'s new `shell_doc_tabs` (`shell_groups_left`, `_right`, `shell_tabs_modified`, `shell_tabs_hover`, `shell_tab_dragged`, `shell_tabs_30`).

**T10. The Open editors switch and stored places (R1 and R3, ruled).**

- **Goal.**
  - Options › General › Editors: **Open editors: As Tabs | As Floating Windows**, with its note. Under it: Open Properties as tabs of one window, Closing a window cancels its changes, Move windows by dragging anywhere.
  - As Tabs opens every editor in the focused group; areas always as tabs.
  - **"This kind goes to a window" is stored** in As Tabs mode (found-on-the-way 5): `remember_window_places` stops skipping `Place::Window` (lib.rs:2187-2189).
  - Eject, or a drag out, stores it; View › Reset Layout forgets it.
  - Escape on a docked tab with OK and Cancel follows R1; plugin panels (`Tab::PluginPanel`) follow R3.
- **Files.** `settings.rs` (`open_editors: OpenEditors`, default `Floating`, `new_installation()` gives `Tabs`), `options.rs` (General page, Editors section; the draft field), `lib.rs:2123` (`place_for` default by mode), `lib.rs:4270-4319` (`OpenTab`).
- **Settings migration.** An existing file has no field and reads `Floating`; no file reads `Tabs`. Stored places keep their meaning in both modes.
- **Tests.**
  - Unit: defaults and the stored "window" place.
  - App, each in both modes: a script opens as a tab or a window; Eject in Tabs mode makes the next script open as a window; Reset Layout clears it; Properties gather in Floating mode only.
  - Migrate `editors_open_in_windows_over_the_area` (keeps Floating) and `properties_and_conversations_open_as_tabs_of_one_window`, `scripts_open_as_tabs_of_one_window` (Floating).
  - Both modes for the existing families. Because `Settings::default()` stays As Floating Windows, the mode every new installation gets (As Tabs, log closed, the docks following the active tab) would otherwise be covered only by new tests. T10 therefore runs these existing test families in both modes, through a shared helper that builds the app with `Settings::new_installation()`: closing a tab with unsaved changes and its Cancel, Escape (per R1), the open-tabs records written and read back, the tab keys per group (Ctrl+Tab, Ctrl+digit, Ctrl+W), and the stored places of kinds.
  - Picture `options_window` against Options.png's Editors section.
- **Manual.** 02 "Tabs and windows" ("Everything else opens in a window of its own…" rewritten for both modes); 10-options (the new switch, the three settings).
- **Risks.** Escape as Cancel (R1). Without an answer, As Tabs users lose Escape on Properties and Options tabs: say so in the manual either way.
- **Inherits from T9:** the editor groups exist (`Moonglow::editor_groups`, `focused_group`, `middle_pane`, `split_of`, `group_off`): "the focused group" for As Tabs is `focused_group`; a kind kept `split right 500` opens into the group off that side where there is one (`group_off`), so a kind's stored place names a group, not one more split; `remember_window_places` notes a tab moved by Split Editor as any move (a script split off is kept `split right 500`). The tab keys per group are tested in `doc_tabs.rs` in the default mode only: T10's run of the families in both modes takes them in.
- **Inherits from T7:** `Settings::new_installation()` exists (the defaults with `hide_log`), used by `Settings::read_eframe` where no settings file exists; its unit test `a_new_installation_differs_from_the_defaults_only_in_the_log_closed` asserts the one difference and says T10 adds `open_editors`. The shared helper above builds on it; `status_bar.rs`'s `a_new_installation_opens_with_the_log_closed` is the window's test of it.

**T11. Floating windows and dialogs.**

- **Goal.**
  - Floating editors get the border, 4 px radius and `--shadow`, a bar row (collapse arrow, tabs with ×, Maximize, Close) and a resize grip, keeping `window_bars`.
  - Dialog titles are **left-aligned in a bar row**: `widgets::centered` (widgets.rs:3212) draws its own bar with `title_bar(false)`, with drag by the bar and a Close named "Close". Known difference "window titles centered", settled here.
  - Dialogs open centered in the editor area, so **wizards no longer cover the Welcome subtitle** (known difference, settled).
  - **Remembered window sizes** taken before the theme are scaled once by `Tab::taller()` of their kind (a marker field `window_sizes_scaled`). Known difference, settled.
- **Files.** `lib.rs:2785-2845`, `widgets.rs:3212-3265`, `tabs.rs:100-242` (a `taller_of_kind(&str)`), `settings.rs`.
- **Tests.**
  - Unit: the size migration runs once; a missing marker means unscaled sizes.
  - App: a dialog drags by its bar and closes by its Close; `a_window_is_maximized_by_its_key_and_button_and_stays_so` and `windows_remember_their_size_and_maximize` still pass (migrated where they read sizes).
  - Pictures `shell_floating` against Floating.png; `dialog_windows` and `script_wizard_pages` against Dialogs.png's title rows; `shell_welcome` with the Area Wizard open (the subtitle in sight).
- **Manual.** 02 (window bar paragraph).
- **Risks.** `click_window_close` (common.rs:811) finds egui's close today and must find the new one; about 40 helper uses run through it.
- **Inherits from T9:** a window's tabs have the document tabs' look already (icon, name, dot, close in front and under the pointer); its bar keeps egui_dock's collapse arrow at its left and close all at its right, which the vendored egui_dock keeps for windows alone (`vendor/egui_dock`: the main surface's bars have neither), and `window_bars`' Maximize before the close. `tabs::tab_room` takes a window's bar as 2 × 24 points of those buttons and 28 of Maximize: the bar row T11 builds changes it. No Split Editor on a window's bar (the Floating board).

**Merge point M3b, after T11.** Coherent: new installations open editors as tabs and existing ones keep windows; the manual says both.

##### M3c: Widths, Outliner, hints (T12 to T15; merge point after T15)

**T12. Status Bar hints and counts.**

- **Goal.**
  - A hints API (`shell::hints(&[Hint])`, key caps and a word, dropping whole from the end).
  - The **area view's hint line** (area_view.rs:3380-3447) moves into it as key caps, state by state: idle, one selected, several selected, placing a blueprint, placing a light, a brush, scaling, a refused scale, Select Tiles with its notice. Its row in the area view goes.
  - The counts "11 by 11 tiles, 192 objects" move from the area toolbar (3305-3307) to the Status Bar's right.
  - The Welcome page's hints (Ctrl+N, Ctrl+O, Ctrl+P).
- **Files.** `area_view.rs:3300-3447`, `tabs.rs` (welcome), `shell.rs`.
- **Tests.**
  - Unit: hints drop whole at a narrow width.
  - App: each state's hints (migrate the tests reading the hint line; grep the tests for "Click to select" and "Placing").
  - Pictures `shell_main` and `shell_welcome` against Main.png and Welcome.png Status Bars.
- **Manual.** 04-areas (where the hint line is described).
- **Risks.** `frame_perf` run (hints laid out every frame).
- **Inherits from T7:** the Status Bar's middle now holds the log's last line while the Bottom Dock is closed (after a separator beside the switch, `StatusLog`, shown in 48 points or more, cut to the rest): the hints share that middle. The boards cap the line (`.sb-log`'s `max-width`, 96 px on Main, 150 to 260 on others) and give the hints the rest (`flex: 1 1 0`); T12 sets the cap, lays the hints out after it, and keeps `the_status_bar_holds_at_every_width` (status_bar.rs) passing with hints added to its sweep. The counts go before the job item at the right (SPEC: counts, job, module, folder) and yield with the job's name.

**T13. The Outliner (R2, ruled).**

- **Goal.**
  - The Left Dock splits: Module/Resources over the **Outliner** (resizable seam).
  - For an area: the placed objects by kind, each with its count and an **eye** bound to `view.show[kind]` (the same state as the area toolbar's Show toggles); rows with a kind icon, name and weak tag; click selects in the view; the selected object is revealed.
  - Shown only while the active editor brings one (the area in M3).
  - **Inherits from T6:** the rule is built (`Moonglow::active_editor`, `shell::editor_brings_palette`); the Outliner's `Panel::available` takes the same test (an area's tab or the placeholder), and a Left Dock whose panels none is available is not drawn (the Module and Resources panels keep it while a module is open).
- **Files.** New `outliner.rs` (reuse the tree's per-area contents code, `tree.rs` contents and row helpers), `shell.rs`.
- **Tests.**
  - App: the eye hides a kind in the view and toggles the area toolbar's "Show Creatures"; selecting in the view marks the row and selecting a row selects in the view.
  - Accessibility: the group is named "Outliner", so `area_tool` and Outliner eyes do not collide.
  - Picture against Main.png and LightArea_crop_outl.png.
- **Risks.**
  - **`frame_perf` must run**: area084's 549 objects; kinds closed by default except the selection's; `show_rows`.
  - The module tree keeps its opened-out area rows until R2 is answered.
- **Manual.** 02 (new Outliner section), 04-areas.

**T14. How the shell yields, and the wide layout.**

- **Goal.** Apply `Yields` (section 5.2):
  - below W 1300: Compile, Verify and Test Module lose their words (built by T2), ~~Quick Open is capped at 300 px and becomes an icon under 240~~ (built by T8, with the order it yields in against the build and quick actions);
  - below 1500: the Left Dock folds to its strip automatically, without writing `hide_tree`, and the user can unfold it;
  - below 1400: the Right Dock goes to 280 px with its panels as tabs, and the Status Bar switches become icons, then the game folder drops, then hints drop.

  Section 5.3 at W ≥ 2560 (U1): Left Dock 320 by default; the Right Dock stands two columns (Palette | Preview) when wider than 2 × 280.
- **Inherits** (left for T14 by T5 and its review, the owner's word of 2026-11-12):
  - `the_area_toolbar_stays_inside_its_tab_at_every_width` folds the Left Dock by hand under 772 (760 until the review, when the Right Dock stayed at the 280 narrower windows had left it): it stops, the yields doing it;
  - from 420 to 760, both docks set open, the editor area is left little (the least window about 24 points, 170 at 760): the yields give it its room;
  - ~~from 420 to 500 the Bottom Dock's Log tab is over its Copy Log button~~ (settled by T7: the tab turns to its icon, the tools give way, a dock narrower than the icon draws its bar empty);
  - the palette's type combo under 300 px of panel width (the strip wraps to rows meanwhile, down to the 190 the dock has in the least window);
  - (from T7) in a 420 px window with both side docks set open the Bottom Dock is 24 points wide and the log's text wraps a character a line (its tab bar is drawn empty there, T7's review);
  - (from T7) under 1400 the Status Bar's switches become icons (`StatusSwitch::icon_only`); the order the bar yields in is built in `shell::status_bar` (the last line, the game folder, the job's name, then the module's name), and `status_bar.rs`'s sweep holds it: the switches' icons go first in that order;
  - (from T8) Quick Open's field yields within the Top Bar already (capped at 300 under 1300, an icon under 240, then the build actions, the icon and the quick actions in that order, `quick_open.rs`'s sweep); T14's tests at 1280, 1399, 1499 and 3440 name it by "Quick Open" in the Top Bar, a text field or an icon button; the Welcome board's Status Bar hint for Ctrl+P is T12's hints;
  - (from T6) the Right Dock's Palette and Preview are two tabs of one bar at every width: under 1400 nothing changes for them (they are tabs already); at W ≥ 2560 they stand two columns (Palette | Preview, U1) where the dock is wider than 560, each column then drawing its panel (the Preview's body in its column, as `PreviewSlot` hands it on).
- **Not in M3.** The area header's compact form (E < 990), Snap and Turn text (E < 550) and More (E < 490) are M4's area header.
- **Tests.**
  - App at 1280, 1399, 1499 and 3440: what is shown, by name.
  - Pictures `shell_narrow` against Narrow.png and Narrow_crop_*.png; `shell_wide` against WideArea.png.
- **Risks.** `frame_perf` at 3440 (and 1280 with `FRAME_TARGET`).
- **Manual.** 02 (a paragraph on narrow and wide windows).

**T15. Closing pass.**

- **Goal.**
  - `frame_perf` and `editor_perf` against the T0 baseline logs.
  - **The icon mesh cache, decided by measure.** If the shell's median view cost rises more than about 0.1 ms, or tessellation grows more than 25% over the baseline, build the cache keyed by icon, size, color and the font atlas's size (proposal line 782). Otherwise record the numbers and the decision in the proposal.
  - Replace README's `docs/images/area.jpg` from `readme_screenshots` (U2).
  - (from T9) the document tabs' icons are tessellated each frame (32 tabs about 20 µs a frame more than before T9): the icon cache's measure includes them; `vendor/egui_dock` is dropped once egui_dock offers its two additions.
  - Tick the Where Things Moved rows done in M3.
  - Close the deferred entries the shell supersedes (issue 10's palettes and arrow parts, deferred.md:921-944; issues 11, 20 and 23, :957-975; the window bar, :1701), each with its reason.
  - Sweep the manual and `docs/parity/checklists.md` (§1 "Main menus…", :315-345 naming `TOOLBAR`; "Area viewer" :261).
- **Merge point M3c.**

#### Part 4. Where the documents disagree

| # | One side | Other side |
|---|---|---|
| 1 | `frame_perf.rs:45` measures at 3440x1400 | Its own doc (`:6`) and the brief say 3440x1440 (probably the usable height under the taskbar; say so in the file) |
| 2 | `ui-redesign-moves.md:47` lists "Test Module (F9)" in today's toolbar | `commands.rs:289-313` `TOOLBAR` has none (Build menu only) |
| 3 | Proposal:252 and moves:202: a floating script window is 1040 by 640 | `tabs.rs:196`: 1120x720, times `ROWS_TALLER` |
| 4 | Proposal:252,604: the tab menu is "Eject, Close, Rename…, Show in Module Tree, Close Others, Close Tabs to the Right", "unchanged" | `tabs.rs:670-753` also has Claim/Release Claim, Maximize/Restore, plugin commands, Close All. Keep the code's. |
| 5 | Proposal:757 (M2): the screenshots are replaced with M3 | Proposal:800 (M5): replaced in M5 |
| 6 | Brief and `CLAUDE.md:6` call `aurora-ui-inventory.md` the parity checklist to update | It is generated from Aurora's exe (frontmatter `process:aurora-uiinv-gen`; CLAUDE.md:113-116) and lists Aurora alone. The Moonglow file is `docs/parity/checklists.md` (:328, :342 name `TOOLBAR`). Consequence: M3's tasks update `docs/parity/checklists.md`; `docs/parity/aurora-ui-inventory.md` is generated and not edited by hand |
| 7 | Mock Status Bar: the module name ("Chapter1 (modified)", Main.png; "No module open", Welcome.png) | `lib.rs:3041-3065` and moves:52: the full path |
| 8 | Moves:576: Plugins "a Right Dock panel or a tab" | Ruling, proposal:697: "Plugins becomes a tab" (the ruling wins) |
| 9 | `mg-uikit/Cargo.toml` and `lib.rs:11`: "Egui alone among the UI crates" | Ruling 9 (proposal:686): one shared crate for theme, icons, **shell** and widgets; the shell's editor area is egui_dock (decided: O5) |
| 10 | Proposal M3 (771): "the Toolbar buttons setting retired" in M3 | The area and editor toolbars read it (`icons::Tools::of`, 19 sites) until M4 replaces them |
| 11 | Boards show Problems and Search Results switches (Main.png, Welcome.png) | Their panels exist only after M4's dialogs-to-panels |
| 12 | Proposal section 5: Start moves "to the area editor (the camera button under the compass; also the View menu)" | The compass is M4: in M3 Start is View › Go to Start and Quick Open |
| 13 | Notices "under the toolbar" (`lib.rs:1719-1723`; 10-options:290,661,670) | No board places them with a module open (Welcome puts the update in its Status section). Plan: strips under the Top Bar |
| 14 | Proposal 5.3 "docks stand two columns wide" at 3440 | No width threshold is given anywhere (U1) |

#### Part 5. Decided

Decided by the implementer on 2026-11-11:

- **U1.** Wide layout from W ≥ 2560: Left Dock 320 by default, Right Dock two columns when wider than 560.
- **U2.** Replace README's `area.jpg` at M3c (the shell picture changes then); the editors' screenshots at M5.
- **U3.** Each dock keeps one fold button on its inner edge (issue 10's rule): the strip chevron for Left and Right; the Status Bar switch for Bottom.
- **U4.** The Status Bar shows the module's name and "(modified)", the path in the tip.
- **U5.** Existing settings keep the Log open; new installations start closed.
- **U6.** No dragging panels between docks in M3 (ruled by the owner as R5).
- **U7.** Ctrl+Shift+P opens Quick Open restricted to commands.
- **O5.** `mg-uikit` takes egui_dock 0.21 behind an optional `dock` feature, so that the viewer gets the dock style and tab marks from the same place: ruling 9 put the shell in the shared crate. The crate's "egui alone" comment is changed with it.
- **Merge points.** M3 is merged into `develop` at three points, M3a, M3b and M3c, each coherent with the manual true.

#### Part 6. Rulings for M3

Put to the owner on 2026-11-11 with a recommendation each, and ruled the same day: "You may proceed as suggested." Each is carried out as its recommendation says.

- **R1 (the plan's O1). Escape on a docked tab with OK and Cancel** (a Properties tab, Options). Today Escape cancels only a floating one (`tabs.rs:393`), so As Tabs takes it away from new users, and the Options board's hints omit it (proposal finding 13.4). **Ruled, as recommended:** Escape cancels the active docked tab that has OK and Cancel while the pointer or keyboard is in it, as floating does; the hints then show Esc. Gates T10.
- **R2 (O2). The module tree's area rows opened out to their objects, once the Outliner exists.** The boards' Module panel lists areas without arrows; the moves tables do not say. **Ruled, as recommended:** keep both in M3 (builders rely on "the tree follows the selected object", and its tests are many); decide the removal with M4's area editor. Gates T13.
- **R3 (O3). Plugin panels** (`Tab::PluginPanel`, Plugins › Panels) as editor tabs or windows, following the switch, or as Right Dock panels. **Ruled, as recommended:** tabs or windows in M3 (they are not one editor's); revisit when the dialogs become panels in M4. Gates T10.
- **R4 (O4). Retiring "Toolbar buttons" at M4, not M3**, since the area toolbar's Icons-only mode is what keeps it on one row until M4's header. **Ruled, as recommended:** yes, at M4. The Top Bar ignores the setting from T2 and the Options control is renamed until then. Gates T2's Options wording only.
- **R5 (new, from U6 with T4 and T5). Panels are fixed to their docks in M3.** Today the resource browser and the palettes can be dragged to a side or into the middle and are docked there (GitHub issues 10, 11, 20 and 23, the `docs/deferred.md` entries named in T4 and T5); with the docks as panels that goes away: Resources lives in the Left Dock, Palette and Preview in the Right Dock, and a dock folds but its panels do not move. **Ruled, as recommended:** accept it for M3 and M4 (the mock's shell places every panel, and no board shows a panel moved), and file moving panels between docks as its own later piece of work only if the owner wants it. Gates T4 and T5; tasks before them (T0 to T3) do not depend on it.

#### Part 7. Scope moves

| Move | Reason |
|---|---|
| **S1** "Toolbar buttons" retirement, M3 to M4 | R4 |
| **S2** The Properties panel (a placed object's properties in the Right Dock, apply-at-once with Undo, ruling 2), M3 to M4's area editor | It changes the editor's insides (its pages as a rail, Cancel becoming Undo) against M3's exit "without yet changing their insides" |
| **S3** Problems and Search Results switches, and every editor's own dock panels (Reference, Node, Cell…), stay in M4 | They come with their panels |
| **S4** The area view's hint line moves *into* M3 (T12) | Rule 5 and the Main board put hints in the Status Bar, and a hints slot with no client would ship empty |
| **S5** The area's Outliner moves into M3 (T13); the other editors' Outliners stay M4 | The M3 bullet names it, and an Outliner frame with no contents would show an empty panel |

#### Handing out the tasks

A standing note for whoever hands out the tasks:

- One code writer at a time in the worktree.
- After each task, an independent read-only reviewer on the commit, then a fix round.
- The owner's findings from live testing M2 take the writer's slot ahead of M3 tasks.
- The hand-run timing tests (`frame_perf`, `editor_perf`, `world_perf`, `campaign_perf`, `nui_perf`) are run after any task that renames or moves controls, since the suite only compiles them.

#### Critical files for implementation

All under `crates/` in the worktree:

- `crates/mg-ui/src/lib.rs`
- `crates/mg-ui/src/tabs.rs`
- `crates/mg-ui/src/commands.rs`
- `crates/mg-ui/src/settings.rs`
- `crates/mg-ui/src/open_tabs.rs`
- `crates/mg-uikit/src/controls.rs`
- `crates/mg-ui/tests/app/windows.rs`

### M4. Editor by editor

Known differences left by the theme pass (M2), which the editors' own changes settle:

- An open combo box fills `bar`; the mock's `.combo.open` is `raised-hover`.
- The slider's thumb at rest is the rail's color (egui fills both with one color); the mock's is `text-2`.
- A disabled control fades whole (egui's opacity), but the check box and radio, which have disabled tokens of their own.
- In small script windows the Compiler and Help strip crowds the code (the script editor's change).
- With Icons and text, the area toolbar takes one row more than before the drawn icons at some widths: its buttons keep 4 px between icon and words, not the mock's 6, and are a point or two wider each than with their glyphs (the area editor's header and popovers settle it).
- A number field's arrows are 22 px square, an icon button's size, beside the field (the forms' own changes size them with the field).
- In the NUI designer's narrow layout (under 760 px), opening a combo box inside the Properties menu closes the menu: egui keeps one popup open at a time. It was so before the redesign, when any click in that menu closed it (the kit's form menus now stay open for a click inside); the designer's own change gives the narrow layout its Properties as a panel, not a menu.
- A color picker's hex field is cut short under a window's larger text (a fixed 70 px). No color button opens where the larger text applies today (the conversation editor), so only a test shows it; the color picker's own change sizes the field by its text.
- ~~With the Palettes pane dragged to about 135 px, a blueprint type's list toolbar (Expand All, List) spills left under the Fold Palettes button. Seen in the icons' fix round and not checked against the code before the redesign; the Right Dock's least width (M3) and the palette's own change settle it, and `views_draw_nothing_over_anything_else` is to exercise that state again then.~~ Settled by M3's T5: the palettes are the Right Dock's panel, at least 280 px wide, its fold button beside its edge outside it; `views_draw_nothing_over_anything_else` looks at the dock at 280, and `the_palette_fits_the_right_dock_from_its_least_width_to_600` sweeps every palette type from 280 to 600.

- The Right Dock's Preview panel (M3's T6) shows the model viewer's toolbar as the window did, wrapped to three rows at 320 px (about 95 points over the 3D view). The blueprints' change gives it the boards' one header row over the 3D view (the moves table's "Preview window: info line, Animation, Pause, Frame, Lights, Shadow, Shaders, Export Files…" row: about 60 points back to the view) and the Creature board's info line in its tab bar. The Properties panel stacked under the Palette and Preview tabs is the area editor's (S2).
In this order, each its own change: the area editor (header, popovers, tool shelf, docks, Paint and Tiles states), the gizmos (section 11), blueprints (page list, columns, Preview panel), the script editor, the conversation editor, the tables (2DA, talk table), the rest (NUI, tileset, hak, resources, journal, factions, Options, Welcome), the galleries and pickers (the shared cell), and the dialogs that become panels. The checklist is [ui-redesign-moves.md](ui-redesign-moves.md); a row is ticked when its control has reached its home and a test (or a picture compared with the board) says so.

### M5. The manual, the parity inventory and a toolset release

- The manual's chapters rewritten where the screens changed, the screenshots replaced, the parity inventory (`docs/parity/aurora-ui-inventory.md`) brought up to date.
- A toolset release, cut as this repository's releases are (an annotated tag, hand-written notes; **the owner signs each release**).
- Exit: `cargo test` suites and clippy clean on the newest stable; the corpus tests pass; frame time recorded against the baseline.

### M6. The viewer

- The viewer moves to the toolset release's tag and takes `mg-uikit`, the shell and the look; the copied helpers go.
- Its renames (Nodes to Outliner, Animation to Timeline, the View menu names) and Gallery mode for Resources, with **its manual updated in the same change**.
- Exit: the viewer's boards (Model with ASCII Editor, Creature and Colors, 3440 by 1440, Gallery) reproduced in pictures; frame time measured.

## 17. Open for the owner

Nothing in the design is open. Two things are the owner's by custom: ruling 7 (Ubuntu Light) is judged after M2, and each release is signed by the owner (M5, M6).
