//! The dock's look in the suite's theme: egui_dock's style made from the
//! tokens and the marks painted over its tabs, which live in the kit
//! (`mg_uikit::dock`, its `dock` feature) so that Moonglow Viewer takes
//! them from the same place; re-exported here for the window's callers.

pub(crate) use mg_uikit::dock::{marks, style};
