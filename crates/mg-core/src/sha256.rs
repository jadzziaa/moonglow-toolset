//! SHA-256 (FIPS 180-4): what a project's plugins are known by when the
//! user allows them (a plugin changed since is asked about again), an
//! update's packages are checked with, and `mg ls --sha256` and `mg cat
//! --to` print. The `sha2` crate's, behind this one type: it uses the
//! CPU's SHA instructions where it finds them at run time (x86-64's
//! SHA-NI, ARMv8's SHA2) and its software rounds elsewhere. The code
//! written here before, software alone, hashed a 1.5 GB hak in 3.6 s
//! against 0.6 s.

use sha2::Digest;

/// A running SHA-256.
#[derive(Debug, Clone, Default)]
pub struct Sha256(sha2::Sha256);

impl Sha256 {
    pub fn new() -> Sha256 {
        Sha256::default()
    }

    pub fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }

    pub fn finish(self) -> [u8; 32] {
        self.0.finalize().into()
    }
}

/// The SHA-256 of some bytes.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    sha2::Sha256::digest(data).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sha1::hex;

    #[test]
    fn standard_vectors() {
        assert_eq!(
            hex(&sha256(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            hex(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let long = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
        assert_eq!(
            hex(&sha256(long)),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        assert_eq!(
            hex(&sha256(&vec![b'a'; 1_000_000])),
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        );
        // In pieces, as files are read.
        let mut h = Sha256::new();
        for chunk in long.chunks(7) {
            h.update(chunk);
        }
        assert_eq!(
            hex(&h.finish()),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        // Pieces of every length from 1 to 130 across a buffer, block
        // edges falling everywhere, as one hash of it whole.
        let data: Vec<u8> =
            (0..20_000u32).map(|i| (i.wrapping_mul(2_654_435_761) >> 13) as u8).collect();
        let mut h = Sha256::new();
        let (mut at, mut n) = (0, 1);
        while at < data.len() {
            let end = (at + n).min(data.len());
            h.update(&data[at..end]);
            (at, n) = (end, n % 130 + 1);
        }
        assert_eq!(h.finish(), sha256(&data));
    }
}
