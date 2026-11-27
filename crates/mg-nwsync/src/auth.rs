//! Signing in to a private NWSync server: a token (`Authorization:
//! Bearer`) or a user and password (`Authorization: Basic`), read from the
//! builder's own `.moonglow/user.toml` alone, never from a checkout.
//!
//! An [`Auth`] is bound to one host. It is sent only to that host's own
//! addresses, never across a redirect to another server (nor to another
//! port or scheme of the same one), and never over plain `http://` but to
//! this computer (`localhost`, `127.0.0.1`, `[::1]`), where nobody else
//! reads it on the way ([`Auth::sent_to`]). Its secret is never in a
//! message, its `Debug` or an address: what is said of it is
//! [`Auth::said`] ("a token").

use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

/// What signs in.
#[derive(Clone, PartialEq, Eq)]
pub enum Credentials {
    /// `Authorization: Bearer <token>`.
    Token(String),
    /// `Authorization: Basic <base64 of user:password>`.
    Basic { user: String, password: String },
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Credentials::Token(_) => f.write_str("Token(<hidden>)"),
            Credentials::Basic { .. } => f.write_str("Basic(<hidden>)"),
        }
    }
}

/// A sign-in to one server's host.
#[derive(Clone, PartialEq, Eq)]
pub struct Auth {
    /// The host it is sent to, in lowercase, without a port
    /// (`nwsync.example.org`).
    pub host: String,
    pub credentials: Credentials,
}

impl fmt::Debug for Auth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Auth")
            .field("host", &self.host)
            .field("credentials", &self.credentials)
            .finish()
    }
}

impl Auth {
    /// The `Authorization` header's value.
    pub fn header(&self) -> String {
        match &self.credentials {
            Credentials::Token(t) => format!("Bearer {t}"),
            Credentials::Basic { user, password } => {
                format!("Basic {}", BASE64.encode(format!("{user}:{password}")))
            }
        }
    }

    /// What it is, as a sentence says it ("a token"), never its value.
    pub fn said(&self) -> &'static str {
        match self.credentials {
            Credentials::Token(_) => "a token",
            Credentials::Basic { .. } => "a user and password",
        }
    }

    /// Whether `url` is on its host (by name, whatever the scheme or
    /// port).
    pub fn for_url(&self, url: &str) -> bool {
        host_of(url).is_some_and(|h| h == self.host)
    }

    /// Whether it is sent with a request of `url`: on its host, and over
    /// HTTPS or to this computer.
    pub fn sent_to(&self, url: &str) -> bool {
        self.for_url(url) && (scheme_of(url).as_deref() == Some("https") || loopback(&self.host))
    }

    /// Why it can't be used for `url`, a server on its host over plain
    /// HTTP to another computer (none where it can, or isn't for `url`).
    pub fn refused_for(&self, url: &str) -> Option<String> {
        (self.for_url(url) && !self.sent_to(url)).then(|| {
            format!(
                "the sign-in in .moonglow/user.toml ({}) is not sent over plain http:// to {}, \
                 where anyone on the way could read it: the source is to be https://",
                self.said(),
                self.host
            )
        })
    }
}

/// An address's scheme, in lowercase (`https`).
pub fn scheme_of(url: &str) -> Option<String> {
    url.split_once("://").map(|(s, _)| s.to_ascii_lowercase())
}

/// An address's `user:password@host:port` part (after `://`, to the path).
fn authority(url: &str) -> Option<&str> {
    let (_, rest) = url.split_once("://")?;
    Some(rest.split(['/', '?', '#']).next().unwrap_or(""))
}

/// Whether an address carries a user or password (`https://u:p@host`).
pub fn has_userinfo(url: &str) -> bool {
    authority(url).is_some_and(|a| a.contains('@'))
}

/// An address's host, in lowercase, without its port (`[::1]` keeps its
/// brackets); none without one, or with a user or password in it.
pub fn host_of(url: &str) -> Option<String> {
    let a = authority(url)?;
    if a.contains('@') {
        return None;
    }
    let host = match a.strip_prefix('[') {
        Some(v6) => format!("[{}]", v6.split(']').next()?),
        None => a.split(':').next().unwrap_or("").to_string(),
    };
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    (!host.is_empty()).then_some(host)
}

/// An address's scheme, host and port (`https://a.example:443` and
/// `https://a.example` are one).
pub fn origin_of(url: &str) -> Option<(String, String, u16)> {
    let scheme = scheme_of(url)?;
    let host = host_of(url)?;
    let a = authority(url)?;
    let port = match a.strip_prefix('[') {
        Some(v6) => v6.split_once(']').map_or("", |(_, r)| r).strip_prefix(':').unwrap_or(""),
        None => a.split_once(':').map_or("", |(_, p)| p),
    };
    let port = match (port, scheme.as_str()) {
        ("", "https") => 443,
        ("", "http") => 80,
        ("", _) => return None,
        (p, _) => p.parse().ok()?,
    };
    Some((scheme, host, port))
}

