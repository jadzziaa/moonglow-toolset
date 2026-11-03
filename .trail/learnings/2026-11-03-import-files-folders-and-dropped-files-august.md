---
title: "Import files, folders and dropped files"
author: August
date: 2026-11-03
branch: worktree-agent-ab01fbce4f0392343
cost: { minutes: 117, tokens: 506750 }
touches:
  - crates/mg-module/src/transfer.rs
  - crates/mg-ui/src/transfer.rs
  - apps/mg/src/main.rs
---

File › Import Files…, Import Folder… and files dropped onto the window share the ERF import's plan and window: mg_module::transfer::Incoming (archive or files), plan_import_of, import_incoming; ImportDraft holds the Incoming. Decisions: names keep their case (ResKey Display lower-cases; ResRef compares case-insensitively; IndexMap keeps the module's spelling on replace); module.ifo, archives, .bak, .tlk refused from files only (an ERF's module.ifo still imports); GFF type check applies to archives too; a .nss without .ncs is compiled into the staged module with its includers (build::compile_named_with) so the import stays one Undo step, more than COMPILED_AT_ONCE go to compile_job after; follows settings.auto_compile (off by default). Drops: winit 0.30.13 has no Wayland file drop (only x11, windows, macos handlers): WAYLAND_DISPLAY= (empty) makes winit pick X11. egui 0.36's DroppedFile is a trait (path() method), raw.dropped_files are Arcs of it: a kittest test implements it and pushes into h.input_mut().dropped_files; hovered_files persists across steps until cleared, dropped_files is taken each frame. A script editor with unsaved typing does not follow a module change (ScriptBuffer::is_dirty), so imports warn of it.
