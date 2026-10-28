The user manual, a chapter per file, also built into the program (Help › User Manual, which shows each chapter without its frontmatter) and shipped beside it. [README.md](README.md) is its contents page.

<!-- okf index: generated; edit freely, or rerun `okf index` -->

# Manual

* [Moonglow Toolset: User Manual](README.md) - Front page and contents of the Moonglow Toolset user manual, which is also built into the program (Help, User Manual).

# Manual Page

* [Getting started](01-getting-started.md) - What Moonglow needs, installing it, the first start (finding the game) and a first module.
* [The main window](02-main-window.md) - The main window - menus, keyboard shortcuts, the module tree, tabs and windows, the palette, the resource browser, the model viewer, the log, and screen readers and desktop agents.
* [Modules](03-modules.md) - Modules - opening and saving, where things are used and renaming, find and replace, advanced search, nasher projects, merging in git (mg merge-gff as the merge driver), the project file (moonglow.toml), your builder code, described variables (variables.toml), claimed and protected areas (the claims table), the project's commands (File › Project, update and submit), recovering unsaved work, Module Properties, haks and talk tables, the hak editor, the 2DA editor, import and export.
* [Areas](04-areas.md) - Areas - the viewer's toolbar, minimaps and camera, selecting, arranging and placing objects, Setup Store, duplicate tags, claimed and protected areas, painting placeables, painting terrain, tiles, Area Properties, area sounds and making tilesets.
* [Blueprints](05-blueprints.md) - Blueprints - palettes, the blueprint editors and wizards, and instances.
* [Conversations](06-conversations.md) - Conversations - the conversation tree, a line's tabs, writing conversations elsewhere, the Script Wizard and backups.
* [Scripts](07-scripts.md) - Scripts - the script editor, finding your way in code, and the compiler.
* [Journal and factions](08-journal-and-factions.md) - The Journal Editor and the Faction Editor.
* [Build, verify and test](09-build-and-test.md) - Build Module, publishing to NWSync, long work, Verify Module and the fixes beside its findings, Test Module, reloading haks and 2DAs, and Area Statistics.
* [Options](10-options.md) - The Options dialog page by page (folders, area, general with the graphics backend, script editor, conversation editor, sounds, verify, language, keyboard, updates) and where the settings are kept.
* [Command-line tools](11-command-line.md) - The mg command-line tool - archives and files, the game's resources, modules, setting fields and edit files, several areas at once, JSON output, nasher projects and NWScript in other editors.
* [Differences from Aurora](12-differences.md) - Where Moonglow deliberately departs from Aurora.
* [Troubleshooting](13-troubleshooting.md) - Troubleshooting - the game is not found, no GPU in the area viewer, tiles that disappear on AMD under Windows (the graphics backend), Test Module does nothing, no sound, after a crash, and where Moonglow keeps its files.
* [Coming from Aurora](14-coming-from-aurora.md) - For builders who know Aurora - the Aurora problems Moonglow doesn't have, the habits to change, what is easy to miss, and going back to Aurora.
* [Plugins](15-plugins.md) - Plugins for the people who use them - what a plugin can and cannot do, installing, enabling and removing, a project's own plugins (asked about when the project is opened), a plugin's settings, commands, checks and the fixes beside their findings, what plugins do by themselves (before a save, on a paste, on what a wizard makes), the console, the command line and when something goes wrong.
* [Writing plugins](16-writing-plugins.md) - Writing plugins - a first plugin, how a plugin runs (events too, a resource's own around a save and the after-create hook), the manifest, reading and editing the module, commands on what is selected, the game's data, talking to the user, settings (ctx.settings, their scopes and where each is kept), files and haks outside the module, areas and terrain, checks and the fixes of their findings, several files, trying and testing (a plugin's own tests, mg plugin test), editor types, versions, sharing, and a project's own plugins.
* [Plugin API reference](17-plugin-api.md) - Reference of the plugin API - mg, findings and their fixes (ctx.fix), ctx.selection and ctx.open, ctx.event (the events, a resource's own around a save, objects placed or pasted, resources made, and what a handler may edit), ctx.module, ctx.game, ctx.edit, ctx.hak, ctx.terrain, logging, progress and UI (files and folders the user chooses), pictures, ctx.plugin, ctx.settings, Luau, limits and what the API doesn't have yet.
* [NUI Creator](18-nui-creator.md) - The NUI Creator - designing an Enhanced Edition UI window (JUI) on a canvas with the game's skin and fonts, images, keys, binds, swap layout variants, event handlers (or one event script for several windows), the settings file, building its scripts, making windows without the window (render, layout, import), and what the game refuses or does on its own.
