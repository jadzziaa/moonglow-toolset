//! Random names from the game's letter tables (`.ltr`, Markov chains of
//! letters; racialtypes.2da `NameGenTableA` names a race's: `humanm`,
//! `humanf`, `humanl` for male, female and last names), as the Creature
//! Wizard's Random buttons give them. The file: `LTR V1.0`, the letter
//! count (26 or 28: a–z, `'`, `-`), then cumulative probabilities (f32)
//! for the first, a middle and the last letter: alone, after each letter,
//! and after each pair. A name takes its first three letters from the
//! start tables, then letters from the pair's middle table until a roll of
//! 1 to 12 is at most its length, and ends with one from the pair's end
//! table (nwn.wiki, "LTR"); between 4 and 13 letters, capitalised.

use crate::GameData;

const LETTERS: &[u8; 28] = b"abcdefghijklmnopqrstuvwxyz'-";

/// One table: start, middle and end probabilities, cumulative.
#[derive(Debug, Clone, PartialEq)]
struct Odds {
    start: Vec<f32>,
    middle: Vec<f32>,
    end: Vec<f32>,
}

/// A letter table.
#[derive(Debug, Clone, PartialEq)]
pub struct Ltr {
    letters: usize,
    single: Odds,
    double: Vec<Odds>,
    triple: Vec<Odds>,
}

/// Why a letter table could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LtrError {
    #[error("not a letter table (no LTR V1.0 header)")]
    Header,
    #[error("{0} letters (26 or 28 expected)")]
    Letters(usize),
    #[error("truncated: {0} bytes, {1} expected")]
    Truncated(usize, usize),
}

impl Ltr {
    /// Reads a letter table.
    pub fn read(data: &[u8]) -> Result<Ltr, LtrError> {
        if data.len() < 9 || &data[..8] != b"LTR V1.0" {
            return Err(LtrError::Header);
        }
        let n = usize::from(data[8]);
        if n != 26 && n != 28 {
            return Err(LtrError::Letters(n));
        }
        let want = 9 + 4 * (3 * n + n * 3 * n + n * n * 3 * n);
        if data.len() < want {
            return Err(LtrError::Truncated(data.len(), want));
        }
        let mut at = 9;
        let mut floats = |count: usize| -> Vec<f32> {
            let v = data[at..at + 4 * count]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|b| f32::from_le_bytes(*b))
                .collect();
            at += 4 * count;
            v
        };
        let mut odds = || Odds { start: floats(n), middle: floats(n), end: floats(n) };
        let single = odds();
        let double = (0..n).map(|_| odds()).collect();
        let triple = (0..n * n).map(|_| odds()).collect();
        Ok(Ltr { letters: n, single, double, triple })
    }

    /// The letter a roll picks from cumulative probabilities.
    fn pick(&self, odds: &[f32], roll: f32) -> Option<usize> {
        odds.iter().position(|&p| p > roll && p > 0.0).filter(|&i| i < self.letters)
    }

    /// A name, or `None` if the table makes none in a hundred tries.
    pub fn generate(&self, rng: &mut fastrand::Rng) -> Option<String> {
        'tries: for _ in 0..100 {
            let mut name: Vec<usize> = Vec::new();
            let a = self.pick(&self.single.start, rng.f32())?;
            name.push(a);
            let Some(b) = self.pick(&self.double[a].start, rng.f32()) else { continue };
            name.push(b);
            let Some(c) = self.pick(&self.triple[a * self.letters + b].start, rng.f32()) else {
                continue;
            };
            name.push(c);
            loop {
                let (x, y) = (name[name.len() - 2], name[name.len() - 1]);
                let t = &self.triple[x * self.letters + y];
                if rng.u32(1..=12) as usize <= name.len()
                    && let Some(e) = self.pick(&t.end, rng.f32())
                {
                    name.push(e);
                    break;
                }
                let Some(m) = self.pick(&t.middle, rng.f32()) else { continue 'tries };
                name.push(m);
                if name.len() > 13 {
                    continue 'tries;
                }
            }
            if name.len() < 4 {
                continue;
            }
            let mut s: String = name.iter().map(|&i| char::from(LETTERS[i])).collect();
            if let Some(first) = s.get_mut(..1) {
                first.make_ascii_uppercase();
            }
            return Some(s);
        }
        None
    }
}

impl GameData {
    /// A random first (`last` false) or last name for a race and gender
    /// (0 male, 1 female), from its letter tables; `None` if it has none.
    pub fn random_name(
        &self,
        race: u32,
        gender: u8,
        last: bool,
        rng: &mut fastrand::Rng,
    ) -> Option<String> {
        let base =
            self.table("racialtypes").ok()?.get(race as usize, "NameGenTableA")?.to_lowercase();
        let suffix = if last {
            "l"
        } else if gender == 1 {
            "f"
        } else {
            "m"
        };
        let data = self.resman.get_named(&format!("{base}{suffix}"), mg_core::ResType::LTR).ok()?;
        Ltr::read(&data).ok()?.generate(rng)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A table that only spells "abab…": a, then b, then a, …; ends after b.
    fn toy() -> Vec<u8> {
        let n = 28;
        let mut out = b"LTR V1.0".to_vec();
        out.push(n as u8);
        let only =
            |i: usize| -> Vec<f32> { (0..n).map(|k| if k >= i { 1.0 } else { 0.0 }).collect() };
        let none = vec![0.0f32; n];
        let mut put = |v: &[f32]| out.extend(v.iter().flat_map(|f| f.to_le_bytes()));
        // Singles: start a.
        put(&only(0));
        put(&none);
        put(&none);
        for a in 0..n {
            put(&if a == 0 { only(1) } else { none.clone() });
            put(&none);
            put(&none);
        }
        for a in 0..n {
            for b in 0..n {
                let (start, middle, end) = match (a, b) {
                    (0, 1) => (only(0), only(0), only(1)),
                    (1, 0) => (none.clone(), only(1), none.clone()),
                    _ => (none.clone(), none.clone(), none.clone()),
                };
                put(&start);
                put(&middle);
                put(&end);
            }
        }
        out
    }

    #[test]
    fn makes_names_from_a_table_and_refuses_bad_ones() {
        let ltr = Ltr::read(&toy()).unwrap();
        let mut rng = fastrand::Rng::with_seed(1);
        for _ in 0..20 {
            let name = ltr.generate(&mut rng).unwrap();
            assert!((4..=13).contains(&name.len()), "{name}");
            assert!(name.starts_with("Aba"), "{name}");
            assert!(name[1..].chars().all(|c| c == 'a' || c == 'b'), "{name}");
        }
        let data = toy();
        assert_eq!(Ltr::read(&data[..100]), Err(LtrError::Truncated(100, data.len())));
        assert_eq!(Ltr::read(b"LTR V2.0\x1c"), Err(LtrError::Header));
    }
}
