# Troubleshooting

## Moonglow does not find the game

The log says "No Neverwinter Nights installation found" and the status
bar "No game install". Choose **Tools › Options… › Folders** and set the game's folder: the one with `data/` and
`bin/` in it (for Steam, `steamapps/common/Neverwinter Nights` in the
library you installed it to). Moonglow needs Enhanced Edition; the
original 1.69 release is not supported.

In the **Flatpak**, the sandbox only sees the game where Steam installs it
by default. Allow another folder with:

```sh
flatpak override --user --filesystem="/path/to/Neverwinter Nights:ro" io.github.moonglow_toolset.Moonglow
```

## The area viewer says "No GPU"

The area viewer and model previews need a graphics card with Vulkan
(Linux, Windows), Metal (macOS) or Direct3D 12 (Windows). Update the
graphics driver; on Linux, install your distribution's Vulkan driver
(Mesa's `vulkan-radeon` or `vulkan-intel`, or NVIDIA's). Everything else
in Moonglow works without one.

## Test Module does nothing

The log tells why. The module must be saved in the user folder's
`modules` folder (File › Save As…), and the game must be installed where
Options › Folders says. The game starts with your first local character;
make one in the game first if there is none.

## Sounds do not play

Sounds play when there is a sound device. On Linux, Moonglow plays
through ALSA (PulseAudio and PipeWire provide it). The area's sounds also
need **🔊 Sounds**, **Ambient** or **Music** on in the area viewer's
toolbar.

## After a crash

Moonglow writes a crash report (`crash-<time>.txt`) in its data folder
and, if the module had unsaved changes, it has a recovery copy from the
last few minutes: the next start offers it back in **Recover Unsaved
Work**. Please include the crash report when you report the problem.

## Where Moonglow keeps its files

| What | Linux | Windows | macOS |
| --- | --- | --- | --- |
| Settings | `~/.local/share/moonglowtoolset` | `%APPDATA%\Moonglow Toolset\data` | `~/Library/Application Support/Moonglow-Toolset` |
| Recovery copies, crash reports, prefabs (`prefabs`) | `~/.local/share/moonglow` | `%APPDATA%\Moonglow` | `~/Library/Application Support/Moonglow` |
| Conversation backups | `/tmp/moonglow-backups` | `%TEMP%\moonglow-backups` | `$TMPDIR/moonglow-backups` |

On Linux, `$XDG_DATA_HOME` takes the place of `~/.local/share` when set
(in the Flatpak: `~/.var/app/io.github.moonglow_toolset.Moonglow/data`).

Moonglow never writes into the game's folder. It writes into your user
folder only what you save there (modules, and exported archives where you
choose).
