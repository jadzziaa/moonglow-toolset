# Update log

## 2026-10-06
* **Creation**: [Running a tileset's own shaders](research/notes_custom_shaders.md): findings and a staged plan, tabled (the `shaders` branch).
* **Update**: [The main window](manual/02-main-window.md) (the module tree's menu on an area's objects, Escape with a tool in hand), [areas](manual/04-areas.md) (Fade Geometry's three choices; prefabs renamed, told of on hover and their delete undone), [blueprints](manual/05-blueprints.md) (View is read-only, waypoints' pictures, an item's parts by picture), [modules](manual/03-modules.md) (Shift+click in the hak editor), [options](manual/10-options.md) (a tab of the dock) and [the deferred list](deferred.md).
* **Update**: [Blueprints](manual/05-blueprints.md): an item's model in its Appearance page; [the deferred list](deferred.md): the Gallery in a narrow palette pane.
* **Update**: [Blueprints](manual/05-blueprints.md): Properties on an item in an inventory (a placed object's item is its own), and armor and cloaks shown on a woman, icon included; [the deferred list](deferred.md).
* **Update**: [Scripts](manual/07-scripts.md): include files aren't compiled on their own; [areas](manual/04-areas.md): the grid from the area's edge, Ctrl + drag with a tileset brush; [the deferred list](deferred.md): builders' reports after 1.14.1.
* **Update**: [Options](manual/10-options.md) and [scripts](manual/07-scripts.md): the External Script Compiler; [getting started](manual/01-getting-started.md): the game is found in other Steam libraries; [the deferred list](deferred.md).
* **Update**: [Options](manual/10-options.md): Steam Workshop content is read; [build, verify and test](manual/09-build-and-test.md): a Steam copy started for a test reaches Steam; [the deferred list](deferred.md).

## 2026-10-04
* **Update**: Made `docs/` an Open Knowledge Format (OKF v0.2) bundle: frontmatter on all 43 documents (the plan, [findings](findings.md), proposals, [research](research/index.md), [parity](parity/index.md), [plugins](plugins/index.md) and every chapter of [the manual](manual/README.md)), index files and this log. No content changed. The program's manual reader (`crates/mg-ui/src/manual.rs`) skips a chapter's frontmatter, and the two generated documents get theirs from their generators (`tools/aurora/uiinv/gen.py`, `crates/mg-corpus-tests/examples/observed_schema.rs`).
* **Update**: [The area chapter](manual/04-areas.md) and [the deferred list](deferred.md): the start location's marker is dragged and turned in the view.
* **Update**: [The main window](manual/02-main-window.md) (Delete… in the module tree), [areas](manual/04-areas.md) (the loading screen's picture) and [the deferred list](deferred.md).
* **Update**: [The deferred list](deferred.md): pale rims round transparent textures.
* **Update**: [Scripts](manual/07-scripts.md): Edit beside a script's name makes a script that doesn't exist; noted in [the deferred list](deferred.md).
* **Update**: [The deferred list](deferred.md): what was fixed for issue 3's custom creatures, and what of it isn't compared with the client.
* **Update**: [Blueprints](manual/05-blueprints.md): a builder reports that Aurora reads palette categories kept in a module; [the main window](manual/02-main-window.md): Expand All and Collapse All.
* **Update**: [Troubleshooting](manual/13-troubleshooting.md) and [Options](manual/10-options.md): the debug log; [the deferred list](deferred.md): what it leaves out, and the report it was made for.
* **Update**: [The main window](manual/02-main-window.md): windows remember their size and maximize; [scripts](manual/07-scripts.md); [the deferred list](deferred.md).
* **Update**: [Modules](manual/03-modules.md): a nasher project's files changed outside are read again; [conversations](manual/06-conversations.md): tokens by language; [options](manual/10-options.md); [the deferred list](deferred.md).
* **Update**: [Options](manual/10-options.md): the game's own text is read in the language edited; [the deferred list](deferred.md).
* **Update**: [Troubleshooting](manual/13-troubleshooting.md) and [the deferred list](deferred.md): the module tree held to its share of the window (a builder's report resolved).
* **Update**: [Areas](manual/04-areas.md): the readout names the tile; [the deferred list](deferred.md): see-through meshes drawn in two parts.
* **Update**: [The main window](manual/02-main-window.md): Escape closes the window in front; [the deferred list](deferred.md): a second builder's review, what was done from it and what is left.
* **Update**: [The main window](manual/02-main-window.md) (Copy…, a window's bar), [areas](manual/04-areas.md) (Fade Geometry, particles, deleting a prefab), [blueprints](manual/05-blueprints.md) (View, Edit Copy…, the wizards' ResRef and Tag), [journal and factions](manual/08-journal-and-factions.md); [the deferred list](deferred.md): builders' reports after 1.10.1.
* **Update**: [The deferred list](deferred.md): issue 5's body parts and werebat fixed with the reporter's files; number fields' limits measured in Aurora.
* **Update**: [Areas](manual/04-areas.md): the scale handle; [the deferred list](deferred.md).
* **Update**: [Areas](manual/04-areas.md): tiles' particles; Alt, not Shift, turns the selection and moves the camera; [the deferred list](deferred.md).
* **Update**: [The deferred list](deferred.md): a worn part (a cloak, a robe) moves at its wearer's animation scale.
* **Update**: [Blueprints](manual/05-blueprints.md): the palette's Gallery and Replace Selected with This; [the deferred list](deferred.md).
* **Update**: [Blueprints](manual/05-blueprints.md): a placeable's appearances as pictures; [the deferred list](deferred.md).
* **Update**: [Blueprints](manual/05-blueprints.md): the Placeable Gallery; [areas](manual/04-areas.md) and [the deferred list](deferred.md): copying leaves text on the clipboard, for Ctrl+V on Windows.
* **Update**: [The deferred list](deferred.md): GitHub issue 6, what was done from it and what is left; [areas](manual/04-areas.md): Escape deselects.
* **Update**: [The deferred list](deferred.md): water ripples (the game's procedural texture, approximated).
* **Update**: [The deferred list](deferred.md): water's waves (Enhanced Edition's water shader, approximated).
* **Update**: [Blueprints](manual/05-blueprints.md): a picture dragged from the Placeable Gallery places a placeable.
* **Update**: [Blueprints](manual/05-blueprints.md): a dragged appearance's ghost; pictures are taken from the side that shows the most of a model.
* **Update**: [Blueprints](manual/05-blueprints.md) (the Appearance Gallery's creatures and doors, the Creature Wizard, the inventory, talk-table text), [areas](manual/04-areas.md) (clicking between tiles and objects); [the deferred list](deferred.md).
* **Update**: [Blueprints](manual/05-blueprints.md): drag and drop and right-click menus in inventories; [the deferred list](deferred.md).
* **Update**: [Blueprints](manual/05-blueprints.md): items dragged between inventories and within a list, Copy and Paste; [the deferred list](deferred.md).
