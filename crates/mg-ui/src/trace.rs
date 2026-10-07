//! The debug log (Options › General › Write a debug log, or the
//! `MOONGLOW_DEBUG_LOG` environment variable): what Moonglow does as it
//! does it — each action, tabs opened and where, the panes, what an area's
//! view and the palettes find — in `debug-log.txt` in Moonglow's data
//! folder, to send with a report of something that fails without a word.
//! A new file each time Moonglow starts; one that grows past
//! [`LIMIT`] is set aside as `debug-log.1.txt` (in place of the one set
//! aside before) and a new one begun, so a log left on takes two files'
//! room at most.

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

static ON: AtomicBool = AtomicBool::new(false);
static STATE: Mutex<Option<State>> = Mutex::new(None);

/// How large the debug log's file grows before it is set aside, in bytes.
pub const LIMIT: u64 = 32 << 20;

struct State {
    file: File,
    /// Where it is, how much is in it, and how much it may hold.
    path: PathBuf,
    written: u64,
    limit: u64,
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
            *state = Some(State {
                file,
                path,
                written: 0,
                limit: LIMIT,
                started: Instant::now(),
                last: HashMap::new(),
            });
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
    let line = format!("[{t:9.3}] {}\n", text.replace('\n', "\n            "));
    // Full: set aside, and a new file begun.
    if state.written + line.len() as u64 > state.limit {
        let aside = state.path.with_file_name("debug-log.1.txt");
        let _ = state.file.flush();
        if std::fs::rename(&state.path, &aside).is_ok()
            && let Ok(file) = File::create(&state.path)
        {
            state.file = file;
            state.written = 0;
            let said = format!("[{t:9.3}] (the log before this is in {})\n", aside.display());
            let _ = state.file.write_all(said.as_bytes());
            state.written += said.len() as u64;
        }
    }
    let _ = state.file.write_all(line.as_bytes());
    state.written += line.len() as u64;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_log_is_set_aside_and_a_new_one_begun() {
        let dir = std::env::temp_dir().join(format!("mg-trace-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("debug-log.txt");
        let mut state = State {
            file: File::create(&path).unwrap(),
            path: path.clone(),
            written: 0,
            limit: 200,
            started: Instant::now(),
            last: HashMap::new(),
        };
        for i in 0..12 {
            write(&mut state, &format!("line {i} of some length to fill the file"));
        }
        let now = std::fs::read_to_string(&path).unwrap();
        let aside = std::fs::read_to_string(dir.join("debug-log.1.txt")).unwrap();
        assert!(now.len() <= 200 && aside.len() <= 200, "{} {}", now.len(), aside.len());
        assert!(now.contains("the log before this is in"));
        assert!(now.contains("line 11") && !now.contains("line 0 "));
        assert!(!aside.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
