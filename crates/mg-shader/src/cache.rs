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
//!
//! The folder is shared (the toolset's window and `mg`, Moonglow Viewer,
//! each of its own release), and a new release names every program anew,
//! so what no program reads any longer is removed: an entry is marked as
//! used when it is read (its file's time, a day at the least between
//! marks), and one not used for [`KEEP`] is removed when the cache is
//! opened ([`ProgramCache::new`]). By its age and not by its release:
//! another program's entries are another release's, and are in use.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use mg_core::bin::{Reader, WriteLe};

use crate::assemble::Assembled;
use crate::spirv::Io;
use crate::{Program, ShaderError};

/// The passes' format: changed when what they make of the same words does
/// (an entry of another format is never read).
const FORMAT: u32 = 4;

const MAGIC: &[u8; 8] = b"MGSHDSPV";

/// How long an entry no program has read is kept.
const KEEP: Duration = Duration::from_secs(30 * 24 * 3600);

/// How old an entry's mark of use is before a read renews it.
const MARK: Duration = Duration::from_secs(24 * 3600);

/// The folders opened by this process, each cleared of its old entries
/// once.
static OPENED: Mutex<BTreeSet<PathBuf>> = Mutex::new(BTreeSet::new());

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
    /// The cache in `dir` (made when the first entry is written). The
    /// first time a process opens a folder, the entries in it that no
    /// program has read for [`KEEP`] are removed.
    pub fn new(dir: impl Into<PathBuf>) -> ProgramCache {
        let cache = ProgramCache { dir: dir.into() };
        let first = OPENED.lock().is_ok_and(|mut opened| opened.insert(cache.dir.clone()));
        if first {
            cache.prune(SystemTime::now());
        }
        cache
    }

    /// Removes the entries (and what a write that was cut off left
    /// beside them) not used for [`KEEP`] before `now`; how many. Only
    /// files named as the cache names its own are touched.
    fn prune(&self, now: SystemTime) -> usize {
        let Ok(files) = std::fs::read_dir(&self.dir) else { return 0 };
        let mut removed = 0;
        for file in files.flatten() {
            let name = file.file_name();
            let own = name.to_str().and_then(|n| n.split_once('.')).is_some_and(|(key, ext)| {
                key.len() == 32
                    && key.bytes().all(|b| b.is_ascii_hexdigit())
                    && (ext == "spv" || ext.strip_prefix("tmp").is_some_and(digits))
            });
            let used = file.metadata().and_then(|m| m.modified());
            let old = used.is_ok_and(|t| now.duration_since(t).is_ok_and(|age| age > KEEP));
            if own && old && std::fs::remove_file(file.path()).is_ok() {
                removed += 1;
            }
        }
        removed
    }

    /// Marks an entry read as used now, where its mark is older than
    /// [`MARK`] (a write a day for an entry in use, not one a read).
    fn mark(path: &Path) {
        let now = SystemTime::now();
        let marked = std::fs::metadata(path).and_then(|m| m.modified());
        if marked.is_ok_and(|t| now.duration_since(t).is_ok_and(|age| age > MARK))
            && let Ok(file) = std::fs::File::options().write(true).open(path)
        {
            let _ = file.set_modified(now);
        }
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
            Self::mark(&path);
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
            Self::mark(&path);
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

fn digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
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
    /// glslang refuses is an error and not kept. (One only Moonglow's passes
    /// refused would be no error, as the game takes it, and not kept: since
    /// arrays of samplers are split, GLSL 330 has none.)
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
        // (An array of samplers, which the passes refused before they split
        // it, is kept as any other.)
        let mut arrays = sources();
        arrays.insert(
            "fs_arrays".into(),
            "uniform sampler2D tex[2];\nvarying vec2 vTexCoord;\n\
             void main () { gl_FragColor = texture2D(tex[0], vTexCoord) + texture2D(tex[1], vTexCoord); }\n"
                .into(),
        );
        let c = assemble("fs_arrays", Stage::Fragment, &Preamble::default(), &arrays).unwrap();
        assert!(crate::compile_assembled(&c).is_ok());
        assert_eq!(cache.check(&c).unwrap(), Cached::Miss);
        assert_eq!(cache.check(&c).unwrap(), Cached::Hit);
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 3);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// What no program has read for a month is removed when the cache is
    /// opened: every release names each program anew, and the old ones'
    /// entries stayed for good (245 MB on the machine this was written
    /// on; the review's M9). An entry read is marked as used, so one in
    /// use stays however long ago it was written, whichever release (or
    /// program: Moonglow Viewer shares the folder) wrote it; and nothing
    /// that is not the cache's own is touched.
    #[test]
    fn entries_not_read_for_a_month_are_removed_when_the_cache_is_opened() {
        let d = dir("pruned");
        // (Made without opening it: the folder is cleared once a process.)
        let cache = ProgramCache { dir: d.clone() };
        let a = assembled();
        cache.compile(&a).unwrap();
        let entry = cache.path(ProgramCache::key(&a));
        let aged = |path: &Path, days: u64| {
            let then = SystemTime::now() - Duration::from_secs(days * 24 * 3600);
            std::fs::File::options().write(true).open(path).unwrap().set_modified(then).unwrap();
        };
        let age = |path: &Path| {
            let at = std::fs::metadata(path).unwrap().modified().unwrap();
            SystemTime::now().duration_since(at).unwrap()
        };
        // Another release's entry, a write cut off, and files of someone
        // else's: all 40 days old. And another release's entry of last week.
        let old = d.join(format!("{:032x}.spv", 7));
        let cut = d.join(format!("{:032x}.tmp4242", 8));
        let recent = d.join(format!("{:032x}.spv", 9));
        let foreign = [d.join("notes.txt"), d.join("abc.spv"), d.join(format!("{:032x}.bak", 7))];
        for f in [&old, &cut, &recent].into_iter().chain(&foreign) {
            std::fs::write(f, b"x").unwrap();
            aged(f, if *f == recent { 7 } else { 40 });
        }
        // This one written 40 days ago and read since: marked as used.
        aged(&entry, 40);
        assert_eq!(cache.compile(&a).unwrap().1, Cached::Hit);
        assert!(age(&entry) < Duration::from_secs(3600), "marked: {:?}", age(&entry));
        // (One marked within the day is not written again.)
        aged(&recent, 0);
        let before = std::fs::metadata(&entry).unwrap().modified().unwrap();
        assert_eq!(cache.check(&a).unwrap(), Cached::Hit);
        assert_eq!(std::fs::metadata(&entry).unwrap().modified().unwrap(), before);
        aged(&recent, 7);
        // Opened: the two old ones go.
        let opened = ProgramCache::new(&d);
        assert!(!old.exists() && !cut.exists());
        assert!(entry.exists() && recent.exists());
        assert!(foreign.iter().all(|f| f.exists()));
        assert_eq!(opened.compile(&a).unwrap().1, Cached::Hit);
        // And once a process: a second opening looks at nothing.
        std::fs::write(&old, b"x").unwrap();
        aged(&old, 40);
        let _ = ProgramCache::new(&d);
        assert!(old.exists());
        assert_eq!(opened.prune(SystemTime::now()), 1);
        // A folder that is not there is no error.
        assert_eq!(ProgramCache::new(d.join("none")).prune(SystemTime::now()), 0);
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
