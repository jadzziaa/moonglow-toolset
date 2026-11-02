//! A server of releases on this computer (127.0.0.1), for tests: the
//! updater is pointed at it ([`crate::Api::at`]) and never at GitHub.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

/// What a path answers.
#[derive(Debug, Clone)]
pub enum Reply {
    /// `200` with the body.
    Body(Vec<u8>),
    /// `200`, saying the whole length, but closing after `sent` bytes (a
    /// download interrupted).
    Cut { body: Vec<u8>, sent: usize },
    /// `200`, saying the whole length, sending `sent` bytes and then
    /// nothing more for `wait` (a slow download, half way).
    Stall { body: Vec<u8>, sent: usize, wait: std::time::Duration },
    /// A redirect (`302`) to another path of this server, as GitHub's
    /// downloads redirect to its file host.
    Redirect(String),
    /// Another status, no body.
    Status(u16),
}

#[derive(Debug, Default)]
struct State {
    replies: HashMap<String, Reply>,
    asked: Vec<String>,
    /// Each request's header lines, in order.
    headers: Vec<Vec<String>>,
}

/// The server, answering for as long as the test runs.
#[derive(Debug)]
pub struct Server {
    pub base: String,
    state: Arc<Mutex<State>>,
}

impl Server {
    pub fn start() -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port on 127.0.0.1");
        let base = format!("http://{}", listener.local_addr().unwrap());
        let state = Arc::new(Mutex::new(State::default()));
        let shared = state.clone();
        std::thread::Builder::new()
            .name("release-server".into())
            .spawn(move || {
                for stream in listener.incoming().flatten() {
                    let shared = shared.clone();
                    std::thread::spawn(move || {
                        let _ = answer(stream, &shared);
                    });
                }
            })
            .unwrap();
        Server { base, state }
    }

    /// `path` (`/latest`) answers `reply`.
    pub fn put(&self, path: &str, reply: Reply) {
        self.state.lock().unwrap().replies.insert(path.to_string(), reply);
    }

    /// The paths asked for so far, in order.
    pub fn asked(&self) -> Vec<String> {
        self.state.lock().unwrap().asked.clone()
    }

    /// The header lines (`name: value`, names in lower case) of the last
    /// request for `path`.
    pub fn headers(&self, path: &str) -> Vec<String> {
        let state = self.state.lock().unwrap();
        let last = state.asked.iter().rposition(|p| p == path);
        last.map(|i| state.headers[i].clone()).unwrap_or_default()
    }

    /// `path`'s full address.
    pub fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }
}

fn answer(stream: TcpStream, state: &Mutex<State>) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let path = line.split_whitespace().nth(1).unwrap_or("/").to_string();
    // (The headers, to the blank line.)
    let mut headers = Vec::new();
    loop {
        let mut header = String::new();
        if reader.read_line(&mut header)? == 0 || header == "\r\n" || header == "\n" {
            break;
        }
        let header = header.trim_end();
        headers.push(match header.split_once(':') {
            Some((name, value)) => format!("{}: {}", name.to_lowercase(), value.trim()),
            None => header.to_string(),
        });
    }
    let reply = {
        let mut state = state.lock().unwrap();
        state.asked.push(path.clone());
        state.headers.push(headers);
        state.replies.get(&path).cloned().unwrap_or(Reply::Status(404))
    };
    let mut stream = stream;
    match reply {
        Reply::Body(body) => {
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )?;
            stream.write_all(&body)?;
        }
        Reply::Cut { body, sent } => {
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )?;
            stream.write_all(&body[..sent.min(body.len())])?;
            stream.flush()?;
        }
        Reply::Stall { body, sent, wait } => {
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            )?;
            stream.write_all(&body[..sent.min(body.len())])?;
            stream.flush()?;
            std::thread::sleep(wait);
            stream.write_all(&body[sent.min(body.len())..])?;
        }
        Reply::Redirect(to) => {
            write!(
                stream,
                "HTTP/1.1 302 Found\r\nLocation: {to}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )?;
        }
        Reply::Status(code) => {
            write!(
                stream,
                "HTTP/1.1 {code} Nope\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )?;
        }
    }
    stream.flush()?;
    // (Whatever else the client sent is read, so that closing doesn't
    // reset the connection before it has read the answer.)
    let _ = stream.shutdown(std::net::Shutdown::Write);
    let _ = reader.get_mut().set_read_timeout(Some(std::time::Duration::from_millis(200)));
    let _ = reader.read_to_end(&mut Vec::new());
    Ok(())
}

/// A release on the server, as GitHub's API describes it: its JSON, and
/// its files under `/download/<tag>/`.
#[derive(Debug, Clone)]
pub struct Published {
    pub tag: String,
    pub prerelease: bool,
    pub notes: String,
    pub files: Vec<(String, Vec<u8>)>,
}

impl Published {
    /// The API's description of it, its files' links on `server`.
    pub fn json(&self, server: &Server) -> serde_json::Value {
        let assets: Vec<_> = self
            .files
            .iter()
            .map(|(name, body)| {
                serde_json::json!({
                    "name": name,
                    "size": body.len(),
                    "browser_download_url": server.url(&format!("/download/{}/{name}", self.tag)),
                })
            })
            .collect();
        serde_json::json!({
            "tag_name": self.tag,
            "name": format!("Moonglow Toolset {}", self.tag),
            "html_url": server.url(&format!("/releases/tag/{}", self.tag)),
            "draft": false,
            "prerelease": self.prerelease,
            "body": self.notes,
            "assets": assets,
        })
    }

