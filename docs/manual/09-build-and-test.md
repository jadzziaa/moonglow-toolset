# Build, verify and test

## Build Module

**Build › Build Module…** prepares the module for play and checks it, as
Aurora's Build does:

- **Compile**: the **Scripts**, recalculated **Creature CR** (challenge
  ratings), **Encounters** (each encounter's creature list brought up to
  date with its creatures' challenge ratings and appearances; creatures
  that no longer exist are taken out) and **Palettes** (the custom
  palettes rebuilt from the module's blueprints).
- **Check that resources are available**: **Missing Resources** (scripts,
  blueprints, conversations, models… that something in the module names
  but neither the module, its haks nor the game has) and, if ticked,
  **Unused** resources (in the module but used by nothing).

Compile and Missing Resources are on by default, Unused off. The
**Results** list what was found; double-click a result to open what it is
about, and **Export…** saves the list as text. With **Build module on
save** (Options › General), every save builds first.

**Build › Verify Module** checks for missing resources alone and writes
what it finds to the log.

## Test Module

**Build › Test Module** (F9) saves the module and starts the game on it:
your first local character appears at the module's start location. The
game looks for the module by name in your user folder's `modules`
folder, so the module must be saved there (File › Save As…). With
**Minimize Toolset on test module** (Options › General), Moonglow's window
gets out of the way.

## Area Statistics

**Build › Area Statistics** shows, for the area shown last, its tiles,
its objects and how much memory its models take.
