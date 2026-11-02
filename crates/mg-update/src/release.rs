//! The releases GitHub lists, read from its REST API's answers.

use serde_json::Value;

use crate::Version;

/// Where the releases are: GitHub's, or a test's server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Api {
    /// The latest release (never a draft or a pre-release).
    pub latest: String,
    /// The releases, newest first (with Include pre-releases).
    pub list: String,
    /// What a release's own page and its downloads begin with: links
    /// elsewhere are not followed.
    pub pages: String,
    pub downloads: String,
}

/// GitHub's latest release of Moonglow (REST API): drafts and pre-releases
/// are never it.
pub const LATEST_API: &str =
    "https://api.github.com/repos/jadzziaa/moonglow-toolset/releases/latest";

/// The releases, newest first: Include pre-releases asks for these.
pub const LIST_API: &str =
    "https://api.github.com/repos/jadzziaa/moonglow-toolset/releases?per_page=20";

/// The releases' page, where a release's own page isn't known.
pub const RELEASES_PAGE: &str = "https://github.com/jadzziaa/moonglow-toolset/releases/latest";

impl Default for Api {
    fn default() -> Api {
        Api {
            latest: LATEST_API.into(),
            list: LIST_API.into(),
            pages: "https://github.com/jadzziaa/moonglow-toolset/releases/".into(),
            downloads: "https://github.com/jadzziaa/moonglow-toolset/releases/download/".into(),
        }
    }
}

impl Api {
    /// A test's server at `base` (`http://127.0.0.1:port`): `/latest`,
    /// `/releases`, pages under `/releases/` and downloads under
    /// `/download/`.
    pub fn at(base: &str) -> Api {
        Api {
            latest: format!("{base}/latest"),
            list: format!("{base}/releases"),
            pages: format!("{base}/releases/"),
            downloads: format!("{base}/download/"),
        }
    }
}

/// A file a release has, to download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    /// Its size, bytes, as GitHub lists it.
    pub size: Option<u64>,
}

/// A release GitHub lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    /// Its tag (`v3.3.0`), which its signature names.
    pub tag: String,
    /// Its page on GitHub (opened in the browser).
    pub page: String,
    /// What is new in it (the release's Markdown).
    pub notes: String,
    /// Its files under the API's downloads (others are left out).
    pub assets: Vec<Asset>,
}

impl Release {
    /// Its file named `name`.
    pub fn asset(&self, name: &str) -> Option<&Asset> {
        self.assets.iter().find(|a| a.name == name)
    }
}

/// A release in one of GitHub's answers; `None` when it is a draft, a
/// pre-release where `pre` is false, or its tag isn't a version.
fn release(value: &Value, api: &Api, pre: bool) -> Result<Option<Release>, String> {
    let tag =
        value.get("tag_name").and_then(Value::as_str).ok_or("GitHub's answer names no release")?;
    let flag = |name| value.get(name).and_then(Value::as_bool).unwrap_or(false);
    if flag("draft") || (flag("prerelease") && !pre) {
        return Ok(None);
    }
    let Some(version) = Version::parse(tag).filter(|v| pre || !v.is_pre()) else {
        return Ok(None);
    };
    // (Only a page of Moonglow's own releases is opened.)
    let page = value
        .get("html_url")
        .and_then(Value::as_str)
        .filter(|u| u.starts_with(&api.pages))
        .map_or_else(|| RELEASES_PAGE.to_string(), str::to_string);
    let notes = value.get("body").and_then(Value::as_str).unwrap_or_default().to_string();
    let assets = value
        .get("assets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|a| {
            let name = a.get("name")?.as_str()?.to_string();
            let url = a.get("browser_download_url")?.as_str()?;
            // (A link elsewhere is never followed.)
            url.starts_with(&api.downloads).then(|| Asset {
                name,
                url: url.to_string(),
                size: a.get("size").and_then(Value::as_u64),
            })
        })
        .collect();
    Ok(Some(Release { version, tag: tag.to_string(), page, notes, assets }))
}

fn json(text: &str) -> Result<Value, String> {
    serde_json::from_str(text).map_err(|e| format!("GitHub's answer isn't JSON: {e}"))
}

