//! Compiled programs kept on disk (`docs/shaders-proposal.md`, M3): the
//! SPIR-V glslang and Moonglow's passes made of an assembled program, so
//! that opening an area again compiles none of the programs it drew
//! before. Only naga's reading of the words is done again (a millisecond
//! or two where glslang takes 5 to 30).
//!
//! An entry is named by a hash of what decides its words: the assembled
//! text (the preamble is its first lines), the stage, Moonglow's release
//! and the passes' [`FORMAT`]. It is the user's machine's own: the game's
//! shaders are read from the user's install, and what is made of them stays
//! in the user's cache folder. An entry that is cut short, damaged, of
//! another format or of another program is passed over and written anew;
//! reading one never panics.

use std::path::{Path, PathBuf};

use mg_core::bin::{Reader, WriteLe};

use crate::assemble::Assembled;
use crate::spirv::Io;
use crate::{Program, ShaderError};

/// The passes' format: changed when what they make of the same words does
/// (an entry of another format is never read).
const FORMAT: u32 = 3;

const MAGIC: &[u8; 8] = b"MGSHDSPV";

/// A folder of compiled programs.
#[derive(Debug, Clone)]
pub struct ProgramCache {
    dir: PathBuf,
}

/// Where a program came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cached {
    /// Read from the cache.
    Hit,
    /// Compiled, and written to the cache.
    Miss,
}

/// FNV-1a, 128 bits.
fn fnv128(parts: &[&[u8]]) -> u128 {
    const PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013B;
    let mut h: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
    for part in parts {
        // (Each part's length first: "ab" + "c" is not "a" + "bc".)
        for b in (part.len() as u64).to_le_bytes().iter().chain(part.iter()) {
            h ^= u128::from(*b);
            h = h.wrapping_mul(PRIME);
        }
    }
    h
}

