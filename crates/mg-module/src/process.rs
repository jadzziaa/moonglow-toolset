//! Running another program (an external script compiler, git) with a time
//! limit: one that hangs (waiting on a licence prompt under Wine, a git
//! asking for credentials) is stopped rather than holding Moonglow up for
//! ever.

use std::io::Read;
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// Runs `command` to its end, as [`Command::output`] does, with nothing on
/// its input; stopped (and an error of kind
/// [`TimedOut`](std::io::ErrorKind::TimedOut)) when it has not ended
/// within `limit`.
pub(crate) fn output_within(command: &mut Command, limit: Duration) -> std::io::Result<Output> {
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command.spawn()?;
    let start = Instant::now();
    // Both pipes are read as the program writes, so a full one does not
    // stop it.
    let read = |pipe: Option<Box<dyn Read + Send>>| {
        let (send, receive) = mpsc::channel();
        if let Some(mut pipe) = pipe {
            std::thread::spawn(move || {
                let mut bytes = Vec::new();
                let _ = pipe.read_to_end(&mut bytes);
                let _ = send.send(bytes);
            });
        }
        receive
    };
    let stdout = read(child.stdout.take().map(|p| Box::new(p) as Box<dyn Read + Send>));
    let stderr = read(child.stderr.take().map(|p| Box::new(p) as Box<dyn Read + Send>));
    let mut pause = Duration::from_millis(1);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if start.elapsed() >= limit {
            let _ = child.kill();
            let _ = child.wait();
            return Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("had not finished after {} s, and was stopped", limit.as_secs()),
            ));
        }
        std::thread::sleep(pause);
        pause = (pause * 2).min(Duration::from_millis(25));
    };
    // What it wrote, as long as the pipes close in time (a program it
    // started may hold them open after it ends).
    let left = || limit.saturating_sub(start.elapsed()).max(Duration::from_secs(1));
    let stdout = stdout.recv_timeout(left()).unwrap_or_default();
    let stderr = stderr.recv_timeout(left()).unwrap_or_default();
    Ok(Output { status, stdout, stderr })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn a_program_is_run_and_what_it_wrote_kept() {
        let out = output_within(
            Command::new("sh").args(["-c", "echo out; echo err >&2; exit 3"]),
            Duration::from_secs(30),
        )
        .unwrap();
        assert_eq!((out.stdout.as_slice(), out.stderr.as_slice()), (&b"out\n"[..], &b"err\n"[..]));
        assert_eq!(out.status.code(), Some(3));
    }

    /// A program that does not end is stopped at the limit, not waited on.
    #[test]
    fn a_program_that_hangs_is_stopped() {
        let start = Instant::now();
        let e =
            output_within(Command::new("sleep").arg("30"), Duration::from_millis(200)).unwrap_err();
        assert_eq!(e.kind(), std::io::ErrorKind::TimedOut);
        assert!(start.elapsed() < Duration::from_secs(10));
    }
}
