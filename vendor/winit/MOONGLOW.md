# winit, patched for Moonglow

winit 0.30.13 as published on crates.io, with one change, in
`src/platform_impl/macos/event_handler.rs`: an event that comes while the
event handler is already running is kept and handled after it returns,
where winit panicked ("tried to handle event while another event is
currently being handled") in a callback that cannot unwind, and the app
aborted. That is rust-windowing/winit#3992, open when this was written
(2026-11-08). A native file dialog (`rfd`'s `runModal`) runs a nested run
loop inside Moonglow's frame, and resizing the dialog, or dragging a file
from Finder over the window, brought such an event: a builder's report on
macOS, the Open dialog resized.

The root `Cargo.toml` patches it in (`[patch.crates-io]`) and leaves it out
of the workspace. Drop it, and the patch, once winit fixes #3992 in a
release eframe uses, or Moonglow's file dialogs no longer block the frame.
Everything else here is winit's own, under its Apache-2.0 license
(`LICENSE`).
