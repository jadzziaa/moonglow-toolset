---
type: Manual Page
title: Troubleshooting
description: Troubleshooting - the game is not found, no GPU in the area viewer, Test Module does nothing, no sound, after a crash, and where Moonglow keeps its files.
tags: [manual, troubleshooting]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-03T21:00:08Z }
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

The area viewer and model previews need a graphics card with Vulkan
(Linux, Windows), Metal (macOS) or Direct3D 12 (Windows). Everything else
in Moonglow works without one. Update the graphics driver; on Linux,
install your distribution's Vulkan driver (Mesa's `vulkan-radeon` or
`vulkan-intel`, or NVIDIA's).

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
If the module had unsaved changes, the next start offers back a recovery
copy from the last few minutes in **Recover Unsaved Work**. Please
include the crash report when you report the problem.

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
open, an empty pane), turn on **Tools › Options › General › Write a debug
log**, choose OK, do the thing again, and send `debug-log.txt` from
Moonglow's data folder with your report. It records each thing asked
for, the tabs opened and where the panes are, what an area's view and
the palettes find as they load, the graphics adapter, and the graphics
libraries' warnings. It holds the names of your module's files and
folders, not their contents. A new one is started each time Moonglow
starts; to log from the very start (a module that fails as it opens),
turn the option on, close Moonglow and start it again. (Setting the
`MOONGLOW_DEBUG_LOG` environment variable does the same without the
option.)

## Where Moonglow keeps its files

| What | Linux | Windows | macOS |
| --- | --- | --- | --- |
| Settings | `~/.local/share/moonglowtoolset` | `%APPDATA%\Moonglow Toolset\data` | `~/Library/Application Support/Moonglow-Toolset` |
| Recovery copies, crash reports, prefabs (`prefabs`), plugins (`plugins`) | `~/.local/share/moonglow` | `%APPDATA%\Moonglow` | `~/Library/Application Support/Moonglow` |
| Conversation backups | `/tmp/moonglow-backups` | `%TEMP%\moonglow-backups` | `$TMPDIR/moonglow-backups` |

On Linux, `$XDG_DATA_HOME` takes the place of `~/.local/share` when set
(in the Flatpak: `~/.var/app/io.github.moonglow_toolset.Moonglow/data`).

Moonglow never writes into the game's folder. In your user folder it
writes only what you save there (modules, and exported archives where you
choose).
