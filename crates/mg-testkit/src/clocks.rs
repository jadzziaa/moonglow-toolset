//! The GPU's core clock, read beside timed frames (read only: nothing is
//! set). On Linux's amdgpu, `/sys/class/drm/card*/device/pp_dpm_sclk` lists
//! the card's clock levels and marks the current one with `*` (or lists
//! the clock between two levels it is at, marked). Elsewhere (another
//! system or driver, CI) there is no such file, and the figures say so.
//!
//! A frame's GPU time follows the clock the card's automatic power
//! management has chosen, and that choice follows how the frames are paced:
//! frame_perf's woods overview took 11.2 ms a frame at about 800 MHz and 6
//! at 1,300 (`docs/research/notes_frames.md`, "the woods test's two GPU
//! levels"), so every view's figures say the clock they were taken at.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// The file that says the core clock of the card whose PCI vendor and
/// device ids are these (a wgpu adapter's `vendor` and `device`), if the
/// system has one.
pub fn sclk_file(vendor: u32, device: u32) -> Option<PathBuf> {
    sclk_file_in(Path::new("/sys/class/drm"), vendor, device)
}

/// [`sclk_file`] among the cards in `drm` (a folder laid out as
/// `/sys/class/drm` is).
pub fn sclk_file_in(drm: &Path, vendor: u32, device: u32) -> Option<PathBuf> {
    let id = |p: PathBuf| {
        let s = std::fs::read_to_string(p).ok()?;
        u32::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok()
    };
    let mut cards: Vec<PathBuf> =
        std::fs::read_dir(drm).ok()?.flatten().map(|e| e.path().join("device")).collect();
    cards.sort();
    cards.into_iter().find_map(|dev| {
        let sclk = dev.join("pp_dpm_sclk");
        (sclk.is_file()
            && id(dev.join("vendor")) == Some(vendor)
            && id(dev.join("device")) == Some(device))
        .then_some(sclk)
    })
}

/// The clock `pp_dpm_sclk`'s text marks as the current one, MHz (0: the
/// core is off, idle).
pub fn sclk_of(text: &str) -> Option<u32> {
    let line = text.lines().find(|l| l.trim_end().ends_with('*'))?;
    let mhz = line.split_whitespace().nth(1)?;
    mhz.to_ascii_lowercase().strip_suffix("mhz")?.parse().ok()
}

/// The clock read every [`Clocks::EVERY`] on a thread of its own, from
/// [`Clocks::start`] to [`Clocks::finish`].
#[derive(Debug)]
pub struct Clocks {
    started: Instant,
    file: Option<PathBuf>,
    /// Dropped to stop the thread, which waits on it between readings (so
    /// what follows `finish` is not kept waiting for its sleep).
    stop: Option<Sender<()>>,
    thread: Option<JoinHandle<Vec<(Duration, u32)>>>,
}

/// What [`Clocks::finish`] read: each clock (MHz) with when, from the
/// start; `None` where there is no file to read.
pub type Read = Option<Vec<(Duration, u32)>>;

impl Clocks {
    pub const EVERY: Duration = Duration::from_millis(100);

    /// Reads `file` (from [`sclk_file`]) until [`Clocks::finish`]; with
    /// none, reads nothing.
    pub fn start(file: Option<PathBuf>) -> Clocks {
        let started = Instant::now();
        let (stop, stopped) = channel::<()>();
        let thread = file.clone().map(|file| {
            std::thread::spawn(move || {
                let mut seen = Vec::new();
                loop {
                    let at = started.elapsed();
                    if let Some(mhz) = read(&file) {
                        seen.push((at, mhz));
                    }
                    let rest = Self::EVERY.saturating_sub(started.elapsed() - at);
                    if stopped.recv_timeout(rest) != Err(RecvTimeoutError::Timeout) {
                        return seen;
                    }
                }
            })
        });
        Clocks { started, file, stop: Some(stop), thread }
    }

    /// When the reading began (the times [`Clocks::finish`] gives are from it).
    pub fn started(&self) -> Instant {
        self.started
    }

    /// The clocks read since [`Clocks::start`] (at least one, however
    /// short the time), or `None` without a file.
    pub fn finish(mut self) -> Read {
        drop(self.stop.take());
        let mut seen = self.thread.take()?.join().ok()?;
        if seen.is_empty()
            && let Some(mhz) = self.file.as_deref().and_then(read)
        {
            seen.push((self.started.elapsed(), mhz));
        }
        Some(seen)
    }
}

