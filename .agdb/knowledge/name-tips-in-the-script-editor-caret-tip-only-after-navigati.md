---
type: Decision
title: "Name tips in the script editor: caret tip only after navigation keys, pointer tip loses to the error tip"
description: "When the script editor shows a declaration tip over a name (pointer or caret), what dismisses the caret tip, and how the completion list's detail is placed."
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-23T17:32:24Z }
touches: ["crates/mg-ui/src/script_view.rs:caret_tip", "crates/mg-ui/src/script_view.rs:completion_detail"]
asks:
  - When does the tip over the name at the cursor show?
  - Why does no declaration tip show over the right-click menu?
  - How is the completion detail kept inside the window?
  - "Why are the tips drawn on the editor's background?"
fingerprint: v2-ac3f1e3c7c0d4700
created: 2026-11-23
author: augustaugur
---
5.2.0 W5 (Jasperre's report), with the review round's fixes. Options › Script Editor › Show declarations in tips (`Settings::no_symbol_tips`, on by default).

- Pointer: `name_at` finds the identifier under the pointer from the text galley's row glyphs (cached per slot by text hash, cursor index and the symbols' Arc), shown with `on_hover_ui_at_pointer` unless the error wave's tip showed this frame (`error_tip`), the completion list is open, or the text menu is open or asked for (`menu_up`).
- Caret: `caret_tip` is true only after ArrowLeft/Right/Up/Down, Home, End, PageUp or PageDown with no other key, text, paste, IME, mouse wheel or pointer press in the frame, and stays while the cursor index is unchanged and the editor has focus (temp data at ("caret-tip", key)). A jump, a click or typing never shows it. Shown with `Tooltip::always_open` under the caret, and only while the caret's rect is inside the editor's clip rect (hidden, not dismissed: PageDown's own scroll lands a frame later).
- Review gotcha: `text_menu` consumes Shift+F10 and, while open, `widgets::text_menus_begin` strips Up/Down/Enter/Escape from the frame's events, so caret_tip never saw a key to dismiss it and the tip stood over Undo/Redo. Hence `kept_off` = `menu_wanted || widgets::text_menu_open(ctx)`, which clears the caret state.
- Contrast: palette colors are chosen for the editor's own ground (`ScriptStyle.background`); on the theme's popup ground they failed (dark scheme in the Light theme). `SymbolDetail.ground` paints the tip's inside (a Noop shape set after, the menu_margin added) and the detail's frame fill with it; the doc line takes the comment color.
- Completion detail: `symbol_detail` cached in ctx temp data; `completion_detail` puts it right of the list where 200 points fit, else left, else under (over where no room under), y clamped to content_rect().shrink(4) in every placement (a tall list in a small window leaves no room either way: the detail covers the list's end).
- Tests: script_bottom::a_name_s_declaration_shows_in_a_tip (menu, wheel, short window), scripts::the_completion_detail_stays_inside_the_window (tall list). A wrapped Label's text is in accesskit as TextRun children with `value`, not one label: query by label or value. In kittest, a zoom factor does not shrink content_rect: a 300% case is a small window in points (430 x 330).
