---
type: Decision
title: "Form windows' pages on one scroll (M4 U9): PageScroll, PageList, form_pages, what broke and why"
description: "How a blueprint's, Module Properties' and Area Properties' pages stand on one scroll since U9 (mg_uikit::pages::PageScroll drawn only in sight, PageList beside, ok_cancel_header), and the egui traps found doing it."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-16T00:38:00Z }
touches: ["crates/mg-uikit/src/pages.rs:PageScroll", crates/mg-ui/src/form_pages.rs, "crates/mg-ui/src/blueprint/mod.rs:edit_many", crates/mg-ui/tests/app/form_pages.rs]
asks:
  - Why does a field lose the keyboard or its text when the form scrolls?
  - How do I go to a page of a blueprint window in a test?
  - Why does the page jump when a page above comes into view?
  - "Why did reading a widget's response take its focus away?"
fingerprint: v2-f563188d41883190
created: 2026-11-16
---
## Decided
- The window's pages (edit_many, module_props::pages, area_props::pages) are `mg_uikit::pages::PageScroll` beside `PageList` (form_pages::list; under `form_pages::LIST_UNDER` = 150 + 28 + NARROW_FORM the U6 `PageRail` instead). The Properties panel keeps one page at a time (U6 decision 1): `FormShown::all` is `None` there.
- `blueprint_pages` / `module_page` mean "the page at the top". A value differing from what the scroll reported last frame is a jump (Inventory…, open_tabs restore, tests inserting it); the host writes the top back only if nothing asked for another during drawing, and asks a repaint when something did.
- The scroll keeps its place as (page, offset into it), not pixels: held to the page at the top while its bar is at the top, else to the first page whose bar is in view (negative offset), so a page above measured anew or growing moves nothing in view; a page above the held one re-measured => `request_discard`.
- Drawn: pages in sight, the page holding the keyboard (else egui's dead-man switch drops focus and the typed text), and on a Tab press the pages either side. A widget the keyboard came to by a key out of sight is scrolled to.
- OK/Cancel: `blueprint::ok_cancel_header` (EditorHeader end slot), only these three hosts; the other 10 callers still `ok_cancel` at the foot.
- Page groups: each page is an accesskit group "Basic page" whose bounds are its part in sight (tests: `common::page`, `in_page`, `page_row`).
## Gotchas (each cost a debugging round)
- `new_child(UiBuilder::id_salt)` mixes the parent's auto-id counter into the child's unique id: a page's widget ids changed when a page before it went out of sight -> focus lost on Tab. Use `UiBuilder::id(explicit)`.
- `ctx.read_response(id)` is not pure: `get_response` surrenders focus when a press happened elsewhere this pass. Read it only after a key moved focus, never on a press.
- `ScrollArea` state is keyed by the parent Ui's id: a tab dragged into its own window gets a fresh offset 0. Compare against `scroll_area::State::load(ui.make_persistent_id(salt))`, not a value you stored.
- Child max height must not depend on the sideways bar (it toggled every frame); a page's width widens the content only when measured the same two frames running.
- `with_layout(left_to_right(Center))` in a page centers its row in the page's view-high room; use `horizontal` / `horizontal_wrapped`.
## Testing
- Short pages put neighbors in view: scope lookups (`in_page`, `.last()` for a window over the page, `menu(&h, "Edit")` for the Top Bar). `type_into_hint` takes the first match.
- The overlap test walks each page by the wheel in the page's left margin (a list on the page takes the wheel over it).
## Review (2026-11-15/16)
- Held to the first page whose bar is in view (negative offset), so a page above growing moves nothing in view; a page with under half a point in sight is not "the top" (heights are whole points; a 0.1 sliver stuck the overlap walk).
- Pages above the held one are placed at their kept tops, never by what they measure (the measure only triggers `request_discard`).
- The page before the first in sight is drawn on a Tab only while the keyboard is in the scroll: from the page list (drawn before the scroll) Tab went to it.
- A widget the keyboard came to counts as the scroll's only if its centre is in the viewport or egui clipped it away whole (`interact_rect` empty): a header button lies over pages drawn out of sight.
- Form windows open at 980 (blueprints) and 930 (Module/Area Properties) beside the 150 list, with no tab margin (flush); a window opens at most 85% of the main pane wide, so tests needing the full width use a 2000-wide screen.
- `form_pages::narrow(body)` judges on the page's width inside its 28 margins; `LIST_UNDER` = 150 + 28 + NARROW_FORM.
- Several areas' scroll and kept page are keyed by the whole set (`area_props::several_page`).

