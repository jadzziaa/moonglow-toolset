---
type: Manual Page
title: Troubleshooting
description: Troubleshooting - the game is not found, no GPU in the area viewer, an area that draws nothing or loses its tiles on AMD under Windows (the graphics backend, its startup check, the lit check and the area view's watch; what to send), Direct3D 12's shader compiler (the DXC shipped beside Moonglow, and FXC where it is missing), Test Module does nothing, no sound, after a crash, and where Moonglow keeps its files.
tags: [manual, troubleshooting]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-29T18:03:45Z }
---

# Troubleshooting

## Moonglow does not find the game

The log says "No Neverwinter Nights installation found" and the status
bar "No game install". Choose **Tools › Options… › Folders** and set the
game's folder: the one with `data/` and `bin/` in it (for Steam,
`steamapps/common/Neverwinter Nights` in the library you installed it
to). Moonglow needs Enhanced Edition; the original 1.69 release is not
supported.

In the **Flatpak**, the sandbox only sees the game where Steam installs it
by default. Allow another folder with:

```sh
flatpak override --user --filesystem="/path/to/Neverwinter Nights:ro" io.github.moonglow_toolset.Moonglow
```

## The area viewer says "No GPU"

The area viewer and model previews need a graphics card with Vulkan or
Direct3D 12 (Windows), Vulkan (Linux) or Metal (macOS). Where only
OpenGL is there, Moonglow draws with it, but its renderer is not made
for it: particles, the game's shaders and more are drawn wrong. Everything else in Moonglow works
without one. Update the graphics driver; on Linux, install your
distribution's Vulkan driver (Mesa's `vulkan-radeon` or `vulkan-intel`,
or NVIDIA's). **Tools › Options… › General › Graphics backend** chooses
the backend (from the next start); the debug log's `graphics:` line
says which adapter was taken, and why.

## An area draws nothing, or loses its tiles, on an AMD GPU under Windows

Two builders' AMD GPUs under Windows went wrong in opposite ways
(October 2026):

- A Radeon PRO, through AMD's **Vulkan** driver (25.10.37.03), drew an
  area's tiles when the area opened, then lost them one by one; through
  Direct3D 12 they stayed.
- A Radeon RX 580 (Windows 10, since Moonglow 2.11.0), through
  **Direct3D 12**, drew nothing of an area but the start location's
  marker and the doors' arrows; through Vulkan it drew it all. (The
  marker and the arrows are drawn over the picture, not in it: they show
  whatever the GPU draws.) Moonglow 2.13.0's checks didn't catch it.

So no fixed rule serves every AMD GPU. Automatic (**Options › General
› Graphics backend**) still takes Direct3D 12 first for AMD on Windows,
and Moonglow looks at what it draws:

- **As it starts**, before the window opens, it draws a small scene
  offscreen and checks it. Where nothing is drawn, it takes the same
  GPU's other backend (once: if that fails too, it keeps the first and
  says so in a notice under the toolbar). The backend that failed is
  remembered for this GPU and driver; Automatic passes over it until a
  driver update. The log says what happened, and the debug log's
  `graphics:` line what the check found on each backend.