impl ProgramCache {
    /// The cache in `dir` (made when the first entry is written).
    pub fn new(dir: impl Into<PathBuf>) -> ProgramCache {
        ProgramCache { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// What names an assembled program's entry.
    fn key(a: &Assembled) -> u128 {
        let stage = [a.stage as u8];
        fnv128(&[
            mg_core::release::version().as_bytes(),
            &FORMAT.to_le_bytes(),
            &stage,
            a.text.as_bytes(),
        ])
    }

    fn path(&self, key: u128) -> PathBuf {
        self.dir.join(format!("{key:032x}.spv"))
    }

    /// Compiles an assembled program, or reads what was compiled of it
    /// before; and which it was. A program that doesn't compile is an error
    /// as [`crate::compile_assembled`] has it (and nothing is kept).
    pub fn compile(&self, a: &Assembled) -> Result<(Program, Cached), ShaderError> {
        let key = Self::key(a);
        let path = self.path(key);
        if let Ok(bytes) = std::fs::read(&path)
            && let Some((words, io)) = read_entry(&bytes, key, a)
            && let Ok(p) = crate::from_words(a, &words, &io)
        {
            return Ok((p, Cached::Hit));
        }
        let (words, io) = crate::to_words(a)?;
        let program = crate::from_words(a, &words, &io)?;
        // (A cache that can't be written is no cache: the program stands.)
        let _ = self.write(&path, &entry(key, a, &words, &io));
        Ok((program, Cached::Miss))
    }

    /// [`crate::check_assembled`] with the cache: a program kept before
    /// (by an area's view or an earlier Verify) is taken without compiling
    /// it ([`Cached::Hit`]); one glslang takes now is kept, where Moonglow's
    /// passes take it too. Only glslang's refusals are errors.
    pub fn check(&self, a: &Assembled) -> Result<Cached, ShaderError> {
        let key = Self::key(a);
        let path = self.path(key);
        if let Ok(bytes) = std::fs::read(&path)
            && read_entry(&bytes, key, a).is_some()
        {
            return Ok(Cached::Hit);
        }
        let words = crate::glsl::compile(a)?;
        // (A program the passes don't take is the game's to compile all the
        // same: it is checked again next time, as it isn't kept.)
        if let Ok((words, io)) = crate::passes(&words) {
            let _ = self.write(&path, &entry(key, a, &words, &io));
        }
        Ok(Cached::Miss)
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        // (Whole or not at all: written beside it, then put in its place.)
        let tmp = path.with_extension(format!("tmp{}", std::process::id()));
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(&tmp, path).inspect_err(|_| {
            let _ = std::fs::remove_file(&tmp);
        })
    }
}

/// An entry's bytes: the magic, the key, the stage and the text's length,
/// the words, the inputs and outputs by name, and a hash of all before it.
fn entry(key: u128, a: &Assembled, words: &[u32], io: &[Io]) -> Vec<u8> {
    let mut out = Vec::with_capacity(32 + words.len() * 4);
    out.put_bytes(MAGIC);
    out.put_bytes(&key.to_le_bytes());
    out.put_u8(a.stage as u8);
    out.put_u64(a.text.len() as u64);
    out.put_u32_usize(words.len());
    for w in words {
        out.put_u32(*w);
    }
    out.put_u32_usize(io.len());
    for v in io {
        out.put_u8(u8::from(v.output));
        out.put_u32(v.location);
        let name = v.name.as_bytes();
        let name = &name[..name.len().min(u16::MAX as usize)];
        out.put_u16(name.len() as u16);
        out.put_bytes(name);
    }
    let sum = fnv128(&[&out]) as u64;
    out.put_u64(sum);
    out
}

/// The words and inputs and outputs of an entry, if it is whole and the
/// entry of `a` (`key`).
fn read_entry(bytes: &[u8], key: u128, a: &Assembled) -> Option<(Vec<u32>, Vec<Io>)> {
    let body = bytes.len().checked_sub(8)?;
    let sum = u64::from_le_bytes(bytes.get(body..)?.try_into().ok()?);
    if fnv128(&[&bytes[..body]]) as u64 != sum {
        return None;
    }
    let mut r = Reader::new(&bytes[..body]);
    if r.array::<8>().ok()? != *MAGIC
        || u128::from_le_bytes(r.array::<16>().ok()?) != key
        || r.u8().ok()? != a.stage as u8
        || r.u64().ok()? != a.text.len() as u64
    {
        return None;
    }
    let n = r.u32_usize().ok()?;
    // (A count the bytes can't hold is no entry, not an allocation.)
    if n.checked_mul(4)? > r.remaining() {
        return None;
    }
    let words = (0..n).map(|_| r.u32()).collect::<Result<Vec<u32>, _>>().ok()?;
    let n = r.u32_usize().ok()?;
    if n.checked_mul(7)? > r.remaining() {
        return None;
    }
    let mut io = Vec::with_capacity(n);
    for _ in 0..n {
        let output = r.u8().ok()? != 0;
        let location = r.u32().ok()?;
        let len = usize::from(r.u16().ok()?);
        let name = String::from_utf8(r.bytes(len).ok()?.to_vec()).ok()?;
        io.push(Io { output, name, location });
    }
    (r.remaining() == 0).then_some((words, io))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::{Preamble, Stage, assemble};

    fn sources() -> BTreeMap<String, String> {
        let fs = r#"
#define SHADER_TYPE 2
uniform sampler2D texUnit0;
uniform vec4 materialFrontDiffuse;
varying vec2 vTexCoord;
void main ()
{
    gl_FragColor = texture2D(texUnit0, vTexCoord) * materialFrontDiffuse;
}
"#;
        [("fs_cached".to_string(), fs.to_string())].into()
    }

    fn assembled() -> Assembled {
        assemble("fs_cached", Stage::Fragment, &Preamble::default(), &sources()).unwrap()
    }

    fn dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("mg-shader-cache-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    /// Compiled once and kept; read back the same program.
    #[test]
    fn a_program_compiled_once_is_read_back() {
        let d = dir("once");
        let cache = ProgramCache::new(&d);
        let a = assembled();
        let (first, how) = cache.compile(&a).unwrap();
        assert_eq!(how, Cached::Miss);
        let (second, how) = cache.compile(&a).unwrap();
        assert_eq!(how, Cached::Hit);
        assert_eq!(first.reflection, second.reflection);
        assert_eq!(first.files, second.files);
        // Another preamble is another program.
        let other = assemble(
            "fs_cached",
            Stage::Fragment,
            &Preamble { max_lights: 8, ..Preamble::default() },
            &sources(),
        )
        .unwrap();
        assert_eq!(cache.compile(&other).unwrap().1, Cached::Miss);
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 2);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Verify's check: glslang alone, kept as a compile keeps it (and a
    /// program compiled for an area's view is checked by reading it); one
    /// glslang refuses is an error and not kept; one only Moonglow's passes
    /// refuse (an array of samplers) is no error, as the game takes it.
    #[test]
    fn a_check_keeps_what_glslang_takes() {
        let d = dir("check");
        let cache = ProgramCache::new(&d);
        let a = assembled();
        assert_eq!(cache.check(&a).unwrap(), Cached::Miss);
        assert_eq!(cache.check(&a).unwrap(), Cached::Hit);
        assert_eq!(cache.compile(&a).unwrap().1, Cached::Hit, "the same entry");
        let other = assemble(
            "fs_cached",
            Stage::Fragment,
            &Preamble { quality: 0, ..Preamble::default() },
            &sources(),
        )
        .unwrap();
        cache.compile(&other).unwrap();
        assert_eq!(cache.check(&other).unwrap(), Cached::Hit, "compiled first");
        let mut broken = sources();
        broken.insert("fs_broken".into(), "void main () { gl_FragColor = vec4(nope); }\n".into());
        let b = assemble("fs_broken", Stage::Fragment, &Preamble::default(), &broken).unwrap();
        assert!(matches!(cache.check(&b), Err(ShaderError::Compile(_))));
        assert!(matches!(cache.check(&b), Err(ShaderError::Compile(_))), "not kept");
        let mut arrays = sources();
        arrays.insert(
            "fs_arrays".into(),
            "uniform sampler2D tex[2];\nvarying vec2 vTexCoord;\n\
             void main () { gl_FragColor = texture2D(tex[0], vTexCoord) + texture2D(tex[1], vTexCoord); }\n"
                .into(),
        );
        let c = assemble("fs_arrays", Stage::Fragment, &Preamble::default(), &arrays).unwrap();
        assert!(matches!(crate::compile_assembled(&c), Err(ShaderError::Spirv(_))));
        assert!(crate::check_assembled(&c).is_ok());
        assert_eq!(cache.check(&c).unwrap(), Cached::Miss);
        assert_eq!(cache.check(&c).unwrap(), Cached::Miss, "not kept: the passes refuse it");
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 2);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// An entry cut short at any length, or with a byte changed, is passed
    /// over (no panic) and written anew; garbage likewise.
    #[test]
    fn a_damaged_entry_is_compiled_again() {
        let d = dir("damaged");
        let cache = ProgramCache::new(&d);
        let a = assembled();
        cache.compile(&a).unwrap();
        let path = cache.path(ProgramCache::key(&a));
        let whole = std::fs::read(&path).unwrap();
        let key = ProgramCache::key(&a);
        for cut in (0..whole.len()).step_by(7) {
            assert!(read_entry(&whole[..cut], key, &a).is_none(), "{cut}");
        }
        for at in [0, 9, 30, whole.len() / 2, whole.len() - 1] {
            let mut bad = whole.clone();
            bad[at] ^= 0x55;
            assert!(read_entry(&bad, key, &a).is_none(), "{at}");
        }
        // Another program's entry under its name.
        let other = assemble(
            "fs_cached",
            Stage::Fragment,
            &Preamble { max_lights: 4, ..Preamble::default() },
            &sources(),
        )
        .unwrap();
        assert!(read_entry(&whole, ProgramCache::key(&other), &other).is_none());
        // On disk: cut, then compiled and written whole again.
        std::fs::write(&path, &whole[..whole.len() / 3]).unwrap();
        assert_eq!(cache.compile(&a).unwrap().1, Cached::Miss);
        assert_eq!(std::fs::read(&path).unwrap(), whole);
        std::fs::write(&path, b"\xff\xfe not an entry").unwrap();
        assert_eq!(cache.compile(&a).unwrap().1, Cached::Miss);
        assert_eq!(cache.compile(&a).unwrap().1, Cached::Hit);
        let _ = std::fs::remove_dir_all(&d);
    }
}
