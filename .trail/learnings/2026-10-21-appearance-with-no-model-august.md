---
title: "An appearance with no model: one rule, said under the field and in the preview"
author: August
date: 2026-10-21
branch: worktree-agent-a19acbc88a8fae64e
touches:
  - crates/mg-rules/src/appearance.rs
  - crates/mg-module/src/doctor.rs
  - crates/mg-preview/src/lib.rs
  - crates/mg-preview/src/object.rs
  - crates/mg-ui/src/blueprint/mod.rs
  - crates/mg-ui/src/model_view.rs
  - crates/mg-ui/src/appearance_gallery.rs
---

## Decided
- "Does this object's appearance have a usable model" is `mg_rules::appearance` (`AppearanceRow::of(kind, struct)` then `.model(table, resman)`; `GameData::appearance_problem` for both at once). It lives in mg-rules because mg-preview and mg-module both build on it and on neither of each other. It takes `&TwoDa` and `&ResMan`, not `GameData`, because the content doctor has only a `ResMan` and its own table cache.
- Its `Display` is the one wording everywhere: Verify's `object-row` message, `PreviewError::Appearance` (the preview pane and the model viewer) and the red line under the Appearance field (`Form::model_warning`). The corpus doctor test counts findings by check, not text, so the wording can change.
- A row "is there" when its label column or its model column has something; past the end or blank is `NotARow`, a label with no model `NamesNone`, `USER` `Reserved`, else `Missing(model)` by resman name. Items: the base item row only (their models are put together from parts). Creatures: `MODELTYPE` starting with P has no model of its own; an empty MODELTYPE is a single model (the preview's reading; the doctor used to skip it).
- A missing field reads as 0 (the game's reading), except an item's `BaseItem`: an inventory entry has none, and is not checked.

## Watch out
- `ModelView::refresh` used to `return` when the preview failed, so the page kept drawing the previous model after a bad appearance was chosen. It now sets `model = Err` and drops the composed model; `open` already did.
- A UI test with a GPU render state (to see the inline preview) cannot use `h.run()` while a model plays: the view keeps repainting. Step frames with `run_steps`, and type into a Filter field by hand rather than with `type_into_hint`.
- The module's own 2DAs are in the game data only from the module layer made when it opens (`Module::container()` is a copy): a test that needs a module 2DA writes it into the .mod before `open_module`.
- `mg_testkit::scratch_dir` empties its folder each call: call it once per test for pictures.
