//! The debug log (Options › General › Write a debug log, or the
//! `MOONGLOW_DEBUG_LOG` environment variable): what Moonglow does as it
//! does it — each action, tabs opened and where, the panes, what an area's
//! view and the palettes find — in `debug-log.txt` in Moonglow's data
//! folder, to send with a report of something that fails without a word.
//! A new file each time Moonglow starts.

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

static ON: AtomicBool = AtomicBool::new(false);
static STATE: Mutex<Option<State>> = Mutex::new(None);

struct State {
    file: File,
    started: Instant,
    /// What each topic last said ([`changed`]).
    last: HashMap<String, String>,
}

/// Where the debug log is written.
pub fn path() -> Option<PathBuf> {
    crate::recovery::data_dir().map(|d| d.join("debug-log.txt"))
}

/// Whether the debug log is being written.
pub fn on() -> bool {
    ON.load(Ordering::Relaxed)
}

/// Turns the debug log on (opening its file, emptied, the first time) or
/// off. Graphics libraries' warnings go into it too.
pub fn set(on: bool) {
    if on == self::on() {
        return;
    }
    if on {
        let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
        if state.is_none() {
            let Some(path) = path() else { return };
            if let Some(dir) = path.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            let Ok(file) = File::create(&path) else { return };
            *state = Some(State { file, started: Instant::now(), last: HashMap::new() });
            // (Once: another logger may be there in tests.)
            if log::set_logger(&LIBRARIES).is_ok() {
                log::set_max_level(log::LevelFilter::Info);
            }
        }
    }
    ON.store(on, Ordering::Relaxed);
    if on {
        note(format!(
            "Moonglow Toolset {} on {} {}",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH
        ));
    }
}

fn write(state: &mut State, text: &str) {
    let t = state.started.elapsed().as_secs_f64();
    // (Long values, a resource's bytes in an action, are cut.)
    let mut text: String = text.chars().take(1200).collect();
    if text.len() == 1200 {
        text.push('…');
    }
    let _ = writeln!(state.file, "[{t:9.3}] {}", text.replace('\n', "\n            "));
    let _ = state.file.flush();
}

/// Writes a line.
pub fn note(text: impl AsRef<str>) {
    if !on() {
        return;
    }
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(state) = state.as_mut() {
        write(state, text.as_ref());
    }
}

/// Writes what `topic` says now, if it is not what it said last: for what
/// is looked at every frame (the panes, an area's view).
pub fn changed(topic: &str, text: impl FnOnce() -> String) {
    if !on() {
        return;
    }
    let text = text();
    let mut state = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let Some(state) = state.as_mut() else { return };
    if state.last.get(topic).is_some_and(|last| *last == text) {
        return;
    }
    state.last.insert(topic.to_string(), text.clone());
    write(state, &format!("{topic}: {text}"));
}

/// The graphics libraries' messages (wgpu, eframe): warnings and errors,
/// and what they say of the adapter chosen.
struct Libraries;
static LIBRARIES: Libraries = Libraries;

impl log::Log for Libraries {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        on() && (metadata.level() <= log::Level::Warn
            || ["eframe", "egui_wgpu", "egui_winit"]
                .iter()
                .any(|t| metadata.target().starts_with(t)))
    }

    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            note(format!("[{} {}] {}", record.level(), record.target(), record.args()));
        }
    }

    fn flush(&self) {}
}
