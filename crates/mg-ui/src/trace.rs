//! The debug log (Options › General › Write a debug log, on unless
//! switched off; or the `MOONGLOW_DEBUG_LOG` environment variable): what Moonglow does as it
//! does it — each action, tabs opened and where, the panes, what an area's
//! view and the palettes find — in the `logs` folder of Moonglow's data
//! folder, to send with a report of something that fails without a word.
//! Each line has the time of day (UTC) and the seconds since the start.
//! A new file each time Moonglow starts, named by when
//! (`debug-log-20261007-155903.txt`), the last [`SESSIONS`] kept: the
//! log of a session that crashed is still there after the next start. One
//! that grows past [`LIMIT`] is set aside (`….1.txt`, in place of the one
//! set aside before) and a new one begun, so a session takes two files'
//! room at most.

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

static ON: AtomicBool = AtomicBool::new(false);
static STATE: Mutex<Option<State>> = Mutex::new(None);

/// How large the debug log's file grows before it is set aside, in bytes.
pub const LIMIT: u64 = 4 << 20;

static BY_DEFAULT: AtomicBool = AtomicBool::new(false);

/// The debug log is written unless the option is off, from here on: the
/// application says so as it starts. (Not so in tests, which write no
/// logs into the user's data folder.)
pub fn by_default() {
    BY_DEFAULT.store(true, Ordering::Relaxed);
}

/// Whether the debug log is to be written, the option being `option`:
/// always with the `MOONGLOW_DEBUG_LOG` environment variable set.
pub fn wanted(option: bool) -> bool {
    std::env::var_os("MOONGLOW_DEBUG_LOG").is_some()
        || (option && BY_DEFAULT.load(Ordering::Relaxed))
}

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

/// How many sessions' debug logs are kept, this one included.
pub const SESSIONS: usize = 5;

/// The folder the debug logs are written in.
pub fn dir() -> Option<PathBuf> {
    crate::recovery::data_dir().map(|d| d.join("logs"))
}

/// Where this session's debug log is written.
pub fn path() -> Option<PathBuf> {
    static STARTED: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    let stamp = STARTED.get_or_init(|| {
        let (date, time) = utc(SystemTime::now());
        format!("{}-{}", date.replace('-', ""), time[..8].replace(':', ""))
    });
    dir().map(|d| d.join(format!("debug-log-{stamp}.txt")))
}

/// A time as UTC: its date (`2026-10-07`) and its time of day
/// (`15:59:03.123`).
fn utc(at: SystemTime) -> (String, String) {
    let since = at.duration_since(UNIX_EPOCH).unwrap_or_default();
    let (days, of_day) = (since.as_secs() / 86_400, since.as_secs() % 86_400);
    // (Days to a date, the proleptic Gregorian calendar: Howard Hinnant's
    // `civil_from_days`.)
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let (day, month) = (doy - (153 * mp + 2) / 5 + 1, if mp < 10 { mp + 3 } else { mp - 9 });
    let year = yoe + era * 400 + i64::from(month <= 2);
    (
        format!("{year:04}-{month:02}-{day:02}"),
        format!(
            "{:02}:{:02}:{:02}.{:03}",
            of_day / 3600,
            of_day / 60 % 60,
            of_day % 60,
            since.subsec_millis()
        ),
    )
}

/// Lets go of the debug logs of all but the last `keep` sessions in
/// `dir` (a session's files: its log and the one set aside).
fn prune(dir: &std::path::Path, keep: usize) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut logs: Vec<(String, PathBuf)> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let stamp = name.strip_prefix("debug-log-")?.split('.').next()?.to_string();
            name.ends_with(".txt").then(|| (stamp, e.path()))
        })
        .collect();
    let mut sessions: Vec<String> = logs.iter().map(|(s, _)| s.clone()).collect();
    sessions.sort();
    sessions.dedup();
    let old = sessions.len().saturating_sub(keep);
    logs.retain(|(s, _)| sessions[..old].contains(s));
    for (_, path) in logs {
        let _ = std::fs::remove_file(path);
    }
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
            if let Some(dir) = path.parent() {
                prune(dir, SESSIONS);
            }
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
            "Moonglow Toolset {} on {} {}, {} UTC",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH,
            utc(SystemTime::now()).0
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
    let clock = utc(SystemTime::now()).1;
    let indent = "\n".to_string() + &" ".repeat(26);
    let line = format!("[{clock} {t:9.3}] {}\n", text.replace('\n', &indent));
    // Full: set aside, and a new file begun.
    if state.written + line.len() as u64 > state.limit {
        let aside = state.path.with_extension("1.txt");
        let _ = state.file.flush();
        if std::fs::rename(&state.path, &aside).is_ok()
            && let Ok(file) = File::create(&state.path)
        {
            state.file = file;
            state.written = 0;
            let said =
                format!("[{clock} {t:9.3}] (the log before this is in {})\n", aside.display());
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
            limit: 300,
            started: Instant::now(),
            last: HashMap::new(),
        };
        for i in 0..12 {
            write(&mut state, &format!("line {i} of some length to fill the file"));
        }
        let now = std::fs::read_to_string(&path).unwrap();
        let aside = std::fs::read_to_string(dir.join("debug-log.1.txt")).unwrap();
        assert!(now.len() <= 300 && aside.len() <= 300, "{} {}", now.len(), aside.len());
        // Each line with the time of day and the seconds since the start.
        let first = aside.lines().next().unwrap();
        assert_eq!(&first[..1], "[");
        assert_eq!((&first[3..4], &first[6..7], &first[9..10]), (":", ":", "."), "{first}");
        assert!(now.contains("the log before this is in"));
        assert!(now.contains("line 11") && !now.contains("line 0 "));
        assert!(!aside.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dates_and_the_sessions_kept() {
        let at = |secs: u64| utc(UNIX_EPOCH + std::time::Duration::from_millis(secs * 1000 + 123));
        assert_eq!(at(0), ("1970-01-01".into(), "00:00:00.123".into()));
        // 2026-10-07 15:59:03 UTC, and a leap day.
        assert_eq!(at(1_791_388_743), ("2026-10-07".into(), "15:59:03.123".into()));
        assert_eq!(at(1_709_164_800).0, "2024-02-29");
        // The last sessions' logs stay, with what was set aside of them;
        // older ones and nothing else go.
        let dir = std::env::temp_dir().join(format!("mg-trace-keep-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        for name in [
            "debug-log-20261001-100000.txt",
            "debug-log-20261001-100000.1.txt",
            "debug-log-20261002-100000.txt",
            "debug-log-20261003-100000.txt",
            "debug-log-20261003-100000.1.txt",
            "notes.txt",
        ] {
            std::fs::write(dir.join(name), "x").unwrap();
        }
        prune(&dir, 2);
        let mut left: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(
            left,
            [
                "debug-log-20261002-100000.txt",
                "debug-log-20261003-100000.1.txt",
                "debug-log-20261003-100000.txt",
                "notes.txt"
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
