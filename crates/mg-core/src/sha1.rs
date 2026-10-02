//! SHA-1 (FIPS 180-4), which NWSync names files by. Written here rather than
//! taken from a crate: it is short, and the NWSync tests compare its results
//! with neverwinter.nim's.

/// A running SHA-1.
#[derive(Debug, Clone)]
pub struct Sha1 {
    state: [u32; 5],
    block: [u8; 64],
    filled: usize,
    length: u64,
}

impl Default for Sha1 {
    fn default() -> Sha1 {
        Sha1 {
            state: [0x6745_2301, 0xEFCD_AB89, 0x98BA_DCFE, 0x1032_5476, 0xC3D2_E1F0],
            block: [0; 64],
            filled: 0,
            length: 0,
        }
    }
}

impl Sha1 {
    pub fn new() -> Sha1 {
        Sha1::default()
    }

    fn compress(state: &mut [u32; 5], block: &[u8; 64]) {
        let mut w = [0u32; 80];
        for (i, word) in block.as_chunks::<4>().0.iter().enumerate() {
            w[i] = u32::from_be_bytes(*word);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = *state;
        for (i, wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | (!b & d), 0x5A82_7999),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6),
            };
            let t =
                a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(*wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = t;
        }
        for (s, v) in state.iter_mut().zip([a, b, c, d, e]) {
            *s = s.wrapping_add(v);
        }
    }

    pub fn update(&mut self, mut data: &[u8]) {
        self.length = self.length.wrapping_add(data.len() as u64);
        while !data.is_empty() {
            let n = (64 - self.filled).min(data.len());
            self.block[self.filled..self.filled + n].copy_from_slice(&data[..n]);
            self.filled += n;
            data = &data[n..];
            if self.filled == 64 {
                Sha1::compress(&mut self.state, &self.block);
                self.filled = 0;
            }
        }
    }

    pub fn finish(mut self) -> [u8; 20] {
        let bits = self.length.wrapping_mul(8);
        self.update(&[0x80]);
        while self.filled != 56 {
            self.update(&[0]);
        }
        self.update(&bits.to_be_bytes());
        let mut out = [0u8; 20];
        for (i, s) in self.state.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&s.to_be_bytes());
        }
        out
    }
}

/// The SHA-1 of some bytes.
pub fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h = Sha1::new();
    h.update(data);
    h.finish()
}

/// A digest as lower-case hexadecimal.
pub fn hex(digest: &[u8]) -> String {
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_vectors() {
        assert_eq!(hex(&sha1(b"")), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(hex(&sha1(b"abc")), "a9993e364706816aba3e25717850c26c9cd0d89d");
        let long = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";
        assert_eq!(hex(&sha1(long)), "84983e441c3bd26ebaae4aa1f95129e5e54670f1");
        assert_eq!(hex(&sha1(&vec![b'a'; 1_000_000])), "34aa973cd4c4daa4f61eeb2bdbad27316534016f");
        // In pieces, as a file is read.
        let mut h = Sha1::new();
        for chunk in long.chunks(7) {
            h.update(chunk);
        }
        assert_eq!(hex(&h.finish()), "84983e441c3bd26ebaae4aa1f95129e5e54670f1");
        // nwn_nwsync_write's name for this 2DA's file.
        assert_eq!(hex(&sha1(b"h2 2da")), "9db8c0213cd6324c99f97b9d8dd8f2a94f1fd9f4");
    }
}
