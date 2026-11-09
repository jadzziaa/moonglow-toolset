//! What a release's packages are checked against: its `SHA256SUMS`,
//! signed with minisign by the owner's key, whose public half is built
//! into the app.

use std::io::Read;
use std::path::Path;

use mg_core::sha256::Sha256;

/// The checksums' file a release publishes, and its signature.
pub const SUMS: &str = "SHA256SUMS";
pub const SIGNATURE: &str = "SHA256SUMS.minisig";

/// The trusted comment a release's signature carries: its tag, so that a
/// list signed for one release can't pass for another's.
pub fn trusted_comment(tag: &str) -> String {
    format!("Moonglow Toolset {tag}")
}

/// A minisign public key: the file `minisign -G` writes (an untrusted
/// comment, then the key), or the key's line alone.
#[derive(Clone)]
pub struct PublicKey(minisign_verify::PublicKey);

impl std::fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PublicKey")
    }
}

impl PublicKey {
    pub fn parse(text: &str) -> Result<PublicKey, String> {
        let text = text.trim();
        let key = if text.lines().count() > 1 {
            minisign_verify::PublicKey::decode(text)
        } else {
            minisign_verify::PublicKey::from_base64(text)
        };
        key.map(PublicKey).map_err(|e| format!("the update key isn't a minisign key: {e}"))
    }
}

/// A checksum, as hexadecimal.
pub type Hash = [u8; 32];

/// The lines of a `SHA256SUMS` (`sha256sum`'s: the hash, two spaces or a
/// space and `*`, the file's name); lines that are none are passed over.
pub fn parse_sums(text: &str) -> Vec<(String, Hash)> {
    text.lines()
        .filter_map(|line| {
            let (hex, name) = line.split_once(' ')?;
            let name = name.strip_prefix([' ', '*'])?;
            Some((name.trim_end_matches('\r').to_string(), from_hex(hex)?))
        })
        .collect()
}

fn from_hex(hex: &str) -> Option<Hash> {
    let bytes = hex.as_bytes();
    if bytes.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, pair) in bytes.chunks(2).enumerate() {
        let s = std::str::from_utf8(pair).ok()?;
        out[i] = u8::from_str_radix(s, 16).ok()?;
    }
    Some(out)
}

/// `hash` as `sha256sum` writes it.
pub fn to_hex(hash: &Hash) -> String {
    hash.iter().map(|b| format!("{b:02x}")).collect()
}

/// The hash `SHA256SUMS` gives `file`, once its signature is checked
/// against `key` and found to be the release `tag`'s; or why the package
/// isn't to be installed.
pub fn checked_hash(
    key: &PublicKey,
    sums: &[u8],
    signature: &str,
    tag: &str,
    file: &str,
) -> Result<Hash, String> {
    let signature = minisign_verify::Signature::decode(signature)
        .map_err(|_| "the release's signature can't be read".to_string())?;
    // (Prehashed only: minisign's legacy mode is refused.)
    key.0.verify(sums, &signature, false).map_err(|e| match e {
        minisign_verify::Error::UnexpectedKeyId => {
            "the release is signed with another key than this copy of Moonglow knows".to_string()
        }
        minisign_verify::Error::UnexpectedAlgorithm => {
            "the release is signed in minisign's legacy mode, which isn't accepted".to_string()
        }
        _ => "the release's signature doesn't verify".to_string(),
    })?;
    if signature.trusted_comment() != trusted_comment(tag) {
        return Err(format!(
            "the release's signature is for {:?}, not {}",
            signature.trusted_comment(),
            trusted_comment(tag)
        ));
    }
    let text = std::str::from_utf8(sums).map_err(|_| "the release's checksums aren't text")?;
    parse_sums(text)
        .into_iter()
        .find(|(name, _)| name == file)
        .map(|(_, hash)| hash)
        .ok_or_else(|| format!("the release's checksums don't name {file}"))
}

/// A hash taken as a file is written.
#[derive(Debug, Default)]
pub struct Hasher(Sha256);

impl Hasher {
    pub fn update(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }

    pub fn finish(self) -> Hash {
        self.0.finish()
    }
}

/// `path`'s SHA-256.
pub fn hash_file(path: &Path) -> Result<Hash, String> {
    let mut file =
        std::fs::File::open(path).map_err(|e| format!("can't read {}: {e}", path.display()))?;
    let mut hasher = Hasher::default();
    let mut buffer = vec![0u8; 1 << 16];
    loop {
        let n =
            file.read(&mut buffer).map_err(|e| format!("can't read {}: {e}", path.display()))?;
        if n == 0 {
            return Ok(hasher.finish());
        }
        hasher.update(&buffer[..n]);
    }
}