/// The release in GitHub's answer for the latest one; `None` when it is a
/// draft or a pre-release (which never count), or its tag isn't a version.
pub fn parse_latest(text: &str, api: &Api) -> Result<Option<Release>, String> {
    release(&json(text)?, api, false)
}

/// The newest release in GitHub's list of them, pre-releases counted;
/// drafts never are.
pub fn parse_list(text: &str, api: &Api) -> Result<Option<Release>, String> {
    let value = json(text)?;
    let list = value.as_array().ok_or("GitHub's answer lists no releases")?;
    let mut newest: Option<Release> = None;
    for r in list {
        if let Some(r) = release(r, api, true)?
            && newest.as_ref().is_none_or(|n| r.version > n.version)
        {
            newest = Some(r);
        }
    }
    Ok(newest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(tag: &str, draft: bool, pre: bool) -> Value {
        serde_json::json!({
            "tag_name": tag,
            "name": format!("Moonglow Toolset {tag}"),
            "html_url": format!("https://github.com/jadzziaa/moonglow-toolset/releases/tag/{tag}"),
            "draft": draft,
            "prerelease": pre,
            "body": "## What's new\n\n- Updates install from the app.",
            "assets": [
                {
                    "name": "SHA256SUMS",
                    "size": 300,
                    "browser_download_url": format!("https://github.com/jadzziaa/moonglow-toolset/releases/download/{tag}/SHA256SUMS"),
                },
                {
                    "name": "evil.AppImage",
                    "browser_download_url": "https://example.com/evil.AppImage",
                },
            ],
        })
    }

    fn v(text: &str) -> Version {
        Version::parse(text).unwrap()
    }

    #[test]
    fn the_latest_release_is_read_and_drafts_and_pre_releases_never_count() {
        let api = Api::default();
        let latest =
            parse_latest(&answer("v2.10.0", false, false).to_string(), &api).unwrap().unwrap();
        assert_eq!(latest.version, v("2.10.0"));
        assert_eq!(latest.tag, "v2.10.0");
        assert_eq!(
            latest.page,
            "https://github.com/jadzziaa/moonglow-toolset/releases/tag/v2.10.0"
        );
        assert!(latest.notes.starts_with("## What's new"));
        // Its downloads, but not a link elsewhere.
        assert_eq!(latest.assets.len(), 1);
        assert_eq!(latest.asset("SHA256SUMS").unwrap().size, Some(300));
        assert!(latest.asset("evil.AppImage").is_none());
        let parse = |a: Value| parse_latest(&a.to_string(), &api).unwrap();
        assert_eq!(parse(answer("v2.5.0", true, false)), None);
        assert_eq!(parse(answer("v2.5.0", false, true)), None);
        // (A pre-release's tag, though not marked one.)
        assert_eq!(parse(answer("v2.5.0-beta.2", false, false)), None);
        assert_eq!(parse(answer("nightly", false, false)), None);
        // A page elsewhere isn't opened: the releases' page is.
        let elsewhere = r#"{"tag_name":"v3.0.0","html_url":"https://example.com/x"}"#;
        assert_eq!(parse_latest(elsewhere, &api).unwrap().unwrap().page, RELEASES_PAGE);
        assert!(parse_latest("<html>", &api).is_err());
        assert!(parse_latest(r#"{"message":"Not Found"}"#, &api).is_err());
    }

    #[test]
    fn the_list_gives_the_newest_pre_release_or_release_but_no_draft() {
        let api = Api::default();
        let list = |items: Vec<Value>| Value::Array(items).to_string();
        let newest = |items| parse_list(&list(items), &api).unwrap().map(|r| r.version);
        assert_eq!(
            newest(vec![
                answer("v3.4.0", true, false),
                answer("v3.3.0-rc.2", false, true),
                answer("v3.3.0-rc.1", false, true),
                answer("v3.2.0", false, false),
            ]),
            Some(v("3.3.0-rc.2"))
        );
        // A release after its pre-releases.
        assert_eq!(
            newest(vec![answer("v3.3.0-rc.2", false, true), answer("v3.3.0", false, false)]),
            Some(v("3.3.0"))
        );
        assert_eq!(newest(vec![]), None);
        assert!(parse_list(r#"{"message":"Not Found"}"#, &api).is_err());
    }
}
