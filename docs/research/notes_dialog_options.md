---
type: Research Note
title: Conversation Editor options (Aurora's)
description: What each of Aurora's Conversation Editor options does, worked out on a probe conversation, with Aurora's defaults.
tags: [conversations, options, aurora]
generated: { by: claude-code/claude-opus-5-5, at: 2026-10-01T07:22:02Z }
---

# Conversation Editor options (Aurora's)

Options › Conversation Editor, worked out in Aurora 89.8193.37 on a probe
conversation (`examples/dialog_probe_module.rs`: "Hello" answered by "Hi",
and "Second"; captures under `dialog-options/`). Defaults as Aurora's
`nwtoolset.ini` has them. Moonglow: `dialog_view.rs`, `mg_module::dialog`
(`move_link`, `link_lines`); tested in
`conversation_options_popup_link_directions_drag_and_backup`.

- **Show popup when creating a new text entry** (on): Add opens Input
  Text, "Enter what the NPC says next:" or "Enter what the player says
  next:" (dialog.tlk 67057, 67056) over "<< Enter text here >>" (10336),
  selected so typing replaces it. OK adds the line with that text and
  leaves the parent selected (Add again adds a sibling); Cancel adds
  nothing. Off, Moonglow adds an empty line, selects it and edits its text
  in place.
- **Paste Link Options** (Link Destination To Source): Paste As Link gives
  the selected line (the destination) a link to the copied one (the
  source). Link Source To Destination gives the copied line a link to the
  selected one ("Second" copied, "Hi" selected: "Second" gets a grey "Hi").
- **Drag Link Options** (Link Source To Destination): a plain drag moves a
  line with its branch under the line it is dropped on ("Hi" dropped on
  "Second" leaves "Hello" without it). Ctrl+drag links instead: by default
  the dragged line (the source) gets a link to the one dropped on; the
  other setting links the other way.
- **Automatically backup the conversation files, every 5 minutes** (on):
  Aurora writes each open conversation as `<name>.bak` (a DLG) into the
  module's working folder (`modules/temp0`). Moonglow writes the open
  conversations, while the module has unsaved changes, to
  `moonglow-backups/<module>/<name>.bak` in the temporary folder: never into
  the NWN user folder.