fn read(file: &Path) -> Option<u32> {
    std::fs::read_to_string(file).ok().as_deref().and_then(sclk_of)
}

/// What was read, said for a report: the median clock, the least and the
/// most, and how many readings; or that there was nothing to read.
pub fn said(seen: &Read) -> String {
    match seen {
        Some(seen) if !seen.is_empty() => {
            let mut mhz: Vec<u32> = seen.iter().map(|s| s.1).collect();
            mhz.sort_unstable();
            format!(
                "GPU clock {} MHz (median; {} to {}, {} readings)",
                mhz[mhz.len() / 2],
                mhz[0],
                mhz[mhz.len() - 1],
                mhz.len()
            )
        }
        Some(_) => "GPU clock not read (no reading came)".into(),
        None => "GPU clock n/a (no amdgpu pp_dpm_sclk for this adapter: not read)".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_marked_clock() {
        assert_eq!(sclk_of("0: 500Mhz \n1: 1399Mhz *\n2: 2520Mhz \n"), Some(1399));
        assert_eq!(sclk_of("0: 500Mhz *\n1: 2520Mhz \n"), Some(500));
        assert_eq!(sclk_of("0: 0Mhz *\n"), Some(0));
        assert_eq!(sclk_of("0: 500Mhz \n1: 2520Mhz \n"), None);
        assert_eq!(sclk_of(""), None);
        assert_eq!(sclk_of("garbage *"), None);
        assert_eq!(sclk_of("1: fastMhz *"), None);
    }

    #[test]
    fn says_what_was_read() {
        assert!(said(&None).contains("n/a"));
        assert!(said(&Some(Vec::new())).contains("not read"));
        let seen = vec![(Duration::ZERO, 815), (Duration::ZERO, 288), (Duration::ZERO, 1514)];
        assert_eq!(said(&Some(seen)), "GPU clock 815 MHz (median; 288 to 1514, 3 readings)");
    }

    /// The card is found by its ids among cards laid out as sysfs has
    /// them, read beside frames until finished, and with none nothing is
    /// read.
    #[test]
    fn finds_the_card_and_reads_it() {
        let drm = crate::scratch_dir("clocks_drm");
        let card = |name: &str, vendor: &str, device: &str, sclk: Option<&str>| {
            let dev = drm.join(name).join("device");
            std::fs::create_dir_all(&dev).unwrap();
            std::fs::write(dev.join("vendor"), vendor).unwrap();
            std::fs::write(dev.join("device"), device).unwrap();
            if let Some(s) = sclk {
                std::fs::write(dev.join("pp_dpm_sclk"), s).unwrap();
            }
        };
        // (An integrated GPU of the same vendor first, and a connector.)
        card("card0", "0x1002\n", "0x13c0\n", Some("0: 600Mhz *\n"));
        std::fs::create_dir_all(drm.join("card1-DP-1")).unwrap();
        card("card1", "0x1002\n", "0x7550\n", Some("0: 500Mhz \n1: 1321Mhz *\n2: 2520Mhz \n"));
        card("card2", "0x10de\n", "0x2684\n", None);
        let file = sclk_file_in(&drm, 0x1002, 0x7550).expect("card1");
        assert!(file.ends_with("card1/device/pp_dpm_sclk"), "{}", file.display());
        assert_eq!(sclk_file_in(&drm, 0x10de, 0x2684), None, "no pp_dpm_sclk: not amdgpu");
        assert_eq!(sclk_file_in(&drm, 0x8086, 0x1), None);
        assert_eq!(sclk_file_in(&drm.join("none"), 0x1002, 0x7550), None);

        let clocks = Clocks::start(Some(file));
        std::thread::sleep(Clocks::EVERY * 2 + Clocks::EVERY / 2);
        let quick = Instant::now();
        let seen = clocks.finish().unwrap();
        assert!(quick.elapsed() < Clocks::EVERY / 2, "stopped at once: {:?}", quick.elapsed());
        assert!(seen.len() >= 2 && seen.iter().all(|s| s.1 == 1321), "{seen:?}");
        assert!(said(&Some(seen)).starts_with("GPU clock 1321 MHz"));
        // (Finished at once, it still says the clock.)
        let file = sclk_file_in(&drm, 0x1002, 0x13c0);
        assert_eq!(Clocks::start(file).finish().unwrap()[0].1, 600);
        assert_eq!(Clocks::start(None).finish(), None);
    }
}