/// Whether a host is this computer.
pub fn loopback(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    host == "localhost"
        || host == "[::1]"
        || host.parse::<std::net::Ipv4Addr>().is_ok_and(|ip| ip.is_loopback())
}

/// A host as the project file gives it: a name (`nwsync.example.org`), or
/// an address it is taken from; none for one that isn't.
pub fn host_given(text: &str) -> Option<String> {
    let text = text.trim();
    if text.contains("://") {
        return host_of(text);
    }
    if text.contains(['/', '@', ' ', '?', '#']) {
        return None;
    }
    host_of(&format!("https://{text}"))
}

/// Where a redirect's `Location` leads from `from`: an address of its
/// own, one from `//`, from `/`, or beside `from`'s path.
pub fn redirected(from: &str, location: &str) -> Option<String> {
    let location = location.trim();
    if location.contains("://") {
        return Some(location.to_string());
    }
    let scheme = scheme_of(from)?;
    if let Some(rest) = location.strip_prefix("//") {
        return Some(format!("{scheme}://{rest}"));
    }
    let a = authority(from)?;
    if location.starts_with('/') {
        return Some(format!("{scheme}://{a}{location}"));
    }
    let path = &from[from.find("://")? + 3 + a.len()..];
    let path = path.split(['?', '#']).next().unwrap_or("");
    let dir = path.rsplit_once('/').map_or("", |(d, _)| d);
    Some(format!("{scheme}://{a}{dir}/{location}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(host: &str) -> Auth {
        Auth { host: host.into(), credentials: Credentials::Token("s3cret-t0ken".into()) }
    }

    #[test]
    fn the_secret_is_in_the_header_alone() {
        let t = token("a.example");
        assert_eq!(t.header(), "Bearer s3cret-t0ken");
        let b = Auth {
            host: "a.example".into(),
            credentials: Credentials::Basic { user: "ann".into(), password: "pa:ss".into() },
        };
        assert_eq!(b.header(), format!("Basic {}", BASE64.encode("ann:pa:ss")));
        for (a, secret) in [(&t, "s3cret-t0ken"), (&b, "pa:ss")] {
            let debug = format!("{a:?} {:?}", a.credentials);
            assert!(!debug.contains(secret) && debug.contains("a.example"), "{debug}");
            assert!(!a.said().contains(secret));
            let refused = a.refused_for("http://a.example").unwrap();
            assert!(!refused.contains(secret), "{refused}");
        }
        assert!(!format!("{b:?}").contains("ann"));
    }

    /// Sent to its own host over HTTPS, and over plain HTTP to this
    /// computer alone; to no other host.
    #[test]
    fn it_is_sent_to_its_own_host_alone() {
        let t = token("a.example");
        assert!(t.sent_to("https://a.example/latest"));
        assert!(t.sent_to("HTTPS://A.Example.:8443/manifests/x"));
        assert!(!t.sent_to("http://a.example/latest"));
        assert!(t.refused_for("http://a.example").is_some());
        assert!(t.refused_for("http://b.example").is_none());
        assert!(t.refused_for("https://a.example").is_none());
        for other in [
            "https://b.example/latest",
            "https://a.example.evil.example/",
            "https://evil.example/a.example",
            "https://a.example@evil.example/",
            "https://evil.example?a.example",
        ] {
            assert!(!t.sent_to(other), "{other}");
        }
        for (host, url) in [
            ("127.0.0.1", "http://127.0.0.1:8080/x"),
            ("localhost", "http://localhost/"),
            ("[::1]", "http://[::1]:9/"),
        ] {
            assert!(token(host).sent_to(url), "{url}");
        }
        assert!(!token("10.0.0.1").sent_to("http://10.0.0.1/"));
    }

    #[test]
    fn addresses_are_taken_apart() {
        assert!(has_userinfo("https://u:p@a.example/x"));
        assert!(!has_userinfo("https://a.example/x@y"));
        assert_eq!(host_of("https://u:p@a.example"), None);
        assert_eq!(host_of("http://[::1]:80/x").as_deref(), Some("[::1]"));
        assert_eq!(host_given("NWSync.Example.org").as_deref(), Some("nwsync.example.org"));
        assert_eq!(host_given("https://a.example:8443/x").as_deref(), Some("a.example"));
        for bad in ["", "a b", "u@a.example", "a.example/x"] {
            assert_eq!(host_given(bad), None, "{bad}");
        }
        let o = |u| origin_of(u).unwrap();
        assert_eq!(o("https://a.example/x"), o("HTTPS://a.example:443"));
        assert_ne!(o("https://a.example"), o("https://a.example:8443"));
        assert_ne!(o("https://a.example"), o("http://a.example"));
        assert_eq!(o("http://[::1]:9/").2, 9);
        let from = "https://a.example/repo/manifests/x?y";
        assert_eq!(redirected(from, "https://b.example/z").unwrap(), "https://b.example/z");
        assert_eq!(redirected(from, "//b.example/z").unwrap(), "https://b.example/z");
        assert_eq!(redirected(from, "/z").unwrap(), "https://a.example/z");
        assert_eq!(redirected(from, "z").unwrap(), "https://a.example/repo/manifests/z");
    }
}
