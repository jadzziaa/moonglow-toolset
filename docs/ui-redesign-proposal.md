---
type: Proposal
title: 'One look and one shell for the suite: the UI redesign'
description: The approved redesign of Moonglow Toolset and Moonglow Viewer - the seven rules, every color token in dark and light, type and metrics, the shell and how it yields at small widths, the components, the icons, what the redesign does to each editor, the area view's handles (Fine Lines), the owner's rulings, the features it adds, what is decided but not drawn, the probable bugs found in today's app, and the milestones to build it (M1 the shared crate mg-uikit to M6 the viewer).
tags: [proposal, ui, redesign, theme, tokens, shell, icons, gizmos, galleries, light-theme, viewer, mg-uikit]
generated: { by: claude-code/claude-opus-5-5, at: 2026-11-12T02:48:58Z }
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

**Done, 2026-11-12.** The frame is `crates/mg-ui/src/shell.rs`, drawn by `Moonglow::ui` in this order: the Top Bar (`Panel::top("menu")`, 30 px and its line), the notices under it, the Status Bar (24 px), the Left Dock (`Panel::left("tree")`, the whole height between the bars, 260 px by default, a remembered width kept), the Bottom Dock (`Panel::bottom("log")`, under the editor area only, as WideArea.png has it, 180 px by default), then the editor area unchanged. The Top Bar is the kit's `top_bar` over `commands::TOP_BAR` (quick actions after the menus, build actions at the right end) and `TOP_BAR_TEST_MENU` (Choose Character, a menu under Test Module's arrow, as "under its arrow" says; the board draws a plain button); its middle is left empty for T8. The Left Dock is a `DockTabs` with the Module tab, Properties and New Area (named so, their tips "Module properties" and "Area Wizard") and the kit's fold button at its right; folded, `Panel::left("tree-strip")` holds the kit's `FoldStrip`, whose chevron and Module button unfold it. The Bottom Dock is a `DockTabs` with the Log tab and the log's Copy Log, Save Log and Clear Log as icons on its tab bar (T7's place for them; their corner is gone). The Status Bar has the Log switch at its left and, at its right, "Reading files changed outside Moonglow…" while that runs (as the status line said it), the module's name with "(modified)" and the modified dot (the path in its tip; "No module open", "New module, not saved yet"), and the game folder's last part ("…/Neverwinter Nights", the path in its tip), left out where it does not fit. Undo and Redo on the Top Bar keep `undo_before_bars` and, with the pointer come from no side editor, fall back to the side editor in front of the focused pane (`side_doc::focused_side_editor`). Beyond the plan, found on the way: egui's menu bar takes the whole width it is given, so `commands::menu_bar` gives it the menus' own width (measured the frame before); the kit's right-hand groups (`at_right_end`) no longer widen what holds them before their width is known (a dock's panel grew by its tab bar's tools for good); a pass asked for again (`request_discard`) runs without the frame's input, and the Status Bar's right group changing width on a click's frame (the module becoming modified) lost the click, so the Status Bar is laid out by its right group's computed width (`status::bar_sized`); in a window too narrow for the whole Top Bar (under about 640 px) the build actions go whole, then the quick actions, every one of them being in the menus; the build actions as icons (under 1300) are framed as their words are (`IconButton::framed`), and Test Module and its arrow are joined (`IconButton::joined`, `joined_spacing`). Differences from the plan: the Left Dock's header buttons are named Properties and New Area, not Module Properties (the tree's row of that name is found by name in tests; T3 renames them with the Module panel); the Bottom Dock's and Left Dock's default sizes are the kit's (180 and 260, from 120 and 240). `frame_perf` on a quiet machine against M2's quiet run (median CPU ms, idle): module open with no area 0.35 (0.34), as it opens 0.50 (0.48), with its area 0.45 (0.38), File menu open 0.27 (0.26), the tree with every group open 0.81 (0.77); tessellation unchanged (0.03 to 0.11 ms) and about 1,400 vertices fewer in each view (the toolbar's row gone); none over its budget but the known area084. Its click on "Properties" now takes the blueprint's page, not the first of that name (the Module panel's, or before it the toolbar's): the item's Properties page view draws 11,860 vertices against the baseline's 14,754, so that view's baseline may have been another view.

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

**T6. The Preview panel; the docks follow the active editor.**

- **Goal.** `preview_window` (`area_tools::preview_window`, area_tools.rs:787) becomes `Panel::Preview` under the Palette, stacked, each with its own tab bar. The registry rule goes live: the Right Dock shows the active docked editor's panels and is not drawn when there are none.
- **User sees.** Preview under the palette. With a script tab in front (once As Tabs exists), the Right Dock goes away.
- **Files.** `area_tools.rs:787-833`, `commands.rs:901` (`Id::PreviewWindow` toggles the panel), `shell.rs`.
- **Tests.**
  - App: Preview shows the palette's blueprint and the toggle hides it; the Right Dock follows the front tab of the focused main leaf, not a floating window's.
  - Migrate `tool(&h, "Preview")`.
  - Picture against Main.png's right column.
- **Manual.** 02 (the Preview button line); 05-blueprints if it names the Preview window.
- **Risks.** `frame_perf` run (the preview's 3D view now draws every frame while shown).

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

**T13. The Outliner (R2, ruled).**

- **Goal.**
  - The Left Dock splits: Module/Resources over the **Outliner** (resizable seam).
  - For an area: the placed objects by kind, each with its count and an **eye** bound to `view.show[kind]` (the same state as the area toolbar's Show toggles); rows with a kind icon, name and weak tag; click selects in the view; the selected object is revealed.
  - Shown only while the active editor brings one (the area in M3).
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
  - below W 1300: Compile, Verify and Test Module lose their words, Quick Open is capped at 300 px and becomes an icon under 240;
  - below 1500: the Left Dock folds to its strip automatically, without writing `hide_tree`, and the user can unfold it;
  - below 1400: the Right Dock goes to 280 px with its panels as tabs, and the Status Bar switches become icons, then the game folder drops, then hints drop.

  Section 5.3 at W ≥ 2560 (U1): Left Dock 320 by default; the Right Dock stands two columns (Palette | Preview) when wider than 2 × 280.
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
- With the Palettes pane dragged to about 135 px, a blueprint type's list toolbar (Expand All, List) spills left under the Fold Palettes button. Seen in the icons' fix round and not checked against the code before the redesign; the Right Dock's least width (M3) and the palette's own change settle it, and `views_draw_nothing_over_anything_else` is to exercise that state again then.

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
