# Getting started

## What you need

- **Neverwinter Nights: Enhanced Edition**, installed (Steam, GOG or
  Beamdog). Moonglow reads the game's data (tilesets, models, 2DA tables,
  the talk table, scripts) from the installation; it ships none of it.
- A graphics card with **Vulkan**, **Metal** (macOS) or **Direct3D 12**
  (Windows) for the area viewer and model previews. Everything else works
  without one.
- Linux (x86-64, a 2022 or newer distribution), Windows 10 or 11 (x64),
  or macOS 11 or newer (Apple silicon or Intel).

## Installing

- **Linux, AppImage**: make the file executable (`chmod +x
  Moonglow-*.AppImage`) and run it. To have it in your application menu,
  use your desktop's AppImage integration (for example Gear Lever or
  AppImageLauncher).
- **Linux, Flatpak**: `flatpak install Moonglow-*.flatpak`. The sandbox
  sees the game where Steam installs it and the game's user folder; a game
  elsewhere needs its folder allowed (see
  [Troubleshooting](13-troubleshooting.md)).
- **Windows**: run `Moonglow-*-setup.exe`. It installs for your user
  without administrator rights (or for all users, if you choose), adds a
  Start menu entry and, if you tick it, opens `.mod` files with Moonglow.
- **macOS**: open the disk image and drag **Moonglow Toolset** into
  Applications. The first time, right-click the app and choose **Open**
  (the app is not notarized by Apple).

## The first start

Moonglow looks for the game where Steam installs it and for the game's
user folder (where your modules, haks, talk tables and override live):

| System | Game | User folder |
| --- | --- | --- |
| Linux | `~/.local/share/Steam/steamapps/common/Neverwinter Nights` (also Steam's Flatpak) | `~/.local/share/Neverwinter Nights` |
| Windows | `C:\Program Files (x86)\Steam\steamapps\common\Neverwinter Nights` | `Documents\Neverwinter Nights` |
| macOS | `~/Library/Application Support/Steam/steamapps/common/Neverwinter Nights` | `~/Documents/Neverwinter Nights` |

The log at the bottom of the window says where it found the game ("Game
data loaded from …"), and the status bar shows the folder. If the game is
somewhere else (another Steam library, GOG, Beamdog's client), choose
**Tools › Options… › Folders** and set **Neverwinter Nights installation**
(the folder with `data/` and `bin/` in it) and, if needed, the **NWN user
folder**.

The **Welcome** tab offers **New Module…** and **Open Module…**; later it
also lists your recent modules.

## A first module

1. **File › New Module…** (Ctrl+N) asks for a name and makes an empty
   module, as Aurora's module wizard does.
2. **Wizards › Area Wizard…** (Ctrl+Alt+A, or **New Area** on the toolbar)
   makes an area: pick a tileset, a size and a name. The area opens in the
   area viewer.
3. Choose a blueprint in the palette on the right (a creature, a
   placeable…) and click in the area to place it.
4. **File › Save** (Ctrl+S). Save the module in your user folder's
   `modules` folder if you want to test it in the game.
5. **Build › Test Module** (F9) saves the module and starts the game on it,
   with your first local character at the start location.

The chapters that follow describe each part in detail.
