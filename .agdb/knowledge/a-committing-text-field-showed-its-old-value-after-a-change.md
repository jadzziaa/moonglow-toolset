---
type: Gotcha
title: A committing text field showed its old value after a change underneath with no input after it
description: "Why commit_field asks a frame when it replaces its buffer, found by a tileset read again after an outside change"
tags: []
status: stable
generated: { by: agdb/0.2.0, at: 2026-11-22T10:55:57Z }
touches: ["crates/mg-ui/src/widgets.rs:commit_field", "crates/mg-ui/src/tileset_view.rs:text_key"]
asks:
  - "Why does a tileset's form field show the old model after the file changed outside?"
  - Why does commit_field call request_repaint?
  - "Why did W5's tileset reread test pass while the field showed the old value?"
fingerprint: v2-df911dddd9dca855
created: 2026-11-22
author: augustaugur
---
`widgets::commit_field` keeps what is typed in `app.buffers` and, when the value changes underneath while the field lacks the keys (undo, another selection, a file read again), puts the value in the buffer after the field is drawn. It asked for no frame, so where no input followed (W5's reread of a tileset changed by another program) the window rested with the field showing the old text until the next input: the table, the Preview and the form's heading had the new model, the Model field the old (seen docked and floating; M5 S1, 2026-11-22). Now it asks a frame there, so the new value is drawn at once (once: the buffer then equals the value). An earlier claim that a field reached by Tab would set the old text back was not shown (the review could not make it happen: the buffer is updated in the frame the stale text is drawn) and is withdrawn. W5's test had checked "the form shows it" with any node of that value, which the table's row satisfied; it now looks for the text input (`field_with`). Tests: `tileset_panels::a_tileset_changed_outside_is_read_again`, `a_floating_tileset_changed_outside_is_read_again_and_typed_text_kept`.