- **Once the game's data is read**, it draws a lit, textured test mesh
  (red) as a tile is drawn, with Moonglow's own shader and with the
  game's, and says whether each drew nothing of it or drew it in the
  wrong colors. Where Moonglow's own draws it wrong, the notice offers
  **Restart with …** (below). Where only the game's shaders draw it
  wrong, Moonglow draws with its own for the rest of the session and the
  notice says so (the **Shaders** button turns the game's on again): the
  area is still drawn, only not by the game's shaders. The check draws
  the game's own shader files, as the game installs them, even where
  `override`, `development`, a Workshop item or a hak has its own copy
  of one (a builder's `inc_lighting`, say), since what the game's own
  draw is known; the debug log's `graphics: the lit check` line names
  such copies and the colors each shader drew. The area view draws with
  the copies the game reads.
- **While an area is open**, it looks about once a second at how much
  of the area's ground the GPU drew against what the camera should see
  (the game's shaders included, once they are compiled; tiles whose
  models failed included). Where less than a fifth of it has been drawn
  for five seconds, a notice under the toolbar names the GPU and the
  backend and offers **Restart with …** the other one. It closes
  Moonglow as File › Exit does (asking about unsaved work first) and
  starts it again with the same module; on Automatic the failing
  backend is remembered as above, and a backend you chose in Options is
  changed there. The watch says nothing on an empty area, while the
  game's shaders are being compiled or textures read, with Fade
  Geometry, with the camera turned to the sky, or while zooming; but
  where an area waits twenty seconds for textures or shaders and none
  comes, it judges what is drawn all the same.

A shader of the game's that the graphics driver can't compile is drawn
with Moonglow's own shader instead, and the log says which. (Under
Direct3D 12, see the next section.)

Neither builder's machine was reproduced where Moonglow is developed:
both failures were imitated there, and the checks are waiting on the
two builders to confirm them. If an area still draws nothing, a report
helps most with these files from `%APPDATA%\Moonglow` (paste it into
the File Explorer's address bar): the **debug log** of the session
(`logs\debug-log-<date>-<time>.txt`; its lines starting `graphics` say
what the checks found), and any `crash-<time>.txt` or
`model-failure-<time>.txt` written then, even where Moonglow went on.
Say too whether the area is drawn with the area view's **Shaders**
button off.

If you set a backend in Options yourself, it is kept even where the
check fails (the notice offers the other). Where nothing helps, update
the graphics driver, or turn the game's shaders off with the area
view's **Shaders** button. To try a backend without changing the setting, start Moonglow
with the `WGPU_BACKEND` environment variable set (`dx12`, `vulkan` or
`gl`): it decides over the setting. In a Command Prompt, in the folder
Moonglow is installed in:

```bat
set WGPU_BACKEND=dx12
moonglow.exe
```

## Direct3D 12's shader compiler

Under Direct3D 12, shaders are compiled with Microsoft's DirectX Shader
Compiler (DXC): the `dxcompiler.dll` installed beside `moonglow.exe` and
`mg.exe` (the installer and the zip both have it). The debug log's
`graphics:` line ends with which compiler was taken, "Direct3D 12
compiles shaders with DXC (…\dxcompiler.dll)".

Where that file is missing (deleted, or not copied with the programs),
Moonglow takes Windows' older compiler, FXC, and the line says "with FXC
(no dxcompiler.dll beside moonglow.exe)". FXC can't compile the game's
normal-mapped lit shaders (`fslit_nm`, `fslit_sm_nm`, and copies of them
in a module's haks, such as a skin tile's): their height map is read in
a loop of as many steps as the view asks. Those meshes are then drawn
with Moonglow's own shader, and the log says which; the rest draws as
before. Install Moonglow again, or unpack the zip again, to put the file
back.

With DXC, Direct3D 12 needs a driver with shader model 6, which current
drivers have; on an older one Direct3D 12 isn't offered at all, and
Moonglow draws with Vulkan. To use Direct3D 12 with FXC there, start
Moonglow with the `WGPU_DX12_COMPILER` environment variable set to `fxc`
(it decides over the file; `dxc` takes a `dxcompiler.dll` wherever
Windows finds one):

```bat
set WGPU_DX12_COMPILER=fxc
moonglow.exe
```

## Test Module does nothing

The log tells why. The module must be saved in the user folder's
`modules` folder (File › Save As…), and the game must be installed where
Options › Folders says. The game starts with your first local character;
if there is none, make one in the game first.

## Sounds do not play

Sounds need a sound device. On Linux, Moonglow plays through ALSA
(PulseAudio and PipeWire provide it). The area's sounds also need
**🔊 Sounds**, **Ambient** or **Music** on in the area viewer's toolbar.

## A model is left out

A model that Moonglow reads but can't build or pose (custom content with
something in it no model has had before) is left out of what is drawn:
its object shows without it, as one whose model is missing does, and the
log says "A model could not be shown and is left out", with its name.
The module is unharmed and stays open. A report (`model-failure-<time>.txt`)
is written in Moonglow's data folder, beside the crash reports: please
send it, with the model if you can, when you report the problem.

## After a crash

Moonglow writes a crash report (`crash-<time>.txt`) in its data folder.
If the module had unsaved changes, it writes a recovery copy of them as
it stops (unless Options › General keeps none), and the next start offers
it back in **Recover Unsaved Work**. The module's own file is as it was
last saved. Please include the crash report when you report the problem.

Something that fails in the background (long work, an autosave, a
plugin's run, an area being read) leaves the window going: the log says
so, and the unsaved work is kept in a recovery copy straight away, in
case what failed shows later. Saving, or closing the module without
saving, removes the copy as usual.

The report names the [plugins](15-plugins.md) that were enabled. To see
whether one of them is the cause, start Moonglow with `--no-plugins`
(none is loaded) and try again.

## The window shows only the module tree

Before 1.10, a very long name in the module tree (an area listed by
name) could widen the tree over the whole window, hiding the area and the
palettes, and it could not be dragged narrower. The tree now takes at
most two fifths of the window and cuts long names short.

## Something fails without a word

When something doesn't happen and nothing says why (an area that won't
open, an empty pane), or after a crash, send the newest
`debug-log-<date>-<time>.txt` from the `logs` folder of Moonglow's data
folder with your report. Moonglow writes this debug log unless **Tools ›
Options › General › Write a debug log** is switched off. Each line has the time of day (UTC) and the
seconds since Moonglow started. It records each thing asked
for, the tabs opened and where the panes are, what an area's view and
the palettes find as they load, the graphics adapter, and the graphics
libraries' warnings. It holds the names of your module's files and
folders, not their contents. A new one is started each time Moonglow
starts, named by when, and the last five are kept: after a crash, the
log of the session that crashed is still there when you start again, and
its last line says where it stopped. One that passes 4 MB is set aside
(the same name ending `.1.txt`) and
begun again (send both, then), so the five take 40 MB at most. (Setting the
`MOONGLOW_DEBUG_LOG` environment variable writes it with the option
off.)

## Where Moonglow keeps its files

| What | Linux | Windows | macOS |
| --- | --- | --- | --- |
| Settings | `~/.local/share/moonglowtoolset` | `%APPDATA%\Moonglow Toolset\data` | `~/Library/Application Support/Moonglow-Toolset` |
| Recovery copies, crash reports, debug logs (`logs`), prefabs (`prefabs`), variable sets (`variable-sets`), plugins (`plugins`), each module's open tabs (`open-tabs`) | `~/.local/share/moonglow` | `%APPDATA%\Moonglow` | `~/Library/Application Support/Moonglow` |
| Conversation backups | `/tmp/moonglow-backups` | `%TEMP%\moonglow-backups` | `$TMPDIR/moonglow-backups` |

On Linux, `$XDG_DATA_HOME` takes the place of `~/.local/share` when set
(in the Flatpak: `~/.var/app/io.github.moonglow_toolset.Moonglow/data`).
On every system, `MOONGLOW_DATA_DIR` names the second row's folder
itself, and `MOONGLOW_CACHE_DIR` the folder of what Moonglow makes and
can make again (the game's shaders compiled, `mg lsp`'s scripts), when
they are set: for a run that should keep apart from yours, as the tests
do.

Moonglow never writes into the game's folder. In your user folder it
writes only what you save there (modules, and exported archives where you
choose).
