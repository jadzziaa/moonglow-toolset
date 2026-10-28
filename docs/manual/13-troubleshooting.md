---
type: Manual Page
title: Troubleshooting
description: Troubleshooting - the game is not found, no GPU in the area viewer, tiles that disappear on AMD under Windows (the graphics backend), Test Module does nothing, no sound, after a crash, and where Moonglow keeps its files.
tags: [manual, troubleshooting]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-28T00:19:15Z }
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

## Tiles disappear on an AMD GPU under Windows

On an AMD GPU under Windows, AMD's Vulkan driver drew an area's tiles
when the area opened, then lost them one by one (a builder's Radeon PRO,
driver 25.10.37.03, October 2026); through Direct3D 12 they stay. Moonglow now
draws through Direct3D 12 on such a GPU by itself (**Options › General ›
Graphics backend**: Automatic). If you set **Vulkan** there, or
something still looks wrong, choose **Direct3D 12** and start Moonglow
again. To try a backend without changing the setting, start Moonglow
with the `WGPU_BACKEND` environment variable set (`dx12`, `vulkan` or
`gl`): it decides over the setting. In a Command Prompt, in the folder
Moonglow is installed in:

```bat
set WGPU_BACKEND=dx12
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
| Recovery copies, crash reports, debug logs (`logs`), prefabs (`prefabs`), plugins (`plugins`), each module's open tabs (`open-tabs`) | `~/.local/share/moonglow` | `%APPDATA%\Moonglow` | `~/Library/Application Support/Moonglow` |
| Conversation backups | `/tmp/moonglow-backups` | `%TEMP%\moonglow-backups` | `$TMPDIR/moonglow-backups` |

On Linux, `$XDG_DATA_HOME` takes the place of `~/.local/share` when set
(in the Flatpak: `~/.var/app/io.github.moonglow_toolset.Moonglow/data`).

Moonglow never writes into the game's folder. In your user folder it
writes only what you save there (modules, and exported archives where you
choose).
