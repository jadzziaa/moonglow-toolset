//! The requests: GitHub's API for the releases, and a release's files.
//!
//! [`Http`] is the seam: the desktop app gives [`Client`] (feature
//! `http`), the tests a server of their own on this computer (`serve`) or
//! answers made up ([`from_fn`]). The command-line tool and a window
//! given none never reach the network.

use std::io::Write;
use std::sync::Arc;

/// The most of an API answer that is read (a release's is a few kilobytes,
/// a list of twenty some hundred).
pub const ANSWER_LIMIT: u64 = 4 << 20;

/// How far a download has got: bytes so far, and of how many (if said).
pub type Progress<'a> = &'a mut dyn FnMut(u64, Option<u64>);

/// What makes the requests.
pub trait Http: Send + Sync {
    /// An API request (`GET`, JSON): its body, or why there is none.
    fn ask(&self, url: &str) -> Result<String, String>;

    /// A file: its body written to `to`, at most `limit` bytes.
    fn download(
        &self,
        url: &str,
        to: &mut dyn Write,
        limit: u64,
        progress: Progress,
    ) -> Result<(), String>;
}

/// A file's body, at most `limit` bytes.
pub fn get(http: &dyn Http, url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    http.download(url, &mut body, limit, &mut |_, _| {})?;
    Ok(body)
}

/// Answers made up by `answer` (a URL's body, or why there is none), for
/// tests.
pub fn from_fn(
    answer: impl Fn(&str) -> Result<Vec<u8>, String> + Send + Sync + 'static,
) -> Arc<dyn Http> {
    struct Answers<F>(F);
    impl<F: Fn(&str) -> Result<Vec<u8>, String> + Send + Sync> Http for Answers<F> {
        fn ask(&self, url: &str) -> Result<String, String> {
            String::from_utf8((self.0)(url)?).map_err(|e| e.to_string())
        }
        fn download(
            &self,
            url: &str,
            to: &mut dyn Write,
            limit: u64,
            progress: Progress,
        ) -> Result<(), String> {
            let body = (self.0)(url)?;
            if body.len() as u64 > limit {
                return Err(format!("{url} is larger than it should be"));
            }
            to.write_all(&body).map_err(|e| e.to_string())?;
            progress(body.len() as u64, Some(body.len() as u64));
            Ok(())
        }
    }
    Arc::new(Answers(answer))
}

#[cfg(feature = "http")]
pub use client::Client;

#[cfg(feature = "http")]
mod client {
    //! What is sent is the request alone: `GET` of the address, with
    //! `User-Agent: Moonglow/<version>` (GitHub's API refuses a request
    //! without one), and for the API its `Accept` and version headers. No
    //! identifier, path, cookie or token; a proxy is used where the
    //! environment names one (`HTTPS_PROXY`, `ALL_PROXY`).

    use std::io::{Read, Write};
    use std::time::Duration;

    use super::{ANSWER_LIMIT, Http, Progress};

    /// The longest an API request may take, all told.
    const ASK_TIMEOUT: Duration = Duration::from_secs(10);
    /// A download: to connect, and the longest wait for its next bytes.
    const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
    const READ_TIMEOUT: Duration = Duration::from_secs(30);

    /// The requests, over HTTPS with ureq (rustls, its own certificates).
    #[derive(Debug)]
    pub struct Client {
        agent: ureq::Agent,
    }

    impl Client {
        /// HTTPS only: the app's.
        pub fn new() -> Client {
            Client::with(true)
        }

        /// Plain HTTP allowed, for a test's server on this computer.
        pub fn plain_http_for_tests() -> Client {
            Client::with(false)
        }

        fn with(https_only: bool) -> Client {
            let agent = ureq::Agent::config_builder()
                .https_only(https_only)
                .user_agent(format!("Moonglow/{}", mg_core::release::version()))
                // (GitHub's downloads redirect to its file host.)
                .max_redirects(10)
                .build()
                .into();
            Client { agent }
        }
    }

    impl Default for Client {
        fn default() -> Client {
            Client::new()
        }
    }

    fn why(e: ureq::Error) -> String {
        match e {
            ureq::Error::StatusCode(403 | 429) => {
                "GitHub's limit on requests was reached; try again in an hour".to_string()
            }
            ureq::Error::StatusCode(404) => "GitHub has no such release or file".to_string(),
            ureq::Error::StatusCode(code) => format!("GitHub answered {code}"),
            ureq::Error::HostNotFound => "not connected to the Internet".to_string(),
            ureq::Error::Timeout(_) => "GitHub didn't answer in time".to_string(),
            e => e.to_string(),
        }
    }

    impl Http for Client {
        fn ask(&self, url: &str) -> Result<String, String> {
            let mut answer = self
                .agent
                .get(url)
                .header("Accept", "application/vnd.github+json")
                .header("X-GitHub-Api-Version", "2022-11-28")
                .config()
                .timeout_global(Some(ASK_TIMEOUT))
                .build()
                .call()
                .map_err(|e| match e {
                    ureq::Error::StatusCode(404) => "GitHub lists no release".to_string(),
                    e => why(e),
                })?;
            answer
                .body_mut()
                .with_config()
                .limit(ANSWER_LIMIT)
                .read_to_string()
                .map_err(|e| e.to_string())
        }

        fn download(
            &self,
            url: &str,
            to: &mut dyn Write,
            limit: u64,
            progress: Progress,
        ) -> Result<(), String> {
            let mut answer = self
                .agent
                .get(url)
                .header("Accept", "application/octet-stream")
                .config()
                .timeout_connect(Some(CONNECT_TIMEOUT))
                .timeout_recv_response(Some(READ_TIMEOUT))
                .timeout_per_read(Some(READ_TIMEOUT))
                .build()
                .call()
                .map_err(why)?;
            let total = answer.body().content_length();
            if total.is_some_and(|t| t > limit) {
                return Err("the file is larger than it should be".into());
            }
            let mut reader = answer.body_mut().as_reader();
            let mut buffer = vec![0u8; 1 << 16];
            let mut got = 0u64;
            loop {
                let n = reader.read(&mut buffer).map_err(|e| match e.kind() {
                    std::io::ErrorKind::TimedOut => "the download stalled".to_string(),
                    _ => format!("the download broke off: {e}"),
                })?;
                if n == 0 {
                    break;
                }
                got += n as u64;
                if got > limit {
                    return Err("the file is larger than it should be".into());
                }
                to.write_all(&buffer[..n]).map_err(|e| format!("can't write the download: {e}"))?;
                progress(got, total);
            }
            if total.is_some_and(|t| got < t) {
                return Err("the download broke off".into());
            }
            Ok(())
        }
    }
}
