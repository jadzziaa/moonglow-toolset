//! The request Help › Check for Updates… and the daily check make
//! (`mg_ui::updates`): GitHub's latest release of Moonglow, as JSON.
//!
//! What is sent is the request alone: `GET` of the release's address, with
//! `User-Agent: Moonglow/<version>` (GitHub's API refuses a request without
//! one) and the API's `Accept` and version headers. No identifier, path,
//! cookie or token; a proxy is used where the environment names one
//! (`HTTPS_PROXY`, `ALL_PROXY`).

use std::time::Duration;

/// The longest the request may take, all told.
const TIMEOUT: Duration = Duration::from_secs(10);

/// The most of an answer that is read (a release's is a few kilobytes).
const LIMIT: u64 = 1 << 20;

/// GitHub's answer for the latest release, or why there is none.
pub(crate) fn latest_release() -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(TIMEOUT))
        .https_only(true)
        .user_agent(concat!("Moonglow/", env!("CARGO_PKG_VERSION")))
        .accept("application/vnd.github+json")
        .build()
        .into();
    let mut answer = agent
        .get(mg_ui::updates::LATEST_API)
        .header("X-GitHub-Api-Version", "2022-11-28")
        .call()
        .map_err(|e| match e {
            ureq::Error::StatusCode(403 | 429) => {
                "GitHub's limit on requests was reached; try again in an hour".to_string()
            }
            ureq::Error::StatusCode(404) => "GitHub lists no release".to_string(),
            ureq::Error::StatusCode(code) => format!("GitHub answered {code}"),
            ureq::Error::HostNotFound => "not connected to the Internet".to_string(),
            ureq::Error::Timeout(_) => "GitHub didn't answer in time".to_string(),
            e => e.to_string(),
        })?;
    answer.body_mut().with_config().limit(LIMIT).read_to_string().map_err(|e| e.to_string())
}
