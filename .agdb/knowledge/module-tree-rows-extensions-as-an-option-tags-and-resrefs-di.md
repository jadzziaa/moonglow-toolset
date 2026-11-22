---
type: Decision
title: "Module tree rows: extensions as an option, tags and ResRefs dimmed, rows cut to the dock (M5 W1, rulings 7 and 8)"
description: "How the module tree's resource and placed-object rows decide their words, dimmed second word, accessible name, tip, sort and Filter text under List by name, Show ResRefs, Show tags and Show file extensions, and why."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-21T14:28:26Z }
touches: ["crates/mg-ui/src/tree.rs:RowWords", "crates/mg-ui/src/tree.rs:contents_ui", "crates/mg-uikit/src/tree.rs:keep_after", "crates/mg-ui/src/settings.rs:tree_extensions"]
asks:
  - why does the module tree show blueprints without .utc
  - how do tags and resrefs show in the module tree rows
  - "why is a tree row's accessible name different from what it draws"
  - "why doesn't the module tree scroll sideways any more"
fingerprint: v2-51f488b4419a68da
created: 2026-11-21
author: augustaugur
---
## Decided (owner's rulings 7 and 8 of 2026-11-21)
- `Settings::tree_extensions` (Options › General › Module Tree and Palettes, "Show file extensions in the module tree", off). It governs only the rows that showed an extension before: blueprints, NUI windows, the journal and factions. Areas, conversations and scripts stay ResRef-only (as Aurora and the boards). Option on = the tree as it was before W1.
- One rule, `tree::RowWords::of` (made LAZILY per row when first drawn, cached in `tree::Rows` (Mutex<HashMap<ResKey, Arc<RowWords>>>) inside the groups' lists, never per frame; eager making of all rows with the lists cost 12-16 ms per 25,000 resources, and the lists are remade at every `ws.revision()` change = every edit, so it was a frame over budget per edit; the make is after the out-of-sight skip in the row loop; the Filter uses `Rows::sought` (made anew, not cached); `AreaNames::words_made` counts, `Moonglow::tree_words_made` / `tree_row_words` for tests): words = name when listed by name, else ResRef (+ext where shown or where the group holds another file of that ResRef: only Journal and factions mixes types, and a module's own `module.jrl` and `repute.fac` never share a ResRef, so it bites only for a file put in by hand such as `repute.jrl`; the brief's "other resources" group does not exist); second (dimmed, kit `.second`) = ResRef where Show ResRefs and named, then tag where Show tags, joined " · "; label (AccessKit) = the old full text "Name (resref) (tag)", "guard.utc (TAG)" with the extension always for ResRef-listed filed rows, so tests elsewhere (plugins.rs, area_view.rs, search.rs…) that find rows by `guard.utc` keep working and the label begins with the drawn words; tip = name, file name with ext, "Tag: x"; sought = fold(file name + name + tag-if-shown), so ".utc" and a whole file name find rows whatever is shown.
- Placed objects: with tags, name (or "(no name)") + dimmed tag (Outliner's look), AccessKit stays develop's "Name (TAG)" (apart from the Outliner's "Name, tag TAG"); tip (tree and Outliner) = name + "Tag: x" + how, built lazily in `on_hover_ui`.
- Every tree row is `fit` (ellipsis at the dock edge) as the Outliner's; area marks (claims, "(not in the area list)") keep room via kit `TreeRow::keep_after` with the width measured the frame before (temp id `("tree-area-marks", resref)`), drawn at `TreeRowShown::after_words`. The kept room is the marks' WHOLE width (the unlisted label's galley measured unwrapped), capped at half the row (`available_width() * 0.5`), and the label is `Label::truncate()` in a child ending at the row's right: measuring the drawn (cut) width instead collapses the room to nothing, and no cap left a long name as `A…` beside the 112-point mark at a 190 dock.
- The Resources panel keeps extensions (board Resources.html draws them; its list holds .nss/.ncs pairs).
## Watch out
- A sideways mouse wheel (delta.x) did not scroll the tree in egui_kittest: a guard built on it passed when broken. Measure the row's rect against `left_dock_rect` instead (the kit's row takes `needed` width, past the dock when not fit).
- The before note "builders' asks of 2026-11-16" says tags are baked into the groups' `names` map: no longer true; tags are in `RowWords::second`/`sought`.