/// `path` is still what was checked (as it is installed, a while after).
pub fn still(path: &Path, hash: &Hash) -> Result<(), String> {
    if hash_file(path)? == *hash {
        Ok(())
    } else {
        Err(format!("{} changed after it was checked", path.display()))
    }
}

/// One hash of everything under `dir`: each file's path and contents,
/// each link's path and where it points, each folder's path, in the
/// paths' order. What an unpacked zip or a copied-out app is checked
/// again by ([`still_tree`]), as a single file is by its own hash.
pub fn hash_tree(dir: &Path) -> Result<Hash, String> {
    let bad = |e: std::io::Error| format!("can't read {}: {e}", dir.display());
    let mut entries: Vec<(std::path::PathBuf, u8, Vec<u8>)> = Vec::new();
    let mut stack = vec![std::path::PathBuf::new()];
    while let Some(rel) = stack.pop() {
        for entry in std::fs::read_dir(dir.join(&rel)).map_err(bad)? {
            let entry = entry.map_err(bad)?;
            let path = rel.join(entry.file_name());
            let kind = entry.file_type().map_err(bad)?;
            if kind.is_symlink() {
                let to = std::fs::read_link(dir.join(&path)).map_err(bad)?;
                entries.push((path, b'l', to.as_os_str().as_encoded_bytes().to_vec()));
            } else if kind.is_dir() {
                entries.push((path.clone(), b'd', Vec::new()));
                stack.push(path);
            } else {
                let hash = hash_file(&dir.join(&path))?;
                entries.push((path, b'f', hash.to_vec()));
            }
        }
    }
    entries.sort();
    let mut hasher = Hasher::default();
    for (path, kind, what) in entries {
        let path = path.as_os_str().as_encoded_bytes();
        hasher.update(&(path.len() as u64).to_le_bytes());
        hasher.update(path);
        hasher.update(&[kind]);
        hasher.update(&(what.len() as u64).to_le_bytes());
        hasher.update(&what);
    }
    Ok(hasher.finish())
}

/// `dir` still holds what was checked, no file of it changed, added or
/// gone ([`hash_tree`]).
pub fn still_tree(dir: &Path, hash: &Hash) -> Result<(), String> {
    if hash_tree(dir)? == *hash {
        Ok(())
    } else {
        Err(format!("{} changed after it was checked", dir.display()))
    }
}

/// Signing, for these tests: a key made by the test, never the owner's
/// (other crates' tests: `mg_testkit::signing`).
#[cfg(test)]
pub(crate) mod testing {
    /// A key pair made for a test, and its public half as the app takes it.
    pub(crate) struct TestKey {
        pub(crate) pair: minisign::KeyPair,
    }