    /// Its files put on `server` (each through a redirect, as GitHub's).
    pub fn put_files(&self, server: &Server) {
        for (name, body) in &self.files {
            let path = format!("/download/{}/{name}", self.tag);
            let stored = format!("/files/{}/{name}", self.tag);
            server.put(&path, Reply::Redirect(server.url(&stored)));
            server.put(&stored, Reply::Body(body.clone()));
        }
    }
}

/// What signs a release's `SHA256SUMS` in a test (`mg_testkit::signing`).
pub trait Signer {
    /// `data` signed with `comment` as its trusted comment, as `minisign
    /// -S -t` writes it.
    fn sign(&self, data: &[u8], comment: &str) -> String;
}

/// A release's files with their `SHA256SUMS`, signed by `key` for `tag`
/// (as `packaging/sign-release.sh` signs it).
pub fn signed(key: &dyn Signer, tag: &str, packages: &[(&str, &[u8])]) -> Vec<(String, Vec<u8>)> {
    let mut sums = String::new();
    let mut files = Vec::new();
    for (name, body) in packages {
        let mut h = crate::verify::Hasher::default();
        h.update(body);
        sums += &format!("{}  {name}\n", crate::verify::to_hex(&h.finish()));
        files.push((name.to_string(), body.to_vec()));
    }
    let signature = key.sign(sums.as_bytes(), &crate::verify::trusted_comment(tag));
    files.push((crate::verify::SUMS.into(), sums.into_bytes()));
    files.push((crate::verify::SIGNATURE.into(), signature.into_bytes()));
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_server_answers_what_it_was_given() {
        let server = Server::start();
        server.put("/a", Reply::Body(b"hello".to_vec()));
        let mut s = TcpStream::connect(server.base.trim_start_matches("http://")).unwrap();
        s.write_all(b"GET /a HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();
        let mut got = String::new();
        s.read_to_string(&mut got).unwrap();
        assert!(got.starts_with("HTTP/1.1 200 OK") && got.ends_with("hello"), "{got}");
        assert_eq!(server.asked(), ["/a"]);
        assert!(server.headers("/a").contains(&"host: x".to_string()));
    }

    /// The client the app makes its requests with (plain HTTP allowed for
    /// the test alone), against the server: the check's request as it was,
    /// a download through a redirect, one broken off, a missing file, and a
    /// whole update fetched and installed on a scratch AppImage.
    #[cfg(feature = "http")]
    #[test]
    fn the_app_s_client_asks_downloads_and_says_what_failed() {
        use crate::Http;
        use crate::http::Client;
        let server = Server::start();
        let client = Client::plain_http_for_tests();
        server.put("/latest", Reply::Body(br#"{"tag_name":"v99.0.0"}"#.to_vec()));
        assert_eq!(client.ask(&server.url("/latest")).unwrap(), r#"{"tag_name":"v99.0.0"}"#);
        let sent = server.headers("/latest");
        for h in [
            format!("user-agent: Moonglow/{}", mg_core::release::version()),
            "accept: application/vnd.github+json".into(),
            "x-github-api-version: 2022-11-28".into(),
        ] {
            assert!(sent.contains(&h), "{h:?} in {sent:?}");
        }
        // (Nothing else of note: no cookie, no token.)
        assert!(!sent.iter().any(|h| h.starts_with("cookie") || h.starts_with("authorization")));
        assert_eq!(client.ask(&server.url("/nothing")).unwrap_err(), "GitHub lists no release");

        let body: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
        server.put("/download/f", Reply::Redirect(server.url("/files/f")));
        server.put("/files/f", Reply::Body(body.clone()));
        let mut got = Vec::new();
        let mut last = (0, None);
        client
            .download(&server.url("/download/f"), &mut got, 1 << 20, &mut |n, of| last = (n, of))
            .unwrap();
        assert_eq!(got, body);
        assert_eq!(last, (200_000, Some(200_000)));
        // Larger than the release says.
        let e = client.download(&server.url("/files/f"), &mut Vec::new(), 1000, &mut |_, _| {});
        assert_eq!(e.unwrap_err(), "the file is larger than it should be");
        // Broken off half way.
        server.put("/files/cut", Reply::Cut { body: body.clone(), sent: 100_000 });
        let e =
            client.download(&server.url("/files/cut"), &mut Vec::new(), 1 << 20, &mut |_, _| {});
        assert!(e.unwrap_err().contains("broke off"));
        let e =
            client.download(&server.url("/files/none"), &mut Vec::new(), 1 << 20, &mut |_, _| {});
        assert_eq!(e.unwrap_err(), "GitHub has no such release or file");

        // A release served whole: fetched, checked and installed.
        let key = crate::verify::testing::TestKey::new();
        let published = Published {
            tag: "v99.0.0".into(),
            prerelease: false,
            notes: "New.".into(),
            files: signed(&key, "v99.0.0", &[("Moonglow-99.0.0-x86_64.AppImage", b"new image")]),
        };
        published.put_files(&server);
        let api = crate::Api::at(&server.base);
        let release = crate::release::parse_latest(&published.json(&server).to_string(), &api)
            .unwrap()
            .unwrap();
        assert_eq!(release.page, server.url("/releases/tag/v99.0.0"));
        let dir = crate::tests::scratch("serve-whole");
        let image = dir.join("Moonglow.AppImage");
        std::fs::write(&image, b"old image").unwrap();
        let kind = crate::Kind::AppImage { image: image.clone(), arch: "x86_64".into() };
        let staged = crate::stage::fetch(
            &client,
            &key.public(),
            &release,
            &kind,
            &dir.join("cache"),
            &mut |_, _| {},
        )
        .unwrap();
        crate::install::install(&staged, true, &dir.join("cache")).unwrap();
        assert_eq!(std::fs::read(&image).unwrap(), b"new image");
    }
}
