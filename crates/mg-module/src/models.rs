//! Models kept as text (ASCII) compiled for a hak, by `mg_mdl::compile`:
//! what `mg pack --compile-models` and the hak editor do with them.

use mg_core::ResType;
use mg_resman::ResKey;

/// What the compiler left out or made up, each with the text's line
/// (from 1; 0: none).
pub type Notes = Vec<(usize, String)>;

/// The linter's notes on a model's text (`mg_mdl::lint`'s `Info`: what
/// the game or nwnmdlcomp does not read, a name longer than a resource's,
/// a second node of a name animations find the first of), by line from 1:
/// said with the compiler's, as Verify says the linter's warnings and
/// errors.
pub fn linter_notes(text: &str) -> Notes {
    notes_of(&mg_mdl::lint::check(text))
}

/// [`linter_notes`] of what the linter found.
fn notes_of(found: &[mg_mdl::lint::Diagnostic]) -> Notes {
    (found.iter())
        .filter(|d| d.severity == mg_mdl::lint::Severity::Info)
        .map(|d| (d.line + 1, d.message.clone()))
        .collect()
}

/// What became of one model kept as text.
#[derive(Debug, Clone, PartialEq)]
pub struct Compiled {
    pub model: ResKey,
    /// The compiled model and the compiler's notes, or why the model
    /// stays as text.
    pub result: Result<(Vec<u8>, Notes), String>,
}

/// Compiles the models among `files` that are text, each against what
/// there is: `lookup` gives a resource's file by name and type, where the
/// files themselves have none (the haks and the game): a model's
/// supermodels, the compiled model of its own name where one lies under it
/// (whose part numbers it keeps), and the materials (`.mtr`) a mesh with no
/// render hint takes its hint from (nwn-mdl-spec MDL-MESH-041). Compiled
/// models and other resources are passed over.
///
/// A model whose text has errors (`mg_mdl::lint`) or holds no node stays
/// as text, and so does one whose supermodel is not found: animations go by
/// part numbers, which are the supermodel's, and compiled without them a
/// creature would stand still in the game.
pub fn compile_models(
    files: &[(ResKey, &[u8])],
    lookup: &(dyn Fn(&str, ResType) -> Option<Vec<u8>> + Sync),
) -> Vec<Compiled> {
    compile_models_each(files, lookup, &|_, _| true)
}

