//! A minisign key made by a test, to sign a release as
//! `packaging/sign-release.sh` signs one (`mg_update::serve::signed`):
//! never the owner's key. The signing crate is the tests' alone, so the
//! programs are built without it.

/// A key pair made for a test.
pub struct TestKey {
    pair: minisign::KeyPair,
}

impl std::fmt::Debug for TestKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TestKey")
    }
}

impl TestKey {
    pub fn new() -> TestKey {
        TestKey { pair: minisign::KeyPair::generate_unencrypted_keypair().expect("a key") }
    }

    /// Its public half, as the app takes it.
    pub fn public(&self) -> mg_update::PublicKey {
        let text = self.pair.pk.to_box().expect("the public key").into_string();
        mg_update::PublicKey::parse(&text).expect("a minisign key")
    }
}

impl Default for TestKey {
    fn default() -> TestKey {
        TestKey::new()
    }
}

impl mg_update::serve::Signer for TestKey {
    /// `data` signed with `comment` as its trusted comment, as `minisign
    /// -S -t` writes it.
    fn sign(&self, data: &[u8], comment: &str) -> String {
        minisign::sign(Some(&self.pair.pk), &self.pair.sk, data, Some(comment), None)
            .expect("signed")
            .into_string()
    }
}
