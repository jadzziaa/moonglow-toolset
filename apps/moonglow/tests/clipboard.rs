//! The app's text goes through the system clipboard (a builder's report
//! on 5.5.0: Copy in a Properties field did nothing in other programs, and
//! Paste gave what was copied in Moonglow before). eframe takes it only
//! with its `clipboard` feature; without it egui-winit keeps a string of
//! its own. Here, text set through one of egui-winit's clipboards (eframe's,
//! with the features eframe gives it) is read back through another: the
//! same only through the system's.
//!
//! On Windows and macOS it runs as is (CI's runners). On Linux it would
//! take the desktop's clipboard, so it is ignored there and run by hand on
//! the off-screen display (`tools/locks.sh hold display -- …`, with
//! `DISPLAY` the off-screen session's and no `WAYLAND_DISPLAY`).

/// The text set through one clipboard is read through another, and what
/// the clipboard held before is put back.
#[test]
#[cfg_attr(target_os = "linux", ignore = "takes the desktop's clipboard: by hand, off-screen")]
fn text_goes_through_the_system_clipboard() {
    use egui_winit::clipboard::Clipboard;
    // (SAFETY: no display handle: arboard opens its own connection.)
    #[expect(unsafe_code)]
    let open = || unsafe { Clipboard::new(None) };
    let mut one = open();
    let before = one.get();
    let text = format!("Moonglow clipboard test {}", std::process::id());
    one.set_text(text.clone());
    let read = open().get();
    if let Some(before) = before {
        one.set_text(before);
    }
    assert_eq!(read.as_deref(), Some(text.as_str()), "read through another clipboard");
}