/// [`compile_models`] on every core, telling `each` before every model
/// how many are behind and how many there are; it stops (with what is
/// compiled so far) once `each` says no. The models come back in the
/// files' order.
pub fn compile_models_each(
    files: &[(ResKey, &[u8])],
    lookup: &(dyn Fn(&str, ResType) -> Option<Vec<u8>> + Sync),
    each: &(dyn Fn(usize, usize) -> bool + Sync),
) -> Vec<Compiled> {
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    let own = |name: &str, t: ResType| {
        files
            .iter()
            .find(|(k, _)| k.restype == t && k.resref.to_string().eq_ignore_ascii_case(name))
            .map(|(_, d)| d.to_vec())
    };
    let find = |name: &str| own(name, ResType::MDL).or_else(|| lookup(name, ResType::MDL));
    // (A mesh with no render hint takes its material's.)
    let material = |name: &str| own(name, ResType::MTR).or_else(|| lookup(name, ResType::MTR));
    let texts: Vec<&(ResKey, &[u8])> = (files.iter())
        .filter(|(key, data)| key.restype == ResType::MDL && !mg_mdl::is_binary(data))
        .collect();
    let (done, stop) = (AtomicUsize::new(0), AtomicBool::new(false));
    let one = |key: &ResKey, data: &[u8]| -> Result<(Vec<u8>, Notes), String> {
        let name = key.resref.to_string();
        // (The reader takes nearly any text: what the game would refuse or
        // crash on, and what holds no model, is not compiled into
        // something that looks sound.)
        let text = String::from_utf8_lossy(data);
        let found = mg_mdl::lint::check(&text);
        let errors = (found.iter())
            .filter(|d| d.severity == mg_mdl::lint::Severity::Error)
            .map(|d| format!("line {}: {}", d.line + 1, d.message))
            .collect::<Vec<_>>();
        if let Some(first) = errors.first() {
            let more = errors.len() - 1;
            let rest = if more > 0 { format!(" (and {more} more)") } else { String::new() };
            return Err(format!("{first}{rest}"));
        }
        let model = mg_mdl::Model::read(data).map_err(|e| e.to_string())?;
        if model.nodes.is_empty() {
            return Err("its text holds no model".into());
        }
        if let Some(s) = model.supermodel.as_deref().filter(|s| !s.eq_ignore_ascii_case(&name))
            && find(s).is_none()
        {
            return Err(format!("its supermodel {s} was not found"));
        }
        // (The compiled model under it: not the text itself.)
        let before = |n: &str| lookup(n, ResType::MDL).filter(|d| mg_mdl::is_binary(d));
        let c = mg_mdl::compile::compile_named_with(data, &name, &find, &before, &material)
            .map_err(|e| e.to_string())?;
        let mut notes = c.notes;
        notes.extend(notes_of(&found));
        notes.sort();
        notes.dedup();
        Ok((c.binary, notes))
    };
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(texts.len());
    let mut out: Vec<Option<Compiled>> = vec![None; texts.len()];
    if threads == 0 {
        return Vec::new();
    }
    let chunk = texts.len().div_ceil(threads);
    std::thread::scope(|s| {
        for (part, into) in texts.chunks(chunk).zip(out.chunks_mut(chunk)) {
            let (done, stop, one) = (&done, &stop, &one);
            let total = texts.len();
            s.spawn(move || {
                for ((key, data), slot) in part.iter().zip(into) {
                    if stop.load(Ordering::Relaxed) || !each(done.load(Ordering::Relaxed), total) {
                        stop.store(true, Ordering::Relaxed);
                        return;
                    }
                    *slot = Some(Compiled { model: *key, result: one(key, data) });
                    done.fetch_add(1, Ordering::Relaxed);
                }
            });
        }
    });
    out.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mg_core::ResRef;

    const BASE: &str = "newmodel base\nsetsupermodel base NULL\nclassification character\n\
setanimationscale 1\nbeginmodelgeom base\n\
node dummy base\n  parent NULL\nendnode\n\
node dummy arm\n  parent base\nendnode\n\
endmodelgeom base\n\
newanim wave base\n  length 1\n  transtime 0.25\n  animroot base\n\
  node dummy base\n    parent NULL\n  endnode\n\
  node dummy arm\n    parent base\n    positionkey\n      0 0 0 0\n      1 0 0 1\n    endlist\n  endnode\n\
doneanim wave base\ndonemodel base\n";

    fn key(name: &str, t: ResType) -> ResKey {
        ResKey::parse(name, t).unwrap()
    }

    /// Text models are compiled, each against its supermodel where that is
    /// among the files or found outside; one whose supermodel is nowhere
    /// stays as text, and so does one that can't be read. What is compiled
    /// already, and what is no model, is passed over.
    #[test]
    fn text_models_compile_against_their_supermodels() {
        let child = BASE
            .replace("base", "child")
            .replace("setsupermodel child NULL", "setsupermodel child base");
        let orphan = child
            .replace("child", "orphan")
            .replace("setsupermodel orphan base", "setsupermodel orphan nowhere");
        let done = mg_mdl::compile::compile(BASE.as_bytes(), &Default::default()).unwrap().binary;
        let files: Vec<(ResKey, &[u8])> = vec![
            (key("child", ResType::MDL), child.as_bytes()),
            (key("base", ResType::MDL), BASE.as_bytes()),
            (key("orphan", ResType::MDL), orphan.as_bytes()),
            (key("broken", ResType::MDL), b"node\n"),
            (key("done", ResType::MDL), &done),
            (key("notes", ResType::TXT), b"newmodel x\n"),
        ];
        let out = compile_models(&files, &|_, _| None);
        let named: Vec<String> = out.iter().map(|c| c.model.resref.to_string()).collect();
        assert_eq!(named, ["child", "base", "orphan", "broken"]);
        let (child_bin, _) = out[0].result.clone().unwrap();
        assert!(mg_mdl::is_binary(&child_bin));
        // The child's nodes have its supermodel's numbers.
        let numbers = |d: &[u8]| mg_mdl::binary_write::PartNumbers::read(d).unwrap().numbers;
        assert_eq!(numbers(&child_bin), numbers(&out[1].result.clone().unwrap().0));
        assert!(out[2].result.as_ref().unwrap_err().contains("supermodel nowhere"));
        assert!(out[3].result.as_ref().unwrap_err().starts_with("line 1"), "{:?}", out[3].result);
        let empty = compile_models(&[(key("empty", ResType::MDL), b"# nothing\n")], &|_, _| None);
        assert!(empty[0].result.is_err());
        // Found outside the files: compiled.
        let outside = |n: &str, t: ResType| {
            (n == "nowhere" && t == ResType::MDL)
                .then(|| BASE.replace("base", "nowhere").into_bytes())
        };
        let out = compile_models(&files[2..3], &outside);
        assert!(out[0].result.is_ok(), "{:?}", out[0].result);
    }

    /// The linter's notes on a text come with the compiler's: what the
    /// game does not read (`setfillumcolor`, nwn-mdl-spec MDL-ASCII-070),
    /// by its line.
    #[test]
    fn the_linter_s_notes_are_said_with_the_compiler_s() {
        let text = BASE.replace(
            "node dummy arm\n  parent base\nendnode\n",
            "node dummy arm\n  parent base\n  setfillumcolor 1 0 0\nendnode\n",
        );
        let out = compile_models(&[(key("base", ResType::MDL), text.as_bytes())], &|_, _| None);
        let (_, notes) = out[0].result.clone().unwrap();
        let line = text.lines().position(|l| l.contains("setfillumcolor")).unwrap() + 1;
        assert!(
            notes.iter().any(|(l, n)| *l == line && n.contains("reads selfillumcolor")),
            "{notes:?}"
        );
        assert_eq!(linter_notes(&text).len(), 1);
    }

    /// A mesh with no render hint takes its material's (nwn-mdl-spec
    /// MDL-MESH-041): a normal-mapped one, found among the files, gives it
    /// tangents, as the game's compiler makes them.
    #[test]
    fn a_mesh_takes_its_material_s_render_hint() {
        let key = |n: &str, t| ResKey::new(ResRef::from_str(n).unwrap(), t);
        let model = "newmodel quad\nsetsupermodel quad NULL\nbeginmodelgeom quad\n\
            node dummy quad\n  parent NULL\nendnode\nnode trimesh q\n  parent quad\n\
            bitmap stone\n  verts 4\n    0 0 0\n    1 0 0\n    1 1 0\n    0 1 0\n\
            tverts 4\n    0 0 0\n    1 0 0\n    1 1 0\n    0 1 0\n\
            faces 2\n    0 1 2 1 0 1 2 0\n    0 2 3 1 0 2 3 0\nendnode\n\
            endmodelgeom quad\ndonemodel quad\n";
        let mtr = b"renderhint NormalAndSpecMapped\ntexture0 stone\n";
        // (The binary reader takes the tangent streams only with a hint of
        // the mesh's own: 1.69 files used them otherwise. Here they show as
        // four tangents and their signs, 64 bytes, more.)
        let size = |files: &[(ResKey, &[u8])]| {
            let out = compile_models(files, &|_, _| None);
            out[0].result.clone().unwrap().0.len()
        };
        let quad = (key("quad", ResType::MDL), model.as_bytes());
        let alone = size(&[quad]);
        assert_eq!(size(&[quad, (key("stone", ResType::MTR), mtr)]), alone + 64);
        let plain = b"renderhint None\ntexture0 stone\n";
        assert_eq!(size(&[quad, (key("stone", ResType::MTR), plain)]), alone);
    }
}