    impl std::fmt::Debug for TestKey {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("TestKey")
        }
    }

    impl TestKey {
        pub(crate) fn new() -> TestKey {
            TestKey { pair: minisign::KeyPair::generate_unencrypted_keypair().unwrap() }
        }

        pub(crate) fn public(&self) -> super::PublicKey {
            let text = self.pair.pk.to_box().unwrap().into_string();
            super::PublicKey::parse(&text).unwrap()
        }

        /// `data` signed with `comment` as its trusted comment, as
        /// `minisign -S -t` writes it.
        pub(crate) fn sign(&self, data: &[u8], comment: &str) -> String {
            minisign::sign(Some(&self.pair.pk), &self.pair.sk, data, Some(comment), None)
                .unwrap()
                .into_string()
        }
    }

    impl crate::serve::Signer for TestKey {
        fn sign(&self, data: &[u8], comment: &str) -> String {
            TestKey::sign(self, data, comment)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::TestKey;
    use super::*;

    fn sums(files: &[(&str, &[u8])]) -> String {
        files
            .iter()
            .map(|(name, data)| {
                let mut h = Hasher::default();
                h.update(data);
                format!("{}  {name}\n", to_hex(&h.finish()))
            })
            .collect()
    }

    #[test]
    fn sums_lines_are_read_as_sha256sum_writes_them() {
        let hash = "a".repeat(64);
        let text = format!(
            "{hash}  Moonglow-3.3.0-x86_64.AppImage\n{hash} *Moonglow-3.3.0-macos.dmg\r\n\
             not a line\n{}  short\n{hash} one-space\n",
            "b".repeat(10)
        );
        let lines = parse_sums(&text);
        let names: Vec<_> = lines.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["Moonglow-3.3.0-x86_64.AppImage", "Moonglow-3.3.0-macos.dmg"]);
        assert_eq!(lines[0].1, [0xaa; 32]);
        assert_eq!(to_hex(&lines[0].1), hash);
        assert!(from_hex(&"g".repeat(64)).is_none());
    }

    #[test]
    fn a_signed_list_gives_the_package_s_hash_and_anything_else_is_refused() {
        let key = TestKey::new();
        let package: &[u8] = b"the new AppImage";
        let list = sums(&[("Moonglow-3.3.0-x86_64.AppImage", package), ("other", b"x")]);
        let signature = key.sign(list.as_bytes(), "Moonglow Toolset v3.3.0");
        let file = "Moonglow-3.3.0-x86_64.AppImage";
        let hash = checked_hash(&key.public(), list.as_bytes(), &signature, "v3.3.0", file)
            .expect("verifies");
        let mut h = Hasher::default();
        h.update(package);
        assert_eq!(hash, h.finish());

        // The list changed after it was signed.
        let tampered = list.replace(&to_hex(&hash), &"0".repeat(64));
        let e = checked_hash(&key.public(), tampered.as_bytes(), &signature, "v3.3.0", file);
        assert_eq!(e.unwrap_err(), "the release's signature doesn't verify");
        // Signed by another key.
        let other = TestKey::new();
        let e = checked_hash(&other.public(), list.as_bytes(), &signature, "v3.3.0", file);
        assert!(e.unwrap_err().contains("another key"));
        // Another release's list, replayed.
        let e = checked_hash(&key.public(), list.as_bytes(), &signature, "v3.4.0", file);
        assert!(e.unwrap_err().contains("not Moonglow Toolset v3.4.0"), "the tag is signed");
        // A package the list doesn't name.
        let e = checked_hash(&key.public(), list.as_bytes(), &signature, "v3.3.0", "x.dmg");
        assert_eq!(e.unwrap_err(), "the release's checksums don't name x.dmg");
        // Not a signature.
        let e = checked_hash(&key.public(), list.as_bytes(), "hello", "v3.3.0", file);
        assert_eq!(e.unwrap_err(), "the release's signature can't be read");
        // The signature's trusted comment changed (the global signature).
        let forged = signature.replace("Moonglow Toolset v3.3.0", "Moonglow Toolset v3.9.0");
        let e = checked_hash(&key.public(), list.as_bytes(), &forged, "v3.9.0", file);
        assert_eq!(e.unwrap_err(), "the release's signature doesn't verify");
    }

    /// A legacy (not prehashed) signature, as minisign before 0.8 wrote
    /// it with `-l`: refused.
    #[test]
    fn a_legacy_signature_is_refused() {
        // minisign-verify's own example of a legacy signature ("test").
        let key =
            PublicKey::parse("RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3").unwrap();
        let signature = "untrusted comment: signature from minisign secret key
RWQf6LRCGA9i59SLOFxz6NxvASXDJeRtuZykwQepbDEGt87ig1BNpWaVWuNrm73YiIiJbq71Wi+dP9eKL8OC351vwIasSSbXxwA=
trusted comment: timestamp:1555779966\tfile:test
QtKMXWyYcwdpZAlPF7tE2ENJkRd1ujvKjlj1m9RtHTBnZPa5WKU5uWRs5GoP5M/VqE81QFuMKI5k/SfNQUaOAA==";
        let e = checked_hash(&key, b"test", signature, "v3.3.0", "x").unwrap_err();
        assert!(e.contains("legacy"), "{e}");
    }

    #[test]
    fn a_key_is_read_from_the_file_minisign_writes_or_its_line() {
        let key = TestKey::new();
        let file = key.pair.pk.to_box().unwrap().into_string();
        assert!(file.starts_with("untrusted comment:"), "{file}");
        assert!(PublicKey::parse(&file).is_ok());
        assert!(PublicKey::parse(file.lines().nth(1).unwrap()).is_ok());
        assert!(PublicKey::parse("not a key").is_err());
    }

    #[test]
    fn a_file_is_still_what_was_checked() {
        let dir = crate::tests::scratch("still");
        let path = dir.join("f");
        std::fs::write(&path, b"one").unwrap();
        let hash = hash_file(&path).unwrap();
        assert!(still(&path, &hash).is_ok());
        std::fs::write(&path, b"two").unwrap();
        assert!(still(&path, &hash).unwrap_err().contains("changed after it was checked"));
    }
}
